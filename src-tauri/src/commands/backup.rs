use crate::db::AppState;
use crate::error::{AppError, AppResult};
use rusqlite::{Connection, OpenFlags};
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::State;

/// Uses SQLite's Online Backup API (not a raw file copy) so the backup is
/// always consistent even while the app is running with WAL mode enabled.
#[tauri::command]
pub fn backup_database(state: State<AppState>, dest_path: String) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    let mut dst = rusqlite::Connection::open(&dest_path)?;
    {
        let backup = rusqlite::backup::Backup::new(&conn, &mut dst)?;
        backup.run_to_completion(5, std::time::Duration::from_millis(250), None)?;
    }
    Ok(())
}

// --- Restore -------------------------------------------------------------
//
// BUG #2 fix: the old restore path only checked for a table literally named
// `schema_migrations` before overwriting the live database. That is not a
// TIQR-specific signal - several common migration tools (Rails, Diesel,
// Knex, ...) use that exact table name - so picking an unrelated SQLite
// file could silently wipe out the user's real data. There was also no
// safety net: if the chosen file turned out to be wrong, or something
// failed partway through, there was no way back.
//
// This keeps the same restore mechanism (SQLite's Online Backup API,
// unchanged - see `backup_file_into` below) and the same forward-only
// migration runner (`db::run_migrations`, unchanged), and wraps three
// things around them: (1) a much stronger pre-check of the candidate file
// before it is allowed anywhere near the live connection, (2) an automatic
// safety backup of the *current* database taken right before anything is
// overwritten, and (3) automatic rollback to that safety backup if
// anything after it fails, so a bad restore can never leave the app worse
// off than before the attempt.

/// Tables that have existed since migration 001 and together identify a
/// genuine TIQR Manager database. Not an exhaustive schema dump - just
/// enough that an unrelated SQLite file (even one that happens to share a
/// table name or two) cannot pass.
const REQUIRED_TABLES: &[&str] = &[
    "schema_migrations",
    "app_settings",
    "counters",
    "platforms",
    "suppliers",
    "events",
    "orders",
    "tickets",
    "sales",
];

/// Baseline (001-era) columns checked on the tables most likely to collide
/// by name with some unrelated app's schema ("tickets", "orders", "sales"
/// and "events" are all common generic table names). Deliberately checked
/// against the *baseline* shape only, not columns added later by 002/003/004
/// (e.g. `refunded_at`, `batch_id`) - so a genuinely older TIQR backup,
/// taken before a later app update, still passes and is carried forward by
/// the normal forward-only migration runner after restore, exactly as it
/// is today.
const BASELINE_COLUMNS: &[(&str, &[&str])] = &[
    ("events", &["name", "status"]),
    ("orders", &["code", "event_id", "quantity"]),
    ("tickets", &["code", "event_id", "order_id", "status"]),
    ("sales", &["code", "ticket_id", "sale_price_cents", "payment_status"]),
];

fn not_a_valid_backup() -> AppError {
    AppError::Validation("This file is not a valid TIQR Manager backup.".into())
}

/// True if `table` has at least all of `required` among its columns.
fn has_columns(conn: &Connection, table: &str, required: &[&str]) -> bool {
    // `table` only ever comes from the constants above, never from user
    // input, so interpolating it into the SQL text is not an injection risk
    // (PRAGMA also does not accept table names as bound parameters).
    let sql = format!("PRAGMA table_info({table})");
    let mut stmt = match conn.prepare(&sql) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let existing: std::collections::HashSet<String> =
        match stmt.query_map([], |r| r.get::<_, String>(1)) {
            Ok(rows) => match rows.collect::<Result<_, _>>() {
                Ok(set) => set,
                Err(_) => return false,
            },
            Err(_) => return false,
        };
    required.iter().all(|c| existing.contains(*c))
}

/// The real validation gate. Opens the candidate file read-only (validation
/// must never modify it - not even the incidental `-wal`/`-shm` sidecar
/// files a normal read-write open can create) and rejects it unless it is
/// both a structurally sound SQLite database and recognizably a TIQR
/// Manager database. Every failure path returns the same user-facing
/// message, so a corrupted file and a wrong-app file are equally clearly
/// rejected without leaking internal check names.
fn validate_tiqr_backup(path: &Path) -> AppResult<()> {
    if !path.is_file() {
        return Err(not_a_valid_backup());
    }

    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|_| not_a_valid_backup())?;

    // Catches corrupted, truncated/incomplete, or not-actually-SQLite
    // files. Restore is a rare, explicit, user-initiated action and this
    // app's databases are at most tens of MB (see db.rs's perf_smoke
    // module), so the extra cost of a full check is a non-issue.
    let integrity: String = conn
        .query_row("PRAGMA integrity_check(1)", [], |r| r.get(0))
        .map_err(|_| not_a_valid_backup())?;
    if integrity != "ok" {
        return Err(not_a_valid_backup());
    }

    for table in REQUIRED_TABLES {
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
                [table],
                |r| r.get(0),
            )
            .map_err(|_| not_a_valid_backup())?;
        if !exists {
            return Err(not_a_valid_backup());
        }
    }

    // schema_migrations must have TIQR's own shape, not just its name -
    // rules out other tools' same-named migration-tracking tables (Rails/
    // Diesel/Knex-style trackers all use different columns).
    if !has_columns(&conn, "schema_migrations", &["version", "applied_at"]) {
        return Err(not_a_valid_backup());
    }
    // The single strongest signal that this file was actually produced by
    // this app family: migration 001 specifically recorded as applied.
    let has_001: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = '001_initial_schema')",
            [],
            |r| r.get(0),
        )
        .map_err(|_| not_a_valid_backup())?;
    if !has_001 {
        return Err(not_a_valid_backup());
    }

    for (table, cols) in BASELINE_COLUMNS {
        if !has_columns(&conn, table, cols) {
            return Err(not_a_valid_backup());
        }
    }

    Ok(())
}

/// The one mechanism used for every "replace a database's contents with
/// another file's" operation here - SQLite's Online Backup API, the same
/// one `backup_database` above always used. Used both for the actual
/// restore and, symmetrically, to roll back to the safety backup if
/// anything after it fails.
fn backup_file_into(src_path: &Path, dst: &mut Connection) -> rusqlite::Result<()> {
    let src = Connection::open_with_flags(src_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let backup = rusqlite::backup::Backup::new(&src, dst)?;
    backup.run_to_completion(5, std::time::Duration::from_millis(250), None)
}

/// Snapshots the *current* live database into `dir` before restore touches
/// anything, using the same Online Backup API as above - not a new backup
/// mechanism, just the existing one pointed at an app-managed location
/// instead of a user-chosen one, so a bad restore can always be undone.
fn create_safety_backup(conn: &Connection, dir: &Path) -> AppResult<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S%3f");
    let dest_path = dir.join(format!("pre-restore-{stamp}.sqlite3"));
    let mut dst = Connection::open(&dest_path)?;
    {
        let backup = rusqlite::backup::Backup::new(conn, &mut dst)?;
        backup.run_to_completion(5, std::time::Duration::from_millis(250), None)?;
    }
    Ok(dest_path)
}

/// Performs the actual overwrite-and-migrate: backs the candidate file into
/// the live connection, then runs the normal forward-only migration runner
/// so an older (but genuine) schema is carried forward exactly as it would
/// be on a normal app startup.
fn perform_restore(conn: &mut Connection, src_path: &Path) -> AppResult<()> {
    backup_file_into(src_path, conn)?;
    crate::db::run_migrations(conn)?;
    Ok(())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreOutcome {
    pub safety_backup_path: String,
}

/// Split out from the `restore_database` command so the full validate ->
/// safety-backup -> restore -> migrate -> (rollback on failure) sequence is
/// directly unit-testable against plain `&Connection`/`&Path` values,
/// without needing a real Tauri `AppHandle`. Same "impl fn + thin command"
/// pattern already used elsewhere in this codebase (list_tickets_impl,
/// create_sale_impl, ...).
pub(crate) fn restore_database_impl(
    conn: &mut Connection,
    src_path: &Path,
    safety_backup_dir: &Path,
) -> AppResult<RestoreOutcome> {
    // Validated against the candidate file only - `conn` (the live
    // database) is not touched at all if this fails.
    validate_tiqr_backup(src_path)?;

    // Safety net, taken before anything destructive: if the restore or the
    // post-restore migration below fails for any reason, we can always get
    // back to exactly this point.
    let safety_backup_path = create_safety_backup(conn, safety_backup_dir)?;

    match perform_restore(conn, src_path) {
        Ok(()) => Ok(RestoreOutcome {
            safety_backup_path: safety_backup_path.display().to_string(),
        }),
        Err(primary_err) => {
            // The live connection may already be partway overwritten (the
            // backup API is not something that can be wrapped in an outer
            // SQL transaction). Automatically roll back to the safety
            // backup taken moments ago so the app is never left worse off
            // than before this attempt.
            let rollback_result = match backup_file_into(&safety_backup_path, conn) {
                Ok(()) => crate::db::run_migrations(conn).map_err(AppError::from),
                Err(e) => Err(AppError::from(e)),
            };
            match rollback_result {
                Ok(()) => Err(AppError::Validation(format!(
                    "Restore failed, so your previous data was automatically restored and is unchanged. Details: {primary_err}"
                ))),
                Err(rollback_err) => Err(AppError::Other(format!(
                    "Restore failed ({primary_err}) and automatic recovery also failed ({rollback_err}). Your previous data is safely saved at {} - restore it manually from Settings.",
                    safety_backup_path.display()
                ))),
            }
        }
    }
}

/// Lightweight pre-check the frontend calls immediately after the user
/// picks a file, before showing the "this will replace your data"
/// confirmation - so a doomed restore is rejected with a clear message
/// right away instead of behind a scary confirmation dialog. Calls the
/// exact same validation `restore_database` itself relies on as the real
/// safety boundary, so the two can never drift apart.
#[tauri::command]
pub fn validate_backup_file(src_path: String) -> AppResult<()> {
    validate_tiqr_backup(Path::new(&src_path))
}

/// Restores the live database from a chosen backup file. The frontend is
/// responsible for relaunching the app right after this succeeds (via the
/// process plugin's `relaunch()`), so every screen reloads with fresh data
/// instead of risking stale in-memory state.
///
/// 2.0.72: derives `db_path` from `state.db_path` (the CURRENTLY active
/// file - whichever account is signed in right now) rather than always the
/// original legacy file, so a restore for a per-account file puts its safety
/// backup next to THAT account's own folder, not always next to marko's. No
/// longer needs `tauri::AppHandle` at all now that it isn't calling
/// `resolve_db_path` itself - Tauri simply stops injecting the parameter,
/// nothing else (the invoke_handler entry, the frontend call) needs to know.
#[tauri::command]
pub fn restore_database(state: State<AppState>, src_path: String) -> AppResult<RestoreOutcome> {
    let mut conn = state.db.lock().unwrap();
    let db_path = state.db_path.lock().unwrap().clone();
    let safety_dir = db_path
        .parent()
        .ok_or_else(|| AppError::Other("Could not resolve app data directory".into()))?
        .join("safety-backups");
    restore_database_impl(&mut conn, Path::new(&src_path), &safety_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_conn;
    use std::fs;

    /// A fresh, guaranteed-unique-within-this-process directory under the
    /// OS temp dir. Safe under `cargo test`'s parallel test execution
    /// without needing any new dependency.
    fn unique_temp_dir(label: &str) -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("tiqr_test_{label}_{}_{n}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Dumps an existing (possibly in-memory) connection out to a real file
    /// on disk, using the same Online Backup API under test - a convenient,
    /// realistic way to build file-backed fixtures from a seeded
    /// `test_conn()`.
    fn dump_conn_to_file(conn: &Connection, path: &Path) {
        let mut dst = Connection::open(path).unwrap();
        let backup = rusqlite::backup::Backup::new(conn, &mut dst).unwrap();
        backup
            .run_to_completion(5, std::time::Duration::from_millis(250), None)
            .unwrap();
    }

    fn seed_event(conn: &Connection, name: &str) {
        conn.execute("INSERT INTO events (name) VALUES (?1)", [name]).unwrap();
    }

    fn first_event_name(conn: &Connection) -> String {
        conn.query_row("SELECT name FROM events LIMIT 1", [], |r| r.get(0))
            .unwrap()
    }

    /// Builds a file-backed DB manually stopped at migrations 001-003 (the
    /// schema shape from before BUG #1's migration 004 existed), with one
    /// refunded sale. Mirrors the same "existing installation upgrades"
    /// scenario already covered in db.rs's migration_004_tests, but
    /// exercised here through the *restore* path: an older, genuinely-TIQR
    /// backup must still be accepted and correctly carried forward by the
    /// normal forward-only migration runner after restore, not rejected as
    /// "incompatible".
    fn write_pre_004_backup(path: &Path) {
        let conn = Connection::open(path).unwrap();
        conn.execute_batch(include_str!("../../migrations/001_initial_schema.sql"))
            .unwrap();
        conn.execute_batch(include_str!("../../migrations/002_refunds.sql"))
            .unwrap();
        conn.execute_batch(include_str!("../../migrations/003_sale_batch_id.sql"))
            .unwrap();
        conn.execute_batch(
            "CREATE TABLE schema_migrations (version TEXT PRIMARY KEY, applied_at TEXT NOT NULL);",
        )
        .unwrap();
        for v in ["001_initial_schema", "002_refunds", "003_sale_batch_id"] {
            conn.execute(
                "INSERT INTO schema_migrations(version, applied_at) VALUES (?1, '2026-01-01T00:00:00Z')",
                [v],
            )
            .unwrap();
        }
        // Ticket status is 'available', matching what a real refund_sale_impl
        // call always leaves behind (it flips the ticket back to available
        // in the same transaction that marks the sale 'refunded') - this
        // fixture hand-writes the end state of a genuine sell-then-refund,
        // not the refund call itself.
        conn.execute_batch(
            "INSERT INTO events (id, name) VALUES (1, 'Old Event');
             INSERT INTO orders (id, code, event_id, purchase_date, quantity, unit_price_cents)
               VALUES (1, 'ORD-000001', 1, '2026-01-01', 1, 1000);
             INSERT INTO tickets (id, code, event_id, order_id, status)
               VALUES (1, 'TIX-000001', 1, 1, 'available');
             INSERT INTO sales (id, code, ticket_id, sale_date, sale_price_cents, payment_status, refunded_at, refund_reason)
               VALUES (1, 'SAL-000001', 1, '2026-01-02', 2000, 'refunded', '2026-01-03T00:00:00Z', 'test refund');
             -- Real code generation (codes::next_code) increments these
             -- counters on every insert; since the rows above were inserted
             -- directly rather than through that path, the counters must be
             -- bumped to match, or a later create_sale_impl call in the
             -- live app would regenerate 'SAL-000001' and collide with it.
             UPDATE counters SET value = 1 WHERE name = 'order';
             UPDATE counters SET value = 1 WHERE name = 'ticket';
             UPDATE counters SET value = 1 WHERE name = 'sale';",
        )
        .unwrap();
    }

    /// Requirement: a valid TIQR backup restores successfully. Also checks
    /// (7) that the automatic safety backup is created *before* the live
    /// database is overwritten - by reading the safety backup's own
    /// contents back and confirming it holds the pre-restore data, not the
    /// newly-restored data.
    #[test]
    fn valid_tiqr_backup_restores_successfully_and_creates_a_correct_safety_backup() {
        let dir = unique_temp_dir("valid");
        let mut live = test_conn();
        seed_event(&live, "Live Event Before Restore");

        let candidate = test_conn();
        seed_event(&candidate, "Candidate Backup Event");
        let candidate_path = dir.join("candidate.sqlite3");
        dump_conn_to_file(&candidate, &candidate_path);

        let safety_dir = dir.join("safety-backups");
        let outcome = restore_database_impl(&mut live, &candidate_path, &safety_dir).unwrap();

        // (6) Data is correctly restored.
        assert_eq!(first_event_name(&live), "Candidate Backup Event");

        // (7) The safety backup exists and holds the *pre*-restore data.
        let safety_path = Path::new(&outcome.safety_backup_path);
        assert!(safety_path.is_file(), "safety backup file must exist");
        let safety_conn = Connection::open(safety_path).unwrap();
        assert_eq!(first_event_name(&safety_conn), "Live Event Before Restore");
    }

    /// (2) A random, unrelated (but structurally valid) SQLite file must be
    /// rejected, and (5) the live database must remain completely
    /// untouched.
    #[test]
    fn restore_rejects_an_unrelated_sqlite_file_and_leaves_live_database_untouched() {
        let dir = unique_temp_dir("unrelated");
        let mut live = test_conn();
        seed_event(&live, "Untouched Event");

        let other_path = dir.join("other_app.sqlite3");
        let other = Connection::open(&other_path).unwrap();
        other
            .execute_batch("CREATE TABLE unrelated_app_table (id INTEGER PRIMARY KEY, whatever TEXT);")
            .unwrap();
        drop(other);

        let safety_dir = dir.join("safety-backups");
        let err = restore_database_impl(&mut live, &other_path, &safety_dir).unwrap_err();

        assert_eq!(err.to_string(), "This file is not a valid TIQR Manager backup.");
        assert_eq!(first_event_name(&live), "Untouched Event");
        // Validation failed before any safety backup was even attempted.
        assert!(!safety_dir.exists());
    }

    /// (3) A corrupted (not-actually-a-database) file must be rejected, and
    /// (5) the live database must remain untouched.
    #[test]
    fn restore_rejects_a_corrupted_file_and_leaves_live_database_untouched() {
        let dir = unique_temp_dir("corrupted");
        let mut live = test_conn();
        seed_event(&live, "Untouched Event");

        let garbage_path = dir.join("garbage.sqlite3");
        fs::write(&garbage_path, b"this is not a sqlite database, just plain bytes").unwrap();

        let safety_dir = dir.join("safety-backups");
        let err = restore_database_impl(&mut live, &garbage_path, &safety_dir).unwrap_err();

        assert_eq!(err.to_string(), "This file is not a valid TIQR Manager backup.");
        assert_eq!(first_event_name(&live), "Untouched Event");
        assert!(!safety_dir.exists());
    }

    /// Bonus coverage for "incomplete backup" (section 6's threat list): a
    /// truncated copy of an otherwise-valid TIQR file (as if the copy was
    /// interrupted partway) must also be rejected, not partially restored.
    #[test]
    fn restore_rejects_a_truncated_incomplete_backup_and_leaves_live_database_untouched() {
        let dir = unique_temp_dir("truncated");
        let mut live = test_conn();
        seed_event(&live, "Untouched Event");

        let full = test_conn();
        seed_event(&full, "Complete Candidate Event");
        let full_path = dir.join("full.sqlite3");
        dump_conn_to_file(&full, &full_path);

        let bytes = fs::read(&full_path).unwrap();
        assert!(bytes.len() > 200, "fixture should be larger than the truncation point");
        let truncated_path = dir.join("truncated.sqlite3");
        fs::write(&truncated_path, &bytes[..200]).unwrap();

        let safety_dir = dir.join("safety-backups");
        let err = restore_database_impl(&mut live, &truncated_path, &safety_dir).unwrap_err();

        assert_eq!(err.to_string(), "This file is not a valid TIQR Manager backup.");
        assert_eq!(first_event_name(&live), "Untouched Event");
        assert!(!safety_dir.exists());
    }

    /// (4) THE key regression test: a SQLite file that has a table literally
    /// named `schema_migrations` (as several unrelated migration tools also
    /// use) but none of TIQR's actual domain tables must be rejected. Under
    /// the pre-fix validation (a bare `EXISTS` check on `schema_migrations`
    /// alone) this exact file would have been wrongly accepted - this is
    /// the regression BUG #2 fixes.
    #[test]
    fn restore_rejects_a_file_with_a_schema_migrations_table_but_no_tiqr_schema() {
        let dir = unique_temp_dir("fake_migrations");
        let mut live = test_conn();
        seed_event(&live, "Untouched Event");

        let fake_path = dir.join("fake.sqlite3");
        let fake = Connection::open(&fake_path).unwrap();
        fake.execute_batch(
            "CREATE TABLE schema_migrations (version TEXT PRIMARY KEY, applied_at TEXT NOT NULL);
             INSERT INTO schema_migrations(version, applied_at) VALUES ('some_other_apps_migration', '2020-01-01');",
        )
        .unwrap();
        drop(fake);

        let safety_dir = dir.join("safety-backups");
        let err = restore_database_impl(&mut live, &fake_path, &safety_dir).unwrap_err();

        assert_eq!(err.to_string(), "This file is not a valid TIQR Manager backup.");
        assert_eq!(first_event_name(&live), "Untouched Event");
        assert!(!safety_dir.exists());
    }

    /// Defends the *structural* (column-level) part of validation
    /// specifically: a file with all the right table *names* (including a
    /// correctly-shaped `schema_migrations` with `001_initial_schema`
    /// recorded) but unrelated column shapes - as if some other app happens
    /// to use identical generic table names like "tickets"/"orders" - must
    /// still be rejected.
    #[test]
    fn restore_rejects_a_file_with_matching_table_names_but_wrong_columns() {
        let dir = unique_temp_dir("wrong_columns");
        let mut live = test_conn();
        seed_event(&live, "Untouched Event");

        let fake_path = dir.join("fake_shape.sqlite3");
        let fake = Connection::open(&fake_path).unwrap();
        fake.execute_batch(
            "CREATE TABLE schema_migrations (version TEXT PRIMARY KEY, applied_at TEXT NOT NULL);
             INSERT INTO schema_migrations(version, applied_at) VALUES ('001_initial_schema', '2020-01-01');
             CREATE TABLE app_settings (k TEXT);
             CREATE TABLE counters (k TEXT);
             CREATE TABLE platforms (k TEXT);
             CREATE TABLE suppliers (k TEXT);
             CREATE TABLE events (id INTEGER PRIMARY KEY, movie_title TEXT);
             CREATE TABLE orders (id INTEGER PRIMARY KEY, customer_name TEXT);
             CREATE TABLE tickets (id INTEGER PRIMARY KEY, seat_row INTEGER);
             CREATE TABLE sales (id INTEGER PRIMARY KEY, total REAL);",
        )
        .unwrap();
        drop(fake);

        let safety_dir = dir.join("safety-backups");
        let err = restore_database_impl(&mut live, &fake_path, &safety_dir).unwrap_err();

        assert_eq!(err.to_string(), "This file is not a valid TIQR Manager backup.");
        assert_eq!(first_event_name(&live), "Untouched Event");
        assert!(!safety_dir.exists());
    }

    /// (8) An older, genuinely-TIQR backup (pre-004, see BUG #1) must still
    /// be accepted and correctly carried forward by the normal forward-only
    /// migration runner after restore - ties BUG #1 and BUG #2 together:
    /// restoring an old backup must not break the refund/resell fix, and
    /// refund history must survive the round trip.
    #[test]
    fn restore_accepts_an_older_pre_migration_004_backup_and_migrations_carry_it_forward() {
        let dir = unique_temp_dir("older_schema");
        let mut live = test_conn();
        seed_event(&live, "Live Event Before Restore");

        let old_backup_path = dir.join("pre_004_backup.sqlite3");
        write_pre_004_backup(&old_backup_path);

        let safety_dir = dir.join("safety-backups");
        restore_database_impl(&mut live, &old_backup_path, &safety_dir).unwrap();

        // run_migrations must have carried the restored DB all the way
        // forward to 004, exactly as it would on a normal app startup.
        let has_004: bool = live
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = '004_sales_active_unique')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(has_004, "restore must carry an older backup forward through all pending migrations");

        // BUG #1's fix must be reachable end to end through a restored,
        // just-migrated-forward database: the refunded ticket can be resold...
        let second_sale = crate::models::SaleInput {
            ticket_id: 1,
            platform_id: None,
            sale_date: "2026-02-01".to_string(),
            sale_price_cents: 1800,
            selling_fees_cents: 0,
            payment_status: Some("paid".to_string()),
            buyer_reference: None,
            notes: None,
        };
        crate::commands::sales::create_sale_impl(&mut live, &second_sale)
            .expect("previously-refunded ticket must be resellable after restore + migrate");

        // ...and the original refund is still there (history preserved).
        let statuses: Vec<String> = {
            let mut stmt = live
                .prepare("SELECT payment_status FROM sales WHERE ticket_id = 1 ORDER BY id")
                .unwrap();
            stmt.query_map([], |r| r.get(0))
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap()
        };
        assert_eq!(statuses, vec!["refunded".to_string(), "paid".to_string()]);
    }

    /// Safety-net test for the automatic-rollback path itself: constructs a
    /// candidate file that passes the structural pre-check (baseline 001
    /// shape, `schema_migrations` correctly shaped with `001_initial_schema`
    /// recorded) but cannot actually be carried forward by `run_migrations`
    /// (it already has migration 002's column under a different history,
    /// so re-applying 002 fails with "duplicate column name"). This is a
    /// deliberately artificial way to reach the "restore started, then
    /// failed" branch deterministically, to prove the safety net itself
    /// works - not a claim that this exact scenario occurs naturally.
    #[test]
    fn restore_automatically_rolls_back_if_migrating_the_restored_db_fails() {
        let dir = unique_temp_dir("rollback");
        let mut live = test_conn();
        seed_event(&live, "Live Event Before Rollback Test");

        let bad_path = dir.join("bad.sqlite3");
        let bad = Connection::open(&bad_path).unwrap();
        bad.execute_batch(include_str!("../../migrations/001_initial_schema.sql"))
            .unwrap();
        // Pre-empt what migration 002 will try to do, without recording 002
        // as applied - run_migrations will attempt it again and fail.
        bad.execute_batch("ALTER TABLE sales ADD COLUMN refunded_at TEXT;")
            .unwrap();
        bad.execute_batch(
            "CREATE TABLE schema_migrations (version TEXT PRIMARY KEY, applied_at TEXT NOT NULL);
             INSERT INTO schema_migrations(version, applied_at) VALUES ('001_initial_schema', '2026-01-01T00:00:00Z');",
        )
        .unwrap();
        drop(bad);

        let safety_dir = dir.join("safety-backups");
        let err = restore_database_impl(&mut live, &bad_path, &safety_dir).unwrap_err();

        assert!(
            err.to_string().contains("automatically restored"),
            "error should explain that automatic recovery happened, got: {err}"
        );
        // The live database must be back to exactly what it was before this
        // attempt - not left partially overwritten.
        assert_eq!(first_event_name(&live), "Live Event Before Rollback Test");
        // The safety backup step itself did run before the failure.
        assert!(fs::read_dir(&safety_dir).unwrap().count() >= 1);
    }
}

use rusqlite::Connection;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tauri::Manager;

/// Shared app state: a single mutex-guarded SQLite connection.
/// The app is single-user/local-first, so a single serialized connection
/// is simple and safe, and plenty fast for tens of thousands of rows.
///
/// 2.0.72: "single-user" above is now "single signed-in ACCOUNT at a time",
/// not "single file forever" - see commands::database::switch_active_database.
/// `db` can be swapped out mid-session to point at a completely different
/// account's own file. `db_path` tracks which file that currently is, purely
/// so restore_database/get_app_info can report/derive things relative to the
/// CURRENT file instead of always the original legacy one. LOCK ORDERING
/// RULE: whenever a function needs both fields together, lock `db` first,
/// then `db_path`, inside the same critical section - never as two separate
/// lock/unlock cycles. This is what stops a caller from ever observing the
/// new connection paired with the old path (or vice versa) mid-switch.
/// `switch_active_database_impl` is the only place that writes `db_path`,
/// and it already follows this order.
pub struct AppState {
    pub db: Mutex<Connection>,
    pub db_path: Mutex<PathBuf>,
    /// 2.0.12: lets a "Cancel" click interrupt an in-flight "Sign in with
    /// Google" wait (see google_oauth::run_sign_in/accept_one_redirect) -
    /// `Some` only while commands::google_auth::start_google_sign_in is
    /// actually blocked waiting for Google's browser redirect, `None`
    /// otherwise (including right after an attempt finishes on its own, so a
    /// stale cancel can never reach a later, unrelated sign-in attempt). A
    /// plain `Mutex`, not part of the `db` one above - cancelling must never
    /// have to wait on whatever the database happens to be doing.
    pub oauth_cancel_flag: Mutex<Option<Arc<AtomicBool>>>,
    /// 2.0.46: the same idea as `oauth_cancel_flag` above, but for the
    /// SEPARATE "Continue with Google" app sign-in button
    /// (commands::firebase_google_auth) - a genuinely different in-flight
    /// attempt from the Sheets one above, so it gets its own slot rather
    /// than sharing: cancelling one must never accidentally interrupt the
    /// other if both somehow ended up in flight at once.
    pub firebase_oauth_cancel_flag: Mutex<Option<Arc<AtomicBool>>>,
    /// 2.1.9: every currently-open Price Checker "Visible Scanner" window,
    /// keyed by the request/session id `PriceChecker.tsx` mints per "Open &
    /// Scan" click (same "frontend mints the id, backend only ever echoes
    /// it back" convention the old auto-check's `request_id` already
    /// established) - see commands::price_checker_scanner's own module doc
    /// comment for the full design. Unlike the single-slot cancel flags
    /// above, this is a MAP: the old hidden auto-check could only ever have
    /// one attempt in flight (a single background thread, a single slot),
    /// but a visible window is something marko looks at and interacts with
    /// directly, so nothing stops him opening StubHub AND Vivid Seats side
    /// by side - each gets its own independent entry here, and one
    /// marketplace's window misbehaving can never affect another's (marko's
    /// own explicit spec requirement).
    pub price_scanner_sessions: Mutex<HashMap<u64, ScannerSession>>,
}

/// One open Visible Scanner window's live state (2.1.9). Lives here, next to
/// `AppState` itself, rather than in `commands::price_checker_scanner` - the
/// same reasoning `AppState` itself already follows: this is shared mutable
/// app state a command module reaches INTO, not something private to one
/// command function. Never persisted to SQLite - see migrations/
/// 018_price_checker_scanner.sql's own doc comment for why the only thing
/// that ever reaches the database is the final, marko-reviewed
/// `price_checks` row, through the ordinary `save_price_check` command.
#[derive(Debug)]
pub struct ScannerSession {
    /// Tauri window label for `AppHandle::get_webview_window` - the actual
    /// `WebviewWindow` handle itself is never stored here; every command
    /// that needs it looks it up fresh by label, matching how this app
    /// already looks up its own main window (see lib.rs's single-instance
    /// plugin callback, `app.get_webview_window("main")`).
    pub window_label: String,
    pub event_id: i64,
    pub marketplace_id: i64,
    /// Flips to interrupt an in-flight `scan_visible_prices` eval - same
    /// `AtomicBool`-checked-during-a-bounded-wait pattern every other
    /// cancellable operation in this codebase already uses (see
    /// google_oauth::accept_one_redirect). Per-session, not a single shared
    /// slot - cancelling one marketplace's scan must never touch another's.
    pub cancel_flag: Arc<AtomicBool>,
    /// "ready" | "scanning" | "success" | "partial" | "unable_to_read" |
    /// "blocked" | "error" - see commands::price_checker_scanner's own
    /// `derive_session_status` for exactly how this is computed after each
    /// scan (reflects the ACCUMULATED session, not just the latest scan's
    /// own delta - see that function's own doc comment).
    pub status: String,
    /// Deduplicated across every scan so far this session - see
    /// commands::price_checker_scanner's own `merge_scan_into_session`.
    pub listings: Vec<crate::models::NormalizedListing>,
    /// The fingerprint of every listing already in `listings` above - kept
    /// alongside it (not recomputed from it) so a repeat scan's dedup check
    /// is a single HashSet lookup per candidate, not an O(n) rescan.
    pub fingerprints: HashSet<String>,
    pub scan_count: u32,
    pub last_scan_at: Option<String>,
}

const MIGRATIONS: &[(&str, &str)] = &[
    (
        "001_initial_schema",
        include_str!("../migrations/001_initial_schema.sql"),
    ),
    (
        "002_refunds",
        include_str!("../migrations/002_refunds.sql"),
    ),
    (
        "003_sale_batch_id",
        include_str!("../migrations/003_sale_batch_id.sql"),
    ),
    (
        "004_sales_active_unique",
        include_str!("../migrations/004_sales_active_unique.sql"),
    ),
    (
        "005_pulls",
        include_str!("../migrations/005_pulls.sql"),
    ),
    (
        "006_pulls_seat_fields",
        include_str!("../migrations/006_pulls_seat_fields.sql"),
    ),
    (
        "007_payments",
        include_str!("../migrations/007_payments.sql"),
    ),
    (
        "008_sheet_sync",
        include_str!("../migrations/008_sheet_sync.sql"),
    ),
    (
        "009_orders_external_reference",
        include_str!("../migrations/009_orders_external_reference.sql"),
    ),
    (
        "010_ticket_resale_delivery_status",
        include_str!("../migrations/010_ticket_resale_delivery_status.sql"),
    ),
    (
        "011_pulls_received",
        include_str!("../migrations/011_pulls_received.sql"),
    ),
    (
        "012_event_categories",
        include_str!("../migrations/012_event_categories.sql"),
    ),
    (
        "013_notifications",
        include_str!("../migrations/013_notifications.sql"),
    ),
    (
        "014_price_checker",
        include_str!("../migrations/014_price_checker.sql"),
    ),
    (
        "015_finance",
        include_str!("../migrations/015_finance.sql"),
    ),
    (
        "016_finance_v2",
        include_str!("../migrations/016_finance_v2.sql"),
    ),
    (
        "017_price_checker_viagogo",
        include_str!("../migrations/017_price_checker_viagogo.sql"),
    ),
    (
        "018_price_checker_scanner",
        include_str!("../migrations/018_price_checker_scanner.sql"),
    ),
    (
        "019_price_checker_market_analysis",
        include_str!("../migrations/019_price_checker_market_analysis.sql"),
    ),
    (
        "020_remove_stubhub",
        include_str!("../migrations/020_remove_stubhub.sql"),
    ),
];

/// Resolves the per-user, per-installation database file path.
/// On Windows this lives under `%APPDATA%\com.tiqrmanager.app\` (never inside
/// the Program Files install folder), on Linux under `~/.local/share/...`.
pub fn resolve_db_path(app: &tauri::AppHandle) -> anyhow::Result<PathBuf> {
    let dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join("tiqr-manager.sqlite3"))
}

/// 2.0.72: the one thing standing between a corrupted/unexpected Firebase
/// uid value and a path-traversal-shaped filename. Rejects (never silently
/// strips-and-continues) anything outside a plain alphanumeric/underscore/
/// hyphen allowlist, plus empty input and anything implausibly long (real
/// Firebase uids are short, fixed-length, URL-safe strings - 128 is a very
/// generous ceiling, not a measured limit).
pub fn sanitize_uid_for_filename(uid: &str) -> anyhow::Result<String> {
    if uid.is_empty() {
        anyhow::bail!("uid must not be empty");
    }
    if uid.len() > 128 {
        anyhow::bail!("uid is implausibly long ({} chars)", uid.len());
    }
    if !uid.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        anyhow::bail!("uid contains characters outside [A-Za-z0-9_-]");
    }
    Ok(uid.to_string())
}

/// 2.0.72: per-account counterpart to `resolve_db_path` above - one whole
/// separate database file per signed-in Firebase account, so every existing
/// query in this codebase keeps working completely unchanged (see
/// commands::database's own module doc comment for the full reasoning).
/// Deliberately NESTED (`users/<uid>/tiqr-manager.sqlite3`, not a flat
/// `users/<uid>.sqlite3`) so that `db_path.parent()` - already used by
/// restore_database to derive its `safety-backups` sibling directory -
/// naturally lands inside that one account's own folder, with zero
/// special-casing needed there.
pub fn resolve_user_db_path(app: &tauri::AppHandle, uid: &str) -> anyhow::Result<PathBuf> {
    let safe_uid = sanitize_uid_for_filename(uid)?;
    let dir = app.path().app_data_dir()?.join("users").join(safe_uid);
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join("tiqr-manager.sqlite3"))
}

pub fn open_connection(path: &std::path::Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    // journal_mode returns a row with the resulting mode, so query_row it explicitly.
    let _mode: String = conn.query_row("PRAGMA journal_mode=WAL", [], |r| r.get(0))?;
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA synchronous = NORMAL;
         PRAGMA busy_timeout = 5000;",
    )?;
    Ok(conn)
}

/// Very small forward-only migration runner. Each migration is applied at
/// most once, tracked in `schema_migrations`. Future features (invoices,
/// payments, fx rates, ...) just add a new `002_xxx.sql` file + array entry.
pub fn run_migrations(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version TEXT PRIMARY KEY,
            applied_at TEXT NOT NULL
         );",
    )?;
    for (version, sql) in MIGRATIONS {
        let already: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = ?1)",
            [version],
            |r| r.get(0),
        )?;
        if already {
            continue;
        }
        conn.execute_batch(sql)?;
        conn.execute(
            "INSERT INTO schema_migrations(version, applied_at) VALUES (?1, strftime('%Y-%m-%dT%H:%M:%fZ','now'))",
            [version],
        )?;
    }
    Ok(())
}

/// In-memory, fully-migrated connection for unit tests. Foreign keys are
/// enabled (in-memory DBs default them off, same as any SQLite connection)
/// so delete-safety / cascade behaviour is exercised exactly like production.
#[cfg(test)]
pub fn test_conn() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory db");
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .expect("enable foreign keys");
    run_migrations(&conn).expect("run migrations");
    conn
}

#[cfg(test)]
mod sanitize_uid_tests {
    use super::sanitize_uid_for_filename;

    #[test]
    fn a_normal_firebase_shaped_uid_passes_through_unchanged() {
        assert_eq!(sanitize_uid_for_filename("abc123XYZ_-9").unwrap(), "abc123XYZ_-9");
    }

    #[test]
    fn the_same_input_always_gives_the_same_output() {
        let a = sanitize_uid_for_filename("Sameuid123").unwrap();
        let b = sanitize_uid_for_filename("Sameuid123").unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn two_different_uids_give_two_different_outputs() {
        let a = sanitize_uid_for_filename("userAAA").unwrap();
        let b = sanitize_uid_for_filename("userBBB").unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn a_uid_containing_path_traversal_characters_is_rejected_not_mangled() {
        assert!(sanitize_uid_for_filename("../../etc/passwd").is_err());
        assert!(sanitize_uid_for_filename("a/b").is_err());
        assert!(sanitize_uid_for_filename("a.b").is_err());
    }

    #[test]
    fn an_empty_uid_is_rejected() {
        assert!(sanitize_uid_for_filename("").is_err());
    }

    #[test]
    fn an_implausibly_long_uid_is_rejected() {
        let too_long = "a".repeat(129);
        assert!(sanitize_uid_for_filename(&too_long).is_err());
    }
}

/// Regression tests for a real bug an independent review of 2.1.6 found:
/// `run_migrations` applies each migration file via a plain `execute_batch`
/// with no transaction of its own (see that function's own doc comment) -
/// so a migration that fails partway leaves its earlier statements
/// committed and itself unrecorded in `schema_migrations`, meaning every
/// later launch retries the SAME sql against an already-half-applied
/// schema. Migration 017 (Viagogo added, StubHub retired) is now wrapped in
/// its own explicit transaction specifically because of this - see that
/// file's own doc comment for the full story - these tests prove the
/// actual scenario that motivated it stays fixed.
#[cfg(test)]
mod migration_017_safety_tests {
    use super::*;

    fn migrated_through_016() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        conn.execute_batch(
            "CREATE TABLE schema_migrations (version TEXT PRIMARY KEY, applied_at TEXT NOT NULL);",
        )
        .unwrap();
        for (version, sql) in &MIGRATIONS[..16] {
            conn.execute_batch(sql).unwrap();
            conn.execute(
                "INSERT INTO schema_migrations(version, applied_at) VALUES (?1, strftime('%Y-%m-%dT%H:%M:%fZ','now'))",
                [version],
            )
            .unwrap();
        }
        conn
    }

    #[test]
    fn migration_017_succeeds_even_if_a_viagogo_marketplace_already_exists() {
        // Simulates the exact scenario the review found: something (in
        // practice, only create_marketplace could do this - no UI calls it
        // today, but it IS a real, registered command) creates a 'Viagogo'
        // marketplace row before migration 017 ever gets a chance to run.
        let conn = migrated_through_016();
        conn.execute("INSERT INTO marketplaces(name) VALUES ('Viagogo')", []).unwrap();

        // Must not error, and must not leave migration 017 half-applied -
        // this calls the REAL run_migrations, not a hand-replayed 017.
        run_migrations(&conn).expect("migration 017 must tolerate a pre-existing Viagogo row, not brick the app");

        let viagogo_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM marketplaces WHERE name = 'Viagogo'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(viagogo_count, 1, "must never end up with two Viagogo rows");

        let applied: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = '017_price_checker_viagogo')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(applied, "017 must be recorded as applied, not silently skipped or left pending forever");

        // The rest of the chain must still have run too - a partial,
        // silently-rolled-back-except-for-the-conflicting-statement outcome
        // would be just as bad as a hard failure. 2.2.0: StubHub is now
        // fully DELETED by migrations/020_remove_stubhub.sql (marko's own
        // explicit follow-up request, confirmed via AskUserQuestion) rather
        // than just retired by 017 - see that migration's own doc comment.
        // run_migrations always runs the whole chain, so asserting it's
        // gone here still proves the chain ran to completion past 017,
        // which is this test's real point.
        let stubhub_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM marketplaces WHERE name = 'StubHub'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(stubhub_count, 0, "StubHub must be fully gone once the whole migration chain (through 020) has run");
    }

    #[test]
    fn migration_017_still_works_normally_with_no_pre_existing_viagogo_row() {
        // The ordinary case (every real install today) - OR IGNORE/the
        // transaction wrapper must change nothing about it.
        let conn = migrated_through_016();

        run_migrations(&conn).unwrap();

        let viagogo: (String, bool) = conn
            .query_row("SELECT name, active FROM marketplaces WHERE name = 'Viagogo'", [], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap();
        assert_eq!(viagogo, ("Viagogo".to_string(), true));
        // 2.2.0: StubHub is fully DELETED by migrations/020_remove_
        // stubhub.sql by the time the whole chain has run, not just
        // retired by 017 anymore - see that migration's own doc comment.
        let stubhub_count: i64 =
            conn.query_row("SELECT COUNT(*) FROM marketplaces WHERE name = 'StubHub'", [], |r| r.get(0)).unwrap();
        assert_eq!(stubhub_count, 0);
    }
}

/// Performance sanity check (section 10 of the stability pass): seeds a
/// realistic-shape, disk-backed (WAL mode, same pragmas as production)
/// database at 10k/50k/100k tickets and times the exact query shapes the
/// Dashboard, Tickets, Sales and Orders screens run, plus checks the
/// filtered queries hit an index instead of a full table scan.
///
/// Ignored by default (`cargo test` stays fast) - run explicitly with:
///   cargo test --release -- --ignored --nocapture perf_smoke
#[cfg(test)]
mod perf_smoke {
    use super::*;
    use std::time::Instant;

    struct TempDb {
        path: std::path::PathBuf,
    }
    impl TempDb {
        fn new(tag: &str) -> Self {
            let pid = std::process::id();
            let path = std::env::temp_dir().join(format!("tiqr_perf_{tag}_{pid}.sqlite3"));
            let _ = std::fs::remove_file(&path);
            TempDb { path }
        }
    }
    impl Drop for TempDb {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
            let _ = std::fs::remove_file(self.path.with_extension("sqlite3-wal"));
            let _ = std::fs::remove_file(self.path.with_extension("sqlite3-shm"));
        }
    }

    /// Seeds `n_tickets` tickets spread across ~`n_tickets/50` orders and
    /// ~`n_tickets/200` events, with ~70% of tickets sold (so the sales
    /// table is large too), matching the general shape of a real reseller
    /// database rather than one giant order.
    fn seed(conn: &mut Connection, n_tickets: i64) {
        let tx = conn.transaction().unwrap();
        let n_events = (n_tickets / 200).max(1);
        let n_orders = (n_tickets / 50).max(1);
        for i in 0..n_events {
            tx.execute(
                "INSERT INTO events (name, event_date, status) VALUES (?1, ?2, 'upcoming')",
                rusqlite::params![
                    format!("Perf Event {i}"),
                    format!("2026-{:02}-01", (i % 12) + 1)
                ],
            )
            .unwrap();
        }
        for i in 0..n_orders {
            let event_id = (i % n_events) + 1;
            tx.execute(
                "INSERT INTO orders (code, event_id, purchase_date, quantity, unit_price_cents, fees_cents, total_cost_cents, currency)
                 VALUES (?1, ?2, ?3, 1, 5000, 100, 5100, 'EUR')",
                rusqlite::params![format!("PERF-ORD-{i}"), event_id, format!("2026-{:02}-15", (i % 12) + 1)],
            )
            .unwrap();
        }
        for i in 0..n_tickets {
            let order_id = (i % n_orders) + 1;
            let event_id = (i % n_events) + 1;
            let sold = i % 10 < 7; // ~70% sold
            let status = if sold { "sold" } else { "available" };
            tx.execute(
                "INSERT INTO tickets (code, event_id, order_id, purchase_cost_cents, purchase_fees_cents, currency, status)
                 VALUES (?1, ?2, ?3, 5000, 100, 'EUR', ?4)",
                rusqlite::params![format!("PERF-TKT-{i}"), event_id, order_id, status],
            )
            .unwrap();
            if sold {
                let ticket_id = i + 1;
                tx.execute(
                    "INSERT INTO sales (code, ticket_id, sale_date, sale_price_cents, selling_fees_cents, currency, payment_status)
                     VALUES (?1, ?2, ?3, 7000, 200, 'EUR', 'paid')",
                    rusqlite::params![format!("PERF-SAL-{i}"), ticket_id, format!("2026-{:02}-20", (i % 12) + 1)],
                )
                .unwrap();
            }
        }
        tx.commit().unwrap();
    }

    fn explain_uses_index(conn: &Connection, sql: &str, params: &[&dyn rusqlite::ToSql]) -> String {
        let plan_sql = format!("EXPLAIN QUERY PLAN {sql}");
        let mut stmt = conn.prepare(&plan_sql).unwrap();
        let rows = stmt
            .query_map(params, |r| r.get::<_, String>(3))
            .unwrap();
        rows.map(|r| r.unwrap()).collect::<Vec<_>>().join(" | ")
    }

    fn run_at_scale(n_tickets: i64) {
        let db = TempDb::new(&n_tickets.to_string());
        let mut conn = open_connection(&db.path).expect("open temp db");
        run_migrations(&conn).expect("migrate");

        let t_seed = Instant::now();
        seed(&mut conn, n_tickets);
        eprintln!("[perf {n_tickets}] seed: {:?}", t_seed.elapsed());

        // ---- Dashboard: currency-mix distinct scan -----------------------
        let t = Instant::now();
        conn.query_row(
            "SELECT COUNT(*) FROM (SELECT DISTINCT currency FROM tickets)",
            [],
            |r| r.get::<_, i64>(0),
        )
        .unwrap();
        eprintln!("[perf {n_tickets}] distinct currency: {:?}", t.elapsed());

        // ---- Dashboard: inventory snapshot (filtered by currency) --------
        let t = Instant::now();
        conn.query_row(
            "SELECT COUNT(*), COALESCE(SUM(purchase_cost_cents+purchase_fees_cents+other_costs_cents),0)
             FROM tickets WHERE currency = ?1",
            ["EUR"],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)),
        )
        .unwrap();
        eprintln!("[perf {n_tickets}] inventory snapshot: {:?}", t.elapsed());
        let plan = explain_uses_index(
            &conn,
            "SELECT COUNT(*) FROM tickets WHERE currency = ?1",
            &[&"EUR"],
        );
        eprintln!("[perf {n_tickets}] plan(tickets by currency): {plan}");

        // ---- Dashboard: period-filtered purchase/sales aggregates --------
        let t = Instant::now();
        conn.query_row(
            "SELECT COUNT(t.id), COALESCE(SUM(t.purchase_cost_cents+t.purchase_fees_cents+t.other_costs_cents),0)
             FROM tickets t JOIN orders o ON o.id = t.order_id
             WHERE o.purchase_date BETWEEN ?1 AND ?2 AND t.currency = ?3",
            rusqlite::params!["0001-01-01", "9999-12-31", "EUR"],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)),
        )
        .unwrap();
        eprintln!("[perf {n_tickets}] period purchase aggregate: {:?}", t.elapsed());

        let t = Instant::now();
        conn.query_row(
            "SELECT COUNT(*), COALESCE(SUM(s.sale_price_cents),0)
             FROM sales s JOIN tickets t ON t.id = s.ticket_id
             WHERE s.sale_date BETWEEN ?1 AND ?2 AND s.currency = ?3 AND s.payment_status != 'refunded'",
            rusqlite::params!["0001-01-01", "9999-12-31", "EUR"],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)),
        )
        .unwrap();
        eprintln!("[perf {n_tickets}] period sales aggregate: {:?}", t.elapsed());

        // ---- Tickets screen: filtered by event (should use idx_tickets_event) --
        let plan = explain_uses_index(
            &conn,
            "SELECT t.id FROM tickets t WHERE t.event_id = ?1",
            &[&1i64],
        );
        eprintln!("[perf {n_tickets}] plan(tickets by event_id): {plan}");
        assert!(
            plan.contains("USING INDEX") || plan.contains("idx_tickets_event"),
            "tickets-by-event should hit an index, got: {plan}"
        );

        // ---- Tickets screen: unfiltered full list (worst case, no filters) --
        let t = Instant::now();
        let mut stmt = conn
            .prepare(
                "SELECT t.id, t.code, t.event_id, e.name, t.order_id, o.code,
                    t.section, t.row_label, t.seat, t.ticket_type,
                    t.purchase_cost_cents, t.purchase_fees_cents, t.other_costs_cents,
                    t.listing_price_cents, t.currency, t.status, t.notes, t.is_demo,
                    t.created_at, t.updated_at, sa.sale_price_cents
                 FROM tickets t
                 JOIN events e ON e.id = t.event_id
                 JOIN orders o ON o.id = t.order_id
                 LEFT JOIN sales sa ON sa.ticket_id = t.id AND sa.payment_status != 'refunded'
                 ORDER BY t.id DESC",
            )
            .unwrap();
        let rows: Vec<i64> = stmt
            .query_map([], |r| r.get::<_, i64>(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        eprintln!(
            "[perf {n_tickets}] full unfiltered ticket list ({} rows): {:?}",
            rows.len(),
            t.elapsed()
        );

        // ---- Sales screen: unfiltered full list ---------------------------
        let t = Instant::now();
        let mut stmt = conn
            .prepare(
                "SELECT s.id, s.code, s.ticket_id, t.code, t.event_id, e.name,
                    s.platform_id, p.name, s.sale_date, s.sale_price_cents, s.selling_fees_cents,
                    s.currency, s.payment_status, s.buyer_reference, s.notes, s.is_demo, s.created_at, s.updated_at,
                    s.refunded_at, s.refund_reason,
                    (t.purchase_cost_cents + t.purchase_fees_cents + t.other_costs_cents)
                 FROM sales s
                 JOIN tickets t ON t.id = s.ticket_id
                 JOIN events e ON e.id = t.event_id
                 LEFT JOIN platforms p ON p.id = s.platform_id
                 ORDER BY s.sale_date DESC, s.id DESC",
            )
            .unwrap();
        let rows: Vec<i64> = stmt
            .query_map([], |r| r.get::<_, i64>(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        eprintln!(
            "[perf {n_tickets}] full unfiltered sales list ({} rows): {:?}",
            rows.len(),
            t.elapsed()
        );

        // ---- Tickets/Inventory screen (order-grouped list), unfiltered ---
        // Mirrors list_orders_impl's BASE_SQL: GROUP BY o.id over a LEFT JOIN
        // to every one of that order's tickets (using idx_tickets_order).
        // This is now what the Tickets page actually loads on open, so it
        // must stay fast even though it aggregates every ticket in the
        // database on an unfiltered load.
        let plan = explain_uses_index(
            &conn,
            "SELECT o.id FROM orders o LEFT JOIN tickets t ON t.order_id = o.id GROUP BY o.id",
            &[],
        );
        eprintln!("[perf {n_tickets}] plan(order-grouped tickets list): {plan}");
        let t = Instant::now();
        let mut stmt = conn
            .prepare(
                "SELECT o.id,
                    COUNT(CASE WHEN t.status='sold' THEN 1 END),
                    COUNT(CASE WHEN t.status='available' THEN 1 END),
                    COUNT(CASE WHEN t.status='listed' THEN 1 END),
                    COUNT(CASE WHEN t.status='cancelled' THEN 1 END)
                 FROM orders o
                 JOIN events e ON e.id = o.event_id
                 LEFT JOIN suppliers sup ON sup.id = o.supplier_id
                 LEFT JOIN platforms p ON p.id = o.platform_id
                 LEFT JOIN tickets t ON t.order_id = o.id
                 GROUP BY o.id
                 ORDER BY o.purchase_date DESC, o.id DESC",
            )
            .unwrap();
        let rows: Vec<i64> = stmt
            .query_map([], |r| r.get::<_, i64>(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        eprintln!(
            "[perf {n_tickets}] order-grouped tickets list ({} orders): {:?}",
            rows.len(),
            t.elapsed()
        );

        // ---- Sales screen (batch-grouped list), unfiltered ---------------
        // Mirrors list_sale_groups_impl's GROUP_BASE_SELECT: GROUP BY the
        // expression COALESCE(batch_id, 'single:'||id), so every unfiltered
        // load scans and aggregates every `sales` row - no index can speed up
        // grouping by an expression. The seed data gives this the worst case
        // for grouping overhead (every sale is its own group of one, i.e. the
        // maximum possible group count) rather than a few huge batches.
        let plan = explain_uses_index(
            &conn,
            "SELECT COUNT(*) FROM sales s GROUP BY COALESCE(s.batch_id, 'single:' || s.id)",
            &[],
        );
        eprintln!("[perf {n_tickets}] plan(batch-grouped sales list): {plan}");
        let t = Instant::now();
        let mut stmt = conn
            .prepare(
                "SELECT MIN(s.id),
                    COUNT(*),
                    COALESCE(SUM(CASE WHEN s.payment_status != 'refunded' THEN s.sale_price_cents END), 0)
                 FROM sales s
                 JOIN tickets t ON t.id = s.ticket_id
                 JOIN events e ON e.id = t.event_id
                 LEFT JOIN platforms p ON p.id = s.platform_id
                 GROUP BY COALESCE(s.batch_id, 'single:' || s.id)
                 ORDER BY MAX(s.sale_date) DESC, MIN(s.id) DESC",
            )
            .unwrap();
        let rows: Vec<i64> = stmt
            .query_map([], |r| r.get::<_, i64>(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        eprintln!(
            "[perf {n_tickets}] batch-grouped sales list ({} groups): {:?}",
            rows.len(),
            t.elapsed()
        );

        eprintln!("[perf {n_tickets}] db file size: {} bytes", std::fs::metadata(&db.path).map(|m| m.len()).unwrap_or(0));
    }

    #[test]
    #[ignore]
    fn perf_10k() {
        run_at_scale(10_000);
    }

    #[test]
    #[ignore]
    fn perf_50k() {
        run_at_scale(50_000);
    }

    #[test]
    #[ignore]
    fn perf_100k() {
        run_at_scale(100_000);
    }
}

/// Regression coverage for migration 004 (BUG #1 fix: a refunded ticket
/// could never be resold, because sales.ticket_id was UNIQUE across ALL
/// rows forever, not just active ones). Unlike test_conn() (which always
/// applies every migration to a brand-new empty database, i.e. what a FRESH
/// install experiences), this simulates the scenario that actually matters
/// most for a schema-rebuild migration: an EXISTING v1.4.0 install, with
/// real data already sitting under the OLD schema, opening the app after an
/// update and having migration 004 run against that real data for the first
/// time.
#[cfg(test)]
mod migration_004_tests {
    use super::*;
    use crate::commands::orders::insert_order_with_tickets;
    use crate::models::OrderInput;

    #[test]
    fn migration_004_preserves_existing_data_and_fixes_refund_resell_on_upgrade() {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch("PRAGMA foreign_keys = ON;")
            .expect("enable foreign keys");

        // Apply exactly what every existing v1.4.0 install already has on
        // disk today: migrations 001-003, nothing more. This mirrors
        // run_migrations' own bookkeeping so the real run_migrations() call
        // below sees a legitimate "003 already applied" state.
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version TEXT PRIMARY KEY,
                applied_at TEXT NOT NULL
             );",
        )
        .unwrap();
        for (version, sql) in &MIGRATIONS[..3] {
            conn.execute_batch(sql).unwrap();
            conn.execute(
                "INSERT INTO schema_migrations(version, applied_at) VALUES (?1, strftime('%Y-%m-%dT%H:%M:%fZ','now'))",
                [version],
            )
            .unwrap();
        }

        // Seed real-shaped pre-existing data under the OLD schema (where
        // ticket_id UNIQUE still covers every row, refunded or not).
        conn.execute("INSERT INTO events (name) VALUES ('Old Event')", [])
            .unwrap();
        let event_id = conn.last_insert_rowid();
        let order_input = OrderInput {
            event_id,
            supplier_id: None,
            platform_id: None,
            purchase_date: "2026-01-01".to_string(),
            quantity: 3,
            unit_price_cents: 1000,
            fees_cents: 0,
            other_costs_cents: 0,
            currency: "EUR".to_string(),
            payment_status: Some("paid".to_string()),
            notes: None,
            ticket_type: None,
            section: None,
            row_label: None,
            seats: None,
        };
        let order_id = insert_order_with_tickets(&conn, &order_input, false).unwrap();
        let tickets: Vec<i64> = {
            let mut stmt = conn
                .prepare("SELECT id FROM tickets WHERE order_id=?1 ORDER BY id")
                .unwrap();
            stmt.query_map([order_id], |r| r.get::<_, i64>(0))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
        };

        // Ticket 0: sold, still active.
        conn.execute(
            "INSERT INTO sales (code, ticket_id, sale_date, sale_price_cents, payment_status)
             VALUES ('SAL-000001', ?1, '2026-02-01', 2000, 'paid')",
            [tickets[0]],
        )
        .unwrap();
        conn.execute("UPDATE tickets SET status='sold' WHERE id=?1", [tickets[0]])
            .unwrap();
        // Ticket 1: sold, then refunded - exactly the row that used to make
        // this ticket permanently unsellable under the old schema.
        conn.execute(
            "INSERT INTO sales (code, ticket_id, sale_date, sale_price_cents, payment_status, refunded_at, refund_reason)
             VALUES ('SAL-000002', ?1, '2026-02-02', 1500, 'refunded', '2026-02-03T00:00:00.000Z', 'buyer cancelled')",
            [tickets[1]],
        )
        .unwrap();
        conn.execute("UPDATE tickets SET status='available' WHERE id=?1", [tickets[1]])
            .unwrap();
        // Ticket 2: never sold.

        let snapshot_before: Vec<(i64, String, i64, i64, String, Option<String>, Option<String>)> = {
            let mut stmt = conn
                .prepare("SELECT id, code, ticket_id, sale_price_cents, payment_status, refunded_at, refund_reason FROM sales ORDER BY id")
                .unwrap();
            stmt.query_map([], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?))
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
        };
        assert_eq!(snapshot_before.len(), 2);

        // This is the real upgrade moment: an existing user opens the app
        // after updating. run_migrations sees 001-003 already recorded, so
        // it applies ONLY 004 - exactly like a real upgrade would.
        run_migrations(&conn).expect("migration 004 must apply cleanly on top of real pre-existing data");

        // Nothing lost, nothing changed, nothing duplicated by the rebuild.
        let snapshot_after: Vec<(i64, String, i64, i64, String, Option<String>, Option<String>)> = {
            let mut stmt = conn
                .prepare("SELECT id, code, ticket_id, sale_price_cents, payment_status, refunded_at, refund_reason FROM sales ORDER BY id")
                .unwrap();
            stmt.query_map([], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?))
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
        };
        assert_eq!(
            snapshot_before, snapshot_after,
            "every pre-existing sale row must survive the table rebuild byte-for-byte"
        );

        // Referential integrity must be clean after the rebuild.
        let violations: Vec<String> = {
            let mut stmt = conn.prepare("PRAGMA foreign_key_check").unwrap();
            stmt.query_map([], |r| r.get::<_, String>(0))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
        };
        assert!(
            violations.is_empty(),
            "foreign_key_check must be clean after the rebuild, found: {violations:?}"
        );

        // The actual bug fix: ticket 1 (refunded) can now be sold again.
        conn.execute(
            "INSERT INTO sales (code, ticket_id, sale_date, sale_price_cents, payment_status)
             VALUES ('SAL-000003', ?1, '2026-03-01', 1800, 'pending')",
            [tickets[1]],
        )
        .expect("a previously-refunded ticket must be sellable again after migration 004");

        // AUTOINCREMENT/id continuity: the new row's id must be strictly
        // greater than every pre-existing id - never reused, never collided
        // - even though the table itself was dropped and recreated.
        let new_id: i64 = conn
            .query_row("SELECT id FROM sales WHERE code='SAL-000003'", [], |r| r.get(0))
            .unwrap();
        let max_old_id = snapshot_before.iter().map(|r| r.0).max().unwrap();
        assert!(
            new_id > max_old_id,
            "id sequence must continue past pre-existing ids, not collide: new_id={new_id}, max_old_id={max_old_id}"
        );

        // History fully intact: the refunded row AND the new active sale
        // both exist side by side for this one ticket.
        let ticket1_sales: i64 = conn
            .query_row("SELECT COUNT(*) FROM sales WHERE ticket_id=?1", [tickets[1]], |r| r.get(0))
            .unwrap();
        assert_eq!(
            ticket1_sales, 2,
            "refund history plus the new sale must both be visible - nothing overwritten"
        );

        // Still impossible to have two ACTIVE sales of the same ticket at once.
        let dup = conn.execute(
            "INSERT INTO sales (code, ticket_id, sale_date, sale_price_cents, payment_status)
             VALUES ('SAL-000004', ?1, '2026-03-02', 999, 'pending')",
            [tickets[1]],
        );
        assert!(
            dup.is_err(),
            "a ticket must never have two simultaneously active sales, even after the rebuild"
        );
    }
}

/// Regression coverage for migration 007 (the `payments` table). The 2.0.0
/// Payments Ledger feature that used to read/write this table was reverted
/// in 2.0.1 (marko tried it and decided against it) - but the migration
/// itself, and any real `payments` rows an already-installed 2.0.0 build may
/// have written, must never be touched: migrations are forward-only (see
/// this file's module doc comment), and this table can hold real user data
/// on an upgrade from 2.0.0. This test mirrors migration_004_tests' own
/// approach (simulate a real EXISTING install - here, v1.9.10, migrations
/// 001-006 already applied, with real sales/orders data sitting under the
/// old schema - opening the app after an update) but only proves the
/// migration itself is still safe and the table it creates is usable via
/// plain SQL - it no longer exercises payments.rs (removed in 2.0.1, since
/// nothing in the app calls into it any more).
#[cfg(test)]
mod migration_007_tests {
    use super::*;
    use crate::commands::orders::insert_order_with_tickets;
    use crate::models::OrderInput;

    #[test]
    fn migration_007_preserves_existing_data_and_creates_a_usable_empty_ledger_on_upgrade() {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch("PRAGMA foreign_keys = ON;")
            .expect("enable foreign keys");

        // Apply exactly what every existing v1.9.10 install already has on
        // disk today: migrations 001-006, nothing more.
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version TEXT PRIMARY KEY,
                applied_at TEXT NOT NULL
             );",
        )
        .unwrap();
        for (version, sql) in &MIGRATIONS[..6] {
            conn.execute_batch(sql).unwrap();
            conn.execute(
                "INSERT INTO schema_migrations(version, applied_at) VALUES (?1, strftime('%Y-%m-%dT%H:%M:%fZ','now'))",
                [version],
            )
            .unwrap();
        }

        // Seed real pre-existing data under the pre-payments-ledger schema:
        // an order and a sale, exactly as any real installation would have
        // right up until the moment it upgrades.
        conn.execute("INSERT INTO events (name) VALUES ('Old Event')", [])
            .unwrap();
        let event_id = conn.last_insert_rowid();
        let order_input = OrderInput {
            event_id,
            supplier_id: None,
            platform_id: None,
            purchase_date: "2026-01-01".to_string(),
            quantity: 1,
            unit_price_cents: 1000,
            fees_cents: 0,
            other_costs_cents: 0,
            currency: "EUR".to_string(),
            payment_status: Some("paid".to_string()),
            notes: None,
            ticket_type: None,
            section: None,
            row_label: None,
            seats: None,
        };
        let order_id = insert_order_with_tickets(&conn, &order_input, false).unwrap();
        let ticket_id: i64 = conn
            .query_row("SELECT id FROM tickets WHERE order_id=?1", [order_id], |r| r.get(0))
            .unwrap();
        conn.execute(
            "INSERT INTO sales (code, ticket_id, sale_date, sale_price_cents, payment_status)
             VALUES ('SAL-000001', ?1, '2026-02-01', 2000, 'paid')",
            [ticket_id],
        )
        .unwrap();
        let sale_id: i64 = conn.query_row("SELECT id FROM sales WHERE code='SAL-000001'", [], |r| r.get(0)).unwrap();

        let orders_before: Vec<(i64, String, String)> = {
            let mut stmt = conn.prepare("SELECT id, code, payment_status FROM orders ORDER BY id").unwrap();
            stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap().collect::<Result<Vec<_>, _>>().unwrap()
        };
        let sales_before: Vec<(i64, String, i64, String)> = {
            let mut stmt = conn.prepare("SELECT id, code, sale_price_cents, payment_status FROM sales ORDER BY id").unwrap();
            stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))).unwrap().collect::<Result<Vec<_>, _>>().unwrap()
        };

        // The real upgrade moment: run_migrations sees 001-006 already
        // recorded, so it applies ONLY 007 - exactly like a real upgrade.
        run_migrations(&conn).expect("migration 007 must apply cleanly on top of real pre-existing data");

        // Nothing about existing orders/sales changed at all - 007 is pure
        // addition, it touches neither table.
        let orders_after: Vec<(i64, String, String)> = {
            let mut stmt = conn.prepare("SELECT id, code, payment_status FROM orders ORDER BY id").unwrap();
            stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap().collect::<Result<Vec<_>, _>>().unwrap()
        };
        let sales_after: Vec<(i64, String, i64, String)> = {
            let mut stmt = conn.prepare("SELECT id, code, sale_price_cents, payment_status FROM sales ORDER BY id").unwrap();
            stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))).unwrap().collect::<Result<Vec<_>, _>>().unwrap()
        };
        assert_eq!(orders_before, orders_after, "every pre-existing order must survive the upgrade untouched");
        assert_eq!(sales_before, sales_after, "every pre-existing sale must survive the upgrade untouched");

        // The table is immediately usable via plain SQL against this
        // pre-existing data, even though nothing in the app writes to it any
        // more after 2.0.1 - a stray already-upgraded 2.0.0 install must
        // never see the table become unusable/corrupt.
        conn.execute(
            "INSERT INTO payments (code, sale_group_key, amount_cents, currency, payment_date, method)
             VALUES ('PAY-TEST-000001', ?1, 2000, 'EUR', '2026-04-01', 'cash')",
            [format!("single:{sale_id}")],
        )
        .expect("the payments table must stay usable after upgrading, even though the app no longer writes to it");

        // Referential integrity must be clean after the upgrade.
        let violations: Vec<String> = {
            let mut stmt = conn.prepare("PRAGMA foreign_key_check").unwrap();
            stmt.query_map([], |r| r.get::<_, String>(0)).unwrap().collect::<Result<Vec<_>, _>>().unwrap()
        };
        assert!(violations.is_empty(), "foreign_key_check must be clean after the upgrade, found: {violations:?}");
    }

    #[test]
    fn migration_007_on_a_completely_fresh_database_starts_with_an_empty_ledger() {
        // The other half of "old database -> new database": a BRAND NEW
        // install (test_conn() applies every migration, including 007, to
        // an empty database) must start with zero payments - upgrading
        // must never fabricate payment history that never happened.
        let conn = test_conn();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM payments", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 0);
    }
}

/// Regression coverage for migration 008 (`sheet_sync_links` - Google Sheets
/// sync, see commands/sheets_sync.rs and google_sheets.rs). Same shape as
/// migration_007_tests: prove the migration is safe on top of a real
/// existing install's data, and that a brand-new database starts with no
/// sync history fabricated out of nothing.
#[cfg(test)]
mod migration_008_tests {
    use super::*;

    #[test]
    fn migration_008_preserves_existing_data_and_creates_a_usable_empty_table_on_upgrade() {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch("PRAGMA foreign_keys = ON;").expect("enable foreign keys");

        // Apply exactly what every existing v2.0.1 install already has on
        // disk today: migrations 001-007, nothing more.
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version TEXT PRIMARY KEY,
                applied_at TEXT NOT NULL
             );",
        )
        .unwrap();
        for (version, sql) in &MIGRATIONS[..7] {
            conn.execute_batch(sql).unwrap();
            conn.execute(
                "INSERT INTO schema_migrations(version, applied_at) VALUES (?1, strftime('%Y-%m-%dT%H:%M:%fZ','now'))",
                [version],
            )
            .unwrap();
        }

        // Seed real pre-existing data (a pull, exactly the kind of record
        // 008 exists to sync) under the pre-sync schema.
        conn.execute(
            "INSERT INTO pulls (code, buyer_name, event_name, quantity, price_cents, currency, transfer_done)
             VALUES ('PULL-000001', 'A Buyer', 'An Event', 2, 5000, 'EUR', 0)",
            [],
        )
        .unwrap();
        let pulls_before: Vec<(i64, String, String)> = {
            let mut stmt = conn.prepare("SELECT id, code, buyer_name FROM pulls ORDER BY id").unwrap();
            stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap().collect::<Result<Vec<_>, _>>().unwrap()
        };

        // The real upgrade moment: run_migrations sees 001-007 already
        // recorded, so it applies ONLY 008 - exactly like a real upgrade.
        run_migrations(&conn).expect("migration 008 must apply cleanly on top of real pre-existing data");

        let pulls_after: Vec<(i64, String, String)> = {
            let mut stmt = conn.prepare("SELECT id, code, buyer_name FROM pulls ORDER BY id").unwrap();
            stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap().collect::<Result<Vec<_>, _>>().unwrap()
        };
        assert_eq!(pulls_before, pulls_after, "every pre-existing pull must survive the upgrade untouched");

        // The new table is immediately usable via plain SQL.
        conn.execute(
            "INSERT INTO sheet_sync_links (data_source, local_id, sheet_marker, last_synced_snapshot, last_synced_at)
             VALUES ('pulls', 1, 'PULL-000001', '{}', strftime('%Y-%m-%dT%H:%M:%fZ','now'))",
            [],
        )
        .expect("the sheet_sync_links table must be usable right after the upgrade");

        let violations: Vec<String> = {
            let mut stmt = conn.prepare("PRAGMA foreign_key_check").unwrap();
            stmt.query_map([], |r| r.get::<_, String>(0)).unwrap().collect::<Result<Vec<_>, _>>().unwrap()
        };
        assert!(violations.is_empty(), "foreign_key_check must be clean after the upgrade, found: {violations:?}");
    }

    #[test]
    fn migration_008_on_a_completely_fresh_database_starts_with_no_sync_links() {
        let conn = test_conn();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM sheet_sync_links", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn migration_008_rejects_a_duplicate_link_for_the_same_data_source_and_local_id() {
        // The primary key is the whole safety property here: it's what
        // stops a repeat sync from ever double-linking the same local
        // record to two different sheet rows.
        let conn = test_conn();
        conn.execute(
            "INSERT INTO sheet_sync_links (data_source, local_id, sheet_marker, last_synced_snapshot, last_synced_at)
             VALUES ('pulls', 1, 'PULL-000001', '{}', strftime('%Y-%m-%dT%H:%M:%fZ','now'))",
            [],
        )
        .unwrap();
        let dup = conn.execute(
            "INSERT INTO sheet_sync_links (data_source, local_id, sheet_marker, last_synced_snapshot, last_synced_at)
             VALUES ('pulls', 1, 'PULL-000002', '{}', strftime('%Y-%m-%dT%H:%M:%fZ','now'))",
            [],
        );
        assert!(dup.is_err(), "a local record must never be linked to two different sheet rows at once");
    }
}

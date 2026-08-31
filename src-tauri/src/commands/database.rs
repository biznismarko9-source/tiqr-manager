use crate::db::AppState;
use crate::error::AppResult;
use serde::Serialize;
use std::path::PathBuf;
use tauri::State;

/// 2.0.72: powers per-account data isolation - marko's own follow-up request
/// after noticing two different signed-in accounts on the same computer saw
/// the exact same tickets/orders/etc. Rather than adding an `owner_uid`
/// column and a `WHERE owner_uid = ?` to every one of this app's ~100
/// existing queries (invasive, and the one thing that could accidentally
/// leak one account's rows into another's if a single query got it wrong),
/// this swaps the LIVE CONNECTION to point at a completely different whole
/// SQLite file - one per signed-in Firebase account. Every existing query in
/// this codebase keeps working completely unchanged; it has no idea whose
/// data it's looking at, because from its point of view there has only ever
/// been one file.
///
/// See db.rs's `resolve_db_path` (the one original/legacy file - kept
/// exactly as-is forever, for any account that already existed before this
/// feature shipped) and `resolve_user_db_path` (a brand-new, empty file per
/// account approved from now on). Which of the two applies to a given
/// account (`legacy` below) is decided ONCE, on the frontend, by
/// src/lib/auth.tsx's `isGrandfatheredAccount` - Rust trusts that boolean
/// rather than re-deriving it from Firebase account-creation timestamps
/// itself. That's a deliberate, acceptable trust boundary here specifically
/// because this is a single-user local desktop app - the frontend and this
/// backend run as the same person, on the same machine, not across a
/// multi-tenant server boundary. The worst case of a hypothetical frontend
/// bug here is data landing in the wrong file (a data-hygiene mistake to fix
/// once found), never a security breach.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseSwitchOutcome {
    pub db_path: String,
    /// True the first time this exact file is ever switched to (it didn't
    /// exist yet, so it was just created from scratch and fully migrated) -
    /// lets the frontend know for certain it's showing a brand-new, empty
    /// workspace rather than inferring that from an empty ticket list, which
    /// would also be (mis)read as "the switch silently failed."
    pub is_new: bool,
}

/// Does the actual swap. Takes a plain `&AppState` (not `State<AppState>`)
/// and a resolved `PathBuf`, the same "impl fn takes plain values, command
/// wrapper resolves Tauri-specific types" split every other command in this
/// codebase already uses - see e.g. `commands::backup::restore_database_impl`.
pub(crate) fn switch_active_database_impl(state: &AppState, target_path: PathBuf) -> AppResult<DatabaseSwitchOutcome> {
    // Lock ordering per AppState's own doc comment: `db` first, then
    // `db_path`, both held for this entire function - never released and
    // separately re-acquired. That's what guarantees no other command can
    // ever observe the new connection paired with the old path, or vice
    // versa, mid-switch.
    let mut conn_guard = state.db.lock().unwrap();
    let mut path_guard = state.db_path.lock().unwrap();

    if *path_guard == target_path {
        // Already the active file - the common case on every ordinary
        // launch (marko's own legacy account signing back in). A no-op:
        // opening a second connection to a file the live one already has
        // open and fully migrated would just be wasted work, and would
        // needlessly drop/reopen a perfectly good connection.
        return Ok(DatabaseSwitchOutcome { db_path: target_path.display().to_string(), is_new: false });
    }

    let is_new = !target_path.exists();
    let new_conn = crate::db::open_connection(&target_path)?;
    crate::db::run_migrations(&new_conn)?;
    *conn_guard = new_conn; // old Connection drops here - WAL flushed, same as a normal app quit
    *path_guard = target_path.clone();
    Ok(DatabaseSwitchOutcome { db_path: target_path.display().to_string(), is_new })
}

/// Called exactly once by the frontend right after Firebase confirms who's
/// signed in AND that they're approved (src/lib/auth.tsx's
/// `switchDatabaseFor`) - never before, since there's nothing meaningful to
/// switch to for a not-yet-approved account (App.tsx's `RequireAuth` never
/// even reaches a data-consuming page for one).
#[tauri::command]
pub fn switch_active_database(
    app: tauri::AppHandle,
    state: State<AppState>,
    uid: String,
    legacy: bool,
) -> AppResult<DatabaseSwitchOutcome> {
    let target_path = if legacy {
        crate::db::resolve_db_path(&app)?
    } else {
        crate::db::resolve_user_db_path(&app, &uid)?
    };
    switch_active_database_impl(&state, target_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{open_connection, run_migrations};
    use rusqlite::Connection;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Mutex;

    fn unique_temp_path(label: &str) -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("tiqr_test_db_switch_{}_{}", std::process::id(), n));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(format!("{label}.sqlite3"))
    }

    fn state_at(path: &std::path::Path) -> AppState {
        let conn = open_connection(path).unwrap();
        run_migrations(&conn).unwrap();
        AppState {
            db: Mutex::new(conn),
            db_path: Mutex::new(path.to_path_buf()),
            oauth_cancel_flag: Mutex::new(None),
            firebase_oauth_cancel_flag: Mutex::new(None),
            price_checker_auto_cancel_flag: Mutex::new(None),
        }
    }

    fn insert_event(conn: &Connection, name: &str) {
        conn.execute(
            "INSERT INTO events (name, event_date, status) VALUES (?1, '2026-01-01', 'upcoming')",
            [name],
        )
        .unwrap();
    }

    fn event_names(conn: &Connection) -> Vec<String> {
        let mut stmt = conn.prepare("SELECT name FROM events ORDER BY name").unwrap();
        stmt.query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    }

    #[test]
    fn switching_to_the_already_active_path_is_a_no_op() {
        let path_a = unique_temp_path("same_a");
        let state = state_at(&path_a);
        insert_event(&state.db.lock().unwrap(), "Original Event");

        let outcome = switch_active_database_impl(&state, path_a.clone()).unwrap();
        assert!(!outcome.is_new, "switching to the path marko is already on must never report is_new");
        assert_eq!(event_names(&state.db.lock().unwrap()), vec!["Original Event".to_string()]);
    }

    #[test]
    fn data_survives_a_round_trip_from_a_to_b_and_back_to_a() {
        let path_a = unique_temp_path("roundtrip_a");
        let path_b = unique_temp_path("roundtrip_b");
        let state = state_at(&path_a);
        insert_event(&state.db.lock().unwrap(), "A's Event");

        switch_active_database_impl(&state, path_b.clone()).unwrap();
        assert!(
            event_names(&state.db.lock().unwrap()).is_empty(),
            "B must start completely empty - it must never see A's data"
        );
        insert_event(&state.db.lock().unwrap(), "B's Event");

        switch_active_database_impl(&state, path_a.clone()).unwrap();
        assert_eq!(
            event_names(&state.db.lock().unwrap()),
            vec!["A's Event".to_string()],
            "A's own data must still be there, and B's event must never leak into it"
        );
    }

    #[test]
    fn a_brand_new_path_ends_up_fully_migrated() {
        let path_a = unique_temp_path("fresh_a");
        let state = state_at(&path_a);
        let path_new = unique_temp_path("fresh_new");

        let outcome = switch_active_database_impl(&state, path_new).unwrap();
        assert!(outcome.is_new, "a path that never existed before must report is_new");

        let conn = state.db.lock().unwrap();
        let migration_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |r| r.get(0))
            .unwrap();
        assert_eq!(migration_count, 17, "a fresh per-account file must end up on the same schema version as every other file");
        assert!(event_names(&conn).is_empty());
    }

    #[test]
    fn is_new_is_true_only_the_first_time_a_path_is_used() {
        let path_a = unique_temp_path("isnew_a");
        let state = state_at(&path_a);
        let path_b = unique_temp_path("isnew_b");

        let first = switch_active_database_impl(&state, path_b.clone()).unwrap();
        assert!(first.is_new);

        switch_active_database_impl(&state, path_a.clone()).unwrap();
        let second = switch_active_database_impl(&state, path_b.clone()).unwrap();
        assert!(!second.is_new, "the second time this same path is switched to, it already exists");
    }
}

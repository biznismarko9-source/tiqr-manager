use crate::db::AppState;
use crate::error::{AppError, AppResult};
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

/// Restores the live database from a chosen backup file. The frontend is
/// responsible for relaunching the app right after this succeeds (via the
/// process plugin's `relaunch()`), so every screen reloads with fresh data
/// instead of risking stale in-memory state.
#[tauri::command]
pub fn restore_database(state: State<AppState>, src_path: String) -> AppResult<()> {
    {
        let src = rusqlite::Connection::open(&src_path)
            .map_err(|_| AppError::Validation("Selected file is not a valid SQLite database".into()))?;
        let looks_valid: bool = src
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='schema_migrations')",
                [],
                |r| r.get(0),
            )
            .unwrap_or(false);
        if !looks_valid {
            return Err(AppError::Validation(
                "Selected file does not look like a TIQR Manager backup".into(),
            ));
        }
    }

    let mut conn = state.db.lock().unwrap();
    let src = rusqlite::Connection::open(&src_path)?;
    {
        let backup = rusqlite::backup::Backup::new(&src, &mut conn)?;
        backup.run_to_completion(5, std::time::Duration::from_millis(250), None)?;
    }
    crate::db::run_migrations(&conn)?;
    Ok(())
}

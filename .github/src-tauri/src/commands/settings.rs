use crate::db::AppState;
use crate::error::AppResult;
use rusqlite::params;
use tauri::State;

/// Generic key/value storage backed by the `app_settings` table, used for
/// small UI preferences (e.g. theme) that don't warrant their own column or
/// command. Not for business data - everything else in this app has a
/// proper typed table and command.

#[tauri::command]
pub fn get_app_setting(state: State<AppState>, key: String) -> AppResult<Option<String>> {
    let conn = state.db.lock().unwrap();
    let value = conn
        .query_row(
            "SELECT value FROM app_settings WHERE key = ?1",
            params![key],
            |r| r.get(0),
        )
        .ok();
    Ok(value)
}

#[tauri::command]
pub fn set_app_setting(state: State<AppState>, key: String, value: String) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    conn.execute(
        "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

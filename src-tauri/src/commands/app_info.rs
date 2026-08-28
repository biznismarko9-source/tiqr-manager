use crate::db::AppState;
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub db_path: String,
}

// 2.0.72: `db_path` now reads `state.db_path` (whichever file is currently
// active for the signed-in account) instead of always calling
// `resolve_db_path` directly - that always returned the one legacy file
// regardless of who was signed in, a pre-existing inaccuracy that only
// became visible once more than one file could ever be active. `app` is
// still needed for the version string.
#[tauri::command]
pub fn get_app_info(app: tauri::AppHandle, state: State<AppState>) -> AppInfo {
    let version = app.package_info().version.to_string();
    let db_path = state.db_path.lock().unwrap().display().to_string();
    AppInfo { version, db_path }
}

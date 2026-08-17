use crate::db::AppState;
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub db_path: String,
}

#[tauri::command]
pub fn get_app_info(app: tauri::AppHandle, _state: State<AppState>) -> AppInfo {
    let version = app.package_info().version.to_string();
    let db_path = crate::db::resolve_db_path(&app)
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    AppInfo { version, db_path }
}

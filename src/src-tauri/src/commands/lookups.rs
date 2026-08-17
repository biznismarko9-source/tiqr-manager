use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::models::{Platform, Supplier};
use rusqlite::Row;
use tauri::State;

fn map_platform(row: &Row) -> rusqlite::Result<Platform> {
    Ok(Platform {
        id: row.get("id")?,
        name: row.get("name")?,
        kind: row.get("kind")?,
        is_demo: row.get("is_demo")?,
        created_at: row.get("created_at")?,
    })
}

fn map_supplier(row: &Row) -> rusqlite::Result<Supplier> {
    Ok(Supplier {
        id: row.get("id")?,
        name: row.get("name")?,
        contact: row.get("contact")?,
        is_demo: row.get("is_demo")?,
        created_at: row.get("created_at")?,
    })
}

#[tauri::command]
pub fn list_platforms(state: State<AppState>) -> AppResult<Vec<Platform>> {
    let conn = state.db.lock().unwrap();
    let mut stmt = conn.prepare("SELECT * FROM platforms ORDER BY name COLLATE NOCASE")?;
    let rows = stmt.query_map([], map_platform)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[tauri::command]
pub fn create_platform(
    state: State<AppState>,
    name: String,
    kind: Option<String>,
) -> AppResult<Platform> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::Validation("Platform name cannot be empty".into()));
    }
    let conn = state.db.lock().unwrap();
    conn.execute(
        "INSERT INTO platforms(name, kind) VALUES (?1, ?2)",
        rusqlite::params![name, kind.unwrap_or_else(|| "both".to_string())],
    )
    .map_err(|e| match &e {
        rusqlite::Error::SqliteFailure(_, Some(m)) if m.contains("UNIQUE") => {
            AppError::Validation(format!("Platform '{name}' already exists"))
        }
        _ => AppError::from(e),
    })?;
    let id = conn.last_insert_rowid();
    Ok(conn.query_row("SELECT * FROM platforms WHERE id = ?1", [id], map_platform)?)
}

#[tauri::command]
pub fn delete_platform(state: State<AppState>, id: i64) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    conn.execute("DELETE FROM platforms WHERE id = ?1", [id])?;
    Ok(())
}

#[tauri::command]
pub fn list_suppliers(state: State<AppState>) -> AppResult<Vec<Supplier>> {
    let conn = state.db.lock().unwrap();
    let mut stmt = conn.prepare("SELECT * FROM suppliers ORDER BY name COLLATE NOCASE")?;
    let rows = stmt.query_map([], map_supplier)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[tauri::command]
pub fn create_supplier(
    state: State<AppState>,
    name: String,
    contact: Option<String>,
) -> AppResult<Supplier> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::Validation("Supplier name cannot be empty".into()));
    }
    let conn = state.db.lock().unwrap();
    conn.execute(
        "INSERT INTO suppliers(name, contact) VALUES (?1, ?2)",
        rusqlite::params![name, contact],
    )
    .map_err(|e| match &e {
        rusqlite::Error::SqliteFailure(_, Some(m)) if m.contains("UNIQUE") => {
            AppError::Validation(format!("Supplier '{name}' already exists"))
        }
        _ => AppError::from(e),
    })?;
    let id = conn.last_insert_rowid();
    Ok(conn.query_row("SELECT * FROM suppliers WHERE id = ?1", [id], map_supplier)?)
}

#[tauri::command]
pub fn delete_supplier(state: State<AppState>, id: i64) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    conn.execute("DELETE FROM suppliers WHERE id = ?1", [id])?;
    Ok(())
}

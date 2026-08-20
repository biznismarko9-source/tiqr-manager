use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::models::{Platform, Supplier};
use rusqlite::{Connection, Row};
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

/// Core logic behind `update_platform_kind` - split out for direct
/// unit-testability against a plain `&Connection` (same "impl function + thin
/// #[tauri::command] wrapper" pattern the other command modules use). Lets
/// Settings re-tag an existing platform between Purchase/Sale/Both.
///
/// 1.9.3: marko wanted separate "where you bought it" vs "where you sold it"
/// platform lists instead of one shared pool. The `kind` column has existed
/// on `platforms` (CHECK'd to exactly these 3 values) since the very first
/// migration - it just wasn't exposed anywhere in the UI yet, and
/// `list_platforms`/`create_platform` already read/write it. So this is the
/// only genuinely new piece: a way to change an *existing* platform's kind
/// after the fact. No migration, no new table - see the 1.9.3 report for why
/// the originally-discussed small migration turned out not to be necessary.
pub(crate) fn update_platform_kind_impl(conn: &Connection, id: i64, kind: &str) -> AppResult<Platform> {
    if !["purchase", "sale", "both"].contains(&kind) {
        return Err(AppError::Validation(format!(
            "Invalid platform kind '{kind}' - must be 'purchase', 'sale' or 'both'"
        )));
    }
    let updated = conn.execute(
        "UPDATE platforms SET kind = ?1 WHERE id = ?2",
        rusqlite::params![kind, id],
    )?;
    if updated == 0 {
        return Err(AppError::NotFound(format!("Platform #{id} not found")));
    }
    Ok(conn.query_row("SELECT * FROM platforms WHERE id = ?1", [id], map_platform)?)
}

#[tauri::command]
pub fn update_platform_kind(state: State<AppState>, id: i64, kind: String) -> AppResult<Platform> {
    let conn = state.db.lock().unwrap();
    update_platform_kind_impl(&conn, id, &kind)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_conn;

    fn seed_platform(conn: &Connection, name: &str, kind: &str) -> i64 {
        conn.execute(
            "INSERT INTO platforms(name, kind) VALUES (?1, ?2)",
            rusqlite::params![name, kind],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    fn platform_kind(conn: &Connection, id: i64) -> String {
        conn.query_row("SELECT kind FROM platforms WHERE id=?1", [id], |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn update_platform_kind_changes_an_existing_platform() {
        let conn = test_conn();
        let id = seed_platform(&conn, "StubHub", "both");
        let updated = update_platform_kind_impl(&conn, id, "sale").unwrap();
        assert_eq!(updated.kind, "sale");
        assert_eq!(platform_kind(&conn, id), "sale");
    }

    #[test]
    fn update_platform_kind_rejects_an_invalid_kind() {
        let conn = test_conn();
        let id = seed_platform(&conn, "StubHub", "both");
        assert!(update_platform_kind_impl(&conn, id, "nonsense").is_err());
        assert_eq!(platform_kind(&conn, id), "both", "a rejected update must change nothing");
    }

    #[test]
    fn update_platform_kind_rejects_a_missing_platform() {
        let conn = test_conn();
        assert!(update_platform_kind_impl(&conn, 999_999, "purchase").is_err());
    }

    #[test]
    fn update_platform_kind_allows_every_valid_value() {
        let conn = test_conn();
        let id = seed_platform(&conn, "Viagogo", "both");
        for kind in ["purchase", "sale", "both"] {
            update_platform_kind_impl(&conn, id, kind).unwrap();
            assert_eq!(platform_kind(&conn, id), kind);
        }
    }
}

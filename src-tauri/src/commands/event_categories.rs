use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::models::EventCategory;
use rusqlite::{Connection, Row};
use tauri::State;

fn map_event_category(row: &Row) -> rusqlite::Result<EventCategory> {
    Ok(EventCategory {
        id: row.get("id")?,
        name: row.get("name")?,
        color_slot: row.get("color_slot")?,
        is_demo: row.get("is_demo")?,
        created_at: row.get("created_at")?,
    })
}

/// 2.0.27: managed event categories (Settings -> Lookups, "like Platforms" -
/// marko's own answer when asked how these should be managed). Same CRUD
/// shape as `lookups::list_platforms`/`create_platform`/`delete_platform`,
/// with two differences: `create` also assigns the next free `color_slot`
/// (see `create_event_category_impl`'s doc comment), and `delete` clears the
/// old free-text `events.category` mirror too, not just `category_id` (see
/// `delete_event_category_impl`'s doc comment) - which is why both are split
/// into impl+wrapper (unlike the platform/supplier commands, which have no
/// logic beyond a straight insert/delete worth unit-testing on its own).
#[tauri::command]
pub fn list_event_categories(state: State<AppState>) -> AppResult<Vec<EventCategory>> {
    let conn = state.db.lock().unwrap();
    let mut stmt = conn.prepare("SELECT * FROM event_categories ORDER BY name COLLATE NOCASE")?;
    let rows = stmt.query_map([], map_event_category)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Core logic behind `create_event_category` - split out for direct
/// unit-testability (same "impl function + thin #[tauri::command] wrapper"
/// pattern used throughout this codebase).
///
/// `color_slot` is assigned here, not left to the frontend, so it's always
/// exactly "one past whatever the highest slot currently in use is" - a
/// single source of truth that can't drift even if two categories are
/// created close together (this app's single mutex-guarded connection - see
/// AppState - means this read-then-insert is never actually racy). See
/// migrations/012_event_categories.sql's doc comment for why a plain integer
/// index (not a hex string) is stored at all, and EventCategoryBadge.tsx for
/// how the frontend turns a slot into an actual color, wrapping past the
/// palette's own length rather than failing once more than 8 categories exist.
pub(crate) fn create_event_category_impl(conn: &Connection, name: &str) -> AppResult<EventCategory> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::Validation("Category name cannot be empty".into()));
    }
    let next_slot: i64 = conn.query_row(
        "SELECT COALESCE(MAX(color_slot), -1) + 1 FROM event_categories",
        [],
        |r| r.get(0),
    )?;
    conn.execute(
        "INSERT INTO event_categories(name, color_slot) VALUES (?1, ?2)",
        rusqlite::params![name, next_slot],
    )
    .map_err(|e| match &e {
        rusqlite::Error::SqliteFailure(_, Some(m)) if m.contains("UNIQUE") => {
            AppError::Validation(format!("Category '{name}' already exists"))
        }
        _ => AppError::from(e),
    })?;
    let id = conn.last_insert_rowid();
    Ok(conn.query_row(
        "SELECT * FROM event_categories WHERE id = ?1",
        [id],
        map_event_category,
    )?)
}

#[tauri::command]
pub fn create_event_category(state: State<AppState>, name: String) -> AppResult<EventCategory> {
    let conn = state.db.lock().unwrap();
    create_event_category_impl(&conn, &name)
}

/// Core logic behind `delete_event_category`. Unlike `delete_platform`/
/// `delete_supplier` (a blind DELETE that leans entirely on the FK's own ON
/// DELETE SET NULL), this runs in its own transaction that ALSO clears the
/// legacy free-text `events.category` mirror for every affected event -
/// migrations/012_event_categories.sql's `category_id` FK only ever touches
/// `category_id` itself on delete, so without this, `category` would keep
/// showing the deleted category's name forever (e.g. in a CSV export) even
/// though the app itself would show "no category". Both columns are cleared
/// together or not at all.
pub(crate) fn delete_event_category_impl(conn: &mut Connection, id: i64) -> AppResult<()> {
    let tx = conn.transaction()?;
    tx.execute(
        "UPDATE events SET category_id = NULL, category = NULL WHERE category_id = ?1",
        [id],
    )?;
    tx.execute("DELETE FROM event_categories WHERE id = ?1", [id])?;
    tx.commit()?;
    Ok(())
}

#[tauri::command]
pub fn delete_event_category(state: State<AppState>, id: i64) -> AppResult<()> {
    let mut conn = state.db.lock().unwrap();
    delete_event_category_impl(&mut conn, id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_conn;

    // migrations/012_event_categories.sql seeds 6 categories (slots 0-5) in
    // every freshly-migrated connection, including test_conn()'s - tests
    // below account for that rather than assuming a blank table.

    fn seed_event_with_category(conn: &Connection, category_id: Option<i64>, category_text: Option<&str>) -> i64 {
        conn.execute(
            "INSERT INTO events (name, category, category_id) VALUES ('Test Event', ?1, ?2)",
            rusqlite::params![category_text, category_id],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    fn event_category_fields(conn: &Connection, event_id: i64) -> (Option<i64>, Option<String>) {
        conn.query_row(
            "SELECT category_id, category FROM events WHERE id = ?1",
            [event_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap()
    }

    #[test]
    fn seed_migration_creates_the_six_expected_categories() {
        let conn = test_conn();
        let mut stmt = conn.prepare("SELECT name, color_slot FROM event_categories ORDER BY color_slot").unwrap();
        let rows: Vec<(String, i64)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            rows,
            vec![
                ("Concert".to_string(), 0),
                ("Sports".to_string(), 1),
                ("Theatre / Musical".to_string(), 2),
                ("Festival".to_string(), 3),
                ("Comedy".to_string(), 4),
                ("Motorsport".to_string(), 5),
            ]
        );
    }

    #[test]
    fn create_event_category_assigns_the_next_free_color_slot() {
        let conn = test_conn();
        let created = create_event_category_impl(&conn, "Wrestling").unwrap();
        assert_eq!(created.name, "Wrestling");
        assert_eq!(created.color_slot, 6, "6 seeded categories already occupy slots 0-5");

        let second = create_event_category_impl(&conn, "Opera").unwrap();
        assert_eq!(second.color_slot, 7);
    }

    #[test]
    fn create_event_category_rejects_empty_name() {
        let conn = test_conn();
        let err = create_event_category_impl(&conn, "   ").unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn create_event_category_rejects_a_duplicate_name() {
        let conn = test_conn();
        create_event_category_impl(&conn, "Wrestling").unwrap();
        let err = create_event_category_impl(&conn, "Wrestling").unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn delete_event_category_clears_category_id_and_text_on_affected_events_only() {
        let mut conn = test_conn();
        let concert_id: i64 = conn
            .query_row("SELECT id FROM event_categories WHERE name = 'Concert'", [], |r| r.get(0))
            .unwrap();
        let sports_id: i64 = conn
            .query_row("SELECT id FROM event_categories WHERE name = 'Sports'", [], |r| r.get(0))
            .unwrap();

        let concert_event = seed_event_with_category(&conn, Some(concert_id), Some("Concert"));
        let sports_event = seed_event_with_category(&conn, Some(sports_id), Some("Sports"));

        delete_event_category_impl(&mut conn, concert_id).unwrap();

        assert_eq!(event_category_fields(&conn, concert_event), (None, None), "deleted category must clear both columns");
        assert_eq!(
            event_category_fields(&conn, sports_event),
            (Some(sports_id), Some("Sports".to_string())),
            "an event using a DIFFERENT category must be untouched"
        );

        // The category row itself is gone.
        let still_there: i64 = conn
            .query_row("SELECT COUNT(*) FROM event_categories WHERE id = ?1", [concert_id], |r| r.get(0))
            .unwrap();
        assert_eq!(still_there, 0);
    }
}

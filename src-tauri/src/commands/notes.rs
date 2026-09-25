//! Notes - sheets whose columns marko names himself.
//!
//! 2.51.0. marko: "chcem urobit miesto kde si viem zapisovat vsetky dolezite
//! info, urobit nieco podobne v apke ako su google sheets a tam si vediet
//! vpisat dolezite veci, datumy, atd". Offered three shapes, he picked sheets
//! with his own columns (over free-text pages) and standalone (over notes
//! attached to an event or order).
//!
//! ## Cells are JSON, and columns change one at a time
//!
//! A sheet holds `columns_json` (an array of names) and each row holds
//! `cells_json` (an array of strings, positionally aligned with it). See
//! migration 031 for why that beats a third table here.
//!
//! The alignment is the one invariant worth defending, so columns are never
//! changed by handing this module a whole new list and asking it to work out
//! what moved. There are exactly three column operations - add, rename,
//! delete - each of which knows precisely what it does to every row, and the
//! two that touch rows do it inside one transaction. A row whose cells are
//! shorter than the columns (an older row, a merge from the other machine) is
//! padded on read rather than rejected: a notepad must never refuse to show
//! what is in it.

use crate::commands::cloud_sync::now_iso;
use crate::db::AppState;
use crate::error::{AppError, AppResult};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NoteSheet {
    pub id: i64,
    pub name: String,
    pub columns: Vec<String>,
    pub position: i64,
    pub row_count: i64,
    pub updated_at: String,
    /// 2.52.0 (Workspace): a table is an item like any other, so it carries
    /// the same handles - what it is for, and whether it is pinned or put
    /// away. Column TYPES exist as a column in migration 032 but are not read
    /// yet; every cell is still free text.
    pub description: String,
    pub pinned: bool,
    pub archived: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NoteRow {
    pub id: i64,
    pub sheet_id: i64,
    pub position: i64,
    pub cells: Vec<String>,
    pub updated_at: String,
}

/// Stored JSON to a real list. Anything unreadable becomes an empty list
/// rather than an error: this is a notepad, and refusing to open a sheet
/// because one cell blob is malformed would lose more than it protects.
fn parse_list(raw: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(raw).unwrap_or_default()
}

fn dump_list(list: &[String]) -> String {
    serde_json::to_string(list).unwrap_or_else(|_| "[]".to_string())
}

/// Every row is shown with exactly as many cells as the sheet has columns -
/// padded when short, trimmed when long. Both happen legitimately: a column
/// added while the other machine was offline, or a row merged in from it.
fn fit(cells: Vec<String>, width: usize) -> Vec<String> {
    let mut c = cells;
    c.truncate(width);
    while c.len() < width {
        c.push(String::new());
    }
    c
}

fn sheet_columns(conn: &Connection, sheet_id: i64) -> AppResult<Vec<String>> {
    let raw: Option<String> = conn
        .query_row("SELECT columns_json FROM note_sheets WHERE id = ?1", [sheet_id], |r| r.get(0))
        .optional()?;
    match raw {
        Some(j) => Ok(parse_list(&j)),
        None => Err(AppError::NotFound(format!("Note sheet {sheet_id} no longer exists."))),
    }
}

fn touch_sheet(conn: &Connection, sheet_id: i64) -> AppResult<()> {
    conn.execute("UPDATE note_sheets SET updated_at = ?2 WHERE id = ?1", rusqlite::params![sheet_id, now_iso()])?;
    Ok(())
}

fn read_sheet(conn: &Connection, id: i64) -> AppResult<NoteSheet> {
    let row = conn
        .query_row(
            "SELECT s.id, s.name, s.columns_json, s.position, s.updated_at,
                    (SELECT COUNT(*) FROM note_rows r WHERE r.sheet_id = s.id),
                    s.description, s.pinned, s.archived
             FROM note_sheets s WHERE s.id = ?1",
            [id],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, i64>(5)?,
                    r.get::<_, String>(6)?,
                    r.get::<_, i64>(7)?,
                    r.get::<_, i64>(8)?,
                ))
            },
        )
        .optional()?;
    match row {
        Some((id, name, columns_json, position, updated_at, row_count, description, pinned, archived)) => Ok(NoteSheet {
            id,
            name,
            columns: parse_list(&columns_json),
            position,
            row_count,
            updated_at,
            description,
            pinned: pinned != 0,
            archived: archived != 0,
        }),
        None => Err(AppError::NotFound(format!("Note sheet {id} no longer exists."))),
    }
}

#[tauri::command]
pub fn list_note_sheets(state: State<AppState>) -> AppResult<Vec<NoteSheet>> {
    let conn = state.db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT s.id, s.name, s.columns_json, s.position, s.updated_at,
                (SELECT COUNT(*) FROM note_rows r WHERE r.sheet_id = s.id),
                s.description, s.pinned, s.archived
         FROM note_sheets s ORDER BY s.pinned DESC, s.position, s.id",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(NoteSheet {
            id: r.get(0)?,
            name: r.get(1)?,
            columns: parse_list(&r.get::<_, String>(2)?),
            position: r.get(3)?,
            row_count: r.get(5)?,
            updated_at: r.get(4)?,
            description: r.get(6)?,
            pinned: r.get::<_, i64>(7)? != 0,
            archived: r.get::<_, i64>(8)? != 0,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[tauri::command]
pub fn create_note_sheet(state: State<AppState>, name: String, columns: Vec<String>) -> AppResult<NoteSheet> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::Validation("A sheet needs a name.".to_string()));
    }
    // A sheet with no columns cannot be typed into, so one is provided rather
    // than leaving marko with a grid he has to configure before it is useful.
    let columns: Vec<String> = {
        let cleaned: Vec<String> = columns.iter().map(|c| c.trim().to_string()).filter(|c| !c.is_empty()).collect();
        if cleaned.is_empty() {
            vec!["Note".to_string()]
        } else {
            cleaned
        }
    };
    let conn = state.db.lock().unwrap();
    let next_position: i64 = conn.query_row("SELECT COALESCE(MAX(position), -1) + 1 FROM note_sheets", [], |r| r.get(0))?;
    conn.execute(
        "INSERT INTO note_sheets(name, columns_json, position, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)",
        rusqlite::params![name, dump_list(&columns), next_position, now_iso()],
    )?;
    read_sheet(&conn, conn.last_insert_rowid())
}

#[tauri::command]
pub fn rename_note_sheet(state: State<AppState>, id: i64, name: String) -> AppResult<NoteSheet> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::Validation("A sheet needs a name.".to_string()));
    }
    let conn = state.db.lock().unwrap();
    let changed = conn.execute(
        "UPDATE note_sheets SET name = ?2, updated_at = ?3 WHERE id = ?1",
        rusqlite::params![id, name, now_iso()],
    )?;
    if changed == 0 {
        return Err(AppError::NotFound(format!("Note sheet {id} no longer exists.")));
    }
    read_sheet(&conn, id)
}

/// Pin / put away / describe a table, without opening it. Each field is
/// `Option`: absent means leave it as it is, so one command serves all three.
#[tauri::command]
pub fn set_note_sheet_flags(
    state: State<AppState>,
    id: i64,
    pinned: Option<bool>,
    archived: Option<bool>,
    description: Option<String>,
) -> AppResult<NoteSheet> {
    let conn = state.db.lock().unwrap();
    let current = read_sheet(&conn, id)?;
    conn.execute(
        "UPDATE note_sheets SET pinned=?2, archived=?3, description=?4, updated_at=?5 WHERE id=?1",
        rusqlite::params![
            id,
            i64::from(pinned.unwrap_or(current.pinned)),
            i64::from(archived.unwrap_or(current.archived)),
            description.unwrap_or(current.description),
            now_iso()
        ],
    )?;
    read_sheet(&conn, id)
}

#[tauri::command]
pub fn delete_note_sheet(state: State<AppState>, id: i64) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    // The rows go with it through ON DELETE CASCADE, and each one leaves its
    // own tombstone (migration 031) so the other machine deletes them too.
    let changed = conn.execute("DELETE FROM note_sheets WHERE id = ?1", [id])?;
    if changed == 0 {
        return Err(AppError::NotFound(format!("Note sheet {id} no longer exists.")));
    }
    Ok(())
}

#[tauri::command]
pub fn add_note_column(state: State<AppState>, sheet_id: i64, name: String) -> AppResult<NoteSheet> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::Validation("A column needs a name.".to_string()));
    }
    let mut conn = state.db.lock().unwrap();
    let tx = conn.transaction()?;
    let mut columns = sheet_columns(&tx, sheet_id)?;
    columns.push(name);
    tx.execute(
        "UPDATE note_sheets SET columns_json = ?2, updated_at = ?3 WHERE id = ?1",
        rusqlite::params![sheet_id, dump_list(&columns), now_iso()],
    )?;
    // Every row grows by exactly one empty cell, in the same transaction, so
    // cells and columns can never be seen out of step.
    let width = columns.len();
    reshape_rows(&tx, sheet_id, |cells| fit(cells, width))?;
    tx.commit()?;
    read_sheet(&conn, sheet_id)
}

#[tauri::command]
pub fn rename_note_column(state: State<AppState>, sheet_id: i64, index: usize, name: String) -> AppResult<NoteSheet> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::Validation("A column needs a name.".to_string()));
    }
    let conn = state.db.lock().unwrap();
    let mut columns = sheet_columns(&conn, sheet_id)?;
    if index >= columns.len() {
        return Err(AppError::Validation("That column no longer exists.".to_string()));
    }
    // Renaming touches no cell at all - the alignment is positional.
    columns[index] = name;
    conn.execute(
        "UPDATE note_sheets SET columns_json = ?2, updated_at = ?3 WHERE id = ?1",
        rusqlite::params![sheet_id, dump_list(&columns), now_iso()],
    )?;
    read_sheet(&conn, sheet_id)
}

#[tauri::command]
pub fn delete_note_column(state: State<AppState>, sheet_id: i64, index: usize) -> AppResult<NoteSheet> {
    let mut conn = state.db.lock().unwrap();
    let tx = conn.transaction()?;
    let mut columns = sheet_columns(&tx, sheet_id)?;
    if index >= columns.len() {
        return Err(AppError::Validation("That column no longer exists.".to_string()));
    }
    if columns.len() == 1 {
        return Err(AppError::Validation("A sheet needs at least one column.".to_string()));
    }
    columns.remove(index);
    tx.execute(
        "UPDATE note_sheets SET columns_json = ?2, updated_at = ?3 WHERE id = ?1",
        rusqlite::params![sheet_id, dump_list(&columns), now_iso()],
    )?;
    // The same index comes out of every row, so what is left still lines up
    // with the columns that remain.
    reshape_rows(&tx, sheet_id, |mut cells| {
        if index < cells.len() {
            cells.remove(index);
        }
        cells
    })?;
    tx.commit()?;
    read_sheet(&conn, sheet_id)
}

/// Moves one column to another position, taking that column's cell with it in
/// every row.
///
/// 2.53.0. Like the other three column operations this one knows exactly what
/// it does to every row rather than diffing an old list against a new one -
/// see `PROTECTED_AREAS.md`, "a note row's cells are POSITIONAL". A row is
/// fitted to the column count first, so a short row from a merge cannot make
/// the move land on the wrong index.
#[tauri::command]
pub fn reorder_note_column(
    state: State<AppState>,
    sheet_id: i64,
    from_index: usize,
    to_index: usize,
) -> AppResult<NoteSheet> {
    let mut conn = state.db.lock().unwrap();
    {
        let width = sheet_columns(&conn, sheet_id)?.len();
        if from_index >= width || to_index >= width {
            return Err(AppError::Validation("That column no longer exists.".to_string()));
        }
        if from_index == to_index {
            return read_sheet(&conn, sheet_id);
        }
    }
    let tx = conn.transaction()?;
    let mut columns = sheet_columns(&tx, sheet_id)?;
    let width = columns.len();
    if from_index >= width || to_index >= width {
        return Err(AppError::Validation("That column no longer exists.".to_string()));
    }
    let moved = columns.remove(from_index);
    columns.insert(to_index, moved);
    tx.execute(
        "UPDATE note_sheets SET columns_json = ?2, updated_at = ?3 WHERE id = ?1",
        rusqlite::params![sheet_id, dump_list(&columns), now_iso()],
    )?;
    reshape_rows(&tx, sheet_id, |cells| {
        let mut cells = fit(cells, width);
        let moved = cells.remove(from_index);
        cells.insert(to_index, moved);
        cells
    })?;
    tx.commit()?;
    read_sheet(&conn, sheet_id)
}

/// Rewrites every row of one sheet through `f`, inside the caller's
/// transaction. Read fully before writing: rewriting while iterating a live
/// statement on the same table is the classic way to half-apply a change.
fn reshape_rows<F>(conn: &Connection, sheet_id: i64, f: F) -> AppResult<()>
where
    F: Fn(Vec<String>) -> Vec<String>,
{
    let existing: Vec<(i64, String)> = {
        let mut stmt = conn.prepare("SELECT id, cells_json FROM note_rows WHERE sheet_id = ?1")?;
        let rows = stmt.query_map([sheet_id], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    for (id, raw) in existing {
        let next = f(parse_list(&raw));
        conn.execute(
            "UPDATE note_rows SET cells_json = ?2, updated_at = ?3 WHERE id = ?1",
            rusqlite::params![id, dump_list(&next), now_iso()],
        )?;
    }
    Ok(())
}

#[tauri::command]
pub fn list_note_rows(state: State<AppState>, sheet_id: i64) -> AppResult<Vec<NoteRow>> {
    let conn = state.db.lock().unwrap();
    let width = sheet_columns(&conn, sheet_id)?.len();
    let mut stmt =
        conn.prepare("SELECT id, sheet_id, position, cells_json, updated_at FROM note_rows WHERE sheet_id = ?1 ORDER BY position, id")?;
    let rows = stmt.query_map([sheet_id], |r| {
        Ok(NoteRow {
            id: r.get(0)?,
            sheet_id: r.get(1)?,
            position: r.get(2)?,
            cells: fit(parse_list(&r.get::<_, String>(3)?), width),
            updated_at: r.get(4)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[tauri::command]
pub fn create_note_row(state: State<AppState>, sheet_id: i64, cells: Vec<String>) -> AppResult<NoteRow> {
    let conn = state.db.lock().unwrap();
    let width = sheet_columns(&conn, sheet_id)?.len();
    let cells = fit(cells, width);
    let next_position: i64 = conn.query_row(
        "SELECT COALESCE(MAX(position), -1) + 1 FROM note_rows WHERE sheet_id = ?1",
        [sheet_id],
        |r| r.get(0),
    )?;
    conn.execute(
        "INSERT INTO note_rows(sheet_id, position, cells_json, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)",
        rusqlite::params![sheet_id, next_position, dump_list(&cells), now_iso()],
    )?;
    let id = conn.last_insert_rowid();
    touch_sheet(&conn, sheet_id)?;
    Ok(NoteRow { id, sheet_id, position: next_position, cells, updated_at: now_iso() })
}

#[tauri::command]
pub fn update_note_row(state: State<AppState>, id: i64, cells: Vec<String>) -> AppResult<NoteRow> {
    let conn = state.db.lock().unwrap();
    let sheet_id: Option<i64> = conn
        .query_row("SELECT sheet_id FROM note_rows WHERE id = ?1", [id], |r| r.get(0))
        .optional()?;
    let Some(sheet_id) = sheet_id else {
        return Err(AppError::NotFound(format!("Note row {id} no longer exists.")));
    };
    let width = sheet_columns(&conn, sheet_id)?.len();
    let cells = fit(cells, width);
    let now = now_iso();
    conn.execute(
        "UPDATE note_rows SET cells_json = ?2, updated_at = ?3 WHERE id = ?1",
        rusqlite::params![id, dump_list(&cells), now],
    )?;
    touch_sheet(&conn, sheet_id)?;
    let position: i64 = conn.query_row("SELECT position FROM note_rows WHERE id = ?1", [id], |r| r.get(0))?;
    Ok(NoteRow { id, sheet_id, position, cells, updated_at: now })
}

#[tauri::command]
pub fn delete_note_row(state: State<AppState>, id: i64) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    let changed = conn.execute("DELETE FROM note_rows WHERE id = ?1", [id])?;
    if changed == 0 {
        return Err(AppError::NotFound(format!("Note row {id} no longer exists.")));
    }
    Ok(())
}

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NoteHit {
    pub sheet_id: i64,
    pub sheet_name: String,
    pub row_id: i64,
    pub cells: Vec<String>,
    /// Which cell matched, so the UI can point straight at it instead of
    /// leaving marko to re-scan the row he just searched for.
    pub matched_column: usize,
    pub column_name: String,
}

/// One search across every sheet. marko: "najst vsetky jednoducho".
///
/// Deliberately a plain case-insensitive substring over the cells, not FTS:
/// he is looking for a nick, a ticket code or a name he half-remembers, and
/// a substring finds "TKT-88" inside "TKT-8801" where a word index would not.
/// The data here is his own typing, measured in hundreds of rows, so the
/// simple scan is also the honest one.
#[tauri::command]
pub fn search_notes(state: State<AppState>, query: String) -> AppResult<Vec<NoteHit>> {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return Ok(Vec::new());
    }
    let conn = state.db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT r.id, r.cells_json, s.id, s.name, s.columns_json
         FROM note_rows r JOIN note_sheets s ON s.id = r.sheet_id
         ORDER BY s.position, s.id, r.position, r.id",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, i64>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, String>(4)?,
        ))
    })?;

    let mut hits = Vec::new();
    for row in rows {
        let (row_id, cells_json, sheet_id, sheet_name, columns_json) = row?;
        let columns = parse_list(&columns_json);
        let cells = fit(parse_list(&cells_json), columns.len());
        // The FIRST matching cell wins - one row is one hit, however many of
        // its cells contain the word.
        if let Some(idx) = cells.iter().position(|c| c.to_lowercase().contains(&needle)) {
            hits.push(NoteHit {
                sheet_id,
                sheet_name: sheet_name.clone(),
                row_id,
                cells,
                matched_column: idx,
                column_name: columns.get(idx).cloned().unwrap_or_default(),
            });
        }
    }
    Ok(hits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_conn;

    fn sheet(conn: &Connection, columns: &[&str]) -> i64 {
        let cols: Vec<String> = columns.iter().map(|c| c.to_string()).collect();
        conn.execute(
            "INSERT INTO note_sheets(name, columns_json, position, created_at, updated_at) VALUES ('S', ?1, 0, ?2, ?2)",
            rusqlite::params![dump_list(&cols), now_iso()],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    fn row(conn: &Connection, sheet_id: i64, cells: &[&str]) -> i64 {
        let c: Vec<String> = cells.iter().map(|s| s.to_string()).collect();
        conn.execute(
            "INSERT INTO note_rows(sheet_id, position, cells_json, created_at, updated_at) VALUES (?1, 0, ?2, ?3, ?3)",
            rusqlite::params![sheet_id, dump_list(&c), now_iso()],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    fn cells_of(conn: &Connection, id: i64) -> Vec<String> {
        parse_list(&conn.query_row("SELECT cells_json FROM note_rows WHERE id=?1", [id], |r| r.get::<_, String>(0)).unwrap())
    }

    #[test]
    fn a_short_row_is_padded_to_the_column_count_not_rejected() {
        assert_eq!(fit(vec!["a".into()], 3), vec!["a".to_string(), String::new(), String::new()]);
    }

    #[test]
    fn a_long_row_is_trimmed_to_the_column_count() {
        assert_eq!(fit(vec!["a".into(), "b".into(), "c".into()], 2), vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn unreadable_stored_json_reads_as_empty_rather_than_failing() {
        assert!(parse_list("not json at all").is_empty());
    }

    #[test]
    fn deleting_a_column_removes_that_cell_from_every_row() {
        let conn = test_conn();
        let s = sheet(&conn, &["A", "B", "C"]);
        let r1 = row(&conn, s, &["a1", "b1", "c1"]);
        let r2 = row(&conn, s, &["a2", "b2", "c2"]);

        // The middle column goes; what is left must still line up.
        reshape_rows(&conn, s, |mut cells| {
            cells.remove(1);
            cells
        })
        .unwrap();

        assert_eq!(cells_of(&conn, r1), vec!["a1".to_string(), "c1".to_string()]);
        assert_eq!(cells_of(&conn, r2), vec!["a2".to_string(), "c2".to_string()]);
    }

    #[test]
    fn adding_a_column_gives_every_row_one_empty_cell() {
        let conn = test_conn();
        let s = sheet(&conn, &["A"]);
        let r = row(&conn, s, &["a"]);
        reshape_rows(&conn, s, |cells| fit(cells, 2)).unwrap();
        assert_eq!(cells_of(&conn, r), vec!["a".to_string(), String::new()]);
    }

    #[test]
    fn moving_a_column_takes_its_cell_along_in_every_row() {
        let conn = test_conn();
        let s = sheet(&conn, &["A", "B", "C"]);
        let r1 = row(&conn, s, &["a1", "b1", "c1"]);
        // A short row from a merge must survive the move, not shift by one.
        let r2 = row(&conn, s, &["a2"]);

        // Move the last column to the front: C, A, B.
        reshape_rows(&conn, s, |cells| {
            let mut cells = fit(cells, 3);
            let moved = cells.remove(2);
            cells.insert(0, moved);
            cells
        })
        .unwrap();

        assert_eq!(cells_of(&conn, r1), vec!["c1".to_string(), "a1".to_string(), "b1".to_string()]);
        assert_eq!(cells_of(&conn, r2), vec![String::new(), "a2".to_string(), String::new()]);
    }
}

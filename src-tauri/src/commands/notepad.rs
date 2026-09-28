//! The notepad - notes, their sub-tabs, their images and what they are about.
//!
//! 2.58.0, from marko's brief after he picked design 02:
//!
//!   · "si vies zaskrtnut ... vies si vybrat kde chces mat fotku, kde chces
//!      pisat, kde ten text ma byt vacsi, aka farba"   -> blocks, see 036
//!   · "v jednej karte si vies pridat podkarty niekde dole ako to je v google
//!      sheets"                                        -> `note_pages`
//!   · "neni to len pre kody ale aj ... eventu, pullu, inventaru, sellu" and
//!     "aj finance"                                    -> `note_links`
//!
//! ## Not to be confused with `commands::notes`
//!
//! `commands::notes` is the SPREADSHEET (`note_sheets` / `note_rows`). This is
//! the notepad. Same unfortunate prefix, different feature - see migration 036.
//!
//! ## Blocks are passed through, not parsed
//!
//! A page's body is `serde_json::Value` end to end. This module never looks
//! inside it except to pull a one-line preview for the list. That is
//! deliberate: block kinds will grow, and a backend that validated them would
//! have to be redeployed in step with the UI on BOTH machines before the newer
//! one could save anything.

use crate::commands::cloud_sync::now_iso;
use crate::db::AppState;
use crate::error::{AppError, AppResult};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::State;

/* ------------------------------------------------------------------ *
 * Shapes
 * ------------------------------------------------------------------ */

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NoteLink {
    pub id: i64,
    /// order | event | ticket | sale | pull | finance
    pub kind: String,
    pub ref_id: i64,
    /// Already resolved for display, so the list does not have to fetch six
    /// other tables to draw one row.
    pub label: String,
    /// 2.62.0: where clicking the chip goes, resolved here because only the
    /// backend can answer it for a ticket (its detail lives inside its ORDER,
    /// so the order id has to be looked up).
    ///
    /// Three of the six kinds have no detail page in this app at all - pull
    /// and finance have only a list, and a ticket has none of its own. Those
    /// point at the nearest thing that EXISTS rather than at a new page: the
    /// brief was explicit that no new module may be built for this.
    pub href: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: i64,
    pub title: String,
    pub tag: String,
    pub pinned: bool,
    pub archived: bool,
    pub note_date: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub page_count: i64,
    pub image_count: i64,
    /// First bit of readable text, for the list.
    pub preview: String,
    pub links: Vec<NoteLink>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NotePage {
    pub id: i64,
    pub note_id: i64,
    pub position: i64,
    pub name: String,
    pub blocks: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NoteImage {
    pub id: i64,
    pub note_id: i64,
    pub data_uri: String,
    pub caption: String,
}

/// The six link columns, in one place, so nothing can be added to the table
/// and forgotten here.
const LINK_COLUMNS: [(&str, &str); 6] = [
    ("order_id", "order"),
    ("event_id", "event"),
    ("ticket_id", "ticket"),
    ("sale_id", "sale"),
    ("pull_id", "pull"),
    ("finance_entry_id", "finance"),
];

/* ------------------------------------------------------------------ *
 * Reads
 * ------------------------------------------------------------------ */

/// One line for a linked record, built from whatever actually identifies it.
/// A record that has since been deleted cannot appear here at all - the link
/// row goes with it (ON DELETE CASCADE, migration 036).
fn link_label(conn: &Connection, kind: &str, id: i64) -> String {
    let sql = match kind {
        "order" => {
            "SELECT o.code || ' · ' || COALESCE(e.name,'—') || ' · ' || o.quantity || ' ks' \
             FROM orders o LEFT JOIN events e ON e.id = o.event_id WHERE o.id = ?1"
        }
        "event" => "SELECT name FROM events WHERE id = ?1",
        "ticket" => {
            "SELECT t.code || ' · ' || COALESCE(t.section,'') || ' ' || COALESCE(t.row_label,'') \
             || ' ' || COALESCE(t.seat,'') FROM tickets t WHERE t.id = ?1"
        }
        "sale" => "SELECT code FROM sales WHERE id = ?1",
        "pull" => {
            "SELECT code || ' · ' || quantity || ' ks' FROM pulls WHERE id = ?1"
        }
        "finance" => {
            "SELECT entry_date || ' · ' || COALESCE(NULLIF(note,''),'—') FROM finance_entries WHERE id = ?1"
        }
        _ => return String::new(),
    };
    conn.query_row(sql, [id], |r| r.get::<_, String>(0))
        .optional()
        .ok()
        .flatten()
        .unwrap_or_default()
        .trim()
        .to_string()
}

/// 2.61.0: links hang off a SUB-TAB, not the note - marko: "to priradenie musi
/// fungovat osobitne v kazdej podkarte zvlast nie spolu". `links_of` is still
/// used for the note-wide view the list column needs; `page_links_of` is what
/// the editor asks for.
/// The route a chip opens. Only routes App.tsx already declares.
fn link_href(conn: &Connection, kind: &str, id: i64) -> String {
    match kind {
        "order" => format!("/orders/{id}"),
        "event" => format!("/events/{id}"),
        "sale" => format!("/sales/{id}"),
        // A ticket has no page of its own; it is shown inside its order.
        "ticket" => conn
            .query_row("SELECT order_id FROM tickets WHERE id = ?1", [id], |r| r.get::<_, i64>(0))
            .optional()
            .ok()
            .flatten()
            .map(|o| format!("/orders/{o}"))
            .unwrap_or_else(|| "/orders".to_string()),
        "pull" => "/pulls".to_string(),
        "finance" => "/finance".to_string(),
        _ => String::new(),
    }
}

fn links_of(conn: &Connection, note_id: i64) -> AppResult<Vec<NoteLink>> {
    let cols = LINK_COLUMNS.map(|(c, _)| c).join(", ");
    let sql = format!("SELECT id, {cols} FROM note_links WHERE note_id = ?1 ORDER BY id");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([note_id], |r| {
        let id: i64 = r.get(0)?;
        let mut found: Option<(String, i64)> = None;
        for (i, (_col, kind)) in LINK_COLUMNS.iter().enumerate() {
            if let Some(v) = r.get::<_, Option<i64>>(i + 1)? {
                found = Some(((*kind).to_string(), v));
                break;
            }
        }
        Ok((id, found))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (id, found) = row?;
        // A link row with every column null cannot happen through this module,
        // but it would be a silent blank line if it ever did - skip it.
        if let Some((kind, ref_id)) = found {
            let label = link_label(conn, &kind, ref_id);
            let href = link_href(conn, &kind, ref_id);
            out.push(NoteLink { id, kind, ref_id, label, href });
        }
    }
    Ok(out)
}

/// The first readable line of the note, for the list. Walks the pages in order
/// and takes the first block that has text in it.
fn preview_of(conn: &Connection, note_id: i64) -> String {
    let raw: Option<String> = conn
        .query_row(
            "SELECT blocks_json FROM note_pages WHERE note_id = ?1 AND blocks_json <> '[]' \
             ORDER BY position, id LIMIT 1",
            [note_id],
            |r| r.get(0),
        )
        .optional()
        .ok()
        .flatten();
    let Some(raw) = raw else { return String::new() };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) else { return String::new() };
    let Some(arr) = v.as_array() else { return String::new() };
    for b in arr {
        if let Some(t) = b.get("t").and_then(|x| x.as_str()) {
            let t = t.trim();
            if !t.is_empty() {
                return t.chars().take(160).collect();
            }
        }
    }
    String::new()
}

fn read_note(conn: &Connection, id: i64) -> AppResult<Note> {
    let row = conn
        .query_row(
            "SELECT id, title, tag, pinned, archived, note_date, created_at, updated_at, \
                    (SELECT COUNT(*) FROM note_pages p WHERE p.note_id = n.id), \
                    (SELECT COUNT(*) FROM note_images i WHERE i.note_id = n.id) \
             FROM notes n WHERE id = ?1",
            [id],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, i64>(4)?,
                    r.get::<_, Option<String>>(5)?,
                    r.get::<_, String>(6)?,
                    r.get::<_, String>(7)?,
                    r.get::<_, i64>(8)?,
                    r.get::<_, i64>(9)?,
                ))
            },
        )
        .optional()?;
    let Some((id, title, tag, pinned, archived, note_date, created_at, updated_at, page_count, image_count)) = row
    else {
        return Err(AppError::NotFound(format!("Note {id} no longer exists.")));
    };
    Ok(Note {
        id,
        title,
        tag,
        pinned: pinned != 0,
        archived: archived != 0,
        note_date,
        created_at,
        updated_at,
        page_count,
        image_count,
        preview: preview_of(conn, id),
        links: links_of(conn, id)?,
    })
}

#[tauri::command]
pub fn list_notes(state: State<AppState>, include_archived: bool) -> AppResult<Vec<Note>> {
    let conn = state.db.lock().unwrap();
    // 2.61.0: hand-set `position` first, most-recently-changed as the
    // tiebreaker - so a note marko put at the top stays there when he edits a
    // different one, which is what "hybat s poznamkamy" is for.
    let sql = if include_archived {
        "SELECT id FROM notes ORDER BY pinned DESC, position, updated_at DESC"
    } else {
        "SELECT id FROM notes WHERE archived = 0 ORDER BY pinned DESC, position, updated_at DESC"
    };
    let ids: Vec<i64> = {
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map([], |r| r.get::<_, i64>(0))?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    ids.into_iter().map(|id| read_note(&conn, id)).collect()
}

#[tauri::command]
pub fn get_note(state: State<AppState>, id: i64) -> AppResult<Note> {
    let conn = state.db.lock().unwrap();
    read_note(&conn, id)
}

/* ------------------------------------------------------------------ *
 * Notes
 * ------------------------------------------------------------------ */

#[tauri::command]
pub fn create_note(state: State<AppState>, title: String) -> AppResult<Note> {
    let mut conn = state.db.lock().unwrap();
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO notes(title, created_at, updated_at) VALUES (?1, ?2, ?2)",
        rusqlite::params![title.trim(), now_iso()],
    )?;
    let id = tx.last_insert_rowid();
    // A note always has at least one sub-tab, so the editor never has to draw
    // an empty shell and marko never has to create one before he can type.
    tx.execute(
        "INSERT INTO note_pages(note_id, position, name, created_at, updated_at) VALUES (?1, 0, '', ?2, ?2)",
        rusqlite::params![id, now_iso()],
    )?;
    tx.commit()?;
    read_note(&conn, id)
}

#[tauri::command]
pub fn update_note(
    state: State<AppState>,
    id: i64,
    title: String,
    tag: String,
    note_date: Option<String>,
) -> AppResult<Note> {
    let conn = state.db.lock().unwrap();
    let date = note_date.map(|d| d.trim().to_string()).filter(|d| !d.is_empty());
    let n = conn.execute(
        "UPDATE notes SET title = ?2, tag = ?3, note_date = ?4, updated_at = ?5 WHERE id = ?1",
        rusqlite::params![id, title.trim(), tag.trim(), date, now_iso()],
    )?;
    if n == 0 {
        return Err(AppError::NotFound(format!("Note {id} no longer exists.")));
    }
    read_note(&conn, id)
}

#[tauri::command]
pub fn set_note_flags(
    state: State<AppState>,
    id: i64,
    pinned: Option<bool>,
    archived: Option<bool>,
) -> AppResult<Note> {
    let conn = state.db.lock().unwrap();
    // Each flag is optional and absent means LEAVE ALONE - pinning must not
    // quietly un-archive, and vice versa.
    if let Some(p) = pinned {
        conn.execute(
            "UPDATE notes SET pinned = ?2, updated_at = ?3 WHERE id = ?1",
            rusqlite::params![id, if p { 1 } else { 0 }, now_iso()],
        )?;
    }
    if let Some(a) = archived {
        conn.execute(
            "UPDATE notes SET archived = ?2, updated_at = ?3 WHERE id = ?1",
            rusqlite::params![id, if a { 1 } else { 0 }, now_iso()],
        )?;
    }
    read_note(&conn, id)
}

#[tauri::command]
pub fn delete_note(state: State<AppState>, id: i64) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    conn.execute("DELETE FROM notes WHERE id = ?1", [id])?;
    Ok(())
}

/* ------------------------------------------------------------------ *
 * Sub-tabs
 * ------------------------------------------------------------------ */

fn read_page(conn: &Connection, id: i64) -> AppResult<NotePage> {
    let row = conn
        .query_row(
            "SELECT id, note_id, position, name, blocks_json FROM note_pages WHERE id = ?1",
            [id],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                ))
            },
        )
        .optional()?;
    let Some((id, note_id, position, name, blocks_json)) = row else {
        return Err(AppError::NotFound(format!("Note tab {id} no longer exists.")));
    };
    Ok(NotePage {
        id,
        note_id,
        position,
        name,
        // Unreadable stored JSON reads as an empty page rather than failing the
        // whole note - one bad blob must not lock him out of the rest.
        blocks: serde_json::from_str(&blocks_json).unwrap_or_else(|_| serde_json::json!([])),
    })
}

#[tauri::command]
pub fn list_note_pages(state: State<AppState>, note_id: i64) -> AppResult<Vec<NotePage>> {
    let conn = state.db.lock().unwrap();
    let ids: Vec<i64> = {
        let mut stmt =
            conn.prepare("SELECT id FROM note_pages WHERE note_id = ?1 ORDER BY position, id")?;
        let rows = stmt.query_map([note_id], |r| r.get::<_, i64>(0))?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    ids.into_iter().map(|id| read_page(&conn, id)).collect()
}

#[tauri::command]
pub fn create_note_page(state: State<AppState>, note_id: i64, name: String) -> AppResult<NotePage> {
    let conn = state.db.lock().unwrap();
    let next: i64 = conn.query_row(
        "SELECT COALESCE(MAX(position), -1) + 1 FROM note_pages WHERE note_id = ?1",
        [note_id],
        |r| r.get(0),
    )?;
    conn.execute(
        "INSERT INTO note_pages(note_id, position, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)",
        rusqlite::params![note_id, next, name.trim(), now_iso()],
    )?;
    let id = conn.last_insert_rowid();
    touch_note(&conn, note_id)?;
    read_page(&conn, id)
}

#[tauri::command]
pub fn rename_note_page(state: State<AppState>, id: i64, name: String) -> AppResult<NotePage> {
    let conn = state.db.lock().unwrap();
    conn.execute(
        "UPDATE note_pages SET name = ?2, updated_at = ?3 WHERE id = ?1",
        rusqlite::params![id, name.trim(), now_iso()],
    )?;
    let page = read_page(&conn, id)?;
    touch_note(&conn, page.note_id)?;
    Ok(page)
}

/// The whole page body at once. The UI debounces; this is not called per
/// keystroke.
#[tauri::command]
pub fn save_note_page(state: State<AppState>, id: i64, blocks: serde_json::Value) -> AppResult<NotePage> {
    let conn = state.db.lock().unwrap();
    let json = serde_json::to_string(&blocks)
        .map_err(|e| AppError::Other(format!("could not store this note: {e}")))?;
    let n = conn.execute(
        "UPDATE note_pages SET blocks_json = ?2, updated_at = ?3 WHERE id = ?1",
        rusqlite::params![id, json, now_iso()],
    )?;
    if n == 0 {
        return Err(AppError::NotFound(format!("Note tab {id} no longer exists.")));
    }
    let page = read_page(&conn, id)?;
    touch_note(&conn, page.note_id)?;
    Ok(page)
}

#[tauri::command]
pub fn delete_note_page(state: State<AppState>, id: i64) -> AppResult<Vec<NotePage>> {
    let conn = state.db.lock().unwrap();
    let note_id: i64 = conn
        .query_row("SELECT note_id FROM note_pages WHERE id = ?1", [id], |r| r.get(0))
        .optional()?
        .ok_or_else(|| AppError::NotFound(format!("Note tab {id} no longer exists.")))?;
    let count: i64 =
        conn.query_row("SELECT COUNT(*) FROM note_pages WHERE note_id = ?1", [note_id], |r| r.get(0))?;
    if count <= 1 {
        return Err(AppError::Validation("A note needs at least one tab.".to_string()));
    }
    conn.execute("DELETE FROM note_pages WHERE id = ?1", [id])?;
    touch_note(&conn, note_id)?;
    drop(conn);
    list_note_pages(state, note_id)
}

/// Re-numbers a whole set from 0 upward in the order given. One statement per
/// row inside the caller's transaction.
///
/// Rewriting EVERY position, rather than swapping two, is deliberate: a list
/// that has ever had a row deleted has gaps, and swapping two numbers across a
/// gap silently reorders something else. After this the positions are always
/// 0..n-1 with no gaps and no duplicates, whatever shape they were in before.
fn renumber(conn: &Connection, table: &str, parent_col: &str, parent: i64, ordered: &[i64]) -> AppResult<()> {
    for (i, id) in ordered.iter().enumerate() {
        conn.execute(
            &format!("UPDATE {table} SET position = ?2 WHERE id = ?1 AND {parent_col} = ?3"),
            rusqlite::params![id, i as i64, parent],
        )?;
    }
    Ok(())
}

/// Moves one sub-tab to another slot. The UI sends the ids it wants, in order.
#[tauri::command]
pub fn reorder_note_pages(state: State<AppState>, note_id: i64, ordered_ids: Vec<i64>) -> AppResult<Vec<NotePage>> {
    let mut conn = state.db.lock().unwrap();
    let tx = conn.transaction()?;
    renumber(&tx, "note_pages", "note_id", note_id, &ordered_ids)?;
    tx.execute(
        "UPDATE notes SET updated_at = ?2 WHERE id = ?1",
        rusqlite::params![note_id, now_iso()],
    )?;
    tx.commit()?;
    let ids: Vec<i64> = {
        let mut stmt = conn.prepare("SELECT id FROM note_pages WHERE note_id = ?1 ORDER BY position, id")?;
        let rows = stmt.query_map([note_id], |r| r.get::<_, i64>(0))?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    ids.into_iter().map(|id| read_page(&conn, id)).collect()
}

/// Moves notes in the list. Same whole-set renumber, for the same reason.
#[tauri::command]
pub fn reorder_notes(state: State<AppState>, ordered_ids: Vec<i64>) -> AppResult<()> {
    let mut conn = state.db.lock().unwrap();
    let tx = conn.transaction()?;
    for (i, id) in ordered_ids.iter().enumerate() {
        // No `updated_at` bump: moving a note in a list is not editing it, and
        // bumping it would reshuffle the very ordering being set.
        tx.execute(
            "UPDATE notes SET position = ?2 WHERE id = ?1",
            rusqlite::params![id, i as i64],
        )?;
    }
    tx.commit()?;
    Ok(())
}

fn touch_note(conn: &Connection, note_id: i64) -> AppResult<()> {
    conn.execute(
        "UPDATE notes SET updated_at = ?2 WHERE id = ?1",
        rusqlite::params![note_id, now_iso()],
    )?;
    Ok(())
}

fn page_links_of(conn: &Connection, page_id: i64) -> AppResult<Vec<NoteLink>> {
    let cols = LINK_COLUMNS.map(|(c, _)| c).join(", ");
    let sql = format!("SELECT id, {cols} FROM note_links WHERE page_id = ?1 ORDER BY id");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([page_id], |r| {
        let id: i64 = r.get(0)?;
        let mut found: Option<(String, i64)> = None;
        for (i, (_col, kind)) in LINK_COLUMNS.iter().enumerate() {
            if let Some(v) = r.get::<_, Option<i64>>(i + 1)? {
                found = Some(((*kind).to_string(), v));
                break;
            }
        }
        Ok((id, found))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (id, found) = row?;
        if let Some((kind, ref_id)) = found {
            let label = link_label(conn, &kind, ref_id);
            let href = link_href(conn, &kind, ref_id);
            out.push(NoteLink { id, kind, ref_id, label, href });
        }
    }
    Ok(out)
}

#[tauri::command]
pub fn list_note_page_links(state: State<AppState>, page_id: i64) -> AppResult<Vec<NoteLink>> {
    let conn = state.db.lock().unwrap();
    page_links_of(&conn, page_id)
}

/// A name that is not already taken in this set: "X copy", then "X copy 2".
fn copy_name(base: &str, taken: &[String]) -> String {
    let stem = if base.trim().is_empty() { "Bez názvu" } else { base };
    let first = format!("{stem} copy");
    if !taken.iter().any(|t| t == &first) {
        return first;
    }
    for i in 2..1000 {
        let c = format!("{stem} copy {i}");
        if !taken.iter().any(|t| t == &c) {
            return c;
        }
    }
    first
}

/// Copies one sub-tab, directly after the original.
///
/// ONE transaction. A half-copied tab - blocks in, links missing, position not
/// shifted - is worse than no copy at all, and there is no way for marko to
/// tell it happened.
///
/// `with_links` defaults to false at the call site: a duplicate is a starting
/// point, and silently attaching it to the same order as the original is the
/// kind of thing that is noticed only after something has been sent twice.
///
/// Images are NOT copied. The copy points at the same `note_images` rows, and
/// it may: those rows belong to the NOTE, and the copy is in the same note.
/// They live in the database and sync uploads the whole file, so copying the
/// bytes would double what is paid for on every sync from then on.
/// `duplicate_note` below is the case where they genuinely have to be copied.
#[tauri::command]
pub fn duplicate_note_page(state: State<AppState>, page_id: i64, with_links: bool) -> AppResult<Vec<NotePage>> {
    let mut conn = state.db.lock().unwrap();
    let tx = conn.transaction()?;
    let (note_id, position, name, blocks_json) = tx
        .query_row(
            "SELECT note_id, position, name, blocks_json FROM note_pages WHERE id = ?1",
            [page_id],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?)),
        )
        .optional()?
        .ok_or_else(|| AppError::NotFound(format!("Note tab {page_id} no longer exists.")))?;

    let taken: Vec<String> = {
        let mut stmt = tx.prepare("SELECT name FROM note_pages WHERE note_id = ?1")?;
        let rows = stmt.query_map([note_id], |r| r.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    tx.execute(
        "UPDATE note_pages SET position = position + 1 WHERE note_id = ?1 AND position > ?2",
        rusqlite::params![note_id, position],
    )?;
    tx.execute(
        "INSERT INTO note_pages(note_id, position, name, blocks_json, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
        rusqlite::params![note_id, position + 1, copy_name(&name, &taken), blocks_json, now_iso()],
    )?;
    let new_page = tx.last_insert_rowid();

    if with_links {
        let cols = LINK_COLUMNS.map(|(c, _)| c).join(", ");
        tx.execute(
            &format!(
                "INSERT INTO note_links(note_id, page_id, {cols}, created_at) \
                 SELECT note_id, ?2, {cols}, ?3 FROM note_links WHERE page_id = ?1"
            ),
            rusqlite::params![page_id, new_page, now_iso()],
        )?;
    }
    tx.execute("UPDATE notes SET updated_at = ?2 WHERE id = ?1", rusqlite::params![note_id, now_iso()])?;
    tx.commit()?;
    drop(conn);
    list_note_pages(state, note_id)
}

/// Re-points `{"k":"image","id":…}` blocks at the copy's own image rows.
///
/// Everything it does not recognise is left exactly as it was: an id with no
/// entry in the map, a block of another kind, or a body that is not the JSON
/// array it is supposed to be. Rewriting is the only thing that can go wrong
/// here, so it does as little of it as possible.
fn remap_images(blocks_json: &str, map: &[(i64, i64)]) -> String {
    let Ok(mut v) = serde_json::from_str::<serde_json::Value>(blocks_json) else {
        return blocks_json.to_string();
    };
    let Some(arr) = v.as_array_mut() else {
        return blocks_json.to_string();
    };
    for b in arr.iter_mut() {
        if b.get("k").and_then(|x| x.as_str()) != Some("image") {
            continue;
        }
        let Some(old) = b.get("id").and_then(|x| x.as_i64()) else {
            continue;
        };
        let Some(&(_, fresh)) = map.iter().find(|(o, _)| *o == old) else {
            continue;
        };
        if let Some(obj) = b.as_object_mut() {
            obj.insert("id".to_string(), serde_json::Value::from(fresh));
        }
    }
    serde_json::to_string(&v).unwrap_or_else(|_| blocks_json.to_string())
}

/// Copies a whole note - every tab, in order, with its blocks and its images.
///
/// Same one-transaction rule, and for a stronger reason: a note is a tree, and
/// a partial tree is a note marko would have to unpick by hand.
#[tauri::command]
pub fn duplicate_note(state: State<AppState>, note_id: i64, with_links: bool) -> AppResult<Note> {
    let mut conn = state.db.lock().unwrap();
    let tx = conn.transaction()?;
    let (title, tag, note_date) = tx
        .query_row(
            "SELECT title, tag, note_date FROM notes WHERE id = ?1",
            [note_id],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, Option<String>>(2)?)),
        )
        .optional()?
        .ok_or_else(|| AppError::NotFound(format!("Note {note_id} no longer exists.")))?;

    let taken: Vec<String> = {
        let mut stmt = tx.prepare("SELECT title FROM notes")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    let position: i64 =
        tx.query_row("SELECT position FROM notes WHERE id = ?1", [note_id], |r| r.get(0))?;
    // The copy sits directly under the original rather than at the end, which
    // is where you look for it.
    tx.execute("UPDATE notes SET position = position + 1 WHERE position > ?1", [position])?;
    tx.execute(
        "INSERT INTO notes(title, tag, note_date, position, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
        rusqlite::params![copy_name(&title, &taken), tag, note_date, position + 1, now_iso()],
    )?;
    let new_note = tx.last_insert_rowid();

    // Images belong to a NOTE, so the copy needs its own rows and its blocks
    // have to be re-pointed at them. Without this every picture in the copy
    // would draw "Obrázok sa nenašiel": the block would still name the
    // original's row, which `list_note_images` never returns for this note.
    //
    // This is the one place the bytes are copied. A sub-tab copy stays inside
    // the same note and keeps pointing at the same rows, which is why it does
    // not pay this - see `duplicate_note_page`.
    let images: Vec<(i64, String, String)> = {
        let mut stmt = tx.prepare("SELECT id, data_uri, caption FROM note_images WHERE note_id = ?1 ORDER BY id")?;
        let rows = stmt.query_map([note_id], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?))
        })?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    let mut image_map: Vec<(i64, i64)> = Vec::new();
    for (old_image, data_uri, caption) in images {
        tx.execute(
            "INSERT INTO note_images(note_id, data_uri, caption, created_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![new_note, data_uri, caption, now_iso()],
        )?;
        image_map.push((old_image, tx.last_insert_rowid()));
    }

    let pages: Vec<(i64, i64, String, String)> = {
        let mut stmt = tx.prepare(
            "SELECT id, position, name, blocks_json FROM note_pages WHERE note_id = ?1 ORDER BY position, id",
        )?;
        let rows = stmt.query_map([note_id], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?))
        })?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    let cols = LINK_COLUMNS.map(|(c, _)| c).join(", ");
    for (old_page, pos, name, blocks) in pages {
        tx.execute(
            "INSERT INTO note_pages(note_id, position, name, blocks_json, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
            rusqlite::params![new_note, pos, name, remap_images(&blocks, &image_map), now_iso()],
        )?;
        if with_links {
            let np = tx.last_insert_rowid();
            tx.execute(
                &format!(
                    "INSERT INTO note_links(note_id, page_id, {cols}, created_at) \
                     SELECT ?2, ?3, {cols}, ?4 FROM note_links WHERE page_id = ?1"
                ),
                rusqlite::params![old_page, new_note, np, now_iso()],
            )?;
        }
    }
    tx.commit()?;
    read_note(&conn, new_note)
}

/* ------------------------------------------------------------------ *
 * Images
 * ------------------------------------------------------------------ */

/// The UI downscales and re-encodes before this is called (migration 036).
/// The cap here is the backstop, not the mechanism: a 6MB screenshot must not
/// reach the file that gets uploaded on every sync.
const MAX_IMAGE_CHARS: usize = 1_200_000; // ~900 KB of JPEG once base64 is undone

#[tauri::command]
pub fn add_note_image(
    state: State<AppState>,
    note_id: i64,
    data_uri: String,
    caption: String,
) -> AppResult<NoteImage> {
    if !data_uri.starts_with("data:image/") {
        return Err(AppError::Validation("That is not an image.".to_string()));
    }
    if data_uri.len() > MAX_IMAGE_CHARS {
        return Err(AppError::Validation(
            "That image is too big even after shrinking - try a smaller one.".to_string(),
        ));
    }
    let conn = state.db.lock().unwrap();
    conn.execute(
        "INSERT INTO note_images(note_id, data_uri, caption, created_at) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![note_id, data_uri, caption.trim(), now_iso()],
    )?;
    let id = conn.last_insert_rowid();
    touch_note(&conn, note_id)?;
    Ok(NoteImage { id, note_id, data_uri, caption: caption.trim().to_string() })
}

#[tauri::command]
pub fn list_note_images(state: State<AppState>, note_id: i64) -> AppResult<Vec<NoteImage>> {
    let conn = state.db.lock().unwrap();
    let mut stmt =
        conn.prepare("SELECT id, note_id, data_uri, caption FROM note_images WHERE note_id = ?1 ORDER BY id")?;
    let rows = stmt.query_map([note_id], |r| {
        Ok(NoteImage { id: r.get(0)?, note_id: r.get(1)?, data_uri: r.get(2)?, caption: r.get(3)? })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// The caption of an attached image.
///
/// `note_images.caption` has existed since migration 036 and was never
/// written to after creation - the UI kept a second copy inside the block
/// instead. This is the command that column was always missing; the block's
/// own copy goes away with 2.62.0.
#[tauri::command]
pub fn set_note_image_caption(state: State<AppState>, id: i64, caption: String) -> AppResult<NoteImage> {
    let conn = state.db.lock().unwrap();
    let n = conn.execute(
        "UPDATE note_images SET caption = ?2 WHERE id = ?1",
        rusqlite::params![id, caption.trim()],
    )?;
    if n == 0 {
        return Err(AppError::NotFound(format!("Image {id} no longer exists.")));
    }
    let img = conn.query_row(
        "SELECT id, note_id, data_uri, caption FROM note_images WHERE id = ?1",
        [id],
        |r| Ok(NoteImage { id: r.get(0)?, note_id: r.get(1)?, data_uri: r.get(2)?, caption: r.get(3)? }),
    )?;
    touch_note(&conn, img.note_id)?;
    Ok(img)
}

#[tauri::command]
pub fn delete_note_image(state: State<AppState>, id: i64) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    conn.execute("DELETE FROM note_images WHERE id = ?1", [id])?;
    Ok(())
}

/* ------------------------------------------------------------------ *
 * Links
 * ------------------------------------------------------------------ */

#[tauri::command]
pub fn add_note_link(state: State<AppState>, page_id: i64, kind: String, ref_id: i64) -> AppResult<Vec<NoteLink>> {
    let column = LINK_COLUMNS
        .iter()
        .find(|(_, k)| *k == kind)
        .map(|(c, _)| *c)
        .ok_or_else(|| AppError::Validation(format!("Cannot attach a note to a {kind}.")))?;
    let conn = state.db.lock().unwrap();
    // `note_id` is derived from the page, never passed in, so the two columns
    // cannot disagree about which note a link belongs to.
    let note_id: i64 = conn
        .query_row("SELECT note_id FROM note_pages WHERE id = ?1", [page_id], |r| r.get(0))
        .optional()?
        .ok_or_else(|| AppError::NotFound(format!("Note tab {page_id} no longer exists.")))?;
    // The same thing twice on one TAB is a no-op, not an error - he clicked it
    // twice, he did not do anything wrong. Twice on two different tabs is two
    // real links, which is the whole point of them being per-tab.
    let exists: Option<i64> = conn
        .query_row(
            &format!("SELECT id FROM note_links WHERE page_id = ?1 AND {column} = ?2"),
            rusqlite::params![page_id, ref_id],
            |r| r.get(0),
        )
        .optional()?;
    if exists.is_none() {
        conn.execute(
            &format!("INSERT INTO note_links(note_id, page_id, {column}, created_at) VALUES (?1, ?2, ?3, ?4)"),
            rusqlite::params![note_id, page_id, ref_id, now_iso()],
        )?;
        touch_note(&conn, note_id)?;
    }
    page_links_of(&conn, page_id)
}

#[tauri::command]
pub fn delete_note_link(state: State<AppState>, id: i64) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    conn.execute("DELETE FROM note_links WHERE id = ?1", [id])?;
    Ok(())
}

/* ------------------------------------------------------------------ *
 * Search
 * ------------------------------------------------------------------ */

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NotepadHit {
    pub note_id: i64,
    pub note_title: String,
    pub page_id: i64,
    pub page_name: String,
    pub excerpt: String,
}

/// Titles, tab names and the text inside blocks. Archived notes are skipped.
#[tauri::command]
pub fn search_notepad(state: State<AppState>, query: String) -> AppResult<Vec<NotepadHit>> {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return Ok(Vec::new());
    }
    let conn = state.db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT n.id, n.title, p.id, p.name, p.blocks_json \
         FROM notes n JOIN note_pages p ON p.note_id = n.id \
         WHERE n.archived = 0 ORDER BY n.pinned DESC, n.updated_at DESC, p.position",
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
    let mut seen_note: Option<i64> = None;
    for row in rows {
        let (note_id, note_title, page_id, page_name, blocks_json) = row?;
        let mut excerpt = String::new();
        if note_title.to_lowercase().contains(&needle) || page_name.to_lowercase().contains(&needle) {
            excerpt = page_name.clone();
        } else if let Ok(v) = serde_json::from_str::<serde_json::Value>(&blocks_json) {
            if let Some(arr) = v.as_array() {
                for b in arr {
                    if let Some(t) = b.get("t").and_then(|x| x.as_str()) {
                        if t.to_lowercase().contains(&needle) {
                            excerpt = t.chars().take(140).collect();
                            break;
                        }
                    }
                }
            }
        }
        if excerpt.is_empty() {
            continue;
        }
        // One line per note for a title match; a text match is worth its own
        // line because it says WHICH tab it is in.
        if seen_note == Some(note_id) && excerpt == page_name {
            continue;
        }
        seen_note = Some(note_id);
        hits.push(NotepadHit { note_id, note_title, page_id, page_name, excerpt });
    }
    Ok(hits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_copy_never_takes_a_name_that_is_already_there() {
        assert_eq!(copy_name("ABC123", &[]), "ABC123 copy");
        assert_eq!(copy_name("ABC123", &["ABC123 copy".into()]), "ABC123 copy 2");
        assert_eq!(
            copy_name("ABC123", &["ABC123 copy".into(), "ABC123 copy 2".into()]),
            "ABC123 copy 3"
        );
        // An unnamed tab still gets a name rather than " copy".
        assert_eq!(copy_name("", &[]), "Bez názvu copy");
        assert_eq!(copy_name("   ", &[]), "Bez názvu copy");
    }

    #[test]
    fn every_link_column_has_a_kind_and_they_are_unique() {
        let kinds: Vec<&str> = LINK_COLUMNS.iter().map(|(_, k)| *k).collect();
        let mut sorted = kinds.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), kinds.len(), "two link columns share a kind");
        assert_eq!(kinds.len(), 6);
    }

    #[test]
    fn a_link_kind_the_table_has_no_column_for_is_refused() {
        assert!(LINK_COLUMNS.iter().any(|(_, k)| *k == "finance"));
        assert!(!LINK_COLUMNS.iter().any(|(_, k)| *k == "supplier"));
    }
}

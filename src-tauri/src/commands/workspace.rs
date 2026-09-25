//! Workspace - notes, records and tasks, plus the search that spans them and
//! the tables in `commands::notes`.
//!
//! 2.52.0, from marko's written brief: "miesto na ktore sa mozem spolahnut",
//! with the rule that a quick note must be able to become a task or a record
//! later without being retyped. That is why `workspace_items` is one table
//! with a `kind` rather than three (see migration 032) - converting is an
//! UPDATE, not a move between tables.
//!
//! ## What this module deliberately does NOT do
//!
//! No reminders, no recurrence, no per-field types on a record, no sharing.
//! The brief is explicit about not turning this into Notion or a CRM. What is
//! here is capture, organise, find, use.
//!
//! ## Storing credentials
//!
//! A record may hold a field marko names "Password". This app has no secure
//! credential store - the database is a plain file - so nothing here pretends
//! otherwise: such a field is stored as typed, like any other. What the UI
//! does do is keep a masked field out of search previews (see
//! `search_workspace`, which skips fields whose name looks like a secret), so
//! it is at least never shown somewhere marko did not open on purpose.

use crate::commands::cloud_sync::now_iso;
use crate::db::AppState;
use crate::error::{AppError, AppResult};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceField {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ChecklistItem {
    pub text: String,
    pub done: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceItem {
    pub id: i64,
    /// note | record | task. One row changes kind in place.
    pub kind: String,
    pub title: String,
    pub content: String,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub fields: Vec<WorkspaceField>,
    pub checklist: Vec<ChecklistItem>,
    /// open | done, meaningful on a task.
    pub status: Option<String>,
    pub due_date: Option<String>,
    pub pinned: bool,
    pub archived: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// What the UI sends to create or save one item. `id` absent means create.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceItemInput {
    pub id: Option<i64>,
    pub kind: String,
    pub title: String,
    pub content: String,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub fields: Vec<WorkspaceField>,
    pub checklist: Vec<ChecklistItem>,
    pub status: Option<String>,
    pub due_date: Option<String>,
    pub pinned: bool,
    pub archived: bool,
}

const KINDS: [&str; 3] = ["note", "record", "task"];

fn json_of<T: Serialize>(v: &T) -> String {
    serde_json::to_string(v).unwrap_or_else(|_| "[]".to_string())
}

/// Stored JSON to a value. Anything unreadable becomes the empty default
/// rather than an error - a workspace that refuses to open because one blob
/// is malformed would lose far more than it protects.
fn from_json<T: for<'de> Deserialize<'de> + Default>(raw: &str) -> T {
    serde_json::from_str(raw).unwrap_or_default()
}

/// Lower-cased, trimmed, no leading '#', no blanks, no duplicates - so
/// "#Oasis", "oasis " and "oasis" are one tag and the filter list stays short.
fn clean_tags(raw: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for t in raw {
        let t = t.trim().trim_start_matches('#').trim().to_lowercase();
        if !t.is_empty() && !out.contains(&t) {
            out.push(t);
        }
    }
    out
}

/// A field whose NAME suggests a secret. Used only to keep such a value out of
/// search previews - it is not encryption and is not presented as security.
pub(crate) fn looks_secret(name: &str) -> bool {
    let n = name.to_lowercase();
    ["password", "heslo", "pass", "pin", "secret", "token", "api key", "apikey", "2fa", "seed"]
        .iter()
        .any(|needle| n.contains(needle))
}

fn row_to_item(r: &rusqlite::Row<'_>) -> rusqlite::Result<WorkspaceItem> {
    Ok(WorkspaceItem {
        id: r.get("id")?,
        kind: r.get("kind")?,
        title: r.get("title")?,
        content: r.get("content")?,
        category: r.get("category")?,
        tags: from_json(&r.get::<_, String>("tags_json")?),
        fields: from_json(&r.get::<_, String>("fields_json")?),
        checklist: from_json(&r.get::<_, String>("checklist_json")?),
        status: r.get("status")?,
        due_date: r.get("due_date")?,
        pinned: r.get::<_, i64>("pinned")? != 0,
        archived: r.get::<_, i64>("archived")? != 0,
        created_at: r.get("created_at")?,
        updated_at: r.get("updated_at")?,
    })
}

const SELECT_ITEM: &str = "SELECT id, kind, title, content, category, tags_json, fields_json, \
     checklist_json, status, due_date, pinned, archived, created_at, updated_at FROM workspace_items";

fn read_item(conn: &Connection, id: i64) -> AppResult<WorkspaceItem> {
    let sql = format!("{SELECT_ITEM} WHERE id = ?1");
    conn.query_row(&sql, [id], |r| row_to_item(r))
        .optional()?
        .ok_or_else(|| AppError::NotFound(format!("Workspace item {id} no longer exists.")))
}

#[tauri::command]
pub fn list_workspace_items(state: State<AppState>, include_archived: bool) -> AppResult<Vec<WorkspaceItem>> {
    let conn = state.db.lock().unwrap();
    // Pinned first, then most recently touched - the order the home screen
    // wants, decided here so every caller agrees on it.
    let sql = format!(
        "{SELECT_ITEM} {} ORDER BY pinned DESC, updated_at DESC, id DESC",
        if include_archived { "" } else { "WHERE archived = 0" }
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], |r| row_to_item(r))?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Creates when `id` is absent, updates when it is present. One command
/// rather than a create/update pair: the editor holds a whole item either way,
/// and two commands would only give the UI a second thing to get wrong.
#[tauri::command]
pub fn save_workspace_item(state: State<AppState>, item: WorkspaceItemInput) -> AppResult<WorkspaceItem> {
    if !KINDS.contains(&item.kind.as_str()) {
        return Err(AppError::Validation(format!(
            "{} is not a workspace item kind ({}).",
            item.kind,
            KINDS.join(", ")
        )));
    }
    let title = item.title.trim().to_string();
    let content = item.content.trim_end().to_string();
    // A quick note is a line of text with no title, and the brief asks for it
    // to be savable in seconds - so an item with a body is never rejected for
    // having no title. An item with NEITHER is nothing at all.
    if title.is_empty() && content.is_empty() && item.fields.is_empty() {
        return Err(AppError::Validation("Write something first - a title, a line, or a field.".to_string()));
    }
    let tags = clean_tags(&item.tags);
    let category = item.category.as_ref().map(|c| c.trim().to_string()).filter(|c| !c.is_empty());
    // A task always has a state; nothing else carries one.
    let status = if item.kind == "task" {
        Some(match item.status.as_deref() {
            Some("done") => "done".to_string(),
            _ => "open".to_string(),
        })
    } else {
        None
    };
    let due = item.due_date.as_ref().map(|d| d.trim().to_string()).filter(|d| !d.is_empty());
    let now = now_iso();

    let conn = state.db.lock().unwrap();
    match item.id {
        Some(id) => {
            let changed = conn.execute(
                "UPDATE workspace_items SET kind=?2, title=?3, content=?4, category=?5, tags_json=?6, \
                 fields_json=?7, checklist_json=?8, status=?9, due_date=?10, pinned=?11, archived=?12, \
                 updated_at=?13 WHERE id=?1",
                rusqlite::params![
                    id,
                    item.kind,
                    title,
                    content,
                    category,
                    json_of(&tags),
                    json_of(&item.fields),
                    json_of(&item.checklist),
                    status,
                    due,
                    i64::from(item.pinned),
                    i64::from(item.archived),
                    now
                ],
            )?;
            if changed == 0 {
                return Err(AppError::NotFound(format!("Workspace item {id} no longer exists.")));
            }
            read_item(&conn, id)
        }
        None => {
            conn.execute(
                "INSERT INTO workspace_items(kind, title, content, category, tags_json, fields_json, \
                 checklist_json, status, due_date, pinned, archived, created_at, updated_at) \
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?12)",
                rusqlite::params![
                    item.kind,
                    title,
                    content,
                    category,
                    json_of(&tags),
                    json_of(&item.fields),
                    json_of(&item.checklist),
                    status,
                    due,
                    i64::from(item.pinned),
                    i64::from(item.archived),
                    now
                ],
            )?;
            read_item(&conn, conn.last_insert_rowid())
        }
    }
}

/// Pin / unpin, archive / restore, and a task's done state - the three things
/// done from a list without opening anything. Each is `Option`: absent means
/// "leave it alone", so one command serves all three without a caller ever
/// having to send back a value it did not mean to change.
#[tauri::command]
pub fn set_workspace_item_flags(
    state: State<AppState>,
    id: i64,
    pinned: Option<bool>,
    archived: Option<bool>,
    status: Option<String>,
) -> AppResult<WorkspaceItem> {
    let conn = state.db.lock().unwrap();
    let current = read_item(&conn, id)?;
    let next_status = match status {
        Some(s) if current.kind == "task" => Some(if s == "done" { "done".to_string() } else { "open".to_string() }),
        Some(_) => current.status.clone(),
        None => current.status.clone(),
    };
    conn.execute(
        "UPDATE workspace_items SET pinned=?2, archived=?3, status=?4, updated_at=?5 WHERE id=?1",
        rusqlite::params![
            id,
            i64::from(pinned.unwrap_or(current.pinned)),
            i64::from(archived.unwrap_or(current.archived)),
            next_status,
            now_iso()
        ],
    )?;
    read_item(&conn, id)
}

#[tauri::command]
pub fn delete_workspace_item(state: State<AppState>, id: i64) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    let changed = conn.execute("DELETE FROM workspace_items WHERE id = ?1", [id])?;
    if changed == 0 {
        return Err(AppError::NotFound(format!("Workspace item {id} no longer exists.")));
    }
    Ok(())
}

/// Duplicating is how a recurring procedure gets reused - the brief's
/// "recurring procedures" without building recurrence.
#[tauri::command]
pub fn duplicate_workspace_item(state: State<AppState>, id: i64) -> AppResult<WorkspaceItem> {
    let conn = state.db.lock().unwrap();
    let src = read_item(&conn, id)?;
    let now = now_iso();
    // The copy is never pinned and never archived: a duplicate is a new piece
    // of work, not a second copy of an old decision. A copied task starts open.
    conn.execute(
        "INSERT INTO workspace_items(kind, title, content, category, tags_json, fields_json, \
         checklist_json, status, due_date, pinned, archived, created_at, updated_at) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,0,0,?10,?10)",
        rusqlite::params![
            src.kind,
            format!("{} (copy)", if src.title.is_empty() { "Untitled" } else { &src.title }),
            src.content,
            src.category,
            json_of(&src.tags),
            json_of(&src.fields),
            json_of(&src.checklist.iter().map(|c| ChecklistItem { text: c.text.clone(), done: false }).collect::<Vec<_>>()),
            src.status.as_ref().map(|_| "open".to_string()),
            src.due_date,
            now
        ],
    )?;
    read_item(&conn, conn.last_insert_rowid())
}

// ---------------------------------------------------------------------------
// Search - one query over items AND tables
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceHit {
    /// note | record | task | table
    pub kind: String,
    /// The workspace item's id, or the table's id.
    pub id: i64,
    pub title: String,
    /// One line of context: the matching cell, field or excerpt.
    pub preview: String,
    /// Where it matched, in words - "Nick", "Content", "Oasis Codes".
    pub where_found: String,
}

fn excerpt(haystack: &str, needle: &str) -> String {
    let lower = haystack.to_lowercase();
    let Some(at) = lower.find(needle) else {
        return haystack.chars().take(90).collect();
    };
    // A window around the hit, so a match 400 characters into a note is
    // actually visible in the result rather than cut off before it.
    let start = haystack[..at].char_indices().rev().nth(30).map(|(i, _)| i).unwrap_or(0);
    let text: String = haystack[start..].chars().take(110).collect();
    if start > 0 {
        format!("…{text}")
    } else {
        text
    }
}

/// One search across every note, record, task and table. Case-insensitive
/// substring, on purpose: marko looks for a nick, a code or half a name, and
/// "ABC12" must find "ABC123" - which a word index would not.
#[tauri::command]
pub fn search_workspace(state: State<AppState>, query: String) -> AppResult<Vec<WorkspaceHit>> {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return Ok(Vec::new());
    }
    let conn = state.db.lock().unwrap();
    let mut hits: Vec<WorkspaceHit> = Vec::new();

    // Archived items are deliberately excluded - the brief: "Archived items
    // should not clutter normal search/results by default."
    let sql = format!("{SELECT_ITEM} WHERE archived = 0 ORDER BY pinned DESC, updated_at DESC");
    let mut stmt = conn.prepare(&sql)?;
    let items = stmt.query_map([], |r| row_to_item(r))?;
    for item in items {
        let item = item?;
        let title_l = item.title.to_lowercase();
        let content_l = item.content.to_lowercase();
        let label = if item.title.is_empty() { "Untitled".to_string() } else { item.title.clone() };

        if title_l.contains(&needle) {
            hits.push(WorkspaceHit {
                kind: item.kind.clone(),
                id: item.id,
                title: label.clone(),
                preview: excerpt(&item.content, &needle),
                where_found: "Title".to_string(),
            });
            continue;
        }
        if content_l.contains(&needle) {
            hits.push(WorkspaceHit {
                kind: item.kind.clone(),
                id: item.id,
                title: label.clone(),
                preview: excerpt(&item.content, &needle),
                where_found: "Content".to_string(),
            });
            continue;
        }
        // A field whose name looks like a secret can still be FOUND by its
        // name, but its value is never echoed into a result line.
        if let Some(f) = item
            .fields
            .iter()
            .find(|f| f.name.to_lowercase().contains(&needle) || f.value.to_lowercase().contains(&needle))
        {
            hits.push(WorkspaceHit {
                kind: item.kind.clone(),
                id: item.id,
                title: label.clone(),
                preview: if looks_secret(&f.name) { "••••••••".to_string() } else { f.value.clone() },
                where_found: f.name.clone(),
            });
            continue;
        }
        if item.tags.iter().any(|t| t.contains(&needle)) {
            hits.push(WorkspaceHit {
                kind: item.kind.clone(),
                id: item.id,
                title: label.clone(),
                preview: item.tags.iter().map(|t| format!("#{t}")).collect::<Vec<_>>().join(" "),
                where_found: "Tag".to_string(),
            });
        }
    }

    // ...and the tables from migration 031.
    let mut tstmt = conn.prepare(
        "SELECT r.cells_json, s.id, s.name, s.columns_json
         FROM note_rows r JOIN note_sheets s ON s.id = r.sheet_id
         WHERE s.archived = 0 ORDER BY s.position, s.id, r.position, r.id",
    )?;
    let rows = tstmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, i64>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
        ))
    })?;
    for row in rows {
        let (cells_json, sheet_id, sheet_name, columns_json) = row?;
        let columns: Vec<String> = from_json(&columns_json);
        let cells: Vec<String> = from_json(&cells_json);
        if let Some(idx) = cells.iter().position(|c| c.to_lowercase().contains(&needle)) {
            hits.push(WorkspaceHit {
                kind: "table".to_string(),
                id: sheet_id,
                title: sheet_name,
                preview: cells.iter().filter(|c| !c.is_empty()).cloned().collect::<Vec<_>>().join(" · "),
                where_found: columns.get(idx).cloned().unwrap_or_else(|| "Cell".to_string()),
            });
        }
    }

    Ok(hits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_are_normalised_deduped_and_stripped_of_the_hash() {
        let got = clean_tags(&["#Oasis".into(), "oasis ".into(), "  ".into(), "FollowUp".into()]);
        assert_eq!(got, vec!["oasis".to_string(), "followup".to_string()]);
    }

    #[test]
    fn a_field_named_like_a_secret_is_recognised_whatever_the_casing() {
        assert!(looks_secret("Password"));
        assert!(looks_secret("heslo k uctu"));
        assert!(looks_secret("API key"));
        assert!(!looks_secret("Nick"));
        assert!(!looks_secret("Email"));
    }

    #[test]
    fn an_excerpt_is_taken_around_the_hit_not_from_the_start() {
        let long = format!("{}John123 bought 4 codes", "x".repeat(300));
        let got = excerpt(&long, "john123");
        assert!(got.starts_with('…'), "a match past the start is windowed: {got}");
        assert!(got.to_lowercase().contains("john123"), "the hit itself must be visible: {got}");
    }

    #[test]
    fn an_excerpt_of_a_short_note_is_not_truncated_or_prefixed() {
        assert_eq!(excerpt("John123 bought 4 codes", "john123"), "John123 bought 4 codes");
    }

    #[test]
    fn unreadable_stored_json_reads_as_the_empty_default() {
        let tags: Vec<String> = from_json("definitely not json");
        assert!(tags.is_empty());
    }
}

//! Sheet alerts - a reminder attached to a sheet, at a time.
//!
//! 2.55.0, from marko: "nejake alert by som si tam chcel nastavit casovo a
//! tak". See migration 033 for why this is its own table and why `notified` is
//! separate from `done`.
//!
//! ## Time here is LOCAL wall-clock, not UTC
//!
//! Everything else in this app stores UTC (`now_iso()`), and this deliberately
//! does not. "Remind me at 9:00" means nine o'clock where marko is, and he
//! types it into a `<input type="datetime-local">` which hands over exactly
//! `YYYY-MM-DDTHH:MM` with no zone. Converting that to UTC and back would let
//! a reminder set on the Mac fire an hour early on the PC after a DST change,
//! for no gain: he is one person in one place.
//!
//! So the due test is `remind_at <= strftime(... ,'now','localtime')` and the
//! string sorts correctly because the format is fixed-width.
//!
//! ## What this module does NOT do
//!
//! It does not poll, schedule or run anything in the background. `check_due`
//! is a plain command the UI calls on a timer while the app is open; nothing
//! fires when the app is closed, and the UI is told so.

use crate::commands::cloud_sync::now_iso;
use crate::db::AppState;
use crate::error::{AppError, AppResult};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SheetAlert {
    pub id: i64,
    pub sheet_id: i64,
    pub sheet_name: String,
    /// Both optional: an alert can point at a cell, at a row, or at neither.
    pub row_id: Option<i64>,
    pub col_index: Option<i64>,
    pub title: String,
    pub note: String,
    /// Local wall-clock, `YYYY-MM-DDTHH:MM`.
    pub remind_at: String,
    pub done: bool,
    pub notified: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// What the UI sends. `id` absent means create.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SheetAlertInput {
    pub id: Option<i64>,
    pub sheet_id: i64,
    pub row_id: Option<i64>,
    pub col_index: Option<i64>,
    pub title: String,
    pub note: String,
    pub remind_at: String,
}

const SELECT_ALERT: &str = "SELECT a.id, a.sheet_id, COALESCE(s.name,''), a.row_id, a.col_index, \
     a.title, a.note, a.remind_at, a.done, a.notified, a.created_at, a.updated_at \
     FROM sheet_alerts a LEFT JOIN note_sheets s ON s.id = a.sheet_id";

fn row_to_alert(r: &rusqlite::Row<'_>) -> rusqlite::Result<SheetAlert> {
    Ok(SheetAlert {
        id: r.get(0)?,
        sheet_id: r.get(1)?,
        sheet_name: r.get(2)?,
        row_id: r.get(3)?,
        col_index: r.get(4)?,
        title: r.get(5)?,
        note: r.get(6)?,
        remind_at: r.get(7)?,
        done: r.get::<_, i64>(8)? != 0,
        notified: r.get::<_, i64>(9)? != 0,
        created_at: r.get(10)?,
        updated_at: r.get(11)?,
    })
}

fn read_alert(conn: &Connection, id: i64) -> AppResult<SheetAlert> {
    let sql = format!("{SELECT_ALERT} WHERE a.id = ?1");
    Ok(conn.query_row(&sql, [id], |r| row_to_alert(r))?)
}

/// `2026-09-28T09:00` - exactly what `<input type="datetime-local">` produces.
/// Anything else is refused rather than stored and silently never firing.
fn clean_when(raw: &str) -> AppResult<String> {
    let t = raw.trim();
    let b = t.as_bytes();
    let digits = |from: usize, to: usize| b[from..to].iter().all(u8::is_ascii_digit);
    let ok = b.len() >= 16
        && t.is_ascii()
        && b[4] == b'-'
        && b[7] == b'-'
        && b[10] == b'T'
        && b[13] == b':'
        && digits(0, 4)
        && digits(5, 7)
        && digits(8, 10)
        && digits(11, 13)
        && digits(14, 16);
    if !ok {
        return Err(AppError::Validation("Pick a date and a time for the reminder.".to_string()));
    }
    // Trimmed to the minute: seconds would make the stored value sort the same
    // but read back oddly in the editor, which round-trips this exact string.
    Ok(t[..16].to_string())
}

#[tauri::command]
pub fn list_sheet_alerts(state: State<AppState>, include_done: bool) -> AppResult<Vec<SheetAlert>> {
    let conn = state.db.lock().unwrap();
    let sql = if include_done {
        format!("{SELECT_ALERT} ORDER BY a.done, a.remind_at")
    } else {
        format!("{SELECT_ALERT} WHERE a.done = 0 ORDER BY a.remind_at")
    };
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], |r| row_to_alert(r))?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[tauri::command]
pub fn save_sheet_alert(state: State<AppState>, alert: SheetAlertInput) -> AppResult<SheetAlert> {
    let when = clean_when(&alert.remind_at)?;
    let title = alert.title.trim().to_string();
    if title.is_empty() {
        return Err(AppError::Validation("Give the reminder a title so it means something later.".to_string()));
    }
    let conn = state.db.lock().unwrap();
    match alert.id {
        Some(id) => {
            // Moving the time makes it fire again: a reminder pushed to next
            // week has plainly not been delivered for next week yet.
            conn.execute(
                "UPDATE sheet_alerts SET row_id = ?2, col_index = ?3, title = ?4, note = ?5, \
                 remind_at = ?6, notified = CASE WHEN remind_at = ?6 THEN notified ELSE 0 END, \
                 updated_at = ?7 WHERE id = ?1",
                rusqlite::params![id, alert.row_id, alert.col_index, title, alert.note, when, now_iso()],
            )?;
            read_alert(&conn, id)
        }
        None => {
            conn.execute(
                "INSERT INTO sheet_alerts(sheet_id, row_id, col_index, title, note, remind_at, created_at, updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
                rusqlite::params![alert.sheet_id, alert.row_id, alert.col_index, title, alert.note, when, now_iso()],
            )?;
            read_alert(&conn, conn.last_insert_rowid())
        }
    }
}

#[tauri::command]
pub fn set_sheet_alert_done(state: State<AppState>, id: i64, done: bool) -> AppResult<SheetAlert> {
    let conn = state.db.lock().unwrap();
    // Ticking it off also stops it shouting: an alert marked done that is still
    // overdue must not pop up again on the next tick.
    conn.execute(
        "UPDATE sheet_alerts SET done = ?2, notified = CASE WHEN ?2 = 1 THEN 1 ELSE notified END, \
         updated_at = ?3 WHERE id = ?1",
        rusqlite::params![id, if done { 1 } else { 0 }, now_iso()],
    )?;
    read_alert(&conn, id)
}

#[tauri::command]
pub fn delete_sheet_alert(state: State<AppState>, id: i64) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    conn.execute("DELETE FROM sheet_alerts WHERE id = ?1", [id])?;
    Ok(())
}

/// Alerts that have come due since the last check. Marks them notified and
/// shows a desktop notification for each, then hands them back so the UI can
/// say the same thing on screen.
///
/// Called by the UI on a timer while the app is open. It is NOT a scheduler:
/// with the app closed nothing fires, and an alert that came due meanwhile is
/// delivered on the next start, which is the honest behaviour for a local app.
#[tauri::command]
pub fn check_sheet_alerts(app: tauri::AppHandle, state: State<AppState>) -> AppResult<Vec<SheetAlert>> {
    let due: Vec<SheetAlert> = {
        let conn = state.db.lock().unwrap();
        let sql = format!(
            "{SELECT_ALERT} WHERE a.done = 0 AND a.notified = 0 \
             AND a.remind_at <= strftime('%Y-%m-%dT%H:%M','now','localtime') \
             ORDER BY a.remind_at"
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], |r| row_to_alert(r))?;
        let due = rows.collect::<Result<Vec<_>, _>>()?;
        // Marked BEFORE the notification goes out. Showing one twice is worse
        // than the rare case of one lost to a failing notification service -
        // and the alert itself stays on screen in the app either way.
        for a in &due {
            conn.execute(
                "UPDATE sheet_alerts SET notified = 1, updated_at = ?2 WHERE id = ?1",
                rusqlite::params![a.id, now_iso()],
            )?;
        }
        due
    };

    for a in &due {
        let body = if a.note.trim().is_empty() {
            a.sheet_name.clone()
        } else {
            format!("{} — {}", a.sheet_name, a.note.trim())
        };
        // A reminder that cannot be shown must not take the others down with
        // it, nor fail the whole command: they are already on screen in the app.
        let _ = crate::commands::notifications::send_desktop_notification(&app, &a.title, &body);
    }
    Ok(due)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_datetime_local_value_is_accepted_and_trimmed_to_the_minute() {
        assert_eq!(clean_when("2026-09-28T09:00").unwrap(), "2026-09-28T09:00");
        assert_eq!(clean_when("2026-09-28T09:00:31").unwrap(), "2026-09-28T09:00");
        assert_eq!(clean_when("  2026-09-28T09:00  ").unwrap(), "2026-09-28T09:00");
    }

    #[test]
    fn anything_that_would_never_fire_is_refused_rather_than_stored() {
        for bad in ["", "2026-09-28", "28/09/2026 09:00", "2026-09-28 09:00", "not a date at all"] {
            assert!(clean_when(bad).is_err(), "{bad} should be refused");
        }
    }

    #[test]
    fn the_fixed_width_format_sorts_chronologically_as_text() {
        let mut v = ["2026-10-01T08:00", "2026-09-28T09:00", "2026-09-28T08:59", "2026-09-09T23:00"];
        v.sort();
        assert_eq!(v, ["2026-09-09T23:00", "2026-09-28T08:59", "2026-09-28T09:00", "2026-10-01T08:00"]);
    }
}

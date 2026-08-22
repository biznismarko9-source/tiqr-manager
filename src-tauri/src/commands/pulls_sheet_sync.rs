//! Pulls <-> Google Sheet row sync (2.0.3) - the second half of Settings ->
//! Integrations, now that the connection itself (2.0.2) and the exact
//! column mapping (REDESIGN-2.0.2-REPORT.md section 5, confirmed by marko
//! in the follow-up conversation) are both settled.
//!
//! **Sheet -> app only in this pass.** A "Sync now" click reads every row
//! of the connected sheet and creates/updates matching Pulls in the app.
//! Pushing an app-side edit back out to the sheet is deliberately NOT built
//! yet - that is a separate, later step. What already exists as protection
//! against that gap: a genuine two-sided edit (both the sheet AND the app
//! changed the same linked pull since the last sync) is detected and
//! reported, never silently overwritten in either direction - see
//! `apply_pull_rows`'s doc comment.
//!
//! ## Column mapping (confirmed with marko)
//!
//! | Sheet column (header, case/spacing-insensitive) | Pull field |
//! |---|---|
//! | `pull` | `buyer_name` |
//! | `Event name` / `Event` | `event_name` |
//! | `event date` | `event_date` (`DD.MM.YYYY` or `DD.<sk month abbrev>.YYYY`) |
//! | `Ks` / `quantity` / `qty` | `quantity` (`"2x"` accepted) |
//! | `Platform` | `platform_id` (resolve-or-create by name, same as CSV import) |
//! | `More info` | `more_info` |
//! | `Section` / `Sector` | `section` (new column - marko's old sheet had none) |
//! | `Row` | `row_label` (new column) |
//! | `Seats` / `Seat` | `seat` |
//! | `Transfer` | `transfer_done` (Ano/Yes/true/1/done -> true, anything else -> false) |
//! | `Price` | `price_cents` - currency comes from the **connection**, not a column (marko's sheet has none; see `SheetsConnectionConfig::currency`) |
//! | `date` | ignored (marko: just when he logged the row, not modeled) |
//! | (unnamed blank column) | ignored (marko: no purpose) |
//! | `TIQR ID` (appended by the app itself the first time it's missing) | the sync marker - never typed by hand |
//!
//! `pull`/`Event name`/`Ks`/`Price` are required: a sheet missing any of
//! them fails the whole sync with one clear message up front, rather than
//! silently skipping every row. Every other column is optional - if absent,
//! that field is just blank for every row, so marko can add Section/Row
//! later without sync breaking in the meantime.
//!
//! ## Why best-effort per row, not all-or-nothing like CSV import
//!
//! CSV import is a one-off, reviewed-before-confirming action, so
//! all-or-nothing (`REDESIGN` house rule) is the right call there. A sync is
//! different: it is meant to be clicked repeatedly against a live,
//! continuously-edited sheet, so one malformed historical row (a typo, a
//! half-filled-in row) must never block every other valid row from
//! importing every single time - marko fixes that one row later and syncs
//! again, exactly like re-running any other sync tool. Every row is
//! therefore independent: a bad row is skipped and reported, good rows
//! still go through.

use crate::commands::csv_import::resolve_or_create_platform;
use crate::commands::pulls::{create_pull_impl, fetch_one as fetch_pull, set_pull_transfer_done_impl, update_pull_impl};
use crate::commands::sheets_sync::{
    last_pushed_key, last_synced_key, load_connection, set_setting, set_sheets_connection_impl, ALLOWED_CURRENCIES,
};
use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::google_sheets;
use crate::models::{CreatedSheetResult, Pull, PullEditInput, PullInput, SheetSyncIssue, SheetSyncResult};
use crate::money::{format_cents, parse_decimal_to_cents};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::State;

/// The header this module appends to the sheet (once) to hold its own
/// generated Pull code (e.g. "PULL-000001") per row - see
/// migrations/008_sheet_sync.sql's doc comment for why this must be a
/// dedicated, app-owned column rather than reusing an existing one.
const MARKER_HEADER: &str = "TIQR ID";

const REQUIRED_HEADERS: &[(&str, &[&str])] = &[
    ("\"pull\" (buyer)", &["pull"]),
    ("\"Event name\"", &["event name", "event"]),
    ("\"Ks\" (quantity)", &["ks", "quantity", "qty"]),
    ("\"Price\"", &["price"]),
];

// ---------------------------------------------------------------------------
// Header matching - case/spacing-insensitive, alias-tolerant, same spirit as
// csv_import.rs's `normalize`/`field` helpers (kept separate rather than
// shared: this reads `Vec<String>` sheet rows, not `csv::StringRecord`).
// ---------------------------------------------------------------------------

fn normalize_header(h: &str) -> String {
    h.trim().to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ")
}

fn build_header_map(headers: &[String]) -> HashMap<String, usize> {
    headers.iter().enumerate().map(|(i, h)| (normalize_header(h), i)).collect()
}

fn find_col(map: &HashMap<String, usize>, aliases: &[&str]) -> Option<usize> {
    aliases.iter().find_map(|a| map.get(*a).copied())
}

fn check_required_headers(map: &HashMap<String, usize>) -> AppResult<()> {
    let missing: Vec<&str> = REQUIRED_HEADERS
        .iter()
        .filter(|(_, aliases)| find_col(map, aliases).is_none())
        .map(|(label, _)| *label)
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(AppError::Validation(format!(
            "The connected sheet is missing required column(s): {}",
            missing.join(", ")
        )))
    }
}

/// Reads one cell, trimmed, treating blank as absent - the one place "is
/// this row short a trailing cell" (Sheets omits trailing empty cells per
/// row) and "is this cell just whitespace" both collapse to the same thing.
fn cell(row: &[String], idx: Option<usize>) -> Option<String> {
    let idx = idx?;
    let v = row.get(idx)?.trim();
    if v.is_empty() {
        None
    } else {
        Some(v.to_string())
    }
}

/// Finds the existing `TIQR ID` column, or - if this sheet has never been
/// synced before - decides where a new one will go (one past every existing
/// header). The second element is `false` exactly when the caller still
/// needs to write the header cell itself before any row data is touched.
fn resolve_marker_column(headers: &[String]) -> (usize, bool) {
    let map = build_header_map(headers);
    match map.get(&normalize_header(MARKER_HEADER)) {
        Some(&idx) => (idx, true),
        None => (headers.len(), false),
    }
}

/// 0 -> "A", 25 -> "Z", 26 -> "AA", ... - the standard bijective base-26
/// column-letter scheme Sheets' A1 notation uses.
fn column_index_to_a1(mut idx: usize) -> String {
    let mut letters = Vec::new();
    loop {
        let rem = idx % 26;
        letters.push((b'A' + rem as u8) as char);
        if idx < 26 {
            break;
        }
        idx = idx / 26 - 1;
    }
    letters.iter().rev().collect()
}

// ---------------------------------------------------------------------------
// Field parsers - each rejects clearly rather than guessing, same principle
// money.rs already holds for amounts.
// ---------------------------------------------------------------------------

/// "2x" / "2" -> 2. Anything else (an empty cell, "2 tickets", "0") is
/// reported rather than guessed at.
fn parse_quantity(raw: &str) -> Result<i64, String> {
    let s = raw.trim();
    if s.is_empty() {
        return Err("is empty".to_string());
    }
    let trailing = s.trim_start_matches(|c: char| c.is_ascii_digit());
    if !(trailing.is_empty() || trailing.eq_ignore_ascii_case("x")) {
        return Err(format!("'{s}' is not a recognized quantity (expected e.g. \"2\" or \"2x\")"));
    }
    let digits: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return Err(format!("'{s}' is not a recognized quantity"));
    }
    let n: i64 = digits.parse().map_err(|_| format!("'{s}' is not a recognized quantity"))?;
    if n <= 0 {
        return Err("quantity must be at least 1".to_string());
    }
    Ok(n)
}

/// Ano/Yes/true/1/done (diacritic- and case-insensitive) -> true; blank,
/// "Nie", or anything else unrecognized -> false. Never errors: this is a
/// status flag, not financial data, and a fresh pull already defaults to
/// false the same way when created by hand.
fn parse_transfer_done(raw: &str) -> bool {
    let s = strip_diacritics(&raw.trim().to_lowercase());
    matches!(s.as_str(), "ano" | "yes" | "y" | "true" | "1" | "done")
}

fn strip_diacritics(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'á' | 'ä' => 'a',
            'é' => 'e',
            'í' => 'i',
            'ó' | 'ô' => 'o',
            'ú' => 'u',
            'ý' => 'y',
            'č' => 'c',
            'š' => 's',
            'ž' => 'z',
            'ť' => 't',
            'ď' => 'd',
            'ň' => 'n',
            'ľ' | 'ĺ' => 'l',
            'ŕ' => 'r',
            other => other,
        })
        .collect()
}

fn month_from_abbrev(s: &str) -> Option<u32> {
    let normalized = strip_diacritics(&s.to_lowercase());
    let key: String = normalized.chars().take(3).collect();
    match key.as_str() {
        "jan" => Some(1),
        "feb" => Some(2),
        "mar" => Some(3),
        "apr" => Some(4),
        "maj" => Some(5),
        "jun" => Some(6),
        "jul" => Some(7),
        "aug" => Some(8),
        "sep" => Some(9),
        "okt" => Some(10),
        "nov" => Some(11),
        "dec" => Some(12),
        _ => None,
    }
}

/// Empty -> `Ok(None)`. Otherwise expects `DD.MM.YYYY` or
/// `DD.<sk month abbrev>.YYYY` (e.g. "15.05.2026" or "26.jan.2026" - both
/// seen in marko's real sheet), normalized to plain ISO `YYYY-MM-DD` for
/// storage. Rejects anything else, including a real-looking but
/// out-of-range year, rather than guessing.
fn parse_sheet_date(raw: &str) -> Result<Option<String>, String> {
    let s = raw.trim();
    if s.is_empty() {
        return Ok(None);
    }

    // A cell someone typed an actual date into (rather than typing the date
    // as plain text) comes back from Sheets as a bare serial-day number -
    // see google_sheets::ValueRange's doc comment for why the wire format
    // allows this at all. Recognized here, ahead of the DD.MM.YYYY text
    // parsing below, by "every character is a digit": a real DD.MM.YYYY (or
    // DD.<sk month>.YYYY) string always contains at least one '.', so there
    // is no overlap between the two forms.
    if s.chars().all(|c| c.is_ascii_digit()) {
        return parse_sheet_serial_date(s);
    }

    let parts: Vec<&str> = s.split('.').map(|p| p.trim()).collect();
    if parts.len() != 3 {
        return Err(format!("'{s}' is not a recognized date (expected DD.MM.YYYY or DD.Mon.YYYY)"));
    }
    let day: u32 = parts[0].parse().map_err(|_| format!("'{s}' has an invalid day"))?;
    let year: i32 = parts[2].parse().map_err(|_| format!("'{s}' has an invalid year"))?;
    if !(2000..=2100).contains(&year) {
        return Err(format!("'{s}' has an implausible year"));
    }
    let month: u32 = match parts[1].parse::<u32>() {
        Ok(m) => m,
        Err(_) => month_from_abbrev(parts[1])
            .ok_or_else(|| format!("'{s}' has an unrecognized month '{}'", parts[1]))?,
    };
    chrono::NaiveDate::from_ymd_opt(year, month, day)
        .map(|d| Some(d.format("%Y-%m-%d").to_string()))
        .ok_or_else(|| format!("'{s}' is not a valid calendar date"))
}

/// Converts a Google Sheets/Excel serial-date number (days since
/// 1899-12-30 - the long-standing Lotus-1-2-3-compatible epoch both Sheets
/// and Excel still use, deliberately including its famous "Feb 29 1900
/// never existed" quirk, which this epoch anchor already absorbs for every
/// real calendar date after that point) into the same `YYYY-MM-DD` shape
/// `parse_sheet_date` already returns for text input. Rejects the same
/// implausible range (year 2000-2100) as the text path and for the same
/// reason: a bare number in the event-date column is almost certainly a
/// real date, but a wildly out-of-range one reads more like a mistake (a
/// stray row number, a quantity typed in the wrong column) than an
/// intentional 15th- or 22nd-century pull.
///
/// `pub(crate)` (not just `fn`) since 2.0.8: commands::orders_sheet_sync
/// reuses this exact same conversion for its own "Date (DD/MM/YYYY)" column,
/// which hits the identical Sheets serial-number wire format whenever
/// someone types an actual date rather than plain text - no reason to
/// duplicate the epoch/range logic a second time for the same underlying
/// fact.
pub(crate) fn parse_sheet_serial_date(digits: &str) -> Result<Option<String>, String> {
    let serial: i64 = digits.parse().map_err(|_| format!("'{digits}' is not a recognized date"))?;
    let epoch = chrono::NaiveDate::from_ymd_opt(1899, 12, 30).expect("1899-12-30 is a valid calendar date");
    let date = epoch
        .checked_add_signed(chrono::Duration::days(serial))
        .ok_or_else(|| format!("'{digits}' is not a recognized date"))?;
    let min_date = chrono::NaiveDate::from_ymd_opt(2000, 1, 1).expect("2000-01-01 is a valid calendar date");
    let max_date = chrono::NaiveDate::from_ymd_opt(2100, 12, 31).expect("2100-12-31 is a valid calendar date");
    if date < min_date || date > max_date {
        return Err(format!("'{digits}' has an implausible year"));
    }
    Ok(Some(date.format("%Y-%m-%d").to_string()))
}

fn now_iso(conn: &Connection) -> AppResult<String> {
    Ok(conn.query_row("SELECT strftime('%Y-%m-%dT%H:%M:%fZ','now')", [], |r| r.get(0))?)
}

// ---------------------------------------------------------------------------
// Row snapshotting - the subset of a Pull's fields that came from the sheet,
// stored right after a successful create/update. Comparing this against a
// freshly parsed row tells "the sheet changed this row" apart from
// "nothing changed", and comparing the *local pull's* `updated_at` against
// the link's `last_synced_at` tells "only the sheet changed it" apart from
// "the app changed it too" (a genuine conflict) - see
// migrations/008_sheet_sync.sql's doc comment for why this exists at all.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct PullRowSnapshot {
    buyer_name: String,
    event_name: String,
    event_date: Option<String>,
    quantity: i64,
    /// The platform *name*, not its id, deliberately: comparing the name the
    /// sheet showed is what tells "the sheet changed this" apart from "marko
    /// renamed the platform in Settings", which is not a sheet-side change.
    platform_name: Option<String>,
    section: Option<String>,
    row_label: Option<String>,
    seat: Option<String>,
    more_info: Option<String>,
    transfer_done: bool,
    price_cents: i64,
    currency: String,
}

struct ParsedRow {
    buyer_name: String,
    event_name: String,
    event_date: Option<String>,
    quantity: i64,
    platform_name: Option<String>,
    section: Option<String>,
    row_label: Option<String>,
    seat: Option<String>,
    more_info: Option<String>,
    transfer_done: bool,
    price_cents: i64,
}

impl ParsedRow {
    fn snapshot(&self, currency: &str) -> PullRowSnapshot {
        PullRowSnapshot {
            buyer_name: self.buyer_name.clone(),
            event_name: self.event_name.clone(),
            event_date: self.event_date.clone(),
            quantity: self.quantity,
            platform_name: self.platform_name.clone(),
            section: self.section.clone(),
            row_label: self.row_label.clone(),
            seat: self.seat.clone(),
            more_info: self.more_info.clone(),
            transfer_done: self.transfer_done,
            price_cents: self.price_cents,
            currency: currency.to_string(),
        }
    }
}

struct SyncLink {
    local_id: i64,
    /// Needed by the push direction (`load_sync_link_by_local_id`) to know
    /// which marker to look for in the sheet's *current* data - the
    /// sheet -> app lookup (`load_sync_link` below) already has the marker
    /// as its input, so it never needed this field until now.
    sheet_marker: String,
    last_synced_snapshot: String,
    last_synced_at: String,
}

fn load_sync_link(conn: &Connection, marker: &str) -> AppResult<Option<SyncLink>> {
    Ok(conn
        .query_row(
            "SELECT local_id, sheet_marker, last_synced_snapshot, last_synced_at FROM sheet_sync_links
             WHERE data_source = 'pulls' AND sheet_marker = ?1",
            params![marker],
            |r| {
                Ok(SyncLink {
                    local_id: r.get(0)?,
                    sheet_marker: r.get(1)?,
                    last_synced_snapshot: r.get(2)?,
                    last_synced_at: r.get(3)?,
                })
            },
        )
        .optional()?)
}

/// The push direction's counterpart to `load_sync_link` above: same table,
/// looked up by which local pull it belongs to instead of by sheet marker,
/// since a push starts from the app's own pulls rather than from sheet rows.
fn load_sync_link_by_local_id(conn: &Connection, local_id: i64) -> AppResult<Option<SyncLink>> {
    Ok(conn
        .query_row(
            "SELECT local_id, sheet_marker, last_synced_snapshot, last_synced_at FROM sheet_sync_links
             WHERE data_source = 'pulls' AND local_id = ?1",
            params![local_id],
            |r| {
                Ok(SyncLink {
                    local_id: r.get(0)?,
                    sheet_marker: r.get(1)?,
                    last_synced_snapshot: r.get(2)?,
                    last_synced_at: r.get(3)?,
                })
            },
        )
        .optional()?)
}

/// Parses one raw sheet row into every Pull-shaped field this sync
/// understands, purely from already-fetched cells - no DB access, no
/// platform resolution (the caller decides what to do with the returned
/// `platform_name`, since `apply_pull_rows` may create a new platform from it
/// while the push direction's conflict check only ever needs the name itself
/// to compare against a stored snapshot). `Ok(None)` for a fully blank row
/// (nothing in any required column) - just a gap in the sheet, not a
/// mistake. `Err` carries one joined message covering every problem found in
/// the row, so a single bad row is reported once rather than piecemeal.
fn parse_pull_row(map: &HashMap<String, usize>, raw_row: &[String]) -> Result<Option<ParsedRow>, String> {
    let buyer_col = find_col(map, &["pull"]);
    let event_name_col = find_col(map, &["event name", "event"]);
    let event_date_col = find_col(map, &["event date"]);
    let quantity_col = find_col(map, &["ks", "quantity", "qty"]);
    let platform_col = find_col(map, &["platform"]);
    let more_info_col = find_col(map, &["more info"]);
    let section_col = find_col(map, &["section", "sector"]);
    let row_label_col = find_col(map, &["row"]);
    let seat_col = find_col(map, &["seats", "seat"]);
    let transfer_col = find_col(map, &["transfer"]);
    let price_col = find_col(map, &["price"]);

    let buyer_raw = cell(raw_row, buyer_col);
    let event_name_raw = cell(raw_row, event_name_col);
    let quantity_raw = cell(raw_row, quantity_col);
    let price_raw = cell(raw_row, price_col);

    // A fully blank row (nothing in any required column) is just a gap in
    // the sheet, not a mistake - skip it without comment.
    if buyer_raw.is_none() && event_name_raw.is_none() && quantity_raw.is_none() && price_raw.is_none() {
        return Ok(None);
    }

    let mut row_errors: Vec<String> = vec![];

    let buyer_name = buyer_raw.unwrap_or_default();
    if buyer_name.is_empty() {
        row_errors.push("missing 'pull' (buyer) value".to_string());
    }
    let event_name = event_name_raw.unwrap_or_default();
    if event_name.is_empty() {
        row_errors.push("missing 'Event name' value".to_string());
    }
    let quantity = match quantity_raw.as_deref().map(parse_quantity) {
        Some(Ok(q)) => q,
        Some(Err(e)) => {
            row_errors.push(format!("'Ks': {e}"));
            0
        }
        None => {
            row_errors.push("missing 'Ks' value".to_string());
            0
        }
    };
    let price_cents = match price_raw.as_deref().map(parse_decimal_to_cents) {
        Some(Ok(v)) if v >= 0 => v,
        Some(Ok(_)) => {
            row_errors.push("'Price' cannot be negative".to_string());
            0
        }
        Some(Err(e)) => {
            row_errors.push(format!("'Price': {e}"));
            0
        }
        None => {
            row_errors.push("missing 'Price' value".to_string());
            0
        }
    };
    let event_date = match cell(raw_row, event_date_col).as_deref().map(parse_sheet_date) {
        Some(Ok(d)) => d,
        Some(Err(e)) => {
            row_errors.push(format!("'event date': {e}"));
            None
        }
        None => None,
    };

    if !row_errors.is_empty() {
        return Err(row_errors.join("; "));
    }

    let transfer_done = cell(raw_row, transfer_col).map(|s| parse_transfer_done(&s)).unwrap_or(false);
    let platform_name = cell(raw_row, platform_col);
    let more_info = cell(raw_row, more_info_col);
    let section = cell(raw_row, section_col);
    let row_label = cell(raw_row, row_label_col);
    let seat = cell(raw_row, seat_col);

    Ok(Some(ParsedRow {
        buyer_name,
        event_name,
        event_date,
        quantity,
        platform_name,
        section,
        row_label,
        seat,
        more_info,
        transfer_done,
        price_cents,
    }))
}

// ---------------------------------------------------------------------------
// The core - no network call anywhere in this function, which is what makes
// it directly unit-testable with a plain in-memory `test_conn()`. The
// network-calling `sync_pulls_impl` below fetches the rows via
// `google_sheets::get_values`, then calls this.
// ---------------------------------------------------------------------------

/// Applies already-fetched sheet rows to the `pulls` table. Returns the
/// user-facing result summary plus the list of (0-based data row index,
/// marker value to write) pairs the caller still needs to write back to the
/// actual sheet - this function itself never talks to Google.
fn apply_pull_rows(
    conn: &Connection,
    headers: &[String],
    data_rows: &[Vec<String>],
    currency: &str,
    marker_col_index: usize,
) -> AppResult<(SheetSyncResult, Vec<(usize, String)>)> {
    let map = build_header_map(headers);
    check_required_headers(&map)?;

    let mut result = SheetSyncResult {
        created: 0,
        updated: 0,
        unchanged: 0,
        conflicts: vec![],
        errors: vec![],
        synced_at: String::new(),
    };
    let mut marker_writes = vec![];

    for (i, raw_row) in data_rows.iter().enumerate() {
        let row_number = (i + 2) as i64; // header is sheet row 1

        let parsed = match parse_pull_row(&map, raw_row) {
            Ok(None) => continue,
            Ok(Some(p)) => p,
            Err(msg) => {
                result.errors.push(SheetSyncIssue { row_number, message: msg });
                continue;
            }
        };

        let platform_id = match &parsed.platform_name {
            Some(name) => match resolve_or_create_platform(conn, name) {
                Ok(id) => Some(id),
                Err(e) => {
                    result.errors.push(SheetSyncIssue { row_number, message: format!("platform '{name}': {e}") });
                    continue;
                }
            },
            None => None,
        };

        let snapshot = parsed.snapshot(currency);
        let marker = cell(raw_row, Some(marker_col_index));

        match marker {
            None => {
                let input = PullInput {
                    buyer_name: parsed.buyer_name.clone(),
                    event_name: parsed.event_name.clone(),
                    event_date: parsed.event_date.clone(),
                    quantity: parsed.quantity,
                    platform_id,
                    section: parsed.section.clone(),
                    row_label: parsed.row_label.clone(),
                    seat: parsed.seat.clone(),
                    more_info: parsed.more_info.clone(),
                    price_cents: parsed.price_cents,
                    currency: currency.to_string(),
                };
                match create_pull_impl(conn, &input, false) {
                    Ok(pull) => {
                        // create_pull_impl always starts transfer_done=false
                        // (same rule the manual "New pull" form follows) -
                        // flip it through the one function that already owns
                        // transfer_done_at's timestamp rule, rather than a
                        // second, competing one invented here.
                        if parsed.transfer_done {
                            set_pull_transfer_done_impl(conn, pull.id, true)?;
                        }
                        let snapshot_json =
                            serde_json::to_string(&snapshot).map_err(|e| AppError::Other(e.to_string()))?;
                        let now = now_iso(conn)?;
                        conn.execute(
                            "INSERT INTO sheet_sync_links (data_source, local_id, sheet_marker, last_synced_snapshot, last_synced_at)
                             VALUES ('pulls', ?1, ?2, ?3, ?4)",
                            params![pull.id, pull.code, snapshot_json, now],
                        )?;
                        result.created += 1;
                        marker_writes.push((i, pull.code.clone()));
                    }
                    Err(e) => {
                        result.errors.push(SheetSyncIssue { row_number, message: e.to_string() });
                    }
                }
            }
            Some(marker_value) => {
                let Some(link) = load_sync_link(conn, &marker_value)? else {
                    result.errors.push(SheetSyncIssue {
                        row_number,
                        message: format!(
                            "column \"{MARKER_HEADER}\" has an unrecognized value '{marker_value}' - clear the cell if this should become a new pull"
                        ),
                    });
                    continue;
                };
                let Ok(stored_snapshot) = serde_json::from_str::<PullRowSnapshot>(&link.last_synced_snapshot) else {
                    result.errors.push(SheetSyncIssue {
                        row_number,
                        message: "this row's saved sync data is unreadable - disconnect and reconnect the sheet to reset it".to_string(),
                    });
                    continue;
                };
                if stored_snapshot == snapshot {
                    result.unchanged += 1;
                    continue;
                }
                let pull = match fetch_pull(conn, link.local_id) {
                    Ok(p) => p,
                    Err(_) => {
                        result.errors.push(SheetSyncIssue {
                            row_number,
                            message: format!("linked pull #{} no longer exists in the app", link.local_id),
                        });
                        continue;
                    }
                };
                if pull.updated_at > link.last_synced_at {
                    result.conflicts.push(SheetSyncIssue {
                        row_number,
                        message: format!(
                            "both the sheet and the app changed this pull ({}) since the last sync - resolve manually, then sync again",
                            pull.code
                        ),
                    });
                    continue;
                }
                let edit = PullEditInput {
                    buyer_name: parsed.buyer_name.clone(),
                    event_name: parsed.event_name.clone(),
                    event_date: parsed.event_date.clone(),
                    quantity: parsed.quantity,
                    platform_id,
                    section: parsed.section.clone(),
                    row_label: parsed.row_label.clone(),
                    seat: parsed.seat.clone(),
                    more_info: parsed.more_info.clone(),
                    price_cents: parsed.price_cents,
                    currency: currency.to_string(),
                    transfer_done: parsed.transfer_done,
                };
                match update_pull_impl(conn, link.local_id, &edit) {
                    Ok(_) => {
                        let snapshot_json =
                            serde_json::to_string(&snapshot).map_err(|e| AppError::Other(e.to_string()))?;
                        let now = now_iso(conn)?;
                        conn.execute(
                            "UPDATE sheet_sync_links SET last_synced_snapshot = ?1, last_synced_at = ?2
                             WHERE data_source = 'pulls' AND local_id = ?3",
                            params![snapshot_json, now, link.local_id],
                        )?;
                        result.updated += 1;
                    }
                    Err(e) => {
                        result.errors.push(SheetSyncIssue { row_number, message: e.to_string() });
                    }
                }
            }
        }
    }

    Ok((result, marker_writes))
}

// ---------------------------------------------------------------------------
// Push (app -> sheet), 2.0.18. Reuses every parsing/matching primitive above
// (`parse_pull_row`, `PullRowSnapshot`, `sheet_sync_links`) rather than a
// second copy of any of it - the only genuinely new ideas here are "which
// direction does a cell's value flow" and "what does a brand-new local-only
// pull look like as a sheet row". Same non-negotiable safety rule as the
// sheet -> app direction: a row this can't confidently reconcile is reported
// and left alone, never guessed at or silently overwritten either way.
// ---------------------------------------------------------------------------

/// The mirror of `ParsedRow::snapshot` - the same comparable shape, built
/// from the *local* Pull's current fields instead of a freshly parsed sheet
/// row. `Pull` already carries every field `PullRowSnapshot` needs, named
/// and shaped identically, since both ultimately describe the same pull.
fn pull_to_snapshot(pull: &Pull) -> PullRowSnapshot {
    PullRowSnapshot {
        buyer_name: pull.buyer_name.clone(),
        event_name: pull.event_name.clone(),
        event_date: pull.event_date.clone(),
        quantity: pull.quantity,
        platform_name: pull.platform_name.clone(),
        section: pull.section.clone(),
        row_label: pull.row_label.clone(),
        seat: pull.seat.clone(),
        more_info: pull.more_info.clone(),
        transfer_done: pull.transfer_done,
        price_cents: pull.price_cents,
        currency: pull.currency.clone(),
    }
}

/// The inverse of `parse_sheet_date` for plain `DD.MM.YYYY` text (never the
/// `Mon`-abbreviation or serial-number variants - those are only ever things
/// this app *reads*, marko's own typed shorthand, not something it should
/// start writing back out): turns a stored `YYYY-MM-DD` event date back into
/// the sheet's own display format. Falls back to the raw value unchanged if
/// it isn't actually `YYYY-MM-DD` (should never happen - every
/// `Pull.event_date` was itself produced by `parse_sheet_date` or this same
/// app's own date picker, both of which only ever emit ISO - but this is
/// data heading out to marko's real spreadsheet, so guessing is never worth
/// the risk of silently writing something wrong).
fn format_date_for_sheet(iso: &str) -> String {
    match chrono::NaiveDate::parse_from_str(iso, "%Y-%m-%d") {
        Ok(d) => d.format("%d.%m.%Y").to_string(),
        Err(_) => iso.to_string(),
    }
}

/// Every (column index, value) pair worth pushing for `pull`, at whichever
/// columns this sheet actually has (tolerant of a reordered/narrower sheet,
/// same as the read direction's `find_col` calls). Deliberately per-cell
/// rather than one contiguous range write: marko's real sheet may hold
/// columns this app has never heard of, and a per-cell write can never
/// clobber one by accident. Never includes the marker ("TIQR ID") column -
/// that is written once, on creation, and never touched again.
fn pull_push_cells(map: &HashMap<String, usize>, pull: &Pull) -> Vec<(usize, String)> {
    let mut cells = vec![];
    if let Some(c) = find_col(map, &["pull"]) {
        cells.push((c, pull.buyer_name.clone()));
    }
    if let Some(c) = find_col(map, &["event name", "event"]) {
        cells.push((c, pull.event_name.clone()));
    }
    if let Some(c) = find_col(map, &["event date"]) {
        cells.push((c, pull.event_date.as_deref().map(format_date_for_sheet).unwrap_or_default()));
    }
    if let Some(c) = find_col(map, &["ks", "quantity", "qty"]) {
        cells.push((c, pull.quantity.to_string()));
    }
    if let Some(c) = find_col(map, &["platform"]) {
        cells.push((c, pull.platform_name.clone().unwrap_or_default()));
    }
    if let Some(c) = find_col(map, &["more info"]) {
        cells.push((c, pull.more_info.clone().unwrap_or_default()));
    }
    if let Some(c) = find_col(map, &["section", "sector"]) {
        cells.push((c, pull.section.clone().unwrap_or_default()));
    }
    if let Some(c) = find_col(map, &["row"]) {
        cells.push((c, pull.row_label.clone().unwrap_or_default()));
    }
    if let Some(c) = find_col(map, &["seats", "seat"]) {
        cells.push((c, pull.seat.clone().unwrap_or_default()));
    }
    if let Some(c) = find_col(map, &["transfer"]) {
        cells.push((c, if pull.transfer_done { "Yes".to_string() } else { "No".to_string() }));
    }
    if let Some(c) = find_col(map, &["price"]) {
        cells.push((c, format_cents(pull.price_cents)));
    }
    cells
}

/// Lays `pull_push_cells` out as a full positional row (plus its marker
/// cell) ready for `append_values`, which - unlike `update_values` - writes
/// whichever columns you give it starting at column A, so a brand-new row
/// needs every in-between column accounted for (blank string where this
/// sheet has a column the app doesn't write to, e.g. marko's own "date").
fn build_pull_append_row(map: &HashMap<String, usize>, marker_col_index: usize, header_count: usize, pull: &Pull) -> Vec<String> {
    let mut cells = pull_push_cells(map, pull);
    cells.push((marker_col_index, pull.code.clone()));
    let width = cells.iter().map(|(i, _)| i + 1).max().unwrap_or(0).max(header_count);
    let mut row = vec![String::new(); width];
    for (i, v) in cells {
        row[i] = v;
    }
    row
}

/// One pending write this run wants to make to the actual sheet - kept as
/// data rather than performed inline in `apply_pull_push`, same "no network
/// call in the pure core" split every other _impl/apply_* pair in this
/// module already follows.
#[derive(Debug)]
enum PullPushWrite {
    /// A brand-new local-only pull, laid out as one full sheet row
    /// (`build_pull_append_row`) ready to hand to `append_values` as-is.
    Append(Vec<String>),
    /// A change to an already-linked pull's row. `sheet_row_number` is the
    /// real 1-based sheet row (header is row 1); `cells` are exactly the
    /// columns that changed, at their real column index - never a
    /// contiguous range, for the same reason `pull_push_cells` is per-cell.
    Update { sheet_row_number: i64, cells: Vec<(usize, String)> },
}

/// The push direction's own core, mirroring `apply_pull_rows` exactly in
/// spirit but walking the *app's* pulls instead of the sheet's rows. For
/// every non-demo pull (demo/seed data must never reach marko's real
/// sheet):
///
/// - never linked yet -> a brand-new row, appended (marko's own choice - see
///   REDESIGN-2.0.18-REPORT.md);
/// - linked, but its marker is nowhere in the sheet's current data -> its
///   row was deleted from the sheet since linking; reported as an error,
///   never silently re-appended (would create a duplicate);
/// - linked, sheet row found, but it no longer matches what was stored at
///   the last sync -> the sheet changed it since then; this run only ever
///   pushes, it never *pulls*, so this is reported as a conflict asking
///   marko to run "Sync from sheet" first, and nothing is pushed;
/// - linked, sheet row matches what was stored, and the local pull also
///   still matches -> nothing changed on either side, skipped as
///   `unchanged`;
/// - linked, sheet row matches what was stored, but the local pull no
///   longer does -> exactly the case this feature exists for: push the
///   local values out, per cell, and advance the link's stored snapshot.
fn apply_pull_push(
    conn: &Connection,
    headers: &[String],
    data_rows: &[Vec<String>],
    connection_currency: &str,
    marker_col_index: usize,
) -> AppResult<(SheetSyncResult, Vec<PullPushWrite>)> {
    let map = build_header_map(headers);
    check_required_headers(&map)?;

    let mut marker_row_map: HashMap<String, usize> = HashMap::new();
    for (i, raw_row) in data_rows.iter().enumerate() {
        if let Some(marker) = cell(raw_row, Some(marker_col_index)) {
            marker_row_map.insert(marker, i);
        }
    }

    let mut result = SheetSyncResult {
        created: 0,
        updated: 0,
        unchanged: 0,
        conflicts: vec![],
        errors: vec![],
        synced_at: String::new(),
    };
    let mut writes = vec![];

    let pull_ids: Vec<i64> = {
        let mut stmt = conn.prepare("SELECT id FROM pulls WHERE is_demo = 0 ORDER BY id")?;
        let ids = stmt.query_map([], |r| r.get(0))?.collect::<Result<Vec<_>, _>>()?;
        ids
    };

    for local_id in pull_ids {
        // The id came from a fresh scan of this same table a moment ago, on
        // this same (mutex-held, single-threaded) connection - it cannot
        // realistically be gone already. Skip rather than abort the whole
        // run in the one case that would somehow still happen.
        let pull = match fetch_pull(conn, local_id) {
            Ok(p) => p,
            Err(_) => continue,
        };

        let Some(link) = load_sync_link_by_local_id(conn, local_id)? else {
            let snapshot = pull_to_snapshot(&pull);
            let snapshot_json = serde_json::to_string(&snapshot).map_err(|e| AppError::Other(e.to_string()))?;
            let now = now_iso(conn)?;
            conn.execute(
                "INSERT INTO sheet_sync_links (data_source, local_id, sheet_marker, last_synced_snapshot, last_synced_at)
                 VALUES ('pulls', ?1, ?2, ?3, ?4)",
                params![pull.id, pull.code, snapshot_json, now],
            )?;
            let row = build_pull_append_row(&map, marker_col_index, headers.len(), &pull);
            writes.push(PullPushWrite::Append(row));
            result.created += 1;
            continue;
        };

        let Some(&row_idx) = marker_row_map.get(&link.sheet_marker) else {
            result.errors.push(SheetSyncIssue {
                row_number: 0,
                message: format!(
                    "pull {}: its row in the sheet could not be found (\"{MARKER_HEADER}\" = \"{}\" is missing) - nothing was pushed for it. If that row was deleted on purpose, no action is needed; otherwise add it back to the sheet with the same {MARKER_HEADER}.",
                    pull.code, link.sheet_marker
                ),
            });
            continue;
        };
        let row_number = (row_idx + 2) as i64;

        let Ok(stored_snapshot) = serde_json::from_str::<PullRowSnapshot>(&link.last_synced_snapshot) else {
            result.errors.push(SheetSyncIssue {
                row_number,
                message: "this row's saved sync data is unreadable - disconnect and reconnect the sheet to reset it".to_string(),
            });
            continue;
        };

        let raw_row = &data_rows[row_idx];
        let current_sheet_snapshot = match parse_pull_row(&map, raw_row) {
            Ok(Some(parsed)) => Some(parsed.snapshot(connection_currency)),
            Ok(None) | Err(_) => None,
        };
        if current_sheet_snapshot.as_ref() != Some(&stored_snapshot) {
            result.conflicts.push(SheetSyncIssue {
                row_number,
                message: format!(
                    "the sheet changed this row ({}) since the last sync - run \"Sync from sheet\" first, then push again",
                    pull.code
                ),
            });
            continue;
        }

        let local_snapshot = pull_to_snapshot(&pull);
        if local_snapshot == stored_snapshot {
            result.unchanged += 1;
            continue;
        }

        let cells = pull_push_cells(&map, &pull);
        writes.push(PullPushWrite::Update { sheet_row_number: row_number, cells });
        let snapshot_json = serde_json::to_string(&local_snapshot).map_err(|e| AppError::Other(e.to_string()))?;
        let now = now_iso(conn)?;
        conn.execute(
            "UPDATE sheet_sync_links SET last_synced_snapshot = ?1, last_synced_at = ?2
             WHERE data_source = 'pulls' AND local_id = ?3",
            params![snapshot_json, now, pull.id],
        )?;
        result.updated += 1;
    }

    Ok((result, writes))
}

// ---------------------------------------------------------------------------
// The network-calling shell - fetches the sheet, calls apply_pull_rows
// above, writes back whatever markers it asked for. See google_sheets.rs's
// module doc comment for why this half can't be exercised in this sandbox.
// ---------------------------------------------------------------------------

fn sync_pulls_impl(conn: &Connection) -> AppResult<SheetSyncResult> {
    let connection = load_connection(conn, "pulls")?
        .ok_or_else(|| AppError::Validation("No spreadsheet is connected for Pulls yet - connect one in Settings first.".to_string()))?;
    // 2.0.5: the signed-in person's own OAuth token when there is one, the
    // shared service account otherwise - see
    // commands::google_auth::resolve_google_credential's doc comment. Either
    // way this is just a bearer token from here on; get_values/update_values
    // below do not need to know or care which kind it is.
    let credential = crate::commands::google_auth::resolve_google_credential(conn, false)?;
    let token = credential.access_token();

    let range = google_sheets::a1_range(&connection.sheet_tab, "A1:Z");
    let value_range = google_sheets::get_values(token, &connection.spreadsheet_id, &range)?;
    if value_range.values.is_empty() {
        return Err(AppError::Validation("The connected sheet/tab has no header row yet.".to_string()));
    }
    let headers = value_range.values[0].clone();
    let data_rows: &[Vec<String>] = if value_range.values.len() > 1 { &value_range.values[1..] } else { &[] };

    let (marker_col_index, marker_exists) = resolve_marker_column(&headers);
    let letter = column_index_to_a1(marker_col_index);
    if !marker_exists {
        let header_range = google_sheets::a1_range(&connection.sheet_tab, &format!("{letter}1"));
        google_sheets::update_values(token, &connection.spreadsheet_id, &header_range, &[vec![MARKER_HEADER.to_string()]])?;
    }

    let (mut result, marker_writes) = apply_pull_rows(conn, &headers, data_rows, &connection.currency, marker_col_index)?;

    for (row_idx, marker_value) in marker_writes {
        let sheet_row_number = (row_idx + 2) as i64;
        let cell_range = google_sheets::a1_range(&connection.sheet_tab, &format!("{letter}{sheet_row_number}"));
        if let Err(e) = google_sheets::update_values(token, &connection.spreadsheet_id, &cell_range, &[vec![marker_value]]) {
            result.errors.push(SheetSyncIssue {
                row_number: sheet_row_number,
                message: format!("saved in the app, but could not write its ID back to the sheet: {e}"),
            });
        }
    }

    refresh_pulls_sheet_structure_soft_fail(
        conn,
        token,
        &connection.spreadsheet_id,
        &connection.sheet_tab,
        &headers,
        data_rows.len(),
        &mut result,
    );

    result.synced_at = now_iso(conn)?;
    set_setting(conn, &last_synced_key("pulls"), &result.synced_at)?;
    Ok(result)
}

/// Manual "Sync now" button (Settings -> Integrations, Pulls card). Never
/// runs on its own.
#[tauri::command]
pub fn sync_pulls(state: State<AppState>) -> AppResult<SheetSyncResult> {
    let conn = state.db.lock().unwrap();
    sync_pulls_impl(&conn)
}

// ---------------------------------------------------------------------------
// The push direction's own network-calling shell - same split as
// sync_pulls_impl above (fetch the sheet once, hand its parsed shape to the
// pure core, then perform whatever writes it asked for), just walking in the
// opposite direction. See google_sheets.rs's module doc comment for why this
// half can't be exercised in this sandbox.
// ---------------------------------------------------------------------------

fn push_pulls_impl(conn: &Connection) -> AppResult<SheetSyncResult> {
    let connection = load_connection(conn, "pulls")?
        .ok_or_else(|| AppError::Validation("No spreadsheet is connected for Pulls yet - connect one in Settings first.".to_string()))?;
    let credential = crate::commands::google_auth::resolve_google_credential(conn, false)?;
    let token = credential.access_token();

    let range = google_sheets::a1_range(&connection.sheet_tab, "A1:Z");
    let value_range = google_sheets::get_values(token, &connection.spreadsheet_id, &range)?;
    if value_range.values.is_empty() {
        return Err(AppError::Validation("The connected sheet/tab has no header row yet.".to_string()));
    }
    let headers = value_range.values[0].clone();
    let data_rows: &[Vec<String>] = if value_range.values.len() > 1 { &value_range.values[1..] } else { &[] };

    let (marker_col_index, marker_exists) = resolve_marker_column(&headers);
    let letter = column_index_to_a1(marker_col_index);
    if !marker_exists {
        let header_range = google_sheets::a1_range(&connection.sheet_tab, &format!("{letter}1"));
        google_sheets::update_values(token, &connection.spreadsheet_id, &header_range, &[vec![MARKER_HEADER.to_string()]])?;
    }

    let (mut result, writes) = apply_pull_push(conn, &headers, data_rows, &connection.currency, marker_col_index)?;

    let mut append_rows: Vec<Vec<String>> = vec![];
    for write in writes {
        match write {
            PullPushWrite::Append(row) => append_rows.push(row),
            PullPushWrite::Update { sheet_row_number, cells } => {
                for (col, value) in cells {
                    let col_letter = column_index_to_a1(col);
                    let cell_range = google_sheets::a1_range(&connection.sheet_tab, &format!("{col_letter}{sheet_row_number}"));
                    if let Err(e) = google_sheets::update_values(token, &connection.spreadsheet_id, &cell_range, &[vec![value]]) {
                        result.errors.push(SheetSyncIssue {
                            row_number: sheet_row_number,
                            message: format!("saved locally, but could not write this change back to the sheet: {e}"),
                        });
                    }
                }
            }
        }
    }
    if !append_rows.is_empty() {
        let new_count = append_rows.len();
        let append_range = google_sheets::a1_range(&connection.sheet_tab, "A1");
        if let Err(e) = google_sheets::append_values(token, &connection.spreadsheet_id, &append_range, &append_rows) {
            result.errors.push(SheetSyncIssue {
                row_number: 0,
                message: format!("{new_count} new pull(s) were prepared but could not be written to the sheet: {e}"),
            });
        }
    }

    // Pre-append `data_rows` (not re-fetched after the writes above) - same
    // choice as orders_sheet_sync::push_orders_impl/push_sales_impl, and for
    // the same reason: DROPDOWN_ROW_BUFFER already covers far more rows than
    // any real sheet has, so a row just appended in this very call already
    // has a working dropdown without needing its own re-fetch first.
    refresh_pulls_sheet_structure_soft_fail(
        conn,
        token,
        &connection.spreadsheet_id,
        &connection.sheet_tab,
        &headers,
        data_rows.len(),
        &mut result,
    );

    result.synced_at = now_iso(conn)?;
    set_setting(conn, &last_pushed_key("pulls"), &result.synced_at)?;
    Ok(result)
}

/// "Push to sheet" button (Settings -> Integrations, Pulls card) - the new
/// sibling of "Sync now" (marko's own choice of two separate buttons rather
/// than merging the two directions into one). Never runs on its own.
#[tauri::command]
pub fn push_pulls(state: State<AppState>) -> AppResult<SheetSyncResult> {
    let conn = state.db.lock().unwrap();
    push_pulls_impl(&conn)
}

// ---------------------------------------------------------------------------
// Auto-create-and-share (2.0.4, OAuth-aware since 2.0.5) - "Create a new
// sheet for me", the alternative to pasting an existing sheet's URL, built
// for marko's original ask: one click, a brand-new Pulls sheet appears
// already ready to use, no Google sign-in window forced on anyone who does
// not want one (see google_sheets.rs's `SHEETS_AND_DRIVE_SCOPE` doc comment
// for the original service-account-only design). Whether the new sheet
// needs an explicit *share* step at all now depends on which credential
// created it - see `create_pulls_sheet_impl`'s doc comment. Fully additive
// either way: the paste-a-URL flow (commands/sheets_sync.rs::
// set_sheets_connection) keeps working exactly as before, unchanged - this
// is one more way to arrive at the same connected state, not a replacement.
// ---------------------------------------------------------------------------

/// The header row written into a freshly-created sheet - exactly the columns
/// `apply_pull_rows` above understands, in the same order as the mapping
/// table in this module's doc comment. Deliberately excludes `date` and the
/// unnamed blank column (nothing reads either) and `TIQR ID` (sync appends
/// that itself the first time it's missing - see `resolve_marker_column`),
/// so someone opening a freshly-created sheet sees only columns that
/// actually do something.
const PULLS_SHEET_HEADERS: &[&str] =
    &["pull", "Event name", "event date", "Ks", "Platform", "More info", "Section", "Row", "Seats", "Transfer", "Price"];

const NEW_SHEET_TITLE: &str = "TIQR Manager - Pulls";
const NEW_SHEET_TAB_NAME: &str = "Pulls";

/// A light, offline pre-check - not a full RFC 5322 parser. Drive itself is
/// the real, authoritative validator (a syntactically plausible but
/// non-existent address just fails later at `share_file` with Google's own
/// message); this only exists so an obvious mistake (empty, no "@", "@" at
/// the very start/end, embedded whitespace) fails instantly, before spending
/// a real API round-trip on it - and, more importantly, before creating and
/// sharing a real sheet first and only then discovering the address was
/// never usable.
///
/// `pub(crate)` (not just `fn`) since 2.0.9: commands::orders_sheet_sync
/// reuses this exact same check for its own "Create a new sheet for me"
/// button - the rule is generic (a plausible email address is a plausible
/// email address, regardless of which sheet it's sharing), so there's no
/// reason to duplicate it a second time for the same underlying rule.
pub(crate) fn validate_share_email(email: &str) -> AppResult<String> {
    let trimmed = email.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation("Enter the email address to share the new sheet with.".to_string()));
    }
    if trimmed.chars().any(|c| c.is_whitespace()) {
        return Err(AppError::Validation("Email address must not contain spaces.".to_string()));
    }
    match trimmed.find('@') {
        Some(pos) if pos > 0 && pos < trimmed.len() - 1 => Ok(trimmed.to_string()),
        _ => Err(AppError::Validation("That doesn't look like a valid email address.".to_string())),
    }
}

/// Deliberately duplicates `set_sheets_connection_impl`'s own currency
/// check: that function only runs at the very end of
/// `create_pulls_sheet_impl`, *after* a real spreadsheet has already been
/// created and shared - failing late on a bad currency would leave that real
/// sheet orphaned (created, shared with someone, but never connected in the
/// app). Checking it here first means a bad currency never reaches the
/// network at all.
///
/// `pub(crate)` (not just `fn`) since 2.0.9: commands::orders_sheet_sync
/// reuses this exact same check for its own "Create a new sheet for me"
/// button - `ALLOWED_CURRENCIES` is a single shared list either way, so
/// there's no reason to duplicate the check itself a second time.
pub(crate) fn validate_currency(currency: &str) -> AppResult<String> {
    let upper = currency.trim().to_uppercase();
    if !ALLOWED_CURRENCIES.contains(&upper.as_str()) {
        return Err(AppError::Validation(format!(
            "Currency must be one of {} - got '{currency}'",
            ALLOWED_CURRENCIES.join(", ")
        )));
    }
    Ok(upper)
}

/// Creates a brand-new Google Sheet for Pulls, writes `PULLS_SHEET_HEADERS`
/// as its header row, and connects it - all in one call, with no Google
/// sign-in window at any point. `email` and `currency` are fully validated
/// before the first network call - see `validate_share_email`/
/// `validate_currency`'s doc comments for why that ordering matters here
/// specifically.
///
/// 2.0.5: uses the signed-in person's own OAuth token when there is one, the
/// shared service account otherwise (see
/// commands::google_auth::resolve_google_credential's doc comment) - but
/// `email` is only ever *used* on the service-account path. Signed in via
/// OAuth, the new sheet already belongs to the signed-in person the moment
/// Sheets creates it, so there is no separate *share* step to run at all;
/// `email` is still validated regardless of which path runs (simpler than
/// making validation itself conditional, and it costs nothing - the
/// frontend never shows that field, or asks for it, once someone is signed
/// in - see Settings.tsx).
fn create_pulls_sheet_impl(conn: &Connection, email: &str, currency: &str) -> AppResult<CreatedSheetResult> {
    let email = validate_share_email(email)?;
    let currency_upper = validate_currency(currency)?;

    let credential = crate::commands::google_auth::resolve_google_credential(conn, true)?;
    let token = credential.access_token();

    let created = google_sheets::create_spreadsheet(token, NEW_SHEET_TITLE, NEW_SHEET_TAB_NAME)?;

    let header_row: Vec<String> = PULLS_SHEET_HEADERS.iter().map(|s| s.to_string()).collect();
    let header_range = google_sheets::a1_range(NEW_SHEET_TAB_NAME, "A1");
    google_sheets::update_values(token, &created.spreadsheet_id, &header_range, &[header_row])?;

    if !credential.is_oauth() {
        google_sheets::share_file(token, &created.spreadsheet_id, &email)?;
    }

    let connection =
        set_sheets_connection_impl(conn, "pulls", &created.spreadsheet_id, NEW_SHEET_TAB_NAME, &currency_upper)?;

    Ok(CreatedSheetResult { connection, spreadsheet_url: created.spreadsheet_url })
}

/// "Create a new sheet for me" button (Settings -> Integrations, Pulls card)
/// - sits right next to the existing paste-a-URL form as a second way to
/// connect, not a replacement for it. Never runs on its own.
#[tauri::command]
pub fn create_pulls_sheet(state: State<AppState>, email: String, currency: String) -> AppResult<CreatedSheetResult> {
    let conn = state.db.lock().unwrap();
    create_pulls_sheet_impl(&conn, &email, &currency)
}

// ---------------------------------------------------------------------------
// Sheet structure (Platform + Transfer dropdowns), 2.0.21 - mirrors
// commands::orders_sheet_sync's own "Sheet structure" section (2.0.19) for
// this sheet's two columns that call for one, marko's own request: "Platform
// tu daj na výber možnosti a spoj to s dashboardom, ked do dashboradu pridas
// nove policko tak po update sa to opravi aj v sheete ... Transfer tu davame
// 2 moznosti bud ano/nie". Every other column he listed (pull/Event name/
// event date/Ks/More info/Section/Row/Seats/Price) he explicitly said to
// leave exactly as it is - no dropdown, no formula, untouched. Applied
// across the WHOLE sheet, same "všetky riadky" choice marko already made for
// Orders & Sales (see that module's doc comment) - every time
// sync_pulls/push_pulls/setup_pulls_sheet runs, no separate button to
// remember. Unlike Orders & Sales, this never rewrites any cell's actual
// VALUE (no Revenue/Profit-style formula here - Pulls has no computed column
// of its own), so unlike 2.0.19 there is no first-run "this will overwrite
// existing values" risk at all: Data validation only restricts what a click
// on an empty dropdown arrow offers, it never touches what a cell already
// contains.
// ---------------------------------------------------------------------------

/// Fixed - "Transfer" is a closed yes/no choice (marko's own spec), not
/// grown from anything. Deliberately the exact same two strings the push
/// direction already writes into this column (see `apply_pull_push` below:
/// `if pull.transfer_done { "Yes" } else { "No" }`), so anything this
/// dropdown offers is always a value `parse_transfer_done` above already
/// understands too.
const TRANSFER_OPTIONS: &[&str] = &["Yes", "No"];

/// Background colors for `plan_pulls_sheet_color_updates` below - marko's
/// own request (2.0.21 follow-up, 2.0.22): "yes zelenou, nie modrou". Given
/// as plain color names, not exact shades, so these are this app's own
/// reasonable pick - light/pastel enough that the cell's default black text
/// stays easily readable on top. Same shade of green as
/// orders_sheet_sync::COLOR_GREEN, kept as this module's own copy rather
/// than a shared constant - same file-local duplication convention this
/// module already follows for column_index_to_a1/now_iso/DROPDOWN_ROW_BUFFER.
const COLOR_GREEN: (f64, f64, f64) = (0.71, 0.88, 0.80);
const COLOR_BLUE: (f64, f64, f64) = (0.79, 0.86, 0.97);

/// Same reasoning/value as orders_sheet_sync::DROPDOWN_ROW_BUFFER - far
/// beyond however much data is in the sheet right now, so a newly-added row
/// has a working dropdown immediately, without needing a re-run just to get
/// one.
const DROPDOWN_ROW_BUFFER: i64 = 500;

/// `Pull.platform_id`'s own name pool, filtered to platforms that can
/// actually be a PURCHASE platform (`kind IN ('purchase', 'both')`) - the
/// exact same filter the Pulls "Add pull" form's own Platform picker already
/// uses (Pulls.tsx: `platforms.filter((p) => p.kind === "purchase" || p.kind
/// === "both")`), and the same `kind` `resolve_or_create_platform` above
/// already writes when a sheet row's platform name is brand new. Mirrors
/// orders_sheet_sync::sale_platform_names exactly, just the purchase side of
/// the same `platforms` table - growable the same way: whatever platform
/// exists in the app right now becomes this dropdown's option list on the
/// next sync/push/update, nothing to separately maintain.
fn purchase_platform_names(conn: &Connection) -> AppResult<Vec<String>> {
    let mut stmt =
        conn.prepare("SELECT name FROM platforms WHERE kind IN ('purchase', 'both') AND is_demo = 0 ORDER BY name COLLATE NOCASE")?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

struct DropdownSpec {
    col_index: usize,
    values: Vec<String>,
}

/// Pure core: given the sheet's actual headers (tolerant of reordering/extra
/// columns, same `find_col`/`build_header_map` every other function in this
/// module already uses), works out exactly which dropdown columns exist and
/// what their option lists should be right now. No network calls - see
/// `ensure_pulls_sheet_structure` for the network shell that sends this as
/// real Sheets API requests. A dropdown column not present in `headers` at
/// all is simply skipped, same tolerance as everywhere else in this module;
/// Platform is additionally skipped when there are currently zero purchase
/// platforms to offer - nothing meaningful to restrict the column to yet.
fn plan_pulls_sheet_structure_updates(conn: &Connection, headers: &[String]) -> AppResult<Vec<DropdownSpec>> {
    let map = build_header_map(headers);
    let mut dropdowns: Vec<DropdownSpec> = vec![];

    if let Some(c) = find_col(&map, &["platform"]) {
        let values = purchase_platform_names(conn)?;
        if !values.is_empty() {
            dropdowns.push(DropdownSpec { col_index: c, values });
        }
    }
    if let Some(c) = find_col(&map, &["transfer"]) {
        dropdowns.push(DropdownSpec { col_index: c, values: TRANSFER_OPTIONS.iter().map(|s| s.to_string()).collect() });
    }

    Ok(dropdowns)
}

struct ColorSpec {
    col_index: usize,
    /// Exact cell text -> background color. Sibling of `DropdownSpec` above
    /// rather than folded into it - color-coding is a separate kind of
    /// structure decision (conditional formatting, not data validation) that
    /// happens to target the same column here. Never needs `conn`: Transfer
    /// is a fixed-option column, not a growable/DB-backed one (Platform is
    /// deliberately NOT colored - marko only asked for Transfer here).
    colors: Vec<(String, (f64, f64, f64))>,
}

/// Pure core, mirrors orders_sheet_sync::plan_sheet_color_updates - marko's
/// own request: "yes zelenou, nie modrou" for this sheet's Transfer column.
/// Same tolerance as everywhere else in this module: skipped entirely when
/// the column isn't present in `headers`.
fn plan_pulls_sheet_color_updates(headers: &[String]) -> Vec<ColorSpec> {
    let map = build_header_map(headers);
    let mut specs: Vec<ColorSpec> = vec![];

    if let Some(c) = find_col(&map, &["transfer"]) {
        specs.push(ColorSpec { col_index: c, colors: vec![("Yes".to_string(), COLOR_GREEN), ("No".to_string(), COLOR_BLUE)] });
    }

    specs
}

/// The network shell for `plan_pulls_sheet_structure_updates`/
/// `plan_pulls_sheet_color_updates` above - sends their plan as real
/// `batchUpdate` (Data validation + conditional formatting) calls, same
/// pattern as orders_sheet_sync::ensure_orders_sheet_structure just without a
/// formula-writing half (nothing here needs one).
fn ensure_pulls_sheet_structure(
    conn: &Connection,
    token: &str,
    spreadsheet_id: &str,
    sheet_tab: &str,
    headers: &[String],
    data_row_count: usize,
) -> AppResult<()> {
    let dropdowns = plan_pulls_sheet_structure_updates(conn, headers)?;
    let colors = plan_pulls_sheet_color_updates(headers);
    if dropdowns.is_empty() && colors.is_empty() {
        return Ok(());
    }

    // One shared metadata fetch for both - see
    // orders_sheet_sync::ensure_orders_sheet_structure's own comment on the
    // same call for why this replaces a separate get_sheet_numeric_id call.
    let metadata = google_sheets::get_sheet_structure_metadata(token, spreadsheet_id, sheet_tab)?;
    let sheet_id = metadata.sheet_id;
    let end_row = (data_row_count as i64).max(DROPDOWN_ROW_BUFFER) + 1;
    let mut requests: Vec<serde_json::Value> = vec![];

    if !colors.is_empty() {
        let managed_columns: Vec<i64> = colors.iter().map(|c| c.col_index as i64).collect();
        let to_delete = google_sheets::conditional_format_indices_to_replace(&metadata.conditional_format_columns, &managed_columns);
        for index in to_delete {
            requests.push(google_sheets::delete_conditional_format_rule_request(sheet_id, index));
        }
        for spec in &colors {
            for (value, color) in &spec.colors {
                requests.push(google_sheets::add_conditional_format_color_request(
                    sheet_id,
                    1,
                    end_row,
                    spec.col_index as i64,
                    value,
                    *color,
                ));
            }
        }
    }

    requests.extend(dropdowns.iter().map(|d| google_sheets::set_data_validation_request(sheet_id, 1, end_row, d.col_index as i64, &d.values)));

    if requests.is_empty() {
        return Ok(());
    }
    google_sheets::batch_update(token, spreadsheet_id, requests)
}

/// Runs `ensure_pulls_sheet_structure` and folds any error it returns into
/// `result` as a soft warning instead of propagating it - same convention as
/// orders_sheet_sync::refresh_sheet_structure_soft_fail, shared by every call
/// site below so a structure-refresh problem is reported the exact same way
/// everywhere and never blocks/discards the real sync/push/setup work it
/// rode along with.
fn refresh_pulls_sheet_structure_soft_fail(
    conn: &Connection,
    token: &str,
    spreadsheet_id: &str,
    sheet_tab: &str,
    headers: &[String],
    data_row_count: usize,
    result: &mut SheetSyncResult,
) {
    if let Err(e) = ensure_pulls_sheet_structure(conn, token, spreadsheet_id, sheet_tab, headers, data_row_count) {
        result.errors.push(SheetSyncIssue {
            row_number: 0,
            message: format!("the sheet's Platform/Transfer dropdowns could not be refreshed this time: {e}"),
        });
    }
}

// ---------------------------------------------------------------------------
// "Update sheet" (2.0.20) - marko hit a real "missing field `sheetId`" error
// (fixed in google_sheets.rs - see that module's SpreadsheetMetadata doc
// comment) while connecting a sheet by pasting its URL, and asked in the same
// message for a way to bring an already-connected sheet up to the correct
// shape on demand: "ked manualne si tam das tabulku tak vies si dat update
// tlacitko a ten sheet ti vytvorí presne tak ako ma byt keby nahodou mu tam
// posles prazny" - i.e. a button for the case where the sheet/tab he pasted
// in (not "Create a new sheet for me", which already always writes a correct
// header) turns out to have no header row yet. `create_pulls_sheet_impl`
// above can't be reused directly: it always creates a brand-new spreadsheet,
// never writes into a sheet that's already connected.
// ---------------------------------------------------------------------------

/// Writes `PULLS_SHEET_HEADERS` as row 1 of the already-connected sheet/tab,
/// but ONLY when it currently has no header row at all - an existing header
/// (whatever its exact columns are) is never touched, reordered, or
/// overwritten, so clicking this on a sheet that's already set up correctly
/// is always a safe no-op that simply reports `unchanged`. See this
/// section's own doc comment above for why this exists alongside
/// `create_pulls_sheet_impl` rather than replacing it.
fn setup_pulls_sheet_impl(conn: &Connection) -> AppResult<SheetSyncResult> {
    let connection = load_connection(conn, "pulls")?
        .ok_or_else(|| AppError::Validation("No spreadsheet is connected for Pulls yet - connect one in Settings first.".to_string()))?;
    let credential = crate::commands::google_auth::resolve_google_credential(conn, false)?;
    let token = credential.access_token();

    let range = google_sheets::a1_range(&connection.sheet_tab, "A1:Z");
    let value_range = google_sheets::get_values(token, &connection.spreadsheet_id, &range)?;

    let mut result = SheetSyncResult {
        created: 0,
        updated: 0,
        unchanged: 0,
        conflicts: vec![],
        errors: vec![],
        synced_at: String::new(),
    };

    let headers: Vec<String> = if value_range.values.is_empty() {
        let header_row: Vec<String> = PULLS_SHEET_HEADERS.iter().map(|s| s.to_string()).collect();
        let header_range = google_sheets::a1_range(&connection.sheet_tab, "A1");
        google_sheets::update_values(token, &connection.spreadsheet_id, &header_range, &[header_row.clone()])?;
        result.created = 1;
        header_row
    } else {
        result.unchanged = 1;
        value_range.values[0].clone()
    };
    let data_row_count = value_range.values.len().saturating_sub(1);

    refresh_pulls_sheet_structure_soft_fail(
        conn,
        token,
        &connection.spreadsheet_id,
        &connection.sheet_tab,
        &headers,
        data_row_count,
        &mut result,
    );

    result.synced_at = now_iso(conn)?;
    Ok(result)
}

/// "Update sheet" button (Settings -> Integrations, Pulls card) - sits next
/// to "Sync now"/"Push to sheet", for the already-connected sheet rather than
/// the separate "Create a new sheet for me" flow. Never runs on its own.
#[tauri::command]
pub fn setup_pulls_sheet(state: State<AppState>) -> AppResult<SheetSyncResult> {
    let conn = state.db.lock().unwrap();
    setup_pulls_sheet_impl(&conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_conn;

    // Header order mirrors marko's real sheet, with Section/Row inserted
    // before Seats (the 2.0.2 follow-up conversation) and the unnamed blank
    // column kept as an empty header string, exactly as Sheets reports it.
    fn full_headers() -> Vec<String> {
        vec![
            "pull", "Event name", "event date", "Ks", "Platform", "More info", "Section", "Row", "Seats", "", "Transfer", "Price", "date",
        ]
        .into_iter()
        .map(String::from)
        .collect()
    }

    fn row(cells: &[&str]) -> Vec<String> {
        cells.iter().map(|s| s.to_string()).collect()
    }

    fn marek_row(marker: &str) -> Vec<String> {
        // pull, Event name, event date, Ks, Platform, More info, Section, Row, Seats, "", Transfer, Price, date
        let mut r = row(&[
            "marek480",
            "Bruno mars",
            "25.07.2026",
            "2x",
            "ticketmaster",
            "esosamko@gmail.com 84 AUS",
            "507",
            "24",
            "202 - 203",
            "",
            "Nie",
            "20",
            "14.1.2026",
        ]);
        r.push(marker.to_string());
        r
    }

    const MARKER_COL: usize = 13; // one past the 13 headers in full_headers()

    fn headers_with_marker() -> Vec<String> {
        let mut h = full_headers();
        h.push(MARKER_HEADER.to_string());
        h
    }

    // ---- header / column-letter plumbing -------------------------------

    #[test]
    fn column_index_to_a1_matches_sheets_own_scheme() {
        assert_eq!(column_index_to_a1(0), "A");
        assert_eq!(column_index_to_a1(25), "Z");
        assert_eq!(column_index_to_a1(26), "AA");
        assert_eq!(column_index_to_a1(27), "AB");
        assert_eq!(column_index_to_a1(51), "AZ");
    }

    #[test]
    fn resolve_marker_column_appends_one_past_the_last_header_when_absent() {
        let (idx, exists) = resolve_marker_column(&full_headers());
        assert_eq!(idx, 13);
        assert!(!exists);
    }

    #[test]
    fn resolve_marker_column_finds_it_case_and_space_insensitively_when_present() {
        let mut headers = full_headers();
        headers.push("  tiqr id ".to_string());
        let (idx, exists) = resolve_marker_column(&headers);
        assert_eq!(idx, 13);
        assert!(exists);
    }

    #[test]
    fn missing_required_headers_fail_the_whole_sync_with_one_clear_message() {
        let conn = test_conn();
        let headers: Vec<String> = vec!["Event name".to_string(), "Platform".to_string()];
        let err = apply_pull_rows(&conn, &headers, &[], "EUR", 2).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("pull"), "{msg}");
        assert!(msg.contains("Ks"), "{msg}");
        assert!(msg.contains("Price"), "{msg}");
    }

    // ---- field parsers ---------------------------------------------------

    #[test]
    fn parse_quantity_accepts_trailing_x_and_plain_numbers() {
        assert_eq!(parse_quantity("2x").unwrap(), 2);
        assert_eq!(parse_quantity("2X").unwrap(), 2);
        assert_eq!(parse_quantity(" 3 ").unwrap(), 3);
    }

    #[test]
    fn parse_quantity_rejects_zero_and_garbage() {
        assert!(parse_quantity("0").is_err());
        assert!(parse_quantity("").is_err());
        assert!(parse_quantity("2 tickets").is_err());
    }

    #[test]
    fn parse_transfer_done_recognizes_slovak_and_english_yes_values() {
        assert!(parse_transfer_done("Áno"));
        assert!(parse_transfer_done("ano"));
        assert!(parse_transfer_done("Yes"));
        assert!(parse_transfer_done("1"));
    }

    #[test]
    fn parse_transfer_done_defaults_everything_else_to_false() {
        assert!(!parse_transfer_done("Nie"));
        assert!(!parse_transfer_done(""));
        assert!(!parse_transfer_done("no"));
        assert!(!parse_transfer_done("maybe"));
    }

    #[test]
    fn parse_sheet_date_handles_numeric_and_slovak_month_abbreviation_formats() {
        assert_eq!(parse_sheet_date("15.05.2026").unwrap(), Some("2026-05-15".to_string()));
        assert_eq!(parse_sheet_date("26.jan.2026").unwrap(), Some("2026-01-26".to_string()));
        assert_eq!(parse_sheet_date("14.08.2026").unwrap(), Some("2026-08-14".to_string()));
    }

    #[test]
    fn parse_sheet_date_treats_blank_as_none_not_an_error() {
        assert_eq!(parse_sheet_date("").unwrap(), None);
        assert_eq!(parse_sheet_date("   ").unwrap(), None);
    }

    #[test]
    fn parse_sheet_date_rejects_invalid_calendar_dates_and_garbage() {
        assert!(parse_sheet_date("31.02.2026").is_err(), "February never has 31 days");
        assert!(parse_sheet_date("IDK").is_err());
        assert!(parse_sheet_date("2026-01-26").is_err(), "ISO input is not the sheet's own format");
    }

    #[test]
    fn parse_sheet_date_converts_a_sheets_serial_date_number() {
        // 46291 = 2026-09-26 - what a cell someone typed a real date into
        // looks like by the time it reaches this function (Sheets returns
        // its underlying serial-day number for such a cell; see
        // google_sheets::ValueRange's doc comment for why, and
        // REDESIGN-2.0.7-REPORT.md for the bug this fixes).
        assert_eq!(parse_sheet_date("46291").unwrap(), Some("2026-09-26".to_string()));
    }

    #[test]
    fn parse_sheet_date_rejects_an_implausible_serial_number() {
        assert!(parse_sheet_date("5").is_err(), "day 5 of the 1899 epoch is not a plausible pull date");
    }

    #[test]
    fn a_row_with_a_real_sheets_date_and_numeric_columns_syncs_cleanly() {
        // The exact real-world shape that used to crash the whole sync
        // before it even reached this function (google_sheets.rs's
        // deserialization) - and would have failed here too, wrongly, had
        // "46291" stayed unrecognized as a date. See REDESIGN-2.0.7-REPORT.md.
        let conn = test_conn();
        let sheet_row = row(&[
            "sojky", "England vs Spain", "46291", "8", "ticketmaster", "", "410", "25", "11-18", "", "TRUE", "50", "",
        ]);
        let (result, _writes) = apply_pull_rows(&conn, &full_headers(), &[sheet_row], "EUR", MARKER_COL).unwrap();
        assert_eq!(result.errors.len(), 0, "errors: {:?}", result.errors);
        assert_eq!(result.created, 1);
        let pull = fetch_pull(&conn, 1).unwrap();
        assert_eq!(pull.event_date.as_deref(), Some("2026-09-26"));
        assert_eq!(pull.quantity, 8);
        assert_eq!(pull.section.as_deref(), Some("410"));
        assert_eq!(pull.row_label.as_deref(), Some("25"));
        assert_eq!(pull.price_cents, 5000);
        assert!(pull.transfer_done);
    }

    // ---- apply_pull_rows: creating ---------------------------------------

    #[test]
    fn a_brand_new_row_with_no_marker_creates_a_pull_and_asks_for_a_marker_write() {
        let conn = test_conn();
        let (result, writes) = apply_pull_rows(&conn, &full_headers(), &[marek_row("")], "EUR", MARKER_COL).unwrap();
        assert_eq!(result.created, 1);
        assert_eq!(result.updated, 0);
        assert_eq!(result.errors.len(), 0);
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].0, 0);
        assert!(writes[0].1.starts_with("PULL-"));

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM pulls", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn a_created_pull_gets_exactly_the_parsed_field_values() {
        let conn = test_conn();
        apply_pull_rows(&conn, &full_headers(), &[marek_row("")], "EUR", MARKER_COL).unwrap();
        let pull = fetch_pull(&conn, 1).unwrap();
        assert_eq!(pull.buyer_name, "marek480");
        assert_eq!(pull.event_name, "Bruno mars");
        assert_eq!(pull.event_date.as_deref(), Some("2026-07-25"));
        assert_eq!(pull.quantity, 2);
        assert_eq!(pull.platform_name.as_deref(), Some("ticketmaster"));
        assert_eq!(pull.section.as_deref(), Some("507"));
        assert_eq!(pull.row_label.as_deref(), Some("24"));
        assert_eq!(pull.seat.as_deref(), Some("202 - 203"));
        assert_eq!(pull.price_cents, 2000);
        assert_eq!(pull.currency, "EUR");
        assert!(!pull.transfer_done, "row's Transfer column was 'Nie'");
    }

    #[test]
    fn creating_resolves_platform_case_insensitively_and_creates_it_when_missing() {
        let conn = test_conn();
        conn.execute("INSERT INTO platforms(name, kind) VALUES ('TicketMaster', 'purchase')", []).unwrap();
        apply_pull_rows(&conn, &full_headers(), &[marek_row("")], "EUR", MARKER_COL).unwrap();
        let platform_count: i64 = conn.query_row("SELECT COUNT(*) FROM platforms", [], |r| r.get(0)).unwrap();
        assert_eq!(platform_count, 1, "must reuse the existing platform, not create a duplicate");
    }

    #[test]
    fn a_row_marked_transfer_done_is_created_already_done_with_a_timestamp() {
        let conn = test_conn();
        let mut cells = marek_row("");
        cells[10] = "Áno".to_string(); // Transfer column
        apply_pull_rows(&conn, &full_headers(), &[cells], "EUR", MARKER_COL).unwrap();
        let pull = fetch_pull(&conn, 1).unwrap();
        assert!(pull.transfer_done);
        assert!(pull.transfer_done_at.is_some());
    }

    #[test]
    fn section_row_seat_are_blank_when_those_columns_are_absent_from_the_sheet() {
        let conn = test_conn();
        // A sheet that hasn't added Section/Row yet - only the original columns.
        let headers: Vec<String> = vec!["pull", "Event name", "Ks", "Price"].into_iter().map(String::from).collect();
        let data = row(&["raxik", "Fred Again", "2x", "50"]);
        let (result, _) = apply_pull_rows(&conn, &headers, &[data], "EUR", 4).unwrap();
        assert_eq!(result.created, 1);
        let pull = fetch_pull(&conn, 1).unwrap();
        assert!(pull.section.is_none());
        assert!(pull.row_label.is_none());
        assert!(pull.seat.is_none());
        assert!(pull.event_date.is_none());
    }

    #[test]
    fn a_fully_blank_row_is_skipped_silently() {
        let conn = test_conn();
        let blank = vec![String::new(); full_headers().len()];
        let (result, writes) = apply_pull_rows(&conn, &full_headers(), &[blank], "EUR", MARKER_COL).unwrap();
        assert_eq!(result.created, 0);
        assert_eq!(result.errors.len(), 0);
        assert!(writes.is_empty());
    }

    #[test]
    fn a_row_with_a_bad_quantity_is_reported_and_does_not_block_the_next_row() {
        let conn = test_conn();
        let mut bad = marek_row("");
        bad[3] = "abc".to_string(); // Ks column
        let good = {
            let mut r = marek_row("");
            r[0] = "davidess_".to_string();
            r
        };
        let (result, _) = apply_pull_rows(&conn, &full_headers(), &[bad, good], "EUR", MARKER_COL).unwrap();
        assert_eq!(result.created, 1, "the second, valid row must still import");
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].row_number, 2, "row 2 is the first data row (row 1 is the header)");
    }

    #[test]
    fn an_unrecognized_marker_value_is_reported_rather_than_silently_creating_a_duplicate() {
        let conn = test_conn();
        let (result, writes) = apply_pull_rows(&conn, &full_headers(), &[marek_row("PULL-999999")], "EUR", MARKER_COL).unwrap();
        assert_eq!(result.created, 0);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].message.contains("unrecognized"));
        assert!(writes.is_empty());
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM pulls", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 0);
    }

    // ---- apply_pull_rows: matching / updating / idempotency --------------

    fn sync_twice_seeds_a_linked_pull(conn: &Connection) -> String {
        let (result, writes) = apply_pull_rows(conn, &full_headers(), &[marek_row("")], "EUR", MARKER_COL).unwrap();
        assert_eq!(result.created, 1);
        let marker = writes[0].1.clone();
        // The shell would normally write this back to the sheet - simulate
        // that by inserting the marker into a second sync's own row data.
        marker
    }

    #[test]
    fn running_sync_twice_on_an_unchanged_row_creates_nothing_the_second_time() {
        let conn = test_conn();
        let marker = sync_twice_seeds_a_linked_pull(&conn);
        let (result, writes) = apply_pull_rows(&conn, &full_headers(), &[marek_row(&marker)], "EUR", MARKER_COL).unwrap();
        assert_eq!(result.created, 0);
        assert_eq!(result.updated, 0);
        assert_eq!(result.unchanged, 1);
        assert!(writes.is_empty());
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM pulls", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1, "must not duplicate an already-linked row");
    }

    #[test]
    fn a_sheet_side_edit_on_an_untouched_pull_is_applied() {
        let conn = test_conn();
        let marker = sync_twice_seeds_a_linked_pull(&conn);
        let mut edited = marek_row(&marker);
        edited[11] = "35".to_string(); // Price changed 20 -> 35
        let (result, _) = apply_pull_rows(&conn, &full_headers(), &[edited], "EUR", MARKER_COL).unwrap();
        assert_eq!(result.updated, 1);
        assert_eq!(result.unchanged, 0);
        let pull = fetch_pull(&conn, 1).unwrap();
        assert_eq!(pull.price_cents, 3500);
    }

    #[test]
    fn a_genuine_two_sided_conflict_is_reported_and_neither_side_is_overwritten() {
        let conn = test_conn();
        let marker = sync_twice_seeds_a_linked_pull(&conn);
        // Backdate the link's last_synced_at so the app's edit below is
        // unambiguously "after the last sync" regardless of how fast this
        // test itself runs (real wall-clock time between two in-memory
        // queries can easily land in the same millisecond, which is a test
        // artifact, not something that happens between a real sync and a
        // person actually clicking around the UI afterward).
        conn.execute(
            "UPDATE sheet_sync_links SET last_synced_at = '2020-01-01T00:00:00.000Z' WHERE data_source = 'pulls'",
            [],
        )
        .unwrap();

        // The app changes the pull locally (e.g. marko edits it in the UI).
        let pull = fetch_pull(&conn, 1).unwrap();
        let mut edit = PullEditInput {
            buyer_name: pull.buyer_name.clone(),
            event_name: pull.event_name.clone(),
            event_date: pull.event_date.clone(),
            quantity: pull.quantity,
            platform_id: pull.platform_id,
            section: pull.section.clone(),
            row_label: pull.row_label.clone(),
            seat: pull.seat.clone(),
            more_info: pull.more_info.clone(),
            price_cents: pull.price_cents,
            currency: pull.currency.clone(),
            transfer_done: pull.transfer_done,
        };
        edit.price_cents = 9999;
        update_pull_impl(&conn, pull.id, &edit).unwrap();

        // ...and the sheet ALSO changed the same row since the last sync.
        let mut edited = marek_row(&marker);
        edited[11] = "35".to_string();
        let (result, _) = apply_pull_rows(&conn, &full_headers(), &[edited], "EUR", MARKER_COL).unwrap();

        assert_eq!(result.conflicts.len(), 1, "a conflict must be reported, not silently resolved");
        assert_eq!(result.updated, 0);
        let unchanged_pull = fetch_pull(&conn, 1).unwrap();
        assert_eq!(unchanged_pull.price_cents, 9999, "the app's own edit must survive a conflicting sync untouched");
    }

    #[test]
    fn a_linked_pull_deleted_from_the_app_is_reported_not_silently_recreated() {
        let conn = test_conn();
        let marker = sync_twice_seeds_a_linked_pull(&conn);
        conn.execute("DELETE FROM pulls WHERE id = 1", []).unwrap();

        let mut edited = marek_row(&marker);
        edited[11] = "35".to_string();
        let (result, _) = apply_pull_rows(&conn, &full_headers(), &[edited], "EUR", MARKER_COL).unwrap();
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].message.contains("no longer exists"));
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM pulls", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 0, "must not recreate a pull marko deliberately deleted");
    }

    #[test]
    fn currency_comes_from_the_connection_not_the_sheet() {
        let conn = test_conn();
        apply_pull_rows(&conn, &full_headers(), &[marek_row("")], "GBP", MARKER_COL).unwrap();
        let pull = fetch_pull(&conn, 1).unwrap();
        assert_eq!(pull.currency, "GBP");
    }

    #[test]
    fn headers_with_marker_helper_lines_up_with_marker_col_constant() {
        // Guards the two test fixtures above against silently drifting apart.
        assert_eq!(headers_with_marker().len() - 1, MARKER_COL);
    }

    // ---- push (app -> sheet), 2.0.18 --------------------------------------

    fn make_test_pull_input() -> PullInput {
        PullInput {
            buyer_name: "peter123".to_string(),
            event_name: "Coldplay".to_string(),
            event_date: Some("2026-10-05".to_string()),
            quantity: 3,
            platform_id: None,
            section: Some("A1".to_string()),
            row_label: Some("5".to_string()),
            seat: Some("10-12".to_string()),
            more_info: Some("vip".to_string()),
            price_cents: 4500,
            currency: "EUR".to_string(),
        }
    }

    #[test]
    fn pull_push_cells_places_every_field_at_its_real_column_position() {
        let conn = test_conn();
        let pull = create_pull_impl(&conn, &make_test_pull_input(), false).unwrap();
        let map = build_header_map(&full_headers());
        let as_map: HashMap<usize, String> = pull_push_cells(&map, &pull).into_iter().collect();

        assert_eq!(as_map.get(&0), Some(&"peter123".to_string()));
        assert_eq!(as_map.get(&1), Some(&"Coldplay".to_string()));
        assert_eq!(as_map.get(&2), Some(&"05.10.2026".to_string()), "must round-trip back to the sheet's own DD.MM.YYYY shape, not the app's internal ISO storage");
        assert_eq!(as_map.get(&3), Some(&"3".to_string()));
        assert_eq!(as_map.get(&4), Some(&String::new()), "no platform was resolved - must write blank, not panic");
        assert_eq!(as_map.get(&5), Some(&"vip".to_string()));
        assert_eq!(as_map.get(&6), Some(&"A1".to_string()));
        assert_eq!(as_map.get(&7), Some(&"5".to_string()));
        assert_eq!(as_map.get(&8), Some(&"10-12".to_string()));
        assert_eq!(as_map.get(&10), Some(&"No".to_string()));
        assert_eq!(as_map.get(&11), Some(&"45.00".to_string()));
        assert!(!as_map.contains_key(&9), "the unnamed blank column is never written");
        assert!(!as_map.contains_key(&12), "'date' is ignored, never written");
        assert!(!as_map.contains_key(&13), "must never write the marker column itself");
    }

    #[test]
    fn pull_push_cells_skips_columns_the_sheet_does_not_have() {
        let conn = test_conn();
        let pull = create_pull_impl(&conn, &make_test_pull_input(), false).unwrap();
        let narrow_headers: Vec<String> =
            vec!["pull".to_string(), "Event name".to_string(), "Ks".to_string(), "Price".to_string()];
        let map = build_header_map(&narrow_headers);
        let cells = pull_push_cells(&map, &pull);
        assert_eq!(cells.len(), 4, "only the 4 columns that actually exist on this sheet");
    }

    #[test]
    fn build_pull_append_row_places_the_marker_and_pads_every_other_column_blank() {
        let conn = test_conn();
        let pull = create_pull_impl(&conn, &make_test_pull_input(), false).unwrap();
        let map = build_header_map(&full_headers());
        let row = build_pull_append_row(&map, MARKER_COL, full_headers().len(), &pull);
        assert_eq!(row.len(), MARKER_COL + 1, "must extend far enough to include the brand-new marker column");
        assert_eq!(row[0], "peter123");
        assert_eq!(row[MARKER_COL], pull.code);
        assert_eq!(row[9], "", "the unnamed blank column marko's sheet has must stay blank");
        assert_eq!(row[12], "", "'date' is ignored, never written");
    }

    #[test]
    fn pull_to_snapshot_matches_a_freshly_parsed_rows_snapshot_for_the_same_values() {
        // The whole conflict-detection mechanism hinges on this: a row this
        // module itself just wrote must parse back to exactly the local
        // pull's own snapshot, or every push would look like a conflict.
        let conn = test_conn();
        let pull = create_pull_impl(&conn, &make_test_pull_input(), false).unwrap();
        let map = build_header_map(&full_headers());
        let row = build_pull_append_row(&map, MARKER_COL, full_headers().len(), &pull);
        let parsed = parse_pull_row(&map, &row).unwrap().unwrap();
        assert_eq!(pull_to_snapshot(&pull), parsed.snapshot("EUR"));
    }

    #[test]
    fn a_never_linked_pull_is_queued_as_an_append_and_linked_immediately() {
        let conn = test_conn();
        create_pull_impl(&conn, &make_test_pull_input(), false).unwrap();

        let (result, writes) = apply_pull_push(&conn, &full_headers(), &[], "EUR", MARKER_COL).unwrap();
        assert_eq!(result.created, 1);
        assert_eq!(result.updated, 0);
        assert_eq!(result.errors.len(), 0);
        assert_eq!(writes.len(), 1);
        match &writes[0] {
            PullPushWrite::Append(row) => assert_eq!(row[0], "peter123"),
            _ => panic!("expected an Append write"),
        }
        let links: i64 = conn
            .query_row("SELECT COUNT(*) FROM sheet_sync_links WHERE data_source='pulls'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(links, 1, "must be linked right away, exactly like the sheet -> app create path already does");
    }

    #[test]
    fn a_demo_pull_is_never_pushed() {
        let conn = test_conn();
        create_pull_impl(&conn, &make_test_pull_input(), true).unwrap(); // is_demo = true

        let (result, writes) = apply_pull_push(&conn, &full_headers(), &[], "EUR", MARKER_COL).unwrap();
        assert_eq!(result.created, 0);
        assert!(writes.is_empty());
    }

    #[test]
    fn pushing_two_brand_new_pulls_appends_both() {
        let conn = test_conn();
        create_pull_impl(&conn, &make_test_pull_input(), false).unwrap();
        let mut second = make_test_pull_input();
        second.buyer_name = "iveta".to_string();
        create_pull_impl(&conn, &second, false).unwrap();

        let (result, writes) = apply_pull_push(&conn, &full_headers(), &[], "EUR", MARKER_COL).unwrap();
        assert_eq!(result.created, 2);
        assert_eq!(writes.len(), 2);
    }

    #[test]
    fn apply_pull_push_also_requires_the_same_headers_as_apply_pull_rows() {
        let conn = test_conn();
        let headers: Vec<String> = vec!["Event name".to_string(), "Platform".to_string()];
        let err = apply_pull_push(&conn, &headers, &[], "EUR", 2).unwrap_err();
        assert!(err.to_string().contains("Ks"), "{err}");
    }

    /// Pushes a brand-new pull once, then hands back the exact sheet row a
    /// real "Push to sheet" would have produced - simulating that the
    /// append actually reached the sheet, the same way
    /// `sync_twice_seeds_a_linked_pull` simulates a completed sheet -> app
    /// sync for the read direction above.
    fn push_once_seeds_a_linked_pull(conn: &Connection) -> (i64, Vec<String>) {
        let pull = create_pull_impl(conn, &make_test_pull_input(), false).unwrap();
        let (result, writes) = apply_pull_push(conn, &full_headers(), &[], "EUR", MARKER_COL).unwrap();
        assert_eq!(result.created, 1);
        let row = match &writes[0] {
            PullPushWrite::Append(row) => row.clone(),
            _ => panic!("expected an Append write"),
        };
        (pull.id, row)
    }

    #[test]
    fn pushing_again_with_the_sheet_now_matching_reports_unchanged() {
        let conn = test_conn();
        let (_id, row) = push_once_seeds_a_linked_pull(&conn);

        let (result, writes) = apply_pull_push(&conn, &full_headers(), &[row], "EUR", MARKER_COL).unwrap();
        assert_eq!(result.created, 0);
        assert_eq!(result.updated, 0);
        assert_eq!(result.unchanged, 1);
        assert!(writes.is_empty());
    }

    #[test]
    fn editing_the_local_pull_after_linking_queues_an_update_and_advances_the_snapshot() {
        let conn = test_conn();
        let (id, row) = push_once_seeds_a_linked_pull(&conn);

        let mut edit = PullEditInput {
            buyer_name: "peter123".to_string(),
            event_name: "Coldplay".to_string(),
            event_date: Some("2026-10-05".to_string()),
            quantity: 3,
            platform_id: None,
            section: Some("A1".to_string()),
            row_label: Some("5".to_string()),
            seat: Some("10-12".to_string()),
            more_info: Some("vip".to_string()),
            price_cents: 4500,
            currency: "EUR".to_string(),
            transfer_done: false,
        };
        edit.price_cents = 6000; // was 4500
        update_pull_impl(&conn, id, &edit).unwrap();

        let (result, writes) = apply_pull_push(&conn, &full_headers(), &[row], "EUR", MARKER_COL).unwrap();
        assert_eq!(result.updated, 1);
        assert_eq!(result.unchanged, 0);
        assert_eq!(writes.len(), 1);
        match &writes[0] {
            PullPushWrite::Update { sheet_row_number, cells } => {
                assert_eq!(*sheet_row_number, 2, "row 2 is the first (and only) data row");
                let as_map: HashMap<usize, String> = cells.iter().cloned().collect();
                assert_eq!(as_map.get(&11), Some(&"60.00".to_string()));
            }
            _ => panic!("expected an Update write"),
        }

        let stored: String = conn
            .query_row(
                "SELECT last_synced_snapshot FROM sheet_sync_links WHERE data_source='pulls' AND local_id=?1",
                params![id],
                |r| r.get(0),
            )
            .unwrap();
        assert!(stored.contains("6000"), "the link's stored snapshot must advance to the new price: {stored}");
    }

    #[test]
    fn a_sheet_side_change_since_the_last_push_blocks_the_push_and_is_reported_as_a_conflict() {
        let conn = test_conn();
        let (id, mut row) = push_once_seeds_a_linked_pull(&conn);
        // marko edited the price directly in the sheet, never synced back in
        // - the local pull itself never changed, but a push run only ever
        // pushes, it can never pull that sheet-side change in, so it must
        // refuse rather than clobber it.
        row[11] = "99.00".to_string();

        let (result, writes) = apply_pull_push(&conn, &full_headers(), &[row], "EUR", MARKER_COL).unwrap();
        assert_eq!(result.conflicts.len(), 1);
        assert_eq!(result.conflicts[0].row_number, 2);
        assert!(result.conflicts[0].message.contains("Sync from sheet"));
        assert!(writes.is_empty());

        let stored: String = conn
            .query_row(
                "SELECT last_synced_snapshot FROM sheet_sync_links WHERE data_source='pulls' AND local_id=?1",
                params![id],
                |r| r.get(0),
            )
            .unwrap();
        assert!(stored.contains("4500"), "a conflict must never advance the stored snapshot: {stored}");
    }

    #[test]
    fn a_row_deleted_from_the_sheet_since_linking_is_reported_and_never_silently_reappended() {
        let conn = test_conn();
        push_once_seeds_a_linked_pull(&conn);

        // The sheet no longer has any row carrying this pull's marker.
        let (result, writes) = apply_pull_push(&conn, &full_headers(), &[], "EUR", MARKER_COL).unwrap();
        assert_eq!(result.created, 0, "must never re-append a row that was deliberately deleted from the sheet");
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].message.contains("could not be found"));
        assert!(writes.is_empty());
    }

    // -----------------------------------------------------------------------
    // Auto-create-and-share (2.0.4). `create_pulls_sheet_impl` itself calls
    // out to Google (create + write header + share) once validation passes,
    // so - same limitation as `sync_pulls_impl` - only the parts before that
    // first network call are exercised here: input validation, and that a
    // fully valid call still fails cleanly rather than reaching the network
    // when this test build has no service account embedded (see
    // google_sheets.rs's embedded_service_account_is_none_on_a_plain_local_build
    // test, which the last test below relies on).
    // -----------------------------------------------------------------------

    #[test]
    fn pulls_sheet_headers_satisfy_the_required_header_check() {
        // Regression guard: if apply_pull_rows's required-header list ever
        // grows, a freshly auto-created sheet must still pass its own sync
        // immediately, with zero manual editing needed first.
        let headers: Vec<String> = PULLS_SHEET_HEADERS.iter().map(|s| s.to_string()).collect();
        let map = build_header_map(&headers);
        assert!(check_required_headers(&map).is_ok(), "a freshly auto-created sheet must satisfy its own required columns");
    }

    #[test]
    fn validate_share_email_accepts_ordinary_addresses() {
        for ok in ["marko@example.com", "  marko@example.com  ", "a@b.co"] {
            assert!(validate_share_email(ok).is_ok(), "'{ok}' must be accepted");
        }
    }

    #[test]
    fn validate_share_email_rejects_empty_missing_at_or_whitespace() {
        for bad in ["", "   ", "not-an-email", "@example.com", "marko@", "mar ko@example.com", "marko@exa mple.com"] {
            assert!(validate_share_email(bad).is_err(), "'{bad}' must be rejected");
        }
    }

    #[test]
    fn validate_currency_accepts_only_eur_usd_gbp_and_uppercases() {
        for ok in ["EUR", "usd", "Gbp"] {
            assert_eq!(validate_currency(ok).unwrap(), ok.to_uppercase());
        }
        for bad in ["CZK", "", "   ", "EURO"] {
            assert!(validate_currency(bad).is_err(), "'{bad}' must be rejected");
        }
    }

    #[test]
    fn create_pulls_sheet_rejects_a_bad_email_before_touching_anything_else() {
        let conn = test_conn();
        let err = create_pulls_sheet_impl(&conn, "not-an-email", "EUR").unwrap_err();
        assert!(err.to_string().to_lowercase().contains("email"), "the error must mention the actual problem: {err}");
    }

    #[test]
    fn create_pulls_sheet_rejects_a_bad_currency_before_touching_anything_else() {
        let conn = test_conn();
        let err = create_pulls_sheet_impl(&conn, "marko@example.com", "CZK").unwrap_err();
        assert!(err.to_string().to_lowercase().contains("currency"), "the error must mention the actual problem: {err}");
    }

    #[test]
    fn create_pulls_sheet_with_valid_input_fails_cleanly_when_no_service_account_is_embedded() {
        let conn = test_conn();
        let err = create_pulls_sheet_impl(&conn, "marko@example.com", "EUR").unwrap_err();
        assert!(
            err.to_string().contains("isn't available in this build"),
            "fully valid input must still stop cleanly before any network call in a test build: {err}"
        );
    }

    #[test]
    fn setup_pulls_sheet_rejects_up_front_when_nothing_is_connected_yet() {
        let conn = test_conn();
        let err = setup_pulls_sheet_impl(&conn).unwrap_err();
        assert!(
            err.to_string().contains("No spreadsheet is connected"),
            "must fail with the same clear message sync/push already use, not a generic error: {err}"
        );
    }

    #[test]
    fn setup_pulls_sheet_with_a_real_connection_fails_cleanly_when_no_service_account_is_embedded() {
        let conn = test_conn();
        set_sheets_connection_impl(&conn, "pulls", "1AbC-XyZ_9900", "Pulls", "EUR").unwrap();
        let err = setup_pulls_sheet_impl(&conn).unwrap_err();
        assert!(
            err.to_string().contains("isn't available in this build"),
            "a real connection must reach the credential step (not panic/short-circuit some other way) and then stop cleanly before any network call in a test build: {err}"
        );
    }

    // -----------------------------------------------------------------------
    // Sheet structure (Platform + Transfer dropdowns), 2.0.21 - mirrors
    // orders_sheet_sync's own "Sheet structure" test section (2.0.19).
    // `full_headers()` above already has both "Platform" (index 4) and
    // "Transfer" (index 10), so it's reused as-is rather than a second
    // near-identical fixture.
    // -----------------------------------------------------------------------

    fn dropdown_values<'a>(dropdowns: &'a [DropdownSpec], headers: &[String], header_name: &str) -> Option<&'a Vec<String>> {
        let col = headers.iter().position(|h| h.eq_ignore_ascii_case(header_name))?;
        dropdowns.iter().find(|d| d.col_index == col).map(|d| &d.values)
    }

    #[test]
    fn plan_pulls_sheet_structure_updates_transfer_options_are_exactly_yes_no() {
        let conn = test_conn();
        let headers = full_headers();
        let dropdowns = plan_pulls_sheet_structure_updates(&conn, &headers).unwrap();
        assert_eq!(dropdown_values(&dropdowns, &headers, "Transfer"), Some(&vec!["Yes".to_string(), "No".to_string()]));
    }

    #[test]
    fn plan_pulls_sheet_structure_updates_platform_options_are_platforms_tagged_purchase_or_both_only() {
        let conn = test_conn();
        conn.execute("INSERT INTO platforms(name, kind) VALUES ('TicketMaster', 'purchase')", []).unwrap();
        conn.execute("INSERT INTO platforms(name, kind) VALUES ('Resell4U', 'both')", []).unwrap();
        conn.execute("INSERT INTO platforms(name, kind) VALUES ('SaleOnlyCo', 'sale')", []).unwrap();

        let headers = full_headers();
        let dropdowns = plan_pulls_sheet_structure_updates(&conn, &headers).unwrap();
        let values = dropdown_values(&dropdowns, &headers, "Platform").unwrap();
        assert_eq!(values, &vec!["Resell4U".to_string(), "TicketMaster".to_string()], "sale-only platform must be excluded");
    }

    #[test]
    fn plan_pulls_sheet_structure_updates_skips_the_platform_dropdown_when_there_are_no_purchase_platforms_yet() {
        let conn = test_conn();
        let headers = full_headers();
        let dropdowns = plan_pulls_sheet_structure_updates(&conn, &headers).unwrap();
        assert!(dropdown_values(&dropdowns, &headers, "Platform").is_none());
    }

    #[test]
    fn plan_pulls_sheet_structure_updates_skips_a_dropdown_column_the_sheet_does_not_have() {
        let conn = test_conn();
        conn.execute("INSERT INTO platforms(name, kind) VALUES ('TicketMaster', 'purchase')", []).unwrap();
        // Only "Transfer" exists - no Platform column anywhere in this sheet.
        let headers: Vec<String> = vec!["Transfer".to_string()];
        let dropdowns = plan_pulls_sheet_structure_updates(&conn, &headers).unwrap();
        assert_eq!(dropdowns.len(), 1);
        assert_eq!(dropdowns[0].col_index, 0);
    }

    #[test]
    fn plan_pulls_sheet_structure_updates_leaves_every_other_column_alone() {
        // marko's own request listed exactly these two columns for a
        // dropdown - everything else (pull/Event name/event date/Ks/More
        // info/Section/Row/Seats/Price/the unnamed blank column/date) must
        // never appear here, no matter what full_headers() contains.
        let conn = test_conn();
        conn.execute("INSERT INTO platforms(name, kind) VALUES ('TicketMaster', 'purchase')", []).unwrap();
        let headers = full_headers();
        let dropdowns = plan_pulls_sheet_structure_updates(&conn, &headers).unwrap();
        assert_eq!(dropdowns.len(), 2, "exactly Platform and Transfer, nothing else");
    }

    // -----------------------------------------------------------------------
    // Color-coding (2.0.22) - marko's own request: "yes zelenou, nie
    // modrou" for Transfer - never Platform, which he did not mention.
    // -----------------------------------------------------------------------

    fn color_values<'a>(specs: &'a [ColorSpec], headers: &[String], header_name: &str) -> Option<&'a Vec<(String, (f64, f64, f64))>> {
        let col = headers.iter().position(|h| h.eq_ignore_ascii_case(header_name))?;
        specs.iter().find(|s| s.col_index == col).map(|s| &s.colors)
    }

    #[test]
    fn plan_pulls_sheet_color_updates_transfer_colors_are_exactly_yes_green_no_blue() {
        let headers = full_headers();
        let specs = plan_pulls_sheet_color_updates(&headers);
        assert_eq!(
            color_values(&specs, &headers, "Transfer"),
            Some(&vec![("Yes".to_string(), COLOR_GREEN), ("No".to_string(), COLOR_BLUE)])
        );
    }

    #[test]
    fn plan_pulls_sheet_color_updates_skips_when_transfer_column_is_absent() {
        let headers: Vec<String> = vec!["Platform".to_string()];
        let specs = plan_pulls_sheet_color_updates(&headers);
        assert!(specs.is_empty());
    }

    #[test]
    fn plan_pulls_sheet_color_updates_never_colors_platform() {
        // marko only asked for Transfer's colors on this sheet - Platform
        // already has its own (growable) dropdown but must never gain a
        // color rule too.
        let headers = full_headers();
        let specs = plan_pulls_sheet_color_updates(&headers);
        assert!(color_values(&specs, &headers, "Platform").is_none());
        assert_eq!(specs.len(), 1, "exactly Transfer, nothing else");
    }
}

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
use crate::commands::sheets_sync::{last_synced_key, load_connection, set_setting, set_sheets_connection_impl, ALLOWED_CURRENCIES};
use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::google_sheets;
use crate::models::{CreatedSheetResult, PullEditInput, PullInput, PullsSyncResult, SheetSyncIssue};
use crate::money::parse_decimal_to_cents;
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
    last_synced_snapshot: String,
    last_synced_at: String,
}

fn load_sync_link(conn: &Connection, marker: &str) -> AppResult<Option<SyncLink>> {
    Ok(conn
        .query_row(
            "SELECT local_id, last_synced_snapshot, last_synced_at FROM sheet_sync_links
             WHERE data_source = 'pulls' AND sheet_marker = ?1",
            params![marker],
            |r| {
                Ok(SyncLink {
                    local_id: r.get(0)?,
                    last_synced_snapshot: r.get(1)?,
                    last_synced_at: r.get(2)?,
                })
            },
        )
        .optional()?)
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
) -> AppResult<(PullsSyncResult, Vec<(usize, String)>)> {
    let map = build_header_map(headers);
    check_required_headers(&map)?;

    let buyer_col = find_col(&map, &["pull"]);
    let event_name_col = find_col(&map, &["event name", "event"]);
    let event_date_col = find_col(&map, &["event date"]);
    let quantity_col = find_col(&map, &["ks", "quantity", "qty"]);
    let platform_col = find_col(&map, &["platform"]);
    let more_info_col = find_col(&map, &["more info"]);
    let section_col = find_col(&map, &["section", "sector"]);
    let row_label_col = find_col(&map, &["row"]);
    let seat_col = find_col(&map, &["seats", "seat"]);
    let transfer_col = find_col(&map, &["transfer"]);
    let price_col = find_col(&map, &["price"]);

    let mut result = PullsSyncResult {
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

        let buyer_raw = cell(raw_row, buyer_col);
        let event_name_raw = cell(raw_row, event_name_col);
        let quantity_raw = cell(raw_row, quantity_col);
        let price_raw = cell(raw_row, price_col);

        // A fully blank row (nothing in any required column) is just a gap
        // in the sheet, not a mistake - skip it without comment.
        if buyer_raw.is_none() && event_name_raw.is_none() && quantity_raw.is_none() && price_raw.is_none() {
            continue;
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
            result.errors.push(SheetSyncIssue { row_number, message: row_errors.join("; ") });
            continue;
        }

        let transfer_done = cell(raw_row, transfer_col).map(|s| parse_transfer_done(&s)).unwrap_or(false);
        let platform_name = cell(raw_row, platform_col);
        let more_info = cell(raw_row, more_info_col);
        let section = cell(raw_row, section_col);
        let row_label = cell(raw_row, row_label_col);
        let seat = cell(raw_row, seat_col);

        let platform_id = match &platform_name {
            Some(name) => match resolve_or_create_platform(conn, name) {
                Ok(id) => Some(id),
                Err(e) => {
                    result.errors.push(SheetSyncIssue { row_number, message: format!("platform '{name}': {e}") });
                    continue;
                }
            },
            None => None,
        };

        let parsed = ParsedRow {
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
// The network-calling shell - fetches the sheet, calls apply_pull_rows
// above, writes back whatever markers it asked for. See google_sheets.rs's
// module doc comment for why this half can't be exercised in this sandbox.
// ---------------------------------------------------------------------------

fn sync_pulls_impl(conn: &Connection) -> AppResult<PullsSyncResult> {
    let account = google_sheets::embedded_service_account().ok_or_else(|| {
        AppError::External("Google Sheets sync isn't available in this build (no service account configured).".to_string())
    })?;
    let connection = load_connection(conn, "pulls")?
        .ok_or_else(|| AppError::Validation("No spreadsheet is connected for Pulls yet - connect one in Settings first.".to_string()))?;

    let token = google_sheets::fetch_access_token(&account, google_sheets::SHEETS_SCOPE)?;
    let range = format!("{}!A1:Z", connection.sheet_tab);
    let value_range = google_sheets::get_values(&token, &connection.spreadsheet_id, &range)?;
    if value_range.values.is_empty() {
        return Err(AppError::Validation("The connected sheet/tab has no header row yet.".to_string()));
    }
    let headers = value_range.values[0].clone();
    let data_rows: &[Vec<String>] = if value_range.values.len() > 1 { &value_range.values[1..] } else { &[] };

    let (marker_col_index, marker_exists) = resolve_marker_column(&headers);
    let letter = column_index_to_a1(marker_col_index);
    if !marker_exists {
        let header_range = format!("{}!{letter}1", connection.sheet_tab);
        google_sheets::update_values(&token, &connection.spreadsheet_id, &header_range, &[vec![MARKER_HEADER.to_string()]])?;
    }

    let (mut result, marker_writes) = apply_pull_rows(conn, &headers, data_rows, &connection.currency, marker_col_index)?;

    for (row_idx, marker_value) in marker_writes {
        let sheet_row_number = (row_idx + 2) as i64;
        let cell_range = format!("{}!{letter}{sheet_row_number}", connection.sheet_tab);
        if let Err(e) = google_sheets::update_values(&token, &connection.spreadsheet_id, &cell_range, &[vec![marker_value]]) {
            result.errors.push(SheetSyncIssue {
                row_number: sheet_row_number,
                message: format!("saved in the app, but could not write its ID back to the sheet: {e}"),
            });
        }
    }

    result.synced_at = now_iso(conn)?;
    set_setting(conn, &last_synced_key("pulls"), &result.synced_at)?;
    Ok(result)
}

/// Manual "Sync now" button (Settings -> Integrations, Pulls card). Never
/// runs on its own.
#[tauri::command]
pub fn sync_pulls(state: State<AppState>) -> AppResult<PullsSyncResult> {
    let conn = state.db.lock().unwrap();
    sync_pulls_impl(&conn)
}

// ---------------------------------------------------------------------------
// Auto-create-and-share (2.0.4) - "Create a new sheet for me", the
// alternative to pasting an existing sheet's URL, built for marko's original
// ask: one click, a brand-new Pulls sheet appears already shared with him,
// no Google sign-in window (see google_sheets.rs's `SHEETS_AND_DRIVE_SCOPE`
// doc comment for the full design rationale versus real OAuth). Fully
// additive: the paste-a-URL flow (commands/sheets_sync.rs::
// set_sheets_connection) keeps working exactly as before, unchanged, for
// marko's own historical sheet - this is a second way to arrive at the same
// connected state, not a replacement.
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
fn validate_share_email(email: &str) -> AppResult<String> {
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
fn validate_currency(currency: &str) -> AppResult<String> {
    let upper = currency.trim().to_uppercase();
    if !ALLOWED_CURRENCIES.contains(&upper.as_str()) {
        return Err(AppError::Validation(format!(
            "Currency must be one of {} - got '{currency}'",
            ALLOWED_CURRENCIES.join(", ")
        )));
    }
    Ok(upper)
}

/// Creates a brand-new Google Sheet for Pulls, shares it with `email`,
/// writes `PULLS_SHEET_HEADERS` as its header row, and connects it - all in
/// one call, with no Google sign-in window at any point (the same service
/// account every other connection in this app already uses). `email` and
/// `currency` are fully validated before the first network call - see
/// `validate_share_email`/`validate_currency`'s doc comments for why that
/// ordering matters here specifically.
fn create_pulls_sheet_impl(conn: &Connection, email: &str, currency: &str) -> AppResult<CreatedSheetResult> {
    let email = validate_share_email(email)?;
    let currency_upper = validate_currency(currency)?;

    let account = google_sheets::embedded_service_account().ok_or_else(|| {
        AppError::External("Google Sheets sync isn't available in this build (no service account configured).".to_string())
    })?;
    let token = google_sheets::fetch_access_token(&account, google_sheets::SHEETS_AND_DRIVE_SCOPE)?;

    let created = google_sheets::create_spreadsheet(&token, NEW_SHEET_TITLE, NEW_SHEET_TAB_NAME)?;

    let header_row: Vec<String> = PULLS_SHEET_HEADERS.iter().map(|s| s.to_string()).collect();
    let header_range = format!("{NEW_SHEET_TAB_NAME}!A1");
    google_sheets::update_values(&token, &created.spreadsheet_id, &header_range, &[header_row])?;

    google_sheets::share_file(&token, &created.spreadsheet_id, &email)?;

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
}

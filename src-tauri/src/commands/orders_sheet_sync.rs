//! Orders/Tickets <-> Google Sheet row sync (2.0.8) - the second connected
//! data source in Settings -> Integrations, after Pulls (2.0.3). Reuses the
//! exact same connection layer (commands::sheets_sync) and the exact same
//! `sheet_sync_links` table (migrations/008_sheet_sync.sql was already
//! designed data-source-agnostic for exactly this) with zero schema changes
//! beyond one new `orders.external_reference` column this needs for its own
//! "Order ID" concept (migrations/009_orders_external_reference.sql).
//!
//! marko's real sheet is one combined "buy + sell" tracker: he fills in one
//! batch of columns when he buys tickets, then a second, later batch of
//! columns on the SAME row once they sell. This module only reads the FIRST
//! batch - creating an Order (and one Ticket per unit) from it. The second
//! batch (Sales) is a separate, later sync against the same rows, matched
//! back to these tickets via the same "TIQR ID" marker this module writes.
//!
//! **Creation-only in this pass, deliberately** - unlike Pulls sync
//! (commands::pulls_sheet_sync), which also updates an already-linked row
//! when the sheet changes it. A row here that already carries a "TIQR ID"
//! marker is simply left alone - nothing on it is even parsed again (see
//! `apply_order_rows`). Editing an order's purchase-side numbers after
//! tickets already exist would touch `insert_order_with_tickets`'s exact-cent
//! cost allocation across those tickets - exactly the kind of protected
//! financial/data-integrity logic this project's house rules say not to
//! touch without asking first. Out of scope for v1, the same scope-cut marko
//! already accepted for Pulls sync's own v1.
//!
//! ## Column mapping (first batch only - see module doc comment above)
//!
//! | Sheet column (header, case/spacing-insensitive) | Order/Ticket field |
//! |---|---|
//! | `Event Name` | `Event.name` (resolve-or-create by name - see `resolve_or_create_event`) |
//! | `Date (DD/MM/YYYY)` | BOTH `Event.event_date` (only set when the event is first created) AND `Order.purchase_date` - marko's sheet has no separate "purchase date" in this first batch, and `purchase_date` is required by the schema, so this one column has to serve both |
//! | `platform` | `Order.platform_id` (resolve-or-create by name, same as CSV import/Pulls sync) |
//! | `Section` | `Ticket.section` (same value stamped on every generated ticket - same convention CSV import already uses for a whole order at once) |
//! | `Row` | `Ticket.row_label` (same convention) |
//! | `Seats` | comma-separated, one label per ticket - must match `Number of Tickets` exactly if present, or be left blank entirely (identical rule to CSV import's own `seats` column) |
//! | `Order ID` | `Order.external_reference` - marko's own reference, set via a follow-up UPDATE once the order exists (see migrations/009's doc comment for why not `OrderInput` itself) |
//! | `Total Purchase Price` | not stored anywhere - cross-checked against `Number of Tickets x Price Per Ticket` when present, and the row is rejected on a mismatch rather than silently trusting one number over the other |
//! | `Number of Tickets` | `Order.quantity` |
//! | `Price Per Ticket` | `Order.unit_price_cents` |
//! | `currency` | `Order.currency` - a row's own value if present and one of EUR/USD/GBP, otherwise the connection's configured currency (unlike Pulls, whose sheet has no currency column at all) |
//! | `Email (used)` | folded into `Order.notes` as "Email used: ..." - no dedicated field for this exists anywhere in the schema |
//! | `Ticket Type` | `Order.ticket_type` (existing field, copied onto every generated ticket - unchanged behaviour) |
//! | `TIQR ID` (appended by the app itself the first time it's missing) | the sync marker - never typed by hand |
//!
//! `Event Name`/`Date (DD/MM/YYYY)`/`Number of Tickets`/`Price Per Ticket`
//! are required: a sheet missing any of them fails the whole sync with one
//! clear message up front, same as Pulls sync. Every other column is
//! optional.
//!
//! Auto-creating a missing Event by name is a deliberate departure from CSV
//! import (which requires the event to already exist, on the theory a
//! one-off file gets reviewed by hand first) - a sync is meant to be re-run
//! against a sheet marko is actively filling in, so requiring every event to
//! be pre-created by hand in the app first would defeat the point.

use crate::commands::csv_import::resolve_or_create_platform;
use crate::commands::orders::insert_order_with_tickets;
use crate::commands::pulls_sheet_sync::parse_sheet_serial_date;
use crate::commands::sheets_sync::{
    last_synced_key, load_connection, set_setting, set_sheets_connection_impl, ALLOWED_CURRENCIES,
};
use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::google_sheets;
use crate::models::{CreatedSheetResult, OrderInput, SheetSyncIssue, SheetSyncResult};
use crate::money::{format_cents, parse_decimal_to_cents};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashMap;
use tauri::State;

const MARKER_HEADER: &str = "TIQR ID";

/// The exact label used in required-header error messages and in this
/// module's own field-error messages - kept as one constant so the two never
/// drift apart from each other.
const DATE_HEADER_LABEL: &str = "Date (DD/MM/YYYY)";

const REQUIRED_HEADERS: &[(&str, &[&str])] = &[
    ("\"Event Name\"", &["event name", "event"]),
    ("\"Date (DD/MM/YYYY)\"", &["date (dd/mm/yyyy)", "date"]),
    ("\"Number of Tickets\"", &["number of tickets", "quantity", "qty", "ks"]),
    ("\"Price Per Ticket\"", &["price per ticket", "unit price", "price"]),
];

// ---------------------------------------------------------------------------
// Header matching / marker plumbing - deliberately a plain duplicate of
// pulls_sheet_sync.rs's own small helpers of the same name, not a shared
// import, matching that module's own stated design (kept separate rather
// than shared, since each sync's exact matching rules are free to diverge
// later without risking the other).
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

/// Reads one cell, trimmed, treating blank as absent - same rule
/// pulls_sheet_sync.rs's own `cell` uses (Sheets omits trailing empty cells
/// per row, so a short row and a blank cell both collapse to `None` here).
fn cell(row: &[String], idx: Option<usize>) -> Option<String> {
    let idx = idx?;
    let v = row.get(idx)?.trim();
    if v.is_empty() {
        None
    } else {
        Some(v.to_string())
    }
}

fn resolve_marker_column(headers: &[String]) -> (usize, bool) {
    let map = build_header_map(headers);
    match map.get(&normalize_header(MARKER_HEADER)) {
        Some(&idx) => (idx, true),
        None => (headers.len(), false),
    }
}

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

fn now_iso(conn: &Connection) -> AppResult<String> {
    Ok(conn.query_row("SELECT strftime('%Y-%m-%dT%H:%M:%fZ','now')", [], |r| r.get(0))?)
}

// ---------------------------------------------------------------------------
// Field parsers
// ---------------------------------------------------------------------------

/// Never empty by the time this is called (an empty cell is reported as
/// "missing" by the caller before parsing is even attempted - the column is
/// required, see this module's doc comment). Expects "DD/MM/YYYY" (marko's
/// own header literally says so) - '.' is also accepted as a separator,
/// since that's the format his Pulls sheet already uses, so either habit
/// works here too. A cell someone typed as an actual date (not text) arrives
/// as a bare Sheets/Excel serial-day number instead - recognized here the
/// same way pulls_sheet_sync::parse_sheet_date already does, by reusing its
/// own `parse_sheet_serial_date` rather than re-deriving the same epoch/range
/// logic a second time.
fn parse_order_date(raw: &str) -> Result<String, String> {
    let s = raw.trim();
    if s.is_empty() {
        return Err("is required".to_string());
    }
    if s.chars().all(|c| c.is_ascii_digit()) {
        return match parse_sheet_serial_date(s)? {
            Some(d) => Ok(d),
            None => Err(format!("'{s}' is not a recognized date")),
        };
    }
    let parts: Vec<&str> = s.split(['/', '.']).map(|p| p.trim()).collect();
    if parts.len() != 3 {
        return Err(format!("'{s}' is not a recognized date (expected DD/MM/YYYY)"));
    }
    let day: u32 = parts[0].parse().map_err(|_| format!("'{s}' has an invalid day"))?;
    let month: u32 = parts[1].parse().map_err(|_| format!("'{s}' has an invalid month"))?;
    let year: i32 = parts[2].parse().map_err(|_| format!("'{s}' has an invalid year"))?;
    if !(2000..=2100).contains(&year) {
        return Err(format!("'{s}' has an implausible year"));
    }
    chrono::NaiveDate::from_ymd_opt(year, month, day)
        .map(|d| d.format("%Y-%m-%d").to_string())
        .ok_or_else(|| format!("'{s}' is not a valid calendar date"))
}

/// "8" -> 8. Empty/non-numeric/zero-or-negative is reported rather than
/// guessed at. The same upper bound (50000) is enforced again later, inside
/// insert_order_with_tickets's own validate_order_input, so it isn't
/// duplicated here.
fn parse_ticket_count(raw: &str) -> Result<i64, String> {
    let s = raw.trim();
    let n: i64 = s.parse().map_err(|_| format!("'{s}' is not a whole number"))?;
    if n <= 0 {
        return Err("must be at least 1".to_string());
    }
    Ok(n)
}

/// Case-insensitive find-or-create by name, mirroring
/// `resolve_or_create_platform`'s own convention - but unlike CSV import
/// (which requires the event to already exist, on the theory a first-time
/// CSV import is reviewed by hand before committing), a live sync is meant
/// to be clicked repeatedly against a sheet marko is actively filling in, so
/// requiring him to pre-create every event in the app first would defeat the
/// point. A freshly created event gets this row's own date and otherwise
/// default fields (status "upcoming") - exactly like typing only the
/// name+date into "New event" by hand. An event that already exists by name
/// is reused as-is and its date is deliberately left untouched even if a
/// later row's date differs - see this function's own tests.
fn resolve_or_create_event(conn: &Connection, name: &str, event_date: &str) -> AppResult<i64> {
    if let Some(id) = conn
        .query_row("SELECT id FROM events WHERE LOWER(name) = LOWER(?1)", [name], |r| r.get(0))
        .optional()?
    {
        return Ok(id);
    }
    conn.execute(
        "INSERT INTO events (name, event_date, status) VALUES (?1, ?2, 'upcoming')",
        params![name, event_date],
    )?;
    Ok(conn.last_insert_rowid())
}

// ---------------------------------------------------------------------------
// The core - no network call anywhere in this function, which is what makes
// it directly unit-testable with a plain in-memory `test_conn()`. The
// network-calling `sync_orders_impl` below fetches the rows via
// `google_sheets::get_values`, then calls this.
// ---------------------------------------------------------------------------

/// Applies already-fetched sheet rows, creating a new Order (with its
/// Tickets) for every row that doesn't yet carry a "TIQR ID" marker. A row
/// that already carries one is left alone entirely - v1 is creation-only
/// (see this module's doc comment), so nothing on that row is even parsed,
/// and no platform/event is auto-created on its behalf. Returns the
/// user-facing result summary plus the list of (0-based data row index,
/// marker value to write) pairs the caller still needs to write back to the
/// actual sheet - this function itself never talks to Google.
fn apply_order_rows(
    conn: &Connection,
    headers: &[String],
    data_rows: &[Vec<String>],
    connection_currency: &str,
    marker_col_index: usize,
) -> AppResult<(SheetSyncResult, Vec<(usize, String)>)> {
    let map = build_header_map(headers);
    check_required_headers(&map)?;

    let event_name_col = find_col(&map, &["event name", "event"]);
    let date_col = find_col(&map, &["date (dd/mm/yyyy)", "date"]);
    let platform_col = find_col(&map, &["platform"]);
    let section_col = find_col(&map, &["section"]);
    let row_col = find_col(&map, &["row"]);
    let seats_col = find_col(&map, &["seats", "seat"]);
    let order_ref_col = find_col(&map, &["order id", "orderid"]);
    let total_price_col = find_col(&map, &["total purchase price", "total price"]);
    let quantity_col = find_col(&map, &["number of tickets", "quantity", "qty", "ks"]);
    let unit_price_col = find_col(&map, &["price per ticket", "unit price", "price"]);
    let currency_col = find_col(&map, &["currency"]);
    let email_col = find_col(&map, &["email (used)", "email"]);
    let ticket_type_col = find_col(&map, &["ticket type"]);

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

        // A row that already carries a marker was already synced once - v1
        // never updates it, so it's left alone before anything on it is even
        // parsed (no point auto-creating a platform/event for a row whose
        // values will never be used).
        if cell(raw_row, Some(marker_col_index)).is_some() {
            result.unchanged += 1;
            continue;
        }

        let event_name_raw = cell(raw_row, event_name_col);
        let date_raw = cell(raw_row, date_col);
        let quantity_raw = cell(raw_row, quantity_col);
        let unit_price_raw = cell(raw_row, unit_price_col);

        // A fully blank row (nothing in any required column) is just a gap
        // in the sheet, not a mistake - skip it without comment, same rule
        // Pulls sync already uses.
        if event_name_raw.is_none() && date_raw.is_none() && quantity_raw.is_none() && unit_price_raw.is_none() {
            continue;
        }

        let mut row_errors: Vec<String> = vec![];

        let event_name = event_name_raw.unwrap_or_default();
        if event_name.is_empty() {
            row_errors.push("missing 'Event Name' value".to_string());
        }

        let event_date: Option<String> = match date_raw.as_deref().map(parse_order_date) {
            Some(Ok(d)) => Some(d),
            Some(Err(e)) => {
                row_errors.push(format!("'{DATE_HEADER_LABEL}': {e}"));
                None
            }
            None => {
                row_errors.push(format!("missing '{DATE_HEADER_LABEL}' value"));
                None
            }
        };

        let quantity: Option<i64> = match quantity_raw.as_deref().map(parse_ticket_count) {
            Some(Ok(q)) => Some(q),
            Some(Err(e)) => {
                row_errors.push(format!("'Number of Tickets': {e}"));
                None
            }
            None => {
                row_errors.push("missing 'Number of Tickets' value".to_string());
                None
            }
        };

        let unit_price_cents: Option<i64> = match unit_price_raw.as_deref().map(parse_decimal_to_cents) {
            Some(Ok(v)) if v >= 0 => Some(v),
            Some(Ok(_)) => {
                row_errors.push("'Price Per Ticket' cannot be negative".to_string());
                None
            }
            Some(Err(e)) => {
                row_errors.push(format!("'Price Per Ticket': {e}"));
                None
            }
            None => {
                row_errors.push("missing 'Price Per Ticket' value".to_string());
                None
            }
        };

        // Total Purchase Price is optional, but when present it must agree
        // with Number of Tickets x Price Per Ticket exactly - marko fills in
        // both independently, so a mismatch is almost always a typo in one
        // of the three, and this app's own "never guess, stop and report"
        // rule for money (see money.rs's module doc comment) says to surface
        // it rather than silently trusting one number over the other.
        if let (Some(total_raw), Some(q), Some(unit)) = (cell(raw_row, total_price_col), quantity, unit_price_cents) {
            match parse_decimal_to_cents(&total_raw) {
                Ok(total_cents) => {
                    let expected = unit * q;
                    if total_cents != expected {
                        row_errors.push(format!(
                            "'Total Purchase Price' ({}) does not match Number of Tickets x Price Per Ticket ({}) - check these values",
                            format_cents(total_cents),
                            format_cents(expected)
                        ));
                    }
                }
                Err(e) => row_errors.push(format!("'Total Purchase Price': {e}")),
            }
        }

        let currency = match cell(raw_row, currency_col) {
            Some(c) => {
                let upper = c.trim().to_uppercase();
                if ALLOWED_CURRENCIES.contains(&upper.as_str()) {
                    upper
                } else {
                    row_errors.push(format!("'currency' must be one of {} - got '{c}'", ALLOWED_CURRENCIES.join(", ")));
                    connection_currency.to_string()
                }
            }
            None => connection_currency.to_string(),
        };

        let seats: Option<Vec<String>> = cell(raw_row, seats_col).map(|s| {
            s.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect::<Vec<_>>()
        }).filter(|v| !v.is_empty());
        if let (Some(seat_list), Some(q)) = (&seats, quantity) {
            if seat_list.len() as i64 != q {
                row_errors.push(format!(
                    "'Seats' has {} value(s) but Number of Tickets is {} - provide one seat per ticket or leave 'Seats' empty",
                    seat_list.len(),
                    q
                ));
            }
        }

        if !row_errors.is_empty() {
            result.errors.push(SheetSyncIssue { row_number, message: row_errors.join("; ") });
            continue;
        }

        let quantity = quantity.unwrap();
        let unit_price_cents = unit_price_cents.unwrap();
        let purchase_date = event_date.unwrap();

        let platform_name = cell(raw_row, platform_col);
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

        let event_id = match resolve_or_create_event(conn, &event_name, &purchase_date) {
            Ok(id) => id,
            Err(e) => {
                result.errors.push(SheetSyncIssue { row_number, message: format!("event '{event_name}': {e}") });
                continue;
            }
        };

        let email = cell(raw_row, email_col);
        let input = OrderInput {
            event_id,
            supplier_id: None,
            platform_id,
            purchase_date,
            quantity,
            unit_price_cents,
            fees_cents: 0,
            other_costs_cents: 0,
            currency,
            payment_status: None,
            notes: email.as_ref().map(|e| format!("Email used: {e}")),
            ticket_type: cell(raw_row, ticket_type_col),
            section: cell(raw_row, section_col),
            row_label: cell(raw_row, row_col),
            seats,
        };

        match insert_order_with_tickets(conn, &input, false) {
            Ok(order_id) => {
                let code: String = match conn.query_row("SELECT code FROM orders WHERE id = ?1", [order_id], |r| r.get(0)) {
                    Ok(c) => c,
                    Err(e) => {
                        result.errors.push(SheetSyncIssue {
                            row_number,
                            message: format!("order was created but could not be read back: {e}"),
                        });
                        continue;
                    }
                };
                if let Some(reference) = cell(raw_row, order_ref_col) {
                    if let Err(e) =
                        conn.execute("UPDATE orders SET external_reference = ?1 WHERE id = ?2", params![reference, order_id])
                    {
                        result.errors.push(SheetSyncIssue {
                            row_number,
                            message: format!("order {code} was created but its Order ID could not be saved: {e}"),
                        });
                    }
                }
                // No real snapshot to store yet - v1 never compares against
                // one (no update path exists), so '{}' is just a valid-JSON
                // placeholder. A future update-aware version of this sync
                // would replace it with a real one, same shape as Pulls
                // sync's own PullRowSnapshot.
                let now = now_iso(conn)?;
                conn.execute(
                    "INSERT INTO sheet_sync_links (data_source, local_id, sheet_marker, last_synced_snapshot, last_synced_at)
                     VALUES ('orders', ?1, ?2, '{}', ?3)",
                    params![order_id, code, now],
                )?;
                result.created += 1;
                marker_writes.push((i, code));
            }
            Err(e) => {
                result.errors.push(SheetSyncIssue { row_number, message: e.to_string() });
            }
        }
    }

    Ok((result, marker_writes))
}

// ---------------------------------------------------------------------------
// The network-calling shell - fetches the sheet, calls apply_order_rows
// above, writes back whatever markers it asked for. Mechanically the same
// shape as pulls_sheet_sync::sync_pulls_impl, just for data_source "orders".
// ---------------------------------------------------------------------------

fn sync_orders_impl(conn: &Connection) -> AppResult<SheetSyncResult> {
    let connection = load_connection(conn, "orders")?
        .ok_or_else(|| AppError::Validation("No spreadsheet is connected for Orders yet - connect one in Settings first.".to_string()))?;
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

    let (mut result, marker_writes) = apply_order_rows(conn, &headers, data_rows, &connection.currency, marker_col_index)?;

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

    result.synced_at = now_iso(conn)?;
    set_setting(conn, &last_synced_key("orders"), &result.synced_at)?;
    Ok(result)
}

/// Manual "Sync now" button (Settings -> Integrations, Orders & Tickets
/// card). Never runs on its own.
#[tauri::command]
pub fn sync_orders(state: State<AppState>) -> AppResult<SheetSyncResult> {
    let conn = state.db.lock().unwrap();
    sync_orders_impl(&conn)
}

// ---------------------------------------------------------------------------
// "Create a new sheet for me" (2.0.9) - mirrors pulls_sheet_sync.rs's own
// PULLS_SHEET_HEADERS/NEW_SHEET_TITLE/NEW_SHEET_TAB_NAME/
// create_pulls_sheet_impl/create_pulls_sheet exactly, reusing that module's
// validate_share_email/validate_currency directly (see their doc comments)
// rather than duplicating them - only the header list, sheet name, and
// data_source string differ.
// ---------------------------------------------------------------------------

/// Header row written into a freshly-created "Orders & Tickets" sheet -
/// exactly the columns `apply_order_rows` above understands, in the same
/// order as the mapping table in this module's doc comment (and the exact
/// order this module's own `full_headers()` test fixture already uses, so a
/// freshly auto-created sheet and the test fixtures can never quietly drift
/// apart from each other). Deliberately excludes `TIQR ID` - sync appends
/// that itself the first time it's missing, see `resolve_marker_column` -
/// same reasoning as `PULLS_SHEET_HEADERS`.
const ORDERS_SHEET_HEADERS: &[&str] = &[
    "Event Name",
    "Date (DD/MM/YYYY)",
    "platform",
    "Section",
    "Row",
    "Seats",
    "Order ID",
    "Total Purchase Price",
    "Number of Tickets",
    "Price Per Ticket",
    "currency",
    "Email (used)",
    "Ticket Type",
];

const NEW_SHEET_TITLE: &str = "TIQR Manager - Orders";
const NEW_SHEET_TAB_NAME: &str = "Orders";

/// Creates a brand-new Google Sheet for Orders & Tickets, writes
/// `ORDERS_SHEET_HEADERS` as its header row, and connects it - all in one
/// call, with no Google sign-in window at any point. See
/// `pulls_sheet_sync::create_pulls_sheet_impl`'s doc comment (this function
/// mirrors it line for line) for why `email`/`currency` are validated before
/// the first network call, and for the OAuth-vs-service-account share-step
/// distinction.
fn create_orders_sheet_impl(conn: &Connection, email: &str, currency: &str) -> AppResult<CreatedSheetResult> {
    let email = crate::commands::pulls_sheet_sync::validate_share_email(email)?;
    let currency_upper = crate::commands::pulls_sheet_sync::validate_currency(currency)?;

    let credential = crate::commands::google_auth::resolve_google_credential(conn, true)?;
    let token = credential.access_token();

    let created = google_sheets::create_spreadsheet(token, NEW_SHEET_TITLE, NEW_SHEET_TAB_NAME)?;

    let header_row: Vec<String> = ORDERS_SHEET_HEADERS.iter().map(|s| s.to_string()).collect();
    let header_range = google_sheets::a1_range(NEW_SHEET_TAB_NAME, "A1");
    google_sheets::update_values(token, &created.spreadsheet_id, &header_range, &[header_row])?;

    if !credential.is_oauth() {
        google_sheets::share_file(token, &created.spreadsheet_id, &email)?;
    }

    let connection =
        set_sheets_connection_impl(conn, "orders", &created.spreadsheet_id, NEW_SHEET_TAB_NAME, &currency_upper)?;

    Ok(CreatedSheetResult { connection, spreadsheet_url: created.spreadsheet_url })
}

/// "Create a new sheet for me" button (Settings -> Integrations, Orders &
/// Tickets card) - sits right next to the existing paste-a-URL form as a
/// second way to connect, not a replacement for it. Never runs on its own.
#[tauri::command]
pub fn create_orders_sheet(state: State<AppState>, email: String, currency: String) -> AppResult<CreatedSheetResult> {
    let conn = state.db.lock().unwrap();
    create_orders_sheet_impl(&conn, &email, &currency)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_conn;

    // Header order mirrors marko's own real sheet - see this module's doc
    // comment table for the exact mapping.
    fn full_headers() -> Vec<String> {
        vec![
            "Event Name", "Date (DD/MM/YYYY)", "platform", "Section", "Row", "Seats", "Order ID",
            "Total Purchase Price", "Number of Tickets", "Price Per Ticket", "currency", "Email (used)", "Ticket Type",
        ]
        .into_iter()
        .map(String::from)
        .collect()
    }

    fn row(cells: &[&str]) -> Vec<String> {
        cells.iter().map(|s| s.to_string()).collect()
    }

    fn sample_row(marker: &str) -> Vec<String> {
        // Event Name, Date, platform, Section, Row, Seats, Order ID,
        // Total Purchase Price, Number of Tickets, Price Per Ticket,
        // currency, Email (used), Ticket Type
        let mut r = row(&[
            "Coldplay Arena Show",
            "15/09/2026",
            "ticketmaster",
            "410",
            "25",
            "11,12",
            "TM-88213",
            "100.00",
            "2",
            "50.00",
            "EUR",
            "buyer@example.com",
            "e-ticket",
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

    #[test]
    fn headers_with_marker_helper_lines_up_with_marker_col_constant() {
        assert_eq!(headers_with_marker().len() - 1, MARKER_COL);
    }

    // ---- header plumbing --------------------------------------------------

    #[test]
    fn missing_required_headers_fail_the_whole_sync_with_one_clear_message() {
        let conn = test_conn();
        let headers: Vec<String> = vec!["platform".to_string(), "Section".to_string()];
        let err = apply_order_rows(&conn, &headers, &[], "EUR", 2).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Event Name"), "{msg}");
        assert!(msg.contains("Date"), "{msg}");
        assert!(msg.contains("Number of Tickets"), "{msg}");
        assert!(msg.contains("Price Per Ticket"), "{msg}");
    }

    // ---- field parsers ------------------------------------------------------

    #[test]
    fn parse_order_date_handles_slash_and_dot_separated_formats() {
        assert_eq!(parse_order_date("15/09/2026").unwrap(), "2026-09-15");
        assert_eq!(parse_order_date("15.09.2026").unwrap(), "2026-09-15");
    }

    #[test]
    fn parse_order_date_converts_a_sheets_serial_date_number() {
        // Same real-world shape that used to crash Sync now before 2.0.7's
        // fix - see REDESIGN-2.0.7-REPORT.md and
        // pulls_sheet_sync::parse_sheet_serial_date.
        assert_eq!(parse_order_date("46291").unwrap(), "2026-09-26");
    }

    #[test]
    fn parse_order_date_rejects_empty_since_the_column_is_required() {
        assert!(parse_order_date("").is_err());
        assert!(parse_order_date("   ").is_err());
    }

    #[test]
    fn parse_order_date_rejects_invalid_calendar_dates_and_garbage() {
        assert!(parse_order_date("31/02/2026").is_err(), "February never has 31 days");
        assert!(parse_order_date("IDK").is_err());
    }

    #[test]
    fn parse_ticket_count_accepts_plain_numbers() {
        assert_eq!(parse_ticket_count("8").unwrap(), 8);
        assert_eq!(parse_ticket_count(" 2 ").unwrap(), 2);
    }

    #[test]
    fn parse_ticket_count_rejects_zero_and_garbage() {
        assert!(parse_ticket_count("0").is_err());
        assert!(parse_ticket_count("-1").is_err());
        assert!(parse_ticket_count("two").is_err());
    }

    // ---- apply_order_rows: creating -----------------------------------------

    #[test]
    fn a_brand_new_row_with_no_marker_creates_an_order_with_tickets_and_asks_for_a_marker_write() {
        let conn = test_conn();
        let (result, writes) = apply_order_rows(&conn, &full_headers(), &[sample_row("")], "EUR", MARKER_COL).unwrap();
        assert_eq!(result.created, 1);
        assert_eq!(result.updated, 0);
        assert_eq!(result.errors.len(), 0, "errors: {:?}", result.errors);
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].0, 0);
        assert!(writes[0].1.starts_with("ORD-"));

        let order_count: i64 = conn.query_row("SELECT COUNT(*) FROM orders", [], |r| r.get(0)).unwrap();
        assert_eq!(order_count, 1);
        let ticket_count: i64 = conn.query_row("SELECT COUNT(*) FROM tickets", [], |r| r.get(0)).unwrap();
        assert_eq!(ticket_count, 2, "Number of Tickets was 2");
    }

    #[test]
    fn a_created_orders_fields_match_exactly_what_was_parsed() {
        let conn = test_conn();
        apply_order_rows(&conn, &full_headers(), &[sample_row("")], "EUR", MARKER_COL).unwrap();

        let (event_id, purchase_date, currency, quantity, unit_price_cents, notes): (i64, String, String, i64, i64, Option<String>) = conn
            .query_row(
                "SELECT event_id, purchase_date, currency, quantity, unit_price_cents, notes FROM orders WHERE id = 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
            )
            .unwrap();
        assert_eq!(purchase_date, "2026-09-15");
        assert_eq!(currency, "EUR");
        assert_eq!(quantity, 2);
        assert_eq!(unit_price_cents, 5000);
        assert_eq!(notes.as_deref(), Some("Email used: buyer@example.com"));

        let event_name: String = conn.query_row("SELECT name FROM events WHERE id = ?1", [event_id], |r| r.get(0)).unwrap();
        assert_eq!(event_name, "Coldplay Arena Show");

        let (section, row_label, ticket_type): (Option<String>, Option<String>, Option<String>) = conn
            .query_row("SELECT section, row_label, ticket_type FROM tickets WHERE order_id = 1 LIMIT 1", [], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })
            .unwrap();
        assert_eq!(section.as_deref(), Some("410"));
        assert_eq!(row_label.as_deref(), Some("25"));
        assert_eq!(ticket_type.as_deref(), Some("e-ticket"));
    }

    #[test]
    fn seats_are_assigned_one_per_ticket_from_the_seats_column() {
        let conn = test_conn();
        apply_order_rows(&conn, &full_headers(), &[sample_row("")], "EUR", MARKER_COL).unwrap();
        let mut stmt = conn.prepare("SELECT seat FROM tickets ORDER BY id").unwrap();
        let seats: Vec<Option<String>> = stmt.query_map([], |r| r.get(0)).unwrap().collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(seats, vec![Some("11".to_string()), Some("12".to_string())]);
    }

    #[test]
    fn seat_count_mismatch_is_reported_and_nothing_is_created() {
        let conn = test_conn();
        let mut cells = sample_row("");
        cells[5] = "11,12,13".to_string(); // Seats - 3 values but quantity is 2
        let (result, _) = apply_order_rows(&conn, &full_headers(), &[cells], "EUR", MARKER_COL).unwrap();
        assert_eq!(result.created, 0);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].message.contains("Seats"), "{}", result.errors[0].message);
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM orders", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn matching_total_purchase_price_is_accepted() {
        let conn = test_conn();
        let (result, _) = apply_order_rows(&conn, &full_headers(), &[sample_row("")], "EUR", MARKER_COL).unwrap();
        assert_eq!(result.created, 1);
        assert_eq!(result.errors.len(), 0);
    }

    #[test]
    fn mismatched_total_purchase_price_is_reported_and_nothing_is_created() {
        let conn = test_conn();
        let mut cells = sample_row("");
        cells[7] = "999.00".to_string(); // Total Purchase Price - does not match 2 x 50.00
        let (result, _) = apply_order_rows(&conn, &full_headers(), &[cells], "EUR", MARKER_COL).unwrap();
        assert_eq!(result.created, 0);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].message.contains("Total Purchase Price"), "{}", result.errors[0].message);
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM orders", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn missing_total_purchase_price_skips_the_cross_check() {
        let conn = test_conn();
        let mut cells = sample_row("");
        cells[7] = "".to_string(); // Total Purchase Price left blank
        let (result, _) = apply_order_rows(&conn, &full_headers(), &[cells], "EUR", MARKER_COL).unwrap();
        assert_eq!(result.created, 1, "a blank Total Purchase Price must not block creation");
        assert_eq!(result.errors.len(), 0);
    }

    #[test]
    fn blank_currency_cell_falls_back_to_the_connection_currency() {
        let conn = test_conn();
        let mut cells = sample_row("");
        cells[10] = "".to_string(); // currency column blank
        apply_order_rows(&conn, &full_headers(), &[cells], "GBP", MARKER_COL).unwrap();
        let currency: String = conn.query_row("SELECT currency FROM orders WHERE id = 1", [], |r| r.get(0)).unwrap();
        assert_eq!(currency, "GBP");
    }

    #[test]
    fn a_rows_own_currency_overrides_the_connection_currency() {
        let conn = test_conn();
        apply_order_rows(&conn, &full_headers(), &[sample_row("")], "GBP", MARKER_COL).unwrap(); // row itself says EUR
        let currency: String = conn.query_row("SELECT currency FROM orders WHERE id = 1", [], |r| r.get(0)).unwrap();
        assert_eq!(currency, "EUR", "the row's own currency column must win over the connection's default");
    }

    #[test]
    fn an_invalid_currency_value_is_reported() {
        let conn = test_conn();
        let mut cells = sample_row("");
        cells[10] = "CZK".to_string();
        let (result, _) = apply_order_rows(&conn, &full_headers(), &[cells], "EUR", MARKER_COL).unwrap();
        assert_eq!(result.created, 0);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].message.contains("currency"), "{}", result.errors[0].message);
    }

    #[test]
    fn creating_resolves_platform_case_insensitively_and_creates_it_when_missing() {
        let conn = test_conn();
        conn.execute("INSERT INTO platforms(name, kind) VALUES ('TicketMaster', 'purchase')", []).unwrap();
        apply_order_rows(&conn, &full_headers(), &[sample_row("")], "EUR", MARKER_COL).unwrap();
        let platform_count: i64 = conn.query_row("SELECT COUNT(*) FROM platforms", [], |r| r.get(0)).unwrap();
        assert_eq!(platform_count, 1, "must reuse the existing platform, not create a duplicate");
    }

    #[test]
    fn creating_auto_creates_a_missing_event_with_the_rows_own_date() {
        let conn = test_conn();
        let before: i64 = conn.query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0)).unwrap();
        assert_eq!(before, 0);
        apply_order_rows(&conn, &full_headers(), &[sample_row("")], "EUR", MARKER_COL).unwrap();
        let (name, event_date): (String, Option<String>) =
            conn.query_row("SELECT name, event_date FROM events WHERE id = 1", [], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        assert_eq!(name, "Coldplay Arena Show");
        assert_eq!(event_date.as_deref(), Some("2026-09-15"));
    }

    #[test]
    fn a_second_order_for_an_existing_event_reuses_it_and_never_touches_its_date() {
        let conn = test_conn();
        apply_order_rows(&conn, &full_headers(), &[sample_row("")], "EUR", MARKER_COL).unwrap();
        let mut second = sample_row("");
        second[1] = "20/09/2026".to_string(); // a different date on a second row for the same event name
        apply_order_rows(&conn, &full_headers(), &[second], "EUR", MARKER_COL).unwrap();

        let event_count: i64 = conn.query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0)).unwrap();
        assert_eq!(event_count, 1, "must reuse the existing event by name, not create a second one");
        let event_date: Option<String> = conn.query_row("SELECT event_date FROM events WHERE id = 1", [], |r| r.get(0)).unwrap();
        assert_eq!(
            event_date.as_deref(),
            Some("2026-09-15"),
            "the event's own date must stay as first created, never overwritten by a later order's row"
        );

        let order_count: i64 = conn.query_row("SELECT COUNT(*) FROM orders", [], |r| r.get(0)).unwrap();
        assert_eq!(order_count, 2, "both orders must still be created, just sharing one event");
    }

    #[test]
    fn order_id_column_is_saved_as_external_reference_after_creation() {
        let conn = test_conn();
        apply_order_rows(&conn, &full_headers(), &[sample_row("")], "EUR", MARKER_COL).unwrap();
        let reference: Option<String> = conn.query_row("SELECT external_reference FROM orders WHERE id = 1", [], |r| r.get(0)).unwrap();
        assert_eq!(reference.as_deref(), Some("TM-88213"));
    }

    #[test]
    fn a_fully_blank_row_is_skipped_silently() {
        let conn = test_conn();
        let blank = vec![String::new(); full_headers().len()];
        let (result, writes) = apply_order_rows(&conn, &full_headers(), &[blank], "EUR", MARKER_COL).unwrap();
        assert_eq!(result.created, 0);
        assert_eq!(result.errors.len(), 0);
        assert!(writes.is_empty());
    }

    #[test]
    fn a_row_with_a_bad_quantity_is_reported_and_does_not_block_the_next_row() {
        let conn = test_conn();
        let mut bad = sample_row("");
        bad[8] = "abc".to_string(); // Number of Tickets column
        let good = {
            let mut r = sample_row("");
            r[0] = "A Totally Different Show".to_string();
            r
        };
        let (result, _) = apply_order_rows(&conn, &full_headers(), &[bad, good], "EUR", MARKER_COL).unwrap();
        assert_eq!(result.created, 1, "the second, valid row must still import");
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].row_number, 2, "row 2 is the first data row (row 1 is the header)");
    }

    // ---- apply_order_rows: an already-linked row is left fully alone -------

    #[test]
    fn a_row_that_already_has_a_marker_is_left_alone_and_counted_unchanged() {
        let conn = test_conn();
        let (result, writes) = apply_order_rows(&conn, &full_headers(), &[sample_row("ORD-000001")], "EUR", MARKER_COL).unwrap();
        assert_eq!(result.created, 0);
        assert_eq!(result.unchanged, 1);
        assert_eq!(result.errors.len(), 0);
        assert!(writes.is_empty());
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM orders", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 0, "a row that already carries a marker must never be created again - v1 is creation-only");
    }

    #[test]
    fn a_row_that_already_has_a_marker_never_triggers_platform_or_event_auto_create() {
        let conn = test_conn();
        apply_order_rows(&conn, &full_headers(), &[sample_row("ORD-000001")], "EUR", MARKER_COL).unwrap();
        let platform_count: i64 = conn.query_row("SELECT COUNT(*) FROM platforms", [], |r| r.get(0)).unwrap();
        let event_count: i64 = conn.query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0)).unwrap();
        assert_eq!(platform_count, 0, "an already-synced row must do nothing at all, not even side-effect auto-creates");
        assert_eq!(event_count, 0);
    }

    // ---- "Create a new sheet for me" (2.0.9) -------------------------------
    //
    // create_orders_sheet_impl reuses pulls_sheet_sync::validate_share_email/
    // validate_currency directly rather than duplicating them, and those two
    // functions already have their own thorough tests in
    // pulls_sheet_sync.rs's test module - no need to re-test the same shared
    // logic a second time here. What IS specific to this module, and does
    // need its own coverage: that ORDERS_SHEET_HEADERS actually satisfies
    // this module's own required-header check, and that
    // create_orders_sheet_impl wires validation + the network call together
    // correctly (same "only the parts before the first network call are
    // exercised here" limitation as pulls_sheet_sync.rs's own equivalent
    // tests - see that module's comment just above its mirrored tests).

    #[test]
    fn orders_sheet_headers_satisfy_the_required_header_check() {
        // Regression guard: if apply_order_rows's required-header list ever
        // grows, a freshly auto-created sheet must still pass its own sync
        // immediately, with zero manual editing needed first.
        let headers: Vec<String> = ORDERS_SHEET_HEADERS.iter().map(|s| s.to_string()).collect();
        let map = build_header_map(&headers);
        assert!(check_required_headers(&map).is_ok(), "a freshly auto-created sheet must satisfy its own required columns");
    }

    #[test]
    fn orders_sheet_headers_match_full_headers_test_fixture_exactly() {
        // Regression guard for the doc comment's own claim: the header list
        // written into a freshly-created sheet and the full_headers() sample
        // row used throughout this module's tests must never quietly drift
        // apart from each other.
        let headers: Vec<String> = ORDERS_SHEET_HEADERS.iter().map(|s| s.to_string()).collect();
        assert_eq!(headers, full_headers());
    }

    #[test]
    fn create_orders_sheet_rejects_a_bad_email_before_touching_anything_else() {
        let conn = test_conn();
        let err = create_orders_sheet_impl(&conn, "not-an-email", "EUR").unwrap_err();
        assert!(err.to_string().to_lowercase().contains("email"), "the error must mention the actual problem: {err}");
    }

    #[test]
    fn create_orders_sheet_rejects_a_bad_currency_before_touching_anything_else() {
        let conn = test_conn();
        let err = create_orders_sheet_impl(&conn, "marko@example.com", "CZK").unwrap_err();
        assert!(err.to_string().to_lowercase().contains("currency"), "the error must mention the actual problem: {err}");
    }

    #[test]
    fn create_orders_sheet_with_valid_input_fails_cleanly_when_no_service_account_is_embedded() {
        let conn = test_conn();
        let err = create_orders_sheet_impl(&conn, "marko@example.com", "EUR").unwrap_err();
        assert!(
            err.to_string().contains("isn't available in this build"),
            "fully valid input must still stop cleanly before any network call in a test build: {err}"
        );
    }
}

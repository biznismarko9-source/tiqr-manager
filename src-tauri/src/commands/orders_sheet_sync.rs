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
//! columns on the SAME row once they sell. This module has two separate sync
//! entry points, both against the exact same connection (`load_connection`
//! with data_source `"orders"` - marko deliberately only ever connects this
//! sheet once, see REDESIGN-2.0.10-REPORT.md), so he never has to paste the
//! same URL/tab twice: `sync_orders` reads the FIRST batch, creating an
//! Order (and one Ticket per unit) from it - see "Column mapping (first
//! batch)" below. `sync_sales` (2.0.10) reads the SECOND batch from the SAME
//! rows, matching back to the tickets `sync_orders` already created via the
//! "TIQR ID" marker it writes - see "Column mapping (second batch - Sales)"
//! further down. Settings -> Integrations shows both as two buttons - "Order
//! sync" / "Sales sync" - on the one "Orders & Tickets" card.
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
//!
//! ## Column mapping (second batch - Sales sync, 2.0.10)
//!
//! | Sheet column | Field |
//! |---|---|
//! | `Site Listed` | `Sale.platform_id` (resolve-or-create by name - see `resolve_or_create_sale_platform`, deliberately NOT `csv_import::resolve_or_create_platform`: a brand-new platform here is created with `kind='sale'`, and an existing `kind='purchase'` platform found by the same name is promoted to `'both'`, so it actually shows up in the Sales screen's own platform picker, which filters to `kind IN ('sale','both')`) |
//! | `Payout Per Ticket` | `Sale.sale_price_cents` - same value on every line of the batch (same "one row = N tickets" convention as Section/Row/Ticket Type above). Presence of this column is what tells this sync a row is actually ready to be recorded as sold at all - see `apply_sales_rows` |
//! | `Revenue`, `Profit` | not read at all (marko's own choice) - the app always computes both itself from `Sale.sale_price_cents` and the ticket's own purchase cost (see finance.rs), never stores them, so there is nothing here to cross-check against without risking exactly the kind of drift finance.rs's own module doc comment says never to allow |
//! | `Status` | `Ticket.resale_status` (new free-text field, 2.0.10 - migrations/010) - stamped on every ticket of the order, same convention as Section/Row. Deliberately separate from `Ticket.status`, which this sync never touches directly - creating the sale already flips that to `'sold'` |
//! | `Delivery status` | `Ticket.delivery_status` (new free-text field, 2.0.10 - migrations/010), same stamp-on-every-ticket convention |
//! | `Payout status` | `Sale.payment_status` - blank means `pending` (same default `create_sales_batch_impl` itself uses); `pending`/`paid` map directly; anything else (including `refunded` - a sale can't be created as already refunded, same rule `validate_new_payment_status` already enforces) is a row error |
//! | (a date column - see `find_col` aliases in `apply_sales_rows`) | `Sale.sale_date`, required whenever `Payout Per Ticket` is present. marko's own past description of this column was ambiguous (see REDESIGN-2.0.10-REPORT.md) - deliberately NOT defaulted to today's date or the order's own purchase date, since either could silently mis-date months of real sales history. A sheet where none of the tried aliases match fails every such row with one clear, specific message instead |
//! | `paid by` | `Sale.buyer_reference` |
//! | `pull`, `who pulled`, `how much pull` | folded into `Sale.notes` as plain text (marko's own choice - keeps the standalone Pulls feature exactly as standalone as migrations/005_pulls.sql originally decided; no real link to a `pulls` row is ever created) |
//!
//! Sales sync is creation-only, same philosophy as `sync_orders` above and
//! marko's own explicit choice for this pass too: a ticket that already has
//! an active sale is left completely alone on every later sync, including
//! its `resale_status`/`delivery_status` - see `apply_sales_rows`. Unlike
//! `sync_orders`, it never writes anything back to the sheet at all (no new
//! marker column) - idempotency instead comes directly from each ticket's
//! own `status`/active `sales` row, checked fresh every run, and a row with
//! no "TIQR ID" yet is simply not ready for this sync (silently skipped, not
//! an error - it hasn't been through `sync_orders` yet).
//!
//! Every ticket belonging to one sheet row's order is sold together in one
//! `create_sales_batch_impl` call (same all-or-nothing "New sale" action the
//! UI itself uses for a multi-seat sale, and the exact function the UI uses
//! even for a single ticket - see that function's own doc comment) rather
//! than one `create_sale_impl` call per ticket - this also means marko sees
//! all N tickets from one sheet row grouped as a single sale on the Sales
//! screen, matching how his own sheet row represents one sale transaction.
//! Only tickets not already sold/cancelled are included in that batch; a
//! ticket that's already sold is left alone, one that's cancelled is simply
//! never offered to the batch (offering it would fail the WHOLE batch, since
//! `create_sales_batch_impl` is all-or-nothing).

use crate::commands::csv_import::resolve_or_create_platform;
use crate::commands::orders::insert_order_with_tickets;
use crate::commands::pulls_sheet_sync::parse_sheet_serial_date;
use crate::commands::sales::create_sales_batch_impl;
use crate::commands::sheets_sync::{
    last_synced_key, load_connection, set_setting, set_sheets_connection_impl, ALLOWED_CURRENCIES,
};
use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::google_sheets;
use crate::models::{CreatedSheetResult, OrderInput, SaleBatchInput, SaleBatchLineInput, SheetSyncIssue, SheetSyncResult};
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

    // 2.0.11: widened from "A1:Z" now that ORDERS_SHEET_HEADERS itself is 25
    // columns wide - together with the "TIQR ID" marker sync appends as
    // column 26, that already lands exactly on Z with zero room left for any
    // column marko adds later (a silent truncation, not an error, if it ever
    // happened). "A1:AZ" covers 52 columns - the same generous headroom the
    // 13-column original had relative to its own needs, restored.
    let range = google_sheets::a1_range(&connection.sheet_tab, "A1:AZ");
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
// Sales sync (2.0.10) - second batch of the SAME sheet rows, same
// connection. See this module's own doc comment ("Column mapping (second
// batch - Sales sync, 2.0.10)") for the full column mapping and design
// rationale.
// ---------------------------------------------------------------------------

/// Same lookup-by-name-or-create pattern as
/// `csv_import::resolve_or_create_platform`, but for a SALE-side platform
/// (marko's "Site Listed" column) rather than a purchase-side one: a
/// brand-new platform is created with `kind='sale'` so it immediately shows
/// up in the Sales/Sale Detail platform pickers (which filter to
/// `kind IN ('sale','both')` - see Sales.tsx/SaleDetail.tsx), and an
/// EXISTING platform found by name that currently has `kind='purchase'` is
/// promoted to `'both'` rather than left as-is, since it's now confirmed
/// used for a real sale too - otherwise it would stay invisible in those
/// same pickers even though a real sale now references it. A platform
/// that's already `'sale'` or `'both'` is left untouched.
fn resolve_or_create_sale_platform(conn: &Connection, name: &str) -> AppResult<i64> {
    if let Some((id, kind)) = conn
        .query_row("SELECT id, kind FROM platforms WHERE LOWER(name) = LOWER(?1)", [name], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
        })
        .optional()?
    {
        if kind == "purchase" {
            conn.execute("UPDATE platforms SET kind = 'both' WHERE id = ?1", [id])?;
        }
        return Ok(id);
    }
    conn.execute("INSERT INTO platforms(name, kind) VALUES (?1, 'sale')", [name])?;
    Ok(conn.last_insert_rowid())
}

/// Core logic behind `sync_sales`, taking a plain connection so it's
/// directly unit-testable without a Tauri app around it. Mutable (unlike
/// `apply_order_rows`'s `&Connection`) because `create_sales_batch_impl`
/// needs its own transaction per row.
fn apply_sales_rows(
    conn: &mut Connection,
    headers: &[String],
    data_rows: &[Vec<String>],
    marker_col_index: usize,
) -> AppResult<SheetSyncResult> {
    let map = build_header_map(headers);

    let payout_col = find_col(&map, &["payout per ticket", "payout"]);
    if payout_col.is_none() {
        return Err(AppError::Validation(
            "The connected sheet is missing required column(s): \"Payout Per Ticket\"".to_string(),
        ));
    }
    let site_listed_col = find_col(&map, &["site listed", "site", "listing site"]);
    let status_col = find_col(&map, &["status"]);
    let delivery_status_col = find_col(&map, &["delivery status", "delivery"]);
    let payout_status_col = find_col(&map, &["payout status"]);
    let sale_date_col = find_col(
        &map,
        &["date sold", "sale date", "date of sale", "date of purchase", "sold date", "payout date"],
    );
    let paid_by_col = find_col(&map, &["paid by", "paidby"]);
    let pull_col = find_col(&map, &["pull"]);
    let who_pulled_col = find_col(&map, &["who pulled"]);
    let how_much_pull_col = find_col(&map, &["how much pull"]);

    let mut result =
        SheetSyncResult { created: 0, updated: 0, unchanged: 0, conflicts: vec![], errors: vec![], synced_at: String::new() };

    for (i, raw_row) in data_rows.iter().enumerate() {
        let row_number = (i + 2) as i64;

        // No "TIQR ID" yet - this row hasn't even been through Order sync,
        // so there is no order/tickets to attach sales info to. Not an
        // error, not even worth counting - completely normal for a row
        // marko is still filling in.
        let Some(order_code) = cell(raw_row, Some(marker_col_index)) else {
            continue;
        };

        // Order exists but nothing sold yet on this row - normal, and
        // (unlike the no-TIQR-ID case above) counted, since this row WAS
        // looked at - same convention as sync_orders's own already-marked
        // rows.
        let Some(payout_raw) = cell(raw_row, payout_col) else {
            result.unchanged += 1;
            continue;
        };

        let mut row_errors: Vec<String> = vec![];

        let sale_price_cents = match parse_decimal_to_cents(&payout_raw) {
            Ok(v) if v >= 0 => Some(v),
            Ok(_) => {
                row_errors.push("'Payout Per Ticket' cannot be negative".to_string());
                None
            }
            Err(e) => {
                row_errors.push(format!("'Payout Per Ticket': {e}"));
                None
            }
        };

        let sale_date = match cell(raw_row, sale_date_col).as_deref().map(parse_order_date) {
            Some(Ok(d)) => Some(d),
            Some(Err(e)) => {
                row_errors.push(format!("sale date: {e}"));
                None
            }
            None => {
                row_errors.push(
                    "missing a recognized sale-date column (tried \"Date sold\"/\"Sale date\"/\"Date of sale\"/\"Date of purchase\"/\"Sold date\"/\"Payout date\") - tell marko which header your sheet actually uses"
                        .to_string(),
                );
                None
            }
        };

        let payment_status = match cell(raw_row, payout_status_col).as_deref().map(|v| v.trim().to_lowercase()) {
            None => "pending".to_string(),
            Some(v) if v == "pending" => "pending".to_string(),
            Some(v) if v == "paid" => "paid".to_string(),
            Some(v) if v == "refunded" => {
                row_errors.push(
                    "'Payout status' can't be 'refunded' from a sync - record it as pending/paid here, then use the Refund action in the app"
                        .to_string(),
                );
                "pending".to_string()
            }
            Some(v) => {
                row_errors.push(format!("'Payout status' must be 'pending' or 'paid' - got '{v}'"));
                "pending".to_string()
            }
        };

        if !row_errors.is_empty() {
            result.errors.push(SheetSyncIssue { row_number, message: row_errors.join("; ") });
            continue;
        }
        let sale_price_cents = sale_price_cents.unwrap();
        let sale_date = sale_date.unwrap();

        let order_id: Option<i64> =
            conn.query_row("SELECT id FROM orders WHERE code = ?1", [&order_code], |r| r.get(0)).optional()?;
        let Some(order_id) = order_id else {
            result.errors.push(SheetSyncIssue {
                row_number,
                message: format!("'TIQR ID' value '{order_code}' does not match any order in the app - was it edited by hand?"),
            });
            continue;
        };

        let platform_name = cell(raw_row, site_listed_col);
        let platform_id = match &platform_name {
            Some(name) => match resolve_or_create_sale_platform(conn, name) {
                Ok(id) => Some(id),
                Err(e) => {
                    result.errors.push(SheetSyncIssue { row_number, message: format!("'Site Listed' platform '{name}': {e}") });
                    continue;
                }
            },
            None => None,
        };

        let ticket_rows: Vec<(i64, String)> = {
            let mut stmt = conn.prepare("SELECT id, status FROM tickets WHERE order_id = ?1 ORDER BY id")?;
            let rows = stmt.query_map([order_id], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<Result<Vec<_>, _>>()?;
            rows
        };

        let sellable_ticket_ids: Vec<i64> = ticket_rows
            .iter()
            .filter(|(_, status)| status == "available" || status == "listed")
            .map(|(id, _)| *id)
            .collect();

        if sellable_ticket_ids.is_empty() {
            // Every ticket on this order already has an active sale (or is
            // cancelled) - fully synced already, nothing new to do. Same
            // creation-only rule as sync_orders: resale_status/
            // delivery_status are NOT touched here either once a row is
            // fully synced - see this module's own doc comment.
            result.unchanged += 1;
            continue;
        }

        let mut notes_parts: Vec<String> = vec![];
        if let Some(v) = cell(raw_row, pull_col) {
            notes_parts.push(format!("Pull: {v}"));
        }
        if let Some(v) = cell(raw_row, who_pulled_col) {
            notes_parts.push(format!("Who pulled: {v}"));
        }
        if let Some(v) = cell(raw_row, how_much_pull_col) {
            notes_parts.push(format!("How much pull: {v}"));
        }
        let notes = if notes_parts.is_empty() { None } else { Some(notes_parts.join("; ")) };

        let batch_input = SaleBatchInput {
            lines: sellable_ticket_ids
                .iter()
                .map(|&ticket_id| SaleBatchLineInput { ticket_id, sale_price_cents, selling_fees_cents: 0 })
                .collect(),
            platform_id,
            sale_date,
            payment_status: Some(payment_status),
            buyer_reference: cell(raw_row, paid_by_col),
            notes,
        };

        match create_sales_batch_impl(conn, &batch_input) {
            Ok(_sale_ids) => {
                let resale_status = cell(raw_row, status_col);
                let delivery_status = cell(raw_row, delivery_status_col);
                for (ticket_id, _) in &ticket_rows {
                    conn.execute(
                        "UPDATE tickets SET resale_status=?1, delivery_status=?2, updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?3",
                        params![resale_status, delivery_status, ticket_id],
                    )?;
                }
                result.created += 1;
            }
            Err(e) => {
                result.errors.push(SheetSyncIssue { row_number, message: e.to_string() });
            }
        }
    }

    Ok(result)
}

/// The network-calling shell for Sales sync - fetches the SAME connected
/// sheet `sync_orders_impl` reads (data_source `"orders"` - see this
/// module's own doc comment for why there is only ever one connection to
/// manage), then hands off to `apply_sales_rows`. Never writes anything back
/// to the sheet itself, unlike `sync_orders_impl` writing the "TIQR ID"
/// marker - see `apply_sales_rows`'s own doc comment above for why none is
/// needed.
fn sync_sales_impl(conn: &mut Connection) -> AppResult<SheetSyncResult> {
    let connection = load_connection(conn, "orders")?
        .ok_or_else(|| AppError::Validation("No spreadsheet is connected for Orders yet - connect one in Settings first.".to_string()))?;
    let credential = crate::commands::google_auth::resolve_google_credential(conn, false)?;
    let token = credential.access_token();

    // 2.0.11: widened from "A1:Z" now that ORDERS_SHEET_HEADERS itself is 25
    // columns wide - together with the "TIQR ID" marker sync appends as
    // column 26, that already lands exactly on Z with zero room left for any
    // column marko adds later (a silent truncation, not an error, if it ever
    // happened). "A1:AZ" covers 52 columns - the same generous headroom the
    // 13-column original had relative to its own needs, restored.
    let range = google_sheets::a1_range(&connection.sheet_tab, "A1:AZ");
    let value_range = google_sheets::get_values(token, &connection.spreadsheet_id, &range)?;
    if value_range.values.is_empty() {
        return Err(AppError::Validation("The connected sheet/tab has no header row yet.".to_string()));
    }
    let headers = value_range.values[0].clone();
    let data_rows: &[Vec<String>] = if value_range.values.len() > 1 { &value_range.values[1..] } else { &[] };

    let (marker_col_index, _marker_exists) = resolve_marker_column(&headers);

    let mut result = apply_sales_rows(conn, &headers, data_rows, marker_col_index)?;

    result.synced_at = now_iso(conn)?;
    set_setting(conn, &last_synced_key("orders"), &result.synced_at)?;
    Ok(result)
}

/// Manual "Sales sync" button (Settings -> Integrations, Orders & Tickets
/// card) - sits next to "Order sync" on the same card, same connection.
/// Never runs on its own.
#[tauri::command]
pub fn sync_sales(state: State<AppState>) -> AppResult<SheetSyncResult> {
    let mut conn = state.db.lock().unwrap();
    sync_sales_impl(&mut conn)
}

// ---------------------------------------------------------------------------
// "Create a new sheet for me" (2.0.9) - mirrors pulls_sheet_sync.rs's own
// PULLS_SHEET_HEADERS/NEW_SHEET_TITLE/NEW_SHEET_TAB_NAME/
// create_pulls_sheet_impl/create_pulls_sheet exactly, reusing that module's
// validate_share_email/validate_currency directly (see their doc comments)
// rather than duplicating them - only the header list, sheet name, and
// data_source string differ.
// ---------------------------------------------------------------------------

/// Header row written into a freshly-created "Orders & Tickets" sheet.
/// Covers BOTH sync entry points that read this one connection (2.0.11) -
/// marko's own real sheet is one combined buy+sell tracker, and a freshly
/// auto-created sheet must be immediately ready for both "Order sync" AND
/// "Sales sync" with zero manual column-editing first, exactly like his own
/// real sheet already is. In marko's own exact column order: the 13 columns
/// `apply_order_rows` understands (Order sync's own batch - see the "Column
/// mapping (first batch)" table above), followed by the columns
/// `apply_sales_rows` understands (Sales sync's own batch - `Site Listed`
/// through `how much pull`, see the "Column mapping (second batch)" table
/// above), including `Revenue`/`Profit` - neither sync ever reads those two
/// (see that same table), they're written here purely as marko's own
/// on-sheet reference, since the app always computes both figures itself
/// (finance.rs) rather than trusting a stored number that could drift out of
/// sync with what the app actually shows. Deliberately excludes `TIQR ID` -
/// sync appends that itself the first time it's missing, see
/// `resolve_marker_column` - same reasoning as `PULLS_SHEET_HEADERS`. Only
/// the first 13 entries are asserted equal to this module's own
/// `full_headers()` test fixture (see
/// `orders_sheet_headers_start_with_order_sync_full_headers` below) - that
/// fixture deliberately stays the 13-column Order-sync-only shape, since
/// every existing Order sync test already builds rows against it.
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
    "Site Listed",
    "Payout Per Ticket",
    "Revenue",
    "Profit",
    "Status",
    "Delivery status",
    "Payout status",
    "date of purchase",
    "paid by",
    "pull",
    "who pulled",
    "how much pull",
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
    fn orders_sheet_headers_start_with_order_sync_full_headers() {
        // Regression guard, updated for 2.0.11: ORDERS_SHEET_HEADERS grew
        // from 13 to 25 columns (Order sync's own batch, followed by Sales
        // sync's - see its own doc comment), so it can no longer be exactly
        // equal to full_headers(), which deliberately stays the 13-column
        // Order-sync-only fixture every existing Order sync test already
        // builds rows against, unchanged. What must still hold: the
        // original 13 columns are still there, unchanged, in the same
        // order, at the front - so a freshly auto-created sheet is still
        // exactly as valid for Order sync as it always was.
        let headers: Vec<String> = ORDERS_SHEET_HEADERS.iter().map(|s| s.to_string()).collect();
        let order_sync_prefix = &headers[..full_headers().len()];
        assert_eq!(order_sync_prefix, &full_headers()[..], "the first 13 columns must still match full_headers() exactly");
    }

    #[test]
    fn orders_sheet_headers_satisfy_the_sales_sync_required_header_check() {
        // Same idea as orders_sheet_headers_satisfy_the_required_header_
        // check above, but for Sales sync's own requirement (2.0.11): a
        // freshly auto-created sheet must be immediately ready for "Sales
        // sync" too, with zero manual editing needed first - not only
        // "Order sync". apply_sales_rows only hard-requires "Payout Per
        // Ticket", already present in ORDERS_SHEET_HEADERS. "TIQR ID" is
        // appended by hand here since ORDERS_SHEET_HEADERS deliberately
        // excludes it (sync appends it itself) - this mirrors exactly what
        // a real sheet looks like after its first successful Order sync
        // run, which is the only time Sales sync is ever actually run
        // against it.
        let mut headers: Vec<String> = ORDERS_SHEET_HEADERS.iter().map(|s| s.to_string()).collect();
        headers.push("TIQR ID".to_string());
        let (marker_col_index, marker_exists) = resolve_marker_column(&headers);
        assert!(marker_exists);
        let mut conn = test_conn();
        let result = apply_sales_rows(&mut conn, &headers, &[], marker_col_index);
        assert!(
            result.is_ok(),
            "a freshly auto-created sheet (plus its own TIQR ID marker) must satisfy Sales sync's own required columns: {:?}",
            result.err()
        );
    }

    #[test]
    fn sales_sync_recognizes_marko_confirmed_sale_date_column_name() {
        // marko confirmed (2.0.11) that "date of purchase" is the real,
        // exact header text his own sheet uses for the sale-date column -
        // resolving the uncertainty flagged in REDESIGN-2.0.10-REPORT.md
        // section 3 (until now, only the "Date sold" alias was exercised by
        // a real test). Swaps just that one header for marko's own
        // confirmed real text - everything else, including sales_row()'s
        // own column order, stays exactly as
        // sales_sync_creates_a_sale_for_a_row_with_tiqr_id_and_payout above
        // already exercises.
        let mut headers = sales_headers();
        let date_col = headers.iter().position(|h| h == "Date sold").unwrap();
        headers[date_col] = "date of purchase".to_string();

        let mut conn = test_conn();
        let code = seed_order_with_quantity(&conn, 1);
        let result = apply_sales_rows(&mut conn, &headers, &[sales_row(&code, "45.00")], 0).unwrap();
        assert_eq!(result.created, 1, "\"date of purchase\" must be recognized as the sale-date column, not rejected as missing");
        assert_eq!(result.errors.len(), 0);
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

    // =========================================================================
    // Sales sync (2.0.10) - apply_sales_rows
    // =========================================================================

    // Header order is irrelevant here (unlike full_headers() above, which
    // mirrors marko's real column order for documentation purposes) - every
    // column is looked up by name via build_header_map/find_col, never by
    // position, so this fixture is free to list "TIQR ID" wherever's
    // convenient.
    fn sales_headers() -> Vec<String> {
        vec![
            "TIQR ID",
            "Site Listed",
            "Payout Per Ticket",
            "Status",
            "Delivery status",
            "Payout status",
            "Date sold",
            "paid by",
            "pull",
            "who pulled",
            "how much pull",
        ]
        .into_iter()
        .map(String::from)
        .collect()
    }

    fn sales_row(order_code: &str, payout: &str) -> Vec<String> {
        vec![order_code, "viagogo", payout, "Listed", "Not yet", "paid", "20/09/2026", "buyer@example.com", "", "", ""]
            .into_iter()
            .map(String::from)
            .collect()
    }

    /// Seeds a real Order + N Tickets via apply_order_rows itself (the same
    /// path sync_orders uses), so Sales sync tests exercise the real
    /// TIQR-ID-lookup integration rather than hand-inserted rows. Returns
    /// the generated order code (the value that goes in a sales row's own
    /// "TIQR ID" cell). Total Purchase Price and Seats are left blank so
    /// this works for any quantity without those cross-checks getting in
    /// the way.
    fn seed_order_with_quantity(conn: &Connection, quantity: i64) -> String {
        let mut cells = row(&[
            "Coldplay Arena Show",
            "15/09/2026",
            "ticketmaster",
            "410",
            "25",
            "",
            "TM-88213",
            "",
            "",
            "50.00",
            "EUR",
            "buyer@example.com",
            "e-ticket",
        ]);
        cells[8] = quantity.to_string();
        cells.push(String::new());
        let (_, writes) = apply_order_rows(conn, &full_headers(), &[cells], "EUR", MARKER_COL).unwrap();
        writes[0].1.clone()
    }

    fn ticket_ids_for_order(conn: &Connection, order_code: &str) -> Vec<i64> {
        let order_id: i64 = conn.query_row("SELECT id FROM orders WHERE code = ?1", [order_code], |r| r.get(0)).unwrap();
        let mut stmt = conn.prepare("SELECT id FROM tickets WHERE order_id = ?1 ORDER BY id").unwrap();
        stmt.query_map([order_id], |r| r.get(0)).unwrap().collect::<Result<Vec<i64>, _>>().unwrap()
    }

    #[test]
    fn a_row_with_no_tiqr_id_yet_is_silently_skipped() {
        let mut conn = test_conn();
        let mut blank_marker_row = sales_row("", "45.00");
        blank_marker_row[0] = String::new();
        let result = apply_sales_rows(&mut conn, &sales_headers(), &[blank_marker_row], 0).unwrap();
        assert_eq!(result.created, 0);
        assert_eq!(result.unchanged, 0, "not even counted - this row simply hasn't been through Order sync yet");
        assert_eq!(result.errors.len(), 0);
    }

    #[test]
    fn a_row_with_tiqr_id_but_no_payout_yet_is_counted_unchanged() {
        let mut conn = test_conn();
        let code = seed_order_with_quantity(&conn, 1);
        let mut r = sales_row(&code, "");
        r[2] = String::new(); // Payout Per Ticket blank
        let result = apply_sales_rows(&mut conn, &sales_headers(), &[r], 0).unwrap();
        assert_eq!(result.created, 0);
        assert_eq!(result.unchanged, 1);
        assert_eq!(result.errors.len(), 0);
    }

    #[test]
    fn sales_sync_creates_a_sale_for_a_row_with_tiqr_id_and_payout() {
        let mut conn = test_conn();
        let code = seed_order_with_quantity(&conn, 1);
        let result = apply_sales_rows(&mut conn, &sales_headers(), &[sales_row(&code, "45.00")], 0).unwrap();
        assert_eq!(result.created, 1);
        assert_eq!(result.errors.len(), 0);

        let ticket_id = ticket_ids_for_order(&conn, &code)[0];
        let (sale_price_cents, payment_status, status): (i64, String, String) = conn
            .query_row(
                "SELECT s.sale_price_cents, s.payment_status, t.status FROM sales s JOIN tickets t ON t.id = s.ticket_id WHERE s.ticket_id = ?1",
                [ticket_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(sale_price_cents, 4500);
        assert_eq!(payment_status, "paid");
        assert_eq!(status, "sold");
    }

    #[test]
    fn sales_sync_sells_every_ticket_of_a_multi_ticket_order_together_as_one_batch() {
        let mut conn = test_conn();
        let code = seed_order_with_quantity(&conn, 2);
        let result = apply_sales_rows(&mut conn, &sales_headers(), &[sales_row(&code, "45.00")], 0).unwrap();
        assert_eq!(result.created, 1, "one ROW created, even though it sells 2 tickets - same per-row counting as sync_orders");

        let ticket_ids = ticket_ids_for_order(&conn, &code);
        assert_eq!(ticket_ids.len(), 2);
        let batch_ids: Vec<Option<String>> = ticket_ids
            .iter()
            .map(|&tid| conn.query_row("SELECT batch_id FROM sales WHERE ticket_id = ?1", [tid], |r| r.get(0)).unwrap())
            .collect();
        assert!(batch_ids[0].is_some(), "2 tickets sold together must share a real batch_id");
        assert_eq!(batch_ids[0], batch_ids[1]);
    }

    #[test]
    fn a_row_whose_tiqr_id_does_not_match_any_order_is_reported() {
        let mut conn = test_conn();
        let result = apply_sales_rows(&mut conn, &sales_headers(), &[sales_row("ORD-999999", "45.00")], 0).unwrap();
        assert_eq!(result.created, 0);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].message.contains("does not match any order"), "{}", result.errors[0].message);
    }

    #[test]
    fn already_fully_sold_row_is_unchanged_on_a_second_sync_and_touches_nothing_further() {
        let mut conn = test_conn();
        let code = seed_order_with_quantity(&conn, 1);
        let first = apply_sales_rows(&mut conn, &sales_headers(), &[sales_row(&code, "45.00")], 0).unwrap();
        assert_eq!(first.created, 1);

        let ticket_id = ticket_ids_for_order(&conn, &code)[0];
        let sales_count_before: i64 = conn.query_row("SELECT COUNT(*) FROM sales WHERE ticket_id = ?1", [ticket_id], |r| r.get(0)).unwrap();

        // Same row, but now with different Status/Delivery status/Payout -
        // a second sync must change nothing, same creation-only rule as
        // sync_orders.
        let mut changed_row = sales_row(&code, "999.00");
        changed_row[3] = "Sold - different".to_string();
        let second = apply_sales_rows(&mut conn, &sales_headers(), &[changed_row], 0).unwrap();
        assert_eq!(second.created, 0);
        assert_eq!(second.unchanged, 1);

        let sales_count_after: i64 = conn.query_row("SELECT COUNT(*) FROM sales WHERE ticket_id = ?1", [ticket_id], |r| r.get(0)).unwrap();
        assert_eq!(sales_count_before, sales_count_after);
        let resale_status: Option<String> =
            conn.query_row("SELECT resale_status FROM tickets WHERE id = ?1", [ticket_id], |r| r.get(0)).unwrap();
        assert_eq!(resale_status.as_deref(), Some("Listed"), "must keep the FIRST sync's value, not the second sync's");
    }

    #[test]
    fn negative_payout_per_ticket_is_rejected() {
        let mut conn = test_conn();
        let code = seed_order_with_quantity(&conn, 1);
        let result = apply_sales_rows(&mut conn, &sales_headers(), &[sales_row(&code, "-5.00")], 0).unwrap();
        assert_eq!(result.created, 0);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].message.contains("Payout Per Ticket"), "{}", result.errors[0].message);
    }

    #[test]
    fn non_numeric_payout_per_ticket_is_rejected() {
        let mut conn = test_conn();
        let code = seed_order_with_quantity(&conn, 1);
        let result = apply_sales_rows(&mut conn, &sales_headers(), &[sales_row(&code, "abc")], 0).unwrap();
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].message.contains("Payout Per Ticket"), "{}", result.errors[0].message);
    }

    #[test]
    fn missing_sale_date_is_rejected_with_a_clear_message() {
        let mut conn = test_conn();
        let code = seed_order_with_quantity(&conn, 1);
        let mut r = sales_row(&code, "45.00");
        r[6] = String::new(); // Date sold
        let result = apply_sales_rows(&mut conn, &sales_headers(), &[r], 0).unwrap();
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].message.contains("sale-date"), "{}", result.errors[0].message);
    }

    #[test]
    fn invalid_payout_status_value_is_rejected() {
        let mut conn = test_conn();
        let code = seed_order_with_quantity(&conn, 1);
        let mut r = sales_row(&code, "45.00");
        r[5] = "banana".to_string();
        let result = apply_sales_rows(&mut conn, &sales_headers(), &[r], 0).unwrap();
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].message.contains("Payout status"), "{}", result.errors[0].message);
    }

    #[test]
    fn refunded_payout_status_is_rejected_with_a_clear_message() {
        let mut conn = test_conn();
        let code = seed_order_with_quantity(&conn, 1);
        let mut r = sales_row(&code, "45.00");
        r[5] = "refunded".to_string();
        let result = apply_sales_rows(&mut conn, &sales_headers(), &[r], 0).unwrap();
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].message.to_lowercase().contains("refund"), "{}", result.errors[0].message);
    }

    #[test]
    fn blank_payout_status_defaults_to_pending() {
        let mut conn = test_conn();
        let code = seed_order_with_quantity(&conn, 1);
        let mut r = sales_row(&code, "45.00");
        r[5] = String::new();
        let result = apply_sales_rows(&mut conn, &sales_headers(), &[r], 0).unwrap();
        assert_eq!(result.created, 1);
        let ticket_id = ticket_ids_for_order(&conn, &code)[0];
        let payment_status: String =
            conn.query_row("SELECT payment_status FROM sales WHERE ticket_id = ?1", [ticket_id], |r| r.get(0)).unwrap();
        assert_eq!(payment_status, "pending");
    }

    #[test]
    fn site_listed_platform_is_created_with_kind_sale() {
        let mut conn = test_conn();
        let code = seed_order_with_quantity(&conn, 1);
        apply_sales_rows(&mut conn, &sales_headers(), &[sales_row(&code, "45.00")], 0).unwrap();
        let kind: String = conn.query_row("SELECT kind FROM platforms WHERE LOWER(name) = 'viagogo'", [], |r| r.get(0)).unwrap();
        assert_eq!(kind, "sale");
    }

    #[test]
    fn an_existing_purchase_platform_is_promoted_to_both_when_also_used_for_a_sale() {
        let mut conn = test_conn();
        // seed_order_with_quantity's own order uses "ticketmaster" as its
        // PURCHASE platform (via resolve_or_create_platform, kind='purchase').
        let code = seed_order_with_quantity(&conn, 1);
        let (purchase_platform_id, kind_before): (i64, String) =
            conn.query_row("SELECT id, kind FROM platforms WHERE LOWER(name) = 'ticketmaster'", [], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        assert_eq!(kind_before, "purchase");

        let mut r = sales_row(&code, "45.00");
        r[1] = "ticketmaster".to_string(); // Site Listed - same name, same platform
        apply_sales_rows(&mut conn, &sales_headers(), &[r], 0).unwrap();

        let (platform_id_after, kind_after): (i64, String) =
            conn.query_row("SELECT id, kind FROM platforms WHERE LOWER(name) = 'ticketmaster'", [], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        assert_eq!(platform_id_after, purchase_platform_id, "must reuse the SAME platform row, not create a duplicate");
        assert_eq!(kind_after, "both");
    }

    #[test]
    fn resale_status_and_delivery_status_are_stamped_on_every_ticket_of_the_order() {
        let mut conn = test_conn();
        let code = seed_order_with_quantity(&conn, 2);
        apply_sales_rows(&mut conn, &sales_headers(), &[sales_row(&code, "45.00")], 0).unwrap();
        for ticket_id in ticket_ids_for_order(&conn, &code) {
            let (resale_status, delivery_status): (Option<String>, Option<String>) = conn
                .query_row("SELECT resale_status, delivery_status FROM tickets WHERE id = ?1", [ticket_id], |r| {
                    Ok((r.get(0)?, r.get(1)?))
                })
                .unwrap();
            assert_eq!(resale_status.as_deref(), Some("Listed"));
            assert_eq!(delivery_status.as_deref(), Some("Not yet"));
        }
    }

    #[test]
    fn pull_who_pulled_and_how_much_pull_are_folded_into_sale_notes() {
        let mut conn = test_conn();
        let code = seed_order_with_quantity(&conn, 1);
        let mut r = sales_row(&code, "45.00");
        r[8] = "yes".to_string();
        r[9] = "Jozef".to_string();
        r[10] = "15".to_string();
        apply_sales_rows(&mut conn, &sales_headers(), &[r], 0).unwrap();
        let ticket_id = ticket_ids_for_order(&conn, &code)[0];
        let notes: Option<String> = conn.query_row("SELECT notes FROM sales WHERE ticket_id = ?1", [ticket_id], |r| r.get(0)).unwrap();
        let notes = notes.unwrap();
        assert!(notes.contains("Pull: yes"), "{notes}");
        assert!(notes.contains("Who pulled: Jozef"), "{notes}");
        assert!(notes.contains("How much pull: 15"), "{notes}");
    }

    #[test]
    fn paid_by_maps_to_buyer_reference() {
        let mut conn = test_conn();
        let code = seed_order_with_quantity(&conn, 1);
        apply_sales_rows(&mut conn, &sales_headers(), &[sales_row(&code, "45.00")], 0).unwrap();
        let ticket_id = ticket_ids_for_order(&conn, &code)[0];
        let buyer_reference: Option<String> =
            conn.query_row("SELECT buyer_reference FROM sales WHERE ticket_id = ?1", [ticket_id], |r| r.get(0)).unwrap();
        assert_eq!(buyer_reference.as_deref(), Some("buyer@example.com"));
    }

    #[test]
    fn missing_payout_per_ticket_column_entirely_fails_the_whole_sync() {
        let mut conn = test_conn();
        let headers: Vec<String> = vec!["TIQR ID".to_string(), "Status".to_string()];
        let err = apply_sales_rows(&mut conn, &headers, &[], 0).unwrap_err();
        assert!(err.to_string().contains("Payout Per Ticket"), "{err}");
    }

    #[test]
    fn a_cancelled_ticket_in_the_order_is_excluded_from_the_batch_and_does_not_block_the_others() {
        let mut conn = test_conn();
        let code = seed_order_with_quantity(&conn, 2);
        let ticket_ids = ticket_ids_for_order(&conn, &code);
        conn.execute("UPDATE tickets SET status = 'cancelled' WHERE id = ?1", [ticket_ids[0]]).unwrap();

        let result = apply_sales_rows(&mut conn, &sales_headers(), &[sales_row(&code, "45.00")], 0).unwrap();
        assert_eq!(result.created, 1);
        assert_eq!(result.errors.len(), 0);

        let sold_count: i64 = conn.query_row("SELECT COUNT(*) FROM sales", [], |r| r.get(0)).unwrap();
        assert_eq!(sold_count, 1, "only the non-cancelled ticket must have been sold");
        let cancelled_still_cancelled: String =
            conn.query_row("SELECT status FROM tickets WHERE id = ?1", [ticket_ids[0]], |r| r.get(0)).unwrap();
        assert_eq!(cancelled_still_cancelled, "cancelled");
    }

    #[test]
    fn a_ticket_already_sold_before_this_sync_is_left_alone_and_only_the_remaining_one_is_sold() {
        let mut conn = test_conn();
        let code = seed_order_with_quantity(&conn, 2);
        let ticket_ids = ticket_ids_for_order(&conn, &code);

        // Simulate marko having already recorded a sale for one ticket by
        // hand in the app (same create_sales_batch_impl the UI itself uses)
        // before ever running Sales sync.
        create_sales_batch_impl(
            &mut conn,
            &SaleBatchInput {
                lines: vec![SaleBatchLineInput { ticket_id: ticket_ids[0], sale_price_cents: 3000, selling_fees_cents: 0 }],
                platform_id: None,
                sale_date: "2026-01-01".to_string(),
                payment_status: Some("paid".to_string()),
                buyer_reference: None,
                notes: None,
            },
        )
        .unwrap();

        let result = apply_sales_rows(&mut conn, &sales_headers(), &[sales_row(&code, "45.00")], 0).unwrap();
        assert_eq!(result.created, 1, "the row still counts as created - a NEW sale was made for the other ticket");

        let total_sales: i64 = conn.query_row("SELECT COUNT(*) FROM sales", [], |r| r.get(0)).unwrap();
        assert_eq!(total_sales, 2, "1 pre-existing + 1 new");
        let first_ticket_price: i64 =
            conn.query_row("SELECT sale_price_cents FROM sales WHERE ticket_id = ?1", [ticket_ids[0]], |r| r.get(0)).unwrap();
        assert_eq!(first_ticket_price, 3000, "the pre-existing sale must be untouched, not overwritten with this sync's 45.00");
    }
}

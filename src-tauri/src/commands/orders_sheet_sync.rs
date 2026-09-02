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
//! sync" / "Sales sync" - on the one "Orders & Sales" card.
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
//! | `Total Purchase Price` | not stored anywhere - reconciled against `Number of Tickets x Price Per Ticket` when present (see `reconcile_order_pricing`, 2.0.42): an exact match or a small, honestly rounding-explainable gap is accepted (and the sheet's own cell corrected to match, transparently, never silently); anything bigger still gets the row rejected rather than silently trusting one number over the other |
//! | `Number of Tickets` | `Order.quantity` |
//! | `Price Per Ticket` | `Order.unit_price_cents` - more than 2 decimal places (marko's own automated order sources sometimes produce this dividing a total across tickets) is rounded rather than rejected, see `reconcile_order_pricing` |
//! | `currency` | `Order.currency` - a row's own value if present and one of EUR/USD/GBP, otherwise the connection's configured currency (unlike Pulls, whose sheet has no currency column at all) |
//! | *(no sheet column)* | `Order.payment_status` - stamped `"paid"` unconditionally on every order this sync creates (2.0.43 - see note below the table) |
//! | `Email (used)` | copied as-is into `Order.notes` (raw value, no label prefix - see 2.0.12) - no dedicated field for this exists anywhere in the schema |
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
//! **2.0.43: every order this sync creates is stamped `Order.payment_status
//! = "paid"` outright, never left to `insert_order_with_tickets`'s own
//! `"unpaid"` fallback for a `None` value.** marko's own report: the
//! Dashboard was counting freshly-synced orders as unpaid (its "unpaid
//! orders" figure is a plain `payment_status IN ('unpaid','partial')` count
//! - see dashboard.rs), which he did not want - an order already sitting in
//! his connected sheet is, to him, already a real, confirmed purchase.
//! Unlike CSV import (csv_import.rs), which supports an OPTIONAL
//! payment_status column a row may set, this sync's first batch has no such
//! column, and marko asked for this unconditionally rather than a new
//! column he'd have to remember to fill in - so there is nothing here to
//! read, only a fixed value to stamp.
//!
//! ## Column mapping (second batch - Sales sync, 2.0.10)
//!
//! | Sheet column | Field |
//! |---|---|
//! | `Site Listed` | `Sale.platform_id` (resolve-or-create by name - see `resolve_or_create_sale_platform`, deliberately NOT `csv_import::resolve_or_create_platform`: a brand-new platform here is created with `kind='sale'`, and an existing `kind='purchase'` platform found by the same name is promoted to `'both'`, so it actually shows up in the Sales screen's own platform picker, which filters to `kind IN ('sale','both')`) |
//! | `Payout Per Ticket` | `Sale.sale_price_cents` - same value on every line of the batch (same "one row = N tickets" convention as Section/Row/Ticket Type above). Presence of this column is what tells this sync a row is actually ready to be recorded as sold at all - see `apply_sales_rows` |
//! | `Revenue`, `Profit` | never READ (marko's own choice, unchanged) - the app always computes both itself from `Sale.sale_price_cents` and the ticket's own purchase cost (see finance.rs) for its own Dashboard, never from these cells. 2.0.19: now WRITTEN, but only ever as live Sheets formulas (`=Payout*Tickets`, `=Revenue-TotalPurchasePrice`) that Sheets itself evaluates - never a number the app computes - see "Sheet structure" below |
//! | `Status` | `Ticket.resale_status` (new free-text field, 2.0.10 - migrations/010) - stamped on every ticket of the order, same convention as Section/Row. Deliberately separate from `Ticket.status`, which this sync never touches directly - creating the sale already flips that to `'sold'` |
//! | `Delivery status` | `Ticket.delivery_status` (new free-text field, 2.0.10 - migrations/010), same stamp-on-every-ticket convention |
//! | `Payout status` | `Sale.payment_status` - blank means `pending` (same default `create_sales_batch_impl` itself uses); `pending`/`paid` map directly; anything else (including `refunded` - a sale can't be created as already refunded, same rule `validate_new_payment_status` already enforces) is a row error |
//! | (a date column - see `find_col` aliases in `apply_sales_rows`) | `Sale.sale_date`, required whenever `Payout Per Ticket` is present. marko's own past description of this column was ambiguous (see REDESIGN-2.0.10-REPORT.md) - deliberately NOT defaulted to today's date or the order's own purchase date, since either could silently mis-date months of real sales history. A sheet where none of the tried aliases match fails every such row with one clear, specific message instead |
//! | `paid by` | `Sale.buyer_reference` |
//! | `pull`, `who pulled`, `how much pull` | 2.0.17: when `pull` trims+lowercases to exactly "yes" AND `who pulled` isn't blank, creates one linked `pulls_received` row instead (`who pulled` -> `puller_name`, `how much pull` -> `amount_cents`, defaulting to 0 if blank/unparseable - see `maybe_link_pull_received`). Idempotent per order: no sequence of syncs ever creates a second linked row. Replaces the pre-2.0.17 behaviour of folding these 3 columns into `Sale.notes` as plain text entirely - marko's own request, so this data shows up as a real, browsable record instead ("aby si mal o tom dobry prehlad"). **2.0.23: unlike every other column in this table, these 3 are (re-)checked on EVERY sync of a linked row, not only the one that first creates the sale** - marko's real workflow is often "sync the sale first, add pull info to that same row later", and he expects that to still link automatically on the next sync rather than only working when both land in the same sync pass |
//!
//! Sales sync is creation-only for the sale itself, same philosophy as
//! `sync_orders` above and marko's own explicit choice for this pass too: a
//! ticket that already has an active sale is left completely alone on every
//! later sync, including its `resale_status`/`delivery_status` - see
//! `apply_sales_rows`. The one deliberate exception is `pull`/`who pulled`/
//! `how much pull` - see the table row above and `maybe_link_pull_received`'s
//! own doc comment for why linking is (idempotently) re-attempted on every
//! sync of an already-sold row, not just its first. Unlike `sync_orders`, it
//! never writes anything back to the sheet at all (no new marker column) -
//! idempotency instead comes directly from each ticket's own `status`/active
//! `sales` row, checked fresh every run, and a row with no "TIQR ID" yet is
//! simply not ready for this sync (silently skipped, not an error - it
//! hasn't been through `sync_orders` yet).
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
use crate::commands::pulls_received;
use crate::commands::pulls_sheet_sync::parse_sheet_serial_date;
use crate::commands::sales::create_sales_batch_impl;
use crate::commands::sheets_sync::{
    last_pushed_key, last_synced_key, load_connection, set_setting, set_sheets_connection_impl, ALLOWED_CURRENCIES,
};
use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::google_sheets;
use crate::models::{
    CreatedSheetResult, OrderInput, PullReceivedInput, SaleBatchInput, SaleBatchLineInput, SheetSyncIssue, SheetSyncResult,
};
use crate::money::{format_cents, format_cents_for_sheet, parse_decimal_to_cents, round_decimal_to_cents};
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

/// The inverse of `parse_order_date` for plain `DD/MM/YYYY` text (never the
/// serial-number variant - that's only ever something this app *reads*, a
/// side effect of someone typing an actual date into a cell rather than
/// text, not something it should start writing back out): turns a stored
/// `YYYY-MM-DD` date (an `Order.purchase_date` or `Sale.sale_date`) back
/// into the sheet's own display format. Falls back to the raw value
/// unchanged if it isn't actually `YYYY-MM-DD` (should never happen - see
/// `pulls_sheet_sync::format_date_for_sheet`'s identical doc comment for why
/// guessing is never worth the risk here either). Shared by both push
/// directions below (`build_order_append_row`, `uniform_sale_for_order`)
/// since both write a date back into this same sheet's own DD/MM/YYYY
/// convention.
fn format_order_date_for_sheet(iso: &str) -> String {
    match chrono::NaiveDate::parse_from_str(iso, "%Y-%m-%d") {
        Ok(d) => d.format("%d/%m/%Y").to_string(),
        Err(_) => iso.to_string(),
    }
}

/// Case-insensitive find-or-create by name, mirroring
/// `resolve_or_create_platform`'s own convention - but unlike CSV import
/// (which requires the event to already exist, on the theory a first-time
/// CSV import is reviewed by hand before committing), a live sync is meant
/// to be clicked repeatedly against a sheet marko is actively filling in, so
/// requiring him to pre-create every event in the app first would defeat the
/// point. A freshly created event gets this row's own date and otherwise
/// default fields (status "upcoming") - exactly like typing only the
/// name+date into "New event" by hand - EXCEPT for category: 2.0.63 has a
/// freshly created event also try `ai_categorize::detect_category_for_event_
/// name` on its own name, writing both `category_id` and its `category` text
/// mirror when that finds a confident match (see that module's doc comment
/// for the full free-rules-then-AI design), exactly as if marko had picked a
/// category by hand right after typing the name. Never blocks or fails this
/// function if detection finds nothing (or isn't configured) - the event is
/// still created, just with no category, same as every event before 2.0.63.
/// An event that already exists by name is reused as-is - its date AND
/// category (or lack of one) are deliberately left untouched even if a later
/// row's own data would suggest something different - see this function's
/// own tests.
fn resolve_or_create_event(conn: &Connection, name: &str, event_date: &str) -> AppResult<i64> {
    if let Some(id) = conn
        .query_row("SELECT id FROM events WHERE LOWER(name) = LOWER(?1)", [name], |r| r.get(0))
        .optional()?
    {
        return Ok(id);
    }
    let category_match = crate::ai_categorize::detect_category_for_event_name(conn, name);
    conn.execute(
        "INSERT INTO events (name, event_date, status, category_id, category) VALUES (?1, ?2, 'upcoming', ?3, ?4)",
        params![
            name,
            event_date,
            category_match.as_ref().map(|m| m.id),
            category_match.as_ref().map(|m| m.name.clone())
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

// ---------------------------------------------------------------------------
// 2.0.42: automatic reconciliation of automated-order pricing. marko's own
// words (a real sync result screenshot, 4 rows skipped): "niekedy sa stane
// v google sheets lebo tie orders su zautomatizovane ze ta cena nesedi uplne
// do centu, nechcem, aby ukazalo error, ze to musis opravit, ale chcem, aby
// to apka sama opravila a posunula to do dashboardu a taktiez updatla v
// google sheets, jasne nemoze tam napisat hlupost, musi to davat zmysel" -
// sometimes in Google Sheets, because the orders are automated, the price
// doesn't match exactly to the cent; he doesn't want an error he has to fix
// by hand, he wants the app to fix it itself and push it to the dashboard
// AND update Google Sheets - but of course it can't write nonsense there, it
// has to make sense. His own screenshot showed two distinct shapes of this:
// a 'Total Purchase Price' a couple of cents off Number of Tickets x Price
// Per Ticket, and a 'Price Per Ticket' with more than 2 decimal places
// (his automation dividing a whole total across tickets without rounding).
//
// The functions below implement "make sense" as an actual, checkable rule
// rather than a vibe: a gap is only ever auto-corrected when it's SMALL
// ENOUGH to be honestly explained by rounding one value to the nearest cent
// - see `rounding_tolerance_cents`'s own doc comment for the exact bound and
// why it's mathematically, not arbitrarily, chosen. Anything bigger still
// hard-errors exactly like before this version - never silently "corrected"
// into a number that doesn't actually add up.
// ---------------------------------------------------------------------------

/// The largest gap, in cents, between a whole 'Total Purchase Price' and
/// 'Number of Tickets' x 'Price Per Ticket' that can honestly be explained
/// by rounding a single per-ticket price to the nearest cent, for an order
/// of `quantity` tickets.
///
/// Rounding one value (a price per ticket derived as total/quantity) to the
/// nearest cent moves it by at most half a cent either way. Multiplied back
/// out across `quantity` tickets, the total gap this can honestly produce is
/// at most `quantity` half-cents - i.e. `quantity as f64 / 2.0`, rounded up
/// to the next whole cent since cents themselves are always whole. A gap
/// bigger than this is not explainable by rounding alone - it's a real
/// mismatch (a typo, a missed fee, ...) - and 2.0.42 deliberately leaves
/// that as a hard error, same as every version before it: marko's own
/// explicit requirement ("nemoze tam napisat hlupost, musi to davat zmysel" -
/// it can't write nonsense there, it has to make sense) is a real constraint
/// this code enforces, not just a design note.
fn rounding_tolerance_cents(quantity: i64) -> i64 {
    (quantity + 1) / 2
}

/// `total_cents / quantity`, rounded to the nearest whole cent (round-half-
/// up), computed entirely in integer arithmetic - no float anywhere near
/// money, per this app's own money.rs house rule. `(2*total + quantity) /
/// (2*quantity)` is the standard integer trick for "round a/b to the
/// nearest integer": scaling both sides by 2 turns the usual "add half the
/// divisor before truncating" into whole numbers, avoiding a fractional
/// `quantity/2` when `quantity` is odd. `quantity` is always >= 1 by the
/// time this is called (parsed and validated earlier in `apply_order_rows`).
fn derive_unit_price_from_total(total_cents: i64, quantity: i64) -> i64 {
    (2 * total_cents + quantity) / (2 * quantity)
}

/// What `reconcile_order_pricing` decided for one row's 'Price Per Ticket'
/// (always present - it's a required column) and 'Total Purchase Price'
/// (only when that optional column has a value on this row).
#[derive(Debug)]
struct PricingOutcome {
    /// The value to actually use for `OrderInput.unit_price_cents` - either
    /// exactly what was typed, or a sensible reconciled value.
    unit_price_cents: i64,
    /// `Some(new text)` only when the sheet's own 'Price Per Ticket' cell
    /// should be overwritten with this - i.e. it was imprecise or wrong
    /// enough to need correcting, but not so far off that this function
    /// refused to guess (see `rounding_tolerance_cents`).
    corrected_unit_price_text: Option<String>,
    /// `Some(new text)` only when the sheet's own 'Total Purchase Price'
    /// cell should be overwritten with this - same rule as above, for the
    /// other of the two cells this function can correct.
    corrected_total_price_text: Option<String>,
    /// A human-readable one-line explanation of what was auto-corrected and
    /// why, for `SheetSyncResult.corrected` - `Some` exactly when at least
    /// one of the two `corrected_*_text` fields above is `Some`.
    note: Option<String>,
}

/// Reconciles one order row's 'Price Per Ticket' against its (optional)
/// 'Total Purchase Price' - see this section's own doc comment above for
/// marko's real request and why this exists at all. Returns `Err` with the
/// exact same wording as pre-2.0.42 whenever nothing here can be sensibly
/// reconciled (the gap is too large to be rounding, or the text isn't a
/// number at all) - `apply_order_rows` skips the row exactly like before in
/// that case, so this never changes what the strict, "make sense" path
/// looks like from the outside, only what's NOW accepted as sensible.
///
/// `total_raw` is `None` when the row simply has nothing in the 'Total
/// Purchase Price' column (it's optional - see this module's own doc
/// comment table) - there's nothing to reconcile against in that case, and
/// an over-precise 'Price Per Ticket' is just rounded on its own.
fn reconcile_order_pricing(unit_price_raw: &str, total_raw: Option<&str>, quantity: i64) -> Result<PricingOutcome, String> {
    // 'Total Purchase Price' is always parsed strictly (2.0.42 never relaxes
    // this - marko's own report was specifically about 'Price Per Ticket'
    // having too much precision, never about a malformed Total) - a
    // genuinely malformed Total is still a hard error, exactly as before.
    let total_cents: Option<i64> = match total_raw {
        Some(raw) => Some(parse_decimal_to_cents(raw).map_err(|e| format!("'Total Purchase Price': {e}"))?),
        None => None,
    };
    let tolerance = rounding_tolerance_cents(quantity);

    match parse_decimal_to_cents(unit_price_raw) {
        Ok(v) if v < 0 => Err("'Price Per Ticket' cannot be negative".to_string()),
        Ok(unit_price_cents) => {
            // Typed cleanly (2 or fewer decimals) - only 'Total Purchase
            // Price' (if present) might still need reconciling against it.
            let Some(total_cents) = total_cents else {
                return Ok(PricingOutcome { unit_price_cents, corrected_unit_price_text: None, corrected_total_price_text: None, note: None });
            };
            let expected = unit_price_cents * quantity;
            let gap = (total_cents - expected).abs();
            if gap == 0 {
                Ok(PricingOutcome { unit_price_cents, corrected_unit_price_text: None, corrected_total_price_text: None, note: None })
            } else if gap <= tolerance {
                let note = format!(
                    "'Total Purchase Price' ({}) was {} off Number of Tickets x Price Per Ticket ({}) - close enough to be automation rounding, so it was corrected to match on the sheet",
                    format_cents(total_cents),
                    format_cents(gap),
                    format_cents(expected)
                );
                Ok(PricingOutcome {
                    unit_price_cents,
                    corrected_unit_price_text: None,
                    corrected_total_price_text: Some(format_cents_for_sheet(expected)),
                    note: Some(note),
                })
            } else {
                Err(format!(
                    "'Total Purchase Price' ({}) does not match Number of Tickets x Price Per Ticket ({}) - check these values",
                    format_cents(total_cents),
                    format_cents(expected)
                ))
            }
        }
        Err(strict_err) => {
            // Didn't parse as a plain 2-decimal amount - the one shape
            // 2.0.42 now makes sense of automatically is exactly what an
            // automated Total/Quantity division produces: more than 2
            // decimal places, otherwise a perfectly normal positive number.
            // round_decimal_to_cents rejects anything that ISN'T that (plain
            // garbage text), in which case the ORIGINAL strict error is the
            // more accurate one to report, not "not a valid amount".
            let rounded = round_decimal_to_cents(unit_price_raw).map_err(|_| format!("'Price Per Ticket': {strict_err}"))?;
            if rounded < 0 {
                return Err("'Price Per Ticket' cannot be negative".to_string());
            }
            match total_cents {
                Some(total_cents) => {
                    // 'Total Purchase Price' is present and clean - trust it
                    // over 'Price Per Ticket's own over-precise text (a
                    // whole amount someone actually paid is more meaningful
                    // than a computed-looking fraction), and derive a clean
                    // per-ticket price from it instead.
                    let derived = derive_unit_price_from_total(total_cents, quantity);
                    let expected = derived * quantity;
                    let gap = (total_cents - expected).abs();
                    // Mathematically this can never exceed `tolerance` (see
                    // derive_unit_price_from_total/rounding_tolerance_cents'
                    // own doc comments) - checked anyway rather than assumed,
                    // same never-trust-an-invariant-you-don't-also-check
                    // spirit as every other money path in this app.
                    if gap > tolerance {
                        return Err(format!(
                            "'Price Per Ticket' ({unit_price_raw}) has more than 2 decimal places and 'Total Purchase Price' ({}) doesn't sensibly divide across {quantity} ticket(s) either - check these values",
                            format_cents(total_cents)
                        ));
                    }
                    let note = format!(
                        "'Price Per Ticket' ({unit_price_raw}) had more than 2 decimal places - corrected to {} ('Total Purchase Price' {} / {quantity} ticket(s), rounded to the nearest cent)",
                        format_cents(derived),
                        format_cents(total_cents)
                    );
                    Ok(PricingOutcome {
                        unit_price_cents: derived,
                        corrected_unit_price_text: Some(format_cents_for_sheet(derived)),
                        corrected_total_price_text: None,
                        note: Some(note),
                    })
                }
                None => {
                    // No 'Total Purchase Price' to sensibly derive from -
                    // best effort: round 'Price Per Ticket's own over-precise
                    // value to the nearest cent, same convention money.rs
                    // uses everywhere else.
                    let note =
                        format!("'Price Per Ticket' ({unit_price_raw}) had more than 2 decimal places - rounded to {}", format_cents(rounded));
                    Ok(PricingOutcome {
                        unit_price_cents: rounded,
                        corrected_unit_price_text: Some(format_cents_for_sheet(rounded)),
                        corrected_total_price_text: None,
                        note: Some(note),
                    })
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// The core - no network call anywhere in this function, which is what makes
// it directly unit-testable with a plain in-memory `test_conn()`. The
// network-calling `sync_orders_impl` below fetches the rows via
// `google_sheets::get_values`, then calls this.
// ---------------------------------------------------------------------------

/// Every actual Google Sheets cell write `apply_order_rows` asks its caller
/// to make on its behalf - this function itself never talks to Google (see
/// its own doc comment). Bundled into one struct, rather than a bigger tuple,
/// specifically so most of this module's existing tests (which only care
/// about `SheetSyncResult`, not what got written back to the sheet) keep
/// destructuring `apply_order_rows`'s result as a plain 2-tuple unchanged -
/// only the handful that actually inspect `.markers` needed updating for
/// 2.0.42's new `price_corrections` field.
#[derive(Debug, Default)]
struct RowWriteBacks {
    /// (0-based data row index, "TIQR ID" marker value to write) - created
    /// for every row this call turned into a real order, same as every
    /// version before 2.0.42.
    markers: Vec<(usize, String)>,
    /// (0-based data row index, 0-based column index, new cell text) - one
    /// entry per sheet cell `reconcile_order_pricing` decided to correct on
    /// an otherwise-successful row (2.0.42) - see that function's own doc
    /// comment. Never populated for a row that ends up in `errors` instead;
    /// correcting a cell on a row that wasn't actually saved would be
    /// exactly the "writing nonsense" marko explicitly didn't want.
    price_corrections: Vec<(usize, usize, String)>,
}

/// Applies already-fetched sheet rows, creating a new Order (with its
/// Tickets) for every row that doesn't yet carry a "TIQR ID" marker. A row
/// that already carries one is left alone entirely - v1 is creation-only
/// (see this module's doc comment), so nothing on that row is even parsed,
/// and no platform/event is auto-created on its behalf. Returns the
/// user-facing result summary plus every sheet cell write the caller still
/// needs to make on the actual sheet - this function itself never talks to
/// Google.
fn apply_order_rows(
    conn: &Connection,
    headers: &[String],
    data_rows: &[Vec<String>],
    connection_currency: &str,
    marker_col_index: usize,
) -> AppResult<(SheetSyncResult, RowWriteBacks)> {
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
        corrected: vec![],
        synced_at: String::new(),
    };
    let mut writes = RowWriteBacks::default();

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

        // 2.0.42: 'Price Per Ticket' and (optional) 'Total Purchase Price'
        // are reconciled together rather than validated independently - see
        // this section's own "automatic reconciliation" doc comment above
        // `rounding_tolerance_cents` for marko's real request and the exact
        // rule this now applies. A small, honestly-explainable gap between
        // the two (or a 'Price Per Ticket' with more than 2 decimals) is
        // corrected automatically and reported via `correction_note`/the two
        // `*_correction_text` variables below; anything bigger still hard-
        // errors into `row_errors`, exactly like every version before this.
        let total_price_raw = cell(raw_row, total_price_col);
        let mut correction_note: Option<String> = None;
        let mut unit_price_correction_text: Option<String> = None;
        let mut total_price_correction_text: Option<String> = None;

        let unit_price_cents: Option<i64> = match (unit_price_raw.as_deref(), quantity) {
            (None, _) => {
                row_errors.push("missing 'Price Per Ticket' value".to_string());
                None
            }
            (Some(raw), None) => {
                // Quantity itself is already invalid/missing (reported
                // above) - nothing sensible to reconcile without it, so this
                // falls back to a plain strict parse (still needed so an
                // obviously-broken 'Price Per Ticket' is reported too, not
                // masked by the quantity error alone).
                match parse_decimal_to_cents(raw) {
                    Ok(v) if v >= 0 => Some(v),
                    Ok(_) => {
                        row_errors.push("'Price Per Ticket' cannot be negative".to_string());
                        None
                    }
                    Err(e) => {
                        row_errors.push(format!("'Price Per Ticket': {e}"));
                        None
                    }
                }
            }
            (Some(raw), Some(q)) => match reconcile_order_pricing(raw, total_price_raw.as_deref(), q) {
                Ok(outcome) => {
                    correction_note = outcome.note;
                    unit_price_correction_text = outcome.corrected_unit_price_text;
                    total_price_correction_text = outcome.corrected_total_price_text;
                    Some(outcome.unit_price_cents)
                }
                Err(e) => {
                    row_errors.push(e);
                    None
                }
            },
        };

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
            // 2.0.43: stamped "paid" unconditionally, never left to
            // `insert_order_with_tickets`'s own "unpaid" default - marko's
            // own explicit instruction. An order already sitting in his
            // connected sheet is, to him, already a real confirmed
            // purchase; this first batch has no payment-status column of
            // its own for Order sync to read instead (see the module doc
            // comment's "Column mapping" table), and CSV import's own
            // OPTIONAL payment_status column (csv_import.rs) was
            // deliberately not mirrored here - he asked for this
            // unconditionally, not gated behind a new column he'd have to
            // remember to fill in.
            payment_status: Some("paid".to_string()),
            // 2.0.12: the raw cell value, unlabeled - marko's own report:
            // this used to prepend "Email used: " (see the module doc
            // comment's "Column mapping" table), which was never what he
            // wanted to see on the order. Order.notes has nothing else
            // folded into it in this first batch (unlike Sale.notes in the
            // second - see apply_sales_rows - which genuinely does combine
            // several sheet columns and still needs its own structure), so
            // there is no ambiguity a label was ever disambiguating here.
            notes: email.clone(),
            ticket_type: cell(raw_row, ticket_type_col),
            section: cell(raw_row, section_col),
            row_label: cell(raw_row, row_col),
            // 2.2.7: no sheet column exists for tier/level (see this
            // module's "Column mapping" table) and adding one is a separate,
            // out-of-scope decision (which column, what header text marko
            // would need to add to his own sheet) - out of scope for the
            // ticket-metadata task that added `OrderInput.tier`. Same
            // "doesn't try to source everything CSV import can" precedent
            // already set by payment_status just above.
            tier: None,
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
                writes.markers.push((i, code));
                // 2.0.42: only reported/written back now that the row is
                // confirmed saved - see RowWriteBacks.price_corrections' own
                // doc comment for why a row that instead ended up in
                // `result.errors` must never reach here.
                if let Some(note) = correction_note {
                    result.corrected.push(SheetSyncIssue { row_number, message: note });
                }
                if let (Some(text), Some(col)) = (unit_price_correction_text, unit_price_col) {
                    writes.price_corrections.push((i, col, text));
                }
                if let (Some(text), Some(col)) = (total_price_correction_text, total_price_col) {
                    writes.price_corrections.push((i, col, text));
                }
            }
            Err(e) => {
                result.errors.push(SheetSyncIssue { row_number, message: e.to_string() });
            }
        }
    }

    Ok((result, writes))
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

    let (mut result, writes) = apply_order_rows(conn, &headers, data_rows, &connection.currency, marker_col_index)?;

    for (row_idx, marker_value) in writes.markers {
        let sheet_row_number = (row_idx + 2) as i64;
        let cell_range = google_sheets::a1_range(&connection.sheet_tab, &format!("{letter}{sheet_row_number}"));
        if let Err(e) = google_sheets::update_values(token, &connection.spreadsheet_id, &cell_range, &[vec![marker_value]]) {
            result.errors.push(SheetSyncIssue {
                row_number: sheet_row_number,
                message: format!("saved in the app, but could not write its ID back to the sheet: {e}"),
            });
        }
    }

    // 2.0.42: writes back whatever `reconcile_order_pricing` decided to
    // auto-correct (see apply_order_rows/RowWriteBacks) - plain text, same
    // as the marker write-back just above, never a formula (these cells are
    // marko's own typed data, not computed ones).
    for (row_idx, col_index, new_value) in writes.price_corrections {
        let sheet_row_number = (row_idx + 2) as i64;
        let corrected_letter = column_index_to_a1(col_index);
        let cell_range = google_sheets::a1_range(&connection.sheet_tab, &format!("{corrected_letter}{sheet_row_number}"));
        if let Err(e) = google_sheets::update_values(token, &connection.spreadsheet_id, &cell_range, &[vec![new_value]]) {
            result.errors.push(SheetSyncIssue {
                row_number: sheet_row_number,
                message: format!("saved in the app with a corrected price, but the sheet's own cell could not be updated: {e}"),
            });
        }
    }

    refresh_sheet_structure_soft_fail(conn, token, &connection.spreadsheet_id, &connection.sheet_tab, &headers, data_rows, &mut result);

    result.synced_at = now_iso(conn)?;
    set_setting(conn, &last_synced_key("orders"), &result.synced_at)?;
    Ok(result)
}

/// Manual "Sync now" button (Settings -> Integrations, Orders & Sales
/// card). Never runs on its own.
#[tauri::command]
pub fn sync_orders(state: State<AppState>) -> AppResult<SheetSyncResult> {
    let conn = state.db.lock().unwrap();
    sync_orders_impl(&conn)
}

// ---------------------------------------------------------------------------
// Push (app -> sheet), 2.0.18 - the Order-sync half. Deliberately
// APPEND-ONLY: an order that already carries a "TIQR ID" (i.e. already has a
// `sheet_sync_links` row - whether it got there via Order sync above or via
// this very push, on an earlier run) is never revisited here, full stop, no
// comparison, no update path at all - see this module's own "Creation-only
// in this pass, deliberately" doc comment above for exactly why: editing an
// order's purchase-side numbers after tickets exist would touch
// `insert_order_with_tickets`'s exact-cent cost allocation, which is
// protected financial logic this project's house rules say not to touch
// without asking first. Only a brand-new, never-linked local order becomes
// a new sheet row. Unlike Pulls push, there is therefore no snapshot/
// conflict machinery needed at all here - "never linked yet" is the only
// case this handles.
// ---------------------------------------------------------------------------

/// Just enough of one order to build its pushed sheet row - a dedicated,
/// narrower shape than the full `Order` model (which doesn't even carry
/// `external_reference` - see `commands::orders::map_order` - and whose
/// `section`/`row_label`/`ticket_type` genuinely live on `Ticket`, not
/// `Order`, per `insert_order_with_tickets`'s own stamp-onto-every-ticket
/// convention).
struct OrderForPush {
    id: i64,
    code: String,
    event_name: String,
    purchase_date: String,
    platform_name: Option<String>,
    quantity: i64,
    unit_price_cents: i64,
    currency: String,
    notes: Option<String>,
    external_reference: Option<String>,
}

/// Section/Row/Ticket Type are stamped identically across every ticket of an
/// order by `insert_order_with_tickets` - this only ever reads the first
/// ticket's copy of each, on that assumption. `seat` is the one field that
/// genuinely varies per ticket (see `join_seats` below).
struct TicketForPush {
    section: Option<String>,
    row_label: Option<String>,
    ticket_type: Option<String>,
    seat: Option<String>,
}

/// Rebuilds the sheet's comma-separated "Seats" cell from individual ticket
/// rows - the exact inverse of `apply_order_rows`'s own `seats` parsing.
/// Blank unless *every* ticket has its own seat (the same "one seat per
/// ticket, or none at all" rule the read direction already enforces on the
/// way in - a partial mix, e.g. after a manual per-ticket edit, has no
/// faithful single-cell representation, so this never guesses at one).
fn join_seats(tickets: &[TicketForPush]) -> String {
    if tickets.is_empty() {
        return String::new();
    }
    let seats: Vec<&str> = tickets.iter().filter_map(|t| t.seat.as_deref()).collect();
    if seats.len() == tickets.len() {
        seats.join(", ")
    } else {
        String::new()
    }
}

/// Lays out one brand-new order as a full positional sheet row (plus its
/// marker cell), ready for `append_values` - same "every in-between column
/// accounted for" reasoning as `pulls_sheet_sync::build_pull_append_row`.
/// Never touches `Site Listed`/`Payout Per Ticket`/.../`how much pull` (the
/// Sales-sync batch of columns) - those are `push_sales`'s job, once this
/// order has actually sold something.
fn build_order_append_row(
    map: &HashMap<String, usize>,
    marker_col_index: usize,
    header_count: usize,
    order: &OrderForPush,
    tickets: &[TicketForPush],
) -> Vec<String> {
    let mut cells: Vec<(usize, String)> = vec![];
    if let Some(c) = find_col(map, &["event name", "event"]) {
        cells.push((c, order.event_name.clone()));
    }
    if let Some(c) = find_col(map, &["date (dd/mm/yyyy)", "date"]) {
        cells.push((c, format_order_date_for_sheet(&order.purchase_date)));
    }
    if let Some(c) = find_col(map, &["platform"]) {
        cells.push((c, order.platform_name.clone().unwrap_or_default()));
    }
    if let Some(c) = find_col(map, &["section"]) {
        cells.push((c, tickets.first().and_then(|t| t.section.clone()).unwrap_or_default()));
    }
    if let Some(c) = find_col(map, &["row"]) {
        cells.push((c, tickets.first().and_then(|t| t.row_label.clone()).unwrap_or_default()));
    }
    if let Some(c) = find_col(map, &["seats", "seat"]) {
        cells.push((c, join_seats(tickets)));
    }
    if let Some(c) = find_col(map, &["order id", "orderid"]) {
        cells.push((c, order.external_reference.clone().unwrap_or_default()));
    }
    if let Some(c) = find_col(map, &["total purchase price", "total price"]) {
        cells.push((c, format_cents_for_sheet(order.unit_price_cents * order.quantity)));
    }
    if let Some(c) = find_col(map, &["number of tickets", "quantity", "qty", "ks"]) {
        cells.push((c, order.quantity.to_string()));
    }
    if let Some(c) = find_col(map, &["price per ticket", "unit price", "price"]) {
        cells.push((c, format_cents_for_sheet(order.unit_price_cents)));
    }
    if let Some(c) = find_col(map, &["currency"]) {
        cells.push((c, order.currency.clone()));
    }
    if let Some(c) = find_col(map, &["email (used)", "email"]) {
        cells.push((c, order.notes.clone().unwrap_or_default()));
    }
    if let Some(c) = find_col(map, &["ticket type"]) {
        cells.push((c, tickets.first().and_then(|t| t.ticket_type.clone()).unwrap_or_default()));
    }
    cells.push((marker_col_index, order.code.clone()));

    let width = cells.iter().map(|(i, _)| i + 1).max().unwrap_or(0).max(header_count);
    let mut row = vec![String::new(); width];
    for (i, v) in cells {
        row[i] = v;
    }
    row
}

/// The push direction's own core for Order sync - see this section's own
/// doc comment above for the append-only scope. Every non-demo, never-linked
/// order becomes one appended row; a `sheet_sync_links` row is inserted
/// immediately (marker = the order's own `code`, already generated at
/// creation - same placeholder `'{}'` snapshot `apply_order_rows`'s own
/// create path already uses, since there is no update path here to ever
/// compare it against).
/// 2.2.10: now returns the (order_id, code) pairs it prepared as a THIRD
/// element instead of writing their `sheet_sync_links` rows itself - see
/// `push_orders_impl`'s own doc comment (2.2.10) for why that link must
/// never be recorded before the sheet write it describes is confirmed to
/// have actually happened.
fn apply_order_push(
    conn: &Connection,
    headers: &[String],
    marker_col_index: usize,
) -> AppResult<(SheetSyncResult, Vec<Vec<String>>, Vec<(i64, String)>)> {
    let map = build_header_map(headers);
    check_required_headers(&map)?;

    let mut result =
        SheetSyncResult { created: 0, updated: 0, unchanged: 0, conflicts: vec![], errors: vec![], corrected: vec![], synced_at: String::new() };
    let mut append_rows = vec![];
    let mut pending_links: Vec<(i64, String)> = vec![];

    let orders: Vec<OrderForPush> = {
        let mut stmt = conn.prepare(
            "SELECT o.id, o.code, e.name, o.purchase_date, p.name, o.quantity, o.unit_price_cents, o.currency, o.notes, o.external_reference
             FROM orders o
             JOIN events e ON e.id = o.event_id
             LEFT JOIN platforms p ON p.id = o.platform_id
             WHERE o.is_demo = 0
               AND NOT EXISTS (SELECT 1 FROM sheet_sync_links l WHERE l.data_source = 'orders' AND l.local_id = o.id)
             ORDER BY o.id",
        )?;
        let rows = stmt
            .query_map([], |r| {
                Ok(OrderForPush {
                    id: r.get(0)?,
                    code: r.get(1)?,
                    event_name: r.get(2)?,
                    purchase_date: r.get(3)?,
                    platform_name: r.get(4)?,
                    quantity: r.get(5)?,
                    unit_price_cents: r.get(6)?,
                    currency: r.get(7)?,
                    notes: r.get(8)?,
                    external_reference: r.get(9)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        rows
    };

    for order in orders {
        let tickets: Vec<TicketForPush> = {
            let mut stmt = conn.prepare("SELECT section, row_label, ticket_type, seat FROM tickets WHERE order_id = ?1 ORDER BY id")?;
            let rows = stmt
                .query_map([order.id], |r| {
                    Ok(TicketForPush { section: r.get(0)?, row_label: r.get(1)?, ticket_type: r.get(2)?, seat: r.get(3)? })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            rows
        };

        let row = build_order_append_row(&map, marker_col_index, headers.len(), &order, &tickets);
        append_rows.push(row);
        pending_links.push((order.id, order.code));
        result.created += 1;
    }

    Ok((result, append_rows, pending_links))
}

/// The push direction's own network-calling shell for Order sync - same
/// split as `sync_orders_impl` above (fetch the sheet once, hand its parsed
/// shape to the pure core, then perform whatever writes it asked for).
fn push_orders_impl(conn: &Connection) -> AppResult<SheetSyncResult> {
    let connection = load_connection(conn, "orders")?
        .ok_or_else(|| AppError::Validation("No spreadsheet is connected for Orders yet - connect one in Settings first.".to_string()))?;
    let credential = crate::commands::google_auth::resolve_google_credential(conn, false)?;
    let token = credential.access_token();

    let range = google_sheets::a1_range(&connection.sheet_tab, "A1:AZ");
    let value_range = google_sheets::get_values(token, &connection.spreadsheet_id, &range)?;
    if value_range.values.is_empty() {
        return Err(AppError::Validation("The connected sheet/tab has no header row yet.".to_string()));
    }
    let headers = value_range.values[0].clone();

    let (marker_col_index, marker_exists) = resolve_marker_column(&headers);
    let letter = column_index_to_a1(marker_col_index);
    if !marker_exists {
        let header_range = google_sheets::a1_range(&connection.sheet_tab, &format!("{letter}1"));
        google_sheets::update_values(token, &connection.spreadsheet_id, &header_range, &[vec![MARKER_HEADER.to_string()]])?;
    }

    let (mut result, append_rows, pending_links) = apply_order_push(conn, &headers, marker_col_index)?;

    if !append_rows.is_empty() {
        let new_count = append_rows.len();
        let append_range = google_sheets::a1_range(&connection.sheet_tab, "A1");
        match google_sheets::append_values(token, &connection.spreadsheet_id, &append_range, &append_rows) {
            // 2.2.10: the `sheet_sync_links` rows are only written here, now
            // that the append above is confirmed to have actually reached
            // the sheet - see `apply_order_push`'s own doc comment (2.2.10)
            // for the bug this fixes (marko: "tabulka napisala ze bola
            // updated, no ziadna zmena nenastala"). Same placeholder '{}'
            // snapshot `apply_order_rows`'s own create path uses, for the
            // same reason (no update path exists here to ever compare it
            // against).
            Ok(()) => {
                let now = now_iso(conn)?;
                for (order_id, code) in &pending_links {
                    conn.execute(
                        "INSERT INTO sheet_sync_links (data_source, local_id, sheet_marker, last_synced_snapshot, last_synced_at)
                         VALUES ('orders', ?1, ?2, '{}', ?3)",
                        params![order_id, code, now],
                    )?;
                }
            }
            Err(e) => {
                // Nothing was actually written to the sheet, so nothing was
                // actually "created" from the user's point of view either -
                // nor are these orders linked, so the very next push will
                // correctly try them again instead of silently forgetting
                // them.
                result.created = 0;
                result.errors.push(SheetSyncIssue {
                    row_number: 0,
                    message: format!("{new_count} new order(s) were prepared but could not be written to the sheet: {e}"),
                });
            }
        }
    }

    // Pre-append snapshot - any row(s) `append_rows` above just added land
    // one row past whatever this covers, so they get their own Revenue/
    // Profit formula/dropdowns on the NEXT sync/push instead of this one
    // (all four commands in this module refresh the structure, so that is
    // never more than one click away) rather than a second full-sheet
    // re-fetch on every single push just to cover this run's own appends a
    // few seconds sooner.
    let data_rows: &[Vec<String>] = if value_range.values.len() > 1 { &value_range.values[1..] } else { &[] };

    // 2.0.54: catch up any order whose currency drifted from the sheet -
    // see reconcile_order_currencies' own doc comment for why this exists.
    let currency_writes = reconcile_order_currencies(conn, &headers, data_rows);
    for (row_number, cells) in currency_writes {
        let mut row_ok = true;
        for (col, value) in cells {
            let col_letter = column_index_to_a1(col);
            let cell_range = google_sheets::a1_range(&connection.sheet_tab, &format!("{col_letter}{row_number}"));
            if let Err(e) = google_sheets::update_values(token, &connection.spreadsheet_id, &cell_range, &[vec![value]]) {
                row_ok = false;
                result.errors.push(SheetSyncIssue { row_number, message: format!("currency catch-up failed: {e}") });
            }
        }
        if row_ok {
            result.updated += 1;
        }
    }

    refresh_sheet_structure_soft_fail(conn, token, &connection.spreadsheet_id, &connection.sheet_tab, &headers, data_rows, &mut result);

    result.synced_at = now_iso(conn)?;
    set_setting(conn, &last_pushed_key("orders"), &result.synced_at)?;
    Ok(result)
}

/// "Push orders" button (Settings -> Integrations, Orders & Sales card) -
/// the new sibling of "Order sync". Never runs on its own.
#[tauri::command]
pub fn push_orders(state: State<AppState>) -> AppResult<SheetSyncResult> {
    let conn = state.db.lock().unwrap();
    push_orders_impl(&conn)
}

// ---------------------------------------------------------------------------
// Currency-conversion push (2.0.53) - a narrow, deliberate EXCEPTION to the
// "Order push above is append-only, an already-linked order is never
// revisited, full stop" rule documented at the top of that section. marko
// pointed out that after using Order Detail's/the Dashboard's "Convert to
// EUR" (2.0.50/2.0.51) on an order that came from a sheet, his actual Google
// Sheet kept showing the old currency and old amounts forever after -
// exactly what the append-only rule guarantees, since nothing ever writes
// back to an already-linked row otherwise.
//
// This does NOT reopen general updates to a linked order (still no path to
// edit quantity/platform/date/... back to the sheet, and still no cell-by-
// cell conflict detection - both would still risk the exact-cent cost
// allocation problem the append-only rule protects against). It writes
// EXACTLY the 3 cells a currency conversion itself just changed locally -
// Currency, Price Per Ticket, Total Purchase Price - and only ever runs as
// a direct follow-up to that same conversion succeeding, never as its own
// button or on any regular sync. An order with no sheet link at all (most
// orders - manually entered, CSV-imported, or Sheets never connected) is
// simply not touched, silently - there is nowhere to push to.
// ---------------------------------------------------------------------------

/// Pure/testable half: given already-fetched sheet headers+rows and the
/// marker for one order, works out which (column index, new value) cells
/// need writing for its Currency/Price Per Ticket/Total Purchase Price -
/// or `None` if the marker can't be found in the sheet at all (the row was
/// deleted or moved since this order was last linked - reported to the
/// user as an error by the caller below, never guessed at).
fn currency_push_cells(
    headers: &[String],
    data_rows: &[Vec<String>],
    marker: &str,
    new_currency: &str,
    new_unit_price_cents: i64,
    new_total_cost_cents: i64,
) -> Option<(i64, Vec<(usize, String)>)> {
    let (marker_col_index, marker_exists) = resolve_marker_column(headers);
    if !marker_exists {
        return None;
    }
    let row_idx = data_rows
        .iter()
        .position(|raw_row| cell(raw_row, Some(marker_col_index)).as_deref() == Some(marker))?;
    let row_number = (row_idx + 2) as i64; // header is sheet row 1, same convention as everywhere else in this module

    let map = build_header_map(headers);
    let mut cells: Vec<(usize, String)> = vec![];
    if let Some(c) = find_col(&map, &["currency"]) {
        cells.push((c, new_currency.to_string()));
    }
    if let Some(c) = find_col(&map, &["price per ticket", "unit price", "price"]) {
        cells.push((c, format_cents_for_sheet(new_unit_price_cents)));
    }
    // 2.0.61: never push a computed total of exactly zero over a cell that
    // might hold a real, non-zero number already - a real order's total
    // purchase price is never actually free, so a "correction" to 0,00 is
    // far more likely to mean this order's own `total_cost_cents` is itself
    // wrong/unset than that the sheet's existing value needs replacing.
    // Silently trusting it either way is exactly the kind of thing this
    // module elsewhere refuses to do (see `reconcile_order_pricing`'s own
    // "anything bigger still gets the row rejected rather than silently
    // trusting one number over the other") - skip the cell here rather than
    // risk wiping a real value, and let whoever calls this decide whether to
    // surface that as something to look into.
    if new_total_cost_cents != 0 {
        if let Some(c) = find_col(&map, &["total purchase price", "total price"]) {
            cells.push((c, format_cents_for_sheet(new_total_cost_cents)));
        }
    }
    Some((row_number, cells))
}

/// Checks EVERY linked order's Currency/Price Per Ticket/Total Purchase
/// Price against already-fetched sheet data and returns only the writes
/// that are actually needed - a cell that already matches the order's
/// current local value is left alone, never rewritten just because it was
/// checked (same "never touch what's already right" spirit as
/// apply_sales_push's own blank-cells-only rule above). Takes headers/
/// data_rows the caller already fetched for its own purposes rather than
/// re-fetching the whole sheet once per linked order, which would be both
/// slow and wasteful for anyone with more than a handful of them.
///
/// 2.0.54: marko reported push_order_currency_to_sheet's own immediately-
/// after-conversion push (2.0.53) didn't visibly work for him. This is the
/// fallback he asked for: "Push Orders"/"Push Sales" (the existing manual
/// sync buttons, wired in below) now also catch up any currency mismatch
/// they find on a normal run, regardless of why the automatic push may
/// have missed it for a given order.
fn reconcile_order_currencies(conn: &Connection, headers: &[String], data_rows: &[Vec<String>]) -> Vec<(i64, Vec<(usize, String)>)> {
    let links: Vec<(i64, String)> = {
        let stmt = conn.prepare(
            "SELECT l.local_id, l.sheet_marker FROM sheet_sync_links l
             JOIN orders o ON o.id = l.local_id
             WHERE l.data_source = 'orders' AND o.is_demo = 0
             ORDER BY l.local_id",
        );
        match stmt {
            Ok(mut s) => match s.query_map([], |r| Ok((r.get(0)?, r.get(1)?))) {
                Ok(rows) => rows.collect::<Result<Vec<_>, _>>().unwrap_or_default(),
                Err(_) => vec![],
            },
            Err(_) => vec![],
        }
    };

    let mut writes = vec![];
    for (order_id, marker) in links {
        let order_row: rusqlite::Result<(String, i64, i64)> = conn.query_row(
            "SELECT currency, unit_price_cents, total_cost_cents FROM orders WHERE id = ?1",
            [order_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        );
        let Ok((currency, unit_price_cents, total_cost_cents)) = order_row else { continue };
        let Some((row_number, cells)) = currency_push_cells(headers, data_rows, &marker, &currency, unit_price_cents, total_cost_cents)
        else {
            continue;
        };
        let raw_row = &data_rows[(row_number - 2) as usize];
        let stale: Vec<(usize, String)> =
            cells.into_iter().filter(|(col, value)| cell(raw_row, Some(*col)).as_deref() != Some(value.as_str())).collect();
        if !stale.is_empty() {
            writes.push((row_number, stale));
        }
    }
    writes
}


/// order's own (now-converted) currency/unit_price_cents/total_cost_cents
/// fresh from the database rather than taking them as parameters, so this
/// can never drift from what was actually saved.
///
/// Returns `(linked_to_sheet, sheet_push_error)` - see
/// `OrderCurrencyConversion`'s own doc comment on those two fields for what
/// each value means to the caller/frontend. Every failure path here returns
/// `Ok`-shaped data (never propagates an `Err` up) precisely because a
/// failed push must never look like a failed conversion - the conversion
/// itself is already safely committed before this function is ever called.
pub(crate) fn push_order_currency_to_sheet(conn: &Connection, order_id: i64) -> (bool, Option<String>) {
    let marker: Option<String> = match conn
        .query_row(
            "SELECT sheet_marker FROM sheet_sync_links WHERE data_source = 'orders' AND local_id = ?1",
            [order_id],
            |r| r.get(0),
        )
        .optional()
    {
        Ok(m) => m,
        Err(_) => None, // best-effort lookup for a best-effort feature - treated the same as "not linked" rather than surfaced
    };
    let Some(marker) = marker else {
        return (false, None);
    };

    let order_row: rusqlite::Result<(String, i64, i64)> = conn.query_row(
        "SELECT currency, unit_price_cents, total_cost_cents FROM orders WHERE id = ?1",
        [order_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    );
    let (new_currency, new_unit_price_cents, new_total_cost_cents) = match order_row {
        Ok(v) => v,
        Err(e) => return (true, Some(format!("could not re-read the converted order: {e}"))),
    };

    let connection = match load_connection(conn, "orders") {
        Ok(Some(c)) => c,
        Ok(None) => return (true, Some("no spreadsheet is connected for Orders anymore".to_string())),
        Err(e) => return (true, Some(e.to_string())),
    };
    let credential = match crate::commands::google_auth::resolve_google_credential(conn, false) {
        Ok(c) => c,
        Err(e) => return (true, Some(e.to_string())),
    };
    let token = credential.access_token();

    let range = google_sheets::a1_range(&connection.sheet_tab, "A1:AZ");
    let value_range = match google_sheets::get_values(token, &connection.spreadsheet_id, &range) {
        Ok(v) => v,
        Err(e) => return (true, Some(e.to_string())),
    };
    if value_range.values.is_empty() {
        return (true, Some("the connected sheet has no header row".to_string()));
    }
    let headers = &value_range.values[0];
    let data_rows: &[Vec<String>] = if value_range.values.len() > 1 { &value_range.values[1..] } else { &[] };

    let Some((row_number, cells)) =
        currency_push_cells(headers, data_rows, &marker, &new_currency, new_unit_price_cents, new_total_cost_cents)
    else {
        return (true, Some(format!("could not find row \"{marker}\" in the connected sheet anymore")));
    };

    let mut last_error: Option<String> = None;
    for (col, value) in cells {
        let col_letter = column_index_to_a1(col);
        let cell_range = google_sheets::a1_range(&connection.sheet_tab, &format!("{col_letter}{row_number}"));
        if let Err(e) = google_sheets::update_values(token, &connection.spreadsheet_id, &cell_range, &[vec![value]]) {
            last_error = Some(e.to_string());
        }
    }
    (true, last_error)
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
        SheetSyncResult { created: 0, updated: 0, unchanged: 0, conflicts: vec![], errors: vec![], corrected: vec![], synced_at: String::new() };

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
            // cancelled) - fully synced already, nothing new for the sale
            // itself to do. Same creation-only rule as sync_orders:
            // resale_status/delivery_status are NOT touched here either once
            // a row is fully synced - see this module's own doc comment.
            //
            // 2.0.23: pull/who pulled/how much pull are the one deliberate
            // exception - marko's real workflow is often "sync the sale
            // first, add pull info to that same row later" (or edit it after
            // an earlier pull-less sync), and he expects that to still end
            // up linked without retyping the sale's own columns just to
            // force a fresh "creation". `maybe_link_pull_received`'s own
            // idempotency check (by order_id, not by row/sync-pass) makes
            // this safe to attempt on every single sync, however many times
            // - see its own doc comment. Quantity here is the order's actual
            // SOLD ticket count (not `sellable_ticket_ids.len()`, which is 0
            // in this branch by definition) - linking is skipped entirely,
            // same as a blank pull cell, when nothing on the order has sold
            // yet (e.g. every ticket cancelled), so this never attempts to
            // link a pull with a zero/invalid quantity.
            let pull_cell = cell(raw_row, pull_col);
            let who_pulled_cell = cell(raw_row, who_pulled_col);
            let how_much_pull_cell = cell(raw_row, how_much_pull_col);
            let sold_quantity = ticket_rows.iter().filter(|(_, status)| status == "sold").count() as i64;
            let newly_linked = if sold_quantity > 0 {
                maybe_link_pull_received(
                    conn,
                    order_id,
                    sold_quantity,
                    pull_cell.as_deref(),
                    who_pulled_cell.as_deref(),
                    how_much_pull_cell.as_deref(),
                )?
            } else {
                false
            };
            if newly_linked {
                result.updated += 1;
            } else {
                result.unchanged += 1;
            }
            continue;
        }

        // 2.0.17: these 3 cells used to be folded into Sale.notes as plain
        // text - now they (optionally) create a real linked `pulls_received`
        // row instead, once the sale itself is safely created below. See
        // `maybe_link_pull_received`'s own doc comment.
        let pull_cell = cell(raw_row, pull_col);
        let who_pulled_cell = cell(raw_row, who_pulled_col);
        let how_much_pull_cell = cell(raw_row, how_much_pull_col);

        let batch_input = SaleBatchInput {
            lines: sellable_ticket_ids
                .iter()
                .map(|&ticket_id| SaleBatchLineInput { ticket_id, sale_price_cents, selling_fees_cents: 0 })
                .collect(),
            platform_id,
            sale_date,
            payment_status: Some(payment_status),
            buyer_reference: cell(raw_row, paid_by_col),
            notes: None,
            // 2.0.57: Sheets' Sales tab has no currency column of its own
            // (see `SaleBatchInput::currency`'s own doc comment) - keep
            // deriving each line's currency from its own ticket, exactly as
            // before this field existed.
            currency: None,
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
                maybe_link_pull_received(
                    conn,
                    order_id,
                    sellable_ticket_ids.len() as i64,
                    pull_cell.as_deref(),
                    who_pulled_cell.as_deref(),
                    how_much_pull_cell.as_deref(),
                )?;
                result.created += 1;
            }
            Err(e) => {
                result.errors.push(SheetSyncIssue { row_number, message: e.to_string() });
            }
        }
    }

    Ok(result)
}

/// Mirrors a `pull` = "yes" cell into a real, linked `pulls_received` row
/// (2.0.17) - see this module's own doc comment's "Column mapping (second
/// batch)" table for the full rationale. Does nothing at all (not even
/// counted as an error) unless `pull_cell` trims+lowercases to exactly "yes"
/// AND `who_pulled_cell` isn't blank - `who_pulled_cell` becomes
/// `puller_name`, which the schema requires, so there is simply nothing
/// sensible to create without it. Returns whether it actually linked a new
/// row - `false` covers every "nothing to do" case above, plus "already
/// linked" below - so callers can tell "created something new" apart from
/// "truly nothing happened" for their own `SheetSyncResult` counters.
///
/// 2.0.23: called from TWO places in `apply_sales_rows` now - right after a
/// sale is newly created (as before 2.0.23), AND, separately, on a row whose
/// order was ALREADY fully sold before this sync even started (see that
/// function's own `sellable_ticket_ids.is_empty()` branch). marko's real
/// workflow is often "sync the sale first, fill in pull info on that same
/// row later" - he expects that to still end up linked on the next sync,
/// not only when both happen to land in the same sync pass.
///
/// Idempotent per order regardless of which of those two call sites reaches
/// it, or how many times: guarded by a `SELECT` before inserting (and backed
/// by a DB-level partial UNIQUE index as a second line of defence - see
/// migrations/011_pulls_received.sql), so an order can never end up with two
/// linked rows no matter how many times it gets synced. Once linked though,
/// a `pulls_received` row is still never revisited or updated by a later
/// sync - only the one-time act of creating the link at all is
/// re-attempted on every sync; the row's own fields, once created, follow
/// the same creation-only philosophy as the rest of this module (marko can
/// still edit it by hand in the app - see commands/pulls_received.rs).
///
/// A blank or unparseable "how much pull" cell defaults `amount_cents` to 0
/// rather than blocking anything - this whole record is informational only
/// (marko confirmed via AskUserQuestion: never affects Profit/Revenue).
/// event_name/event_date/currency are copied from the order's own linked
/// event/currency rather than asked of the sheet a second time, since the
/// sheet has no columns of its own for either. `quantity` is passed in by
/// the caller rather than derived here, since its correct source differs
/// between the two call sites (the tickets just sold THIS pass, vs. the
/// order's total already-sold count for a row that was already fully sold
/// before this sync started).
fn maybe_link_pull_received(
    conn: &Connection,
    order_id: i64,
    quantity: i64,
    pull_cell: Option<&str>,
    who_pulled_cell: Option<&str>,
    how_much_pull_cell: Option<&str>,
) -> AppResult<bool> {
    let pull_is_yes = pull_cell.map(|v| v.trim().eq_ignore_ascii_case("yes")).unwrap_or(false);
    if !pull_is_yes {
        return Ok(false);
    }
    let Some(puller_name) = who_pulled_cell.map(str::trim).filter(|v| !v.is_empty()) else {
        return Ok(false);
    };

    let already_linked: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM pulls_received WHERE order_id = ?1 AND source = 'sheet_sync')",
        [order_id],
        |r| r.get(0),
    )?;
    if already_linked {
        return Ok(false);
    }

    let (event_id, currency): (i64, String) =
        conn.query_row("SELECT event_id, currency FROM orders WHERE id = ?1", [order_id], |r| Ok((r.get(0)?, r.get(1)?)))?;
    let (event_name, event_date): (String, Option<String>) =
        conn.query_row("SELECT name, event_date FROM events WHERE id = ?1", [event_id], |r| Ok((r.get(0)?, r.get(1)?)))?;

    let amount_cents = how_much_pull_cell.and_then(|v| parse_decimal_to_cents(v).ok()).unwrap_or(0);

    let input = PullReceivedInput {
        puller_name: puller_name.to_string(),
        event_name,
        event_date,
        quantity,
        amount_cents,
        currency,
        more_info: None,
        order_id: Some(order_id),
    };
    pulls_received::create_pull_received_with_source(conn, &input, false, "sheet_sync")?;
    Ok(true)
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

    refresh_sheet_structure_soft_fail(conn, token, &connection.spreadsheet_id, &connection.sheet_tab, &headers, data_rows, &mut result);

    result.synced_at = now_iso(conn)?;
    set_setting(conn, &last_synced_key("orders"), &result.synced_at)?;
    Ok(result)
}

/// Manual "Sales sync" button (Settings -> Integrations, Orders & Sales
/// card) - sits next to "Order sync" on the same card, same connection.
/// Never runs on its own.
#[tauri::command]
pub fn sync_sales(state: State<AppState>) -> AppResult<SheetSyncResult> {
    let mut conn = state.db.lock().unwrap();
    sync_sales_impl(&mut conn)
}

// ---------------------------------------------------------------------------
// Push (app -> sheet), 2.0.18 - the Sales-sync half. Unlike Order push
// above, this never appends a new row at all - it only ever fills in the
// Sales-sync batch of columns (`Site Listed` through `paid by`, plus `pull`/
// `who pulled`/`how much pull`) on a row that is ALREADY linked (has a
// "TIQR ID" - whether that got there via Order sync or Order push), and only
// when every one of those target cells is still completely blank. That
// "only when fully blank" rule is deliberately simpler than Pulls push's own
// snapshot-based conflict detection: marko's normal workflow already has him
// filling the sheet's Sales-sync columns in by hand and then running "Sales
// sync" to pull them in, so a row that already has ANY value there is left
// alone unconditionally, never compared cell-by-cell - there is no realistic
// case where this app would know better than a value marko already typed,
// and "never touch a row with anything already in it" is impossible to get
// subtly wrong. The rows this actually helps are the ones that came from
// Order push itself (blank Sales-sync columns by construction) and later
// sold from inside the app.
// ---------------------------------------------------------------------------

/// One ticket's currently active (non-refunded) sale, just the fields
/// `uniform_sale_for_order` needs to compare across every ticket of an
/// order. A ticket with `status = 'sold'` is guaranteed to have exactly one
/// of these - `refund_sale_impl` atomically flips the sale to `refunded` AND
/// the ticket back to `available` in the same transaction, so "sold but no
/// active sale" is not a state this app can normally produce (see that
/// function's own doc comment) - `uniform_sale_for_order` still treats it as
/// "can't push" rather than panicking, on this module's usual "never guess"
/// principle.
struct ActiveSale {
    platform_id: Option<i64>,
    sale_price_cents: i64,
    payment_status: String,
    sale_date: String,
    buyer_reference: Option<String>,
}

/// What `push_sales` actually writes for one order's Sales-sync batch of
/// columns, once every ticket agrees closely enough to be represented as
/// this sheet's single row per order.
struct UniformSale {
    platform_name: Option<String>,
    sale_price_cents: i64,
    payment_status: String,
    sale_date: String,
    buyer_reference: Option<String>,
    resale_status: Option<String>,
    delivery_status: Option<String>,
}

/// `Ok(None)` for anything short of "every ticket in this order is sold,
/// with one identical active sale (same platform/price/date/payment status/
/// buyer reference) and identical resale_status/delivery_status stamped on
/// every ticket" - not ready yet, or a real but sheet-unrepresentable state
/// (e.g. marko split one order's tickets across two different listings at
/// two different prices - the sheet only has one row for the whole order).
/// Both are simply "nothing to push" here, not an error - see this
/// section's own doc comment for why marko is never alarmed about either.
fn uniform_sale_for_order(conn: &Connection, order_id: i64) -> AppResult<Option<UniformSale>> {
    let tickets: Vec<(i64, String, Option<String>, Option<String>)> = {
        let mut stmt = conn.prepare("SELECT id, status, resale_status, delivery_status FROM tickets WHERE order_id = ?1 ORDER BY id")?;
        let rows = stmt
            .query_map([order_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        rows
    };
    if tickets.is_empty() || tickets.iter().any(|(_, status, _, _)| status != "sold") {
        return Ok(None);
    }
    let (first_resale, first_delivery) = (tickets[0].2.clone(), tickets[0].3.clone());
    if !tickets.iter().all(|(_, _, rs, ds)| *rs == first_resale && *ds == first_delivery) {
        return Ok(None);
    }

    let mut sales: Vec<ActiveSale> = Vec::with_capacity(tickets.len());
    for (ticket_id, _, _, _) in &tickets {
        let sale: Option<ActiveSale> = conn
            .query_row(
                "SELECT platform_id, sale_price_cents, payment_status, sale_date, buyer_reference
                 FROM sales WHERE ticket_id = ?1 AND payment_status != 'refunded'",
                [ticket_id],
                |r| {
                    Ok(ActiveSale {
                        platform_id: r.get(0)?,
                        sale_price_cents: r.get(1)?,
                        payment_status: r.get(2)?,
                        sale_date: r.get(3)?,
                        buyer_reference: r.get(4)?,
                    })
                },
            )
            .optional()?;
        let Some(sale) = sale else {
            return Ok(None);
        };
        sales.push(sale);
    }

    let first = &sales[0];
    let uniform = sales.iter().all(|s| {
        s.platform_id == first.platform_id
            && s.sale_price_cents == first.sale_price_cents
            && s.payment_status == first.payment_status
            && s.sale_date == first.sale_date
            && s.buyer_reference == first.buyer_reference
    });
    if !uniform {
        return Ok(None);
    }

    let platform_name: Option<String> = match first.platform_id {
        Some(pid) => conn.query_row("SELECT name FROM platforms WHERE id = ?1", [pid], |r| r.get(0)).optional()?,
        None => None,
    };

    Ok(Some(UniformSale {
        platform_name,
        sale_price_cents: first.sale_price_cents,
        payment_status: first.payment_status.clone(),
        sale_date: first.sale_date.clone(),
        buyer_reference: first.buyer_reference.clone(),
        resale_status: first_resale,
        delivery_status: first_delivery,
    }))
}

/// 2.0.80: marko's own bug report - once every ticket on an order has been
/// refunded, `uniform_sale_for_order` above correctly starts returning `None`
/// for it (a refunded ticket goes back to `status = 'available'`, so it fails
/// that function's own first check - see `refund_sale_impl`'s doc comment),
/// but NOTHING was ever pushing that change to the sheet: the row's Sales-
/// sync columns (Site Listed/Payout Per Ticket/Status/Delivery status/Payout
/// status/Sale date/paid by) simply kept whatever they said before the
/// refund, forever - not even "Fix sync" could correct them, since its force
/// path also goes through `uniform_sale_for_order` and finds nothing to
/// compare against. Two real consequences, both reported at once: the
/// Summary block's Total Revenue/Profit/Paid/Unpaid kept counting a sale that
/// no longer exists (see `plan_orders_summary_updates`'s own 2.0.80 doc
/// comment for the other half of the same report), and a future "Sales
/// sync" pull could mistake the still-non-blank `Payout Per Ticket` cell for
/// a brand new sale ready to record - `apply_sales_rows` only checks whether
/// that one cell is blank, not whether the ticket has a refund in its
/// history - silently creating a duplicate sale from stale numbers.
///
/// `true` exactly when this order needs its stale row cleared: nothing on it
/// is currently `sold`, but at least one of its tickets carries a refunded
/// sale in `sales` - i.e. this row used to be represented as sold in the
/// sheet and no longer should be. `false` for an order that was simply never
/// sold at all (its target cells are already blank, so clearing would be a
/// no-op anyway) and for a partially-refunded order (some tickets still
/// genuinely `sold` - `sold_count` above zero) - same "can't represent,
/// leave it alone" territory `uniform_sale_for_order` already stakes out for
/// any other non-uniform mix, not something this function tries to improve
/// on.
fn order_fully_refunded(conn: &Connection, order_id: i64) -> AppResult<bool> {
    let (sold_count, refunded_count): (i64, i64) = conn.query_row(
        "SELECT
            (SELECT COUNT(*) FROM tickets WHERE order_id = ?1 AND status = 'sold'),
            (SELECT COUNT(*) FROM tickets t JOIN sales s ON s.ticket_id = t.id
             WHERE t.order_id = ?1 AND s.payment_status = 'refunded')",
        [order_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    Ok(sold_count == 0 && refunded_count > 0)
}

/// One order can have several manually-added `pulls_received` rows linked
/// to it (only the auto-linked-from-sheet-sync kind is limited to one per
/// order - see migrations/011_pulls_received.sql's partial unique index) -
/// `Ok(None)` whenever there isn't *exactly* one, same "can't represent,
/// nothing to push" spirit as `uniform_sale_for_order` above, rather than
/// guessing which one the sheet's single trio of columns should show.
fn linked_pull_received_for_order(conn: &Connection, order_id: i64) -> AppResult<Option<(String, i64)>> {
    let rows: Vec<(String, i64)> = {
        let mut stmt =
            conn.prepare("SELECT puller_name, amount_cents FROM pulls_received WHERE order_id = ?1 AND is_demo = 0 ORDER BY id")?;
        let rows = stmt.query_map([order_id], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<Result<Vec<_>, _>>()?;
        rows
    };
    if rows.len() == 1 {
        Ok(Some(rows.into_iter().next().unwrap()))
    } else {
        Ok(None)
    }
}

/// The push direction's own core for Sales sync - see this section's own doc
/// comment for the "only when fully blank" rule. Walks every linked order
/// (`sheet_sync_links` for data_source `"orders"`, regardless of whether it
/// got linked via Order sync or Order push), finds its current row in the
/// sheet's already-fetched data, and queues per-cell writes for whichever of
/// the two independent column groups (sale info / linked pull info) is both
/// ready on the app side and still blank on the sheet side. An order whose
/// marker isn't found anywhere in the sheet's current data (row deleted, or
/// an Order push whose append never actually landed) is quietly skipped,
/// not reported - `push_orders`/a future re-sync already surfaces that
/// loudly on its own.
fn apply_sales_push(
    conn: &Connection,
    headers: &[String],
    data_rows: &[Vec<String>],
    marker_col_index: usize,
) -> AppResult<(SheetSyncResult, Vec<(i64, Vec<(usize, String)>)>)> {
    apply_sales_push_internal(conn, headers, data_rows, marker_col_index, false)
}

/// "Fix sync" (2.0.60) calls this with `force: true` instead - see
/// `force_push_sales_impl`'s own doc comment for why that button exists.
/// Shares every bit of setup and the uniform-sale/linked-pull lookups with
/// the ordinary push above (which is now just `force: false` here), so the
/// two buttons can never quietly drift apart on what counts as "ready to
/// push" - only what happens once a target cell turns out NOT to be blank
/// differs between them.
fn apply_sales_push_internal(
    conn: &Connection,
    headers: &[String],
    data_rows: &[Vec<String>],
    marker_col_index: usize,
    force: bool,
) -> AppResult<(SheetSyncResult, Vec<(i64, Vec<(usize, String)>)>)> {
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

    let mut marker_row_map: HashMap<String, usize> = HashMap::new();
    for (i, raw_row) in data_rows.iter().enumerate() {
        if let Some(marker) = cell(raw_row, Some(marker_col_index)) {
            marker_row_map.insert(marker, i);
        }
    }

    let mut result =
        SheetSyncResult { created: 0, updated: 0, unchanged: 0, conflicts: vec![], errors: vec![], corrected: vec![], synced_at: String::new() };
    let mut writes: Vec<(i64, Vec<(usize, String)>)> = vec![];

    let links: Vec<(i64, String)> = {
        let mut stmt = conn.prepare(
            "SELECT l.local_id, l.sheet_marker FROM sheet_sync_links l
             JOIN orders o ON o.id = l.local_id
             WHERE l.data_source = 'orders' AND o.is_demo = 0
             ORDER BY l.local_id",
        )?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<Result<Vec<_>, _>>()?;
        rows
    };

    for (order_id, marker) in links {
        let Some(&row_idx) = marker_row_map.get(&marker) else {
            continue;
        };
        let row_number = (row_idx + 2) as i64;
        let raw_row = &data_rows[row_idx];

        let mut cells: Vec<(usize, String)> = vec![];

        if let Some(sale) = uniform_sale_for_order(conn, order_id)? {
            let target_cols = [site_listed_col, payout_col, status_col, delivery_status_col, payout_status_col, sale_date_col, paid_by_col];
            if target_cols.iter().all(|c| cell(raw_row, *c).is_none()) {
                if let Some(c) = site_listed_col {
                    cells.push((c, sale.platform_name.clone().unwrap_or_default()));
                }
                if let Some(c) = payout_col {
                    cells.push((c, format_cents_for_sheet(sale.sale_price_cents)));
                }
                if let Some(c) = status_col {
                    cells.push((c, sale.resale_status.clone().unwrap_or_default()));
                }
                if let Some(c) = delivery_status_col {
                    cells.push((c, sale.delivery_status.clone().unwrap_or_default()));
                }
                if let Some(c) = payout_status_col {
                    cells.push((c, sale.payment_status.clone()));
                }
                if let Some(c) = sale_date_col {
                    cells.push((c, format_order_date_for_sheet(&sale.sale_date)));
                }
                if let Some(c) = paid_by_col {
                    cells.push((c, sale.buyer_reference.clone().unwrap_or_default()));
                }
            } else if force {
                // 2.0.60 "Fix sync": the ordinary rule above (write only when
                // EVERY target cell is still blank) just failed - exactly the
                // situation marko hit with a real sale. Instead of skipping
                // the whole group, compare each cell to what the app would
                // put there and queue a write only for the ones that
                // actually disagree. A cell that's already correct - blank
                // or not - is left completely alone, so running this again
                // on an already-fixed row is always a no-op.
                //
                // 2.0.61 correction: `sale.resale_status`/`sale.delivery_
                // status`/`sale.platform_name`/`sale.buyer_reference` are
                // each `Option<String>`, and a ticket sold through the app's
                // own "record a sale" flow (Sales.tsx / the Dashboard "New
                // sale" shortcut - the exact flow behind the original Push
                // sales report) never sets resale_status/delivery_status at
                // all - only Sales sync's PULL direction stamps those two
                // from the sheet's own Status/Delivery status columns (see
                // `apply_sales_rows`). So `None` here does not mean "the app
                // knows this should be blank" - it means "the app has no
                // opinion on this cell" - and the first version of this
                // branch used `.unwrap_or_default()` to turn that `None`
                // into `""`, which then "disagreed" with whatever real value
                // (e.g. "Listed"/"Not yet", typed in by marko or written by
                // an earlier push) already sat in the cell, and blanked it.
                // That is exactly the regression marko reported (Fix sync
                // wiping Status/Delivery status on rows that were already
                // correct). Fixed by only ever comparing/writing a field the
                // app actually has a value for; a field the app has no
                // opinion on is skipped entirely, whatever the sheet holds.
                let desired: [(Option<usize>, Option<String>); 7] = [
                    (site_listed_col, sale.platform_name.clone()),
                    (payout_col, Some(format_cents_for_sheet(sale.sale_price_cents))),
                    (status_col, sale.resale_status.clone()),
                    (delivery_status_col, sale.delivery_status.clone()),
                    (payout_status_col, Some(sale.payment_status.clone())),
                    (sale_date_col, Some(format_order_date_for_sheet(&sale.sale_date))),
                    (paid_by_col, sale.buyer_reference.clone()),
                ];
                for (col_opt, value_opt) in desired {
                    let (Some(c), Some(value)) = (col_opt, value_opt) else { continue };
                    if cell(raw_row, Some(c)).as_deref() != Some(value.as_str()) {
                        cells.push((c, value));
                    }
                }
            }
        } else if order_fully_refunded(conn, order_id)? {
            // 2.0.80: see order_fully_refunded's own doc comment for the full
            // incident. Blank every one of the 7 Sales-sync target cells that
            // isn't already blank - the exact same columns uniform_sale_for_
            // order's own branch above would have written - so the row goes
            // back to looking like a not-yet-sold ticket: Revenue/Profit
            // formulas naturally re-evaluate to 0 for it with no change
            // needed to the Summary formulas themselves, and it's ready to
            // represent a genuine future resale if marko fills it in again.
            // Deliberately unconditional on `force`, unlike the block above -
            // this isn't "the app's opinion might be wrong so only overwrite
            // when marko asks for a repair" - a refunded ticket is definitely
            // not an active sale any more, so there is nothing to weigh
            // against. Pull/who pulled/how much pull are a separate concern
            // (that money was actually received, refunding the resale later
            // doesn't undo it) and are never touched here.
            let target_cols = [site_listed_col, payout_col, status_col, delivery_status_col, payout_status_col, sale_date_col, paid_by_col];
            for c in target_cols.into_iter().flatten() {
                if cell(raw_row, Some(c)).is_some() {
                    cells.push((c, String::new()));
                }
            }
        }

        if let Some((puller_name, amount_cents)) = linked_pull_received_for_order(conn, order_id)? {
            let target_cols = [pull_col, who_pulled_col, how_much_pull_col];
            if target_cols.iter().all(|c| cell(raw_row, *c).is_none()) {
                if let Some(c) = pull_col {
                    cells.push((c, "yes".to_string()));
                }
                if let Some(c) = who_pulled_col {
                    cells.push((c, puller_name.clone()));
                }
                if let Some(c) = how_much_pull_col {
                    cells.push((c, format_cents_for_sheet(amount_cents)));
                }
            } else if force {
                let desired: [(Option<usize>, String); 3] = [
                    (pull_col, "yes".to_string()),
                    (who_pulled_col, puller_name.clone()),
                    (how_much_pull_col, format_cents_for_sheet(amount_cents)),
                ];
                for (col_opt, value) in desired {
                    if let Some(c) = col_opt {
                        if cell(raw_row, Some(c)).as_deref() != Some(value.as_str()) {
                            cells.push((c, value));
                        }
                    }
                }
            }
        }

        if cells.is_empty() {
            result.unchanged += 1;
        } else {
            result.updated += 1;
            writes.push((row_number, cells));
        }
    }

    Ok((result, writes))
}

/// The push direction's own network-calling shell for Sales sync. Never
/// creates the "TIQR ID" marker column if it's missing (unlike
/// `push_orders_impl`) - this never assigns a new marker to anything, only
/// ever matches against markers that already exist, so an absent marker
/// column just means every link fails to find its row (handled the same as
/// any other not-found row) rather than something worth creating.
fn push_sales_impl(conn: &Connection) -> AppResult<SheetSyncResult> {
    let connection = load_connection(conn, "orders")?
        .ok_or_else(|| AppError::Validation("No spreadsheet is connected for Orders yet - connect one in Settings first.".to_string()))?;
    let credential = crate::commands::google_auth::resolve_google_credential(conn, false)?;
    let token = credential.access_token();

    let range = google_sheets::a1_range(&connection.sheet_tab, "A1:AZ");
    let value_range = google_sheets::get_values(token, &connection.spreadsheet_id, &range)?;
    if value_range.values.is_empty() {
        return Err(AppError::Validation("The connected sheet/tab has no header row yet.".to_string()));
    }
    let headers = value_range.values[0].clone();
    let data_rows: &[Vec<String>] = if value_range.values.len() > 1 { &value_range.values[1..] } else { &[] };

    let (marker_col_index, _marker_exists) = resolve_marker_column(&headers);

    let (mut result, writes) = apply_sales_push(conn, &headers, data_rows, marker_col_index)?;

    for (sheet_row_number, cells) in writes {
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

    // 2.0.54: same currency catch-up as push_orders_impl - see
    // reconcile_order_currencies' own doc comment for why. marko asked for
    // this on both buttons, not just Push Orders.
    let currency_writes = reconcile_order_currencies(conn, &headers, data_rows);
    for (row_number, cells) in currency_writes {
        let mut row_ok = true;
        for (col, value) in cells {
            let col_letter = column_index_to_a1(col);
            let cell_range = google_sheets::a1_range(&connection.sheet_tab, &format!("{col_letter}{row_number}"));
            if let Err(e) = google_sheets::update_values(token, &connection.spreadsheet_id, &cell_range, &[vec![value]]) {
                row_ok = false;
                result.errors.push(SheetSyncIssue { row_number, message: format!("currency catch-up failed: {e}") });
            }
        }
        if row_ok {
            result.updated += 1;
        }
    }

    refresh_sheet_structure_soft_fail(conn, token, &connection.spreadsheet_id, &connection.sheet_tab, &headers, data_rows, &mut result);

    result.synced_at = now_iso(conn)?;
    set_setting(conn, &last_pushed_key("orders"), &result.synced_at)?;
    Ok(result)
}

/// "Push sales" button (Settings -> Integrations, Orders & Sales card) -
/// the new sibling of "Sales sync". Never runs on its own.
#[tauri::command]
pub fn push_sales(state: State<AppState>) -> AppResult<SheetSyncResult> {
    let conn = state.db.lock().unwrap();
    push_sales_impl(&conn)
}

/// "Fix sync" (2.0.60, narrowed in 2.0.61) - marko's own request after a
/// real sale made via the Dashboard's "New sale" shortcut didn't get pushed
/// into the sheet by the ordinary "Push sales" button, for a reason that
/// couldn't be pinned down from what was available to check (the order
/// already had a sheet row, every ticket in it sold at once at one
/// identical price, and the target cells were blank beforehand - by
/// push_sales_impl's own rule that should already have been enough for it
/// to write). Rather than keep guessing at increasingly specific hypotheses
/// against marko's real, live spreadsheet, this is a separate, more
/// permissive repair action: it shares every bit of "is this order even
/// ready to push" logic with push_sales_impl (apply_sales_push_internal,
/// uniform_sale_for_order, linked_pull_received_for_order - none of that
/// changes here), but drops the "only if every target cell is still blank"
/// gate and instead overwrites individual cells whenever they disagree with
/// a value the app actually has for them (never a field the app has no
/// opinion on - see apply_sales_push_internal's own `force` branch comment,
/// added in 2.0.61 after this button wiped a real Status/Delivery status
/// value on the first try). An order that still doesn't have one clean
/// uniform sale (or more than one linked pull) is left alone here exactly
/// like it is by push_sales_impl - this button repairs a push that should
/// have happened, it doesn't invent a value for a state this module has
/// never been able to represent as one sheet row. Because it only ever
/// queues a cell whose current text actually differs from a value the app
/// actually has, running this repeatedly - or on a sheet that's already
/// correct - never touches anything and is always safe to click again.
///
/// 2.0.61: no longer also runs the currency catch-up
/// (`reconcile_order_currencies`) or the structure refresh
/// (`refresh_sheet_structure_soft_fail`) that `push_sales_impl` runs at its
/// own tail - see this function's own body comment for why marko's "Total
/// Cost" summary going to 0 right after using this button pointed straight
/// at the currency catch-up step, and why removing it here (rather than
/// changing what `push_sales_impl` itself does) is the correct fix.
fn force_push_sales_impl(conn: &Connection) -> AppResult<SheetSyncResult> {
    let connection = load_connection(conn, "orders")?
        .ok_or_else(|| AppError::Validation("No spreadsheet is connected for Orders yet - connect one in Settings first.".to_string()))?;
    let credential = crate::commands::google_auth::resolve_google_credential(conn, false)?;
    let token = credential.access_token();

    let range = google_sheets::a1_range(&connection.sheet_tab, "A1:AZ");
    let value_range = google_sheets::get_values(token, &connection.spreadsheet_id, &range)?;
    if value_range.values.is_empty() {
        return Err(AppError::Validation("The connected sheet/tab has no header row yet.".to_string()));
    }
    let headers = value_range.values[0].clone();
    let data_rows: &[Vec<String>] = if value_range.values.len() > 1 { &value_range.values[1..] } else { &[] };

    let (marker_col_index, _marker_exists) = resolve_marker_column(&headers);

    let (mut result, writes) = apply_sales_push_internal(conn, &headers, data_rows, marker_col_index, true)?;

    for (sheet_row_number, cells) in writes {
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

    // 2.0.61: this used to also re-run the 2.0.54 currency catch-up
    // (`reconcile_order_currencies`) and the dropdown/Revenue-Profit/Summary
    // structure refresh (`refresh_sheet_structure_soft_fail`) here, on the
    // reasoning that "Fix sync" should clean up anything the sheet is
    // missing, not just the sale-info/pull-info repair above. Removed:
    // marko never asked for either of those, and `reconcile_order_currencies`
    // pushes whatever `orders.total_cost_cents` currently holds for EVERY
    // linked order straight into its sheet row's "Total Purchase Price" cell
    // the moment the two disagree, with no check that the app's own number
    // still makes sense - the same run that fixed the Status/Delivery status
    // regression (see the force-branch's own 2.0.61 comment above) is the
    // most likely way marko's "Total Cost" summary also went to 0 right
    // after clicking this button. "Push sales"/"Push orders" still run both
    // of those steps exactly as before (unchanged, not implicated here) -
    // this button is now scoped to only what marko actually asked it to fix.
    result.synced_at = now_iso(conn)?;
    set_setting(conn, &last_pushed_key("orders"), &result.synced_at)?;
    Ok(result)
}

/// "Fix sync" button (Settings -> Integrations, Orders & Sales card) - see
/// `force_push_sales_impl`'s own doc comment for exactly how this differs
/// from "Push sales" right above it on the same card.
#[tauri::command]
pub fn force_push_sales(state: State<AppState>) -> AppResult<SheetSyncResult> {
    let conn = state.db.lock().unwrap();
    force_push_sales_impl(&conn)
}

// ---------------------------------------------------------------------------
// "Create a new sheet for me" (2.0.9) - mirrors pulls_sheet_sync.rs's own
// PULLS_SHEET_HEADERS/NEW_SHEET_TITLE/NEW_SHEET_TAB_NAME/
// create_pulls_sheet_impl/create_pulls_sheet exactly, reusing that module's
// validate_share_email/validate_currency directly (see their doc comments)
// rather than duplicating them - only the header list, sheet name, and
// data_source string differ.
// ---------------------------------------------------------------------------

/// Header row written into a freshly-created "Orders & Sales" sheet.
/// Covers BOTH sync entry points that read this one connection (2.0.11) -
/// marko's own real sheet is one combined buy+sell tracker, and a freshly
/// auto-created sheet must be immediately ready for both "Order sync" AND
/// "Sales sync" with zero manual column-editing first, exactly like his own
/// real sheet already is. In marko's own exact column order: the 13 columns
/// `apply_order_rows` understands (Order sync's own batch - see the "Column
/// mapping (first batch)" table above), followed by the columns
/// `apply_sales_rows` understands (Sales sync's own batch - `Site Listed`
/// through `how much pull`, see the "Column mapping (second batch)" table
/// above), including `Revenue`/`Profit` as plain header text - a brand-new
/// sheet has no data rows yet, so there is nothing to put a formula into
/// here; `ensure_orders_sheet_structure` (2.0.19, see "Sheet structure"
/// below) fills in the live formulas the very first time a real data row
/// exists under these headers, exactly as it does for any other sheet.
/// Deliberately excludes `TIQR ID` -
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

/// Creates a brand-new Google Sheet for Orders & Sales, writes
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

// ---------------------------------------------------------------------------
// Sheet structure (dropdowns + Revenue/Profit formulas), 2.0.19 - marko's own
// request ("tabuľka sa musí automaticky aktualizovať presne tak, ako má
// byť"). Keeps 6 columns restricted to a real dropdown (Ticket Type/Site
// Listed/Status/Delivery status/Payout status/pull) and Revenue/Profit as
// live formulas, across the WHOLE sheet - not just rows the app created or
// has ever synced (marko confirmed via AskUserQuestion: "všetky riadky"/all
// rows, over "len riadky, čo appka pozná"). `ensure_orders_sheet_structure`
// is called at the end of every one of this module's four commands
// (sync_orders/sync_sales/push_orders/push_sales) so the sheet stays
// correctly set up no matter which button marko happens to click - there is
// no separate "fix my sheet" button to remember to run.
//
// Split into a pure planning function (`plan_sheet_structure_updates` - unit
// tested below, no network) and a network shell
// (`ensure_orders_sheet_structure`) that turns the plan into real
// `batchUpdate`/`update_values_as_formulas` calls - the same "impl function +
// thin network shell" split this whole module already uses everywhere else.
// ---------------------------------------------------------------------------

/// Fixed dropdown options - unlike Ticket Type/Site Listed below, these three
/// are never extended by anything the app or the sheet discovers; marko gave
/// an exact, closed list for each, so there is nothing to grow.
const STATUS_OPTIONS: &[&str] = &["Listed", "Unlisted", "Sold"];
const DELIVERY_STATUS_OPTIONS: &[&str] = &["Delivered", "Not delivered"];
const PAYOUT_STATUS_OPTIONS: &[&str] = &["Pending", "Paid"];
const PULL_OPTIONS: &[&str] = &["Yes", "No"];

/// Background colors for `plan_sheet_color_updates` below - marko's own
/// request (2.0.22), given as plain color names ("oranzova"/"hneda"/
/// "zelena"), not exact shades, so these are this app's own reasonable pick
/// for each - light/pastel enough that the cell's default black text stays
/// easily readable on top. `(red, green, blue)`, each 0.0-1.0, the Sheets API
/// `Color` shape `google_sheets::add_conditional_format_color_request`
/// expects directly.
const COLOR_GREEN: (f64, f64, f64) = (0.71, 0.88, 0.80);
const COLOR_ORANGE: (f64, f64, f64) = (0.99, 0.80, 0.61);
const COLOR_BROWN: (f64, f64, f64) = (0.82, 0.70, 0.55);

/// How many data rows (below the header) the dropdown validations cover, at
/// minimum - deliberately far beyond however much data is in the sheet right
/// now, so marko (or a future sync) can add a new row and immediately have a
/// working dropdown on it, without needing to remember to re-run anything
/// just to get one. Formulas are NOT covered by this constant - see
/// `plan_sheet_structure_updates`'s own doc comment for why those only ever
/// cover rows that actually have data.
const DROPDOWN_ROW_BUFFER: i64 = 500;

/// 2.0.42: the row bound used by the Summary block's Total Paid/Total Unpaid
/// formulas (`plan_orders_summary_updates`), which need SUMPRODUCT rather
/// than a true whole-column reference like `C:C` - see that function's own
/// doc comment for why. Unlike SUM/SUMIF (which skip past empty trailing
/// cells cheaply), SUMPRODUCT is a real array function that materializes
/// its whole argument, so a genuine whole-column SUMPRODUCT can be
/// meaningfully slower on a large sheet. 100,000 rows is nowhere close to
/// anything marko's real sheet will ever hold (its default grid is ~1,000
/// rows - see `grow_grid_request_if_needed`'s own history) while still
/// being effectively "cover everything he could ever type here".
const SUMPRODUCT_ROW_BOUND: i64 = 100_000;

/// `Sale.platform_id`'s own name pool, filtered to platforms that can
/// actually be a SALE platform (`kind IN ('sale','both')`) - the exact same
/// filter the Sales screen's own platform picker already uses, and the same
/// `kind` column `resolve_or_create_sale_platform` above already maintains.
/// Feeds Site Listed's dropdown only - `platform` (the PURCHASE-side column)
/// is deliberately untouched by this whole section, marko was explicit that
/// part of the sheet is already good as it is.
fn sale_platform_names(conn: &Connection) -> AppResult<Vec<String>> {
    let mut stmt =
        conn.prepare("SELECT name FROM platforms WHERE kind IN ('sale', 'both') AND is_demo = 0 ORDER BY name COLLATE NOCASE")?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

struct DropdownSpec {
    col_index: usize,
    values: Vec<String>,
}

struct FormulaSpec {
    col_index: usize,
    /// One formula string per data row, e.g. `"=O2*I2"` for row 2 - already
    /// resolved to this sheet's own real column letters, ready to send to
    /// `google_sheets::update_values_as_formulas` exactly as-is.
    formulas: Vec<String>,
}

/// Pure core: given the sheet's actual headers (tolerant of reordering/
/// extra columns, same `find_col`/`build_header_map` matching every other
/// function in this module already uses) and how many data rows it
/// currently has, works out exactly which dropdown columns exist and what
/// their option lists should be right now, plus the Revenue/Profit formulas
/// (when their required source columns are present). No network calls - see
/// `ensure_orders_sheet_structure` for the network shell that sends this as
/// real Sheets API requests.
///
/// A dropdown column that isn't in `headers` at all is simply skipped - the
/// same "an unknown/absent column is not an error" tolerance this module
/// already applies everywhere else. Site Listed is additionally skipped
/// when there are currently zero sale platforms to offer - nothing
/// meaningful to restrict the column to yet.
///
/// Revenue needs `Payout Per Ticket` + `Number of Tickets` + its own column;
/// Profit additionally needs `Total Purchase Price`. Either formula is
/// skipped independently of the other if any of ITS OWN required columns is
/// missing - same optional-column tolerance as everywhere else. Formulas
/// only ever cover `data_row_count` rows - unlike the dropdown buffer above,
/// writing a formula into a truly empty future row would show a stray "0"
/// for a nonexistent order, so a short/empty sheet simply gets fewer (or
/// zero) formula rows for now, picked up automatically once real data
/// reaches them on a later sync/push.
///
/// **2.0.43: Profit is additionally gated on `Status = "Sold"` when a
/// Status column exists in the sheet.** marko's own report: for a ticket
/// that has not sold yet, `Payout Per Ticket` is blank (0 in Sheets
/// arithmetic), so Revenue is legitimately 0 - but Profit
/// (`Revenue - Total Purchase Price`) still worked out to a large,
/// misleading NEGATIVE number, looking exactly like a loss on every ticket
/// still sitting unsold. The gate is written as
/// `(Status="Sold")*(Revenue-Total)` - a plain boolean-multiply expression,
/// deliberately NOT `IF(...)`: `IF` is itself a multi-argument function
/// call and would reintroduce the exact same comma-vs-semicolon locale trap
/// `plan_orders_summary_updates` was already rewritten off `SUMIF` for in
/// 2.0.42 (see that function's own doc comment). This stays locale-safe by
/// construction - only `=`/`*`/`-`, parens, and a quoted literal, never a
/// function-argument separator. A sheet with no Status column at all falls
/// back to the unconditional formula this function always produced before
/// 2.0.43 - same optional-column tolerance as everywhere else here.
/// `Total Profit` in the Summary block (`plan_orders_summary_updates`,
/// `=SUM(profit:profit)`) needed no direct change: `SUM` already silently
/// adds whatever real number ends up in each row - 0 for a not-yet-sold
/// ticket, the real profit for a sold one - so fixing this one formula
/// fixes that total automatically.
fn plan_sheet_structure_updates(
    conn: &Connection,
    headers: &[String],
    data_row_count: usize,
) -> AppResult<(Vec<DropdownSpec>, Option<FormulaSpec>, Option<FormulaSpec>)> {
    let map = build_header_map(headers);

    let mut dropdowns: Vec<DropdownSpec> = vec![];

    if let Some(c) = find_col(&map, &["ticket type"]) {
        let values = crate::commands::tickets::known_ticket_type_names(conn)?;
        if !values.is_empty() {
            dropdowns.push(DropdownSpec { col_index: c, values });
        }
    }
    if let Some(c) = find_col(&map, &["site listed", "site", "listing site"]) {
        let values = sale_platform_names(conn)?;
        if !values.is_empty() {
            dropdowns.push(DropdownSpec { col_index: c, values });
        }
    }
    if let Some(c) = find_col(&map, &["status"]) {
        dropdowns.push(DropdownSpec { col_index: c, values: STATUS_OPTIONS.iter().map(|s| s.to_string()).collect() });
    }
    if let Some(c) = find_col(&map, &["delivery status", "delivery"]) {
        dropdowns.push(DropdownSpec { col_index: c, values: DELIVERY_STATUS_OPTIONS.iter().map(|s| s.to_string()).collect() });
    }
    if let Some(c) = find_col(&map, &["payout status"]) {
        dropdowns.push(DropdownSpec { col_index: c, values: PAYOUT_STATUS_OPTIONS.iter().map(|s| s.to_string()).collect() });
    }
    if let Some(c) = find_col(&map, &["pull"]) {
        dropdowns.push(DropdownSpec { col_index: c, values: PULL_OPTIONS.iter().map(|s| s.to_string()).collect() });
    }

    let payout_col = find_col(&map, &["payout per ticket", "payout"]);
    let qty_col = find_col(&map, &["number of tickets", "quantity", "qty"]);
    let total_col = find_col(&map, &["total purchase price"]);
    let revenue_col = find_col(&map, &["revenue"]);
    let profit_col = find_col(&map, &["profit"]);
    let status_col = find_col(&map, &["status"]);

    let revenue = match (payout_col, qty_col, revenue_col) {
        (Some(payout_col), Some(qty_col), Some(revenue_col)) => {
            let payout_letter = column_index_to_a1(payout_col);
            let qty_letter = column_index_to_a1(qty_col);
            let formulas = (0..data_row_count)
                .map(|i| {
                    let row = i + 2;
                    format!("={payout_letter}{row}*{qty_letter}{row}")
                })
                .collect();
            Some(FormulaSpec { col_index: revenue_col, formulas })
        }
        _ => None,
    };

    let profit = match (&revenue, total_col, profit_col) {
        (Some(rev), Some(total_col), Some(profit_col)) => {
            let revenue_letter = column_index_to_a1(rev.col_index);
            let total_letter = column_index_to_a1(total_col);
            let status_letter = status_col.map(column_index_to_a1);
            let formulas = (0..data_row_count)
                .map(|i| {
                    let row = i + 2;
                    match &status_letter {
                        Some(status_letter) => {
                            format!("=({status_letter}{row}=\"Sold\")*({revenue_letter}{row}-{total_letter}{row})")
                        }
                        None => format!("={revenue_letter}{row}-{total_letter}{row}"),
                    }
                })
                .collect();
            Some(FormulaSpec { col_index: profit_col, formulas })
        }
        _ => None,
    };

    Ok((dropdowns, revenue, profit))
}

struct ColorSpec {
    col_index: usize,
    /// Exact cell text -> background color, e.g. `("Listed", COLOR_ORANGE)`.
    /// Every color-coded column in this app is a FIXED-option one (Status/
    /// Delivery status/Payout status here, Transfer in
    /// pulls_sheet_sync::plan_pulls_sheet_color_updates) - unlike
    /// `plan_sheet_structure_updates` above, this never needs `conn`, since
    /// there is no growable/DB-backed column among the ones marko asked to
    /// color (Ticket Type/Site Listed/pull are deliberately NOT in this
    /// list - marko listed exactly Status/Delivery status/Payout status for
    /// Orders & Sales, nothing else).
    colors: Vec<(String, (f64, f64, f64))>,
}

/// Pure core, sibling of `plan_sheet_structure_updates` above rather than
/// folded into it - color-coding is a separate kind of structure decision
/// (conditional formatting, not data validation) that happens to target some
/// of the same columns. marko's own request (2.0.22): "pri status listed
/// oranzova farba, pri unlisted hneda, pri sold zelena... pri delivery
/// status delivered zelena not delivered oranzova, a payout status pending
/// oranzova paid zelena." Same tolerance as everywhere else in this module:
/// a column not present in `headers` is simply skipped, never an error.
fn plan_sheet_color_updates(headers: &[String]) -> Vec<ColorSpec> {
    let map = build_header_map(headers);
    let mut specs: Vec<ColorSpec> = vec![];

    if let Some(c) = find_col(&map, &["status"]) {
        specs.push(ColorSpec {
            col_index: c,
            colors: vec![
                ("Listed".to_string(), COLOR_ORANGE),
                ("Unlisted".to_string(), COLOR_BROWN),
                ("Sold".to_string(), COLOR_GREEN),
            ],
        });
    }
    if let Some(c) = find_col(&map, &["delivery status", "delivery"]) {
        specs.push(ColorSpec {
            col_index: c,
            colors: vec![("Delivered".to_string(), COLOR_GREEN), ("Not delivered".to_string(), COLOR_ORANGE)],
        });
    }
    if let Some(c) = find_col(&map, &["payout status"]) {
        specs.push(ColorSpec {
            col_index: c,
            colors: vec![("Pending".to_string(), COLOR_ORANGE), ("Paid".to_string(), COLOR_GREEN)],
        });
    }

    specs
}

/// 2.0.40: background for the Summary/Summary-Paid/Summary-Unpaid header
/// cells only - same flat, unconditional header style (and same shade) as
/// pulls_sheet_sync::TOTAL_PRICE_HEADER_BACKGROUND, kept as this module's own
/// copy rather than a shared constant, same file-local duplication
/// convention this module already follows for column_index_to_a1/now_iso/
/// DROPDOWN_ROW_BUFFER. Deliberately distinct from COLOR_GREEN/COLOR_ORANGE/
/// COLOR_BROWN above - those are conditional-format cell colors keyed on a
/// row's own data value, this is an unconditional label style.
const SUMMARY_HEADER_BACKGROUND: (f64, f64, f64) = (0.85, 0.88, 0.95);

/// One column's worth of a `plan_orders_summary_updates` write - either RAW
/// text (a label) or USER_ENTERED (a formula), never mixed within one
/// column. `values[0]` is always row 1.
struct SheetColumnWrite {
    col_index: usize,
    values: Vec<String>,
}

/// The full plan `plan_orders_summary_updates` below produces - two lists of
/// column writes, kept separate because RAW and USER_ENTERED are two
/// different Sheets API calls (`google_sheets::update_values` vs.
/// `update_values_as_formulas`) with two different `valueInputOption`s, and
/// this module never mixes the two within one call - see google_sheets.rs's
/// own doc comment on why that split exists at all.
struct OrdersSummarySpec {
    text_columns: Vec<SheetColumnWrite>,
    formula_columns: Vec<SheetColumnWrite>,
}

/// 2.0.40: marko's own request - a small automatically-calculated summary
/// table (Total Cost/Revenue/Profit, plus a Paid/Unpaid revenue split),
/// placed to the right of the sheet's own data columns rather than mixed
/// into them: "nedavaj to hned za how much pull ale nechaj 2 volne stlpce a
/// do 3. zacni" (don't put it right after `how much pull`, leave 2 free
/// columns, start at the 3rd) - so `start_col` is always `how_much_pull_col
/// + 3`, recomputed fresh from the sheet's real current headers every time,
/// never a hardcoded letter (same convention as every other column lookup
/// in this module - marko's own draft used fixed letters H/P/Q/T, which
/// happen to match `ORDERS_SHEET_HEADERS`'s canonical order exactly, but a
/// dynamic lookup is what actually keeps this correct if his real sheet ever
/// differs from that canonical order).
///
/// Layout (6 columns wide, `start_col` = leftmost):
/// | col+0 | col+1 | col+2 | col+3 | col+4 | col+5 |
/// |---|---|---|---|---|---|
/// | Summary | (total cost) | Summary-Paid | (total paid) | Summary-Unpaid | (total unpaid) |
/// | Total Cost | =SUMPRODUCT((total purchase price)*1) | Total Paid | =SUMPRODUCT((status="Paid")*revenue) | Total Unpaid | =SUM(revenue)-SUMPRODUCT((status="Paid")*revenue) |
/// | Total Revenue | =SUMPRODUCT((status="Paid")*revenue) | | | | |
/// | Total Profit | =SUMPRODUCT((status="Paid")*profit) | | | | |
///
/// All 5 source columns (`how much pull` for placement, `Total Purchase
/// Price`/`Revenue`/`Profit`/`Payout status` for the actual math) must be
/// present or the WHOLE block is skipped (`None`) - deliberately
/// all-or-nothing, unlike the finer-grained per-formula tolerance
/// `plan_sheet_structure_updates` above uses for Revenue/Profit. Those are
/// two independent standalone columns elsewhere in the sheet; this is one
/// small coherent visual table, and a partially-rendered version of it (a
/// "Total Profit" label with no formula next to it, or a table missing one
/// of its own rows) would look broken rather than simply absent. marko's
/// own real sheet has all 5 (his own draft formulas used exactly these), so
/// this is not expected to bite in practice.
///
/// **The "Unpaid" formula is deliberately NOT `SUMIF(status, "Unpaid",
/// revenue)`, unlike marko's own first draft.** `Payout status` only ever
/// actually contains `pending` (including blank) or `paid` - see the parser
/// above (`let payment_status = match ... "pending"/"paid"/"refunded" ...`)
/// and `PAYOUT_STATUS_OPTIONS`'s own dropdown ("Pending"/"Paid"). Literal
/// text "Unpaid" never appears in that column, so a literal `SUMIF` match
/// against it would always compute zero - not what marko is actually after.
/// "Total Unpaid" is instead `total revenue - whatever is marked Paid`,
/// which correctly means "everything not yet paid" regardless of whether a
/// not-yet-paid row is blank or literally "Pending".
///
/// **2.0.42: "Total Paid"/"Total Unpaid" use `SUMPRODUCT`, not `SUMIF`,
/// deliberately.** The original `SUMIF(status:status,"Paid",revenue:revenue)`
/// produced `#ERROR!` on marko's real sheet - Google Sheets' function
/// argument separator is tied to the spreadsheet's own locale: comma-decimal
/// locales (Slovak among them - his own screenshots show "3488,06") require
/// `;` between a function's arguments instead of `,`, since `,` is already
/// the decimal point there. This app has no reliable way to know a
/// connected spreadsheet's locale (only its 3-letter currency CODE, which is
/// a completely different setting - see `bold_header_request`'s own doc
/// comment for the currency-formatting side of this same lesson), so rather
/// than guess at `,` vs `;`, this is written as `SUMPRODUCT((status_range=
/// "Paid")*revenue_range)` - ONE array-expression argument, built with `*`
/// and `=` (plain operators, never locale-sensitive), so there is no
/// function-argument separator anywhere in it to get wrong, for any locale.
/// Ranges are bounded to `SUMPRODUCT_ROW_BOUND` rows rather than a true
/// whole-column reference like the other 3 (SUM-based, still whole-column
/// and untouched by this fix) formulas above - see that constant's own
/// comment for why. Deliberately starts at row 2 (never row 1): unlike
/// `SUM`/`SUMIF`, which silently ignore non-numeric cells, `SUMPRODUCT`
/// multiplies its arrays elementwise BEFORE summing, and the header row's
/// own text (e.g. "Revenue") in a numeric range makes the whole calculation
/// error out - even on the elements where the paired condition is FALSE,
/// since both arrays are evaluated in full rather than short-circuited.
///
/// **2.0.80: "Total Revenue"/"Total Profit" also gate on `Payout status =
/// "Paid"` now, not just "Total Paid" - marko's own bug report**: *"summary
/// paid a unpaid nefunguju dobre v tabulke, az ked je payment status paid az
/// vtedy sa to moze zapocitat do tej tabulky, inak nie"* (the paid/unpaid
/// summary don't work well - only once payment_status is paid can something
/// be counted into that table, otherwise not), confirmed literally via
/// AskUserQuestion rather than assumed. Before this version "Total Revenue"/
/// "Total Profit" summed every SOLD row regardless of payment status (Paid
/// and still-Pending rows counted alike) - only "Total Paid" applied the
/// Payout-status filter. Now all three (`revenue_formula`/`profit_formula`/
/// `paid_formula`) share the exact same `SUMPRODUCT((status_range="Paid")*
/// ...)` shape, which deliberately makes "Total Revenue" numerically
/// IDENTICAL to "Total Paid" from now on (both are the same expression
/// against the same Revenue column) - not a bug if a future reader notices
/// the two cells always match; marko confirmed this exact scope rather than,
/// say, only the separate refund-staleness fix below. "Total Unpaid" is
/// deliberately UNCHANGED: it still needs the *ungated* `SUM(revenue)` as
/// its own base to mean "everything sold but not yet paid" - gating it the
/// same way would make it always read zero. See `order_fully_refunded`'s own
/// doc comment (`apply_sales_push_internal`, earlier in this file) for the
/// other half of the same report - a refunded sale's stale pre-refund data
/// never being cleared from the sheet, which this Paid-only gate alone does
/// not fix for a row whose Payout status cell still (wrongly) says "Paid".
fn plan_orders_summary_updates(headers: &[String]) -> Option<OrdersSummarySpec> {
    let map = build_header_map(headers);
    let how_much_pull_col = find_col(&map, &["how much pull"])?;
    let total_col = find_col(&map, &["total purchase price"])?;
    let revenue_col = find_col(&map, &["revenue"])?;
    let profit_col = find_col(&map, &["profit"])?;
    let payout_status_col = find_col(&map, &["payout status"])?;

    let total_letter = column_index_to_a1(total_col);
    let revenue_letter = column_index_to_a1(revenue_col);
    let profit_letter = column_index_to_a1(profit_col);
    let payout_status_letter = column_index_to_a1(payout_status_col);

    let start_col = how_much_pull_col + 3;
    let bound = SUMPRODUCT_ROW_BOUND;

    // 2.0.62 correction: `Total Purchase Price` is written by this app via
    // `update_values` (`valueInputOption=RAW`, deliberately - see that
    // function's own doc comment), same as every other plain value this
    // module writes - so Sheets stores it as literal TEXT, never as a
    // genuine number, no matter how numeric it looks. `SUM()` silently skips
    // text cells rather than coercing them, so `=SUM({total_letter}:
    // {total_letter})` was *always* going to show 0,00 the moment marko
    // actually looked at it - not something Fix sync (or anything else
    // recent) broke. `Revenue`/`Profit` don't have this problem because
    // they're the one exception (`update_values_as_formulas`, USER_ENTERED)
    // - a formula's result is a genuine number, so plain `SUM()` would have
    // coerced correctly for them even before 2.0.80. Their own move to
    // `SUMPRODUCT` below is for a completely unrelated reason - the Paid-only
    // gate, see this function's own 2.0.80 doc comment - not this text-vs-
    // number issue, which only ever applied to `cost_formula`.
    let cost_formula = format!("=SUMPRODUCT(({total_letter}2:{total_letter}{bound})*1)");
    let paid_sumproduct = format!(
        "SUMPRODUCT(({payout_status_letter}2:{payout_status_letter}{bound}=\"Paid\")*{revenue_letter}2:{revenue_letter}{bound})"
    );
    // 2.0.80: same SUMPRODUCT shape as `paid_sumproduct` above, against the
    // Profit column instead of Revenue - see this function's own 2.0.80 doc
    // comment for why "Total Profit" now needs this gate too.
    let profit_sumproduct = format!(
        "SUMPRODUCT(({payout_status_letter}2:{payout_status_letter}{bound}=\"Paid\")*{profit_letter}2:{profit_letter}{bound})"
    );
    let revenue_formula = format!("={paid_sumproduct}");
    let profit_formula = format!("={profit_sumproduct}");
    let paid_formula = format!("={paid_sumproduct}");
    let unpaid_formula = format!("=SUM({revenue_letter}:{revenue_letter})-{paid_sumproduct}");

    Some(OrdersSummarySpec {
        text_columns: vec![
            SheetColumnWrite {
                col_index: start_col,
                values: vec!["Summary".to_string(), "Total Cost".to_string(), "Total Revenue".to_string(), "Total Profit".to_string()],
            },
            SheetColumnWrite { col_index: start_col + 2, values: vec!["Summary-Paid".to_string(), "Total Paid".to_string()] },
            SheetColumnWrite { col_index: start_col + 4, values: vec!["Summary-Unpaid".to_string(), "Total Unpaid".to_string()] },
        ],
        formula_columns: vec![
            SheetColumnWrite { col_index: start_col + 1, values: vec![String::new(), cost_formula, revenue_formula, profit_formula] },
            SheetColumnWrite { col_index: start_col + 3, values: vec![String::new(), paid_formula] },
            SheetColumnWrite { col_index: start_col + 5, values: vec![String::new(), unpaid_formula] },
        ],
    })
}

/// The network shell for `plan_sheet_structure_updates`/
/// `plan_orders_summary_updates` above - sends their plan as real
/// `batchUpdate` (dropdowns), `update_values_as_formulas` (Revenue/Profit,
/// summary formulas) and `update_values` (summary labels) calls. Called at
/// the end of `sync_orders_impl`/`sync_sales_impl`/`push_orders_impl`/
/// `push_sales_impl` - see this section's own doc comment for why all four
/// call it.
///
/// Deliberately never fails the calling command outright - a problem here
/// (e.g. the tab's numeric ID couldn't be resolved, or Sheets rejected the
/// batchUpdate) is reported as a soft warning on the result the caller
/// already has, never as a reason to discard whatever real sync/push work
/// that command just did. See each call site.
/// The single widest column any of `dropdowns`/`colors`/`summary` is about
/// to reference - `None` when none of the three have anything to write at
/// all. Pure (no network, no `conn`) specifically so it can be tested
/// directly, unlike `ensure_orders_sheet_structure` itself below - see that
/// function's own comment on `grow_grid_request_if_needed` for why this
/// number matters. `summary`'s FORMULA columns are included here alongside
/// its TEXT columns - originally (2.0.41) only the text columns got a real
/// `repeatCell` request (`bold_header_request`) here, with formula columns
/// written separately via `values.*` (which grows the grid on its own), so
/// including them was a "make this correct regardless" precaution rather
/// than a known-necessary one. 2.0.42 removed that ambiguity: formula
/// columns now also get their own `repeatCell` request here (`currency_
/// number_format_request`), so this is load-bearing, not just defensive.
fn widest_referenced_column(dropdowns: &[DropdownSpec], colors: &[ColorSpec], summary: &Option<OrdersSummarySpec>) -> Option<usize> {
    [
        dropdowns.iter().map(|d| d.col_index).max(),
        colors.iter().map(|c| c.col_index).max(),
        summary.as_ref().and_then(|s| s.text_columns.iter().chain(s.formula_columns.iter()).map(|c| c.col_index).max()),
    ]
    .into_iter()
    .flatten()
    .max()
}

fn ensure_orders_sheet_structure(
    conn: &Connection,
    token: &str,
    spreadsheet_id: &str,
    sheet_tab: &str,
    headers: &[String],
    data_rows: &[Vec<String>],
) -> AppResult<()> {
    let (dropdowns, revenue, profit) = plan_sheet_structure_updates(conn, headers, data_rows.len())?;
    let colors = plan_sheet_color_updates(headers);
    let summary = plan_orders_summary_updates(headers);

    if !dropdowns.is_empty() || !colors.is_empty() || summary.is_some() {
        // One shared metadata fetch for all three - `get_sheet_structure_metadata`
        // folds in the same numeric sheetId `get_sheet_numeric_id` alone
        // would return, so there is never a reason to call both.
        let metadata = google_sheets::get_sheet_structure_metadata(token, spreadsheet_id, sheet_tab)?;
        let sheet_id = metadata.sheet_id;
        let end_row = (data_rows.len() as i64).max(DROPDOWN_ROW_BUFFER) + 1;
        let mut requests: Vec<serde_json::Value> = vec![];

        // 2.0.41: grow the sheet's own grid FIRST, in this same batchUpdate
        // call, if anything below is about to reference a row/column past
        // its CURRENT size - see grow_grid_request_if_needed's own doc
        // comment for the real incident this fixes (the Summary block's
        // header cell at column AB against a real sheet's default 26-column
        // grid took down the ENTIRE refresh, dropdowns/colors included, not
        // just the new Summary styling - batch_update is all-or-nothing).
        if let Some(widest) = widest_referenced_column(&dropdowns, &colors, &summary) {
            if let Some(grow) =
                google_sheets::grow_grid_request_if_needed(sheet_id, metadata.row_count, metadata.column_count, end_row, widest as i64 + 1)
            {
                requests.push(grow);
            }
        }

        if !colors.is_empty() {
            // Delete THIS refresh's own previously-added color rules before
            // re-adding - never anything on a column colors doesn't cover
            // (see conditional_format_indices_to_replace's own doc comment).
            // Ordered before every add below in the same batchUpdate call,
            // so no add can ever land at an index a still-pending delete is
            // about to shift.
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

        requests.extend(
            dropdowns.iter().map(|d| google_sheets::set_data_validation_request(sheet_id, 1, end_row, d.col_index as i64, &d.values)),
        );

        if let Some(spec) = &summary {
            // Bold+background on row 1 of each of the block's 3 label
            // columns only (Summary/Summary-Paid/Summary-Unpaid) - the
            // formula columns next to them get their own, different
            // treatment right below (EUR currency formatting on the actual
            // number cells, never bold/background - same "style the label,
            // the number looks like money" split pulls_sheet_sync now makes
            // for Total price (€), see that module's own call site).
            for col in &spec.text_columns {
                requests.push(google_sheets::bold_header_request(sheet_id, 0, 1, col.col_index as i64, Some(SUMMARY_HEADER_BACKGROUND)));
            }
            // 2.0.42: row 0 of every formula column is always the blank
            // placeholder next to the label row (see plan_orders_summary_
            // updates's own layout table) - `col.values.len()` is exactly
            // how many rows this column actually writes, so starting at 1
            // covers every real number cell and nothing past it.
            for col in &spec.formula_columns {
                requests.push(google_sheets::currency_number_format_request(sheet_id, 1, col.values.len() as i64, col.col_index as i64));
            }
        }

        if !requests.is_empty() {
            google_sheets::batch_update(token, spreadsheet_id, requests)?;
        }
    }

    for spec in [revenue, profit].into_iter().flatten() {
        if spec.formulas.is_empty() {
            continue;
        }
        let letter = column_index_to_a1(spec.col_index);
        let range = google_sheets::a1_range(sheet_tab, &format!("{letter}2:{letter}{}", 1 + spec.formulas.len()));
        let values: Vec<Vec<String>> = spec.formulas.iter().map(|f| vec![f.clone()]).collect();
        google_sheets::update_values_as_formulas(token, spreadsheet_id, &range, &values)?;
    }

    if let Some(summary) = summary {
        for col in &summary.text_columns {
            let letter = column_index_to_a1(col.col_index);
            let range = google_sheets::a1_range(sheet_tab, &format!("{letter}1:{letter}{}", col.values.len()));
            let values: Vec<Vec<String>> = col.values.iter().map(|v| vec![v.clone()]).collect();
            google_sheets::update_values(token, spreadsheet_id, &range, &values)?;
        }
        for col in &summary.formula_columns {
            let letter = column_index_to_a1(col.col_index);
            let range = google_sheets::a1_range(sheet_tab, &format!("{letter}1:{letter}{}", col.values.len()));
            let values: Vec<Vec<String>> = col.values.iter().map(|v| vec![v.clone()]).collect();
            google_sheets::update_values_as_formulas(token, spreadsheet_id, &range, &values)?;
        }
    }

    Ok(())
}

/// Runs `ensure_orders_sheet_structure` and folds any error it returns into
/// `result` as a soft warning instead of propagating it - shared by all four
/// call sites below so a structure-refresh problem is reported the exact
/// same way everywhere. Never called when the caller has nothing to work
/// with (e.g. a header-row-only sheet is still handled - `data_rows` is
/// simply empty and formulas end up empty too, not skipped here).
fn refresh_sheet_structure_soft_fail(
    conn: &Connection,
    token: &str,
    spreadsheet_id: &str,
    sheet_tab: &str,
    headers: &[String],
    data_rows: &[Vec<String>],
    result: &mut SheetSyncResult,
) {
    if let Err(e) = ensure_orders_sheet_structure(conn, token, spreadsheet_id, sheet_tab, headers, data_rows) {
        result.errors.push(SheetSyncIssue {
            row_number: 0,
            message: format!("the sheet's dropdowns/Revenue/Profit formulas could not be refreshed this time: {e}"),
        });
    }
}

// ---------------------------------------------------------------------------
// "Update sheet" (2.0.20) - the Orders & Sales sibling of
// pulls_sheet_sync::setup_pulls_sheet, see that function's own doc comment
// for marko's exact request. Unlike Pulls, a freshly-written header here is
// not the whole story - Orders & Sales also has real dropdowns/formulas
// (`ensure_orders_sheet_structure`, 2.0.19) that a brand-new header needs
// applied to it right away rather than waiting for the next Order sync/Sales
// sync/Push orders/Push sales, so this command always runs that step too,
// whether or not the header itself needed writing.
// ---------------------------------------------------------------------------

/// Writes `ORDERS_SHEET_HEADERS` as row 1 of the already-connected sheet/tab
/// when it currently has no header row at all (never touches an existing
/// header), then always runs `ensure_orders_sheet_structure` on whatever
/// header is now in place - so this doubles as the on-demand way to
/// (re-)apply dropdowns/formulas immediately, without needing to click one of
/// the four sync/push buttons first. See this section's own doc comment.
fn setup_orders_sheet_impl(conn: &Connection) -> AppResult<SheetSyncResult> {
    let connection = load_connection(conn, "orders")?.ok_or_else(|| {
        AppError::Validation("No spreadsheet is connected for Orders & Sales yet - connect one in Settings first.".to_string())
    })?;
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
        corrected: vec![],
        synced_at: String::new(),
    };

    let headers: Vec<String> = if value_range.values.is_empty() {
        let header_row: Vec<String> = ORDERS_SHEET_HEADERS.iter().map(|s| s.to_string()).collect();
        let header_range = google_sheets::a1_range(&connection.sheet_tab, "A1");
        google_sheets::update_values(token, &connection.spreadsheet_id, &header_range, &[header_row.clone()])?;
        result.created = 1;
        header_row
    } else {
        result.unchanged = 1;
        value_range.values[0].clone()
    };
    let data_rows: &[Vec<String>] = if value_range.values.len() > 1 { &value_range.values[1..] } else { &[] };

    refresh_sheet_structure_soft_fail(
        conn,
        token,
        &connection.spreadsheet_id,
        &connection.sheet_tab,
        &headers,
        data_rows,
        &mut result,
    );

    result.synced_at = now_iso(conn)?;
    Ok(result)
}

/// "Update sheet" button (Settings -> Integrations, Orders & Sales card) -
/// sits next to "Order sync"/"Sales sync"/"Push orders"/"Push sales", for the
/// already-connected sheet rather than the separate "Create a new sheet for
/// me" flow. Never runs on its own.
#[tauri::command]
pub fn setup_orders_sheet(state: State<AppState>) -> AppResult<SheetSyncResult> {
    let conn = state.db.lock().unwrap();
    setup_orders_sheet_impl(&conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::sales::refund_sale_impl;
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

    // -- resolve_or_create_event's 2.0.63 category-detection hook ----------
    //
    // `test_conn()` runs the real migrations, including 012_event_
    // categories.sql's 6-category seed, and this test environment never has
    // ANTHROPIC_API_KEY set (see ai_categorize::embedded_anthropic_api_key's
    // own test), so only the free keyword rules can actually fire here -
    // exactly like a real build marko hasn't added that optional secret to
    // yet. See ai_categorize.rs's own test module for the rule/AI decision
    // logic itself; these tests are only about resolve_or_create_event
    // actually wiring that result into the INSERT correctly.

    #[test]
    fn a_brand_new_event_gets_auto_categorized_when_its_name_has_a_free_rule_signal() {
        let conn = test_conn();
        let id = resolve_or_create_event(&conn, "Spa-Francorchamps Grand Prix", "2026-09-01").unwrap();
        let (category, category_id): (Option<String>, Option<i64>) = conn
            .query_row("SELECT category, category_id FROM events WHERE id = ?1", [id], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(category.as_deref(), Some("Motorsport"));
        assert!(category_id.is_some(), "category_id must be set alongside the category text mirror");
    }

    #[test]
    fn a_brand_new_event_with_no_recognizable_signal_is_created_with_no_category() {
        // Same as every event before 2.0.63 - no free rule fires on a bare
        // name, and no AI key is embedded in this test build, so this must
        // NOT error and must NOT guess; the event itself still gets created.
        let conn = test_conn();
        let id = resolve_or_create_event(&conn, "Celine Dion", "2026-09-01").unwrap();
        let (category, category_id): (Option<String>, Option<i64>) = conn
            .query_row("SELECT category, category_id FROM events WHERE id = ?1", [id], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(category, None);
        assert_eq!(category_id, None);
    }

    #[test]
    fn an_existing_events_category_is_never_touched_by_a_later_resolve() {
        // marko may have deliberately cleared a category by hand, or set a
        // different one than a rule would now suggest - either way, an
        // event that already exists by name must be reused completely as-
        // is, exactly like its date already was before 2.0.63.
        let conn = test_conn();
        let first_id = resolve_or_create_event(&conn, "Monaco Grand Prix", "2026-05-01").unwrap();
        conn.execute("UPDATE events SET category_id = NULL, category = NULL WHERE id = ?1", [first_id])
            .unwrap();
        let second_id = resolve_or_create_event(&conn, "Monaco Grand Prix", "2026-05-01").unwrap();
        assert_eq!(first_id, second_id);
        let category: Option<String> =
            conn.query_row("SELECT category FROM events WHERE id = ?1", [first_id], |r| r.get(0)).unwrap();
        assert_eq!(category, None, "a re-resolved existing event must not be re-categorized");
    }

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
        assert_eq!(writes.markers.len(), 1);
        assert_eq!(writes.markers[0].0, 0);
        assert!(writes.markers[0].1.starts_with("ORD-"));
        assert!(writes.price_corrections.is_empty(), "a clean row must never trigger a price correction");

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
        // 2.0.12: the raw cell value only, no "Email used: " label prefix -
        // marko's own report.
        assert_eq!(notes.as_deref(), Some("buyer@example.com"));

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
    fn an_order_created_via_sync_is_stamped_payment_status_paid() {
        let conn = test_conn();
        apply_order_rows(&conn, &full_headers(), &[sample_row("")], "EUR", MARKER_COL).unwrap();
        let payment_status: String =
            conn.query_row("SELECT payment_status FROM orders WHERE id = 1", [], |r| r.get(0)).unwrap();
        // 2.0.43: marko's own explicit instruction - an order already
        // sitting in his connected sheet is, to him, already a real
        // confirmed purchase, so it must never land in the Dashboard's
        // unpaid-orders count just because this sync has no payment-status
        // column of its own to read. Previously defaulted to "unpaid" (see
        // insert_order_with_tickets's own fallback for a None
        // OrderInput.payment_status).
        assert_eq!(payment_status, "paid");
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

    // ---- 2.0.42: automatic reconciliation of automated-order pricing ------
    // marko's own real request (a sync result screenshot, 4 skipped rows):
    // small Total Purchase Price gaps and over-precise Price Per Ticket
    // values must be auto-corrected, never treated as an error he has to
    // fix by hand - but a gap too big to be honest rounding must still be
    // rejected ("nemoze tam napisat hlupost, musi to davat zmysel"). See
    // rounding_tolerance_cents/reconcile_order_pricing's own doc comments.

    #[test]
    fn rounding_tolerance_cents_is_half_the_quantity_rounded_up() {
        assert_eq!(rounding_tolerance_cents(1), 1);
        assert_eq!(rounding_tolerance_cents(2), 1);
        assert_eq!(rounding_tolerance_cents(3), 2);
        assert_eq!(rounding_tolerance_cents(4), 2);
        assert_eq!(rounding_tolerance_cents(5), 3);
        assert_eq!(rounding_tolerance_cents(10), 5);
    }

    #[test]
    fn derive_unit_price_from_total_matches_markos_own_real_example() {
        // 386.73 / 4 tickets - the exact numbers from marko's own screenshot
        // (a Price Per Ticket cell rejected for 4 decimal places, 96.6825).
        assert_eq!(derive_unit_price_from_total(38673, 4), 9668);
    }

    #[test]
    fn derive_unit_price_from_total_rounds_half_up_and_handles_exact_division() {
        assert_eq!(derive_unit_price_from_total(12500, 2), 6250, "exact division needs no rounding");
        assert_eq!(derive_unit_price_from_total(101, 2), 51, "50.5 cents exactly - a real tie, rounds up per round-half-up");
        assert_eq!(derive_unit_price_from_total(10000, 3), 3333, "33.333... rounds down");
        assert_eq!(derive_unit_price_from_total(20000, 3), 6667, "66.667 rounds up");
    }

    #[test]
    fn reconcile_order_pricing_leaves_an_exact_match_untouched() {
        let outcome = reconcile_order_pricing("50.00", Some("100.00"), 2).unwrap();
        assert_eq!(outcome.unit_price_cents, 5000);
        assert!(outcome.corrected_unit_price_text.is_none());
        assert!(outcome.corrected_total_price_text.is_none());
        assert!(outcome.note.is_none());
    }

    #[test]
    fn reconcile_order_pricing_is_fine_with_no_total_purchase_price_at_all() {
        let outcome = reconcile_order_pricing("50.00", None, 2).unwrap();
        assert_eq!(outcome.unit_price_cents, 5000);
        assert!(outcome.note.is_none());
    }

    #[test]
    fn reconcile_order_pricing_corrects_a_total_purchase_price_gap_exactly_at_the_tolerance_boundary() {
        // quantity 2 -> tolerance 1 cent - a 1-cent gap must be corrected.
        let outcome = reconcile_order_pricing("50.00", Some("100.01"), 2).unwrap();
        assert_eq!(outcome.unit_price_cents, 5000, "Price Per Ticket itself is untouched - it was the clean value");
        assert!(outcome.corrected_unit_price_text.is_none());
        assert_eq!(outcome.corrected_total_price_text, Some("100,00".to_string()));
        assert!(outcome.note.unwrap().contains("Total Purchase Price"));
    }

    #[test]
    fn reconcile_order_pricing_rejects_a_total_purchase_price_gap_one_cent_past_the_tolerance_boundary() {
        // Same setup as the boundary-accepted test above, but 2 cents off
        // instead of 1 - tolerance for quantity 2 is exactly 1 cent.
        let err = reconcile_order_pricing("50.00", Some("100.02"), 2).unwrap_err();
        assert!(err.contains("Total Purchase Price"), "{err}");
        assert!(err.contains("does not match"), "{err}");
    }

    #[test]
    fn reconcile_order_pricing_corrects_markos_own_real_small_gap_example() {
        // marko's own screenshot: Total Purchase Price 337.10 vs a computed
        // 337.12 - a 2-cent gap, exactly this app's own tolerance for a
        // 4-ticket order ((4+1)/2 = 2).
        let outcome = reconcile_order_pricing("84.28", Some("337.10"), 4).unwrap();
        assert_eq!(outcome.unit_price_cents, 8428);
        assert_eq!(outcome.corrected_total_price_text, Some("337,12".to_string()));
    }

    #[test]
    fn reconcile_order_pricing_still_rejects_markos_own_real_large_gap_example() {
        // marko's own screenshot: Total Purchase Price 401.99 vs a computed
        // 399.00 - a ~299-cent gap, nowhere close to being explainable by
        // rounding. Must still hard-error exactly like every version before
        // 2.0.42 - this is the "musi davat zmysel" guarantee as an actual
        // test, not just a design note.
        let err = reconcile_order_pricing("399.00", Some("401.99"), 1).unwrap_err();
        assert!(err.contains("Total Purchase Price"), "{err}");
    }

    #[test]
    fn reconcile_order_pricing_derives_an_over_precise_price_per_ticket_from_total_purchase_price() {
        // marko's own screenshot: Price Per Ticket "96.6825" (4 decimals,
        // his automation's Total/Quantity division not landing on a cent).
        let outcome = reconcile_order_pricing("96.6825", Some("386.73"), 4).unwrap();
        assert_eq!(outcome.unit_price_cents, 9668);
        assert_eq!(outcome.corrected_unit_price_text, Some("96,68".to_string()));
        assert!(outcome.corrected_total_price_text.is_none(), "Total Purchase Price was already clean - must be left alone");
        let note = outcome.note.unwrap();
        assert!(note.contains("Price Per Ticket"), "{note}");
        assert!(note.contains("more than 2 decimal"), "{note}");
    }

    #[test]
    fn reconcile_order_pricing_rounds_an_over_precise_price_per_ticket_with_no_total_purchase_price_to_derive_from() {
        let outcome = reconcile_order_pricing("96.6825", None, 4).unwrap();
        assert_eq!(outcome.unit_price_cents, 9668);
        assert_eq!(outcome.corrected_unit_price_text, Some("96,68".to_string()));
        assert!(outcome.note.unwrap().contains("rounded to"));
    }

    #[test]
    fn reconcile_order_pricing_still_rejects_a_negative_price_per_ticket() {
        assert_eq!(reconcile_order_pricing("-5.00", None, 1).unwrap_err(), "'Price Per Ticket' cannot be negative");
        assert_eq!(reconcile_order_pricing("-5.00", Some("10.00"), 2).unwrap_err(), "'Price Per Ticket' cannot be negative");
    }

    #[test]
    fn reconcile_order_pricing_still_rejects_genuinely_invalid_price_per_ticket_text() {
        let err = reconcile_order_pricing("abc", None, 1).unwrap_err();
        assert!(err.contains("Price Per Ticket"), "{err}");
    }

    #[test]
    fn reconcile_order_pricing_still_rejects_a_malformed_total_purchase_price() {
        let err = reconcile_order_pricing("50.00", Some("abc"), 2).unwrap_err();
        assert!(err.contains("Total Purchase Price"), "{err}");
    }

    #[test]
    fn a_small_total_purchase_price_gap_creates_the_order_and_is_reported_as_corrected_not_an_error() {
        let conn = test_conn();
        let mut cells = sample_row("");
        cells[7] = "100.01".to_string(); // 1 cent off 2 x 50.00 - within tolerance for quantity 2
        let (result, writes) = apply_order_rows(&conn, &full_headers(), &[cells], "EUR", MARKER_COL).unwrap();
        assert_eq!(result.created, 1);
        assert_eq!(result.errors.len(), 0, "errors: {:?}", result.errors);
        assert_eq!(result.corrected.len(), 1);
        assert!(result.corrected[0].message.contains("Total Purchase Price"), "{}", result.corrected[0].message);
        assert_eq!(writes.price_corrections, vec![(0, 7, "100,00".to_string())], "column 7 is Total Purchase Price in full_headers()");
    }

    #[test]
    fn an_over_precise_price_per_ticket_creates_the_order_with_the_derived_price() {
        let conn = test_conn();
        let mut cells = sample_row("");
        cells[5] = "".to_string(); // Seats - blanked, sample_row's default 2 labels won't match 4 tickets
        cells[8] = "4".to_string(); // Number of Tickets
        cells[9] = "96.6825".to_string(); // Price Per Ticket
        cells[7] = "386.73".to_string(); // Total Purchase Price
        let (result, writes) = apply_order_rows(&conn, &full_headers(), &[cells], "EUR", MARKER_COL).unwrap();
        assert_eq!(result.created, 1);
        assert_eq!(result.errors.len(), 0, "errors: {:?}", result.errors);
        assert_eq!(result.corrected.len(), 1);
        assert_eq!(writes.price_corrections, vec![(0, 9, "96,68".to_string())], "column 9 is Price Per Ticket in full_headers()");
        let unit_price_cents: i64 = conn.query_row("SELECT unit_price_cents FROM orders WHERE id = 1", [], |r| r.get(0)).unwrap();
        assert_eq!(unit_price_cents, 9668, "the order itself must be saved with the corrected price, not the raw over-precise text");
    }

    #[test]
    fn a_too_large_total_purchase_price_gap_still_skips_the_row_and_corrects_nothing() {
        let conn = test_conn();
        let mut cells = sample_row("");
        cells[7] = "100.02".to_string(); // 2 cents off - one past tolerance for quantity 2
        let (result, writes) = apply_order_rows(&conn, &full_headers(), &[cells], "EUR", MARKER_COL).unwrap();
        assert_eq!(result.created, 0);
        assert_eq!(result.errors.len(), 1);
        assert!(result.corrected.is_empty(), "a skipped row must never also be reported as corrected");
        assert!(writes.price_corrections.is_empty(), "a skipped row must never have anything written back to the sheet");
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM orders", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 0);
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
        assert!(writes.markers.is_empty());
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
        assert!(writes.markers.is_empty());
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
        writes.markers[0].1.clone()
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

    // ---- pull / who pulled / how much pull -> linked pulls_received (2.0.17) --

    fn pulls_received_for_order(conn: &Connection, order_code: &str) -> Vec<(String, String, i64, i64, String)> {
        let mut stmt = conn
            .prepare(
                "SELECT pr.puller_name, pr.event_name, pr.quantity, pr.amount_cents, pr.source
                 FROM pulls_received pr JOIN orders o ON o.id = pr.order_id WHERE o.code = ?1",
            )
            .unwrap();
        stmt.query_map([order_code], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    }

    #[test]
    fn pull_yes_with_who_pulled_creates_one_linked_pulls_received_row() {
        let mut conn = test_conn();
        let code = seed_order_with_quantity(&conn, 1);
        let mut r = sales_row(&code, "45.00");
        r[8] = "yes".to_string();
        r[9] = "Jozef".to_string();
        r[10] = "15".to_string();
        let result = apply_sales_rows(&mut conn, &sales_headers(), &[r], 0).unwrap();
        assert_eq!(result.created, 1);

        let rows = pulls_received_for_order(&conn, &code);
        assert_eq!(rows.len(), 1);
        let (puller_name, event_name, quantity, amount_cents, source) = &rows[0];
        assert_eq!(puller_name, "Jozef");
        assert_eq!(event_name, "Coldplay Arena Show", "copied from the order's own linked event");
        assert_eq!(*quantity, 1);
        assert_eq!(*amount_cents, 1500);
        assert_eq!(source, "sheet_sync");
    }

    #[test]
    fn sale_notes_no_longer_contain_the_pull_columns() {
        let mut conn = test_conn();
        let code = seed_order_with_quantity(&conn, 1);
        let mut r = sales_row(&code, "45.00");
        r[8] = "yes".to_string();
        r[9] = "Jozef".to_string();
        r[10] = "15".to_string();
        apply_sales_rows(&mut conn, &sales_headers(), &[r], 0).unwrap();
        let ticket_id = ticket_ids_for_order(&conn, &code)[0];
        let notes: Option<String> = conn.query_row("SELECT notes FROM sales WHERE ticket_id = ?1", [ticket_id], |r| r.get(0)).unwrap();
        assert!(notes.is_none(), "2.0.17: pull data now lives in a real linked pulls_received row, not folded into notes - {notes:?}");
    }

    #[test]
    fn pull_not_yes_creates_no_linked_row() {
        let mut conn = test_conn();
        let code = seed_order_with_quantity(&conn, 1);
        let mut r = sales_row(&code, "45.00");
        r[8] = "no".to_string();
        r[9] = "Jozef".to_string();
        r[10] = "15".to_string();
        apply_sales_rows(&mut conn, &sales_headers(), &[r], 0).unwrap();
        assert!(pulls_received_for_order(&conn, &code).is_empty());
    }

    #[test]
    fn blank_pull_cell_creates_no_linked_row() {
        let mut conn = test_conn();
        let code = seed_order_with_quantity(&conn, 1);
        apply_sales_rows(&mut conn, &sales_headers(), &[sales_row(&code, "45.00")], 0).unwrap();
        assert!(pulls_received_for_order(&conn, &code).is_empty());
    }

    #[test]
    fn pull_yes_with_blank_who_pulled_creates_no_linked_row_but_the_sale_still_succeeds() {
        let mut conn = test_conn();
        let code = seed_order_with_quantity(&conn, 1);
        let mut r = sales_row(&code, "45.00");
        r[8] = "yes".to_string();
        // r[9] (who pulled) left blank
        r[10] = "15".to_string();
        let result = apply_sales_rows(&mut conn, &sales_headers(), &[r], 0).unwrap();
        assert_eq!(result.created, 1, "a missing puller name must never block the sale itself");
        assert!(pulls_received_for_order(&conn, &code).is_empty());
    }

    #[test]
    fn unparseable_how_much_pull_defaults_the_linked_rows_amount_to_zero_and_does_not_block_the_sale() {
        let mut conn = test_conn();
        let code = seed_order_with_quantity(&conn, 1);
        let mut r = sales_row(&code, "45.00");
        r[8] = "yes".to_string();
        r[9] = "Jozef".to_string();
        r[10] = "not a number".to_string();
        let result = apply_sales_rows(&mut conn, &sales_headers(), &[r], 0).unwrap();
        assert_eq!(result.created, 1);
        let rows = pulls_received_for_order(&conn, &code);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].3, 0, "unparseable amount must default to 0, never block the sale");
    }

    #[test]
    fn pull_info_added_to_an_already_fully_synced_row_still_links_on_a_later_sync() {
        // 2.0.23: marko's real workflow - sync the sale first, add pull info
        // to that same sheet row afterwards, sync again.
        let mut conn = test_conn();
        let code = seed_order_with_quantity(&conn, 1);

        let first = apply_sales_rows(&mut conn, &sales_headers(), &[sales_row(&code, "45.00")], 0).unwrap();
        assert_eq!(first.created, 1);
        assert!(pulls_received_for_order(&conn, &code).is_empty(), "no pull info yet - nothing to link");

        let mut r = sales_row(&code, "45.00");
        r[8] = "yes".to_string();
        r[9] = "Jozef".to_string();
        r[10] = "15".to_string();
        let second = apply_sales_rows(&mut conn, &sales_headers(), &[r], 0).unwrap();
        assert_eq!(second.created, 0, "the sale itself was already synced - nothing new to create there");
        assert_eq!(second.updated, 1, "a freshly linked pulls_received row is a real change, not a no-op");
        assert_eq!(second.unchanged, 0);

        let rows = pulls_received_for_order(&conn, &code);
        assert_eq!(rows.len(), 1);
        let (puller_name, event_name, quantity, amount_cents, source) = &rows[0];
        assert_eq!(puller_name, "Jozef");
        assert_eq!(event_name, "Coldplay Arena Show", "copied from the order's own linked event");
        assert_eq!(*quantity, 1);
        assert_eq!(*amount_cents, 1500);
        assert_eq!(source, "sheet_sync");
    }

    #[test]
    fn a_pull_already_linked_on_an_already_sold_row_stays_unchanged_and_unduplicated_on_a_further_sync() {
        let mut conn = test_conn();
        let code = seed_order_with_quantity(&conn, 1);
        apply_sales_rows(&mut conn, &sales_headers(), &[sales_row(&code, "45.00")], 0).unwrap();

        let mut r = sales_row(&code, "45.00");
        r[8] = "yes".to_string();
        r[9] = "Jozef".to_string();
        r[10] = "15".to_string();
        apply_sales_rows(&mut conn, &sales_headers(), &[r.clone()], 0).unwrap();
        assert_eq!(pulls_received_for_order(&conn, &code).len(), 1);

        let third = apply_sales_rows(&mut conn, &sales_headers(), &[r], 0).unwrap();
        assert_eq!(third.created, 0);
        assert_eq!(third.updated, 0, "already linked - a further sync must settle back to unchanged");
        assert_eq!(third.unchanged, 1);
        assert_eq!(pulls_received_for_order(&conn, &code).len(), 1, "must not create a second linked row");
    }

    #[test]
    fn pull_info_added_later_uses_the_orders_full_sold_quantity_not_zero() {
        let mut conn = test_conn();
        let code = seed_order_with_quantity(&conn, 2);
        let first = apply_sales_rows(&mut conn, &sales_headers(), &[sales_row(&code, "45.00")], 0).unwrap();
        assert_eq!(first.created, 1);

        let mut r = sales_row(&code, "45.00");
        r[8] = "yes".to_string();
        r[9] = "Jozef".to_string();
        let second = apply_sales_rows(&mut conn, &sales_headers(), &[r], 0).unwrap();
        assert_eq!(second.updated, 1);

        let rows = pulls_received_for_order(&conn, &code);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].2, 2, "both tickets were sold - quantity must reflect the order's real sold count, not the 0 sellable at sync time");
    }

    #[test]
    fn pull_info_on_a_row_with_no_sold_tickets_at_all_never_links_and_stays_unchanged() {
        let mut conn = test_conn();
        let code = seed_order_with_quantity(&conn, 1);
        let ticket_id = ticket_ids_for_order(&conn, &code)[0];
        conn.execute("UPDATE tickets SET status = 'cancelled' WHERE id = ?1", [ticket_id]).unwrap();

        let mut r = sales_row(&code, "45.00");
        r[8] = "yes".to_string();
        r[9] = "Jozef".to_string();
        r[10] = "15".to_string();
        let result = apply_sales_rows(&mut conn, &sales_headers(), &[r], 0).unwrap();
        assert_eq!(result.created, 0);
        assert_eq!(result.updated, 0, "nothing on this order ever sold - must never try to link a zero-quantity pull");
        assert_eq!(result.unchanged, 1);
        assert!(result.errors.is_empty());
        assert!(pulls_received_for_order(&conn, &code).is_empty());
    }

    #[test]
    fn resyncing_the_same_order_never_creates_a_second_linked_pulls_received_row() {
        let mut conn = test_conn();
        let code = seed_order_with_quantity(&conn, 2);
        let ticket_ids = ticket_ids_for_order(&conn, &code);
        // Simulate one ticket not yet being sellable on the first sync pass
        // (this module's own "creation-only" doc comment: partial
        // fulfillment across more than one apply_sales_rows run).
        conn.execute("UPDATE tickets SET status = 'cancelled' WHERE id = ?1", [ticket_ids[1]]).unwrap();

        let mut r = sales_row(&code, "45.00");
        r[8] = "yes".to_string();
        r[9] = "Jozef".to_string();
        r[10] = "15".to_string();
        let first = apply_sales_rows(&mut conn, &sales_headers(), &[r.clone()], 0).unwrap();
        assert_eq!(first.created, 1);
        assert_eq!(pulls_received_for_order(&conn, &code).len(), 1);

        // The second ticket becomes sellable and the SAME row is synced
        // again - must create the second Sale, but never a second linked row.
        conn.execute("UPDATE tickets SET status = 'available' WHERE id = ?1", [ticket_ids[1]]).unwrap();
        let second = apply_sales_rows(&mut conn, &sales_headers(), &[r], 0).unwrap();
        assert_eq!(second.created, 1, "the newly-sellable ticket must still get its own Sale");
        assert_eq!(pulls_received_for_order(&conn, &code).len(), 1, "must not create a second linked pulls_received row for the same order");
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
                currency: None,
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

    // =========================================================================
    // Push (app -> sheet), 2.0.18
    // =========================================================================

    fn seed_event(conn: &Connection) -> i64 {
        conn.execute("INSERT INTO events (name) VALUES ('Coldplay Arena Show')", []).unwrap();
        conn.last_insert_rowid()
    }

    /// A brand-new, never-synced local order - the opposite starting point
    /// from `seed_order_with_quantity` above (which goes through
    /// `apply_order_rows`, so is already linked). No seats, so this works
    /// for any quantity without that cross-check getting in the way.
    fn local_order_input(event_id: i64, quantity: i64) -> OrderInput {
        OrderInput {
            event_id,
            supplier_id: None,
            platform_id: None,
            purchase_date: "2026-09-15".to_string(),
            quantity,
            unit_price_cents: 5000,
            fees_cents: 0,
            other_costs_cents: 0,
            currency: "EUR".to_string(),
            payment_status: None,
            notes: Some("buyer@example.com".to_string()),
            ticket_type: Some("e-ticket".to_string()),
            section: Some("410".to_string()),
            row_label: Some("25".to_string()),
            tier: None,
            seats: None,
        }
    }

    // ---- push_orders: build_order_append_row / join_seats -------------------

    #[test]
    fn build_order_append_row_places_every_field_at_its_real_column_position() {
        let order = OrderForPush {
            id: 1,
            code: "ORD-000001".to_string(),
            event_name: "Coldplay Arena Show".to_string(),
            purchase_date: "2026-09-15".to_string(),
            platform_name: Some("ticketmaster".to_string()),
            quantity: 2,
            unit_price_cents: 5000,
            currency: "EUR".to_string(),
            notes: Some("buyer@example.com".to_string()),
            external_reference: Some("TM-88213".to_string()),
        };
        let tickets = vec![
            TicketForPush {
                section: Some("410".to_string()),
                row_label: Some("25".to_string()),
                ticket_type: Some("e-ticket".to_string()),
                seat: Some("11".to_string()),
            },
            TicketForPush {
                section: Some("410".to_string()),
                row_label: Some("25".to_string()),
                ticket_type: Some("e-ticket".to_string()),
                seat: Some("12".to_string()),
            },
        ];
        let map = build_header_map(&full_headers());
        let row = build_order_append_row(&map, MARKER_COL, full_headers().len(), &order, &tickets);

        assert_eq!(row.len(), MARKER_COL + 1);
        assert_eq!(row[0], "Coldplay Arena Show");
        assert_eq!(row[1], "15/09/2026", "must round-trip to DD/MM/YYYY, not the app's internal ISO storage");
        assert_eq!(row[2], "ticketmaster");
        assert_eq!(row[3], "410");
        assert_eq!(row[4], "25");
        assert_eq!(row[5], "11, 12");
        assert_eq!(row[6], "TM-88213");
        assert_eq!(row[7], "100,00", "Total Purchase Price = unit price x quantity");
        assert_eq!(row[8], "2");
        assert_eq!(row[9], "50,00");
        assert_eq!(row[10], "EUR");
        assert_eq!(row[11], "buyer@example.com");
        assert_eq!(row[12], "e-ticket");
        assert_eq!(row[MARKER_COL], "ORD-000001");
    }

    #[test]
    fn join_seats_is_blank_unless_every_ticket_has_its_own_seat() {
        let blank_ticket = || TicketForPush { section: None, row_label: None, ticket_type: None, seat: None };
        let all_seated = vec![
            TicketForPush { seat: Some("1".to_string()), ..blank_ticket() },
            TicketForPush { seat: Some("2".to_string()), ..blank_ticket() },
        ];
        assert_eq!(join_seats(&all_seated), "1, 2");

        let partially_seated = vec![TicketForPush { seat: Some("1".to_string()), ..blank_ticket() }, blank_ticket()];
        assert_eq!(join_seats(&partially_seated), "", "a partial mix has no faithful single-cell representation");

        assert_eq!(join_seats(&[]), "");
    }

    // ---- push_orders: apply_order_push ---------------------------------------

    #[test]
    fn a_never_linked_order_is_queued_as_an_append_and_its_link_deferred() {
        let conn = test_conn();
        let event_id = seed_event(&conn);
        let order_id = insert_order_with_tickets(&conn, &local_order_input(event_id, 2), false).unwrap();

        let (result, rows, pending_links) = apply_order_push(&conn, &full_headers(), MARKER_COL).unwrap();
        assert_eq!(result.created, 1);
        assert_eq!(result.errors.len(), 0);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0][0], "Coldplay Arena Show");
        assert_eq!(pending_links.len(), 1);
        assert_eq!(pending_links[0].0, order_id);

        // 2.2.10: apply_order_push is now the pure/testable half only - it
        // must NOT write sheet_sync_links itself any more (that would repeat
        // the exact bug this release fixes: a record marked "synced" before
        // the sheet write it describes is confirmed to have happened). Only
        // push_orders_impl, after a real successful append_values call,
        // writes these rows - see both functions' own doc comments.
        let links: i64 =
            conn.query_row("SELECT COUNT(*) FROM sheet_sync_links WHERE data_source='orders'", [], |r| r.get(0)).unwrap();
        assert_eq!(links, 0, "linking must wait for push_orders_impl to confirm the sheet write actually succeeded");
    }

    #[test]
    fn a_demo_order_is_never_pushed() {
        let conn = test_conn();
        let event_id = seed_event(&conn);
        insert_order_with_tickets(&conn, &local_order_input(event_id, 1), true).unwrap(); // is_demo = true

        let (result, rows, pending_links) = apply_order_push(&conn, &full_headers(), MARKER_COL).unwrap();
        assert_eq!(result.created, 0);
        assert!(rows.is_empty());
        assert!(pending_links.is_empty());
    }

    #[test]
    fn an_already_linked_order_is_never_touched_by_push() {
        let conn = test_conn();
        seed_order_with_quantity(&conn, 1); // via apply_order_rows -> already linked

        let (result, rows, pending_links) = apply_order_push(&conn, &full_headers(), MARKER_COL).unwrap();
        assert_eq!(result.created, 0, "an order that already has a TIQR ID must never be re-appended");
        assert!(rows.is_empty());
        assert!(pending_links.is_empty());
    }

    #[test]
    fn pushing_two_brand_new_orders_appends_both() {
        let conn = test_conn();
        let event_id = seed_event(&conn);
        insert_order_with_tickets(&conn, &local_order_input(event_id, 1), false).unwrap();
        insert_order_with_tickets(&conn, &local_order_input(event_id, 3), false).unwrap();

        let (result, rows, pending_links) = apply_order_push(&conn, &full_headers(), MARKER_COL).unwrap();
        assert_eq!(result.created, 2);
        assert_eq!(rows.len(), 2);
        assert_eq!(pending_links.len(), 2);
    }

    #[test]
    fn apply_order_push_also_requires_the_first_batch_headers() {
        let conn = test_conn();
        let headers: Vec<String> = vec!["platform".to_string()];
        let err = apply_order_push(&conn, &headers, 1).unwrap_err();
        assert!(err.to_string().contains("Event Name"), "{err}");
    }

    // ---- push_sales: apply_sales_push ----------------------------------------

    fn blank_sales_row(marker: &str) -> Vec<String> {
        vec![marker, "", "", "", "", "", "", "", "", "", ""].into_iter().map(String::from).collect()
    }

    #[test]
    fn a_fully_and_uniformly_sold_linked_order_with_a_blank_sheet_row_gets_pushed() {
        let mut conn = test_conn();
        let code = seed_order_with_quantity(&conn, 2);
        apply_sales_rows(&mut conn, &sales_headers(), &[sales_row(&code, "45.00")], 0).unwrap();

        let (result, writes) = apply_sales_push(&conn, &sales_headers(), &[blank_sales_row(&code)], 0).unwrap();
        assert_eq!(result.updated, 1);
        assert_eq!(result.unchanged, 0);
        assert_eq!(writes.len(), 1);
        let (row_number, cells) = &writes[0];
        assert_eq!(*row_number, 2);
        let as_map: HashMap<usize, String> = cells.iter().cloned().collect();
        assert_eq!(as_map.get(&1), Some(&"viagogo".to_string()), "Site Listed");
        assert_eq!(as_map.get(&2), Some(&"45,00".to_string()), "Payout Per Ticket");
        assert_eq!(as_map.get(&3), Some(&"Listed".to_string()), "Status");
        assert_eq!(as_map.get(&4), Some(&"Not yet".to_string()), "Delivery status");
        assert_eq!(as_map.get(&5), Some(&"paid".to_string()), "Payout status");
        assert_eq!(as_map.get(&6), Some(&"20/09/2026".to_string()), "sale date, DD/MM/YYYY");
        assert_eq!(as_map.get(&7), Some(&"buyer@example.com".to_string()), "paid by");
    }

    #[test]
    fn an_order_not_fully_sold_yet_is_left_alone() {
        let conn = test_conn();
        let code = seed_order_with_quantity(&conn, 1); // never sold
        let (result, writes) = apply_sales_push(&conn, &sales_headers(), &[blank_sales_row(&code)], 0).unwrap();
        assert_eq!(result.updated, 0);
        assert_eq!(result.unchanged, 1);
        assert!(writes.is_empty());
    }

    #[test]
    fn a_row_that_already_has_any_sale_info_is_never_touched_even_if_incomplete() {
        let mut conn = test_conn();
        let code = seed_order_with_quantity(&conn, 1);
        apply_sales_rows(&mut conn, &sales_headers(), &[sales_row(&code, "45.00")], 0).unwrap();

        let mut partially_filled = blank_sales_row(&code);
        partially_filled[1] = "someone typed this by hand".to_string(); // Site Listed

        let (result, writes) = apply_sales_push(&conn, &sales_headers(), &[partially_filled], 0).unwrap();
        assert_eq!(result.updated, 0);
        assert_eq!(result.unchanged, 1);
        assert!(writes.is_empty(), "must never touch a row that already has anything in its sale-info columns");
    }

    #[test]
    fn non_uniform_sales_across_tickets_of_one_order_are_left_alone() {
        let conn = test_conn();
        let code = seed_order_with_quantity(&conn, 2);
        let ticket_ids = ticket_ids_for_order(&conn, &code);
        // Sold separately at two different prices - a real, valid app state
        // the sheet's one-row-per-order model simply can't represent.
        conn.execute(
            "INSERT INTO sales (code, ticket_id, sale_date, sale_price_cents, currency, payment_status)
             VALUES ('SALE-000001', ?1, '2026-09-20', 4000, 'EUR', 'paid')",
            [ticket_ids[0]],
        )
        .unwrap();
        conn.execute("UPDATE tickets SET status='sold' WHERE id=?1", [ticket_ids[0]]).unwrap();
        conn.execute(
            "INSERT INTO sales (code, ticket_id, sale_date, sale_price_cents, currency, payment_status)
             VALUES ('SALE-000002', ?1, '2026-09-20', 5000, 'EUR', 'paid')",
            [ticket_ids[1]],
        )
        .unwrap();
        conn.execute("UPDATE tickets SET status='sold' WHERE id=?1", [ticket_ids[1]]).unwrap();

        let (result, writes) = apply_sales_push(&conn, &sales_headers(), &[blank_sales_row(&code)], 0).unwrap();
        assert_eq!(result.updated, 0);
        assert_eq!(result.unchanged, 1);
        assert!(writes.is_empty());
    }

    #[test]
    fn a_linked_pull_received_row_is_pushed_into_blank_pull_columns() {
        let conn = test_conn();
        let code = seed_order_with_quantity(&conn, 1);
        let order_id: i64 = conn.query_row("SELECT id FROM orders WHERE code = ?1", [&code], |r| r.get(0)).unwrap();
        let input = PullReceivedInput {
            puller_name: "Ivan".to_string(),
            event_name: "Coldplay Arena Show".to_string(),
            event_date: None,
            quantity: 1,
            amount_cents: 1000,
            currency: "EUR".to_string(),
            more_info: None,
            order_id: Some(order_id),
        };
        pulls_received::create_pull_received_with_source(&conn, &input, false, "manual").unwrap();

        // This order was never sold at all - proving the pull-info group
        // pushes independently of the sale-info group.
        let (result, writes) = apply_sales_push(&conn, &sales_headers(), &[blank_sales_row(&code)], 0).unwrap();
        assert_eq!(result.updated, 1);
        assert_eq!(writes.len(), 1);
        let as_map: HashMap<usize, String> = writes[0].1.iter().cloned().collect();
        assert_eq!(as_map.get(&8), Some(&"yes".to_string()), "pull");
        assert_eq!(as_map.get(&9), Some(&"Ivan".to_string()), "who pulled");
        assert_eq!(as_map.get(&10), Some(&"10,00".to_string()), "how much pull");
    }

    #[test]
    fn more_than_one_linked_pull_received_row_is_left_alone() {
        let conn = test_conn();
        let code = seed_order_with_quantity(&conn, 1);
        let order_id: i64 = conn.query_row("SELECT id FROM orders WHERE code = ?1", [&code], |r| r.get(0)).unwrap();
        for name in ["Ivan", "Peter"] {
            let input = PullReceivedInput {
                puller_name: name.to_string(),
                event_name: "Coldplay Arena Show".to_string(),
                event_date: None,
                quantity: 1,
                amount_cents: 1000,
                currency: "EUR".to_string(),
                more_info: None,
                order_id: Some(order_id),
            };
            pulls_received::create_pull_received_with_source(&conn, &input, false, "manual").unwrap();
        }

        let (result, writes) = apply_sales_push(&conn, &sales_headers(), &[blank_sales_row(&code)], 0).unwrap();
        assert_eq!(result.updated, 0);
        assert_eq!(result.unchanged, 1);
        assert!(writes.is_empty(), "ambiguous which of 2 linked pulls to show - must not guess");
    }

    #[test]
    fn an_order_whose_marker_is_not_in_the_current_sheet_is_quietly_skipped() {
        let mut conn = test_conn();
        let code = seed_order_with_quantity(&conn, 1);
        apply_sales_rows(&mut conn, &sales_headers(), &[sales_row(&code, "45.00")], 0).unwrap();

        // The sheet's currently-fetched data doesn't contain this order's
        // row at all (e.g. it scrolled outside a narrower range, or was
        // deleted) - nothing to push onto, and nothing worth alarming marko
        // about either; push_orders/a future Order sync already surfaces
        // row-level problems loudly on their own.
        let (result, writes) = apply_sales_push(&conn, &sales_headers(), &[], 0).unwrap();
        assert_eq!(result.updated, 0);
        assert_eq!(result.unchanged, 0, "not found at all - not even counted");
        assert!(result.errors.is_empty());
        assert!(writes.is_empty());
    }

    #[test]
    fn apply_sales_push_still_requires_the_payout_per_ticket_column() {
        let conn = test_conn();
        let headers: Vec<String> = vec!["TIQR ID".to_string(), "Site Listed".to_string()];
        let err = apply_sales_push(&conn, &headers, &[], 0).unwrap_err();
        assert!(err.to_string().contains("Payout Per Ticket"), "{err}");
    }

    // ---- "Fix sync" (force_push_sales_impl / apply_sales_push_internal) ------

    #[test]
    fn force_push_corrects_only_the_cell_that_actually_disagrees() {
        let mut conn = test_conn();
        let code = seed_order_with_quantity(&conn, 1);
        apply_sales_rows(&mut conn, &sales_headers(), &[sales_row(&code, "45.00")], 0).unwrap();

        // Every column already holds exactly what push_sales_impl would
        // write, EXCEPT Site Listed, which is stale (e.g. it was pushed once,
        // then the platform got corrected in the app afterwards). The
        // ordinary push must still refuse to touch this row at all - same
        // "never touch a row with anything already in it" rule as
        // `a_row_that_already_has_any_sale_info_is_never_touched_even_if_incomplete`.
        let stale_row: Vec<String> =
            vec![&code, "an old, now-wrong site", "45,00", "Listed", "Not yet", "paid", "20/09/2026", "buyer@example.com", "", "", ""]
                .into_iter()
                .map(String::from)
                .collect();
        let (result, writes) = apply_sales_push(&conn, &sales_headers(), &[stale_row.clone()], 0).unwrap();
        assert_eq!(result.updated, 0);
        assert!(writes.is_empty(), "ordinary push must still never touch a non-blank row");

        // Force push corrects the one cell that's actually wrong, and
        // nothing else.
        let (result, writes) = apply_sales_push_internal(&conn, &sales_headers(), &[stale_row], 0, true).unwrap();
        assert_eq!(result.updated, 1);
        assert_eq!(writes.len(), 1);
        let (row_number, cells) = &writes[0];
        assert_eq!(*row_number, 2);
        assert_eq!(cells.len(), 1, "every other cell already matched, so only one write should be queued");
        let as_map: HashMap<usize, String> = cells.iter().cloned().collect();
        assert_eq!(as_map.get(&1), Some(&"viagogo".to_string()), "the stale Site Listed cell gets corrected");
    }

    #[test]
    fn force_push_leaves_an_already_correct_row_completely_alone() {
        let mut conn = test_conn();
        let code = seed_order_with_quantity(&conn, 1);
        apply_sales_rows(&mut conn, &sales_headers(), &[sales_row(&code, "45.00")], 0).unwrap();

        // A row that already holds exactly what the app would write BACK -
        // note this is the OUTPUT format (comma decimal separator,
        // DD/MM/YYYY date), not `sales_row`'s own INPUT format (which is
        // what a person, or an Order-sync pull, would type in - "45.00"
        // with a period parses fine going in, but this module always
        // writes "45,00" with a comma going back out, so it is NOT the
        // same text and reusing `sales_row` here would wrongly look like a
        // mismatch). Force push must not queue a single write for a row
        // that already looks like this, so running it again (or clicking
        // it on a sheet that's already fine) is always a no-op.
        let already_correct: Vec<String> =
            vec![&code, "viagogo", "45,00", "Listed", "Not yet", "paid", "20/09/2026", "buyer@example.com", "", "", ""]
                .into_iter()
                .map(String::from)
                .collect();
        let (result, writes) = apply_sales_push_internal(&conn, &sales_headers(), &[already_correct], 0, true).unwrap();
        assert_eq!(result.updated, 0);
        assert_eq!(result.unchanged, 1);
        assert!(writes.is_empty(), "force push must never rewrite a cell that already has the value it wants to write");
    }

    #[test]
    fn force_push_still_requires_a_uniform_sale_across_the_order() {
        let conn = test_conn();
        let code = seed_order_with_quantity(&conn, 2);
        let ticket_ids = ticket_ids_for_order(&conn, &code);
        // Sold separately at two different prices, same as
        // `non_uniform_sales_across_tickets_of_one_order_are_left_alone` - a
        // real state the sheet's one-row-per-order model can't represent,
        // force or not.
        conn.execute(
            "INSERT INTO sales (code, ticket_id, sale_date, sale_price_cents, currency, payment_status)
             VALUES ('SALE-000001', ?1, '2026-09-20', 4000, 'EUR', 'paid')",
            [ticket_ids[0]],
        )
        .unwrap();
        conn.execute("UPDATE tickets SET status='sold' WHERE id=?1", [ticket_ids[0]]).unwrap();
        conn.execute(
            "INSERT INTO sales (code, ticket_id, sale_date, sale_price_cents, currency, payment_status)
             VALUES ('SALE-000002', ?1, '2026-09-20', 5000, 'EUR', 'paid')",
            [ticket_ids[1]],
        )
        .unwrap();
        conn.execute("UPDATE tickets SET status='sold' WHERE id=?1", [ticket_ids[1]]).unwrap();

        let mut row_data = blank_sales_row(&code);
        row_data[1] = "some stale value".to_string();
        let (result, writes) = apply_sales_push_internal(&conn, &sales_headers(), &[row_data], 0, true).unwrap();
        assert_eq!(result.updated, 0);
        assert_eq!(result.unchanged, 1);
        assert!(writes.is_empty(), "force must not invent a value when the order's own tickets disagree on price");
    }

    #[test]
    fn force_push_never_blanks_a_cell_the_app_has_no_opinion_on() {
        // 2.0.61 regression test - this is the exact scenario that made Fix
        // sync wipe a real Status/Delivery status cell on marko's live sheet.
        // A sale recorded through the app's own UI (Sales.tsx / the
        // Dashboard "New sale" shortcut) never sets a ticket's resale_status/
        // delivery_status - only Sales sync's PULL direction does that (see
        // apply_sales_rows) - so inserting the sale directly, the same way
        // `non_uniform_sales_across_tickets_of_one_order_are_left_alone`
        // already does, reproduces that real "the app has no opinion" state
        // instead of accidentally curing it via apply_sales_rows's own
        // stamping (which is what every earlier force-push test in this
        // file does, without realizing it was never exercising this case).
        let conn = test_conn();
        let code = seed_order_with_quantity(&conn, 1);
        let ticket_ids = ticket_ids_for_order(&conn, &code);
        conn.execute(
            "INSERT INTO sales (code, ticket_id, sale_date, sale_price_cents, currency, payment_status)
             VALUES ('SALE-000001', ?1, '2026-09-20', 4500, 'EUR', 'paid')",
            [ticket_ids[0]],
        )
        .unwrap();
        conn.execute("UPDATE tickets SET status='sold' WHERE id=?1", [ticket_ids[0]]).unwrap();

        // The sheet row already has real values in exactly the columns the
        // app has no opinion on for this order (no platform was ever
        // recorded either) - e.g. marko typed them in by hand. Payout/
        // Payout status/sale date are blank, matching his original report
        // (those are the ones "Push sales" should have filled and didn't).
        let mut row_data = blank_sales_row(&code);
        row_data[1] = "viagogo".to_string(); // Site Listed
        row_data[3] = "Listed".to_string(); // Status
        row_data[4] = "Not yet".to_string(); // Delivery status

        let (result, writes) = apply_sales_push_internal(&conn, &sales_headers(), &[row_data], 0, true).unwrap();
        assert_eq!(result.updated, 1);
        assert_eq!(writes.len(), 1);
        let as_map: HashMap<usize, String> = writes[0].1.iter().cloned().collect();
        assert!(!as_map.contains_key(&1), "Site Listed: no platform was ever recorded - must not blank it: {as_map:?}");
        assert!(!as_map.contains_key(&3), "Status: app never stamped resale_status for this ticket - must not blank it: {as_map:?}");
        assert!(!as_map.contains_key(&4), "Delivery status: same as Status - app has no opinion, must not blank it: {as_map:?}");
        assert!(!as_map.contains_key(&7), "paid by: no buyer_reference was ever recorded - must not write anything here: {as_map:?}");
        // The columns the app DOES have a real value for still get filled -
        // this button must still do what marko originally asked for.
        assert_eq!(as_map.get(&2), Some(&"45,00".to_string()), "Payout Per Ticket");
        assert_eq!(as_map.get(&5), Some(&"paid".to_string()), "Payout status");
        assert_eq!(as_map.get(&6), Some(&"20/09/2026".to_string()), "sale date");
    }

    // -----------------------------------------------------------------------
    // Refund staleness (2.0.80) - marko's own bug report: refunding a sale
    // never told the sheet, so a refunded ticket's pre-refund Sales-sync data
    // sat there forever - still counted by the Summary block, and still able
    // to look like "a new sale ready to record" to a future Sales sync pull.
    // See order_fully_refunded's own doc comment for the full incident.
    // -----------------------------------------------------------------------

    #[test]
    fn order_fully_refunded_is_false_for_an_order_never_sold_at_all() {
        let conn = test_conn();
        let code = seed_order_with_quantity(&conn, 1);
        let order_id: i64 = conn.query_row("SELECT id FROM orders WHERE code = ?1", [&code], |r| r.get(0)).unwrap();
        assert!(!order_fully_refunded(&conn, order_id).unwrap());
    }

    #[test]
    fn order_fully_refunded_is_false_while_normally_sold_and_never_refunded() {
        let mut conn = test_conn();
        let code = seed_order_with_quantity(&conn, 1);
        apply_sales_rows(&mut conn, &sales_headers(), &[sales_row(&code, "45.00")], 0).unwrap();
        let order_id: i64 = conn.query_row("SELECT id FROM orders WHERE code = ?1", [&code], |r| r.get(0)).unwrap();
        assert!(!order_fully_refunded(&conn, order_id).unwrap());
    }

    #[test]
    fn order_fully_refunded_is_true_once_every_ticket_on_the_order_is_refunded() {
        let mut conn = test_conn();
        let code = seed_order_with_quantity(&conn, 1);
        apply_sales_rows(&mut conn, &sales_headers(), &[sales_row(&code, "45.00")], 0).unwrap();
        let order_id: i64 = conn.query_row("SELECT id FROM orders WHERE code = ?1", [&code], |r| r.get(0)).unwrap();
        let ticket_id = ticket_ids_for_order(&conn, &code)[0];
        let sale_id: i64 = conn.query_row("SELECT id FROM sales WHERE ticket_id = ?1", [ticket_id], |r| r.get(0)).unwrap();
        refund_sale_impl(&mut conn, sale_id, None).unwrap();
        assert!(order_fully_refunded(&conn, order_id).unwrap());
    }

    #[test]
    fn order_fully_refunded_is_false_when_only_some_tickets_are_refunded() {
        let mut conn = test_conn();
        let code = seed_order_with_quantity(&conn, 2);
        apply_sales_rows(&mut conn, &sales_headers(), &[sales_row(&code, "45.00")], 0).unwrap();
        let order_id: i64 = conn.query_row("SELECT id FROM orders WHERE code = ?1", [&code], |r| r.get(0)).unwrap();
        let ticket_ids = ticket_ids_for_order(&conn, &code);
        let sale_id: i64 =
            conn.query_row("SELECT id FROM sales WHERE ticket_id = ?1", [ticket_ids[0]], |r| r.get(0)).unwrap();
        refund_sale_impl(&mut conn, sale_id, None).unwrap();
        // ticket_ids[1] is still genuinely sold - this order is not "fully"
        // refunded, same "can't represent, leave it alone" territory as any
        // other non-uniform order.
        assert!(!order_fully_refunded(&conn, order_id).unwrap());
    }

    #[test]
    fn a_fully_refunded_orders_stale_sheet_row_gets_cleared_by_push() {
        let mut conn = test_conn();
        let code = seed_order_with_quantity(&conn, 1);
        apply_sales_rows(&mut conn, &sales_headers(), &[sales_row(&code, "45.00")], 0).unwrap();

        let ticket_id = ticket_ids_for_order(&conn, &code)[0];
        let sale_id: i64 = conn.query_row("SELECT id FROM sales WHERE ticket_id = ?1", [ticket_id], |r| r.get(0)).unwrap();
        refund_sale_impl(&mut conn, sale_id, None).unwrap();

        // The sheet still shows the pre-refund row - nothing has pushed
        // since - ordinary "Push sales" must now clear it, unlike the "not
        // fully sold yet" row an ordinary, never-sold order leaves untouched
        // (an_order_not_fully_sold_yet_is_left_alone above).
        let (result, writes) = apply_sales_push(&conn, &sales_headers(), &[sales_row(&code, "45,00")], 0).unwrap();
        assert_eq!(result.updated, 1);
        assert_eq!(writes.len(), 1);
        let (row_number, cells) = &writes[0];
        assert_eq!(*row_number, 2);
        let as_map: HashMap<usize, String> = cells.iter().cloned().collect();
        for col in 1..=7 {
            assert_eq!(as_map.get(&col), Some(&String::new()), "column {col} must be cleared back to blank");
        }
        assert!(
            !as_map.contains_key(&8) && !as_map.contains_key(&9) && !as_map.contains_key(&10),
            "pull columns are a separate concern - must never be touched here: {as_map:?}"
        );
    }

    #[test]
    fn clearing_a_refunded_orders_row_is_idempotent() {
        let mut conn = test_conn();
        let code = seed_order_with_quantity(&conn, 1);
        apply_sales_rows(&mut conn, &sales_headers(), &[sales_row(&code, "45.00")], 0).unwrap();
        let ticket_id = ticket_ids_for_order(&conn, &code)[0];
        let sale_id: i64 = conn.query_row("SELECT id FROM sales WHERE ticket_id = ?1", [ticket_id], |r| r.get(0)).unwrap();
        refund_sale_impl(&mut conn, sale_id, None).unwrap();

        // The row is ALREADY blank (e.g. push already ran once) - running
        // push again must be a true no-op, same "safe to click again"
        // guarantee every other push/force-push path in this module gives.
        let (result, writes) = apply_sales_push(&conn, &sales_headers(), &[blank_sales_row(&code)], 0).unwrap();
        assert_eq!(result.updated, 0);
        assert_eq!(result.unchanged, 1);
        assert!(writes.is_empty());
    }

    #[test]
    fn force_push_also_clears_a_refunded_orders_stale_row() {
        let mut conn = test_conn();
        let code = seed_order_with_quantity(&conn, 1);
        apply_sales_rows(&mut conn, &sales_headers(), &[sales_row(&code, "45.00")], 0).unwrap();
        let ticket_id = ticket_ids_for_order(&conn, &code)[0];
        let sale_id: i64 = conn.query_row("SELECT id FROM sales WHERE ticket_id = ?1", [ticket_id], |r| r.get(0)).unwrap();
        refund_sale_impl(&mut conn, sale_id, None).unwrap();

        // Deliberately unconditional on `force` - see order_fully_refunded's
        // own call site comment for why. "Fix sync" must clear this too, not
        // only the ordinary "Push sales" button.
        let (result, writes) =
            apply_sales_push_internal(&conn, &sales_headers(), &[sales_row(&code, "45,00")], 0, true).unwrap();
        assert_eq!(result.updated, 1);
        assert_eq!(writes.len(), 1);
    }

    #[test]
    fn a_partially_refunded_order_is_left_alone_by_push() {
        let mut conn = test_conn();
        let code = seed_order_with_quantity(&conn, 2);
        apply_sales_rows(&mut conn, &sales_headers(), &[sales_row(&code, "45.00")], 0).unwrap();
        let ticket_ids = ticket_ids_for_order(&conn, &code);
        let sale_id: i64 =
            conn.query_row("SELECT id FROM sales WHERE ticket_id = ?1", [ticket_ids[0]], |r| r.get(0)).unwrap();
        refund_sale_impl(&mut conn, sale_id, None).unwrap();

        // ticket_ids[1] is still genuinely sold - same "can't represent,
        // leave it alone" territory as any other non-uniform order (see
        // non_uniform_sales_across_tickets_of_one_order_are_left_alone
        // above), not something this refund-staleness fix tries to improve.
        let (result, writes) = apply_sales_push(&conn, &sales_headers(), &[sales_row(&code, "45,00")], 0).unwrap();
        assert_eq!(result.updated, 0);
        assert_eq!(result.unchanged, 1);
        assert!(writes.is_empty());
    }

    // -----------------------------------------------------------------
    // Sheet structure (dropdowns + Revenue/Profit formulas), 2.0.19
    // -----------------------------------------------------------------

    /// Every column `plan_sheet_structure_updates` looks for, in one sheet -
    /// its own fixture rather than `full_headers()`/`sales_headers()`
    /// (neither has Total Purchase Price/Number of Tickets alongside
    /// Revenue/Profit at once). Order deliberately does NOT match
    /// `ORDERS_SHEET_HEADERS` - these tests exist specifically to prove
    /// real-column-position tolerance, same as every other `find_col`-based
    /// function in this module.
    fn structure_headers() -> Vec<String> {
        vec![
            "Ticket Type",
            "Site Listed",
            "Total Purchase Price",
            "Number of Tickets",
            "Payout Per Ticket",
            "Revenue",
            "Profit",
            "Status",
            "Delivery status",
            "Payout status",
            "pull",
        ]
        .into_iter()
        .map(String::from)
        .collect()
    }

    fn dropdown_values<'a>(dropdowns: &'a [DropdownSpec], headers: &[String], header_name: &str) -> Option<&'a Vec<String>> {
        let col = headers.iter().position(|h| h.eq_ignore_ascii_case(header_name))?;
        dropdowns.iter().find(|d| d.col_index == col).map(|d| &d.values)
    }

    #[test]
    fn plan_sheet_structure_updates_status_options_are_exactly_listed_unlisted_sold() {
        let conn = test_conn();
        let headers = structure_headers();
        let (dropdowns, _, _) = plan_sheet_structure_updates(&conn, &headers, 0).unwrap();
        assert_eq!(
            dropdown_values(&dropdowns, &headers, "Status"),
            Some(&vec!["Listed".to_string(), "Unlisted".to_string(), "Sold".to_string()])
        );
    }

    #[test]
    fn plan_sheet_structure_updates_delivery_status_options_are_exactly_delivered_not_delivered() {
        let conn = test_conn();
        let headers = structure_headers();
        let (dropdowns, _, _) = plan_sheet_structure_updates(&conn, &headers, 0).unwrap();
        assert_eq!(
            dropdown_values(&dropdowns, &headers, "Delivery status"),
            Some(&vec!["Delivered".to_string(), "Not delivered".to_string()])
        );
    }

    #[test]
    fn plan_sheet_structure_updates_payout_status_options_are_exactly_pending_paid() {
        let conn = test_conn();
        let headers = structure_headers();
        let (dropdowns, _, _) = plan_sheet_structure_updates(&conn, &headers, 0).unwrap();
        assert_eq!(dropdown_values(&dropdowns, &headers, "Payout status"), Some(&vec!["Pending".to_string(), "Paid".to_string()]));
    }

    #[test]
    fn plan_sheet_structure_updates_pull_options_are_exactly_yes_no() {
        let conn = test_conn();
        let headers = structure_headers();
        let (dropdowns, _, _) = plan_sheet_structure_updates(&conn, &headers, 0).unwrap();
        assert_eq!(dropdown_values(&dropdowns, &headers, "pull"), Some(&vec!["Yes".to_string(), "No".to_string()]));
    }

    #[test]
    fn plan_sheet_structure_updates_ticket_type_options_start_with_the_five_seed_defaults() {
        let conn = test_conn();
        let headers = structure_headers();
        let (dropdowns, _, _) = plan_sheet_structure_updates(&conn, &headers, 0).unwrap();
        assert_eq!(
            dropdown_values(&dropdowns, &headers, "Ticket Type"),
            Some(&vec![
                "E-ticket".to_string(),
                "PDF".to_string(),
                "Mobile transfer".to_string(),
                "Physical".to_string(),
                "Will call".to_string()
            ])
        );
    }

    #[test]
    fn plan_sheet_structure_updates_ticket_type_options_include_a_value_only_used_on_a_real_ticket() {
        let conn = test_conn();
        conn.execute("INSERT INTO events (name) VALUES ('Test Event')", []).unwrap();
        let event_id = conn.last_insert_rowid();
        let input = OrderInput {
            event_id,
            supplier_id: None,
            platform_id: None,
            purchase_date: "2026-01-01".to_string(),
            quantity: 1,
            unit_price_cents: 1000,
            fees_cents: 0,
            other_costs_cents: 0,
            currency: "EUR".to_string(),
            payment_status: Some("paid".to_string()),
            notes: None,
            ticket_type: Some("Season pass".to_string()),
            section: None,
            row_label: None,
            tier: None,
            seats: None,
        };
        insert_order_with_tickets(&conn, &input, false).unwrap();

        let headers = structure_headers();
        let (dropdowns, _, _) = plan_sheet_structure_updates(&conn, &headers, 0).unwrap();
        let values = dropdown_values(&dropdowns, &headers, "Ticket Type").unwrap();
        assert!(values.contains(&"Season pass".to_string()), "{values:?}");
    }

    #[test]
    fn plan_sheet_structure_updates_site_listed_options_are_platforms_tagged_sale_or_both_only() {
        let conn = test_conn();
        conn.execute("INSERT INTO platforms(name, kind) VALUES ('Viagogo', 'sale')", []).unwrap();
        conn.execute("INSERT INTO platforms(name, kind) VALUES ('Seatiks', 'both')", []).unwrap();
        conn.execute("INSERT INTO platforms(name, kind) VALUES ('PurchaseOnlyCo', 'purchase')", []).unwrap();

        let headers = structure_headers();
        let (dropdowns, _, _) = plan_sheet_structure_updates(&conn, &headers, 0).unwrap();
        let values = dropdown_values(&dropdowns, &headers, "Site Listed").unwrap();
        assert_eq!(values, &vec!["Seatiks".to_string(), "Viagogo".to_string()], "purchase-only platform must be excluded");
    }

    #[test]
    fn plan_sheet_structure_updates_skips_the_site_listed_dropdown_when_there_are_no_sale_platforms_yet() {
        let conn = test_conn();
        let headers = structure_headers();
        let (dropdowns, _, _) = plan_sheet_structure_updates(&conn, &headers, 0).unwrap();
        assert!(dropdown_values(&dropdowns, &headers, "Site Listed").is_none());
    }

    #[test]
    fn plan_sheet_structure_updates_skips_a_dropdown_column_the_sheet_does_not_have() {
        let conn = test_conn();
        // Only "Status" exists - no Delivery status/Payout status/pull/
        // Ticket Type/Site Listed anywhere in this sheet.
        let headers: Vec<String> = vec!["Status".to_string()];
        let (dropdowns, _, _) = plan_sheet_structure_updates(&conn, &headers, 0).unwrap();
        assert_eq!(dropdowns.len(), 1);
        assert_eq!(dropdowns[0].col_index, 0);
    }

    #[test]
    fn plan_sheet_structure_updates_revenue_formula_multiplies_payout_by_number_of_tickets_at_their_real_columns() {
        let conn = test_conn();
        let headers = structure_headers();
        let (_, revenue, _) = plan_sheet_structure_updates(&conn, &headers, 2).unwrap();
        let revenue = revenue.unwrap();
        assert_eq!(revenue.col_index, 5, "\"Revenue\" is column F (index 5) in structure_headers()");
        assert_eq!(revenue.formulas, vec!["=E2*D2".to_string(), "=E3*D3".to_string()], "Payout Per Ticket=E, Number of Tickets=D");
    }

    #[test]
    fn plan_sheet_structure_updates_profit_formula_subtracts_total_purchase_price_from_revenue_when_sold() {
        let conn = test_conn();
        let headers = structure_headers();
        let (_, _, profit) = plan_sheet_structure_updates(&conn, &headers, 2).unwrap();
        let profit = profit.unwrap();
        assert_eq!(profit.col_index, 6, "\"Profit\" is column G (index 6) in structure_headers()");
        assert_eq!(
            profit.formulas,
            vec!["=(H2=\"Sold\")*(F2-C2)".to_string(), "=(H3=\"Sold\")*(F3-C3)".to_string()],
            "Revenue=F, Total Purchase Price=C, Status=H - profit only counts once Status is Sold"
        );
    }

    #[test]
    fn plan_sheet_structure_updates_profit_formula_falls_back_to_ungated_when_the_sheet_has_no_status_column() {
        let conn = test_conn();
        // Same required columns Profit needs, minus "Status" entirely - same
        // optional-column tolerance as every other column in this function:
        // Profit must still work, just without the Sold-gating multiplier
        // since there is no cell left to gate on.
        let headers: Vec<String> = vec![
            "Total Purchase Price".to_string(),
            "Number of Tickets".to_string(),
            "Payout Per Ticket".to_string(),
            "Revenue".to_string(),
            "Profit".to_string(),
        ];
        let (_, _, profit) = plan_sheet_structure_updates(&conn, &headers, 1).unwrap();
        let profit = profit.unwrap();
        assert_eq!(profit.col_index, 4, "\"Profit\" is column E (index 4) in this header order");
        assert_eq!(profit.formulas, vec!["=D2-A2".to_string()], "Revenue=D, Total Purchase Price=A - no Status column to gate on");
    }

    #[test]
    fn plan_sheet_structure_updates_profit_formula_never_uses_a_locale_sensitive_function_argument_separator() {
        let conn = test_conn();
        let headers = structure_headers();
        let (_, _, profit) = plan_sheet_structure_updates(&conn, &headers, 1).unwrap();
        let formula = profit.unwrap().formulas[0].clone();
        // 2.0.43: the Sold-gate must stay a plain boolean-multiply
        // expression, never IF(...) - IF is itself a multi-argument
        // function call and would reintroduce the exact comma-vs-semicolon
        // locale trap SUMIF was rewritten off of in 2.0.42.
        assert!(!formula.contains(','), "must not use a comma anywhere - {formula}");
        assert!(!formula.to_uppercase().contains("IF("), "must not use IF() - {formula}");
    }

    #[test]
    fn plan_sheet_structure_updates_skips_revenue_and_profit_when_a_required_source_column_is_missing() {
        let conn = test_conn();
        // No "Number of Tickets", no "Total Purchase Price" anywhere.
        let headers: Vec<String> = vec!["Payout Per Ticket".to_string(), "Revenue".to_string(), "Profit".to_string()];
        let (_, revenue, profit) = plan_sheet_structure_updates(&conn, &headers, 3).unwrap();
        assert!(revenue.is_none());
        assert!(profit.is_none());
    }

    #[test]
    fn plan_sheet_structure_updates_skips_profit_but_keeps_revenue_when_only_profits_own_columns_are_missing() {
        let conn = test_conn();
        // Has everything Revenue needs, but no "Total Purchase Price" and no
        // "Profit" column at all - Revenue must still go ahead on its own.
        let headers: Vec<String> = vec!["Payout Per Ticket".to_string(), "Number of Tickets".to_string(), "Revenue".to_string()];
        let (_, revenue, profit) = plan_sheet_structure_updates(&conn, &headers, 3).unwrap();
        assert!(revenue.is_some());
        assert!(profit.is_none());
    }

    #[test]
    fn plan_sheet_structure_updates_writes_one_formula_per_data_row_matching_the_row_count() {
        let conn = test_conn();
        let headers = structure_headers();
        let (_, revenue, profit) = plan_sheet_structure_updates(&conn, &headers, 7).unwrap();
        assert_eq!(revenue.unwrap().formulas.len(), 7);
        assert_eq!(profit.unwrap().formulas.len(), 7);
    }

    #[test]
    fn plan_sheet_structure_updates_with_zero_data_rows_produces_no_formulas() {
        let conn = test_conn();
        let headers = structure_headers();
        let (_, revenue, profit) = plan_sheet_structure_updates(&conn, &headers, 0).unwrap();
        // Still Some (both required-column sets are present) - just an empty
        // formula list, exactly matching there being 0 real data rows yet.
        assert_eq!(revenue.unwrap().formulas.len(), 0);
        assert_eq!(profit.unwrap().formulas.len(), 0);
    }

    // -----------------------------------------------------------------------
    // Summary block (2.0.40) - marko's own request for an automatically-
    // calculated Total Cost/Revenue/Profit + Paid/Unpaid breakdown, placed 2
    // free columns past "how much pull".
    // -----------------------------------------------------------------------

    /// Every column `plan_orders_summary_updates` looks for, deliberately
    /// scrambled relative to `ORDERS_SHEET_HEADERS` - same "prove this is a
    /// real name lookup, not a hardcoded letter" philosophy as
    /// `structure_headers()` above.
    fn summary_headers() -> Vec<String> {
        vec!["Ticket Type", "Payout status", "Site Listed", "Revenue", "Profit", "Total Purchase Price", "how much pull", "Status"]
            .into_iter()
            .map(String::from)
            .collect()
    }

    fn text_col<'a>(spec: &'a OrdersSummarySpec, col_index: usize) -> Option<&'a Vec<String>> {
        spec.text_columns.iter().find(|c| c.col_index == col_index).map(|c| &c.values)
    }

    fn formula_col<'a>(spec: &'a OrdersSummarySpec, col_index: usize) -> Option<&'a Vec<String>> {
        spec.formula_columns.iter().find(|c| c.col_index == col_index).map(|c| &c.values)
    }

    #[test]
    fn plan_orders_summary_updates_starts_3_columns_past_how_much_pull_leaving_2_free() {
        // "how much pull" is index 6 in summary_headers() -> start_col = 9.
        let spec = plan_orders_summary_updates(&summary_headers()).unwrap();
        assert!(text_col(&spec, 9).is_some(), "Summary label column must be at how_much_pull_col + 3");
        assert!(text_col(&spec, 7).is_none(), "column 7 (how_much_pull + 1) must be left free");
        assert!(text_col(&spec, 8).is_none(), "column 8 (how_much_pull + 2) must be left free");
    }

    #[test]
    fn plan_orders_summary_updates_summary_block_has_the_right_labels_and_real_column_formulas() {
        let spec = plan_orders_summary_updates(&summary_headers()).unwrap();
        assert_eq!(text_col(&spec, 9), Some(&vec!["Summary".to_string(), "Total Cost".to_string(), "Total Revenue".to_string(), "Total Profit".to_string()]));
        // Total Purchase Price=F, Revenue=D, Profit=E, Payout status=B in
        // summary_headers(). Total Cost is SUMPRODUCT-with-coercion (Total
        // Purchase Price is written as literal text - see cost_formula's own
        // 2.0.62 comment). Total Revenue/Total Profit are SUMPRODUCT-with-
        // Paid-gate as of 2.0.80 - see plan_orders_summary_updates's own
        // 2.0.80 doc comment for why (marko's own bug report).
        assert_eq!(
            formula_col(&spec, 10),
            Some(&vec![
                String::new(),
                "=SUMPRODUCT((F2:F100000)*1)".to_string(),
                "=SUMPRODUCT((B2:B100000=\"Paid\")*D2:D100000)".to_string(),
                "=SUMPRODUCT((B2:B100000=\"Paid\")*E2:E100000)".to_string(),
            ])
        );
    }

    #[test]
    fn plan_orders_summary_updates_revenue_and_profit_only_count_paid_rows_2_0_80() {
        // marko's own explicit request (confirmed via AskUserQuestion after a
        // 2.0.80 bug report): "Total Revenue"/"Total Profit" must not
        // recognize a sale's revenue/profit until its Payout status is
        // literally "Paid" - the exact rule "Total Paid" already enforced
        // before this version, now shared by Revenue/Profit too. This
        // deliberately makes "Total Revenue" numerically IDENTICAL to "Total
        // Paid" (both are now the exact same SUMPRODUCT expression against
        // the Revenue column) - not a bug, see plan_orders_summary_updates's
        // own 2.0.80 doc comment. "Total Unpaid" is intentionally untouched
        // here - it still needs the *ungated* total (every sold row, Paid or
        // Pending) as its base, to correctly show "revenue sold but not yet
        // paid" - see the dedicated unpaid test below.
        let spec = plan_orders_summary_updates(&summary_headers()).unwrap();
        let revenue = &formula_col(&spec, 10).unwrap()[2];
        let profit = &formula_col(&spec, 10).unwrap()[3];
        let paid = &formula_col(&spec, 12).unwrap()[1];
        assert_eq!(revenue, paid, "Total Revenue must now equal Total Paid exactly - Pending/Refunded rows no longer count");
        assert_eq!(revenue, "=SUMPRODUCT((B2:B100000=\"Paid\")*D2:D100000)");
        assert_eq!(profit, "=SUMPRODUCT((B2:B100000=\"Paid\")*E2:E100000)");
    }

    #[test]
    fn plan_orders_summary_updates_paid_sums_revenue_where_payout_status_is_paid() {
        let spec = plan_orders_summary_updates(&summary_headers()).unwrap();
        assert_eq!(text_col(&spec, 11), Some(&vec!["Summary-Paid".to_string(), "Total Paid".to_string()]));
        // Payout status=B, Revenue=D.
        assert_eq!(
            formula_col(&spec, 12),
            Some(&vec![String::new(), "=SUMPRODUCT((B2:B100000=\"Paid\")*D2:D100000)".to_string()])
        );
    }

    #[test]
    fn plan_orders_summary_updates_unpaid_is_total_revenue_minus_paid_never_a_literal_unpaid_match() {
        // The one deliberate correction vs. marko's own first draft - see
        // plan_orders_summary_updates's own doc comment for why a literal
        // match against the text "Unpaid" would always be zero.
        let spec = plan_orders_summary_updates(&summary_headers()).unwrap();
        assert_eq!(text_col(&spec, 13), Some(&vec!["Summary-Unpaid".to_string(), "Total Unpaid".to_string()]));
        let unpaid = formula_col(&spec, 14).unwrap();
        assert_eq!(unpaid[1], "=SUM(D:D)-SUMPRODUCT((B2:B100000=\"Paid\")*D2:D100000)");
        assert!(!unpaid[1].to_lowercase().contains("\"unpaid\""), "must never literally match the text Unpaid");
    }

    #[test]
    fn plan_orders_summary_updates_cost_paid_and_unpaid_never_use_a_locale_sensitive_function_argument_separator() {
        // 2.0.42 regression test for the real bug: SUMIF(a,b,c) broke as
        // #ERROR! on marko's own comma-decimal-locale sheet, because Google
        // Sheets parses a USER_ENTERED formula's function-argument separator
        // per the spreadsheet's own locale (",", but ";" for comma-decimal
        // locales like Slovak) - see plan_orders_summary_updates's own doc
        // comment. SUMPRODUCT with a single array-expression argument has no
        // function-argument separator at all, for any locale - this asserts
        // that shape directly rather than just the exact formula text above,
        // so a future edit that reintroduces a multi-argument SUMIF/SUMIFS
        // here fails this test even if it changes the exact letters/bound.
        // 2.0.62: `cost` joined this test once it also became a SUMPRODUCT
        // (see its own doc comment for why) - same shape, same reasoning.
        // 2.0.80: `revenue`/`profit` joined for the same reason, once they
        // too became Paid-gated SUMPRODUCT expressions.
        let spec = plan_orders_summary_updates(&summary_headers()).unwrap();
        let cost = &formula_col(&spec, 10).unwrap()[1];
        let revenue = &formula_col(&spec, 10).unwrap()[2];
        let profit = &formula_col(&spec, 10).unwrap()[3];
        let paid = &formula_col(&spec, 12).unwrap()[1];
        let unpaid = &formula_col(&spec, 14).unwrap()[1];
        for formula in [cost, revenue, profit, paid, unpaid] {
            assert!(formula.contains("SUMPRODUCT"), "expected SUMPRODUCT in {formula}");
            assert!(!formula.contains(",\"Paid\","), "must not contain a comma-separated SUMIF-style argument list: {formula}");
        }
    }

    #[test]
    fn plan_orders_summary_updates_against_the_real_canonical_header_order_matches_markos_own_draft_letters() {
        // marko's own draft formulas used fixed letters H/P/Q/T - this proves
        // those exactly match ORDERS_SHEET_HEADERS's real canonical order
        // (Total Purchase Price=H, Revenue=P, Profit=Q, Payout status=T),
        // and that the block lands at AB (how much pull=Y, index 24, +3=27).
        let headers: Vec<String> = ORDERS_SHEET_HEADERS.iter().map(|s| s.to_string()).collect();
        let spec = plan_orders_summary_updates(&headers).unwrap();
        assert_eq!(column_index_to_a1(27), "AB");
        assert!(text_col(&spec, 27).is_some(), "block must start at column AB (index 27)");
        assert_eq!(formula_col(&spec, 28).unwrap()[1], "=SUMPRODUCT((H2:H100000)*1)");
        assert_eq!(formula_col(&spec, 28).unwrap()[2], "=SUMPRODUCT((T2:T100000=\"Paid\")*P2:P100000)");
        assert_eq!(formula_col(&spec, 28).unwrap()[3], "=SUMPRODUCT((T2:T100000=\"Paid\")*Q2:Q100000)");
        assert_eq!(formula_col(&spec, 30).unwrap()[1], "=SUMPRODUCT((T2:T100000=\"Paid\")*P2:P100000)");
        assert_eq!(formula_col(&spec, 32).unwrap()[1], "=SUM(P:P)-SUMPRODUCT((T2:T100000=\"Paid\")*P2:P100000)");
    }

    #[test]
    fn plan_orders_summary_updates_is_none_when_how_much_pull_is_missing() {
        let headers = row(&["Total Purchase Price", "Revenue", "Profit", "Payout status"]);
        assert!(plan_orders_summary_updates(&headers).is_none(), "no anchor column means no defined placement");
    }

    #[test]
    fn plan_orders_summary_updates_is_all_or_nothing_when_any_one_math_column_is_missing() {
        // Deliberately all-or-nothing (unlike plan_sheet_structure_updates's
        // per-formula tolerance) - see this function's own doc comment for
        // why a partially-rendered summary table is worse than none at all.
        let missing_profit = row(&["how much pull", "Total Purchase Price", "Revenue", "Payout status"]);
        assert!(plan_orders_summary_updates(&missing_profit).is_none());

        let missing_payout_status = row(&["how much pull", "Total Purchase Price", "Revenue", "Profit"]);
        assert!(plan_orders_summary_updates(&missing_payout_status).is_none());
    }

    // -----------------------------------------------------------------------
    // widest_referenced_column (2.0.41) - real incident: the Summary block's
    // own header cell landed at column AB (28th column) against a real
    // sheet's default 26-column grid, and batch_update's all-or-nothing
    // behavior took the ENTIRE refresh down with it, not just the new
    // styling. These tests lock in the exact number ensure_orders_sheet_
    // structure now uses to grow the grid before that can happen again.
    // -----------------------------------------------------------------------

    #[test]
    fn widest_referenced_column_is_none_when_nothing_needs_writing() {
        assert_eq!(widest_referenced_column(&[], &[], &None), None);
    }

    #[test]
    fn widest_referenced_column_considers_summarys_formula_columns_not_just_its_text_columns() {
        // Summary's own widest cell is a FORMULA column (Summary-Unpaid's
        // formula, one column right of its "Summary-Unpaid" label) - if this
        // only looked at text_columns, it would under-count by one and the
        // grid would be grown one column too few, right back to the same
        // crash for exactly that one cell.
        let headers = row(&["how much pull", "Total Purchase Price", "Revenue", "Profit", "Payout status"]);
        let summary = plan_orders_summary_updates(&headers);
        assert!(summary.is_some());
        let widest = widest_referenced_column(&[], &[], &summary).unwrap();
        let widest_of_formula_columns = summary.unwrap().formula_columns.iter().map(|c| c.col_index).max().unwrap();
        assert_eq!(widest, widest_of_formula_columns);
    }

    #[test]
    fn widest_referenced_column_without_a_summary_block_still_considers_dropdowns_and_colors() {
        // No "how much pull" at all - summary is None - but Status/Payout
        // status are still there, so dropdowns/colors alone must still
        // produce a real answer, not None. Protects the pre-2.0.40 code path
        // (this function existing at all must never make that case worse).
        let conn = test_conn();
        let headers = row(&["Status", "Payout status", "Delivery status"]);
        let (dropdowns, _, _) = plan_sheet_structure_updates(&conn, &headers, 0).unwrap();
        let colors = plan_sheet_color_updates(&headers);
        let summary = plan_orders_summary_updates(&headers);
        assert!(summary.is_none(), "no how much pull column - summary must not apply here");
        assert!(widest_referenced_column(&dropdowns, &colors, &summary).is_some());
    }

    #[test]
    fn widest_referenced_column_against_the_real_canonical_header_order_is_the_summary_blocks_own_far_column() {
        // The actual regression, reproduced end to end against
        // ORDERS_SHEET_HEADERS (Total Purchase Price=H/Revenue=P/Profit=Q/
        // Payout status=T/how much pull=Y, same real order marko's own real
        // sheet has) - every dropdown/color column in this canonical order
        // is well under column Z (index 25), so the Summary block itself
        // (ending at AG, index 32 - see plan_orders_summary_updates_against_
        // the_real_canonical_header_order_matches_markos_own_draft_letters
        // above) is what actually determines the answer here, exactly as it
        // did on marko's real sheet.
        let conn = test_conn();
        let headers: Vec<String> = ORDERS_SHEET_HEADERS.iter().map(|s| s.to_string()).collect();
        let (dropdowns, _, _) = plan_sheet_structure_updates(&conn, &headers, 0).unwrap();
        let colors = plan_sheet_color_updates(&headers);
        let summary = plan_orders_summary_updates(&headers);
        assert_eq!(widest_referenced_column(&dropdowns, &colors, &summary), Some(32));
        assert_eq!(column_index_to_a1(32), "AG");
    }

    // -----------------------------------------------------------------------
    // Color-coding (2.0.22) - marko's own request, exactly the three columns
    // he named (Status/Delivery status/Payout status) - never Ticket
    // Type/Site Listed/pull, which he did not mention.
    // -----------------------------------------------------------------------

    fn color_values<'a>(specs: &'a [ColorSpec], headers: &[String], header_name: &str) -> Option<&'a Vec<(String, (f64, f64, f64))>> {
        let col = headers.iter().position(|h| h.eq_ignore_ascii_case(header_name))?;
        specs.iter().find(|s| s.col_index == col).map(|s| &s.colors)
    }

    #[test]
    fn plan_sheet_color_updates_status_colors_are_exactly_listed_orange_unlisted_brown_sold_green() {
        let headers = structure_headers();
        let specs = plan_sheet_color_updates(&headers);
        assert_eq!(
            color_values(&specs, &headers, "Status"),
            Some(&vec![
                ("Listed".to_string(), COLOR_ORANGE),
                ("Unlisted".to_string(), COLOR_BROWN),
                ("Sold".to_string(), COLOR_GREEN),
            ])
        );
    }

    #[test]
    fn plan_sheet_color_updates_delivery_status_colors_are_exactly_delivered_green_not_delivered_orange() {
        let headers = structure_headers();
        let specs = plan_sheet_color_updates(&headers);
        assert_eq!(
            color_values(&specs, &headers, "Delivery status"),
            Some(&vec![("Delivered".to_string(), COLOR_GREEN), ("Not delivered".to_string(), COLOR_ORANGE)])
        );
    }

    #[test]
    fn plan_sheet_color_updates_payout_status_colors_are_exactly_pending_orange_paid_green() {
        let headers = structure_headers();
        let specs = plan_sheet_color_updates(&headers);
        assert_eq!(
            color_values(&specs, &headers, "Payout status"),
            Some(&vec![("Pending".to_string(), COLOR_ORANGE), ("Paid".to_string(), COLOR_GREEN)])
        );
    }

    #[test]
    fn plan_sheet_color_updates_skips_a_column_the_sheet_does_not_have() {
        // Only "Status" exists - no Delivery status/Payout status anywhere.
        let headers: Vec<String> = vec!["Status".to_string()];
        let specs = plan_sheet_color_updates(&headers);
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].col_index, 0);
    }

    #[test]
    fn plan_sheet_color_updates_never_colors_ticket_type_site_listed_or_pull() {
        // marko listed exactly Status/Delivery status/Payout status for
        // colors on this sheet - Ticket Type/Site Listed/pull already have
        // their own dropdown (2.0.19) but must never gain a color rule too.
        let headers = structure_headers();
        let specs = plan_sheet_color_updates(&headers);
        assert!(color_values(&specs, &headers, "Ticket Type").is_none());
        assert!(color_values(&specs, &headers, "Site Listed").is_none());
        assert!(color_values(&specs, &headers, "pull").is_none());
        assert_eq!(specs.len(), 3, "exactly Status, Delivery status, Payout status - nothing else");
    }

    // -----------------------------------------------------------------------
    // "Update sheet" (2.0.20) - same two tests as
    // pulls_sheet_sync::setup_pulls_sheet, see those tests' own comments for
    // why a real network call is never needed to exercise both of these
    // paths in this sandbox.
    // -----------------------------------------------------------------------

    #[test]
    fn setup_orders_sheet_rejects_up_front_when_nothing_is_connected_yet() {
        let conn = test_conn();
        let err = setup_orders_sheet_impl(&conn).unwrap_err();
        assert!(
            err.to_string().contains("No spreadsheet is connected"),
            "must fail with the same clear message sync/push already use, not a generic error: {err}"
        );
    }

    #[test]
    fn setup_orders_sheet_with_a_real_connection_fails_cleanly_when_no_service_account_is_embedded() {
        let conn = test_conn();
        set_sheets_connection_impl(&conn, "orders", "1AbC-XyZ_9900", "Orders", "EUR").unwrap();
        let err = setup_orders_sheet_impl(&conn).unwrap_err();
        assert!(
            err.to_string().contains("isn't available in this build"),
            "a real connection must reach the credential step (not panic/short-circuit some other way) and then stop cleanly before any network call in a test build: {err}"
        );
    }

    // -----------------------------------------------------------------------
    // currency_push_cells / push_order_currency_to_sheet (2.0.53) - see the
    // section's own doc comment above push_order_currency_to_sheet for the
    // full "why" (marko: a converted order's sheet row never got updated).
    // -----------------------------------------------------------------------

    #[test]
    fn currency_push_cells_finds_the_right_row_by_marker_and_builds_all_three_cells() {
        let headers = headers_with_marker();
        let data_rows = vec![sample_row("ORD-000001"), sample_row("ORD-000002")];
        let (row_number, cells) = currency_push_cells(&headers, &data_rows, "ORD-000002", "EUR", 5000, 10000).unwrap();
        assert_eq!(row_number, 3, "header is row 1, ORD-000001 is row 2, ORD-000002 is row 3");
        // currency=10, "Price Per Ticket"=9, "Total Purchase Price"=7 in full_headers()
        assert!(cells.contains(&(10, "EUR".to_string())), "{cells:?}");
        assert!(cells.contains(&(9, "50,00".to_string())), "{cells:?}");
        assert!(cells.contains(&(7, "100,00".to_string())), "{cells:?}");
        assert_eq!(cells.len(), 3);
    }

    #[test]
    fn currency_push_cells_never_pushes_a_computed_total_purchase_price_of_exactly_zero() {
        // 2.0.61: a real order's total purchase price is never actually
        // free - a computed 0 almost always means this order's own
        // total_cost_cents is itself wrong/unset, not that the sheet's
        // existing (possibly real, non-zero) cell needs replacing. Currency
        // and unit price still get pushed independently - only the
        // suspicious zero total is held back.
        let headers = headers_with_marker();
        let data_rows = vec![sample_row("ORD-000001")];
        let (_, cells) = currency_push_cells(&headers, &data_rows, "ORD-000001", "EUR", 5000, 0).unwrap();
        assert!(cells.iter().all(|(_, v)| v != "0,00"), "must never queue a 0,00 write: {cells:?}");
        assert_eq!(cells.len(), 2, "currency + unit price only - the zero total purchase price is skipped: {cells:?}");
    }

    #[test]
    fn currency_push_cells_returns_none_when_the_marker_is_not_in_any_row() {
        let headers = headers_with_marker();
        let data_rows = vec![sample_row("ORD-000001")];
        assert!(currency_push_cells(&headers, &data_rows, "ORD-999999", "EUR", 5000, 10000).is_none());
    }

    #[test]
    fn currency_push_cells_returns_none_when_the_sheet_has_no_marker_column_at_all() {
        let headers = full_headers(); // no TIQR ID column appended
        let data_rows = vec![sample_row("")];
        assert!(currency_push_cells(&headers, &data_rows, "ORD-000001", "EUR", 5000, 10000).is_none());
    }

    #[test]
    fn currency_push_cells_still_writes_the_columns_that_do_exist_when_one_is_missing() {
        // A sheet without its own "currency" column at all (unlikely for a
        // real connected sheet - check_required_headers doesn't require it
        // either way - but this must degrade to "write what we can", never
        // panic on a missing find_col result).
        let mut headers = full_headers();
        headers.retain(|h| h.to_lowercase() != "currency");
        headers.push(MARKER_HEADER.to_string());
        let mut r = row(&[
            "Coldplay Arena Show", "15/09/2026", "ticketmaster", "410", "25", "11,12", "TM-88213", "100.00", "2",
            "50.00", "buyer@example.com", "e-ticket",
        ]);
        r.push("ORD-000001".to_string());
        let (_, cells) = currency_push_cells(&headers, &[r], "ORD-000001", "EUR", 5000, 10000).unwrap();
        assert_eq!(cells.len(), 2, "currency column is gone - only unit price + total price cells: {cells:?}");
    }

    #[test]
    fn push_order_currency_to_sheet_is_a_silent_no_op_for_an_order_never_linked_to_any_sheet() {
        let conn = test_conn();
        // No sheet_sync_links row at all for order id 999 - and no orders
        // table row either, since the whole point is this returns before
        // ever needing one to exist.
        let (linked, err) = push_order_currency_to_sheet(&conn, 999);
        assert!(!linked);
        assert!(err.is_none());
    }

    #[test]
    fn push_order_currency_to_sheet_fails_cleanly_when_linked_but_nothing_is_connected() {
        let conn = test_conn();
        // apply_order_rows already links a newly-created order (inserts its
        // own sheet_sync_links row, marker = the order's generated code) -
        // no need to insert a second one here, and doing so would collide
        // with sheet_sync_links' own (data_source, local_id) primary key.
        apply_order_rows(&conn, &full_headers(), &[sample_row("")], "EUR", MARKER_COL).unwrap();
        let order_id: i64 = conn.query_row("SELECT id FROM orders LIMIT 1", [], |r| r.get(0)).unwrap();
        let (linked, err) = push_order_currency_to_sheet(&conn, order_id);
        assert!(linked, "this order IS linked, so it must attempt the push, not silently skip");
        let msg = err.expect("no Sheets connection is configured in this test - must report an error, not silently succeed");
        assert!(msg.contains("no spreadsheet is connected"), "{msg}");
    }

    #[test]
    fn push_order_currency_to_sheet_fails_cleanly_when_linked_and_connected_but_no_credential_is_embedded() {
        let conn = test_conn();
        apply_order_rows(&conn, &full_headers(), &[sample_row("")], "EUR", MARKER_COL).unwrap();
        let order_id: i64 = conn.query_row("SELECT id FROM orders LIMIT 1", [], |r| r.get(0)).unwrap();
        set_sheets_connection_impl(&conn, "orders", "1AbC-XyZ_9900", "Orders", "EUR").unwrap();
        let (linked, err) = push_order_currency_to_sheet(&conn, order_id);
        assert!(linked);
        let msg = err.expect("a real connection but no embedded credential in this test build must still report an error");
        assert!(msg.contains("isn't available in this build"), "{msg}");
    }

    #[test]
    fn reconcile_order_currencies_writes_only_the_order_whose_sheet_cell_is_actually_stale() {
        let conn = test_conn();
        let (_, writes) = apply_order_rows(&conn, &full_headers(), &[sample_row(""), sample_row("")], "EUR", MARKER_COL).unwrap();
        assert_eq!(writes.markers.len(), 2);
        let marker_1 = &writes.markers[0].1; // this run's own actual generated codes -
        let marker_2 = &writes.markers[1].1; // never assumed as literal "ORD-000001"/"-2"

        let order_ids: Vec<i64> = {
            let mut stmt = conn.prepare("SELECT id FROM orders ORDER BY id").unwrap();
            stmt.query_map([], |r| r.get(0)).unwrap().collect::<Result<Vec<_>, _>>().unwrap()
        };
        assert_eq!(order_ids.len(), 2);

        // Both orders converted to GBP locally (skipping the real conversion
        // path here on purpose - this test is only about the reconcile
        // function's own comparison logic, not convert_order_currency_impl,
        // which already has its own dedicated tests elsewhere).
        for &id in &order_ids {
            conn.execute("UPDATE orders SET currency = 'GBP' WHERE id = ?1", [id]).unwrap();
        }

        let headers = headers_with_marker();
        let data_rows = vec![
            sample_row(marker_1), // sheet still says EUR/50.00/100.00 here - stale
            {
                // This order's sheet row already says GBP with the exact
                // numbers the order was just set to above (comma-decimal,
                // matching what format_cents_for_sheet actually writes) -
                // already correct.
                let mut r = row(&[
                    "Coldplay Arena Show", "15/09/2026", "ticketmaster", "410", "25", "11,12", "TM-88213", "100,00",
                    "2", "50,00", "GBP", "buyer@example.com", "e-ticket",
                ]);
                r.push(marker_2.clone());
                r
            },
        ];

        let writes = reconcile_order_currencies(&conn, &headers, &data_rows);
        assert_eq!(writes.len(), 1, "only the first order's row is actually stale: {writes:?}");
        let (row_number, cells) = &writes[0];
        assert_eq!(*row_number, 2, "the first order's row is the first data row - sheet row 2");
        assert!(cells.iter().any(|(_, v)| v == "GBP"));
    }
}

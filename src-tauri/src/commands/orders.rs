use crate::codes;
use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::finance::{self, allocate_cents};
use crate::fx;
use crate::models::{
    BulkCurrencyConversionResult, BulkDeleteResult, BulkDeleteSkip, BulkOrdersDeliveryStatusInput,
    BulkOrdersPaymentStatusInput, Order, OrderCurrencyConversion, OrderCurrencyConversionResult, OrderEditInput,
    OrderInput, OrderSalesSummary, SeatEntry,
};
use rusqlite::{params, Connection, Row};
use tauri::State;

// Safety cap on the unfiltered list view - see the identical constant in
// commands/tickets.rs for the rationale.
const LIST_CAP: i64 = 5000;

const BASE_SQL: &str = "
    SELECT
      o.id, o.code, o.event_id, e.name as event_name,
      e.event_date as event_date, e.status as event_status,
      e.category_id, ec.name as category_name, ec.color_slot as category_color_slot,
      o.supplier_id, sup.name as supplier_name,
      o.platform_id, p.name as platform_name,
      o.purchase_date, o.quantity, o.unit_price_cents, o.fees_cents, o.other_costs_cents,
      o.total_cost_cents, o.currency, o.payment_status, o.notes, o.is_demo,
      o.created_at, o.updated_at,
      COUNT(CASE WHEN t.status='sold' THEN 1 END) as sold_count,
      COUNT(CASE WHEN t.status='available' THEN 1 END) as available_count,
      COUNT(CASE WHEN t.status='listed' THEN 1 END) as listed_count,
      COUNT(CASE WHEN t.status='cancelled' THEN 1 END) as cancelled_count,
      -- 2.0.66: the other two legs of the new 'Completed' indicator (see
      -- REDESIGN-2.0.66-REPORT.md) - both scoped to SOLD tickets only, same
      -- as this order's true 'how many are actually resolved' question. The
      -- `sa` join right below mirrors fetch_sales_summary's own identical
      -- join/comment further down this file: a ticket has at most one
      -- CURRENT (non-refunded) sale row, so this never fans out the ticket
      -- count above.
      COUNT(CASE WHEN t.status='sold' AND t.delivery_status='Delivered' THEN 1 END) as delivered_count,
      COUNT(CASE WHEN t.status='sold' AND sa.payment_status='paid' THEN 1 END) as paid_count,
      -- 2.0.38: raw per-ticket seat data for the new Seats column - see
      -- SeatEntry::parse_aggregate's own doc comment (models.rs) for exactly
      -- how this string is encoded/decoded and why (not a plain DISTINCT
      -- GROUP_CONCAT). Reuses the SAME `t` join every count above already
      -- uses - no new JOIN needed.
      GROUP_CONCAT(
        COALESCE(t.section,'') || char(31) || COALESCE(t.row_label,'') || char(31) || COALESCE(t.seat,''),
        char(30)
      ) as seats_raw
    FROM orders o
    JOIN events e ON e.id = o.event_id
    LEFT JOIN event_categories ec ON ec.id = e.category_id
    LEFT JOIN suppliers sup ON sup.id = o.supplier_id
    LEFT JOIN platforms p ON p.id = o.platform_id
    LEFT JOIN tickets t ON t.order_id = o.id
    LEFT JOIN sales sa ON sa.ticket_id = t.id AND sa.payment_status != 'refunded'
";

fn map_order(row: &Row) -> rusqlite::Result<Order> {
    Ok(Order {
        id: row.get("id")?,
        code: row.get("code")?,
        event_id: row.get("event_id")?,
        event_name: row.get("event_name")?,
        event_date: row.get("event_date")?,
        event_status: row.get("event_status")?,
        category_id: row.get("category_id")?,
        category_name: row.get("category_name")?,
        category_color_slot: row.get("category_color_slot")?,
        supplier_id: row.get("supplier_id")?,
        supplier_name: row.get("supplier_name")?,
        platform_id: row.get("platform_id")?,
        platform_name: row.get("platform_name")?,
        purchase_date: row.get("purchase_date")?,
        quantity: row.get("quantity")?,
        unit_price_cents: row.get("unit_price_cents")?,
        fees_cents: row.get("fees_cents")?,
        other_costs_cents: row.get("other_costs_cents")?,
        total_cost_cents: row.get("total_cost_cents")?,
        currency: row.get("currency")?,
        payment_status: row.get("payment_status")?,
        notes: row.get("notes")?,
        is_demo: row.get("is_demo")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        sold_count: row.get("sold_count")?,
        available_count: row.get("available_count")?,
        listed_count: row.get("listed_count")?,
        cancelled_count: row.get("cancelled_count")?,
        delivered_count: row.get("delivered_count")?,
        paid_count: row.get("paid_count")?,
        seats: SeatEntry::parse_aggregate(row.get::<_, Option<String>>("seats_raw")?.as_deref()),
    })
}

pub(crate) fn fetch_recent(conn: &Connection, limit: i64) -> AppResult<Vec<Order>> {
    let sql = format!("{BASE_SQL} GROUP BY o.id ORDER BY o.created_at DESC LIMIT ?1");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([limit], map_order)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

fn fetch_one(conn: &Connection, id: i64) -> AppResult<Order> {
    let sql = format!("{BASE_SQL} WHERE o.id = ?1 GROUP BY o.id");
    conn.query_row(&sql, [id], map_order)
        .map_err(|_| AppError::NotFound(format!("Order #{id} not found")))
}

/// Powers both the plain "Orders" list (search/event_id only, unchanged
/// behaviour) and the order-grouped "Tickets/Inventory" view, which needs a
/// few more filters that are inherently ticket-level even though the rows
/// returned are order-level. Those (`status`, `section`, and - as of BUG #5 -
/// ticket code within `search`) are applied as a semi-join on `tickets` -
/// "keep this order if it HAS a matching ticket" - so the row's own
/// sold/available/listed/cancelled counts always stay the order's true,
/// complete counts, never a partial/filtered count.
#[allow(clippy::too_many_arguments)]
fn list_orders_impl(
    conn: &Connection,
    search: Option<String>,
    event_id: Option<i64>,
    order_id: Option<i64>,
    supplier_id: Option<i64>,
    platform_id: Option<i64>,
    status: Option<String>,
    section: Option<String>,
    date_from: Option<String>,
    date_to: Option<String>,
    // 2.0.27: appended at the end rather than inserted between existing
    // params, same "every pre-existing call site just gains one trailing
    // None" convention sales.rs's list_sale_groups_impl already documents.
    category_id: Option<i64>,
) -> AppResult<Vec<Order>> {
    let mut sql = format!("{BASE_SQL} WHERE 1=1");
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![];

    if let Some(oid) = order_id {
        sql.push_str(" AND o.id = ?");
        params_vec.push(Box::new(oid));
    }
    if let Some(eid) = event_id {
        sql.push_str(" AND o.event_id = ?");
        params_vec.push(Box::new(eid));
    }
    if let Some(sid) = supplier_id {
        sql.push_str(" AND o.supplier_id = ?");
        params_vec.push(Box::new(sid));
    }
    if let Some(pid) = platform_id {
        sql.push_str(" AND o.platform_id = ?");
        params_vec.push(Box::new(pid));
    }
    if let Some(cid) = category_id {
        sql.push_str(" AND e.category_id = ?");
        params_vec.push(Box::new(cid));
    }
    if let Some(from) = date_from.as_deref() {
        if !from.is_empty() {
            sql.push_str(" AND o.purchase_date >= ?");
            params_vec.push(Box::new(from.to_string()));
        }
    }
    if let Some(to) = date_to.as_deref() {
        if !to.is_empty() {
            sql.push_str(" AND o.purchase_date <= ?");
            params_vec.push(Box::new(to.to_string()));
        }
    }
    if let Some(s) = status.as_deref() {
        if !s.is_empty() {
            // Comma-separated, same convention as list_tickets' own status filter.
            let statuses: Vec<String> = s
                .split(',')
                .map(|x| x.trim().to_string())
                .filter(|x| !x.is_empty())
                .collect();
            if !statuses.is_empty() {
                let placeholders = statuses.iter().map(|_| "?").collect::<Vec<_>>().join(",");
                sql.push_str(&format!(
                    " AND o.id IN (SELECT order_id FROM tickets WHERE status IN ({placeholders}))"
                ));
                for st in statuses {
                    params_vec.push(Box::new(st));
                }
            }
        }
    }
    if let Some(sec) = section.as_deref() {
        let sec = sec.trim();
        if !sec.is_empty() {
            sql.push_str(" AND o.id IN (SELECT order_id FROM tickets WHERE section LIKE ?)");
            params_vec.push(Box::new(format!("%{sec}%")));
        }
    }
    if let Some(q) = search.as_deref() {
        let q = q.trim();
        if !q.is_empty() {
            // BUG #5 fix: ticket code added as one more OR-branch, same
            // parenthesized search group as before - "additive", not a
            // second search system. It has to be a semi-join subquery
            // (like status/section above), not a direct `t.code LIKE ?` on
            // the already-joined `t` - a direct predicate would filter the
            // pre-aggregation rows and corrupt this order's own
            // sold/available/listed/cancelled counts down to just the
            // matching ticket instead of the order's true, complete counts.
            sql.push_str(
                " AND (o.code LIKE ? OR e.name LIKE ? OR sup.name LIKE ? OR p.name LIKE ? OR o.id IN (SELECT order_id FROM tickets WHERE code LIKE ?))",
            );
            let like = format!("%{q}%");
            for _ in 0..5 {
                params_vec.push(Box::new(like.clone()));
            }
        }
    }
    sql.push_str(&format!(" GROUP BY o.id ORDER BY o.purchase_date DESC, o.id DESC LIMIT {LIST_CAP}"));

    let mut stmt = conn.prepare(&sql)?;
    let param_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
    let rows = stmt.query_map(param_refs.as_slice(), map_order)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn list_orders(
    state: State<AppState>,
    search: Option<String>,
    event_id: Option<i64>,
    order_id: Option<i64>,
    supplier_id: Option<i64>,
    platform_id: Option<i64>,
    status: Option<String>,
    section: Option<String>,
    date_from: Option<String>,
    date_to: Option<String>,
    category_id: Option<i64>,
) -> AppResult<Vec<Order>> {
    let conn = state.db.lock().unwrap();
    list_orders_impl(
        &conn, search, event_id, order_id, supplier_id, platform_id, status, section, date_from,
        date_to, category_id,
    )
}

#[tauri::command]
pub fn get_order(state: State<AppState>, id: i64) -> AppResult<Order> {
    let conn = state.db.lock().unwrap();
    fetch_one(&conn, id)
}

/// Sales-side rollup for one order's "ORDER SUMMARY" (Order Detail): revenue,
/// fees and cost only from this order's tickets that are actually sold and
/// NOT refunded - refunded/unsold tickets must never inflate realized
/// revenue/profit. A ticket has at most one `sales` row (ticket_id UNIQUE),
/// so this LEFT JOIN never fans out the ticket count. Kept as its own tiny
/// query (not folded into BASE_SQL) so the main order LIST never pays for
/// this extra join - only Order Detail, opened one order at a time, does.
fn fetch_sales_summary(conn: &Connection, order_id: i64) -> AppResult<OrderSalesSummary> {
    let (revenue_cents, selling_fees_cents, cogs_cents): (i64, i64, i64) = conn.query_row(
        "SELECT
           COALESCE(SUM(sa.sale_price_cents), 0),
           COALESCE(SUM(sa.selling_fees_cents), 0),
           COALESCE(SUM(t.purchase_cost_cents + t.purchase_fees_cents + t.other_costs_cents), 0)
         FROM tickets t
         JOIN sales sa ON sa.ticket_id = t.id AND sa.payment_status != 'refunded'
         WHERE t.order_id = ?1",
        [order_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    let profit_cents = finance::profit_cents(revenue_cents, cogs_cents, selling_fees_cents);
    Ok(OrderSalesSummary {
        revenue_cents,
        selling_fees_cents,
        cogs_cents,
        profit_cents,
        margin: finance::safe_ratio(profit_cents, revenue_cents),
        roi: finance::safe_ratio(profit_cents, cogs_cents),
    })
}

#[tauri::command]
pub fn get_order_sales_summary(state: State<AppState>, id: i64) -> AppResult<OrderSalesSummary> {
    let conn = state.db.lock().unwrap();
    // Give a proper 404 rather than a silent all-zero summary for a bad id.
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM orders WHERE id = ?1)",
        [id],
        |r| r.get(0),
    )?;
    if !exists {
        return Err(AppError::NotFound(format!("Order #{id} not found")));
    }
    fetch_sales_summary(&conn, id)
}

fn validate_order_input(input: &OrderInput) -> AppResult<()> {
    if input.quantity <= 0 {
        return Err(AppError::Validation("Quantity must be at least 1".into()));
    }
    if input.quantity > 50_000 {
        return Err(AppError::Validation("Quantity is unreasonably large".into()));
    }
    if input.unit_price_cents < 0 || input.fees_cents < 0 || input.other_costs_cents < 0 {
        return Err(AppError::Validation("Costs cannot be negative".into()));
    }
    if input.purchase_date.trim().is_empty() {
        return Err(AppError::Validation("Purchase date is required".into()));
    }
    if input.currency.trim().is_empty() {
        return Err(AppError::Validation("Currency is required".into()));
    }
    if let Some(seats) = &input.seats {
        if !seats.is_empty() && seats.len() as i64 != input.quantity {
            return Err(AppError::Validation(format!(
                "You entered {} seat(s) but quantity is {} - provide one seat per ticket or leave seats empty",
                seats.len(),
                input.quantity
            )));
        }
    }
    Ok(())
}

/// Creates an order and generates one ticket row per unit, allocating costs
/// exactly (no floating point) across all generated tickets. Shared by the
/// create_order command, CSV bulk import, and demo data seeding.
pub(crate) fn insert_order_with_tickets(
    conn: &Connection,
    input: &OrderInput,
    is_demo: bool,
) -> AppResult<i64> {
    validate_order_input(input)?;

    let event_exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM events WHERE id = ?1)",
        [input.event_id],
        |r| r.get(0),
    )?;
    if !event_exists {
        return Err(AppError::Validation(format!(
            "Event #{} does not exist",
            input.event_id
        )));
    }

    let total_cost_cents =
        input.unit_price_cents * input.quantity + input.fees_cents + input.other_costs_cents;
    let code = codes::next_code(conn, "order", "ORD")?;
    let payment_status = input
        .payment_status
        .clone()
        .unwrap_or_else(|| "unpaid".to_string());

    conn.execute(
        "INSERT INTO orders (code, event_id, supplier_id, platform_id, purchase_date, quantity,
          unit_price_cents, fees_cents, other_costs_cents, total_cost_cents, currency,
          payment_status, notes, is_demo)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
        params![
            code,
            input.event_id,
            input.supplier_id,
            input.platform_id,
            input.purchase_date,
            input.quantity,
            input.unit_price_cents,
            input.fees_cents,
            input.other_costs_cents,
            total_cost_cents,
            input.currency,
            payment_status,
            input.notes,
            is_demo as i64,
        ],
    )?;
    let order_id = conn.last_insert_rowid();

    let ticket_codes = codes::next_code_batch(conn, "ticket", "TKT", input.quantity)?;
    let fees_alloc = allocate_cents(input.fees_cents, input.quantity);
    let other_alloc = allocate_cents(input.other_costs_cents, input.quantity);

    let mut stmt = conn.prepare(
        "INSERT INTO tickets (code, event_id, order_id, section, row_label, tier, seat, ticket_type,
           purchase_cost_cents, purchase_fees_cents, other_costs_cents, currency, status, is_demo)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,'available',?13)",
    )?;
    let seats = input.seats.as_ref().filter(|s| !s.is_empty());
    for i in 0..input.quantity as usize {
        let seat: Option<&String> = seats.and_then(|s| s.get(i));
        stmt.execute(params![
            ticket_codes[i],
            input.event_id,
            order_id,
            input.section,
            input.row_label,
            input.tier,
            seat,
            input.ticket_type,
            input.unit_price_cents,
            fees_alloc[i],
            other_alloc[i],
            input.currency,
            is_demo as i64,
        ])?;
    }

    Ok(order_id)
}

pub(crate) fn create_order_impl(conn: &Connection, input: &OrderInput) -> AppResult<i64> {
    insert_order_with_tickets(conn, input, false)
}

#[tauri::command]
pub fn create_order(state: State<AppState>, input: OrderInput) -> AppResult<Order> {
    let mut conn = state.db.lock().unwrap();
    let tx = conn.transaction()?;
    let order_id = create_order_impl(&tx, &input)?;
    tx.commit()?;
    fetch_one(&conn, order_id)
}

pub(crate) fn update_order_impl(conn: &Connection, id: i64, input: &OrderEditInput) -> AppResult<Order> {
    if input.purchase_date.trim().is_empty() {
        return Err(AppError::Validation("Purchase date is required".into()));
    }
    let changed = conn.execute(
        "UPDATE orders SET supplier_id=?1, platform_id=?2, purchase_date=?3, currency=?4,
         payment_status=?5, notes=?6, updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now')
         WHERE id=?7",
        params![
            input.supplier_id,
            input.platform_id,
            input.purchase_date,
            input.currency,
            input.payment_status,
            input.notes,
            id,
        ],
    )?;
    if changed == 0 {
        return Err(AppError::NotFound(format!("Order #{id} not found")));
    }
    fetch_one(conn, id)
}

#[tauri::command]
pub fn update_order(state: State<AppState>, id: i64, input: OrderEditInput) -> AppResult<Order> {
    let mut conn = state.db.lock().unwrap();
    let tx = conn.transaction()?;
    let order = update_order_impl(&tx, id, &input)?;
    tx.commit()?;
    Ok(order)
}

// ---------------------------------------------------------------------------
// Convert an EXISTING order's currency to EUR (2.0.51). marko's own request,
// after 2.0.50's "Convert to EUR" on the New Order form turned out to solve
// only part of his real problem: it only ever helps while a NEW order is
// still being typed in, but his actual pain was with orders already sitting
// in the app (imported from Google Sheets, which has no currency-editing UI
// at all - see commands::orders_sheet_sync - or simply created earlier in
// some other currency). See PROTECTED-AREAS-NOTES.md's 2.0.51 section for
// the full investigation this is built from.
//
// The one hard rule this whole feature exists to protect: a ticket's own
// `currency` and its sale's `currency` have ALWAYS matched by construction
// (sales.currency is copied from the ticket at sale-creation time -
// sales.rs's create_sale_impl/create_sales_batch_impl - and nothing before
// this version has ever been able to change one without the other). Several
// existing queries lean on that invariant without checking it explicitly -
// most importantly `fetch_sales_summary` above (Order Detail's Revenue/
// Profit cards), which sums `sale_price_cents` and `purchase_cost_cents` with
// NO currency guard of its own. Converting a ticket's currency WITHOUT also
// converting every sale tied to it, in the very same transaction, would
// silently start blending two currencies' cents together there. Every
// function below exists to make that impossible.
// ---------------------------------------------------------------------------

/// Converts ONE order's currency to `to_currency` (always "EUR" in practice -
/// see the two callers below), atomically, using an already-fetched
/// `rate`/`rate_date`. Deliberately never fetches its own rate - the bulk
/// path (`apply_bulk_currency_conversion`) fetches exactly one rate per
/// distinct SOURCE currency and reuses it across every order in that
/// currency, rather than one live HTTP round trip per order.
///
/// What actually gets converted, in order:
/// 1. Every ticket belonging to this order - `purchase_cost_cents`,
///    `purchase_fees_cents`, `other_costs_cents`, and `listing_price_cents`
///    when it's set.
/// 2. EVERY sale ever recorded against any of those tickets - `sale_price_
///    cents`, `selling_fees_cents` - including refunded/historical ones, not
///    just each ticket's current active sale. A ticket can carry more than
///    one sale over its lifetime (migration 004 - a refunded sale plus a
///    later resale), and every one of them was created with that ticket's
///    currency at the time (always the same currency, since nothing has ever
///    been able to change it before now) - converting only the active sale
///    would leave old refunded rows silently stuck in the previous currency,
///    inconsistent with the ticket they belong to.
/// 3. The order's own aggregate fields (`unit_price_cents`/`fees_cents`/
///    `other_costs_cents`/`total_cost_cents`) - DERIVED from the
///    just-converted ticket values using the exact same formula
///    `insert_order_with_tickets` uses to compute them in the first place
///    (`unit_price_cents * quantity + fees_cents + other_costs_cents`).
///    `fees_cents`/`other_costs_cents` are summed from the independently-
///    rounded per-ticket shares; `unit_price_cents` is read directly off one
///    already-converted ticket row (every ticket's `purchase_cost_cents` is
///    guaranteed identical to the order's own `unit_price_cents` by the
///    guard below, so this is a genuine per-ticket value, not a second,
///    independently-converted copy of the order's old aggregate). None of
///    the four are ever produced by converting the OLD aggregate fields
///    directly: `fees_cents`/`other_costs_cents` were originally split
///    unevenly across tickets by `allocate_cents`, and converting each of
///    those already-rounded shares independently can land a cent away from
///    converting the one combined total directly (see this file's tests for
///    a worked example). Deriving every aggregate from the converted
///    tickets - never the reverse - is what keeps `total_cost_cents` exactly
///    equal to `unit_price_cents * quantity + fees_cents + other_costs_cents`
///    after conversion, the same exact-sum guarantee this app has always had
///    for a brand new order.
///
/// Must run inside a transaction the caller controls - a single commit for
/// the Order Detail button (`convert_order_currency_command_impl`), one
/// transaction PER ORDER for the bulk path, so one bad order in a batch can
/// never leave a DIFFERENT order half-converted.
///
/// Refuses (rather than guesses) in three cases, all checked BEFORE any row
/// is written:
/// - the order is already in `to_currency`;
/// - this order's tickets don't all currently share exactly one currency, or
///   that currency doesn't match the order's own `currency` column;
/// - this order's tickets don't all currently share exactly one purchase
///   cost, or that cost doesn't match the order's own `unit_price_cents`
///   column (see point 3 above - the derivation only makes sense when this
///   holds).
/// Both of the last two should be impossible under normal use (currency and
/// unit price are set once, identically, across an order and every ticket it
/// generates - `insert_order_with_tickets` above - and `OrderEditInput` has
/// no field that could change either independently of tickets afterwards),
/// but Order Detail's Edit Order form has always let `currency` be freely
/// retyped as plain text WITHOUT cascading to any ticket (a pre-existing,
/// deliberately-unfixed quirk - see PROTECTED-AREAS-NOTES.md's 2.0.50
/// section), and a restored or hand-edited backup could in principle violate
/// either invariant too - so neither is actually enforced anywhere before
/// this feature. Converting under a silent mismatch would apply the wrong
/// rate/formula to some tickets and quietly corrupt real financial data;
/// refusing with a clear error is far safer than guessing which value is the
/// "real" one.
pub(crate) fn convert_order_currency_impl(
    conn: &Connection,
    order_id: i64,
    to_currency: &str,
    rate: f64,
    rate_date: &str,
) -> AppResult<OrderCurrencyConversion> {
    let (order_code, from_currency, quantity, old_unit_price_cents): (String, String, i64, i64) = conn
        .query_row(
            "SELECT code, currency, quantity, unit_price_cents FROM orders WHERE id = ?1",
            [order_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .map_err(|_| AppError::NotFound(format!("Order #{order_id} not found")))?;

    if from_currency == to_currency {
        return Err(AppError::Validation(format!(
            "Order {order_code} is already in {to_currency}."
        )));
    }

    let ticket_currencies: Vec<String> = {
        let mut stmt = conn.prepare("SELECT DISTINCT currency FROM tickets WHERE order_id = ?1")?;
        let rows = stmt.query_map([order_id], |r| r.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    if ticket_currencies.len() != 1 || ticket_currencies[0] != from_currency {
        return Err(AppError::Validation(format!(
            "Order {order_code}'s tickets don't all match the order's own currency ({from_currency}) - can't safely convert without risking wrong numbers. This needs fixing by hand first."
        )));
    }

    let ticket_rows: Vec<(i64, i64, i64, i64, Option<i64>)> = {
        let mut stmt = conn.prepare(
            "SELECT id, purchase_cost_cents, purchase_fees_cents, other_costs_cents, listing_price_cents
             FROM tickets WHERE order_id = ?1",
        )?;
        let rows = stmt.query_map([order_id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })?;
        rows.collect::<Result<Vec<_>, _>>()?
    };

    // Second half of this function's cost-consistency guard (see the doc
    // comment above) - every ticket's own purchase_cost_cents must equal the
    // order's own unit_price_cents, so deriving the order's new unit price
    // from any one (already-converted) ticket row below is provably the same
    // value converting the order's old aggregate directly would have given -
    // never a guess. ticket_rows is never empty here: the currency guard
    // above already refused any order with zero tickets (ticket_currencies
    // would have length 0, not 1).
    if ticket_rows.iter().any(|(_, cost, _, _, _)| *cost != old_unit_price_cents) {
        return Err(AppError::Validation(format!(
            "Order {order_code}'s tickets don't all share the same purchase cost as the order's own unit price - can't safely convert without risking wrong numbers. This needs fixing by hand first."
        )));
    }
    // Derived from a ticket's own (not-yet-converted) cost, converted the
    // same way every ticket's cost is converted below - deliberately NOT
    // `fx::convert_cents(old_unit_price_cents, rate)` (converting the order's
    // old aggregate directly), even though the guard above guarantees the
    // two are numerically identical - see point 3 of the doc comment above
    // for why this distinction matters for `fees_cents`/`other_costs_cents`
    // and is kept consistent here for `unit_price_cents` too.
    let new_unit_price_cents = fx::convert_cents(ticket_rows[0].1, rate);

    let mut new_fees_total = 0i64;
    let mut new_other_total = 0i64;
    let mut ticket_ids: Vec<i64> = Vec::with_capacity(ticket_rows.len());
    {
        let mut ticket_stmt = conn.prepare(
            "UPDATE tickets SET purchase_cost_cents=?1, purchase_fees_cents=?2, other_costs_cents=?3,
             listing_price_cents=?4, currency=?5, updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now')
             WHERE id=?6",
        )?;
        for (ticket_id, cost, fees, other, listing) in &ticket_rows {
            let new_cost = fx::convert_cents(*cost, rate);
            let new_fees = fx::convert_cents(*fees, rate);
            let new_other = fx::convert_cents(*other, rate);
            let new_listing = listing.map(|l| fx::convert_cents(l, rate));
            ticket_stmt.execute(params![new_cost, new_fees, new_other, new_listing, to_currency, ticket_id])?;
            new_fees_total += new_fees;
            new_other_total += new_other;
            ticket_ids.push(*ticket_id);
        }
    }

    let mut sales_converted = 0i64;
    if !ticket_ids.is_empty() {
        let placeholders = ticket_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sale_rows: Vec<(i64, i64, i64)> = {
            let sql =
                format!("SELECT id, sale_price_cents, selling_fees_cents FROM sales WHERE ticket_id IN ({placeholders})");
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(rusqlite::params_from_iter(ticket_ids.iter()), |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        };
        let mut sale_stmt = conn.prepare(
            "UPDATE sales SET sale_price_cents=?1, selling_fees_cents=?2, currency=?3,
             updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?4",
        )?;
        for (sale_id, price, sfees) in &sale_rows {
            let new_price = fx::convert_cents(*price, rate);
            let new_sfees = fx::convert_cents(*sfees, rate);
            sale_stmt.execute(params![new_price, new_sfees, to_currency, sale_id])?;
            sales_converted += 1;
        }
    }

    let new_total_cost_cents = new_unit_price_cents * quantity + new_fees_total + new_other_total;
    conn.execute(
        "UPDATE orders SET unit_price_cents=?1, fees_cents=?2, other_costs_cents=?3, total_cost_cents=?4,
         currency=?5, updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?6",
        params![new_unit_price_cents, new_fees_total, new_other_total, new_total_cost_cents, to_currency, order_id],
    )?;

    Ok(OrderCurrencyConversion {
        order_id,
        order_code,
        from_currency,
        to_currency: to_currency.to_string(),
        rate,
        rate_date: rate_date.to_string(),
        tickets_converted: ticket_rows.len() as i64,
        sales_converted,
        // 2.0.53: filled in by the caller after this transaction commits -
        // pushing to a sheet is a network call and must never happen while
        // a DB transaction (this function's own `conn` here is a
        // Transaction, borrowed as &Connection) is still open. See
        // orders_sheet_sync::push_order_currency_to_sheet's own doc comment.
        linked_to_sheet: false,
        sheet_push_error: None,
    })
}

/// Core logic behind the single-order `convert_order_currency` command -
/// split out (same pattern as every other command in this file) so it's
/// directly unit-testable against a plain `&mut Connection`. Fetches ONE live
/// rate (this order's own currency -> EUR - `fx::fetch_rate` short-circuits
/// without a network call when they're already equal, which is exactly the
/// case `convert_order_currency_impl`'s own "already EUR" guard below then
/// reports clearly), converts inside one transaction, and returns the
/// order re-read fresh from the database alongside the conversion summary.
pub(crate) fn convert_order_currency_command_impl(
    conn: &mut Connection,
    id: i64,
) -> AppResult<OrderCurrencyConversionResult> {
    let from_currency: String = conn
        .query_row("SELECT currency FROM orders WHERE id = ?1", [id], |r| r.get(0))
        .map_err(|_| AppError::NotFound(format!("Order #{id} not found")))?;
    let quote = fx::fetch_rate(&from_currency, "EUR")?;
    let tx = conn.transaction()?;
    let conversion = convert_order_currency_impl(&tx, id, "EUR", quote.rate, &quote.date)?;
    tx.commit()?;
    // 2.0.53: only after the transaction above has actually committed - see
    // push_order_currency_to_sheet's own doc comment for why a network call
    // must never happen while that transaction is still open.
    let (linked_to_sheet, sheet_push_error) = crate::commands::orders_sheet_sync::push_order_currency_to_sheet(conn, id);
    let conversion = OrderCurrencyConversion { linked_to_sheet, sheet_push_error, ..conversion };
    let order = fetch_one(conn, id)?;
    Ok(OrderCurrencyConversionResult { order, conversion })
}

/// Order Detail's "Convert to EUR" action, next to the Currency field -
/// visible whenever an order's currency isn't already EUR, regardless of how
/// that order was created (the New Order form, CSV import, or Google Sheets
/// sync - see this section's own doc comment above).
#[tauri::command]
pub fn convert_order_currency(state: State<AppState>, id: i64) -> AppResult<OrderCurrencyConversionResult> {
    let mut conn = state.db.lock().unwrap();
    convert_order_currency_command_impl(&mut conn, id)
}

/// Core bulk-application logic behind `convert_currencies_to_eur` - given
/// already-fetched rates (one per distinct source currency, paired with the
/// order ids to convert in that currency), converts every one of those
/// orders to EUR, each in its OWN transaction so a problem with one order can
/// never block or roll back any other - same per-item philosophy
/// `bulk_delete_orders_impl` already uses for deletion (see
/// `BulkCurrencyConversionResult`'s doc comment, models.rs). Split out from
/// the `#[tauri::command]` wrapper specifically so this - the actual
/// orchestration logic - is testable without a real network call; the
/// wrapper's own job (fetching each currency's real rate) can't be exercised
/// in this dev sandbox at all, same documented limitation as every other
/// live-network path in this app (fx.rs's own doc comment).
pub(crate) fn apply_bulk_currency_conversion(
    conn: &mut Connection,
    order_ids_by_currency: &[(Vec<i64>, fx::RateQuote)],
) -> AppResult<BulkCurrencyConversionResult> {
    let mut converted = Vec::new();
    let mut skipped = Vec::new();
    for (order_ids, quote) in order_ids_by_currency {
        for &order_id in order_ids {
            let tx = conn.transaction()?;
            match convert_order_currency_impl(&tx, order_id, "EUR", quote.rate, &quote.date) {
                Ok(summary) => {
                    tx.commit()?;
                    // 2.0.53: same rule as the single-order path - only
                    // after commit, never inside the transaction above.
                    let (linked_to_sheet, sheet_push_error) =
                        crate::commands::orders_sheet_sync::push_order_currency_to_sheet(conn, order_id);
                    converted.push(OrderCurrencyConversion { linked_to_sheet, sheet_push_error, ..summary });
                }
                // tx is dropped here without commit - an automatic rollback
                // scoped to just this one order, never the ones before or after it.
                Err(e) => skipped.push(BulkDeleteSkip { id: order_id, reason: e.to_string() }),
            }
        }
    }
    Ok(BulkCurrencyConversionResult { converted, skipped })
}

/// Resolves which currencies the Dashboard banner's bulk "Convert to EUR"
/// action should target, and which order ids currently sit in each - the
/// entirely-DB-only half of `convert_currencies_to_eur` below, split out so
/// THIS part (unlike the live `fx::fetch_rate` call per currency) is directly
/// unit-testable against a plain `&Connection` - same "impl carries the
/// testable logic, the command adds the untestable network bit" split as
/// every other rate-fetching entry point in this file.
///
/// `currencies`: the caller's own explicit list (trimmed, uppercased, EUR
/// filtered out - so accidentally passing "eur" is just a no-op, never an
/// error), or `None`/empty for marko's own "alebo vsetky", "or all" option -
/// every non-EUR currency actually present on any order right now.
///
/// Either way, every match against `orders.currency` is done case/
/// whitespace-insensitively (`UPPER(TRIM(currency))`), both for the "which
/// currencies exist" query and the "which orders are in this currency" one.
/// This app has never normalized that column's casing at write time - CSV
/// import stores a cell's raw text verbatim (see csv_import.rs), unlike
/// Sheets sync, which does normalize - so e.g. a CSV-imported "usd" order is
/// real, storable data. A caller-supplied "USD" (typed by hand, or read
/// straight off `non_eur_order_currencies`/dashboard.rs, which itself
/// reports the raw stored casing) must still find it, not silently match
/// zero rows. The actual `UPDATE` inside `convert_order_currency_impl`
/// always writes a clean literal "EUR" regardless, so every order converted
/// through here ends up normalized going forward even though how it got
/// FOUND had to tolerate whatever casing it started in. Only orders that
/// actually have at least one match are included - never an empty entry for
/// a currency nothing is left in.
pub(crate) fn resolve_currency_order_ids(
    conn: &Connection,
    currencies: &Option<Vec<String>>,
) -> AppResult<Vec<(String, Vec<i64>)>> {
    let target_currencies: Vec<String> = match currencies {
        Some(list) if !list.is_empty() => list
            .iter()
            .map(|c| c.trim().to_uppercase())
            .filter(|c| c != "EUR")
            .collect(),
        _ => {
            let mut stmt = conn
                .prepare("SELECT DISTINCT UPPER(TRIM(currency)) FROM orders WHERE currency != 'EUR' ORDER BY 1")?;
            let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
            rows.collect::<Result<Vec<_>, _>>()?
        }
    };

    let mut result = Vec::with_capacity(target_currencies.len());
    for currency in target_currencies {
        let order_ids: Vec<i64> = {
            let mut stmt = conn.prepare("SELECT id FROM orders WHERE UPPER(TRIM(currency)) = ?1 ORDER BY id")?;
            let rows = stmt.query_map([&currency], |r| r.get::<_, i64>(0))?;
            rows.collect::<Result<Vec<_>, _>>()?
        };
        if !order_ids.is_empty() {
            result.push((currency, order_ids));
        }
    }
    Ok(result)
}

/// The Dashboard mixed-currency banner's bulk "Convert to EUR" action -
/// converts every order in `currencies` (or every non-EUR order at all when
/// `currencies` is `None`/empty - marko's own "alebo vsetky", "or all"
/// option) to EUR. Fetches exactly one live rate per distinct currency
/// actually present among target orders, never one per order, then applies
/// them via `apply_bulk_currency_conversion` above.
#[tauri::command]
pub fn convert_currencies_to_eur(
    state: State<AppState>,
    currencies: Option<Vec<String>>,
) -> AppResult<BulkCurrencyConversionResult> {
    let mut conn = state.db.lock().unwrap();

    let currency_order_ids = resolve_currency_order_ids(&conn, &currencies)?;

    let mut order_ids_by_currency: Vec<(Vec<i64>, fx::RateQuote)> = Vec::new();
    let mut rate_fetch_skips: Vec<BulkDeleteSkip> = Vec::new();
    for (currency, order_ids) in currency_order_ids {
        match fx::fetch_rate(&currency, "EUR") {
            Ok(quote) => order_ids_by_currency.push((order_ids, quote)),
            Err(e) => {
                // The rate lookup itself failed - every order in this
                // currency is skipped for the same reason, not judged
                // individually (there's nothing order-specific to judge yet).
                for id in order_ids {
                    rate_fetch_skips.push(BulkDeleteSkip {
                        id,
                        reason: format!("Could not fetch a {currency} -> EUR rate: {e}"),
                    });
                }
            }
        }
    }

    let mut result = apply_bulk_currency_conversion(&mut conn, &order_ids_by_currency)?;
    result.skipped.splice(0..0, rate_fetch_skips);
    Ok(result)
}

/// Returns `Some(reason)` if order `id` cannot be safely deleted (it has
/// sold tickets, or any sales history at all - including a refunded one),
/// `None` if it's safe. Split out of `delete_order_impl` in 2.0.28 so
/// `bulk_delete_orders_impl` enforces EXACTLY the same rule, word-for-word,
/// as deleting one order at a time from Order Detail always has - the two
/// paths can never drift on what "safe to delete" means. Deliberately
/// doesn't check whether the order itself exists: the two callers want
/// different behavior for a missing id (single delete errors immediately;
/// bulk delete records it as one skip among possibly many and keeps going),
/// so that check stays with each caller.
fn order_delete_blocker(conn: &Connection, id: i64) -> AppResult<Option<String>> {
    let sold_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tickets WHERE order_id = ?1 AND status = 'sold'",
        [id],
        |r| r.get(0),
    )?;
    if sold_count > 0 {
        return Ok(Some("This order has sold tickets and cannot be deleted.".into()));
    }
    // A refunded sale is no longer "sold" (the ticket already returned to
    // available), but any sales row still on record - including a refunded
    // one - blocks the order until it's gone, with its own clear message.
    // (Sale Detail now allows deleting a refunded sale directly - once that's
    // done this count drops and the order becomes deletable on its own,
    // nothing here needs to change for that.) Without this check at all, the
    // DB's own foreign key (sales.ticket_id -> RESTRICT) would still stop the
    // delete, but only with a generic "blocked by other records" error
    // instead of telling the user why.
    let sale_history_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sales s JOIN tickets t ON t.id = s.ticket_id WHERE t.order_id = ?1",
        [id],
        |r| r.get(0),
    )?;
    if sale_history_count > 0 {
        return Ok(Some(
            "This order has sales history (including refunds) and cannot be deleted.".into(),
        ));
    }
    Ok(None)
}

fn delete_order_impl(conn: &Connection, id: i64) -> AppResult<()> {
    if let Some(reason) = order_delete_blocker(conn, id)? {
        return Err(AppError::Validation(reason));
    }
    let changed = conn.execute("DELETE FROM orders WHERE id = ?1", [id])?;
    if changed == 0 {
        return Err(AppError::NotFound(format!("Order #{id} not found")));
    }
    Ok(())
}

#[tauri::command]
pub fn delete_order(state: State<AppState>, id: i64) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    delete_order_impl(&conn, id)
}

/// 2.0.28: bulk delete for the new "Delete" selection mode on the Orders
/// list. See `models::BulkDeleteResult`'s doc comment for why this uses a
/// per-id skip-with-reason model instead of the codebase's usual
/// all-or-nothing bulk-write pattern: everything that passes
/// `order_delete_blocker` is removed together in one transaction, and
/// anything that doesn't is reported back with the exact same message
/// `delete_order`/Order Detail already show for that same order, one at a
/// time.
pub(crate) fn bulk_delete_orders_impl(conn: &mut Connection, ids: &[i64]) -> AppResult<BulkDeleteResult> {
    if ids.is_empty() {
        return Err(AppError::Validation("Select at least one order to delete".into()));
    }
    let tx = conn.transaction()?;
    let mut deleted_ids = Vec::new();
    let mut skipped = Vec::new();
    for &id in ids {
        if let Some(reason) = order_delete_blocker(&tx, id)? {
            skipped.push(BulkDeleteSkip { id, reason });
            continue;
        }
        let changed = tx.execute("DELETE FROM orders WHERE id = ?1", [id])?;
        if changed > 0 {
            deleted_ids.push(id);
        } else {
            skipped.push(BulkDeleteSkip {
                id,
                reason: "Not found - already deleted?".into(),
            });
        }
    }
    tx.commit()?;
    Ok(BulkDeleteResult { deleted_ids, skipped })
}

#[tauri::command]
pub fn bulk_delete_orders(state: State<AppState>, ids: Vec<i64>) -> AppResult<BulkDeleteResult> {
    let mut conn = state.db.lock().unwrap();
    bulk_delete_orders_impl(&mut conn, &ids)
}

/// 2.0.67: resolves selected Orders-list order ids down to the ticket ids
/// eligible for a bulk delivery-status change - only tickets that are
/// actually `status='sold'` (an order can freely mix sold/available/listed/
/// cancelled tickets; delivery status only makes sense once a ticket has
/// actually been sold to someone). Used by the new bulk 'Mark Delivered/Not
/// delivered' action on the Orders list (see BulkCompletionBar.tsx) so that
/// selecting orders with some not-yet-sold tickets only ever touches the
/// sold ones - exactly like `orderCompletionChecks` (Orders.tsx) already
/// only judges 'Delivered'/'Paid' against sold tickets, never the whole
/// order.
pub(crate) fn resolve_orders_sold_ticket_ids(conn: &Connection, order_ids: &[i64]) -> AppResult<Vec<i64>> {
    if order_ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = order_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!("SELECT id FROM tickets WHERE order_id IN ({placeholders}) AND status='sold'");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(order_ids.iter()), |r| r.get::<_, i64>(0))?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Same idea as `resolve_orders_sold_ticket_ids` right above, but resolves to
/// each sold ticket's CURRENT sale id instead - the same `payment_status !=
/// 'refunded'` join `fetch_sales_summary`/BASE_SQL's own `sa` join already
/// use (a ticket has at most one non-refunded sale at a time - see migration
/// 004's partial unique index). A sold ticket whose only sale was later
/// refunded (see `refund_sale_impl` in sales.rs) contributes no id here at
/// all, so the bulk 'Mark Paid/Pending' action below can never resurrect a
/// refunded sale's payment_status just because its ticket happened to be
/// selected via its order.
pub(crate) fn resolve_orders_active_sale_ids(conn: &Connection, order_ids: &[i64]) -> AppResult<Vec<i64>> {
    if order_ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = order_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT sa.id FROM sales sa JOIN tickets t ON t.id = sa.ticket_id \
         WHERE t.order_id IN ({placeholders}) AND sa.payment_status != 'refunded'"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(order_ids.iter()), |r| r.get::<_, i64>(0))?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Core logic behind the Orders-list bulk 'Mark Delivered/Not delivered'
/// action (2.0.67): resolves the selected orders down to their sold tickets
/// (see `resolve_orders_sold_ticket_ids`) and delegates the actual write to
/// `tickets::bulk_update_ticket_delivery_status_impl` - the exact same write
/// path the Sales-list bulk action (`commands::sales::
/// bulk_set_sale_groups_delivery_status_impl`) also delegates to, so there is
/// only ever one place that writes `tickets.delivery_status` in bulk.
/// Selecting orders with zero sold tickets is a harmless no-op (`Ok(0)`), not
/// an error - marking a whole page of freshly-created, nothing-sold-yet
/// orders as 'Delivered' should simply do nothing rather than fail the
/// entire selection.
pub(crate) fn bulk_set_orders_delivery_status_impl(
    conn: &mut Connection,
    order_ids: &[i64],
    delivery_status: &str,
) -> AppResult<usize> {
    let ticket_ids = resolve_orders_sold_ticket_ids(conn, order_ids)?;
    if ticket_ids.is_empty() {
        return Ok(0);
    }
    let updated =
        crate::commands::tickets::bulk_update_ticket_delivery_status_impl(conn, &ticket_ids, delivery_status)?;
    Ok(updated.len())
}

/// Sets `tickets.delivery_status` for every SOLD ticket across the selected
/// orders at once - the Orders-list equivalent of Order Detail's own
/// ticket-status bulk bar, next to the list's existing selection checkboxes
/// (same ones bulk-delete already uses). Returns how many tickets were
/// actually changed, so the frontend can show "N tickets marked Delivered"
/// even though the selection itself was made in terms of whole orders.
#[tauri::command]
pub fn bulk_set_orders_delivery_status(
    state: State<AppState>,
    input: BulkOrdersDeliveryStatusInput,
) -> AppResult<usize> {
    let mut conn = state.db.lock().unwrap();
    bulk_set_orders_delivery_status_impl(&mut conn, &input.order_ids, &input.delivery_status)
}

/// Core logic behind the Orders-list bulk 'Mark Paid/Pending' action
/// (2.0.67): resolves the selected orders down to their current
/// (non-refunded) sale ids (see `resolve_orders_active_sale_ids`) and
/// delegates to the existing `sales::bulk_update_sale_payment_status_impl` -
/// the SAME primitive Sale Detail's own bulk Paid/Pending action already
/// uses, so 'paid' means exactly one thing everywhere in this app. Orders
/// with nothing currently sold (or whose only sale was refunded) resolve to
/// zero ids and are a harmless no-op (`Ok(0)`).
pub(crate) fn bulk_set_orders_payment_status_impl(
    conn: &mut Connection,
    order_ids: &[i64],
    payment_status: &str,
) -> AppResult<usize> {
    let sale_ids = resolve_orders_active_sale_ids(conn, order_ids)?;
    if sale_ids.is_empty() {
        return Ok(0);
    }
    let updated = crate::commands::sales::bulk_update_sale_payment_status_impl(conn, &sale_ids, payment_status)?;
    Ok(updated.len())
}

/// Sets `sales.payment_status` (pending/paid only - refunding stays its own
/// dedicated action, see `BulkOrdersPaymentStatusInput`'s doc comment) for
/// every currently-sold ticket's sale across the selected orders at once.
/// Returns how many sales were actually changed.
#[tauri::command]
pub fn bulk_set_orders_payment_status(
    state: State<AppState>,
    input: BulkOrdersPaymentStatusInput,
) -> AppResult<usize> {
    let mut conn = state.db.lock().unwrap();
    bulk_set_orders_payment_status_impl(&mut conn, &input.order_ids, &input.payment_status)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_conn;

    fn seed_event(conn: &Connection) -> i64 {
        conn.execute("INSERT INTO events (name) VALUES ('Test Event')", [])
            .unwrap();
        conn.last_insert_rowid()
    }

    fn base_input(event_id: i64, quantity: i64) -> OrderInput {
        OrderInput {
            event_id,
            supplier_id: None,
            platform_id: None,
            purchase_date: "2026-01-01".to_string(),
            quantity,
            unit_price_cents: 1000,
            fees_cents: 100,
            other_costs_cents: 50,
            currency: "EUR".to_string(),
            payment_status: Some("paid".to_string()),
            notes: None,
            ticket_type: None,
            section: Some("A".to_string()),
            row_label: Some("12".to_string()),
            tier: None,
            seats: None,
        }
    }

    #[test]
    fn seats_are_assigned_one_per_ticket_in_order() {
        let conn = test_conn();
        let event_id = seed_event(&conn);
        let mut input = base_input(event_id, 4);
        input.seats = Some(vec!["11".into(), "12".into(), "13".into(), "14".into()]);

        let order_id = insert_order_with_tickets(&conn, &input, false).unwrap();

        let mut stmt = conn
            .prepare("SELECT seat FROM tickets WHERE order_id = ?1 ORDER BY id")
            .unwrap();
        let seats: Vec<Option<String>> = stmt
            .query_map([order_id], |r| r.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            seats,
            vec![
                Some("11".to_string()),
                Some("12".to_string()),
                Some("13".to_string()),
                Some("14".to_string())
            ]
        );
    }

    #[test]
    fn tier_is_copied_onto_every_generated_ticket() {
        // 2.2.7: `OrderInput.tier` - same "set once at order creation,
        // copied onto every generated ticket" convention already used by
        // section/row_label/ticket_type.
        let conn = test_conn();
        let event_id = seed_event(&conn);
        let mut input = base_input(event_id, 3);
        input.tier = Some("VIP".to_string());

        let order_id = insert_order_with_tickets(&conn, &input, false).unwrap();

        let tiers: Vec<Option<String>> = conn
            .prepare("SELECT tier FROM tickets WHERE order_id = ?1 ORDER BY id")
            .unwrap()
            .query_map([order_id], |r| r.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(tiers, vec![Some("VIP".to_string()); 3]);
    }

    #[test]
    fn a_null_tier_at_order_creation_leaves_every_generated_ticket_with_no_tier() {
        // marko's explicit instruction: existing/untouched tickets get
        // NULL/empty, never a fabricated value - `base_input`'s own default
        // is already `tier: None`, this just makes that guarantee explicit.
        let conn = test_conn();
        let event_id = seed_event(&conn);
        let input = base_input(event_id, 2);
        let order_id = insert_order_with_tickets(&conn, &input, false).unwrap();
        let tiers: Vec<Option<String>> = conn
            .prepare("SELECT tier FROM tickets WHERE order_id = ?1 ORDER BY id")
            .unwrap()
            .query_map([order_id], |r| r.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(tiers, vec![None, None]);
    }

    #[test]
    fn seat_count_must_match_quantity() {
        let conn = test_conn();
        let event_id = seed_event(&conn);
        let mut input = base_input(event_id, 4);
        input.seats = Some(vec!["11".into(), "12".into(), "13".into()]); // only 3 for qty 4

        let err = insert_order_with_tickets(&conn, &input, false).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));

        // Nothing should have been written - order creation is all-or-nothing.
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM orders", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn empty_seats_still_valid() {
        let conn = test_conn();
        let event_id = seed_event(&conn);
        let input = base_input(event_id, 3); // seats: None

        let order_id = insert_order_with_tickets(&conn, &input, false).unwrap();
        let ticket_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM tickets WHERE order_id = ?1",
                [order_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(ticket_count, 3);
    }

    // 2.0.38: exercises the REAL SQL (GROUP_CONCAT + char(30)/char(31), added
    // to BASE_SQL for the new Order.seats field) against a real connection -
    // not just SeatEntry::parse_aggregate's own pure-Rust unit tests
    // (models.rs), which can't catch a SQL syntax/runtime mistake since the
    // query is just a string literal as far as the Rust compiler is
    // concerned. Compares as a HashSet (order-independent) deliberately -
    // GROUP_CONCAT's row order is not something this app relies on or should
    // couple a test to; the frontend does its own sorting for display.
    #[test]
    fn fetch_one_returns_every_tickets_seat_via_the_real_group_concat_aggregate() {
        let conn = test_conn();
        let event_id = seed_event(&conn);
        let mut input = base_input(event_id, 4);
        input.seats = Some(vec!["11".into(), "12".into(), "13".into(), "14".into()]);

        let order_id = insert_order_with_tickets(&conn, &input, false).unwrap();
        let order = fetch_one(&conn, order_id).unwrap();

        let got: std::collections::HashSet<_> = order.seats.into_iter().collect();
        let expected: std::collections::HashSet<_> = ["11", "12", "13", "14"]
            .into_iter()
            .map(|seat| crate::models::SeatEntry {
                section: Some("A".to_string()),
                row_label: Some("12".to_string()),
                seat: Some(seat.to_string()),
            })
            .collect();
        assert_eq!(got, expected);
    }

    #[test]
    fn fetch_one_seats_is_general_admission_shaped_when_the_order_has_no_seat_fields() {
        let conn = test_conn();
        let event_id = seed_event(&conn);
        let mut input = base_input(event_id, 2);
        input.section = None;
        input.row_label = None;
        // seats already None from base_input.

        let order_id = insert_order_with_tickets(&conn, &input, false).unwrap();
        let order = fetch_one(&conn, order_id).unwrap();

        assert_eq!(
            order.seats,
            vec![crate::models::SeatEntry { section: None, row_label: None, seat: None }],
            "2 tickets with identical (all-None) seat info collapse to exactly one General-admission-shaped entry"
        );
    }

    #[test]
    fn fees_and_costs_allocate_exactly_across_tickets() {
        let conn = test_conn();
        let event_id = seed_event(&conn);
        let mut input = base_input(event_id, 3);
        input.fees_cents = 100; // does not divide evenly by 3
        input.other_costs_cents = 0;

        let order_id = insert_order_with_tickets(&conn, &input, false).unwrap();
        let total_fees: i64 = conn
            .query_row(
                "SELECT SUM(purchase_fees_cents) FROM tickets WHERE order_id = ?1",
                [order_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(total_fees, 100, "allocated fees must sum back exactly, no cent lost to rounding");
    }

    #[test]
    fn delete_blocked_by_sale_history_even_after_refund() {
        let conn = test_conn();
        let event_id = seed_event(&conn);
        let input = base_input(event_id, 1);
        let order_id = insert_order_with_tickets(&conn, &input, false).unwrap();
        let ticket_id: i64 = conn
            .query_row(
                "SELECT id FROM tickets WHERE order_id = ?1",
                [order_id],
                |r| r.get(0),
            )
            .unwrap();

        // Simulate a sale that was later refunded: ticket back to
        // 'available', but the sales row (history) still exists.
        conn.execute(
            "INSERT INTO sales (code, ticket_id, sale_date, sale_price_cents, payment_status)
             VALUES ('SAL-000001', ?1, '2026-02-01', 1500, 'refunded')",
            [ticket_id],
        )
        .unwrap();
        conn.execute("UPDATE tickets SET status='available' WHERE id=?1", [ticket_id])
            .unwrap();

        let err = delete_order_impl(&conn, order_id).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));

        let still_there: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM orders WHERE id = ?1",
                [order_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(still_there, 1, "order must survive - it has sale history");
    }

    #[test]
    fn delete_allowed_once_no_tickets_and_no_sale_history() {
        let conn = test_conn();
        let event_id = seed_event(&conn);
        let input = base_input(event_id, 2); // never sold
        let order_id = insert_order_with_tickets(&conn, &input, false).unwrap();

        delete_order_impl(&conn, order_id).unwrap();

        let still_there: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM orders WHERE id = ?1",
                [order_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(still_there, 0);
    }

    // ---- bulk delete (2.0.28) ---------------------------------------------

    #[test]
    fn bulk_delete_orders_removes_every_selected_safe_order() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn);
        let order_a = insert_order_with_tickets(&conn, &base_input(event_id, 1), false).unwrap();
        let order_b = insert_order_with_tickets(&conn, &base_input(event_id, 1), false).unwrap();
        let order_c = insert_order_with_tickets(&conn, &base_input(event_id, 1), false).unwrap();

        let result = bulk_delete_orders_impl(&mut conn, &[order_a, order_b]).unwrap();

        assert_eq!(result.deleted_ids, vec![order_a, order_b]);
        assert!(result.skipped.is_empty());
        let remaining: i64 = conn.query_row("SELECT COUNT(*) FROM orders", [], |r| r.get(0)).unwrap();
        assert_eq!(remaining, 1, "only the unselected order_c should be left");
        let c_still_there: i64 = conn
            .query_row("SELECT COUNT(*) FROM orders WHERE id = ?1", [order_c], |r| r.get(0))
            .unwrap();
        assert_eq!(c_still_there, 1);
    }

    #[test]
    fn bulk_delete_orders_skips_one_with_sale_history_but_still_deletes_the_rest() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn);

        // A safe order - never sold.
        let safe_order = insert_order_with_tickets(&conn, &base_input(event_id, 1), false).unwrap();

        // An order blocked by sale history, same setup as
        // delete_blocked_by_sale_history_even_after_refund above.
        let blocked_order = insert_order_with_tickets(&conn, &base_input(event_id, 1), false).unwrap();
        let ticket_id: i64 = conn
            .query_row("SELECT id FROM tickets WHERE order_id = ?1", [blocked_order], |r| r.get(0))
            .unwrap();
        conn.execute(
            "INSERT INTO sales (code, ticket_id, sale_date, sale_price_cents, payment_status)
             VALUES ('SAL-000001', ?1, '2026-02-01', 1500, 'refunded')",
            [ticket_id],
        )
        .unwrap();
        conn.execute("UPDATE tickets SET status='available' WHERE id=?1", [ticket_id])
            .unwrap();

        let result = bulk_delete_orders_impl(&mut conn, &[safe_order, blocked_order]).unwrap();

        assert_eq!(result.deleted_ids, vec![safe_order], "the safe order must still go through");
        assert_eq!(result.skipped.len(), 1);
        assert_eq!(result.skipped[0].id, blocked_order);
        assert!(result.skipped[0].reason.contains("sales history"));

        let blocked_still_there: i64 = conn
            .query_row("SELECT COUNT(*) FROM orders WHERE id = ?1", [blocked_order], |r| r.get(0))
            .unwrap();
        assert_eq!(blocked_still_there, 1, "the blocked order must survive, not be partially touched");
    }

    #[test]
    fn bulk_delete_orders_rejects_an_empty_selection() {
        let mut conn = test_conn();
        let err = bulk_delete_orders_impl(&mut conn, &[]).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    // ---- Convert order currency to EUR (2.0.51) ----------------------------
    // marko's own expanded request: Sheets-imported orders had NO way to
    // change currency at all, and even manually-created orders could only be
    // converted at CREATION time (2.0.50), never afterward. These tests cover
    // `convert_order_currency_impl` (the core per-order conversion, given an
    // already-fetched rate) and `apply_bulk_currency_conversion` (the
    // per-order-transaction bulk path) - both fully offline/deterministic,
    // since neither ever calls `fx::fetch_rate` itself. The two
    // network-touching entry points (`convert_order_currency_command_impl`,
    // `convert_currencies_to_eur`) are exercised only for the specific paths
    // that never actually reach the network (same-currency short-circuit,
    // not-found) - see fx.rs's own doc comment for why a real rate lookup
    // can't be exercised in this sandbox.

    #[test]
    fn convert_order_currency_converts_ticket_and_order_amounts_and_flips_currency() {
        let conn = test_conn();
        let event_id = seed_event(&conn);
        let mut input = base_input(event_id, 1);
        input.currency = "GBP".to_string();
        input.unit_price_cents = 2000;
        input.fees_cents = 100;
        input.other_costs_cents = 50;
        let order_id = insert_order_with_tickets(&conn, &input, false).unwrap();
        let ticket_id = ticket_ids(&conn, order_id)[0];
        conn.execute(
            "UPDATE tickets SET listing_price_cents=?1 WHERE id=?2",
            params![3000, ticket_id],
        )
        .unwrap();

        // marko's own 2.0.50 report example rate (20 GBP -> 23.38 EUR).
        let rate = 1.1689_f64;
        let result = convert_order_currency_impl(&conn, order_id, "EUR", rate, "2026-08-25").unwrap();

        assert_eq!(result.order_id, order_id);
        assert_eq!(result.from_currency, "GBP");
        assert_eq!(result.to_currency, "EUR");
        assert_eq!(result.rate, rate);
        assert_eq!(result.rate_date, "2026-08-25");
        assert_eq!(result.tickets_converted, 1);
        assert_eq!(result.sales_converted, 0, "this ticket has no sales at all");

        let (cost, fees, other, listing, currency): (i64, i64, i64, Option<i64>, String) = conn
            .query_row(
                "SELECT purchase_cost_cents, purchase_fees_cents, other_costs_cents, listing_price_cents, currency
                 FROM tickets WHERE id=?1",
                [ticket_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .unwrap();
        assert_eq!(cost, fx::convert_cents(2000, rate));
        assert_eq!(fees, fx::convert_cents(100, rate));
        assert_eq!(other, fx::convert_cents(50, rate));
        assert_eq!(listing, Some(fx::convert_cents(3000, rate)));
        assert_eq!(currency, "EUR");

        let order = fetch_one(&conn, order_id).unwrap();
        assert_eq!(order.currency, "EUR");
        assert_eq!(order.unit_price_cents, fx::convert_cents(2000, rate));
        assert_eq!(order.fees_cents, fx::convert_cents(100, rate));
        assert_eq!(order.other_costs_cents, fx::convert_cents(50, rate));
        assert_eq!(
            order.total_cost_cents,
            order.unit_price_cents * order.quantity + order.fees_cents + order.other_costs_cents,
            "the exact-sum invariant (total = unit*qty + fees + other) must survive conversion, \
             same as it holds for a brand new order"
        );
    }

    /// The core proof behind this feature's most important design decision:
    /// order-level fees/other-costs are DERIVED by summing each already-
    /// converted ticket's own share, never by converting the old order-level
    /// total directly - because `allocate_cents` can split a total unevenly
    /// across tickets, and independently rounding each uneven share can land
    /// a cent away from rounding the combined total in one shot. This test
    /// picks a total/rate pair where the two approaches provably disagree, so
    /// the assertion below is a real proof, not a tautology.
    #[test]
    fn order_level_totals_are_derived_from_summed_converted_tickets_not_from_converting_the_old_total_directly() {
        let conn = test_conn();
        let event_id = seed_event(&conn);
        let mut input = base_input(event_id, 3);
        input.currency = "GBP".to_string();
        input.unit_price_cents = 1000;
        input.fees_cents = 150; // allocate_cents(150, 3) = [50, 50, 50] - evenly split on purpose
        input.other_costs_cents = 0;
        let order_id = insert_order_with_tickets(&conn, &input, false).unwrap();

        let rate = 0.85_f64;
        // Sanity-check the premise, verified against the real `convert_cents`
        // rather than hand-derived (an earlier draft of this test hand-picked
        // a rate assuming plain decimal rounding and got it wrong - f64
        // arithmetic doesn't represent every rate exactly). Even though the
        // 150-cent fee total splits EVENLY across 3 tickets (50 each - no
        // `allocate_cents` unevenness involved at all), each 50-cent share
        // lands exactly on a rounding half-way point (50 * 0.85 = 42.5) and
        // `convert_cents` rounds every one of those three ties UP (away from
        // zero) to 43 - summing to 129. Converting the combined 150-cent
        // total directly hits its OWN half-way point once (150 * 0.85 =
        // 127.5) and rounds up ONCE, to 128. Applying the same "round the
        // tie up" bump three times instead of once is exactly the kind of
        // divergence the derive-from-tickets approach exists to avoid. If
        // this assertion ever stopped holding (e.g. `convert_cents`'s
        // rounding rule changed), the rest of this test would no longer
        // actually be proving anything.
        assert_eq!(fx::convert_cents(150, rate), 128, "direct-total conversion rounds down to 128");

        convert_order_currency_impl(&conn, order_id, "EUR", rate, "2026-08-25").unwrap();

        let order = fetch_one(&conn, order_id).unwrap();
        assert_eq!(
            order.fees_cents, 129,
            "must equal the SUM of each independently-converted per-ticket fee (43+43+43=129), not round(150 * 0.85) = 128"
        );

        let (tickets_cost_sum, tickets_fees_sum, tickets_other_sum): (i64, i64, i64) = conn
            .query_row(
                "SELECT SUM(purchase_cost_cents), SUM(purchase_fees_cents), SUM(other_costs_cents)
                 FROM tickets WHERE order_id=?1",
                [order_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(tickets_fees_sum, order.fees_cents, "order.fees_cents must equal the tickets' own sum, always");
        assert_eq!(order.unit_price_cents * order.quantity, tickets_cost_sum);
        assert_eq!(order.other_costs_cents, tickets_other_sum);
        assert_eq!(
            order.total_cost_cents,
            tickets_cost_sum + tickets_fees_sum + tickets_other_sum,
            "order.total_cost_cents must equal the tickets' own combined cost+fees+other, exactly"
        );
    }

    #[test]
    fn convert_order_currency_converts_every_sale_on_a_ticket_including_refunded_history() {
        let conn = test_conn();
        let event_id = seed_event(&conn);
        let mut input = base_input(event_id, 1);
        input.currency = "GBP".to_string();
        let order_id = insert_order_with_tickets(&conn, &input, false).unwrap();
        let ticket_id = ticket_ids(&conn, order_id)[0];

        // A refunded sale (history) plus a later active resale on the SAME
        // ticket - migration 004 allows this (only ACTIVE sales are unique
        // per ticket). Both share the ticket's GBP currency at the time each
        // was created, exactly like create_sale_impl would produce.
        conn.execute(
            "INSERT INTO sales (code, ticket_id, sale_date, sale_price_cents, selling_fees_cents, currency, payment_status)
             VALUES ('SAL-000001', ?1, '2026-02-01', 1500, 50, 'GBP', 'refunded')",
            [ticket_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO sales (code, ticket_id, sale_date, sale_price_cents, selling_fees_cents, currency, payment_status)
             VALUES ('SAL-000002', ?1, '2026-03-01', 1800, 60, 'GBP', 'paid')",
            [ticket_id],
        )
        .unwrap();

        let rate = 1.2_f64;
        let result = convert_order_currency_impl(&conn, order_id, "EUR", rate, "2026-08-25").unwrap();
        assert_eq!(result.sales_converted, 2, "both the refunded AND the active sale must convert");

        let mut stmt = conn
            .prepare("SELECT sale_price_cents, selling_fees_cents, currency FROM sales WHERE ticket_id=?1 ORDER BY code")
            .unwrap();
        let rows: Vec<(i64, i64, String)> = stmt
            .query_map([ticket_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(
            rows,
            vec![
                (fx::convert_cents(1500, rate), fx::convert_cents(50, rate), "EUR".to_string()),
                (fx::convert_cents(1800, rate), fx::convert_cents(60, rate), "EUR".to_string()),
            ]
        );
    }

    #[test]
    fn convert_order_currency_leaves_a_ticket_with_no_listing_price_as_none_not_zero() {
        let conn = test_conn();
        let event_id = seed_event(&conn);
        let mut input = base_input(event_id, 1);
        input.currency = "USD".to_string();
        let order_id = insert_order_with_tickets(&conn, &input, false).unwrap();
        let ticket_id = ticket_ids(&conn, order_id)[0];
        // listing_price_cents is NULL by default here - never set.

        convert_order_currency_impl(&conn, order_id, "EUR", 0.92, "2026-08-25").unwrap();

        let listing: Option<i64> = conn
            .query_row("SELECT listing_price_cents FROM tickets WHERE id=?1", [ticket_id], |r| r.get(0))
            .unwrap();
        assert_eq!(listing, None, "no listing price before conversion must mean no listing price after, not Some(0)");
    }

    #[test]
    fn refuses_to_convert_when_orders_currency_column_does_not_match_its_tickets() {
        let conn = test_conn();
        let event_id = seed_event(&conn);
        let mut input = base_input(event_id, 1);
        input.currency = "GBP".to_string();
        let order_id = insert_order_with_tickets(&conn, &input, false).unwrap();
        let ticket_id = ticket_ids(&conn, order_id)[0];
        let (cost_before, currency_before): (i64, String) = conn
            .query_row("SELECT purchase_cost_cents, currency FROM tickets WHERE id=?1", [ticket_id], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();

        // Simulates the pre-existing Edit Order free-text Currency field,
        // which updates ONLY orders.currency and never cascades to tickets -
        // see convert_order_currency_impl's own doc comment for why this
        // must be refused rather than guessed at.
        conn.execute("UPDATE orders SET currency='USD' WHERE id=?1", [order_id]).unwrap();

        let err = convert_order_currency_impl(&conn, order_id, "EUR", 0.92, "2026-08-25").unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));

        let (cost_after, currency_after): (i64, String) = conn
            .query_row("SELECT purchase_cost_cents, currency FROM tickets WHERE id=?1", [ticket_id], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(cost_after, cost_before, "a refused conversion must leave the ticket completely untouched");
        assert_eq!(currency_after, currency_before);
    }

    #[test]
    fn refuses_to_convert_when_two_tickets_on_the_same_order_disagree_on_currency() {
        let conn = test_conn();
        let event_id = seed_event(&conn);
        let mut input = base_input(event_id, 2);
        input.currency = "GBP".to_string();
        let order_id = insert_order_with_tickets(&conn, &input, false).unwrap();
        let tickets = ticket_ids(&conn, order_id);
        conn.execute("UPDATE tickets SET currency='USD' WHERE id=?1", [tickets[1]]).unwrap();

        let err = convert_order_currency_impl(&conn, order_id, "EUR", 0.92, "2026-08-25").unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    /// Second pair of eyes flagged that `new_unit_price_cents` used to be
    /// converted directly from the order's own (pre-conversion) aggregate
    /// field rather than genuinely derived from a ticket's own row like
    /// fees/other costs are - harmless only because nothing in the app can
    /// currently make a ticket's `purchase_cost_cents` disagree with its
    /// order's `unit_price_cents` (no edit form touches either
    /// independently), but a restored/hand-edited backup could. This test
    /// proves the new guard added for that now actually refuses rather than
    /// silently deriving from the wrong number - same "refused conversions
    /// leave the database untouched" contract as every other guard here.
    #[test]
    fn refuses_to_convert_when_a_tickets_purchase_cost_does_not_match_the_orders_unit_price() {
        let conn = test_conn();
        let event_id = seed_event(&conn);
        let mut input = base_input(event_id, 2);
        input.currency = "GBP".to_string();
        input.unit_price_cents = 1000;
        let order_id = insert_order_with_tickets(&conn, &input, false).unwrap();
        let tickets = ticket_ids(&conn, order_id);
        // Simulates the only way this could happen in practice - a hand-
        // edited or restored-from-backup database (see this test's own doc
        // comment above).
        conn.execute("UPDATE tickets SET purchase_cost_cents=1500 WHERE id=?1", [tickets[1]]).unwrap();

        let err = convert_order_currency_impl(&conn, order_id, "EUR", 1.1, "2026-08-25").unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));

        let (t0_cost, t1_cost, order_currency): (i64, i64, String) = conn
            .query_row(
                "SELECT
                    (SELECT purchase_cost_cents FROM tickets WHERE id=?1),
                    (SELECT purchase_cost_cents FROM tickets WHERE id=?2),
                    (SELECT currency FROM orders WHERE id=?3)",
                params![tickets[0], tickets[1], order_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(t0_cost, 1000, "a refused conversion must leave every ticket completely untouched");
        assert_eq!(t1_cost, 1500);
        assert_eq!(order_currency, "GBP", "the order itself must stay untouched too");
    }

    // ---- resolve_currency_order_ids (2.0.51) -------------------------------
    // The DB-only half of convert_currencies_to_eur's currency resolution -
    // see that function's own doc comment for why a caller-supplied "USD"
    // must still find an order whose currency is literally stored as "usd"
    // (an un-normalized CSV import, most concretely). This is exactly the
    // bug a second pair of eyes caught: the old inline version of this logic
    // uppercased the CALLER's input but matched it against the RAW column
    // with a plain `=`, so a lowercase-stored currency silently matched zero
    // rows - no error, no skip, just nothing happening.

    #[test]
    fn resolve_currency_order_ids_matches_a_currency_regardless_of_stored_casing() {
        let conn = test_conn();
        let event_id = seed_event(&conn);
        let mut input = base_input(event_id, 1);
        input.currency = "usd".to_string(); // lower-case, exactly like an un-normalized CSV import cell.
        let order_id = insert_order_with_tickets(&conn, &input, false).unwrap();

        let resolved = resolve_currency_order_ids(&conn, &Some(vec!["USD".to_string()])).unwrap();

        assert_eq!(resolved, vec![("USD".to_string(), vec![order_id])]);
    }

    #[test]
    fn resolve_currency_order_ids_all_option_groups_mixed_casing_of_the_same_currency_together() {
        let conn = test_conn();
        let event_id = seed_event(&conn);
        let mut lower = base_input(event_id, 1);
        lower.currency = "usd".to_string();
        let order_a = insert_order_with_tickets(&conn, &lower, false).unwrap();
        let mut upper = base_input(event_id, 1);
        upper.currency = "USD".to_string();
        let order_b = insert_order_with_tickets(&conn, &upper, false).unwrap();
        let mut other = base_input(event_id, 1);
        other.currency = "GBP".to_string();
        let order_c = insert_order_with_tickets(&conn, &other, false).unwrap();

        let resolved = resolve_currency_order_ids(&conn, &None).unwrap();

        assert_eq!(resolved.len(), 2, "usd/USD must collapse into one normalized USD entry, not two");
        let usd_entry = resolved.iter().find(|(c, _)| c == "USD").unwrap();
        let mut usd_ids = usd_entry.1.clone();
        usd_ids.sort();
        let mut expected = vec![order_a, order_b];
        expected.sort();
        assert_eq!(usd_ids, expected);
        let gbp_entry = resolved.iter().find(|(c, _)| c == "GBP").unwrap();
        assert_eq!(gbp_entry.1, vec![order_c]);
    }

    #[test]
    fn resolve_currency_order_ids_explicit_list_filters_out_eur_and_omits_currencies_with_no_orders() {
        let conn = test_conn();
        let event_id = seed_event(&conn);
        let mut input = base_input(event_id, 1);
        input.currency = "GBP".to_string();
        let order_id = insert_order_with_tickets(&conn, &input, false).unwrap();

        let resolved = resolve_currency_order_ids(
            &conn,
            &Some(vec!["eur".to_string(), "gbp".to_string(), "jpy".to_string()]),
        )
        .unwrap();

        assert_eq!(
            resolved,
            vec![("GBP".to_string(), vec![order_id])],
            "EUR is filtered out entirely, and JPY (no matching orders) is simply absent - not an empty entry"
        );
    }

    #[test]
    fn refuses_to_convert_an_order_already_in_the_target_currency() {
        let conn = test_conn();
        let event_id = seed_event(&conn);
        let order_id = insert_order_with_tickets(&conn, &base_input(event_id, 1), false).unwrap(); // EUR by default

        let err = convert_order_currency_impl(&conn, order_id, "EUR", 1.0, "2026-08-25").unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn convert_order_currency_impl_errors_not_found_for_a_missing_order() {
        let conn = test_conn();
        let err = convert_order_currency_impl(&conn, 999999, "EUR", 1.1, "2026-08-25").unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[test]
    fn apply_bulk_currency_conversion_skips_one_bad_order_without_blocking_the_rest() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn);

        let mut gbp_input = base_input(event_id, 1);
        gbp_input.currency = "GBP".to_string();
        let order_a = insert_order_with_tickets(&conn, &gbp_input, false).unwrap();
        let order_bad = insert_order_with_tickets(&conn, &gbp_input, false).unwrap();
        // Same corruption as the "Edit Order" guard test above - order_bad's
        // own currency column no longer matches its tickets, so
        // convert_order_currency_impl must refuse it.
        conn.execute("UPDATE orders SET currency='USD' WHERE id=?1", [order_bad]).unwrap();

        let mut usd_input = base_input(event_id, 1);
        usd_input.currency = "USD".to_string();
        let order_c = insert_order_with_tickets(&conn, &usd_input, false).unwrap();

        let gbp_quote = fx::RateQuote { rate: 1.15, date: "2026-08-25".to_string() };
        let usd_quote = fx::RateQuote { rate: 0.92, date: "2026-08-25".to_string() };

        let result = apply_bulk_currency_conversion(
            &mut conn,
            &[(vec![order_a, order_bad], gbp_quote), (vec![order_c], usd_quote)],
        )
        .unwrap();

        assert_eq!(result.converted.len(), 2, "order_a and order_c must both convert despite order_bad failing");
        let converted_ids: Vec<i64> = result.converted.iter().map(|c| c.order_id).collect();
        assert!(converted_ids.contains(&order_a));
        assert!(converted_ids.contains(&order_c));
        assert_eq!(result.skipped.len(), 1);
        assert_eq!(result.skipped[0].id, order_bad);

        let order_a_currency: String =
            conn.query_row("SELECT currency FROM orders WHERE id=?1", [order_a], |r| r.get(0)).unwrap();
        assert_eq!(order_a_currency, "EUR", "order_a shares a bucket with order_bad but must still convert");
        let order_c_currency: String =
            conn.query_row("SELECT currency FROM orders WHERE id=?1", [order_c], |r| r.get(0)).unwrap();
        assert_eq!(order_c_currency, "EUR");
        let order_bad_currency: String =
            conn.query_row("SELECT currency FROM orders WHERE id=?1", [order_bad], |r| r.get(0)).unwrap();
        assert_eq!(order_bad_currency, "USD", "the refused order must be untouched, not partially converted");
    }

    #[test]
    fn convert_order_currency_command_impl_errors_not_found_without_any_network_call() {
        let mut conn = test_conn();
        let err = convert_order_currency_command_impl(&mut conn, 999999).unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    /// `fx::fetch_rate("EUR","EUR")` short-circuits with no network call (see
    /// fx.rs) - so this whole path, all the way through
    /// `convert_order_currency_impl`'s own "already in EUR" guard, is
    /// reachable without a real network request, unlike the general case.
    #[test]
    fn convert_order_currency_command_impl_refuses_an_order_already_in_eur_without_any_network_call() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn);
        let order_id = insert_order_with_tickets(&conn, &base_input(event_id, 1), false).unwrap(); // EUR by default

        let err = convert_order_currency_command_impl(&mut conn, order_id).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));

        let still_eur: String =
            conn.query_row("SELECT currency FROM orders WHERE id=?1", [order_id], |r| r.get(0)).unwrap();
        assert_eq!(still_eur, "EUR");
    }

    // ---- Order-grouped Tickets view: counts, filters, sales summary ------

    fn ticket_ids(conn: &Connection, order_id: i64) -> Vec<i64> {
        let mut stmt = conn
            .prepare("SELECT id FROM tickets WHERE order_id = ?1 ORDER BY id")
            .unwrap();
        stmt.query_map([order_id], |r| r.get::<_, i64>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    }

    fn ticket_code(conn: &Connection, ticket_id: i64) -> String {
        conn.query_row("SELECT code FROM tickets WHERE id = ?1", [ticket_id], |r| r.get(0))
            .unwrap()
    }

    fn set_status(conn: &Connection, ticket_id: i64, status: &str) {
        conn.execute("UPDATE tickets SET status=?1 WHERE id=?2", params![status, ticket_id])
            .unwrap();
    }

    /// Inserts a `sales` row directly (bypassing commands::sales, which is
    /// out of scope for this module's tests - mirrors the existing
    /// `delete_blocked_by_sale_history_even_after_refund` test above).
    fn insert_sale(conn: &Connection, code: &str, ticket_id: i64, price_cents: i64, payment_status: &str) {
        conn.execute(
            "INSERT INTO sales (code, ticket_id, sale_date, sale_price_cents, selling_fees_cents, payment_status)
             VALUES (?1, ?2, '2026-02-01', ?3, 0, ?4)",
            params![code, ticket_id, price_cents, payment_status],
        )
        .unwrap();
    }

    /// 2.0.66: sets a ticket's free-text `delivery_status` directly - same
    /// "raw UPDATE, bypass the command layer" convention as `set_status`
    /// right above.
    fn set_delivery_status(conn: &Connection, ticket_id: i64, delivery_status: &str) {
        conn.execute(
            "UPDATE tickets SET delivery_status=?1 WHERE id=?2",
            params![delivery_status, ticket_id],
        )
        .unwrap();
    }

    /// 2.0.67: reads a ticket's own `delivery_status` back - the read-side
    /// counterpart of `set_delivery_status` above, used by the new bulk
    /// delivery-status tests below.
    fn ticket_delivery_status(conn: &Connection, ticket_id: i64) -> Option<String> {
        conn.query_row(
            "SELECT delivery_status FROM tickets WHERE id = ?1",
            [ticket_id],
            |r| r.get(0),
        )
        .unwrap()
    }

    /// 2.0.67: reads back the `payment_status` of a ticket's own sale row -
    /// used by the new bulk payment-status tests below. Assumes the ticket
    /// has exactly one sale row, which is all these tests ever create.
    fn sale_payment_status_for_ticket(conn: &Connection, ticket_id: i64) -> String {
        conn.query_row(
            "SELECT payment_status FROM sales WHERE ticket_id = ?1",
            [ticket_id],
            |r| r.get(0),
        )
        .unwrap()
    }

    #[test]
    fn sold_available_listed_cancelled_counts_always_sum_to_quantity() {
        let conn = test_conn();
        let event_id = seed_event(&conn);
        let input = base_input(event_id, 10);
        let order_id = insert_order_with_tickets(&conn, &input, false).unwrap();
        let tickets = ticket_ids(&conn, order_id);

        set_status(&conn, tickets[0], "sold");
        set_status(&conn, tickets[1], "sold");
        set_status(&conn, tickets[2], "sold");
        set_status(&conn, tickets[3], "listed");
        set_status(&conn, tickets[4], "cancelled");
        // tickets[5..10) stay 'available' (5 of them)

        let order = fetch_one(&conn, order_id).unwrap();
        assert_eq!(order.sold_count, 3);
        assert_eq!(order.listed_count, 1);
        assert_eq!(order.cancelled_count, 1);
        assert_eq!(order.available_count, 5);
        assert_eq!(
            order.sold_count + order.listed_count + order.cancelled_count + order.available_count,
            order.quantity,
            "every ticket must be in exactly one bucket - none lost, none double-counted"
        );
    }

    /// 2.0.66: `delivered_count`/`paid_count` (the other two legs of the new
    /// "Completed" indicator, see REDESIGN-2.0.66-REPORT.md) must only ever
    /// count SOLD tickets - an available ticket that happens to already have
    /// `delivery_status='Delivered'` set (e.g. leftover pre-resale stock)
    /// must not count, and a sold-but-still-pending sale must not count
    /// toward `paid_count`.
    #[test]
    fn delivered_and_paid_counts_only_count_sold_tickets() {
        let conn = test_conn();
        let event_id = seed_event(&conn);
        let order_id = insert_order_with_tickets(&conn, &base_input(event_id, 4), false).unwrap();
        let tickets = ticket_ids(&conn, order_id);

        // t0: sold, delivered, paid - counts toward both.
        set_status(&conn, tickets[0], "sold");
        set_delivery_status(&conn, tickets[0], "Delivered");
        insert_sale(&conn, "SAL-100001", tickets[0], 2000, "paid");
        // t1: sold, NOT delivered, paid - counts toward paid_count only.
        set_status(&conn, tickets[1], "sold");
        insert_sale(&conn, "SAL-100002", tickets[1], 2000, "paid");
        // t2: sold, delivered, but sale still pending - counts toward
        // delivered_count only.
        set_status(&conn, tickets[2], "sold");
        set_delivery_status(&conn, tickets[2], "Delivered");
        insert_sale(&conn, "SAL-100003", tickets[2], 2000, "pending");
        // t3: never sold (still available) - even with delivery_status set,
        // must NOT count toward delivered_count (nothing to deliver yet).
        set_delivery_status(&conn, tickets[3], "Delivered");

        let order = fetch_one(&conn, order_id).unwrap();
        assert_eq!(order.sold_count, 3);
        assert_eq!(order.available_count, 1);
        assert_eq!(order.delivered_count, 2, "only t0 and t2 are both sold and delivered");
        assert_eq!(order.paid_count, 2, "only t0 and t1 are both sold and paid");
    }

    /// 2.0.66: a refund reverts the ticket to `status='available'` (mirrors
    /// `refund_sale_impl` in commands/sales.rs) - once that happens the
    /// ticket must drop out of `sold_count` (and therefore out of
    /// `delivered_count`/`paid_count` too), never double-penalized as
    /// "sold but unpaid".
    #[test]
    fn refunded_ticket_drops_out_of_sold_delivered_and_paid_counts() {
        let conn = test_conn();
        let event_id = seed_event(&conn);
        let order_id = insert_order_with_tickets(&conn, &base_input(event_id, 2), false).unwrap();
        let tickets = ticket_ids(&conn, order_id);

        // t0: sold and paid, never refunded.
        set_status(&conn, tickets[0], "sold");
        set_delivery_status(&conn, tickets[0], "Delivered");
        insert_sale(&conn, "SAL-200001", tickets[0], 2000, "paid");
        // t1: was sold and paid, then refunded - mirror refund_sale_impl's
        // own two effects (payment_status='refunded' AND ticket reverts to
        // 'available').
        set_status(&conn, tickets[1], "sold");
        set_delivery_status(&conn, tickets[1], "Delivered");
        insert_sale(&conn, "SAL-200002", tickets[1], 2000, "paid");
        conn.execute("UPDATE sales SET payment_status='refunded' WHERE ticket_id=?1", [tickets[1]])
            .unwrap();
        set_status(&conn, tickets[1], "available");

        let order = fetch_one(&conn, order_id).unwrap();
        assert_eq!(order.sold_count, 1);
        assert_eq!(order.available_count, 1);
        assert_eq!(order.delivered_count, 1, "refunded ticket must not count even though delivery_status is still 'Delivered'");
        assert_eq!(order.paid_count, 1, "refunded ticket must not count - its only sale row is now payment_status='refunded'");
    }

    // 2.0.67: the new Orders-list bulk "Mark Delivered/Paid" action (see
    // REDESIGN-2.0.67-REPORT.md) - `resolve_orders_sold_ticket_ids`/
    // `resolve_orders_active_sale_ids` and the two `bulk_set_orders_*_impl`
    // functions that delegate to them.

    #[test]
    fn bulk_set_orders_delivery_status_only_touches_sold_tickets_in_the_order() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn);
        let order_id = insert_order_with_tickets(&conn, &base_input(event_id, 3), false).unwrap();
        let tickets = ticket_ids(&conn, order_id);

        set_status(&conn, tickets[0], "sold");
        set_status(&conn, tickets[1], "listed");
        // tickets[2] stays 'available'.

        let updated = bulk_set_orders_delivery_status_impl(&mut conn, &[order_id], "Delivered").unwrap();

        assert_eq!(updated, 1, "only the one sold ticket should be touched");
        assert_eq!(ticket_delivery_status(&conn, tickets[0]).as_deref(), Some("Delivered"));
        assert_eq!(ticket_delivery_status(&conn, tickets[1]), None, "listed ticket must be untouched");
        assert_eq!(ticket_delivery_status(&conn, tickets[2]), None, "available ticket must be untouched");
    }

    #[test]
    fn bulk_set_orders_delivery_status_spans_every_selected_order() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn);
        let order_a = insert_order_with_tickets(&conn, &base_input(event_id, 1), false).unwrap();
        let order_b = insert_order_with_tickets(&conn, &base_input(event_id, 1), false).unwrap();
        let ticket_a = ticket_ids(&conn, order_a)[0];
        let ticket_b = ticket_ids(&conn, order_b)[0];
        set_status(&conn, ticket_a, "sold");
        set_status(&conn, ticket_b, "sold");

        let updated =
            bulk_set_orders_delivery_status_impl(&mut conn, &[order_a, order_b], "Delivered").unwrap();

        assert_eq!(updated, 2, "both orders' sold tickets must be updated together");
        assert_eq!(ticket_delivery_status(&conn, ticket_a).as_deref(), Some("Delivered"));
        assert_eq!(ticket_delivery_status(&conn, ticket_b).as_deref(), Some("Delivered"));
    }

    #[test]
    fn bulk_set_orders_delivery_status_is_a_harmless_no_op_when_nothing_is_sold() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn);
        let order_id = insert_order_with_tickets(&conn, &base_input(event_id, 2), false).unwrap();
        // Every ticket stays 'available' - nothing sold yet.

        let updated = bulk_set_orders_delivery_status_impl(&mut conn, &[order_id], "Delivered").unwrap();

        assert_eq!(updated, 0, "no sold tickets means nothing to update, not an error");
    }

    #[test]
    fn bulk_set_orders_payment_status_excludes_a_refunded_sale_but_updates_the_rest() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn);
        let order_id = insert_order_with_tickets(&conn, &base_input(event_id, 2), false).unwrap();
        let tickets = ticket_ids(&conn, order_id);

        // t0: sold, still pending - eligible to be marked paid in bulk.
        set_status(&conn, tickets[0], "sold");
        insert_sale(&conn, "SAL-300001", tickets[0], 2000, "pending");
        // t1: sold, but its sale was refunded - must be left completely alone.
        set_status(&conn, tickets[1], "sold");
        insert_sale(&conn, "SAL-300002", tickets[1], 2000, "paid");
        conn.execute("UPDATE sales SET payment_status='refunded' WHERE ticket_id=?1", [tickets[1]])
            .unwrap();

        let updated = bulk_set_orders_payment_status_impl(&mut conn, &[order_id], "paid").unwrap();

        assert_eq!(updated, 1, "only the one non-refunded sale should be touched");
        assert_eq!(sale_payment_status_for_ticket(&conn, tickets[0]), "paid");
        assert_eq!(
            sale_payment_status_for_ticket(&conn, tickets[1]),
            "refunded",
            "a refunded sale must never be resurrected by a bulk action"
        );
    }

    #[test]
    fn bulk_set_orders_payment_status_is_a_harmless_no_op_when_only_refunded_sales_exist() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn);
        let order_id = insert_order_with_tickets(&conn, &base_input(event_id, 1), false).unwrap();
        let tickets = ticket_ids(&conn, order_id);
        set_status(&conn, tickets[0], "sold");
        insert_sale(&conn, "SAL-300003", tickets[0], 2000, "paid");
        conn.execute("UPDATE sales SET payment_status='refunded' WHERE ticket_id=?1", [tickets[0]])
            .unwrap();

        let updated = bulk_set_orders_payment_status_impl(&mut conn, &[order_id], "paid").unwrap();

        assert_eq!(updated, 0, "the only sale is refunded, so there is nothing left to mark paid");
        assert_eq!(sale_payment_status_for_ticket(&conn, tickets[0]), "refunded");
    }

    #[test]
    fn order_sales_summary_excludes_unsold_and_refunded_tickets() {
        let conn = test_conn();
        let event_id = seed_event(&conn);
        let input = base_input(event_id, 4); // unit price 1000, fees 100, other 50 -> cost/ticket varies slightly after allocation
        let order_id = insert_order_with_tickets(&conn, &input, false).unwrap();
        let tickets = ticket_ids(&conn, order_id);

        // ticket 0: sold & paid (realized)
        insert_sale(&conn, "SAL-000001", tickets[0], 2000, "paid");
        set_status(&conn, tickets[0], "sold");
        // ticket 1: sold & pending (still realized - only 'refunded' is excluded)
        insert_sale(&conn, "SAL-000002", tickets[1], 1800, "pending");
        set_status(&conn, tickets[1], "sold");
        // ticket 2: sold then refunded (must be excluded from revenue)
        insert_sale(&conn, "SAL-000003", tickets[2], 5000, "refunded");
        // ticket 3: never sold, stays available (must not appear at all)

        let summary = fetch_sales_summary(&conn, order_id).unwrap();
        assert_eq!(summary.revenue_cents, 2000 + 1800, "refunded ticket's price must be excluded");

        let cost_per_ticket = {
            let o = fetch_one(&conn, order_id).unwrap();
            o.total_cost_cents / o.quantity
        };
        // COGS only for the 2 realized tickets (allocation is exact-ish; just
        // assert it's non-zero and strictly less than the order's full cost).
        assert!(summary.cogs_cents > 0);
        assert!(summary.cogs_cents < cost_per_ticket * 4);
        assert_eq!(
            summary.profit_cents,
            summary.revenue_cents - summary.cogs_cents - summary.selling_fees_cents
        );
    }

    #[test]
    fn order_sales_summary_is_zero_when_nothing_sold_yet() {
        let conn = test_conn();
        let event_id = seed_event(&conn);
        let input = base_input(event_id, 3);
        let order_id = insert_order_with_tickets(&conn, &input, false).unwrap();

        let summary = fetch_sales_summary(&conn, order_id).unwrap();
        assert_eq!(summary.revenue_cents, 0);
        assert_eq!(summary.profit_cents, 0);
        assert_eq!(summary.margin, None, "0 revenue -> N/A, not 0%");
    }

    #[test]
    fn list_orders_status_filter_keeps_the_orders_full_counts() {
        let conn = test_conn();
        let event_id = seed_event(&conn);
        let order1 = insert_order_with_tickets(&conn, &base_input(event_id, 3), false).unwrap();
        let order2 = insert_order_with_tickets(&conn, &base_input(event_id, 2), false).unwrap();
        let t1 = ticket_ids(&conn, order1);
        set_status(&conn, t1[0], "sold");
        // order2 has no sold tickets at all.

        let sold_orders = list_orders_impl(
            &conn,
            None,
            None,
            None,
            None,
            None,
            Some("sold".to_string()),
            None,
            None,
            None,
            None,
        )
        .unwrap();
        assert_eq!(sold_orders.len(), 1, "only order1 has a sold ticket");
        assert_eq!(sold_orders[0].id, order1);
        // The filter narrows WHICH orders appear, but this order's own counts
        // must stay its true, complete counts - not just the matching ticket.
        assert_eq!(sold_orders[0].quantity, 3);
        assert_eq!(sold_orders[0].sold_count, 1);
        assert_eq!(sold_orders[0].available_count, 2);

        let _ = order2; // present only to prove it's excluded above
    }

    #[test]
    fn list_orders_section_and_date_filters() {
        let conn = test_conn();
        let event_id = seed_event(&conn);
        let mut a = base_input(event_id, 1);
        a.section = Some("VIP".to_string());
        a.purchase_date = "2026-01-05".to_string();
        let order_a = insert_order_with_tickets(&conn, &a, false).unwrap();

        let mut b = base_input(event_id, 1);
        b.section = Some("General".to_string());
        b.purchase_date = "2026-06-05".to_string();
        insert_order_with_tickets(&conn, &b, false).unwrap();

        let by_section = list_orders_impl(
            &conn, None, None, None, None, None, None, Some("VIP".to_string()), None, None, None,
        )
        .unwrap();
        assert_eq!(by_section.len(), 1);
        assert_eq!(by_section[0].id, order_a);

        let by_date = list_orders_impl(
            &conn,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("2026-01-01".to_string()),
            Some("2026-02-01".to_string()),
            None,
        )
        .unwrap();
        assert_eq!(by_date.len(), 1);
        assert_eq!(by_date[0].id, order_a);
    }

    #[test]
    fn hundred_ticket_order_summary_and_counts_stay_correct_at_that_scale() {
        let conn = test_conn();
        let event_id = seed_event(&conn);
        let input = base_input(event_id, 100);
        let order_id = insert_order_with_tickets(&conn, &input, false).unwrap();
        let tickets = ticket_ids(&conn, order_id);
        assert_eq!(tickets.len(), 100);

        for (i, &tid) in tickets.iter().enumerate() {
            if i < 60 {
                insert_sale(&conn, &format!("SAL-{i:06}"), tid, 2000, "paid");
                set_status(&conn, tid, "sold");
            }
            // remaining 40 stay available
        }

        let order = fetch_one(&conn, order_id).unwrap();
        assert_eq!(order.sold_count, 60);
        assert_eq!(order.available_count, 40);
        assert_eq!(order.quantity, 100);

        let summary = fetch_sales_summary(&conn, order_id).unwrap();
        assert_eq!(summary.revenue_cents, 2000 * 60);
    }

    // ---- BUG #5: ticket-code search on the order-grouped Tickets/Inventory view ----
    //
    // The Tickets and Inventory pages are both powered by list_orders (see
    // TicketsView in the frontend) - after the order-grouped redesign, the
    // free-text search only matched o.code/e.name/sup.name/p.name, never a
    // ticket's own code. These tests cover exactly the regression the audit
    // found: ticket code must be searchable again, additively, alongside
    // every field that already worked - without breaking the grouping.

    #[test]
    fn search_finds_order_by_exact_ticket_code() {
        let conn = test_conn();
        let event_id = seed_event(&conn);
        let order_id = insert_order_with_tickets(&conn, &base_input(event_id, 3), false).unwrap();
        let tickets = ticket_ids(&conn, order_id);
        let target_code = ticket_code(&conn, tickets[1]);

        let results =
            list_orders_impl(&conn, Some(target_code.clone()), None, None, None, None, None, None, None, None, None)
                .unwrap();

        assert_eq!(results.len(), 1, "exact ticket code must find its order: {target_code}");
        assert_eq!(results[0].id, order_id);
    }

    #[test]
    fn search_finds_order_by_partial_ticket_code() {
        let conn = test_conn();
        let event_id = seed_event(&conn);
        let order_id = insert_order_with_tickets(&conn, &base_input(event_id, 1), false).unwrap();
        let tickets = ticket_ids(&conn, order_id);
        let full_code = ticket_code(&conn, tickets[0]); // e.g. "TKT-000001"
        // Same substring/LIKE semantics every other search field already
        // uses (the shared `like = format!("%{q}%")` in list_orders_impl) -
        // just the numeric part, without the "TKT-" prefix, so this is a
        // genuine substring match, not accidentally the full code again.
        let partial = full_code.trim_start_matches("TKT-");
        assert_ne!(partial, full_code, "sanity: must be a genuine substring, not the whole code");

        let results =
            list_orders_impl(&conn, Some(partial.to_string()), None, None, None, None, None, None, None, None, None)
                .unwrap();

        assert_eq!(results.len(), 1, "partial ticket code {partial:?} must still find the order");
        assert_eq!(results[0].id, order_id);
    }

    #[test]
    fn search_still_finds_orders_by_order_code_event_name_supplier_and_platform() {
        let conn = test_conn();

        conn.execute("INSERT INTO events (name) VALUES ('Coldplay Arena Show')", [])
            .unwrap();
        let named_event_id = conn.last_insert_rowid();
        let plain_event_id = seed_event(&conn);

        conn.execute("INSERT INTO suppliers (name) VALUES ('Acme Supplier')", [])
            .unwrap();
        let supplier_id = conn.last_insert_rowid();
        conn.execute("INSERT INTO platforms (name) VALUES ('Viagogo')", [])
            .unwrap();
        let platform_id = conn.last_insert_rowid();

        let order_by_event = insert_order_with_tickets(&conn, &base_input(named_event_id, 1), false).unwrap();

        let mut sup_input = base_input(plain_event_id, 1);
        sup_input.supplier_id = Some(supplier_id);
        let order_by_supplier = insert_order_with_tickets(&conn, &sup_input, false).unwrap();

        let mut plat_input = base_input(plain_event_id, 1);
        plat_input.platform_id = Some(platform_id);
        let order_by_platform = insert_order_with_tickets(&conn, &plat_input, false).unwrap();

        // A plain, unrelated order - must never show up in any search below,
        // proving these searches actually narrow results, not just pass
        // everything through.
        insert_order_with_tickets(&conn, &base_input(plain_event_id, 1), false).unwrap();

        let order_code = fetch_one(&conn, order_by_event).unwrap().code;

        let by_order_code =
            list_orders_impl(&conn, Some(order_code.clone()), None, None, None, None, None, None, None, None, None)
                .unwrap();
        assert_eq!(by_order_code.len(), 1);
        assert_eq!(by_order_code[0].id, order_by_event);

        let by_event_name =
            list_orders_impl(&conn, Some("Coldplay".to_string()), None, None, None, None, None, None, None, None, None)
                .unwrap();
        assert_eq!(by_event_name.len(), 1);
        assert_eq!(by_event_name[0].id, order_by_event);

        let by_supplier =
            list_orders_impl(&conn, Some("Acme".to_string()), None, None, None, None, None, None, None, None, None)
                .unwrap();
        assert_eq!(by_supplier.len(), 1);
        assert_eq!(by_supplier[0].id, order_by_supplier);

        let by_platform =
            list_orders_impl(&conn, Some("Viagogo".to_string()), None, None, None, None, None, None, None, None, None)
                .unwrap();
        assert_eq!(by_platform.len(), 1);
        assert_eq!(by_platform[0].id, order_by_platform);
    }

    #[test]
    fn search_by_nonexistent_ticket_code_returns_no_results() {
        let conn = test_conn();
        let event_id = seed_event(&conn);
        insert_order_with_tickets(&conn, &base_input(event_id, 2), false).unwrap();

        let results =
            list_orders_impl(&conn, Some("TKT-999999".to_string()), None, None, None, None, None, None, None, None, None)
                .unwrap();

        assert!(results.is_empty(), "a ticket code that doesn't exist must find nothing");
    }

    #[test]
    fn search_by_ticket_code_returns_the_whole_order_group_with_correct_full_counts() {
        let conn = test_conn();
        let event_id = seed_event(&conn);
        let order_id = insert_order_with_tickets(&conn, &base_input(event_id, 5), false).unwrap();
        let tickets = ticket_ids(&conn, order_id);
        assert_eq!(tickets.len(), 5);

        // Sell two, cancel one, leave two available - non-trivial counts -
        // then search by ONE of these tickets' own codes.
        set_status(&conn, tickets[0], "sold");
        set_status(&conn, tickets[1], "sold");
        set_status(&conn, tickets[2], "cancelled");
        let searched_code = ticket_code(&conn, tickets[3]); // still 'available'

        let results =
            list_orders_impl(&conn, Some(searched_code.clone()), None, None, None, None, None, None, None, None, None)
                .unwrap();

        // Multiple tickets belong to the same order - searching by one of
        // them must still produce exactly ONE grouped row for that order,
        // never fan out into one row per matching ticket.
        assert_eq!(results.len(), 1, "must return one grouped order row, not one row per ticket");
        assert_eq!(results[0].id, order_id);

        // And that row's counts must be the order's TRUE, complete counts -
        // not just the single matching ticket. This is exactly the hazard a
        // direct `t.code LIKE ?` predicate (instead of the semi-join
        // subquery used here) would have caused.
        assert_eq!(results[0].quantity, 5);
        assert_eq!(results[0].sold_count, 2);
        assert_eq!(results[0].cancelled_count, 1);
        assert_eq!(results[0].available_count, 2);
    }
}

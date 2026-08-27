use crate::codes;
use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::finance;
use crate::models::{
    BulkDeleteResult, BulkDeleteSkip, BulkSalePaymentStatusInput, Sale, SaleBatchInput, SaleEditInput, SaleGroup,
    SaleInput, SeatEntry,
};
use rusqlite::{params, Connection, Row, Transaction};
use std::collections::HashSet;
use tauri::State;

// Safety cap on the unfiltered list view - see the identical constant in
// commands/tickets.rs for the rationale.
const LIST_CAP: i64 = 5000;

const BASE_SQL: &str = "
    SELECT s.id, s.code, s.ticket_id, t.code as ticket_code,
      t.section, t.row_label, t.seat,
      t.event_id, e.name as event_name,
      t.order_id, o.code as order_code,
      s.platform_id, p.name as platform_name, s.sale_date, s.sale_price_cents, s.selling_fees_cents,
      s.currency, t.currency as ticket_currency, s.payment_status, s.buyer_reference, s.notes, s.is_demo, s.created_at, s.updated_at,
      s.refunded_at, s.refund_reason, s.batch_id,
      (t.purchase_cost_cents + t.purchase_fees_cents + t.other_costs_cents) as cost_cents
    FROM sales s
    JOIN tickets t ON t.id = s.ticket_id
    JOIN events e ON e.id = t.event_id
    JOIN orders o ON o.id = t.order_id
    LEFT JOIN platforms p ON p.id = s.platform_id
";

/// Groups sales rows that were submitted together as one "New sale" action.
/// NULL batch_id (an ordinary single-ticket sale) is its own group of one -
/// `'single:' || id` can never collide with a real batch_id, which is always
/// a `SAL-xxxxxx` code (see migration 003).
pub(crate) const GROUP_KEY_EXPR: &str = "COALESCE(s.batch_id, 'single:' || s.id)";

fn map_sale(row: &Row) -> rusqlite::Result<Sale> {
    let sale_price_cents: i64 = row.get("sale_price_cents")?;
    let selling_fees_cents: i64 = row.get("selling_fees_cents")?;
    let cost_cents: i64 = row.get("cost_cents")?;
    let currency: String = row.get("currency")?;
    let ticket_currency: String = row.get("ticket_currency")?;
    // 2.0.57: before this version a sale's own currency always equalled its
    // ticket's purchase currency by construction (see
    // create_sales_batch_impl), so cost_cents (in the ticket's currency) and
    // sale_price_cents (in the sale's currency) were always safely
    // subtractable. New Sale can now record a line in a DIFFERENT currency
    // than that ticket was bought in, so profit/margin/ROI for THIS row
    // must not silently blend two currencies into one number - same "never
    // blend, show Mixed" rule GROUP_BASE_SELECT's own currency CASE WHEN
    // enforces at the batch level.
    let currency_mismatch = currency != ticket_currency;
    let profit = finance::profit_cents(sale_price_cents, cost_cents, selling_fees_cents);
    let (margin, roi) = if currency_mismatch {
        (None, None)
    } else {
        (
            finance::safe_ratio(profit, sale_price_cents),
            finance::safe_ratio(profit, cost_cents),
        )
    };
    Ok(Sale {
        id: row.get("id")?,
        code: row.get("code")?,
        ticket_id: row.get("ticket_id")?,
        ticket_code: row.get("ticket_code")?,
        section: row.get("section")?,
        row_label: row.get("row_label")?,
        seat: row.get("seat")?,
        event_id: row.get("event_id")?,
        event_name: row.get("event_name")?,
        order_id: row.get("order_id")?,
        order_code: row.get("order_code")?,
        platform_id: row.get("platform_id")?,
        platform_name: row.get("platform_name")?,
        sale_date: row.get("sale_date")?,
        sale_price_cents,
        selling_fees_cents,
        currency,
        currency_mismatch,
        payment_status: row.get("payment_status")?,
        buyer_reference: row.get("buyer_reference")?,
        notes: row.get("notes")?,
        is_demo: row.get("is_demo")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        cost_cents,
        profit_cents: profit,
        margin,
        roi,
        refunded_at: row.get("refunded_at")?,
        refund_reason: row.get("refund_reason")?,
        batch_id: row.get("batch_id")?,
    })
}

fn map_sale_group(row: &Row) -> rusqlite::Result<SaleGroup> {
    let revenue_cents: i64 = row.get("revenue_cents")?;
    let selling_fees_cents: i64 = row.get("selling_fees_cents")?;
    let cost_cents: i64 = row.get("cost_cents")?;
    let currency: Option<String> = row.get("currency")?;
    let profit_cents = finance::profit_cents(revenue_cents, cost_cents, selling_fees_cents);
    // BUG #6: `currency` above is already None whenever this group's lines
    // don't all share one currency (GROUP_BASE_SELECT's
    // `COUNT(DISTINCT s.currency) = 1` check). Margin/ROI are ratios
    // expressed IN that currency, so blending e.g. EUR and USD cents into
    // one percentage is mathematically well-formed but economically
    // meaningless. Mirror the same "mixed -> None, never blend" rule
    // Revenue/Fees/Profit already follow on the frontend
    // (formatMoneyOrMixed) instead of computing a currency-blind ratio.
    let (margin, roi) = if currency.is_some() {
        (
            finance::safe_ratio(profit_cents, revenue_cents),
            finance::safe_ratio(profit_cents, cost_cents),
        )
    } else {
        (None, None)
    };
    Ok(SaleGroup {
        id: row.get("id")?,
        code: row.get("code")?,
        batch_id: row.get("batch_id")?,
        ticket_count: row.get("ticket_count")?,
        event_id: row.get("event_id")?,
        event_name: row.get("event_name")?,
        category_id: row.get("category_id")?,
        category_name: row.get("category_name")?,
        category_color_slot: row.get("category_color_slot")?,
        sale_date: row.get("sale_date")?,
        platform_id: row.get("platform_id")?,
        platform_name: row.get("platform_name")?,
        currency,
        revenue_cents,
        selling_fees_cents,
        cost_cents,
        profit_cents,
        margin,
        roi,
        payment_status: row.get("payment_status")?,
        refunded_count: row.get("refunded_count")?,
        is_demo: row.get("is_demo")?,
        seats: SeatEntry::parse_aggregate(row.get::<_, Option<String>>("seats_raw")?.as_deref()),
    })
}

pub(crate) fn fetch_recent(conn: &Connection, limit: i64) -> AppResult<Vec<Sale>> {
    let sql = format!("{BASE_SQL} ORDER BY s.created_at DESC LIMIT ?1");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([limit], map_sale)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Same grouping the main Sales list already uses (GROUP_BASE_SELECT/
/// GROUP_KEY_EXPR/map_sale_group) - one row per sale ACTION (a single
/// ticket, or a multi-ticket batch), never one row per ticket - for the
/// Dashboard's "Recent Sales" card. Before 2.0.54 that card called
/// `fetch_recent` above directly, so one 4-ticket batch sale showed as 4
/// identical entries there - marko caught this on a real screenshot (4x
/// the same event/date/price, which really was one sale of 4 tickets).
/// Ordered by MAX(s.created_at) - a real batch's lines all share the exact
/// same created_at from create_sales_batch_impl anyway, so this is really
/// just "this group's own timestamp" rather than picking one line over
/// another.
pub(crate) fn fetch_recent_groups(conn: &Connection, limit: i64) -> AppResult<Vec<SaleGroup>> {
    let sql = format!("{GROUP_BASE_SELECT} GROUP BY {GROUP_KEY_EXPR} ORDER BY MAX(s.created_at) DESC LIMIT ?1");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([limit], map_sale_group)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

fn fetch_one(conn: &Connection, id: i64) -> AppResult<Sale> {
    let sql = format!("{BASE_SQL} WHERE s.id = ?1");
    conn.query_row(&sql, [id], map_sale)
        .map_err(|_| AppError::NotFound(format!("Sale #{id} not found")))
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn list_sales(
    state: State<AppState>,
    search: Option<String>,
    event_id: Option<i64>,
    platform_id: Option<i64>,
    payment_status: Option<String>,
    date_from: Option<String>,
    date_to: Option<String>,
) -> AppResult<Vec<Sale>> {
    let conn = state.db.lock().unwrap();
    let mut sql = format!("{BASE_SQL} WHERE 1=1");
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![];

    if let Some(eid) = event_id {
        sql.push_str(" AND t.event_id = ?");
        params_vec.push(Box::new(eid));
    }
    if let Some(pid) = platform_id {
        sql.push_str(" AND s.platform_id = ?");
        params_vec.push(Box::new(pid));
    }
    if let Some(ps) = payment_status.as_deref() {
        if !ps.is_empty() {
            sql.push_str(" AND s.payment_status = ?");
            params_vec.push(Box::new(ps.to_string()));
        }
    }
    if let Some(from) = date_from.as_deref() {
        if !from.is_empty() {
            sql.push_str(" AND s.sale_date >= ?");
            params_vec.push(Box::new(from.to_string()));
        }
    }
    if let Some(to) = date_to.as_deref() {
        if !to.is_empty() {
            sql.push_str(" AND s.sale_date <= ?");
            params_vec.push(Box::new(to.to_string()));
        }
    }
    if let Some(q) = search.as_deref() {
        let q = q.trim();
        if !q.is_empty() {
            sql.push_str(" AND (s.code LIKE ? OR t.code LIKE ? OR e.name LIKE ? OR s.buyer_reference LIKE ?)");
            let like = format!("%{q}%");
            for _ in 0..4 {
                params_vec.push(Box::new(like.clone()));
            }
        }
    }
    sql.push_str(&format!(" ORDER BY s.sale_date DESC, s.id DESC LIMIT {LIST_CAP}"));

    let mut stmt = conn.prepare(&sql)?;
    let param_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
    let rows = stmt.query_map(param_refs.as_slice(), map_sale)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[tauri::command]
pub fn get_sale(state: State<AppState>, id: i64) -> AppResult<Sale> {
    let conn = state.db.lock().unwrap();
    fetch_one(&conn, id)
}

pub(crate) const GROUP_BASE_SELECT: &str = "
    SELECT
      MIN(s.id) as id,
      MIN(s.code) as code,
      MAX(s.batch_id) as batch_id,
      COUNT(*) as ticket_count,
      CASE WHEN COUNT(DISTINCT t.event_id) = 1 THEN MAX(t.event_id) END as event_id,
      CASE WHEN COUNT(DISTINCT t.event_id) = 1 THEN MAX(e.name) END as event_name,
      -- 2.0.27: a category is itself just an attribute of the group's shared
      -- event, so it uses the exact same only-when-every-lines-event-agrees
      -- guard as event_id/event_name right above, not a separate
      -- COUNT(DISTINCT ...) of its own.
      CASE WHEN COUNT(DISTINCT t.event_id) = 1 THEN MAX(e.category_id) END as category_id,
      CASE WHEN COUNT(DISTINCT t.event_id) = 1 THEN MAX(ec.name) END as category_name,
      CASE WHEN COUNT(DISTINCT t.event_id) = 1 THEN MAX(ec.color_slot) END as category_color_slot,
      MAX(s.sale_date) as sale_date,
      MAX(s.platform_id) as platform_id,
      MAX(p.name) as platform_name,
      -- 1.6.0 audit H5: currency must be derived the same way the money
      -- fields right below it are - from non-refunded lines only. Deriving
      -- it from ALL lines (including refunded ones) meant a batch whose
      -- ONLY differently-currencied line had been refunded still showed
      -- Mixed for money/margin/ROI, even though what's left is a clean,
      -- fully-computable single-currency total. Falls back to checking ALL
      -- lines only when every line in the group is refunded (COUNT = 0
      -- among non-refunded ones), so a fully-refunded single-currency group
      -- still reports its currency instead of going blank.
      -- 2.0.57: also requires every counted row's OWN s.currency = t.currency
      -- - New Sale can now record a line in a currency that differs from
      -- that same ticket's own purchase currency (see
      -- SaleBatchInput::currency), so every line merely agreeing with every
      -- other line is no longer enough on its own: a batch entirely in USD
      -- whose tickets were all bought in EUR must still show Mixed (cost
      -- and revenue aren't in one currency), even though COUNT(DISTINCT
      -- s.currency) = 1. See map_sale's identical per-row guard below for
      -- the ungrouped equivalent.
      CASE
        WHEN COUNT(DISTINCT CASE WHEN s.payment_status != 'refunded' THEN s.currency END) = 1
          AND COUNT(CASE WHEN s.payment_status != 'refunded' AND s.currency != t.currency THEN 1 END) = 0
          THEN MAX(CASE WHEN s.payment_status != 'refunded' THEN s.currency END)
        WHEN COUNT(DISTINCT CASE WHEN s.payment_status != 'refunded' THEN s.currency END) = 0
          AND COUNT(CASE WHEN s.currency != t.currency THEN 1 END) = 0
          THEN CASE WHEN COUNT(DISTINCT s.currency) = 1 THEN MAX(s.currency) END
        ELSE NULL
      END as currency,
      COALESCE(SUM(CASE WHEN s.payment_status != 'refunded' THEN s.sale_price_cents END), 0) as revenue_cents,
      COALESCE(SUM(CASE WHEN s.payment_status != 'refunded' THEN s.selling_fees_cents END), 0) as selling_fees_cents,
      COALESCE(SUM(CASE WHEN s.payment_status != 'refunded' THEN (t.purchase_cost_cents+t.purchase_fees_cents+t.other_costs_cents) END), 0) as cost_cents,
      CASE WHEN COUNT(DISTINCT s.payment_status) = 1 THEN MAX(s.payment_status) END as payment_status,
      SUM(CASE WHEN s.payment_status = 'refunded' THEN 1 ELSE 0 END) as refunded_count,
      MAX(s.is_demo) as is_demo,
      -- 2.0.38: raw per-ticket seat data for the new Seats column - same
      -- encoding/parsing as orders.rs's own identical addition, see
      -- SeatEntry::parse_aggregate's doc comment (models.rs). Reuses the SAME
      -- `t` join every other field above already uses - no new JOIN needed.
      -- Deliberately NOT filtered by refund status, matching ticket_count
      -- above (a refunded line is still one of this group's tickets) - see
      -- SaleGroup.seats' own doc comment (models.rs).
      GROUP_CONCAT(
        COALESCE(t.section,'') || char(31) || COALESCE(t.row_label,'') || char(31) || COALESCE(t.seat,''),
        char(30)
      ) as seats_raw
    FROM sales s
    JOIN tickets t ON t.id = s.ticket_id
    JOIN events e ON e.id = t.event_id
    LEFT JOIN event_categories ec ON ec.id = e.category_id
    LEFT JOIN platforms p ON p.id = s.platform_id
";

/// Powers the Sales screen's main (grouped) list - one row per sale action
/// (single ticket or multi-ticket batch) instead of one row per ticket.
/// Never pulls individual ticket rows to the frontend; all aggregation
/// happens here in SQL (see GROUP_BASE_SELECT/GROUP_KEY_EXPR above).
///
/// Filtering a grouped view is subtle: most fields (event, platform, date,
/// currency) are uniform within a real batch by construction, but a batch
/// CAN span events (ticket selection isn't restricted to one event) and
/// payment_status CAN diverge afterwards (one ticket refunded, others not).
/// So every line-level filter below is applied as "does this GROUP contain
/// at least one line matching every active filter" (the inner subquery),
/// and once a group qualifies its FULL set of lines is aggregated - never a
/// partial/undercounted group whose ticket_count wouldn't match what Sale
/// Detail then shows for the same group. `refund_status` is a true
/// group-level property (needs the whole group's refund count), so it's
/// applied as a HAVING clause after aggregation instead.
#[allow(clippy::too_many_arguments)]
fn list_sale_groups_impl(
    conn: &Connection,
    search: Option<String>,
    event_id: Option<i64>,
    platform_id: Option<i64>,
    payment_status: Option<String>,
    date_from: Option<String>,
    date_to: Option<String>,
    refund_status: Option<String>,
    // 1.8.0: appended at the END of the existing parameter list rather than
    // inserted between existing ones, on purpose - every pre-1.8.0 call site
    // (tests included) keeps its original argument order/positions and just
    // gains two trailing `None`s, instead of every position after the
    // insertion point silently shifting (which `cargo check` would catch,
    // but nothing here can run it this round - see the 1.8.0 report).
    currency: Option<String>,
    sort_by: Option<String>,
    // 2.0.27: same append-only convention as currency/sort_by right above -
    // every pre-2.0.27 call site (tests included) just gains one trailing
    // `None`.
    category_id: Option<i64>,
) -> AppResult<Vec<SaleGroup>> {
    let mut inner_sql = String::from(
        "SELECT DISTINCT COALESCE(s3.batch_id, 'single:' || s3.id) FROM sales s3
         JOIN tickets t3 ON t3.id = s3.ticket_id
         JOIN events e3 ON e3.id = t3.event_id
         JOIN orders o3 ON o3.id = t3.order_id
         WHERE 1=1",
    );
    let mut inner_params: Vec<Box<dyn rusqlite::ToSql>> = vec![];
    let mut has_line_filter = false;

    if let Some(eid) = event_id {
        inner_sql.push_str(" AND t3.event_id = ?");
        inner_params.push(Box::new(eid));
        has_line_filter = true;
    }
    if let Some(pid) = platform_id {
        inner_sql.push_str(" AND s3.platform_id = ?");
        inner_params.push(Box::new(pid));
        has_line_filter = true;
    }
    // 2.0.27: same semi-join pattern as every other line-level filter here -
    // "does this group contain at least one line whose event has this
    // category". e3 (events) is already joined in this inner query.
    if let Some(cid) = category_id {
        inner_sql.push_str(" AND e3.category_id = ?");
        inner_params.push(Box::new(cid));
        has_line_filter = true;
    }
    if let Some(ps) = payment_status.as_deref() {
        if !ps.is_empty() {
            inner_sql.push_str(" AND s3.payment_status = ?");
            inner_params.push(Box::new(ps.to_string()));
            has_line_filter = true;
        }
    }
    // 1.8.0: currency filter - "does this group contain at least one line in
    // this currency", same semi-join pattern as every other line-level
    // filter here (see the doc comment above this function). An exact match,
    // not LIKE - currency is a short code (EUR, USD, ...), never free text.
    if let Some(cur) = currency.as_deref() {
        if !cur.is_empty() {
            inner_sql.push_str(" AND s3.currency = ?");
            inner_params.push(Box::new(cur.to_string()));
            has_line_filter = true;
        }
    }
    if let Some(from) = date_from.as_deref() {
        if !from.is_empty() {
            inner_sql.push_str(" AND s3.sale_date >= ?");
            inner_params.push(Box::new(from.to_string()));
            has_line_filter = true;
        }
    }
    if let Some(to) = date_to.as_deref() {
        if !to.is_empty() {
            inner_sql.push_str(" AND s3.sale_date <= ?");
            inner_params.push(Box::new(to.to_string()));
            has_line_filter = true;
        }
    }
    if let Some(q) = search.as_deref() {
        let q = q.trim();
        if !q.is_empty() {
            // 1.8.0: also match the order code (o3.code, e.g. "ORD-001234")
            // - section 3 of the 1.8.0 brief explicitly asks that searching
            // an order code finds every sale tied to that order, the same
            // way a ticket code already finds the sale group it was sold in.
            inner_sql.push_str(
                " AND (s3.code LIKE ? OR t3.code LIKE ? OR e3.name LIKE ? OR s3.buyer_reference LIKE ? OR o3.code LIKE ?)",
            );
            let like = format!("%{q}%");
            for _ in 0..5 {
                inner_params.push(Box::new(like.clone()));
            }
            has_line_filter = true;
        }
    }

    let mut sql = format!("{GROUP_BASE_SELECT} WHERE 1=1");
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![];
    // Skip the semi-join entirely when nothing is filtered, so the common
    // "just open the Sales page" case stays exactly as cheap as before.
    if has_line_filter {
        sql.push_str(&format!(" AND {GROUP_KEY_EXPR} IN ({inner_sql})"));
        params_vec.extend(inner_params);
    }
    sql.push_str(&format!(" GROUP BY {GROUP_KEY_EXPR}"));

    if let Some(rs) = refund_status.as_deref() {
        match rs {
            "has_refund" => sql.push_str(" HAVING refunded_count > 0"),
            "no_refund" => sql.push_str(" HAVING refunded_count = 0"),
            // 1.8.0: split "has a refund" into partial vs. fully refunded -
            // section 4 of the brief asks for both as distinct filter
            // options. "has_refund" (any refund at all) is kept working
            // above too, even though the 1.8.0 frontend no longer sends it,
            // so nothing else that might call this with the old value breaks.
            "partial_refund" => sql.push_str(" HAVING refunded_count > 0 AND refunded_count < ticket_count"),
            "full_refund" => sql.push_str(" HAVING refunded_count > 0 AND refunded_count = ticket_count"),
            _ => {}
        }
    }

    // 1.8.0: sortable result (section 5 of the brief). Every arm is a
    // hardcoded literal from this whitelist - `sort_by` never gets
    // interpolated into the query - and the default (unset/unrecognized)
    // arm is byte-identical to the pre-1.8.0 hardcoded clause, so every
    // caller that doesn't pass sort_by keeps its exact previous ordering.
    // Profit has no `profit_cents` alias to sort by here (unlike
    // revenue_cents/cost_cents/selling_fees_cents, it's only computed in
    // Rust afterwards, in map_sale_group - see finance::profit_cents), so
    // it's spelled out as the same subtraction inline instead.
    //
    // 2.0.65: added "soonest"/"furthest" as explicit new values, additively -
    // marko asked for Orders/Tickets/Sales/Events/Pulls' date sort to all
    // become "Soonest/Furthest first" with soonest as the new default (see
    // REDESIGN-2.0.65-REPORT.md). Sales.tsx now sends one of these two
    // explicitly instead of ever relying on the unset-default/"oldest" pair
    // above, which are both left completely untouched - same
    // "old values keep working" convention this match already holds itself
    // to for refund_status's own "has_refund". This also sidesteps any risk
    // to the many older tests in this file that call this function with
    // sort_by left `None` and implicitly rely on today's default direction.
    let order_clause = match sort_by.as_deref() {
        Some("oldest") | Some("soonest") => "sale_date ASC, id ASC",
        Some("furthest") => "sale_date DESC, id DESC",
        Some("revenue_desc") => "revenue_cents DESC, id DESC",
        Some("revenue_asc") => "revenue_cents ASC, id DESC",
        Some("profit_desc") => "(revenue_cents - cost_cents - selling_fees_cents) DESC, id DESC",
        Some("profit_asc") => "(revenue_cents - cost_cents - selling_fees_cents) ASC, id DESC",
        Some("tickets_desc") => "ticket_count DESC, id DESC",
        _ => "sale_date DESC, id DESC",
    };
    sql.push_str(&format!(" ORDER BY {order_clause} LIMIT {LIST_CAP}"));

    let mut stmt = conn.prepare(&sql)?;
    let param_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
    let rows = stmt.query_map(param_refs.as_slice(), map_sale_group)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// See `list_sale_groups_impl` doc comment above for the filtering rationale.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn list_sale_groups(
    state: State<AppState>,
    search: Option<String>,
    event_id: Option<i64>,
    platform_id: Option<i64>,
    payment_status: Option<String>,
    date_from: Option<String>,
    date_to: Option<String>,
    refund_status: Option<String>,
    currency: Option<String>,
    sort_by: Option<String>,
    category_id: Option<i64>,
) -> AppResult<Vec<SaleGroup>> {
    let conn = state.db.lock().unwrap();
    list_sale_groups_impl(
        &conn,
        search,
        event_id,
        platform_id,
        payment_status,
        date_from,
        date_to,
        refund_status,
        currency,
        sort_by,
        category_id,
    )
}

/// 1.8.0: distinct currencies actually present in `sales`, ordered
/// alphabetically - powers the Sales screen's Currency filter dropdown so it
/// always matches real data (including any "custom" currency not in the
/// app's preferred list) instead of a hardcoded, possibly-stale list.
fn list_sale_currencies_impl(conn: &Connection) -> AppResult<Vec<String>> {
    let mut stmt = conn.prepare("SELECT DISTINCT currency FROM sales ORDER BY currency")?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[tauri::command]
pub fn list_sale_currencies(state: State<AppState>) -> AppResult<Vec<String>> {
    let conn = state.db.lock().unwrap();
    list_sale_currencies_impl(&conn)
}

/// Loads every individual ticket-line for the sale/batch that `id` belongs
/// to - i.e. exactly what Sale Detail shows once opened. `id` is normally
/// the representative id a SaleGroup row carries, but any sale id in the
/// group works the same way. Not capped by LIST_CAP: a batch is naturally
/// small (the New Sale UI is a manual pick-list), never anywhere near the
/// scale the main list guards against.
fn list_sales_by_group_impl(conn: &Connection, id: i64) -> AppResult<Vec<Sale>> {
    let batch_id: Option<String> = conn
        .query_row("SELECT batch_id FROM sales WHERE id = ?1", [id], |r| {
            r.get(0)
        })
        .map_err(|_| AppError::NotFound(format!("Sale #{id} not found")))?;

    let rows: Vec<Sale> = if let Some(b) = batch_id {
        let sql = format!("{BASE_SQL} WHERE s.batch_id = ?1 ORDER BY s.id ASC");
        let mut stmt = conn.prepare(&sql)?;
        let mapped = stmt.query_map([b], map_sale)?.collect::<Result<Vec<_>, _>>()?;
        mapped
    } else {
        let sql = format!("{BASE_SQL} WHERE s.id = ?1 ORDER BY s.id ASC");
        let mut stmt = conn.prepare(&sql)?;
        let mapped = stmt.query_map([id], map_sale)?.collect::<Result<Vec<_>, _>>()?;
        mapped
    };
    Ok(rows)
}

#[tauri::command]
pub fn list_sales_by_group(state: State<AppState>, id: i64) -> AppResult<Vec<Sale>> {
    let conn = state.db.lock().unwrap();
    list_sales_by_group_impl(&conn, id)
}

/// 1.8.0: resolves a list of SaleGroup representative ids (the ids a user
/// checked on the Sales screen) to the full flat list of underlying
/// `sales.id` rows across ALL of them - i.e. every ticket line belonging to
/// any of the selected groups, not just their representative rows. Powers
/// "Export selected" (see `export_sales_csv_selected_impl` in
/// commands/csv_export.rs) so exporting N selected groups exports every line
/// inside each one. Reuses `list_sales_by_group_impl`'s own group
/// resolution (by batch_id, or the row itself when batch_id is NULL), so
/// "the group" always means the exact same set of rows here as it does on
/// Sale Detail. Sorted and de-duplicated so passing the same group twice (or
/// two representative ids that happen to resolve to the same batch) never
/// exports a line more than once.
pub(crate) fn resolve_group_sale_ids(conn: &Connection, group_ids: &[i64]) -> AppResult<Vec<i64>> {
    let mut all_ids: Vec<i64> = Vec::new();
    for &id in group_ids {
        let lines = list_sales_by_group_impl(conn, id)?;
        all_ids.extend(lines.into_iter().map(|s| s.id));
    }
    all_ids.sort_unstable();
    all_ids.dedup();
    Ok(all_ids)
}

fn validate_new_payment_status(payment_status: Option<&str>) -> AppResult<()> {
    if let Some(ps) = payment_status {
        if ps == "refunded" {
            return Err(AppError::Validation(
                "A sale can't be created as already refunded. Record it as pending/paid, then use the Refund action.".into(),
            ));
        }
        if !["pending", "paid"].contains(&ps) {
            return Err(AppError::Validation(format!("Invalid payment status '{ps}'")));
        }
    }
    Ok(())
}

/// Core logic behind `create_sale`, taking a plain connection so it's usable
/// directly from tests without a Tauri app around it. Returns the new sale id.
pub(crate) fn create_sale_impl(conn: &mut Connection, input: &SaleInput) -> AppResult<i64> {
    if input.sale_price_cents < 0 || input.selling_fees_cents < 0 {
        return Err(AppError::Validation("Amounts cannot be negative".into()));
    }
    if input.sale_date.trim().is_empty() {
        return Err(AppError::Validation("Sale date is required".into()));
    }
    validate_new_payment_status(input.payment_status.as_deref())?;

    let tx = conn.transaction()?;

    let ticket_status: Option<String> = tx
        .query_row(
            "SELECT status FROM tickets WHERE id = ?1",
            [input.ticket_id],
            |r| r.get(0),
        )
        .ok();
    let ticket_status = ticket_status.ok_or_else(|| {
        AppError::Validation(format!("Ticket #{} does not exist", input.ticket_id))
    })?;
    if ticket_status == "sold" {
        return Err(AppError::Validation(
            "This ticket has already been sold.".into(),
        ));
    }
    if ticket_status == "cancelled" {
        return Err(AppError::Validation(
            "This ticket is cancelled and cannot be sold.".into(),
        ));
    }

    let (currency,): (String,) = tx.query_row(
        "SELECT currency FROM tickets WHERE id = ?1",
        [input.ticket_id],
        |r| Ok((r.get(0)?,)),
    )?;

    let code = codes::next_code(&tx, "sale", "SAL")?;
    let payment_status = input
        .payment_status
        .clone()
        .unwrap_or_else(|| "pending".to_string());

    let insert_result = tx.execute(
        "INSERT INTO sales (code, ticket_id, platform_id, sale_date, sale_price_cents,
           selling_fees_cents, currency, payment_status, buyer_reference, notes)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        params![
            code,
            input.ticket_id,
            input.platform_id,
            input.sale_date,
            input.sale_price_cents,
            input.selling_fees_cents,
            currency,
            payment_status,
            input.buyer_reference,
            input.notes,
        ],
    );
    match insert_result {
        Ok(_) => {}
        Err(rusqlite::Error::SqliteFailure(_, Some(m))) if m.contains("UNIQUE") => {
            return Err(AppError::Validation(
                "This ticket has already been sold.".into(),
            ));
        }
        Err(e) => return Err(AppError::from(e)),
    }
    let sale_id = tx.last_insert_rowid();

    tx.execute(
        "UPDATE tickets SET status='sold', updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",
        [input.ticket_id],
    )?;

    tx.commit()?;
    Ok(sale_id)
}

#[tauri::command]
pub fn create_sale(state: State<AppState>, input: SaleInput) -> AppResult<Sale> {
    let mut conn = state.db.lock().unwrap();
    let sale_id = create_sale_impl(&mut conn, &input)?;
    fetch_one(&conn, sale_id)
}

/// Core logic behind `create_sales_batch` - see that command's doc comment
/// for the "why one sale row per ticket" rationale. Returns the new sale ids.
pub(crate) fn create_sales_batch_impl(conn: &mut Connection, input: &SaleBatchInput) -> AppResult<Vec<i64>> {
    if input.lines.is_empty() {
        return Err(AppError::Validation(
            "Select at least one ticket to sell".into(),
        ));
    }
    if input.sale_date.trim().is_empty() {
        return Err(AppError::Validation("Sale date is required".into()));
    }
    for line in &input.lines {
        if line.sale_price_cents < 0 || line.selling_fees_cents < 0 {
            return Err(AppError::Validation("Amounts cannot be negative".into()));
        }
    }
    // 2.0.57: mirrors OrderInput's own `if input.currency.trim().is_empty()`
    // guard (orders.rs) - only checked when the caller actually sent an
    // explicit currency at all; `None` (the pre-2.0.57 "derive per ticket"
    // path) has nothing to validate here.
    if let Some(c) = &input.currency {
        if c.trim().is_empty() {
            return Err(AppError::Validation("Currency cannot be empty".into()));
        }
    }
    validate_new_payment_status(input.payment_status.as_deref())?;
    {
        let mut seen = HashSet::new();
        for line in &input.lines {
            if !seen.insert(line.ticket_id) {
                return Err(AppError::Validation(
                    "The same ticket was selected twice in this sale".into(),
                ));
            }
        }
    }

    let tx = conn.transaction()?;

    let payment_status = input
        .payment_status
        .clone()
        .unwrap_or_else(|| "pending".to_string());
    let codes_batch = codes::next_code_batch(&tx, "sale", "SAL", input.lines.len() as i64)?;
    // Only a real multi-ticket batch gets a batch_id (using its own first
    // code as the shared identifier - codes are sequential, so it's always
    // the lowest code in the group). A batch of one line is just an ordinary
    // single-ticket sale and stays NULL, same as create_sale.
    let batch_id: Option<&str> = if codes_batch.len() > 1 {
        Some(codes_batch[0].as_str())
    } else {
        None
    };

    let mut sale_ids = Vec::with_capacity(input.lines.len());

    for (line, code) in input.lines.iter().zip(codes_batch.iter()) {
        let ticket_status: Option<String> = tx
            .query_row(
                "SELECT status FROM tickets WHERE id = ?1",
                [line.ticket_id],
                |r| r.get(0),
            )
            .ok();
        let ticket_status = ticket_status.ok_or_else(|| {
            AppError::Validation(format!("Ticket #{} does not exist", line.ticket_id))
        })?;
        if ticket_status == "sold" {
            return Err(AppError::Validation(
                "One of the selected tickets has already been sold - nothing in this sale was saved. Remove it and try again.".into(),
            ));
        }
        if ticket_status == "cancelled" {
            return Err(AppError::Validation(
                "One of the selected tickets is cancelled and cannot be sold - nothing in this sale was saved. Remove it and try again.".into(),
            ));
        }

        // 2.0.57: an explicit `input.currency` (New Sale's own currency
        // picker) wins for every line in the batch; omitting it falls back
        // to the original per-ticket lookup - see `SaleBatchInput::currency`'s
        // own doc comment for exactly who still relies on that fallback.
        let currency: String = match &input.currency {
            Some(c) => c.trim().to_string(),
            None => tx.query_row(
                "SELECT currency FROM tickets WHERE id = ?1",
                [line.ticket_id],
                |r| r.get(0),
            )?,
        };

        let insert_result = tx.execute(
            "INSERT INTO sales (code, ticket_id, platform_id, sale_date, sale_price_cents,
               selling_fees_cents, currency, payment_status, buyer_reference, notes, batch_id)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            params![
                code,
                line.ticket_id,
                input.platform_id,
                input.sale_date,
                line.sale_price_cents,
                line.selling_fees_cents,
                currency,
                payment_status,
                input.buyer_reference,
                input.notes,
                batch_id,
            ],
        );
        match insert_result {
            Ok(_) => {}
            Err(rusqlite::Error::SqliteFailure(_, Some(m))) if m.contains("UNIQUE") => {
                return Err(AppError::Validation(
                    "One of the selected tickets has already been sold - nothing in this sale was saved. Remove it and try again.".into(),
                ));
            }
            Err(e) => return Err(AppError::from(e)),
        }
        sale_ids.push(tx.last_insert_rowid());

        tx.execute(
            "UPDATE tickets SET status='sold', updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",
            [line.ticket_id],
        )?;
    }

    tx.commit()?;
    Ok(sale_ids)
}

/// Records a sale that can cover multiple tickets in one action (e.g. a
/// block of seats sold together to the same buyer). Every ticket still gets
/// its own `sales` row - so per-ticket revenue/cost/profit/margin/ROI stay
/// exact - but all rows share the buyer/platform/date/payment details from
/// `input` and are inserted in a single all-or-nothing transaction: if any
/// selected ticket turns out to be unavailable, nothing in the batch is
/// saved. This is also the path used for a single-ticket sale (a batch of
/// one line), so there is only one code path to keep correct.
///
/// 2.0.57: `input.currency`, when set, records every line in the batch in
/// that ONE currency, chosen by marko in the New Sale form - independent of
/// whatever currency the ticket(s) being sold were themselves bought in
/// (e.g. a ticket bought in USD but paid out by the marketplace in EUR).
/// Left `None`, the original behaviour applies: each line silently copies
/// its own ticket's purchase currency, exactly as before 2.0.57. Deliberately
/// only settable at the moment a sale is created here, never editable
/// afterward - the same creation-time-only scope `fx.rs`'s own doc comment
/// already establishes for order currency.
#[tauri::command]
pub fn create_sales_batch(state: State<AppState>, input: SaleBatchInput) -> AppResult<Vec<Sale>> {
    let mut conn = state.db.lock().unwrap();
    let sale_ids = create_sales_batch_impl(&mut conn, &input)?;
    sale_ids.into_iter().map(|id| fetch_one(&conn, id)).collect()
}

/// Core logic behind `update_sale`. A refunded sale is locked - its history
/// must stay exactly as it was at the moment of refund - and this path can
/// never itself set/clear the refunded state (that's `refund_sale_impl`'s
/// job, since it has ticket-status side effects a plain field edit must not).
fn update_sale_impl(conn: &Connection, id: i64, input: &SaleEditInput) -> AppResult<()> {
    if input.sale_price_cents < 0 || input.selling_fees_cents < 0 {
        return Err(AppError::Validation("Amounts cannot be negative".into()));
    }
    if !["pending", "paid"].contains(&input.payment_status.as_str()) {
        return Err(AppError::Validation(
            "Use the Refund action to refund a sale - payment status here can only be pending or paid.".into(),
        ));
    }
    let current_status: String = conn
        .query_row(
            "SELECT payment_status FROM sales WHERE id = ?1",
            [id],
            |r| r.get(0),
        )
        .map_err(|_| AppError::NotFound(format!("Sale #{id} not found")))?;
    if current_status == "refunded" {
        return Err(AppError::Validation(
            "This sale has been refunded and can no longer be edited.".into(),
        ));
    }

    let changed = conn.execute(
        "UPDATE sales SET platform_id=?1, sale_date=?2, sale_price_cents=?3, selling_fees_cents=?4,
         payment_status=?5, buyer_reference=?6, notes=?7, updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now')
         WHERE id=?8",
        params![
            input.platform_id,
            input.sale_date,
            input.sale_price_cents,
            input.selling_fees_cents,
            input.payment_status,
            input.buyer_reference,
            input.notes,
            id,
        ],
    )?;
    if changed == 0 {
        return Err(AppError::NotFound(format!("Sale #{id} not found")));
    }
    Ok(())
}

#[tauri::command]
pub fn update_sale(state: State<AppState>, id: i64, input: SaleEditInput) -> AppResult<Sale> {
    let conn = state.db.lock().unwrap();
    update_sale_impl(&conn, id, &input)?;
    fetch_one(&conn, id)
}

/// Core logic behind `bulk_update_sale_payment_status` (1.9.2): set many
/// sales' `payment_status` to "pending" or "paid" in one all-or-nothing
/// transaction. Deliberately narrower than `update_sale_impl`: it only ever
/// touches `payment_status` (never `sale_price_cents`/`platform_id`/etc, and
/// never `tickets.status`), so it's safe to expose as a single small action
/// next to Sale Detail's selection checkboxes rather than the old general
/// Bulk Ticket Edit bar (removed from Sale Detail this round - see
/// SaleDetail.tsx).
///
/// Same "validate everything, then write everything" shape as
/// `bulk_update_tickets_impl` (tickets.rs): every id is checked to exist AND
/// to not already be `refunded` BEFORE any row is written, so one bad id in
/// the batch changes nothing at all rather than partially applying. A
/// refunded sale is locked (see `update_sale_impl`'s doc comment above) - it
/// must never be silently flipped back to pending/paid by a bulk action just
/// because it happened to be selected alongside valid rows.
pub(crate) fn bulk_update_sale_payment_status_impl(
    conn: &mut Connection,
    sale_ids: &[i64],
    payment_status: &str,
) -> AppResult<Vec<i64>> {
    if sale_ids.is_empty() {
        return Err(AppError::Validation(
            "Select at least one sale to update".into(),
        ));
    }
    if !["pending", "paid"].contains(&payment_status) {
        return Err(AppError::Validation(
            "Use the Refund action to refund a sale - bulk status here can only be pending or paid.".into(),
        ));
    }

    // Dedupe so the same id selected twice (e.g. a stale double click) is
    // applied once, not treated as two separate writes - same convention as
    // bulk_update_tickets_impl.
    let mut ids: Vec<i64> = Vec::new();
    {
        let mut seen = HashSet::new();
        for &id in sale_ids {
            if seen.insert(id) {
                ids.push(id);
            }
        }
    }

    let tx = conn.transaction()?;

    // Validate every id exists AND is not already refunded BEFORE writing
    // anything - all-or-nothing. One query, not one per id (same technique
    // as bulk_update_tickets_impl's own validation step).
    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let existing: std::collections::HashMap<i64, String> = {
        let mut stmt = tx.prepare(&format!(
            "SELECT id, payment_status FROM sales WHERE id IN ({placeholders})"
        ))?;
        let rows = stmt.query_map(rusqlite::params_from_iter(ids.iter()), |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
        })?;
        rows.collect::<Result<std::collections::HashMap<_, _>, _>>()?
    };
    if let Some(missing) = ids.iter().copied().find(|id| !existing.contains_key(id)) {
        return Err(AppError::Validation(format!("Sale #{missing} does not exist")));
    }
    if existing.values().any(|status| status == "refunded") {
        return Err(AppError::Validation(
            "One of the selected sales has been refunded and can no longer be edited - nothing was changed. Deselect it and try again.".into(),
        ));
    }

    let sql = format!(
        "UPDATE sales SET payment_status = ?1, updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id IN ({placeholders})"
    );
    let mut update_params: Vec<Box<dyn rusqlite::ToSql>> = Vec::with_capacity(ids.len() + 1);
    update_params.push(Box::new(payment_status.to_string()));
    for &id in &ids {
        update_params.push(Box::new(id));
    }
    let update_refs: Vec<&dyn rusqlite::ToSql> = update_params.iter().map(|p| p.as_ref()).collect();
    tx.execute(&sql, update_refs.as_slice())?;

    tx.commit()?;
    Ok(ids)
}

/// Sets `payment_status` (pending/paid only - see the impl's doc comment) for
/// many sales at once, e.g. marking a whole batch as paid once a buyer
/// settles up. Lives next to Sale Detail's selection checkboxes as a single
/// small action - replaces the old general Bulk Ticket Edit bar there, which
/// this round removed in favor of individual per-ticket editing for
/// Section/Row/Seat/Listing price (see SaleDetail.tsx / BulkTicketEditBar.tsx
/// doc comments). Returns the updated sales, refetched the same way
/// `create_sales_batch` does.
#[tauri::command]
pub fn bulk_update_sale_payment_status(
    state: State<AppState>,
    input: BulkSalePaymentStatusInput,
) -> AppResult<Vec<Sale>> {
    let mut conn = state.db.lock().unwrap();
    let ids = bulk_update_sale_payment_status_impl(&mut conn, &input.sale_ids, &input.payment_status)?;
    ids.into_iter().map(|id| fetch_one(&conn, id)).collect()
}

/// Core logic behind `refund_sale`. Atomic: the sale becomes `refunded`
/// (with a timestamp and optional reason) and its ticket returns to
/// `available` in the same transaction, so it's never possible to observe
/// "sale=refunded, ticket=sold" or a refunded sale still counted as an
/// available-for-resale ticket being stuck as sold forever. The sale row
/// itself is never touched again after this - see `update_sale_impl`.
pub(crate) fn refund_sale_impl(conn: &mut Connection, id: i64, reason: Option<&str>) -> AppResult<()> {
    let tx = conn.transaction()?;

    let row: Option<(i64, String)> = tx
        .query_row(
            "SELECT ticket_id, payment_status FROM sales WHERE id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .ok();
    let (ticket_id, payment_status) =
        row.ok_or_else(|| AppError::NotFound(format!("Sale #{id} not found")))?;
    if payment_status == "refunded" {
        return Err(AppError::Validation(
            "This sale has already been refunded.".into(),
        ));
    }

    let reason = reason.map(|r| r.trim()).filter(|r| !r.is_empty());

    tx.execute(
        "UPDATE sales SET payment_status='refunded',
           refunded_at=strftime('%Y-%m-%dT%H:%M:%fZ','now'),
           refund_reason=?1,
           updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now')
         WHERE id=?2",
        params![reason, id],
    )?;
    tx.execute(
        "UPDATE tickets SET status='available', updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",
        [ticket_id],
    )?;

    tx.commit()?;
    Ok(())
}

/// Refunds a sale: the sale stays in the database forever (history is never
/// lost) with `paymentStatus` flipped to `refunded`, and its ticket returns
/// to inventory as `available` so it can be sold again. Both changes commit
/// together or not at all. Once refunded, a sale can no longer be edited or
/// refunded again - see `update_sale_impl` / the guard above.
#[tauri::command]
pub fn refund_sale(state: State<AppState>, id: i64, reason: Option<String>) -> AppResult<Sale> {
    let mut conn = state.db.lock().unwrap();
    refund_sale_impl(&mut conn, id, reason.as_deref())?;
    fetch_one(&conn, id)
}

/// Core logic behind `delete_sale`. Two cases, handled differently:
///
/// - A non-refunded sale is a genuine mistake to undo (e.g. the wrong ticket
///   was picked) - the row is removed and the ticket goes back to
///   `available`, because this WAS the ticket's one and only active sale
///   (migration 004's partial unique index guarantees at most one active,
///   non-refunded sale can ever exist per ticket at a time).
///
/// - A refunded sale is historical record-keeping. Originally this function
///   refused to delete one at all, so refund history could never be lost.
///   Marko explicitly asked to relax that (2026-08 - he needed to clear out
///   test data, and accepted this also permanently changes real-world
///   behavior) so it's now allowed - but deleting a refunded row must NEVER
///   touch `tickets.status`. A refunded row does not own the ticket's
///   current state: the ticket may already be `available` (refund already
///   did that - nothing left to do) or it may have been resold since, under
///   a *different*, newer active sale row (still correctly `sold` under
///   that other row). Touching status here would either be a no-op or,
///   worse, silently un-sell a currently-sold ticket out from under its
///   real active sale. Only a ticket's own active sale (the non-refunded
///   branch below) is ever allowed to change its status.
fn delete_sale_impl(conn: &mut Connection, id: i64) -> AppResult<()> {
    let tx = conn.transaction()?;
    let row: Option<(i64, String)> = tx
        .query_row(
            "SELECT ticket_id, payment_status FROM sales WHERE id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .ok();
    let (ticket_id, payment_status) =
        row.ok_or_else(|| AppError::NotFound(format!("Sale #{id} not found")))?;

    tx.execute("DELETE FROM sales WHERE id = ?1", [id])?;
    if payment_status != "refunded" {
        // This was the ticket's one active sale - see the doc comment above
        // for why a refunded row must never reach this branch.
        tx.execute(
            "UPDATE tickets SET status='available', updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",
            [ticket_id],
        )?;
    }
    tx.commit()?;
    Ok(())
}

#[tauri::command]
pub fn delete_sale(state: State<AppState>, id: i64) -> AppResult<()> {
    let mut conn = state.db.lock().unwrap();
    delete_sale_impl(&mut conn, id)
}

/// Core logic behind `delete_sale_group` - the "Delete entire sale" action
/// on Sale Detail (1.7.3), for when the whole sale should go away at once
/// instead of removing lines one at a time. Resolves "the group" exactly
/// like `list_sales_by_group_impl`: every row sharing this sale's
/// `batch_id`, or just this one row if it has no `batch_id` (a plain
/// single-ticket sale) - so "the sale you're looking at" always means the
/// same set of rows here as it does on the page itself.
///
/// Every row in the group is deleted inside ONE transaction, applying the
/// exact same per-line rule `delete_sale_impl` uses: a non-refunded (active)
/// line resets its ticket to `available` (it was that ticket's one active
/// sale); a refunded line never touches ticket status, because that ticket
/// may have been resold since under a *different*, newer sale outside this
/// group entirely (see `delete_sale_impl`'s doc comment for the full
/// reasoning - it applies per-ticket, not per-group). Doing the whole group
/// in one transaction means a mid-way failure can never leave half a sale
/// deleted and half still on record.
/// Core per-group delete logic, operating directly on the given transaction
/// with no transaction boundary of its own. Split out of
/// `delete_sale_group_impl` in 2.0.28 so `bulk_delete_sale_groups_impl` can
/// delete N selected groups inside ONE shared transaction - exactly as
/// atomic as deleting them one at a time would have been, just reported back
/// as a single summary instead of N separate round trips.
/// `delete_sale_group_impl` below wraps one call to this in its own
/// transaction, unchanged in behavior from before this split.
fn delete_sale_group_rows(tx: &Transaction, id: i64) -> AppResult<usize> {
    let batch_id: Option<String> = tx
        .query_row("SELECT batch_id FROM sales WHERE id = ?1", [id], |r| {
            r.get(0)
        })
        .map_err(|_| AppError::NotFound(format!("Sale #{id} not found")))?;

    let rows: Vec<(i64, i64, String)> = if let Some(b) = &batch_id {
        let mut stmt =
            tx.prepare("SELECT id, ticket_id, payment_status FROM sales WHERE batch_id = ?1")?;
        let mapped = stmt
            .query_map([b], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        mapped
    } else {
        let mut stmt = tx.prepare("SELECT id, ticket_id, payment_status FROM sales WHERE id = ?1")?;
        let mapped = stmt
            .query_map([id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        mapped
    };

    let count = rows.len();
    for (sale_id, ticket_id, payment_status) in &rows {
        tx.execute("DELETE FROM sales WHERE id = ?1", [sale_id])?;
        if payment_status != "refunded" {
            tx.execute(
                "UPDATE tickets SET status='available', updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",
                [ticket_id],
            )?;
        }
    }

    Ok(count)
}

fn delete_sale_group_impl(conn: &mut Connection, id: i64) -> AppResult<usize> {
    let tx = conn.transaction()?;
    let count = delete_sale_group_rows(&tx, id)?;
    tx.commit()?;
    Ok(count)
}

/// Deletes an entire sale (every line in its batch, or the single line
/// itself) in one atomic transaction. Returns how many sale lines were
/// removed, so the frontend can confirm exactly what happened.
#[tauri::command]
pub fn delete_sale_group(state: State<AppState>, id: i64) -> AppResult<usize> {
    let mut conn = state.db.lock().unwrap();
    delete_sale_group_impl(&mut conn, id)
}

/// 2.0.28: bulk delete for the new "Delete" selection mode on the Sales
/// list (one row there = one sale group/batch, same as `list_sale_groups`
/// already shows - selected ids are group anchor ids, exactly what the list
/// already uses to link to Sale Detail). Unlike orders/events, a sale group
/// has NO precondition blocking its deletion at all -
/// `delete_sale_group_impl` above already allows removing an active
/// (non-refunded) sale just as freely as a refunded one (marko explicitly
/// asked for that relaxation, 2026-08 - see `delete_sale_impl`'s own doc
/// comment for the full history), so the only way a selected id can be
/// skipped here is if it no longer resolves to any sale at all. Every
/// selected group is deleted inside ONE shared transaction via
/// `delete_sale_group_rows`, exactly as atomic as doing them one at a time
/// would have been.
pub(crate) fn bulk_delete_sale_groups_impl(conn: &mut Connection, ids: &[i64]) -> AppResult<BulkDeleteResult> {
    if ids.is_empty() {
        return Err(AppError::Validation("Select at least one sale to delete".into()));
    }
    let tx = conn.transaction()?;
    let mut deleted_ids = Vec::new();
    let mut skipped = Vec::new();
    for &id in ids {
        match delete_sale_group_rows(&tx, id) {
            Ok(_) => deleted_ids.push(id),
            Err(AppError::NotFound(_)) => skipped.push(BulkDeleteSkip {
                id,
                reason: "Not found - already deleted?".into(),
            }),
            Err(e) => return Err(e),
        }
    }
    tx.commit()?;
    Ok(BulkDeleteResult { deleted_ids, skipped })
}

#[tauri::command]
pub fn bulk_delete_sale_groups(state: State<AppState>, ids: Vec<i64>) -> AppResult<BulkDeleteResult> {
    let mut conn = state.db.lock().unwrap();
    bulk_delete_sale_groups_impl(&mut conn, &ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_conn;
    use crate::models::OrderInput;

    /// Creates one event, one order with `qty` tickets (all EUR, 1000 cents
    /// cost each), and returns the ticket ids in creation order.
    fn seed_tickets(conn: &mut Connection, qty: i64) -> Vec<i64> {
        conn.execute(
            "INSERT INTO events (name) VALUES ('Test Event')",
            [],
        )
        .unwrap();
        let event_id = conn.last_insert_rowid();
        let input = OrderInput {
            event_id,
            supplier_id: None,
            platform_id: None,
            purchase_date: "2026-01-01".to_string(),
            quantity: qty,
            unit_price_cents: 1000,
            fees_cents: 0,
            other_costs_cents: 0,
            currency: "EUR".to_string(),
            payment_status: Some("paid".to_string()),
            notes: None,
            ticket_type: None,
            section: None,
            row_label: None,
            seats: None,
        };
        let order_id =
            crate::commands::orders::insert_order_with_tickets(conn, &input, false).unwrap();
        let mut stmt = conn
            .prepare("SELECT id FROM tickets WHERE order_id = ?1 ORDER BY id")
            .unwrap();
        let ids = stmt
            .query_map([order_id], |r| r.get::<_, i64>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        ids
    }

    fn ticket_status(conn: &Connection, ticket_id: i64) -> String {
        conn.query_row(
            "SELECT status FROM tickets WHERE id = ?1",
            [ticket_id],
            |r| r.get(0),
        )
        .unwrap()
    }

    fn sale_payment_status(conn: &Connection, sale_id: i64) -> String {
        conn.query_row(
            "SELECT payment_status FROM sales WHERE id = ?1",
            [sale_id],
            |r| r.get(0),
        )
        .unwrap()
    }

    fn sale_input(ticket_id: i64, price_cents: i64) -> SaleInput {
        SaleInput {
            ticket_id,
            platform_id: None,
            sale_date: "2026-02-01".to_string(),
            sale_price_cents: price_cents,
            selling_fees_cents: 0,
            payment_status: Some("paid".to_string()),
            buyer_reference: None,
            notes: None,
        }
    }

    /// Like `seed_tickets`, but lets the caller set section/row_label/seats
    /// so 2.0.38's new `SaleGroup.seats` field has something real to check -
    /// `seed_tickets` itself always leaves every ticket's seat fields NULL.
    fn seed_tickets_with_seats(conn: &mut Connection, seats: &[&str], section: &str, row_label: &str) -> Vec<i64> {
        conn.execute("INSERT INTO events (name) VALUES ('Test Event')", []).unwrap();
        let event_id = conn.last_insert_rowid();
        let input = OrderInput {
            event_id,
            supplier_id: None,
            platform_id: None,
            purchase_date: "2026-01-01".to_string(),
            quantity: seats.len() as i64,
            unit_price_cents: 1000,
            fees_cents: 0,
            other_costs_cents: 0,
            currency: "EUR".to_string(),
            payment_status: Some("paid".to_string()),
            notes: None,
            ticket_type: None,
            section: Some(section.to_string()),
            row_label: Some(row_label.to_string()),
            seats: Some(seats.iter().map(|s| s.to_string()).collect()),
        };
        let order_id = crate::commands::orders::insert_order_with_tickets(conn, &input, false).unwrap();
        let mut stmt = conn.prepare("SELECT id FROM tickets WHERE order_id = ?1 ORDER BY id").unwrap();
        stmt.query_map([order_id], |r| r.get::<_, i64>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    }

    // 2.0.38: exercises the REAL SQL (GROUP_CONCAT + char(30)/char(31), added
    // to GROUP_BASE_SELECT for the new SaleGroup.seats field) against a real
    // connection - see orders.rs's identical-purpose test for why this can't
    // be caught by SeatEntry::parse_aggregate's own pure-Rust unit tests
    // alone. HashSet comparison is deliberately order-independent, same
    // reasoning as that test.
    #[test]
    fn a_batch_sale_reports_the_seats_of_exactly_the_tickets_it_sold() {
        let mut conn = test_conn();
        let tickets = seed_tickets_with_seats(&mut conn, &["11", "12", "13"], "A", "1");
        let input = SaleBatchInput {
            lines: tickets
                .iter()
                .map(|&tid| crate::models::SaleBatchLineInput { ticket_id: tid, sale_price_cents: 2000, selling_fees_cents: 0 })
                .collect(),
            platform_id: None,
            sale_date: "2026-03-01".to_string(),
            payment_status: Some("paid".to_string()),
            buyer_reference: None,
            notes: None,
            currency: None,
        };
        create_sales_batch_impl(&mut conn, &input).unwrap();

        let groups =
            list_sale_groups_impl(&conn, None, None, None, None, None, None, None, None, None, None).unwrap();
        assert_eq!(groups.len(), 1, "all 3 lines were submitted together - one group");
        assert_eq!(groups[0].ticket_count, 3);

        let got: std::collections::HashSet<_> = groups[0].seats.iter().cloned().collect();
        let expected: std::collections::HashSet<_> = ["11", "12", "13"]
            .into_iter()
            .map(|seat| crate::models::SeatEntry {
                section: Some("A".to_string()),
                row_label: Some("1".to_string()),
                seat: Some(seat.to_string()),
            })
            .collect();
        assert_eq!(got, expected);
    }

    #[test]
    fn refund_returns_ticket_and_excludes_from_revenue() {
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 1);
        let sale_id = create_sale_impl(&mut conn, &sale_input(tickets[0], 2500)).unwrap();

        assert_eq!(ticket_status(&conn, tickets[0]), "sold");
        let revenue_before: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(sale_price_cents),0) FROM sales WHERE payment_status != 'refunded'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(revenue_before, 2500);

        refund_sale_impl(&mut conn, sale_id, Some("buyer changed their mind")).unwrap();

        // Ticket back in inventory.
        assert_eq!(ticket_status(&conn, tickets[0]), "available");
        // Sale still exists (history preserved), flagged refunded.
        let (status, refunded_at, reason): (String, Option<String>, Option<String>) = conn
            .query_row(
                "SELECT payment_status, refunded_at, refund_reason FROM sales WHERE id = ?1",
                [sale_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(status, "refunded");
        assert!(refunded_at.is_some());
        assert_eq!(reason.as_deref(), Some("buyer changed their mind"));
        // Excluded from revenue now.
        let revenue_after: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(sale_price_cents),0) FROM sales WHERE payment_status != 'refunded'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(revenue_after, 0);
    }

    #[test]
    fn cannot_refund_twice() {
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 1);
        let sale_id = create_sale_impl(&mut conn, &sale_input(tickets[0], 1000)).unwrap();
        refund_sale_impl(&mut conn, sale_id, None).unwrap();
        let err = refund_sale_impl(&mut conn, sale_id, None).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
        // Still available, not flipped back to sold or anything odd.
        assert_eq!(ticket_status(&conn, tickets[0]), "available");
    }

    #[test]
    fn cannot_edit_a_refunded_sale() {
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 1);
        let sale_id = create_sale_impl(&mut conn, &sale_input(tickets[0], 1000)).unwrap();
        refund_sale_impl(&mut conn, sale_id, None).unwrap();

        let edit = SaleEditInput {
            platform_id: None,
            sale_date: "2026-02-02".to_string(),
            sale_price_cents: 9999,
            selling_fees_cents: 0,
            payment_status: "paid".to_string(),
            buyer_reference: None,
            notes: None,
        };
        let err = update_sale_impl(&conn, sale_id, &edit).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn cannot_set_refunded_via_plain_edit_or_create() {
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 1);
        let sale_id = create_sale_impl(&mut conn, &sale_input(tickets[0], 1000)).unwrap();

        let edit = SaleEditInput {
            platform_id: None,
            sale_date: "2026-02-02".to_string(),
            sale_price_cents: 1000,
            selling_fees_cents: 0,
            payment_status: "refunded".to_string(),
            buyer_reference: None,
            notes: None,
        };
        assert!(matches!(
            update_sale_impl(&conn, sale_id, &edit),
            Err(AppError::Validation(_))
        ));
        // Ticket untouched by the rejected attempt.
        assert_eq!(ticket_status(&conn, tickets[0]), "sold");

        let mut refunded_input = sale_input(tickets[0], 1000);
        refunded_input.payment_status = Some("refunded".to_string());
        assert!(matches!(
            create_sale_impl(&mut conn, &refunded_input),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn bulk_sale_of_one_five_and_hundred_tickets() {
        for &n in &[1usize, 5, 100] {
            let mut conn = test_conn();
            let tickets = seed_tickets(&mut conn, n as i64);
            let input = SaleBatchInput {
                lines: tickets
                    .iter()
                    .map(|&tid| crate::models::SaleBatchLineInput {
                        ticket_id: tid,
                        sale_price_cents: 2000,
                        selling_fees_cents: 100,
                    })
                    .collect(),
                platform_id: None,
                sale_date: "2026-03-01".to_string(),
                payment_status: Some("paid".to_string()),
                buyer_reference: Some("Buyer X".to_string()),
                currency: None,
                notes: None,
            };
            let ids = create_sales_batch_impl(&mut conn, &input).unwrap();
            assert_eq!(ids.len(), n, "expected {n} sales rows");

            // Every ticket sold, unique sale codes, correct per-line totals.
            for &tid in &tickets {
                assert_eq!(ticket_status(&conn, tid), "sold");
            }
            let distinct_codes: i64 = conn
                .query_row("SELECT COUNT(DISTINCT code) FROM sales", [], |r| r.get(0))
                .unwrap();
            assert_eq!(distinct_codes, n as i64);
            let total_revenue: i64 = conn
                .query_row("SELECT COALESCE(SUM(sale_price_cents),0) FROM sales", [], |r| {
                    r.get(0)
                })
                .unwrap();
            assert_eq!(total_revenue, 2000 * n as i64);
        }
    }

    #[test]
    fn bulk_sale_rejects_duplicate_ticket_in_same_batch() {
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 1);
        let input = SaleBatchInput {
            lines: vec![
                crate::models::SaleBatchLineInput {
                    ticket_id: tickets[0],
                    sale_price_cents: 1000,
                    selling_fees_cents: 0,
                },
                crate::models::SaleBatchLineInput {
                    ticket_id: tickets[0],
                    sale_price_cents: 1000,
                    selling_fees_cents: 0,
                },
            ],
            platform_id: None,
            sale_date: "2026-03-01".to_string(),
            payment_status: None,
            buyer_reference: None,
            notes: None,
            currency: None,
        };
        assert!(create_sales_batch_impl(&mut conn, &input).is_err());
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM sales", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0, "nothing should have been saved");
    }

    #[test]
    fn bulk_sale_is_all_or_nothing_when_one_ticket_is_already_sold() {
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 3);
        // Sell the second ticket ahead of time via the normal single-sale path.
        create_sale_impl(&mut conn, &sale_input(tickets[1], 500)).unwrap();

        let input = SaleBatchInput {
            lines: tickets
                .iter()
                .map(|&tid| crate::models::SaleBatchLineInput {
                    ticket_id: tid,
                    sale_price_cents: 1500,
                    selling_fees_cents: 0,
                })
                .collect(),
            platform_id: None,
            sale_date: "2026-03-01".to_string(),
            payment_status: Some("pending".to_string()),
            buyer_reference: None,
            notes: None,
            currency: None,
        };
        assert!(create_sales_batch_impl(&mut conn, &input).is_err());

        // Only the one pre-existing sale exists - the batch made no partial writes.
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM sales", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
        assert_eq!(ticket_status(&conn, tickets[0]), "available");
        assert_eq!(ticket_status(&conn, tickets[2]), "available");
    }

    #[test]
    fn refunded_sale_can_now_be_deleted_and_does_not_touch_an_already_available_ticket() {
        // Marko explicitly asked (2026-08) to relax the original BUG #1 fix's
        // guarantee that a refunded sale can never be deleted - he needed to
        // clear out test data, and understood this also permanently changes
        // real-world behavior going forward, not just for testing.
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 1);
        let sale_id = create_sale_impl(&mut conn, &sale_input(tickets[0], 1000)).unwrap();
        refund_sale_impl(&mut conn, sale_id, None).unwrap();
        assert_eq!(ticket_status(&conn, tickets[0]), "available");

        delete_sale_impl(&mut conn, sale_id).unwrap();

        let still_present: i64 = conn
            .query_row("SELECT COUNT(*) FROM sales WHERE id = ?1", [sale_id], |r| r.get(0))
            .unwrap();
        assert_eq!(still_present, 0, "refunded sale row can now be removed");
        assert_eq!(
            ticket_status(&conn, tickets[0]),
            "available",
            "ticket was already available from the refund - deleting the refund record must not change that"
        );
    }

    #[test]
    fn deleting_an_old_refunded_sale_never_disturbs_a_newer_active_sale_on_the_same_ticket() {
        // The critical safety property of allowing refunded-sale deletion: a
        // ticket can carry BOTH an old refunded sale and a newer active one
        // (migration 004's partial unique index). Deleting the old refunded
        // row must leave the ticket exactly as the *active* sale says it
        // should be - never reset to Available out from under a ticket that
        // is genuinely sold right now under a different sale row.
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 1);
        let ticket_id = tickets[0];

        let refunded_id = create_sale_impl(&mut conn, &sale_input(ticket_id, 2000)).unwrap();
        refund_sale_impl(&mut conn, refunded_id, None).unwrap();
        let active_id = create_sale_impl(&mut conn, &sale_input(ticket_id, 1800)).unwrap();
        assert_eq!(ticket_status(&conn, ticket_id), "sold");

        delete_sale_impl(&mut conn, refunded_id).unwrap();

        assert_eq!(
            ticket_status(&conn, ticket_id),
            "sold",
            "deleting the old refunded record must not un-sell a ticket that is actively sold under a different sale"
        );
        let active_still_present: i64 = conn
            .query_row("SELECT COUNT(*) FROM sales WHERE id = ?1", [active_id], |r| r.get(0))
            .unwrap();
        assert_eq!(active_still_present, 1, "the newer active sale must be untouched");
    }

    // ---- 1.7.3: "Delete entire sale" (delete_sale_group_impl) ------------

    #[test]
    fn delete_sale_group_removes_every_line_in_a_batch_and_resets_only_non_refunded_tickets() {
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 3);
        let ids = create_sales_batch_impl(&mut conn, &batch_input(&tickets, 1000, "paid")).unwrap();
        assert_eq!(ids.len(), 3);

        // One line refunded before the group delete, so the group is a
        // realistic mix - exactly the case the per-line rule exists for.
        refund_sale_impl(&mut conn, ids[0], None).unwrap();
        assert_eq!(ticket_status(&conn, tickets[0]), "available"); // refunded
        assert_eq!(ticket_status(&conn, tickets[1]), "sold");
        assert_eq!(ticket_status(&conn, tickets[2]), "sold");

        // Resolve via ids[1] (a non-refunded line, not the batch's lowest
        // id) - the group must still be found and deleted as a whole,
        // proving resolution doesn't depend on which row you start from.
        let deleted = delete_sale_group_impl(&mut conn, ids[1]).unwrap();
        assert_eq!(deleted, 3, "all 3 lines in the batch must be deleted");

        let remaining: i64 = conn.query_row("SELECT COUNT(*) FROM sales", [], |r| r.get(0)).unwrap();
        assert_eq!(remaining, 0, "every row in the group must be gone");

        assert_eq!(ticket_status(&conn, tickets[0]), "available", "was already available from the refund");
        assert_eq!(ticket_status(&conn, tickets[1]), "available", "was actively sold - must return to Available");
        assert_eq!(ticket_status(&conn, tickets[2]), "available", "was actively sold - must return to Available");
    }

    #[test]
    fn delete_sale_group_on_a_single_non_batch_sale_behaves_like_deleting_just_that_one_line() {
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 1);
        let sale_id = create_sale_impl(&mut conn, &sale_input(tickets[0], 1000)).unwrap();

        let deleted = delete_sale_group_impl(&mut conn, sale_id).unwrap();
        assert_eq!(deleted, 1);

        assert_eq!(ticket_status(&conn, tickets[0]), "available");
        let remaining: i64 = conn.query_row("SELECT COUNT(*) FROM sales", [], |r| r.get(0)).unwrap();
        assert_eq!(remaining, 0);
    }

    #[test]
    fn delete_sale_group_never_disturbs_a_different_newer_sale_on_a_resold_ticket() {
        // Same hazard as deleting a single refunded line (see the test
        // above this section), but through the group-delete path: the
        // "group" being deleted here is just the old refunded sale's own
        // (batch-less) group - a DIFFERENT, newer sale exists on the same
        // ticket and must survive untouched.
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 1);
        let ticket_id = tickets[0];

        let refunded_id = create_sale_impl(&mut conn, &sale_input(ticket_id, 2000)).unwrap();
        refund_sale_impl(&mut conn, refunded_id, None).unwrap();
        let active_id = create_sale_impl(&mut conn, &sale_input(ticket_id, 1800)).unwrap();
        assert_eq!(ticket_status(&conn, ticket_id), "sold");

        let deleted = delete_sale_group_impl(&mut conn, refunded_id).unwrap();
        assert_eq!(deleted, 1, "the refunded sale has no batch_id, so its group is just itself");

        assert_eq!(
            ticket_status(&conn, ticket_id),
            "sold",
            "deleting the old refunded group must not un-sell a ticket actively sold under a different sale"
        );
        let active_still_present: i64 = conn
            .query_row("SELECT COUNT(*) FROM sales WHERE id = ?1", [active_id], |r| r.get(0))
            .unwrap();
        assert_eq!(active_still_present, 1, "the newer active sale must be untouched");
    }

    // ---- bulk delete (2.0.28) ----------------------------------------------

    #[test]
    fn bulk_delete_sale_groups_removes_every_selected_group_including_active_ones() {
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 3);
        // sale_input() defaults to payment_status "paid" (not refunded) - this
        // also doubles as proof bulk delete never blocks on an active sale,
        // same as delete_sale_group_impl already doesn't.
        let sale_a = create_sale_impl(&mut conn, &sale_input(tickets[0], 1000)).unwrap();
        let sale_b = create_sale_impl(&mut conn, &sale_input(tickets[1], 1200)).unwrap();
        let sale_c = create_sale_impl(&mut conn, &sale_input(tickets[2], 1400)).unwrap();

        let result = bulk_delete_sale_groups_impl(&mut conn, &[sale_a, sale_b]).unwrap();

        assert_eq!(result.deleted_ids, vec![sale_a, sale_b]);
        assert!(result.skipped.is_empty());
        assert_eq!(ticket_status(&conn, tickets[0]), "available");
        assert_eq!(ticket_status(&conn, tickets[1]), "available");
        assert_eq!(ticket_status(&conn, tickets[2]), "sold", "the unselected sale_c must be untouched");
        let sale_c_still_there: i64 = conn
            .query_row("SELECT COUNT(*) FROM sales WHERE id = ?1", [sale_c], |r| r.get(0))
            .unwrap();
        assert_eq!(sale_c_still_there, 1);
    }

    #[test]
    fn bulk_delete_sale_groups_deletes_a_whole_batch_via_one_selected_id() {
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 3);
        let batch_ids = create_sales_batch_impl(&mut conn, &batch_input(&tickets, 1000, "paid")).unwrap();
        assert_eq!(batch_ids.len(), 3);

        // Only the batch's own id is in the selection - the other 2 lines
        // must still be resolved and deleted as part of the same group,
        // exactly like delete_sale_group_impl already does for a single id.
        let result = bulk_delete_sale_groups_impl(&mut conn, &[batch_ids[1]]).unwrap();

        assert_eq!(result.deleted_ids, vec![batch_ids[1]]);
        assert!(result.skipped.is_empty());
        let remaining: i64 = conn.query_row("SELECT COUNT(*) FROM sales", [], |r| r.get(0)).unwrap();
        assert_eq!(remaining, 0, "every line in the batch must be gone, not just the selected id");
        for t in &tickets {
            assert_eq!(ticket_status(&conn, *t), "available");
        }
    }

    #[test]
    fn bulk_delete_sale_groups_reports_a_missing_id_as_skipped_not_as_a_failure() {
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 1);
        let sale_id = create_sale_impl(&mut conn, &sale_input(tickets[0], 1000)).unwrap();

        let result = bulk_delete_sale_groups_impl(&mut conn, &[sale_id, 999_999]).unwrap();

        assert_eq!(result.deleted_ids, vec![sale_id]);
        assert_eq!(result.skipped.len(), 1);
        assert_eq!(result.skipped[0].id, 999_999);
    }

    #[test]
    fn bulk_delete_sale_groups_rejects_an_empty_selection() {
        let mut conn = test_conn();
        let err = bulk_delete_sale_groups_impl(&mut conn, &[]).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn non_refunded_sale_can_still_be_deleted_to_undo_a_mistake() {
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 1);
        let sale_id = create_sale_impl(&mut conn, &sale_input(tickets[0], 1000)).unwrap();

        delete_sale_impl(&mut conn, sale_id).unwrap();

        assert_eq!(ticket_status(&conn, tickets[0]), "available");
        let still_present: i64 = conn
            .query_row("SELECT COUNT(*) FROM sales WHERE id = ?1", [sale_id], |r| r.get(0))
            .unwrap();
        assert_eq!(still_present, 0);
    }

    #[test]
    fn duplicate_sale_of_same_ticket_rejected() {
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 1);
        create_sale_impl(&mut conn, &sale_input(tickets[0], 1000)).unwrap();
        let err = create_sale_impl(&mut conn, &sale_input(tickets[0], 1000)).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    // ---- Sale grouping (batch_id) ----------------------------------------

    fn batch_input(tickets: &[i64], price_cents: i64, payment_status: &str) -> SaleBatchInput {
        SaleBatchInput {
            lines: tickets
                .iter()
                .map(|&tid| crate::models::SaleBatchLineInput {
                    ticket_id: tid,
                    sale_price_cents: price_cents,
                    selling_fees_cents: 0,
                })
                .collect(),
            platform_id: None,
            sale_date: "2026-03-01".to_string(),
            payment_status: Some(payment_status.to_string()),
            buyer_reference: Some("Buyer X".to_string()),
            currency: None,
            notes: None,
        }
    }

    fn all_groups(conn: &Connection) -> Vec<SaleGroup> {
        list_sale_groups_impl(conn, None, None, None, None, None, None, None, None, None, None).unwrap()
    }

    #[test]
    fn single_ticket_sale_via_create_sale_has_no_batch_id_and_is_its_own_group() {
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 1);
        let sale_id = create_sale_impl(&mut conn, &sale_input(tickets[0], 1000)).unwrap();

        let batch_id: Option<String> = conn
            .query_row("SELECT batch_id FROM sales WHERE id=?1", [sale_id], |r| r.get(0))
            .unwrap();
        assert_eq!(batch_id, None, "a plain single sale must never get a batch_id");

        let groups = all_groups(&conn);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].ticket_count, 1);
        assert_eq!(groups[0].id, sale_id);

        let lines = list_sales_by_group_impl(&conn, sale_id).unwrap();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].id, sale_id);
    }

    #[test]
    fn batch_of_exactly_one_ticket_also_has_no_batch_id() {
        // create_sales_batch is also the path used for a single-ticket "New
        // sale" in the UI - a batch of one line must behave identically to
        // create_sale (no batch_id, no artificial grouping of nothing).
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 1);
        let ids = create_sales_batch_impl(&mut conn, &batch_input(&tickets, 1000, "paid")).unwrap();
        assert_eq!(ids.len(), 1);

        let batch_id: Option<String> = conn
            .query_row("SELECT batch_id FROM sales WHERE id=?1", [ids[0]], |r| r.get(0))
            .unwrap();
        assert_eq!(batch_id, None);
        assert_eq!(all_groups(&conn).len(), 1);
    }

    #[test]
    fn bulk_sale_of_hundred_tickets_groups_into_one_row_with_correct_totals() {
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 100);
        let ids = create_sales_batch_impl(&mut conn, &batch_input(&tickets, 2000, "paid")).unwrap();
        assert_eq!(ids.len(), 100);

        // Every row shares the exact same batch_id.
        let distinct_batch_ids: i64 = conn
            .query_row(
                "SELECT COUNT(DISTINCT batch_id) FROM sales WHERE id IN (SELECT id FROM sales)",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(distinct_batch_ids, 1);

        let groups = all_groups(&conn);
        assert_eq!(groups.len(), 1, "100 tickets sold in one batch must collapse to one group");
        let g = &groups[0];
        assert_eq!(g.ticket_count, 100);
        assert_eq!(g.revenue_cents, 2000 * 100);
        assert_eq!(g.refunded_count, 0);
        assert_eq!(g.payment_status.as_deref(), Some("paid"));

        // Opening the group returns every one of the 100 lines, none lost,
        // none duplicated.
        let lines = list_sales_by_group_impl(&conn, g.id).unwrap();
        assert_eq!(lines.len(), 100);
        let distinct_tickets: HashSet<i64> = lines.iter().map(|s| s.ticket_id).collect();
        assert_eq!(distinct_tickets.len(), 100, "no ticket duplicated within the group");
    }

    #[test]
    fn refund_within_a_batch_excludes_only_that_line_from_revenue() {
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 4);
        let ids = create_sales_batch_impl(&mut conn, &batch_input(&tickets, 1000, "paid")).unwrap();
        assert_eq!(ids.len(), 4);

        refund_sale_impl(&mut conn, ids[0], Some("buyer cancelled")).unwrap();

        let groups = all_groups(&conn);
        assert_eq!(groups.len(), 1, "the batch is still one group after a partial refund");
        let g = &groups[0];
        assert_eq!(g.ticket_count, 4, "refunded ticket must not disappear from the count");
        assert_eq!(g.refunded_count, 1);
        assert_eq!(g.revenue_cents, 1000 * 3, "refunded line excluded from realized revenue");
        assert_eq!(
            g.payment_status, None,
            "3 paid + 1 refunded is a mixed status, not a single badge"
        );

        // The refunded ticket is back in inventory, not silently dropped.
        assert_eq!(ticket_status(&conn, tickets[0]), "available");

        // Sale Detail for this group must still show all 4 lines (including
        // the refunded one, clearly marked) - nothing is ever hidden there.
        let lines = list_sales_by_group_impl(&conn, g.id).unwrap();
        assert_eq!(lines.len(), 4);
        let refunded_lines: Vec<_> = lines.iter().filter(|s| s.payment_status == "refunded").collect();
        assert_eq!(refunded_lines.len(), 1);
        assert_eq!(refunded_lines[0].ticket_id, tickets[0]);
    }

    #[test]
    fn multiple_single_sales_for_the_same_event_stay_distinct_groups() {
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 2);
        create_sale_impl(&mut conn, &sale_input(tickets[0], 1000)).unwrap();
        create_sale_impl(&mut conn, &sale_input(tickets[1], 1500)).unwrap();

        let groups = all_groups(&conn);
        assert_eq!(groups.len(), 2, "two separate sales must never be merged into one group");
        assert!(groups.iter().all(|g| g.ticket_count == 1));

        let event_id = groups[0].event_id.unwrap();
        let filtered =
            list_sale_groups_impl(&conn, None, Some(event_id), None, None, None, None, None, None, None, None).unwrap();
        assert_eq!(filtered.len(), 2, "both sales are for the same event and must both match");
    }

    #[test]
    fn event_filter_returns_the_whole_group_even_when_a_batch_spans_two_events() {
        // The "New sale" ticket picker doesn't restrict selection to one
        // event, so a batch CAN span events. Filtering by one of them must
        // still show the FULL group (correct ticket_count/revenue) - never a
        // partial view that would disagree with what Sale Detail shows.
        let mut conn = test_conn();
        conn.execute("INSERT INTO events (name) VALUES ('Event A')", []).unwrap();
        let event_a = conn.last_insert_rowid();
        conn.execute("INSERT INTO events (name) VALUES ('Event B')", []).unwrap();
        let event_b = conn.last_insert_rowid();

        let mk_order_ticket = |conn: &mut Connection, event_id: i64| -> i64 {
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
                ticket_type: None,
                section: None,
                row_label: None,
                seats: None,
            };
            let order_id = crate::commands::orders::insert_order_with_tickets(conn, &input, false).unwrap();
            conn.query_row(
                "SELECT id FROM tickets WHERE order_id=?1",
                [order_id],
                |r| r.get(0),
            )
            .unwrap()
        };
        let ticket_a = mk_order_ticket(&mut conn, event_a);
        let ticket_b = mk_order_ticket(&mut conn, event_b);

        let ids =
            create_sales_batch_impl(&mut conn, &batch_input(&[ticket_a, ticket_b], 1000, "paid")).unwrap();
        assert_eq!(ids.len(), 2);

        let all = all_groups(&conn);
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].event_id, None, "mixed events -> None, never a guessed/blended event");
        assert_eq!(all[0].ticket_count, 2);

        for eid in [event_a, event_b] {
            let filtered =
                list_sale_groups_impl(&conn, None, Some(eid), None, None, None, None, None, None, None, None).unwrap();
            assert_eq!(filtered.len(), 1, "the group must still match when filtered by either event");
            assert_eq!(
                filtered[0].ticket_count, 2,
                "the matched group must aggregate BOTH lines, not just the one for the filtered event"
            );
        }
    }

    #[test]
    fn refund_status_filter_finds_batches_with_a_refunded_line() {
        let mut conn = test_conn();
        let clean = seed_tickets(&mut conn, 2);
        create_sales_batch_impl(&mut conn, &batch_input(&clean, 1000, "paid")).unwrap();

        let with_refund = seed_tickets(&mut conn, 2);
        let ids = create_sales_batch_impl(&mut conn, &batch_input(&with_refund, 1000, "paid")).unwrap();
        refund_sale_impl(&mut conn, ids[0], None).unwrap();

        let has_refund =
            list_sale_groups_impl(&conn, None, None, None, None, None, None, Some("has_refund".into()), None, None, None)
                .unwrap();
        assert_eq!(has_refund.len(), 1);
        assert_eq!(has_refund[0].refunded_count, 1);

        let no_refund =
            list_sale_groups_impl(&conn, None, None, None, None, None, None, Some("no_refund".into()), None, None, None)
                .unwrap();
        assert_eq!(no_refund.len(), 1);
        assert_eq!(no_refund[0].refunded_count, 0);
    }

    #[test]
    fn group_ticket_counts_always_sum_back_to_total_sales_rows() {
        let mut conn = test_conn();
        let t1 = seed_tickets(&mut conn, 1);
        create_sale_impl(&mut conn, &sale_input(t1[0], 1000)).unwrap();
        let t5 = seed_tickets(&mut conn, 5);
        create_sales_batch_impl(&mut conn, &batch_input(&t5, 1000, "paid")).unwrap();
        let t20 = seed_tickets(&mut conn, 20);
        create_sales_batch_impl(&mut conn, &batch_input(&t20, 1000, "pending")).unwrap();

        let total_rows: i64 = conn.query_row("SELECT COUNT(*) FROM sales", [], |r| r.get(0)).unwrap();
        assert_eq!(total_rows, 26);

        let groups = all_groups(&conn);
        assert_eq!(groups.len(), 3, "one group per sale action, regardless of size");
        let sum: i64 = groups.iter().map(|g| g.ticket_count).sum();
        assert_eq!(sum, 26, "no ticket lost or duplicated across groups");
    }

    // ---- BUG #6 fix: margin/ROI must be None for a mixed-currency batch --

    /// Same idea as `seed_tickets`, but lets the caller pick the ticket's
    /// currency (and reuse one event across calls) - needed only for BUG #6's
    /// mixed-currency tests below. A sale's own currency is copied from its
    /// ticket whenever the caller doesn't explicitly override it (see
    /// `create_sale_impl`/`create_sales_batch_impl` above), so - for every
    /// test in this file that leaves `SaleBatchInput::currency` as `None`,
    /// i.e. all of them except 2.0.57's own override tests further below -
    /// the only way to get a mixed-currency batch is still to sell tickets
    /// that were themselves purchased in different currencies.
    fn seed_ticket_with_currency(conn: &mut Connection, event_id: i64, currency: &str) -> i64 {
        let input = OrderInput {
            event_id,
            supplier_id: None,
            platform_id: None,
            purchase_date: "2026-01-01".to_string(),
            quantity: 1,
            unit_price_cents: 1000,
            fees_cents: 0,
            other_costs_cents: 0,
            currency: currency.to_string(),
            payment_status: Some("paid".to_string()),
            notes: None,
            ticket_type: None,
            section: None,
            row_label: None,
            seats: None,
        };
        let order_id = crate::commands::orders::insert_order_with_tickets(conn, &input, false).unwrap();
        conn.query_row("SELECT id FROM tickets WHERE order_id=?1", [order_id], |r| {
            r.get(0)
        })
        .unwrap()
    }

    // ---- 2.0.57: New Sale's own currency, independent of the ticket's ----

    #[test]
    fn explicit_batch_currency_overrides_every_tickets_own_purchase_currency() {
        // marko's request: New Sale can now record a sale in a currency he
        // picks himself (e.g. a ticket bought in USD but paid out by the
        // marketplace in EUR) instead of always inheriting the ticket's own
        // purchase currency - see `SaleBatchInput::currency`'s own doc
        // comment for the full reasoning.
        let mut conn = test_conn();
        conn.execute("INSERT INTO events (name) VALUES ('Test Event')", []).unwrap();
        let event_id = conn.last_insert_rowid();
        let usd_ticket = seed_ticket_with_currency(&mut conn, event_id, "USD");

        let mut input = batch_input(&[usd_ticket], 5000, "paid");
        input.currency = Some("EUR".to_string());
        let ids = create_sales_batch_impl(&mut conn, &input).unwrap();

        let stored: String = conn
            .query_row("SELECT currency FROM sales WHERE id = ?1", [ids[0]], |r| r.get(0))
            .unwrap();
        assert_eq!(stored, "EUR", "an explicit batch currency must win over the ticket's own USD");
    }

    #[test]
    fn explicit_batch_currency_applies_to_every_line_even_when_tickets_differ() {
        // Two tickets bought in two different currencies, sold together in
        // ONE sale action, paid out in a third currency the user chose -
        // every resulting `sales` row must carry that SAME chosen currency,
        // never fall back to either ticket's own (this is what makes the
        // batch's own revenue total always a single, real number in the New
        // Sale preview, regardless of what the tickets themselves cost).
        let mut conn = test_conn();
        conn.execute("INSERT INTO events (name) VALUES ('Test Event')", []).unwrap();
        let event_id = conn.last_insert_rowid();
        let eur_ticket = seed_ticket_with_currency(&mut conn, event_id, "EUR");
        let usd_ticket = seed_ticket_with_currency(&mut conn, event_id, "USD");

        let input = SaleBatchInput {
            lines: vec![
                crate::models::SaleBatchLineInput { ticket_id: eur_ticket, sale_price_cents: 1000, selling_fees_cents: 0 },
                crate::models::SaleBatchLineInput { ticket_id: usd_ticket, sale_price_cents: 1000, selling_fees_cents: 0 },
            ],
            platform_id: None,
            sale_date: "2026-03-01".to_string(),
            payment_status: Some("paid".to_string()),
            buyer_reference: None,
            notes: None,
            currency: Some("GBP".to_string()),
        };
        let ids = create_sales_batch_impl(&mut conn, &input).unwrap();
        for id in ids {
            let stored: String = conn.query_row("SELECT currency FROM sales WHERE id = ?1", [id], |r| r.get(0)).unwrap();
            assert_eq!(stored, "GBP", "every line in the batch must share the one chosen sale currency");
        }
    }

    #[test]
    fn omitting_batch_currency_still_falls_back_to_each_tickets_own_currency() {
        // Regression guard for `orders_sheet_sync::apply_sales_rows` (the
        // Sales tab of Google Sheets sync has no currency column of its
        // own) and every pre-2.0.57 test in this file: `currency: None`
        // must reproduce the exact original behaviour, including staying
        // correctly mixed across two tickets bought in different currencies.
        let mut conn = test_conn();
        conn.execute("INSERT INTO events (name) VALUES ('Test Event')", []).unwrap();
        let event_id = conn.last_insert_rowid();
        let eur_ticket = seed_ticket_with_currency(&mut conn, event_id, "EUR");
        let usd_ticket = seed_ticket_with_currency(&mut conn, event_id, "USD");

        let input = SaleBatchInput {
            lines: vec![
                crate::models::SaleBatchLineInput { ticket_id: eur_ticket, sale_price_cents: 1000, selling_fees_cents: 0 },
                crate::models::SaleBatchLineInput { ticket_id: usd_ticket, sale_price_cents: 1000, selling_fees_cents: 0 },
            ],
            platform_id: None,
            sale_date: "2026-03-01".to_string(),
            payment_status: Some("paid".to_string()),
            buyer_reference: None,
            notes: None,
            currency: None,
        };
        let ids = create_sales_batch_impl(&mut conn, &input).unwrap();
        let eur_stored: String =
            conn.query_row("SELECT currency FROM sales WHERE id = ?1", [ids[0]], |r| r.get(0)).unwrap();
        let usd_stored: String =
            conn.query_row("SELECT currency FROM sales WHERE id = ?1", [ids[1]], |r| r.get(0)).unwrap();
        assert_eq!(eur_stored, "EUR");
        assert_eq!(usd_stored, "USD");
    }

    #[test]
    fn blank_explicit_batch_currency_is_rejected() {
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 1);
        let mut input = batch_input(&tickets, 1000, "paid");
        input.currency = Some("   ".to_string());
        let err = create_sales_batch_impl(&mut conn, &input).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn a_sale_whose_currency_mismatches_its_own_tickets_currency_hides_margin_and_roi() {
        // The actual correctness gap 2.0.57's currency override opens up:
        // cost_cents comes from the TICKET (its own purchase currency),
        // sale_price_cents from the SALE (now possibly a different,
        // explicitly-chosen currency) - map_sale must never blend the two
        // into a real-looking margin/ROI percentage. profit_cents/cost_cents
        // stay real numbers (each meaningful in its own currency on its
        // own) - `currencyMismatch` is what tells the frontend not to
        // display them as if they were one number (formatMoneyOrMixed).
        let mut conn = test_conn();
        conn.execute("INSERT INTO events (name) VALUES ('Test Event')", []).unwrap();
        let event_id = conn.last_insert_rowid();
        let usd_ticket = seed_ticket_with_currency(&mut conn, event_id, "USD");

        let mut input = batch_input(&[usd_ticket], 5000, "paid");
        input.currency = Some("EUR".to_string());
        let ids = create_sales_batch_impl(&mut conn, &input).unwrap();
        let sale = fetch_one(&conn, ids[0]).unwrap();

        assert!(sale.currency_mismatch, "USD ticket sold in EUR must be flagged as a currency mismatch");
        assert_eq!(sale.margin, None, "a cross-currency margin is meaningless, never a real-looking percentage");
        assert_eq!(sale.roi, None, "a cross-currency ROI is meaningless, never a real-looking percentage");
        // profit_cents/cost_cents are still populated (not zeroed or
        // panicking) - it's the caller's job (currencyMismatch) to know not
        // to trust them as one currency, not this function's job to hide them.
        assert_eq!(sale.cost_cents, 1000);
    }

    #[test]
    fn a_sale_whose_currency_matches_its_own_tickets_currency_computes_margin_and_roi_normally() {
        // The ordinary, overwhelmingly common case (explicit currency equal
        // to the ticket's own, or no override at all) must be completely
        // unaffected by 2.0.57 - same real numbers as always.
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 1); // EUR, cost 1000 cents
        let mut input = batch_input(&tickets, 2000, "paid");
        input.currency = Some("EUR".to_string());
        let ids = create_sales_batch_impl(&mut conn, &input).unwrap();
        let sale = fetch_one(&conn, ids[0]).unwrap();

        assert!(!sale.currency_mismatch);
        assert_eq!(sale.profit_cents, 1000);
        assert_eq!(sale.margin, Some(0.5));
    }

    #[test]
    fn group_currency_is_mixed_when_a_single_lines_sale_currency_differs_from_its_own_tickets_currency() {
        // GROUP_BASE_SELECT's own equivalent of the map_sale test above:
        // before 2.0.57 this scenario was simply impossible (a sale's
        // currency always equalled its ticket's), so "every line's currency
        // agrees with every other line's" (COUNT(DISTINCT s.currency) = 1)
        // was previously a sufficient check all on its own for a
        // single-line group. It no longer is.
        let mut conn = test_conn();
        conn.execute("INSERT INTO events (name) VALUES ('Test Event')", []).unwrap();
        let event_id = conn.last_insert_rowid();
        let usd_ticket = seed_ticket_with_currency(&mut conn, event_id, "USD");
        let mut input = batch_input(&[usd_ticket], 5000, "paid");
        input.currency = Some("EUR".to_string());
        create_sales_batch_impl(&mut conn, &input).unwrap();

        let groups = fetch_recent_groups(&conn, 5).unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].currency, None, "cost (USD) and revenue (EUR) don't share a currency - must be Mixed");
        assert_eq!(groups[0].margin, None);
        assert_eq!(groups[0].roi, None);
    }

    #[test]
    fn single_currency_batch_computes_margin_and_roi_normally() {
        // BUG #6 baseline: an ordinary all-EUR batch must keep computing a
        // real Margin/ROI exactly as before - only a MIXED-currency batch is
        // supposed to change.
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 3); // all EUR, cost 1000 cents each
        let ids = create_sales_batch_impl(&mut conn, &batch_input(&tickets, 2000, "paid")).unwrap();
        assert_eq!(ids.len(), 3);

        let groups = all_groups(&conn);
        assert_eq!(groups.len(), 1);
        let g = &groups[0];
        assert_eq!(g.currency.as_deref(), Some("EUR"));
        // revenue 3*2000=6000, cost 3*1000=3000, fees 0 -> profit 3000.
        assert_eq!(g.revenue_cents, 6000);
        assert_eq!(g.cost_cents, 3000);
        assert_eq!(g.profit_cents, 3000);
        assert_eq!(g.margin, Some(0.5));
        assert_eq!(g.roi, Some(1.0));
    }

    #[test]
    fn mixed_currency_batch_leaves_margin_and_roi_as_none() {
        // The exact BUG #6 scenario: EUR + USD in one batch. `currency` is
        // already correctly None (GROUP_BASE_SELECT); margin/ROI must now
        // ALSO be None instead of a blended, economically meaningless ratio.
        let mut conn = test_conn();
        conn.execute("INSERT INTO events (name) VALUES ('Mixed Currency Event')", [])
            .unwrap();
        let event_id = conn.last_insert_rowid();
        let eur_ticket = seed_ticket_with_currency(&mut conn, event_id, "EUR");
        let usd_ticket = seed_ticket_with_currency(&mut conn, event_id, "USD");

        let ids = create_sales_batch_impl(
            &mut conn,
            &batch_input(&[eur_ticket, usd_ticket], 2000, "paid"),
        )
        .unwrap();
        assert_eq!(ids.len(), 2);

        let groups = all_groups(&conn);
        assert_eq!(groups.len(), 1);
        let g = &groups[0];
        assert_eq!(g.currency, None, "two different currencies -> Mixed/None");
        assert_eq!(g.margin, None, "margin must not be computed across mixed currencies");
        assert_eq!(g.roi, None, "roi must not be computed across mixed currencies");
        // Revenue/cost/profit are still returned as summed cents - showing
        // "Mixed" instead of a blended money amount is the FRONTEND's job
        // (formatMoneyOrMixed, keyed off this same `currency` field), not
        // something this fix should start hiding from the API.
        assert_eq!(g.revenue_cents, 4000);
    }

    #[test]
    fn mixed_currency_batch_with_a_refund_still_leaves_margin_and_roi_none_and_refund_accounting_is_correct(
    ) {
        // User's scenario 3: EUR active + USD active + EUR refunded.
        let mut conn = test_conn();
        conn.execute(
            "INSERT INTO events (name) VALUES ('Mixed Currency Refund Event')",
            [],
        )
        .unwrap();
        let event_id = conn.last_insert_rowid();
        let eur_active = seed_ticket_with_currency(&mut conn, event_id, "EUR");
        let usd_active = seed_ticket_with_currency(&mut conn, event_id, "USD");
        let eur_refunded = seed_ticket_with_currency(&mut conn, event_id, "EUR");

        let ids = create_sales_batch_impl(
            &mut conn,
            &batch_input(&[eur_active, usd_active, eur_refunded], 2000, "paid"),
        )
        .unwrap();
        assert_eq!(ids.len(), 3);
        refund_sale_impl(&mut conn, ids[2], Some("test refund")).unwrap();

        let groups = all_groups(&conn);
        assert_eq!(groups.len(), 1);
        let g = &groups[0];
        assert_eq!(g.ticket_count, 3, "refunded line stays in the group, never disappears");
        assert_eq!(g.refunded_count, 1);
        assert_eq!(
            g.currency, None,
            "still mixed (EUR+USD) even with a refund present - refund status doesn't change currency mixing"
        );
        assert_eq!(g.margin, None);
        assert_eq!(g.roi, None);
        // Refund accounting itself is untouched by this fix: revenue/cost
        // still excludes the refunded line, same as every other refund test
        // in this file.
        assert_eq!(g.revenue_cents, 4000, "only the 2 active lines (EUR+USD) - refunded line excluded");
        assert_eq!(ticket_status(&conn, eur_refunded), "available");
    }

    #[test]
    fn single_currency_batch_with_a_refund_computes_margin_and_roi_only_from_the_realized_line() {
        // User's scenario 4: EUR active + EUR refunded -> currency stays EUR,
        // and margin/ROI are computed normally, but only from the still-
        // active (realized) line.
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 2); // both EUR, cost 1000 cents each
        let ids = create_sales_batch_impl(&mut conn, &batch_input(&tickets, 2000, "paid")).unwrap();
        refund_sale_impl(&mut conn, ids[0], Some("test refund")).unwrap();

        let groups = all_groups(&conn);
        assert_eq!(groups.len(), 1);
        let g = &groups[0];
        assert_eq!(g.currency.as_deref(), Some("EUR"), "single currency throughout -> never Mixed");
        assert_eq!(g.refunded_count, 1);
        // Only the one surviving active line counts: revenue 2000, cost
        // 1000, fees 0 -> profit 1000.
        assert_eq!(g.revenue_cents, 2000);
        assert_eq!(g.cost_cents, 1000);
        assert_eq!(g.profit_cents, 1000);
        assert_eq!(g.margin, Some(0.5));
        assert_eq!(g.roi, Some(1.0));
    }

    // ---- 1.6.0 audit H5 fix: currency must follow the same non-refunded --
    // ---- scope as revenue/cost/profit, not all lines ----------------------

    #[test]
    fn refunding_the_only_differently_currencied_line_reveals_the_real_single_currency_total() {
        // The exact H5 scenario: EUR active + USD active. While both are
        // active this is correctly Mixed (same as the BUG #6 test above).
        // Once the USD line is refunded, only the EUR line still counts
        // toward revenue/cost/profit (exactly like every other refund test),
        // so currency/margin/ROI should now report the real EUR numbers
        // instead of staying stuck on Mixed just because a refunded, no-
        // longer-counted line happens to be a different currency.
        let mut conn = test_conn();
        conn.execute("INSERT INTO events (name) VALUES ('H5 Event')", [])
            .unwrap();
        let event_id = conn.last_insert_rowid();
        let eur_active = seed_ticket_with_currency(&mut conn, event_id, "EUR");
        let usd_to_refund = seed_ticket_with_currency(&mut conn, event_id, "USD");

        let ids = create_sales_batch_impl(
            &mut conn,
            &batch_input(&[eur_active, usd_to_refund], 2000, "paid"),
        )
        .unwrap();
        assert_eq!(ids.len(), 2);

        // Still both active -> Mixed, unchanged from BUG #6 behavior.
        let before = all_groups(&conn);
        assert_eq!(before[0].currency, None);

        refund_sale_impl(&mut conn, ids[1], Some("test refund")).unwrap();

        let groups = all_groups(&conn);
        assert_eq!(groups.len(), 1);
        let g = &groups[0];
        assert_eq!(g.ticket_count, 2, "refunded line stays in the group");
        assert_eq!(g.refunded_count, 1);
        assert_eq!(
            g.currency.as_deref(),
            Some("EUR"),
            "only one currency left among non-refunded lines -> must not stay Mixed"
        );
        // revenue 2000, cost 1000 (seed_ticket_with_currency's order costs
        // 1000 cents), fees 0 -> profit 1000, from the EUR line alone.
        assert_eq!(g.revenue_cents, 2000);
        assert_eq!(g.cost_cents, 1000);
        assert_eq!(g.profit_cents, 1000);
        assert_eq!(g.margin, Some(0.5));
        assert_eq!(g.roi, Some(1.0));
    }

    #[test]
    fn fully_refunded_mixed_currency_batch_falls_back_to_all_lines_for_currency() {
        // Edge case the H5 fix must not break: if EVERY line in the group
        // ends up refunded, there are zero non-refunded lines to derive
        // currency from - fall back to checking all lines so a group that
        // was always mixed still reports Mixed (not some arbitrary pick),
        // and per-group money stays 0 (nothing realized), not blank/error.
        let mut conn = test_conn();
        conn.execute("INSERT INTO events (name) VALUES ('H5 Fully Refunded Event')", [])
            .unwrap();
        let event_id = conn.last_insert_rowid();
        let eur_line = seed_ticket_with_currency(&mut conn, event_id, "EUR");
        let usd_line = seed_ticket_with_currency(&mut conn, event_id, "USD");

        let ids =
            create_sales_batch_impl(&mut conn, &batch_input(&[eur_line, usd_line], 2000, "paid")).unwrap();
        refund_sale_impl(&mut conn, ids[0], Some("test refund")).unwrap();
        refund_sale_impl(&mut conn, ids[1], Some("test refund")).unwrap();

        let groups = all_groups(&conn);
        assert_eq!(groups.len(), 1);
        let g = &groups[0];
        assert_eq!(g.refunded_count, 2);
        assert_eq!(g.currency, None, "still mixed when falling back to all (refunded) lines");
        assert_eq!(g.margin, None);
        assert_eq!(g.roi, None);
        assert_eq!(g.revenue_cents, 0, "nothing realized - both lines refunded");
    }

    // ---- BUG #1 fix: refund must not permanently block a resale ----------

    #[test]
    fn refund_then_resell_creates_a_new_active_sale_and_keeps_full_history() {
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 1);
        let ticket_id = tickets[0];

        let sale_id_1 = create_sale_impl(&mut conn, &sale_input(ticket_id, 2000)).unwrap();
        assert_eq!(ticket_status(&conn, ticket_id), "sold");

        refund_sale_impl(&mut conn, sale_id_1, Some("buyer cancelled")).unwrap();
        assert_eq!(ticket_status(&conn, ticket_id), "available");

        // This is the actual bug: before migration 004, this insert failed
        // with "This ticket has already been sold." even though the ticket
        // is correctly Available again.
        let sale_id_2 = create_sale_impl(&mut conn, &sale_input(ticket_id, 1800))
            .expect("a refunded ticket must be sellable again");
        assert_ne!(sale_id_1, sale_id_2);
        assert_eq!(ticket_status(&conn, ticket_id), "sold");

        // Both rows exist - refund history is never lost or overwritten.
        let rows: Vec<(i64, String)> = {
            let mut stmt = conn
                .prepare("SELECT id, payment_status FROM sales WHERE ticket_id=?1 ORDER BY id")
                .unwrap();
            stmt.query_map([ticket_id], |r| Ok((r.get(0)?, r.get(1)?)))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
        };
        assert_eq!(
            rows,
            // sale_input() always sets payment_status "paid" (see helper above).
            vec![(sale_id_1, "refunded".to_string()), (sale_id_2, "paid".to_string())]
        );

        // Finance only ever counts the active sale - the refunded one never
        // contributes to realized revenue, even though it's still on record.
        let revenue: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(sale_price_cents),0) FROM sales WHERE ticket_id=?1 AND payment_status != 'refunded'",
                [ticket_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(revenue, 1800, "only the new active sale counts as realized revenue");

        // A second SIMULTANEOUS active sale must still be impossible - the
        // fix only relaxes uniqueness for history, never for two active
        // sales at once.
        let err = create_sale_impl(&mut conn, &sale_input(ticket_id, 999)).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn refund_then_resell_via_the_real_new_sale_batch_path_works() {
        // The "New Sale" UI always submits through create_sales_batch, even
        // for a single ticket - this is the exact path a user hits when
        // reselling a previously-refunded ticket, so it must be verified
        // directly rather than only inferred from create_sale_impl.
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 1);
        let ticket_id = tickets[0];

        let ids1 = create_sales_batch_impl(&mut conn, &batch_input(&[ticket_id], 2000, "paid")).unwrap();
        refund_sale_impl(&mut conn, ids1[0], None).unwrap();
        assert_eq!(ticket_status(&conn, ticket_id), "available");

        let ids2 = create_sales_batch_impl(&mut conn, &batch_input(&[ticket_id], 1800, "pending"))
            .expect("reselling a refunded ticket through the batch path must succeed");
        assert_eq!(ticket_status(&conn, ticket_id), "sold");
        assert_ne!(ids1[0], ids2[0]);

        let total_rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM sales WHERE ticket_id=?1", [ticket_id], |r| r.get(0))
            .unwrap();
        assert_eq!(total_rows, 2, "refund history preserved alongside the new active sale");

        // The Sales screen must show both as separate, independent entries
        // (original refunded sale, new active sale) - never merged.
        let groups = all_groups(&conn);
        assert_eq!(groups.len(), 2, "the refunded sale and the resale are two distinct sale actions");
    }

    /// BUG #3: Dashboard's "Recent Sales" widget (fetch_recent, used by
    /// get_dashboard) intentionally includes refunded sales as part of
    /// recent activity - same "history is never hidden" rule the Sales
    /// screen already follows - but the frontend needs `payment_status` (and
    /// `refunded_at`) on every row to visually distinguish a refund from a
    /// normal completed sale, instead of showing both identically (the
    /// actual bug: Dashboard.tsx rendered every row the same green
    /// "successful sale" style regardless of status). This test guards the
    /// data contract that fix relies on: refunds must stay present in
    /// fetch_recent, correctly flagged, never silently dropped or
    /// indistinguishable from an active sale.
    #[test]
    fn fetch_recent_includes_refunded_sales_clearly_flagged_alongside_active_ones() {
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 2);

        create_sale_impl(&mut conn, &sale_input(tickets[0], 2500)).unwrap();

        let refunded_sale_id = create_sale_impl(&mut conn, &sale_input(tickets[1], 3000)).unwrap();
        refund_sale_impl(&mut conn, refunded_sale_id, Some("wrong seat")).unwrap();

        let recent = fetch_recent(&conn, 10).unwrap();
        assert_eq!(recent.len(), 2, "recent activity must include the refund, not hide it");

        let refunded = recent
            .iter()
            .find(|s| s.id == refunded_sale_id)
            .expect("refunded sale must still appear in recent activity");
        assert_eq!(refunded.payment_status, "refunded");
        assert!(
            refunded.refunded_at.is_some(),
            "refund timestamp must be available for the frontend to key off of"
        );

        let active = recent
            .iter()
            .find(|s| s.ticket_id == tickets[0])
            .expect("the normal active sale must also appear, unaffected");
        assert_eq!(active.payment_status, "paid");
        assert!(active.refunded_at.is_none());
    }

    #[test]
    fn fetch_recent_groups_collapses_a_multi_ticket_batch_into_one_row() {
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 4);
        let input = SaleBatchInput {
            lines: tickets
                .iter()
                .map(|&tid| crate::models::SaleBatchLineInput { ticket_id: tid, sale_price_cents: 11000, selling_fees_cents: 0 })
                .collect(),
            platform_id: None,
            sale_date: "2026-08-13".to_string(),
            payment_status: Some("paid".to_string()),
            buyer_reference: None,
            notes: None,
            currency: None,
        };
        create_sales_batch_impl(&mut conn, &input).unwrap();

        let recent = fetch_recent_groups(&conn, 5).unwrap();
        assert_eq!(recent.len(), 1, "4 tickets sold together in one batch must be exactly one Recent Sales row, not 4");
        assert_eq!(recent[0].ticket_count, 4);
        assert_eq!(recent[0].revenue_cents, 44000, "the whole batch's total, not one ticket's own price");
    }

    #[test]
    fn fetch_recent_groups_respects_the_limit_by_group_not_by_underlying_ticket_row() {
        let mut conn = test_conn();
        // One 3-ticket batch, then 2 separate single-ticket sales - 3 real
        // sale actions total, so a limit of 2 must return exactly 2 groups
        // even though the batch alone already has 3 underlying `sales` rows.
        let batch_tickets = seed_tickets(&mut conn, 3);
        let input = SaleBatchInput {
            lines: batch_tickets
                .iter()
                .map(|&tid| crate::models::SaleBatchLineInput { ticket_id: tid, sale_price_cents: 1000, selling_fees_cents: 0 })
                .collect(),
            platform_id: None,
            sale_date: "2026-08-01".to_string(),
            payment_status: Some("paid".to_string()),
            buyer_reference: None,
            notes: None,
            currency: None,
        };
        create_sales_batch_impl(&mut conn, &input).unwrap();

        let singles = seed_tickets(&mut conn, 2);
        create_sale_impl(&mut conn, &sale_input(singles[0], 2000)).unwrap();
        create_sale_impl(&mut conn, &sale_input(singles[1], 2500)).unwrap();

        let recent = fetch_recent_groups(&conn, 2).unwrap();
        assert_eq!(recent.len(), 2, "3 real sale actions exist, but the limit is 2 groups");
    }

    /// BUG #4 (original audit): a SaleGroup's id is always the batch's
    /// lowest sale id (see GROUP_BASE_SELECT's MIN(s.id)), and Sale Detail's
    /// route/reload always queries by that exact id
    /// (list_sales_by_group_impl looks up `batch_id` via `WHERE id = ?1`
    /// first). If that specific row is deleted while the rest of the batch
    /// remains, re-querying by the now-gone id 404s even though the batch is
    /// otherwise intact. That part is unavoidable at the data layer - the
    /// deleted row's own batch_id is gone with it, so there is nothing left
    /// to look up "what batch did id X belong to" from. The actual fix is
    /// client-side: Sale Detail must stop reloading via a row id it just
    /// deleted and re-point itself at a surviving id instead. This test
    /// locks in both halves: the raw data-layer 404 (documents exactly why
    /// the client-side fix is needed) and, critically, that any SURVIVING
    /// row's id still correctly resolves the whole remaining group - which
    /// is what the fix relies on.
    #[test]
    fn deleting_a_batchs_lowest_id_row_orphans_that_id_but_not_the_rest_of_the_batch() {
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 3);
        let ids = create_sales_batch_impl(&mut conn, &batch_input(&tickets, 1000, "paid")).unwrap();
        assert_eq!(ids.len(), 3);

        let lowest_id = *ids.iter().min().unwrap();
        let remaining_ids: Vec<i64> = ids.iter().copied().filter(|id| *id != lowest_id).collect();
        assert_eq!(remaining_ids.len(), 2);

        // Sanity: before deleting anything, the group is reachable via its
        // lowest id and has all 3 lines - exactly what Sale Detail shows.
        assert_eq!(list_sales_by_group_impl(&conn, lowest_id).unwrap().len(), 3);

        delete_sale_impl(&mut conn, lowest_id).unwrap();

        // The deleted id itself can no longer resolve the group - its
        // batch_id left with it. This is the exact failure the audit
        // describes, and why the client must not keep re-querying via this
        // id after deleting it.
        let err = list_sales_by_group_impl(&conn, lowest_id).unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));

        // But the batch itself is intact: any surviving row's id still
        // resolves the full remaining group. This is exactly what
        // SaleDetail.tsx's fix relies on - re-pointing itself at
        // remaining_ids.min() after deleting the anchor row.
        let via_survivor = list_sales_by_group_impl(&conn, remaining_ids[0]).unwrap();
        assert_eq!(via_survivor.len(), 2, "the rest of the batch must remain fully reachable");
        let mut got_ids: Vec<i64> = via_survivor.iter().map(|s| s.id).collect();
        got_ids.sort();
        let mut want_ids = remaining_ids.clone();
        want_ids.sort();
        assert_eq!(got_ids, want_ids);
    }

    // ======================================================================
    // 1.8.0: Sales 2.0 - search/filters/sorting/order-linking/export-selected
    // ======================================================================

    /// 1.8.0 addition: `Sale` rows now carry the ticket's own order_id/
    /// order_code (added so Sale Detail can link straight to Order Detail -
    /// see SaleDetail.tsx). Every ticket belongs to exactly one order
    /// (tickets.order_id NOT NULL), so this must always be populated, never
    /// silently null/zero.
    #[test]
    fn sale_rows_carry_their_tickets_order_id_and_order_code() {
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 1);
        let sale_id = create_sale_impl(&mut conn, &sale_input(tickets[0], 1000)).unwrap();

        let order_id: i64 = conn
            .query_row("SELECT order_id FROM tickets WHERE id=?1", [tickets[0]], |r| r.get(0))
            .unwrap();
        let order_code: String = conn
            .query_row("SELECT code FROM orders WHERE id=?1", [order_id], |r| r.get(0))
            .unwrap();

        let sale = fetch_one(&conn, sale_id).unwrap();
        assert_eq!(sale.order_id, order_id);
        assert_eq!(sale.order_code, order_code);

        // Same fields must also come through the grouped Sale Detail path.
        let lines = list_sales_by_group_impl(&conn, sale_id).unwrap();
        assert_eq!(lines[0].order_id, order_id);
        assert_eq!(lines[0].order_code, order_code);
    }

    /// 1.8.0 section 3: searching an order code must find the sale(s) tied
    /// to that order, the same way a ticket code already does.
    #[test]
    fn list_sale_groups_search_matches_order_code() {
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 1);
        let sale_id = create_sale_impl(&mut conn, &sale_input(tickets[0], 1000)).unwrap();
        let order_id: i64 = conn
            .query_row("SELECT order_id FROM tickets WHERE id=?1", [tickets[0]], |r| r.get(0))
            .unwrap();
        let order_code: String = conn
            .query_row("SELECT code FROM orders WHERE id=?1", [order_id], |r| r.get(0))
            .unwrap();

        let found = list_sale_groups_impl(
            &conn, Some(order_code.clone()), None, None, None, None, None, None, None, None,
        None)
        .unwrap();
        assert_eq!(found.len(), 1, "searching the order code must find the sale made from that order");
        assert_eq!(found[0].id, sale_id);

        let miss = list_sale_groups_impl(
            &conn, Some("ORD-999999".into()), None, None, None, None, None, None, None, None,
        None)
        .unwrap();
        assert_eq!(miss.len(), 0, "an unrelated order code must not match");
    }

    /// 1.8.0 section 4: currency filter - "does this group contain a line in
    /// this currency", same semi-join pattern as event/platform/payment.
    #[test]
    fn list_sale_groups_currency_filter_matches_only_that_currency() {
        let mut conn = test_conn();
        conn.execute("INSERT INTO events (name) VALUES ('Currency Filter Event')", [])
            .unwrap();
        let event_id = conn.last_insert_rowid();
        let eur_ticket = seed_ticket_with_currency(&mut conn, event_id, "EUR");
        let usd_ticket = seed_ticket_with_currency(&mut conn, event_id, "USD");
        create_sale_impl(&mut conn, &sale_input(eur_ticket, 1000)).unwrap();
        create_sale_impl(&mut conn, &sale_input(usd_ticket, 1000)).unwrap();

        let eur_only = list_sale_groups_impl(
            &conn, None, None, None, None, None, None, None, Some("EUR".into()), None,
        None)
        .unwrap();
        assert_eq!(eur_only.len(), 1);
        assert_eq!(eur_only[0].currency.as_deref(), Some("EUR"));

        let usd_only = list_sale_groups_impl(
            &conn, None, None, None, None, None, None, None, Some("USD".into()), None,
        None)
        .unwrap();
        assert_eq!(usd_only.len(), 1);
        assert_eq!(usd_only[0].currency.as_deref(), Some("USD"));

        let gbp_none = list_sale_groups_impl(
            &conn, None, None, None, None, None, None, None, Some("GBP".into()), None,
        None)
        .unwrap();
        assert_eq!(gbp_none.len(), 0, "a currency with no sales must match nothing");
    }

    // ---- 1.9.0: payment_status filter on list_sale_groups (test gap) -----
    // The `payment_status` parameter on `list_sale_groups_impl` already
    // existed before 1.9.0 (it's what powers Sales.tsx's Payment dropdown -
    // see the "Payment" filter's Select) but, unlike every OTHER positional
    // filter on this function (search/event/platform/currency/refund_status/
    // sort_by), it had no direct test of its own. Closing that gap was one
    // of the 1.9.0 brief's explicit test scenarios ("Sales payment filter") -
    // this is a test-only addition, the filter implementation itself
    // (GROUP_BASE_SELECT/the semi-join below it) is untouched.

    /// 1.9.0: the payment_status filter is the same "does this group contain
    /// at least one line with this status" semi-join already used for event/
    /// platform/currency/refund_status above (see the doc comment on the
    /// currency filter). For these 3 groups - each a single-ticket sale, so
    /// it has only one line to match - that's indistinguishable from "every
    /// line has this status"; the mixed-group nuance is covered separately
    /// by the test right below this one.
    #[test]
    fn list_sale_groups_payment_status_filter_matches_the_right_groups() {
        let mut conn = test_conn();
        let paid_tickets = seed_tickets(&mut conn, 1);
        create_sale_impl(&mut conn, &sale_input(paid_tickets[0], 1000)).unwrap(); // sale_input() always uses "paid"

        let pending_tickets = seed_tickets(&mut conn, 1);
        create_sales_batch_impl(&mut conn, &batch_input(&pending_tickets, 1000, "pending")).unwrap();

        let refunded_tickets = seed_tickets(&mut conn, 1);
        let refunded_id = create_sale_impl(&mut conn, &sale_input(refunded_tickets[0], 1000)).unwrap();
        refund_sale_impl(&mut conn, refunded_id, None).unwrap();

        let paid_only =
            list_sale_groups_impl(&conn, None, None, None, Some("paid".into()), None, None, None, None, None, None)
                .unwrap();
        assert_eq!(paid_only.len(), 1);
        assert_eq!(paid_only[0].payment_status.as_deref(), Some("paid"));

        let pending_only =
            list_sale_groups_impl(&conn, None, None, None, Some("pending".into()), None, None, None, None, None, None)
                .unwrap();
        assert_eq!(pending_only.len(), 1);
        assert_eq!(pending_only[0].payment_status.as_deref(), Some("pending"));

        let refunded_only =
            list_sale_groups_impl(&conn, None, None, None, Some("refunded".into()), None, None, None, None, None, None)
                .unwrap();
        assert_eq!(refunded_only.len(), 1);
        assert_eq!(refunded_only[0].payment_status.as_deref(), Some("refunded"));
    }

    /// The payment_status filter is a "contains a line" semi-join, not an
    /// "every line matches" filter (see the doc comment on the test above
    /// and on the pre-existing currency filter). A group with a genuinely
    /// MIXED status (one line paid, one still pending) therefore matches
    /// BOTH the "paid" and the "pending" filter - it has a line of each, so
    /// it is correctly not excluded from either. This mirrors how the
    /// currency filter already behaves for a mixed-currency batch, and is a
    /// real, useful property: filtering by "paid" surfaces every sale action
    /// with at least some money collected, even if part of it is still
    /// outstanding.
    #[test]
    fn list_sale_groups_payment_status_filter_is_a_contains_a_line_semi_join() {
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 2);
        let ids = create_sales_batch_impl(&mut conn, &batch_input(&tickets, 1000, "paid")).unwrap();
        // Move just one line of the batch to pending via a plain edit, so the
        // group as a whole becomes genuinely mixed (1 paid + 1 pending).
        update_sale_impl(
            &conn,
            ids[0],
            &crate::models::SaleEditInput {
                platform_id: None,
                sale_date: "2026-03-01".to_string(),
                sale_price_cents: 1000,
                selling_fees_cents: 0,
                payment_status: "pending".to_string(),
                buyer_reference: None,
                notes: None,
            },
        )
        .unwrap();
        assert_eq!(
            all_groups(&conn)[0].payment_status, None,
            "sanity check: the group is genuinely mixed (1 paid + 1 pending)"
        );

        let paid_only =
            list_sale_groups_impl(&conn, None, None, None, Some("paid".into()), None, None, None, None, None, None)
                .unwrap();
        assert_eq!(paid_only.len(), 1, "the group has a paid line, so it must match the 'paid' filter");

        let pending_only =
            list_sale_groups_impl(&conn, None, None, None, Some("pending".into()), None, None, None, None, None, None)
                .unwrap();
        assert_eq!(pending_only.len(), 1, "the same group also has a pending line, so it matches 'pending' too");
    }

    /// 1.8.0 section 5: sortable results. Revenue and the (revenue - cost -
    /// fees) profit expression both need to actually execute in SQL without
    /// error - profit_cents itself is only computed in Rust afterwards (see
    /// map_sale_group), so the ORDER BY must spell out the same subtraction
    /// rather than reference a column that doesn't exist in the query.
    #[test]
    fn list_sale_groups_sorts_by_revenue_and_profit_both_directions() {
        let mut conn = test_conn();
        // seed_tickets' order always costs 1000 cents/ticket, so with equal
        // cost and zero fees, profit ordering tracks revenue ordering
        // exactly - this still genuinely exercises the SQL profit
        // expression (the thing that broke before this fix), just via data
        // where the expected order is easy to state.
        let tickets = seed_tickets(&mut conn, 3);
        create_sale_impl(&mut conn, &sale_input(tickets[0], 1000)).unwrap(); // profit 0
        create_sale_impl(&mut conn, &sale_input(tickets[1], 3000)).unwrap(); // profit 2000
        create_sale_impl(&mut conn, &sale_input(tickets[2], 2000)).unwrap(); // profit 1000

        let revenue_desc = list_sale_groups_impl(
            &conn, None, None, None, None, None, None, None, None, Some("revenue_desc".into()),
        None)
        .unwrap();
        assert_eq!(revenue_desc.iter().map(|g| g.revenue_cents).collect::<Vec<_>>(), vec![3000, 2000, 1000]);

        let revenue_asc = list_sale_groups_impl(
            &conn, None, None, None, None, None, None, None, None, Some("revenue_asc".into()),
        None)
        .unwrap();
        assert_eq!(revenue_asc.iter().map(|g| g.revenue_cents).collect::<Vec<_>>(), vec![1000, 2000, 3000]);

        let profit_desc = list_sale_groups_impl(
            &conn, None, None, None, None, None, None, None, None, Some("profit_desc".into()),
        None)
        .unwrap();
        assert_eq!(profit_desc.iter().map(|g| g.profit_cents).collect::<Vec<_>>(), vec![2000, 1000, 0]);

        let profit_asc = list_sale_groups_impl(
            &conn, None, None, None, None, None, None, None, None, Some("profit_asc".into()),
        None)
        .unwrap();
        assert_eq!(profit_asc.iter().map(|g| g.profit_cents).collect::<Vec<_>>(), vec![0, 1000, 2000]);
    }

    /// 1.8.0 section 5: "Most Tickets" sort and the "oldest" direction (the
    /// default/unset case, i.e. plain "Newest", is already covered by every
    /// pre-1.8.0 test in this file that never passes sort_by at all).
    #[test]
    fn list_sale_groups_sorts_by_ticket_count_and_oldest_first() {
        let mut conn = test_conn();
        let single = seed_tickets(&mut conn, 1);
        create_sale_impl(&mut conn, &{
            let mut i = sale_input(single[0], 1000);
            i.sale_date = "2026-01-01".to_string();
            i
        })
        .unwrap();
        let batch = seed_tickets(&mut conn, 3);
        create_sales_batch_impl(&mut conn, &{
            let mut b = batch_input(&batch, 1000, "paid");
            b.sale_date = "2026-06-01".to_string();
            b
        })
        .unwrap();

        let by_tickets = list_sale_groups_impl(
            &conn, None, None, None, None, None, None, None, None, Some("tickets_desc".into()),
        None)
        .unwrap();
        assert_eq!(by_tickets[0].ticket_count, 3, "the 3-ticket batch must sort first");
        assert_eq!(by_tickets[1].ticket_count, 1);

        let oldest_first = list_sale_groups_impl(
            &conn, None, None, None, None, None, None, None, None, Some("oldest".into()),
        None)
        .unwrap();
        assert_eq!(oldest_first[0].sale_date, "2026-01-01", "the earlier sale must come first when sorted oldest");
    }

    /// 2.0.65: "soonest"/"furthest" are new synonyms for "oldest"/the unset
    /// default respectively (see this function's own doc comment) - same
    /// data and assertions as `list_sale_groups_sorts_by_ticket_count_and_
    /// oldest_first` above, just spelled with the new values Sales.tsx now
    /// actually sends.
    #[test]
    fn list_sale_groups_sorts_by_soonest_and_furthest_first() {
        let mut conn = test_conn();
        let single = seed_tickets(&mut conn, 1);
        create_sale_impl(&mut conn, &{
            let mut i = sale_input(single[0], 1000);
            i.sale_date = "2026-01-01".to_string();
            i
        })
        .unwrap();
        let batch = seed_tickets(&mut conn, 3);
        create_sales_batch_impl(&mut conn, &{
            let mut b = batch_input(&batch, 1000, "paid");
            b.sale_date = "2026-06-01".to_string();
            b
        })
        .unwrap();

        let soonest_first = list_sale_groups_impl(
            &conn, None, None, None, None, None, None, None, None, Some("soonest".into()),
        None)
        .unwrap();
        assert_eq!(soonest_first[0].sale_date, "2026-01-01", "soonest must be an ascending synonym for oldest");

        let furthest_first = list_sale_groups_impl(
            &conn, None, None, None, None, None, None, None, None, Some("furthest".into()),
        None)
        .unwrap();
        assert_eq!(furthest_first[0].sale_date, "2026-06-01", "furthest must be a descending synonym for the unset default");
    }

    /// 1.8.0 section 4: refund status must distinguish partially- from
    /// fully-refunded groups, not just "has any refund at all".
    #[test]
    fn list_sale_groups_refund_status_distinguishes_partial_from_full() {
        let mut conn = test_conn();
        let partial_tickets = seed_tickets(&mut conn, 4);
        let partial_ids = create_sales_batch_impl(&mut conn, &batch_input(&partial_tickets, 1000, "paid")).unwrap();
        refund_sale_impl(&mut conn, partial_ids[0], None).unwrap(); // 1 of 4 refunded

        let full_tickets = seed_tickets(&mut conn, 2);
        let full_ids = create_sales_batch_impl(&mut conn, &batch_input(&full_tickets, 1000, "paid")).unwrap();
        refund_sale_impl(&mut conn, full_ids[0], None).unwrap();
        refund_sale_impl(&mut conn, full_ids[1], None).unwrap(); // 2 of 2 refunded

        let clean_tickets = seed_tickets(&mut conn, 1);
        create_sale_impl(&mut conn, &sale_input(clean_tickets[0], 1000)).unwrap(); // 0 refunded

        let partial = list_sale_groups_impl(
            &conn, None, None, None, None, None, None, Some("partial_refund".into()), None, None,
        None)
        .unwrap();
        assert_eq!(partial.len(), 1);
        assert_eq!(partial[0].refunded_count, 1);
        assert_eq!(partial[0].ticket_count, 4);

        let full = list_sale_groups_impl(
            &conn, None, None, None, None, None, None, Some("full_refund".into()), None, None,
        None)
        .unwrap();
        assert_eq!(full.len(), 1);
        assert_eq!(full[0].refunded_count, 2);
        assert_eq!(full[0].ticket_count, 2);

        let none = list_sale_groups_impl(
            &conn, None, None, None, None, None, None, Some("no_refund".into()), None, None,
        None)
        .unwrap();
        assert_eq!(none.len(), 1);
        assert_eq!(none[0].refunded_count, 0);
    }

    /// 1.8.0: the Currency filter dropdown's data source.
    #[test]
    fn list_sale_currencies_returns_distinct_sorted_currencies() {
        let mut conn = test_conn();
        conn.execute("INSERT INTO events (name) VALUES ('Currencies Event')", [])
            .unwrap();
        let event_id = conn.last_insert_rowid();
        let usd = seed_ticket_with_currency(&mut conn, event_id, "USD");
        let eur1 = seed_ticket_with_currency(&mut conn, event_id, "EUR");
        let eur2 = seed_ticket_with_currency(&mut conn, event_id, "EUR");
        create_sale_impl(&mut conn, &sale_input(usd, 1000)).unwrap();
        create_sale_impl(&mut conn, &sale_input(eur1, 1000)).unwrap();
        create_sale_impl(&mut conn, &sale_input(eur2, 1000)).unwrap();

        let currencies = list_sale_currencies_impl(&conn).unwrap();
        assert_eq!(currencies, vec!["EUR".to_string(), "USD".to_string()], "distinct and alphabetically sorted");
    }

    /// 1.8.0: "Export selected" resolves each selected representative id to
    /// its FULL group (every line), not just that one row, and de-dupes if
    /// the same underlying group is reachable via more than one selected id.
    #[test]
    fn resolve_group_sale_ids_expands_batches_and_dedupes() {
        let mut conn = test_conn();
        let single = seed_tickets(&mut conn, 1);
        let single_id = create_sale_impl(&mut conn, &sale_input(single[0], 1000)).unwrap();

        let batch = seed_tickets(&mut conn, 3);
        let batch_ids = create_sales_batch_impl(&mut conn, &batch_input(&batch, 1000, "paid")).unwrap();

        // Select the single sale, plus the batch via TWO of its own lines -
        // the second reference must not duplicate the batch's lines.
        let resolved = resolve_group_sale_ids(&conn, &[single_id, batch_ids[0], batch_ids[1]]).unwrap();

        let mut expected = vec![single_id, batch_ids[0], batch_ids[1], batch_ids[2]];
        expected.sort_unstable();
        assert_eq!(resolved, expected, "every line of every selected group, exactly once each");
    }

    // 1.9.2 (sections 3/4): the new Sale Detail "Mark as Paid"/"Mark as
    // Pending" bulk action, replacing the old general Bulk Ticket Edit bar
    // there. See `bulk_update_sale_payment_status_impl`'s doc comment for the
    // "validate everything, then write everything" contract these tests
    // check.

    #[test]
    fn bulk_update_sale_payment_status_only_changes_the_selected_sales_out_of_four() {
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 4);
        let sale_ids =
            create_sales_batch_impl(&mut conn, &batch_input(&tickets, 1000, "pending")).unwrap();
        let selected = vec![sale_ids[0], sale_ids[1], sale_ids[2]];
        let untouched = sale_ids[3];

        let updated_ids =
            bulk_update_sale_payment_status_impl(&mut conn, &selected, "paid").unwrap();
        assert_eq!(updated_ids.len(), 3);

        for &id in &selected {
            assert_eq!(sale_payment_status(&conn, id), "paid");
        }
        assert_eq!(
            sale_payment_status(&conn, untouched),
            "pending",
            "the 4th sale was never selected, so it must stay untouched"
        );
    }

    #[test]
    fn bulk_update_sale_payment_status_rejects_a_refunded_sale_and_changes_nothing() {
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 3);
        let sale_ids =
            create_sales_batch_impl(&mut conn, &batch_input(&tickets, 1000, "paid")).unwrap();
        refund_sale_impl(&mut conn, sale_ids[0], Some("buyer cancelled")).unwrap();

        let result = bulk_update_sale_payment_status_impl(&mut conn, &sale_ids, "pending");
        assert!(
            result.is_err(),
            "a batch containing a refunded sale must be rejected entirely"
        );

        assert_eq!(sale_payment_status(&conn, sale_ids[0]), "refunded");
        for &id in &sale_ids[1..] {
            assert_eq!(
                sale_payment_status(&conn, id),
                "paid",
                "a failed bulk update must change nothing at all"
            );
        }
    }

    #[test]
    fn bulk_update_sale_payment_status_is_all_or_nothing_with_a_missing_id() {
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 2);
        let sale_ids =
            create_sales_batch_impl(&mut conn, &batch_input(&tickets, 1000, "pending")).unwrap();

        let mut selection = sale_ids.clone();
        selection.push(999_999);
        let result = bulk_update_sale_payment_status_impl(&mut conn, &selection, "paid");
        assert!(result.is_err());

        for &id in &sale_ids {
            assert_eq!(
                sale_payment_status(&conn, id),
                "pending",
                "a failed bulk update must change nothing at all"
            );
        }
    }

    #[test]
    fn bulk_update_sale_payment_status_rejects_refunded_as_a_target_status() {
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 1);
        let sale_id = create_sale_impl(&mut conn, &sale_input(tickets[0], 1000)).unwrap();

        let result = bulk_update_sale_payment_status_impl(&mut conn, &[sale_id], "refunded");
        assert!(
            result.is_err(),
            "refunding must only ever happen via the dedicated refund action"
        );
        assert_eq!(sale_payment_status(&conn, sale_id), "paid");
    }

    #[test]
    fn bulk_update_sale_payment_status_rejects_empty_selection() {
        let mut conn = test_conn();
        let result = bulk_update_sale_payment_status_impl(&mut conn, &[], "paid");
        assert!(result.is_err());
    }

    #[test]
    fn bulk_update_sale_payment_status_dedupes_ids() {
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 1);
        let sale_id = create_sale_impl(&mut conn, &sale_input(tickets[0], 1000)).unwrap();
        // sale_input() already creates it as "paid" - move it to "pending"
        // first so the assertion below observes a real change, not a no-op.
        bulk_update_sale_payment_status_impl(&mut conn, &[sale_id], "pending").unwrap();

        let updated_ids = bulk_update_sale_payment_status_impl(
            &mut conn,
            &[sale_id, sale_id, sale_id],
            "paid",
        )
        .unwrap();
        assert_eq!(updated_ids, vec![sale_id]);
        assert_eq!(sale_payment_status(&conn, sale_id), "paid");
    }

    #[test]
    fn bulk_update_sale_payment_status_can_move_paid_sales_back_to_pending() {
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 2);
        let sale_ids =
            create_sales_batch_impl(&mut conn, &batch_input(&tickets, 1000, "paid")).unwrap();

        let updated_ids =
            bulk_update_sale_payment_status_impl(&mut conn, &sale_ids, "pending").unwrap();
        assert_eq!(updated_ids.len(), 2);
        for &id in &sale_ids {
            assert_eq!(sale_payment_status(&conn, id), "pending");
        }
    }

    /// Mirrors tickets.rs's
    /// `bulk_update_tickets_impl_changes_selected_fields_and_ignores_status`:
    /// a bulk *sale* payment-status change must never touch `tickets.status`
    /// - the ticket lifecycle (available/listed/sold/cancelled) and the
    /// money-owed state (pending/paid/refunded) are separate state machines.
    #[test]
    fn bulk_update_sale_payment_status_does_not_disturb_ticket_status() {
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 2);
        let sale_ids =
            create_sales_batch_impl(&mut conn, &batch_input(&tickets, 1000, "paid")).unwrap();
        for &id in &tickets {
            assert_eq!(ticket_status(&conn, id), "sold");
        }

        bulk_update_sale_payment_status_impl(&mut conn, &sale_ids, "pending").unwrap();

        for &id in &sale_ids {
            assert_eq!(sale_payment_status(&conn, id), "pending");
        }
        for &id in &tickets {
            assert_eq!(
                ticket_status(&conn, id),
                "sold",
                "a bulk payment-status change must never touch ticket status"
            );
        }
    }
}

use crate::codes;
use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::finance;
use crate::models::{Sale, SaleBatchInput, SaleEditInput, SaleGroup, SaleInput};
use rusqlite::{params, Connection, Row};
use std::collections::HashSet;
use tauri::State;

// Safety cap on the unfiltered list view - see the identical constant in
// commands/tickets.rs for the rationale.
const LIST_CAP: i64 = 5000;

const BASE_SQL: &str = "
    SELECT s.id, s.code, s.ticket_id, t.code as ticket_code,
      t.section, t.row_label, t.seat,
      t.event_id, e.name as event_name,
      s.platform_id, p.name as platform_name, s.sale_date, s.sale_price_cents, s.selling_fees_cents,
      s.currency, s.payment_status, s.buyer_reference, s.notes, s.is_demo, s.created_at, s.updated_at,
      s.refunded_at, s.refund_reason, s.batch_id,
      (t.purchase_cost_cents + t.purchase_fees_cents + t.other_costs_cents) as cost_cents
    FROM sales s
    JOIN tickets t ON t.id = s.ticket_id
    JOIN events e ON e.id = t.event_id
    LEFT JOIN platforms p ON p.id = s.platform_id
";

/// Groups sales rows that were submitted together as one "New sale" action.
/// NULL batch_id (an ordinary single-ticket sale) is its own group of one -
/// `'single:' || id` can never collide with a real batch_id, which is always
/// a `SAL-xxxxxx` code (see migration 003).
const GROUP_KEY_EXPR: &str = "COALESCE(s.batch_id, 'single:' || s.id)";

fn map_sale(row: &Row) -> rusqlite::Result<Sale> {
    let sale_price_cents: i64 = row.get("sale_price_cents")?;
    let selling_fees_cents: i64 = row.get("selling_fees_cents")?;
    let cost_cents: i64 = row.get("cost_cents")?;
    let profit = finance::profit_cents(sale_price_cents, cost_cents, selling_fees_cents);
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
        platform_id: row.get("platform_id")?,
        platform_name: row.get("platform_name")?,
        sale_date: row.get("sale_date")?,
        sale_price_cents,
        selling_fees_cents,
        currency: row.get("currency")?,
        payment_status: row.get("payment_status")?,
        buyer_reference: row.get("buyer_reference")?,
        notes: row.get("notes")?,
        is_demo: row.get("is_demo")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        cost_cents,
        profit_cents: profit,
        margin: finance::safe_ratio(profit, sale_price_cents),
        roi: finance::safe_ratio(profit, cost_cents),
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
    })
}

pub(crate) fn fetch_recent(conn: &Connection, limit: i64) -> AppResult<Vec<Sale>> {
    let sql = format!("{BASE_SQL} ORDER BY s.created_at DESC LIMIT ?1");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([limit], map_sale)?;
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

const GROUP_BASE_SELECT: &str = "
    SELECT
      MIN(s.id) as id,
      MIN(s.code) as code,
      MAX(s.batch_id) as batch_id,
      COUNT(*) as ticket_count,
      CASE WHEN COUNT(DISTINCT t.event_id) = 1 THEN MAX(t.event_id) END as event_id,
      CASE WHEN COUNT(DISTINCT t.event_id) = 1 THEN MAX(e.name) END as event_name,
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
      CASE
        WHEN COUNT(DISTINCT CASE WHEN s.payment_status != 'refunded' THEN s.currency END) = 1
          THEN MAX(CASE WHEN s.payment_status != 'refunded' THEN s.currency END)
        WHEN COUNT(DISTINCT CASE WHEN s.payment_status != 'refunded' THEN s.currency END) = 0
          THEN CASE WHEN COUNT(DISTINCT s.currency) = 1 THEN MAX(s.currency) END
        ELSE NULL
      END as currency,
      COALESCE(SUM(CASE WHEN s.payment_status != 'refunded' THEN s.sale_price_cents END), 0) as revenue_cents,
      COALESCE(SUM(CASE WHEN s.payment_status != 'refunded' THEN s.selling_fees_cents END), 0) as selling_fees_cents,
      COALESCE(SUM(CASE WHEN s.payment_status != 'refunded' THEN (t.purchase_cost_cents+t.purchase_fees_cents+t.other_costs_cents) END), 0) as cost_cents,
      CASE WHEN COUNT(DISTINCT s.payment_status) = 1 THEN MAX(s.payment_status) END as payment_status,
      SUM(CASE WHEN s.payment_status = 'refunded' THEN 1 ELSE 0 END) as refunded_count,
      MAX(s.is_demo) as is_demo
    FROM sales s
    JOIN tickets t ON t.id = s.ticket_id
    JOIN events e ON e.id = t.event_id
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
) -> AppResult<Vec<SaleGroup>> {
    let mut inner_sql = String::from(
        "SELECT DISTINCT COALESCE(s3.batch_id, 'single:' || s3.id) FROM sales s3
         JOIN tickets t3 ON t3.id = s3.ticket_id
         JOIN events e3 ON e3.id = t3.event_id
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
    if let Some(ps) = payment_status.as_deref() {
        if !ps.is_empty() {
            inner_sql.push_str(" AND s3.payment_status = ?");
            inner_params.push(Box::new(ps.to_string()));
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
            inner_sql.push_str(
                " AND (s3.code LIKE ? OR t3.code LIKE ? OR e3.name LIKE ? OR s3.buyer_reference LIKE ?)",
            );
            let like = format!("%{q}%");
            for _ in 0..4 {
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
            _ => {}
        }
    }

    sql.push_str(&format!(" ORDER BY sale_date DESC, id DESC LIMIT {LIST_CAP}"));

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
    )
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

        let (currency,): (String,) = tx.query_row(
            "SELECT currency FROM tickets WHERE id = ?1",
            [line.ticket_id],
            |r| Ok((r.get(0)?,)),
        )?;

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
fn delete_sale_group_impl(conn: &mut Connection, id: i64) -> AppResult<usize> {
    let tx = conn.transaction()?;

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
            notes: None,
        }
    }

    fn all_groups(conn: &Connection) -> Vec<SaleGroup> {
        list_sale_groups_impl(conn, None, None, None, None, None, None, None).unwrap()
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
            list_sale_groups_impl(&conn, None, Some(event_id), None, None, None, None, None).unwrap();
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
                list_sale_groups_impl(&conn, None, Some(eid), None, None, None, None, None).unwrap();
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
            list_sale_groups_impl(&conn, None, None, None, None, None, None, Some("has_refund".into()))
                .unwrap();
        assert_eq!(has_refund.len(), 1);
        assert_eq!(has_refund[0].refunded_count, 1);

        let no_refund =
            list_sale_groups_impl(&conn, None, None, None, None, None, None, Some("no_refund".into()))
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
    /// mixed-currency tests below. A sale's own currency is always copied
    /// from its ticket (see `create_sale_impl`/`create_sales_batch_impl`
    /// above), so the only way to get a mixed-currency batch is to sell
    /// tickets that were themselves purchased in different currencies.
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
}

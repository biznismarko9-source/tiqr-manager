use crate::codes;
use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::finance;
use crate::models::{Sale, SaleBatchInput, SaleEditInput, SaleInput};
use rusqlite::{params, Connection, Row};
use std::collections::HashSet;
use tauri::State;

// Safety cap on the unfiltered list view - see the identical constant in
// commands/tickets.rs for the rationale.
const LIST_CAP: i64 = 5000;

const BASE_SQL: &str = "
    SELECT s.id, s.code, s.ticket_id, t.code as ticket_code, t.event_id, e.name as event_name,
      s.platform_id, p.name as platform_name, s.sale_date, s.sale_price_cents, s.selling_fees_cents,
      s.currency, s.payment_status, s.buyer_reference, s.notes, s.is_demo, s.created_at, s.updated_at,
      s.refunded_at, s.refund_reason,
      (t.purchase_cost_cents + t.purchase_fees_cents + t.other_costs_cents) as cost_cents
    FROM sales s
    JOIN tickets t ON t.id = s.ticket_id
    JOIN events e ON e.id = t.event_id
    LEFT JOIN platforms p ON p.id = s.platform_id
";

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
fn create_sale_impl(conn: &mut Connection, input: &SaleInput) -> AppResult<i64> {
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
fn create_sales_batch_impl(conn: &mut Connection, input: &SaleBatchInput) -> AppResult<Vec<i64>> {
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
               selling_fees_cents, currency, payment_status, buyer_reference, notes)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
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
fn refund_sale_impl(conn: &mut Connection, id: i64, reason: Option<&str>) -> AppResult<()> {
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

/// Core logic behind `delete_sale`. This is for correcting a genuine mistake
/// (e.g. the wrong ticket was picked) - nothing of value ever really
/// happened, so nothing needs to stay on record. A real refund to an actual
/// buyer should use `refund_sale_impl` instead, which keeps the sale in
/// history with a `refunded` status - so this refuses to touch one.
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
    // A refunded sale is history, not a mistake to undo - deleting it here
    // would silently destroy the very record refund_sale exists to keep.
    if payment_status == "refunded" {
        return Err(AppError::Validation(
            "This sale has been refunded and is kept as history - it can't be deleted.".into(),
        ));
    }

    tx.execute("DELETE FROM sales WHERE id = ?1", [id])?;
    tx.execute(
        "UPDATE tickets SET status='available', updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",
        [ticket_id],
    )?;
    tx.commit()?;
    Ok(())
}

#[tauri::command]
pub fn delete_sale(state: State<AppState>, id: i64) -> AppResult<()> {
    let mut conn = state.db.lock().unwrap();
    delete_sale_impl(&mut conn, id)
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
    fn refunded_sale_cannot_be_hard_deleted() {
        let mut conn = test_conn();
        let tickets = seed_tickets(&mut conn, 1);
        let sale_id = create_sale_impl(&mut conn, &sale_input(tickets[0], 1000)).unwrap();
        refund_sale_impl(&mut conn, sale_id, None).unwrap();

        let err = delete_sale_impl(&mut conn, sale_id).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));

        let still_present: i64 = conn
            .query_row("SELECT COUNT(*) FROM sales WHERE id = ?1", [sale_id], |r| r.get(0))
            .unwrap();
        assert_eq!(still_present, 1, "refunded sale row must still exist");
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
}

use crate::codes;
use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::finance::{self, allocate_cents};
use crate::models::{BulkDeleteResult, BulkDeleteSkip, Order, OrderEditInput, OrderInput, OrderSalesSummary};
use rusqlite::{params, Connection, Row};
use tauri::State;

// Safety cap on the unfiltered list view - see the identical constant in
// commands/tickets.rs for the rationale.
const LIST_CAP: i64 = 5000;

const BASE_SQL: &str = "
    SELECT
      o.id, o.code, o.event_id, e.name as event_name,
      e.category_id, ec.name as category_name, ec.color_slot as category_color_slot,
      o.supplier_id, sup.name as supplier_name,
      o.platform_id, p.name as platform_name,
      o.purchase_date, o.quantity, o.unit_price_cents, o.fees_cents, o.other_costs_cents,
      o.total_cost_cents, o.currency, o.payment_status, o.notes, o.is_demo,
      o.created_at, o.updated_at,
      COUNT(CASE WHEN t.status='sold' THEN 1 END) as sold_count,
      COUNT(CASE WHEN t.status='available' THEN 1 END) as available_count,
      COUNT(CASE WHEN t.status='listed' THEN 1 END) as listed_count,
      COUNT(CASE WHEN t.status='cancelled' THEN 1 END) as cancelled_count
    FROM orders o
    JOIN events e ON e.id = o.event_id
    LEFT JOIN event_categories ec ON ec.id = e.category_id
    LEFT JOIN suppliers sup ON sup.id = o.supplier_id
    LEFT JOIN platforms p ON p.id = o.platform_id
    LEFT JOIN tickets t ON t.order_id = o.id
";

fn map_order(row: &Row) -> rusqlite::Result<Order> {
    Ok(Order {
        id: row.get("id")?,
        code: row.get("code")?,
        event_id: row.get("event_id")?,
        event_name: row.get("event_name")?,
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
        "INSERT INTO tickets (code, event_id, order_id, section, row_label, seat, ticket_type,
           purchase_cost_cents, purchase_fees_cents, other_costs_cents, currency, status, is_demo)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,'available',?12)",
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

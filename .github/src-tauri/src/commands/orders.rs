use crate::codes;
use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::finance::allocate_cents;
use crate::models::{Order, OrderEditInput, OrderInput};
use rusqlite::{params, Connection, Row};
use tauri::State;

const BASE_SQL: &str = "
    SELECT
      o.id, o.code, o.event_id, e.name as event_name,
      o.supplier_id, sup.name as supplier_name,
      o.platform_id, p.name as platform_name,
      o.purchase_date, o.quantity, o.unit_price_cents, o.fees_cents, o.other_costs_cents,
      o.total_cost_cents, o.currency, o.payment_status, o.notes, o.is_demo,
      o.created_at, o.updated_at,
      COUNT(CASE WHEN t.status='sold' THEN 1 END) as sold_count,
      COUNT(CASE WHEN t.status='available' THEN 1 END) as available_count
    FROM orders o
    JOIN events e ON e.id = o.event_id
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

#[tauri::command]
pub fn list_orders(
    state: State<AppState>,
    search: Option<String>,
    event_id: Option<i64>,
) -> AppResult<Vec<Order>> {
    let conn = state.db.lock().unwrap();
    let mut sql = format!("{BASE_SQL} WHERE 1=1");
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![];

    if let Some(eid) = event_id {
        sql.push_str(" AND o.event_id = ?");
        params_vec.push(Box::new(eid));
    }
    if let Some(q) = search.as_deref() {
        let q = q.trim();
        if !q.is_empty() {
            sql.push_str(" AND (o.code LIKE ? OR e.name LIKE ? OR sup.name LIKE ? OR p.name LIKE ?)");
            let like = format!("%{q}%");
            for _ in 0..4 {
                params_vec.push(Box::new(like.clone()));
            }
        }
    }
    sql.push_str(" GROUP BY o.id ORDER BY o.purchase_date DESC, o.id DESC");

    let mut stmt = conn.prepare(&sql)?;
    let param_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
    let rows = stmt.query_map(param_refs.as_slice(), map_order)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[tauri::command]
pub fn get_order(state: State<AppState>, id: i64) -> AppResult<Order> {
    let conn = state.db.lock().unwrap();
    fetch_one(&conn, id)
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

#[tauri::command]
pub fn create_order(state: State<AppState>, input: OrderInput) -> AppResult<Order> {
    let mut conn = state.db.lock().unwrap();
    let tx = conn.transaction()?;
    let order_id = insert_order_with_tickets(&tx, &input, false)?;
    tx.commit()?;
    fetch_one(&conn, order_id)
}

#[tauri::command]
pub fn update_order(state: State<AppState>, id: i64, input: OrderEditInput) -> AppResult<Order> {
    if input.purchase_date.trim().is_empty() {
        return Err(AppError::Validation("Purchase date is required".into()));
    }
    if !["unpaid", "partial", "paid"].contains(&input.payment_status.as_str()) {
        return Err(AppError::Validation("Invalid payment status".into()));
    }
    let conn = state.db.lock().unwrap();
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
    fetch_one(&conn, id)
}

#[tauri::command]
pub fn delete_order(state: State<AppState>, id: i64) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    let sold_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tickets WHERE order_id = ?1 AND status = 'sold'",
        [id],
        |r| r.get(0),
    )?;
    if sold_count > 0 {
        return Err(AppError::Validation(
            "This order has sold tickets and cannot be deleted.".into(),
        ));
    }
    let changed = conn.execute("DELETE FROM orders WHERE id = ?1", [id])?;
    if changed == 0 {
        return Err(AppError::NotFound(format!("Order #{id} not found")));
    }
    Ok(())
}

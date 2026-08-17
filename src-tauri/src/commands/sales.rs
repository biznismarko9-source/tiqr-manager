use crate::codes;
use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::finance;
use crate::models::{Sale, SaleEditInput, SaleInput};
use rusqlite::{params, Connection, Row};
use tauri::State;

const BASE_SQL: &str = "
    SELECT s.id, s.code, s.ticket_id, t.code as ticket_code, t.event_id, e.name as event_name,
      s.platform_id, p.name as platform_name, s.sale_date, s.sale_price_cents, s.selling_fees_cents,
      s.currency, s.payment_status, s.buyer_reference, s.notes, s.is_demo, s.created_at, s.updated_at,
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
    sql.push_str(" ORDER BY s.sale_date DESC, s.id DESC");

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

#[tauri::command]
pub fn create_sale(state: State<AppState>, input: SaleInput) -> AppResult<Sale> {
    if input.sale_price_cents < 0 || input.selling_fees_cents < 0 {
        return Err(AppError::Validation("Amounts cannot be negative".into()));
    }
    if input.sale_date.trim().is_empty() {
        return Err(AppError::Validation("Sale date is required".into()));
    }

    let mut conn = state.db.lock().unwrap();
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
    fetch_one(&conn, sale_id)
}

#[tauri::command]
pub fn update_sale(state: State<AppState>, id: i64, input: SaleEditInput) -> AppResult<Sale> {
    if input.sale_price_cents < 0 || input.selling_fees_cents < 0 {
        return Err(AppError::Validation("Amounts cannot be negative".into()));
    }
    if !["pending", "paid", "refunded"].contains(&input.payment_status.as_str()) {
        return Err(AppError::Validation("Invalid payment status".into()));
    }
    let conn = state.db.lock().unwrap();
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
    fetch_one(&conn, id)
}

/// Deletes a sale and reverts its ticket back to "available" so a mistaken
/// sale can be undone cleanly.
#[tauri::command]
pub fn delete_sale(state: State<AppState>, id: i64) -> AppResult<()> {
    let mut conn = state.db.lock().unwrap();
    let tx = conn.transaction()?;
    let ticket_id: Option<i64> = tx
        .query_row("SELECT ticket_id FROM sales WHERE id = ?1", [id], |r| {
            r.get(0)
        })
        .ok();
    let ticket_id =
        ticket_id.ok_or_else(|| AppError::NotFound(format!("Sale #{id} not found")))?;

    tx.execute("DELETE FROM sales WHERE id = ?1", [id])?;
    tx.execute(
        "UPDATE tickets SET status='available', updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",
        [ticket_id],
    )?;
    tx.commit()?;
    Ok(())
}

use crate::db::AppState;
use crate::error::AppResult;
use crate::money::format_cents;
use tauri::State;

fn opt(v: Option<String>) -> String {
    v.unwrap_or_default()
}
fn yesno(b: bool) -> &'static str {
    if b {
        "yes"
    } else {
        "no"
    }
}

#[tauri::command]
pub fn export_events_csv(state: State<AppState>, path: String) -> AppResult<i64> {
    let conn = state.db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, name, artist_team, venue, city, country, event_date, category, status, notes, is_demo, created_at
         FROM events ORDER BY id",
    )?;
    let mut wtr = csv::Writer::from_path(&path)?;
    wtr.write_record([
        "id", "name", "artist_team", "venue", "city", "country", "event_date", "category",
        "status", "notes", "is_demo", "created_at",
    ])?;
    let mut count = 0i64;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        wtr.write_record([
            row.get::<_, i64>(0)?.to_string(),
            row.get::<_, String>(1)?,
            opt(row.get(2)?),
            opt(row.get(3)?),
            opt(row.get(4)?),
            opt(row.get(5)?),
            opt(row.get(6)?),
            opt(row.get(7)?),
            row.get::<_, String>(8)?,
            opt(row.get(9)?),
            yesno(row.get(10)?).to_string(),
            row.get::<_, String>(11)?,
        ])?;
        count += 1;
    }
    wtr.flush()?;
    Ok(count)
}

#[tauri::command]
pub fn export_orders_csv(state: State<AppState>, path: String) -> AppResult<i64> {
    let conn = state.db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT o.code, e.name, sup.name, p.name, o.purchase_date, o.quantity,
                o.unit_price_cents, o.fees_cents, o.other_costs_cents, o.total_cost_cents,
                o.currency, o.payment_status, o.notes, o.is_demo, o.created_at
         FROM orders o
         JOIN events e ON e.id = o.event_id
         LEFT JOIN suppliers sup ON sup.id = o.supplier_id
         LEFT JOIN platforms p ON p.id = o.platform_id
         ORDER BY o.id",
    )?;
    let mut wtr = csv::Writer::from_path(&path)?;
    wtr.write_record([
        "order_code", "event", "supplier", "platform", "purchase_date", "quantity",
        "unit_price", "fees", "other_costs", "total_cost", "currency", "payment_status",
        "notes", "is_demo", "created_at",
    ])?;
    let mut count = 0i64;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        wtr.write_record([
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            opt(row.get(2)?),
            opt(row.get(3)?),
            row.get::<_, String>(4)?,
            row.get::<_, i64>(5)?.to_string(),
            format_cents(row.get(6)?),
            format_cents(row.get(7)?),
            format_cents(row.get(8)?),
            format_cents(row.get(9)?),
            row.get::<_, String>(10)?,
            row.get::<_, String>(11)?,
            opt(row.get(12)?),
            yesno(row.get(13)?).to_string(),
            row.get::<_, String>(14)?,
        ])?;
        count += 1;
    }
    wtr.flush()?;
    Ok(count)
}

fn export_tickets_inner(
    state: &State<AppState>,
    path: &str,
    status_filter: Option<Vec<String>>,
    event_id: Option<i64>,
) -> AppResult<i64> {
    let conn = state.db.lock().unwrap();
    let mut sql = "SELECT t.code, e.name, o.code, t.section, t.row_label, t.seat, t.ticket_type,
                t.purchase_cost_cents, t.purchase_fees_cents, t.other_costs_cents, t.listing_price_cents,
                t.currency, t.status, t.notes, t.is_demo, t.created_at
         FROM tickets t
         JOIN events e ON e.id = t.event_id
         JOIN orders o ON o.id = t.order_id
         WHERE 1=1"
        .to_string();
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![];
    if let Some(statuses) = &status_filter {
        if !statuses.is_empty() {
            let placeholders = statuses.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            sql.push_str(&format!(" AND t.status IN ({placeholders})"));
            for s in statuses {
                params_vec.push(Box::new(s.clone()));
            }
        }
    }
    if let Some(eid) = event_id {
        sql.push_str(" AND t.event_id = ?");
        params_vec.push(Box::new(eid));
    }
    sql.push_str(" ORDER BY t.id");

    let mut stmt = conn.prepare(&sql)?;
    let mut wtr = csv::Writer::from_path(path)?;
    wtr.write_record([
        "ticket_code", "event", "order_code", "section", "row", "seat", "ticket_type",
        "purchase_cost", "purchase_fees", "other_costs", "listing_price", "currency",
        "status", "notes", "is_demo", "created_at",
    ])?;
    let mut count = 0i64;
    let param_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
    let mut rows = stmt.query(param_refs.as_slice())?;
    while let Some(row) = rows.next()? {
        let listing_price: Option<i64> = row.get(10)?;
        wtr.write_record([
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            opt(row.get(3)?),
            opt(row.get(4)?),
            opt(row.get(5)?),
            opt(row.get(6)?),
            format_cents(row.get(7)?),
            format_cents(row.get(8)?),
            format_cents(row.get(9)?),
            listing_price.map(format_cents).unwrap_or_default(),
            row.get::<_, String>(11)?,
            row.get::<_, String>(12)?,
            opt(row.get(13)?),
            yesno(row.get(14)?).to_string(),
            row.get::<_, String>(15)?,
        ])?;
        count += 1;
    }
    wtr.flush()?;
    Ok(count)
}

#[tauri::command]
pub fn export_tickets_csv(
    state: State<AppState>,
    path: String,
    status: Option<String>,
    event_id: Option<i64>,
) -> AppResult<i64> {
    let statuses = status.map(|s| {
        s.split(',')
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty())
            .collect::<Vec<_>>()
    });
    export_tickets_inner(&state, &path, statuses, event_id)
}

/// "Inventory" = current stock only (available + listed), excluding sold/cancelled.
#[tauri::command]
pub fn export_inventory_csv(
    state: State<AppState>,
    path: String,
    event_id: Option<i64>,
) -> AppResult<i64> {
    export_tickets_inner(
        &state,
        &path,
        Some(vec!["available".to_string(), "listed".to_string()]),
        event_id,
    )
}

#[tauri::command]
pub fn export_sales_csv(state: State<AppState>, path: String) -> AppResult<i64> {
    let conn = state.db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT s.code, t.code, e.name, p.name, s.sale_date, s.sale_price_cents, s.selling_fees_cents,
                (t.purchase_cost_cents+t.purchase_fees_cents+t.other_costs_cents) as cost_cents,
                s.currency, s.payment_status, s.buyer_reference, s.notes, s.is_demo, s.created_at
         FROM sales s
         JOIN tickets t ON t.id = s.ticket_id
         JOIN events e ON e.id = t.event_id
         LEFT JOIN platforms p ON p.id = s.platform_id
         ORDER BY s.id",
    )?;
    let mut wtr = csv::Writer::from_path(&path)?;
    wtr.write_record([
        "sale_code", "ticket_code", "event", "platform", "sale_date", "sale_price",
        "selling_fees", "cost", "profit", "currency", "payment_status", "buyer_reference",
        "notes", "is_demo", "created_at",
    ])?;
    let mut count = 0i64;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let sale_price: i64 = row.get(5)?;
        let fees: i64 = row.get(6)?;
        let cost: i64 = row.get(7)?;
        let profit = sale_price - cost - fees;
        wtr.write_record([
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            opt(row.get(3)?),
            row.get::<_, String>(4)?,
            format_cents(sale_price),
            format_cents(fees),
            format_cents(cost),
            format_cents(profit),
            row.get::<_, String>(8)?,
            row.get::<_, String>(9)?,
            opt(row.get(10)?),
            opt(row.get(11)?),
            yesno(row.get(12)?).to_string(),
            row.get::<_, String>(13)?,
        ])?;
        count += 1;
    }
    wtr.flush()?;
    Ok(count)
}

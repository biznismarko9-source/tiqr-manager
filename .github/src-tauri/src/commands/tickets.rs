use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::models::{Ticket, TicketUpdateInput};
use rusqlite::{params, Row};
use tauri::State;

// Safety cap on unfiltered list views. Ordinary use (hundreds to low
// thousands of tickets) never hits this; it only kicks in for very large,
// unfiltered inventories so the UI never has to serialize/render an
// unbounded number of rows in one go. Results are already ordered, so a
// capped result is simply "the most relevant N", not an arbitrary cut.
const LIST_CAP: i64 = 5000;

const BASE_SQL: &str = "
    SELECT t.id, t.code, t.event_id, e.name as event_name, t.order_id, o.code as order_code,
      t.section, t.row_label, t.seat, t.ticket_type,
      t.purchase_cost_cents, t.purchase_fees_cents, t.other_costs_cents,
      t.listing_price_cents, t.currency, t.status, t.notes, t.is_demo,
      t.created_at, t.updated_at, sa.sale_price_cents as sale_price_cents
    FROM tickets t
    JOIN events e ON e.id = t.event_id
    JOIN orders o ON o.id = t.order_id
    LEFT JOIN sales sa ON sa.ticket_id = t.id
";

fn map_ticket(row: &Row) -> rusqlite::Result<Ticket> {
    let purchase_cost_cents: i64 = row.get("purchase_cost_cents")?;
    let purchase_fees_cents: i64 = row.get("purchase_fees_cents")?;
    let other_costs_cents: i64 = row.get("other_costs_cents")?;
    Ok(Ticket {
        id: row.get("id")?,
        code: row.get("code")?,
        event_id: row.get("event_id")?,
        event_name: row.get("event_name")?,
        order_id: row.get("order_id")?,
        order_code: row.get("order_code")?,
        section: row.get("section")?,
        row_label: row.get("row_label")?,
        seat: row.get("seat")?,
        ticket_type: row.get("ticket_type")?,
        purchase_cost_cents,
        purchase_fees_cents,
        other_costs_cents,
        total_cost_cents: purchase_cost_cents + purchase_fees_cents + other_costs_cents,
        listing_price_cents: row.get("listing_price_cents")?,
        currency: row.get("currency")?,
        status: row.get("status")?,
        notes: row.get("notes")?,
        is_demo: row.get("is_demo")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        sale_price_cents: row.get("sale_price_cents")?,
    })
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn list_tickets(
    state: State<AppState>,
    search: Option<String>,
    status: Option<String>,
    event_id: Option<i64>,
    order_id: Option<i64>,
    sort_by: Option<String>,
    sort_dir: Option<String>,
) -> AppResult<Vec<Ticket>> {
    let conn = state.db.lock().unwrap();
    let mut sql = format!("{BASE_SQL} WHERE 1=1");
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![];

    if let Some(s) = status.as_deref() {
        if !s.is_empty() {
            // Accepts a single status or a comma-separated list (e.g. "available,listed").
            let statuses: Vec<String> = s
                .split(',')
                .map(|x| x.trim().to_string())
                .filter(|x| !x.is_empty())
                .collect();
            if statuses.len() == 1 {
                sql.push_str(" AND t.status = ?");
                params_vec.push(Box::new(statuses[0].clone()));
            } else if statuses.len() > 1 {
                let placeholders = statuses.iter().map(|_| "?").collect::<Vec<_>>().join(",");
                sql.push_str(&format!(" AND t.status IN ({placeholders})"));
                for st in statuses {
                    params_vec.push(Box::new(st));
                }
            }
        }
    }
    if let Some(eid) = event_id {
        sql.push_str(" AND t.event_id = ?");
        params_vec.push(Box::new(eid));
    }
    if let Some(oid) = order_id {
        sql.push_str(" AND t.order_id = ?");
        params_vec.push(Box::new(oid));
    }
    if let Some(q) = search.as_deref() {
        let q = q.trim();
        if !q.is_empty() {
            sql.push_str(" AND (t.code LIKE ? OR t.section LIKE ? OR t.seat LIKE ? OR t.row_label LIKE ? OR e.name LIKE ? OR o.code LIKE ?)");
            let like = format!("%{q}%");
            for _ in 0..6 {
                params_vec.push(Box::new(like.clone()));
            }
        }
    }

    let sort_col = match sort_by.as_deref() {
        Some("event") => "e.name",
        Some("status") => "t.status",
        Some("price") => "t.listing_price_cents",
        Some("cost") => "(t.purchase_cost_cents + t.purchase_fees_cents + t.other_costs_cents)",
        Some("created") => "t.created_at",
        Some("code") => "t.code",
        _ => "t.id",
    };
    let dir = match sort_dir.as_deref() {
        Some("asc") => "ASC",
        _ => "DESC",
    };
    sql.push_str(&format!(" ORDER BY {sort_col} {dir}, t.id DESC LIMIT {LIST_CAP}"));

    let mut stmt = conn.prepare(&sql)?;
    let param_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
    let rows = stmt.query_map(param_refs.as_slice(), map_ticket)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[tauri::command]
pub fn get_ticket(state: State<AppState>, id: i64) -> AppResult<Ticket> {
    let conn = state.db.lock().unwrap();
    let sql = format!("{BASE_SQL} WHERE t.id = ?1");
    conn.query_row(&sql, [id], map_ticket)
        .map_err(|_| AppError::NotFound(format!("Ticket #{id} not found")))
}

#[tauri::command]
pub fn update_ticket(
    state: State<AppState>,
    id: i64,
    input: TicketUpdateInput,
) -> AppResult<Ticket> {
    let conn = state.db.lock().unwrap();
    let current_status: String = conn
        .query_row("SELECT status FROM tickets WHERE id = ?1", [id], |r| {
            r.get(0)
        })
        .map_err(|_| AppError::NotFound(format!("Ticket #{id} not found")))?;

    if let Some(new_status) = &input.status {
        if !["available", "listed", "sold", "cancelled"].contains(&new_status.as_str()) {
            return Err(AppError::Validation(format!("Invalid status '{new_status}'")));
        }
        if (current_status == "sold" && new_status != "sold")
            || (new_status == "sold" && current_status != "sold")
        {
            return Err(AppError::Validation(
                "Ticket sold status can only be changed via the Sales screen (create or delete a sale)."
                    .into(),
            ));
        }
    }

    if let Some(price) = input.listing_price_cents {
        if price < 0 {
            return Err(AppError::Validation("Listing price cannot be negative".into()));
        }
    }

    let next_status = input.status.clone().unwrap_or(current_status);

    conn.execute(
        "UPDATE tickets SET section=?1, row_label=?2, seat=?3, ticket_type=?4,
         listing_price_cents=?5, status=?6, notes=?7, updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now')
         WHERE id=?8",
        params![
            input.section,
            input.row_label,
            input.seat,
            input.ticket_type,
            input.listing_price_cents,
            next_status,
            input.notes,
            id,
        ],
    )?;

    let sql = format!("{BASE_SQL} WHERE t.id = ?1");
    Ok(conn.query_row(&sql, [id], map_ticket)?)
}

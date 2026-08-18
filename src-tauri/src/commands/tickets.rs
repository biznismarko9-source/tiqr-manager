use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::models::{Ticket, TicketUpdateInput};
use rusqlite::{params, Connection, Row};
use tauri::State;

// Safety cap on unfiltered list views. Ordinary use (hundreds to low
// thousands of tickets) never hits this; it only kicks in for very large,
// unfiltered inventories so the UI never has to serialize/render an
// unbounded number of rows in one go. Results are already ordered, so a
// capped result is simply "the most relevant N", not an arbitrary cut.
const LIST_CAP: i64 = 5000;

// The `sa.payment_status != 'refunded'` join guard matters as of migration
// 004: a ticket can now legitimately have more than one `sales` row over its
// lifetime (a refunded sale plus a later active resale - see BUG #1 fix), so
// an unfiltered join here would fan a single ticket out into two result
// rows. Restricting the join to the ACTIVE sale (there is at most one, by
// construction - see idx_sales_ticket_active_unique) keeps this a true
// one-row-per-ticket view and makes `sale_price_cents` reflect the current
// sale, never a stale refunded one. Same pattern already used in
// orders.rs's fetch_sales_summary and events.rs's stats query.
const BASE_SQL: &str = "
    SELECT t.id, t.code, t.event_id, e.name as event_name, t.order_id, o.code as order_code,
      t.section, t.row_label, t.seat, t.ticket_type,
      t.purchase_cost_cents, t.purchase_fees_cents, t.other_costs_cents,
      t.listing_price_cents, t.currency, t.status, t.notes, t.is_demo,
      t.created_at, t.updated_at, sa.sale_price_cents as sale_price_cents
    FROM tickets t
    JOIN events e ON e.id = t.event_id
    JOIN orders o ON o.id = t.order_id
    LEFT JOIN sales sa ON sa.ticket_id = t.id AND sa.payment_status != 'refunded'
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

/// Split out from the `list_tickets` command (same pattern as
/// list_orders/list_sale_groups) so it's directly unit-testable against a
/// plain `&Connection` - in particular so BUG #1's fix can be verified end
/// to end: a ticket with both a refunded and a new active sale must still
/// come back as exactly one row here, carrying the active sale's price.
#[allow(clippy::too_many_arguments)]
pub(crate) fn list_tickets_impl(
    conn: &Connection,
    search: Option<String>,
    status: Option<String>,
    event_id: Option<i64>,
    order_id: Option<i64>,
    sort_by: Option<String>,
    sort_dir: Option<String>,
) -> AppResult<Vec<Ticket>> {
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
    list_tickets_impl(&conn, search, status, event_id, order_id, sort_by, sort_dir)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::orders::insert_order_with_tickets;
    use crate::commands::sales::{create_sale_impl, refund_sale_impl};
    use crate::db::test_conn;
    use crate::models::{OrderInput, SaleInput};

    fn seed_one_ticket(conn: &Connection) -> i64 {
        conn.execute("INSERT INTO events (name) VALUES ('Test Event')", [])
            .unwrap();
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
            ticket_type: None,
            section: None,
            row_label: None,
            seats: None,
        };
        let order_id = insert_order_with_tickets(conn, &input, false).unwrap();
        conn.query_row("SELECT id FROM tickets WHERE order_id=?1", [order_id], |r| r.get(0))
            .unwrap()
    }

    /// BUG #1 fix, ticket-view half: once a ticket can carry both a
    /// refunded sale and a new active one (migration 004), list_tickets_impl
    /// must still show that ticket exactly once - never fanned out into two
    /// rows by the LEFT JOIN sales - and its `sale_price_cents` must reflect
    /// the current active sale, not the refunded one.
    #[test]
    fn ticket_with_a_refunded_and_a_new_active_sale_appears_exactly_once() {
        let mut conn = test_conn();
        let ticket_id = seed_one_ticket(&conn);
        let ticket_code: String = conn
            .query_row("SELECT code FROM tickets WHERE id=?1", [ticket_id], |r| r.get(0))
            .unwrap();

        let first_sale = SaleInput {
            ticket_id,
            platform_id: None,
            sale_date: "2026-02-01".to_string(),
            sale_price_cents: 2000,
            selling_fees_cents: 0,
            payment_status: Some("paid".to_string()),
            buyer_reference: None,
            notes: None,
        };
        let sale_id_1 = create_sale_impl(&mut conn, &first_sale).unwrap();
        refund_sale_impl(&mut conn, sale_id_1, Some("buyer cancelled")).unwrap();

        let second_sale = SaleInput {
            sale_price_cents: 1800,
            ..first_sale
        };
        create_sale_impl(&mut conn, &second_sale).unwrap();

        let results = list_tickets_impl(&conn, Some(ticket_code), None, None, None, None, None).unwrap();
        assert_eq!(results.len(), 1, "the ticket must appear exactly once, never duplicated by the sales join");
        assert_eq!(
            results[0].sale_price_cents,
            Some(1800),
            "sale_price_cents must reflect the current active sale, not the refunded one"
        );

        // Same guarantee for get_ticket's single-row lookup.
        let sql = format!("{BASE_SQL} WHERE t.id = ?1");
        let single = conn.query_row(&sql, [ticket_id], map_ticket).unwrap();
        assert_eq!(single.sale_price_cents, Some(1800));
    }
}

use crate::commands::{events as events_cmd, orders as orders_cmd, sales as sales_cmd};
use crate::db::AppState;
use crate::error::AppResult;
use crate::finance;
use crate::models::DashboardData;
use chrono::{Datelike, Duration, Local, NaiveDate};
use tauri::State;

fn period_bounds(period: Option<&str>, from: Option<String>, to: Option<String>) -> (String, String) {
    let today = Local::now().date_naive();
    match period {
        Some("today") | None => (today.to_string(), today.to_string()),
        Some("7d") => ((today - Duration::days(6)).to_string(), today.to_string()),
        Some("30d") => ((today - Duration::days(29)).to_string(), today.to_string()),
        Some("month") => {
            let start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
            (start.to_string(), today.to_string())
        }
        Some("custom") => (
            from.filter(|s| !s.is_empty())
                .unwrap_or_else(|| "0001-01-01".to_string()),
            to.filter(|s| !s.is_empty())
                .unwrap_or_else(|| today.to_string()),
        ),
        Some("all") => ("0001-01-01".to_string(), "9999-12-31".to_string()),
        _ => (today.to_string(), today.to_string()),
    }
}

#[tauri::command]
pub fn get_dashboard(
    state: State<AppState>,
    period: Option<String>,
    from: Option<String>,
    to: Option<String>,
    event_id: Option<i64>,
    platform_id: Option<i64>,
) -> AppResult<DashboardData> {
    let conn = state.db.lock().unwrap();
    let (period_from, period_to) = period_bounds(period.as_deref(), from, to);

    // ---- current inventory snapshot (all time, not period filtered) ----
    let (
        purchased_tickets,
        available_tickets,
        listed_tickets,
        sold_tickets,
        cancelled_tickets,
        total_cost_cents,
        cogs_cents,
    ): (i64, i64, i64, i64, i64, i64, i64) = conn.query_row(
        "SELECT
            COUNT(*),
            COUNT(CASE WHEN status='available' THEN 1 END),
            COUNT(CASE WHEN status='listed' THEN 1 END),
            COUNT(CASE WHEN status='sold' THEN 1 END),
            COUNT(CASE WHEN status='cancelled' THEN 1 END),
            COALESCE(SUM(purchase_cost_cents+purchase_fees_cents+other_costs_cents),0),
            COALESCE(SUM(CASE WHEN status='sold' THEN purchase_cost_cents+purchase_fees_cents+other_costs_cents ELSE 0 END),0)
         FROM tickets",
        [],
        |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
            ))
        },
    )?;
    let (revenue_cents, selling_fees_cents): (i64, i64) = conn.query_row(
        "SELECT COALESCE(SUM(sale_price_cents),0), COALESCE(SUM(selling_fees_cents),0) FROM sales",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let inventory = finance::compute_summary(
        purchased_tickets,
        available_tickets,
        listed_tickets,
        sold_tickets,
        cancelled_tickets,
        total_cost_cents,
        cogs_cents,
        revenue_cents,
        selling_fees_cents,
    );

    // ---- period-filtered purchase activity (by order purchase_date) ----
    let mut purchase_sql = String::from(
        "SELECT COUNT(t.id), COALESCE(SUM(t.purchase_cost_cents+t.purchase_fees_cents+t.other_costs_cents),0)
         FROM tickets t JOIN orders o ON o.id = t.order_id
         WHERE o.purchase_date BETWEEN ?1 AND ?2",
    );
    let mut p_params: Vec<Box<dyn rusqlite::ToSql>> =
        vec![Box::new(period_from.clone()), Box::new(period_to.clone())];
    if let Some(eid) = event_id {
        purchase_sql.push_str(" AND t.event_id = ?3");
        p_params.push(Box::new(eid));
    }
    if let Some(pid) = platform_id {
        purchase_sql.push_str(if event_id.is_some() { " AND o.platform_id = ?4" } else { " AND o.platform_id = ?3" });
        p_params.push(Box::new(pid));
    }
    let p_refs: Vec<&dyn rusqlite::ToSql> = p_params.iter().map(|p| p.as_ref()).collect();
    let (period_purchased, period_total_cost): (i64, i64) =
        conn.query_row(&purchase_sql, p_refs.as_slice(), |r| Ok((r.get(0)?, r.get(1)?)))?;

    // ---- period-filtered sales activity (by sale_date) ----
    let mut sales_sql = String::from(
        "SELECT COUNT(*), COALESCE(SUM(s.sale_price_cents),0), COALESCE(SUM(s.selling_fees_cents),0),
            COALESCE(SUM(t.purchase_cost_cents+t.purchase_fees_cents+t.other_costs_cents),0)
         FROM sales s JOIN tickets t ON t.id = s.ticket_id
         WHERE s.sale_date BETWEEN ?1 AND ?2",
    );
    let mut s_params: Vec<Box<dyn rusqlite::ToSql>> =
        vec![Box::new(period_from.clone()), Box::new(period_to.clone())];
    if let Some(eid) = event_id {
        sales_sql.push_str(" AND t.event_id = ?3");
        s_params.push(Box::new(eid));
    }
    if let Some(pid) = platform_id {
        sales_sql.push_str(if event_id.is_some() { " AND s.platform_id = ?4" } else { " AND s.platform_id = ?3" });
        s_params.push(Box::new(pid));
    }
    let s_refs: Vec<&dyn rusqlite::ToSql> = s_params.iter().map(|p| p.as_ref()).collect();
    let (period_sold, period_revenue, period_fees, period_cogs): (i64, i64, i64, i64) =
        conn.query_row(&sales_sql, s_refs.as_slice(), |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })?;

    let period_summary = finance::compute_summary(
        period_purchased,
        0,
        0,
        period_sold,
        0,
        period_total_cost,
        period_cogs,
        period_revenue,
        period_fees,
    );

    let recent_orders = orders_cmd::fetch_recent(&conn, 5)?;
    let recent_sales = sales_cmd::fetch_recent(&conn, 5)?;
    let recent_events = events_cmd::fetch_recent(&conn, 5)?;

    Ok(DashboardData {
        inventory,
        period: period_summary,
        period_from,
        period_to,
        recent_orders,
        recent_sales,
        recent_events,
    })
}

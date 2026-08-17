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

    // ---- currency mix detection --------------------------------------
    // Every total below is computed for ONE currency only - mixing e.g. EUR
    // and USD cents into a single sum would be meaningless. If the database
    // only ever saw one currency (the common case) this changes nothing. If
    // it saw more than one, everything below is scoped to a single "primary"
    // currency (EUR if present, else whichever sorts first) and the UI is
    // told so it can warn instead of silently showing a blended number.
    let currencies: Vec<String> = {
        // Already sorted by the ORDER BY, so "whichever sorts first" below
        // just means the first element.
        let mut stmt = conn.prepare("SELECT DISTINCT currency FROM tickets ORDER BY currency")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    let mixed_currencies = currencies.len() > 1;
    let primary_currency = if currencies.is_empty() {
        "EUR".to_string()
    } else if currencies.iter().any(|c| c == "EUR") {
        "EUR".to_string()
    } else {
        currencies[0].clone()
    };

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
         FROM tickets
         WHERE currency = ?1",
        [&primary_currency],
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
    // Refunded sales are excluded - they must never count as revenue.
    let (revenue_cents, selling_fees_cents): (i64, i64) = conn.query_row(
        "SELECT COALESCE(SUM(sale_price_cents),0), COALESCE(SUM(selling_fees_cents),0)
         FROM sales WHERE currency = ?1 AND payment_status != 'refunded'",
        [&primary_currency],
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
        Some(primary_currency.clone()),
    );

    // ---- period-filtered purchase activity (by order purchase_date) ----
    let mut purchase_sql = String::from(
        "SELECT COUNT(t.id), COALESCE(SUM(t.purchase_cost_cents+t.purchase_fees_cents+t.other_costs_cents),0)
         FROM tickets t JOIN orders o ON o.id = t.order_id
         WHERE o.purchase_date BETWEEN ?1 AND ?2 AND t.currency = ?3",
    );
    let mut p_params: Vec<Box<dyn rusqlite::ToSql>> = vec![
        Box::new(period_from.clone()),
        Box::new(period_to.clone()),
        Box::new(primary_currency.clone()),
    ];
    if let Some(eid) = event_id {
        purchase_sql.push_str(&format!(" AND t.event_id = ?{}", p_params.len() + 1));
        p_params.push(Box::new(eid));
    }
    if let Some(pid) = platform_id {
        purchase_sql.push_str(&format!(" AND o.platform_id = ?{}", p_params.len() + 1));
        p_params.push(Box::new(pid));
    }
    let p_refs: Vec<&dyn rusqlite::ToSql> = p_params.iter().map(|p| p.as_ref()).collect();
    let (period_purchased, period_total_cost): (i64, i64) =
        conn.query_row(&purchase_sql, p_refs.as_slice(), |r| Ok((r.get(0)?, r.get(1)?)))?;

    // ---- period-filtered sales activity (by sale_date) ----
    // Refunded sales excluded here too, and only the primary currency counts.
    let mut sales_sql = String::from(
        "SELECT COUNT(*), COALESCE(SUM(s.sale_price_cents),0), COALESCE(SUM(s.selling_fees_cents),0),
            COALESCE(SUM(t.purchase_cost_cents+t.purchase_fees_cents+t.other_costs_cents),0)
         FROM sales s JOIN tickets t ON t.id = s.ticket_id
         WHERE s.sale_date BETWEEN ?1 AND ?2 AND s.currency = ?3 AND s.payment_status != 'refunded'",
    );
    let mut s_params: Vec<Box<dyn rusqlite::ToSql>> = vec![
        Box::new(period_from.clone()),
        Box::new(period_to.clone()),
        Box::new(primary_currency.clone()),
    ];
    if let Some(eid) = event_id {
        sales_sql.push_str(&format!(" AND t.event_id = ?{}", s_params.len() + 1));
        s_params.push(Box::new(eid));
    }
    if let Some(pid) = platform_id {
        sales_sql.push_str(&format!(" AND s.platform_id = ?{}", s_params.len() + 1));
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
        Some(primary_currency.clone()),
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
        primary_currency,
        mixed_currencies,
    })
}

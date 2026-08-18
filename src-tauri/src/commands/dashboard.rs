use crate::commands::{events as events_cmd, orders as orders_cmd, sales as sales_cmd};
use crate::db::AppState;
use crate::error::AppResult;
use crate::finance;
use crate::models::{
    DashboardAlerts, DashboardData, InventoryPotential, RevenueTimeSeriesPoint, UpcomingEventAlert,
};
use chrono::{Datelike, Duration, Local, NaiveDate};
use rusqlite::{params, Connection};
use tauri::State;

// Attention section: fixed, transparent rules (see NEXT TASK spec - "simple,
// transparent rules", no new alert/notification engine, nothing
// user-configurable). "Upcoming" here means status='upcoming' AND an
// event_date within this many days from today (inclusive of today).
const UPCOMING_EVENT_WINDOW_DAYS: i64 = 14;
// Same cap/convention already used for Recent Events/Orders/Sales below.
const UPCOMING_EVENTS_CAP: i64 = 5;

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

/// Picks the revenue/profit chart's bucket width from the period's span, so
/// "Last 7 days" isn't 7 bars' worth of nothing else and "All time" for a
/// shop that's been running for years isn't thousands of daily bars. Falls
/// back to "day" if the bounds can't be parsed. That shouldn't happen in
/// practice (period_bounds() always emits valid dates); this is just a safe
/// default, never a panic.
fn time_series_granularity(period_from: &str, period_to: &str) -> &'static str {
    let span_days = match (
        NaiveDate::parse_from_str(period_from, "%Y-%m-%d"),
        NaiveDate::parse_from_str(period_to, "%Y-%m-%d"),
    ) {
        (Ok(from), Ok(to)) => (to - from).num_days(),
        _ => 0,
    };
    if span_days <= 31 {
        "day"
    } else if span_days <= 180 {
        "week"
    } else {
        "month"
    }
}

/// The SQL expression used to bucket `s.sale_date` for a given granularity.
/// Grouping by this key (rather than the display date directly) keeps week/
/// month bucketing correct even though `bucket_start` - the MIN(sale_date)
/// within that key - is what's actually shown/used by the frontend.
fn bucket_key_expr(granularity: &str) -> &'static str {
    match granularity {
        "week" => "strftime('%Y-W%W', s.sale_date)",
        "month" => "strftime('%Y-%m', s.sale_date)",
        _ => "s.sale_date",
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
    get_dashboard_impl(&conn, period.as_deref(), from, to, event_id, platform_id)
}

/// Split out from the `get_dashboard` command (same "impl function + thin
/// tauri::command wrapper" pattern already used by list_orders/list_tickets/
/// list_sale_groups) so it is directly unit-testable against a plain
/// `&Connection`, without needing a Tauri `State<AppState>`.
pub(crate) fn get_dashboard_impl(
    conn: &Connection,
    period: Option<&str>,
    from: Option<String>,
    to: Option<String>,
    event_id: Option<i64>,
    platform_id: Option<i64>,
) -> AppResult<DashboardData> {
    let (period_from, period_to) = period_bounds(period, from, to);

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

    // ---- Revenue/Profit over time (chart) --------------------------------
    // Same scope as `period_summary` right above (period_from/period_to,
    // primary_currency, event_id/platform_id, refund-excluded) - just
    // broken out by date bucket instead of collapsed into one total, so the
    // chart and the StatCards above it can never silently disagree. Bucket
    // width adapts to the period's span - see time_series_granularity().
    let granularity = time_series_granularity(&period_from, &period_to);
    let bucket_expr = bucket_key_expr(granularity);
    let mut ts_sql = format!(
        "SELECT {bucket_expr} as bucket_key, MIN(s.sale_date) as bucket_start,
            COALESCE(SUM(s.sale_price_cents),0) as revenue_cents,
            COALESCE(SUM(s.selling_fees_cents),0) as selling_fees_cents,
            COALESCE(SUM(t.purchase_cost_cents+t.purchase_fees_cents+t.other_costs_cents),0) as cogs_cents
         FROM sales s JOIN tickets t ON t.id = s.ticket_id
         WHERE s.sale_date BETWEEN ?1 AND ?2 AND s.currency = ?3 AND s.payment_status != 'refunded'"
    );
    let mut ts_params: Vec<Box<dyn rusqlite::ToSql>> = vec![
        Box::new(period_from.clone()),
        Box::new(period_to.clone()),
        Box::new(primary_currency.clone()),
    ];
    if let Some(eid) = event_id {
        ts_sql.push_str(&format!(" AND t.event_id = ?{}", ts_params.len() + 1));
        ts_params.push(Box::new(eid));
    }
    if let Some(pid) = platform_id {
        ts_sql.push_str(&format!(" AND s.platform_id = ?{}", ts_params.len() + 1));
        ts_params.push(Box::new(pid));
    }
    ts_sql.push_str(" GROUP BY bucket_key ORDER BY bucket_start ASC");
    let ts_refs: Vec<&dyn rusqlite::ToSql> = ts_params.iter().map(|p| p.as_ref()).collect();
    let mut ts_stmt = conn.prepare(&ts_sql)?;
    let revenue_time_series = ts_stmt
        .query_map(ts_refs.as_slice(), |r| {
            let revenue_cents: i64 = r.get("revenue_cents")?;
            let selling_fees_cents: i64 = r.get("selling_fees_cents")?;
            let cogs_cents: i64 = r.get("cogs_cents")?;
            Ok(RevenueTimeSeriesPoint {
                bucket_start: r.get("bucket_start")?,
                revenue_cents,
                selling_fees_cents,
                cogs_cents,
                profit_cents: finance::profit_cents(revenue_cents, cogs_cents, selling_fees_cents),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    drop(ts_stmt);

    // ---- Inventory Cost / Listing Value / Potential Profit --------------
    // Deliberately separate from `inventory`/`period_summary` above (both
    // realized-only FinanceSummary blocks) - this is about UNSOLD stock, so
    // it is never called "profit" alone, only "Potential Profit", and is
    // never blended into any realized figure. Not period-filtered - same as
    // the "current inventory snapshot" block above, this is a right-now
    // state, not activity within a date range.
    //
    // Scope is `available`+`listed` tickets (i.e. not yet sold, and not
    // cancelled - cancelled stock isn't sellable inventory any more).
    // Listing Value only counts tickets that actually have a
    // listing_price_cents set (SUM already ignores NULLs); tickets missing
    // one still count against Inventory Cost but contribute nothing to
    // Listing Value, and are surfaced separately below via
    // `missing_listing_price_count` so Potential Profit's understatement
    // for unpriced stock is never silent.
    let (inventory_cost_cents, listing_value_cents): (i64, i64) = conn.query_row(
        "SELECT
            COALESCE(SUM(purchase_cost_cents + purchase_fees_cents + other_costs_cents), 0),
            COALESCE(SUM(listing_price_cents), 0)
         FROM tickets
         WHERE status IN ('available','listed') AND currency = ?1",
        [&primary_currency],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    // Reuses `finance::profit_cents` (revenue - cost - fees) with fees=0,
    // rather than a new "subtract two numbers" helper.
    let potential_profit_cents = finance::profit_cents(listing_value_cents, inventory_cost_cents, 0);
    let inventory_potential = InventoryPotential {
        inventory_cost_cents,
        listing_value_cents,
        potential_profit_cents,
        // Same mixed-currency signal the rest of this dashboard already
        // uses - deliberately NOT a second, narrower "is the unsold subset
        // itself mixed" check, so this stays consistent with how every
        // other total on this page already handles currency mixing.
        currency: if mixed_currencies { None } else { Some(primary_currency.clone()) },
    };

    // ---- Attention / Alerts ----------------------------------------------
    // Simple, transparent counts - no scoring, no new alert engine, no
    // persisted state, not period-filtered (these are "right now" facts).
    let unpaid_orders_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM orders WHERE payment_status IN ('unpaid','partial')",
        [],
        |r| r.get(0),
    )?;
    let missing_listing_price_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tickets
         WHERE status IN ('available','listed') AND listing_price_cents IS NULL",
        [],
        |r| r.get(0),
    )?;

    let today = Local::now().date_naive().to_string();
    let window_end = (Local::now().date_naive() + Duration::days(UPCOMING_EVENT_WINDOW_DAYS)).to_string();
    let upcoming_events_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM (
            SELECT e.id
            FROM events e JOIN tickets t ON t.event_id = e.id
            WHERE e.status = 'upcoming' AND e.event_date IS NOT NULL
              AND e.event_date >= ?1 AND e.event_date <= ?2
              AND t.status IN ('available','listed')
            GROUP BY e.id
         )",
        params![today, window_end],
        |r| r.get(0),
    )?;
    let mut upcoming_stmt = conn.prepare(
        "SELECT e.id, e.name, e.event_date, COUNT(t.id) AS relevant_inventory
         FROM events e JOIN tickets t ON t.event_id = e.id
         WHERE e.status = 'upcoming' AND e.event_date IS NOT NULL
           AND e.event_date >= ?1 AND e.event_date <= ?2
           AND t.status IN ('available','listed')
         GROUP BY e.id
         ORDER BY e.event_date ASC, e.id ASC
         LIMIT ?3",
    )?;
    let upcoming_events = upcoming_stmt
        .query_map(params![today, window_end, UPCOMING_EVENTS_CAP], |r| {
            Ok(UpcomingEventAlert {
                id: r.get(0)?,
                name: r.get(1)?,
                event_date: r.get(2)?,
                relevant_inventory: r.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    drop(upcoming_stmt);

    let alerts = DashboardAlerts {
        unpaid_orders_count,
        missing_listing_price_count,
        upcoming_events_count,
        upcoming_events,
    };

    let recent_orders = orders_cmd::fetch_recent(conn, 5)?;
    let recent_sales = sales_cmd::fetch_recent(conn, 5)?;
    let recent_events = events_cmd::fetch_recent(conn, 5)?;

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
        inventory_potential,
        alerts,
        revenue_time_series,
        time_series_granularity: granularity.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_conn;

    fn seed_event(conn: &Connection, name: &str, status: &str, event_date: Option<&str>) -> i64 {
        conn.execute(
            "INSERT INTO events (name, status, event_date) VALUES (?1, ?2, ?3)",
            params![name, status, event_date],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    fn seed_order_only(conn: &Connection, code_suffix: &str, event_id: i64, payment_status: &str) -> i64 {
        conn.execute(
            "INSERT INTO orders (code, event_id, purchase_date, quantity, currency, payment_status)
             VALUES (?1, ?2, '2026-01-01', 1, 'EUR', ?3)",
            params![format!("ORD-{code_suffix}"), event_id, payment_status],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    /// Creates one order + one ticket, with full control over the ticket's
    /// status/currency/cost/listing price - the exact knobs these tests
    /// need. `code_suffix` must be unique per call (code columns are UNIQUE).
    #[allow(clippy::too_many_arguments)]
    fn seed_ticket(
        conn: &Connection,
        code_suffix: &str,
        event_id: i64,
        status: &str,
        currency: &str,
        purchase_cost_cents: i64,
        listing_price_cents: Option<i64>,
    ) -> i64 {
        let order_id = seed_order_only(conn, &format!("t{code_suffix}"), event_id, "paid");
        conn.execute(
            "INSERT INTO tickets (code, event_id, order_id, purchase_cost_cents, listing_price_cents, currency, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                format!("TKT-{code_suffix}"),
                event_id,
                order_id,
                purchase_cost_cents,
                listing_price_cents,
                currency,
                status,
            ],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    // ---- Inventory Cost / Listing Value / Potential Profit ----------------

    #[test]
    fn inventory_cost_and_listing_value_only_count_unsold_tickets() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event", "upcoming", None);
        seed_ticket(&conn, "1", event_id, "available", "EUR", 1000, Some(1500));
        seed_ticket(&conn, "2", event_id, "listed", "EUR", 2000, Some(2500));
        seed_ticket(&conn, "3", event_id, "sold", "EUR", 3000, Some(3500)); // must be excluded
        seed_ticket(&conn, "4", event_id, "cancelled", "EUR", 4000, Some(4500)); // must be excluded

        let data = get_dashboard_impl(&conn, None, None, None, None, None).unwrap();

        assert_eq!(data.inventory_potential.inventory_cost_cents, 1000 + 2000);
        assert_eq!(data.inventory_potential.listing_value_cents, 1500 + 2500);
        assert_eq!(data.inventory_potential.currency, Some("EUR".to_string()));
    }

    #[test]
    fn potential_profit_is_listing_value_minus_inventory_cost() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event", "upcoming", None);
        seed_ticket(&conn, "1", event_id, "available", "EUR", 1000, Some(1800));

        let data = get_dashboard_impl(&conn, None, None, None, None, None).unwrap();

        assert_eq!(data.inventory_potential.inventory_cost_cents, 1000);
        assert_eq!(data.inventory_potential.listing_value_cents, 1800);
        assert_eq!(data.inventory_potential.potential_profit_cents, 800);
    }

    #[test]
    fn potential_profit_can_be_negative_without_panicking() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event", "upcoming", None);
        // Listed for less than it cost - a real, valid (if unfortunate) state.
        seed_ticket(&conn, "1", event_id, "listed", "EUR", 5000, Some(3000));

        let data = get_dashboard_impl(&conn, None, None, None, None, None).unwrap();

        assert_eq!(data.inventory_potential.potential_profit_cents, -2000);
    }

    #[test]
    fn unpriced_ticket_counts_toward_cost_but_not_listing_value() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event", "upcoming", None);
        seed_ticket(&conn, "1", event_id, "available", "EUR", 1000, None);

        let data = get_dashboard_impl(&conn, None, None, None, None, None).unwrap();

        assert_eq!(data.inventory_potential.inventory_cost_cents, 1000);
        assert_eq!(data.inventory_potential.listing_value_cents, 0);
        assert_eq!(data.inventory_potential.potential_profit_cents, -1000);
        assert_eq!(data.alerts.missing_listing_price_count, 1);
    }

    #[test]
    fn mixed_currency_inventory_shows_none_currency_not_a_blended_number() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event", "upcoming", None);
        seed_ticket(&conn, "1", event_id, "available", "EUR", 1000, Some(1500));
        seed_ticket(&conn, "2", event_id, "available", "USD", 1000, Some(1500));

        let data = get_dashboard_impl(&conn, None, None, None, None, None).unwrap();

        assert_eq!(
            data.inventory_potential.currency, None,
            "two currencies among unsold tickets must never be blended into one number"
        );
        assert!(data.mixed_currencies);
    }

    #[test]
    fn empty_inventory_gives_zeroed_potential_not_an_error() {
        let conn = test_conn();
        // Nothing seeded at all - fresh database.
        let data = get_dashboard_impl(&conn, None, None, None, None, None).unwrap();

        assert_eq!(data.inventory_potential.inventory_cost_cents, 0);
        assert_eq!(data.inventory_potential.listing_value_cents, 0);
        assert_eq!(data.inventory_potential.potential_profit_cents, 0);
        assert_eq!(data.inventory_potential.currency, Some("EUR".to_string()));
        assert_eq!(data.alerts.unpaid_orders_count, 0);
        assert_eq!(data.alerts.missing_listing_price_count, 0);
        assert_eq!(data.alerts.upcoming_events_count, 0);
        assert!(data.alerts.upcoming_events.is_empty());
    }

    // ---- Attention: unpaid orders ------------------------------------------

    #[test]
    fn unpaid_orders_count_includes_unpaid_and_partial_but_not_paid() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event", "upcoming", None);
        seed_order_only(&conn, "1", event_id, "unpaid");
        seed_order_only(&conn, "2", event_id, "partial");
        seed_order_only(&conn, "3", event_id, "paid");

        let data = get_dashboard_impl(&conn, None, None, None, None, None).unwrap();

        assert_eq!(data.alerts.unpaid_orders_count, 2);
    }

    // ---- Attention: missing listing price ----------------------------------

    #[test]
    fn missing_listing_price_only_counts_available_and_listed() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event", "upcoming", None);
        seed_ticket(&conn, "1", event_id, "available", "EUR", 1000, None);
        seed_ticket(&conn, "2", event_id, "listed", "EUR", 1000, None);
        seed_ticket(&conn, "3", event_id, "sold", "EUR", 1000, None);
        seed_ticket(&conn, "4", event_id, "cancelled", "EUR", 1000, None);
        seed_ticket(&conn, "5", event_id, "available", "EUR", 1000, Some(1200)); // has a price - must not count

        let data = get_dashboard_impl(&conn, None, None, None, None, None).unwrap();

        assert_eq!(data.alerts.missing_listing_price_count, 2);
    }

    // ---- Attention: upcoming events ----------------------------------------

    #[test]
    fn upcoming_events_alert_applies_all_three_rules_together() {
        let conn = test_conn();
        let today = Local::now().date_naive();
        let soon = (today + Duration::days(5)).to_string();
        let far = (today + Duration::days(30)).to_string(); // outside the 14-day window

        // (a) upcoming, soon, has available inventory -> included
        let included = seed_event(&conn, "Included Event", "upcoming", Some(&soon));
        seed_ticket(&conn, "a", included, "available", "EUR", 1000, Some(1200));

        // (b) upcoming, but too far away -> excluded
        let too_far = seed_event(&conn, "Too Far Event", "upcoming", Some(&far));
        seed_ticket(&conn, "b", too_far, "available", "EUR", 1000, Some(1200));

        // (c) upcoming, soon, but everything already sold -> excluded
        let sold_out = seed_event(&conn, "Sold Out Event", "upcoming", Some(&soon));
        seed_ticket(&conn, "c", sold_out, "sold", "EUR", 1000, Some(1200));

        // (d) wrong status (completed) -> excluded even though the date is soon
        let completed = seed_event(&conn, "Completed Event", "completed", Some(&soon));
        seed_ticket(&conn, "d", completed, "available", "EUR", 1000, Some(1200));

        // (e) upcoming, no event_date at all -> excluded (can't judge "soon")
        let no_date = seed_event(&conn, "TBD Event", "upcoming", None);
        seed_ticket(&conn, "e", no_date, "available", "EUR", 1000, Some(1200));

        let data = get_dashboard_impl(&conn, None, None, None, None, None).unwrap();

        assert_eq!(data.alerts.upcoming_events_count, 1);
        assert_eq!(data.alerts.upcoming_events.len(), 1);
        assert_eq!(data.alerts.upcoming_events[0].id, included);
        assert_eq!(data.alerts.upcoming_events[0].relevant_inventory, 1);

        // Sanity: none of the deliberately-excluded events leaked in.
        let leaked_ids: Vec<i64> = data.alerts.upcoming_events.iter().map(|e| e.id).collect();
        assert!(!leaked_ids.contains(&too_far));
        assert!(!leaked_ids.contains(&sold_out));
        assert!(!leaked_ids.contains(&completed));
        assert!(!leaked_ids.contains(&no_date));
    }

    #[test]
    fn upcoming_events_list_is_capped_but_count_reflects_the_true_total() {
        let conn = test_conn();
        let today = Local::now().date_naive();
        let soon = (today + Duration::days(3)).to_string();

        for i in 0..7 {
            let event_id = seed_event(&conn, &format!("Event {i}"), "upcoming", Some(&soon));
            seed_ticket(&conn, &format!("cap{i}"), event_id, "available", "EUR", 1000, Some(1200));
        }

        let data = get_dashboard_impl(&conn, None, None, None, None, None).unwrap();

        assert_eq!(data.alerts.upcoming_events_count, 7, "the true total must not be capped");
        assert_eq!(
            data.alerts.upcoming_events.len(),
            UPCOMING_EVENTS_CAP as usize,
            "the returned list must be capped at UPCOMING_EVENTS_CAP"
        );
    }

    #[test]
    fn upcoming_events_are_ordered_soonest_first() {
        let conn = test_conn();
        let today = Local::now().date_naive();
        let later = (today + Duration::days(10)).to_string();
        let sooner = (today + Duration::days(2)).to_string();

        let later_event = seed_event(&conn, "Later Event", "upcoming", Some(&later));
        seed_ticket(&conn, "later", later_event, "available", "EUR", 1000, Some(1200));
        let sooner_event = seed_event(&conn, "Sooner Event", "upcoming", Some(&sooner));
        seed_ticket(&conn, "sooner", sooner_event, "available", "EUR", 1000, Some(1200));

        let data = get_dashboard_impl(&conn, None, None, None, None, None).unwrap();

        assert_eq!(data.alerts.upcoming_events.len(), 2);
        assert_eq!(data.alerts.upcoming_events[0].id, sooner_event, "the sooner event must come first");
        assert_eq!(data.alerts.upcoming_events[1].id, later_event);
    }

    // ---- Regression: new blocks must not disturb existing summaries -------

    #[test]
    fn new_fields_do_not_change_existing_realized_summaries() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event", "upcoming", None);
        // A sold, realized ticket - only affects `inventory`/`period`, never inventory_potential.
        seed_ticket(&conn, "1", event_id, "sold", "EUR", 1000, None);
        // An unsold, priced ticket - only affects inventory_potential, never realized revenue/profit.
        seed_ticket(&conn, "2", event_id, "available", "EUR", 500, Some(900));

        let data = get_dashboard_impl(&conn, None, None, None, None, None).unwrap();

        assert_eq!(data.inventory.revenue_cents, 0, "no sale exists, so realized revenue must stay 0");
        assert_eq!(data.inventory_potential.inventory_cost_cents, 500, "only the unsold ticket counts here");
    }

    // ---- 1.6.0: revenue/profit chart --------------------------------------

    /// Sells an already-seeded (available/listed) ticket on a specific date,
    /// via the real `create_sale_impl` (not a raw INSERT) so every column
    /// (currency copied from the ticket, code assignment, etc.) is exactly
    /// what the app itself would produce.
    fn seed_sale(conn: &mut Connection, ticket_id: i64, sale_date: &str, price_cents: i64) -> i64 {
        crate::commands::sales::create_sale_impl(
            conn,
            &crate::models::SaleInput {
                ticket_id,
                platform_id: None,
                sale_date: sale_date.to_string(),
                sale_price_cents: price_cents,
                selling_fees_cents: 0,
                payment_status: Some("paid".to_string()),
                buyer_reference: None,
                notes: None,
            },
        )
        .unwrap()
    }

    #[test]
    fn time_series_granularity_switches_on_span_thresholds() {
        assert_eq!(time_series_granularity("2026-01-01", "2026-01-01"), "day"); // 0 days
        assert_eq!(time_series_granularity("2026-01-01", "2026-01-31"), "day"); // 30 days, <= 31
        assert_eq!(time_series_granularity("2026-01-01", "2026-02-15"), "week"); // 45 days, > 31
        assert_eq!(time_series_granularity("2026-01-01", "2026-06-30"), "week"); // 180 days, <= 180
        assert_eq!(time_series_granularity("2026-01-01", "2026-07-01"), "month"); // 181 days, > 180
        assert_eq!(time_series_granularity("0001-01-01", "9999-12-31"), "month", "All time -> coarsest bucket");
    }

    #[test]
    fn revenue_time_series_buckets_by_day_and_sums_back_to_the_period_total() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn, "Chart Event", "upcoming", None);
        let t1 = seed_ticket(&conn, "1", event_id, "available", "EUR", 1000, None);
        let t2 = seed_ticket(&conn, "2", event_id, "available", "EUR", 1000, None);
        let t3 = seed_ticket(&conn, "3", event_id, "available", "EUR", 1000, None);
        seed_sale(&mut conn, t1, "2026-03-01", 2000);
        seed_sale(&mut conn, t2, "2026-03-01", 3000); // same day as t1 -> same bucket
        seed_sale(&mut conn, t3, "2026-03-03", 1500); // different day -> its own bucket

        let data = get_dashboard_impl(
            &conn,
            Some("custom"),
            Some("2026-03-01".to_string()),
            Some("2026-03-07".to_string()),
            None,
            None,
        )
        .unwrap();

        assert_eq!(data.time_series_granularity, "day");
        assert_eq!(data.revenue_time_series.len(), 2, "two distinct sale dates -> two buckets");
        assert_eq!(data.revenue_time_series[0].bucket_start, "2026-03-01");
        assert_eq!(data.revenue_time_series[0].revenue_cents, 5000, "2000 + 3000 on the same day");
        assert_eq!(data.revenue_time_series[1].bucket_start, "2026-03-03");
        assert_eq!(data.revenue_time_series[1].revenue_cents, 1500);
        // Cross-check against the existing, already-trusted period total -
        // the chart must never be able to silently drift from the StatCards
        // showing the same period.
        let chart_revenue: i64 = data.revenue_time_series.iter().map(|p| p.revenue_cents).sum();
        let chart_profit: i64 = data.revenue_time_series.iter().map(|p| p.profit_cents).sum();
        assert_eq!(chart_revenue, data.period.revenue_cents);
        assert_eq!(chart_profit, data.period.profit_cents);
    }

    #[test]
    fn revenue_time_series_excludes_refunds_same_as_the_period_total() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn, "Chart Refund Event", "upcoming", None);
        let active = seed_ticket(&conn, "1", event_id, "available", "EUR", 1000, None);
        let refunded = seed_ticket(&conn, "2", event_id, "available", "EUR", 1000, None);
        seed_sale(&mut conn, active, "2026-03-01", 2000);
        let refunded_sale_id = seed_sale(&mut conn, refunded, "2026-03-02", 9000);
        crate::commands::sales::refund_sale_impl(&mut conn, refunded_sale_id, Some("test refund")).unwrap();

        let data = get_dashboard_impl(
            &conn,
            Some("custom"),
            Some("2026-03-01".to_string()),
            Some("2026-03-07".to_string()),
            None,
            None,
        )
        .unwrap();

        assert_eq!(data.revenue_time_series.len(), 1, "refunded day contributes no bucket at all");
        assert_eq!(data.revenue_time_series[0].revenue_cents, 2000);
    }

    #[test]
    fn revenue_time_series_groups_the_same_iso_week_together_at_week_granularity() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn, "Chart Week Event", "upcoming", None);
        let t1 = seed_ticket(&conn, "1", event_id, "available", "EUR", 1000, None);
        let t2 = seed_ticket(&conn, "2", event_id, "available", "EUR", 1000, None);
        // Both Tuesday/Wednesday of the same Mon-Sun week (2026-02-02 is a
        // Monday), well inside a 90-day period so granularity is "week".
        seed_sale(&mut conn, t1, "2026-02-03", 2000);
        seed_sale(&mut conn, t2, "2026-02-04", 1000);

        let data = get_dashboard_impl(
            &conn,
            Some("custom"),
            Some("2026-01-01".to_string()),
            Some("2026-04-01".to_string()),
            None,
            None,
        )
        .unwrap();

        assert_eq!(data.time_series_granularity, "week");
        assert_eq!(data.revenue_time_series.len(), 1, "same ISO week -> one bucket");
        assert_eq!(data.revenue_time_series[0].revenue_cents, 3000);
        assert_eq!(data.revenue_time_series[0].bucket_start, "2026-02-03", "earliest date in the bucket");
    }

    #[test]
    fn revenue_time_series_groups_the_same_month_together_at_month_granularity() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn, "Chart Month Event", "upcoming", None);
        let t1 = seed_ticket(&conn, "1", event_id, "available", "EUR", 1000, None);
        let t2 = seed_ticket(&conn, "2", event_id, "available", "EUR", 1000, None);
        seed_sale(&mut conn, t1, "2026-02-03", 2000);
        seed_sale(&mut conn, t2, "2026-02-20", 1000); // same month, different day

        let data = get_dashboard_impl(
            &conn,
            Some("custom"),
            Some("2026-01-01".to_string()),
            Some("2026-12-31".to_string()),
            None,
            None,
        )
        .unwrap();

        assert_eq!(data.time_series_granularity, "month");
        assert_eq!(data.revenue_time_series.len(), 1, "same month -> one bucket");
        assert_eq!(data.revenue_time_series[0].revenue_cents, 3000);
    }

    #[test]
    fn revenue_time_series_respects_event_filter_same_as_period_total() {
        let mut conn = test_conn();
        let event_a = seed_event(&conn, "Event A", "upcoming", None);
        let event_b = seed_event(&conn, "Event B", "upcoming", None);
        let ta = seed_ticket(&conn, "1", event_a, "available", "EUR", 1000, None);
        let tb = seed_ticket(&conn, "2", event_b, "available", "EUR", 1000, None);
        seed_sale(&mut conn, ta, "2026-03-01", 2000);
        seed_sale(&mut conn, tb, "2026-03-01", 9000);

        let data = get_dashboard_impl(
            &conn,
            Some("custom"),
            Some("2026-03-01".to_string()),
            Some("2026-03-07".to_string()),
            Some(event_a),
            None,
        )
        .unwrap();

        assert_eq!(data.revenue_time_series.len(), 1);
        assert_eq!(data.revenue_time_series[0].revenue_cents, 2000, "event_b's sale must not leak in");
        assert_eq!(data.revenue_time_series[0].revenue_cents, data.period.revenue_cents);
    }
}

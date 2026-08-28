use crate::commands::{events as events_cmd, orders as orders_cmd, sales as sales_cmd};
use crate::db::AppState;
use crate::error::AppResult;
use crate::finance;
use crate::models::{
    CashflowSummary, CurrencyOrderCount, DashboardAlerts, DashboardData, InventoryPotential,
    PlatformSales, RevenueTimeSeriesPoint, UpcomingEventAlert,
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

/// Subtracts `months` whole calendar months from `date`, clamping to the
/// last valid day of the resulting month when `date`'s day-of-month doesn't
/// exist there (e.g. Mar 31 minus 1 month -> Feb 28, or Feb 29 in a leap
/// year - never an error, never silently skipping into the next month).
///
/// Written from first principles (plain integer year/month arithmetic +
/// `NaiveDate::from_ymd_opt`) instead of reaching for chrono's `Months`/
/// `checked_sub_months`, so every step is spelled out and covered by the
/// tests right below rather than trusted from the dependency - this file
/// can't be compiled/run in the sandbox that wrote it (see 1.7.5 report),
/// so the implementation leans on primitives already proven elsewhere in
/// this file (`NaiveDate::from_ymd_opt`, `Duration::days`) rather than an
/// API surface nobody here could double-check against the real crate.
fn months_ago(date: NaiveDate, months: u32) -> NaiveDate {
    let total_months = date.year() * 12 + date.month() as i32 - 1 - months as i32;
    let year = total_months.div_euclid(12);
    let month = (total_months.rem_euclid(12) + 1) as u32;
    // Last valid day of (year, month) = one day before the 1st of the
    // following month. Always succeeds for any real (year, month).
    let (next_year, next_month) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
    let first_of_next = NaiveDate::from_ymd_opt(next_year, next_month, 1).unwrap();
    let last_day_of_month = (first_of_next - Duration::days(1)).day();
    let day = date.day().min(last_day_of_month);
    NaiveDate::from_ymd_opt(year, month, day).unwrap()
}

/// `today` is passed in (rather than calling `Local::now()` here) purely so
/// this function stays a pure, deterministically unit-testable calculation -
/// see the "impl function" convention used throughout this codebase. The
/// only real caller (`get_dashboard_impl`) passes the real wall-clock date.
///
/// 1.7.5: replaced the old 7d/30d/month presets with a standard
/// Today/1W/1M/3M/YTD/1Y/5Y/All range-picker set (marko's reference
/// screenshot) - relative presets now subtract whole calendar months via
/// `months_ago` (e.g. "1y" is "the same date last year", not "365 days
/// ago"), matching how this kind of range picker conventionally works.
/// "custom" and "all" are unchanged.
fn period_bounds(period: Option<&str>, from: Option<String>, to: Option<String>, today: NaiveDate) -> (String, String) {
    match period {
        Some("today") | None => (today.to_string(), today.to_string()),
        Some("1w") => ((today - Duration::days(6)).to_string(), today.to_string()),
        Some("1m") => (months_ago(today, 1).to_string(), today.to_string()),
        Some("3m") => (months_ago(today, 3).to_string(), today.to_string()),
        Some("ytd") => {
            let start = NaiveDate::from_ymd_opt(today.year(), 1, 1).unwrap();
            (start.to_string(), today.to_string())
        }
        Some("1y") => (months_ago(today, 12).to_string(), today.to_string()),
        Some("5y") => (months_ago(today, 60).to_string(), today.to_string()),
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

/// The same bucket key `bucket_key_expr`'s SQL computes for one given date -
/// used by `fill_time_series_gaps` both to build the "expected" list of
/// every bucket that should exist, and, on the real rows the main query
/// returns, to know which expected slot each one fills. Day and month are
/// answered directly (a month's key is just its own year/month - no need to
/// ask SQLite); week goes through one tiny `strftime` query so it is
/// byte-identical to SQLite's own Monday-based `%W` week numbering instead
/// of a hand-rolled reimplementation that could quietly drift out of sync
/// with it.
fn bucket_key_of_date(conn: &Connection, date: NaiveDate, granularity: &str) -> AppResult<String> {
    match granularity {
        "month" => Ok(format!("{:04}-{:02}", date.year(), date.month())),
        "week" => Ok(conn.query_row("SELECT strftime('%Y-W%W', ?1)", [date.to_string()], |r| r.get(0))?),
        _ => Ok(date.to_string()),
    }
}

/// Every bucket key that SHOULD exist between `from` and `to` (inclusive) at
/// `granularity`, each paired with a representative date to use as that
/// bucket's `bucket_start` if it turns out to have no real sales at all - see
/// `fill_time_series_gaps`'s own doc comment for why this matters. Walks one
/// calendar day at a time to discover key transitions, which is always a
/// small, bounded amount of work in practice: week/month granularity only
/// kick in past 31/180 days (`time_series_granularity`), and
/// `fill_time_series_gaps` itself clamps away `period_bounds`'s "All time"/
/// empty-Custom sentinel dates before this is ever called, so `to - from` is
/// always a real, human-scale business-history span, never literally "year 1
/// to year 9999".
fn expected_bucket_keys(
    conn: &Connection,
    from: NaiveDate,
    to: NaiveDate,
    granularity: &str,
) -> AppResult<Vec<(String, NaiveDate)>> {
    let mut keys = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut d = from;
    while d <= to {
        let key = bucket_key_of_date(conn, d, granularity)?;
        if seen.insert(key.clone()) {
            keys.push((key, d));
        }
        d = d + Duration::days(1);
    }
    Ok(keys)
}

/// Backfills every calendar-gap bucket the main `revenue_time_series` query
/// can never produce on its own with an explicit zero-value point, so the
/// chart never has to connect two real, far-apart sale days with one
/// misleadingly smooth sloped line across days that actually had no sales at
/// all.
///
/// 2.0.68 (marko's report - "niečo sa s [grafmi] urobilo"): `MetricChart.tsx`
/// places points EVENLY SPACED BY INDEX along the x-axis (`xAt = i * stepX`),
/// not by real calendar date - exactly right when every bucket in the range
/// is present, but the query above only ever returns a row for a bucket that
/// had at least one real sale (`GROUP BY bucket_key`), so a week with sales
/// on Monday and Friday only, say, comes back as exactly 2 points. The chart
/// then draws them right next to each other with one smooth connecting
/// curve, which reads as a continuous rise-or-decline across the WHOLE week
/// even though Tue-Thu had zero sales and should be a flat line at 0. This
/// backfills every missing bucket in range with an explicit zero point so
/// the x-axis spacing (and the line drawn across it) reflects real calendar
/// time again.
///
/// Pads the FULL requested `[period_from, period_to]` - not just the span
/// between the first and last real sale - so e.g. a quiet last few days of
/// "1 Wk" still show as a real flat 0 tail rather than the chart silently
/// ending early. The one exception is `period_bounds`'s own sentinel dates
/// ("0001-01-01" for "All time"/an empty Custom `from`, "9999-12-31" for an
/// empty Custom `to`) - padding out from year 1 or to year 9999 would be
/// both useless and slow, so a sentinel bound is clamped to the earliest/
/// latest REAL bucket already returned instead (`points` is already sorted
/// ascending by the SQL's own `ORDER BY`, so its first/last entries are that
/// real min/max).
///
/// Falls back to returning `points` completely unpadded, rather than ever
/// erroring the whole dashboard, if a date somehow fails to parse - every
/// date here is either `period_bounds`'s own output or a real `sale_date`
/// already trusted elsewhere in this file, so this should never actually
/// trigger; it exists purely so one malformed row can't take the rest of the
/// dashboard down with it (same defensive spirit as
/// `time_series_granularity`'s own parse-failure fallback above).
fn fill_time_series_gaps(
    conn: &Connection,
    points: Vec<RevenueTimeSeriesPoint>,
    period_from: &str,
    period_to: &str,
    granularity: &str,
) -> AppResult<Vec<RevenueTimeSeriesPoint>> {
    if points.is_empty() {
        // Nothing to anchor a range on - MetricChart.tsx already shows its
        // own "No sales in this period yet" empty state for this case.
        return Ok(points);
    }
    if points
        .iter()
        .any(|p| NaiveDate::parse_from_str(&p.bucket_start, "%Y-%m-%d").is_err())
    {
        return Ok(points);
    }

    let effective_from = if period_from == "0001-01-01" {
        points[0].bucket_start.clone()
    } else {
        period_from.to_string()
    };
    let effective_to = if period_to == "9999-12-31" {
        points[points.len() - 1].bucket_start.clone()
    } else {
        period_to.to_string()
    };
    let (from, to) = match (
        NaiveDate::parse_from_str(&effective_from, "%Y-%m-%d"),
        NaiveDate::parse_from_str(&effective_to, "%Y-%m-%d"),
    ) {
        (Ok(from), Ok(to)) if from <= to => (from, to),
        _ => return Ok(points),
    };

    let mut by_key: std::collections::HashMap<String, RevenueTimeSeriesPoint> = std::collections::HashMap::new();
    for p in points {
        // Already validated as parseable just above.
        let bucket_date = NaiveDate::parse_from_str(&p.bucket_start, "%Y-%m-%d").unwrap();
        let key = bucket_key_of_date(conn, bucket_date, granularity)?;
        by_key.insert(key, p);
    }

    let expected = expected_bucket_keys(conn, from, to, granularity)?;
    let mut filled = Vec::with_capacity(expected.len());
    for (key, representative_date) in expected {
        if let Some(p) = by_key.remove(&key) {
            filled.push(p);
        } else {
            filled.push(RevenueTimeSeriesPoint {
                bucket_start: representative_date.to_string(),
                sold_tickets: 0,
                revenue_cents: 0,
                selling_fees_cents: 0,
                cogs_cents: 0,
                profit_cents: 0,
            });
        }
    }
    if !by_key.is_empty() {
        // Defensive: every real point's date is expected to fall inside
        // [from, to] by construction (see this function's own doc comment
        // above), so this should never actually trigger - append rather
        // than silently drop, just in case a future edge case ever disagrees.
        let mut leftovers: Vec<RevenueTimeSeriesPoint> = by_key.into_values().collect();
        leftovers.sort_by(|a, b| a.bucket_start.cmp(&b.bucket_start));
        filled.extend(leftovers);
        filled.sort_by(|a, b| a.bucket_start.cmp(&b.bucket_start));
    }
    Ok(filled)
}

/// The equal-length window immediately preceding `period_from..period_to`
/// (2.0.47) - used for the Dashboard KPI cards' "vs previous period" trend
/// (DIR-001). Deliberately the generic "immediately preceding period of the
/// same length" comparison - the same default GA4/Stripe-style dashboards
/// fall back to when not explicitly toggled to a calendar-aware "same period
/// last year" - not a per-period-type special case (which YTD/1Y/5Y would
/// each need their own rule for). This is simple, consistent, and easy to
/// explain, with one honestly-documented limitation: for a long or irregular
/// range (YTD, 5Y) the "previous" window is still a real, equal-length prior
/// window and still a meaningful trend signal, just not literally "the same
/// range last year".
///
/// Returns None when there is no sensible previous period to compare against
/// - "All time" (there's no "before all time") or a Custom range with no
/// explicit start (same `0001-01-01`/`9999-12-31` sentinels `period_bounds`
/// itself uses for those two cases).
fn previous_period_bounds(period_from: &str, period_to: &str) -> Option<(String, String)> {
    if period_from == "0001-01-01" || period_to == "9999-12-31" {
        return None;
    }
    let from = NaiveDate::parse_from_str(period_from, "%Y-%m-%d").ok()?;
    let to = NaiveDate::parse_from_str(period_to, "%Y-%m-%d").ok()?;
    let span_days = (to - from).num_days() + 1; // inclusive on both ends
    let prev_to = from - Duration::days(1);
    let prev_from = prev_to - Duration::days(span_days - 1);
    Some((prev_from.to_string(), prev_to.to_string()))
}

/// Runs the "purchase activity in [from,to]" + "sales activity in [from,to]"
/// pair of queries `period_summary` has always used, and folds the results
/// into one `FinanceSummary` via `finance::compute_summary` - byte-identical
/// shape/scope to what used to be inlined directly in `get_dashboard_impl`.
/// Pulled out into its own function in 2.0.47 purely so the new "previous
/// period" comparison (`previous_period_bounds` above) can reuse it verbatim
/// instead of duplicating ~40 lines of SQL - the current-period call site
/// keeps computing exactly the same values it always did, just via this
/// function now instead of inline.
///
/// 2.0.68 (marko's report): `total_cost_cents` used to be the cost of
/// tickets whose ORDER was purchased in [from,to] - a different population of
/// tickets than the ones Revenue/Profit/Margin/ROI below are scoped to
/// (tickets actually SOLD in [from,to]). The two numbers could legitimately
/// never reconcile (e.g. Revenue minus the old "Purchase cost" not equal to
/// the Profit shown right next to it), which read as a calculation bug even
/// though each number was independently correct for what it measured.
/// Fixed by making `total_cost_cents` equal `cogs_cents` here - the SAME
/// sold-tickets-in-period cost already computed below for ROI - so the
/// Dashboard's "Purchase cost" StatCard is now guaranteed to satisfy
/// Revenue - Purchase cost - fees = Profit, exactly like a normal P&L. This
/// only changes what "in scope" means for THIS caller of the shared
/// `finance::compute_summary` (see finance.rs's own doc comment for the
/// general, still-unchanged contract) - Order/Sale/Event screens that call
/// `compute_summary` for their own all-time inventory snapshots are
/// untouched. `period_purchased` (tickets newly bought in [from,to], by order
/// purchase_date) is kept exactly as before - it only ever feeds the
/// "Tickets sold" StatCard's separate "N purchased in period" sub-line,
/// which was never part of marko's report and is a genuinely different,
/// non-misleading fact ("how much new stock did I buy" vs "what did the
/// stock I sold cost").
fn period_activity_summary(
    conn: &Connection,
    from: &str,
    to: &str,
    currency: &str,
    event_id: Option<i64>,
    platform_id: Option<i64>,
) -> AppResult<finance::FinanceSummary> {
    let mut purchase_sql = String::from(
        "SELECT COUNT(t.id)
         FROM tickets t JOIN orders o ON o.id = t.order_id
         WHERE o.purchase_date BETWEEN ?1 AND ?2 AND t.currency = ?3",
    );
    let mut p_params: Vec<Box<dyn rusqlite::ToSql>> = vec![
        Box::new(from.to_string()),
        Box::new(to.to_string()),
        Box::new(currency.to_string()),
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
    let period_purchased: i64 = conn.query_row(&purchase_sql, p_refs.as_slice(), |r| r.get(0))?;

    // Refunded sales excluded here too, and only the given currency counts.
    let mut sales_sql = String::from(
        "SELECT COUNT(*), COALESCE(SUM(s.sale_price_cents),0), COALESCE(SUM(s.selling_fees_cents),0),
            COALESCE(SUM(t.purchase_cost_cents+t.purchase_fees_cents+t.other_costs_cents),0)
         FROM sales s JOIN tickets t ON t.id = s.ticket_id
         WHERE s.sale_date BETWEEN ?1 AND ?2 AND s.currency = ?3 AND s.payment_status != 'refunded'",
    );
    let mut s_params: Vec<Box<dyn rusqlite::ToSql>> = vec![
        Box::new(from.to_string()),
        Box::new(to.to_string()),
        Box::new(currency.to_string()),
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

    Ok(finance::compute_summary(
        period_purchased,
        0,
        0,
        period_sold,
        0,
        period_cogs,
        period_cogs,
        period_revenue,
        period_fees,
        Some(currency.to_string()),
    ))
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
    let today = Local::now().date_naive();
    let (period_from, period_to) = period_bounds(period, from, to, today);

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

    // 2.0.51: every non-EUR currency actually present on an ORDER (not
    // ticket - orders is what the new "Convert to EUR" action on the
    // Dashboard banner and Order Detail actually converts), with how many
    // orders hold it, so the banner can offer one button per currency plus
    // an "all" button when there's more than one. Deliberately its own
    // query rather than reusing `currencies` above - that one is
    // ticket-scoped (drives primary_currency/mixed_currencies), this one is
    // order-scoped (drives what's actually convertible). The two normally
    // agree (ticket currency == its order's currency by construction - see
    // `insert_order_with_tickets`), but marko's pre-existing Edit Order
    // currency-relabel-only path means they're not GUARANTEED to, so this
    // stays independent rather than assumed derivable from `currencies`.
    let non_eur_order_currencies: Vec<CurrencyOrderCount> = {
        let mut stmt = conn.prepare(
            "SELECT currency, COUNT(*) FROM orders WHERE currency != 'EUR' GROUP BY currency ORDER BY currency",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(CurrencyOrderCount {
                currency: r.get(0)?,
                order_count: r.get(1)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()?
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

    // ---- period-filtered purchase + sales activity (by purchase_date/
    // sale_date respectively), and its "vs previous period" comparison
    // (2.0.47, DIR-001) - both computed via the shared
    // `period_activity_summary` helper right above, so `period_summary`
    // keeps computing exactly the same numbers it always did.
    let period_summary = period_activity_summary(
        conn,
        &period_from,
        &period_to,
        &primary_currency,
        event_id,
        platform_id,
    )?;
    let previous_period = match previous_period_bounds(&period_from, &period_to) {
        Some((prev_from, prev_to)) => Some(period_activity_summary(
            conn,
            &prev_from,
            &prev_to,
            &primary_currency,
            event_id,
            platform_id,
        )?),
        None => None,
    };

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
            COUNT(*) as sold_tickets,
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
                sold_tickets: r.get("sold_tickets")?,
                revenue_cents,
                selling_fees_cents,
                cogs_cents,
                profit_cents: finance::profit_cents(revenue_cents, cogs_cents, selling_fees_cents),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    drop(ts_stmt);
    // 2.0.68: backfill zero-sale calendar gaps - see fill_time_series_gaps's
    // own doc comment for exactly why the chart needs this.
    let revenue_time_series = fill_time_series_gaps(conn, revenue_time_series, &period_from, &period_to, granularity)?;

    // ---- Sales by platform (2.0.47, DIR-001 signature idea #02) ---------
    // Same scope as `period_summary`/`revenue_time_series` right above
    // (period_from/period_to, primary_currency, event_id/platform_id,
    // refund-excluded) - just grouped by platform instead of collapsed into
    // one total or broken out by date. LEFT JOIN so a sale with no platform
    // set still gets its own row (platform_id/platform_name both None -
    // "No platform" on the frontend) rather than being silently dropped.
    // Ordered by revenue so the platform actually earning the most sorts
    // first - the whole point of this widget (see PlatformSales doc
    // comment).
    let mut plat_sql = String::from(
        "SELECT s.platform_id, p.name as platform_name, COUNT(*) as sold_tickets,
            COALESCE(SUM(s.sale_price_cents),0) as revenue_cents,
            COALESCE(SUM(s.selling_fees_cents),0) as selling_fees_cents,
            COALESCE(SUM(t.purchase_cost_cents+t.purchase_fees_cents+t.other_costs_cents),0) as cogs_cents
         FROM sales s
         JOIN tickets t ON t.id = s.ticket_id
         LEFT JOIN platforms p ON p.id = s.platform_id
         WHERE s.sale_date BETWEEN ?1 AND ?2 AND s.currency = ?3 AND s.payment_status != 'refunded'",
    );
    let mut plat_params: Vec<Box<dyn rusqlite::ToSql>> = vec![
        Box::new(period_from.clone()),
        Box::new(period_to.clone()),
        Box::new(primary_currency.clone()),
    ];
    if let Some(eid) = event_id {
        plat_sql.push_str(&format!(" AND t.event_id = ?{}", plat_params.len() + 1));
        plat_params.push(Box::new(eid));
    }
    if let Some(pid) = platform_id {
        plat_sql.push_str(&format!(" AND s.platform_id = ?{}", plat_params.len() + 1));
        plat_params.push(Box::new(pid));
    }
    plat_sql.push_str(" GROUP BY s.platform_id ORDER BY revenue_cents DESC");
    let plat_refs: Vec<&dyn rusqlite::ToSql> = plat_params.iter().map(|p| p.as_ref()).collect();
    let mut plat_stmt = conn.prepare(&plat_sql)?;
    let sales_by_platform = plat_stmt
        .query_map(plat_refs.as_slice(), |r| {
            let revenue_cents: i64 = r.get("revenue_cents")?;
            let selling_fees_cents: i64 = r.get("selling_fees_cents")?;
            let cogs_cents: i64 = r.get("cogs_cents")?;
            Ok(PlatformSales {
                platform_id: r.get("platform_id")?,
                platform_name: r.get("platform_name")?,
                sold_tickets: r.get("sold_tickets")?,
                revenue_cents,
                profit_cents: finance::profit_cents(revenue_cents, cogs_cents, selling_fees_cents),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    drop(plat_stmt);

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
    // 2.0.48: marko's report - the Activity tab's "Missing listing price"
    // card reads confusingly ticket-by-ticket (showed "27") when he thinks
    // in orders - one order can hold many tickets, so one half-priced order
    // shouldn't look like a pile of 27 unrelated problems. Deliberately a
    // SEPARATE field from `missing_listing_price_count` above rather than
    // changing what that one counts: that field stays ticket-scoped because
    // the Overview "Potential Profit" sentence (Dashboard.tsx, and the
    // identical wording on Event Detail) is genuinely about how many
    // individual tickets are dragging the estimate down, which IS a
    // per-ticket fact and must not change. This new field is order-scoped,
    // for the Activity alert card specifically - counts an order once no
    // matter how many of its still-unsold tickets are missing a price.
    let missing_listing_price_orders_count: i64 = conn.query_row(
        "SELECT COUNT(DISTINCT order_id) FROM tickets
         WHERE status IN ('available','listed') AND listing_price_cents IS NULL",
        [],
        |r| r.get(0),
    )?;
    // 1.8.3 (section 13, Payments visibility): sales side of "money not yet
    // settled" - the counterpart to unpaid_orders_count above. Scoped to
    // primary_currency like every other money total on this dashboard (see
    // pending_sales_currency doc comment on DashboardAlerts for why that's
    // safe even though this query itself doesn't touch tickets.currency).
    //
    // 2.0.48: COUNT used to be a plain COUNT(*) against the `sales` table,
    // which stores one row PER TICKET - a multi-ticket "New sale" batch
    // shares one batch_id across several rows (see sales.rs's GROUP_KEY_EXPR
    // doc comment), so a single 4-ticket pending sale used to count as 4.
    // That's exactly marko's report: "shows 12, I only made 3 sales". Now
    // counts distinct sale GROUPS with the same GROUP_KEY_EXPR the Sales
    // screen itself already groups by. The SUM stays a plain per-row sum on
    // purpose - the total money outstanding is correct regardless of how
    // many tickets any one sale bundled together.
    //
    // Second pair of eyes caught one honest caveat worth stating rather than
    // overclaiming: this query is scoped to `primary_currency`, same as
    // every other money total on this dashboard (pre-existing behavior,
    // unchanged by this fix) - but `create_sales_batch_impl` never enforces
    // one currency across a batch's lines, so a (very unusual) batch mixing
    // two currencies could have some of its lines counted here and others
    // not, while the Sales screen's own grouped view has no such currency
    // filter and would still show it as one row. Not something marko
    // reported and not fixed here - flagging it precisely so nobody
    // mistakes this dashboard number for a byte-for-byte match of "rows on
    // the Sales screen" in that specific edge case.
    let (pending_sales_count, pending_sales_amount_cents): (i64, i64) = conn.query_row(
        &format!(
            "SELECT COUNT(DISTINCT {key}), COALESCE(SUM(s.sale_price_cents),0)
             FROM sales s WHERE s.payment_status = 'pending' AND s.currency = ?1",
            key = sales_cmd::GROUP_KEY_EXPR
        ),
        [&primary_currency],
        |r| Ok((r.get(0)?, r.get(1)?)),
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
        missing_listing_price_orders_count,
        upcoming_events_count,
        upcoming_events,
        pending_sales_count,
        pending_sales_amount_cents,
        pending_sales_currency: if mixed_currencies { None } else { Some(primary_currency.clone()) },
    };

    // ---- Cashflow (1.9.0) ---------------------------------------------------
    // Money actually collected from buyers vs. still owed to be collected -
    // trusts sales.payment_status='paid'/'pending' as a binary yes/no (no
    // partial-payment tracking - a sale is either fully paid or not).
    // Revenue/profit still reuse the already-computed `inventory`
    // FinanceSummary (all-time, realized, refund-excluded). Not
    // period-filtered, same "right now" rule as pending_sales_amount_cents.
    // Refunded sales are excluded by construction - payment_status is never
    // both 'refunded' and 'paid'/'pending' on the same row.
    let paid_cents: i64 = conn.query_row(
        "SELECT COALESCE(SUM(sale_price_cents), 0) FROM sales WHERE payment_status = 'paid' AND currency = ?1",
        [&primary_currency],
        |r| r.get(0),
    )?;
    let outstanding_cents: i64 = conn.query_row(
        "SELECT COALESCE(SUM(sale_price_cents), 0) FROM sales WHERE payment_status = 'pending' AND currency = ?1",
        [&primary_currency],
        |r| r.get(0),
    )?;
    let cashflow = CashflowSummary {
        revenue_cents: inventory.revenue_cents,
        profit_cents: inventory.profit_cents,
        paid_cents,
        outstanding_cents,
        currency: if mixed_currencies { None } else { Some(primary_currency.clone()) },
    };

    // 2.0.70: fetches 15 now (was 5) so the Activity tab's "Show more" button
    // (Dashboard.tsx) has real rows to reveal beyond the first 5 shown by
    // default - marko's own request ("tuto cast mozes urobit dlhsiu, alebo
    // daj tlacitko... a ukaze sa toho viac"). Still a fixed, cheap LIMIT on a
    // local SQLite query, not real pagination - fine for what this section
    // is for (a recent-activity glance, not a full list - Orders/Sales/
    // Events already exist as complete, filterable pages for that).
    let recent_orders = orders_cmd::fetch_recent(conn, 15)?;
    let recent_sales = sales_cmd::fetch_recent_groups(conn, 15)?;
    let recent_events = events_cmd::fetch_recent(conn, 15)?;

    Ok(DashboardData {
        inventory,
        period: period_summary,
        previous_period,
        period_from,
        period_to,
        recent_orders,
        recent_sales,
        recent_events,
        primary_currency,
        mixed_currencies,
        non_eur_order_currencies,
        inventory_potential,
        alerts,
        cashflow,
        revenue_time_series,
        time_series_granularity: granularity.to_string(),
        sales_by_platform,
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

    /// Same as `seed_ticket` but attaches to an EXISTING order instead of
    /// creating a brand new one each call - needed to build the "several
    /// unpriced tickets on one order" shape that
    /// `missing_listing_price_orders_count` (2.0.48) exists to test.
    #[allow(clippy::too_many_arguments)]
    fn seed_ticket_for_order(
        conn: &Connection,
        code_suffix: &str,
        event_id: i64,
        order_id: i64,
        status: &str,
        currency: &str,
        purchase_cost_cents: i64,
        listing_price_cents: Option<i64>,
    ) -> i64 {
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

    // ---- non_eur_order_currencies (2.0.51) ---------------------------------
    // Powers the Dashboard mixed-currency banner's per-currency/"Convert to
    // EUR" buttons - deliberately ORDER-scoped (see this field's own doc
    // comment for why it's a separate query from the ticket-scoped
    // `currencies`/`mixed_currencies` computed just above).

    #[test]
    fn non_eur_order_currencies_groups_and_counts_non_eur_orders_only() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event", "upcoming", None);
        conn.execute(
            "INSERT INTO orders (code, event_id, purchase_date, quantity, currency, payment_status)
             VALUES ('ORD-g1', ?1, '2026-01-01', 1, 'GBP', 'paid')",
            params![event_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO orders (code, event_id, purchase_date, quantity, currency, payment_status)
             VALUES ('ORD-g2', ?1, '2026-01-01', 1, 'GBP', 'paid')",
            params![event_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO orders (code, event_id, purchase_date, quantity, currency, payment_status)
             VALUES ('ORD-u1', ?1, '2026-01-01', 1, 'USD', 'paid')",
            params![event_id],
        )
        .unwrap();
        seed_order_only(&conn, "e1", event_id, "paid"); // EUR - must never be counted here.

        let data = get_dashboard_impl(&conn, None, None, None, None, None).unwrap();

        let mut got = data.non_eur_order_currencies.clone();
        got.sort_by(|a, b| a.currency.cmp(&b.currency));
        assert_eq!(got.len(), 2, "exactly GBP and USD - the EUR order must not appear at all");
        assert_eq!(got[0].currency, "GBP");
        assert_eq!(got[0].order_count, 2);
        assert_eq!(got[1].currency, "USD");
        assert_eq!(got[1].order_count, 1);
    }

    #[test]
    fn non_eur_order_currencies_is_empty_when_every_order_is_eur() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event", "upcoming", None);
        seed_order_only(&conn, "e1", event_id, "paid");

        let data = get_dashboard_impl(&conn, None, None, None, None, None).unwrap();

        assert!(data.non_eur_order_currencies.is_empty());
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
        assert_eq!(data.alerts.pending_sales_count, 0);
        assert_eq!(data.alerts.pending_sales_amount_cents, 0);
        assert_eq!(data.alerts.pending_sales_currency, Some("EUR".to_string()));
        assert_eq!(data.cashflow.paid_cents, 0);
        assert_eq!(data.cashflow.outstanding_cents, 0);
        assert_eq!(data.cashflow.revenue_cents, 0);
        assert_eq!(data.cashflow.profit_cents, 0);
        assert_eq!(data.cashflow.currency, Some("EUR".to_string()));
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

    /// 2.0.48: marko's report - the Activity tab showed "27" against
    /// `missing_listing_price_count`, which is a raw per-ticket tally, when
    /// he thinks in orders (one order with several unpriced tickets should
    /// look like one thing to fix, not a pile of unrelated ones). This test
    /// is the one place that actually proves the two fields diverge: 3 raw
    /// unpriced tickets, spread across only 2 orders.
    #[test]
    fn missing_listing_price_orders_count_counts_each_order_once_no_matter_how_many_tickets() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event", "upcoming", None);
        let order_a = seed_order_only(&conn, "a", event_id, "paid");
        seed_ticket_for_order(&conn, "a1", event_id, order_a, "available", "EUR", 1000, None);
        seed_ticket_for_order(&conn, "a2", event_id, order_a, "listed", "EUR", 1000, None);
        // A second order with its own single unpriced ticket.
        seed_ticket(&conn, "b1", event_id, "available", "EUR", 1000, None);
        // A priced ticket, on a third order - must not count toward either field.
        seed_ticket(&conn, "c1", event_id, "available", "EUR", 1000, Some(1200));

        let data = get_dashboard_impl(&conn, None, None, None, None, None).unwrap();

        assert_eq!(
            data.alerts.missing_listing_price_count, 3,
            "still 3 raw unpriced tickets - the existing ticket-scoped field must not change"
        );
        assert_eq!(
            data.alerts.missing_listing_price_orders_count, 2,
            "order A's two unpriced tickets count once, plus order B's one unpriced ticket = 2 orders"
        );
    }

    /// Second pair of eyes flagged this as an untested combination: does the
    /// order-level count correctly ignore a same-order ticket that fails the
    /// status filter, rather than letting it drag the whole order in? A sold
    /// ticket with no listing price (normal - it doesn't need one any more)
    /// sharing an order with an ordinary priced available ticket must
    /// contribute 0 to both fields, not 1.
    #[test]
    fn missing_listing_price_orders_count_ignores_an_order_whose_only_unpriced_ticket_is_not_available_or_listed() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event", "upcoming", None);
        let order_a = seed_order_only(&conn, "a", event_id, "paid");
        seed_ticket_for_order(&conn, "a1", event_id, order_a, "sold", "EUR", 1000, None);
        seed_ticket_for_order(&conn, "a2", event_id, order_a, "available", "EUR", 1000, Some(1500));

        let data = get_dashboard_impl(&conn, None, None, None, None, None).unwrap();

        assert_eq!(data.alerts.missing_listing_price_count, 0);
        assert_eq!(data.alerts.missing_listing_price_orders_count, 0);
    }

    // ---- Attention: pending sales (1.8.3 section 13) -----------------------

    /// Same as `seed_sale` but with an explicit payment_status - used only by
    /// the pending-sales tests below. Every existing chart test keeps using
    /// the plain `seed_sale` helper (always "paid"), which is deliberately
    /// left untouched.
    fn seed_sale_with_status(
        conn: &mut Connection,
        ticket_id: i64,
        sale_date: &str,
        price_cents: i64,
        payment_status: &str,
    ) -> i64 {
        crate::commands::sales::create_sale_impl(
            conn,
            &crate::models::SaleInput {
                ticket_id,
                platform_id: None,
                sale_date: sale_date.to_string(),
                sale_price_cents: price_cents,
                selling_fees_cents: 0,
                payment_status: Some(payment_status.to_string()),
                buyer_reference: None,
                notes: None,
            },
        )
        .unwrap()
    }

    #[test]
    fn pending_sales_counts_and_sums_only_pending_payment_status() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn, "Test Event", "upcoming", None);
        let t1 = seed_ticket(&conn, "1", event_id, "available", "EUR", 1000, None);
        let t2 = seed_ticket(&conn, "2", event_id, "available", "EUR", 1000, None);
        let t3 = seed_ticket(&conn, "3", event_id, "available", "EUR", 1000, None);
        seed_sale_with_status(&mut conn, t1, "2026-03-01", 2000, "pending");
        seed_sale_with_status(&mut conn, t2, "2026-03-02", 3000, "pending");
        seed_sale_with_status(&mut conn, t3, "2026-03-03", 5000, "paid"); // must not count

        let data = get_dashboard_impl(&conn, None, None, None, None, None).unwrap();

        assert_eq!(data.alerts.pending_sales_count, 2);
        assert_eq!(data.alerts.pending_sales_amount_cents, 5000, "2000 + 3000 pending, the paid one excluded");
        assert_eq!(data.alerts.pending_sales_currency, Some("EUR".to_string()));
    }

    /// 2.0.48: marko's report - "shows 12, I only made 3 sales". Cause: this
    /// query used to be `COUNT(*) FROM sales`, and `sales` stores one row
    /// PER TICKET - a multi-ticket "New sale" batch shares one batch_id
    /// across several rows (sales.rs's GROUP_KEY_EXPR). This test builds
    /// exactly that shape: one 3-ticket pending batch (1 real sale, 3 rows)
    /// plus one ordinary single-ticket pending sale (1 real sale, 1 row) -
    /// four rows in `sales`, two real sales.
    #[test]
    fn pending_sales_count_counts_sale_groups_not_raw_per_ticket_rows() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn, "Test Event", "upcoming", None);
        let t1 = seed_ticket(&conn, "1", event_id, "available", "EUR", 1000, None);
        let t2 = seed_ticket(&conn, "2", event_id, "available", "EUR", 1000, None);
        let t3 = seed_ticket(&conn, "3", event_id, "available", "EUR", 1000, None);
        let t4 = seed_ticket(&conn, "4", event_id, "available", "EUR", 1000, None);
        crate::commands::sales::create_sales_batch_impl(
            &mut conn,
            &crate::models::SaleBatchInput {
                lines: vec![
                    crate::models::SaleBatchLineInput {
                        ticket_id: t1,
                        sale_price_cents: 2000,
                        selling_fees_cents: 0,
                    },
                    crate::models::SaleBatchLineInput {
                        ticket_id: t2,
                        sale_price_cents: 2000,
                        selling_fees_cents: 0,
                    },
                    crate::models::SaleBatchLineInput {
                        ticket_id: t3,
                        sale_price_cents: 2000,
                        selling_fees_cents: 0,
                    },
                ],
                platform_id: None,
                sale_date: "2026-03-01".to_string(),
                payment_status: Some("pending".to_string()),
                buyer_reference: None,
                notes: None,
                currency: None,
            },
        )
        .unwrap();
        seed_sale_with_status(&mut conn, t4, "2026-03-02", 1500, "pending");

        let data = get_dashboard_impl(&conn, None, None, None, None, None).unwrap();

        assert_eq!(
            data.alerts.pending_sales_count, 2,
            "one 3-ticket batch (1 group) + one single sale (1 group) = 2, not 4 raw rows"
        );
        assert_eq!(
            data.alerts.pending_sales_amount_cents,
            2000 * 3 + 1500,
            "the money total still sums every row - grouping only changes the COUNT, never the SUM"
        );
    }

    #[test]
    fn pending_sales_excludes_refunded_even_if_it_was_pending_before_the_refund() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn, "Test Event", "upcoming", None);
        let t1 = seed_ticket(&conn, "1", event_id, "available", "EUR", 1000, None);
        let sale_id = seed_sale_with_status(&mut conn, t1, "2026-03-01", 2000, "pending");
        crate::commands::sales::refund_sale_impl(&mut conn, sale_id, Some("test refund")).unwrap();

        let data = get_dashboard_impl(&conn, None, None, None, None, None).unwrap();

        assert_eq!(data.alerts.pending_sales_count, 0, "a refund must flip payment_status away from pending");
        assert_eq!(data.alerts.pending_sales_amount_cents, 0);
    }

    #[test]
    fn pending_sales_amount_is_not_period_filtered() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn, "Test Event", "upcoming", None);
        let t1 = seed_ticket(&conn, "1", event_id, "available", "EUR", 1000, None);
        // Sold long before any of the "recent" period presets would cover.
        seed_sale_with_status(&mut conn, t1, "2001-01-01", 2000, "pending");

        let data = get_dashboard_impl(&conn, Some("today"), None, None, None, None).unwrap();

        assert_eq!(
            data.alerts.pending_sales_count, 1,
            "pending sales is a right-now fact, like unpaid_orders_count - never period-filtered"
        );
    }

    // ---- Cashflow (1.9.0) ---------------------------------------------------

    #[test]
    fn cashflow_splits_revenue_into_paid_and_outstanding_and_the_invariant_holds() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn, "Test Event", "upcoming", None);
        let t1 = seed_ticket(&conn, "1", event_id, "available", "EUR", 1000, None);
        let t2 = seed_ticket(&conn, "2", event_id, "available", "EUR", 1000, None);
        seed_sale_with_status(&mut conn, t1, "2026-03-01", 2000, "paid");
        seed_sale_with_status(&mut conn, t2, "2026-03-02", 3000, "pending");

        let data = get_dashboard_impl(&conn, None, None, None, None, None).unwrap();

        assert_eq!(data.cashflow.paid_cents, 2000);
        assert_eq!(data.cashflow.outstanding_cents, 3000);
        assert_eq!(data.cashflow.revenue_cents, 5000);
        assert_eq!(
            data.cashflow.revenue_cents,
            data.cashflow.paid_cents + data.cashflow.outstanding_cents,
            "revenue must always equal paid + outstanding - every non-refunded sale is exactly one or the other, never both, never neither"
        );
        assert_eq!(
            data.cashflow.profit_cents, data.inventory.profit_cents,
            "cashflow.profit is the same realized figure as inventory.profit, not a second, independently-computed number"
        );
        assert_eq!(data.cashflow.revenue_cents, data.inventory.revenue_cents);
        assert_eq!(data.cashflow.currency, Some("EUR".to_string()));
    }

    #[test]
    fn cashflow_excludes_refunded_sales_from_paid_outstanding_and_revenue() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn, "Test Event", "upcoming", None);
        let t1 = seed_ticket(&conn, "1", event_id, "available", "EUR", 1000, None);
        let t2 = seed_ticket(&conn, "2", event_id, "available", "EUR", 1000, None);
        let t3 = seed_ticket(&conn, "3", event_id, "available", "EUR", 1000, None);
        seed_sale_with_status(&mut conn, t1, "2026-03-01", 2000, "paid");
        seed_sale_with_status(&mut conn, t2, "2026-03-02", 3000, "pending");
        let refunded_sale_id = seed_sale_with_status(&mut conn, t3, "2026-03-03", 9000, "paid");
        crate::commands::sales::refund_sale_impl(&mut conn, refunded_sale_id, Some("test refund")).unwrap();

        let data = get_dashboard_impl(&conn, None, None, None, None, None).unwrap();

        assert_eq!(data.cashflow.paid_cents, 2000, "the refunded sale's real 9000 price must not count as paid");
        assert_eq!(data.cashflow.outstanding_cents, 3000);
        assert_eq!(data.cashflow.revenue_cents, 5000, "refunded sale excluded from revenue too");
    }

    #[test]
    fn cashflow_outstanding_drops_to_zero_once_a_pending_sale_is_refunded() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn, "Test Event", "upcoming", None);
        let t1 = seed_ticket(&conn, "1", event_id, "available", "EUR", 1000, None);
        let sale_id = seed_sale_with_status(&mut conn, t1, "2026-03-01", 2000, "pending");

        let before = get_dashboard_impl(&conn, None, None, None, None, None).unwrap();
        assert_eq!(before.cashflow.outstanding_cents, 2000);

        crate::commands::sales::refund_sale_impl(&mut conn, sale_id, Some("test refund")).unwrap();

        let after = get_dashboard_impl(&conn, None, None, None, None, None).unwrap();
        assert_eq!(
            after.cashflow.outstanding_cents, 0,
            "a refund must remove the sale from outstanding, not leave it stuck there"
        );
        assert_eq!(after.cashflow.paid_cents, 0, "a refund must never move money into paid either");
        assert_eq!(after.cashflow.revenue_cents, 0);
    }

    #[test]
    fn cashflow_is_scoped_to_primary_currency_and_shows_none_when_mixed() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn, "Test Event", "upcoming", None);
        let eur_ticket = seed_ticket(&conn, "1", event_id, "available", "EUR", 1000, None);
        let usd_ticket = seed_ticket(&conn, "2", event_id, "available", "USD", 1000, None);
        seed_sale_with_status(&mut conn, eur_ticket, "2026-03-01", 2000, "paid");
        seed_sale_with_status(&mut conn, usd_ticket, "2026-03-01", 5000, "paid");

        let data = get_dashboard_impl(&conn, None, None, None, None, None).unwrap();

        assert!(data.mixed_currencies);
        assert_eq!(data.cashflow.currency, None, "two currencies present - must never be blended into one figure");
        assert_eq!(
            data.cashflow.paid_cents, 2000,
            "only EUR (the primary currency here) counts - the USD sale must not leak in"
        );
    }

    #[test]
    fn cashflow_reflects_only_the_active_sale_after_refund_then_resell() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn, "Test Event", "upcoming", None);
        let t1 = seed_ticket(&conn, "1", event_id, "available", "EUR", 1000, None);

        let first_sale_id = seed_sale_with_status(&mut conn, t1, "2026-03-01", 2000, "paid");
        crate::commands::sales::refund_sale_impl(&mut conn, first_sale_id, Some("buyer cancelled")).unwrap();
        // Resold at a different price, initially pending - the exact refund
        // -> resell flow BUG #1 exists to support (see sales.rs).
        seed_sale_with_status(&mut conn, t1, "2026-03-05", 1800, "pending");

        let data = get_dashboard_impl(&conn, None, None, None, None, None).unwrap();

        assert_eq!(data.cashflow.paid_cents, 0, "the original sale's real payment is refunded, not paid");
        assert_eq!(data.cashflow.outstanding_cents, 1800, "only the new resale counts, at its own price");
        assert_eq!(data.cashflow.revenue_cents, 1800);
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
        // 2.0.68: one bucket per calendar day in the whole requested range
        // (2026-03-01..07 inclusive = 7 days) - not just the 2 days that
        // actually had a sale. See fill_time_series_gaps's own doc comment
        // for why the zero-sale days must be real, explicit points now.
        assert_eq!(data.revenue_time_series.len(), 7, "every day in the period, including the zero-sale ones");
        assert_eq!(data.revenue_time_series[0].bucket_start, "2026-03-01");
        assert_eq!(data.revenue_time_series[0].revenue_cents, 5000, "2000 + 3000 on the same day");
        assert_eq!(data.revenue_time_series[0].sold_tickets, 2, "two sales lines that day");
        assert_eq!(data.revenue_time_series[1].bucket_start, "2026-03-02");
        assert_eq!(data.revenue_time_series[1].revenue_cents, 0, "no sale that day -> an explicit zero bucket, not a gap");
        assert_eq!(data.revenue_time_series[2].bucket_start, "2026-03-03");
        assert_eq!(data.revenue_time_series[2].revenue_cents, 1500);
        assert_eq!(data.revenue_time_series[2].sold_tickets, 1);
        assert_eq!(data.revenue_time_series[6].bucket_start, "2026-03-07", "the period's last day is still included");
        // Cross-check against the existing, already-trusted period total -
        // the chart must never be able to silently drift from the StatCards
        // showing the same period.
        let chart_revenue: i64 = data.revenue_time_series.iter().map(|p| p.revenue_cents).sum();
        let chart_profit: i64 = data.revenue_time_series.iter().map(|p| p.profit_cents).sum();
        let chart_sold_tickets: i64 = data.revenue_time_series.iter().map(|p| p.sold_tickets).sum();
        assert_eq!(chart_revenue, data.period.revenue_cents);
        assert_eq!(chart_profit, data.period.profit_cents);
        assert_eq!(chart_sold_tickets, data.period.sold_tickets, "1.7.5: same cross-check, now for the Sales metric too");
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

        // 2.0.68: every day in the period gets a bucket now (see
        // fill_time_series_gaps) - the refunded day's own bucket must still
        // show as an explicit zero, not silently disappear, since "no data"
        // and "a real day with zero after its only sale was refunded" need
        // to look the same on the chart.
        assert_eq!(data.revenue_time_series.len(), 7, "every day in the period, including the refunded one");
        assert_eq!(data.revenue_time_series[0].bucket_start, "2026-03-01");
        assert_eq!(data.revenue_time_series[0].revenue_cents, 2000);
        assert_eq!(data.revenue_time_series[0].sold_tickets, 1, "the refunded sale must not count as a sold ticket either");
        assert_eq!(data.revenue_time_series[1].bucket_start, "2026-03-02");
        assert_eq!(data.revenue_time_series[1].revenue_cents, 0, "the refunded day's only sale must not count");
        assert_eq!(data.revenue_time_series[1].sold_tickets, 0);
    }

    // 1.7.5: sold_tickets is a real COUNT(*) of sales lines, deliberately
    // independent of the money columns on the same row - a bucket's revenue
    // alone can't tell you how many tickets made it up (one expensive ticket
    // reads identically to several cheap ones in a money-only view). This
    // seeds a low-count/high-price bucket next to a high-count/low-price one
    // so a bug that derived sold_tickets from revenue_cents (or vice versa)
    // would show up as a wrong count on at least one of them.
    #[test]
    fn revenue_time_series_sold_tickets_counts_lines_not_money() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn, "Chart Count Event", "upcoming", None);
        let expensive = seed_ticket(&conn, "1", event_id, "available", "EUR", 1000, None);
        let cheap_a = seed_ticket(&conn, "2", event_id, "available", "EUR", 1000, None);
        let cheap_b = seed_ticket(&conn, "3", event_id, "available", "EUR", 1000, None);
        let cheap_c = seed_ticket(&conn, "4", event_id, "available", "EUR", 1000, None);
        seed_sale(&mut conn, expensive, "2026-03-01", 9000); // 1 ticket, high revenue
        seed_sale(&mut conn, cheap_a, "2026-03-02", 1000);
        seed_sale(&mut conn, cheap_b, "2026-03-02", 1000);
        seed_sale(&mut conn, cheap_c, "2026-03-02", 1000); // 3 tickets, lower total revenue

        let data = get_dashboard_impl(
            &conn,
            Some("custom"),
            Some("2026-03-01".to_string()),
            Some("2026-03-07".to_string()),
            None,
            None,
        )
        .unwrap();

        // 2.0.68: 7 days in the requested range now, not just the 2 that had
        // a sale - see fill_time_series_gaps.
        assert_eq!(data.revenue_time_series.len(), 7);
        assert_eq!(data.revenue_time_series[0].bucket_start, "2026-03-01");
        assert_eq!(data.revenue_time_series[0].sold_tickets, 1);
        assert_eq!(data.revenue_time_series[0].revenue_cents, 9000);
        assert_eq!(data.revenue_time_series[1].bucket_start, "2026-03-02");
        assert_eq!(data.revenue_time_series[1].sold_tickets, 3);
        assert_eq!(data.revenue_time_series[1].revenue_cents, 3000);
        assert_eq!(data.revenue_time_series[2].sold_tickets, 0, "the rest of the period has no sales");
    }

    #[test]
    fn months_ago_subtracts_whole_calendar_months_when_the_day_exists_in_the_target_month() {
        let d = NaiveDate::from_ymd_opt(2026, 8, 19).unwrap();
        assert_eq!(months_ago(d, 1), NaiveDate::from_ymd_opt(2026, 7, 19).unwrap());
        assert_eq!(months_ago(d, 3), NaiveDate::from_ymd_opt(2026, 5, 19).unwrap());
        assert_eq!(months_ago(d, 12), NaiveDate::from_ymd_opt(2025, 8, 19).unwrap(), "1y = same date last year");
        assert_eq!(months_ago(d, 60), NaiveDate::from_ymd_opt(2021, 8, 19).unwrap(), "5y = same date 5 years back");
    }

    #[test]
    fn months_ago_clamps_to_the_last_day_of_the_target_month_when_the_original_day_does_not_exist() {
        // March 31 minus 1 month: February never has a 31st. Must clamp to
        // Feb 28 (2026 is not a leap year), not error and not roll over into
        // March.
        let d = NaiveDate::from_ymd_opt(2026, 3, 31).unwrap();
        assert_eq!(months_ago(d, 1), NaiveDate::from_ymd_opt(2026, 2, 28).unwrap());
    }

    #[test]
    fn months_ago_clamps_to_february_29_in_a_leap_year() {
        let d = NaiveDate::from_ymd_opt(2028, 3, 31).unwrap(); // 2028 is a leap year
        assert_eq!(months_ago(d, 1), NaiveDate::from_ymd_opt(2028, 2, 29).unwrap());
    }

    #[test]
    fn period_bounds_relative_presets_match_the_reference_range_picker() {
        let today = NaiveDate::from_ymd_opt(2026, 8, 19).unwrap();
        assert_eq!(period_bounds(Some("today"), None, None, today), ("2026-08-19".into(), "2026-08-19".into()));
        assert_eq!(period_bounds(Some("1w"), None, None, today), ("2026-08-13".into(), "2026-08-19".into()));
        assert_eq!(period_bounds(Some("1m"), None, None, today), ("2026-07-19".into(), "2026-08-19".into()));
        assert_eq!(period_bounds(Some("3m"), None, None, today), ("2026-05-19".into(), "2026-08-19".into()));
        assert_eq!(period_bounds(Some("1y"), None, None, today), ("2025-08-19".into(), "2026-08-19".into()));
        assert_eq!(period_bounds(Some("5y"), None, None, today), ("2021-08-19".into(), "2026-08-19".into()));
        assert_eq!(period_bounds(Some("all"), None, None, today), ("0001-01-01".into(), "9999-12-31".into()));
    }

    #[test]
    fn period_bounds_ytd_starts_at_january_first_of_todays_year() {
        let today = NaiveDate::from_ymd_opt(2026, 8, 19).unwrap();
        assert_eq!(period_bounds(Some("ytd"), None, None, today), ("2026-01-01".into(), "2026-08-19".into()));

        // Jan 1st itself: YTD must still be a real, non-empty one-day range,
        // not an off-by-one that starts "before" today.
        let jan_first = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        assert_eq!(period_bounds(Some("ytd"), None, None, jan_first), ("2026-01-01".into(), "2026-01-01".into()));
    }

    #[test]
    fn period_bounds_custom_is_unaffected_by_the_1_7_5_preset_rename() {
        // Guards the existing Custom-date-filter fix (BUG list) - "custom"
        // with both from/to empty must still fall back to the "no bound"
        // sentinels exactly as before, unrelated to the preset key rename.
        let today = NaiveDate::from_ymd_opt(2026, 8, 19).unwrap();
        assert_eq!(
            period_bounds(Some("custom"), Some("2026-02-01".to_string()), Some("2026-02-15".to_string()), today),
            ("2026-02-01".into(), "2026-02-15".into())
        );
        assert_eq!(period_bounds(Some("custom"), None, None, today), ("0001-01-01".into(), "2026-08-19".into()));
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
        // 2.0.68: every ISO week in the 90-day requested range now gets its
        // own bucket (see fill_time_series_gaps), so the two same-week sales
        // still collapse into exactly ONE non-zero point among many zero
        // ones - not a hardcoded total bucket count, which would make this
        // test as fragile as hand-counting ISO weeks by eye.
        assert!(data.revenue_time_series.len() > 1, "the zero-sale weeks must be real buckets too, not just this one");
        let non_zero: Vec<_> = data.revenue_time_series.iter().filter(|p| p.revenue_cents != 0).collect();
        assert_eq!(non_zero.len(), 1, "same ISO week -> still just one NON-ZERO bucket");
        assert_eq!(non_zero[0].revenue_cents, 3000);
        assert_eq!(non_zero[0].bucket_start, "2026-02-03", "earliest date in the bucket");
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
        // 2.0.68: Jan-Dec 2026 was requested, so all 12 real calendar months
        // now get their own bucket (see fill_time_series_gaps) - not just the
        // one month that actually had sales.
        assert_eq!(data.revenue_time_series.len(), 12, "one bucket per real calendar month in the requested range");
        let feb = &data.revenue_time_series[1];
        // A real (non-empty) bucket keeps the SQL row's own bucket_start -
        // MIN(sale_date) within the month, i.e. the earlier of the two sales -
        // rather than the synthetic "first of the month" used to backfill an
        // EMPTY bucket (see fill_time_series_gaps's doc comment).
        assert_eq!(feb.bucket_start, "2026-02-03");
        assert_eq!(feb.revenue_cents, 3000, "same month -> one non-zero bucket combining both sales");
        let non_zero: Vec<_> = data.revenue_time_series.iter().filter(|p| p.revenue_cents != 0).collect();
        assert_eq!(non_zero.len(), 1, "every other month must be a real zero bucket, not just absent");
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

        // 2.0.68: the 7-day requested range now yields 7 real daily buckets
        // (see fill_time_series_gaps), not just the one day that had a sale.
        assert_eq!(data.revenue_time_series.len(), 7, "one bucket per day across the 7-day requested range");
        assert_eq!(data.revenue_time_series[0].bucket_start, "2026-03-01");
        assert_eq!(data.revenue_time_series[0].revenue_cents, 2000, "event_b's sale must not leak in");
        let total: i64 = data.revenue_time_series.iter().map(|p| p.revenue_cents).sum();
        assert_eq!(total, data.period.revenue_cents);
    }

    // ---- 2.0.68: Purchase cost reconciles with Profit (marko's report) ----

    /// marko's exact report: "Purchase cost" (scoped by order purchase_date)
    /// and Profit/Margin/ROI (scoped by sale_date) could legitimately
    /// disagree for the very same period - even though each number was
    /// independently correct for what it measured, side by side they never
    /// added up. Seeds exactly the shape that exposes it: one ticket bought
    /// well BEFORE the period but SOLD inside it (the old scoping excluded
    /// its cost from "Purchase cost" entirely, even though it's exactly the
    /// ticket Profit/COGS/ROI are about), and one ticket bought INSIDE the
    /// period but not yet sold (the old scoping counted its cost toward
    /// "Purchase cost" anyway, inflating it with a ticket Profit/COGS/ROI
    /// never touch).
    #[test]
    fn period_purchase_cost_now_matches_cogs_of_tickets_actually_sold_in_the_period() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn, "Reconcile Event", "upcoming", None);

        let sold_in_period = seed_ticket(&conn, "1", event_id, "available", "EUR", 4000, None);
        seed_sale(&mut conn, sold_in_period, "2026-03-02", 9000);

        let bought_in_period_unsold = seed_ticket(&conn, "2", event_id, "available", "EUR", 2500, None);
        conn.execute(
            "UPDATE orders SET purchase_date = '2026-03-03' WHERE id = (SELECT order_id FROM tickets WHERE id = ?1)",
            [bought_in_period_unsold],
        )
        .unwrap();

        let data = get_dashboard_impl(
            &conn,
            Some("custom"),
            Some("2026-03-01".to_string()),
            Some("2026-03-07".to_string()),
            None,
            None,
        )
        .unwrap();

        assert_eq!(
            data.period.purchased_tickets, 1,
            "still just the ticket actually bought in-period - unchanged meaning, still feeds the 'N purchased in period' sub-line"
        );
        assert_eq!(
            data.period.total_cost_cents, 4000,
            "Purchase cost is now the cost of the SOLD ticket (COGS), not the unsold ticket bought in-period"
        );
        assert_eq!(data.period.total_cost_cents, data.period.cogs_cents, "Purchase cost and COGS must be the same figure now");
        assert_eq!(
            data.period.revenue_cents - data.period.total_cost_cents - data.period.selling_fees_cents,
            data.period.profit_cents,
            "Revenue - Purchase cost - fees must always equal the Profit shown next to it"
        );
    }

    // ---- 2.0.47: previous-period comparison (DIR-001 KPI trend) -----------

    #[test]
    fn previous_period_bounds_is_the_immediately_preceding_equal_length_window() {
        assert_eq!(
            previous_period_bounds("2026-03-01", "2026-03-07"),
            Some(("2026-02-22".to_string(), "2026-02-28".to_string())),
            "7-day period -> 7-day window ending the day before it starts"
        );
        assert_eq!(
            previous_period_bounds("2026-08-19", "2026-08-19"),
            Some(("2026-08-18".to_string(), "2026-08-18".to_string())),
            "a single-day period (Today) compares against yesterday"
        );
        assert_eq!(
            previous_period_bounds("2026-01-01", "2026-01-05"),
            Some(("2025-12-27".to_string(), "2025-12-31".to_string())),
            "must cross a year boundary correctly"
        );
    }

    #[test]
    fn previous_period_bounds_returns_none_when_there_is_no_real_start_or_end() {
        assert_eq!(previous_period_bounds("0001-01-01", "9999-12-31"), None, "All time");
        assert_eq!(previous_period_bounds("0001-01-01", "2026-08-19"), None, "Custom with an empty From");
        assert_eq!(previous_period_bounds("2026-01-01", "9999-12-31"), None, "defensive: an unbounded end too");
    }

    #[test]
    fn previous_period_is_populated_and_scoped_to_the_immediately_preceding_window() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn, "Trend Event", "upcoming", None);
        let current_ticket = seed_ticket(&conn, "1", event_id, "available", "EUR", 1000, None);
        let prior_ticket = seed_ticket(&conn, "2", event_id, "available", "EUR", 1000, None);
        seed_sale(&mut conn, current_ticket, "2026-03-01", 2000); // inside the current period
        seed_sale(&mut conn, prior_ticket, "2026-02-25", 5000); // inside the immediately preceding period

        let data = get_dashboard_impl(
            &conn,
            Some("custom"),
            Some("2026-03-01".to_string()),
            Some("2026-03-07".to_string()),
            None,
            None,
        )
        .unwrap();

        assert_eq!(data.period.revenue_cents, 2000, "only the current-period sale");
        let previous = data.previous_period.expect("a real custom range must have a previous period");
        assert_eq!(previous.revenue_cents, 5000, "only the prior-window sale, none of the current period's");
        assert_eq!(previous.sold_tickets, 1);
    }

    #[test]
    fn previous_period_is_none_when_there_is_no_sensible_prior_window() {
        let conn = test_conn();
        let data_all = get_dashboard_impl(&conn, Some("all"), None, None, None, None).unwrap();
        assert!(data_all.previous_period.is_none(), "there is no 'before all time'");

        let data_custom_empty_from =
            get_dashboard_impl(&conn, Some("custom"), None, Some("2026-08-19".to_string()), None, None).unwrap();
        assert!(
            data_custom_empty_from.previous_period.is_none(),
            "a custom range with no explicit start has no real prior window either"
        );
    }

    // ---- 2.0.47: sales by platform (DIR-001 signature idea #02) -----------

    fn seed_platform(conn: &Connection, name: &str) -> i64 {
        conn.execute("INSERT INTO platforms (name) VALUES (?1)", params![name]).unwrap();
        conn.last_insert_rowid()
    }

    /// Same as `seed_sale` but with an explicit platform_id - used only by
    /// the sales-by-platform tests below. Every existing chart test keeps
    /// using the plain `seed_sale` helper (always platform_id: None),
    /// deliberately left untouched.
    fn seed_sale_with_platform(
        conn: &mut Connection,
        ticket_id: i64,
        sale_date: &str,
        price_cents: i64,
        platform_id: Option<i64>,
    ) -> i64 {
        crate::commands::sales::create_sale_impl(
            conn,
            &crate::models::SaleInput {
                ticket_id,
                platform_id,
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
    fn sales_by_platform_groups_and_orders_by_revenue_descending() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn, "Platform Event", "upcoming", None);
        let stubhub = seed_platform(&conn, "StubHub");
        let viagogo = seed_platform(&conn, "Viagogo");
        let t1 = seed_ticket(&conn, "1", event_id, "available", "EUR", 1000, None);
        let t2 = seed_ticket(&conn, "2", event_id, "available", "EUR", 1000, None);
        let t3 = seed_ticket(&conn, "3", event_id, "available", "EUR", 1000, None);
        seed_sale_with_platform(&mut conn, t1, "2026-03-01", 2000, Some(stubhub));
        seed_sale_with_platform(&mut conn, t2, "2026-03-02", 9000, Some(viagogo)); // biggest -> must sort first
        seed_sale_with_platform(&mut conn, t3, "2026-03-03", 1000, Some(stubhub));

        let data = get_dashboard_impl(
            &conn,
            Some("custom"),
            Some("2026-03-01".to_string()),
            Some("2026-03-07".to_string()),
            None,
            None,
        )
        .unwrap();

        assert_eq!(data.sales_by_platform.len(), 2);
        assert_eq!(data.sales_by_platform[0].platform_name, Some("Viagogo".to_string()));
        assert_eq!(data.sales_by_platform[0].revenue_cents, 9000);
        assert_eq!(data.sales_by_platform[0].sold_tickets, 1);
        assert_eq!(data.sales_by_platform[1].platform_name, Some("StubHub".to_string()));
        assert_eq!(data.sales_by_platform[1].revenue_cents, 3000, "2000 + 1000, StubHub's two sales combined");
        assert_eq!(data.sales_by_platform[1].sold_tickets, 2);
    }

    #[test]
    fn sales_by_platform_gives_sales_with_no_platform_their_own_row() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn, "No Platform Event", "upcoming", None);
        let t1 = seed_ticket(&conn, "1", event_id, "available", "EUR", 1000, None);
        seed_sale_with_platform(&mut conn, t1, "2026-03-01", 2000, None);

        let data = get_dashboard_impl(
            &conn,
            Some("custom"),
            Some("2026-03-01".to_string()),
            Some("2026-03-07".to_string()),
            None,
            None,
        )
        .unwrap();

        assert_eq!(data.sales_by_platform.len(), 1);
        assert_eq!(data.sales_by_platform[0].platform_id, None);
        assert_eq!(data.sales_by_platform[0].platform_name, None, "frontend shows this row as \"No platform\"");
        assert_eq!(data.sales_by_platform[0].revenue_cents, 2000);
    }

    #[test]
    fn sales_by_platform_excludes_refunds_same_as_period_total() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn, "Refund Platform Event", "upcoming", None);
        let platform = seed_platform(&conn, "SeatGeek");
        let active = seed_ticket(&conn, "1", event_id, "available", "EUR", 1000, None);
        let refunded = seed_ticket(&conn, "2", event_id, "available", "EUR", 1000, None);
        seed_sale_with_platform(&mut conn, active, "2026-03-01", 2000, Some(platform));
        let refunded_sale_id = seed_sale_with_platform(&mut conn, refunded, "2026-03-02", 9000, Some(platform));
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

        assert_eq!(data.sales_by_platform.len(), 1);
        assert_eq!(data.sales_by_platform[0].revenue_cents, 2000, "the refunded sale must not count");
        assert_eq!(data.sales_by_platform[0].sold_tickets, 1);
    }

    #[test]
    fn sales_by_platform_profit_is_revenue_minus_cogs_minus_fees() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn, "Profit Platform Event", "upcoming", None);
        let platform = seed_platform(&conn, "TickPick");
        // purchase_fees_cents/other_costs_cents both default to 0 - see
        // seed_ticket - so this ticket's full cost is just its 1500.
        let t1 = seed_ticket(&conn, "1", event_id, "available", "EUR", 1500, None);

        crate::commands::sales::create_sale_impl(
            &mut conn,
            &crate::models::SaleInput {
                ticket_id: t1,
                platform_id: Some(platform),
                sale_date: "2026-03-01".to_string(),
                sale_price_cents: 5000,
                selling_fees_cents: 400,
                payment_status: Some("paid".to_string()),
                buyer_reference: None,
                notes: None,
            },
        )
        .unwrap();

        let data = get_dashboard_impl(
            &conn,
            Some("custom"),
            Some("2026-03-01".to_string()),
            Some("2026-03-07".to_string()),
            None,
            None,
        )
        .unwrap();

        assert_eq!(data.sales_by_platform.len(), 1);
        assert_eq!(data.sales_by_platform[0].revenue_cents, 5000);
        assert_eq!(data.sales_by_platform[0].profit_cents, 5000 - 1500 - 400, "revenue - cogs - selling fees");
    }
}

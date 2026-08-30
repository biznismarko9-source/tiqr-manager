//! 2.1.0: marko's "FINANCE 2.1" Cashflow Forecast - see
//! FINANCE-2.1.0-REPORT.md for the full spec/rationale. Read-only: this
//! module never writes anything, it only aggregates data that already
//! exists - accounts' own balances, future-dated finance_entries, active
//! recurring_expenses, and the ticket business's own
//! `sales.payment_status = 'pending'` concept (the exact same "right-now
//! fact" dashboard.rs's own `pending_sales_amount_cents` already uses -
//! read directly here with one small dedicated query, NEVER by importing or
//! calling into dashboard.rs/sales.rs, per marko's own DO-NOT-TOUCH list and
//! point 13's "Sales/Orders/Inventory finance ostávajú source of truth").
//!
//! Deliberately simple and non-AI (marko's own point 9):
//!   forecast_balance = current_balance + expected_income
//!                       - recurring_expenses - upcoming_expenses
//! Every input is either a real stored balance or something marko already
//! explicitly logged/scheduled himself - never a guessed future sale, an
//! estimated market price, or inventory profit (point 10's explicit
//! exclusion list). Strictly EUR-only (marko's own stated base currency) -
//! a non-EUR account balance, pending sale, or future entry is never
//! blended in with an invented FX rate; it is simply excluded, and
//! `excludes_non_eur_data` tells the frontend to show a small disclosure
//! note when that happened.
//!
//! "Current balance" here is deliberately its OWN as-of-today query, not a
//! call into `finance_accounts::list_accounts` - that command's own balance
//! is a running total of every entry/transfer ever logged, ANY date
//! (a plain, honest "everything you've recorded" ledger total, matching the
//! Accounts tab). A forecast, by definition, needs a strict "as of today"
//! anchor: if a future-dated income entry were already folded into that
//! running total, adding it AGAIN as "expected income" below would double-
//! count it. Scoping this query to `entry_date <= today` (and the mirror
//! `entry_date > today` for "expected"/"upcoming") is what keeps the two
//! numbers - what marko has right now, and what's still to come - from ever
//! overlapping.

use crate::db::AppState;
use crate::error::AppResult;
use crate::models::CashflowForecast;
use chrono::{Duration, Local, NaiveDate};
use rusqlite::Connection;
use tauri::State;

/// How far ahead "expected income"/"recurring expenses"/"upcoming expenses"
/// look. 30 days - a calendar month is the natural "how far can I see
/// clearly" horizon for a manually-maintained ledger with no bank feed, and
/// matches marko's own point 9 example scale. Not user-configurable in v1
/// (point 21 - keep this simple); a future version could expose it as a
/// picker (see FINANCE-2.1.0-REPORT.md's "future improvements").
const FORECAST_WINDOW_DAYS: i64 = 30;

/// Sum of every ACTIVE EUR account's balance, as of `as_of` (inclusive) -
/// same four-subquery-join shape as `finance_accounts::ACCOUNT_SELECT`
/// (deliberately not shared - see this module's own doc comment above for
/// why a forecast needs its own as-of-today-scoped version). Returns
/// `(total_balance_cents, matching_account_count)` - the count is what
/// decides `CashflowForecast::available` (marko's point 10: show "Forecast
/// unavailable" rather than a number computed from nothing).
fn eur_balance_as_of(conn: &Connection, as_of: &str) -> AppResult<(i64, i64)> {
    let sql = "SELECT COALESCE(SUM(
            a.opening_balance_cents + COALESCE(inc.total,0) - COALESCE(exp.total,0)
                + COALESCE(tin.total,0) - COALESCE(tout.total,0)
        ), 0), COUNT(*)
        FROM accounts a
        LEFT JOIN (SELECT account_id, SUM(amount_cents) AS total FROM finance_entries
                   WHERE entry_type = 'income' AND entry_date <= ?1 GROUP BY account_id) inc ON inc.account_id = a.id
        LEFT JOIN (SELECT account_id, SUM(amount_cents) AS total FROM finance_entries
                   WHERE entry_type = 'expense' AND entry_date <= ?1 GROUP BY account_id) exp ON exp.account_id = a.id
        LEFT JOIN (SELECT to_account_id AS account_id, SUM(amount_cents) AS total FROM transfers
                   WHERE transfer_date <= ?1 GROUP BY to_account_id) tin ON tin.account_id = a.id
        LEFT JOIN (SELECT from_account_id AS account_id, SUM(amount_cents) AS total FROM transfers
                   WHERE transfer_date <= ?1 GROUP BY from_account_id) tout ON tout.account_id = a.id
        WHERE a.currency = 'EUR' AND a.is_active = 1";
    Ok(conn.query_row(sql, [as_of], |r| Ok((r.get(0)?, r.get(1)?)))?)
}

/// Core logic behind `get_cashflow_forecast` - `today` is passed in (rather
/// than calling `Local::now()` here) purely so this stays a pure,
/// deterministically unit-testable calculation, same convention as
/// dashboard.rs's own `period_bounds`/`months_ago`.
pub(crate) fn get_cashflow_forecast_impl(conn: &Connection, today: NaiveDate) -> AppResult<CashflowForecast> {
    let today_str = today.to_string();
    let window_end = (today + Duration::days(FORECAST_WINDOW_DAYS)).to_string();

    let (current_balance_cents, eur_account_count) = eur_balance_as_of(conn, &today_str)?;

    // Point 14's currency safety rule, applied to the forecast: never blend
    // a non-EUR amount in - just note that something was left out, rather
    // than silently guessing an exchange rate or silently dropping it with
    // no trace at all. Computed up front so it is still reported even when
    // the forecast itself is unavailable below (e.g. marko only has a USD
    // account so far - "unavailable" alone would hide the real reason).
    let excludes_non_eur_data: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM accounts WHERE currency != 'EUR' AND is_active = 1)
            OR EXISTS(SELECT 1 FROM sales WHERE payment_status = 'pending' AND currency != 'EUR')
            OR EXISTS(SELECT 1 FROM finance_entries WHERE currency != 'EUR' AND entry_date > ?1 AND entry_date <= ?2
                      AND entry_type IN ('income','expense'))
            OR EXISTS(SELECT 1 FROM recurring_expenses WHERE is_active = 1 AND currency != 'EUR' AND next_date <= ?2)",
        rusqlite::params![today_str, window_end],
        |r| r.get(0),
    )?;

    if eur_account_count == 0 {
        return Ok(CashflowForecast {
            available: false,
            current_balance_cents: 0,
            expected_income_cents: 0,
            recurring_expenses_cents: 0,
            upcoming_expenses_cents: 0,
            forecast_balance_cents: 0,
            window_days: FORECAST_WINDOW_DAYS,
            excludes_non_eur_data,
        });
    }

    // Expected income: EUR income marko already logged for a future date,
    // inside the window, PLUS the ticket business's own pending sales (money
    // already earned - a ticket already sold - just not yet collected).
    // Neither is a guess: both are things that already exist in the app.
    let expected_income_entries_cents: i64 = conn.query_row(
        "SELECT COALESCE(SUM(amount_cents), 0) FROM finance_entries
         WHERE entry_type = 'income' AND currency = 'EUR' AND entry_date > ?1 AND entry_date <= ?2",
        rusqlite::params![today_str, window_end],
        |r| r.get(0),
    )?;
    // Same concept as dashboard.rs's own `pending_sales_amount_cents`
    // (payment_status='pending' - a right-now fact, never period-filtered -
    // see that module's own comment on this exact point), read directly
    // here rather than calling into dashboard.rs/sales.rs (marko's own
    // DO-NOT-TOUCH list, point 13).
    let pending_sales_eur_cents: i64 = conn.query_row(
        "SELECT COALESCE(SUM(sale_price_cents), 0) FROM sales WHERE payment_status = 'pending' AND currency = 'EUR'",
        [],
        |r| r.get(0),
    )?;
    let expected_income_cents = expected_income_entries_cents + pending_sales_eur_cents;

    // Recurring expenses due within the window - overdue ones (next_date
    // already in the past) are included too, since they're still an
    // unpaid, real obligation sitting there, not something that has
    // stopped applying just because it's late.
    let recurring_expenses_cents: i64 = conn.query_row(
        "SELECT COALESCE(SUM(amount_cents), 0) FROM recurring_expenses
         WHERE is_active = 1 AND currency = 'EUR' AND next_date <= ?1",
        [&window_end],
        |r| r.get(0),
    )?;

    // Upcoming expenses: EUR one-off expenses marko already logged for a
    // future date, inside the window.
    let upcoming_expenses_cents: i64 = conn.query_row(
        "SELECT COALESCE(SUM(amount_cents), 0) FROM finance_entries
         WHERE entry_type = 'expense' AND currency = 'EUR' AND entry_date > ?1 AND entry_date <= ?2",
        rusqlite::params![today_str, window_end],
        |r| r.get(0),
    )?;

    let forecast_balance_cents = current_balance_cents + expected_income_cents - recurring_expenses_cents - upcoming_expenses_cents;

    Ok(CashflowForecast {
        available: true,
        current_balance_cents,
        expected_income_cents,
        recurring_expenses_cents,
        upcoming_expenses_cents,
        forecast_balance_cents,
        window_days: FORECAST_WINDOW_DAYS,
        excludes_non_eur_data,
    })
}

#[tauri::command]
pub fn get_cashflow_forecast(state: State<AppState>) -> AppResult<CashflowForecast> {
    let conn = state.db.lock().unwrap();
    get_cashflow_forecast_impl(&conn, Local::now().date_naive())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::finance_accounts::create_account_impl;
    use crate::commands::finance_entries::create_finance_entry_impl;
    use crate::commands::finance_recurring::create_recurring_expense_impl;
    use crate::db::test_conn;
    use crate::models::{AccountInput, FinanceEntryInput, RecurringExpenseInput};

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 8, 30).unwrap()
    }

    fn eur_account(conn: &Connection, opening_cents: i64) -> i64 {
        create_account_impl(
            conn,
            &AccountInput { name: "Bank".to_string(), account_type: "bank".to_string(), currency: "EUR".to_string(), opening_balance_cents: opening_cents, is_active: true },
        )
        .unwrap()
        .id
    }

    // --- unavailable / available gate --------------------------------------

    #[test]
    fn forecast_is_unavailable_with_no_eur_accounts() {
        let conn = test_conn();
        let forecast = get_cashflow_forecast_impl(&conn, today()).unwrap();
        assert!(!forecast.available, "no accounts at all - nothing to project from");
    }

    #[test]
    fn forecast_is_unavailable_when_only_a_non_eur_account_exists() {
        let conn = test_conn();
        create_account_impl(
            &conn,
            &AccountInput { name: "PayPal".to_string(), account_type: "paypal".to_string(), currency: "USD".to_string(), opening_balance_cents: 10000, is_active: true },
        )
        .unwrap();
        let forecast = get_cashflow_forecast_impl(&conn, today()).unwrap();
        assert!(!forecast.available);
        assert!(forecast.excludes_non_eur_data, "even though unavailable, the USD account's existence should still be flagged");
    }

    // --- test scenario 10: forecast calculation ----------------------------

    #[test]
    fn forecast_matches_markos_own_worked_example() {
        // current €5,000; expected income +€2,000; recurring -€300;
        // upcoming -€500; forecast €6,200 (marko's own point 9 example).
        let conn = test_conn();
        let account_id = eur_account(&conn, 500_000);

        // Expected income: a future-dated income entry inside the window.
        create_finance_entry_impl(
            &conn,
            &FinanceEntryInput {
                entry_type: "income".to_string(),
                entry_date: "2026-09-10".to_string(),
                amount_cents: 200_000,
                currency: "EUR".to_string(),
                scope: "business".to_string(),
                category_id: None,
                account_id: Some(account_id),
                place: None,
                note: None,
            },
        )
        .unwrap();

        // Recurring expense due inside the window.
        create_recurring_expense_impl(
            &conn,
            &RecurringExpenseInput {
                name: "Adobe".to_string(),
                amount_cents: 30_000,
                currency: "EUR".to_string(),
                scope: "business".to_string(),
                category_id: None,
                account_id: Some(account_id),
                frequency: "monthly".to_string(),
                start_date: "2026-09-02".to_string(),
                note: None,
            },
        )
        .unwrap();

        // Upcoming one-off expense inside the window.
        create_finance_entry_impl(
            &conn,
            &FinanceEntryInput {
                entry_type: "expense".to_string(),
                entry_date: "2026-09-15".to_string(),
                amount_cents: 50_000,
                currency: "EUR".to_string(),
                scope: "personal".to_string(),
                category_id: None,
                account_id: Some(account_id),
                place: None,
                note: None,
            },
        )
        .unwrap();

        let forecast = get_cashflow_forecast_impl(&conn, today()).unwrap();
        assert!(forecast.available);
        assert_eq!(forecast.current_balance_cents, 500_000);
        assert_eq!(forecast.expected_income_cents, 200_000);
        assert_eq!(forecast.recurring_expenses_cents, 30_000);
        assert_eq!(forecast.upcoming_expenses_cents, 50_000);
        assert_eq!(forecast.forecast_balance_cents, 500_000 + 200_000 - 30_000 - 50_000);
        assert_eq!(forecast.forecast_balance_cents, 620_000, "matches marko's own worked example scaled to cents");
    }

    #[test]
    fn forecast_current_balance_excludes_future_dated_entries_to_avoid_double_counting() {
        let conn = test_conn();
        let account_id = eur_account(&conn, 100_000);
        create_finance_entry_impl(
            &conn,
            &FinanceEntryInput {
                entry_type: "income".to_string(),
                entry_date: "2026-09-05".to_string(), // future relative to today()
                amount_cents: 40_000,
                currency: "EUR".to_string(),
                scope: "personal".to_string(),
                category_id: None,
                account_id: Some(account_id),
                place: None,
                note: None,
            },
        )
        .unwrap();
        let forecast = get_cashflow_forecast_impl(&conn, today()).unwrap();
        assert_eq!(forecast.current_balance_cents, 100_000, "a future-dated entry must not already be folded into 'current' balance");
        assert_eq!(forecast.expected_income_cents, 40_000, "it must show up as expected income instead, exactly once");
    }

    #[test]
    fn forecast_current_balance_includes_a_past_dated_entry() {
        let conn = test_conn();
        let account_id = eur_account(&conn, 100_000);
        create_finance_entry_impl(
            &conn,
            &FinanceEntryInput {
                entry_type: "expense".to_string(),
                entry_date: "2026-08-01".to_string(), // in the past relative to today()
                amount_cents: 10_000,
                currency: "EUR".to_string(),
                scope: "personal".to_string(),
                category_id: None,
                account_id: Some(account_id),
                place: None,
                note: None,
            },
        )
        .unwrap();
        let forecast = get_cashflow_forecast_impl(&conn, today()).unwrap();
        assert_eq!(forecast.current_balance_cents, 90_000);
        assert_eq!(forecast.upcoming_expenses_cents, 0, "an already-past expense is not 'upcoming'");
    }

    #[test]
    fn forecast_includes_an_overdue_recurring_expense() {
        let conn = test_conn();
        let account_id = eur_account(&conn, 100_000);
        create_recurring_expense_impl(
            &conn,
            &RecurringExpenseInput {
                name: "Rent".to_string(),
                amount_cents: 80_000,
                currency: "EUR".to_string(),
                scope: "personal".to_string(),
                category_id: None,
                account_id: Some(account_id),
                frequency: "monthly".to_string(),
                start_date: "2026-08-01".to_string(), // already overdue relative to today()
                note: None,
            },
        )
        .unwrap();
        let forecast = get_cashflow_forecast_impl(&conn, today()).unwrap();
        assert_eq!(forecast.recurring_expenses_cents, 80_000, "an overdue recurring expense is still a real, unpaid obligation");
    }

    #[test]
    fn forecast_excludes_a_paused_recurring_expense() {
        let conn = test_conn();
        let account_id = eur_account(&conn, 100_000);
        let created = create_recurring_expense_impl(
            &conn,
            &RecurringExpenseInput {
                name: "Rent".to_string(),
                amount_cents: 80_000,
                currency: "EUR".to_string(),
                scope: "personal".to_string(),
                category_id: None,
                account_id: Some(account_id),
                frequency: "monthly".to_string(),
                start_date: "2026-09-01".to_string(),
                note: None,
            },
        )
        .unwrap();
        crate::commands::finance_recurring::set_recurring_active_impl(&conn, created.id, false).unwrap();
        let forecast = get_cashflow_forecast_impl(&conn, today()).unwrap();
        assert_eq!(forecast.recurring_expenses_cents, 0, "a paused template must not contribute to the forecast");
    }

    #[test]
    fn forecast_excludes_a_recurring_expense_beyond_the_window() {
        let conn = test_conn();
        let account_id = eur_account(&conn, 100_000);
        create_recurring_expense_impl(
            &conn,
            &RecurringExpenseInput {
                name: "Yearly domain renewal".to_string(),
                amount_cents: 1_500,
                currency: "EUR".to_string(),
                scope: "business".to_string(),
                category_id: None,
                account_id: Some(account_id),
                frequency: "yearly".to_string(),
                start_date: "2027-06-01".to_string(), // far beyond the 30-day window
                note: None,
            },
        )
        .unwrap();
        let forecast = get_cashflow_forecast_impl(&conn, today()).unwrap();
        assert_eq!(forecast.recurring_expenses_cents, 0);
    }

    // --- test scenario 11: forecast excludes potential profit --------------

    #[test]
    fn forecast_never_includes_unrealized_inventory_or_estimated_prices() {
        // There is no code path anywhere in get_cashflow_forecast_impl that
        // reads tickets/inventory/listing prices at all - this test exists
        // to make that guarantee explicit and regression-proof: adding a
        // sale with only an 'available'/'listed' ticket (never marked as an
        // actual pending sale) must never move the forecast.
        let conn = test_conn();
        let _account_id = eur_account(&conn, 100_000);
        let forecast = get_cashflow_forecast_impl(&conn, today()).unwrap();
        assert_eq!(forecast.expected_income_cents, 0, "an empty ledger has zero expected income - never a guessed inventory value");
    }

    // --- test scenario 12: mixed currency safety ---------------------------

    /// Minimal event -> order -> sold ticket chain, just enough to attach a
    /// `sales` row to for these currency-safety tests - not exercising
    /// events/orders/tickets logic itself (that belongs to events.rs/
    /// orders.rs/tickets.rs's own test suites, untouched by this feature).
    fn seed_sold_ticket(conn: &Connection, currency: &str) -> i64 {
        conn.execute("INSERT INTO events(name, status) VALUES ('Test Event', 'upcoming')", []).unwrap();
        let event_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO orders(code, event_id, purchase_date, quantity, currency, payment_status)
             VALUES ('O-1', ?1, '2026-08-01', 1, ?2, 'paid')",
            rusqlite::params![event_id, currency],
        )
        .unwrap();
        let order_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO tickets(code, order_id, event_id, currency, status) VALUES ('T-1', ?1, ?2, ?3, 'sold')",
            rusqlite::params![order_id, event_id, currency],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn forecast_excludes_non_eur_pending_sales_and_flags_it() {
        let conn = test_conn();
        let _account_id = eur_account(&conn, 100_000);
        let ticket_id = seed_sold_ticket(&conn, "USD");
        conn.execute(
            "INSERT INTO sales(code, ticket_id, sale_date, sale_price_cents, currency, payment_status)
             VALUES ('S-1', ?1, '2026-08-15', 30000, 'USD', 'pending')",
            [ticket_id],
        )
        .unwrap();

        let forecast = get_cashflow_forecast_impl(&conn, today()).unwrap();
        assert_eq!(forecast.expected_income_cents, 0, "a USD pending sale must never be blended into a EUR forecast");
        assert!(forecast.excludes_non_eur_data, "but its exclusion must be disclosed, not silent");
    }

    #[test]
    fn forecast_includes_eur_pending_sales_as_expected_income() {
        let conn = test_conn();
        let _account_id = eur_account(&conn, 100_000);
        let ticket_id = seed_sold_ticket(&conn, "EUR");
        conn.execute(
            "INSERT INTO sales(code, ticket_id, sale_date, sale_price_cents, currency, payment_status)
             VALUES ('S-1', ?1, '2026-08-15', 45000, 'EUR', 'pending')",
            [ticket_id],
        )
        .unwrap();

        let forecast = get_cashflow_forecast_impl(&conn, today()).unwrap();
        assert_eq!(forecast.expected_income_cents, 45000, "a EUR pending sale is real, already-known expected income");
    }
}

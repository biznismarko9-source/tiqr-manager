//! 2.73.0 - "Check my data", marko's own pick after a week in which two real
//! problems were only ever found by someone querying his database by hand:
//!
//!  - an order whose currency was stored as `US $` rather than `USD`, which
//!    is why "Convert to EUR" answered `404 not found` for days (2.71.0), and
//!  - three orders at 0.00 with six tickets on them, which silently make
//!    every profit figure on those tickets a 100% margin.
//!
//! Neither is reachable from inside the app today. This module is those
//! queries, turned into something he can run himself.
//!
//! ## What this does NOT do
//!
//! It never changes anything. Every check is a `SELECT`, and the result is a
//! list of things to look at - not a list of things that were done. Fixing
//! any of them means deciding what the right value IS, and that is marko's
//! call about his own money, not a rule a background job should apply. So
//! there is no "fix all" button and deliberately no attempt at one.
//!
//! A check that finds nothing is still returned, with `count: 0`. "I looked
//! and there is nothing wrong" is the answer he needs most often, and a panel
//! that renders nothing cannot give it.

use crate::db::AppState;
use crate::error::AppResult;
use rusqlite::Connection;
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataCheck {
    /// Stable identifier, so the frontend can key on it without matching text.
    pub id: String,
    pub title: String,
    /// What it means and what to do about it - written for marko, not for a
    /// developer reading a stack trace.
    pub detail: String,
    /// "warn" when it affects money or stops something working; "info" when
    /// it is an inconsistency worth knowing about but nothing is broken.
    pub severity: String,
    pub count: i64,
    /// Up to five record codes, so a hit is findable rather than just counted.
    pub examples: Vec<String>,
}

/// Runs one check: a count query and an examples query over the same
/// condition. Kept as one function so a check cannot drift into counting one
/// thing and listing another.
fn run_check(
    conn: &Connection,
    id: &str,
    title: &str,
    detail: &str,
    severity: &str,
    count_sql: &str,
    examples_sql: &str,
) -> AppResult<DataCheck> {
    let count: i64 = conn.query_row(count_sql, [], |r| r.get(0))?;
    let examples: Vec<String> = if count > 0 {
        let mut stmt = conn.prepare(examples_sql)?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>, _>>()?
    } else {
        Vec::new()
    };
    Ok(DataCheck {
        id: id.to_string(),
        title: title.to_string(),
        detail: detail.to_string(),
        severity: severity.to_string(),
        count,
        examples,
    })
}

/// Every check, in the order they are shown. Money-affecting ones first, same
/// principle as `TICKET_RESTRICTIONS`' own ordering.
pub(crate) fn check_data_health_impl(conn: &Connection) -> AppResult<Vec<DataCheck>> {
    // A currency the rate service can look up is exactly three A-Z letters.
    // Written as a GLOB rather than a regex because SQLite has no REGEXP
    // without a user function, and `[A-Z]` in GLOB is a real character class.
    const CUR_OK: &str = "GLOB '[A-Z][A-Z][A-Z]'";

    let checks = vec![
        run_check(
            conn,
            "currency_not_a_code",
            "Currency that isn't a real code",
            "The exchange-rate service only knows three-letter codes like EUR, USD or GBP. \
             Anything else makes \"Convert\" fail, and it usually came from an AI import reading \
             a symbol off a screenshot. Open the record and set the currency properly.",
            "warn",
            &format!(
                "SELECT (SELECT COUNT(*) FROM orders WHERE currency NOT {CUR_OK}) \
                      + (SELECT COUNT(*) FROM tickets WHERE currency NOT {CUR_OK}) \
                      + (SELECT COUNT(*) FROM sales WHERE currency NOT {CUR_OK})"
            ),
            &format!(
                "SELECT code || ' (' || currency || ')' FROM orders WHERE currency NOT {CUR_OK} \
                 UNION ALL SELECT code || ' (' || currency || ')' FROM sales WHERE currency NOT {CUR_OK} \
                 LIMIT 5"
            ),
        )?,
        run_check(
            conn,
            "paid_order_costing_nothing",
            "Paid order with no cost on it",
            "The order is marked paid but every amount on it is 0.00. If the tickets really were \
             free, nothing is wrong. If the price just never got typed in, every profit figure on \
             those tickets is wrong in your favour and will keep being wrong.",
            "warn",
            "SELECT COUNT(*) FROM orders WHERE payment_status = 'paid' AND total_cost_cents = 0",
            "SELECT code FROM orders WHERE payment_status = 'paid' AND total_cost_cents = 0 LIMIT 5",
        )?,
        run_check(
            conn,
            "ticket_currency_differs",
            "Ticket in a different currency than its order",
            "Converting an order refuses to run while this is true, because splitting one cost \
             across tickets in two currencies would produce numbers nobody can defend.",
            "warn",
            "SELECT COUNT(*) FROM tickets t JOIN orders o ON o.id = t.order_id WHERE t.currency <> o.currency",
            "SELECT t.code FROM tickets t JOIN orders o ON o.id = t.order_id WHERE t.currency <> o.currency LIMIT 5",
        )?,
        run_check(
            conn,
            "negative_money",
            "A negative amount",
            "No price, fee or cost in this app should ever be below zero. One that is will flow \
             into totals and quietly bend them.",
            "warn",
            "SELECT (SELECT COUNT(*) FROM orders WHERE total_cost_cents < 0 OR unit_price_cents < 0 OR fees_cents < 0) \
                  + (SELECT COUNT(*) FROM tickets WHERE purchase_cost_cents < 0) \
                  + (SELECT COUNT(*) FROM sales WHERE sale_price_cents < 0)",
            "SELECT code FROM orders WHERE total_cost_cents < 0 OR unit_price_cents < 0 OR fees_cents < 0 \
             UNION ALL SELECT code FROM sales WHERE sale_price_cents < 0 LIMIT 5",
        )?,
        run_check(
            conn,
            "sale_without_ticket",
            "Sale pointing at a ticket that is gone",
            "The sale survived but the ticket it was for did not. Revenue from it still counts; \
             the cost behind it cannot be found.",
            "warn",
            "SELECT COUNT(*) FROM sales s LEFT JOIN tickets t ON t.id = s.ticket_id WHERE t.id IS NULL",
            "SELECT s.code FROM sales s LEFT JOIN tickets t ON t.id = s.ticket_id WHERE t.id IS NULL LIMIT 5",
        )?,
        run_check(
            conn,
            "sold_without_a_sale",
            "Ticket marked sold with no sale behind it",
            "Inventory says it is gone, but there is no sale recording who bought it or for how \
             much. Usually a sale that was deleted without the ticket being put back.",
            "warn",
            "SELECT COUNT(*) FROM tickets t WHERE t.status = 'sold' \
               AND NOT EXISTS (SELECT 1 FROM sales s WHERE s.ticket_id = t.id AND s.refunded_at IS NULL)",
            "SELECT t.code FROM tickets t WHERE t.status = 'sold' \
               AND NOT EXISTS (SELECT 1 FROM sales s WHERE s.ticket_id = t.id AND s.refunded_at IS NULL) LIMIT 5",
        )?,
        run_check(
            conn,
            "unsold_with_a_sale",
            "Ticket still for sale although it has been sold",
            "The mirror of the one above: an active sale exists, but the ticket is still listed as \
             available. It can be sold a second time from here.",
            "warn",
            "SELECT COUNT(*) FROM tickets t JOIN sales s ON s.ticket_id = t.id \
              WHERE s.refunded_at IS NULL AND t.status <> 'sold'",
            "SELECT t.code FROM tickets t JOIN sales s ON s.ticket_id = t.id \
              WHERE s.refunded_at IS NULL AND t.status <> 'sold' LIMIT 5",
        )?,
        run_check(
            conn,
            "quantity_mismatch",
            "Order whose quantity doesn't match its tickets",
            "The order says one number, the tickets it generated say another. Per-ticket cost is \
             the order's cost split across them, so this makes that split wrong.",
            "info",
            "SELECT COUNT(*) FROM (SELECT o.id FROM orders o LEFT JOIN tickets t ON t.order_id = o.id \
               GROUP BY o.id HAVING o.quantity <> COUNT(t.id))",
            "SELECT code FROM (SELECT o.code AS code FROM orders o LEFT JOIN tickets t ON t.order_id = o.id \
               GROUP BY o.id HAVING o.quantity <> COUNT(t.id)) LIMIT 5",
        )?,
        run_check(
            conn,
            "duplicate_codes",
            "The same code used twice",
            "Codes are how you find a record and how the Google Sheet matches one. Two records \
             sharing one is ambiguous everywhere it is read.",
            "info",
            "SELECT COUNT(*) FROM (SELECT code FROM orders GROUP BY code HAVING COUNT(*) > 1 \
               UNION ALL SELECT code FROM tickets GROUP BY code HAVING COUNT(*) > 1 \
               UNION ALL SELECT code FROM sales GROUP BY code HAVING COUNT(*) > 1)",
            "SELECT code FROM (SELECT code FROM orders GROUP BY code HAVING COUNT(*) > 1 \
               UNION ALL SELECT code FROM tickets GROUP BY code HAVING COUNT(*) > 1 \
               UNION ALL SELECT code FROM sales GROUP BY code HAVING COUNT(*) > 1) LIMIT 5",
        )?,
    ];
    Ok(checks)
}

#[tauri::command]
pub fn check_data_health(state: State<AppState>) -> AppResult<Vec<DataCheck>> {
    let conn = state.db.lock().unwrap();
    check_data_health_impl(&conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    // The shared helper, not a private copy: it enables foreign keys, which an
    // in-memory connection otherwise leaves off, so these tests exercise the
    // same cascade behaviour production does.
    use crate::db::test_conn;

    #[test]
    fn every_check_runs_against_a_real_schema_and_an_empty_database() {
        // The point of this test is the SQL itself. Every statement here is
        // hand-written against table and column names that could be renamed
        // by a migration, and a typo in one of them would otherwise only be
        // discovered by marko pressing the button.
        let conn = test_conn();
        let checks = check_data_health_impl(&conn).unwrap();
        assert_eq!(checks.len(), 9);
        for c in &checks {
            assert_eq!(c.count, 0, "empty database, but {} counted {}", c.id, c.count);
            assert!(c.examples.is_empty());
            assert!(!c.title.is_empty() && !c.detail.is_empty());
        }
    }

    #[test]
    fn a_currency_that_is_not_three_letters_is_caught() {
        // The exact shape that broke "Convert to EUR" for marko: an AI import
        // storing "US $" off a screenshot.
        let conn = test_conn();
        conn.execute("INSERT INTO events (name) VALUES ('Test')", []).unwrap();
        conn.execute(
            "INSERT INTO orders (code, event_id, purchase_date, quantity, unit_price_cents, \
             fees_cents, other_costs_cents, total_cost_cents, currency, payment_status) \
             VALUES ('T-001', 1, '2026-01-01', 1, 100, 0, 0, 100, 'US $', 'paid')",
            [],
        )
        .unwrap();
        let checks = check_data_health_impl(&conn).unwrap();
        let hit = checks.iter().find(|c| c.id == "currency_not_a_code").unwrap();
        assert_eq!(hit.count, 1);
        assert_eq!(hit.examples, vec!["T-001 (US $)".to_string()]);

        // And a real code is NOT flagged.
        conn.execute("UPDATE orders SET currency = 'USD'", []).unwrap();
        let checks = check_data_health_impl(&conn).unwrap();
        assert_eq!(checks.iter().find(|c| c.id == "currency_not_a_code").unwrap().count, 0);
    }

    #[test]
    fn a_paid_order_costing_nothing_is_caught() {
        let conn = test_conn();
        conn.execute("INSERT INTO events (name) VALUES ('Test')", []).unwrap();
        conn.execute(
            "INSERT INTO orders (code, event_id, purchase_date, quantity, unit_price_cents, \
             fees_cents, other_costs_cents, total_cost_cents, currency, payment_status) \
             VALUES ('NFL-001', 1, '2026-01-01', 2, 0, 0, 0, 0, 'EUR', 'paid')",
            [],
        )
        .unwrap();
        let checks = check_data_health_impl(&conn).unwrap();
        let hit = checks.iter().find(|c| c.id == "paid_order_costing_nothing").unwrap();
        assert_eq!(hit.count, 1);
        assert_eq!(hit.examples, vec!["NFL-001".to_string()]);
    }
}

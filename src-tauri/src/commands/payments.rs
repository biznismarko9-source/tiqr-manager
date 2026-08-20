//! Payments 2.0 (2.0.0): a real payment ledger for Sales and Orders.
//!
//! One Sale GROUP (see `sales::GROUP_KEY_EXPR`/`GROUP_BASE_SELECT` - a
//! group is `COALESCE(batch_id, 'single:'||id)`, NOT a single `sales.id`)
//! or one Order can have many `payments` rows; never the reverse. This
//! module owns:
//!   - CRUD on individual payments (create/update/delete/list)
//!   - deriving Paid/Outstanding/Status from the ledger instead of trusting
//!     `sales.payment_status`/`orders.payment_status` as free-standing truth
//!   - the "Mark as Paid/Pending" shortcut that Sale Detail's bulk action
//!     and Order Edit's Payment status field still offer (marko: keep them,
//!     but make them create/remove a real ledger entry instead of just
//!     flipping a label) - called from sales.rs/orders.rs, not exposed as
//!     its own Tauri command.
//!
//! Neither `sales.payment_status` nor `orders.payment_status` is removed or
//! reinterpreted by this module - they still exist, untouched, and nothing
//! here writes to them. Status shown to the user is computed fresh from
//! `payments` every time, never cached back onto those columns.

use crate::commands::sales::{GROUP_BASE_SELECT, GROUP_KEY_EXPR};
use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::models::{Payment, PaymentInput, PaymentSummary};
use chrono::Local;
use rusqlite::{params, Connection, Row};
use tauri::State;

const VALID_METHODS: &[&str] = &["bank_transfer", "card", "revolut", "cash", "paypal", "other"];

fn map_payment(row: &Row) -> rusqlite::Result<Payment> {
    let is_shortcut: i64 = row.get("is_shortcut")?;
    let is_demo: i64 = row.get("is_demo")?;
    Ok(Payment {
        id: row.get("id")?,
        code: row.get("code")?,
        amount_cents: row.get("amount_cents")?,
        currency: row.get("currency")?,
        payment_date: row.get("payment_date")?,
        method: row.get("method")?,
        method_other_note: row.get("method_other_note")?,
        reference: row.get("reference")?,
        is_shortcut: is_shortcut != 0,
        is_demo: is_demo != 0,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn fetch_one_impl(conn: &Connection, id: i64) -> AppResult<Payment> {
    conn.query_row("SELECT * FROM payments WHERE id = ?1", [id], map_payment)
        .map_err(|_| AppError::NotFound(format!("Payment #{id} not found")))
}

/// What one payment (or a whole batch being applied via the shortcut)
/// belongs to. Mirrors the DB's own CHECK constraint - `resolve_target`
/// below is the single place that enforces "exactly one, never both/neither"
/// at the application level, which is the primary defense; the DB CHECK is
/// a safety net behind it, not the main validation path.
pub(crate) enum PaymentTarget {
    SaleGroup(String),
    Order(i64),
}

fn resolve_target(input: &PaymentInput) -> AppResult<PaymentTarget> {
    match (&input.sale_group_key, input.order_id) {
        (Some(key), None) if !key.trim().is_empty() => Ok(PaymentTarget::SaleGroup(key.clone())),
        (None, Some(order_id)) => Ok(PaymentTarget::Order(order_id)),
        (None, None) => Err(AppError::Validation(
            "A payment needs either a sale or an order to belong to.".into(),
        )),
        _ => Err(AppError::Validation(
            "A payment can belong to a sale OR an order, never both.".into(),
        )),
    }
}

fn validate_method(method: &str, other_note: &Option<String>) -> AppResult<()> {
    if !VALID_METHODS.contains(&method) {
        return Err(AppError::Validation(format!(
            "'{method}' isn't a recognized payment method."
        )));
    }
    if method == "other" && other_note.as_deref().unwrap_or("").trim().is_empty() {
        return Err(AppError::Validation(
            "Describe the payment method when using \"Other\".".into(),
        ));
    }
    Ok(())
}

/// The state a sale group is in *before* considering payments at all: its
/// total (already excludes refunded lines, same as everywhere else in the
/// app - see GROUP_BASE_SELECT), the currency that total is in (None when
/// the group's own non-refunded lines already span more than one currency),
/// and whether the whole group is refunded (every line in it, not just
/// some). None (the outer Option) means the key doesn't resolve to any
/// sales row at all.
struct SaleGroupState {
    total_cents: i64,
    currency: Option<String>,
    fully_refunded: bool,
}

fn sale_group_state(conn: &Connection, key: &str) -> AppResult<Option<SaleGroupState>> {
    let sql = format!("{GROUP_BASE_SELECT} WHERE {GROUP_KEY_EXPR} = ?1 GROUP BY {GROUP_KEY_EXPR}");
    let row: Option<(i64, i64, i64, Option<String>)> = conn
        .query_row(&sql, [key], |r| {
            Ok((
                r.get("ticket_count")?,
                r.get("refunded_count")?,
                r.get("revenue_cents")?,
                r.get("currency")?,
            ))
        })
        .ok();
    Ok(row.map(|(ticket_count, refunded_count, revenue_cents, currency)| SaleGroupState {
        total_cents: revenue_cents,
        currency,
        fully_refunded: refunded_count >= ticket_count && ticket_count > 0,
    }))
}

fn order_total_and_currency(conn: &Connection, order_id: i64) -> AppResult<(i64, String)> {
    conn.query_row(
        "SELECT total_cost_cents, currency FROM orders WHERE id = ?1",
        [order_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .map_err(|_| AppError::NotFound(format!("Order #{order_id} not found")))
}

fn payments_for_sale_group_impl(conn: &Connection, key: &str) -> AppResult<Vec<Payment>> {
    let mut stmt = conn.prepare("SELECT * FROM payments WHERE sale_group_key = ?1 ORDER BY payment_date, id")?;
    let rows = stmt.query_map([key], map_payment)?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn payments_for_order_impl(conn: &Connection, order_id: i64) -> AppResult<Vec<Payment>> {
    let mut stmt = conn.prepare("SELECT * FROM payments WHERE order_id = ?1 ORDER BY payment_date, id")?;
    let rows = stmt.query_map([order_id], map_payment)?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Sums `payments` in the same "never blend, None when mixed" style used
/// everywhere else in this app. Returns (received_cents, currency) - both
/// None only when `payments` is empty (nothing received yet is shown as
/// `Some(0)` with the target's own currency, not as Mixed/unknown).
fn sum_payments(payments: &[Payment]) -> (Option<i64>, Option<String>) {
    if payments.is_empty() {
        return (Some(0), None);
    }
    let currencies: std::collections::HashSet<&str> = payments.iter().map(|p| p.currency.as_str()).collect();
    if currencies.len() == 1 {
        let total: i64 = payments.iter().map(|p| p.amount_cents).sum();
        (Some(total), Some(currencies.into_iter().next().unwrap().to_string()))
    } else {
        (None, None)
    }
}

pub(crate) fn compute_payment_summary_for_sale_group_impl(
    conn: &Connection,
    key: &str,
) -> AppResult<PaymentSummary> {
    let state = sale_group_state(conn, key)?
        .ok_or_else(|| AppError::NotFound(format!("Sale group '{key}' not found")))?;
    let payments = payments_for_sale_group_impl(conn, key)?;

    if state.fully_refunded {
        // Realized-only rule: a fully refunded group is never "owed" or
        // "paid" - it's just Refunded, regardless of what the ledger says.
        // Payments recorded before the refund are NOT deleted (history stays
        // auditable, per marko's explicit instruction) - they just stop
        // driving the status shown.
        return Ok(PaymentSummary {
            total_cents: Some(0),
            total_currency: state.currency,
            received_cents: None,
            outstanding_cents: None,
            currency: None,
            status: "refunded".into(),
            payments,
        });
    }

    let Some(total_currency) = state.currency else {
        // The group's own (non-refunded) lines already span more than one
        // currency, independent of payments - there's no single total to
        // measure payments against, so the whole summary reports Mixed
        // rather than pretending a partial answer is meaningful.
        return Ok(PaymentSummary {
            total_cents: None,
            total_currency: None,
            received_cents: None,
            outstanding_cents: None,
            currency: None,
            status: "mixed".into(),
            payments,
        });
    };

    let (received_cents, received_currency) = sum_payments(&payments);
    let status = derive_status(state.total_cents, received_cents, &received_currency, &total_currency);
    let outstanding_cents = match received_cents {
        Some(r) if received_currency.as_deref() == Some(&total_currency) || payments.is_empty() => {
            Some((state.total_cents - r).max(0))
        }
        _ => None,
    };
    let currency = if outstanding_cents.is_some() { Some(total_currency.clone()) } else { None };

    Ok(PaymentSummary {
        total_cents: Some(state.total_cents),
        total_currency: Some(total_currency),
        received_cents: if currency.is_some() { received_cents } else { None },
        outstanding_cents,
        currency,
        status,
        payments,
    })
}

pub(crate) fn compute_payment_summary_for_order_impl(conn: &Connection, order_id: i64) -> AppResult<PaymentSummary> {
    let (total_cents, total_currency) = order_total_and_currency(conn, order_id)?;
    let payments = payments_for_order_impl(conn, order_id)?;
    let (received_cents, received_currency) = sum_payments(&payments);
    let status = derive_status(total_cents, received_cents, &received_currency, &total_currency);
    let outstanding_cents = match received_cents {
        Some(r) if received_currency.as_deref() == Some(&total_currency) || payments.is_empty() => {
            Some((total_cents - r).max(0))
        }
        _ => None,
    };
    let currency = if outstanding_cents.is_some() { Some(total_currency.clone()) } else { None };

    Ok(PaymentSummary {
        total_cents: Some(total_cents),
        total_currency: Some(total_currency),
        received_cents: if currency.is_some() { received_cents } else { None },
        outstanding_cents,
        currency,
        status,
        payments,
    })
}

/// Pending / Partial / Paid from a total vs. what's been received - Refunded
/// and Mixed (the group's OWN lines, not payments) are decided by the
/// caller before this runs. `received_currency` differing from
/// `total_currency` (payments recorded in a different currency than the
/// sale/order itself) is treated the same as "can't compare" - reported as
/// partial-if-anything-received-else-pending would be a guess, so this
/// falls back to whichever side has real information: pending if nothing's
/// been recorded, otherwise the caller's outstanding/currency already goes
/// None and the frontend shows Mixed rather than a specific status here
/// mattering much - "partial" is a safe, honest default label for "some
/// money came in, but not in a currency we can net against the total".
fn derive_status(total_cents: i64, received_cents: Option<i64>, received_currency: &Option<String>, total_currency: &str) -> String {
    match received_cents {
        Some(0) => "pending".into(),
        Some(r) => {
            if received_currency.as_deref() == Some(total_currency) {
                if r >= total_cents { "paid".into() } else { "partial".into() }
            } else {
                "partial".into()
            }
        }
        None => "partial".into(), // payments exist but span mixed currencies among themselves
    }
}

fn outstanding_for_overpayment_check(
    conn: &Connection,
    target: &PaymentTarget,
    currency: &str,
    excluding_payment_id: Option<i64>,
) -> AppResult<Option<i64>> {
    let (total_cents, total_currency, mut payments) = match target {
        PaymentTarget::SaleGroup(key) => {
            let state = sale_group_state(conn, key)?
                .ok_or_else(|| AppError::NotFound(format!("Sale group '{key}' not found")))?;
            if state.fully_refunded {
                return Err(AppError::Validation(
                    "This sale has been refunded - it can't take new payments.".into(),
                ));
            }
            let Some(total_currency) = state.currency else {
                // Mixed-currency group: no single total to check an
                // overpayment against, so overpayment protection can't run
                // here - allowed through rather than blocked on a check
                // that can't be computed (still capped at nothing negative
                // by the amount_cents > 0 CHECK on the table itself).
                return Ok(None);
            };
            (state.total_cents, total_currency, payments_for_sale_group_impl(conn, key)?)
        }
        PaymentTarget::Order(order_id) => {
            let (total_cents, total_currency) = order_total_and_currency(conn, *order_id)?;
            (total_cents, total_currency, payments_for_order_impl(conn, *order_id)?)
        }
    };
    if let Some(id) = excluding_payment_id {
        payments.retain(|p| p.id != id);
    }
    if total_currency != currency {
        // The new/edited payment isn't even in the total's own currency -
        // same "can't net across currencies" reasoning as derive_status;
        // don't block it, there's nothing sound to compare it against.
        return Ok(None);
    }
    let (received_cents, received_currency) = sum_payments(&payments);
    match (received_cents, received_currency) {
        (Some(r), rc) if rc.is_none() || rc.as_deref() == Some(currency) => {
            Ok(Some((total_cents - r).max(0)))
        }
        _ => Ok(None),
    }
}

pub(crate) fn create_payment_impl(conn: &Connection, input: &PaymentInput, is_shortcut: bool, is_demo: bool) -> AppResult<Payment> {
    if input.amount_cents <= 0 {
        return Err(AppError::Validation("Payment amount must be greater than zero.".into()));
    }
    if input.currency.trim().is_empty() {
        return Err(AppError::Validation("Payment currency is required.".into()));
    }
    if input.payment_date.trim().is_empty() {
        return Err(AppError::Validation("Payment date is required.".into()));
    }
    validate_method(&input.method, &input.method_other_note)?;
    let target = resolve_target(input)?;

    if let Some(outstanding) = outstanding_for_overpayment_check(conn, &target, &input.currency, None)? {
        if input.amount_cents > outstanding {
            return Err(AppError::Validation(format!(
                "This payment ({} {}) would exceed what's still outstanding ({} {}). Reduce the amount, or edit/delete an existing payment first.",
                crate::money::format_cents(input.amount_cents),
                input.currency,
                crate::money::format_cents(outstanding),
                input.currency,
            )));
        }
    }

    let code = crate::codes::next_code(conn, "payment", "PAY")?;
    let (sale_group_key, order_id) = match &target {
        PaymentTarget::SaleGroup(key) => (Some(key.as_str()), None),
        PaymentTarget::Order(id) => (None, Some(*id)),
    };
    conn.execute(
        "INSERT INTO payments (code, sale_group_key, order_id, amount_cents, currency, payment_date,
           method, method_other_note, reference, is_shortcut, is_demo)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
        params![
            code,
            sale_group_key,
            order_id,
            input.amount_cents,
            input.currency.trim(),
            input.payment_date,
            input.method,
            if input.method == "other" { input.method_other_note.as_deref().map(str::trim) } else { None },
            input.reference.as_deref().map(str::trim),
            is_shortcut as i64,
            is_demo as i64,
        ],
    )?;
    let id = conn.last_insert_rowid();
    fetch_one_impl(conn, id)
}

pub(crate) fn update_payment_impl(conn: &Connection, id: i64, input: &PaymentInput) -> AppResult<Payment> {
    if input.amount_cents <= 0 {
        return Err(AppError::Validation("Payment amount must be greater than zero.".into()));
    }
    validate_method(&input.method, &input.method_other_note)?;
    // resolve_target runs so a malformed edit request (both or neither of
    // sale_group_key/order_id set) is rejected the same way create rejects
    // it - but the target actually used below comes from the EXISTING row
    // (target_for_existing), not from `input`: editing only ever changes
    // amount/currency/date/method/reference, it never re-parents a payment
    // to a different sale or order.
    resolve_target(input)?;

    if let Some(outstanding) = outstanding_for_overpayment_check(
        conn,
        &target_for_existing(conn, id)?,
        &input.currency,
        Some(id),
    )? {
        if input.amount_cents > outstanding {
            return Err(AppError::Validation(format!(
                "This payment ({} {}) would exceed what's still outstanding ({} {}). Reduce the amount, or edit/delete another payment first.",
                crate::money::format_cents(input.amount_cents),
                input.currency,
                crate::money::format_cents(outstanding),
                input.currency,
            )));
        }
    }

    conn.execute(
        "UPDATE payments SET amount_cents=?1, currency=?2, payment_date=?3, method=?4,
           method_other_note=?5, reference=?6, updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now')
         WHERE id=?7",
        params![
            input.amount_cents,
            input.currency.trim(),
            input.payment_date,
            input.method,
            if input.method == "other" { input.method_other_note.as_deref().map(str::trim) } else { None },
            input.reference.as_deref().map(str::trim),
            id,
        ],
    )?;
    fetch_one_impl(conn, id)
}

fn target_for_existing(conn: &Connection, id: i64) -> AppResult<PaymentTarget> {
    let (sale_group_key, order_id): (Option<String>, Option<i64>) = conn
        .query_row(
            "SELECT sale_group_key, order_id FROM payments WHERE id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| AppError::NotFound(format!("Payment #{id} not found")))?;
    match (sale_group_key, order_id) {
        (Some(key), None) => Ok(PaymentTarget::SaleGroup(key)),
        (None, Some(id)) => Ok(PaymentTarget::Order(id)),
        _ => Err(AppError::Db(format!("Payment #{id} has an invalid target - this should be unreachable."))),
    }
}

pub(crate) fn delete_payment_impl(conn: &Connection, id: i64) -> AppResult<()> {
    let affected = conn.execute("DELETE FROM payments WHERE id = ?1", [id])?;
    if affected == 0 {
        return Err(AppError::NotFound(format!("Payment #{id} not found")));
    }
    Ok(())
}

// ---------------------------------------------------------------------
// "Mark as Paid/Pending" shortcut - called from sales.rs/orders.rs, not a
// Tauri command of its own. See the module doc comment above for why this
// exists instead of removing the old bulk action.
// ---------------------------------------------------------------------

/// Records one shortcut payment for the given amount against a sale group.
/// Rejects (rather than silently clamping) if the amount would overpay -
/// same rule as a manually-added payment - so "Mark as Paid" on a group
/// that already has more recorded than its total (e.g. via an earlier
/// manual payment) fails with a clear message instead of writing a
/// negative-outstanding payment.
pub(crate) fn apply_paid_shortcut_for_sale_group_impl(
    conn: &Connection,
    sale_group_key: &str,
    amount_cents: i64,
    currency: &str,
) -> AppResult<()> {
    if amount_cents <= 0 {
        return Ok(()); // nothing to record - a zero-value line has nothing to mark paid
    }
    let input = PaymentInput {
        sale_group_key: Some(sale_group_key.to_string()),
        order_id: None,
        amount_cents,
        currency: currency.to_string(),
        payment_date: Local::now().date_naive().to_string(),
        method: "other".into(),
        method_other_note: None,
        reference: Some("Recorded via the Mark as Paid shortcut".into()),
    };
    create_payment_impl(conn, &input, true, false)?;
    Ok(())
}

/// Reverses the shortcut for a sale group: deletes ONLY payments this same
/// shortcut created (is_shortcut = 1), never a manually-entered one. If any
/// non-shortcut payment exists for this group, refuses outright rather than
/// guessing which rows are "safe" to remove - the group's real payment
/// history is never touched by this action.
pub(crate) fn revert_paid_shortcut_for_sale_group_impl(conn: &Connection, sale_group_key: &str) -> AppResult<()> {
    let real_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM payments WHERE sale_group_key = ?1 AND is_shortcut = 0",
        [sale_group_key],
        |r| r.get(0),
    )?;
    if real_count > 0 {
        return Err(AppError::Validation(
            "This sale already has real payment history beyond the quick-paid shortcut - edit or delete the specific payment(s) in its Payments history instead.".into(),
        ));
    }
    conn.execute("DELETE FROM payments WHERE sale_group_key = ?1 AND is_shortcut = 1", [sale_group_key])?;
    Ok(())
}

pub(crate) fn apply_paid_shortcut_for_order_impl(conn: &Connection, order_id: i64, amount_cents: i64, currency: &str) -> AppResult<()> {
    if amount_cents <= 0 {
        return Ok(());
    }
    let input = PaymentInput {
        sale_group_key: None,
        order_id: Some(order_id),
        amount_cents,
        currency: currency.to_string(),
        payment_date: Local::now().date_naive().to_string(),
        method: "other".into(),
        method_other_note: None,
        reference: Some("Recorded via the Mark as Paid shortcut".into()),
    };
    create_payment_impl(conn, &input, true, false)?;
    Ok(())
}

pub(crate) fn revert_paid_shortcut_for_order_impl(conn: &Connection, order_id: i64) -> AppResult<()> {
    let real_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM payments WHERE order_id = ?1 AND is_shortcut = 0",
        [order_id],
        |r| r.get(0),
    )?;
    if real_count > 0 {
        return Err(AppError::Validation(
            "This order already has real payment history beyond the quick-paid shortcut - edit or delete the specific payment(s) in its Payments history instead.".into(),
        ));
    }
    conn.execute("DELETE FROM payments WHERE order_id = ?1 AND is_shortcut = 1", [order_id])?;
    Ok(())
}

// ---------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------

#[tauri::command]
pub fn get_payment_summary_for_sale(state: State<AppState>, sale_group_key: String) -> AppResult<PaymentSummary> {
    let conn = state.db.lock().unwrap();
    compute_payment_summary_for_sale_group_impl(&conn, &sale_group_key)
}

#[tauri::command]
pub fn get_payment_summary_for_order(state: State<AppState>, order_id: i64) -> AppResult<PaymentSummary> {
    let conn = state.db.lock().unwrap();
    compute_payment_summary_for_order_impl(&conn, order_id)
}

#[tauri::command]
pub fn create_payment(state: State<AppState>, input: PaymentInput) -> AppResult<Payment> {
    let mut conn = state.db.lock().unwrap();
    let tx = conn.transaction()?;
    let payment = create_payment_impl(&tx, &input, false, false)?;
    tx.commit()?;
    Ok(payment)
}

#[tauri::command]
pub fn update_payment(state: State<AppState>, id: i64, input: PaymentInput) -> AppResult<Payment> {
    let mut conn = state.db.lock().unwrap();
    let tx = conn.transaction()?;
    let payment = update_payment_impl(&tx, id, &input)?;
    tx.commit()?;
    Ok(payment)
}

#[tauri::command]
pub fn delete_payment(state: State<AppState>, id: i64) -> AppResult<()> {
    let mut conn = state.db.lock().unwrap();
    let tx = conn.transaction()?;
    delete_payment_impl(&tx, id)?;
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::orders::{create_order_impl, update_order_impl, insert_order_with_tickets};
    use crate::commands::sales::{create_sale_impl, create_sales_batch_impl, refund_sale_impl};
    use crate::db::test_conn;
    use crate::models::{OrderEditInput, OrderInput, SaleBatchInput, SaleBatchLineInput, SaleInput};

    fn seed_event(conn: &Connection) -> i64 {
        conn.execute("INSERT INTO events (name, status) VALUES ('Test Event', 'upcoming')", [])
            .unwrap();
        conn.last_insert_rowid()
    }

    fn order_input(event_id: i64, qty: i64, unit_price_cents: i64, currency: &str, payment_status: Option<&str>) -> OrderInput {
        OrderInput {
            event_id,
            supplier_id: None,
            platform_id: None,
            purchase_date: "2026-01-01".into(),
            quantity: qty,
            unit_price_cents,
            fees_cents: 0,
            other_costs_cents: 0,
            currency: currency.to_string(),
            payment_status: payment_status.map(str::to_string),
            notes: None,
            ticket_type: None,
            section: None,
            row_label: None,
            seats: None,
        }
    }

    /// Creates an order (unpaid, so no payment side effects) with `qty`
    /// tickets and returns their ids.
    fn seed_tickets(conn: &Connection, event_id: i64, qty: i64, currency: &str) -> Vec<i64> {
        let order_id = insert_order_with_tickets(conn, &order_input(event_id, qty, 1000, currency, None), false).unwrap();
        let mut stmt = conn.prepare("SELECT id FROM tickets WHERE order_id = ?1 ORDER BY id").unwrap();
        stmt.query_map([order_id], |r| r.get::<_, i64>(0)).unwrap().collect::<Result<Vec<_>, _>>().unwrap()
    }

    /// A single (ungrouped) sale for one ticket, at `price_cents`, in
    /// whatever currency the ticket/order were seeded with. Returns
    /// (sale_id, sale_group_key).
    fn seed_single_sale(conn: &mut Connection, ticket_id: i64, price_cents: i64) -> (i64, String) {
        let sale_id = create_sale_impl(conn, &SaleInput {
            ticket_id,
            platform_id: None,
            sale_date: "2026-03-01".into(),
            sale_price_cents: price_cents,
            selling_fees_cents: 0,
            payment_status: Some("pending".into()),
            buyer_reference: None,
            notes: None,
        }).unwrap();
        (sale_id, format!("single:{sale_id}"))
    }

    /// A batch of sales (one "New sale" action covering several tickets),
    /// all at `price_cents` each. Returns (sale_ids, sale_group_key) - the
    /// group key is the first sale's own code, per migration 003.
    fn seed_batch_sale(conn: &mut Connection, ticket_ids: &[i64], price_cents: i64) -> (Vec<i64>, String) {
        let lines = ticket_ids
            .iter()
            .map(|&ticket_id| SaleBatchLineInput { ticket_id, sale_price_cents: price_cents, selling_fees_cents: 0 })
            .collect();
        let sale_ids = create_sales_batch_impl(conn, &SaleBatchInput {
            lines,
            platform_id: None,
            sale_date: "2026-03-01".into(),
            payment_status: Some("pending".into()),
            buyer_reference: None,
            notes: None,
        }).unwrap();
        let batch_id: String = conn
            .query_row("SELECT batch_id FROM sales WHERE id = ?1", [sale_ids[0]], |r| r.get(0))
            .unwrap();
        (sale_ids, batch_id)
    }

    fn payment_input(target_key: Option<&str>, order_id: Option<i64>, amount_cents: i64, currency: &str) -> PaymentInput {
        PaymentInput {
            sale_group_key: target_key.map(str::to_string),
            order_id,
            amount_cents,
            currency: currency.to_string(),
            payment_date: "2026-03-05".into(),
            method: "bank_transfer".into(),
            method_other_note: None,
            reference: None,
        }
    }

    // ---- 1: sale with no payment -> Pending --------------------------------
    #[test]
    fn sale_with_no_payment_is_pending() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn);
        let tickets = seed_tickets(&conn, event_id, 1, "EUR");
        let (_id, key) = seed_single_sale(&mut conn, tickets[0], 1000);

        let summary = compute_payment_summary_for_sale_group_impl(&conn, &key).unwrap();
        assert_eq!(summary.status, "pending");
        assert_eq!(summary.received_cents, Some(0));
        assert_eq!(summary.outstanding_cents, Some(1000));
        assert!(summary.payments.is_empty());
    }

    // ---- 2: sale with 1 (partial) payment -> Partial -----------------------
    #[test]
    fn sale_with_one_partial_payment_is_partial() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn);
        let tickets = seed_tickets(&conn, event_id, 1, "EUR");
        let (_id, key) = seed_single_sale(&mut conn, tickets[0], 1000);

        create_payment_impl(&conn, &payment_input(Some(&key), None, 400, "EUR"), false, false).unwrap();

        let summary = compute_payment_summary_for_sale_group_impl(&conn, &key).unwrap();
        assert_eq!(summary.status, "partial");
        assert_eq!(summary.received_cents, Some(400));
        assert_eq!(summary.outstanding_cents, Some(600));
    }

    // ---- 3: sale with payments = total -> Paid ------------------------------
    #[test]
    fn sale_with_payments_covering_total_is_paid() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn);
        let tickets = seed_tickets(&conn, event_id, 1, "EUR");
        let (_id, key) = seed_single_sale(&mut conn, tickets[0], 1000);

        create_payment_impl(&conn, &payment_input(Some(&key), None, 1000, "EUR"), false, false).unwrap();

        let summary = compute_payment_summary_for_sale_group_impl(&conn, &key).unwrap();
        assert_eq!(summary.status, "paid");
        assert_eq!(summary.received_cents, Some(1000));
        assert_eq!(summary.outstanding_cents, Some(0));
    }

    // ---- 4: sale with 2 payments, still short of total -> Partial ----------
    #[test]
    fn sale_with_two_payments_not_yet_covering_total_is_partial() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn);
        let tickets = seed_tickets(&conn, event_id, 4, "EUR");
        let (_ids, key) = seed_batch_sale(&mut conn, &tickets, 1000); // 4x1000 = 4000 total

        create_payment_impl(&conn, &payment_input(Some(&key), None, 500, "EUR"), false, false).unwrap();
        create_payment_impl(&conn, &payment_input(Some(&key), None, 300, "EUR"), false, false).unwrap();

        let summary = compute_payment_summary_for_sale_group_impl(&conn, &key).unwrap();
        assert_eq!(summary.status, "partial");
        assert_eq!(summary.received_cents, Some(800));
        assert_eq!(summary.outstanding_cents, Some(3200));
        assert_eq!(summary.payments.len(), 2);
    }

    // ---- 5: payment history --------------------------------------------------
    #[test]
    fn payment_summary_lists_full_history_in_date_order() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn);
        let tickets = seed_tickets(&conn, event_id, 1, "EUR");
        let (_id, key) = seed_single_sale(&mut conn, tickets[0], 3000);

        let mut later = payment_input(Some(&key), None, 1000, "EUR");
        later.payment_date = "2026-03-10".into();
        later.reference = Some("second".into());
        create_payment_impl(&conn, &later, false, false).unwrap();

        let mut earlier = payment_input(Some(&key), None, 500, "EUR");
        earlier.payment_date = "2026-03-01".into();
        earlier.reference = Some("first".into());
        create_payment_impl(&conn, &earlier, false, false).unwrap();

        let summary = compute_payment_summary_for_sale_group_impl(&conn, &key).unwrap();
        assert_eq!(summary.payments.len(), 2);
        assert_eq!(summary.payments[0].reference.as_deref(), Some("first"), "earliest payment_date first");
        assert_eq!(summary.payments[1].reference.as_deref(), Some("second"));
        assert_eq!(summary.received_cents, Some(1500));
    }

    // ---- 6: payment delete -----------------------------------------------------
    #[test]
    fn deleting_a_payment_reduces_received_and_can_revert_status() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn);
        let tickets = seed_tickets(&conn, event_id, 1, "EUR");
        let (_id, key) = seed_single_sale(&mut conn, tickets[0], 1000);
        let payment = create_payment_impl(&conn, &payment_input(Some(&key), None, 1000, "EUR"), false, false).unwrap();
        assert_eq!(compute_payment_summary_for_sale_group_impl(&conn, &key).unwrap().status, "paid");

        delete_payment_impl(&conn, payment.id).unwrap();

        let summary = compute_payment_summary_for_sale_group_impl(&conn, &key).unwrap();
        assert_eq!(summary.status, "pending");
        assert_eq!(summary.received_cents, Some(0));
        assert!(summary.payments.is_empty());
    }

    #[test]
    fn deleting_a_nonexistent_payment_is_a_clear_error() {
        let conn = test_conn();
        let result = delete_payment_impl(&conn, 999_999);
        assert!(result.is_err());
    }

    // ---- 7: payment edit ---------------------------------------------------
    #[test]
    fn editing_a_payment_amount_changes_received() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn);
        let tickets = seed_tickets(&conn, event_id, 1, "EUR");
        let (_id, key) = seed_single_sale(&mut conn, tickets[0], 1000);
        let payment = create_payment_impl(&conn, &payment_input(Some(&key), None, 400, "EUR"), false, false).unwrap();

        let mut edited = payment_input(Some(&key), None, 700, "EUR");
        edited.method = "card".into();
        edited.reference = Some("corrected amount".into());
        update_payment_impl(&conn, payment.id, &edited).unwrap();

        let summary = compute_payment_summary_for_sale_group_impl(&conn, &key).unwrap();
        assert_eq!(summary.received_cents, Some(700));
        assert_eq!(summary.status, "partial");
        assert_eq!(summary.payments[0].method, "card");
        assert_eq!(summary.payments[0].reference.as_deref(), Some("corrected amount"));
    }

    // ---- 8: overpayment rejection ------------------------------------------
    #[test]
    fn overpayment_is_rejected_on_create() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn);
        let tickets = seed_tickets(&conn, event_id, 1, "EUR");
        let (_id, key) = seed_single_sale(&mut conn, tickets[0], 1000);
        create_payment_impl(&conn, &payment_input(Some(&key), None, 700, "EUR"), false, false).unwrap();

        // 700 already received, 300 outstanding - trying to add 301 must fail.
        let result = create_payment_impl(&conn, &payment_input(Some(&key), None, 301, "EUR"), false, false);
        assert!(result.is_err(), "a payment that would exceed outstanding must be rejected");

        // Nothing partial got written - received is still exactly 700.
        let summary = compute_payment_summary_for_sale_group_impl(&conn, &key).unwrap();
        assert_eq!(summary.received_cents, Some(700), "the rejected payment must not have been recorded at all");
    }

    #[test]
    fn overpayment_is_rejected_on_edit_too() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn);
        let tickets = seed_tickets(&conn, event_id, 1, "EUR");
        let (_id, key) = seed_single_sale(&mut conn, tickets[0], 1000);
        let a = create_payment_impl(&conn, &payment_input(Some(&key), None, 400, "EUR"), false, false).unwrap();
        create_payment_impl(&conn, &payment_input(Some(&key), None, 400, "EUR"), false, false).unwrap();
        // 800 received, 200 outstanding. Editing payment `a` up to 401 (i.e.
        // the OTHER payment's 400 + this edited 401 = 801) must be rejected.
        let result = update_payment_impl(&conn, a.id, &payment_input(Some(&key), None, 401, "EUR"));
        assert!(result.is_err());
    }

    // ---- 9: mixed currency --------------------------------------------------
    #[test]
    fn payments_in_mixed_currencies_report_as_mixed_not_blended() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn);
        let tickets = seed_tickets(&conn, event_id, 1, "EUR");
        let (_id, key) = seed_single_sale(&mut conn, tickets[0], 1000);
        create_payment_impl(&conn, &payment_input(Some(&key), None, 500, "EUR"), false, false).unwrap();
        // A payment in a different currency than the sale itself - allowed
        // (real-world payments sometimes do arrive this way - see
        // outstanding_for_overpayment_check's own doc comment), but it must
        // never get silently netted against the EUR total.
        create_payment_impl(&conn, &payment_input(Some(&key), None, 100, "USD"), false, false).unwrap();

        let summary = compute_payment_summary_for_sale_group_impl(&conn, &key).unwrap();
        assert_eq!(summary.received_cents, None, "mixed-currency payments must never be summed into one number");
        assert_eq!(summary.outstanding_cents, None);
        assert_eq!(summary.currency, None);
        assert_eq!(summary.status, "partial", "still an honest 'something came in but can't net it' label");
        assert_eq!(summary.payments.len(), 2, "both payments still show up in history");
    }

    #[test]
    fn sale_groups_own_mixed_currency_lines_report_status_mixed() {
        // Distinct from the payments-in-mixed-currencies case above: here the
        // SALE ITSELF (its own non-refunded lines) already spans more than
        // one currency, independent of any payment.
        let mut conn = test_conn();
        let event_id = seed_event(&conn);
        let eur_tickets = seed_tickets(&conn, event_id, 1, "EUR");
        let usd_tickets = seed_tickets(&conn, event_id, 1, "USD");
        let all_tickets = vec![eur_tickets[0], usd_tickets[0]];
        let (_ids, key) = seed_batch_sale(&mut conn, &all_tickets, 1000);

        let summary = compute_payment_summary_for_sale_group_impl(&conn, &key).unwrap();
        assert_eq!(summary.status, "mixed");
        assert_eq!(summary.total_cents, None);
        assert_eq!(summary.total_currency, None);
    }

    // ---- 10: refunded sale ---------------------------------------------------
    #[test]
    fn fully_refunded_sale_group_reports_refunded_regardless_of_prior_payments() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn);
        let tickets = seed_tickets(&conn, event_id, 1, "EUR");
        let (sale_id, key) = seed_single_sale(&mut conn, tickets[0], 1000);
        create_payment_impl(&conn, &payment_input(Some(&key), None, 1000, "EUR"), false, false).unwrap();
        assert_eq!(compute_payment_summary_for_sale_group_impl(&conn, &key).unwrap().status, "paid");

        refund_sale_impl(&mut conn, sale_id, Some("buyer cancelled")).unwrap();

        let summary = compute_payment_summary_for_sale_group_impl(&conn, &key).unwrap();
        assert_eq!(summary.status, "refunded");
        assert_eq!(summary.received_cents, None);
        assert_eq!(summary.outstanding_cents, None);
        assert_eq!(
            summary.payments.len(), 1,
            "the payment history must stay - refunding never deletes past payments"
        );
    }

    #[test]
    fn new_payments_are_rejected_against_an_already_refunded_sale() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn);
        let tickets = seed_tickets(&conn, event_id, 1, "EUR");
        let (sale_id, key) = seed_single_sale(&mut conn, tickets[0], 1000);
        refund_sale_impl(&mut conn, sale_id, Some("buyer cancelled")).unwrap();

        let result = create_payment_impl(&conn, &payment_input(Some(&key), None, 500, "EUR"), false, false);
        assert!(result.is_err(), "a refunded sale can't take new payments");
    }

    // ---- 11: refund -> resell --------------------------------------------------
    #[test]
    fn refund_then_resell_only_the_new_sale_has_its_own_payments() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn);
        let tickets = seed_tickets(&conn, event_id, 1, "EUR");
        let (first_id, first_key) = seed_single_sale(&mut conn, tickets[0], 1000);
        create_payment_impl(&conn, &payment_input(Some(&first_key), None, 1000, "EUR"), false, false).unwrap();
        refund_sale_impl(&mut conn, first_id, Some("buyer cancelled")).unwrap();

        let (_second_id, second_key) = seed_single_sale(&mut conn, tickets[0], 900);
        assert_ne!(first_key, second_key, "the resale is a distinct sale group with its own key");

        let first_summary = compute_payment_summary_for_sale_group_impl(&conn, &first_key).unwrap();
        assert_eq!(first_summary.status, "refunded");

        let second_summary = compute_payment_summary_for_sale_group_impl(&conn, &second_key).unwrap();
        assert_eq!(second_summary.status, "pending", "the resale starts with no payments of its own");
        assert_eq!(second_summary.outstanding_cents, Some(900));
    }

    // ---- 14/15/16: Order Partial/Paid/Unpaid ----------------------------------
    #[test]
    fn order_with_no_payment_is_unpaid() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn);
        let order_id = create_order_impl(&conn, &order_input(event_id, 2, 1000, "EUR", None)).unwrap();

        let summary = compute_payment_summary_for_order_impl(&conn, order_id).unwrap();
        assert_eq!(summary.status, "pending"); // "unpaid" in orders' own vocabulary, same derived state
        assert_eq!(summary.outstanding_cents, Some(2000));
    }

    #[test]
    fn order_partial_when_a_real_payment_does_not_cover_the_total() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn);
        let order_id = create_order_impl(&conn, &order_input(event_id, 2, 1000, "EUR", None)).unwrap();
        create_payment_impl(&conn, &payment_input(None, Some(order_id), 1200, "EUR"), false, false).unwrap();

        let summary = compute_payment_summary_for_order_impl(&conn, order_id).unwrap();
        assert_eq!(summary.status, "partial");
        assert_eq!(summary.received_cents, Some(1200));
        assert_eq!(summary.outstanding_cents, Some(800));
    }

    #[test]
    fn order_created_as_paid_gets_one_shortcut_payment_for_the_full_total() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn);
        let order_id = create_order_impl(&conn, &order_input(event_id, 2, 1000, "EUR", Some("paid"))).unwrap();

        let summary = compute_payment_summary_for_order_impl(&conn, order_id).unwrap();
        assert_eq!(summary.status, "paid");
        assert_eq!(summary.received_cents, Some(2000));
        assert_eq!(summary.payments.len(), 1);
        assert!(summary.payments[0].is_shortcut);
    }

    #[test]
    fn order_edited_to_unpaid_reverts_the_shortcut() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn);
        let order_id = create_order_impl(&conn, &order_input(event_id, 1, 1000, "EUR", Some("paid"))).unwrap();
        assert_eq!(compute_payment_summary_for_order_impl(&conn, order_id).unwrap().status, "paid");

        let edit = OrderEditInput {
            supplier_id: None,
            platform_id: None,
            purchase_date: "2026-01-01".into(),
            currency: "EUR".into(),
            payment_status: "unpaid".into(),
            notes: None,
        };
        update_order_impl(&conn, order_id, &edit).unwrap();

        let summary = compute_payment_summary_for_order_impl(&conn, order_id).unwrap();
        assert_eq!(summary.status, "pending");
        assert!(summary.payments.is_empty(), "reverting the shortcut removes the payment it created");
    }

    #[test]
    fn order_edited_to_unpaid_refuses_when_real_payment_history_exists() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn);
        let order_id = create_order_impl(&conn, &order_input(event_id, 1, 1000, "EUR", None)).unwrap();
        // A real, manually-entered payment - not the shortcut.
        create_payment_impl(&conn, &payment_input(None, Some(order_id), 1000, "EUR"), false, false).unwrap();

        let edit = OrderEditInput {
            supplier_id: None,
            platform_id: None,
            purchase_date: "2026-01-01".into(),
            currency: "EUR".into(),
            payment_status: "unpaid".into(),
            notes: None,
        };
        let result = update_order_impl(&conn, order_id, &edit);
        assert!(result.is_err(), "Unpaid must refuse rather than silently delete real payment history");

        let summary = compute_payment_summary_for_order_impl(&conn, order_id).unwrap();
        assert_eq!(summary.status, "paid", "nothing was touched by the refused edit");
    }

    #[test]
    fn create_order_rejects_partial_as_an_initial_status() {
        let conn = test_conn();
        let event_id = seed_event(&conn);
        let result = create_order_impl(&conn, &order_input(event_id, 1, 1000, "EUR", Some("partial")));
        assert!(result.is_err(), "Partial is a derived state, never a directly settable one");
    }

    // ---- 19: payment transaction rollback -------------------------------------
    #[test]
    fn a_rejected_overpayment_writes_absolutely_nothing() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn);
        let tickets = seed_tickets(&conn, event_id, 1, "EUR");
        let (_id, key) = seed_single_sale(&mut conn, tickets[0], 1000);

        let before_count: i64 = conn.query_row("SELECT COUNT(*) FROM payments", [], |r| r.get(0)).unwrap();
        let result = create_payment_impl(&conn, &payment_input(Some(&key), None, 1001, "EUR"), false, false);
        assert!(result.is_err());
        let after_count: i64 = conn.query_row("SELECT COUNT(*) FROM payments", [], |r| r.get(0)).unwrap();
        assert_eq!(before_count, after_count, "a rejected payment must leave zero trace, not a half-written row");
    }

    #[test]
    fn bulk_mark_as_paid_shortcut_and_pending_revert_round_trip() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn);
        let tickets = seed_tickets(&conn, event_id, 1, "EUR");
        let (_id, key) = seed_single_sale(&mut conn, tickets[0], 1000);

        apply_paid_shortcut_for_sale_group_impl(&conn, &key, 1000, "EUR").unwrap();
        let summary = compute_payment_summary_for_sale_group_impl(&conn, &key).unwrap();
        assert_eq!(summary.status, "paid");
        assert!(summary.payments[0].is_shortcut);

        revert_paid_shortcut_for_sale_group_impl(&conn, &key).unwrap();
        let summary = compute_payment_summary_for_sale_group_impl(&conn, &key).unwrap();
        assert_eq!(summary.status, "pending");
        assert!(summary.payments.is_empty());
    }

    #[test]
    fn reverting_the_shortcut_refuses_when_real_payment_history_exists() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn);
        let tickets = seed_tickets(&conn, event_id, 1, "EUR");
        let (_id, key) = seed_single_sale(&mut conn, tickets[0], 1000);
        create_payment_impl(&conn, &payment_input(Some(&key), None, 1000, "EUR"), false, false).unwrap();

        let result = revert_paid_shortcut_for_sale_group_impl(&conn, &key);
        assert!(result.is_err(), "a manually-entered payment must never be silently deleted by the shortcut's revert");
        let summary = compute_payment_summary_for_sale_group_impl(&conn, &key).unwrap();
        assert_eq!(summary.payments.len(), 1, "the real payment must still be there");
    }
}

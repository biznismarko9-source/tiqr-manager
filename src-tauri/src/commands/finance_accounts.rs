//! 2.1.0: marko's "FINANCE 2.1" Accounts/Wallets + Transfers - see
//! FINANCE-2.1.0-REPORT.md for the full spec/rationale and
//! migrations/016_finance_v2.sql for the schema this mirrors. Same
//! "impl function + thin #[tauri::command] wrapper" pattern as
//! commands::finance_entries, and the same "no duplicate financial system"
//! rule: this reuses `finance_entries`'s own table for income/expense
//! (only reading its `account_id`/`amount_cents`/`entry_type` columns to
//! fold into an account's balance), never a second ledger.
//!
//! Two entities:
//!   - Accounts: marko's own wallets. `current_balance_cents` is NEVER
//!     stored - `list_accounts` computes it fresh for every account in ONE
//!     query (see `ACCOUNT_SELECT` below), joining three pre-aggregated
//!     subqueries (income, expense, transfers in/out) rather than issuing a
//!     separate query per account - this is the whole answer to marko's own
//!     performance requirement (point 18: no N+1, account balances computed
//!     efficiently).
//!   - Transfers: a movement of marko's own money between two of his own
//!     accounts. Deliberately atomic by construction, not by a multi-step
//!     transaction: a transfer is ONE row in ONE table, and every account's
//!     balance is *computed* from it (never a second write to update "the
//!     other side"), so there is no possible partially-applied state to
//!     guard against - either the single INSERT below commits, and both
//!     accounts' computed balances reflect it immediately, or it fails and
//!     neither does. Cross-currency transfers are rejected outright in v1
//!     (marko's own preferred "simpler, safer" option, point 6) - the
//!     transfer's `currency` is always derived from the two accounts
//!     themselves (which must already agree), never trusted from client
//!     input.

use crate::commands::finance_entries::normalize_optional;
use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::models::{Account, AccountInput, Transfer, TransferInput};
use rusqlite::{Connection, OptionalExtension, Row};
use tauri::State;

// --- Accounts --------------------------------------------------------------

const ACCOUNT_TYPES: [&str; 6] = ["bank", "revolut", "paypal", "cash", "credit_card", "other"];

/// Every account read goes through this one SELECT - same "list and the
/// create/update read-back can never disagree" discipline as
/// `finance_entries::FINANCE_ENTRY_SELECT`. The three `LEFT JOIN`s are each
/// a single `GROUP BY account_id` pass over their own table (finance_entries
/// once for income, once for expense; transfers once for in, once for out -
/// four aggregate passes total, not four-times-N) - this is the "ONE
/// query for every account's balance" mentioned in this module's own doc
/// comment above.
const ACCOUNT_SELECT: &str = "SELECT a.id, a.name, a.account_type, a.currency, a.opening_balance_cents,
    a.opening_balance_cents + COALESCE(inc.total, 0) - COALESCE(exp.total, 0)
        + COALESCE(tin.total, 0) - COALESCE(tout.total, 0) AS current_balance_cents,
    a.is_active, a.is_demo, a.created_at, a.updated_at
    FROM accounts a
    LEFT JOIN (SELECT account_id, SUM(amount_cents) AS total FROM finance_entries
               WHERE entry_type = 'income' GROUP BY account_id) inc ON inc.account_id = a.id
    LEFT JOIN (SELECT account_id, SUM(amount_cents) AS total FROM finance_entries
               WHERE entry_type = 'expense' GROUP BY account_id) exp ON exp.account_id = a.id
    LEFT JOIN (SELECT to_account_id AS account_id, SUM(amount_cents) AS total FROM transfers
               GROUP BY to_account_id) tin ON tin.account_id = a.id
    LEFT JOIN (SELECT from_account_id AS account_id, SUM(amount_cents) AS total FROM transfers
               GROUP BY from_account_id) tout ON tout.account_id = a.id";

fn map_account(row: &Row) -> rusqlite::Result<Account> {
    Ok(Account {
        id: row.get("id")?,
        name: row.get("name")?,
        account_type: row.get("account_type")?,
        currency: row.get("currency")?,
        opening_balance_cents: row.get("opening_balance_cents")?,
        current_balance_cents: row.get("current_balance_cents")?,
        is_active: row.get("is_active")?,
        is_demo: row.get("is_demo")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

/// Creation order (oldest first) - same convention as
/// `finance_entries::list_finance_categories`'s own lookup-list ordering
/// intent, just by `created_at` instead of name since accounts are few and
/// marko is more likely to think of them in "the order I set them up" than
/// alphabetically.
#[tauri::command]
pub fn list_accounts(state: State<AppState>) -> AppResult<Vec<Account>> {
    let conn = state.db.lock().unwrap();
    let sql = format!("{ACCOUNT_SELECT} ORDER BY a.created_at ASC, a.id ASC");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], map_account)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

fn validate_account_fields(input: &AccountInput) -> AppResult<()> {
    if input.name.trim().is_empty() {
        return Err(AppError::Validation("Account name cannot be empty".into()));
    }
    if !ACCOUNT_TYPES.contains(&input.account_type.as_str()) {
        return Err(AppError::Validation(format!(
            "Invalid account type '{}' - must be one of: {}",
            input.account_type,
            ACCOUNT_TYPES.join(", ")
        )));
    }
    if input.currency.trim().is_empty() {
        return Err(AppError::Validation("Currency cannot be empty".into()));
    }
    Ok(())
}

pub(crate) fn create_account_impl(conn: &Connection, input: &AccountInput) -> AppResult<Account> {
    validate_account_fields(input)?;
    let name = input.name.trim();
    let currency = input.currency.trim().to_ascii_uppercase();
    conn.execute(
        "INSERT INTO accounts(name, account_type, currency, opening_balance_cents, is_active)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![name, input.account_type, currency, input.opening_balance_cents, input.is_active],
    )?;
    let id = conn.last_insert_rowid();
    let sql = format!("{ACCOUNT_SELECT} WHERE a.id = ?1");
    Ok(conn.query_row(&sql, [id], map_account)?)
}

#[tauri::command]
pub fn create_account(state: State<AppState>, input: AccountInput) -> AppResult<Account> {
    let conn = state.db.lock().unwrap();
    create_account_impl(&conn, &input)
}

pub(crate) fn update_account_impl(conn: &Connection, id: i64, input: &AccountInput) -> AppResult<Account> {
    validate_account_fields(input)?;
    let name = input.name.trim();
    let currency = input.currency.trim().to_ascii_uppercase();
    let updated = conn.execute(
        "UPDATE accounts SET name = ?1, account_type = ?2, currency = ?3, opening_balance_cents = ?4,
             is_active = ?5, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
         WHERE id = ?6",
        rusqlite::params![name, input.account_type, currency, input.opening_balance_cents, input.is_active, id],
    )?;
    if updated == 0 {
        return Err(AppError::NotFound(format!("Account #{id} not found")));
    }
    let sql = format!("{ACCOUNT_SELECT} WHERE a.id = ?1");
    Ok(conn.query_row(&sql, [id], map_account)?)
}

#[tauri::command]
pub fn update_account(state: State<AppState>, id: i64, input: AccountInput) -> AppResult<Account> {
    let conn = state.db.lock().unwrap();
    update_account_impl(&conn, id, &input)
}

/// Plain blind delete, same as `finance_entries::delete_finance_category`.
/// Unlike that category delete, this one CAN be rejected by the database:
/// `finance_entries.account_id`/`recurring_expenses.account_id` are
/// `ON DELETE SET NULL` (an account referenced only by entries/recurring
/// templates deletes cleanly, same as a category), but
/// `transfers.from_account_id`/`to_account_id` are `ON DELETE RESTRICT`
/// (migrations/016_finance_v2.sql's own header comment explains why a
/// transfer can never be left half-detached) - `AppError`'s own
/// `From<rusqlite::Error>` already turns that specific SQLite error into a
/// clear "other records still reference it" validation message, so no
/// extra check is needed here.
#[tauri::command]
pub fn delete_account(state: State<AppState>, id: i64) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    conn.execute("DELETE FROM accounts WHERE id = ?1", [id])?;
    Ok(())
}

// --- Transfers ---------------------------------------------------------

const TRANSFER_SELECT: &str = "SELECT t.id, t.transfer_date, t.from_account_id, fa.name AS from_account_name,
    t.to_account_id, ta.name AS to_account_name, t.amount_cents, t.currency, t.note, t.is_demo, t.created_at
    FROM transfers t
    LEFT JOIN accounts fa ON fa.id = t.from_account_id
    LEFT JOIN accounts ta ON ta.id = t.to_account_id";

fn map_transfer(row: &Row) -> rusqlite::Result<Transfer> {
    Ok(Transfer {
        id: row.get("id")?,
        transfer_date: row.get("transfer_date")?,
        from_account_id: row.get("from_account_id")?,
        from_account_name: row.get("from_account_name")?,
        to_account_id: row.get("to_account_id")?,
        to_account_name: row.get("to_account_name")?,
        amount_cents: row.get("amount_cents")?,
        currency: row.get("currency")?,
        note: row.get("note")?,
        is_demo: row.get("is_demo")?,
        created_at: row.get("created_at")?,
    })
}

/// Newest first, same convention as `finance_entries::list_finance_entries`.
#[tauri::command]
pub fn list_transfers(state: State<AppState>) -> AppResult<Vec<Transfer>> {
    let conn = state.db.lock().unwrap();
    let sql = format!("{TRANSFER_SELECT} ORDER BY t.transfer_date DESC, t.id DESC");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], map_transfer)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

fn account_currency(conn: &Connection, id: i64) -> AppResult<String> {
    conn.query_row("SELECT currency FROM accounts WHERE id = ?1", [id], |r| r.get(0))
        .optional()?
        .ok_or_else(|| AppError::Validation(format!("Account #{id} not found")))
}

/// Core logic behind `create_transfer` - see this module's own doc comment
/// for why a transfer needs no separate "commit both sides" step to be
/// atomic, and TRANSFER SAFETY (marko's point 6), all enforced here before
/// a single row is ever written:
///   - both accounts must currently exist (`account_currency` above)
///   - from != to
///   - amount > 0
///   - both accounts must share one currency (no invented exchange rate -
///     marko's own preferred "simpler, safer" v1 option)
pub(crate) fn create_transfer_impl(conn: &Connection, input: &TransferInput) -> AppResult<Transfer> {
    if input.transfer_date.trim().is_empty() {
        return Err(AppError::Validation("Date cannot be empty".into()));
    }
    if input.amount_cents <= 0 {
        return Err(AppError::Validation("Amount must be greater than 0".into()));
    }
    if input.from_account_id == input.to_account_id {
        return Err(AppError::Validation("From and To accounts must be different".into()));
    }
    let from_currency = account_currency(conn, input.from_account_id)?;
    let to_currency = account_currency(conn, input.to_account_id)?;
    if from_currency != to_currency {
        return Err(AppError::Validation(format!(
            "Cross-currency transfers aren't supported yet - the From account is {from_currency} and the To account is {to_currency}."
        )));
    }
    let note = normalize_optional(input.note.clone());
    conn.execute(
        "INSERT INTO transfers(transfer_date, from_account_id, to_account_id, amount_cents, currency, note)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![input.transfer_date, input.from_account_id, input.to_account_id, input.amount_cents, from_currency, note],
    )?;
    let id = conn.last_insert_rowid();
    let sql = format!("{TRANSFER_SELECT} WHERE t.id = ?1");
    Ok(conn.query_row(&sql, [id], map_transfer)?)
}

#[tauri::command]
pub fn create_transfer(state: State<AppState>, input: TransferInput) -> AppResult<Transfer> {
    let conn = state.db.lock().unwrap();
    create_transfer_impl(&conn, &input)
}

/// No `update_transfer` by design (v1 keeps transfers simple, matching
/// marko's own request) - fixing a mistake is delete-and-recreate, same
/// spirit as a bank transfer itself not being "editable" after the fact.
#[tauri::command]
pub fn delete_transfer(state: State<AppState>, id: i64) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    conn.execute("DELETE FROM transfers WHERE id = ?1", [id])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_conn;
    use crate::models::FinanceEntryInput;

    fn sample_account(name: &str, currency: &str, opening_cents: i64) -> AccountInput {
        AccountInput {
            name: name.to_string(),
            account_type: "bank".to_string(),
            currency: currency.to_string(),
            opening_balance_cents: opening_cents,
            is_active: true,
        }
    }

    fn income_entry(account_id: i64, amount_cents: i64, currency: &str) -> FinanceEntryInput {
        FinanceEntryInput {
            entry_type: "income".to_string(),
            entry_date: "2026-08-01".to_string(),
            amount_cents,
            currency: currency.to_string(),
            scope: "personal".to_string(),
            category_id: None,
            account_id: Some(account_id),
            order_id: None,
            place: None,
            note: None,
        }
    }

    fn expense_entry(account_id: i64, amount_cents: i64, currency: &str) -> FinanceEntryInput {
        FinanceEntryInput {
            entry_type: "expense".to_string(),
            entry_date: "2026-08-02".to_string(),
            amount_cents,
            currency: currency.to_string(),
            scope: "personal".to_string(),
            category_id: None,
            account_id: Some(account_id),
            order_id: None,
            place: None,
            note: None,
        }
    }

    // --- Test scenario 1: create account --------------------------------

    #[test]
    fn create_account_stores_every_field_and_starts_at_its_opening_balance() {
        let conn = test_conn();
        let created = create_account_impl(&conn, &sample_account("Revolut", "eur", 245000)).unwrap();
        assert_eq!(created.name, "Revolut");
        assert_eq!(created.currency, "EUR", "currency must be uppercased, same convention as finance_entries");
        assert_eq!(created.opening_balance_cents, 245000);
        assert_eq!(created.current_balance_cents, 245000, "no transactions yet - current balance is just the opening balance");
        assert!(created.is_active);
    }

    #[test]
    fn create_account_rejects_empty_name() {
        let conn = test_conn();
        let err = create_account_impl(&conn, &sample_account("   ", "EUR", 0)).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn create_account_rejects_an_invalid_type() {
        let conn = test_conn();
        let mut input = sample_account("Mystery", "EUR", 0);
        input.account_type = "crypto_wallet".to_string();
        let err = create_account_impl(&conn, &input).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    // --- Test scenarios 2 & 3: income/expense change account balance ----

    #[test]
    fn income_on_an_account_increases_its_balance() {
        let conn = test_conn();
        let account = create_account_impl(&conn, &sample_account("Bank", "EUR", 10000)).unwrap();
        crate::commands::finance_entries::create_finance_entry_impl(&conn, &income_entry(account.id, 50000, "EUR")).unwrap();
        let accounts = {
            let sql = format!("{ACCOUNT_SELECT} WHERE a.id = ?1");
            conn.query_row(&sql, [account.id], map_account).unwrap()
        };
        assert_eq!(accounts.current_balance_cents, 60000, "10000 opening + 50000 income");
    }

    #[test]
    fn expense_on_an_account_decreases_its_balance() {
        let conn = test_conn();
        let account = create_account_impl(&conn, &sample_account("Bank", "EUR", 10000)).unwrap();
        crate::commands::finance_entries::create_finance_entry_impl(&conn, &expense_entry(account.id, 4000, "EUR")).unwrap();
        let sql = format!("{ACCOUNT_SELECT} WHERE a.id = ?1");
        let reloaded = conn.query_row(&sql, [account.id], map_account).unwrap();
        assert_eq!(reloaded.current_balance_cents, 6000, "10000 opening - 4000 expense");
    }

    #[test]
    fn list_accounts_computes_every_balance_in_one_pass_with_no_transactions() {
        let conn = test_conn();
        create_account_impl(&conn, &sample_account("A", "EUR", 100)).unwrap();
        create_account_impl(&conn, &sample_account("B", "USD", 200)).unwrap();
        let accounts = list_accounts_for_test(&conn);
        assert_eq!(accounts.len(), 2);
        assert_eq!(accounts[0].current_balance_cents, 100);
        assert_eq!(accounts[1].current_balance_cents, 200);
    }

    fn list_accounts_for_test(conn: &Connection) -> Vec<Account> {
        let sql = format!("{ACCOUNT_SELECT} ORDER BY a.created_at ASC, a.id ASC");
        let mut stmt = conn.prepare(&sql).unwrap();
        stmt.query_map([], map_account).unwrap().collect::<Result<Vec<_>, _>>().unwrap()
    }

    // --- Test scenarios 4, 5, 6, 7: transfers -----------------------------

    #[test]
    fn transfer_decreases_the_source_account() {
        let conn = test_conn();
        let from = create_account_impl(&conn, &sample_account("Revolut", "EUR", 100000)).unwrap();
        let to = create_account_impl(&conn, &sample_account("Bank", "EUR", 0)).unwrap();
        create_transfer_impl(
            &conn,
            &TransferInput { transfer_date: "2026-08-10".to_string(), from_account_id: from.id, to_account_id: to.id, amount_cents: 50000, note: None },
        )
        .unwrap();
        let reloaded = list_accounts_for_test(&conn);
        let from_reloaded = reloaded.iter().find(|a| a.id == from.id).unwrap();
        assert_eq!(from_reloaded.current_balance_cents, 50000, "100000 - 50000 transferred out");
    }

    #[test]
    fn transfer_increases_the_destination_account() {
        let conn = test_conn();
        let from = create_account_impl(&conn, &sample_account("Revolut", "EUR", 100000)).unwrap();
        let to = create_account_impl(&conn, &sample_account("Bank", "EUR", 0)).unwrap();
        create_transfer_impl(
            &conn,
            &TransferInput { transfer_date: "2026-08-10".to_string(), from_account_id: from.id, to_account_id: to.id, amount_cents: 50000, note: None },
        )
        .unwrap();
        let reloaded = list_accounts_for_test(&conn);
        let to_reloaded = reloaded.iter().find(|a| a.id == to.id).unwrap();
        assert_eq!(to_reloaded.current_balance_cents, 50000, "0 + 50000 transferred in");
    }

    #[test]
    fn transfer_never_changes_the_total_across_both_accounts() {
        let conn = test_conn();
        let from = create_account_impl(&conn, &sample_account("Revolut", "EUR", 100000)).unwrap();
        let to = create_account_impl(&conn, &sample_account("Bank", "EUR", 30000)).unwrap();
        let total_before = 100000 + 30000;
        create_transfer_impl(
            &conn,
            &TransferInput { transfer_date: "2026-08-10".to_string(), from_account_id: from.id, to_account_id: to.id, amount_cents: 12345, note: None },
        )
        .unwrap();
        let reloaded = list_accounts_for_test(&conn);
        let total_after: i64 = reloaded.iter().filter(|a| a.id == from.id || a.id == to.id).map(|a| a.current_balance_cents).sum();
        assert_eq!(total_after, total_before, "a transfer only moves money between marko's own accounts - the combined total must never change");
    }

    #[test]
    fn transfer_is_never_counted_as_income_or_expense() {
        let conn = test_conn();
        let from = create_account_impl(&conn, &sample_account("Revolut", "EUR", 100000)).unwrap();
        let to = create_account_impl(&conn, &sample_account("Bank", "EUR", 0)).unwrap();
        create_transfer_impl(
            &conn,
            &TransferInput { transfer_date: "2026-08-10".to_string(), from_account_id: from.id, to_account_id: to.id, amount_cents: 50000, note: None },
        )
        .unwrap();
        let entry_count: i64 = conn.query_row("SELECT COUNT(*) FROM finance_entries", [], |r| r.get(0)).unwrap();
        assert_eq!(entry_count, 0, "a transfer must never create a finance_entries row - P&L must never see it as income or expense");
    }

    #[test]
    fn create_transfer_rejects_the_same_account_on_both_sides() {
        let conn = test_conn();
        let account = create_account_impl(&conn, &sample_account("Revolut", "EUR", 1000)).unwrap();
        let err = create_transfer_impl(
            &conn,
            &TransferInput { transfer_date: "2026-08-10".to_string(), from_account_id: account.id, to_account_id: account.id, amount_cents: 100, note: None },
        )
        .unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn create_transfer_rejects_a_nonexistent_account() {
        let conn = test_conn();
        let account = create_account_impl(&conn, &sample_account("Revolut", "EUR", 1000)).unwrap();
        let err = create_transfer_impl(
            &conn,
            &TransferInput { transfer_date: "2026-08-10".to_string(), from_account_id: account.id, to_account_id: 999_999, amount_cents: 100, note: None },
        )
        .unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    // --- Test scenario 12 (partial): mixed currency safety on transfers --

    #[test]
    fn create_transfer_rejects_cross_currency_by_default() {
        let conn = test_conn();
        let eur_account = create_account_impl(&conn, &sample_account("Revolut EUR", "EUR", 1000)).unwrap();
        let usd_account = create_account_impl(&conn, &sample_account("PayPal USD", "USD", 1000)).unwrap();
        let err = create_transfer_impl(
            &conn,
            &TransferInput { transfer_date: "2026-08-10".to_string(), from_account_id: eur_account.id, to_account_id: usd_account.id, amount_cents: 100, note: None },
        )
        .unwrap_err();
        assert!(matches!(err, AppError::Validation(_)), "v1 must never invent an exchange rate - cross-currency transfers are rejected outright");
    }

    #[test]
    fn create_transfer_rejects_a_non_positive_amount() {
        let conn = test_conn();
        let from = create_account_impl(&conn, &sample_account("A", "EUR", 1000)).unwrap();
        let to = create_account_impl(&conn, &sample_account("B", "EUR", 0)).unwrap();
        let err = create_transfer_impl(
            &conn,
            &TransferInput { transfer_date: "2026-08-10".to_string(), from_account_id: from.id, to_account_id: to.id, amount_cents: 0, note: None },
        )
        .unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn delete_account_is_blocked_while_a_transfer_still_references_it() {
        let conn = test_conn();
        let from = create_account_impl(&conn, &sample_account("A", "EUR", 1000)).unwrap();
        let to = create_account_impl(&conn, &sample_account("B", "EUR", 0)).unwrap();
        create_transfer_impl(
            &conn,
            &TransferInput { transfer_date: "2026-08-10".to_string(), from_account_id: from.id, to_account_id: to.id, amount_cents: 100, note: None },
        )
        .unwrap();
        let err = conn.execute("DELETE FROM accounts WHERE id = ?1", [from.id]).unwrap_err();
        let app_err: AppError = err.into();
        assert!(matches!(app_err, AppError::Validation(_)), "ON DELETE RESTRICT must block deleting an account still referenced by a transfer");
    }

    #[test]
    fn update_account_changes_fields_and_rejects_a_missing_id() {
        let conn = test_conn();
        let created = create_account_impl(&conn, &sample_account("Cash", "EUR", 0)).unwrap();
        let mut edited = sample_account("Cash wallet", "EUR", 0);
        edited.is_active = false;
        let updated = update_account_impl(&conn, created.id, &edited).unwrap();
        assert_eq!(updated.name, "Cash wallet");
        assert!(!updated.is_active);

        let err = update_account_impl(&conn, 999_999, &sample_account("X", "EUR", 0)).unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }
}

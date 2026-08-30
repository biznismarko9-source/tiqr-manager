//! 2.1.0: marko's "FINANCE 2.1" Recurring Expenses - see
//! FINANCE-2.1.0-REPORT.md for the full spec/rationale and
//! migrations/016_finance_v2.sql for the schema this mirrors.
//!
//! A `recurring_expenses` row is a scheduled TEMPLATE, never a transaction
//! by itself. The actual `FinanceEntry` it eventually produces is only ever
//! created by `create_from_recurring` - an explicit, user-clicked action
//! (marko's own point 8: "Upcoming recurring expenses" with a
//! [Create]/[Skip]/[Pause] choice per item). Nothing in this module runs on
//! app startup, on a timer, or as a side effect of any read - `list_recurring
//! _expenses` (called every time the Accounts tab loads) is pure, read-only,
//! and never advances `next_date` or writes anything. This is what makes
//! "never create a duplicate transaction on repeated app opens" true by
//! construction rather than something that has to be carefully guarded:
//! there is simply no code path that creates one without an explicit click.
//!
//! `next_date` only ever moves forward, and only via two actions:
//!   - `skip_recurring_expense` advances it one occurrence with no entry
//!     created (marko decided this cycle doesn't need logging).
//!   - `create_from_recurring` creates the entry AND advances it, together,
//!     inside one transaction (see that function) - so a failure partway
//!     through can never leave a created entry paired with a stale
//!     `next_date` (which would otherwise risk a real duplicate on the next
//!     click).
//! A paused (`is_active = false`) template's `next_date` never moves at all
//! until resumed - if real time passes while paused, it simply shows up as
//! "overdue" once resumed (the frontend compares `next_date` to today), and
//! the user works through it one explicit Create/Skip click at a time. No
//! "catch-up" logic ever runs automatically - same reasoning as above.

use crate::commands::finance_entries::{normalize_optional, validate_account};
use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::models::{CreateFromRecurringResult, FinanceEntryInput, RecurringExpense, RecurringExpenseInput};
use chrono::{Datelike, Duration, NaiveDate};
use rusqlite::{Connection, OptionalExtension, Row};
use tauri::State;

const RECURRING_FREQUENCIES: [&str; 4] = ["weekly", "monthly", "quarterly", "yearly"];
const RECURRING_SCOPES: [&str; 2] = ["personal", "business"];

const RECURRING_SELECT: &str = "SELECT r.id, r.name, r.amount_cents, r.currency, r.scope,
    r.category_id, c.name AS category_name, c.color_slot AS category_color_slot,
    r.account_id, a.name AS account_name,
    r.frequency, r.start_date, r.next_date, r.is_active, r.note, r.is_demo, r.created_at, r.updated_at
    FROM recurring_expenses r
    LEFT JOIN finance_categories c ON c.id = r.category_id
    LEFT JOIN accounts a ON a.id = r.account_id";

fn map_recurring(row: &Row) -> rusqlite::Result<RecurringExpense> {
    Ok(RecurringExpense {
        id: row.get("id")?,
        name: row.get("name")?,
        amount_cents: row.get("amount_cents")?,
        currency: row.get("currency")?,
        scope: row.get("scope")?,
        category_id: row.get("category_id")?,
        category_name: row.get("category_name")?,
        category_color_slot: row.get("category_color_slot")?,
        account_id: row.get("account_id")?,
        account_name: row.get("account_name")?,
        frequency: row.get("frequency")?,
        start_date: row.get("start_date")?,
        next_date: row.get("next_date")?,
        is_active: row.get("is_active")?,
        note: row.get("note")?,
        is_demo: row.get("is_demo")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn fetch_recurring(conn: &Connection, id: i64) -> AppResult<RecurringExpense> {
    let sql = format!("{RECURRING_SELECT} WHERE r.id = ?1");
    conn.query_row(&sql, [id], map_recurring)
        .optional()?
        .ok_or_else(|| AppError::NotFound(format!("Recurring expense #{id} not found")))
}

/// Soonest (or most overdue) first - matches marko's own "Upcoming recurring
/// expenses" panel (point 8), which wants to lead with whatever needs
/// attention next. The full list is always small (manually curated
/// templates, not transactions), so - same "flat list, no pagination"
/// philosophy as `finance_entries::list_finance_entries` - this always
/// returns everything and lets the frontend do any further grouping.
#[tauri::command]
pub fn list_recurring_expenses(state: State<AppState>) -> AppResult<Vec<RecurringExpense>> {
    let conn = state.db.lock().unwrap();
    let sql = format!("{RECURRING_SELECT} ORDER BY r.next_date ASC, r.id ASC");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], map_recurring)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Validates every field except `account_id` (checked separately via the
/// shared `finance_entries::validate_account`, which needs a DB lookup this
/// function deliberately stays free of). Returns the parsed `start_date` on
/// success so callers never re-parse it.
fn validate_recurring_fields(input: &RecurringExpenseInput) -> AppResult<NaiveDate> {
    if input.name.trim().is_empty() {
        return Err(AppError::Validation("Name cannot be empty".into()));
    }
    if input.amount_cents <= 0 {
        return Err(AppError::Validation("Amount must be greater than 0".into()));
    }
    if input.currency.trim().is_empty() {
        return Err(AppError::Validation("Currency cannot be empty".into()));
    }
    if !RECURRING_SCOPES.contains(&input.scope.as_str()) {
        return Err(AppError::Validation(format!("Invalid scope '{}' - must be 'personal' or 'business'", input.scope)));
    }
    if !RECURRING_FREQUENCIES.contains(&input.frequency.as_str()) {
        return Err(AppError::Validation(format!(
            "Invalid frequency '{}' - must be one of: {}",
            input.frequency,
            RECURRING_FREQUENCIES.join(", ")
        )));
    }
    NaiveDate::parse_from_str(input.start_date.trim(), "%Y-%m-%d")
        .map_err(|_| AppError::Validation(format!("Invalid start date '{}'", input.start_date)))
}

/// Adds `months` calendar months to `date`, clamping the day to the target
/// month's last valid day (e.g. 31 Jan + 1 month -> 28 or 29 Feb, never a
/// crash or an invalid rollover into March). A small, self-contained copy
/// of the same well-understood "last day of next month minus one day"
/// clamping technique dashboard.rs's own `months_ago` already uses
/// (backwards, for its own date-range presets) - written fresh here rather
/// than imported/shared so this feature never has a reason to touch
/// dashboard.rs at all (marko's own DO-NOT-TOUCH list).
fn add_months_clamped(date: NaiveDate, months: i32) -> NaiveDate {
    let total_months = date.year() * 12 + date.month() as i32 - 1 + months;
    let year = total_months.div_euclid(12);
    let month = (total_months.rem_euclid(12) + 1) as u32;
    let (next_year, next_month) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
    let first_of_next = NaiveDate::from_ymd_opt(next_year, next_month, 1).unwrap();
    let last_day_of_month = (first_of_next - Duration::days(1)).day();
    let day = date.day().min(last_day_of_month);
    NaiveDate::from_ymd_opt(year, month, day).unwrap()
}

/// The one place `frequency` turns into an actual date step - every caller
/// that advances `next_date` (`skip_recurring_expense_impl`,
/// `create_from_recurring_impl`) goes through this, so the two can never
/// disagree about what e.g. "quarterly" means.
fn advance_next_date(current: NaiveDate, frequency: &str) -> NaiveDate {
    match frequency {
        "weekly" => current + Duration::days(7),
        "monthly" => add_months_clamped(current, 1),
        "quarterly" => add_months_clamped(current, 3),
        "yearly" => add_months_clamped(current, 12),
        // Unreachable in practice - validate_recurring_fields (create) and
        // the CHECK constraint (migrations/016_finance_v2.sql) both already
        // restrict `frequency` to the four values above. Advancing by zero
        // rather than panicking is the safe fallback if that guarantee is
        // ever violated some other way (e.g. direct DB edit).
        _ => current,
    }
}

pub(crate) fn create_recurring_expense_impl(conn: &Connection, input: &RecurringExpenseInput) -> AppResult<RecurringExpense> {
    let start = validate_recurring_fields(input)?;
    let currency = input.currency.trim().to_ascii_uppercase();
    validate_account(conn, input.account_id, &currency)?;
    let name = input.name.trim();
    let note = normalize_optional(input.note.clone());
    let next_date = start.to_string();
    conn.execute(
        "INSERT INTO recurring_expenses(name, amount_cents, currency, scope, category_id, account_id, frequency, start_date, next_date, note)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        rusqlite::params![
            name,
            input.amount_cents,
            currency,
            input.scope,
            input.category_id,
            input.account_id,
            input.frequency,
            next_date,
            next_date,
            note
        ],
    )?;
    let id = conn.last_insert_rowid();
    fetch_recurring(conn, id)
}

#[tauri::command]
pub fn create_recurring_expense(state: State<AppState>, input: RecurringExpenseInput) -> AppResult<RecurringExpense> {
    let conn = state.db.lock().unwrap();
    create_recurring_expense_impl(&conn, &input)
}

/// Edits the template's own fields only - deliberately never touches
/// `next_date`/`is_active` (see `RecurringExpenseInput`'s own doc comment
/// in models.rs for why: those are runtime state owned by the dedicated
/// Create/Skip/Pause/Resume actions, never by a generic field edit).
pub(crate) fn update_recurring_expense_impl(conn: &Connection, id: i64, input: &RecurringExpenseInput) -> AppResult<RecurringExpense> {
    validate_recurring_fields(input)?;
    let currency = input.currency.trim().to_ascii_uppercase();
    validate_account(conn, input.account_id, &currency)?;
    let name = input.name.trim();
    let note = normalize_optional(input.note.clone());
    let updated = conn.execute(
        "UPDATE recurring_expenses SET name = ?1, amount_cents = ?2, currency = ?3, scope = ?4,
             category_id = ?5, account_id = ?6, frequency = ?7, start_date = ?8, note = ?9,
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
         WHERE id = ?10",
        rusqlite::params![
            name,
            input.amount_cents,
            currency,
            input.scope,
            input.category_id,
            input.account_id,
            input.frequency,
            input.start_date.trim(),
            note,
            id
        ],
    )?;
    if updated == 0 {
        return Err(AppError::NotFound(format!("Recurring expense #{id} not found")));
    }
    fetch_recurring(conn, id)
}

#[tauri::command]
pub fn update_recurring_expense(state: State<AppState>, id: i64, input: RecurringExpenseInput) -> AppResult<RecurringExpense> {
    let conn = state.db.lock().unwrap();
    update_recurring_expense_impl(&conn, id, &input)
}

#[tauri::command]
pub fn delete_recurring_expense(state: State<AppState>, id: i64) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    conn.execute("DELETE FROM recurring_expenses WHERE id = ?1", [id])?;
    Ok(())
}

pub(crate) fn set_recurring_active_impl(conn: &Connection, id: i64, is_active: bool) -> AppResult<RecurringExpense> {
    let updated = conn.execute(
        "UPDATE recurring_expenses SET is_active = ?1, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = ?2",
        rusqlite::params![is_active, id],
    )?;
    if updated == 0 {
        return Err(AppError::NotFound(format!("Recurring expense #{id} not found")));
    }
    fetch_recurring(conn, id)
}

#[tauri::command]
pub fn pause_recurring_expense(state: State<AppState>, id: i64) -> AppResult<RecurringExpense> {
    let conn = state.db.lock().unwrap();
    set_recurring_active_impl(&conn, id, false)
}

#[tauri::command]
pub fn resume_recurring_expense(state: State<AppState>, id: i64) -> AppResult<RecurringExpense> {
    let conn = state.db.lock().unwrap();
    set_recurring_active_impl(&conn, id, true)
}

fn parse_next_date(current: &RecurringExpense) -> AppResult<NaiveDate> {
    NaiveDate::parse_from_str(&current.next_date, "%Y-%m-%d")
        .map_err(|_| AppError::Db(format!("Corrupt next_date on recurring expense #{}", current.id)))
}

/// Advances `next_date` by one occurrence with no `FinanceEntry` created -
/// marko's own explicit second choice alongside Create (point 8).
pub(crate) fn skip_recurring_expense_impl(conn: &Connection, id: i64) -> AppResult<RecurringExpense> {
    let current = fetch_recurring(conn, id)?;
    if !current.is_active {
        return Err(AppError::Validation("This recurring expense is paused - resume it first.".into()));
    }
    let new_next = advance_next_date(parse_next_date(&current)?, &current.frequency).to_string();
    conn.execute(
        "UPDATE recurring_expenses SET next_date = ?1, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = ?2",
        rusqlite::params![new_next, id],
    )?;
    fetch_recurring(conn, id)
}

#[tauri::command]
pub fn skip_recurring_expense(state: State<AppState>, id: i64) -> AppResult<RecurringExpense> {
    let conn = state.db.lock().unwrap();
    skip_recurring_expense_impl(&conn, id)
}

/// Creates the real `FinanceEntry` this occurrence represents AND advances
/// `next_date`, together. `place` is set to the template's own name (e.g.
/// "Adobe", "Rent") - the entry's "Place / who" field is exactly the right
/// home for that; `note` carries over the template's own note, if any.
///
/// Runs inside a transaction (see the `#[tauri::command]` wrapper below) so
/// the entry and the advanced `next_date` are committed together or not at
/// all - if this were two independent writes, a failure between them could
/// leave a real entry paired with a stale `next_date`, and the very next
/// "Create" click would then produce a genuine duplicate. Wrapping both in
/// one transaction is what makes that impossible, the same pattern already
/// used throughout this codebase for any multi-row write (see e.g.
/// `orders::create_order`).
pub(crate) fn create_from_recurring_impl(conn: &Connection, id: i64) -> AppResult<CreateFromRecurringResult> {
    let current = fetch_recurring(conn, id)?;
    if !current.is_active {
        return Err(AppError::Validation("This recurring expense is paused - resume it first.".into()));
    }
    let entry_input = FinanceEntryInput {
        entry_type: "expense".to_string(),
        entry_date: current.next_date.clone(),
        amount_cents: current.amount_cents,
        currency: current.currency.clone(),
        scope: current.scope.clone(),
        category_id: current.category_id,
        account_id: current.account_id,
        place: Some(current.name.clone()),
        note: current.note.clone(),
    };
    let entry = crate::commands::finance_entries::create_finance_entry_impl(conn, &entry_input)?;

    let new_next = advance_next_date(parse_next_date(&current)?, &current.frequency).to_string();
    conn.execute(
        "UPDATE recurring_expenses SET next_date = ?1, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = ?2",
        rusqlite::params![new_next, id],
    )?;
    let recurring = fetch_recurring(conn, id)?;
    Ok(CreateFromRecurringResult { recurring, entry })
}

#[tauri::command]
pub fn create_from_recurring(state: State<AppState>, id: i64) -> AppResult<CreateFromRecurringResult> {
    let mut conn = state.db.lock().unwrap();
    let tx = conn.transaction()?;
    let result = create_from_recurring_impl(&tx, id)?;
    tx.commit()?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::finance_accounts::create_account_impl;
    use crate::db::test_conn;
    use crate::models::AccountInput;

    fn sample_input() -> RecurringExpenseInput {
        RecurringExpenseInput {
            name: "Adobe".to_string(),
            amount_cents: 2000,
            currency: "eur".to_string(),
            scope: "business".to_string(),
            category_id: None,
            account_id: None,
            frequency: "monthly".to_string(),
            start_date: "2026-09-02".to_string(),
            note: None,
        }
    }

    // --- add_months_clamped / advance_next_date (test scenario 9) --------

    #[test]
    fn add_months_clamped_handles_ordinary_months() {
        let d = NaiveDate::from_ymd_opt(2026, 6, 15).unwrap();
        assert_eq!(add_months_clamped(d, 1), NaiveDate::from_ymd_opt(2026, 7, 15).unwrap());
        assert_eq!(add_months_clamped(d, 3), NaiveDate::from_ymd_opt(2026, 9, 15).unwrap());
    }

    #[test]
    fn add_months_clamped_rolls_over_into_next_year() {
        let d = NaiveDate::from_ymd_opt(2026, 11, 30).unwrap();
        assert_eq!(add_months_clamped(d, 3), NaiveDate::from_ymd_opt(2027, 2, 28).unwrap());
    }

    #[test]
    fn add_months_clamped_clamps_31st_into_a_30_day_or_28_day_month() {
        let jan31 = NaiveDate::from_ymd_opt(2026, 1, 31).unwrap();
        assert_eq!(add_months_clamped(jan31, 1), NaiveDate::from_ymd_opt(2026, 2, 28).unwrap(), "2026 is not a leap year");
        let aug31 = NaiveDate::from_ymd_opt(2026, 8, 31).unwrap();
        assert_eq!(add_months_clamped(aug31, 3), NaiveDate::from_ymd_opt(2026, 11, 30).unwrap(), "November only has 30 days");
    }

    #[test]
    fn add_months_clamped_lands_on_feb_29_in_a_leap_year() {
        let jan31_2028 = NaiveDate::from_ymd_opt(2028, 1, 31).unwrap();
        assert_eq!(add_months_clamped(jan31_2028, 1), NaiveDate::from_ymd_opt(2028, 2, 29).unwrap(), "2028 is a leap year");
    }

    #[test]
    fn advance_next_date_covers_every_frequency() {
        let d = NaiveDate::from_ymd_opt(2026, 1, 31).unwrap();
        assert_eq!(advance_next_date(d, "weekly"), NaiveDate::from_ymd_opt(2026, 2, 7).unwrap());
        assert_eq!(advance_next_date(d, "monthly"), NaiveDate::from_ymd_opt(2026, 2, 28).unwrap());
        assert_eq!(advance_next_date(d, "quarterly"), NaiveDate::from_ymd_opt(2026, 4, 30).unwrap());
        assert_eq!(advance_next_date(d, "yearly"), NaiveDate::from_ymd_opt(2027, 1, 31).unwrap());
    }

    // --- create / validation ----------------------------------------------

    #[test]
    fn create_recurring_expense_starts_next_date_at_start_date() {
        let conn = test_conn();
        let created = create_recurring_expense_impl(&conn, &sample_input()).unwrap();
        assert_eq!(created.next_date, "2026-09-02");
        assert_eq!(created.start_date, "2026-09-02");
        assert!(created.is_active, "a brand new template is always active");
        assert_eq!(created.currency, "EUR", "must be uppercased, same convention as finance_entries");
    }

    #[test]
    fn create_recurring_expense_rejects_bad_fields() {
        let conn = test_conn();
        let mut empty_name = sample_input();
        empty_name.name = "  ".to_string();
        assert!(matches!(create_recurring_expense_impl(&conn, &empty_name).unwrap_err(), AppError::Validation(_)));

        let mut bad_amount = sample_input();
        bad_amount.amount_cents = 0;
        assert!(matches!(create_recurring_expense_impl(&conn, &bad_amount).unwrap_err(), AppError::Validation(_)));

        let mut bad_scope = sample_input();
        bad_scope.scope = "nonsense".to_string();
        assert!(matches!(create_recurring_expense_impl(&conn, &bad_scope).unwrap_err(), AppError::Validation(_)));

        let mut bad_frequency = sample_input();
        bad_frequency.frequency = "daily".to_string();
        assert!(matches!(create_recurring_expense_impl(&conn, &bad_frequency).unwrap_err(), AppError::Validation(_)));

        let mut bad_date = sample_input();
        bad_date.start_date = "not-a-date".to_string();
        assert!(matches!(create_recurring_expense_impl(&conn, &bad_date).unwrap_err(), AppError::Validation(_)));
    }

    #[test]
    fn create_recurring_expense_rejects_a_currency_mismatched_account() {
        let conn = test_conn();
        let usd_account = create_account_impl(
            &conn,
            &AccountInput { name: "PayPal".to_string(), account_type: "paypal".to_string(), currency: "USD".to_string(), opening_balance_cents: 0, is_active: true },
        )
        .unwrap();
        let mut input = sample_input(); // EUR
        input.account_id = Some(usd_account.id);
        let err = create_recurring_expense_impl(&conn, &input).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn update_recurring_expense_edits_fields_but_never_next_date_or_active() {
        let conn = test_conn();
        let created = create_recurring_expense_impl(&conn, &sample_input()).unwrap();
        // Advance state first, so we can prove update leaves it alone.
        let skipped = skip_recurring_expense_impl(&conn, created.id).unwrap();
        assert_ne!(skipped.next_date, created.next_date);

        let mut edit = sample_input();
        edit.name = "Adobe Creative Cloud".to_string();
        edit.amount_cents = 2500;
        let updated = update_recurring_expense_impl(&conn, created.id, &edit).unwrap();
        assert_eq!(updated.name, "Adobe Creative Cloud");
        assert_eq!(updated.amount_cents, 2500);
        assert_eq!(updated.next_date, skipped.next_date, "editing fields must never reset or move next_date");
        assert!(updated.is_active);
    }

    #[test]
    fn update_recurring_expense_rejects_a_missing_id() {
        let conn = test_conn();
        let err = update_recurring_expense_impl(&conn, 999_999, &sample_input()).unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    // --- pause / resume -----------------------------------------------------

    #[test]
    fn pause_then_resume_round_trips_is_active() {
        let conn = test_conn();
        let created = create_recurring_expense_impl(&conn, &sample_input()).unwrap();
        let paused = set_recurring_active_impl(&conn, created.id, false).unwrap();
        assert!(!paused.is_active);
        let resumed = set_recurring_active_impl(&conn, created.id, true).unwrap();
        assert!(resumed.is_active);
    }

    // --- skip (test scenario 9, next_date updates correctly) --------------

    #[test]
    fn skip_advances_next_date_by_one_occurrence_and_creates_no_entry() {
        let conn = test_conn();
        let created = create_recurring_expense_impl(&conn, &sample_input()).unwrap();
        let skipped = skip_recurring_expense_impl(&conn, created.id).unwrap();
        assert_eq!(skipped.next_date, "2026-10-02", "monthly from 2026-09-02");
        let entry_count: i64 = conn.query_row("SELECT COUNT(*) FROM finance_entries", [], |r| r.get(0)).unwrap();
        assert_eq!(entry_count, 0, "Skip must never create a transaction");
    }

    #[test]
    fn skip_rejects_a_paused_template() {
        let conn = test_conn();
        let created = create_recurring_expense_impl(&conn, &sample_input()).unwrap();
        set_recurring_active_impl(&conn, created.id, false).unwrap();
        let err = skip_recurring_expense_impl(&conn, created.id).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    // --- create_from_recurring (test scenario 8, no duplicate on repeated
    // opens) -----------------------------------------------------------------

    #[test]
    fn create_from_recurring_creates_a_matching_expense_entry() {
        let conn = test_conn();
        let created = create_recurring_expense_impl(&conn, &sample_input()).unwrap();
        let result = create_from_recurring_impl(&conn, created.id).unwrap();
        assert_eq!(result.entry.entry_type, "expense");
        assert_eq!(result.entry.entry_date, "2026-09-02", "dated for the occurrence that was due, not today");
        assert_eq!(result.entry.amount_cents, 2000);
        assert_eq!(result.entry.currency, "EUR");
        assert_eq!(result.entry.scope, "business");
        assert_eq!(result.entry.place.as_deref(), Some("Adobe"), "the template's own name becomes the entry's Place/who");
    }

    #[test]
    fn create_from_recurring_advances_next_date_so_a_second_click_never_duplicates_the_same_occurrence() {
        let conn = test_conn();
        let created = create_recurring_expense_impl(&conn, &sample_input()).unwrap();
        let first = create_from_recurring_impl(&conn, created.id).unwrap();
        assert_eq!(first.recurring.next_date, "2026-10-02");

        let second = create_from_recurring_impl(&conn, created.id).unwrap();
        assert_eq!(second.recurring.next_date, "2026-11-02");
        assert_ne!(first.entry.id, second.entry.id);
        assert_eq!(second.entry.entry_date, "2026-10-02", "the SECOND click logs the NEXT occurrence, not a repeat of the first");

        let entry_count: i64 = conn.query_row("SELECT COUNT(*) FROM finance_entries", [], |r| r.get(0)).unwrap();
        assert_eq!(entry_count, 2, "two distinct, deliberate clicks - two distinct entries, never more");
    }

    #[test]
    fn listing_recurring_expenses_repeatedly_never_creates_a_transaction() {
        // Simulates "reopening the app" - list is called on every load, and
        // must stay perfectly read-only no matter how many times it runs.
        let conn = test_conn();
        create_recurring_expense_impl(&conn, &sample_input()).unwrap();
        for _ in 0..5 {
            let sql = format!("{RECURRING_SELECT} ORDER BY r.next_date ASC, r.id ASC");
            let mut stmt = conn.prepare(&sql).unwrap();
            let _rows: Vec<RecurringExpense> = stmt.query_map([], map_recurring).unwrap().collect::<Result<Vec<_>, _>>().unwrap();
        }
        let entry_count: i64 = conn.query_row("SELECT COUNT(*) FROM finance_entries", [], |r| r.get(0)).unwrap();
        assert_eq!(entry_count, 0, "repeated 'app opens' (list calls) must never create a duplicate - or any - transaction");
    }

    #[test]
    fn create_from_recurring_rejects_a_paused_template() {
        let conn = test_conn();
        let created = create_recurring_expense_impl(&conn, &sample_input()).unwrap();
        set_recurring_active_impl(&conn, created.id, false).unwrap();
        let err = create_from_recurring_impl(&conn, created.id).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn delete_recurring_expense_removes_it() {
        let conn = test_conn();
        let created = create_recurring_expense_impl(&conn, &sample_input()).unwrap();
        conn.execute("DELETE FROM recurring_expenses WHERE id = ?1", [created.id]).unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM recurring_expenses", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn list_recurring_expenses_orders_soonest_first() {
        let conn = test_conn();
        let mut later = sample_input();
        later.start_date = "2026-12-01".to_string();
        later.name = "Later".to_string();
        let mut sooner = sample_input();
        sooner.start_date = "2026-09-01".to_string();
        sooner.name = "Sooner".to_string();
        create_recurring_expense_impl(&conn, &later).unwrap();
        create_recurring_expense_impl(&conn, &sooner).unwrap();

        let sql = format!("{RECURRING_SELECT} ORDER BY r.next_date ASC, r.id ASC");
        let mut stmt = conn.prepare(&sql).unwrap();
        let rows: Vec<RecurringExpense> = stmt.query_map([], map_recurring).unwrap().collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(rows[0].name, "Sooner");
        assert_eq!(rows[1].name, "Later");
    }
}

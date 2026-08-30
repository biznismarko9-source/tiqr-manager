//! 2.0.83: marko's personal + business money tracker ("Financie" in the
//! Slovak report, "Finance" in the app's own English UI - same convention as
//! Price Checker: the feature UI stays English like the rest of the app, the
//! explanation to marko is in Slovak). See migrations/015_finance.sql's doc
//! comment for the full design rationale, and models.rs's "Finance (2.0.83)"
//! section for why this module is named `finance_entries`, not `finance` -
//! that name is already `crate::finance`, the shared ticket-business P&L
//! calculation module, a completely different concept.
//!
//! Two lookup-list-shaped command sets, same CRUD shapes already used
//! elsewhere in this app:
//!   - finance categories: same "kind" + "color_slot" combination as
//!     `lookups::{list_platforms,create_platform}` (kind) and
//!     `event_categories` (color_slot) - see `create_finance_category_impl`.
//!   - finance entries: same "one Input struct, not flat arguments" shape as
//!     `orders::{create_order,update_order}` (`OrderInput`) - see
//!     `FinanceEntryInput` in models.rs.
//!
//! Deliberately no server-side filtering/aggregation here (contrast
//! `dashboard.rs`, which computes revenue/profit/time-series server-side
//! because Orders/Tickets/Sales need real joins across several tables).
//! `finance_entries` is one flat table with no children, and this is a
//! manual-entry-only ledger (marko's own answer #4 - no bank connection), so
//! realistic data volume is small - `list_finance_entries` always returns
//! every entry, and Finance.tsx does all period/scope/category/search
//! filtering and all chart bucketing client-side from that one array.
//! Currency conversion reuses `commands::currency::convert_currency` as-is
//! (Finance.tsx calls it directly, then calls `update_finance_entry` per
//! converted entry) - no bulk-convert command needed here either.

use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::models::{FinanceCategory, FinanceEntry, FinanceEntryInput};
use rusqlite::{Connection, OptionalExtension, Row};
use tauri::State;

// --- Categories --------------------------------------------------------

fn map_finance_category(row: &Row) -> rusqlite::Result<FinanceCategory> {
    Ok(FinanceCategory {
        id: row.get("id")?,
        name: row.get("name")?,
        kind: row.get("kind")?,
        color_slot: row.get("color_slot")?,
        is_demo: row.get("is_demo")?,
        created_at: row.get("created_at")?,
    })
}

const FINANCE_CATEGORY_KINDS: [&str; 3] = ["expense", "income", "both"];

#[tauri::command]
pub fn list_finance_categories(state: State<AppState>) -> AppResult<Vec<FinanceCategory>> {
    let conn = state.db.lock().unwrap();
    let mut stmt = conn.prepare("SELECT * FROM finance_categories ORDER BY name COLLATE NOCASE")?;
    let rows = stmt.query_map([], map_finance_category)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Core logic behind `create_finance_category` - same impl+wrapper split as
/// `event_categories::create_event_category_impl`, and the same
/// `MAX(color_slot)+1` assignment (see that function's own doc comment for
/// why this is always safe against races in this app - one mutex-guarded
/// connection, see `AppState`).
pub(crate) fn create_finance_category_impl(conn: &Connection, name: &str, kind: &str) -> AppResult<FinanceCategory> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::Validation("Category name cannot be empty".into()));
    }
    if !FINANCE_CATEGORY_KINDS.contains(&kind) {
        return Err(AppError::Validation(format!(
            "Invalid category kind '{kind}' - must be 'expense', 'income' or 'both'"
        )));
    }
    let next_slot: i64 = conn.query_row(
        "SELECT COALESCE(MAX(color_slot), -1) + 1 FROM finance_categories",
        [],
        |r| r.get(0),
    )?;
    conn.execute(
        "INSERT INTO finance_categories(name, kind, color_slot) VALUES (?1, ?2, ?3)",
        rusqlite::params![name, kind, next_slot],
    )
    .map_err(|e| match &e {
        rusqlite::Error::SqliteFailure(_, Some(m)) if m.contains("UNIQUE") => {
            AppError::Validation(format!("Category '{name}' already exists"))
        }
        _ => AppError::from(e),
    })?;
    let id = conn.last_insert_rowid();
    Ok(conn.query_row("SELECT * FROM finance_categories WHERE id = ?1", [id], map_finance_category)?)
}

#[tauri::command]
pub fn create_finance_category(state: State<AppState>, name: String, kind: String) -> AppResult<FinanceCategory> {
    let conn = state.db.lock().unwrap();
    create_finance_category_impl(&conn, &name, &kind)
}

/// Plain blind delete, same as `lookups::delete_platform`/`delete_supplier` -
/// unlike `event_categories::delete_event_category`, there is no legacy
/// free-text mirror column to clear here, so the FK's own
/// `ON DELETE SET NULL` (migrations/015_finance.sql) is the whole story: any
/// entry using this category just loses the label, exactly like an order
/// losing a deleted platform's label today.
#[tauri::command]
pub fn delete_finance_category(state: State<AppState>, id: i64) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    conn.execute("DELETE FROM finance_categories WHERE id = ?1", [id])?;
    Ok(())
}

// --- Entries -------------------------------------------------------------

const FINANCE_ENTRY_TYPES: [&str; 2] = ["income", "expense"];
const FINANCE_ENTRY_SCOPES: [&str; 2] = ["personal", "business"];

/// Every finance-entry read goes through this same SELECT (list, and the
/// create/update "read the row back" step) so the two can never disagree
/// about which columns/joins a `FinanceEntry` is built from - same
/// discipline as every other `map_x`/shared-SELECT pair in this codebase.
const FINANCE_ENTRY_SELECT: &str = "SELECT e.id, e.entry_type, e.entry_date, e.amount_cents, e.currency, e.scope,
    e.category_id, c.name AS category_name, c.color_slot AS category_color_slot,
    e.account_id, a.name AS account_name,
    e.place, e.note, e.is_demo, e.created_at, e.updated_at
    FROM finance_entries e
    LEFT JOIN finance_categories c ON c.id = e.category_id
    LEFT JOIN accounts a ON a.id = e.account_id";

fn map_finance_entry(row: &Row) -> rusqlite::Result<FinanceEntry> {
    Ok(FinanceEntry {
        id: row.get("id")?,
        entry_type: row.get("entry_type")?,
        entry_date: row.get("entry_date")?,
        amount_cents: row.get("amount_cents")?,
        currency: row.get("currency")?,
        scope: row.get("scope")?,
        category_id: row.get("category_id")?,
        category_name: row.get("category_name")?,
        category_color_slot: row.get("category_color_slot")?,
        account_id: row.get("account_id")?,
        account_name: row.get("account_name")?,
        place: row.get("place")?,
        note: row.get("note")?,
        is_demo: row.get("is_demo")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

/// 2.1.0: shared by `create_finance_entry_impl`/`update_finance_entry_impl`
/// and, via re-export, `finance_recurring::create_from_recurring_impl` (a
/// recurring template's own `account_id` needs the exact same guarantees
/// before it can be copied onto a new entry). Two checks in one place:
///
/// 1. The account must currently exist - a clear, specific error before
///    ever reaching the database, backed up by the FK constraint itself
///    (`ON DELETE SET NULL` on the column, but there is no `ON INSERT`/
///    `ON UPDATE` equivalent - a stale/invalid id would otherwise fail with
///    the generic "other records still reference it" message from
///    `AppError`'s own `rusqlite::Error` conversion, which reads backwards
///    for this direction of the same constraint).
/// 2. The entry's own currency must match that account's currency - an
///    entry linked to an account is a movement of THAT account's own
///    balance (see `commands::finance_accounts::list_accounts`'s balance
///    aggregate, which sums `amount_cents` straight across every linked
///    entry), so a mismatched currency would silently blend two currencies
///    into one balance number - exactly what marko's own point 14 says to
///    never do. An entry with no account is unaffected (same as today) and
///    can still be logged in any currency.
///
/// `currency` must already be the trimmed+uppercased value that will
/// actually be stored, so this must run after that normalization step, not
/// before it.
pub(crate) fn validate_account(conn: &Connection, account_id: Option<i64>, currency: &str) -> AppResult<()> {
    let Some(id) = account_id else { return Ok(()) };
    let account_currency: Option<String> = conn
        .query_row("SELECT currency FROM accounts WHERE id = ?1", [id], |r| r.get(0))
        .optional()?;
    let Some(account_currency) = account_currency else {
        return Err(AppError::Validation(format!("Account #{id} not found")));
    };
    if account_currency != currency {
        return Err(AppError::Validation(format!(
            "This account uses {account_currency}, not {currency} - pick a matching currency or leave the account unset."
        )));
    }
    Ok(())
}

/// Newest first (by date, then by id as a tie-breaker for same-day entries -
/// whichever was entered last shows first), same ordering convention as
/// every other "recent activity" list in this app.
#[tauri::command]
pub fn list_finance_entries(state: State<AppState>) -> AppResult<Vec<FinanceEntry>> {
    let conn = state.db.lock().unwrap();
    let sql = format!("{FINANCE_ENTRY_SELECT} ORDER BY e.entry_date DESC, e.id DESC");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], map_finance_entry)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Blank/whitespace-only optional text collapses to `None` rather than being
/// stored as an empty string - keeps `place`/`note` genuinely absent (not
/// just visually empty) when the user leaves them blank, same spirit as
/// every required-text field in this app already being `.trim()`ed.
pub(crate) fn normalize_optional(s: Option<String>) -> Option<String> {
    s.and_then(|v| {
        let t = v.trim().to_string();
        if t.is_empty() {
            None
        } else {
            Some(t)
        }
    })
}

fn validate_entry_fields(input: &FinanceEntryInput) -> AppResult<()> {
    if !FINANCE_ENTRY_TYPES.contains(&input.entry_type.as_str()) {
        return Err(AppError::Validation(format!(
            "Invalid entry type '{}' - must be 'income' or 'expense'",
            input.entry_type
        )));
    }
    if input.entry_date.trim().is_empty() {
        return Err(AppError::Validation("Date cannot be empty".into()));
    }
    if input.amount_cents < 0 {
        return Err(AppError::Validation("Amount cannot be negative".into()));
    }
    if input.currency.trim().is_empty() {
        return Err(AppError::Validation("Currency cannot be empty".into()));
    }
    if !FINANCE_ENTRY_SCOPES.contains(&input.scope.as_str()) {
        return Err(AppError::Validation(format!(
            "Invalid scope '{}' - must be 'personal' or 'business'",
            input.scope
        )));
    }
    Ok(())
}

/// Core logic behind `create_finance_entry` - split out for direct
/// unit-testability, same "impl function + thin `#[tauri::command]` wrapper"
/// pattern used throughout this codebase.
pub(crate) fn create_finance_entry_impl(conn: &Connection, input: &FinanceEntryInput) -> AppResult<FinanceEntry> {
    validate_entry_fields(input)?;
    let currency = input.currency.trim().to_ascii_uppercase();
    validate_account(conn, input.account_id, &currency)?;
    let place = normalize_optional(input.place.clone());
    let note = normalize_optional(input.note.clone());
    conn.execute(
        "INSERT INTO finance_entries(entry_type, entry_date, amount_cents, currency, scope, category_id, account_id, place, note)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            input.entry_type,
            input.entry_date,
            input.amount_cents,
            currency,
            input.scope,
            input.category_id,
            input.account_id,
            place,
            note
        ],
    )?;
    let id = conn.last_insert_rowid();
    let sql = format!("{FINANCE_ENTRY_SELECT} WHERE e.id = ?1");
    Ok(conn.query_row(&sql, [id], map_finance_entry)?)
}

#[tauri::command]
pub fn create_finance_entry(state: State<AppState>, input: FinanceEntryInput) -> AppResult<FinanceEntry> {
    let conn = state.db.lock().unwrap();
    create_finance_entry_impl(&conn, &input)
}

/// Core logic behind `update_finance_entry` - full-row update (every editable
/// field at once), same shape as `orders::update_order_impl`. Returns
/// `AppError::NotFound` for a missing id rather than silently doing nothing,
/// same convention as `lookups::update_platform_kind_impl`.
pub(crate) fn update_finance_entry_impl(conn: &Connection, id: i64, input: &FinanceEntryInput) -> AppResult<FinanceEntry> {
    validate_entry_fields(input)?;
    let currency = input.currency.trim().to_ascii_uppercase();
    validate_account(conn, input.account_id, &currency)?;
    let place = normalize_optional(input.place.clone());
    let note = normalize_optional(input.note.clone());
    let updated = conn.execute(
        "UPDATE finance_entries SET entry_type = ?1, entry_date = ?2, amount_cents = ?3, currency = ?4,
             scope = ?5, category_id = ?6, account_id = ?7, place = ?8, note = ?9, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
         WHERE id = ?10",
        rusqlite::params![
            input.entry_type,
            input.entry_date,
            input.amount_cents,
            currency,
            input.scope,
            input.category_id,
            input.account_id,
            place,
            note,
            id
        ],
    )?;
    if updated == 0 {
        return Err(AppError::NotFound(format!("Finance entry #{id} not found")));
    }
    let sql = format!("{FINANCE_ENTRY_SELECT} WHERE e.id = ?1");
    Ok(conn.query_row(&sql, [id], map_finance_entry)?)
}

#[tauri::command]
pub fn update_finance_entry(state: State<AppState>, id: i64, input: FinanceEntryInput) -> AppResult<FinanceEntry> {
    let conn = state.db.lock().unwrap();
    update_finance_entry_impl(&conn, id, &input)
}

#[tauri::command]
pub fn delete_finance_entry(state: State<AppState>, id: i64) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    conn.execute("DELETE FROM finance_entries WHERE id = ?1", [id])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_conn;

    fn sample_input() -> FinanceEntryInput {
        FinanceEntryInput {
            entry_type: "expense".to_string(),
            entry_date: "2026-08-30".to_string(),
            amount_cents: 1250,
            currency: "eur".to_string(),
            scope: "personal".to_string(),
            category_id: None,
            account_id: None,
            place: Some("  Tesco  ".to_string()),
            note: Some("  ".to_string()),
        }
    }

    fn create_test_account(conn: &Connection, name: &str) -> i64 {
        conn.execute(
            "INSERT INTO accounts(name, account_type, currency, opening_balance_cents) VALUES (?1, 'bank', 'EUR', 0)",
            [name],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    // migrations/015_finance.sql seeds 11 categories (slots 0-10) in every
    // freshly-migrated connection, including test_conn()'s - tests below
    // account for that rather than assuming a blank table.

    #[test]
    fn seed_migration_creates_the_eleven_expected_categories() {
        let conn = test_conn();
        let mut stmt = conn
            .prepare("SELECT name, kind, color_slot FROM finance_categories ORDER BY color_slot")
            .unwrap();
        let rows: Vec<(String, String, i64)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            rows,
            vec![
                ("Jedlo a nákupy".to_string(), "expense".to_string(), 0),
                ("Bývanie".to_string(), "expense".to_string(), 1),
                ("Doprava".to_string(), "expense".to_string(), 2),
                ("Zábava".to_string(), "expense".to_string(), 3),
                ("Zdravie".to_string(), "expense".to_string(), 4),
                ("Predplatné".to_string(), "expense".to_string(), 5),
                ("Biznis náklady".to_string(), "expense".to_string(), 6),
                ("Iné výdavky".to_string(), "expense".to_string(), 7),
                ("Výplata".to_string(), "income".to_string(), 8),
                ("Biznis príjem".to_string(), "income".to_string(), 9),
                ("Iné príjmy".to_string(), "income".to_string(), 10),
            ]
        );
    }

    #[test]
    fn create_finance_category_assigns_the_next_free_color_slot() {
        let conn = test_conn();
        let created = create_finance_category_impl(&conn, "Cestovanie", "expense").unwrap();
        assert_eq!(created.name, "Cestovanie");
        assert_eq!(created.color_slot, 11, "11 seeded categories already occupy slots 0-10");

        let second = create_finance_category_impl(&conn, "Darčeky", "both").unwrap();
        assert_eq!(second.color_slot, 12);
    }

    #[test]
    fn create_finance_category_rejects_empty_name() {
        let conn = test_conn();
        let err = create_finance_category_impl(&conn, "   ", "expense").unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn create_finance_category_rejects_a_duplicate_name() {
        let conn = test_conn();
        let err = create_finance_category_impl(&conn, "Výplata", "income").unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn create_finance_category_rejects_an_invalid_kind() {
        let conn = test_conn();
        let err = create_finance_category_impl(&conn, "Nový", "nonsense").unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn create_finance_category_allows_every_valid_kind() {
        let conn = test_conn();
        for (name, kind) in [("A", "expense"), ("B", "income"), ("C", "both")] {
            let created = create_finance_category_impl(&conn, name, kind).unwrap();
            assert_eq!(created.kind, kind);
        }
    }

    #[test]
    fn delete_finance_category_leaves_existing_entries_with_no_category() {
        let conn = test_conn();
        let category_id: i64 = conn
            .query_row("SELECT id FROM finance_categories WHERE name = 'Doprava'", [], |r| r.get(0))
            .unwrap();
        let mut input = sample_input();
        input.category_id = Some(category_id);
        let entry = create_finance_entry_impl(&conn, &input).unwrap();
        assert_eq!(entry.category_name.as_deref(), Some("Doprava"));

        conn.execute("DELETE FROM finance_categories WHERE id = ?1", [category_id]).unwrap();

        let reloaded = conn
            .query_row(
                &format!("{FINANCE_ENTRY_SELECT} WHERE e.id = ?1"),
                [entry.id],
                map_finance_entry,
            )
            .unwrap();
        assert_eq!(reloaded.category_id, None, "ON DELETE SET NULL must clear category_id");
        assert_eq!(reloaded.category_name, None);
    }

    // --- 2.1.0: account_id ---------------------------------------------

    #[test]
    fn create_finance_entry_joins_account_name() {
        let conn = test_conn();
        let account_id = create_test_account(&conn, "Revolut");
        let mut input = sample_input();
        input.account_id = Some(account_id);
        let created = create_finance_entry_impl(&conn, &input).unwrap();
        assert_eq!(created.account_id, Some(account_id));
        assert_eq!(created.account_name.as_deref(), Some("Revolut"));
    }

    #[test]
    fn create_finance_entry_allows_a_null_account() {
        let conn = test_conn();
        let created = create_finance_entry_impl(&conn, &sample_input()).unwrap();
        assert_eq!(created.account_id, None);
        assert_eq!(created.account_name, None);
    }

    #[test]
    fn create_finance_entry_rejects_a_currency_mismatched_account() {
        let conn = test_conn();
        conn.execute(
            "INSERT INTO accounts(name, account_type, currency, opening_balance_cents) VALUES ('PayPal USD', 'paypal', 'USD', 0)",
            [],
        )
        .unwrap();
        let account_id = conn.last_insert_rowid();
        let mut input = sample_input();
        input.currency = "eur".to_string(); // sample_input's entry currency
        input.account_id = Some(account_id); // but the account is USD
        let err = create_finance_entry_impl(&conn, &input).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)), "an entry's currency must match its linked account's currency");
    }

    #[test]
    fn create_finance_entry_rejects_a_nonexistent_account() {
        let conn = test_conn();
        let mut input = sample_input();
        input.account_id = Some(999_999);
        let err = create_finance_entry_impl(&conn, &input).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn update_finance_entry_rejects_a_nonexistent_account() {
        let conn = test_conn();
        let created = create_finance_entry_impl(&conn, &sample_input()).unwrap();
        let mut bad_input = sample_input();
        bad_input.account_id = Some(999_999);
        let err = update_finance_entry_impl(&conn, created.id, &bad_input).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn delete_account_leaves_existing_entries_with_no_account() {
        let conn = test_conn();
        let account_id = create_test_account(&conn, "Cash");
        let mut input = sample_input();
        input.account_id = Some(account_id);
        let entry = create_finance_entry_impl(&conn, &input).unwrap();
        assert_eq!(entry.account_name.as_deref(), Some("Cash"));

        conn.execute("DELETE FROM accounts WHERE id = ?1", [account_id]).unwrap();

        let reloaded = conn
            .query_row(
                &format!("{FINANCE_ENTRY_SELECT} WHERE e.id = ?1"),
                [entry.id],
                map_finance_entry,
            )
            .unwrap();
        assert_eq!(reloaded.account_id, None, "ON DELETE SET NULL must clear account_id");
        assert_eq!(reloaded.account_name, None);
    }

    #[test]
    fn create_finance_entry_rejects_an_invalid_entry_type() {
        let conn = test_conn();
        let mut input = sample_input();
        input.entry_type = "nonsense".to_string();
        let err = create_finance_entry_impl(&conn, &input).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn create_finance_entry_rejects_an_invalid_scope() {
        let conn = test_conn();
        let mut input = sample_input();
        input.scope = "nonsense".to_string();
        let err = create_finance_entry_impl(&conn, &input).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn create_finance_entry_rejects_a_negative_amount() {
        let conn = test_conn();
        let mut input = sample_input();
        input.amount_cents = -1;
        let err = create_finance_entry_impl(&conn, &input).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn create_finance_entry_rejects_an_empty_date() {
        let conn = test_conn();
        let mut input = sample_input();
        input.entry_date = "  ".to_string();
        let err = create_finance_entry_impl(&conn, &input).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn create_finance_entry_uppercases_currency_and_normalizes_blank_optional_text() {
        let conn = test_conn();
        let created = create_finance_entry_impl(&conn, &sample_input()).unwrap();
        assert_eq!(created.currency, "EUR");
        assert_eq!(created.place.as_deref(), Some("Tesco"), "must be trimmed");
        assert_eq!(created.note, None, "whitespace-only note must collapse to None");
    }

    #[test]
    fn create_finance_entry_joins_category_name_and_color_slot() {
        let conn = test_conn();
        let category_id: i64 = conn
            .query_row("SELECT id FROM finance_categories WHERE name = 'Zábava'", [], |r| r.get(0))
            .unwrap();
        let mut input = sample_input();
        input.category_id = Some(category_id);
        let created = create_finance_entry_impl(&conn, &input).unwrap();
        assert_eq!(created.category_name.as_deref(), Some("Zábava"));
        assert_eq!(created.category_color_slot, Some(3));
    }

    #[test]
    fn create_finance_entry_allows_a_null_category() {
        let conn = test_conn();
        let created = create_finance_entry_impl(&conn, &sample_input()).unwrap();
        assert_eq!(created.category_id, None);
        assert_eq!(created.category_name, None);
    }

    #[test]
    fn update_finance_entry_changes_an_existing_entry() {
        let conn = test_conn();
        let created = create_finance_entry_impl(&conn, &sample_input()).unwrap();
        let mut updated_input = sample_input();
        updated_input.entry_type = "income".to_string();
        updated_input.amount_cents = 5000;
        updated_input.scope = "business".to_string();
        let updated = update_finance_entry_impl(&conn, created.id, &updated_input).unwrap();
        assert_eq!(updated.entry_type, "income");
        assert_eq!(updated.amount_cents, 5000);
        assert_eq!(updated.scope, "business");
        // Not asserting updated_at != created_at here: strftime's millisecond
        // precision means a create immediately followed by an update in the
        // same test can legitimately land in the same millisecond (observed
        // flakily in practice) - the column is still written by the same
        // `strftime('%Y-%m-%dT%H:%M:%fZ','now')` expression every other
        // `updated_at` column in this app uses, exercised by the UPDATE
        // statement above; this test's job is the field values, not clock
        // resolution.
    }

    #[test]
    fn update_finance_entry_rejects_a_missing_entry() {
        let conn = test_conn();
        let err = update_finance_entry_impl(&conn, 999_999, &sample_input()).unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[test]
    fn update_finance_entry_rejects_invalid_fields_same_as_create() {
        let conn = test_conn();
        let created = create_finance_entry_impl(&conn, &sample_input()).unwrap();
        let mut bad_input = sample_input();
        bad_input.currency = "".to_string();
        let err = update_finance_entry_impl(&conn, created.id, &bad_input).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn list_finance_entries_orders_newest_first() {
        let conn = test_conn();
        let mut older = sample_input();
        older.entry_date = "2026-01-01".to_string();
        let mut newer = sample_input();
        newer.entry_date = "2026-06-15".to_string();
        create_finance_entry_impl(&conn, &older).unwrap();
        create_finance_entry_impl(&conn, &newer).unwrap();

        let mut stmt = conn
            .prepare(&format!("{FINANCE_ENTRY_SELECT} ORDER BY e.entry_date DESC, e.id DESC"))
            .unwrap();
        let rows: Vec<FinanceEntry> = stmt
            .query_map([], map_finance_entry)
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].entry_date, "2026-06-15", "newest first");
        assert_eq!(rows[1].entry_date, "2026-01-01");
    }

    #[test]
    fn delete_finance_entry_removes_it() {
        let conn = test_conn();
        let created = create_finance_entry_impl(&conn, &sample_input()).unwrap();
        conn.execute("DELETE FROM finance_entries WHERE id = ?1", [created.id]).unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM finance_entries", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 0);
    }
}

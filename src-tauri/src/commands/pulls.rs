//! Pull (1.9.7): buying tickets on someone else's behalf for a fee. See
//! migrations/005_pulls.sql for the full feature rationale and why this is
//! deliberately standalone (no FK to events/orders/tickets/sales, never
//! folded into finance.rs/Dashboard numbers). 1.9.8 (migrations/
//! 006_pulls_seat_fields.sql) reshaped the old single `seats` free-text
//! field into `section`/`row_label`/`seat`, mirroring `Ticket`'s own shape,
//! and dropped the manual `transfer_deadline` input in favor of a
//! client-side "N days before the event" warning computed from `event_date`
//! (see `Pull.transfer_deadline`'s doc comment in models.rs for why the
//! column itself is still kept around, just unused going forward).

use crate::codes;
use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::models::{Pull, PullEditInput, PullInput};
use rusqlite::{params, Connection, OptionalExtension, Row};
use tauri::State;

// Same safety cap and rationale as every other unfiltered list view in this
// app (see e.g. commands/tickets.rs::LIST_CAP) - ordinary use never gets
// close to it.
const LIST_CAP: i64 = 5000;

const BASE_SQL: &str = "
    SELECT p.id, p.code, p.buyer_name, p.event_name, p.event_date, p.quantity,
      p.platform_id, pl.name as platform_name, p.section, p.row_label, p.seat,
      p.more_info, p.price_cents, p.currency, p.transfer_deadline, p.transfer_done,
      p.transfer_done_at, p.is_demo, p.created_at, p.updated_at
    FROM pulls p
    LEFT JOIN platforms pl ON pl.id = p.platform_id
";

fn map_pull(row: &Row) -> rusqlite::Result<Pull> {
    Ok(Pull {
        id: row.get("id")?,
        code: row.get("code")?,
        buyer_name: row.get("buyer_name")?,
        event_name: row.get("event_name")?,
        event_date: row.get("event_date")?,
        quantity: row.get("quantity")?,
        platform_id: row.get("platform_id")?,
        platform_name: row.get("platform_name")?,
        section: row.get("section")?,
        row_label: row.get("row_label")?,
        seat: row.get("seat")?,
        more_info: row.get("more_info")?,
        price_cents: row.get("price_cents")?,
        currency: row.get("currency")?,
        transfer_deadline: row.get("transfer_deadline")?,
        transfer_done: row.get("transfer_done")?,
        transfer_done_at: row.get("transfer_done_at")?,
        is_demo: row.get("is_demo")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn fetch_one(conn: &Connection, id: i64) -> AppResult<Pull> {
    let sql = format!("{BASE_SQL} WHERE p.id = ?1");
    conn.query_row(&sql, [id], map_pull)
        .map_err(|_| AppError::NotFound(format!("Pull #{id} not found")))
}

/// Free-text search across every field marko would actually recognize a
/// pull by (buyer, event, own code, platform, section/row/seat, more info) -
/// same `LIKE` OR-chain convention as list_orders_impl's search, just
/// without a semi-join since pulls has no child rows to search through.
fn list_pulls_impl(conn: &Connection, search: Option<String>, transfer_done: Option<bool>) -> AppResult<Vec<Pull>> {
    let mut sql = format!("{BASE_SQL} WHERE 1=1");
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![];

    if let Some(done) = transfer_done {
        sql.push_str(" AND p.transfer_done = ?");
        params_vec.push(Box::new(done as i64));
    }
    if let Some(q) = search.as_deref() {
        let q = q.trim();
        if !q.is_empty() {
            sql.push_str(
                " AND (p.buyer_name LIKE ? OR p.event_name LIKE ? OR p.code LIKE ? OR pl.name LIKE ? \
                 OR p.section LIKE ? OR p.row_label LIKE ? OR p.seat LIKE ? OR p.more_info LIKE ?)",
            );
            let like = format!("%{q}%");
            for _ in 0..8 {
                params_vec.push(Box::new(like.clone()));
            }
        }
    }
    sql.push_str(&format!(" ORDER BY p.created_at DESC, p.id DESC LIMIT {LIST_CAP}"));

    let mut stmt = conn.prepare(&sql)?;
    let param_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
    let rows = stmt.query_map(param_refs.as_slice(), map_pull)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[tauri::command]
pub fn list_pulls(state: State<AppState>, search: Option<String>, transfer_done: Option<bool>) -> AppResult<Vec<Pull>> {
    let conn = state.db.lock().unwrap();
    list_pulls_impl(&conn, search, transfer_done)
}

#[tauri::command]
pub fn get_pull(state: State<AppState>, id: i64) -> AppResult<Pull> {
    let conn = state.db.lock().unwrap();
    fetch_one(&conn, id)
}

fn validate_pull_fields(
    buyer_name: &str,
    event_name: &str,
    quantity: i64,
    price_cents: i64,
    currency: &str,
) -> AppResult<()> {
    if buyer_name.trim().is_empty() {
        return Err(AppError::Validation("Buyer name is required".into()));
    }
    if event_name.trim().is_empty() {
        return Err(AppError::Validation("Event name is required".into()));
    }
    if quantity <= 0 {
        return Err(AppError::Validation("Quantity must be at least 1".into()));
    }
    if quantity > 1000 {
        return Err(AppError::Validation("Quantity is unreasonably large".into()));
    }
    if price_cents < 0 {
        return Err(AppError::Validation("Price cannot be negative".into()));
    }
    if currency.trim().is_empty() {
        return Err(AppError::Validation("Currency is required".into()));
    }
    Ok(())
}

pub(crate) fn create_pull_impl(conn: &Connection, input: &PullInput, is_demo: bool) -> AppResult<Pull> {
    validate_pull_fields(
        &input.buyer_name,
        &input.event_name,
        input.quantity,
        input.price_cents,
        &input.currency,
    )?;
    let code = codes::next_code(conn, "pull", "PULL")?;
    conn.execute(
        "INSERT INTO pulls (code, buyer_name, event_name, event_date, quantity, platform_id,
           section, row_label, seat, more_info, price_cents, currency, is_demo)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
        params![
            code,
            input.buyer_name.trim(),
            input.event_name.trim(),
            input.event_date,
            input.quantity,
            input.platform_id,
            input.section,
            input.row_label,
            input.seat,
            input.more_info,
            input.price_cents,
            input.currency,
            is_demo as i64,
        ],
    )?;
    let id = conn.last_insert_rowid();
    fetch_one(conn, id)
}

#[tauri::command]
pub fn create_pull(state: State<AppState>, input: PullInput) -> AppResult<Pull> {
    let conn = state.db.lock().unwrap();
    create_pull_impl(&conn, &input, false)
}

/// Full-edit path (buyer/event/quantity/platform/section/row/seat/more info/
/// price/currency, AND transfer_done - see `PullEditInput`'s doc comment for
/// why the checkbox is correctable here too). `transfer_done_at` is kept
/// consistent with whatever `transfer_done` ends up being via the same
/// three-way rule `set_pull_transfer_done_impl` uses below: only touched on
/// an actual false->true or true->false flip, left exactly as it was on a
/// plain re-save. Spliced as a literal SQL fragment (never user input, one
/// of exactly 3 hardcoded strings this function chooses) rather than a bound
/// parameter, so every `?N` placeholder below is still used exactly once -
/// same convention as every other query in this codebase.
pub(crate) fn update_pull_impl(conn: &Connection, id: i64, input: &PullEditInput) -> AppResult<Pull> {
    validate_pull_fields(
        &input.buyer_name,
        &input.event_name,
        input.quantity,
        input.price_cents,
        &input.currency,
    )?;

    let was_done: bool = conn
        .query_row("SELECT transfer_done FROM pulls WHERE id = ?1", [id], |r| {
            r.get(0)
        })
        .optional()?
        .ok_or_else(|| AppError::NotFound(format!("Pull #{id} not found")))?;

    let transfer_done_at_sql = match (was_done, input.transfer_done) {
        (false, true) => "strftime('%Y-%m-%dT%H:%M:%fZ','now')",
        (true, false) => "NULL",
        _ => "transfer_done_at",
    };

    let sql = format!(
        "UPDATE pulls SET
            buyer_name = ?1, event_name = ?2, event_date = ?3, quantity = ?4,
            platform_id = ?5, section = ?6, row_label = ?7, seat = ?8, more_info = ?9,
            price_cents = ?10, currency = ?11, transfer_done = ?12,
            transfer_done_at = {transfer_done_at_sql},
            updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
         WHERE id = ?13"
    );
    let updated = conn.execute(
        &sql,
        params![
            input.buyer_name.trim(),
            input.event_name.trim(),
            input.event_date,
            input.quantity,
            input.platform_id,
            input.section,
            input.row_label,
            input.seat,
            input.more_info,
            input.price_cents,
            input.currency,
            input.transfer_done as i64,
            id,
        ],
    )?;
    if updated == 0 {
        return Err(AppError::NotFound(format!("Pull #{id} not found")));
    }
    fetch_one(conn, id)
}

#[tauri::command]
pub fn update_pull(state: State<AppState>, id: i64, input: PullEditInput) -> AppResult<Pull> {
    let conn = state.db.lock().unwrap();
    update_pull_impl(&conn, id, &input)
}

/// Dedicated quick-action behind the Pulls list's inline "Transfer done"
/// checkbox - flips just that one field (plus its timestamp) without
/// requiring the full edit form to be opened. Shares the exact same
/// three-way `transfer_done_at` rule as `update_pull_impl` above (see its
/// doc comment) so the two code paths can never disagree about what the
/// timestamp should be after either one runs.
pub(crate) fn set_pull_transfer_done_impl(conn: &Connection, id: i64, done: bool) -> AppResult<Pull> {
    let was_done: bool = conn
        .query_row("SELECT transfer_done FROM pulls WHERE id = ?1", [id], |r| {
            r.get(0)
        })
        .optional()?
        .ok_or_else(|| AppError::NotFound(format!("Pull #{id} not found")))?;

    let transfer_done_at_sql = match (was_done, done) {
        (false, true) => "strftime('%Y-%m-%dT%H:%M:%fZ','now')",
        (true, false) => "NULL",
        _ => "transfer_done_at",
    };
    let sql = format!(
        "UPDATE pulls SET transfer_done = ?1, transfer_done_at = {transfer_done_at_sql},
            updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
         WHERE id = ?2"
    );
    conn.execute(&sql, params![done as i64, id])?;
    fetch_one(conn, id)
}

#[tauri::command]
pub fn set_pull_transfer_done(state: State<AppState>, id: i64, done: bool) -> AppResult<Pull> {
    let conn = state.db.lock().unwrap();
    set_pull_transfer_done_impl(&conn, id, done)
}

#[tauri::command]
pub fn delete_pull(state: State<AppState>, id: i64) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    conn.execute("DELETE FROM pulls WHERE id = ?1", [id])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_conn;

    fn base_input(buyer: &str) -> PullInput {
        PullInput {
            buyer_name: buyer.to_string(),
            event_name: "Coldplay Arena Show".to_string(),
            event_date: Some("2026-09-01".to_string()),
            quantity: 2,
            platform_id: None,
            section: None,
            row_label: None,
            seat: None,
            more_info: None,
            price_cents: 1500,
            currency: "EUR".to_string(),
        }
    }

    fn edit_input_from(p: &Pull, transfer_done: bool) -> PullEditInput {
        PullEditInput {
            buyer_name: p.buyer_name.clone(),
            event_name: p.event_name.clone(),
            event_date: p.event_date.clone(),
            quantity: p.quantity,
            platform_id: p.platform_id,
            section: p.section.clone(),
            row_label: p.row_label.clone(),
            seat: p.seat.clone(),
            more_info: p.more_info.clone(),
            price_cents: p.price_cents,
            currency: p.currency.clone(),
            transfer_done,
        }
    }

    fn seed_platform(conn: &Connection, name: &str, kind: &str) -> i64 {
        conn.execute(
            "INSERT INTO platforms(name, kind) VALUES (?1, ?2)",
            params![name, kind],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    // ---- create -----------------------------------------------------------

    #[test]
    fn create_pull_generates_sequential_pull_codes() {
        let conn = test_conn();
        let a = create_pull_impl(&conn, &base_input("Jano"), false).unwrap();
        let b = create_pull_impl(&conn, &base_input("Maria"), false).unwrap();
        assert_eq!(a.code, "PULL-000001");
        assert_eq!(b.code, "PULL-000002");
    }

    #[test]
    fn create_pull_defaults_transfer_done_to_false() {
        let conn = test_conn();
        let p = create_pull_impl(&conn, &base_input("Jano"), false).unwrap();
        assert!(!p.transfer_done);
        assert!(p.transfer_done_at.is_none());
    }

    #[test]
    fn create_pull_rejects_empty_buyer_name() {
        let conn = test_conn();
        let mut input = base_input("   ");
        input.buyer_name = "   ".to_string();
        assert!(create_pull_impl(&conn, &input, false).is_err());
    }

    #[test]
    fn create_pull_rejects_empty_event_name() {
        let conn = test_conn();
        let mut input = base_input("Jano");
        input.event_name = "".to_string();
        assert!(create_pull_impl(&conn, &input, false).is_err());
    }

    #[test]
    fn create_pull_rejects_zero_quantity() {
        let conn = test_conn();
        let mut input = base_input("Jano");
        input.quantity = 0;
        assert!(create_pull_impl(&conn, &input, false).is_err());
    }

    #[test]
    fn create_pull_rejects_negative_price() {
        let conn = test_conn();
        let mut input = base_input("Jano");
        input.price_cents = -1;
        assert!(create_pull_impl(&conn, &input, false).is_err());
    }

    #[test]
    fn create_pull_with_platform_returns_platform_name() {
        let conn = test_conn();
        let platform_id = seed_platform(&conn, "Ticketmaster", "purchase");
        let mut input = base_input("Jano");
        input.platform_id = Some(platform_id);
        let p = create_pull_impl(&conn, &input, false).unwrap();
        assert_eq!(p.platform_name.as_deref(), Some("Ticketmaster"));
    }

    #[test]
    fn platform_name_is_null_when_no_platform_set() {
        let conn = test_conn();
        let p = create_pull_impl(&conn, &base_input("Jano"), false).unwrap();
        assert!(p.platform_id.is_none());
        assert!(p.platform_name.is_none());
    }

    #[test]
    fn create_pull_stores_section_row_and_seat() {
        let conn = test_conn();
        let mut input = base_input("Jano");
        input.section = Some("Floor".to_string());
        input.row_label = Some("A".to_string());
        input.seat = Some("12".to_string());
        let p = create_pull_impl(&conn, &input, false).unwrap();
        assert_eq!(p.section.as_deref(), Some("Floor"));
        assert_eq!(p.row_label.as_deref(), Some("A"));
        assert_eq!(p.seat.as_deref(), Some("12"));
    }

    #[test]
    fn create_pull_leaves_section_row_seat_null_when_general_admission() {
        let conn = test_conn();
        let p = create_pull_impl(&conn, &base_input("Jano"), false).unwrap();
        assert!(p.section.is_none());
        assert!(p.row_label.is_none());
        assert!(p.seat.is_none());
    }

    // ---- list / search ------------------------------------------------------

    #[test]
    fn list_pulls_orders_newest_first() {
        let conn = test_conn();
        let a = create_pull_impl(&conn, &base_input("Jano"), false).unwrap();
        let b = create_pull_impl(&conn, &base_input("Maria"), false).unwrap();
        let results = list_pulls_impl(&conn, None, None).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, b.id);
        assert_eq!(results[1].id, a.id);
    }

    #[test]
    fn list_pulls_search_finds_by_buyer_name() {
        let conn = test_conn();
        create_pull_impl(&conn, &base_input("Zuzana Kovacova"), false).unwrap();
        create_pull_impl(&conn, &base_input("Peter Novak"), false).unwrap();
        let results = list_pulls_impl(&conn, Some("Kovac".to_string()), None).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].buyer_name, "Zuzana Kovacova");
    }

    #[test]
    fn list_pulls_search_finds_by_event_name() {
        let conn = test_conn();
        let mut input = base_input("Jano");
        input.event_name = "Ed Sheeran Tour".to_string();
        create_pull_impl(&conn, &input, false).unwrap();
        create_pull_impl(&conn, &base_input("Maria"), false).unwrap();
        let results = list_pulls_impl(&conn, Some("sheeran".to_string()), None).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn list_pulls_search_finds_by_platform_name() {
        let conn = test_conn();
        let platform_id = seed_platform(&conn, "Viagogo", "purchase");
        let mut input = base_input("Jano");
        input.platform_id = Some(platform_id);
        create_pull_impl(&conn, &input, false).unwrap();
        create_pull_impl(&conn, &base_input("Maria"), false).unwrap();
        let results = list_pulls_impl(&conn, Some("viagogo".to_string()), None).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn list_pulls_search_finds_by_section() {
        let conn = test_conn();
        let mut input = base_input("Jano");
        input.section = Some("VIP Floor".to_string());
        create_pull_impl(&conn, &input, false).unwrap();
        create_pull_impl(&conn, &base_input("Maria"), false).unwrap();
        let results = list_pulls_impl(&conn, Some("vip floor".to_string()), None).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn list_pulls_search_finds_by_row_label() {
        let conn = test_conn();
        let mut input = base_input("Jano");
        input.row_label = Some("Row Z".to_string());
        create_pull_impl(&conn, &input, false).unwrap();
        create_pull_impl(&conn, &base_input("Maria"), false).unwrap();
        let results = list_pulls_impl(&conn, Some("row z".to_string()), None).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn list_pulls_search_finds_by_seat() {
        let conn = test_conn();
        let mut input = base_input("Jano");
        input.seat = Some("Seat 42".to_string());
        create_pull_impl(&conn, &input, false).unwrap();
        create_pull_impl(&conn, &base_input("Maria"), false).unwrap();
        let results = list_pulls_impl(&conn, Some("seat 42".to_string()), None).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn list_pulls_search_by_nonexistent_term_returns_no_results() {
        let conn = test_conn();
        create_pull_impl(&conn, &base_input("Jano"), false).unwrap();
        let results = list_pulls_impl(&conn, Some("nonexistent".to_string()), None).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn list_pulls_filters_by_transfer_done() {
        let conn = test_conn();
        let a = create_pull_impl(&conn, &base_input("Jano"), false).unwrap();
        let _b = create_pull_impl(&conn, &base_input("Maria"), false).unwrap();
        set_pull_transfer_done_impl(&conn, a.id, true).unwrap();

        let done = list_pulls_impl(&conn, None, Some(true)).unwrap();
        assert_eq!(done.len(), 1);
        assert_eq!(done[0].id, a.id);

        let pending = list_pulls_impl(&conn, None, Some(false)).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, _b.id);
    }

    // ---- update -------------------------------------------------------------

    #[test]
    fn update_pull_changes_fields() {
        let conn = test_conn();
        let p = create_pull_impl(&conn, &base_input("Jano"), false).unwrap();
        let mut edit = edit_input_from(&p, false);
        edit.buyer_name = "Jano Novak".to_string();
        edit.price_cents = 2000;
        let updated = update_pull_impl(&conn, p.id, &edit).unwrap();
        assert_eq!(updated.buyer_name, "Jano Novak");
        assert_eq!(updated.price_cents, 2000);
    }

    #[test]
    fn update_pull_can_set_section_row_and_seat() {
        let conn = test_conn();
        let p = create_pull_impl(&conn, &base_input("Jano"), false).unwrap();
        let mut edit = edit_input_from(&p, false);
        edit.section = Some("Floor".to_string());
        edit.row_label = Some("B".to_string());
        edit.seat = Some("7".to_string());
        let updated = update_pull_impl(&conn, p.id, &edit).unwrap();
        assert_eq!(updated.section.as_deref(), Some("Floor"));
        assert_eq!(updated.row_label.as_deref(), Some("B"));
        assert_eq!(updated.seat.as_deref(), Some("7"));
    }

    #[test]
    fn update_pull_rejects_missing_pull() {
        let conn = test_conn();
        let p = create_pull_impl(&conn, &base_input("Jano"), false).unwrap();
        let edit = edit_input_from(&p, false);
        assert!(update_pull_impl(&conn, 999_999, &edit).is_err());
    }

    #[test]
    fn update_pull_rejects_invalid_fields_same_as_create() {
        let conn = test_conn();
        let p = create_pull_impl(&conn, &base_input("Jano"), false).unwrap();
        let mut edit = edit_input_from(&p, false);
        edit.quantity = 0;
        assert!(update_pull_impl(&conn, p.id, &edit).is_err());
    }

    #[test]
    fn update_pull_via_edit_form_can_mark_transfer_done() {
        let conn = test_conn();
        let p = create_pull_impl(&conn, &base_input("Jano"), false).unwrap();
        let edit = edit_input_from(&p, true);
        let updated = update_pull_impl(&conn, p.id, &edit).unwrap();
        assert!(updated.transfer_done);
        assert!(updated.transfer_done_at.is_some());
    }

    #[test]
    fn update_pull_preserves_transfer_done_at_when_resaving_without_changing_the_checkbox() {
        let conn = test_conn();
        let p = create_pull_impl(&conn, &base_input("Jano"), false).unwrap();
        let done = set_pull_transfer_done_impl(&conn, p.id, true).unwrap();
        let stamp = done.transfer_done_at.clone().unwrap();

        // Re-save the full edit form with transfer_done still true and some
        // unrelated field changed - the timestamp must not move.
        let mut edit = edit_input_from(&done, true);
        edit.more_info = Some("called buyer to confirm".to_string());
        let resaved = update_pull_impl(&conn, p.id, &edit).unwrap();
        assert!(resaved.transfer_done);
        assert_eq!(resaved.transfer_done_at, Some(stamp));
    }

    #[test]
    fn update_pull_clears_timestamp_when_transfer_done_is_switched_back_off() {
        let conn = test_conn();
        let p = create_pull_impl(&conn, &base_input("Jano"), false).unwrap();
        let done = set_pull_transfer_done_impl(&conn, p.id, true).unwrap();
        assert!(done.transfer_done_at.is_some());

        let edit = edit_input_from(&done, false);
        let reverted = update_pull_impl(&conn, p.id, &edit).unwrap();
        assert!(!reverted.transfer_done);
        assert!(reverted.transfer_done_at.is_none());
    }

    // ---- transfer done quick action ------------------------------------------

    #[test]
    fn set_pull_transfer_done_stamps_timestamp_on_first_true() {
        let conn = test_conn();
        let p = create_pull_impl(&conn, &base_input("Jano"), false).unwrap();
        let updated = set_pull_transfer_done_impl(&conn, p.id, true).unwrap();
        assert!(updated.transfer_done);
        assert!(updated.transfer_done_at.is_some());
    }

    #[test]
    fn set_pull_transfer_done_clears_timestamp_when_set_back_to_false() {
        let conn = test_conn();
        let p = create_pull_impl(&conn, &base_input("Jano"), false).unwrap();
        set_pull_transfer_done_impl(&conn, p.id, true).unwrap();
        let reverted = set_pull_transfer_done_impl(&conn, p.id, false).unwrap();
        assert!(!reverted.transfer_done);
        assert!(reverted.transfer_done_at.is_none());
    }

    #[test]
    fn set_pull_transfer_done_does_not_restamp_when_already_true() {
        let conn = test_conn();
        let p = create_pull_impl(&conn, &base_input("Jano"), false).unwrap();
        let first = set_pull_transfer_done_impl(&conn, p.id, true).unwrap();
        let stamp = first.transfer_done_at.clone().unwrap();
        let second = set_pull_transfer_done_impl(&conn, p.id, true).unwrap();
        assert_eq!(second.transfer_done_at, Some(stamp));
    }

    #[test]
    fn set_pull_transfer_done_rejects_a_missing_pull() {
        let conn = test_conn();
        assert!(set_pull_transfer_done_impl(&conn, 999_999, true).is_err());
    }

    // ---- delete ---------------------------------------------------------------

    #[test]
    fn delete_pull_removes_it() {
        let conn = test_conn();
        let p = create_pull_impl(&conn, &base_input("Jano"), false).unwrap();
        conn.execute("DELETE FROM pulls WHERE id = ?1", [p.id]).unwrap();
        assert!(fetch_one(&conn, p.id).is_err());
    }
}

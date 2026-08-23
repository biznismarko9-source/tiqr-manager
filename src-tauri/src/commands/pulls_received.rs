//! Pull received (2.0.17): the mirror direction of `Pull` (commands::pulls) -
//! someone ELSE pulls tickets FOR marko (marko pays them a fee) instead of
//! marko pulling for someone else. See migrations/011_pulls_received.sql for
//! the full schema rationale. Unlike `Pull`, these tickets DO become marko's
//! own inventory, so a row here can optionally link to the resulting
//! `orders` row - nullable, since marko confirmed (via AskUserQuestion) this
//! must also work fully standalone, with no order at all.
//!
//! THREE ways a row gets created: typed directly in the app via the full
//! form on the Pulls screen (`create_pull_received`, `source = "manual"`,
//! optionally linked to any existing order by hand through that form's own
//! order picker - see Pulls.tsx's `OrderLinkPicker`); auto-created by Orders
//! & Sales sheet sync when a synced row's `pull` column says "yes" (see
//! commands::orders_sheet_sync::maybe_link_pull_received, `source =
//! "sheet_sync"`); or, since 2.0.24, filled in directly on the Order Detail
//! screen itself (`link_pull_received_to_order`, also `source = "manual"` -
//! it's the exact same kind of manually-typed row as the Pulls-screen form
//! produces, just entered from a different, narrower screen that already
//! knows which order it's for) - marko's own request, so he isn't forced to
//! leave the order he's looking at and search for it again on the Pulls
//! screen just to record who pulled it. `create_pull_received_with_source`
//! is `pub(crate)` specifically so every one of these paths reuses the exact
//! same validate+insert logic instead of duplicating it, same convention as
//! commands::pulls::fetch_one being `pub(crate)` for commands::pulls_sheet_sync
//! to reuse.

use crate::codes;
use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::models::{BulkDeleteResult, BulkDeleteSkip, PullReceived, PullReceivedEditInput, PullReceivedInput};
use rusqlite::{params, Connection, OptionalExtension, Row};
use tauri::State;

// Same safety cap and rationale as every other unfiltered list view in this
// app (see e.g. commands/pulls.rs::LIST_CAP) - ordinary use never gets close
// to it.
const LIST_CAP: i64 = 5000;

const BASE_SQL: &str = "
    SELECT pr.id, pr.code, pr.puller_name, pr.event_name, pr.event_date, pr.quantity,
      pr.amount_cents, pr.currency, pr.more_info, pr.order_id, o.code as order_code,
      pr.source, pr.is_demo, pr.created_at, pr.updated_at
    FROM pulls_received pr
    LEFT JOIN orders o ON o.id = pr.order_id
";

fn map_pull_received(row: &Row) -> rusqlite::Result<PullReceived> {
    Ok(PullReceived {
        id: row.get("id")?,
        code: row.get("code")?,
        puller_name: row.get("puller_name")?,
        event_name: row.get("event_name")?,
        event_date: row.get("event_date")?,
        quantity: row.get("quantity")?,
        amount_cents: row.get("amount_cents")?,
        currency: row.get("currency")?,
        more_info: row.get("more_info")?,
        order_id: row.get("order_id")?,
        order_code: row.get("order_code")?,
        source: row.get("source")?,
        is_demo: row.get("is_demo")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

/// `pub(crate)` since 2.0.17: commands::orders_sheet_sync reuses this after
/// creating a sheet-sync-linked row, to hand back the full record the same
/// way the manual `#[tauri::command]`s do.
pub(crate) fn fetch_one(conn: &Connection, id: i64) -> AppResult<PullReceived> {
    let sql = format!("{BASE_SQL} WHERE pr.id = ?1");
    conn.query_row(&sql, [id], map_pull_received)
        .map_err(|_| AppError::NotFound(format!("Pull received #{id} not found")))
}

/// Free-text search across every field marko would actually recognize a
/// received pull by (puller, event, own code, more info, linked order's own
/// code) - same `LIKE` OR-chain convention as list_pulls_impl's search.
fn list_pulls_received_impl(conn: &Connection, search: Option<String>) -> AppResult<Vec<PullReceived>> {
    let mut sql = format!("{BASE_SQL} WHERE 1=1");
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![];

    if let Some(q) = search.as_deref() {
        let q = q.trim();
        if !q.is_empty() {
            sql.push_str(
                " AND (pr.puller_name LIKE ? OR pr.event_name LIKE ? OR pr.code LIKE ? \
                 OR pr.more_info LIKE ? OR o.code LIKE ?)",
            );
            let like = format!("%{q}%");
            for _ in 0..5 {
                params_vec.push(Box::new(like.clone()));
            }
        }
    }
    sql.push_str(&format!(" ORDER BY pr.created_at DESC, pr.id DESC LIMIT {LIST_CAP}"));

    let mut stmt = conn.prepare(&sql)?;
    let param_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
    let rows = stmt.query_map(param_refs.as_slice(), map_pull_received)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[tauri::command]
pub fn list_pulls_received(state: State<AppState>, search: Option<String>) -> AppResult<Vec<PullReceived>> {
    let conn = state.db.lock().unwrap();
    list_pulls_received_impl(&conn, search)
}

#[tauri::command]
pub fn get_pull_received(state: State<AppState>, id: i64) -> AppResult<PullReceived> {
    let conn = state.db.lock().unwrap();
    fetch_one(&conn, id)
}

fn validate_pull_received_fields(
    puller_name: &str,
    event_name: &str,
    quantity: i64,
    amount_cents: i64,
    currency: &str,
) -> AppResult<()> {
    if puller_name.trim().is_empty() {
        return Err(AppError::Validation("Puller name is required".into()));
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
    if amount_cents < 0 {
        return Err(AppError::Validation("Amount cannot be negative".into()));
    }
    if currency.trim().is_empty() {
        return Err(AppError::Validation("Currency is required".into()));
    }
    Ok(())
}

/// The real validate+insert logic, shared by both creation paths - see this
/// module's own doc comment above for why `source` is a separate explicit
/// argument here rather than part of `PullReceivedInput`.
pub(crate) fn create_pull_received_with_source(
    conn: &Connection,
    input: &PullReceivedInput,
    is_demo: bool,
    source: &str,
) -> AppResult<PullReceived> {
    validate_pull_received_fields(&input.puller_name, &input.event_name, input.quantity, input.amount_cents, &input.currency)?;
    let code = codes::next_code(conn, "pull_received", "RPULL")?;
    conn.execute(
        "INSERT INTO pulls_received (code, puller_name, event_name, event_date, quantity,
           amount_cents, currency, more_info, order_id, source, is_demo)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
        params![
            code,
            input.puller_name.trim(),
            input.event_name.trim(),
            input.event_date,
            input.quantity,
            input.amount_cents,
            input.currency,
            input.more_info,
            input.order_id,
            source,
            is_demo as i64,
        ],
    )?;
    let id = conn.last_insert_rowid();
    fetch_one(conn, id)
}

pub(crate) fn create_pull_received_impl(conn: &Connection, input: &PullReceivedInput, is_demo: bool) -> AppResult<PullReceived> {
    create_pull_received_with_source(conn, input, is_demo, "manual")
}

#[tauri::command]
pub fn create_pull_received(state: State<AppState>, input: PullReceivedInput) -> AppResult<PullReceived> {
    let conn = state.db.lock().unwrap();
    create_pull_received_impl(&conn, &input, false)
}

/// Full-edit path. Deliberately never touches `source` - see
/// `PullReceivedEditInput`'s doc comment.
pub(crate) fn update_pull_received_impl(conn: &Connection, id: i64, input: &PullReceivedEditInput) -> AppResult<PullReceived> {
    validate_pull_received_fields(&input.puller_name, &input.event_name, input.quantity, input.amount_cents, &input.currency)?;
    let updated = conn.execute(
        "UPDATE pulls_received SET
            puller_name = ?1, event_name = ?2, event_date = ?3, quantity = ?4,
            amount_cents = ?5, currency = ?6, more_info = ?7, order_id = ?8,
            updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
         WHERE id = ?9",
        params![
            input.puller_name.trim(),
            input.event_name.trim(),
            input.event_date,
            input.quantity,
            input.amount_cents,
            input.currency,
            input.more_info,
            input.order_id,
            id,
        ],
    )?;
    if updated == 0 {
        return Err(AppError::NotFound(format!("Pull received #{id} not found")));
    }
    fetch_one(conn, id)
}

#[tauri::command]
pub fn update_pull_received(state: State<AppState>, id: i64, input: PullReceivedEditInput) -> AppResult<PullReceived> {
    let conn = state.db.lock().unwrap();
    update_pull_received_impl(&conn, id, &input)
}

#[tauri::command]
pub fn delete_pull_received(state: State<AppState>, id: i64) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    conn.execute("DELETE FROM pulls_received WHERE id = ?1", [id])?;
    Ok(())
}

/// 2.0.28: bulk delete for the new "Delete" selection mode on the Pulls
/// (Received) list - same reasoning as `pulls::bulk_delete_pulls_impl`
/// (no sibling module to share code with directly since the two tables are
/// unrelated, but the shape and logic are deliberately identical). Deleting a
/// received pull never touches its linked order (`order_id` is
/// `ON DELETE SET NULL`, not a blocker in either direction), so - like Given
/// pulls - the only way an id can be skipped is if it no longer exists.
pub(crate) fn bulk_delete_pulls_received_impl(conn: &mut Connection, ids: &[i64]) -> AppResult<BulkDeleteResult> {
    if ids.is_empty() {
        return Err(AppError::Validation("Select at least one received pull to delete".into()));
    }
    let tx = conn.transaction()?;
    let mut deleted_ids = Vec::new();
    let mut skipped = Vec::new();
    for &id in ids {
        let changed = tx.execute("DELETE FROM pulls_received WHERE id = ?1", [id])?;
        if changed > 0 {
            deleted_ids.push(id);
        } else {
            skipped.push(BulkDeleteSkip {
                id,
                reason: "Not found - already deleted?".into(),
            });
        }
    }
    tx.commit()?;
    Ok(BulkDeleteResult { deleted_ids, skipped })
}

#[tauri::command]
pub fn bulk_delete_pulls_received(state: State<AppState>, ids: Vec<i64>) -> AppResult<BulkDeleteResult> {
    let mut conn = state.db.lock().unwrap();
    bulk_delete_pulls_received_impl(&mut conn, &ids)
}

// ---------------------------------------------------------------------------
// Order Detail's own "Received pulls" section (2.0.24) - see this module's
// own doc comment above for why this is a third, narrower creation path
// rather than reusing the full Pulls-screen form. Deliberately only 2 real
// inputs (puller_name, amount_cents) - event_name/event_date/quantity/
// currency are always copied fresh from the order/event themselves, the
// same server-side auto-derivation `maybe_link_pull_received`
// (orders_sheet_sync.rs) already does for the sheet-sync path, and for the
// same reason: Order Detail already shows all of that about the order, so
// asking marko to retype it here would be redundant AND a second, possibly
// drifting copy of numbers the order itself already owns.
// ---------------------------------------------------------------------------

/// Order Detail's "Add pull info" action. No idempotency guard, deliberately
/// - unlike the sheet-sync path (which must never turn one sheet row into
/// two rows across repeated syncs of the SAME data), every call here is a
/// distinct, explicit action marko took on purpose, exactly like clicking
/// "New received pull" on the Pulls screen itself (which has never had a
/// "only one per order" rule either - see Pulls.tsx's `OrderLinkPicker`,
/// which lets ANY order be linked from there with no such limit). The
/// frontend is what keeps this sane in practice: it only offers this action
/// at all, and disables it while saving - see `list_pulls_received_for_order`
/// below for how Order Detail then shows however many rows exist.
fn link_pull_received_to_order_impl(
    conn: &Connection,
    order_id: i64,
    puller_name: &str,
    amount_cents: i64,
) -> AppResult<PullReceived> {
    let (event_id, quantity, currency): (i64, i64, String) = conn
        .query_row("SELECT event_id, quantity, currency FROM orders WHERE id = ?1", [order_id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .optional()?
        .ok_or_else(|| AppError::NotFound(format!("Order #{order_id} not found")))?;
    let (event_name, event_date): (String, Option<String>) =
        conn.query_row("SELECT name, event_date FROM events WHERE id = ?1", [event_id], |r| Ok((r.get(0)?, r.get(1)?)))?;

    let input = PullReceivedInput {
        puller_name: puller_name.to_string(),
        event_name,
        event_date,
        quantity,
        amount_cents,
        currency,
        more_info: None,
        order_id: Some(order_id),
    };
    create_pull_received_with_source(conn, &input, false, "manual")
}

#[tauri::command]
pub fn link_pull_received_to_order(
    state: State<AppState>,
    order_id: i64,
    puller_name: String,
    amount_cents: i64,
) -> AppResult<PullReceived> {
    let conn = state.db.lock().unwrap();
    link_pull_received_to_order_impl(&conn, order_id, &puller_name, amount_cents)
}

/// Powers Order Detail's "Received pulls" section - same small, dedicated
/// fetch alongside the main entity as `commands::orders::get_order_sales_summary`,
/// rather than folding this onto `Order`/`OrderRecord` itself. Returns every
/// row linked to this order, not just one - nothing stops marko manually
/// linking more than one to the same order from the Pulls screen's own
/// picker (see `link_pull_received_to_order`'s own doc comment), so an
/// `Option` here would silently hide a second or third one. Oldest first,
/// matching the order marko most likely added them in.
fn list_pulls_received_for_order_impl(conn: &Connection, order_id: i64) -> AppResult<Vec<PullReceived>> {
    let sql = format!("{BASE_SQL} WHERE pr.order_id = ?1 ORDER BY pr.id ASC");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([order_id], map_pull_received)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[tauri::command]
pub fn list_pulls_received_for_order(state: State<AppState>, order_id: i64) -> AppResult<Vec<PullReceived>> {
    let conn = state.db.lock().unwrap();
    list_pulls_received_for_order_impl(&conn, order_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_conn;

    fn base_input(puller: &str) -> PullReceivedInput {
        PullReceivedInput {
            puller_name: puller.to_string(),
            event_name: "Coldplay Arena Show".to_string(),
            event_date: Some("2026-09-01".to_string()),
            quantity: 2,
            amount_cents: 1500,
            currency: "EUR".to_string(),
            more_info: None,
            order_id: None,
        }
    }

    fn edit_input_from(p: &PullReceived) -> PullReceivedEditInput {
        PullReceivedEditInput {
            puller_name: p.puller_name.clone(),
            event_name: p.event_name.clone(),
            event_date: p.event_date.clone(),
            quantity: p.quantity,
            amount_cents: p.amount_cents,
            currency: p.currency.clone(),
            more_info: p.more_info.clone(),
            order_id: p.order_id,
        }
    }

    /// Seeds a minimal real Order (with its Event) directly via SQL, for
    /// tests that need a genuine order_id to link against. Not going through
    /// apply_order_rows here - that belongs to orders_sheet_sync.rs's own
    /// test module, and would pull in a dependency this module doesn't need.
    fn seed_order(conn: &Connection) -> i64 {
        conn.execute("INSERT INTO events(name) VALUES ('Coldplay Arena Show')", []).unwrap();
        let event_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO orders(code, event_id, purchase_date, quantity) VALUES ('ORD-000001', ?1, '2026-09-01', 1)",
            [event_id],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    // ---- create -------------------------------------------------------------

    #[test]
    fn create_pull_received_generates_sequential_rpull_codes() {
        let conn = test_conn();
        let a = create_pull_received_impl(&conn, &base_input("Jozef"), false).unwrap();
        let b = create_pull_received_impl(&conn, &base_input("Maria"), false).unwrap();
        assert_eq!(a.code, "RPULL-000001");
        assert_eq!(b.code, "RPULL-000002");
    }

    #[test]
    fn create_pull_received_defaults_source_to_manual_and_order_id_to_none() {
        let conn = test_conn();
        let p = create_pull_received_impl(&conn, &base_input("Jozef"), false).unwrap();
        assert_eq!(p.source, "manual");
        assert!(p.order_id.is_none());
        assert!(p.order_code.is_none());
    }

    #[test]
    fn create_pull_received_rejects_empty_puller_name() {
        let conn = test_conn();
        let mut input = base_input("Jozef");
        input.puller_name = "   ".to_string();
        assert!(create_pull_received_impl(&conn, &input, false).is_err());
    }

    #[test]
    fn create_pull_received_rejects_empty_event_name() {
        let conn = test_conn();
        let mut input = base_input("Jozef");
        input.event_name = "".to_string();
        assert!(create_pull_received_impl(&conn, &input, false).is_err());
    }

    #[test]
    fn create_pull_received_rejects_zero_quantity() {
        let conn = test_conn();
        let mut input = base_input("Jozef");
        input.quantity = 0;
        assert!(create_pull_received_impl(&conn, &input, false).is_err());
    }

    #[test]
    fn create_pull_received_rejects_negative_amount() {
        let conn = test_conn();
        let mut input = base_input("Jozef");
        input.amount_cents = -1;
        assert!(create_pull_received_impl(&conn, &input, false).is_err());
    }

    #[test]
    fn create_pull_received_with_order_id_returns_the_orders_own_code() {
        let conn = test_conn();
        let order_id = seed_order(&conn);
        let mut input = base_input("Jozef");
        input.order_id = Some(order_id);
        let p = create_pull_received_impl(&conn, &input, false).unwrap();
        assert_eq!(p.order_code.as_deref(), Some("ORD-000001"));
    }

    #[test]
    fn create_pull_received_rejects_a_nonexistent_order_id() {
        let conn = test_conn();
        let mut input = base_input("Jozef");
        input.order_id = Some(999_999);
        assert!(create_pull_received_impl(&conn, &input, false).is_err());
    }

    #[test]
    fn create_pull_received_with_source_sheet_sync_is_reflected_on_the_row() {
        let conn = test_conn();
        let order_id = seed_order(&conn);
        let mut input = base_input("Jozef");
        input.order_id = Some(order_id);
        let p = create_pull_received_with_source(&conn, &input, false, "sheet_sync").unwrap();
        assert_eq!(p.source, "sheet_sync");
    }

    #[test]
    fn deleting_the_linked_order_clears_order_id_but_keeps_the_row() {
        let conn = test_conn();
        let order_id = seed_order(&conn);
        let mut input = base_input("Jozef");
        input.order_id = Some(order_id);
        let p = create_pull_received_impl(&conn, &input, false).unwrap();

        conn.execute("DELETE FROM orders WHERE id = ?1", [order_id]).unwrap();

        let after = fetch_one(&conn, p.id).unwrap();
        assert!(after.order_id.is_none(), "ON DELETE SET NULL must clear the link, not the whole row");
        assert!(after.order_code.is_none());
    }

    // ---- list / search --------------------------------------------------------

    #[test]
    fn list_pulls_received_orders_newest_first() {
        let conn = test_conn();
        let a = create_pull_received_impl(&conn, &base_input("Jozef"), false).unwrap();
        let b = create_pull_received_impl(&conn, &base_input("Maria"), false).unwrap();
        let results = list_pulls_received_impl(&conn, None).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, b.id);
        assert_eq!(results[1].id, a.id);
    }

    #[test]
    fn list_pulls_received_search_finds_by_puller_name() {
        let conn = test_conn();
        create_pull_received_impl(&conn, &base_input("Zuzana Kovacova"), false).unwrap();
        create_pull_received_impl(&conn, &base_input("Peter Novak"), false).unwrap();
        let results = list_pulls_received_impl(&conn, Some("Kovac".to_string())).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].puller_name, "Zuzana Kovacova");
    }

    #[test]
    fn list_pulls_received_search_finds_by_event_name() {
        let conn = test_conn();
        let mut input = base_input("Jozef");
        input.event_name = "Ed Sheeran Tour".to_string();
        create_pull_received_impl(&conn, &input, false).unwrap();
        create_pull_received_impl(&conn, &base_input("Maria"), false).unwrap();
        let results = list_pulls_received_impl(&conn, Some("sheeran".to_string())).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn list_pulls_received_search_finds_by_linked_order_code() {
        let conn = test_conn();
        let order_id = seed_order(&conn);
        let mut input = base_input("Jozef");
        input.order_id = Some(order_id);
        create_pull_received_impl(&conn, &input, false).unwrap();
        create_pull_received_impl(&conn, &base_input("Maria"), false).unwrap();
        let results = list_pulls_received_impl(&conn, Some("ORD-000001".to_string())).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn list_pulls_received_search_by_nonexistent_term_returns_no_results() {
        let conn = test_conn();
        create_pull_received_impl(&conn, &base_input("Jozef"), false).unwrap();
        let results = list_pulls_received_impl(&conn, Some("nonexistent".to_string())).unwrap();
        assert!(results.is_empty());
    }

    // ---- update -----------------------------------------------------------------

    #[test]
    fn update_pull_received_changes_fields() {
        let conn = test_conn();
        let p = create_pull_received_impl(&conn, &base_input("Jozef"), false).unwrap();
        let mut edit = edit_input_from(&p);
        edit.puller_name = "Jozef Mrkva".to_string();
        edit.amount_cents = 2000;
        let updated = update_pull_received_impl(&conn, p.id, &edit).unwrap();
        assert_eq!(updated.puller_name, "Jozef Mrkva");
        assert_eq!(updated.amount_cents, 2000);
    }

    #[test]
    fn update_pull_received_can_link_a_standalone_row_to_an_order() {
        let conn = test_conn();
        let p = create_pull_received_impl(&conn, &base_input("Jozef"), false).unwrap();
        assert!(p.order_id.is_none());
        let order_id = seed_order(&conn);
        let mut edit = edit_input_from(&p);
        edit.order_id = Some(order_id);
        let updated = update_pull_received_impl(&conn, p.id, &edit).unwrap();
        assert_eq!(updated.order_code.as_deref(), Some("ORD-000001"));
    }

    #[test]
    fn update_pull_received_can_unlink_from_an_order() {
        let conn = test_conn();
        let order_id = seed_order(&conn);
        let mut input = base_input("Jozef");
        input.order_id = Some(order_id);
        let p = create_pull_received_impl(&conn, &input, false).unwrap();
        let mut edit = edit_input_from(&p);
        edit.order_id = None;
        let updated = update_pull_received_impl(&conn, p.id, &edit).unwrap();
        assert!(updated.order_id.is_none());
    }

    #[test]
    fn update_pull_received_never_changes_source() {
        let conn = test_conn();
        let order_id = seed_order(&conn);
        let mut input = base_input("Jozef");
        input.order_id = Some(order_id);
        let p = create_pull_received_with_source(&conn, &input, false, "sheet_sync").unwrap();
        let mut edit = edit_input_from(&p);
        edit.more_info = Some("edited by marko".to_string());
        let updated = update_pull_received_impl(&conn, p.id, &edit).unwrap();
        assert_eq!(updated.source, "sheet_sync", "source is provenance, not editable from the form");
    }

    #[test]
    fn update_pull_received_rejects_missing_row() {
        let conn = test_conn();
        let p = create_pull_received_impl(&conn, &base_input("Jozef"), false).unwrap();
        let edit = edit_input_from(&p);
        assert!(update_pull_received_impl(&conn, 999_999, &edit).is_err());
    }

    #[test]
    fn update_pull_received_rejects_invalid_fields_same_as_create() {
        let conn = test_conn();
        let p = create_pull_received_impl(&conn, &base_input("Jozef"), false).unwrap();
        let mut edit = edit_input_from(&p);
        edit.quantity = 0;
        assert!(update_pull_received_impl(&conn, p.id, &edit).is_err());
    }

    // ---- delete -------------------------------------------------------------------

    #[test]
    fn delete_pull_received_removes_it() {
        let conn = test_conn();
        let p = create_pull_received_impl(&conn, &base_input("Jozef"), false).unwrap();
        conn.execute("DELETE FROM pulls_received WHERE id = ?1", [p.id]).unwrap();
        assert!(fetch_one(&conn, p.id).is_err());
    }

    // ---- bulk delete (2.0.28) ---------------------------------------------

    #[test]
    fn bulk_delete_pulls_received_removes_every_selected_id() {
        let mut conn = test_conn();
        let a = create_pull_received_impl(&conn, &base_input("Jozef"), false).unwrap();
        let b = create_pull_received_impl(&conn, &base_input("Anna"), false).unwrap();
        let c = create_pull_received_impl(&conn, &base_input("Tomas"), false).unwrap();

        let result = bulk_delete_pulls_received_impl(&mut conn, &[a.id, b.id]).unwrap();

        assert_eq!(result.deleted_ids, vec![a.id, b.id]);
        assert!(result.skipped.is_empty());
        assert!(fetch_one(&conn, a.id).is_err());
        assert!(fetch_one(&conn, b.id).is_err());
        assert!(fetch_one(&conn, c.id).is_ok(), "an unselected received pull must survive");
    }

    #[test]
    fn bulk_delete_pulls_received_reports_a_missing_id_as_skipped_not_as_a_failure() {
        let mut conn = test_conn();
        let a = create_pull_received_impl(&conn, &base_input("Jozef"), false).unwrap();

        let result = bulk_delete_pulls_received_impl(&mut conn, &[a.id, 999_999]).unwrap();

        assert_eq!(result.deleted_ids, vec![a.id]);
        assert_eq!(result.skipped.len(), 1);
        assert_eq!(result.skipped[0].id, 999_999);
    }

    #[test]
    fn bulk_delete_pulls_received_rejects_an_empty_selection() {
        let mut conn = test_conn();
        let err = bulk_delete_pulls_received_impl(&mut conn, &[]).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    // ---- link_pull_received_to_order (Order Detail, 2.0.24) -------------------

    /// Same idea as `seed_order` above, but with a non-default quantity/
    /// currency/event_date so a passing test can't be accidentally hiding a
    /// bug where the copy-through silently falls back to some default
    /// instead of genuinely reading the order's own values.
    fn seed_order_with(conn: &Connection, code: &str, quantity: i64, currency: &str, event_date: &str) -> i64 {
        conn.execute(
            "INSERT INTO events(name, event_date) VALUES ('Ed Sheeran Tour', ?1)",
            [event_date],
        )
        .unwrap();
        let event_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO orders(code, event_id, purchase_date, quantity, currency) VALUES (?1, ?2, '2026-08-01', ?3, ?4)",
            params![code, event_id, quantity, currency],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn link_pull_received_to_order_copies_event_quantity_and_currency_from_the_order_itself() {
        let conn = test_conn();
        let order_id = seed_order_with(&conn, "ORD-000042", 3, "USD", "2026-10-15");
        let p = link_pull_received_to_order_impl(&conn, order_id, "Jozef", 1500).unwrap();
        assert_eq!(p.puller_name, "Jozef");
        assert_eq!(p.event_name, "Ed Sheeran Tour");
        assert_eq!(p.event_date.as_deref(), Some("2026-10-15"));
        assert_eq!(p.quantity, 3, "must be the order's own quantity, not a hardcoded 1");
        assert_eq!(p.currency, "USD", "must be the order's own currency, not a hardcoded default");
        assert_eq!(p.amount_cents, 1500);
        assert_eq!(p.order_id, Some(order_id));
        assert_eq!(p.order_code.as_deref(), Some("ORD-000042"));
    }

    #[test]
    fn link_pull_received_to_order_uses_manual_as_the_source() {
        let conn = test_conn();
        let order_id = seed_order_with(&conn, "ORD-000042", 1, "EUR", "2026-10-15");
        let p = link_pull_received_to_order_impl(&conn, order_id, "Jozef", 0).unwrap();
        assert_eq!(p.source, "manual", "typed on Order Detail is just as manual as the Pulls screen's own form");
    }

    #[test]
    fn link_pull_received_to_order_accepts_a_zero_amount() {
        // "How much pull" is informational only and optional in spirit, same
        // as the sheet-sync path's own blank-defaults-to-0 rule - the
        // frontend sends 0 rather than blank, but the backend must not
        // reject it either way.
        let conn = test_conn();
        let order_id = seed_order_with(&conn, "ORD-000042", 1, "EUR", "2026-10-15");
        let p = link_pull_received_to_order_impl(&conn, order_id, "Jozef", 0).unwrap();
        assert_eq!(p.amount_cents, 0);
    }

    #[test]
    fn link_pull_received_to_order_rejects_an_empty_puller_name() {
        let conn = test_conn();
        let order_id = seed_order_with(&conn, "ORD-000042", 1, "EUR", "2026-10-15");
        assert!(link_pull_received_to_order_impl(&conn, order_id, "   ", 1500).is_err());
    }

    #[test]
    fn link_pull_received_to_order_rejects_a_negative_amount() {
        let conn = test_conn();
        let order_id = seed_order_with(&conn, "ORD-000042", 1, "EUR", "2026-10-15");
        assert!(link_pull_received_to_order_impl(&conn, order_id, "Jozef", -1).is_err());
    }

    #[test]
    fn link_pull_received_to_order_rejects_a_nonexistent_order() {
        let conn = test_conn();
        let err = link_pull_received_to_order_impl(&conn, 999_999, "Jozef", 1500).unwrap_err();
        assert!(err.to_string().contains("not found"), "{err}");
    }

    #[test]
    fn link_pull_received_to_order_allows_linking_a_second_row_to_the_same_order() {
        // Deliberately no "already linked" guard here - see this function's
        // own doc comment for why. Mirrors the Pulls screen's own
        // OrderLinkPicker, which has never restricted this either.
        let conn = test_conn();
        let order_id = seed_order_with(&conn, "ORD-000042", 2, "EUR", "2026-10-15");
        link_pull_received_to_order_impl(&conn, order_id, "Jozef", 1000).unwrap();
        link_pull_received_to_order_impl(&conn, order_id, "Maria", 500).unwrap();
        let rows = list_pulls_received_for_order_impl(&conn, order_id).unwrap();
        assert_eq!(rows.len(), 2);
    }

    // ---- list_pulls_received_for_order (Order Detail, 2.0.24) -----------------

    #[test]
    fn list_pulls_received_for_order_returns_only_that_orders_rows_oldest_first() {
        let conn = test_conn();
        let order_a = seed_order_with(&conn, "ORD-000042", 1, "EUR", "2026-10-15");
        let order_b = seed_order_with(&conn, "ORD-000043", 1, "EUR", "2026-10-16");
        let first = link_pull_received_to_order_impl(&conn, order_a, "Jozef", 1000).unwrap();
        let second = link_pull_received_to_order_impl(&conn, order_a, "Maria", 500).unwrap();
        link_pull_received_to_order_impl(&conn, order_b, "Someone else entirely", 200).unwrap();

        let rows = list_pulls_received_for_order_impl(&conn, order_a).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].id, first.id);
        assert_eq!(rows[1].id, second.id);
    }

    #[test]
    fn list_pulls_received_for_order_is_empty_for_an_order_with_no_linked_pulls() {
        let conn = test_conn();
        let order_id = seed_order_with(&conn, "ORD-000042", 1, "EUR", "2026-10-15");
        assert!(list_pulls_received_for_order_impl(&conn, order_id).unwrap().is_empty());
    }
}

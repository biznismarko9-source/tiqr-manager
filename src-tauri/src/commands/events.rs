use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::finance;
use crate::models::{
    BulkDeleteResult, BulkDeleteSkip, CategoryDetectionResult, Event, EventInput, EventWithStats,
};
use rusqlite::{params, Connection, Row};
use tauri::State;

const STATS_SQL: &str = "
    SELECT
      e.id, e.name, e.artist_team, e.venue, e.city, e.country, e.event_date,
      e.category, e.category_id, ec.color_slot AS category_color_slot,
      e.status, e.notes, e.is_demo, e.created_at, e.updated_at,
      COUNT(DISTINCT t.id) AS purchased_tickets,
      COUNT(DISTINCT CASE WHEN t.status='available' THEN t.id END) AS available_tickets,
      COUNT(DISTINCT CASE WHEN t.status='listed' THEN t.id END) AS listed_tickets,
      COUNT(DISTINCT CASE WHEN t.status='sold' THEN t.id END) AS sold_tickets,
      COUNT(DISTINCT CASE WHEN t.status='cancelled' THEN t.id END) AS cancelled_tickets,
      COALESCE(SUM(t.purchase_cost_cents + t.purchase_fees_cents + t.other_costs_cents), 0) AS total_cost_cents,
      COALESCE(SUM(CASE WHEN t.status='sold' THEN t.purchase_cost_cents + t.purchase_fees_cents + t.other_costs_cents ELSE 0 END), 0) AS cogs_cents,
      COALESCE(SUM(s.sale_price_cents), 0) AS revenue_cents,
      COALESCE(SUM(s.selling_fees_cents), 0) AS selling_fees_cents,
      CASE WHEN COUNT(DISTINCT t.currency) <= 1 THEN MIN(t.currency) ELSE NULL END AS currency
    FROM events e
    -- event_categories.id is a primary key, so this join is always at most
    -- 1:1 - it can never multiply rows the way the tickets/sales joins
    -- below legitimately do. Same reasoning already relied on by orders.rs'
    -- BASE_SQL and sales.rs' GROUP_BASE_SELECT for their own identical join.
    LEFT JOIN event_categories ec ON ec.id = e.category_id
    LEFT JOIN tickets t ON t.event_id = e.id
    -- Refunded sales stay in the table (history) but must never count as
    -- revenue - excluding them from the join keeps every aggregate above
    -- correct without a second pass, and matches tickets whose status has
    -- already been returned to 'available' by the refund itself.
    LEFT JOIN sales s ON s.ticket_id = t.id AND s.payment_status != 'refunded'
";

/// 2.0.27: like STATS_SQL but without the ticket/sales aggregation - used by
/// create_event/update_event, which only need to return the single row just
/// written (plus its resolved category_color_slot, so the caller can render
/// a badge immediately without a second fetch), not full stats.
const PLAIN_SELECT_SQL: &str =
    "SELECT e.*, ec.color_slot AS category_color_slot FROM events e LEFT JOIN event_categories ec ON ec.id = e.category_id WHERE e.id = ?1";

fn map_event_with_stats(row: &Row) -> rusqlite::Result<EventWithStats> {
    let event = Event {
        id: row.get("id")?,
        name: row.get("name")?,
        artist_team: row.get("artist_team")?,
        venue: row.get("venue")?,
        city: row.get("city")?,
        country: row.get("country")?,
        event_date: row.get("event_date")?,
        category: row.get("category")?,
        category_id: row.get("category_id")?,
        category_color_slot: row.get("category_color_slot")?,
        status: row.get("status")?,
        notes: row.get("notes")?,
        is_demo: row.get("is_demo")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    };
    let stats = finance::compute_summary(
        row.get("purchased_tickets")?,
        row.get("available_tickets")?,
        row.get("listed_tickets")?,
        row.get("sold_tickets")?,
        row.get("cancelled_tickets")?,
        row.get("total_cost_cents")?,
        row.get("cogs_cents")?,
        row.get("revenue_cents")?,
        row.get("selling_fees_cents")?,
        row.get("currency")?,
    );
    Ok(EventWithStats { event, stats })
}

fn map_event_plain(row: &Row) -> rusqlite::Result<Event> {
    Ok(Event {
        id: row.get("id")?,
        name: row.get("name")?,
        artist_team: row.get("artist_team")?,
        venue: row.get("venue")?,
        city: row.get("city")?,
        country: row.get("country")?,
        event_date: row.get("event_date")?,
        category: row.get("category")?,
        category_id: row.get("category_id")?,
        category_color_slot: row.get("category_color_slot")?,
        status: row.get("status")?,
        notes: row.get("notes")?,
        is_demo: row.get("is_demo")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

pub(crate) fn fetch_recent(conn: &Connection, limit: i64) -> AppResult<Vec<EventWithStats>> {
    let sql = format!("{STATS_SQL} GROUP BY e.id ORDER BY e.created_at DESC LIMIT ?1");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([limit], map_event_with_stats)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

// 2.0.27: `category_id` appended at the end rather than inserted next to
// `search` - same "new params always go last" convention sales.rs's
// list_sale_groups_impl already documents, so no existing call site's
// argument order shifts.
#[tauri::command]
pub fn list_events(
    state: State<AppState>,
    search: Option<String>,
    category_id: Option<i64>,
) -> AppResult<Vec<EventWithStats>> {
    let conn = state.db.lock().unwrap();
    let mut sql = format!("{STATS_SQL} WHERE 1=1");
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![];
    if let Some(q) = search.as_deref() {
        let q = q.trim();
        if !q.is_empty() {
            sql.push_str(" AND (e.name LIKE ? OR e.artist_team LIKE ? OR e.venue LIKE ? OR e.city LIKE ?)");
            let like = format!("%{q}%");
            for _ in 0..4 {
                params_vec.push(Box::new(like.clone()));
            }
        }
    }
    if let Some(cid) = category_id {
        sql.push_str(" AND e.category_id = ?");
        params_vec.push(Box::new(cid));
    }
    sql.push_str(" GROUP BY e.id ORDER BY (e.event_date IS NULL), e.event_date DESC, e.id DESC");

    let mut stmt = conn.prepare(&sql)?;
    let param_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
    let rows = stmt.query_map(param_refs.as_slice(), map_event_with_stats)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[tauri::command]
pub fn get_event(state: State<AppState>, id: i64) -> AppResult<EventWithStats> {
    let conn = state.db.lock().unwrap();
    let sql = format!("{STATS_SQL} WHERE e.id = ?1 GROUP BY e.id");
    conn.query_row(&sql, [id], map_event_with_stats)
        .map_err(|_| AppError::NotFound(format!("Event #{id} not found")))
}

fn validate_input(input: &EventInput) -> AppResult<()> {
    if input.name.trim().is_empty() {
        return Err(AppError::Validation("Event name is required".into()));
    }
    if let Some(status) = &input.status {
        if !["upcoming", "completed", "cancelled"].contains(&status.as_str()) {
            return Err(AppError::Validation(format!("Invalid event status '{status}'")));
        }
    }
    Ok(())
}

/// 2.0.27: `events.category` (free text) is kept as a denormalized mirror of
/// `category_id`'s own name - see migrations/012_event_categories.sql's doc
/// comment for why. This is the one place that mirror gets written, so
/// `create_event`/`update_event` can never let the two drift apart. Returns a
/// friendly `Validation` error (rather than letting the INSERT/UPDATE fail on
/// the raw foreign-key constraint) when `category_id` doesn't actually exist -
/// same "resolve and validate before writing" spirit as every other write
/// path in this codebase.
fn resolve_category_name(conn: &Connection, category_id: Option<i64>) -> AppResult<Option<String>> {
    match category_id {
        None => Ok(None),
        Some(id) => conn
            .query_row("SELECT name FROM event_categories WHERE id = ?1", [id], |r| r.get(0))
            .map(Some)
            .map_err(|_| AppError::Validation(format!("Category #{id} does not exist"))),
    }
}

#[tauri::command]
pub fn create_event(state: State<AppState>, input: EventInput) -> AppResult<Event> {
    validate_input(&input)?;
    let conn = state.db.lock().unwrap();
    let category = resolve_category_name(&conn, input.category_id)?;
    conn.execute(
        "INSERT INTO events (name, artist_team, venue, city, country, event_date, category, category_id, status, notes)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        params![
            input.name.trim(),
            input.artist_team,
            input.venue,
            input.city,
            input.country,
            input.event_date,
            category,
            input.category_id,
            input.status.unwrap_or_else(|| "upcoming".to_string()),
            input.notes,
        ],
    )?;
    let id = conn.last_insert_rowid();
    Ok(conn.query_row(PLAIN_SELECT_SQL, [id], map_event_plain)?)
}

#[tauri::command]
pub fn update_event(state: State<AppState>, id: i64, input: EventInput) -> AppResult<Event> {
    validate_input(&input)?;
    let conn = state.db.lock().unwrap();
    let category = resolve_category_name(&conn, input.category_id)?;
    let changed = conn.execute(
        "UPDATE events SET name=?1, artist_team=?2, venue=?3, city=?4, country=?5, event_date=?6,
         category=?7, category_id=?8, status=?9, notes=?10, updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now')
         WHERE id=?11",
        params![
            input.name.trim(),
            input.artist_team,
            input.venue,
            input.city,
            input.country,
            input.event_date,
            category,
            input.category_id,
            input.status.unwrap_or_else(|| "upcoming".to_string()),
            input.notes,
            id,
        ],
    )?;
    if changed == 0 {
        return Err(AppError::NotFound(format!("Event #{id} not found")));
    }
    Ok(conn.query_row(PLAIN_SELECT_SQL, [id], map_event_plain)?)
}

/// Returns `Some(reason)` if event `id` cannot be safely deleted (it has any
/// orders - and therefore tickets - linked to it), `None` if it's safe.
/// Split out of `delete_event` in 2.0.28 so `bulk_delete_events_impl`
/// enforces EXACTLY the same rule, word-for-word, as deleting one event at a
/// time from Event Detail always has - the two paths can never drift on what
/// "safe to delete" means. Deliberately doesn't check whether the event
/// itself exists: the two callers want different behavior for a missing id
/// (single delete errors immediately; bulk delete records it as one skip
/// among possibly many and keeps going), so that check stays with each
/// caller.
fn event_delete_blocker(conn: &Connection, id: i64) -> AppResult<Option<String>> {
    let order_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM orders WHERE event_id = ?1",
        [id],
        |r| r.get(0),
    )?;
    if order_count > 0 {
        return Ok(Some(
            "This event has orders/tickets linked to it and cannot be deleted. Delete its orders first.".into(),
        ));
    }
    Ok(None)
}

#[tauri::command]
pub fn delete_event(state: State<AppState>, id: i64) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    if let Some(reason) = event_delete_blocker(&conn, id)? {
        return Err(AppError::Validation(reason));
    }
    let changed = conn.execute("DELETE FROM events WHERE id = ?1", [id])?;
    if changed == 0 {
        return Err(AppError::NotFound(format!("Event #{id} not found")));
    }
    Ok(())
}

/// 2.0.28: bulk delete for the new "Delete" selection mode on the Events
/// list. See `models::BulkDeleteResult`'s doc comment for why this uses a
/// per-id skip-with-reason model instead of the codebase's usual
/// all-or-nothing bulk-write pattern: everything that passes
/// `event_delete_blocker` is removed together in one transaction, and
/// anything that doesn't is reported back with the exact same message
/// `delete_event`/Event Detail already show for that same event, one at a
/// time.
pub(crate) fn bulk_delete_events_impl(conn: &mut Connection, ids: &[i64]) -> AppResult<BulkDeleteResult> {
    if ids.is_empty() {
        return Err(AppError::Validation("Select at least one event to delete".into()));
    }
    let tx = conn.transaction()?;
    let mut deleted_ids = Vec::new();
    let mut skipped = Vec::new();
    for &id in ids {
        if let Some(reason) = event_delete_blocker(&tx, id)? {
            skipped.push(BulkDeleteSkip { id, reason });
            continue;
        }
        let changed = tx.execute("DELETE FROM events WHERE id = ?1", [id])?;
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
pub fn bulk_delete_events(state: State<AppState>, ids: Vec<i64>) -> AppResult<BulkDeleteResult> {
    let mut conn = state.db.lock().unwrap();
    bulk_delete_events_impl(&mut conn, &ids)
}

/// 2.0.63 "Detect categories" - the manual, retroactive sibling of the
/// automatic detection `commands::orders_sheet_sync::resolve_or_create_
/// event` already runs on every brand-new event a sheet sync creates (see
/// ai_categorize.rs's own module doc comment for the full free-rules-then-AI
/// design marko confirmed). Only ever looks at events with `category_id IS
/// NULL` and a non-blank name - an event that already has a category,
/// however it got one (auto-detected, synced from the sheet, or picked by
/// hand), is never touched. That makes running this again, or running it
/// after more events have synced in, always safe - exactly the same "safe
/// to click repeatedly" property "Fix sync" (2.0.60) already holds itself
/// to.
fn detect_event_categories_impl(conn: &Connection) -> AppResult<CategoryDetectionResult> {
    let ai_configured = crate::ai_categorize::embedded_anthropic_api_key().is_some();
    let rows: Vec<(i64, String)> = {
        let mut stmt =
            conn.prepare("SELECT id, name FROM events WHERE category_id IS NULL AND trim(name) <> ''")?;
        let collected = stmt
            .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        collected
    };

    let mut categorized_by_rule = 0i64;
    let mut categorized_by_ai = 0i64;
    let mut left_uncategorized = 0i64;
    for (id, name) in &rows {
        match crate::ai_categorize::detect_category_for_event_name(conn, name) {
            Some(m) => {
                conn.execute(
                    "UPDATE events SET category_id = ?1, category = ?2 WHERE id = ?3",
                    params![m.id, m.name, id],
                )?;
                if m.via_ai {
                    categorized_by_ai += 1;
                } else {
                    categorized_by_rule += 1;
                }
            }
            None => left_uncategorized += 1,
        }
    }
    Ok(CategoryDetectionResult {
        checked: rows.len() as i64,
        categorized_by_rule,
        categorized_by_ai,
        left_uncategorized,
        ai_configured,
    })
}

#[tauri::command]
pub fn detect_event_categories(state: State<AppState>) -> AppResult<CategoryDetectionResult> {
    let conn = state.db.lock().unwrap();
    detect_event_categories_impl(&conn)
}

#[cfg(test)]
mod tests {
    //! events.rs has never had a test module (see PROTECTED-AREAS-NOTES.md -
    //! it deliberately doesn't follow this codebase's usual "impl fn + thin
    //! wrapper" testable-command pattern for its older commands, and that's
    //! left alone here). This module covers ONLY the new 2.0.28 bulk-delete
    //! logic, which - unlike list_events' filters etc. - is genuine new
    //! business logic worth a real regression test, same bar the rest of the
    //! codebase already holds itself to for new behavior.
    use super::*;
    use crate::db::test_conn;

    fn seed_event_named(conn: &Connection, name: &str) -> i64 {
        conn.execute("INSERT INTO events (name) VALUES (?1)", [name]).unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn bulk_delete_events_removes_every_selected_event_with_no_orders() {
        let mut conn = test_conn();
        let a = seed_event_named(&conn, "Event A");
        let b = seed_event_named(&conn, "Event B");
        let c = seed_event_named(&conn, "Event C");

        let result = bulk_delete_events_impl(&mut conn, &[a, b]).unwrap();

        assert_eq!(result.deleted_ids, vec![a, b]);
        assert!(result.skipped.is_empty());
        let remaining: i64 = conn.query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0)).unwrap();
        assert_eq!(remaining, 1, "only the unselected event_c should be left");
        let c_still_there: i64 = conn
            .query_row("SELECT COUNT(*) FROM events WHERE id = ?1", [c], |r| r.get(0))
            .unwrap();
        assert_eq!(c_still_there, 1);
    }

    #[test]
    fn bulk_delete_events_skips_one_with_an_order_but_still_deletes_the_rest() {
        let mut conn = test_conn();
        let safe_event = seed_event_named(&conn, "Deletable event");
        let blocked_event = seed_event_named(&conn, "Event with an order");
        conn.execute(
            "INSERT INTO orders (code, event_id, purchase_date, quantity, currency)
             VALUES ('ORD-000001', ?1, '2026-01-01', 1, 'EUR')",
            [blocked_event],
        )
        .unwrap();

        let result = bulk_delete_events_impl(&mut conn, &[safe_event, blocked_event]).unwrap();

        assert_eq!(result.deleted_ids, vec![safe_event], "the safe event must still go through");
        assert_eq!(result.skipped.len(), 1);
        assert_eq!(result.skipped[0].id, blocked_event);
        assert!(result.skipped[0].reason.contains("orders/tickets linked"));

        let blocked_still_there: i64 = conn
            .query_row("SELECT COUNT(*) FROM events WHERE id = ?1", [blocked_event], |r| r.get(0))
            .unwrap();
        assert_eq!(blocked_still_there, 1, "the blocked event must survive, not be partially touched");
    }

    #[test]
    fn bulk_delete_events_rejects_an_empty_selection() {
        let mut conn = test_conn();
        let err = bulk_delete_events_impl(&mut conn, &[]).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    // -- detect_event_categories_impl (2.0.63) ------------------------------
    //
    // No ANTHROPIC_API_KEY in this test environment (see ai_categorize::
    // embedded_anthropic_api_key's own test), so `ai_configured` is always
    // false here and only the free keyword rules can actually resolve
    // anything - these tests are about the retroactive scan/update logic
    // itself, not the rule/AI decision (covered by ai_categorize.rs's tests).

    #[test]
    fn only_scans_events_that_currently_have_no_category() {
        let conn = test_conn();
        let already_categorized = seed_event_named(&conn, "Monaco Grand Prix");
        conn.execute(
            "UPDATE events SET category_id = 1, category = 'Concert' WHERE id = ?1",
            [already_categorized],
        )
        .unwrap();
        let uncategorized = seed_event_named(&conn, "Reading Festival");

        let result = detect_event_categories_impl(&conn).unwrap();

        assert_eq!(result.checked, 1, "the already-categorized event must not even be looked at");
        assert_eq!(result.categorized_by_rule, 1);
        let category: Option<String> =
            conn.query_row("SELECT category FROM events WHERE id = ?1", [uncategorized], |r| r.get(0)).unwrap();
        assert_eq!(category.as_deref(), Some("Festival"));
        // and the already-categorized one must be completely untouched:
        let untouched: String = conn
            .query_row("SELECT category FROM events WHERE id = ?1", [already_categorized], |r| r.get(0))
            .unwrap();
        assert_eq!(untouched, "Concert");
    }

    #[test]
    fn counts_rule_matches_and_leftovers_separately_and_reports_ai_not_configured() {
        let conn = test_conn();
        seed_event_named(&conn, "Monaco Grand Prix"); // free-rule match
        seed_event_named(&conn, "Celine Dion"); // no signal, no AI key here

        let result = detect_event_categories_impl(&conn).unwrap();

        assert_eq!(result.checked, 2);
        assert_eq!(result.categorized_by_rule, 1);
        assert_eq!(result.categorized_by_ai, 0);
        assert_eq!(result.left_uncategorized, 1);
        assert!(!result.ai_configured, "this test build never has ANTHROPIC_API_KEY set");
    }

    #[test]
    fn running_it_twice_in_a_row_is_a_no_op_the_second_time() {
        let conn = test_conn();
        seed_event_named(&conn, "Monaco Grand Prix");

        let first = detect_event_categories_impl(&conn).unwrap();
        let second = detect_event_categories_impl(&conn).unwrap();

        assert_eq!(first.categorized_by_rule, 1);
        assert_eq!(second.checked, 0, "already-categorized now, so the second run finds nothing left to scan");
        assert_eq!(second.categorized_by_rule, 0);
    }

    #[test]
    fn a_blank_named_event_is_never_scanned() {
        let conn = test_conn();
        seed_event_named(&conn, "");
        let result = detect_event_categories_impl(&conn).unwrap();
        assert_eq!(result.checked, 0);
    }
}

//! 2.4.3: "Ticket Control Center" - marko's own request for one dense work
//! screen to manage/inspect tickets across EVERY event at once, built
//! directly on top of the existing `tickets`/`ticket_listings`/`sales`
//! tables. Explicitly NOT a new parallel ticket system - marko's own words:
//! "Nechcem nový paralelný ticket systém... Má to byť jedna pohodlná
//! pracovná view nad EXISTUJÚCIMI tickets dátami."
//!
//! This module owns exactly ONE thing: a single read-only list query
//! (`list_control_center_tickets`), following this codebase's own "each view
//! aggregator writes its own SELECT" convention already used by
//! `attention_center`/`inventory_intelligence`/`ticket_listings`, rather than
//! calling into `tickets::list_tickets_impl` and reshaping its result -
//! this view needs columns (`events.event_date`, the per-marketplace
//! `ticket_listings` join, a "was this ever refunded" signal) that query
//! doesn't select, and a second, differently-shaped join belongs in its own
//! query rather than bolted onto `tickets::BASE_SQL` (which several other,
//! already-tested call sites depend on staying exactly as it is).
//!
//! EVERY write this screen can trigger reuses a command that already
//! existed before this task - see `TicketControlCenter.tsx`'s own doc
//! comment for the exact mapping (generic field edit incl. the new `Tier`
//! option -> `bulk_update_tickets`; "change listing status" ->
//! `bulk_update_ticket_listings_status`; "export selected" ->
//! `export_tickets_csv_selected`). Nothing in this module ever writes
//! anything - no money/cents, no `batch_id`, no refund/resell logic, no
//! Listings/Sales/Orders/Finance core, are touched here at all.
//!
//! ## Query shape
//!
//! One row is normally one ticket. A ticket currently listed on more than
//! one marketplace at once produces one row PER (ticket, listing) pair
//! instead - the exact same fan-out `ticket_listings::
//! list_ticket_listings_for_event_impl` already produces for its own,
//! listings-only view (see that module's doc comment), just joined here from
//! the ticket side instead of the listing side so a ticket with ZERO
//! listings still gets exactly one row (via `LEFT JOIN`), with every
//! `listing_*`/`marketplace_*` field `None`.
//!
//! The active-sale join reuses the exact same guarded shape
//! `tickets::BASE_SQL` already established (migration 004, BUG #1):
//! `LEFT JOIN sales sa ON sa.ticket_id = t.id AND sa.payment_status !=
//! 'refunded'` - restricting to the one non-refunded sale per ticket so this
//! query can never fan a single ticket out into extra rows just because it
//! was once refunded and later resold. The unavoidable side effect of that
//! guard (also true of `tickets::BASE_SQL` itself) is that a
//! refunded-and-not-yet-resold ticket's `sale_payment_status` reads exactly
//! like a never-sold ticket's - `None` either way. marko's spec asks for a
//! "Refunded" quick filter, which needs to tell those two cases apart, so
//! this query adds one small ADDITIVE, read-only signal neither
//! `tickets.rs` nor `ticket_listings.rs` has any reason to carry:
//! `is_refunded`, an `EXISTS (SELECT 1 FROM sales WHERE ticket_id = t.id AND
//! payment_status = 'refunded')` correlated subquery. It only ever reads the
//! `sales` table - it cannot affect refund/resell behaviour, and nothing
//! else in the app reads it.
//!
//! "Listing price" coalesces the ticket's real per-marketplace
//! `ticket_listings.price_cents` (for the row's own listing, when one
//! exists) with the legacy single `tickets.listing_price_cents` (for a
//! ticket that has never been posted as a real listing) - same "two
//! parallel listing-price systems, both real, deliberately not unified"
//! precedent already recorded in `PROTECTED_AREAS.md`'s "2.2.6" entry for
//! Inventory Intelligence's own Overview cards. "Purchase price" reuses
//! `Ticket.total_cost_cents`'s own definition (purchase cost + fees + other
//! costs), computed here directly in SQL rather than fetching the 3
//! components separately (this view has no use for them individually).

use crate::commands::tickets::LIST_CAP;
use crate::db::AppState;
use crate::error::AppResult;
use crate::models::ControlCenterFilters;
use crate::models::ControlCenterTicket;
use rusqlite::{Connection, Row};
use tauri::State;

const CONTROL_CENTER_SELECT: &str = "
    SELECT t.id, t.code, t.event_id, e.name AS event_name, e.event_date,
      t.order_id, o.code AS order_code,
      t.section, t.row_label, t.tier, t.seat,
      (t.purchase_cost_cents + t.purchase_fees_cents + t.other_costs_cents) AS total_cost_cents,
      COALESCE(tl.price_cents, t.listing_price_cents) AS listing_price_cents,
      t.currency, t.status, t.resale_status, t.delivery_status,
      sa.id AS sale_id, sa.payment_status AS sale_payment_status,
      EXISTS (SELECT 1 FROM sales sr WHERE sr.ticket_id = t.id AND sr.payment_status = 'refunded') AS is_refunded,
      tl.id AS listing_row_id, tl.marketplace_id, m.name AS marketplace_name,
      tl.listing_id AS listing_external_id, tl.listing_url, tl.status AS listing_status,
      t.is_demo
    FROM tickets t
    JOIN events e ON e.id = t.event_id
    JOIN orders o ON o.id = t.order_id
    LEFT JOIN sales sa ON sa.ticket_id = t.id AND sa.payment_status != 'refunded'
    LEFT JOIN ticket_listings tl ON tl.ticket_id = t.id
    LEFT JOIN marketplaces m ON m.id = tl.marketplace_id
";

fn map_control_center_ticket(row: &Row) -> rusqlite::Result<ControlCenterTicket> {
    Ok(ControlCenterTicket {
        id: row.get("id")?,
        code: row.get("code")?,
        event_id: row.get("event_id")?,
        event_name: row.get("event_name")?,
        event_date: row.get("event_date")?,
        order_id: row.get("order_id")?,
        order_code: row.get("order_code")?,
        section: row.get("section")?,
        row_label: row.get("row_label")?,
        tier: row.get("tier")?,
        seat: row.get("seat")?,
        total_cost_cents: row.get("total_cost_cents")?,
        listing_price_cents: row.get("listing_price_cents")?,
        currency: row.get("currency")?,
        status: row.get("status")?,
        resale_status: row.get("resale_status")?,
        delivery_status: row.get("delivery_status")?,
        sale_id: row.get("sale_id")?,
        sale_payment_status: row.get("sale_payment_status")?,
        is_refunded: row.get("is_refunded")?,
        listing_row_id: row.get("listing_row_id")?,
        marketplace_id: row.get("marketplace_id")?,
        marketplace_name: row.get("marketplace_name")?,
        listing_external_id: row.get("listing_external_id")?,
        listing_url: row.get("listing_url")?,
        listing_status: row.get("listing_status")?,
        is_demo: row.get("is_demo")?,
    })
}

/// Appends `AND {column} = ?` (or `AND {column} IN (?,?,...)` for more than
/// one value) for a comma-separated filter string - the exact same
/// accepts-a-single-value-or-a-comma-list convention
/// `tickets::list_tickets_impl` already established for its own `status`
/// parameter, lifted into a small local helper here since this query has 5
/// columns that each need it (ticket status, listing status, sale status,
/// payment status, delivery status) instead of just the one.
fn push_in_filter(sql: &mut String, params_vec: &mut Vec<Box<dyn rusqlite::ToSql>>, column: &str, raw: &Option<String>) {
    let Some(s) = raw.as_deref() else { return };
    let values: Vec<String> = s.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect();
    if values.is_empty() {
        return;
    }
    if values.len() == 1 {
        sql.push_str(&format!(" AND {column} = ?"));
        params_vec.push(Box::new(values[0].clone()));
    } else {
        let placeholders = values.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        sql.push_str(&format!(" AND {column} IN ({placeholders})"));
        for v in values {
            params_vec.push(Box::new(v));
        }
    }
}

/// Split out from the `list_control_center_tickets` command (same
/// `_impl`/thin-wrapper split as every other list query in this codebase) so
/// it's directly unit-testable against a plain `&Connection`.
pub(crate) fn list_control_center_tickets_impl(
    conn: &Connection,
    filters: &ControlCenterFilters,
) -> AppResult<Vec<ControlCenterTicket>> {
    let mut sql = String::from(CONTROL_CENTER_SELECT);
    sql.push_str(" WHERE 1=1");
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![];

    if let Some(eid) = filters.event_id {
        sql.push_str(" AND t.event_id = ?");
        params_vec.push(Box::new(eid));
    }
    if let Some(from) = filters.date_from.as_deref() {
        if !from.is_empty() {
            sql.push_str(" AND e.event_date >= ?");
            params_vec.push(Box::new(from.to_string()));
        }
    }
    if let Some(to) = filters.date_to.as_deref() {
        if !to.is_empty() {
            sql.push_str(" AND e.event_date <= ?");
            params_vec.push(Box::new(to.to_string()));
        }
    }
    // Substring match, same convention `search` below already uses for
    // free-text ticket metadata - see `ControlCenterFilters::tier`'s own doc
    // comment (models.rs) for why this isn't an exact match.
    if let Some(tier) = filters.tier.as_deref() {
        if !tier.is_empty() {
            sql.push_str(" AND t.tier LIKE ?");
            params_vec.push(Box::new(format!("%{tier}%")));
        }
    }
    if let Some(section) = filters.section.as_deref() {
        if !section.is_empty() {
            sql.push_str(" AND t.section LIKE ?");
            params_vec.push(Box::new(format!("%{section}%")));
        }
    }
    if let Some(row_label) = filters.row_label.as_deref() {
        if !row_label.is_empty() {
            sql.push_str(" AND t.row_label LIKE ?");
            params_vec.push(Box::new(format!("%{row_label}%")));
        }
    }
    push_in_filter(&mut sql, &mut params_vec, "t.status", &filters.ticket_status);
    push_in_filter(&mut sql, &mut params_vec, "tl.status", &filters.listing_status);
    push_in_filter(&mut sql, &mut params_vec, "t.resale_status", &filters.sale_status);
    push_in_filter(&mut sql, &mut params_vec, "sa.payment_status", &filters.payment_status);
    push_in_filter(&mut sql, &mut params_vec, "t.delivery_status", &filters.delivery_status);
    if let Some(mid) = filters.marketplace_id {
        sql.push_str(" AND tl.marketplace_id = ?");
        params_vec.push(Box::new(mid));
    }
    // The "Refunded" quick filter - see this module's own doc comment for
    // why it needs its own EXISTS check rather than reusing payment_status.
    if filters.refunded_only == Some(true) {
        sql.push_str(" AND EXISTS (SELECT 1 FROM sales sr WHERE sr.ticket_id = t.id AND sr.payment_status = 'refunded')");
    }
    if let Some(q) = filters.search.as_deref() {
        let q = q.trim();
        if !q.is_empty() {
            // Extends `list_tickets_impl`'s own 6-column search (ticket
            // code/section/seat/row/event name/order code) with marko's 2
            // extra targets for this view: marketplace name, and the
            // marketplace's own listing id/URL text.
            sql.push_str(
                " AND (t.code LIKE ? OR o.code LIKE ? OR e.name LIKE ? OR t.section LIKE ? OR t.row_label LIKE ? \
                 OR m.name LIKE ? OR tl.listing_id LIKE ? OR tl.listing_url LIKE ?)",
            );
            let like = format!("%{q}%");
            for _ in 0..8 {
                params_vec.push(Box::new(like.clone()));
            }
        }
    }

    // Soonest event first (nulls last), same "soonest first" default this
    // app switched every other list to in 2.0.65 - ticket id as a stable
    // tiebreaker. No user-facing Sort control was requested for this screen
    // (see TicketControlCenter.tsx's own doc comment), so this is the one
    // fixed order every view of this data gets.
    sql.push_str(&format!(" ORDER BY (e.event_date IS NULL), e.event_date ASC, t.id ASC LIMIT {LIST_CAP}"));

    let mut stmt = conn.prepare(&sql)?;
    let param_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
    let rows = stmt.query_map(param_refs.as_slice(), map_control_center_ticket)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[tauri::command]
pub fn list_control_center_tickets(state: State<AppState>, filters: ControlCenterFilters) -> AppResult<Vec<ControlCenterTicket>> {
    let conn = state.db.lock().unwrap();
    list_control_center_tickets_impl(&conn, &filters)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::sales::{create_sale_impl, refund_sale_impl};
    use crate::db::test_conn;
    use crate::models::SaleInput;

    // Same minimal, direct-SQL event/order/ticket seeding shape as
    // `ticket_listings::tests`' own `seed_event`/`seed_ticket` (kept as its
    // own local copy for the same reason that module's doc comment gives -
    // these are private to each module's own `mod tests`), extended with a
    // real `event_date` param since this module's date-range/sort behaviour
    // needs one.
    fn seed_event(conn: &Connection, name: &str, event_date: Option<&str>) -> i64 {
        conn.execute(
            "INSERT INTO events (name, event_date) VALUES (?1, ?2)",
            rusqlite::params![name, event_date],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    fn seed_ticket(conn: &Connection, code_suffix: &str, event_id: i64) -> i64 {
        conn.execute(
            "INSERT INTO orders (code, event_id, purchase_date, quantity, currency) VALUES (?1, ?2, '2026-01-01', 1, 'EUR')",
            rusqlite::params![format!("ORD-{code_suffix}"), event_id],
        )
        .unwrap();
        let order_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO tickets (code, event_id, order_id, section, row_label, tier, seat, currency, status)
             VALUES (?1, ?2, ?3, 'A', '1', 'VIP', '12', 'EUR', 'available')",
            rusqlite::params![format!("TKT-{code_suffix}"), event_id, order_id],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    fn seed_marketplace(conn: &Connection, name: &str) -> i64 {
        conn.execute("INSERT INTO marketplaces(name) VALUES (?1)", [name]).unwrap();
        conn.last_insert_rowid()
    }

    fn seed_listing(conn: &Connection, ticket_id: i64, marketplace_id: i64, price_cents: i64, status: &str) -> i64 {
        conn.execute(
            "INSERT INTO ticket_listings (ticket_id, marketplace_id, price_cents, currency, status) VALUES (?1, ?2, ?3, 'EUR', ?4)",
            rusqlite::params![ticket_id, marketplace_id, price_cents, status],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn returns_a_ticket_with_no_listing_exactly_once_with_null_listing_fields() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event", Some("2026-12-01"));
        let ticket_id = seed_ticket(&conn, "1", event_id);

        let results = list_control_center_tickets_impl(&conn, &ControlCenterFilters::default()).unwrap();
        assert_eq!(results.len(), 1);
        let row = &results[0];
        assert_eq!(row.id, ticket_id);
        assert_eq!(row.event_id, event_id);
        assert_eq!(row.event_date.as_deref(), Some("2026-12-01"));
        assert_eq!(row.tier.as_deref(), Some("VIP"));
        assert_eq!(row.status, "available");
        assert_eq!(row.sale_id, None);
        assert!(!row.is_refunded, "a plain never-sold ticket must not read as refunded");
        assert_eq!(row.listing_row_id, None);
        assert_eq!(row.marketplace_id, None);
        assert_eq!(row.marketplace_name, None);
        assert_eq!(row.listing_status, None);
    }

    #[test]
    fn fans_out_one_row_per_marketplace_listing_but_never_drops_an_unlisted_ticket() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event", Some("2026-12-01"));
        let listed_twice = seed_ticket(&conn, "1", event_id);
        let never_listed = seed_ticket(&conn, "2", event_id);
        // Names deliberately distinct from the real marketplaces migrations
        // already seed (StubHub/Vivid Seats/Ticombo/Viagogo) - reusing one of
        // those here would collide with `marketplaces.name`'s UNIQUE
        // constraint against real seed data `test_conn()` already applies.
        let market_a = seed_marketplace(&conn, "TestMarket A");
        let market_b = seed_marketplace(&conn, "TestMarket B");
        seed_listing(&conn, listed_twice, market_a, 15000, "active");
        seed_listing(&conn, listed_twice, market_b, 16000, "active");

        let results = list_control_center_tickets_impl(&conn, &ControlCenterFilters::default()).unwrap();
        assert_eq!(results.len(), 3, "2 listing rows for one ticket + 1 plain row for the other, never fewer/more");

        let listed_rows: Vec<_> = results.iter().filter(|r| r.id == listed_twice).collect();
        assert_eq!(listed_rows.len(), 2);
        let marketplaces: std::collections::HashSet<_> = listed_rows.iter().filter_map(|r| r.marketplace_name.clone()).collect();
        assert_eq!(marketplaces, std::collections::HashSet::from(["TestMarket A".to_string(), "TestMarket B".to_string()]));
        // Ticket-level fields must repeat identically across both of this
        // ticket's own rows - only the listing-specific columns differ.
        for r in &listed_rows {
            assert_eq!(r.tier.as_deref(), Some("VIP"));
        }

        let unlisted_rows: Vec<_> = results.iter().filter(|r| r.id == never_listed).collect();
        assert_eq!(unlisted_rows.len(), 1, "a ticket with zero listings must still appear exactly once");
        assert_eq!(unlisted_rows[0].marketplace_name, None);
    }

    /// Same BUG #1 regression `tickets::list_tickets_impl`'s own test suite
    /// already covers, replayed against THIS module's independent join -
    /// marko's explicit "refund/resell regresia" test requirement.
    #[test]
    fn appears_exactly_once_despite_a_refunded_and_a_new_active_sale() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn, "Test Event", None);
        let ticket_id = seed_ticket(&conn, "1", event_id);

        let first_sale = SaleInput {
            ticket_id,
            platform_id: None,
            sale_date: "2026-02-01".to_string(),
            sale_price_cents: 2000,
            selling_fees_cents: 0,
            payment_status: Some("paid".to_string()),
            buyer_reference: None,
            notes: None,
        };
        let sale_id_1 = create_sale_impl(&mut conn, &first_sale).unwrap();
        refund_sale_impl(&mut conn, sale_id_1, Some("buyer cancelled")).unwrap();
        let second_sale = SaleInput { sale_price_cents: 1800, ..first_sale };
        let sale_id_2 = create_sale_impl(&mut conn, &second_sale).unwrap();

        let results = list_control_center_tickets_impl(&conn, &ControlCenterFilters::default()).unwrap();
        assert_eq!(results.len(), 1, "must never be fanned out by the refunded-plus-active sales join");
        assert_eq!(results[0].sale_id, Some(sale_id_2));
        assert_eq!(results[0].sale_payment_status.as_deref(), Some("paid"));
        assert!(results[0].is_refunded, "this ticket DOES have refund history, even though it's now actively sold again");
    }

    #[test]
    fn a_refunded_and_not_yet_resold_ticket_is_flagged_refunded_with_no_payment_status() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn, "Test Event", None);
        let ticket_id = seed_ticket(&conn, "1", event_id);
        let sale = SaleInput {
            ticket_id,
            platform_id: None,
            sale_date: "2026-02-01".to_string(),
            sale_price_cents: 2000,
            selling_fees_cents: 0,
            payment_status: Some("paid".to_string()),
            buyer_reference: None,
            notes: None,
        };
        let sale_id = create_sale_impl(&mut conn, &sale).unwrap();
        refund_sale_impl(&mut conn, sale_id, Some("buyer cancelled")).unwrap();

        let results = list_control_center_tickets_impl(&conn, &ControlCenterFilters::default()).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, "available", "refund_sale_impl reverts the ticket to available");
        assert_eq!(results[0].sale_id, None);
        assert_eq!(results[0].sale_payment_status, None, "the active-sale join alone can't see a fully-refunded ticket");
        assert!(results[0].is_refunded, "is_refunded is what makes this case distinguishable from a never-sold ticket");
    }

    #[test]
    fn refunded_only_filter_keeps_only_tickets_with_refund_history() {
        let mut conn = test_conn();
        let event_id = seed_event(&conn, "Test Event", None);
        let plain_id = seed_ticket(&conn, "1", event_id);
        let refunded_id = seed_ticket(&conn, "2", event_id);
        let sale = SaleInput {
            ticket_id: refunded_id,
            platform_id: None,
            sale_date: "2026-02-01".to_string(),
            sale_price_cents: 2000,
            selling_fees_cents: 0,
            payment_status: Some("paid".to_string()),
            buyer_reference: None,
            notes: None,
        };
        let sale_id = create_sale_impl(&mut conn, &sale).unwrap();
        refund_sale_impl(&mut conn, sale_id, None).unwrap();

        let filters = ControlCenterFilters { refunded_only: Some(true), ..Default::default() };
        let results = list_control_center_tickets_impl(&conn, &filters).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, refunded_id);
        assert_ne!(results[0].id, plain_id);
    }

    #[test]
    fn filters_by_event_id() {
        let conn = test_conn();
        let event_a = seed_event(&conn, "Event A", None);
        let event_b = seed_event(&conn, "Event B", None);
        seed_ticket(&conn, "1", event_a);
        seed_ticket(&conn, "2", event_b);

        let filters = ControlCenterFilters { event_id: Some(event_a), ..Default::default() };
        let results = list_control_center_tickets_impl(&conn, &filters).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].event_id, event_a);
    }

    #[test]
    fn filters_by_ticket_status_comma_list() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event", None);
        let available_id = seed_ticket(&conn, "1", event_id);
        let cancelled_id = seed_ticket(&conn, "2", event_id);
        let listed_id = seed_ticket(&conn, "3", event_id); // never matched below
        conn.execute("UPDATE tickets SET status='cancelled' WHERE id=?1", [cancelled_id]).unwrap();
        conn.execute("UPDATE tickets SET status='listed' WHERE id=?1", [listed_id]).unwrap();

        let filters = ControlCenterFilters { ticket_status: Some("available,cancelled".to_string()), ..Default::default() };
        let results = list_control_center_tickets_impl(&conn, &filters).unwrap();
        let ids: std::collections::HashSet<_> = results.iter().map(|r| r.id).collect();
        assert_eq!(ids, std::collections::HashSet::from([available_id, cancelled_id]));
    }

    #[test]
    fn filters_by_marketplace_and_listing_status() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event", None);
        let ticket_id = seed_ticket(&conn, "1", event_id);
        let other_ticket_id = seed_ticket(&conn, "2", event_id);
        let market_a = seed_marketplace(&conn, "TestMarket A");
        let market_b = seed_marketplace(&conn, "TestMarket B");
        seed_listing(&conn, ticket_id, market_a, 15000, "active");
        seed_listing(&conn, other_ticket_id, market_b, 16000, "removed");

        let filters = ControlCenterFilters { marketplace_id: Some(market_a), ..Default::default() };
        let results = list_control_center_tickets_impl(&conn, &filters).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, ticket_id);

        let filters = ControlCenterFilters { listing_status: Some("active".to_string()), ..Default::default() };
        let results = list_control_center_tickets_impl(&conn, &filters).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, ticket_id);
    }

    #[test]
    fn search_matches_marketplace_name_and_listing_id() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event", None);
        let ticket_id = seed_ticket(&conn, "1", event_id);
        seed_ticket(&conn, "2", event_id);
        let market_a = seed_marketplace(&conn, "TestMarket A");
        seed_listing(&conn, ticket_id, market_a, 15000, "active");

        let filters = ControlCenterFilters { search: Some("TestMarket A".to_string()), ..Default::default() };
        let results = list_control_center_tickets_impl(&conn, &filters).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, ticket_id);
    }

    #[test]
    fn date_range_filters_by_event_date() {
        let conn = test_conn();
        let soon = seed_event(&conn, "Soon Event", Some("2026-09-10"));
        let later = seed_event(&conn, "Later Event", Some("2026-12-25"));
        let soon_ticket = seed_ticket(&conn, "1", soon);
        seed_ticket(&conn, "2", later);

        let filters = ControlCenterFilters {
            date_from: Some("2026-09-01".to_string()),
            date_to: Some("2026-10-01".to_string()),
            ..Default::default()
        };
        let results = list_control_center_tickets_impl(&conn, &filters).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, soon_ticket);
    }

    #[test]
    fn tier_filter_is_a_free_text_substring_match() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event", None);
        let vip_id = seed_ticket(&conn, "1", event_id); // seed_ticket always sets tier='VIP'

        let filters = ControlCenterFilters { tier: Some("VIP".to_string()), ..Default::default() };
        let results = list_control_center_tickets_impl(&conn, &filters).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, vip_id);

        let filters = ControlCenterFilters { tier: Some("Lower Bowl".to_string()), ..Default::default() };
        let results = list_control_center_tickets_impl(&conn, &filters).unwrap();
        assert_eq!(results.len(), 0);
    }
}

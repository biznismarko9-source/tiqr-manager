//! 2.2.4: marko's own request to turn "Listings" from a read-only view of
//! `Ticket.listing_price_cents`/`status` into a real multi-marketplace
//! listing system - "jeden ticket moze byt zalistovany na viacerych
//! marketplace" (one ticket can be listed on several marketplaces at once:
//! StubHub, Vivid, Ticombo, each its own price/status/URL). See
//! migrations/022_ticket_listings.sql for the full schema design (why a
//! separate table rather than more `tickets` columns, why `ticket_id`/
//! `marketplace_id` are both `ON DELETE CASCADE`, why the dedup `UNIQUE`
//! constraint is shaped the way it is).
//!
//! Pure CRUD plus one event-scoped list query - no automation, no API calls,
//! no repricing (marko's own explicit "Dôležité" list this release). Per
//! marko's own explicit instruction not to change existing tickets/
//! inventory/sales/refund logic, this module never reads or writes
//! `tickets.status`/`tickets.listing_price_cents` at all - those columns
//! stay exactly what they were, completely untouched by this feature.
//!
//! Same "impl function + thin `#[tauri::command]` wrapper" split, and the
//! same "one shared SELECT, used by list AND by the create/update read-back"
//! discipline, as `commands::finance_entries` - see that module for the
//! precedent this one follows throughout.

use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::models::{TicketListing, TicketListingInput};
use rusqlite::{Connection, OptionalExtension, Row};
use tauri::State;

const TICKET_LISTING_STATUSES: [&str; 3] = ["active", "sold", "removed"];

/// Every read goes through this same SELECT (list, and the create/update
/// "read the row back" step) so the two can never disagree about which
/// columns/joins a `TicketListing` is built from - same discipline as
/// `finance_entries::FINANCE_ENTRY_SELECT`.
const TICKET_LISTING_SELECT: &str = "SELECT tl.id, tl.ticket_id,
    t.code AS ticket_code, t.section AS ticket_section, t.row_label AS ticket_row_label, t.seat AS ticket_seat,
    tl.marketplace_id, m.name AS marketplace_name,
    tl.listing_id, tl.listing_url, tl.price_cents, tl.currency, tl.status, tl.is_demo, tl.created_at, tl.updated_at
    FROM ticket_listings tl
    JOIN tickets t ON t.id = tl.ticket_id
    JOIN marketplaces m ON m.id = tl.marketplace_id";

fn map_ticket_listing(row: &Row) -> rusqlite::Result<TicketListing> {
    Ok(TicketListing {
        id: row.get("id")?,
        ticket_id: row.get("ticket_id")?,
        ticket_code: row.get("ticket_code")?,
        ticket_section: row.get("ticket_section")?,
        ticket_row_label: row.get("ticket_row_label")?,
        ticket_seat: row.get("ticket_seat")?,
        marketplace_id: row.get("marketplace_id")?,
        marketplace_name: row.get("marketplace_name")?,
        listing_id: row.get("listing_id")?,
        listing_url: row.get("listing_url")?,
        price_cents: row.get("price_cents")?,
        currency: row.get("currency")?,
        status: row.get("status")?,
        is_demo: row.get("is_demo")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

/// Blank/whitespace-only optional text collapses to `None` rather than being
/// stored as an empty string - same rule (and same reasoning) as
/// `finance_entries::normalize_optional`, kept as its own small local copy
/// here rather than reaching into another command module for a five-line
/// helper - matches how `map_x`/small validators are local to each module
/// throughout this codebase.
fn normalize_optional(s: Option<String>) -> Option<String> {
    s.and_then(|v| {
        let t = v.trim().to_string();
        if t.is_empty() {
            None
        } else {
            Some(t)
        }
    })
}

fn validate_listing_fields(input: &TicketListingInput) -> AppResult<()> {
    if input.price_cents < 0 {
        return Err(AppError::Validation("Listing price cannot be negative".into()));
    }
    if input.currency.trim().is_empty() {
        return Err(AppError::Validation("Currency cannot be empty".into()));
    }
    if !TICKET_LISTING_STATUSES.contains(&input.status.as_str()) {
        return Err(AppError::Validation(format!(
            "Invalid listing status '{}' - must be 'active', 'sold' or 'removed'",
            input.status
        )));
    }
    Ok(())
}

fn validate_ticket_exists(conn: &Connection, ticket_id: i64) -> AppResult<()> {
    let exists: Option<i64> = conn
        .query_row("SELECT id FROM tickets WHERE id = ?1", [ticket_id], |r| r.get(0))
        .optional()?;
    if exists.is_none() {
        return Err(AppError::Validation(format!("Ticket #{ticket_id} not found")));
    }
    Ok(())
}

fn validate_marketplace_exists(conn: &Connection, marketplace_id: i64) -> AppResult<()> {
    let exists: Option<i64> = conn
        .query_row("SELECT id FROM marketplaces WHERE id = ?1", [marketplace_id], |r| r.get(0))
        .optional()?;
    if exists.is_none() {
        return Err(AppError::Validation(format!("Marketplace #{marketplace_id} not found")));
    }
    Ok(())
}

/// Maps a `UNIQUE(ticket_id, marketplace_id, listing_id)` violation to a
/// clear, specific message - same "catch the SQLite constraint name,
/// translate it" pattern as `price_checker::create_marketplace`/
/// `finance_entries::create_finance_category_impl`. This is marko's own
/// "ziadne duplicity" (no duplicates) requirement - see the migration's own
/// doc comment for exactly what counts as a duplicate (same ticket + same
/// marketplace + same listing id - two hand-entered listings with no id yet
/// are NOT duplicates of each other, by design).
fn map_write_error(e: rusqlite::Error) -> AppError {
    match &e {
        rusqlite::Error::SqliteFailure(_, Some(m)) if m.contains("UNIQUE") => AppError::Validation(
            "This ticket already has a listing on that marketplace with the same listing ID.".into(),
        ),
        _ => AppError::from(e),
    }
}

/// This event's real listings across every ticket/marketplace, newest-
/// updated first - the one query the Event Workspace "Listings" tab needs.
/// Scoped by `t.event_id` directly (a single query), rather than the
/// N-calls-per-order pattern `EventDetail.tsx`'s own Finance tab uses (see
/// PROJECT_STATE/PROTECTED_AREAS.md's "2.2.2" entry on why that pattern is
/// fine at order scale but shouldn't be copied) - a listings-heavy event can
/// realistically have far more listings than an event has orders, so this
/// gets its own real event-scoped query from the start instead.
pub(crate) fn list_ticket_listings_for_event_impl(conn: &Connection, event_id: i64) -> AppResult<Vec<TicketListing>> {
    let sql = format!("{TICKET_LISTING_SELECT} WHERE t.event_id = ?1 ORDER BY tl.updated_at DESC, tl.id DESC");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([event_id], map_ticket_listing)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[tauri::command]
pub fn list_ticket_listings_for_event(state: State<AppState>, event_id: i64) -> AppResult<Vec<TicketListing>> {
    let conn = state.db.lock().unwrap();
    list_ticket_listings_for_event_impl(&conn, event_id)
}

/// Core logic behind `create_ticket_listing` - same impl+wrapper split used
/// throughout this codebase.
pub(crate) fn create_ticket_listing_impl(conn: &Connection, input: &TicketListingInput) -> AppResult<TicketListing> {
    validate_listing_fields(input)?;
    validate_ticket_exists(conn, input.ticket_id)?;
    validate_marketplace_exists(conn, input.marketplace_id)?;
    let currency = input.currency.trim().to_ascii_uppercase();
    let listing_id = normalize_optional(input.listing_id.clone());
    let listing_url = normalize_optional(input.listing_url.clone());
    conn.execute(
        "INSERT INTO ticket_listings(ticket_id, marketplace_id, listing_id, listing_url, price_cents, currency, status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            input.ticket_id,
            input.marketplace_id,
            listing_id,
            listing_url,
            input.price_cents,
            currency,
            input.status
        ],
    )
    .map_err(map_write_error)?;
    let id = conn.last_insert_rowid();
    let sql = format!("{TICKET_LISTING_SELECT} WHERE tl.id = ?1");
    Ok(conn.query_row(&sql, [id], map_ticket_listing)?)
}

#[tauri::command]
pub fn create_ticket_listing(state: State<AppState>, input: TicketListingInput) -> AppResult<TicketListing> {
    let conn = state.db.lock().unwrap();
    create_ticket_listing_impl(&conn, &input)
}

/// Core logic behind `update_ticket_listing` - full-row update (every
/// editable field at once), same shape as `finance_entries::
/// update_finance_entry_impl`. Returns `AppError::NotFound` for a missing id
/// rather than silently doing nothing, same convention as every other
/// `update_x_impl` in this codebase.
pub(crate) fn update_ticket_listing_impl(conn: &Connection, id: i64, input: &TicketListingInput) -> AppResult<TicketListing> {
    validate_listing_fields(input)?;
    validate_ticket_exists(conn, input.ticket_id)?;
    validate_marketplace_exists(conn, input.marketplace_id)?;
    let currency = input.currency.trim().to_ascii_uppercase();
    let listing_id = normalize_optional(input.listing_id.clone());
    let listing_url = normalize_optional(input.listing_url.clone());
    let updated = conn
        .execute(
            "UPDATE ticket_listings SET ticket_id = ?1, marketplace_id = ?2, listing_id = ?3, listing_url = ?4,
                 price_cents = ?5, currency = ?6, status = ?7, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
             WHERE id = ?8",
            rusqlite::params![
                input.ticket_id,
                input.marketplace_id,
                listing_id,
                listing_url,
                input.price_cents,
                currency,
                input.status,
                id
            ],
        )
        .map_err(map_write_error)?;
    if updated == 0 {
        return Err(AppError::NotFound(format!("Listing #{id} not found")));
    }
    let sql = format!("{TICKET_LISTING_SELECT} WHERE tl.id = ?1");
    Ok(conn.query_row(&sql, [id], map_ticket_listing)?)
}

#[tauri::command]
pub fn update_ticket_listing(state: State<AppState>, id: i64, input: TicketListingInput) -> AppResult<TicketListing> {
    let conn = state.db.lock().unwrap();
    update_ticket_listing_impl(&conn, id, &input)
}

/// Plain blind delete, same as `finance_entries::delete_finance_entry`/
/// `price_checker`'s own disposable-row deletes - a listing is re-enterable
/// reference data (marko's own price/status snapshot of a marketplace
/// posting), never protected financial history like a Sale. Given its own
/// `_impl` split (unlike `delete_finance_entry`, which has none) purely so
/// the delete path is directly unit-testable, same as marko's own explicit
/// "delete listingu" test requirement this release.
pub(crate) fn delete_ticket_listing_impl(conn: &Connection, id: i64) -> AppResult<()> {
    conn.execute("DELETE FROM ticket_listings WHERE id = ?1", [id])?;
    Ok(())
}

#[tauri::command]
pub fn delete_ticket_listing(state: State<AppState>, id: i64) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    delete_ticket_listing_impl(&conn, id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_conn;

    // Same minimal event/order/ticket seeding shape as
    // `price_checker::tests::seed_event`/`seed_order_only`/`seed_ticket` -
    // kept as its own local copy since those are private to that module's
    // own `mod tests`.

    fn seed_event(conn: &Connection, name: &str) -> i64 {
        conn.execute("INSERT INTO events (name) VALUES (?1)", [name]).unwrap();
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
            "INSERT INTO tickets (code, event_id, order_id, section, row_label, seat, currency, status)
             VALUES (?1, ?2, ?3, 'A', '1', '12', 'EUR', 'available')",
            rusqlite::params![format!("TKT-{code_suffix}"), event_id, order_id],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    fn seed_marketplace(conn: &Connection, name: &str) -> i64 {
        conn.execute("INSERT INTO marketplaces(name) VALUES (?1)", [name]).unwrap();
        conn.last_insert_rowid()
    }

    fn sample_input(ticket_id: i64, marketplace_id: i64) -> TicketListingInput {
        TicketListingInput {
            ticket_id,
            marketplace_id,
            listing_id: Some("ext-123".to_string()),
            listing_url: Some("https://example.com/listing/123".to_string()),
            price_cents: 42000,
            currency: "eur".to_string(),
            status: "active".to_string(),
        }
    }

    #[test]
    fn one_ticket_one_listing_round_trips_all_fields() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event");
        let ticket_id = seed_ticket(&conn, "1", event_id);
        let marketplace_id = seed_marketplace(&conn, "StubHub Clone");

        let created = create_ticket_listing_impl(&conn, &sample_input(ticket_id, marketplace_id)).unwrap();

        assert_eq!(created.ticket_id, ticket_id);
        assert_eq!(created.ticket_code, "TKT-1");
        assert_eq!(created.marketplace_id, marketplace_id);
        assert_eq!(created.marketplace_name, "StubHub Clone");
        assert_eq!(created.listing_id.as_deref(), Some("ext-123"));
        assert_eq!(created.listing_url.as_deref(), Some("https://example.com/listing/123"));
        assert_eq!(created.price_cents, 42000);
        assert_eq!(created.currency, "EUR", "currency must be trimmed+uppercased, same as finance_entries");
        assert_eq!(created.status, "active");
        assert!(!created.is_demo);
    }

    #[test]
    fn one_ticket_can_have_listings_on_several_marketplaces() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event");
        let ticket_id = seed_ticket(&conn, "1", event_id);
        let vivid = seed_marketplace(&conn, "Vivid Test");
        let ticombo = seed_marketplace(&conn, "Ticombo Test");

        let mut on_vivid = sample_input(ticket_id, vivid);
        on_vivid.listing_id = Some("vivid-1".into());
        on_vivid.price_cents = 43500;
        create_ticket_listing_impl(&conn, &on_vivid).unwrap();

        let mut on_ticombo = sample_input(ticket_id, ticombo);
        on_ticombo.listing_id = Some("ticombo-1".into());
        on_ticombo.price_cents = 44900;
        create_ticket_listing_impl(&conn, &on_ticombo).unwrap();

        let listings = list_ticket_listings_for_event_impl(&conn, event_id).unwrap();
        assert_eq!(listings.len(), 2, "one ticket, two marketplaces, must both show up");
        assert!(listings.iter().all(|l| l.ticket_id == ticket_id));
        let names: std::collections::HashSet<_> = listings.iter().map(|l| l.marketplace_name.as_str()).collect();
        assert!(names.contains("Vivid Test") && names.contains("Ticombo Test"));
    }

    #[test]
    fn updating_an_existing_listing_persists_the_changes() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event");
        let ticket_id = seed_ticket(&conn, "1", event_id);
        let marketplace_id = seed_marketplace(&conn, "Test Market");
        let created = create_ticket_listing_impl(&conn, &sample_input(ticket_id, marketplace_id)).unwrap();

        let mut update = sample_input(ticket_id, marketplace_id);
        update.price_cents = 39900;
        update.status = "sold".to_string();
        update.listing_url = Some("https://example.com/listing/updated".to_string());
        let updated = update_ticket_listing_impl(&conn, created.id, &update).unwrap();

        assert_eq!(updated.id, created.id);
        assert_eq!(updated.price_cents, 39900);
        assert_eq!(updated.status, "sold");
        assert_eq!(updated.listing_url.as_deref(), Some("https://example.com/listing/updated"));
        // Not asserting updated_at != created_at here: strftime's millisecond
        // precision means a create immediately followed by an update in the
        // same test can legitimately land in the same millisecond - same
        // observation already documented in finance_entries.rs's own
        // equivalent test. The column is still written by the same
        // `strftime('%Y-%m-%dT%H:%M:%fZ','now')` expression every other
        // `updated_at` column in this app uses; this test's job is the field
        // values, not clock resolution.
    }

    #[test]
    fn updating_a_missing_listing_returns_not_found() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event");
        let ticket_id = seed_ticket(&conn, "1", event_id);
        let marketplace_id = seed_marketplace(&conn, "Test Market");

        let err = update_ticket_listing_impl(&conn, 999_999, &sample_input(ticket_id, marketplace_id)).unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[test]
    fn deleting_a_listing_removes_it() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event");
        let ticket_id = seed_ticket(&conn, "1", event_id);
        let marketplace_id = seed_marketplace(&conn, "Test Market");
        let created = create_ticket_listing_impl(&conn, &sample_input(ticket_id, marketplace_id)).unwrap();

        delete_ticket_listing_impl(&conn, created.id).unwrap();

        let listings = list_ticket_listings_for_event_impl(&conn, event_id).unwrap();
        assert!(listings.is_empty(), "deleted listing must not be returned anymore");
    }

    #[test]
    fn creating_a_duplicate_ticket_marketplace_listing_id_is_rejected() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event");
        let ticket_id = seed_ticket(&conn, "1", event_id);
        let marketplace_id = seed_marketplace(&conn, "Test Market");
        create_ticket_listing_impl(&conn, &sample_input(ticket_id, marketplace_id)).unwrap();

        let err = create_ticket_listing_impl(&conn, &sample_input(ticket_id, marketplace_id)).unwrap_err();

        assert!(matches!(err, AppError::Validation(_)), "an exact duplicate (same ticket+marketplace+listing id) must be rejected, not panic");
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM ticket_listings", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1, "the rejected duplicate must not have been inserted");
    }

    #[test]
    fn two_listings_with_no_listing_id_yet_do_not_collide() {
        // Marko will often enter these by hand with no external id at all -
        // the migration's own doc comment explains why NULL listing_id
        // values must NOT count as duplicates of each other.
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event");
        let ticket_a = seed_ticket(&conn, "1", event_id);
        let ticket_b = seed_ticket(&conn, "2", event_id);
        let marketplace_id = seed_marketplace(&conn, "Test Market");

        let mut input_a = sample_input(ticket_a, marketplace_id);
        input_a.listing_id = None;
        let mut input_b = sample_input(ticket_b, marketplace_id);
        input_b.listing_id = None;

        create_ticket_listing_impl(&conn, &input_a).unwrap();
        create_ticket_listing_impl(&conn, &input_b).unwrap();

        let listings = list_ticket_listings_for_event_impl(&conn, event_id).unwrap();
        assert_eq!(listings.len(), 2, "two different tickets with no listing id must both be kept, not treated as duplicates");
    }

    #[test]
    fn several_listings_for_an_event_list_with_no_duplicates() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event");
        let ticket_a = seed_ticket(&conn, "1", event_id);
        let ticket_b = seed_ticket(&conn, "2", event_id);
        let vivid = seed_marketplace(&conn, "Vivid Test");
        let ticombo = seed_marketplace(&conn, "Ticombo Test");

        let mut l1 = sample_input(ticket_a, vivid);
        l1.listing_id = Some("l1".into());
        let mut l2 = sample_input(ticket_a, ticombo);
        l2.listing_id = Some("l2".into());
        let mut l3 = sample_input(ticket_b, vivid);
        l3.listing_id = Some("l3".into());
        for input in [&l1, &l2, &l3] {
            create_ticket_listing_impl(&conn, input).unwrap();
        }

        let listings = list_ticket_listings_for_event_impl(&conn, event_id).unwrap();
        assert_eq!(listings.len(), 3, "the ticket/marketplace JOIN must never fan a single row out into more than one");
        let ids: std::collections::HashSet<_> = listings.iter().map(|l| l.id).collect();
        assert_eq!(ids.len(), 3, "every listing id in the result must be distinct");
    }

    #[test]
    fn mixed_currency_listings_for_the_same_ticket_keep_their_own_currency() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event");
        let ticket_id = seed_ticket(&conn, "1", event_id);
        let eur_market = seed_marketplace(&conn, "EUR Market");
        let usd_market = seed_marketplace(&conn, "USD Market");

        let mut eur_listing = sample_input(ticket_id, eur_market);
        eur_listing.listing_id = Some("eur-1".into());
        eur_listing.currency = "eur".to_string();
        eur_listing.price_cents = 40000;
        let mut usd_listing = sample_input(ticket_id, usd_market);
        usd_listing.listing_id = Some("usd-1".into());
        usd_listing.currency = "usd".to_string();
        usd_listing.price_cents = 45000;

        create_ticket_listing_impl(&conn, &eur_listing).unwrap();
        create_ticket_listing_impl(&conn, &usd_listing).unwrap();

        let listings = list_ticket_listings_for_event_impl(&conn, event_id).unwrap();
        assert_eq!(listings.len(), 2);
        let eur = listings.iter().find(|l| l.marketplace_name == "EUR Market").unwrap();
        let usd = listings.iter().find(|l| l.marketplace_name == "USD Market").unwrap();
        assert_eq!(eur.currency, "EUR");
        assert_eq!(eur.price_cents, 40000);
        assert_eq!(usd.currency, "USD");
        assert_eq!(usd.price_cents, 45000, "no conversion/blending must ever happen at this layer - each listing keeps its own currency and cents value untouched");
    }

    #[test]
    fn existing_ticket_data_is_unaffected_by_the_new_migration() {
        // test_conn() runs every migration up to and including 022 - this
        // proves the new, additive table doesn't disturb a ticket created
        // the exact same way every earlier release's own tests already do,
        // and that it starts genuinely empty until a listing is explicitly
        // added.
        let conn = test_conn();
        let event_id = seed_event(&conn, "Pre-existing Event");
        let ticket_id = seed_ticket(&conn, "1", event_id);

        let ticket_status: String = conn.query_row("SELECT status FROM tickets WHERE id = ?1", [ticket_id], |r| r.get(0)).unwrap();
        assert_eq!(ticket_status, "available", "ticket's own columns/defaults must be completely unaffected by 022");

        let listings = list_ticket_listings_for_event_impl(&conn, event_id).unwrap();
        assert!(listings.is_empty(), "a freshly-migrated database must start with zero listings, not fabricated ones");
    }

    #[test]
    fn invalid_status_is_rejected() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event");
        let ticket_id = seed_ticket(&conn, "1", event_id);
        let marketplace_id = seed_marketplace(&conn, "Test Market");
        let mut input = sample_input(ticket_id, marketplace_id);
        input.status = "pending".to_string();

        let err = create_ticket_listing_impl(&conn, &input).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn unknown_ticket_id_is_rejected() {
        let conn = test_conn();
        let marketplace_id = seed_marketplace(&conn, "Test Market");
        let input = sample_input(999_999, marketplace_id);

        let err = create_ticket_listing_impl(&conn, &input).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn unknown_marketplace_id_is_rejected() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event");
        let ticket_id = seed_ticket(&conn, "1", event_id);
        let input = sample_input(ticket_id, 999_999);

        let err = create_ticket_listing_impl(&conn, &input).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }
}

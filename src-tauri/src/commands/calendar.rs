//! 2.5.0: "TIQR Operations Calendar" - marko's own request for one Month/Week
//! calendar that pulls together every part of the app that has a real date,
//! instead of five separate places to go check what's happening when.
//!
//! ## Research first - which of marko's 8 candidate categories are real
//!
//! marko named 8 candidate event types up front and was explicit that none
//! of them should be invented if the underlying data doesn't actually exist
//! ("Ak nejaký dátum v databáze neexistuje, NEVYMÝŠĽAJ ho"). Before writing
//! any code, every one of the 8 was checked against the real schema and the
//! real, currently-shipping application code (not just column names):
//!
//! - **EVENTS** - real. `events.event_date`.
//! - **SALES** - real. `sales.sale_date`.
//! - **ORDERS/PURCHASES** - real. `orders.purchase_date`.
//! - **PULLS** - real, but NOT `pulls.transfer_deadline` - that column is a
//!   1.9.7 leftover nothing has written since 1.9.8 (see its own doc comment
//!   in models.rs). The live equivalent is `pulls.event_date`, exactly the
//!   column Pulls.tsx's own client-side warning already keys off.
//! - **ATTENTION** - real. `AttentionCenterItem.event_date` is populated
//!   across every one of its 5 categories.
//! - **PAYOUTS** - NOT real. There is no distinct "payout" entity, table, or
//!   date anywhere in this app - "Payout" only exists as Google Sheets
//!   column-header text (`orders_sheet_sync.rs`'s "Payout Per Ticket"/
//!   "Payout status"), aliasing `Sale.sale_price_cents`/`Sale.payment_status`
//!   1:1. Inventing a "payout date" would mean making one up.
//! - **PAYMENTS** - NOT real. Migration 007's `payments` table exists in the
//!   schema (readable, migrated) but has zero live Rust command code that
//!   reads or writes it anywhere - the `payments.rs` its own migration
//!   comment references was never built. Exactly the same
//!   schema-present-but-functionally-dead shape as migration 026's
//!   `price_checker_monitor` tables (see PROTECTED_AREAS.md).
//! - **FULFILLMENT** - NOT real. `tickets.delivery_status` is a plain
//!   free-text enum ("Delivered"/"Not delivered"/etc.) with no associated
//!   date column anywhere - no `delivery_date`/`delivered_at`/
//!   `delivery_deadline` exists.
//!
//! So `get_calendar` only ever returns 5 kinds: "event", "order", "sale",
//! "pull", "attention" - see `CalendarEntry::kind`'s own doc comment
//! (models.rs). This is stated again in the 2.5.0 release report per
//! marko's own explicit requirement, not just left implicit here.
//!
//! ## Read-only aggregation, not a new parallel system
//!
//! Same "each view aggregator writes its own SELECT" convention already
//! used by `attention_center`/`inventory_intelligence`/`ticket_control_
//! center` - this module owns exactly one read-only command
//! (`get_calendar`) and never writes anything. It deliberately does NOT call
//! into `orders::list_orders_impl`/`sales::list_sale_groups_impl`/etc. and
//! reshape their results (those return far more than a calendar card needs,
//! and `ticket_control_center`'s own doc comment already explains why a
//! second, differently-shaped read belongs in its own query). The one
//! exception is `attention_center::get_attention_center_impl` - reused
//! directly, unmodified, rather than re-implemented, both for the
//! "attention" entries themselves AND (see `soon_event_ids` below) to decide
//! an "event" entry's own severity without duplicating that threshold logic
//! a second time.
//!
//! No new table, no new migration - every one of the 5 kinds reads columns
//! that already exist and are already indexed for exactly this kind of
//! range query (`idx_events_date`, `idx_orders_date`, `idx_sales_date` -
//! migration 001). `pulls.event_date` has no index of its own, but neither
//! does `pulls::list_pulls_impl`'s own, already-shipping `date_from`/
//! `date_to` filter - adding one now would be an index added without a new
//! reason, which marko was explicit not to do.

use crate::commands::attention_center::get_attention_center_impl;
use crate::commands::sales::GROUP_KEY_EXPR;
use crate::db::AppState;
use crate::error::AppResult;
use crate::models::{AttentionCenterItem, CalendarEntry, CalendarFilters};
use chrono::{Local, NaiveDate};
use rusqlite::{params, Connection};
use std::collections::HashSet;
use tauri::State;

/// Mirrors Pulls.tsx's own `WARNING_WINDOW_DAYS` exactly - kept
/// independently here in Rust rather than imported across a page-file
/// boundary, the same "same rule, not the same code" precedent this
/// codebase already uses for `attention_center::event_is_done` mirroring
/// Orders.tsx's `isOrderDone`. A pull's calendar severity must never
/// disagree with the warning Pulls.tsx itself already shows for that exact
/// same pull.
const PULL_WARNING_WINDOW_DAYS: i64 = 3;

fn events_in_range(conn: &Connection, date_from: &str, date_to: &str, soon_event_ids: &HashSet<i64>) -> AppResult<Vec<CalendarEntry>> {
    let mut stmt = conn.prepare("SELECT id, name, event_date, venue, city FROM events WHERE event_date >= ?1 AND event_date <= ?2")?;
    let rows = stmt.query_map(params![date_from, date_to], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?, // NOT NULL for any row that matched the WHERE clause above
            r.get::<_, Option<String>>(3)?,
            r.get::<_, Option<String>>(4)?,
        ))
    })?;

    let mut entries = Vec::new();
    for row in rows {
        let (id, name, event_date, venue, city) = row?;
        let venue = venue.filter(|s| !s.is_empty());
        let city = city.filter(|s| !s.is_empty());
        let subtitle = match (venue, city) {
            (Some(v), Some(c)) => Some(format!("{v}, {c}")),
            (Some(v), None) => Some(v),
            (None, Some(c)) => Some(c),
            (None, None) => None,
        };
        entries.push(CalendarEntry {
            key: format!("event:{id}"),
            kind: "event".to_string(),
            date: event_date,
            title: name,
            subtitle,
            // Reuses attention_center's OWN "event_soon" verdict (see
            // get_calendar_impl below) rather than re-checking
            // EVENT_SOON_DAYS a second time - one threshold, one place.
            severity: if soon_event_ids.contains(&id) { "critical" } else { "neutral" }.to_string(),
            link_kind: "event".to_string(),
            link_id: Some(id),
            amount_cents: None,
            currency: None,
        });
    }
    Ok(entries)
}

fn orders_in_range(conn: &Connection, date_from: &str, date_to: &str) -> AppResult<Vec<CalendarEntry>> {
    let mut stmt = conn.prepare(
        "SELECT o.id, o.code, e.name, o.purchase_date, o.quantity, o.total_cost_cents, o.currency \
         FROM orders o JOIN events e ON e.id = o.event_id \
         WHERE o.purchase_date >= ?1 AND o.purchase_date <= ?2",
    )?;
    let rows = stmt.query_map(params![date_from, date_to], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, i64>(4)?,
            r.get::<_, i64>(5)?,
            r.get::<_, String>(6)?,
        ))
    })?;

    let mut entries = Vec::new();
    for row in rows {
        let (id, code, event_name, purchase_date, quantity, total_cost_cents, currency) = row?;
        entries.push(CalendarEntry {
            key: format!("order:{id}"),
            kind: "order".to_string(),
            date: purchase_date,
            title: format!("Order {code}"),
            subtitle: Some(format!("{quantity} ticket{} · {event_name}", if quantity == 1 { "" } else { "s" })),
            severity: "neutral".to_string(),
            link_kind: "order".to_string(),
            link_id: Some(id),
            // An order is always a single, real currency (never a mixed
            // batch like a sale group can be) - safe to show directly.
            amount_cents: Some(total_cost_cents),
            currency: Some(currency),
        });
    }
    Ok(entries)
}

/// One entry per sale ACTION (a single ticket, or a multi-ticket batch) -
/// never one row per ticket. Reuses `sales::GROUP_KEY_EXPR` (the exact same
/// `COALESCE(batch_id, 'single:' || id)` grouping the main Sales list and
/// Dashboard's own "Recent sales" card already use) rather than writing a
/// second, possibly-inconsistent way to decide what counts as "one sale".
///
/// `amount_cents`/`currency` are a deliberately SIMPLER variant of
/// `SaleGroup`'s own revenue/currency logic: `None` whenever the group's
/// lines don't all share one `sales.currency` - same never-blend-currencies
/// rule, just without SaleGroup's extra profit/margin/ROI machinery, which a
/// calendar card has no use for (this view shows what a batch sold FOR, not
/// what it profited).
fn sales_in_range(conn: &Connection, date_from: &str, date_to: &str) -> AppResult<Vec<CalendarEntry>> {
    let sql = format!(
        "SELECT MIN(s.id) as id, MIN(s.code) as code, MAX(s.sale_date) as sale_date, COUNT(*) as ticket_count, \
           CASE WHEN COUNT(DISTINCT t.event_id) = 1 THEN MAX(e.name) END as event_name, \
           CASE WHEN COUNT(DISTINCT s.currency) = 1 THEN MAX(s.currency) END as currency, \
           CASE WHEN COUNT(DISTINCT s.currency) = 1 \
             THEN SUM(CASE WHEN s.payment_status != 'refunded' THEN s.sale_price_cents ELSE 0 END) END as amount_cents \
         FROM sales s \
         JOIN tickets t ON t.id = s.ticket_id \
         JOIN events e ON e.id = t.event_id \
         WHERE s.sale_date >= ?1 AND s.sale_date <= ?2 \
         GROUP BY {GROUP_KEY_EXPR}"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![date_from, date_to], |r| {
        Ok((
            r.get::<_, i64>("id")?,
            r.get::<_, String>("code")?,
            r.get::<_, String>("sale_date")?,
            r.get::<_, i64>("ticket_count")?,
            r.get::<_, Option<String>>("event_name")?,
            r.get::<_, Option<String>>("currency")?,
            r.get::<_, Option<i64>>("amount_cents")?,
        ))
    })?;

    let mut entries = Vec::new();
    for row in rows {
        let (id, code, sale_date, ticket_count, event_name, currency, amount_cents) = row?;
        let event_part = event_name.unwrap_or_else(|| "Mixed events".to_string());
        entries.push(CalendarEntry {
            key: format!("sale:{id}"),
            kind: "sale".to_string(),
            date: sale_date,
            title: format!("Sale {code}"),
            subtitle: Some(format!("{ticket_count} ticket{} · {event_part}", if ticket_count == 1 { "" } else { "s" })),
            severity: "neutral".to_string(),
            link_kind: "sale".to_string(),
            // The group's own representative id - MIN(s.id), same "anchor"
            // convention SaleDetail.tsx's own route/self-healing redirect
            // already relies on. `/sales/{id}` is the exact route every
            // other sale-group link in this app already uses (Sales.tsx,
            // EventDetail.tsx, FulfillmentCenter.tsx, TicketControlCenter.tsx).
            link_id: Some(id),
            amount_cents,
            currency,
        });
    }
    Ok(entries)
}

/// Pulls has no per-record detail route today (unlike Event/Order/Sale) -
/// `PullEditModal` opens straight off the Pulls list, there is no
/// `/pulls/:id`. So every pull entry links to the LIST page rather than a
/// route that doesn't exist - see `CalendarEntry::link_kind`'s own doc
/// comment.
fn pulls_in_range(conn: &Connection, date_from: &str, date_to: &str, today: NaiveDate) -> AppResult<Vec<CalendarEntry>> {
    let mut stmt = conn.prepare(
        "SELECT id, code, buyer_name, event_name, event_date, quantity, price_cents, currency, transfer_done \
         FROM pulls WHERE event_date >= ?1 AND event_date <= ?2",
    )?;
    let rows = stmt.query_map(params![date_from, date_to], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, String>(4)?,
            r.get::<_, i64>(5)?,
            r.get::<_, i64>(6)?,
            r.get::<_, String>(7)?,
            r.get::<_, bool>(8)?,
        ))
    })?;

    let mut entries = Vec::new();
    for row in rows {
        let (id, code, buyer_name, event_name, event_date, quantity, price_cents, currency, transfer_done) = row?;
        let days_left = NaiveDate::parse_from_str(&event_date, "%Y-%m-%d").ok().map(|d| (d - today).num_days());
        let severity = if transfer_done {
            "neutral"
        } else {
            match days_left {
                Some(d) if d <= 0 => "critical",
                Some(d) if d <= PULL_WARNING_WINDOW_DAYS => "attention",
                _ => "neutral",
            }
        };
        entries.push(CalendarEntry {
            key: format!("pull:{id}"),
            kind: "pull".to_string(),
            date: event_date,
            title: format!("Pull {code}"),
            subtitle: Some(format!("{buyer_name} · {quantity} ticket{} · {event_name}", if quantity == 1 { "" } else { "s" })),
            severity: severity.to_string(),
            link_kind: "pulls".to_string(),
            link_id: None,
            amount_cents: Some(price_cents),
            currency: Some(currency),
        });
    }
    Ok(entries)
}

/// Turns already-computed `AttentionCenterItem`s into calendar entries -
/// pure/in-memory, no DB access of its own (the query already happened in
/// `get_attention_center_impl`). Filters to the requested range here, since
/// `get_attention_center_impl` itself is deliberately global/unbounded (same
/// "already cheap, already called unbounded on every Dashboard load"
/// reasoning as its own module doc comment).
///
/// Navigation mirrors the exact same `order_id.is_some() ? order : event`
/// rule every other consumer of this same struct already uses (e.g.
/// TicketControlCenter.tsx's own click handler) - never a third way to
/// decide where an attention row goes.
fn attention_in_range(items: &[AttentionCenterItem], date_from: &str, date_to: &str) -> Vec<CalendarEntry> {
    items
        .iter()
        .filter_map(|item| {
            let date = item.event_date.clone()?;
            if date.as_str() < date_from || date.as_str() > date_to {
                return None;
            }
            let subtitle = Some(match &item.order_code {
                Some(code) => format!("{} · Order {code}", item.event_name),
                None => item.event_name.clone(),
            });
            let (link_kind, link_id): (&str, Option<i64>) = match item.order_id {
                Some(oid) => ("order", Some(oid)),
                None => ("event", Some(item.event_id)),
            };
            Some(CalendarEntry {
                key: format!("attention:{}", item.key),
                kind: "attention".to_string(),
                date,
                title: item.reason.clone(),
                subtitle,
                // AttentionCenterItem.priority is already "critical" |
                // "attention" | "info" - the exact same 3 values (plus
                // "neutral", which attention items never use) this struct's
                // own severity field uses, so this is a straight passthrough.
                severity: item.priority.clone(),
                link_kind: link_kind.to_string(),
                link_id,
                amount_cents: item.amount_cents,
                currency: item.currency.clone(),
            })
        })
        .collect()
}

/// Split out from the `get_calendar` command (same `_impl`/thin-wrapper
/// split as every other command in this codebase) so it's directly
/// unit-testable against a plain `&Connection` with a pinned `today`.
pub(crate) fn get_calendar_impl(conn: &Connection, filters: &CalendarFilters, today: NaiveDate) -> AppResult<Vec<CalendarEntry>> {
    // Called ONCE, reused twice: for the "attention" entries themselves,
    // and (via `soon_event_ids`) to decide "event" entries' own severity -
    // see `events_in_range`'s own comment. Never re-derives EVENT_SOON_DAYS
    // a second time.
    let attention_items = get_attention_center_impl(conn, today)?;
    let soon_event_ids: HashSet<i64> = attention_items.iter().filter(|i| i.category == "event_soon").map(|i| i.event_id).collect();

    let mut entries = Vec::new();
    entries.extend(events_in_range(conn, &filters.date_from, &filters.date_to, &soon_event_ids)?);
    entries.extend(orders_in_range(conn, &filters.date_from, &filters.date_to)?);
    entries.extend(sales_in_range(conn, &filters.date_from, &filters.date_to)?);
    entries.extend(pulls_in_range(conn, &filters.date_from, &filters.date_to, today)?);
    entries.extend(attention_in_range(&attention_items, &filters.date_from, &filters.date_to));

    // Deterministic order: date first (so the grid can just walk the list
    // top to bottom), then kind/key as stable tie-breakers. Day Detail and
    // any "critical first" grouping are display concerns, decided
    // client-side - this is only ever a baseline order, never the final one.
    entries.sort_by(|a, b| a.date.cmp(&b.date).then_with(|| a.kind.cmp(&b.kind)).then_with(|| a.key.cmp(&b.key)));
    Ok(entries)
}

#[tauri::command]
pub fn get_calendar(state: State<AppState>, filters: CalendarFilters) -> AppResult<Vec<CalendarEntry>> {
    let conn = state.db.lock().unwrap();
    get_calendar_impl(&conn, &filters, Local::now().date_naive())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::sales::{create_sale_impl, create_sales_batch_impl};
    use crate::db::test_conn;
    use crate::models::{SaleBatchInput, SaleBatchLineInput, SaleInput};

    static NEXT_CODE: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);
    fn next_code(prefix: &str) -> String {
        format!("{prefix}-{}", NEXT_CODE.fetch_add(1, std::sync::atomic::Ordering::SeqCst))
    }

    fn seed_event(conn: &Connection, name: &str, event_date: Option<&str>) -> i64 {
        conn.execute("INSERT INTO events (name, event_date) VALUES (?1, ?2)", params![name, event_date]).unwrap();
        conn.last_insert_rowid()
    }

    fn seed_order(conn: &Connection, event_id: i64, purchase_date: &str, currency: &str, total_cost_cents: i64, quantity: i64) -> i64 {
        conn.execute(
            "INSERT INTO orders (code, event_id, purchase_date, quantity, total_cost_cents, currency) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![next_code("ORD"), event_id, purchase_date, quantity, total_cost_cents, currency],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    fn seed_ticket(conn: &Connection, event_id: i64, order_id: i64, currency: &str) -> i64 {
        conn.execute(
            "INSERT INTO tickets (code, event_id, order_id, purchase_cost_cents, currency, status) VALUES (?1, ?2, ?3, 1000, ?4, 'available')",
            params![next_code("TKT"), event_id, order_id, currency],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    fn seed_pull(conn: &Connection, buyer_name: &str, event_name: &str, event_date: &str, price_cents: i64, currency: &str, transfer_done: bool) -> i64 {
        conn.execute(
            "INSERT INTO pulls (code, buyer_name, event_name, event_date, quantity, price_cents, currency, transfer_done) \
             VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6, ?7)",
            params![next_code("PULL"), buyer_name, event_name, event_date, price_cents, currency, transfer_done],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn events_without_a_date_never_appear_on_the_calendar() {
        let conn = test_conn();
        seed_event(&conn, "No Date Event", None);
        let today = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let filters = CalendarFilters { date_from: "2000-01-01".into(), date_to: "2100-01-01".into() };
        let entries = get_calendar_impl(&conn, &filters, today).unwrap();
        assert!(entries.iter().all(|e| e.kind != "event"), "an event with no date must never be invented a place on the calendar");
    }

    #[test]
    fn event_entry_is_critical_exactly_when_attention_centers_event_soon_flags_it() {
        let conn = test_conn();
        let today = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let soon_event = seed_event(&conn, "Soon Event", Some("2026-06-02")); // 1 day out
        let soon_order = seed_order(&conn, soon_event, "2026-01-01", "EUR", 1000, 1);
        seed_ticket(&conn, soon_event, soon_order, "EUR"); // available+unsold -> triggers event_soon
        let far_event = seed_event(&conn, "Far Event", Some("2026-06-20"));

        let filters = CalendarFilters { date_from: "2026-06-01".into(), date_to: "2026-06-30".into() };
        let entries = get_calendar_impl(&conn, &filters, today).unwrap();
        let soon_entry = entries.iter().find(|e| e.kind == "event" && e.link_id == Some(soon_event)).unwrap();
        let far_entry = entries.iter().find(|e| e.kind == "event" && e.link_id == Some(far_event)).unwrap();
        assert_eq!(soon_entry.severity, "critical");
        assert_eq!(far_entry.severity, "neutral");
    }

    #[test]
    fn order_entries_carry_their_own_cost_and_currency() {
        let conn = test_conn();
        let today = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let event_id = seed_event(&conn, "Test Event", Some("2026-06-01"));
        let order_id = seed_order(&conn, event_id, "2026-01-15", "USD", 12_345, 3);

        let filters = CalendarFilters { date_from: "2026-01-01".into(), date_to: "2026-01-31".into() };
        let entries = get_calendar_impl(&conn, &filters, today).unwrap();
        let entry = entries.iter().find(|e| e.kind == "order").expect("expected an order entry");
        assert_eq!(entry.date, "2026-01-15");
        assert_eq!(entry.link_kind, "order");
        assert_eq!(entry.link_id, Some(order_id));
        assert_eq!(entry.amount_cents, Some(12_345));
        assert_eq!(entry.currency.as_deref(), Some("USD"));
        assert!(entry.title.starts_with("Order "));
        assert!(entry.subtitle.as_deref().unwrap().contains("3 tickets"));
    }

    #[test]
    fn sale_batch_is_shown_as_one_grouped_entry_not_one_per_ticket() {
        let mut conn = test_conn();
        let today = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let event_id = seed_event(&conn, "Test Event", Some("2026-06-01"));
        let order_id = seed_order(&conn, event_id, "2026-01-01", "EUR", 1000, 3);
        let t1 = seed_ticket(&conn, event_id, order_id, "EUR");
        let t2 = seed_ticket(&conn, event_id, order_id, "EUR");
        let t3 = seed_ticket(&conn, event_id, order_id, "EUR");
        let batch = SaleBatchInput {
            lines: vec![
                SaleBatchLineInput { ticket_id: t1, sale_price_cents: 1000, selling_fees_cents: 0 },
                SaleBatchLineInput { ticket_id: t2, sale_price_cents: 1000, selling_fees_cents: 0 },
                SaleBatchLineInput { ticket_id: t3, sale_price_cents: 1000, selling_fees_cents: 0 },
            ],
            platform_id: None,
            sale_date: "2026-01-10".to_string(),
            payment_status: Some("paid".to_string()),
            buyer_reference: None,
            notes: None,
            currency: None,
        };
        create_sales_batch_impl(&mut conn, &batch).unwrap();

        let filters = CalendarFilters { date_from: "2026-01-01".into(), date_to: "2026-01-31".into() };
        let entries = get_calendar_impl(&conn, &filters, today).unwrap();
        let sale_entries: Vec<&CalendarEntry> = entries.iter().filter(|e| e.kind == "sale").collect();
        assert_eq!(sale_entries.len(), 1, "one batch of 3 tickets must be exactly one calendar entry, not 3");
        assert!(sale_entries[0].subtitle.as_deref().unwrap().contains("3 tickets"));
        assert_eq!(sale_entries[0].amount_cents, Some(3000));
        assert_eq!(sale_entries[0].currency.as_deref(), Some("EUR"));
    }

    #[test]
    fn sale_batch_spanning_two_events_reports_mixed_events() {
        let mut conn = test_conn();
        let today = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let event_a = seed_event(&conn, "Event A", Some("2026-06-01"));
        let event_b = seed_event(&conn, "Event B", Some("2026-07-01"));
        let order_a = seed_order(&conn, event_a, "2026-01-01", "EUR", 1000, 1);
        let order_b = seed_order(&conn, event_b, "2026-01-01", "EUR", 1000, 1);
        let t1 = seed_ticket(&conn, event_a, order_a, "EUR");
        let t2 = seed_ticket(&conn, event_b, order_b, "EUR");
        let batch = SaleBatchInput {
            lines: vec![
                SaleBatchLineInput { ticket_id: t1, sale_price_cents: 1000, selling_fees_cents: 0 },
                SaleBatchLineInput { ticket_id: t2, sale_price_cents: 1000, selling_fees_cents: 0 },
            ],
            platform_id: None,
            sale_date: "2026-01-10".to_string(),
            payment_status: Some("paid".to_string()),
            buyer_reference: None,
            notes: None,
            currency: None,
        };
        create_sales_batch_impl(&mut conn, &batch).unwrap();

        let filters = CalendarFilters { date_from: "2026-01-01".into(), date_to: "2026-01-31".into() };
        let entries = get_calendar_impl(&conn, &filters, today).unwrap();
        let sale_entry = entries.iter().find(|e| e.kind == "sale").expect("expected one grouped sale entry");
        assert!(sale_entry.subtitle.as_deref().unwrap().contains("Mixed events"));
    }

    #[test]
    fn sale_batch_with_two_currencies_omits_amount_and_currency() {
        let mut conn = test_conn();
        let today = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let event_id = seed_event(&conn, "Test Event", Some("2026-06-01"));
        let order_id = seed_order(&conn, event_id, "2026-01-01", "EUR", 1000, 2);
        let t1 = seed_ticket(&conn, event_id, order_id, "EUR");
        let t2 = seed_ticket(&conn, event_id, order_id, "USD");
        let batch = SaleBatchInput {
            lines: vec![
                SaleBatchLineInput { ticket_id: t1, sale_price_cents: 1000, selling_fees_cents: 0 },
                SaleBatchLineInput { ticket_id: t2, sale_price_cents: 1000, selling_fees_cents: 0 },
            ],
            platform_id: None,
            sale_date: "2026-01-10".to_string(),
            payment_status: Some("paid".to_string()),
            buyer_reference: None,
            notes: None,
            currency: None, // each line keeps its own ticket's currency -> mixed
        };
        create_sales_batch_impl(&mut conn, &batch).unwrap();

        let filters = CalendarFilters { date_from: "2026-01-01".into(), date_to: "2026-01-31".into() };
        let entries = get_calendar_impl(&conn, &filters, today).unwrap();
        let sale_entry = entries.iter().find(|e| e.kind == "sale").expect("expected one grouped sale entry");
        assert_eq!(sale_entry.amount_cents, None, "must never blend EUR and USD into one number");
        assert_eq!(sale_entry.currency, None);
    }

    #[test]
    fn pull_severity_mirrors_pulls_tsx_warning_window_and_ignores_completed_transfers() {
        let conn = test_conn();
        let today = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let overdue = seed_pull(&conn, "Alice", "Event A", "2026-05-30", 5000, "EUR", false); // 2 days past
        let soon = seed_pull(&conn, "Bob", "Event B", "2026-06-03", 5000, "EUR", false); // 2 days out
        let far = seed_pull(&conn, "Cara", "Event C", "2026-06-20", 5000, "EUR", false); // outside window
        let done = seed_pull(&conn, "Dee", "Event D", "2026-06-02", 5000, "EUR", true); // within window but already transferred

        let filters = CalendarFilters { date_from: "2026-05-01".into(), date_to: "2026-07-01".into() };
        let entries = get_calendar_impl(&conn, &filters, today).unwrap();
        let sev = |id: i64| entries.iter().find(|e| e.kind == "pull" && e.key == format!("pull:{id}")).unwrap().severity.clone();
        assert_eq!(sev(overdue), "critical");
        assert_eq!(sev(soon), "attention");
        assert_eq!(sev(far), "neutral");
        assert_eq!(sev(done), "neutral");
    }

    #[test]
    fn attention_entries_link_to_order_when_present_otherwise_the_event() {
        let conn = test_conn();
        let today = NaiveDate::from_ymd_opt(2026, 1, 10).unwrap();
        let event_id = seed_event(&conn, "Test Event", Some("2026-06-02"));
        let order_id = seed_order(&conn, event_id, "2026-01-01", "EUR", 1000, 1);
        seed_ticket(&conn, event_id, order_id, "EUR"); // available, no listing price -> missing_listing_price (order-linked)

        let filters = CalendarFilters { date_from: "2026-01-01".into(), date_to: "2026-12-31".into() };
        let entries = get_calendar_impl(&conn, &filters, today).unwrap();
        let attn = entries.iter().find(|e| e.kind == "attention").expect("expected an attention entry");
        assert_eq!(attn.link_kind, "order");
        assert_eq!(attn.link_id, Some(order_id));
    }

    #[test]
    fn attention_entries_link_to_the_event_when_no_order_is_involved() {
        let conn = test_conn();
        let today = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let event_id = seed_event(&conn, "Soon Event", Some("2026-06-02")); // 1 day out
        let order_id = seed_order(&conn, event_id, "2026-01-01", "EUR", 1000, 1);
        seed_ticket(&conn, event_id, order_id, "EUR"); // available+unsold -> event_soon (event-level, no order_id)

        let filters = CalendarFilters { date_from: "2026-06-01".into(), date_to: "2026-06-30".into() };
        let entries = get_calendar_impl(&conn, &filters, today).unwrap();
        let attn = entries.iter().find(|e| e.kind == "attention" && e.link_id == Some(event_id)).expect("expected an event_soon attention entry");
        assert_eq!(attn.link_kind, "event");
    }

    #[test]
    fn attention_entries_outside_the_requested_range_are_excluded() {
        let conn = test_conn();
        let today = NaiveDate::from_ymd_opt(2026, 1, 10).unwrap();
        // Order/purchase deliberately ALSO in June - the point of this test
        // is that nothing tied to the June event (including the attention
        // item and the event entry itself) leaks into a Jan-Mar query, not
        // just the attention item in isolation.
        let event_id = seed_event(&conn, "Test Event", Some("2026-06-02"));
        let order_id = seed_order(&conn, event_id, "2026-06-01", "EUR", 1000, 1);
        seed_ticket(&conn, event_id, order_id, "EUR");

        let filters = CalendarFilters { date_from: "2026-01-01".into(), date_to: "2026-03-01".into() }; // excludes June
        let entries = get_calendar_impl(&conn, &filters, today).unwrap();
        assert!(entries.is_empty(), "nothing seeded falls inside Jan-Mar - the June-dated event/attention item must not leak in");
    }

    #[test]
    fn multiple_kinds_can_land_on_the_exact_same_day() {
        let conn = test_conn();
        let today = NaiveDate::from_ymd_opt(2026, 1, 10).unwrap();
        let event_id = seed_event(&conn, "Same Day Event", Some("2026-03-15"));
        seed_pull(&conn, "Alice", "Other pull", "2026-03-15", 1000, "EUR", false);
        seed_order(&conn, event_id, "2026-03-15", "EUR", 2000, 1);

        let filters = CalendarFilters { date_from: "2026-03-01".into(), date_to: "2026-03-31".into() };
        let entries = get_calendar_impl(&conn, &filters, today).unwrap();
        let same_day: Vec<&str> = entries.iter().filter(|e| e.date == "2026-03-15").map(|e| e.kind.as_str()).collect();
        assert!(same_day.contains(&"event"));
        assert!(same_day.contains(&"pull"));
        assert!(same_day.contains(&"order"));
    }

    #[test]
    fn every_kind_present_is_one_of_the_five_supported_categories() {
        let mut conn = test_conn();
        let today = NaiveDate::from_ymd_opt(2026, 1, 10).unwrap();
        let event_id = seed_event(&conn, "Test Event", Some("2026-01-15"));
        seed_event(&conn, "No Date Event", None);
        let order_id = seed_order(&conn, event_id, "2026-01-05", "EUR", 2000, 1);
        let ticket_id = seed_ticket(&conn, event_id, order_id, "EUR");
        let sale = SaleInput {
            ticket_id,
            platform_id: None,
            sale_date: "2026-01-06".to_string(),
            sale_price_cents: 3000,
            selling_fees_cents: 0,
            payment_status: Some("paid".to_string()),
            buyer_reference: None,
            notes: None,
        };
        create_sale_impl(&mut conn, &sale).unwrap();
        seed_pull(&conn, "Alice", "Some Event", "2026-01-20", 1000, "EUR", false);
        let unpriced_order = seed_order(&conn, event_id, "2026-01-02", "EUR", 500, 1);
        seed_ticket(&conn, event_id, unpriced_order, "EUR"); // triggers missing_listing_price attention

        let filters = CalendarFilters { date_from: "2026-01-01".into(), date_to: "2026-01-31".into() };
        let entries = get_calendar_impl(&conn, &filters, today).unwrap();
        let kinds: HashSet<&str> = entries.iter().map(|e| e.kind.as_str()).collect();
        for k in &kinds {
            assert!(["event", "order", "sale", "pull", "attention"].contains(k), "unexpected calendar kind: {k}");
        }
        assert!(kinds.contains("event"));
        assert!(kinds.contains("order"));
        assert!(kinds.contains("sale"));
        assert!(kinds.contains("pull"));
        assert!(kinds.contains("attention"));
    }

    #[test]
    fn date_range_is_inclusive_on_both_boundaries() {
        let conn = test_conn();
        let today = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let inside_start = seed_event(&conn, "Start", Some("2026-02-01"));
        let inside_end = seed_event(&conn, "End", Some("2026-02-28"));
        let before = seed_event(&conn, "Before", Some("2026-01-31"));
        let after = seed_event(&conn, "After", Some("2026-03-01"));

        let filters = CalendarFilters { date_from: "2026-02-01".into(), date_to: "2026-02-28".into() };
        let entries = get_calendar_impl(&conn, &filters, today).unwrap();
        let ids: HashSet<i64> = entries.iter().filter(|e| e.kind == "event").filter_map(|e| e.link_id).collect();
        assert!(ids.contains(&inside_start), "the range's own start date must be included");
        assert!(ids.contains(&inside_end), "the range's own end date must be included");
        assert!(!ids.contains(&before));
        assert!(!ids.contains(&after));
    }

    #[test]
    fn empty_database_returns_an_empty_list_not_an_error() {
        let conn = test_conn();
        let today = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let filters = CalendarFilters { date_from: "2026-01-01".into(), date_to: "2026-01-31".into() };
        let entries = get_calendar_impl(&conn, &filters, today).unwrap();
        assert!(entries.is_empty());
    }
}

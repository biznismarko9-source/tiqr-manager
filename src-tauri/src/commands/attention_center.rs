//! 2.2.8: Dashboard "Attention Center" - a compact, GLOBAL (across every
//! event) list of individual things that currently need a look. Built
//! entirely on top of already-shipped, already-tested logic rather than a
//! new scoring/rules engine (marko's own explicit instruction - "Použi
//! existujúce dáta a logiku tam, kde už existuje" / "nepoužívaj AI ani žiadne
//! nové scraping/API mechanizmy"):
//!
//! - Four of the five categories below (`event_soon`, `missing_listing_
//!   price`, `no_active_listing`, `outside_market_price`) are the EXACT same
//!   per-event "Attention" rules `inventory_intelligence::
//!   get_inventory_intelligence_impl` already computes for the Event
//!   Workspace's own Overview tab (2.2.6, its own frontend rendering removed
//!   in 2.2.9 now that this block covers the same ground globally) - this
//!   module calls that function once per event that actually has unsold
//!   inventory (an event with none could never produce any of the four
//!   anyway - see `events_with_unsold` below) and flattens its `attention`
//!   list into individual, clickable rows instead of one per-event count. A
//!   future change to that function's own thresholds (`EVENT_SOON_DAYS`,
//!   `OUTSIDE_MARKET_THRESHOLD_PCT`) or predicates automatically applies
//!   here too - nothing is duplicated that could quietly drift out of sync.
//! - The fifth, new category (`sold_undelivered`) reuses the exact
//!   `delivery_status = 'Delivered'` convention the 2.0.66 "Completed"
//!   indicator already established (see `orders.rs`/`sales.rs`'s own
//!   `delivered_count`) - a ticket counts as delivered if and only if that
//!   column is literally the string `"Delivered"`; anything else (`NULL`,
//!   `"Not delivered"`, or legacy free text from before that convention
//!   existed) counts as not yet delivered. Scoped directly to
//!   `tickets.status = 'sold'` - a refund reverts that column back to
//!   `'available'` (`refund_sale_impl`, `sales.rs`), so a refunded ticket
//!   drops out on its own, with no extra join or filter needed. This is a
//!   RELIABLE, already-established signal, not a guess - so unlike some
//!   other maybes in this task, it is NOT omitted.
//!
//! Deliberately a SEPARATE, additional block from the Dashboard's existing
//! `DashboardAlerts` (`dashboard.rs` - the alert bell + the Activity tab's
//! own "Attention" cards, 2.0.75/2.0.76/2.0.79-era). That feature already
//! ships its own 4 categories (pulls near deadline, pending sales, missing
//! listing price BY ORDER, upcoming events in a 14-day window) tuned for a
//! different purpose (a glanceable summary + outbound notifications) and is
//! completely untouched by this task. The one real overlap - unsold tickets
//! with no listing price - is computed the same way in both places
//! (ticket-scoped, `status IN ('available','listed') AND listing_price_cents
//! IS NULL`) but surfaced differently (per-ORDER count there, per-ORDER
//! GROUP here too as of 2.2.9 - see below); see `PROTECTED_AREAS.md`'s
//! "2.2.8"/"2.2.9" entries for why these were kept as two separate features
//! rather than merged into one.
//!
//! 2.2.9: reworked from one row per TICKET to one row per ORDER for every
//! ticket-level category. marko's own feedback on the 2.2.8 shape - "nedáva
//! zmysel" (doesn't make sense), evidenced by a real screenshot of one order
//! with 49 tickets all missing a listing price, shown as 49 separate rows -
//! was that this flooded the list far worse than the per-event aggregation
//! `event_soon` already used. `missing_listing_price`/`no_active_listing`/
//! `outside_market_price`/`sold_undelivered` now group their flagged tickets
//! by `order_id` first (see `group_by_order` below) and emit ONE row per
//! (event, category, order) with `ticket_ids`/`ticket_codes` carrying every
//! ticket the row stands for - a 49-ticket order now shows as ONE row,
//! "Order <code> · 49 tickets", not 49. Clicking a grouped row navigates
//! straight to that order's own page (`/orders/:id`, `OrderDetail.tsx`),
//! which already lists every one of those tickets with its own status/
//! listing price/delivery indicators - reusing that existing page rather
//! than building a second ticket-list widget inside the Dashboard, and
//! staying consistent with this feature's own original "click-to-navigate
//! via existing routes" design. `event_soon` is unchanged: it was already
//! aggregated at the EVENT level (one row per soon event, not per ticket),
//! and a soon event's unsold tickets can genuinely span more than one
//! order, so there is no single order to group it under.
//!
//! No new migration, no new dependency, no automatic pricing/repricing
//! anywhere in this file, and `tier`/`section`/`row` are never read as a
//! pricing factor - every "value" this module ever shows is a value that
//! already exists verbatim on the ticket (its own listing price), never a
//! suggested or computed one.

use crate::commands::inventory_intelligence::{get_inventory_intelligence_impl, EVENT_SOON_DAYS};
use crate::db::AppState;
use crate::error::AppResult;
use crate::models::AttentionCenterItem;
use chrono::{Local, NaiveDate};
use rusqlite::Connection;
use std::collections::{HashMap, HashSet};
use tauri::State;

/// Sort key only - never serialized as-is (`AttentionCenterItem.priority` is
/// a plain `String`, so the frontend never needs this enum). Declaration
/// order IS sort order via the derived `Ord`: Critical first, Info last.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Priority {
    Critical,
    Attention,
    Info,
}

impl Priority {
    fn as_str(self) -> &'static str {
        match self {
            Priority::Critical => "critical",
            Priority::Attention => "attention",
            Priority::Info => "info",
        }
    }
}

/// Just enough about one ticket to build every category below without a
/// second round-trip per ticket - resolved once, up front, for the whole
/// database (see `get_attention_center_impl`). Realistic scale for a single
/// reseller's local SQLite file, same "plain full-table scan, no pagination"
/// convention `dashboard.rs` already uses (e.g. `SELECT DISTINCT currency
/// FROM tickets`).
///
/// 2.2.9: added `order_id` (a real, `NOT NULL` column on `tickets` since the
/// very first migration) so every ticket-level category can group by order -
/// see this module's own doc comment. Every ticket under one order shares
/// that order's `event_id` by construction (`orders::create_order_impl`
/// inserts both the order and its tickets with the same `input.event_id`),
/// so resolving a group's event from any one of its tickets is always safe.
struct TicketMini {
    code: String,
    event_id: i64,
    order_id: i64,
    status: String,
    delivery_status: Option<String>,
    currency: String,
    listing_price_cents: Option<i64>,
}

/// Sentinel used ONLY to sort items with no `event_date` to the end when
/// ordering by "nearest event first" - mirrors the exact sentinel string
/// `dashboard.rs`'s `period_bounds`/`PERIOD_MAX_SENTINEL` already use for "no
/// upper bound". Never shown to the user and never written into an actual
/// `AttentionCenterItem` - items keep their real `event_date`, which may
/// genuinely be `None`.
const NO_DATE_SORT_SENTINEL: &str = "9999-12-31";

/// Groups a category's flagged ticket ids by their order, returning
/// `(order_id, ticket_ids)` pairs - one per distinct order, each list of
/// ticket ids sorted for deterministic output, pairs themselves sorted by
/// `order_id` too. Ticket ids that no longer resolve in `tickets_by_id`
/// (shouldn't happen - both are built from the same live query - but cheap
/// to guard) are silently skipped rather than panicking.
///
/// Shared by every ticket-level category (all except `event_soon`) - see
/// this module's doc comment for why grouping by order replaced one-row-
/// per-ticket in 2.2.9.
fn group_by_order(ticket_ids: &[i64], tickets_by_id: &HashMap<i64, TicketMini>) -> Vec<(i64, Vec<i64>)> {
    let mut by_order: HashMap<i64, Vec<i64>> = HashMap::new();
    for &ticket_id in ticket_ids {
        if let Some(t) = tickets_by_id.get(&ticket_id) {
            by_order.entry(t.order_id).or_default().push(ticket_id);
        }
    }
    let mut order_ids: Vec<i64> = by_order.keys().copied().collect();
    order_ids.sort_unstable();
    order_ids
        .into_iter()
        .map(|oid| {
            let mut ids = by_order.remove(&oid).unwrap_or_default();
            ids.sort_unstable();
            (oid, ids)
        })
        .collect()
}

fn ticket_codes_for(ticket_ids: &[i64], tickets_by_id: &HashMap<i64, TicketMini>) -> Vec<String> {
    ticket_ids.iter().filter_map(|id| tickets_by_id.get(id).map(|t| t.code.clone())).collect()
}

#[allow(clippy::too_many_arguments)]
fn push_item(
    items: &mut Vec<AttentionCenterItem>,
    category: &str,
    priority: Priority,
    event_id: i64,
    event_name: &str,
    event_date: Option<&str>,
    order_id: Option<i64>,
    order_code: Option<&str>,
    ticket_ids: Vec<i64>,
    ticket_codes: Vec<String>,
    reason: String,
    amount_cents: Option<i64>,
    currency: Option<&str>,
) {
    let key = match order_id {
        Some(oid) => format!("{category}:order:{oid}"),
        None => format!("{category}:{event_id}"),
    };
    items.push(AttentionCenterItem {
        key,
        category: category.to_string(),
        priority: priority.as_str().to_string(),
        event_id,
        event_name: event_name.to_string(),
        event_date: event_date.map(|s| s.to_string()),
        order_id,
        order_code: order_code.map(|s| s.to_string()),
        ticket_ids,
        ticket_codes,
        reason,
        amount_cents,
        currency: currency.map(|s| s.to_string()),
    });
}

/// Whole days between `today` and `date_str` ("YYYY-MM-DD"), or `None` if it
/// doesn't parse. Same plain calendar-day arithmetic as
/// `inventory_intelligence`'s own `event_soon` check and `dashboard.rs`'s
/// `daysUntil`-equivalent reasoning - never a rolling clock, since
/// `event_date` has no time component anywhere in this schema.
fn days_until(today: NaiveDate, date_str: &str) -> Option<i64> {
    NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok().map(|d| (d - today).num_days())
}

fn priority_rank(p: &str) -> u8 {
    match p {
        "critical" => 0,
        "attention" => 1,
        _ => 2,
    }
}

pub(crate) fn get_attention_center_impl(conn: &Connection, today: NaiveDate) -> AppResult<Vec<AttentionCenterItem>> {
    let events_by_id: HashMap<i64, (String, Option<String>)> = {
        let mut stmt = conn.prepare("SELECT id, name, event_date FROM events")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, (r.get(1)?, r.get(2)?))))?;
        rows.collect::<Result<HashMap<_, _>, _>>()?
    };

    let tickets_by_id: HashMap<i64, TicketMini> = {
        let mut stmt = conn
            .prepare("SELECT id, code, event_id, order_id, status, delivery_status, currency, listing_price_cents FROM tickets")?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                TicketMini {
                    code: r.get(1)?,
                    event_id: r.get(2)?,
                    order_id: r.get(3)?,
                    status: r.get(4)?,
                    delivery_status: r.get(5)?,
                    currency: r.get(6)?,
                    listing_price_cents: r.get(7)?,
                },
            ))
        })?;
        rows.collect::<Result<HashMap<_, _>, _>>()?
    };

    // 2.2.9: order codes for display/navigation - `orders.code` is the same
    // human-facing code OrderDetail.tsx's own route/header already use, so
    // the frontend never has to look it up separately.
    let orders_by_id: HashMap<i64, String> = {
        let mut stmt = conn.prepare("SELECT id, code FROM orders")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?;
        rows.collect::<Result<HashMap<_, _>, _>>()?
    };

    let mut items: Vec<AttentionCenterItem> = Vec::new();

    // ---- categories 1-4: reuse the exact per-event Inventory Intelligence
    // Attention rules, one call per event that actually has unsold stock. --
    // (An event with zero unsold tickets could never contribute to any of
    // the 4 unsold-scoped categories anyway - this is a performance
    // narrowing only, never a correctness one.) Sorted purely for
    // deterministic test output; final display order is decided by the
    // priority/date sort at the end of this function regardless.
    let mut events_with_unsold: Vec<i64> = tickets_by_id
        .values()
        .filter(|t| t.status == "available" || t.status == "listed")
        .map(|t| t.event_id)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    events_with_unsold.sort_unstable();

    for event_id in events_with_unsold {
        let Some((event_name, event_date)) = events_by_id.get(&event_id) else { continue };
        let intelligence = get_inventory_intelligence_impl(conn, event_id, today)?;

        for attention_item in &intelligence.attention {
            match attention_item.key.as_str() {
                "event_soon" => {
                    // Deliberately ONE ROW PER EVENT here, not one per unsold
                    // ticket or order - marko's own spec lists "Ticket/code
                    // (ak je relevantný)" as OPTIONAL, and an event with e.g.
                    // 40 unsold tickets 1 day out would otherwise flood the
                    // list with near-identical rows, directly against his own
                    // "UI musí zostať prehľadné" requirement. This mirrors
                    // how the Dashboard's own existing "Upcoming events" list
                    // (dashboard.rs/Dashboard.tsx) already shows one row per
                    // event too, not one per ticket or order.
                    if attention_item.count > 0 {
                        push_item(
                            &mut items,
                            "event_soon",
                            Priority::Critical,
                            event_id,
                            event_name,
                            event_date.as_deref(),
                            None,
                            None,
                            Vec::new(),
                            Vec::new(),
                            format!(
                                "{} unsold ticket{} - event date approaching",
                                attention_item.count,
                                if attention_item.count == 1 { "" } else { "s" }
                            ),
                            None,
                            None,
                        );
                    }
                }
                "missing_listing_price" => {
                    for (order_id, ticket_ids) in group_by_order(&attention_item.ticket_ids, &tickets_by_id) {
                        let ticket_codes = ticket_codes_for(&ticket_ids, &tickets_by_id);
                        let order_code = orders_by_id.get(&order_id).map(|s| s.as_str());
                        push_item(
                            &mut items,
                            "missing_listing_price",
                            Priority::Attention,
                            event_id,
                            event_name,
                            event_date.as_deref(),
                            Some(order_id),
                            order_code,
                            ticket_ids,
                            ticket_codes,
                            "No listing price set".to_string(),
                            None,
                            None,
                        );
                    }
                }
                "no_active_listing" => {
                    for (order_id, ticket_ids) in group_by_order(&attention_item.ticket_ids, &tickets_by_id) {
                        let ticket_codes = ticket_codes_for(&ticket_ids, &tickets_by_id);
                        let order_code = orders_by_id.get(&order_id).map(|s| s.as_str());
                        push_item(
                            &mut items,
                            "no_active_listing",
                            Priority::Attention,
                            event_id,
                            event_name,
                            event_date.as_deref(),
                            Some(order_id),
                            order_code,
                            ticket_ids,
                            ticket_codes,
                            "No active listing on any marketplace".to_string(),
                            None,
                            None,
                        );
                    }
                }
                "outside_market_price" => {
                    // `available` is `false` whenever this event has no
                    // Price Checker data yet - marko's own explicit "iba ak
                    // pre daný event existujú uložené Price Checker dáta".
                    // `ticket_ids` is already guaranteed empty in that case
                    // (see `AttentionItem.available`'s own doc comment), but
                    // the explicit check keeps this arm's intent obvious.
                    if attention_item.available {
                        for (order_id, ticket_ids) in group_by_order(&attention_item.ticket_ids, &tickets_by_id) {
                            let ticket_codes = ticket_codes_for(&ticket_ids, &tickets_by_id);
                            let order_code = orders_by_id.get(&order_id).map(|s| s.as_str());
                            // A specific listing price only means something
                            // for a single-ticket row - a multi-ticket group
                            // has no one "the" price to show, so this stays
                            // `None` rather than picking one arbitrarily. See
                            // AttentionCenterItem.amountCents's own doc
                            // comment.
                            let (amount_cents, currency) = if ticket_ids.len() == 1 {
                                tickets_by_id
                                    .get(&ticket_ids[0])
                                    .map(|t| (t.listing_price_cents, Some(t.currency.as_str())))
                                    .unwrap_or((None, None))
                            } else {
                                (None, None)
                            };
                            push_item(
                                &mut items,
                                "outside_market_price",
                                // Info, not Attention/Critical: marko was
                                // explicit that this task must never
                                // recommend or imply a price action ("žiadne
                                // automatické určovanie ani navrhovanie
                                // ceny") - this is a pricing OBSERVATION, not
                                // a gap you must fill in like the two
                                // categories above.
                                Priority::Info,
                                event_id,
                                event_name,
                                event_date.as_deref(),
                                Some(order_id),
                                order_code,
                                ticket_ids,
                                ticket_codes,
                                "Listing price is significantly outside the market average".to_string(),
                                amount_cents,
                                currency,
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // ---- category 5: sold, delivery not marked complete ------------------
    // Independent of `events_with_unsold` above on purpose - a fully sold-out
    // event (zero unsold tickets) can still have an undelivered sold ticket,
    // and must not be skipped just because the pre-filter above is scoped to
    // unsold inventory. See this module's doc comment for the exact
    // `delivery_status` convention this reuses, and for why grouping by
    // order below is safe (every ticket under one order shares that order's
    // event).
    let mut sold_undelivered_ids: Vec<i64> = tickets_by_id
        .iter()
        .filter(|(_, t)| t.status == "sold" && t.delivery_status.as_deref() != Some("Delivered"))
        .map(|(id, _)| *id)
        .collect();
    sold_undelivered_ids.sort_unstable();

    for (order_id, ticket_ids) in group_by_order(&sold_undelivered_ids, &tickets_by_id) {
        let Some(first) = ticket_ids.first().and_then(|id| tickets_by_id.get(id)) else { continue };
        let event_id = first.event_id;
        let Some((event_name, event_date)) = events_by_id.get(&event_id) else { continue };
        let days = event_date.as_deref().and_then(|d| days_until(today, d));
        // No lower bound on `days` - same reasoning `dashboard.rs`'s own
        // `PULLS_WARNING_WINDOW_DAYS` check already documents: an event that
        // has already happened with delivery still incomplete is MORE
        // urgent, never exempted just because its date is in the past. No
        // `event_date` at all means urgency can't be established, so this
        // defaults to Attention rather than guessing Critical.
        let priority = match days {
            Some(d) if d <= EVENT_SOON_DAYS => Priority::Critical,
            _ => Priority::Attention,
        };
        let ticket_codes = ticket_codes_for(&ticket_ids, &tickets_by_id);
        let order_code = orders_by_id.get(&order_id).map(|s| s.as_str());
        push_item(
            &mut items,
            "sold_undelivered",
            priority,
            event_id,
            event_name,
            event_date.as_deref(),
            Some(order_id),
            order_code,
            ticket_ids,
            ticket_codes,
            "Sold, but delivery isn't marked complete yet".to_string(),
            None,
            None,
        );
    }

    // ---- sort: priority first, then soonest event -------------------------
    // ("zoradenie podľa priority a najbližšieho eventu" - marko's own spec.)
    // The final `.key` tie-break makes ordering fully deterministic (matters
    // for the sort-order test below), not a display-visible concern.
    items.sort_by(|a, b| {
        priority_rank(&a.priority).cmp(&priority_rank(&b.priority)).then_with(|| {
            let da = a.event_date.as_deref().unwrap_or(NO_DATE_SORT_SENTINEL);
            let db_ = b.event_date.as_deref().unwrap_or(NO_DATE_SORT_SENTINEL);
            da.cmp(db_)
        }).then_with(|| a.key.cmp(&b.key))
    });

    Ok(items)
}

#[tauri::command]
pub fn get_attention_center(state: State<AppState>) -> AppResult<Vec<AttentionCenterItem>> {
    let conn = state.db.lock().unwrap();
    get_attention_center_impl(&conn, Local::now().date_naive())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_conn;
    use rusqlite::params;

    static NEXT_CODE: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);
    fn next_code(prefix: &str) -> String {
        format!("{prefix}-{}", NEXT_CODE.fetch_add(1, std::sync::atomic::Ordering::SeqCst))
    }

    fn seed_event(conn: &Connection, event_date: Option<&str>) -> i64 {
        conn.execute(
            "INSERT INTO events (name, event_date, status) VALUES ('Test Event', ?1, 'upcoming')",
            [event_date],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    fn seed_order(conn: &Connection, event_id: i64) -> i64 {
        conn.execute(
            "INSERT INTO orders (code, event_id, purchase_date, quantity, currency) VALUES (?1, ?2, '2026-01-01', 1, 'EUR')",
            params![next_code("ORD"), event_id],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    #[allow(clippy::too_many_arguments)]
    fn seed_ticket(
        conn: &Connection,
        event_id: i64,
        order_id: i64,
        status: &str,
        listing_price_cents: Option<i64>,
        currency: &str,
        delivery_status: Option<&str>,
    ) -> i64 {
        conn.execute(
            "INSERT INTO tickets (code, event_id, order_id, purchase_cost_cents, listing_price_cents, currency, status, delivery_status)
             VALUES (?1, ?2, ?3, 1000, ?4, ?5, ?6, ?7)",
            params![next_code("TKT"), event_id, order_id, listing_price_cents, currency, status, delivery_status],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    fn seed_listing(conn: &Connection, ticket_id: i64, marketplace_id: i64, price_cents: i64, currency: &str) {
        conn.execute(
            "INSERT INTO ticket_listings (ticket_id, marketplace_id, price_cents, currency, status)
             VALUES (?1, ?2, ?3, ?4, 'active')",
            params![ticket_id, marketplace_id, price_cents, currency],
        )
        .unwrap();
    }

    /// `test_conn()` runs every real migration, including 014_price_checker's
    /// own seeded marketplace rows - same helper/reasoning as
    /// `inventory_intelligence.rs`'s own tests.
    fn marketplace_id(conn: &Connection, name: &str) -> i64 {
        conn.execute("INSERT OR IGNORE INTO marketplaces (name) VALUES (?1)", [name]).unwrap();
        conn.query_row("SELECT id FROM marketplaces WHERE name = ?1", [name], |r| r.get(0)).unwrap()
    }

    fn seed_price_check(conn: &Connection, event_id: i64, marketplace_id: i64, average_price_cents: i64, currency: &str) {
        conn.execute(
            "INSERT INTO price_checks (event_id, marketplace_id, lowest_price_cents, average_price_cents, highest_price_cents, listing_count, currency)
             VALUES (?1, ?2, ?3, ?3, ?3, 3, ?4)",
            params![event_id, marketplace_id, average_price_cents, currency],
        )
        .unwrap();
    }

    /// Finds the (at most one) item under `category` whose `ticket_ids`
    /// contains `ticket_id` - 2.2.9's grouped shape means a category can no
    /// longer be looked up by a single ticket id directly (several tickets
    /// now share one row), so tests search by membership instead.
    fn find_containing<'a>(items: &'a [AttentionCenterItem], category: &str, ticket_id: i64) -> Option<&'a AttentionCenterItem> {
        items.iter().find(|i| i.category == category && i.ticket_ids.contains(&ticket_id))
    }

    fn find_event_level<'a>(items: &'a [AttentionCenterItem], category: &str, event_id: i64) -> Option<&'a AttentionCenterItem> {
        items.iter().find(|i| i.category == category && i.event_id == event_id && i.order_id.is_none())
    }

    #[test]
    fn event_soon_fires_for_an_event_within_the_window_with_unsold_tickets() {
        let conn = test_conn();
        let today = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let event_id = seed_event(&conn, Some("2026-06-02")); // 1 day out
        let order_id = seed_order(&conn, event_id);
        seed_ticket(&conn, event_id, order_id, "available", Some(5000), "EUR", None);

        let items = get_attention_center_impl(&conn, today).unwrap();
        let item = find_event_level(&items, "event_soon", event_id).expect("event_soon item expected");
        assert_eq!(item.priority, "critical");
        assert_eq!(item.event_id, event_id);
        assert!(item.order_id.is_none(), "event_soon is aggregated per event, not per order/ticket");
        assert!(item.ticket_ids.is_empty(), "event_soon carries no individual ticket ids");
        assert!(item.reason.contains('1'), "reason should mention the 1 unsold ticket");
    }

    #[test]
    fn event_soon_does_not_fire_outside_the_window() {
        let conn = test_conn();
        let today = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let event_id = seed_event(&conn, Some("2026-06-10")); // 9 days out
        let order_id = seed_order(&conn, event_id);
        seed_ticket(&conn, event_id, order_id, "available", Some(5000), "EUR", None);

        let items = get_attention_center_impl(&conn, today).unwrap();
        assert!(find_event_level(&items, "event_soon", event_id).is_none());
    }

    #[test]
    fn unsold_ticket_without_active_listing_becomes_an_order_level_item() {
        let conn = test_conn();
        let today = NaiveDate::from_ymd_opt(2026, 1, 10).unwrap();
        let event_id = seed_event(&conn, None);
        let order_id = seed_order(&conn, event_id);
        let vivid = marketplace_id(&conn, "Vivid Seats");
        let listed_ok = seed_ticket(&conn, event_id, order_id, "listed", Some(5000), "EUR", None);
        seed_listing(&conn, listed_ok, vivid, 5000, "EUR");
        let listed_bad = seed_ticket(&conn, event_id, order_id, "listed", Some(5000), "EUR", None);

        let items = get_attention_center_impl(&conn, today).unwrap();
        assert!(find_containing(&items, "no_active_listing", listed_ok).is_none());
        let item = find_containing(&items, "no_active_listing", listed_bad).expect("expected item");
        assert_eq!(item.priority, "attention");
        assert_eq!(item.event_id, event_id);
        assert_eq!(item.order_id, Some(order_id));
        assert_eq!(item.ticket_ids, vec![listed_bad], "the ok ticket must not be swept into the same order's row");
    }

    #[test]
    fn unsold_ticket_without_listing_price_becomes_an_order_level_item() {
        let conn = test_conn();
        let today = NaiveDate::from_ymd_opt(2026, 1, 10).unwrap();
        let event_id = seed_event(&conn, None);
        let order_id = seed_order(&conn, event_id);
        let priced = seed_ticket(&conn, event_id, order_id, "available", Some(5000), "EUR", None);
        let unpriced = seed_ticket(&conn, event_id, order_id, "available", None, "EUR", None);

        let items = get_attention_center_impl(&conn, today).unwrap();
        assert!(find_containing(&items, "missing_listing_price", priced).is_none());
        assert!(find_containing(&items, "missing_listing_price", unpriced).is_some());
    }

    #[test]
    fn tickets_sharing_an_order_and_category_are_grouped_into_one_row() {
        // Regression guard for marko's own "nedáva zmysel" feedback: a real
        // example he sent was 49 tickets, one order, all missing a listing
        // price, shown as 49 separate rows in 2.2.8. This seeds a smaller
        // but equivalent shape - 3 tickets, one order, same reason - and
        // asserts exactly ONE row, not 3.
        let conn = test_conn();
        let today = NaiveDate::from_ymd_opt(2026, 1, 10).unwrap();
        let event_id = seed_event(&conn, None);
        let order_id = seed_order(&conn, event_id);
        let t1 = seed_ticket(&conn, event_id, order_id, "available", None, "EUR", None);
        let t2 = seed_ticket(&conn, event_id, order_id, "available", None, "EUR", None);
        let t3 = seed_ticket(&conn, event_id, order_id, "available", None, "EUR", None);

        let items = get_attention_center_impl(&conn, today).unwrap();
        let rows: Vec<&AttentionCenterItem> = items.iter().filter(|i| i.category == "missing_listing_price").collect();
        assert_eq!(rows.len(), 1, "one order, one reason, must be exactly one row");
        let row = rows[0];
        assert_eq!(row.order_id, Some(order_id));
        assert!(row.order_code.is_some(), "a grouped row must resolve its order's human-facing code");
        let mut ids = row.ticket_ids.clone();
        ids.sort_unstable();
        let mut expected = vec![t1, t2, t3];
        expected.sort_unstable();
        assert_eq!(ids, expected);
        assert_eq!(row.ticket_codes.len(), 3);
    }

    #[test]
    fn tickets_under_different_orders_get_separate_rows_even_for_the_same_category_and_event() {
        let conn = test_conn();
        let today = NaiveDate::from_ymd_opt(2026, 1, 10).unwrap();
        let event_id = seed_event(&conn, None);
        let order_a = seed_order(&conn, event_id);
        let order_b = seed_order(&conn, event_id);
        let ticket_a = seed_ticket(&conn, event_id, order_a, "available", None, "EUR", None);
        let ticket_b = seed_ticket(&conn, event_id, order_b, "available", None, "EUR", None);

        let items = get_attention_center_impl(&conn, today).unwrap();
        let rows: Vec<&AttentionCenterItem> = items.iter().filter(|i| i.category == "missing_listing_price").collect();
        assert_eq!(rows.len(), 2, "two different orders must never be merged into one row");
        let row_a = find_containing(&items, "missing_listing_price", ticket_a).unwrap();
        let row_b = find_containing(&items, "missing_listing_price", ticket_b).unwrap();
        assert_ne!(row_a.order_id, row_b.order_id);
        assert_eq!(row_a.ticket_ids, vec![ticket_a]);
        assert_eq!(row_b.ticket_ids, vec![ticket_b]);
    }

    #[test]
    fn outside_market_price_only_fires_when_price_checker_data_exists_for_that_event() {
        let conn = test_conn();
        let today = NaiveDate::from_ymd_opt(2026, 1, 10).unwrap();

        // Event A: priced way off, but NO Price Checker data at all yet.
        let event_a = seed_event(&conn, None);
        let order_a = seed_order(&conn, event_a);
        let ticket_a = seed_ticket(&conn, event_a, order_a, "available", Some(99_999), "EUR", None);

        // Event B: same shape, but WITH real Price Checker data - and priced
        // way above the market average.
        let event_b = seed_event(&conn, None);
        let order_b = seed_order(&conn, event_b);
        let ticket_b = seed_ticket(&conn, event_b, order_b, "available", Some(10_000), "EUR", None);
        let vivid = marketplace_id(&conn, "Vivid Seats");
        seed_price_check(&conn, event_b, vivid, 5_000, "EUR"); // 100% above average

        let items = get_attention_center_impl(&conn, today).unwrap();
        assert!(
            find_containing(&items, "outside_market_price", ticket_a).is_none(),
            "no Price Checker data for event A - must not be invented"
        );
        let item = find_containing(&items, "outside_market_price", ticket_b).expect("expected item for event B");
        assert_eq!(item.priority, "info");
        assert_eq!(item.order_id, Some(order_b));
        assert_eq!(item.amount_cents, Some(10_000), "a single-ticket group still shows that ticket's own listing price");
        assert_eq!(item.currency.as_deref(), Some("EUR"));
    }

    #[test]
    fn outside_market_price_omits_amount_for_a_multi_ticket_group() {
        let conn = test_conn();
        let today = NaiveDate::from_ymd_opt(2026, 1, 10).unwrap();
        let event_id = seed_event(&conn, None);
        let order_id = seed_order(&conn, event_id);
        let t1 = seed_ticket(&conn, event_id, order_id, "available", Some(10_000), "EUR", None);
        let t2 = seed_ticket(&conn, event_id, order_id, "available", Some(11_000), "EUR", None);
        let vivid = marketplace_id(&conn, "Vivid Seats");
        seed_price_check(&conn, event_id, vivid, 1_000, "EUR");

        let items = get_attention_center_impl(&conn, today).unwrap();
        let row = find_containing(&items, "outside_market_price", t1).unwrap();
        assert!(find_containing(&items, "outside_market_price", t2).is_some(), "both tickets must be in the same row");
        assert_eq!(row.ticket_ids.len(), 2);
        assert_eq!(row.amount_cents, None, "no single 'the' price for a 2-ticket group");
        assert_eq!(row.currency, None);
    }

    #[test]
    fn sold_ticket_without_delivered_status_is_flagged_but_delivered_and_refunded_tickets_are_not() {
        let conn = test_conn();
        let today = NaiveDate::from_ymd_opt(2026, 1, 10).unwrap();
        let event_id = seed_event(&conn, None);
        let order_id = seed_order(&conn, event_id);

        let undelivered = seed_ticket(&conn, event_id, order_id, "sold", Some(5000), "EUR", None);
        let delivered = seed_ticket(&conn, event_id, order_id, "sold", Some(5000), "EUR", Some("Delivered"));
        // Was sold and "Delivered", then refunded - refund reverts status
        // back to 'available' (mirrors refund_sale_impl) even though the old
        // delivery_status value is still sitting there unchanged.
        let refunded = seed_ticket(&conn, event_id, order_id, "available", Some(5000), "EUR", Some("Delivered"));
        // Never sold at all, but happens to have a stray "Not delivered"
        // value (e.g. leftover from before this ticket was even created) -
        // must not count, nothing has been sold yet.
        let never_sold = seed_ticket(&conn, event_id, order_id, "available", Some(5000), "EUR", Some("Not delivered"));

        let items = get_attention_center_impl(&conn, today).unwrap();
        assert!(find_containing(&items, "sold_undelivered", undelivered).is_some());
        assert!(find_containing(&items, "sold_undelivered", delivered).is_none());
        assert!(find_containing(&items, "sold_undelivered", refunded).is_none());
        assert!(find_containing(&items, "sold_undelivered", never_sold).is_none());
    }

    #[test]
    fn sold_undelivered_groups_by_order_too() {
        let conn = test_conn();
        let today = NaiveDate::from_ymd_opt(2026, 1, 10).unwrap();
        let event_id = seed_event(&conn, None);
        let order_id = seed_order(&conn, event_id);
        let t1 = seed_ticket(&conn, event_id, order_id, "sold", Some(5000), "EUR", None);
        let t2 = seed_ticket(&conn, event_id, order_id, "sold", Some(5000), "EUR", None);

        let items = get_attention_center_impl(&conn, today).unwrap();
        let rows: Vec<&AttentionCenterItem> = items.iter().filter(|i| i.category == "sold_undelivered").collect();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].order_id, Some(order_id));
        let mut ids = rows[0].ticket_ids.clone();
        ids.sort_unstable();
        let mut expected = vec![t1, t2];
        expected.sort_unstable();
        assert_eq!(ids, expected);
    }

    #[test]
    fn sold_undelivered_is_critical_when_the_event_is_soon_or_past_and_attention_otherwise() {
        let conn = test_conn();
        let today = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();

        let past_event = seed_event(&conn, Some("2026-05-20")); // already happened
        let order_past = seed_order(&conn, past_event);
        let past_ticket = seed_ticket(&conn, past_event, order_past, "sold", Some(5000), "EUR", None);

        let far_event = seed_event(&conn, Some("2026-12-01")); // months away
        let order_far = seed_order(&conn, far_event);
        let far_ticket = seed_ticket(&conn, far_event, order_far, "sold", Some(5000), "EUR", None);

        let no_date_event = seed_event(&conn, None);
        let order_no_date = seed_order(&conn, no_date_event);
        let no_date_ticket = seed_ticket(&conn, no_date_event, order_no_date, "sold", Some(5000), "EUR", None);

        let items = get_attention_center_impl(&conn, today).unwrap();
        assert_eq!(find_containing(&items, "sold_undelivered", past_ticket).unwrap().priority, "critical");
        assert_eq!(find_containing(&items, "sold_undelivered", far_ticket).unwrap().priority, "attention");
        assert_eq!(
            find_containing(&items, "sold_undelivered", no_date_ticket).unwrap().priority,
            "attention",
            "no event_date means urgency can't be established - defaults to attention, never a guessed critical"
        );
    }

    #[test]
    fn sold_undelivered_fires_even_for_an_otherwise_fully_sold_out_event() {
        // Regression guard: this category must NOT be gated by the
        // `events_with_unsold` pre-filter used for the other 4 categories -
        // a sold-out event (zero unsold tickets) can still have an
        // undelivered sold ticket.
        let conn = test_conn();
        let today = NaiveDate::from_ymd_opt(2026, 1, 10).unwrap();
        let event_id = seed_event(&conn, None);
        let order_id = seed_order(&conn, event_id);
        let ticket_id = seed_ticket(&conn, event_id, order_id, "sold", Some(5000), "EUR", None);

        let items = get_attention_center_impl(&conn, today).unwrap();
        assert!(find_containing(&items, "sold_undelivered", ticket_id).is_some());
    }

    #[test]
    fn the_same_ticket_can_appear_under_two_categories_but_never_twice_under_one() {
        let conn = test_conn();
        let today = NaiveDate::from_ymd_opt(2026, 1, 10).unwrap();
        let event_id = seed_event(&conn, None);
        let order_id = seed_order(&conn, event_id);
        // Missing BOTH a listing price AND an active listing at once.
        let ticket_id = seed_ticket(&conn, event_id, order_id, "available", None, "EUR", None);

        let items = get_attention_center_impl(&conn, today).unwrap();
        assert!(find_containing(&items, "missing_listing_price", ticket_id).is_some());
        assert!(find_containing(&items, "no_active_listing", ticket_id).is_some());

        let mut keys: Vec<&str> = items.iter().map(|i| i.key.as_str()).collect();
        let before = keys.len();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), before, "no key must ever repeat - that would mean the same order shown twice under the same reason");
    }

    #[test]
    fn items_are_sorted_critical_then_attention_then_info_and_by_soonest_event_within_each() {
        let conn = test_conn();
        let today = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();

        // Info: outside_market_price.
        let event_info = seed_event(&conn, None);
        let order_info = seed_order(&conn, event_info);
        let ticket_info = seed_ticket(&conn, event_info, order_info, "available", Some(10_000), "EUR", None);
        let vivid = marketplace_id(&conn, "Vivid Seats");
        seed_price_check(&conn, event_info, vivid, 1_000, "EUR");

        // Attention: missing_listing_price.
        let event_attention = seed_event(&conn, None);
        let order_attention = seed_order(&conn, event_attention);
        seed_ticket(&conn, event_attention, order_attention, "available", None, "EUR", None);

        // Critical: two event_soon events, a later one seeded first so a
        // naive "insertion order" would get this wrong if the sort were
        // missing/broken.
        let event_soon_later = seed_event(&conn, Some("2026-01-03"));
        let order_later = seed_order(&conn, event_soon_later);
        seed_ticket(&conn, event_soon_later, order_later, "available", Some(1000), "EUR", None);
        let event_soon_sooner = seed_event(&conn, Some("2026-01-02"));
        let order_sooner = seed_order(&conn, event_soon_sooner);
        seed_ticket(&conn, event_soon_sooner, order_sooner, "available", Some(1000), "EUR", None);

        let items = get_attention_center_impl(&conn, today).unwrap();
        let priorities: Vec<&str> = items.iter().map(|i| i.priority.as_str()).collect();
        let first_attention = priorities.iter().position(|p| *p == "attention").unwrap();
        let first_info = priorities.iter().position(|p| *p == "info").unwrap();
        let last_critical = priorities.iter().rposition(|p| *p == "critical").unwrap();
        assert!(last_critical < first_attention, "every critical item must sort before every attention item");
        assert!(first_attention < first_info, "attention items must sort before info items");

        let critical_event_ids: Vec<i64> = items.iter().filter(|i| i.priority == "critical").map(|i| i.event_id).collect();
        assert_eq!(
            critical_event_ids,
            vec![event_soon_sooner, event_soon_later],
            "within the same priority, the soonest event must come first regardless of seed order"
        );
        let _ = ticket_info; // seeded only to exercise the info-priority item above
    }
}

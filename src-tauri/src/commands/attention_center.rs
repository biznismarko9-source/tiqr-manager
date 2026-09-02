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
//!   Workspace's own Overview tab (2.2.6) - this module calls that function
//!   once per event that actually has unsold inventory (an event with none
//!   could never produce any of the four anyway - see `events_with_unsold`
//!   below) and flattens its `attention` list into individual, clickable
//!   rows instead of one per-event count. A future change to that function's
//!   own thresholds (`EVENT_SOON_DAYS`, `OUTSIDE_MARKET_THRESHOLD_PCT`) or
//!   predicates automatically applies here too - nothing is duplicated that
//!   could quietly drift out of sync.
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
//! IS NULL`) but surfaced differently (per-ORDER count there, per-TICKET row
//! here); see `PROTECTED_AREAS.md`'s "2.2.8" entry for why these were kept
//! as two separate features rather than merged into one.
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
struct TicketMini {
    code: String,
    event_id: i64,
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

#[allow(clippy::too_many_arguments)]
fn push_item(
    items: &mut Vec<AttentionCenterItem>,
    category: &str,
    priority: Priority,
    event_id: i64,
    event_name: &str,
    event_date: Option<&str>,
    ticket_id: Option<i64>,
    ticket_code: Option<&str>,
    reason: String,
    amount_cents: Option<i64>,
    currency: Option<&str>,
) {
    let key = match ticket_id {
        Some(tid) => format!("{category}:{tid}"),
        None => format!("{category}:{event_id}"),
    };
    items.push(AttentionCenterItem {
        key,
        category: category.to_string(),
        priority: priority.as_str().to_string(),
        event_id,
        event_name: event_name.to_string(),
        event_date: event_date.map(|s| s.to_string()),
        ticket_id,
        ticket_code: ticket_code.map(|s| s.to_string()),
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
        let mut stmt =
            conn.prepare("SELECT id, code, event_id, status, delivery_status, currency, listing_price_cents FROM tickets")?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                TicketMini {
                    code: r.get(1)?,
                    event_id: r.get(2)?,
                    status: r.get(3)?,
                    delivery_status: r.get(4)?,
                    currency: r.get(5)?,
                    listing_price_cents: r.get(6)?,
                },
            ))
        })?;
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
                    // ticket - marko's own spec lists "Ticket/code (ak je
                    // relevantný)" as OPTIONAL, and an event with e.g. 40
                    // unsold tickets 1 day out would otherwise flood the list
                    // with 40 near-identical rows, directly against his own
                    // "UI musí zostať prehľadné" requirement. This mirrors
                    // how the Dashboard's own existing "Upcoming events" list
                    // (dashboard.rs/Dashboard.tsx) already shows one row per
                    // event too, not one per ticket.
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
                    for &ticket_id in &attention_item.ticket_ids {
                        let Some(t) = tickets_by_id.get(&ticket_id) else { continue };
                        push_item(
                            &mut items,
                            "missing_listing_price",
                            Priority::Attention,
                            event_id,
                            event_name,
                            event_date.as_deref(),
                            Some(ticket_id),
                            Some(t.code.as_str()),
                            "No listing price set".to_string(),
                            None,
                            None,
                        );
                    }
                }
                "no_active_listing" => {
                    for &ticket_id in &attention_item.ticket_ids {
                        let Some(t) = tickets_by_id.get(&ticket_id) else { continue };
                        push_item(
                            &mut items,
                            "no_active_listing",
                            Priority::Attention,
                            event_id,
                            event_name,
                            event_date.as_deref(),
                            Some(ticket_id),
                            Some(t.code.as_str()),
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
                        for &ticket_id in &attention_item.ticket_ids {
                            let Some(t) = tickets_by_id.get(&ticket_id) else { continue };
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
                                Some(ticket_id),
                                Some(t.code.as_str()),
                                "Listing price is significantly outside the market average".to_string(),
                                t.listing_price_cents,
                                Some(t.currency.as_str()),
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
    // `delivery_status` convention this reuses.
    for (ticket_id, t) in &tickets_by_id {
        if t.status != "sold" {
            continue;
        }
        if t.delivery_status.as_deref() == Some("Delivered") {
            continue;
        }
        let Some((event_name, event_date)) = events_by_id.get(&t.event_id) else { continue };
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
        push_item(
            &mut items,
            "sold_undelivered",
            priority,
            t.event_id,
            event_name,
            event_date.as_deref(),
            Some(*ticket_id),
            Some(t.code.as_str()),
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

    fn find<'a>(items: &'a [AttentionCenterItem], category: &str, ticket_id: Option<i64>) -> Option<&'a AttentionCenterItem> {
        items.iter().find(|i| i.category == category && i.ticket_id == ticket_id)
    }

    #[test]
    fn event_soon_fires_for_an_event_within_the_window_with_unsold_tickets() {
        let conn = test_conn();
        let today = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let event_id = seed_event(&conn, Some("2026-06-02")); // 1 day out
        let order_id = seed_order(&conn, event_id);
        seed_ticket(&conn, event_id, order_id, "available", Some(5000), "EUR", None);

        let items = get_attention_center_impl(&conn, today).unwrap();
        let item = find(&items, "event_soon", None).expect("event_soon item expected");
        assert_eq!(item.priority, "critical");
        assert_eq!(item.event_id, event_id);
        assert!(item.ticket_id.is_none(), "event_soon is aggregated per event, not per ticket");
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
        assert!(find(&items, "event_soon", None).is_none());
    }

    #[test]
    fn unsold_ticket_without_active_listing_becomes_a_ticket_level_item() {
        let conn = test_conn();
        let today = NaiveDate::from_ymd_opt(2026, 1, 10).unwrap();
        let event_id = seed_event(&conn, None);
        let order_id = seed_order(&conn, event_id);
        let vivid = marketplace_id(&conn, "Vivid Seats");
        let listed_ok = seed_ticket(&conn, event_id, order_id, "listed", Some(5000), "EUR", None);
        seed_listing(&conn, listed_ok, vivid, 5000, "EUR");
        let listed_bad = seed_ticket(&conn, event_id, order_id, "listed", Some(5000), "EUR", None);

        let items = get_attention_center_impl(&conn, today).unwrap();
        assert!(find(&items, "no_active_listing", Some(listed_ok)).is_none());
        let item = find(&items, "no_active_listing", Some(listed_bad)).expect("expected item");
        assert_eq!(item.priority, "attention");
        assert_eq!(item.event_id, event_id);
    }

    #[test]
    fn unsold_ticket_without_listing_price_becomes_a_ticket_level_item() {
        let conn = test_conn();
        let today = NaiveDate::from_ymd_opt(2026, 1, 10).unwrap();
        let event_id = seed_event(&conn, None);
        let order_id = seed_order(&conn, event_id);
        let priced = seed_ticket(&conn, event_id, order_id, "available", Some(5000), "EUR", None);
        let unpriced = seed_ticket(&conn, event_id, order_id, "available", None, "EUR", None);

        let items = get_attention_center_impl(&conn, today).unwrap();
        assert!(find(&items, "missing_listing_price", Some(priced)).is_none());
        assert!(find(&items, "missing_listing_price", Some(unpriced)).is_some());
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
            find(&items, "outside_market_price", Some(ticket_a)).is_none(),
            "no Price Checker data for event A - must not be invented"
        );
        let item = find(&items, "outside_market_price", Some(ticket_b)).expect("expected item for event B");
        assert_eq!(item.priority, "info");
        assert_eq!(item.amount_cents, Some(10_000));
        assert_eq!(item.currency.as_deref(), Some("EUR"));
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
        assert!(find(&items, "sold_undelivered", Some(undelivered)).is_some());
        assert!(find(&items, "sold_undelivered", Some(delivered)).is_none());
        assert!(find(&items, "sold_undelivered", Some(refunded)).is_none());
        assert!(find(&items, "sold_undelivered", Some(never_sold)).is_none());
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
        assert_eq!(find(&items, "sold_undelivered", Some(past_ticket)).unwrap().priority, "critical");
        assert_eq!(find(&items, "sold_undelivered", Some(far_ticket)).unwrap().priority, "attention");
        assert_eq!(
            find(&items, "sold_undelivered", Some(no_date_ticket)).unwrap().priority,
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
        assert!(find(&items, "sold_undelivered", Some(ticket_id)).is_some());
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
        assert!(find(&items, "missing_listing_price", Some(ticket_id)).is_some());
        assert!(find(&items, "no_active_listing", Some(ticket_id)).is_some());

        let mut keys: Vec<&str> = items.iter().map(|i| i.key.as_str()).collect();
        let before = keys.len();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), before, "no key must ever repeat - that would mean the same ticket shown twice under the same reason");
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

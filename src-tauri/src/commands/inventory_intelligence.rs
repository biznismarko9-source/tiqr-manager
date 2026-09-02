//! 2.2.6: marko's own request for a compact "Inventory Intelligence" block on
//! the Event Workspace's Overview tab (`EventDetail.tsx`), above the existing
//! Orders/Tickets tables - KPIs, an aging breakdown (days since purchase,
//! still unsold), an attention list, and breakdowns by section and by
//! marketplace.
//!
//! Pure read/aggregation - one new command, no schema change, no writes, and
//! this module never touches `tickets.status`/`tickets.listing_price_cents`,
//! `ticket_listings`, sales, refunds, or Finance at all (same "explicitly
//! read-only, changes nothing" shape as `price_checker_analysis.rs`). Every
//! number below reuses an EXISTING definition already established elsewhere
//! in this codebase rather than inventing a competing one:
//!
//! - Total tickets / total invested / sell-through % / average ticket cost
//!   are derived from the same per-ticket rows `events::STATS_SQL` (via
//!   `finance::compute_summary`) already aggregates for `EventWithStats.
//!   stats` - same scope (ALL tickets, cancelled included), so "Total
//!   tickets"/"Total invested" here can never disagree with the existing
//!   Overview "Tickets"/"Total cost" stat cards for the same event.
//! - Current listed value reuses ListingsTab's own existing "Listed value"
//!   definition (EventDetail.tsx) - sum of `price_cents` across this event's
//!   ACTIVE `ticket_listings` rows, single-currency-safe - just computed
//!   here since that one has only ever existed client-side.
//! - Potential profit reuses SalesTab's own existing "Potential Profit"
//!   definition byte-for-byte (unsold tickets' summed legacy `tickets.
//!   listing_price_cents` minus their summed cost) - deliberately NOT
//!   `ticket_listings`-based, so it can never disagree with the card that
//!   already shows this same number on the Sales tab.
//! - The "outside market price" attention item reuses `commands::
//!   price_checker::get_price_checker_summary_impl` - the exact same
//!   function SalesTab's own "Market vs. mine" card already calls - rather
//!   than a second market-comparison implementation. It is only ever
//!   populated when that summary already has real data for this event
//!   (`market_average_price_cents.is_some()`), matching marko's own explicit
//!   "iba ak uz existuju data z Price Checker/Market Analysis".
//!
//! 2.2.7: also includes a breakdown "by tier/level", where until now
//! `tickets` had no tier/level column anywhere in this schema (see
//! PROTECTED_AREAS.md's "2.2.0" entry and `YourTicketGroup.tier`'s own doc
//! comment in models.rs, which hit this exact gap before marko's 2.2.7
//! "Ticket metadata: Tier / Level" task added `tickets.tier`, migration
//! 024). Grouped exactly like the section breakdown below, over that same
//! real, already-stored value - never inferred from `section` or
//! `ticket_type` (a DELIVERY method, not a price tier - see migration 024's
//! own doc comment for that recurring mix-up). Blank/NULL groups as
//! "Unknown", deliberately different wording from the section breakdown's
//! own "No section" - marko's explicit instruction for this one field. See
//! `InventoryIntelligence`'s own doc comment (models.rs).

use crate::commands::price_checker::get_price_checker_summary_impl;
use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::finance;
use crate::models::{AgingBucket, AttentionItem, InventoryBreakdownGroup, InventoryIntelligence, InventoryIntelligenceKpis};
use chrono::{Local, NaiveDate};
use rusqlite::{Connection, Row};
use std::collections::{HashMap, HashSet};
use tauri::State;

/// marko asked for "<48h"; `events.event_date` has no time component
/// anywhere in this schema (plain "YYYY-MM-DD", same as everywhere else this
/// app deals with dates), so 48h is treated as "within the next 2 calendar
/// days" - the same whole-day granularity Dashboard.tsx's own `daysUntil`/
/// `UPCOMING_WARNING_WINDOW_DAYS` already use for event urgency, rather than
/// inventing hour-level precision the schema doesn't have. Flagged in
/// REDESIGN-2.2.6-REPORT.md as an interpretation, not a data gap.
// 2.2.8: `pub(crate)` (was private) so the new global Attention Center
// (commands::attention_center) can reuse this EXACT constant instead of
// defining a second one that could drift - see that module's own doc
// comment. No behavior change here.
pub(crate) const EVENT_SOON_DAYS: i64 = 2;

/// How far a ticket's own listing price may sit from `market_average_price_
/// cents` before it counts as "significantly outside market price" - a
/// judgment call (marko's request names no number), picked to flag real
/// outliers without drowning the list in ordinary price variation. Easy to
/// retune; not derived from anything else in this codebase.
const OUTSIDE_MARKET_THRESHOLD_PCT: f64 = 0.20;

struct TicketRow {
    id: i64,
    status: String,
    section: Option<String>,
    /// 2.2.7: `tickets.tier` (migration 024) - see this module's doc
    /// comment.
    tier: Option<String>,
    currency: String,
    total_cost_cents: i64,
    listing_price_cents: Option<i64>,
    purchase_date: String,
}

struct ListingRow {
    ticket_id: i64,
    price_cents: i64,
    currency: String,
    marketplace_name: String,
}

fn map_ticket_row(row: &Row) -> rusqlite::Result<TicketRow> {
    Ok(TicketRow {
        id: row.get(0)?,
        status: row.get(1)?,
        section: row.get(2)?,
        currency: row.get(3)?,
        total_cost_cents: row.get(4)?,
        listing_price_cents: row.get(5)?,
        purchase_date: row.get(6)?,
        // Appended at the end (index 7) rather than inserted alongside
        // `section` above, so none of the existing positional indices 0-6
        // shift - same convention csv_export.rs documents explicitly for
        // its own new-column additions.
        tier: row.get(7)?,
    })
}

fn map_listing_row(row: &Row) -> rusqlite::Result<ListingRow> {
    Ok(ListingRow {
        ticket_id: row.get(0)?,
        price_cents: row.get(1)?,
        currency: row.get(2)?,
        marketplace_name: row.get(3)?,
    })
}

fn is_unsold(status: &str) -> bool {
    status == "available" || status == "listed"
}

/// `Some(code)` when every value in `currencies` is the same; `None` when
/// they differ OR the slice is empty - same "never blend, never guess"
/// convention as `FinanceSummary.currency`/`PriceCheckerSummary.my_currency`.
fn single_currency<'a>(currencies: impl Iterator<Item = &'a str>) -> Option<String> {
    let mut set: HashSet<&str> = HashSet::new();
    for c in currencies {
        set.insert(c);
    }
    if set.len() == 1 {
        set.into_iter().next().map(|s| s.to_string())
    } else {
        None
    }
}

/// One "days since purchased" bucket definition - see `AgingBucket`'s own
/// doc comment (models.rs) for why 8-30/30-60 became 8-30/31-60.
struct BucketDef {
    key: &'static str,
    label: &'static str,
    min_days: i64,
    max_days: Option<i64>,
}

const BUCKET_DEFS: [BucketDef; 4] = [
    BucketDef { key: "0_7", label: "0-7 days", min_days: 0, max_days: Some(7) },
    BucketDef { key: "8_30", label: "8-30 days", min_days: 8, max_days: Some(30) },
    BucketDef { key: "31_60", label: "31-60 days", min_days: 31, max_days: Some(60) },
    BucketDef { key: "61_plus", label: "61+ days", min_days: 61, max_days: None },
];

/// Groups `rows` by `label_of`, in first-seen order, summing `ticket_count`/
/// `total_cents` and collecting `ticket_ids` per group - the shared shape
/// both the section and marketplace breakdowns below need. `currency_of`
/// gives that one row's currency, for the group's own single-currency-safe
/// total.
fn group_rows<T>(
    rows: &[T],
    label_of: impl Fn(&T) -> String,
    ticket_id_of: impl Fn(&T) -> i64,
    amount_cents_of: impl Fn(&T) -> i64,
    currency_of: impl Fn(&T) -> &str,
) -> Vec<InventoryBreakdownGroup> {
    let mut order: Vec<String> = Vec::new();
    let mut by_label: HashMap<String, (i64, Vec<i64>, i64, HashSet<String>)> = HashMap::new();
    for row in rows {
        let label = label_of(row);
        let entry = by_label.entry(label.clone()).or_insert_with(|| {
            order.push(label.clone());
            (0, Vec::new(), 0, HashSet::new())
        });
        entry.0 += 1;
        entry.1.push(ticket_id_of(row));
        entry.2 += amount_cents_of(row);
        entry.3.insert(currency_of(row).to_string());
    }
    let mut groups: Vec<InventoryBreakdownGroup> = order
        .into_iter()
        .map(|label| {
            let (ticket_count, ticket_ids, total_cents, currencies) = by_label.remove(&label).unwrap();
            let currency = if currencies.len() == 1 { currencies.into_iter().next() } else { None };
            InventoryBreakdownGroup { label, ticket_count, ticket_ids, total_cents, currency }
        })
        .collect();
    groups.sort_by_key(|g| std::cmp::Reverse(g.ticket_count));
    groups
}

pub(crate) fn get_inventory_intelligence_impl(
    conn: &Connection,
    event_id: i64,
    today: NaiveDate,
) -> AppResult<InventoryIntelligence> {
    let event_date: Option<String> = conn
        .query_row("SELECT event_date FROM events WHERE id = ?1", [event_id], |r| r.get(0))
        .map_err(|_| AppError::NotFound(format!("Event #{event_id} not found")))?;

    let tickets: Vec<TicketRow> = {
        let mut stmt = conn.prepare(
            "SELECT t.id, t.status, t.section, t.currency,
                    t.purchase_cost_cents + t.purchase_fees_cents + t.other_costs_cents,
                    t.listing_price_cents, o.purchase_date, t.tier
             FROM tickets t
             JOIN orders o ON o.id = t.order_id
             WHERE t.event_id = ?1",
        )?;
        let rows = stmt.query_map([event_id], map_ticket_row)?;
        rows.collect::<Result<Vec<_>, _>>()?
    };

    // Active listings only - matches ListingsTab's own summary cards, which
    // are likewise never affected by that tab's status/marketplace filters.
    let listings: Vec<ListingRow> = {
        let mut stmt = conn.prepare(
            "SELECT tl.ticket_id, tl.price_cents, tl.currency, m.name
             FROM ticket_listings tl
             JOIN tickets t ON t.id = tl.ticket_id
             JOIN marketplaces m ON m.id = tl.marketplace_id
             WHERE t.event_id = ?1 AND tl.status = 'active'",
        )?;
        let rows = stmt.query_map([event_id], map_listing_row)?;
        rows.collect::<Result<Vec<_>, _>>()?
    };

    // ---- KPIs --------------------------------------------------------------
    let total_tickets = tickets.len() as i64;
    let total_invested_cents: i64 = tickets.iter().map(|t| t.total_cost_cents).sum();
    let currency = single_currency(tickets.iter().map(|t| t.currency.as_str()));
    let sold_ticket_ids: Vec<i64> = tickets.iter().filter(|t| t.status == "sold").map(|t| t.id).collect();
    let sell_through_pct = finance::safe_ratio(sold_ticket_ids.len() as i64, total_tickets);
    let average_ticket_cost_cents =
        finance::safe_ratio(total_invested_cents, total_tickets).map(|v| v.round() as i64);

    let unsold: Vec<&TicketRow> = tickets.iter().filter(|t| is_unsold(&t.status)).collect();
    let unsold_ticket_ids: Vec<i64> = unsold.iter().map(|t| t.id).collect();
    // Byte-for-byte SalesTab's own "Potential Profit" formula (EventDetail.tsx) -
    // see this module's doc comment.
    let potential_inventory_cost_cents: i64 = unsold.iter().map(|t| t.total_cost_cents).sum();
    let potential_listing_value_cents: i64 = unsold.iter().filter_map(|t| t.listing_price_cents).sum();
    let potential_profit_cents = potential_listing_value_cents - potential_inventory_cost_cents;
    let potential_profit_currency = single_currency(unsold.iter().map(|t| t.currency.as_str()));

    let current_listed_value_cents: i64 = listings.iter().map(|l| l.price_cents).sum();
    let current_listed_value_currency = single_currency(listings.iter().map(|l| l.currency.as_str()));

    let kpis = InventoryIntelligenceKpis {
        total_tickets,
        total_invested_cents,
        currency,
        current_listed_value_cents,
        current_listed_value_currency,
        potential_profit_cents,
        potential_profit_currency,
        sell_through_pct,
        average_ticket_cost_cents,
    };

    // ---- Aging (unsold tickets only) ----------------------------------------
    let mut aging: Vec<AgingBucket> = BUCKET_DEFS
        .iter()
        .map(|b| AgingBucket { key: b.key.to_string(), label: b.label.to_string(), ticket_count: 0, ticket_ids: Vec::new() })
        .collect();
    for t in &unsold {
        let Ok(purchase_date) = NaiveDate::parse_from_str(&t.purchase_date, "%Y-%m-%d") else { continue };
        let days = (today - purchase_date).num_days().max(0);
        let in_bucket = |b: &BucketDef| {
            days >= b.min_days
                && match b.max_days {
                    Some(m) => days <= m,
                    None => true,
                }
        };
        if let Some(idx) = BUCKET_DEFS.iter().position(in_bucket) {
            aging[idx].ticket_count += 1;
            aging[idx].ticket_ids.push(t.id);
        }
    }

    // ---- Attention -----------------------------------------------------------
    let event_soon = event_date
        .as_deref()
        .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
        .map(|ed| {
            let days_until = (ed - today).num_days();
            (0..=EVENT_SOON_DAYS).contains(&days_until)
        })
        .unwrap_or(false)
        && !unsold_ticket_ids.is_empty();
    let event_soon_ids = if event_soon { unsold_ticket_ids.clone() } else { Vec::new() };

    let missing_listing_price_ids: Vec<i64> =
        unsold.iter().filter(|t| t.listing_price_cents.is_none()).map(|t| t.id).collect();

    let listed_ticket_ids: HashSet<i64> = listings.iter().map(|l| l.ticket_id).collect();
    let no_active_listing_ids: Vec<i64> =
        unsold.iter().filter(|t| !listed_ticket_ids.contains(&t.id)).map(|t| t.id).collect();

    // Reuses the exact same summary SalesTab's own "Market vs. mine" card
    // already calls - see this module's doc comment. Only ever populated
    // when that summary already has real market data for THIS currency.
    let price_summary = get_price_checker_summary_impl(conn, event_id)?;
    let (outside_market_available, outside_market_ids) =
        match (price_summary.my_currency.as_deref(), price_summary.market_average_price_cents) {
            (Some(cur), Some(avg)) if avg > 0 => {
                let mut ids = Vec::new();
                for t in &unsold {
                    if t.currency.as_str() != cur {
                        continue;
                    }
                    let Some(price) = t.listing_price_cents else { continue };
                    let deviation = (price - avg).unsigned_abs() as f64 / avg as f64;
                    if deviation >= OUTSIDE_MARKET_THRESHOLD_PCT {
                        ids.push(t.id);
                    }
                }
                (true, ids)
            }
            _ => (false, Vec::new()),
        };

    let attention = vec![
        AttentionItem {
            key: "event_soon".to_string(),
            count: event_soon_ids.len() as i64,
            ticket_ids: event_soon_ids,
            available: true,
        },
        AttentionItem {
            key: "missing_listing_price".to_string(),
            count: missing_listing_price_ids.len() as i64,
            ticket_ids: missing_listing_price_ids,
            available: true,
        },
        AttentionItem {
            key: "no_active_listing".to_string(),
            count: no_active_listing_ids.len() as i64,
            ticket_ids: no_active_listing_ids,
            available: true,
        },
        AttentionItem {
            key: "outside_market_price".to_string(),
            count: outside_market_ids.len() as i64,
            ticket_ids: outside_market_ids,
            available: outside_market_available,
        },
    ];

    // ---- Breakdown by tier/level (unsold tickets) -----------------------------
    // 2.2.7: `tickets.tier` (migration 024) - see this module's doc comment.
    // "Unknown" (NOT "No section"'s wording) for blank/null - marko's own
    // explicit instruction for this one field.
    let breakdown_by_tier = group_rows(
        &unsold,
        |t| {
            t.tier
                .as_deref()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .unwrap_or("Unknown")
                .to_string()
        },
        |t| t.id,
        |t| t.total_cost_cents,
        |t| t.currency.as_str(),
    );

    // ---- Breakdown by section (unsold tickets) --------------------------------
    let breakdown_by_section = group_rows(
        &unsold,
        |t| {
            t.section
                .as_deref()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .unwrap_or("No section")
                .to_string()
        },
        |t| t.id,
        |t| t.total_cost_cents,
        |t| t.currency.as_str(),
    );

    // ---- Breakdown by marketplace (active listings) ---------------------------
    let breakdown_by_marketplace = group_rows(
        &listings,
        |l| l.marketplace_name.clone(),
        |l| l.ticket_id,
        |l| l.price_cents,
        |l| l.currency.as_str(),
    );

    Ok(InventoryIntelligence {
        kpis,
        aging,
        attention,
        breakdown_by_tier,
        breakdown_by_section,
        breakdown_by_marketplace,
        unsold_ticket_ids,
        sold_ticket_ids,
    })
}

#[tauri::command]
pub fn get_inventory_intelligence(state: State<AppState>, event_id: i64) -> AppResult<InventoryIntelligence> {
    let conn = state.db.lock().unwrap();
    get_inventory_intelligence_impl(&conn, event_id, Local::now().date_naive())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_conn;
    use rusqlite::params;

    fn seed_event(conn: &Connection, event_date: Option<&str>) -> i64 {
        conn.execute(
            "INSERT INTO events (name, event_date, status) VALUES ('Test Event', ?1, 'upcoming')",
            [event_date],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    // `conn.last_insert_rowid() + offset` is NOT a safe way to generate a
    // unique code here - it reflects whatever row (any table) was inserted
    // last BEFORE this statement runs, so two seed calls in a row with
    // nothing else inserted between them compute the exact same "next" id
    // and collide on the UNIQUE `code` column. A dedicated counter, unique
    // for the whole test process, sidesteps that entirely.
    static NEXT_CODE: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);
    fn next_code(prefix: &str) -> String {
        format!("{prefix}-{}", NEXT_CODE.fetch_add(1, std::sync::atomic::Ordering::SeqCst))
    }

    fn seed_order(conn: &Connection, event_id: i64, purchase_date: &str) -> i64 {
        conn.execute(
            "INSERT INTO orders (code, event_id, purchase_date, quantity, currency)
             VALUES (?1, ?2, ?3, 1, 'EUR')",
            params![next_code("ORD"), event_id, purchase_date],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    #[allow(clippy::too_many_arguments)]
    fn seed_ticket(
        conn: &Connection,
        event_id: i64,
        order_id: i64,
        section: Option<&str>,
        status: &str,
        purchase_cost_cents: i64,
        listing_price_cents: Option<i64>,
        currency: &str,
    ) -> i64 {
        conn.execute(
            "INSERT INTO tickets (code, event_id, order_id, section, purchase_cost_cents, listing_price_cents, currency, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                next_code("TKT"),
                event_id,
                order_id,
                section,
                purchase_cost_cents,
                listing_price_cents,
                currency,
                status
            ],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    /// Same as `seed_ticket` above, plus a `tier` value - a separate helper
    /// rather than a new parameter on `seed_ticket` itself, so every existing
    /// call site (which has nothing to do with tier) stays untouched.
    #[allow(clippy::too_many_arguments)]
    fn seed_ticket_with_tier(
        conn: &Connection,
        event_id: i64,
        order_id: i64,
        section: Option<&str>,
        tier: Option<&str>,
        status: &str,
        purchase_cost_cents: i64,
        listing_price_cents: Option<i64>,
        currency: &str,
    ) -> i64 {
        conn.execute(
            "INSERT INTO tickets (code, event_id, order_id, section, tier, purchase_cost_cents, listing_price_cents, currency, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                next_code("TKT"),
                event_id,
                order_id,
                section,
                tier,
                purchase_cost_cents,
                listing_price_cents,
                currency,
                status
            ],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    // `test_conn()` runs every real migration, including 014_price_checker's
    // own seeded rows ("Vivid Seats", "Ticombo", ...) - so a test asking for
    // one of those familiar names by name must reuse the existing row
    // (INSERT OR IGNORE + look up) rather than assume the table starts
    // empty, or it collides with `marketplaces.name`'s own UNIQUE constraint.
    fn seed_marketplace(conn: &Connection, name: &str) -> i64 {
        conn.execute("INSERT OR IGNORE INTO marketplaces (name) VALUES (?1)", [name]).unwrap();
        conn.query_row("SELECT id FROM marketplaces WHERE name = ?1", [name], |r| r.get(0)).unwrap()
    }

    fn seed_listing(conn: &Connection, ticket_id: i64, marketplace_id: i64, price_cents: i64, currency: &str, status: &str) {
        conn.execute(
            "INSERT INTO ticket_listings (ticket_id, marketplace_id, price_cents, currency, status)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![ticket_id, marketplace_id, price_cents, currency, status],
        )
        .unwrap();
    }

    #[test]
    fn a_fresh_event_with_no_tickets_is_all_zeroes_not_an_error() {
        let conn = test_conn();
        let event_id = seed_event(&conn, None);
        let result = get_inventory_intelligence_impl(&conn, event_id, NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()).unwrap();
        assert_eq!(result.kpis.total_tickets, 0);
        assert_eq!(result.kpis.total_invested_cents, 0);
        assert_eq!(result.kpis.sell_through_pct, None, "0/0 must be N/A, not 0.0 or a panic");
        assert_eq!(result.kpis.average_ticket_cost_cents, None);
        assert_eq!(result.kpis.current_listed_value_cents, 0);
        assert!(result.aging.iter().all(|b| b.ticket_count == 0));
        assert!(result.attention.iter().all(|a| a.count == 0));
        assert!(result.breakdown_by_tier.is_empty());
        assert!(result.breakdown_by_section.is_empty());
        assert!(result.breakdown_by_marketplace.is_empty());
    }

    #[test]
    fn an_unknown_event_id_is_not_found() {
        let conn = test_conn();
        let err = get_inventory_intelligence_impl(&conn, 999, NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()).unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[test]
    fn total_tickets_and_total_invested_match_finance_compute_summary_scope() {
        // Same scope as events::STATS_SQL/finance::compute_summary: ALL
        // tickets regardless of status, cancelled included - see this
        // module's own doc comment.
        let conn = test_conn();
        let event_id = seed_event(&conn, None);
        let order_id = seed_order(&conn, event_id, "2026-01-01");
        seed_ticket(&conn, event_id, order_id, None, "available", 1000, None, "EUR");
        seed_ticket(&conn, event_id, order_id, None, "sold", 2000, None, "EUR");
        seed_ticket(&conn, event_id, order_id, None, "cancelled", 500, None, "EUR");
        let result = get_inventory_intelligence_impl(&conn, event_id, NaiveDate::from_ymd_opt(2026, 1, 10).unwrap()).unwrap();
        assert_eq!(result.kpis.total_tickets, 3, "cancelled counts too, same as the existing Overview 'Tickets' stat");
        assert_eq!(result.kpis.total_invested_cents, 3500);
        assert_eq!(result.kpis.currency, Some("EUR".to_string()));
        // 3500/3 = 1166.66... - rounded, not truncated, same idiom as
        // `avg_listing_price_cents` elsewhere in this codebase.
        assert_eq!(result.kpis.average_ticket_cost_cents, Some(1167));
    }

    #[test]
    fn sell_through_is_sold_over_all_tickets_including_cancelled() {
        let conn = test_conn();
        let event_id = seed_event(&conn, None);
        let order_id = seed_order(&conn, event_id, "2026-01-01");
        seed_ticket(&conn, event_id, order_id, None, "sold", 1000, None, "EUR");
        seed_ticket(&conn, event_id, order_id, None, "available", 1000, None, "EUR");
        seed_ticket(&conn, event_id, order_id, None, "cancelled", 1000, None, "EUR");
        seed_ticket(&conn, event_id, order_id, None, "cancelled", 1000, None, "EUR");
        let result = get_inventory_intelligence_impl(&conn, event_id, NaiveDate::from_ymd_opt(2026, 1, 10).unwrap()).unwrap();
        assert_eq!(result.kpis.sell_through_pct, Some(0.25));
        assert_eq!(result.sold_ticket_ids.len(), 1);
    }

    #[test]
    fn mixed_currency_tickets_suppress_the_blended_totals_currency_flag() {
        let conn = test_conn();
        let event_id = seed_event(&conn, None);
        let order_id = seed_order(&conn, event_id, "2026-01-01");
        seed_ticket(&conn, event_id, order_id, None, "available", 1000, None, "EUR");
        seed_ticket(&conn, event_id, order_id, None, "available", 1000, None, "USD");
        let result = get_inventory_intelligence_impl(&conn, event_id, NaiveDate::from_ymd_opt(2026, 1, 10).unwrap()).unwrap();
        assert_eq!(result.kpis.currency, None);
        assert_eq!(result.kpis.total_invested_cents, 2000, "the arithmetic itself still happens - only the currency label is suppressed");
    }

    #[test]
    fn potential_profit_matches_sales_tabs_existing_formula() {
        let conn = test_conn();
        let event_id = seed_event(&conn, None);
        let order_id = seed_order(&conn, event_id, "2026-01-01");
        // Unsold, priced: cost 1000, legacy listing price 1500.
        seed_ticket(&conn, event_id, order_id, None, "available", 1000, Some(1500), "EUR");
        // Unsold, unpriced: cost 800, contributes to cost but not listing value.
        seed_ticket(&conn, event_id, order_id, None, "listed", 800, None, "EUR");
        // Sold - excluded entirely from Potential Profit's scope.
        seed_ticket(&conn, event_id, order_id, None, "sold", 5000, Some(6000), "EUR");
        let result = get_inventory_intelligence_impl(&conn, event_id, NaiveDate::from_ymd_opt(2026, 1, 10).unwrap()).unwrap();
        // inventory cost = 1000 + 800 = 1800; listing value = 1500 (only the priced one); profit = -300.
        assert_eq!(result.kpis.potential_profit_cents, 1500 - 1800);
        assert_eq!(result.kpis.potential_profit_currency, Some("EUR".to_string()));
        assert_eq!(result.unsold_ticket_ids.len(), 2);
    }

    #[test]
    fn current_listed_value_only_counts_active_listings_matching_listings_tab() {
        let conn = test_conn();
        let event_id = seed_event(&conn, None);
        let order_id = seed_order(&conn, event_id, "2026-01-01");
        let t1 = seed_ticket(&conn, event_id, order_id, None, "listed", 1000, None, "EUR");
        let t2 = seed_ticket(&conn, event_id, order_id, None, "listed", 1000, None, "EUR");
        let vivid = seed_marketplace(&conn, "Vivid Seats");
        seed_listing(&conn, t1, vivid, 2000, "EUR", "active");
        seed_listing(&conn, t2, vivid, 3000, "EUR", "removed"); // must NOT count
        let result = get_inventory_intelligence_impl(&conn, event_id, NaiveDate::from_ymd_opt(2026, 1, 10).unwrap()).unwrap();
        assert_eq!(result.kpis.current_listed_value_cents, 2000);
        assert_eq!(result.kpis.current_listed_value_currency, Some("EUR".to_string()));
    }

    #[test]
    fn aging_buckets_split_at_7_30_and_60_days_and_only_include_unsold_tickets() {
        let conn = test_conn();
        let event_id = seed_event(&conn, None);
        let today = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
        let mk_order = |days_ago: i64| seed_order(&conn, event_id, &(today - chrono::Duration::days(days_ago)).format("%Y-%m-%d").to_string());
        let o0 = mk_order(0);
        let o7 = mk_order(7);
        let o8 = mk_order(8);
        let o30 = mk_order(30);
        let o31 = mk_order(31);
        let o60 = mk_order(60);
        let o61 = mk_order(61);
        let o200 = mk_order(200);
        seed_ticket(&conn, event_id, o0, None, "available", 100, None, "EUR");
        seed_ticket(&conn, event_id, o7, None, "available", 100, None, "EUR");
        seed_ticket(&conn, event_id, o8, None, "listed", 100, None, "EUR");
        seed_ticket(&conn, event_id, o30, None, "listed", 100, None, "EUR");
        seed_ticket(&conn, event_id, o31, None, "available", 100, None, "EUR");
        seed_ticket(&conn, event_id, o60, None, "available", 100, None, "EUR");
        seed_ticket(&conn, event_id, o61, None, "available", 100, None, "EUR");
        // Sold - must be excluded from aging entirely even though it's old.
        seed_ticket(&conn, event_id, o200, None, "sold", 100, None, "EUR");

        let result = get_inventory_intelligence_impl(&conn, event_id, today).unwrap();
        let counts: Vec<i64> = result.aging.iter().map(|b| b.ticket_count).collect();
        assert_eq!(counts, vec![2, 2, 2, 1], "0-7, 8-30, 31-60, 61+ in that order");
        assert_eq!(result.aging.iter().map(|b| b.ticket_ids.len() as i64).sum::<i64>(), 7, "the sold ticket must not appear in any bucket");
    }

    #[test]
    fn missing_listing_price_and_no_active_listing_are_independent_attention_items() {
        let conn = test_conn();
        let event_id = seed_event(&conn, None);
        let order_id = seed_order(&conn, event_id, "2026-01-01");
        let vivid = seed_marketplace(&conn, "Vivid Seats");
        // Has a legacy listing price AND a real active listing - clean, no attention.
        let t_clean = seed_ticket(&conn, event_id, order_id, None, "listed", 1000, Some(1500), "EUR");
        seed_listing(&conn, t_clean, vivid, 1500, "EUR", "active");
        // Has a legacy listing price but no real active listing.
        seed_ticket(&conn, event_id, order_id, None, "listed", 1000, Some(1500), "EUR");
        // Has no legacy listing price at all (and so no active listing either).
        seed_ticket(&conn, event_id, order_id, None, "available", 1000, None, "EUR");

        let result = get_inventory_intelligence_impl(&conn, event_id, NaiveDate::from_ymd_opt(2026, 1, 10).unwrap()).unwrap();
        let missing_price = result.attention.iter().find(|a| a.key == "missing_listing_price").unwrap();
        let no_listing = result.attention.iter().find(|a| a.key == "no_active_listing").unwrap();
        assert_eq!(missing_price.count, 1);
        assert_eq!(no_listing.count, 2, "both the unpriced ticket AND the priced-but-unlisted one");
    }

    #[test]
    fn event_soon_attention_only_fires_within_2_days_and_with_real_unsold_stock() {
        let conn = test_conn();
        let today = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let soon_event = seed_event(&conn, Some("2026-06-03")); // 2 days out
        let order_id = seed_order(&conn, soon_event, "2026-01-01");
        seed_ticket(&conn, soon_event, order_id, None, "available", 1000, None, "EUR");
        let result = get_inventory_intelligence_impl(&conn, soon_event, today).unwrap();
        let event_soon = result.attention.iter().find(|a| a.key == "event_soon").unwrap();
        assert_eq!(event_soon.count, 1);

        let far_event = seed_event(&conn, Some("2026-06-10")); // 9 days out - not soon
        let order_id2 = seed_order(&conn, far_event, "2026-01-01");
        seed_ticket(&conn, far_event, order_id2, None, "available", 1000, None, "EUR");
        let result2 = get_inventory_intelligence_impl(&conn, far_event, today).unwrap();
        assert_eq!(result2.attention.iter().find(|a| a.key == "event_soon").unwrap().count, 0);

        // Soon, but every ticket already sold - nothing left to worry about.
        let sold_out_event = seed_event(&conn, Some("2026-06-02"));
        let order_id3 = seed_order(&conn, sold_out_event, "2026-01-01");
        seed_ticket(&conn, sold_out_event, order_id3, None, "sold", 1000, None, "EUR");
        let result3 = get_inventory_intelligence_impl(&conn, sold_out_event, today).unwrap();
        assert_eq!(result3.attention.iter().find(|a| a.key == "event_soon").unwrap().count, 0);
    }

    #[test]
    fn outside_market_price_is_unavailable_without_price_checker_data_and_never_a_fake_zero() {
        let conn = test_conn();
        let event_id = seed_event(&conn, None);
        let order_id = seed_order(&conn, event_id, "2026-01-01");
        seed_ticket(&conn, event_id, order_id, None, "available", 1000, Some(9999), "EUR");
        let result = get_inventory_intelligence_impl(&conn, event_id, NaiveDate::from_ymd_opt(2026, 1, 10).unwrap()).unwrap();
        let outside_market = result.attention.iter().find(|a| a.key == "outside_market_price").unwrap();
        assert!(!outside_market.available, "no Price Checker history exists yet for this event");
        assert_eq!(outside_market.count, 0);
    }

    #[test]
    fn breakdown_by_section_groups_unsold_tickets_and_labels_blanks_as_no_section() {
        let conn = test_conn();
        let event_id = seed_event(&conn, None);
        let order_id = seed_order(&conn, event_id, "2026-01-01");
        seed_ticket(&conn, event_id, order_id, Some("A1"), "available", 1000, None, "EUR");
        seed_ticket(&conn, event_id, order_id, Some("A1"), "listed", 2000, None, "EUR");
        seed_ticket(&conn, event_id, order_id, None, "available", 500, None, "EUR");
        seed_ticket(&conn, event_id, order_id, Some("A1"), "sold", 999, None, "EUR"); // excluded (sold)

        let result = get_inventory_intelligence_impl(&conn, event_id, NaiveDate::from_ymd_opt(2026, 1, 10).unwrap()).unwrap();
        let a1 = result.breakdown_by_section.iter().find(|g| g.label == "A1").unwrap();
        assert_eq!(a1.ticket_count, 2);
        assert_eq!(a1.total_cents, 3000);
        let none_group = result.breakdown_by_section.iter().find(|g| g.label == "No section").unwrap();
        assert_eq!(none_group.ticket_count, 1);
    }

    #[test]
    fn breakdown_by_tier_groups_unsold_tickets_and_labels_blanks_as_unknown() {
        let conn = test_conn();
        let event_id = seed_event(&conn, None);
        let order_id = seed_order(&conn, event_id, "2026-01-01");
        seed_ticket_with_tier(&conn, event_id, order_id, None, Some("VIP"), "available", 1000, None, "EUR");
        seed_ticket_with_tier(&conn, event_id, order_id, None, Some("VIP"), "listed", 2000, None, "EUR");
        seed_ticket_with_tier(&conn, event_id, order_id, None, None, "available", 500, None, "EUR");
        seed_ticket_with_tier(&conn, event_id, order_id, None, Some("  "), "available", 300, None, "EUR"); // whitespace-only -> Unknown too
        seed_ticket_with_tier(&conn, event_id, order_id, None, Some("VIP"), "sold", 999, None, "EUR"); // excluded (sold)

        let result = get_inventory_intelligence_impl(&conn, event_id, NaiveDate::from_ymd_opt(2026, 1, 10).unwrap()).unwrap();
        let vip = result.breakdown_by_tier.iter().find(|g| g.label == "VIP").unwrap();
        assert_eq!(vip.ticket_count, 2);
        assert_eq!(vip.total_cents, 3000);
        let unknown_group = result.breakdown_by_tier.iter().find(|g| g.label == "Unknown").unwrap();
        assert_eq!(unknown_group.ticket_count, 2, "both the NULL tier and the whitespace-only tier land here");
        assert_eq!(
            result.breakdown_by_section.iter().find(|g| g.label == "No section").unwrap().ticket_count,
            4,
            "the tier and section breakdowns are independent - all 4 unsold tickets here have no section"
        );
    }

    #[test]
    fn breakdown_by_marketplace_groups_active_listings_only() {
        let conn = test_conn();
        let event_id = seed_event(&conn, None);
        let order_id = seed_order(&conn, event_id, "2026-01-01");
        let t1 = seed_ticket(&conn, event_id, order_id, None, "listed", 1000, None, "EUR");
        let t2 = seed_ticket(&conn, event_id, order_id, None, "listed", 1000, None, "EUR");
        let vivid = seed_marketplace(&conn, "Vivid Seats");
        let ticombo = seed_marketplace(&conn, "Ticombo");
        seed_listing(&conn, t1, vivid, 2000, "EUR", "active");
        seed_listing(&conn, t2, ticombo, 2500, "EUR", "active");
        seed_listing(&conn, t2, vivid, 2600, "EUR", "removed"); // must not count

        let result = get_inventory_intelligence_impl(&conn, event_id, NaiveDate::from_ymd_opt(2026, 1, 10).unwrap()).unwrap();
        assert_eq!(result.breakdown_by_marketplace.len(), 2);
        let vivid_group = result.breakdown_by_marketplace.iter().find(|g| g.label == "Vivid Seats").unwrap();
        assert_eq!(vivid_group.ticket_count, 1);
        assert_eq!(vivid_group.total_cents, 2000);
    }
}

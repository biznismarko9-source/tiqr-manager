//! Price Checker Live Market Monitor (2.4.1) - marko's replacement for the
//! previously-cancelled "Live Event Intelligence" direction: "Predchádzajúci
//! nápad 'Live Event Intelligence' RUŠÍME ÚPLNE" (that idea is cancelled
//! completely). Every online/live-market capability now lives directly
//! inside PRICE CHECKER instead of a separate section - see
//! migrations/026_price_checker_market_monitor.sql's own doc comment for the
//! schema and PROJECT_STATE/PROTECTED_AREAS.md for why migration number 026
//! was safely reused.
//!
//! ## What this module is, in one sentence
//! A thin, additive layer OVER the existing Visible Scanner
//! (`commands::price_checker_scanner`, 2.1.9) and Market Analysis
//! (`commands::price_checker_analysis`, 2.2.0): every time
//! `scan_visible_prices` finishes one attempt (success, partial, or
//! failure), it calls `record_scan_attempt_impl` here, which saves a
//! permanent snapshot, compares it against the previous one, and writes any
//! Market Alerts that comparison turns up. Nothing here opens a window, runs
//! a browser eval, or changes anything about what marko sees from the scan
//! itself - see `record_scan_attempt_impl`'s own doc comment for the exact
//! "never allowed to affect the scan result" contract.
//!
//! ## Reused, not reinvented (marko's own Section 2/14)
//! `commands::price_checker_analysis::{price_stats_for, group_by_tier,
//! partition_by_currency}` (which itself reuses `commands::price_checker_
//! scanner::compute_scan_stats`) are called here exactly as they already
//! exist - `partition_by_currency` is the only one of the three bumped from
//! private to `pub(crate)` for this one new caller, its own logic completely
//! untouched. This is deliberate: Tier/Level
//! grouping, "Unclassified" fallback, and per-currency splitting must behave
//! IDENTICALLY here and in the existing Market Analysis view, or marko would
//! see two different tier breakdowns for the same underlying listings
//! depending on which screen he's looking at.
//!
//! ## HARD CONSTRAINT (marko's own Section 7)
//! Tier/Level is the only grouping this module ever prices or alerts by.
//! `NormalizedListing::section`/`::row` are read by NOTHING in this file -
//! not `record_scan_attempt_impl`, not the change-detection functions, not
//! the alert messages. They stay pure metadata, exactly as marko specified;
//! `record_scan_attempt_never_disturbs_the_last_successful_snapshot_or_
//! status_offline_cache_behavior` and its sibling tests below prove this by
//! construction rather than by comment alone.
//!
//! ## No AI, no automatic repricing (marko's own Section 7/9)
//! Every threshold below is a plain, named, transparent percentage - never
//! learned/inferred. `MARKET_PRICE_CHANGE_THRESHOLD_PCT` mirrors
//! `commands::price_checker::RECOMMENDED_PRICE_UNDERCUT_PCT` (5%);
//! `MARKET_SUPPLY_CHANGE_THRESHOLD_PCT` mirrors
//! `commands::inventory_intelligence::OUTSIDE_MARKET_THRESHOLD_PCT` (20%) -
//! reusing this codebase's own existing, already-reviewed numbers rather
//! than inventing new ones out of thin air.
//!
//! ## Source-failure alerting is transition-only
//! A `source_failure` alert fires only the moment a marketplace goes from
//! "last attempt succeeded" to "this attempt failed" - never on a second,
//! third, ... consecutive failure, and never on a marketplace's very first
//! ever attempt failing (nothing was "working" yet to regress from). See
//! `record_scan_attempt_impl`'s own body for exactly how the previous
//! `market_source_status.last_scan_ok` is read BEFORE being overwritten.
//!
//! ## Attention Center integration (marko's own Section 11, and Section 15's
//! explicit "no new market-calculation logic" there)
//! `latest_active_alerts_impl` is a plain read - the single most recent
//! `market_alerts` row per (event, marketplace) pair, nothing computed or
//! decided. An old alert stops being "the latest" the instant a newer scan
//! produces any alert for that same pair, which is what makes this
//! self-expiring without a separate acknowledged/resolved flag.
//!
//! ## Cache/offline (marko's own Section 12)
//! Every read function here (`get_market_monitor_summary_impl`,
//! `list_market_snapshots_impl`) is a plain SELECT against data already on
//! disk - no network call anywhere in this file (the only network activity
//! in the whole Live Market Monitor happens inside the Visible Scanner's own
//! WebView, entirely outside this module). A run of failures only ever
//! updates `market_source_status`'s failure columns; `last_successful_
//! scan_at`/`last_successful_listing_count` and every past `market_
//! snapshots` row are structurally untouched by a failed attempt - see
//! `upsert_source_status_failure`'s own SQL for the exact columns it leaves
//! alone.

use crate::commands::price_checker_analysis::{group_by_tier, partition_by_currency, price_stats_for};
use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::models::{
    EventMarketplaceLink, Marketplace, MarketAlert, MarketMonitorMarketplaceView, MarketMonitorSummary,
    MarketSnapshot, MarketSnapshotTier, NormalizedListing, PriceStats, TierBreakdown,
};
use rusqlite::{params, Connection, OptionalExtension, Row};
use std::collections::HashMap;
use tauri::State;

/// Mirrors `commands::price_checker::RECOMMENDED_PRICE_UNDERCUT_PCT` - marko
/// never asked for a different number for Market Alerts than the one this
/// codebase already uses elsewhere for "meaningfully different", so this
/// reuses it rather than inventing a second opinion.
pub(crate) const MARKET_PRICE_CHANGE_THRESHOLD_PCT: f64 = 0.05;

/// Mirrors `commands::inventory_intelligence::OUTSIDE_MARKET_THRESHOLD_PCT` -
/// same reasoning as the price threshold above, applied to listing counts.
pub(crate) const MARKET_SUPPLY_CHANGE_THRESHOLD_PCT: f64 = 0.20;

/// How many of a marketplace's most recent alerts `get_market_monitor_
/// summary_impl` attaches per card - a small, fixed cap (not "all history",
/// which lives in `list_market_snapshots_impl`'s own paged Market History
/// view instead) so the summary call marko's Price Checker page loads on
/// every visit stays cheap regardless of how long an event has been tracked.
pub(crate) const RECENT_ALERTS_PER_MARKETPLACE: i64 = 10;

// ---------------------------------------------------------------------------
// Pure/unit-testable core - no Connection, no Tauri types. Mirrors
// commands::price_checker_analysis's own "pure core, thin DB/Tauri glue
// after" layout.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PriceChangeKind {
    Drop,
    Rise,
}

impl PriceChangeKind {
    fn alert_type(self) -> &'static str {
        match self {
            PriceChangeKind::Drop => "market_drop",
            PriceChangeKind::Rise => "market_rise",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SupplyChangeKind {
    New,
    Drop,
}

impl SupplyChangeKind {
    fn alert_type(self) -> &'static str {
        match self {
            SupplyChangeKind::New => "new_supply",
            SupplyChangeKind::Drop => "supply_drop",
        }
    }
}

/// `None` when the two prices don't differ by at least
/// `MARKET_PRICE_CHANGE_THRESHOLD_PCT` of the previous one - marko's own
/// Section 9 "transparent thresholds" requirement. `previous_cents` of 0
/// never happens in real data (`price_stats_for`/`compute_scan_stats` only
/// ever produce a `PriceStats` from a non-empty listing slice, and this
/// app's own money handling has no notion of a real ticket priced at
/// literally zero), so - unlike `detect_supply_change` below - this never
/// needs a from-zero special case; it simply reports no signal rather than
/// dividing by zero if that were ever somehow reached.
pub(crate) fn detect_price_change(previous_cents: i64, current_cents: i64) -> Option<PriceChangeKind> {
    if previous_cents == 0 {
        return None;
    }
    let change = (current_cents - previous_cents) as f64 / previous_cents as f64;
    if change.abs() < MARKET_PRICE_CHANGE_THRESHOLD_PCT {
        return None;
    }
    Some(if change < 0.0 { PriceChangeKind::Drop } else { PriceChangeKind::Rise })
}

/// Same idea as `detect_price_change`, for listing counts - marko's own
/// NEW SUPPLY / SUPPLY DROP. Two special cases the price function doesn't
/// need: no change at all is always `None` regardless of the threshold
/// (avoids a `0.0 < 0.20` no-op division), and a previous count of exactly
/// zero is always `Some(New)` when the current count is non-zero - "listings
/// appeared where there were none before" is worth surfacing every time,
/// there's no meaningful "percent of zero" to threshold against.
pub(crate) fn detect_supply_change(previous_count: i64, current_count: i64) -> Option<SupplyChangeKind> {
    if previous_count == current_count {
        return None;
    }
    if previous_count == 0 {
        return Some(SupplyChangeKind::New);
    }
    let change = (current_count - previous_count) as f64 / previous_count as f64;
    if change.abs() < MARKET_SUPPLY_CHANGE_THRESHOLD_PCT {
        return None;
    }
    Some(if change > 0.0 { SupplyChangeKind::New } else { SupplyChangeKind::Drop })
}

/// "not_connected" | "connected" | "success" | "failed" - see
/// `MarketMonitorMarketplaceView::status`'s own doc comment (models.rs) for
/// why "scanning" is deliberately never one of this function's outputs.
/// `last_scan_ok` is `None` exactly when `market_source_status` has no row
/// yet for this (event, marketplace) pair - i.e. never scanned, successfully
/// or otherwise.
pub(crate) fn derive_source_status(has_link: bool, last_scan_ok: Option<bool>) -> &'static str {
    match last_scan_ok {
        Some(true) => "success",
        Some(false) => "failed",
        None if has_link => "connected",
        None => "not_connected",
    }
}

/// Plain decimal amount plus currency code, e.g. `1234, "EUR"` -> `"12.34
/// EUR"` - the one place every alert message below turns cents into text,
/// reusing `crate::money::format_cents` (this app's one existing money-
/// formatting helper, already used for "the UI, CSV export, in-app messages
/// describing what got corrected" per its own doc comment - a Market Alert
/// message is exactly that same kind of in-app message).
fn format_cents_plain(cents: i64, currency: &str) -> String {
    format!("{} {}", crate::money::format_cents(cents), currency)
}

/// `tier: None` means the whole-market (overall) figure; `Some(name)` means
/// one specific tier's own lowest price. Fully pre-formatted, human-readable
/// text - marko's own Section 13 "no technical detail" rule applied to
/// alerts too, so the frontend never reconstructs meaning from the raw
/// previous/current columns itself (see `MarketAlert::message`'s own doc
/// comment, models.rs).
fn format_price_alert_message(kind: PriceChangeKind, tier: Option<&str>, previous_cents: i64, current_cents: i64, currency: &str) -> String {
    let verb = match kind {
        PriceChangeKind::Drop => "dropped",
        PriceChangeKind::Rise => "rose",
    };
    let pct = (((current_cents - previous_cents).abs() as f64 / previous_cents as f64) * 100.0).round() as i64;
    let scope = match tier {
        Some(t) => format!("\"{t}\" tier lowest price"),
        None => "Overall lowest market price".to_string(),
    };
    format!(
        "{scope} {verb} {pct}% ({} -> {})",
        format_cents_plain(previous_cents, currency),
        format_cents_plain(current_cents, currency)
    )
}

fn format_supply_alert_message(kind: SupplyChangeKind, tier: Option<&str>, previous_count: i64, current_count: i64) -> String {
    let scope = match tier {
        Some(t) => format!("\"{t}\" tier listing count"),
        None => "Overall market listing count".to_string(),
    };
    match kind {
        SupplyChangeKind::New => format!("{scope} increased from {previous_count} to {current_count}"),
        SupplyChangeKind::Drop => format!("{scope} dropped from {previous_count} to {current_count}"),
    }
}

/// `error_message` is already short and human-readable by the time it
/// reaches here - it comes straight from `price_checker_scanner`'s own
/// `build_status_message`/`emit_scan_error`, both of which already follow
/// marko's own "no stack traces" rule (Section 13). This function does not
/// re-sanitize it, just places it in context.
fn format_source_failure_message(marketplace_name: &str, error_message: &str) -> String {
    format!("{marketplace_name} scan failed: {error_message}")
}

// ---------------------------------------------------------------------------
// DB-backed - row mapping, then the two write paths (snapshot+alerts on
// success, source-status+alert on failure), then the read paths, then the
// thin Tauri command wrappers.
// ---------------------------------------------------------------------------

fn map_marketplace(row: &Row) -> rusqlite::Result<Marketplace> {
    Ok(Marketplace {
        id: row.get("id")?,
        name: row.get("name")?,
        active: row.get("active")?,
        is_demo: row.get("is_demo")?,
        created_at: row.get("created_at")?,
    })
}

fn map_link(row: &Row) -> rusqlite::Result<EventMarketplaceLink> {
    Ok(EventMarketplaceLink {
        id: row.get("id")?,
        event_id: row.get("event_id")?,
        marketplace_id: row.get("marketplace_id")?,
        url: row.get("url")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn map_snapshot(row: &Row) -> rusqlite::Result<MarketSnapshot> {
    Ok(MarketSnapshot {
        id: row.get("id")?,
        event_id: row.get("event_id")?,
        marketplace_id: row.get("marketplace_id")?,
        checked_at: row.get("checked_at")?,
        scan_status: row.get("scan_status")?,
        listing_count: row.get("listing_count")?,
        lowest_price_cents: row.get("lowest_price_cents")?,
        median_price_cents: row.get("median_price_cents")?,
        average_price_cents: row.get("average_price_cents")?,
        highest_price_cents: row.get("highest_price_cents")?,
        currency: row.get("currency")?,
        tiers: Vec::new(), // attached separately - see fetch_snapshot_tiers
    })
}

fn map_snapshot_tier(row: &Row) -> rusqlite::Result<MarketSnapshotTier> {
    Ok(MarketSnapshotTier {
        tier: row.get("tier_name")?,
        lowest_price_cents: row.get("lowest_price_cents")?,
        median_price_cents: row.get("median_price_cents")?,
        listing_count: row.get("listing_count")?,
    })
}

fn map_alert(row: &Row) -> rusqlite::Result<MarketAlert> {
    Ok(MarketAlert {
        id: row.get("id")?,
        event_id: row.get("event_id")?,
        marketplace_id: row.get("marketplace_id")?,
        marketplace_name: row.get("marketplace_name")?,
        alert_type: row.get("alert_type")?,
        tier: row.get("tier_name")?,
        message: row.get("message")?,
        previous_price_cents: row.get("previous_price_cents")?,
        current_price_cents: row.get("current_price_cents")?,
        previous_listing_count: row.get("previous_listing_count")?,
        current_listing_count: row.get("current_listing_count")?,
        currency: row.get("currency")?,
        created_at: row.get("created_at")?,
    })
}

fn fetch_snapshot_tiers(conn: &Connection, snapshot_id: i64) -> AppResult<Vec<MarketSnapshotTier>> {
    let mut stmt = conn.prepare(
        "SELECT * FROM market_snapshot_tiers WHERE snapshot_id = ?1 ORDER BY lowest_price_cents ASC, tier_name ASC",
    )?;
    let rows = stmt.query_map([snapshot_id], map_snapshot_tier)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

fn marketplace_name_for(conn: &Connection, marketplace_id: i64) -> AppResult<String> {
    Ok(conn
        .query_row("SELECT name FROM marketplaces WHERE id = ?1", [marketplace_id], |r| r.get(0))
        .optional()?
        .unwrap_or_else(|| "Marketplace".to_string()))
}

/// Most recent snapshot for one (event, marketplace, currency) triple, tiers
/// attached - the "previous" side of every diff in
/// `record_snapshot_and_detect_changes`. Scoped to a single currency
/// deliberately: marko's own "## CURRENCY" rule means an EUR history must
/// never be diffed against a USD one just because they happen to share an
/// event/marketplace.
fn latest_snapshot_for(conn: &Connection, event_id: i64, marketplace_id: i64, currency: &str) -> AppResult<Option<MarketSnapshot>> {
    let row: Option<MarketSnapshot> = conn
        .query_row(
            "SELECT * FROM market_snapshots WHERE event_id = ?1 AND marketplace_id = ?2 AND currency = ?3 ORDER BY id DESC LIMIT 1",
            params![event_id, marketplace_id, currency],
            map_snapshot,
        )
        .optional()?;
    match row {
        Some(mut snap) => {
            snap.tiers = fetch_snapshot_tiers(conn, snap.id)?;
            Ok(Some(snap))
        }
        None => Ok(None),
    }
}

/// Most recent snapshot for one (event, marketplace) regardless of currency -
/// what `MarketMonitorMarketplaceView::latest_snapshot` shows on the summary
/// card. A marketplace scanned in more than one currency across its history
/// simply shows whichever currency was scanned most recently here; the full
/// per-currency picture lives in `list_market_snapshots_impl`'s own history
/// view.
fn latest_snapshot_any_currency(conn: &Connection, event_id: i64, marketplace_id: i64) -> AppResult<Option<MarketSnapshot>> {
    let row: Option<MarketSnapshot> = conn
        .query_row(
            "SELECT * FROM market_snapshots WHERE event_id = ?1 AND marketplace_id = ?2 ORDER BY id DESC LIMIT 1",
            params![event_id, marketplace_id],
            map_snapshot,
        )
        .optional()?;
    match row {
        Some(mut snap) => {
            snap.tiers = fetch_snapshot_tiers(conn, snap.id)?;
            Ok(Some(snap))
        }
        None => Ok(None),
    }
}

#[allow(clippy::too_many_arguments)]
fn insert_alert(
    conn: &Connection,
    event_id: i64,
    marketplace_id: i64,
    alert_type: &str,
    tier: Option<&str>,
    message: &str,
    previous_price_cents: Option<i64>,
    current_price_cents: Option<i64>,
    previous_listing_count: Option<i64>,
    current_listing_count: Option<i64>,
    currency: Option<&str>,
    now: &str,
) -> AppResult<MarketAlert> {
    conn.execute(
        "INSERT INTO market_alerts
            (event_id, marketplace_id, alert_type, tier_name, message, previous_price_cents,
             current_price_cents, previous_listing_count, current_listing_count, currency, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            event_id,
            marketplace_id,
            alert_type,
            tier,
            message,
            previous_price_cents,
            current_price_cents,
            previous_listing_count,
            current_listing_count,
            currency,
            now
        ],
    )?;
    let id = conn.last_insert_rowid();
    Ok(MarketAlert {
        id,
        event_id,
        marketplace_id,
        marketplace_name: marketplace_name_for(conn, marketplace_id)?,
        alert_type: alert_type.to_string(),
        tier: tier.map(str::to_string),
        message: message.to_string(),
        previous_price_cents,
        current_price_cents,
        previous_listing_count,
        current_listing_count,
        currency: currency.map(str::to_string),
        created_at: now.to_string(),
    })
}

fn detect_and_insert_price(
    conn: &Connection,
    event_id: i64,
    marketplace_id: i64,
    currency: &str,
    tier: Option<&str>,
    previous_cents: i64,
    current_cents: i64,
    now: &str,
) -> AppResult<Option<MarketAlert>> {
    let kind = match detect_price_change(previous_cents, current_cents) {
        Some(k) => k,
        None => return Ok(None),
    };
    let message = format_price_alert_message(kind, tier, previous_cents, current_cents, currency);
    let alert = insert_alert(
        conn,
        event_id,
        marketplace_id,
        kind.alert_type(),
        tier,
        &message,
        Some(previous_cents),
        Some(current_cents),
        None,
        None,
        Some(currency),
        now,
    )?;
    Ok(Some(alert))
}

#[allow(clippy::too_many_arguments)]
fn detect_and_insert_supply(
    conn: &Connection,
    event_id: i64,
    marketplace_id: i64,
    currency: &str,
    tier: Option<&str>,
    previous_count: i64,
    current_count: i64,
    now: &str,
) -> AppResult<Option<MarketAlert>> {
    let kind = match detect_supply_change(previous_count, current_count) {
        Some(k) => k,
        None => return Ok(None),
    };
    let message = format_supply_alert_message(kind, tier, previous_count, current_count);
    let alert = insert_alert(
        conn,
        event_id,
        marketplace_id,
        kind.alert_type(),
        tier,
        &message,
        None,
        None,
        Some(previous_count),
        Some(current_count),
        Some(currency),
        now,
    )?;
    Ok(Some(alert))
}

/// Compares one freshly-computed snapshot (`current`/`current_tiers`)
/// against the previous one on record (`previous`) and writes every
/// resulting alert - overall (tier = NULL) first, then one pass per tier
/// NAME (case-insensitive, same key convention as `group_by_tier`) across
/// the UNION of tiers present in either snapshot, so a tier that vanished
/// entirely (present before, absent now) or appeared for the first time
/// (absent before, present now) is compared against an implicit zero on the
/// missing side - never silently skipped just because one side has no row
/// for it.
fn detect_and_record_changes(
    conn: &Connection,
    event_id: i64,
    marketplace_id: i64,
    currency: &str,
    previous: &MarketSnapshot,
    current: &PriceStats,
    current_tiers: &[TierBreakdown],
    now: &str,
) -> AppResult<Vec<MarketAlert>> {
    let mut alerts = Vec::new();

    if let Some(a) = detect_and_insert_price(conn, event_id, marketplace_id, currency, None, previous.lowest_price_cents, current.lowest_price_cents, now)? {
        alerts.push(a);
    }
    if let Some(a) = detect_and_insert_supply(conn, event_id, marketplace_id, currency, None, previous.listing_count, current.listing_count, now)? {
        alerts.push(a);
    }

    let mut prev_by_key: HashMap<String, &MarketSnapshotTier> = HashMap::new();
    for t in &previous.tiers {
        prev_by_key.insert(t.tier.to_ascii_lowercase(), t);
    }
    let mut curr_by_key: HashMap<String, &TierBreakdown> = HashMap::new();
    for t in current_tiers {
        curr_by_key.insert(t.tier.to_ascii_lowercase(), t);
    }
    let mut all_keys: Vec<String> = prev_by_key.keys().chain(curr_by_key.keys()).cloned().collect();
    all_keys.sort();
    all_keys.dedup();

    for key in all_keys {
        let prev_t = prev_by_key.get(&key).copied();
        let curr_t = curr_by_key.get(&key).copied();
        let label = curr_t.map(|t| t.tier.clone()).or_else(|| prev_t.map(|t| t.tier.clone())).unwrap_or(key);

        if let (Some(pt), Some(ct)) = (prev_t, curr_t) {
            if let Some(a) = detect_and_insert_price(conn, event_id, marketplace_id, currency, Some(&label), pt.lowest_price_cents, ct.stats.lowest_price_cents, now)? {
                alerts.push(a);
            }
        }

        // Supply is compared even when one side has no row at all for this
        // tier - "no row" reads as a count of zero, same semantics
        // `detect_supply_change`'s own from-zero branch already gives an
        // ordinary tier whose count merely dropped to zero.
        let prev_count = prev_t.map(|t| t.listing_count).unwrap_or(0);
        let curr_count = curr_t.map(|t| t.stats.listing_count).unwrap_or(0);
        if let Some(a) = detect_and_insert_supply(conn, event_id, marketplace_id, currency, Some(&label), prev_count, curr_count, now)? {
            alerts.push(a);
        }
    }

    Ok(alerts)
}

#[allow(clippy::too_many_arguments)]
fn insert_snapshot(conn: &Connection, event_id: i64, marketplace_id: i64, checked_at: &str, scan_status: &str, stats: &PriceStats, currency: &str) -> AppResult<i64> {
    conn.execute(
        "INSERT INTO market_snapshots
            (event_id, marketplace_id, checked_at, scan_status, listing_count, lowest_price_cents,
             median_price_cents, average_price_cents, highest_price_cents, currency, is_demo)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0)",
        params![
            event_id,
            marketplace_id,
            checked_at,
            scan_status,
            stats.listing_count,
            stats.lowest_price_cents,
            stats.median_price_cents,
            stats.average_price_cents,
            stats.highest_price_cents,
            currency
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

fn insert_snapshot_tier(conn: &Connection, snapshot_id: i64, tier: &TierBreakdown) -> AppResult<()> {
    conn.execute(
        "INSERT INTO market_snapshot_tiers (snapshot_id, tier_name, lowest_price_cents, median_price_cents, listing_count)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![snapshot_id, tier.tier, tier.stats.lowest_price_cents, tier.stats.median_price_cents, tier.stats.listing_count],
    )?;
    Ok(())
}

/// One successful/partial scan's worth of work: split by currency (never
/// blended, marko's own "## CURRENCY" rule - reuses `partition_by_currency`
/// exactly as `price_checker_analysis::compute_market_analysis_impl` already
/// does), save one snapshot (+ per-tier rows, via `group_by_tier`) per
/// currency actually present, and diff each against ITS OWN currency's
/// previous snapshot. A scan with zero listings at all (marko scanning a
/// genuinely sold-out page) is a real, valid "success" - there is simply
/// nothing to snapshot or compare, and no existing snapshot in any currency
/// is disturbed.
fn record_snapshot_and_detect_changes(conn: &Connection, event_id: i64, marketplace_id: i64, listings: &[NormalizedListing], scan_status: &str, now: &str) -> AppResult<Vec<MarketAlert>> {
    let mut alerts = Vec::new();
    if listings.is_empty() {
        return Ok(alerts);
    }

    let (by_currency, _uncurrencied_count) = partition_by_currency(listings);
    // Deterministic order (mostly for tests - a HashMap's own iteration
    // order is otherwise unspecified) - doesn't affect correctness, since
    // every currency's snapshot/diff is fully independent of every other.
    let mut currencies: Vec<String> = by_currency.keys().cloned().collect();
    currencies.sort();

    for currency in currencies {
        let group = &by_currency[&currency];
        let overall = match price_stats_for(group) {
            Some(s) => s,
            None => continue, // unreachable - partition_by_currency never inserts an empty group
        };
        let tiers = group_by_tier(group);

        let previous = latest_snapshot_for(conn, event_id, marketplace_id, &currency)?;

        let snapshot_id = insert_snapshot(conn, event_id, marketplace_id, now, scan_status, &overall, &currency)?;
        for t in &tiers {
            insert_snapshot_tier(conn, snapshot_id, t)?;
        }

        if let Some(prev) = previous {
            alerts.extend(detect_and_record_changes(conn, event_id, marketplace_id, &currency, &prev, &overall, &tiers, now)?);
        }
    }

    Ok(alerts)
}

fn upsert_source_status_success(conn: &Connection, event_id: i64, marketplace_id: i64, now: &str, listing_count: i64) -> AppResult<()> {
    conn.execute(
        "INSERT INTO market_source_status
            (event_id, marketplace_id, last_scan_at, last_scan_ok, last_error_message,
             last_successful_scan_at, last_successful_listing_count)
         VALUES (?1, ?2, ?3, 1, NULL, ?3, ?4)
         ON CONFLICT(event_id, marketplace_id) DO UPDATE SET
            last_scan_at = excluded.last_scan_at,
            last_scan_ok = 1,
            last_error_message = NULL,
            last_successful_scan_at = excluded.last_successful_scan_at,
            last_successful_listing_count = excluded.last_successful_listing_count",
        params![event_id, marketplace_id, now, listing_count],
    )?;
    Ok(())
}

/// Deliberately does NOT touch `last_successful_scan_at`/`last_successful_
/// listing_count` in its `ON CONFLICT` clause - marko's own Section 12
/// "always show Last successful scan": a run of failures must never erase
/// or backdate the last time this marketplace genuinely worked.
fn upsert_source_status_failure(conn: &Connection, event_id: i64, marketplace_id: i64, now: &str, error_message: Option<&str>) -> AppResult<()> {
    conn.execute(
        "INSERT INTO market_source_status
            (event_id, marketplace_id, last_scan_at, last_scan_ok, last_error_message,
             last_successful_scan_at, last_successful_listing_count)
         VALUES (?1, ?2, ?3, 0, ?4, NULL, NULL)
         ON CONFLICT(event_id, marketplace_id) DO UPDATE SET
            last_scan_at = excluded.last_scan_at,
            last_scan_ok = 0,
            last_error_message = excluded.last_error_message",
        params![event_id, marketplace_id, now, error_message],
    )?;
    Ok(())
}

/// The one hook `commands::price_checker_scanner::scan_visible_prices` (and
/// its own `emit_scan_error` failure path) calls after every completed scan
/// attempt - success, partial, or failure alike. Always called through
/// `let _ = ...` at both call sites: nothing about the `ScanResultPayload`
/// marko actually sees depends on this function succeeding, and a bug here
/// must never freeze, slow down, or error out the Visible Scanner he's
/// actively looking at (marko's own Section 15/16 "never affect the
/// existing scanner" + "one broken thing must never block another").
///
/// `scan_status` is the scanner's own already-derived status string
/// ("success"/"partial"/"blocked"/"unable_to_read"/"error") - only
/// "success"/"partial" ever produce a snapshot (mirrors the migration's own
/// `market_snapshots.scan_status` CHECK constraint); every other value is
/// treated as a failed ATTEMPT for `market_source_status` purposes, exactly
/// as that table's own migration doc comment says.
pub(crate) fn record_scan_attempt_impl(
    conn: &Connection,
    event_id: i64,
    marketplace_id: i64,
    listings: &[NormalizedListing],
    scan_status: &str,
    error_message: Option<&str>,
    now: &str,
) -> AppResult<Vec<MarketAlert>> {
    let is_success = matches!(scan_status, "success" | "partial");

    // Read the PREVIOUS attempt's own outcome before this attempt's upsert
    // overwrites it - the only way to tell a genuine transition-into-failure
    // apart from "still failing" or "never worked yet". See this function's
    // own doc comment, "Source-failure alerting is transition-only".
    let previously_ok: Option<bool> = conn
        .query_row(
            "SELECT last_scan_ok FROM market_source_status WHERE event_id = ?1 AND marketplace_id = ?2",
            params![event_id, marketplace_id],
            |r| r.get::<_, i64>(0),
        )
        .optional()?
        .map(|v| v != 0);

    let mut alerts = Vec::new();

    if is_success {
        upsert_source_status_success(conn, event_id, marketplace_id, now, listings.len() as i64)?;
        alerts.extend(record_snapshot_and_detect_changes(conn, event_id, marketplace_id, listings, scan_status, now)?);
    } else {
        upsert_source_status_failure(conn, event_id, marketplace_id, now, error_message)?;
        if previously_ok == Some(true) {
            let marketplace_name = marketplace_name_for(conn, marketplace_id)?;
            let message = format_source_failure_message(&marketplace_name, error_message.unwrap_or("The scan failed."));
            alerts.push(insert_alert(conn, event_id, marketplace_id, "source_failure", None, &message, None, None, None, None, None, now)?);
        }
    }

    Ok(alerts)
}

fn list_alerts_for(conn: &Connection, event_id: i64, marketplace_id: i64, limit: i64) -> AppResult<Vec<MarketAlert>> {
    let mut stmt = conn.prepare(
        "SELECT a.*, m.name AS marketplace_name FROM market_alerts a
         JOIN marketplaces m ON m.id = a.marketplace_id
         WHERE a.event_id = ?1 AND a.marketplace_id = ?2
         ORDER BY a.id DESC LIMIT ?3",
    )?;
    let rows = stmt.query_map(params![event_id, marketplace_id, limit], map_alert)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// The whole Live Market Monitor page for one event - marko's own Section 3:
/// per marketplace, URL/status/last successful scan/latest snapshot/recent
/// alerts, in one round trip. Marketplace selection mirrors
/// `commands::price_checker::get_price_checker_summary_impl`'s own "active
/// OR has existing history" query exactly, extended with this module's own
/// two new history tables so a marketplace that only has Market Monitor
/// history (never an old-style `price_checks` entry) still shows up.
pub(crate) fn get_market_monitor_summary_impl(conn: &Connection, event_id: i64) -> AppResult<MarketMonitorSummary> {
    let event_exists: bool = conn.query_row("SELECT COUNT(*) FROM events WHERE id = ?1", [event_id], |r| r.get::<_, i64>(0)).map(|c| c > 0)?;
    if !event_exists {
        return Err(AppError::NotFound(format!("Event #{event_id} not found")));
    }

    let marketplaces: Vec<Marketplace> = {
        let mut stmt = conn.prepare(
            "SELECT * FROM marketplaces
             WHERE active = 1
                OR EXISTS(SELECT 1 FROM event_marketplace_links WHERE event_id = ?1 AND marketplace_id = marketplaces.id)
                OR EXISTS(SELECT 1 FROM price_checks WHERE event_id = ?1 AND marketplace_id = marketplaces.id)
                OR EXISTS(SELECT 1 FROM market_snapshots WHERE event_id = ?1 AND marketplace_id = marketplaces.id)
                OR EXISTS(SELECT 1 FROM market_source_status WHERE event_id = ?1 AND marketplace_id = marketplaces.id)
             ORDER BY name COLLATE NOCASE",
        )?;
        let rows = stmt.query_map([event_id], map_marketplace)?;
        rows.collect::<Result<Vec<_>, _>>()?
    };

    let mut views: Vec<MarketMonitorMarketplaceView> = Vec::with_capacity(marketplaces.len());
    for m in &marketplaces {
        let link: Option<EventMarketplaceLink> = conn
            .query_row("SELECT * FROM event_marketplace_links WHERE event_id = ?1 AND marketplace_id = ?2", params![event_id, m.id], map_link)
            .optional()?;

        let status_row: Option<(String, bool, Option<String>, Option<String>)> = conn
            .query_row(
                "SELECT last_scan_at, last_scan_ok, last_error_message, last_successful_scan_at
                 FROM market_source_status WHERE event_id = ?1 AND marketplace_id = ?2",
                params![event_id, m.id],
                |r| Ok((r.get(0)?, r.get::<_, i64>(1)? != 0, r.get(2)?, r.get(3)?)),
            )
            .optional()?;

        let last_scan_ok = status_row.as_ref().map(|(_, ok, _, _)| *ok);
        let status = derive_source_status(link.is_some(), last_scan_ok);

        views.push(MarketMonitorMarketplaceView {
            marketplace_id: m.id,
            marketplace_name: m.name.clone(),
            marketplace_active: m.active,
            link,
            status: status.to_string(),
            last_scan_at: status_row.as_ref().map(|(at, _, _, _)| at.clone()),
            last_successful_scan_at: status_row.as_ref().and_then(|(_, _, _, s)| s.clone()),
            last_error_message: status_row.as_ref().and_then(|(_, _, e, _)| e.clone()),
            latest_snapshot: latest_snapshot_any_currency(conn, event_id, m.id)?,
            recent_alerts: list_alerts_for(conn, event_id, m.id, RECENT_ALERTS_PER_MARKETPLACE)?,
        });
    }

    Ok(MarketMonitorSummary { event_id, marketplaces: views })
}

/// Paged Market History for one (event, marketplace) - marko's own Section
/// 8, newest first, tiers attached to each. Deliberately not scoped to a
/// single currency (unlike `latest_snapshot_for`): a marketplace's full
/// history across every currency it's ever been scanned in is exactly what
/// a "show me the trend" view should return; the frontend can filter by
/// currency for display if a marketplace ever has more than one.
pub(crate) fn list_market_snapshots_impl(conn: &Connection, event_id: i64, marketplace_id: i64, limit: i64) -> AppResult<Vec<MarketSnapshot>> {
    let mut stmt = conn.prepare("SELECT * FROM market_snapshots WHERE event_id = ?1 AND marketplace_id = ?2 ORDER BY id DESC LIMIT ?3")?;
    let rows = stmt.query_map(params![event_id, marketplace_id, limit], map_snapshot)?;
    let mut snapshots = rows.collect::<Result<Vec<_>, _>>()?;
    for s in snapshots.iter_mut() {
        s.tiers = fetch_snapshot_tiers(conn, s.id)?;
    }
    Ok(snapshots)
}

/// `commands::attention_center`'s own pure read for its "MARKET ATTENTION"
/// category (Section 11) - the single most recent alert per (event,
/// marketplace) pair, across the whole app, newest first. Zero decision-
/// making happens here beyond "which row is newest for this pair", per
/// Section 15's explicit "no new market-calculation logic in Attention
/// Center" requirement - Price Checker (specifically, `record_scan_attempt_
/// impl` above) remains the sole place any alert is ever decided or
/// created.
pub(crate) fn latest_active_alerts_impl(conn: &Connection) -> AppResult<Vec<MarketAlert>> {
    let mut stmt = conn.prepare(
        "SELECT a.*, m.name AS marketplace_name FROM market_alerts a
         JOIN marketplaces m ON m.id = a.marketplace_id
         WHERE a.id IN (SELECT MAX(id) FROM market_alerts GROUP BY event_id, marketplace_id)
         ORDER BY a.id DESC",
    )?;
    let rows = stmt.query_map([], map_alert)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[tauri::command]
pub fn get_market_monitor_summary(state: State<AppState>, event_id: i64) -> AppResult<MarketMonitorSummary> {
    let conn = state.db.lock().unwrap();
    get_market_monitor_summary_impl(&conn, event_id)
}

#[tauri::command]
pub fn list_market_snapshots(state: State<AppState>, event_id: i64, marketplace_id: i64, limit: i64) -> AppResult<Vec<MarketSnapshot>> {
    let conn = state.db.lock().unwrap();
    list_market_snapshots_impl(&conn, event_id, marketplace_id, limit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_conn;

    // -- pure function tests -------------------------------------------------

    #[test]
    fn detect_price_change_ignores_moves_under_the_threshold() {
        assert_eq!(detect_price_change(1000, 970), None); // -3%
        assert_eq!(detect_price_change(1000, 1030), None); // +3%
    }

    #[test]
    fn detect_price_change_detects_a_drop_at_exactly_the_threshold() {
        assert_eq!(detect_price_change(1000, 950), Some(PriceChangeKind::Drop)); // exactly -5%
    }

    #[test]
    fn detect_price_change_detects_a_rise_at_exactly_the_threshold() {
        assert_eq!(detect_price_change(1000, 1050), Some(PriceChangeKind::Rise)); // exactly +5%
    }

    #[test]
    fn detect_price_change_never_divides_by_a_zero_previous_price() {
        assert_eq!(detect_price_change(0, 500), None);
    }

    #[test]
    fn detect_supply_change_ignores_moves_under_the_threshold() {
        assert_eq!(detect_supply_change(10, 9), None); // -10%
    }

    #[test]
    fn detect_supply_change_detects_a_drop_at_exactly_the_threshold() {
        assert_eq!(detect_supply_change(10, 8), Some(SupplyChangeKind::Drop)); // exactly -20%
    }

    #[test]
    fn detect_supply_change_detects_a_rise_at_exactly_the_threshold() {
        assert_eq!(detect_supply_change(10, 12), Some(SupplyChangeKind::New)); // exactly +20%
    }

    #[test]
    fn detect_supply_change_treats_zero_to_nonzero_as_new_supply_regardless_of_size() {
        assert_eq!(detect_supply_change(0, 1), Some(SupplyChangeKind::New));
        assert_eq!(detect_supply_change(0, 500), Some(SupplyChangeKind::New));
    }

    #[test]
    fn detect_supply_change_treats_no_change_as_no_change() {
        assert_eq!(detect_supply_change(0, 0), None);
        assert_eq!(detect_supply_change(7, 7), None);
    }

    #[test]
    fn detect_supply_change_detects_a_full_drop_to_zero() {
        assert_eq!(detect_supply_change(5, 0), Some(SupplyChangeKind::Drop));
    }

    #[test]
    fn derive_source_status_matches_the_four_documented_cases() {
        assert_eq!(derive_source_status(false, None), "not_connected");
        assert_eq!(derive_source_status(true, None), "connected");
        assert_eq!(derive_source_status(true, Some(true)), "success");
        assert_eq!(derive_source_status(true, Some(false)), "failed");
        // last_scan_ok always implies a link was saved in practice (a scan
        // needs a URL to open), but the derivation must not depend on that -
        // last_scan_ok alone is authoritative once it exists at all.
        assert_eq!(derive_source_status(false, Some(true)), "success");
    }

    // -- DB-backed integration tests -----------------------------------------

    fn seed_event(conn: &Connection, name: &str) -> i64 {
        conn.execute("INSERT INTO events (name) VALUES (?1)", [name]).unwrap();
        conn.last_insert_rowid()
    }

    fn seed_marketplace(conn: &Connection, name: &str) -> i64 {
        conn.execute("INSERT INTO marketplaces (name) VALUES (?1)", [name]).unwrap();
        conn.last_insert_rowid()
    }

    fn nl(price_cents: i64, currency: &str, tier: Option<&str>, section: Option<&str>, row: Option<&str>) -> NormalizedListing {
        NormalizedListing {
            price_cents,
            currency: Some(currency.to_string()),
            section: section.map(str::to_string),
            row: row.map(str::to_string),
            tier: tier.map(str::to_string),
            quantity: None,
            listing_id: None,
            marketplace: "generic".to_string(),
        }
    }

    #[test]
    fn record_scan_attempt_creates_a_snapshot_with_tier_breakdown_reusing_group_by_tier() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Case-insensitive tiers");
        let marketplace_id = seed_marketplace(&conn, "Test Marketplace A");
        let listings = vec![
            nl(1000, "EUR", Some("Floor"), Some("A1"), None),
            nl(1200, "EUR", Some("FLOOR"), Some("A2"), None),
            nl(2000, "EUR", None, None, None), // no tier at all -> Unclassified
        ];

        let alerts = record_scan_attempt_impl(&conn, event_id, marketplace_id, &listings, "success", None, "2026-01-01T00:00:00.000Z").unwrap();
        assert!(alerts.is_empty(), "no previous snapshot exists yet, so there's nothing to diff against");

        let snapshot = latest_snapshot_any_currency(&conn, event_id, marketplace_id).unwrap().expect("a snapshot must have been created");
        assert_eq!(snapshot.listing_count, 3);
        assert_eq!(snapshot.currency, "EUR");
        assert_eq!(snapshot.tiers.len(), 2, "Floor/FLOOR must collapse into one case-insensitive tier row");
        let floor = snapshot.tiers.iter().find(|t| t.tier.eq_ignore_ascii_case("floor")).unwrap();
        assert_eq!(floor.listing_count, 2);
        let unclassified = snapshot.tiers.iter().find(|t| t.tier == "Unclassified").unwrap();
        assert_eq!(unclassified.listing_count, 1);
    }

    #[test]
    fn record_scan_attempt_splits_mixed_currency_listings_into_separate_snapshots() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Mixed currency");
        let marketplace_id = seed_marketplace(&conn, "Test Marketplace B");
        let listings = vec![nl(1000, "EUR", Some("A"), None, None), nl(2000, "USD", Some("A"), None, None)];

        record_scan_attempt_impl(&conn, event_id, marketplace_id, &listings, "success", None, "2026-01-01T00:00:00.000Z").unwrap();

        let eur = latest_snapshot_for(&conn, event_id, marketplace_id, "EUR").unwrap().expect("EUR snapshot");
        let usd = latest_snapshot_for(&conn, event_id, marketplace_id, "USD").unwrap().expect("USD snapshot");
        assert_eq!(eur.listing_count, 1);
        assert_eq!(usd.listing_count, 1);
        assert_ne!(eur.id, usd.id, "EUR and USD must never share or blend into a single snapshot row");
    }

    #[test]
    fn record_scan_attempt_never_overwrites_a_previous_snapshot_multiple_snapshots_accumulate() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "History");
        let marketplace_id = seed_marketplace(&conn, "Test Marketplace C");

        record_scan_attempt_impl(&conn, event_id, marketplace_id, &[nl(1000, "EUR", Some("A"), None, None)], "success", None, "2026-01-01T00:00:00.000Z").unwrap();
        record_scan_attempt_impl(&conn, event_id, marketplace_id, &[nl(1100, "EUR", Some("A"), None, None)], "success", None, "2026-01-02T00:00:00.000Z").unwrap();

        let history = list_market_snapshots_impl(&conn, event_id, marketplace_id, 10).unwrap();
        assert_eq!(history.len(), 2, "both scans must remain as real, separate history rows");
        assert_eq!(history[0].lowest_price_cents, 1100, "newest first");
        assert_eq!(history[1].lowest_price_cents, 1000);
    }

    #[test]
    fn record_scan_attempt_detects_an_overall_price_drop_between_two_scans() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Price drop");
        let marketplace_id = seed_marketplace(&conn, "Test Marketplace A");

        record_scan_attempt_impl(&conn, event_id, marketplace_id, &[nl(1000, "EUR", Some("A"), None, None)], "success", None, "2026-01-01T00:00:00.000Z").unwrap();
        let alerts = record_scan_attempt_impl(&conn, event_id, marketplace_id, &[nl(900, "EUR", Some("A"), None, None)], "success", None, "2026-01-02T00:00:00.000Z").unwrap();

        let overall_drop = alerts.iter().find(|a| a.alert_type == "market_drop" && a.tier.is_none()).expect("an overall market_drop alert");
        assert_eq!(overall_drop.previous_price_cents, Some(1000));
        assert_eq!(overall_drop.current_price_cents, Some(900));
        assert_eq!(overall_drop.currency.as_deref(), Some("EUR"));
        assert!(!overall_drop.message.is_empty());
    }

    #[test]
    fn record_scan_attempt_detects_a_per_tier_supply_increase_as_new_supply() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Tier supply increase");
        let marketplace_id = seed_marketplace(&conn, "Test Marketplace A");

        record_scan_attempt_impl(&conn, event_id, marketplace_id, &[nl(1000, "EUR", Some("Floor"), None, None)], "success", None, "2026-01-01T00:00:00.000Z").unwrap();
        let five_floor: Vec<NormalizedListing> = (0..5).map(|_| nl(1000, "EUR", Some("Floor"), None, None)).collect();
        let alerts = record_scan_attempt_impl(&conn, event_id, marketplace_id, &five_floor, "success", None, "2026-01-02T00:00:00.000Z").unwrap();

        let tier_alert = alerts.iter().find(|a| a.alert_type == "new_supply" && a.tier.as_deref() == Some("Floor")).expect("a per-tier new_supply alert for Floor");
        assert_eq!(tier_alert.previous_listing_count, Some(1));
        assert_eq!(tier_alert.current_listing_count, Some(5));
    }

    #[test]
    fn record_scan_attempt_ignores_section_and_row_changes_entirely() {
        // marko's own HARD CONSTRAINT (Section 7): Section/Row are metadata
        // only. Two scans with the SAME price/tier but COMPLETELY DIFFERENT
        // section/row values must never produce any alert.
        let conn = test_conn();
        let event_id = seed_event(&conn, "Section is not a pricing factor");
        let marketplace_id = seed_marketplace(&conn, "Test Marketplace A");

        record_scan_attempt_impl(&conn, event_id, marketplace_id, &[nl(1000, "EUR", Some("Floor"), Some("Section 12"), Some("Row A"))], "success", None, "2026-01-01T00:00:00.000Z").unwrap();
        let alerts = record_scan_attempt_impl(&conn, event_id, marketplace_id, &[nl(1000, "EUR", Some("Floor"), Some("Section 99"), Some("Row Z"))], "success", None, "2026-01-02T00:00:00.000Z").unwrap();

        assert!(alerts.is_empty(), "changing only section/row, with the same tier and price, must never trigger a Market Alert");
    }

    #[test]
    fn record_scan_attempt_with_zero_listings_is_a_valid_success_and_creates_no_snapshot() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Sold out");
        let marketplace_id = seed_marketplace(&conn, "Test Marketplace A");

        let alerts = record_scan_attempt_impl(&conn, event_id, marketplace_id, &[], "success", None, "2026-01-01T00:00:00.000Z").unwrap();
        assert!(alerts.is_empty());
        assert!(latest_snapshot_any_currency(&conn, event_id, marketplace_id).unwrap().is_none());

        // But the source status must still record that the scan itself
        // succeeded - "zero listings found" is not the same as "the scan
        // failed" (Section 13).
        let summary = get_market_monitor_summary_impl(&conn, event_id).unwrap();
        let view = summary.marketplaces.iter().find(|v| v.marketplace_id == marketplace_id).unwrap();
        assert_eq!(view.status, "success");
    }

    #[test]
    fn record_scan_attempt_marks_source_status_success_and_advances_last_successful_scan_at() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Source status");
        let marketplace_id = seed_marketplace(&conn, "Test Marketplace A");

        record_scan_attempt_impl(&conn, event_id, marketplace_id, &[nl(1000, "EUR", Some("A"), None, None)], "success", None, "2026-01-01T00:00:00.000Z").unwrap();

        let summary = get_market_monitor_summary_impl(&conn, event_id).unwrap();
        let view = summary.marketplaces.iter().find(|v| v.marketplace_id == marketplace_id).unwrap();
        assert_eq!(view.status, "success");
        assert_eq!(view.last_scan_at.as_deref(), Some("2026-01-01T00:00:00.000Z"));
        assert_eq!(view.last_successful_scan_at.as_deref(), Some("2026-01-01T00:00:00.000Z"));
        assert!(view.last_error_message.is_none());
    }

    #[test]
    fn record_scan_attempt_failure_after_a_success_fires_a_source_failure_alert_only_on_the_transition() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Transition-only failure");
        let marketplace_id = seed_marketplace(&conn, "Test Marketplace A");

        record_scan_attempt_impl(&conn, event_id, marketplace_id, &[nl(1000, "EUR", Some("A"), None, None)], "success", None, "2026-01-01T00:00:00.000Z").unwrap();

        let first_failure = record_scan_attempt_impl(&conn, event_id, marketplace_id, &[], "error", Some("The page didn't respond to the scan in time."), "2026-01-02T00:00:00.000Z").unwrap();
        assert_eq!(first_failure.len(), 1);
        assert_eq!(first_failure[0].alert_type, "source_failure");
        assert!(first_failure[0].message.contains("didn't respond"));

        let second_failure = record_scan_attempt_impl(&conn, event_id, marketplace_id, &[], "error", Some("Still failing."), "2026-01-03T00:00:00.000Z").unwrap();
        assert!(second_failure.is_empty(), "a second consecutive failure must not fire another source_failure alert");
    }

    #[test]
    fn record_scan_attempt_failure_as_the_very_first_attempt_never_fires_a_source_failure_alert() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Never worked yet");
        let marketplace_id = seed_marketplace(&conn, "Test Marketplace A");

        let alerts = record_scan_attempt_impl(&conn, event_id, marketplace_id, &[], "blocked", Some("Looks like a CAPTCHA."), "2026-01-01T00:00:00.000Z").unwrap();
        assert!(alerts.is_empty(), "nothing was ever working before, so there is no regression to alert about");

        let summary = get_market_monitor_summary_impl(&conn, event_id).unwrap();
        let view = summary.marketplaces.iter().find(|v| v.marketplace_id == marketplace_id).unwrap();
        assert_eq!(view.status, "failed");
        assert_eq!(view.last_error_message.as_deref(), Some("Looks like a CAPTCHA."));
        assert!(view.last_successful_scan_at.is_none());
    }

    #[test]
    fn record_scan_attempt_failure_never_disturbs_the_last_successful_snapshot_or_status_offline_cache_behavior() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Offline cache");
        let marketplace_id = seed_marketplace(&conn, "Test Marketplace A");

        record_scan_attempt_impl(&conn, event_id, marketplace_id, &[nl(1000, "EUR", Some("A"), None, None)], "success", None, "2026-01-01T00:00:00.000Z").unwrap();
        record_scan_attempt_impl(&conn, event_id, marketplace_id, &[], "error", Some("Network unreachable."), "2026-01-02T00:00:00.000Z").unwrap();

        let summary = get_market_monitor_summary_impl(&conn, event_id).unwrap();
        let view = summary.marketplaces.iter().find(|v| v.marketplace_id == marketplace_id).unwrap();
        assert_eq!(view.status, "failed", "the latest ATTEMPT failed");
        assert_eq!(view.last_successful_scan_at.as_deref(), Some("2026-01-01T00:00:00.000Z"), "but the last SUCCESS must stay exactly where it was");
        assert!(view.latest_snapshot.is_some(), "the old snapshot must still be there, completely undisturbed by the failed attempt");
        assert_eq!(view.latest_snapshot.as_ref().unwrap().lowest_price_cents, 1000);
    }

    #[test]
    fn record_scan_attempt_for_one_marketplace_never_touches_another_marketplaces_status_or_snapshots() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Independence");
        let marketplace_a = seed_marketplace(&conn, "Test Marketplace A");
        let marketplace_b = seed_marketplace(&conn, "Test Marketplace B");

        record_scan_attempt_impl(&conn, event_id, marketplace_b, &[nl(2000, "EUR", Some("A"), None, None)], "success", None, "2026-01-01T00:00:00.000Z").unwrap();
        record_scan_attempt_impl(&conn, event_id, marketplace_a, &[], "error", Some("Blocked."), "2026-01-02T00:00:00.000Z").unwrap();

        let summary = get_market_monitor_summary_impl(&conn, event_id).unwrap();
        let view_a = summary.marketplaces.iter().find(|v| v.marketplace_id == marketplace_a).unwrap();
        let view_b = summary.marketplaces.iter().find(|v| v.marketplace_id == marketplace_b).unwrap();
        assert_eq!(view_a.status, "failed");
        assert_eq!(view_b.status, "success", "marketplace B must be completely unaffected by marketplace A's failure");
        assert!(view_b.latest_snapshot.is_some());
    }

    #[test]
    fn get_market_monitor_summary_impl_includes_a_marketplace_with_only_snapshot_history_and_no_active_link() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "History without a link");
        conn.execute("INSERT INTO marketplaces (name, active) VALUES ('Retired', 0)", []).unwrap();
        let marketplace_id = conn.last_insert_rowid();

        record_scan_attempt_impl(&conn, event_id, marketplace_id, &[nl(500, "EUR", Some("A"), None, None)], "success", None, "2026-01-01T00:00:00.000Z").unwrap();

        let summary = get_market_monitor_summary_impl(&conn, event_id).unwrap();
        assert!(summary.marketplaces.iter().any(|v| v.marketplace_id == marketplace_id), "an inactive marketplace with real Market Monitor history must still appear");
    }

    #[test]
    fn get_market_monitor_summary_impl_rejects_a_missing_event() {
        let conn = test_conn();
        let err = get_market_monitor_summary_impl(&conn, 999999).unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[test]
    fn list_market_snapshots_impl_returns_newest_first_with_tiers_attached() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "History view");
        let marketplace_id = seed_marketplace(&conn, "Test Marketplace A");

        record_scan_attempt_impl(&conn, event_id, marketplace_id, &[nl(1000, "EUR", Some("Floor"), None, None)], "success", None, "2026-01-01T00:00:00.000Z").unwrap();
        record_scan_attempt_impl(&conn, event_id, marketplace_id, &[nl(1200, "EUR", Some("Floor"), None, None)], "partial", None, "2026-01-02T00:00:00.000Z").unwrap();

        let history = list_market_snapshots_impl(&conn, event_id, marketplace_id, 10).unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].scan_status, "partial");
        assert_eq!(history[0].tiers.len(), 1);
        assert_eq!(history[0].tiers[0].tier, "Floor");
    }

    #[test]
    fn latest_active_alerts_impl_returns_only_the_most_recent_alert_per_event_marketplace_pair() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Attention Center feed");
        let marketplace_id = seed_marketplace(&conn, "Test Marketplace A");

        record_scan_attempt_impl(&conn, event_id, marketplace_id, &[nl(1000, "EUR", Some("A"), None, None)], "success", None, "2026-01-01T00:00:00.000Z").unwrap();
        record_scan_attempt_impl(&conn, event_id, marketplace_id, &[nl(900, "EUR", Some("A"), None, None)], "success", None, "2026-01-02T00:00:00.000Z").unwrap(); // -10%, clears the threshold
        record_scan_attempt_impl(&conn, event_id, marketplace_id, &[nl(800, "EUR", Some("A"), None, None)], "success", None, "2026-01-03T00:00:00.000Z").unwrap(); // -11.1% vs 900, clears it again

        let active = latest_active_alerts_impl(&conn).unwrap();
        let for_this_pair: Vec<_> = active.iter().filter(|a| a.event_id == event_id && a.marketplace_id == marketplace_id).collect();
        assert_eq!(for_this_pair.len(), 1, "only the single most recent alert for this (event, marketplace) pair, never the older one too");
        assert_eq!(for_this_pair[0].previous_price_cents, Some(900));
        assert_eq!(for_this_pair[0].current_price_cents, Some(800));
    }
}

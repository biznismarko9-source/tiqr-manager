//! Price Checker Market Analysis (2.2.0) - marko's own "# PRICE CHECKER —
//! MARKET ANALYSIS 2.2" spec, built entirely on top of the Visible Scanner
//! (`commands::price_checker_scanner`, 2.1.9): "Nechcem meniť existujúci
//! scanner/lifecycle, iba naň nadviazať" (I don't want to change the
//! existing scanner/lifecycle, just build on top of it). Nothing here opens
//! a window, runs an eval, or touches `ScannerSession`'s own fields beyond
//! reading `listings` - this module is a pure/DB-read layer over whatever
//! the scanner has already accumulated.
//!
//! ## What this module answers
//! Two questions, two commands:
//! - `compute_market_analysis`: "what does the WHOLE scanned market look
//!   like for this session, and what should I list MY unsold tickets for?"
//!   - tier/section breakdown + stats (marko's "## TIER PRICING" + "## MAP /
//!     SECTION ANALYSIS"), split per currency (never blended - "##
//!     CURRENCY"), plus a `your_tickets` panel reusing the real `tickets`
//!     table (marko's "## YOUR TICKETS" + "## PRICE RECOMMENDATION").
//! - `compute_comparable_market`: "how does the market compare to THIS ONE
//!   reference ticket I have in mind?" (marko's "## COMPARABLE MARKET",
//!   worked example: Section 112 / Row 8 / Quantity 4).
//!
//! ## Two honest, independent classifications (read this before touching
//! either one)
//! `data_quality_for` and `classify_comparable` both look at a single
//! `NormalizedListing`, but answer different questions and must never gate
//! each other:
//! - `data_quality_for` - "how much structured data does THIS listing carry,
//!   in absolute terms" - marko's own "## DATA QUALITY" precedence, checked
//!   strongest-first: section+row+quantity -> "strong_comparable";
//!   section+tier -> "section_comparable"; tier alone -> "tier_comparable";
//!   else "partial". Notably, a listing with a section but NO tier is only
//!   "partial" by this literal precedence (surprising at first glance, but
//!   correct: without a tier it cannot be placed in the tier/section
//!   breakdown this feature is built around - see `group_by_tier`).
//! - `classify_comparable` - "how well does THIS listing match ONE SPECIFIC
//!   reference ticket" - marko's own literal priority list ("same section,
//!   same tier, nearby sections in same tier, same quantity, nearby rows").
//!   Section match is checked FIRST, unconditionally - so a listing can be
//!   `"exact_comparable"` while its own `data_quality` is still `"partial"`.
//!   This is deliberate, not a bug: see `RankedComparable`'s own doc comment
//!   (models.rs) for why an earlier draft that gated one on the other was
//!   wrong and got corrected before anything depended on it.
//!
//! ## Currency safety
//! Every function below that produces a single lowest/median/average number
//! operates on a slice that is ALREADY scoped to one currency by its
//! caller, this module never sums or averages across currencies itself
//! (marko's own "## CURRENCY": "EUR + USD + GBP nikdy nesčítavaj").
//! `partition_by_currency` is the one place mixed-currency input gets
//! split; everything downstream of it just trusts its input is already
//! single-currency. `ComparableReferenceInput::currency` (models.rs) being
//! a required, not optional, field is the same rule applied to the
//! comparable-ticket flow; see that field's own doc comment for why that's
//! a considered addition, not something marko's spec spelled out verbatim.
//!
//! ## Performance (marko's own "## PERFORMANCE")
//! Every function here takes an already-fetched `&[NormalizedListing]` (the
//! scanner's own in-memory session state) or a single `tickets` query - never
//! a browser eval. `compute_market_analysis` reads the session once, then
//! every stat/group/recommendation is derived from that one snapshot in
//! memory; `compute_your_tickets` reuses `by_currency`'s already-computed
//! `PriceStats` for each ticket group's recommendation rather than
//! recomputing market stats a second time per group.
//!
//! ## What's reused, not reinvented
//! `commands::price_checker_scanner::{compute_scan_stats, median_of_sorted_
//! cents}` (made `pub(crate)` for exactly this reuse) and `commands::price_
//! checker::RECOMMENDED_PRICE_UNDERCUT_PCT` (the same "5% under the lowest"
//! formula the manual/history-based Price Checker summary already uses) -
//! see each one's own doc comment for why duplicating either would have been
//! wrong.
//!
//! See PRICE-CHECKER-MARKET-ANALYSIS-2.2-REPORT.md for what's verified vs.
//! derived-but-unverified vs. genuinely unavailable (in particular: `tickets`
//! has no tier/level column at all, so `YourTicketGroup::tier` is always
//! `None` today - see that field's own doc comment, models.rs).

use crate::commands::price_checker::RECOMMENDED_PRICE_UNDERCUT_PCT;
use crate::commands::price_checker_scanner::{compute_scan_stats, median_of_sorted_cents};
use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::finance;
use crate::models::{
    ComparableReferenceInput, CurrencyMarketAnalysis, MarketAnalysisResult, NormalizedListing, PriceRecommendation,
    PriceStats, RankedComparable, SectionBreakdown, TierBreakdown, YourTicketGroup,
};
use rusqlite::Connection;
use std::collections::HashMap;
use tauri::State;

/// The literal string every listing/tier group with no usable tier label
/// collapses to - marko's own spec, "## TIER PRICING": every listing must
/// still show up SOMEWHERE, never silently dropped just because `tierFor`
/// (price_checker_scan.js) couldn't confidently place it. The one and only
/// place this string is ever written - never guessed further upstream (see
/// `NormalizedListing::tier`'s own doc comment, models.rs).
const UNCLASSIFIED_TIER: &str = "Unclassified";

/// Bounded numeric-proximity threshold for "nearby section"/"nearby row" -
/// marko's own spec asks for "nearby sections in same tier"/"nearby rows"
/// without defining exactly how near. A reasoned, bounded heuristic - NOT
/// verified against real marketplace markup, same honesty this codebase
/// already applies to its own per-marketplace selectors (see
/// price_checker_scan.js's own module comment) - flagged in
/// PRICE-CHECKER-MARKET-ANALYSIS-2.2-REPORT.md rather than presented as
/// exact.
const NEARBY_NUMERIC_THRESHOLD: i64 = 3;

/// The 4 comparable levels, most-specific first - marko's own spec priority
/// order for "## COMPARABLE MARKET". One shared list so `rank_comparable`'s
/// sort and `recommend_price`'s pool selection can never silently drift
/// apart from each other.
const COMPARABLE_LEVELS_BY_PRIORITY: [&str; 4] =
    ["exact_comparable", "close_comparable", "tier_comparable", "general_market"];

// ---------------------------------------------------------------------------
// Pure/unit-testable core - no Tauri types, no I/O except where a `tickets`
// query is unavoidable (compute_your_tickets). Mirrors
// commands::price_checker_scanner's own "pure core, thin Tauri glue after"
// layout.
// ---------------------------------------------------------------------------

fn has_text(v: Option<&str>) -> bool {
    v.map(|s| !s.trim().is_empty()).unwrap_or(false)
}

/// Case/whitespace-insensitive equality for two label fields (section, tier,
/// ...) - `None` on either side, or either side blank after trimming, is
/// never "the same" (nothing to honestly compare).
fn same_str(a: Option<&str>, b: Option<&str>) -> bool {
    match (a, b) {
        (Some(x), Some(y)) => {
            let (x, y) = (x.trim(), y.trim());
            !x.is_empty() && x.eq_ignore_ascii_case(y)
        }
        _ => false,
    }
}

/// The trailing run of ASCII digits in a label, e.g. `112` from
/// `"Section 112"` or `8` from `"Row 8"` - `None` when the label has no
/// trailing digits at all (nothing to compare numerically).
fn trailing_digits(s: &str) -> Option<i64> {
    let digits: String = s.chars().rev().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    digits.chars().rev().collect::<String>().parse::<i64>().ok()
}

/// See `NEARBY_NUMERIC_THRESHOLD`'s own doc comment. `false` whenever either
/// side is missing or has no trailing digits to compare - never guessed.
fn nearby_numeric(a: Option<&str>, b: Option<&str>) -> bool {
    match (a.and_then(trailing_digits), b.and_then(trailing_digits)) {
        (Some(x), Some(y)) => (x - y).abs() <= NEARBY_NUMERIC_THRESHOLD,
        _ => false,
    }
}

/// marko's own spec, "## DATA QUALITY" - see this module's own doc comment
/// for why this is deliberately independent of `classify_comparable`.
pub(crate) fn data_quality_for(listing: &NormalizedListing) -> &'static str {
    let has_section = has_text(listing.section.as_deref());
    let has_row = has_text(listing.row.as_deref());
    let has_tier = has_text(listing.tier.as_deref());
    let has_quantity = listing.quantity.is_some();

    if has_section && has_row && has_quantity {
        "strong_comparable"
    } else if has_section && has_tier {
        "section_comparable"
    } else if has_tier {
        "tier_comparable"
    } else {
        "partial"
    }
}

/// marko's own literal priority list for "## COMPARABLE MARKET": "same
/// section, same tier, nearby sections in same tier, same quantity, nearby
/// rows". Checked in exactly that order; the first match wins.
pub(crate) fn classify_comparable(listing: &NormalizedListing, reference: &ComparableReferenceInput) -> &'static str {
    if same_str(listing.section.as_deref(), reference.section.as_deref()) {
        return "exact_comparable";
    }
    if same_str(listing.tier.as_deref(), reference.tier.as_deref()) {
        let close = nearby_numeric(listing.section.as_deref(), reference.section.as_deref())
            || (reference.quantity.is_some() && listing.quantity == reference.quantity)
            || nearby_numeric(listing.row.as_deref(), reference.row.as_deref());
        return if close { "close_comparable" } else { "tier_comparable" };
    }
    "general_market"
}

/// Ranks every listing in `reference.currency` against `reference` - marko's
/// own spec, "## COMPARABLE MARKET". Listings in any OTHER currency are
/// silently excluded (never blended - see this module's own doc comment on
/// currency safety), not reported as a separate "wrong currency" bucket,
/// since `reference.currency` already tells the caller which one currency
/// this whole ranking is scoped to. Sorted most-specific-and-cheapest first:
/// `COMPARABLE_LEVELS_BY_PRIORITY` order, then ascending price within a
/// level.
pub(crate) fn rank_comparable(listings: &[NormalizedListing], reference: &ComparableReferenceInput) -> Vec<RankedComparable> {
    let mut ranked: Vec<RankedComparable> = listings
        .iter()
        .filter(|l| l.currency.as_deref() == Some(reference.currency.as_str()))
        .map(|l| {
            let level = classify_comparable(l, reference);
            RankedComparable { listing: l.clone(), level: level.to_string(), data_quality: data_quality_for(l).to_string() }
        })
        .collect();

    ranked.sort_by(|a, b| {
        let rank_a = COMPARABLE_LEVELS_BY_PRIORITY.iter().position(|&l| l == a.level).unwrap_or(usize::MAX);
        let rank_b = COMPARABLE_LEVELS_BY_PRIORITY.iter().position(|&l| l == b.level).unwrap_or(usize::MAX);
        rank_a.cmp(&rank_b).then_with(|| a.listing.price_cents.cmp(&b.listing.price_cents))
    });
    ranked
}

/// Lowest/median/average/highest/count over a group already known to be
/// non-empty and single-currency - thin wrapper around `commands::price_
/// checker_scanner::compute_scan_stats` (reused, not reimplemented - see
/// `PriceStats`'s own doc comment, models.rs). `None` only for an empty
/// slice; every real call site below only ever calls this with a group it
/// already knows has at least one listing.
pub(crate) fn price_stats_for(listings: &[NormalizedListing]) -> Option<PriceStats> {
    if listings.is_empty() {
        return None;
    }
    let (lowest, median, average, highest, _currency) = compute_scan_stats(listings);
    Some(PriceStats {
        lowest_price_cents: lowest.expect("non-empty slice always has a lowest price"),
        median_price_cents: median.expect("non-empty slice always has a median price"),
        average_price_cents: average.expect("non-empty slice always has an average price"),
        highest_price_cents: highest.expect("non-empty slice always has a highest price"),
        listing_count: listings.len() as i64,
    })
}

/// One section's breakdown within one already-single-tier, single-currency
/// group - marko's own spec, "## MAP / SECTION ANALYSIS". Listings with no
/// section at all simply don't produce a `SectionBreakdown` row (nothing
/// honest to label them with), but they were already folded into the
/// caller's own tier-level `PriceStats` before this runs - see
/// `group_by_tier`. Sorted by lowest price ascending, tier name as a
/// deterministic tiebreaker.
///
/// Grouped case-insensitively (2.2.0 review fix): `tierFor`/section text in
/// `price_checker_scan.js` is raw marketplace DOM text, not normalized, so
/// the exact same section can legitimately be rendered as "Floor" in one
/// spot and "FLOOR" in another on the very same page. Without this, those
/// would silently split into two rows here even though `classify_comparable`
/// (via `same_str`, already case-insensitive) treats them as the same
/// section - two supposedly-consistent notions of "the same label"
/// disagreeing with each other. The DISPLAYED label keeps whichever casing
/// was seen FIRST, same "first occurrence wins, never blended/guessed"
/// convention `compute_scan_stats` already uses for currency.
fn group_by_section(listings: &[NormalizedListing]) -> Vec<SectionBreakdown> {
    let mut by_section: HashMap<String, (String, Vec<NormalizedListing>)> = HashMap::new();
    for l in listings {
        if let Some(section) = l.section.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            let key = section.to_ascii_lowercase();
            by_section.entry(key).or_insert_with(|| (section.to_string(), Vec::new())).1.push(l.clone());
        }
    }
    let mut sections: Vec<SectionBreakdown> = by_section
        .into_values()
        .map(|(section, group)| {
            let stats = price_stats_for(&group).expect("group is non-empty by construction");
            SectionBreakdown { section, stats }
        })
        .collect();
    sections.sort_by(|a, b| a.stats.lowest_price_cents.cmp(&b.stats.lowest_price_cents).then_with(|| a.section.cmp(&b.section)));
    sections
}

/// Groups an already-single-currency slice of listings by tier - marko's own
/// spec, "## TIER PRICING" + "## MAP / SECTION ANALYSIS". A listing with no
/// usable tier label becomes the literal `"Unclassified"` group (see
/// `UNCLASSIFIED_TIER`'s own doc comment), never dropped. Sorted by lowest
/// price ascending, tier name as a deterministic tiebreaker (a plain
/// `HashMap`'s own iteration order is not stable across runs, so this
/// tiebreaker is what keeps result order reproducible for tests and for
/// marko looking at the same data twice).
///
/// Grouped case-insensitively (2.2.0 review fix) - see `group_by_section`'s
/// own doc comment for why: the raw DOM tier text this groups on is not
/// normalized any earlier, so without this, "Level 100" and "LEVEL 100"
/// would silently become two separate tier rows even though
/// `classify_comparable` already treats them as the same tier via
/// `same_str`. Displayed label keeps whichever casing was seen first.
pub(crate) fn group_by_tier(listings: &[NormalizedListing]) -> Vec<TierBreakdown> {
    let mut by_tier: HashMap<String, (String, Vec<NormalizedListing>)> = HashMap::new();
    for l in listings {
        let raw = l.tier.as_deref().map(str::trim).filter(|t| !t.is_empty()).unwrap_or(UNCLASSIFIED_TIER);
        let key = raw.to_ascii_lowercase();
        by_tier.entry(key).or_insert_with(|| (raw.to_string(), Vec::new())).1.push(l.clone());
    }
    let mut tiers: Vec<TierBreakdown> = by_tier
        .into_values()
        .map(|(tier, group)| {
            let stats = price_stats_for(&group).expect("group is non-empty by construction");
            let sections = group_by_section(&group);
            TierBreakdown { tier, stats, sections }
        })
        .collect();
    tiers.sort_by(|a, b| a.stats.lowest_price_cents.cmp(&b.stats.lowest_price_cents).then_with(|| a.tier.cmp(&b.tier)));
    tiers
}

/// Splits a scan session's listings by currency - marko's own spec, "##
/// CURRENCY": "EUR + USD + GBP nikdy nesčítavaj ... alebo rozdeľ podľa
/// meny" (never sum EUR+USD+GBP ... split by currency instead) - this IS
/// that split, and the ONE place in this module mixed-currency input gets
/// separated; everything downstream trusts its input is already
/// single-currency. A listing with a price but no detected currency
/// contributes to neither map entry - counted separately (never guessed
/// into a currency it never reported).
pub(crate) fn partition_by_currency(listings: &[NormalizedListing]) -> (HashMap<String, Vec<NormalizedListing>>, i64) {
    let mut by_currency: HashMap<String, Vec<NormalizedListing>> = HashMap::new();
    let mut uncurrencied = 0i64;
    for l in listings {
        match l.currency.as_deref().map(str::trim).filter(|c| !c.is_empty()) {
            Some(cur) => by_currency.entry(cur.to_string()).or_default().push(l.clone()),
            None => uncurrencied += 1,
        }
    }
    (by_currency, uncurrencied)
}

/// "High" | "Medium" | "Low" - marko's own spec only pins down "Low" for
/// thin data ("Recommendation confidence: Low"); the rest is a reasonable
/// judgment call, flagged as such in
/// PRICE-CHECKER-MARKET-ANALYSIS-2.2-REPORT.md. Keyed off which pool
/// `recommend_price` actually used AND how many listings are in it - 3
/// "general_market" listings say far less than 3 "exact_comparable" ones,
/// so a single raw count alone wouldn't be honest.
pub(crate) fn recommendation_confidence(level: &str, pool_size: usize) -> &'static str {
    match (level, pool_size) {
        ("exact_comparable", n) if n >= 3 => "High",
        ("exact_comparable", _) => "Medium",
        ("close_comparable", n) if n >= 2 => "Medium",
        ("tier_comparable", n) if n >= 3 => "Medium",
        _ => "Low",
    }
}

/// Human-readable form of a comparable level for `PriceRecommendation::
/// based_on` - see that field's own doc comment (models.rs) for the exact 4
/// strings this must stay in sync with.
fn based_on_label(level: &str) -> &'static str {
    match level {
        "exact_comparable" => "Same section",
        "close_comparable" => "Close match (same tier)",
        "tier_comparable" => "Same tier",
        _ => "General market",
    }
}

/// Turns an already-ranked, already-single-currency comparable list into a
/// transparent recommendation - marko's own spec, "## PRICE RECOMMENDATION":
/// "jednoduchý transparentný výpočet ... nie AI". Picks the NARROWEST
/// non-empty pool from `comparables` (`COMPARABLE_LEVELS_BY_PRIORITY` order)
/// and undercuts ITS OWN lowest price by `RECOMMENDED_PRICE_UNDERCUT_PCT` -
/// the same formula `commands::price_checker::get_price_checker_summary_
/// impl` already uses, reused rather than reinvented. `market_average_price_
/// cents` is deliberately the OVERALL currency-wide average (`overall`,
/// passed in by the caller - see `CurrencyMarketAnalysis::overall`), not the
/// narrowed pool's own average - marko's spec lists "market average" as a
/// separate figure from the comparable lowest/median, not the same thing
/// computed twice. `None` only when `comparables` itself is empty (nothing
/// scanned yet in this currency) - a non-empty slice always has at least a
/// `"general_market"` pool to fall back to.
pub(crate) fn recommend_price(comparables: &[RankedComparable], overall: &PriceStats, avg_cost_cents: i64) -> Option<PriceRecommendation> {
    let (level, pool): (&str, Vec<&RankedComparable>) = COMPARABLE_LEVELS_BY_PRIORITY.iter().find_map(|lvl| {
        let pool: Vec<&RankedComparable> = comparables.iter().filter(|c| c.level == *lvl).collect();
        if pool.is_empty() {
            None
        } else {
            Some((*lvl, pool))
        }
    })?;

    let mut prices: Vec<i64> = pool.iter().map(|c| c.listing.price_cents).collect();
    prices.sort_unstable();
    let lowest = *prices.first().expect("pool is non-empty by construction");
    let median = median_of_sorted_cents(&prices);
    let recommended = (lowest as f64 * (1.0 - RECOMMENDED_PRICE_UNDERCUT_PCT)).round() as i64;
    let expected_profit_cents = recommended - avg_cost_cents;
    let expected_roi = finance::safe_ratio(expected_profit_cents, avg_cost_cents);

    Some(PriceRecommendation {
        comparable_lowest_price_cents: lowest,
        comparable_median_price_cents: median,
        market_average_price_cents: overall.average_price_cents,
        recommended_price_cents: recommended,
        expected_profit_cents,
        expected_roi,
        based_on: based_on_label(level).to_string(),
        confidence: recommendation_confidence(level, pool.len()).to_string(),
    })
}

/// One row of marko's own unsold inventory, grouped - see
/// `compute_your_tickets`'s own doc comment for the query this comes from.
/// Kept as a small private struct rather than a tuple purely for readable
/// field access; deliberately not named `Row` to avoid any confusion with
/// `rusqlite::Row` (not imported in this file).
struct TicketGroupRow {
    section: Option<String>,
    row_label: Option<String>,
    currency: String,
    quantity: i64,
    cost_sum_cents: i64,
    priced_count: i64,
    listing_sum_cents: i64,
}

/// marko's own spec, "## YOUR TICKETS" + "## PRICE RECOMMENDATION": reuses
/// the exact same unsold-inventory scope `commands::price_checker::
/// get_price_checker_summary_impl` already uses (`status IN
/// ('available','listed')` - not yet sold, not cancelled), grouped by
/// (section, row, currency) rather than aggregated into one blended figure,
/// since several identical unsold tickets in the same section/row are one
/// real pricing decision, not several. `tier` is always `None` on the
/// result - `tickets` has no tier/level column at all, see `YourTicketGroup
/// ::tier`'s own doc comment (models.rs) for why that's a deliberate,
/// documented gap rather than an oversight.
///
/// Each group's own `recommendation` reuses `by_currency`'s ALREADY-COMPUTED
/// `PriceStats` (never re-derives market stats a second time per group -
/// marko's own "## PERFORMANCE") and ranks the CURRENT session's listings
/// against an reference built from the group's own section/row/quantity -
/// `None` exactly when `by_currency` has no entry for this group's currency
/// (nothing scanned yet in that currency), matching `YourTicketGroup::
/// recommendation`'s own doc comment.
fn compute_your_tickets(
    conn: &Connection,
    event_id: i64,
    listings: &[NormalizedListing],
    request_id: u64,
    by_currency: &[CurrencyMarketAnalysis],
) -> AppResult<Vec<YourTicketGroup>> {
    let mut stmt = conn.prepare(
        "SELECT section, row_label, currency,
                COUNT(*),
                COALESCE(SUM(purchase_cost_cents + purchase_fees_cents + other_costs_cents), 0),
                COUNT(CASE WHEN listing_price_cents IS NOT NULL THEN 1 END),
                COALESCE(SUM(listing_price_cents), 0)
           FROM tickets
          WHERE event_id = ?1 AND status IN ('available','listed')
          GROUP BY section, row_label, currency
          ORDER BY section, row_label",
    )?;
    let rows = stmt.query_map([event_id], |r| {
        Ok(TicketGroupRow {
            section: r.get(0)?,
            row_label: r.get(1)?,
            currency: r.get(2)?,
            quantity: r.get(3)?,
            cost_sum_cents: r.get(4)?,
            priced_count: r.get(5)?,
            listing_sum_cents: r.get(6)?,
        })
    })?;
    let rows: Vec<TicketGroupRow> = rows.collect::<Result<Vec<_>, _>>()?;

    let mut groups = Vec::with_capacity(rows.len());
    for r in rows {
        let avg_cost_cents = finance::safe_ratio(r.cost_sum_cents, r.quantity).map(|v| v.round() as i64).unwrap_or(0);
        let avg_listing_price_cents = finance::safe_ratio(r.listing_sum_cents, r.priced_count).map(|v| v.round() as i64);

        let reference = ComparableReferenceInput {
            request_id,
            section: r.section.clone(),
            tier: None,
            row: r.row_label.clone(),
            quantity: u32::try_from(r.quantity).ok(),
            currency: r.currency.clone(),
        };
        let recommendation = by_currency.iter().find(|c| c.currency == r.currency).and_then(|c| {
            let ranked = rank_comparable(listings, &reference);
            recommend_price(&ranked, &c.overall, avg_cost_cents)
        });

        groups.push(YourTicketGroup {
            tier: None,
            section: r.section,
            row: r.row_label,
            quantity: r.quantity,
            currency: r.currency,
            avg_cost_cents,
            avg_listing_price_cents,
            recommendation,
        });
    }
    Ok(groups)
}

/// Core of `compute_market_analysis` - split out for direct
/// unit-testability, same `_impl`-function convention this codebase already
/// uses everywhere (see `commands::price_checker::get_price_checker_summary_
/// impl`). `listings` is a snapshot the caller already pulled out of the
/// scanner session (see the `#[tauri::command]` wrapper below for why that
/// happens before this runs, not inside it).
pub(crate) fn compute_market_analysis_impl(
    conn: &Connection,
    listings: &[NormalizedListing],
    request_id: u64,
    event_id: i64,
) -> AppResult<MarketAnalysisResult> {
    let (by_currency_map, uncurrencied_listing_count) = partition_by_currency(listings);

    let mut by_currency: Vec<CurrencyMarketAnalysis> = by_currency_map
        .into_iter()
        .map(|(currency, group)| {
            let overall = price_stats_for(&group).expect("currency group is non-empty by construction");
            let tiers = group_by_tier(&group);
            CurrencyMarketAnalysis { currency, overall, tiers }
        })
        .collect();
    by_currency.sort_by(|a, b| a.currency.cmp(&b.currency));
    let mixed_currencies = by_currency.len() > 1;

    let your_tickets = compute_your_tickets(conn, event_id, listings, request_id, &by_currency)?;

    Ok(MarketAnalysisResult { request_id, by_currency, mixed_currencies, uncurrencied_listing_count, your_tickets })
}

// ---------------------------------------------------------------------------
// Tauri glue - thin State/session lookups over the pure functions above.
// ---------------------------------------------------------------------------

/// Reads the scan session's already-accumulated listings, then defers to
/// `compute_market_analysis_impl` - marko's own spec, "## MARKET OVERVIEW" +
/// "## TIER PRICING" + "## YOUR TICKETS", all in one round trip per his own
/// "## PERFORMANCE" requirement. Locks `price_scanner_sessions` just long
/// enough to clone the listings back out, then drops it before touching
/// `state.db` - this command is the first thing in the app that needs BOTH
/// locks, and never holding them at the same time is what keeps it
/// deadlock-safe regardless of what order any other command happens to take
/// them in.
#[tauri::command]
pub fn compute_market_analysis(state: State<AppState>, request_id: u64, event_id: i64) -> AppResult<MarketAnalysisResult> {
    let listings: Vec<NormalizedListing> = {
        let sessions = state.price_scanner_sessions.lock().unwrap();
        let session = sessions
            .get(&request_id)
            .ok_or_else(|| AppError::NotFound("Scanner session not found - the window may have been closed".into()))?;
        session.listings.clone()
    };
    let conn = state.db.lock().unwrap();
    compute_market_analysis_impl(&conn, &listings, request_id, event_id)
}

/// Ranks the scan session's listings against one specific reference ticket -
/// marko's own spec, "## COMPARABLE MARKET" (worked example: Section 112 /
/// Row 8 / Quantity 4). Pure in-memory - never touches `state.db` at all,
/// unlike `compute_market_analysis` above.
#[tauri::command]
pub fn compute_comparable_market(state: State<AppState>, input: ComparableReferenceInput) -> AppResult<Vec<RankedComparable>> {
    let sessions = state.price_scanner_sessions.lock().unwrap();
    let session = sessions
        .get(&input.request_id)
        .ok_or_else(|| AppError::NotFound("Scanner session not found - the window may have been closed".into()))?;
    Ok(rank_comparable(&session.listings, &input))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_conn;
    use rusqlite::params;

    fn listing(price_cents: i64, currency: &str, section: Option<&str>, tier: Option<&str>, row: Option<&str>, quantity: Option<u32>) -> NormalizedListing {
        NormalizedListing {
            price_cents,
            currency: Some(currency.to_string()),
            section: section.map(str::to_string),
            row: row.map(str::to_string),
            tier: tier.map(str::to_string),
            quantity,
            listing_id: None,
            marketplace: "generic".to_string(),
        }
    }

    fn reference(section: Option<&str>, tier: Option<&str>, row: Option<&str>, quantity: Option<u32>, currency: &str) -> ComparableReferenceInput {
        ComparableReferenceInput {
            request_id: 1,
            section: section.map(str::to_string),
            tier: tier.map(str::to_string),
            row: row.map(str::to_string),
            quantity,
            currency: currency.to_string(),
        }
    }

    // -- same_str / nearby_numeric ------------------------------------------

    #[test]
    fn same_str_matches_case_and_whitespace_insensitively() {
        assert!(same_str(Some(" Level 100 "), Some("level 100")));
    }

    #[test]
    fn same_str_is_false_when_either_side_missing_or_blank() {
        assert!(!same_str(None, Some("112")));
        assert!(!same_str(Some("112"), None));
        assert!(!same_str(Some("  "), Some("  ")));
    }

    #[test]
    fn nearby_numeric_true_within_threshold_false_beyond_it() {
        assert!(nearby_numeric(Some("Section 110"), Some("Section 112")));
        assert!(!nearby_numeric(Some("Section 100"), Some("Section 112")));
    }

    #[test]
    fn nearby_numeric_false_when_a_side_has_no_digits() {
        assert!(!nearby_numeric(Some("Floor"), Some("Section 112")));
        assert!(!nearby_numeric(None, Some("Section 112")));
    }

    // -- data_quality_for -----------------------------------------------------

    #[test]
    fn strong_comparable_needs_section_row_and_quantity_together() {
        let l = listing(5000, "EUR", Some("112"), None, Some("A"), Some(2));
        assert_eq!(data_quality_for(&l), "strong_comparable");
    }

    #[test]
    fn section_comparable_needs_section_and_tier_without_row_or_quantity() {
        let l = listing(5000, "EUR", Some("112"), Some("Level 100"), None, None);
        assert_eq!(data_quality_for(&l), "section_comparable");
    }

    #[test]
    fn tier_comparable_needs_only_a_tier() {
        let l = listing(5000, "EUR", None, Some("Level 100"), None, None);
        assert_eq!(data_quality_for(&l), "tier_comparable");
    }

    #[test]
    fn a_section_alone_without_a_tier_is_only_partial() {
        // The literal spec precedence names only tier / section+tier /
        // section+row+quantity - "section alone" isn't a named level, so it
        // falls through to Partial even though it feels like real data.
        // Documented explicitly here so this doesn't look like a bug later.
        let l = listing(5000, "EUR", Some("112"), None, None, None);
        assert_eq!(data_quality_for(&l), "partial");
    }

    #[test]
    fn a_bare_price_only_listing_is_partial() {
        let l = listing(5000, "EUR", None, None, None, None);
        assert_eq!(data_quality_for(&l), "partial");
    }

    // -- classify_comparable ---------------------------------------------------

    #[test]
    fn exact_comparable_on_matching_section_even_with_partial_data_quality() {
        // Proves the two classifications are independent - see this
        // module's own doc comment. Only a section is present (no
        // tier/row/quantity), so data_quality is "partial", but the section
        // still matches the reference exactly.
        let l = listing(5000, "EUR", Some("112"), None, None, None);
        let r = reference(Some("112"), None, None, None, "EUR");
        assert_eq!(classify_comparable(&l, &r), "exact_comparable");
        assert_eq!(data_quality_for(&l), "partial", "data_quality must stay honest about the missing fields");
    }

    #[test]
    fn close_comparable_via_nearby_section_within_the_same_tier() {
        let l = listing(5000, "EUR", Some("110"), Some("Level 100"), None, None);
        let r = reference(Some("112"), Some("Level 100"), None, None, "EUR");
        assert_eq!(classify_comparable(&l, &r), "close_comparable");
    }

    #[test]
    fn close_comparable_via_same_quantity_within_the_same_tier() {
        let l = listing(5000, "EUR", None, Some("Level 100"), None, Some(4));
        let r = reference(None, Some("Level 100"), None, Some(4), "EUR");
        assert_eq!(classify_comparable(&l, &r), "close_comparable");
    }

    #[test]
    fn close_comparable_via_nearby_row_within_the_same_tier() {
        let l = listing(5000, "EUR", None, Some("Level 100"), Some("Row 9"), None);
        let r = reference(None, Some("Level 100"), Some("Row 8"), None, "EUR");
        assert_eq!(classify_comparable(&l, &r), "close_comparable");
    }

    #[test]
    fn tier_comparable_when_same_tier_but_nothing_else_close() {
        let l = listing(5000, "EUR", Some("999"), Some("Level 100"), Some("Row 99"), Some(9));
        let r = reference(Some("112"), Some("Level 100"), Some("Row 8"), Some(4), "EUR");
        assert_eq!(classify_comparable(&l, &r), "tier_comparable");
    }

    #[test]
    fn general_market_when_nothing_matches_at_all() {
        let l = listing(5000, "EUR", Some("999"), Some("Level 900"), None, None);
        let r = reference(Some("112"), Some("Level 100"), None, None, "EUR");
        assert_eq!(classify_comparable(&l, &r), "general_market");
    }

    #[test]
    fn general_market_when_the_reference_itself_gives_nothing_to_match() {
        let l = listing(5000, "EUR", Some("112"), Some("Level 100"), None, None);
        let r = reference(None, None, None, None, "EUR");
        assert_eq!(classify_comparable(&l, &r), "general_market");
    }

    // -- rank_comparable ---------------------------------------------------------

    #[test]
    fn rank_comparable_excludes_listings_in_a_different_currency() {
        let listings = vec![listing(5000, "EUR", Some("112"), None, None, None), listing(4000, "USD", Some("112"), None, None, None)];
        let r = reference(Some("112"), None, None, None, "EUR");
        let ranked = rank_comparable(&listings, &r);
        assert_eq!(ranked.len(), 1, "the USD listing must never be blended into a EUR comparable ranking");
        assert_eq!(ranked[0].listing.currency.as_deref(), Some("EUR"));
    }

    #[test]
    fn rank_comparable_sorts_by_level_priority_then_by_price() {
        let listings = vec![
            listing(3000, "EUR", Some("999"), None, None, None), // general_market
            listing(1000, "EUR", Some("112"), None, None, None), // exact_comparable, cheaper
            listing(2000, "EUR", Some("112"), None, None, None), // exact_comparable, pricier
        ];
        let r = reference(Some("112"), None, None, None, "EUR");
        let ranked = rank_comparable(&listings, &r);
        assert_eq!(ranked.iter().map(|c| c.level.as_str()).collect::<Vec<_>>(), vec!["exact_comparable", "exact_comparable", "general_market"]);
        assert_eq!(ranked[0].listing.price_cents, 1000, "within the same level, cheapest must come first");
        assert_eq!(ranked[1].listing.price_cents, 2000);
    }

    // -- price_stats_for -----------------------------------------------------

    #[test]
    fn price_stats_for_an_empty_slice_is_none() {
        assert!(price_stats_for(&[]).is_none());
    }

    #[test]
    fn price_stats_for_reports_lowest_median_average_highest_and_count() {
        let listings = vec![listing(1000, "EUR", None, None, None, None), listing(2000, "EUR", None, None, None, None), listing(3000, "EUR", None, None, None, None)];
        let stats = price_stats_for(&listings).unwrap();
        assert_eq!(stats.lowest_price_cents, 1000);
        assert_eq!(stats.median_price_cents, 2000);
        assert_eq!(stats.average_price_cents, 2000);
        assert_eq!(stats.highest_price_cents, 3000);
        assert_eq!(stats.listing_count, 3);
    }

    // -- group_by_tier / group_by_section ------------------------------------

    #[test]
    fn listings_without_a_usable_tier_are_grouped_as_unclassified() {
        let listings = vec![listing(1000, "EUR", Some("112"), None, None, None), listing(2000, "EUR", None, Some("   "), None, None)];
        let tiers = group_by_tier(&listings);
        assert_eq!(tiers.len(), 1);
        assert_eq!(tiers[0].tier, "Unclassified");
        assert_eq!(tiers[0].stats.listing_count, 2, "a blank tier string must be treated the same as no tier at all");
    }

    #[test]
    fn tiers_with_inconsistent_casing_merge_into_one_group_keeping_the_first_seen_label() {
        // Review-pass fix: tierFor (price_checker_scan.js) returns raw,
        // unnormalized DOM text, so the SAME real tier can legitimately show
        // up as "Level 100" in one spot and "LEVEL 100" in another on the
        // same page. classify_comparable already treats those as the same
        // tier via same_str (case-insensitive) - group_by_tier must agree,
        // not silently split them into two rows.
        let listings = vec![
            listing(1000, "EUR", None, Some("Level 100"), None, None),
            listing(2000, "EUR", None, Some("LEVEL 100"), None, None),
            listing(3000, "EUR", None, Some("  level 100  "), None, None),
        ];
        let tiers = group_by_tier(&listings);
        assert_eq!(tiers.len(), 1, "differently-cased/whitespaced variants of the same tier must merge");
        assert_eq!(tiers[0].tier, "Level 100", "displayed label keeps whichever casing was seen first");
        assert_eq!(tiers[0].stats.listing_count, 3);
        assert_eq!(tiers[0].stats.lowest_price_cents, 1000);
    }

    #[test]
    fn sections_with_inconsistent_casing_merge_into_one_group_keeping_the_first_seen_label() {
        let listings = vec![
            listing(5000, "EUR", Some("Floor"), Some("Level 100"), None, None),
            listing(4000, "EUR", Some("FLOOR"), Some("Level 100"), None, None),
        ];
        let tiers = group_by_tier(&listings);
        assert_eq!(tiers[0].sections.len(), 1, "differently-cased section variants must merge");
        assert_eq!(tiers[0].sections[0].section, "Floor", "displayed label keeps whichever casing was seen first");
        assert_eq!(tiers[0].sections[0].stats.listing_count, 2);
    }

    #[test]
    fn tiers_are_sorted_by_lowest_price_ascending() {
        let listings = vec![
            listing(9000, "EUR", None, Some("Level 500"), None, None),
            listing(3000, "EUR", None, Some("Level 100"), None, None),
            listing(6000, "EUR", None, Some("Level 200"), None, None),
        ];
        let tiers = group_by_tier(&listings);
        assert_eq!(tiers.iter().map(|t| t.tier.as_str()).collect::<Vec<_>>(), vec!["Level 100", "Level 200", "Level 500"]);
    }

    #[test]
    fn each_tiers_own_lowest_median_and_average_are_computed_from_only_that_tiers_listings() {
        // marko's own testing checklist (point #17): "lowest/median/average
        // per tier" - explicitly proving these are each tier's OWN numbers,
        // not copies of the overall/other tier's stats, since a bug that
        // accidentally passed the whole session into price_stats_for instead
        // of just one tier's group would still pass every OTHER group_by_tier
        // test above (they only ever check `lowest`).
        let listings = vec![
            listing(1000, "EUR", None, Some("Level 100"), None, None),
            listing(2000, "EUR", None, Some("Level 100"), None, None),
            listing(3000, "EUR", None, Some("Level 100"), None, None),
            listing(9000, "EUR", None, Some("Level 900"), None, None),
            listing(9400, "EUR", None, Some("Level 900"), None, None),
        ];
        let tiers = group_by_tier(&listings);
        let level_100 = tiers.iter().find(|t| t.tier == "Level 100").unwrap();
        assert_eq!(level_100.stats.lowest_price_cents, 1000);
        assert_eq!(level_100.stats.median_price_cents, 2000);
        assert_eq!(level_100.stats.average_price_cents, 2000);
        assert_eq!(level_100.stats.listing_count, 3);

        let level_900 = tiers.iter().find(|t| t.tier == "Level 900").unwrap();
        assert_eq!(level_900.stats.lowest_price_cents, 9000);
        assert_eq!(level_900.stats.average_price_cents, 9200, "Level 900's own average must not leak in Level 100's listings");
        assert_eq!(level_900.stats.listing_count, 2);
    }

    #[test]
    fn a_listing_without_a_section_still_counts_toward_its_tiers_overall_stats() {
        let listings = vec![listing(1000, "EUR", Some("112"), Some("Level 100"), None, None), listing(500, "EUR", None, Some("Level 100"), None, None)];
        let tiers = group_by_tier(&listings);
        assert_eq!(tiers.len(), 1);
        assert_eq!(tiers[0].stats.listing_count, 2, "both listings must count toward the tier total");
        assert_eq!(tiers[0].stats.lowest_price_cents, 500);
        assert_eq!(tiers[0].sections.len(), 1, "but only the one WITH a section produces a SectionBreakdown row");
        assert_eq!(tiers[0].sections[0].section, "112");
    }

    #[test]
    fn sections_within_a_tier_are_sorted_by_lowest_price_ascending() {
        let listings = vec![
            listing(9000, "EUR", Some("300"), Some("Level 100"), None, None),
            listing(3000, "EUR", Some("100"), Some("Level 100"), None, None),
            listing(6000, "EUR", Some("200"), Some("Level 100"), None, None),
        ];
        let tiers = group_by_tier(&listings);
        assert_eq!(tiers[0].sections.iter().map(|s| s.section.as_str()).collect::<Vec<_>>(), vec!["100", "200", "300"]);
    }

    // -- recommend_price / recommendation_confidence -------------------------

    fn overall_stats(lowest: i64, average: i64, highest: i64) -> PriceStats {
        PriceStats { lowest_price_cents: lowest, median_price_cents: (lowest + highest) / 2, average_price_cents: average, highest_price_cents: highest, listing_count: 10 }
    }

    #[test]
    fn recommend_price_is_none_for_an_empty_comparables_slice() {
        assert!(recommend_price(&[], &overall_stats(1000, 1200, 1500), 800).is_none());
    }

    #[test]
    fn recommend_price_prefers_the_narrowest_non_empty_pool() {
        let listings = vec![
            listing(2000, "EUR", Some("999"), None, None, None), // general_market, cheaper than the exact match
            listing(3000, "EUR", Some("112"), None, None, None), // exact_comparable
        ];
        let r = reference(Some("112"), None, None, None, "EUR");
        let ranked = rank_comparable(&listings, &r);
        let rec = recommend_price(&ranked, &overall_stats(2000, 2500, 3000), 1000).unwrap();

        assert_eq!(rec.based_on, "Same section", "must use the exact_comparable pool, not the cheaper general_market listing");
        assert_eq!(rec.comparable_lowest_price_cents, 3000);
        assert_eq!(rec.recommended_price_cents, 2850, "3000 undercut by RECOMMENDED_PRICE_UNDERCUT_PCT (5%)");
        assert_eq!(rec.expected_profit_cents, 2850 - 1000);
    }

    #[test]
    fn recommend_price_falls_back_to_general_market_when_nothing_closer_exists() {
        let listings = vec![listing(4000, "EUR", Some("999"), Some("Level 900"), None, None)];
        let r = reference(Some("112"), Some("Level 100"), None, None, "EUR");
        let ranked = rank_comparable(&listings, &r);
        let rec = recommend_price(&ranked, &overall_stats(4000, 4000, 4000), 1000).unwrap();
        assert_eq!(rec.based_on, "General market");
    }

    #[test]
    fn market_average_comes_from_the_overall_stats_not_the_narrowed_pool() {
        let listings = vec![listing(3000, "EUR", Some("112"), None, None, None)];
        let r = reference(Some("112"), None, None, None, "EUR");
        let ranked = rank_comparable(&listings, &r);
        let rec = recommend_price(&ranked, &overall_stats(2000, 9999, 5000), 1000).unwrap();
        assert_eq!(rec.market_average_price_cents, 9999, "market average is the whole-currency figure, never recomputed from the narrowed pool");
    }

    #[test]
    fn confidence_is_high_for_a_well_populated_exact_match() {
        assert_eq!(recommendation_confidence("exact_comparable", 5), "High");
    }

    #[test]
    fn confidence_is_low_for_a_thin_general_market_pool() {
        assert_eq!(recommendation_confidence("general_market", 1), "Low");
    }

    #[test]
    fn confidence_is_medium_for_a_small_exact_match() {
        assert_eq!(recommendation_confidence("exact_comparable", 1), "Medium");
    }

    // -- partition_by_currency -------------------------------------------------

    #[test]
    fn partition_by_currency_splits_groups_and_counts_uncurrencied_separately() {
        let listings = vec![listing(1000, "EUR", None, None, None, None), listing(2000, "USD", None, None, None, None), NormalizedListing { currency: None, ..listing(3000, "EUR", None, None, None, None) }];
        let (by_currency, uncurrencied) = partition_by_currency(&listings);
        assert_eq!(by_currency.len(), 2);
        assert_eq!(by_currency.get("EUR").unwrap().len(), 1);
        assert_eq!(by_currency.get("USD").unwrap().len(), 1);
        assert_eq!(uncurrencied, 1);
    }

    // -- compute_market_analysis_impl / compute_your_tickets (DB-backed) -----

    fn seed_event(conn: &Connection, name: &str) -> i64 {
        conn.execute("INSERT INTO events (name) VALUES (?1)", [name]).unwrap();
        conn.last_insert_rowid()
    }

    fn seed_order(conn: &Connection, code_suffix: &str, event_id: i64) -> i64 {
        conn.execute(
            "INSERT INTO orders (code, event_id, purchase_date, quantity, currency) VALUES (?1, ?2, '2026-01-01', 1, 'EUR')",
            params![format!("ORD-{code_suffix}"), event_id],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    #[allow(clippy::too_many_arguments)]
    fn seed_ticket(conn: &Connection, code_suffix: &str, event_id: i64, section: Option<&str>, row_label: Option<&str>, status: &str, currency: &str, purchase_cost_cents: i64, listing_price_cents: Option<i64>) {
        let order_id = seed_order(conn, code_suffix, event_id);
        conn.execute(
            "INSERT INTO tickets (code, event_id, order_id, section, row_label, purchase_cost_cents, listing_price_cents, currency, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![format!("TKT-{code_suffix}"), event_id, order_id, section, row_label, purchase_cost_cents, listing_price_cents, currency, status],
        )
        .unwrap();
    }

    #[test]
    fn compute_market_analysis_splits_the_session_by_currency() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event");
        let listings = vec![listing(1000, "EUR", Some("112"), None, None, None), listing(1200, "USD", Some("112"), None, None, None)];

        let result = compute_market_analysis_impl(&conn, &listings, 1, event_id).unwrap();

        assert_eq!(result.by_currency.len(), 2);
        assert!(result.mixed_currencies);
        assert_eq!(result.by_currency.iter().map(|c| c.currency.as_str()).collect::<Vec<_>>(), vec!["EUR", "USD"], "sorted alphabetically for a deterministic result");
    }

    #[test]
    fn uncurrencied_listings_are_counted_but_join_no_currency_group() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event");
        let mut no_currency = listing(1000, "EUR", None, None, None, None);
        no_currency.currency = None;
        let listings = vec![listing(2000, "EUR", None, None, None, None), no_currency];

        let result = compute_market_analysis_impl(&conn, &listings, 1, event_id).unwrap();

        assert_eq!(result.by_currency.len(), 1);
        assert_eq!(result.by_currency[0].overall.listing_count, 1, "the uncurrencied listing must not be blended into EUR");
        assert_eq!(result.uncurrencied_listing_count, 1);
    }

    #[test]
    fn your_tickets_groups_by_section_and_row_and_counts_quantity() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event");
        seed_ticket(&conn, "1", event_id, Some("112"), Some("A"), "available", "EUR", 4000, Some(6000));
        seed_ticket(&conn, "2", event_id, Some("112"), Some("A"), "available", "EUR", 4200, Some(6200));
        seed_ticket(&conn, "3", event_id, Some("112"), Some("A"), "sold", "EUR", 9999, Some(99999)); // excluded

        let result = compute_market_analysis_impl(&conn, &[], 1, event_id).unwrap();

        assert_eq!(result.your_tickets.len(), 1);
        let group = &result.your_tickets[0];
        assert_eq!(group.quantity, 2, "the sold ticket must not be counted");
        assert_eq!(group.tier, None, "tickets have no tier column - must never be fabricated");
        assert_eq!(group.avg_cost_cents, (4000 + 4200) / 2);
        assert_eq!(group.avg_listing_price_cents, Some((6000 + 6200) / 2));
    }

    #[test]
    fn your_tickets_recommendation_is_none_when_nothing_scanned_in_that_currency_yet() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event");
        seed_ticket(&conn, "1", event_id, Some("112"), Some("A"), "available", "EUR", 4000, Some(6000));

        let result = compute_market_analysis_impl(&conn, &[], 1, event_id).unwrap(); // no scan session listings at all

        assert!(result.your_tickets[0].recommendation.is_none());
    }

    #[test]
    fn your_tickets_recommendation_is_present_once_the_matching_currency_has_been_scanned() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event");
        seed_ticket(&conn, "1", event_id, Some("112"), Some("A"), "available", "EUR", 4000, None);
        let listings = vec![listing(6000, "EUR", Some("112"), None, None, None)];

        let result = compute_market_analysis_impl(&conn, &listings, 1, event_id).unwrap();

        let rec = result.your_tickets[0].recommendation.as_ref().expect("a EUR scan exists, so a EUR recommendation must be produced");
        assert_eq!(rec.based_on, "Same section");
    }
}

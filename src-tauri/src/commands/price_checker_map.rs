//! Price Checker Market Map (2.26.0) - a second VIEW of one scan session.
//!
//! # What this is, and what it deliberately is not
//!
//! This module adds NO scanning of any kind. It is the exact same shape as
//! `price_checker_analysis` next door: take the listings a manual scan already
//! accumulated in `ScannerSession`, take marko's own unsold tickets for the
//! same event out of the database, and fold the two into one read-only
//! structure the frontend can draw as a venue map. No new table, no new
//! migration, no background work, no polling - and the scanner lifecycle
//! (open / scan / cancel / close) is untouched.
//!
//! # What the scanner ACTUALLY gives us (audited, not assumed)
//!
//! `price_checker_scan.js` is ONE generic DOM reader, not three per-marketplace
//! ones - the three `read*` functions differ only in the `marketplace` label
//! they stamp on a candidate. Per listing it can produce: price, currency,
//! section, row, tier, quantity, listing id, marketplace. Every one of those
//! except price and marketplace is best-effort and routinely `None`.
//!
//! It does NOT produce, anywhere, at all:
//!
//!   * **seat numbers** - there is no seat regex in the reader. So this map
//!     stops at section/row. A seat-level map would be a fabrication.
//!   * **venue identity or geometry** - nothing reads the page's seat-map
//!     widget, so there are no coordinates, no polygons and no real section
//!     adjacency. The layout here is therefore DETERMINISTIC (sorted), never
//!     a claim about where a section physically is.
//!   * **a per-listing URL** - no href is captured, so a single listing cannot
//!     be linked back to its own page. The event's marketplace link is the
//!     only URL that exists, and the UI says so rather than implying more.
//!
//! Those three gaps are the whole reason this file has a "source value is
//! kept" rule instead of a "fill in the blanks" rule.
//!
//! # Normalization: safe only
//!
//! `"Sec 102"`, `"Section 102"` and `"102"` all reach us as the token after
//! the word - so the remaining job is small and is kept small on purpose:
//! trim, collapse inner whitespace, upper-case for the grouping KEY, and drop
//! leading zeroes ONLY when what is left is entirely digits. Anything else is
//! passed through untouched.
//!
//! Every group keeps BOTH: `key` (what grouping compares) and `label` (the
//! first source value that produced it, shown on screen). A value that cannot
//! be normalized safely is never rewritten into a made-up one - it becomes its
//! own group under its own source text.
//!
//! # Pricing
//!
//! There is none. This module computes lowest / median / highest per section
//! purely as a description of what is listed there. It never compares one
//! section to another, never interpolates between them and never produces a
//! recommendation - section, row and seat are metadata, not pricing inputs.

use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::models::{
    MarketMap, MarketMapListing, MarketMapMyTicket, MarketMapSection, MarketMapTier,
    NormalizedListing,
};
use rusqlite::Connection;
use std::collections::BTreeMap;
use tauri::State;

/// Same literal `price_checker_analysis::group_by_tier` already displays for
/// listings with no detectable tier. Reused rather than re-invented so the two
/// views of one session never disagree about what to call the same group.
const UNCLASSIFIED_TIER: &str = "Unclassified";

/// Shown for listings that have a price but no section at all. NOT a section:
/// it is rendered as its own explicitly-labelled bucket outside the map grid,
/// because a scan that read prices without sections must say so rather than
/// scatter those listings across invented blocks.
pub(crate) const NO_SECTION: &str = "No section data";

/// Trim, collapse inner whitespace, upper-case. The shared first half of every
/// normalizer here - deliberately the only text transformation applied to a
/// value that is not purely numeric.
fn tidy(raw: &str) -> Option<String> {
    let collapsed = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = collapsed.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_uppercase())
    }
}

/// Leading zeroes come off ONLY when every remaining character is a digit, so
/// `"0102"` and `"102"` are one section while `"0A"` is left exactly as it is.
/// `"000"` stays `"0"` rather than becoming empty.
fn strip_numeric_zeroes(value: &str) -> String {
    if !value.is_empty() && value.chars().all(|c| c.is_ascii_digit()) {
        let trimmed = value.trim_start_matches('0');
        return if trimmed.is_empty() { "0".to_string() } else { trimmed.to_string() };
    }
    value.to_string()
}

/// The grouping key for a section. `None` when there is genuinely nothing to
/// group by - never a placeholder string, which the caller turns into the
/// explicit `NO_SECTION` bucket instead.
pub(crate) fn normalize_section(raw: Option<&str>) -> Option<String> {
    let value = tidy(raw?)?;
    // A stray "SECTION"/"SEC" prefix can still arrive from the generic text
    // layer (the selector layer's regex already strips it). Only removed when
    // something is actually left after it - "SECTION" on its own is not a
    // section name, it is a header that leaked in.
    let stripped = value
        .strip_prefix("SECTION ")
        .or_else(|| value.strip_prefix("SEC "))
        .unwrap_or(&value)
        .trim()
        .to_string();
    if stripped.is_empty() {
        return None;
    }
    Some(strip_numeric_zeroes(&stripped))
}

/// The grouping key for a tier. No vocabulary mapping whatsoever: the page's
/// own wording is what groups, because "Level 100" and "Tier 1" mean whatever
/// that marketplace decided they mean and this module has no way to know.
pub(crate) fn normalize_tier(raw: Option<&str>) -> Option<String> {
    tidy(raw?)
}

/// The grouping key for a row. Same numeric rule as sections, so row `"07"`
/// and row `"7"` are one row while row `"AA"` is untouched.
pub(crate) fn normalize_row(raw: Option<&str>) -> Option<String> {
    let value = tidy(raw?)?;
    let stripped = value.strip_prefix("ROW ").unwrap_or(&value).trim().to_string();
    if stripped.is_empty() {
        return None;
    }
    Some(strip_numeric_zeroes(&stripped))
}

/// Sort key that puts `"102"` before `"1002"` and both before `"FLOOR"`, so a
/// map's blocks land in the same order every time it is drawn. Purely a
/// display ordering - it is NOT a claim that section 102 sits next to 103 in
/// the real building.
fn order_key(key: &str) -> (u8, u64, String) {
    let leading: String = key.chars().take_while(|c| c.is_ascii_digit()).collect();
    if leading.is_empty() {
        (1, 0, key.to_string())
    } else {
        (0, leading.parse::<u64>().unwrap_or(u64::MAX), key.to_string())
    }
}

fn median_of(sorted: &[i64]) -> Option<i64> {
    if sorted.is_empty() {
        return None;
    }
    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 1 {
        Some(sorted[mid])
    } else {
        // Same even-length convention `price_checker_analysis` uses: the mean
        // of the two middle values, in integer cents.
        Some((sorted[mid - 1] + sorted[mid]) / 2)
    }
}

/// One of marko's own unsold tickets, read straight out of `tickets`.
#[derive(Debug, Clone)]
pub(crate) struct MyTicketRow {
    pub id: i64,
    pub order_id: i64,
    pub code: String,
    pub section: Option<String>,
    pub row_label: Option<String>,
    pub seat: Option<String>,
    pub tier: Option<String>,
    pub listing_price_cents: Option<i64>,
    pub currency: String,
    pub status: String,
}

/// Folds one session's listings plus marko's own tickets into the map.
///
/// Pure - no database, no state, no clock - which is what makes every rule in
/// this file directly testable.
pub(crate) fn build_market_map(listings: &[NormalizedListing], mine: &[MyTicketRow]) -> MarketMap {
    // (tier key, section key) -> bucket. BTreeMap purely for determinism; the
    // real display order is applied at the end via `order_key`.
    struct Bucket {
        tier_label: String,
        section_label: String,
        has_section: bool,
        listings: Vec<MarketMapListing>,
        mine: Vec<MarketMapMyTicket>,
    }
    let mut buckets: BTreeMap<(String, String), Bucket> = BTreeMap::new();

    for l in listings {
        let tier_key = normalize_tier(l.tier.as_deref());
        let section_key = normalize_section(l.section.as_deref());
        let has_section = section_key.is_some();
        let tk = tier_key.clone().unwrap_or_else(|| UNCLASSIFIED_TIER.to_uppercase());
        let sk = section_key.clone().unwrap_or_else(|| NO_SECTION.to_uppercase());
        let entry = buckets.entry((tk, sk)).or_insert_with(|| Bucket {
            // The label is the FIRST source value that landed in this bucket -
            // the page's own wording, not the upper-cased key.
            tier_label: l.tier.clone().unwrap_or_else(|| UNCLASSIFIED_TIER.to_string()),
            section_label: l.section.clone().unwrap_or_else(|| NO_SECTION.to_string()),
            has_section,
            listings: Vec::new(),
            mine: Vec::new(),
        });
        entry.listings.push(MarketMapListing {
            marketplace: l.marketplace.clone(),
            listing_id: l.listing_id.clone(),
            row: l.row.clone(),
            row_key: normalize_row(l.row.as_deref()),
            price_cents: l.price_cents,
            currency: l.currency.clone(),
            quantity: l.quantity,
            incomplete: l.incomplete,
        });
    }

    for t in mine {
        let tier_key = normalize_tier(t.tier.as_deref());
        let section_key = normalize_section(t.section.as_deref());
        let has_section = section_key.is_some();
        let tk = tier_key.clone().unwrap_or_else(|| UNCLASSIFIED_TIER.to_uppercase());
        let sk = section_key.clone().unwrap_or_else(|| NO_SECTION.to_uppercase());
        let entry = buckets.entry((tk, sk)).or_insert_with(|| Bucket {
            tier_label: t.tier.clone().unwrap_or_else(|| UNCLASSIFIED_TIER.to_string()),
            section_label: t.section.clone().unwrap_or_else(|| NO_SECTION.to_string()),
            has_section,
            listings: Vec::new(),
            mine: Vec::new(),
        });
        entry.mine.push(MarketMapMyTicket {
            ticket_id: t.id,
            order_id: t.order_id,
            code: t.code.clone(),
            row: t.row_label.clone(),
            seat: t.seat.clone(),
            listing_price_cents: t.listing_price_cents,
            currency: t.currency.clone(),
            status: t.status.clone(),
        });
    }

    // Roll the flat buckets up into tiers.
    let mut by_tier: BTreeMap<String, (String, Vec<MarketMapSection>)> = BTreeMap::new();
    let mut total_listings: i64 = 0;
    let mut total_mine: i64 = 0;
    let mut without_section: i64 = 0;

    for ((tier_key, section_key), b) in buckets {
        let mut prices: Vec<i64> = b.listings.iter().map(|l| l.price_cents).collect();
        prices.sort_unstable();
        total_listings += b.listings.len() as i64;
        total_mine += b.mine.len() as i64;
        if !b.has_section {
            without_section += b.listings.len() as i64;
        }
        // A section whose listings span more than one currency reports no
        // lowest/median/highest at all - the same "never blend currencies"
        // rule the rest of the app follows, rather than a number that is the
        // minimum of two different moneys.
        let currencies: std::collections::BTreeSet<String> = b
            .listings
            .iter()
            .filter_map(|l| l.currency.clone())
            .collect();
        let mixed = currencies.len() > 1;
        let single_currency = if currencies.len() == 1 { currencies.into_iter().next() } else { None };
        let listing_count = b.listings.len() as i64;
        let my_ticket_count = b.mine.len() as i64;
        let lowest = if mixed { None } else { prices.first().copied() };
        let median = if mixed { None } else { median_of(&prices) };
        let highest = if mixed { None } else { prices.last().copied() };
        let section = MarketMapSection {
            key: section_key,
            label: b.section_label,
            has_section: b.has_section,
            listing_count,
            my_ticket_count,
            currency: single_currency,
            mixed_currencies: mixed,
            lowest_price_cents: lowest,
            median_price_cents: median,
            highest_price_cents: highest,
            listings: b.listings,
            my_tickets: b.mine,
        };
        by_tier
            .entry(tier_key)
            .or_insert_with(|| (b.tier_label, Vec::new()))
            .1
            .push(section);
    }

    let mut tiers: Vec<MarketMapTier> = by_tier
        .into_iter()
        .map(|(key, (label, mut sections))| {
            sections.sort_by(|a, b| order_key(&a.key).cmp(&order_key(&b.key)));
            MarketMapTier {
                listing_count: sections.iter().map(|s| s.listing_count).sum(),
                my_ticket_count: sections.iter().map(|s| s.my_ticket_count).sum(),
                key,
                label,
                sections,
            }
        })
        .collect();
    tiers.sort_by(|a, b| order_key(&a.key).cmp(&order_key(&b.key)));

    let marketplaces: Vec<String> = {
        let set: std::collections::BTreeSet<String> =
            listings.iter().map(|l| l.marketplace.clone()).collect();
        set.into_iter().collect()
    };

    MarketMap {
        tiers,
        total_listings,
        total_my_tickets: total_mine,
        listings_without_section: without_section,
        // True when the scan produced prices but nothing this map can lay out.
        // The UI shows an honest "no section data in this scan" state on it
        // rather than drawing an empty building.
        has_section_data: total_listings > without_section,
        marketplaces,
    }
}

fn read_my_tickets(conn: &Connection, event_id: i64) -> AppResult<Vec<MyTicketRow>> {
    // Same scope `price_checker_analysis::your_tickets` already uses -
    // available/listed only, i.e. what is still marko's to sell. Unlike that
    // function this reads ROWS, not groups, because the map has to link each
    // one back to its own order detail.
    let mut stmt = conn.prepare(
        "SELECT id, order_id, code, section, row_label, seat, tier, listing_price_cents, currency, status
           FROM tickets
          WHERE event_id = ?1 AND status IN ('available','listed')
          ORDER BY section, row_label, seat, id",
    )?;
    let rows = stmt.query_map([event_id], |r| {
        Ok(MyTicketRow {
            id: r.get(0)?,
            order_id: r.get(1)?,
            code: r.get(2)?,
            section: r.get(3)?,
            row_label: r.get(4)?,
            seat: r.get(5)?,
            tier: r.get(6)?,
            listing_price_cents: r.get(7)?,
            currency: r.get(8)?,
            status: r.get(9)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Read-only. Takes the two locks one at a time and never together - the same
/// deadlock-safety rule `compute_market_analysis` documents next door.
#[tauri::command(async)]
pub fn compute_market_map(state: State<'_, AppState>, request_id: u64, event_id: i64) -> AppResult<MarketMap> {
    let listings: Vec<NormalizedListing> = {
        let sessions = state.price_scanner_sessions.lock().unwrap();
        let session = sessions
            .get(&request_id)
            .ok_or_else(|| AppError::NotFound("Scanner session not found - the window may have been closed".into()))?;
        session.listings.clone()
    };
    let mine = {
        let conn = state.db.lock().unwrap();
        read_my_tickets(&conn, event_id)?
    };
    Ok(build_market_map(&listings, &mine))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn listing(marketplace: &str, price: i64, section: Option<&str>, row: Option<&str>, tier: Option<&str>) -> NormalizedListing {
        NormalizedListing {
            price_cents: price,
            currency: Some("EUR".to_string()),
            section: section.map(|s| s.to_string()),
            row: row.map(|s| s.to_string()),
            tier: tier.map(|s| s.to_string()),
            quantity: Some(2),
            listing_id: None,
            incomplete: false,
            marketplace: marketplace.to_string(),
        }
    }

    fn mine(id: i64, section: Option<&str>, row: Option<&str>, tier: Option<&str>) -> MyTicketRow {
        MyTicketRow {
            id,
            order_id: 7,
            code: format!("TKT-{id:06}"),
            section: section.map(|s| s.to_string()),
            row_label: row.map(|s| s.to_string()),
            seat: Some("12".to_string()),
            tier: tier.map(|s| s.to_string()),
            listing_price_cents: Some(9000),
            currency: "EUR".to_string(),
            status: "listed".to_string(),
        }
    }

    #[test]
    fn section_normalization_only_does_what_is_safe() {
        // The three spellings marko named explicitly.
        assert_eq!(normalize_section(Some("Sec 102")).as_deref(), Some("102"));
        assert_eq!(normalize_section(Some("Section 102")).as_deref(), Some("102"));
        assert_eq!(normalize_section(Some("102")).as_deref(), Some("102"));
        assert_eq!(normalize_section(Some("  102  ")).as_deref(), Some("102"));
        // Leading zeroes only when what is left is entirely digits.
        assert_eq!(normalize_section(Some("0102")).as_deref(), Some("102"));
        assert_eq!(normalize_section(Some("0A")).as_deref(), Some("0A"));
        assert_eq!(normalize_section(Some("000")).as_deref(), Some("0"));
        // Anything that is not safely reducible keeps its own identity.
        assert_eq!(normalize_section(Some("FLOOR A")).as_deref(), Some("FLOOR A"));
        assert_eq!(normalize_section(Some("N23")).as_deref(), Some("N23"));
        // Nothing to group by is None, never a placeholder.
        assert_eq!(normalize_section(None), None);
        assert_eq!(normalize_section(Some("   ")), None);
        // A bare header word is not a section name.
        assert_eq!(normalize_section(Some("Section")).as_deref(), Some("SECTION"));
        assert_eq!(normalize_section(Some("Section ")).as_deref(), Some("SECTION"));
    }

    #[test]
    fn two_spellings_of_one_section_become_one_block_and_keep_a_real_label() {
        let map = build_market_map(
            &[
                listing("viagogo", 18000, Some("Sec 102"), Some("12"), Some("Level 100")),
                listing("vividseats", 19500, Some("Section 102"), Some("14"), Some("Level 100")),
                listing("ticombo", 17500, Some("102"), Some("18"), Some("Level 100")),
            ],
            &[],
        );
        assert_eq!(map.tiers.len(), 1);
        assert_eq!(map.tiers[0].sections.len(), 1, "three spellings, one section block");
        let s = &map.tiers[0].sections[0];
        assert_eq!(s.key, "102");
        // The label is a real source value, never the upper-cased key.
        assert_eq!(s.label, "Sec 102");
        assert_eq!(s.listing_count, 3);
        assert_eq!(s.lowest_price_cents, Some(17500));
        assert_eq!(s.median_price_cents, Some(18000));
        assert_eq!(s.highest_price_cents, Some(19500));
        assert_eq!(map.marketplaces, vec!["ticombo", "viagogo", "vividseats"]);
    }

    #[test]
    fn tier_wording_is_never_mapped_or_invented() {
        let map = build_market_map(
            &[
                listing("viagogo", 100, Some("101"), None, Some("Level 100")),
                listing("viagogo", 100, Some("201"), None, Some("Tier 1")),
                listing("viagogo", 100, Some("301"), None, None),
            ],
            &[],
        );
        let labels: Vec<&str> = map.tiers.iter().map(|t| t.label.as_str()).collect();
        // "Level 100" and "Tier 1" are NOT merged - this module has no way to
        // know whether they mean the same thing.
        assert!(labels.contains(&"Level 100"));
        assert!(labels.contains(&"Tier 1"));
        assert!(labels.contains(&"Unclassified"));
        assert_eq!(map.tiers.len(), 3);
    }

    #[test]
    fn row_normalization_matches_the_section_rule() {
        assert_eq!(normalize_row(Some("Row 7")).as_deref(), Some("7"));
        assert_eq!(normalize_row(Some("07")).as_deref(), Some("7"));
        assert_eq!(normalize_row(Some("AA")).as_deref(), Some("AA"));
        assert_eq!(normalize_row(Some(" 12 ")).as_deref(), Some("12"));
        assert_eq!(normalize_row(None), None);
    }

    #[test]
    fn listings_without_a_section_go_to_their_own_labelled_bucket_never_a_made_up_one() {
        let map = build_market_map(
            &[
                listing("viagogo", 12000, None, None, None),
                listing("viagogo", 13000, None, None, None),
                listing("viagogo", 14000, Some("102"), None, None),
            ],
            &[],
        );
        assert_eq!(map.total_listings, 3);
        assert_eq!(map.listings_without_section, 2);
        assert!(map.has_section_data, "one real section is still section data");
        let no_section = map.tiers[0]
            .sections
            .iter()
            .find(|s| !s.has_section)
            .expect("the no-section bucket exists");
        assert_eq!(no_section.label, NO_SECTION);
        assert_eq!(no_section.listing_count, 2);
    }

    #[test]
    fn a_scan_with_prices_but_no_sections_at_all_reports_no_section_data() {
        let map = build_market_map(
            &[listing("ticombo", 5000, None, None, None), listing("ticombo", 6000, None, None, None)],
            &[],
        );
        assert!(!map.has_section_data, "the UI must be able to say section data is unavailable");
        assert_eq!(map.listings_without_section, 2);
    }

    #[test]
    fn my_tickets_land_in_the_same_section_as_the_market_and_carry_their_ids() {
        let map = build_market_map(
            &[
                listing("viagogo", 18000, Some("Section 102"), Some("12"), Some("Level 100")),
                listing("vividseats", 19500, Some("102"), Some("14"), Some("Level 100")),
            ],
            &[mine(41, Some("Sec 102"), Some("18"), Some("Level 100")), mine(42, Some("102"), Some("18"), Some("Level 100"))],
        );
        let s = &map.tiers[0].sections[0];
        assert_eq!(s.listing_count, 2);
        assert_eq!(s.my_ticket_count, 2, "marko's own two tickets are on the same block");
        // Counted separately, never folded into the market count.
        assert_eq!(map.total_listings, 2);
        assert_eq!(map.total_my_tickets, 2);
        // Ids are carried so the UI can open the existing order detail.
        assert_eq!(s.my_tickets[0].ticket_id, 41);
        assert_eq!(s.my_tickets[0].order_id, 7);
    }

    #[test]
    fn a_section_where_only_marko_holds_tickets_still_appears() {
        let map = build_market_map(
            &[listing("viagogo", 18000, Some("102"), None, None)],
            &[mine(9, Some("305"), Some("2"), None)],
        );
        let sections = &map.tiers[0].sections;
        let only_mine = sections.iter().find(|s| s.key == "305").expect("section 305 exists");
        assert_eq!(only_mine.listing_count, 0);
        assert_eq!(only_mine.my_ticket_count, 1);
        assert_eq!(only_mine.lowest_price_cents, None, "no market listings means no market price");
    }

    #[test]
    fn a_section_mixing_currencies_reports_no_prices_rather_than_blending_them() {
        let mut usd = listing("viagogo", 20000, Some("102"), None, None);
        usd.currency = Some("USD".to_string());
        let map = build_market_map(&[listing("viagogo", 18000, Some("102"), None, None), usd], &[]);
        let s = &map.tiers[0].sections[0];
        assert!(s.mixed_currencies);
        assert_eq!(s.lowest_price_cents, None);
        assert_eq!(s.median_price_cents, None);
        assert_eq!(s.highest_price_cents, None);
        assert_eq!(s.listing_count, 2, "the listings are still there, only the blended stat is withheld");
    }

    #[test]
    fn section_order_is_numeric_then_alphabetical_and_stable() {
        let map = build_market_map(
            &[
                listing("viagogo", 100, Some("1002"), None, Some("A")),
                listing("viagogo", 100, Some("102"), None, Some("A")),
                listing("viagogo", 100, Some("FLOOR"), None, Some("A")),
                listing("viagogo", 100, Some("2"), None, Some("A")),
            ],
            &[],
        );
        let keys: Vec<&str> = map.tiers[0].sections.iter().map(|s| s.key.as_str()).collect();
        assert_eq!(keys, vec!["2", "102", "1002", "FLOOR"]);
    }

    #[test]
    fn several_marketplaces_share_one_section_without_being_merged_into_one_listing() {
        let map = build_market_map(
            &[
                listing("viagogo", 18000, Some("102"), Some("12"), None),
                listing("vividseats", 19500, Some("102"), Some("14"), None),
                listing("ticombo", 17500, Some("102"), Some("18"), None),
            ],
            &[],
        );
        let s = &map.tiers[0].sections[0];
        assert_eq!(s.listings.len(), 3);
        let mps: Vec<&str> = s.listings.iter().map(|l| l.marketplace.as_str()).collect();
        assert!(mps.contains(&"viagogo") && mps.contains(&"vividseats") && mps.contains(&"ticombo"));
    }

    #[test]
    fn an_empty_session_produces_an_empty_map_not_an_error() {
        let map = build_market_map(&[], &[]);
        assert!(map.tiers.is_empty());
        assert_eq!(map.total_listings, 0);
        assert!(!map.has_section_data);
    }

    #[test]
    fn median_of_an_even_count_is_the_mean_of_the_middle_two() {
        let map = build_market_map(
            &[
                listing("viagogo", 1000, Some("1"), None, None),
                listing("viagogo", 2000, Some("1"), None, None),
                listing("viagogo", 3000, Some("1"), None, None),
                listing("viagogo", 5000, Some("1"), None, None),
            ],
            &[],
        );
        assert_eq!(map.tiers[0].sections[0].median_price_cents, Some(2500));
    }
}

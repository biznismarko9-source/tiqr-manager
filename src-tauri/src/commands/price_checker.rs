//! Price Checker (2.0.81) - marko's own new top-level section: "Chcem teraz
//! pridať úplne novú sekciu do appky s názvom Price Checker" (I want to add
//! a whole new section to the app called Price Checker), scoped per Event,
//! for comparing his own unsold inventory against the going market rate on
//! StubHub / Vivid Seats / Ticombo.
//!
//! Deliberately manual-entry only, not live scraping/API - marko's own
//! instruction was explicit: "ak automatické načítanie cien z niektorej
//! stránky bez API nie je technicky alebo podľa jej podmienok možné, navrhni
//! fallback cez manuálne zadanie ceny/linku namiesto obchádzania ochrany"
//! (if automatic price-fetching from a site isn't technically or ToS-possible
//! without an API, propose a manual-entry fallback instead of bypassing that
//! site's protection). Researched before building, not assumed: neither
//! StubHub nor Vivid Seats offers a public read API to an individual seller
//! (Vivid Seats' API is partner/approved-seller only), StubHub actively
//! blocks casual scraping, and Ticombo has no public API either - so this
//! whole feature is built around marko typing in what he sees on each site
//! himself for all three from day one, rather than "automate until it's
//! blocked, then fall back."
//!
//! Three tables (see migrations/014_price_checker.sql for the full schema
//! reasoning): `marketplaces` is a plain lookup table (same pattern as
//! `platforms`/`suppliers`/`event_categories` - marko manages the list
//! himself, seeded with his own 3), `event_marketplace_links` holds one
//! saved URL per (event, marketplace), and `price_checks` is an APPEND-ONLY
//! history of what marko typed in after looking at a marketplace's listings
//! page - marko explicitly asked to keep old checks too ("nech si aj pamata
//! stare aby sa vedela porovnat cena ci sla hore alebo dole" - let it
//! remember old ones too so it can compare whether the price went up or
//! down), so a later check never overwrites an earlier one.
//!
//! `get_price_checker_summary_impl` is the one real piece of new business
//! logic here: it assembles the whole page for one event in a single round
//! trip (every marketplace's link + full history, marko's own
//! unsold-inventory cost/listing price, and the derived market comparison),
//! matching marko's own list of fields to show: "moja priemerná nákupná
//! cena, aktuálne listing ceny, najnižšia trhová cena, priemerná trhová
//! cena, recommended price, expected profit, expected ROI".
//! `recommended_price_cents` uses marko's own answer for how that should
//! work - "Mierne pod najnižšou trhovou cenou" (slightly under the lowest
//! market price) - a plain, transparent percentage (see
//! `RECOMMENDED_PRICE_UNDERCUT_PCT`), not AI, per his explicit "Recommended
//! price nech je zatiaľ jednoduchý transparentný výpočet podľa existujúcich
//! dát, nie AI."
//!
//! 2.1.6: marko asked to replace StubHub with Viagogo ("stubhub by som chcel
//! zmenit za viagogo.com"), but also to keep StubHub's own history readable
//! (confirmed via AskUserQuestion) - so `marketplaces` gained an `active`
//! column (migrations/017_price_checker_viagogo.sql) rather than StubHub's
//! row being renamed or deleted. `active = false` only ever means "not
//! offered for something NEW"; every existing link/check against it is
//! completely untouched, and `get_price_checker_summary_impl` below still
//! shows it for exactly the events that already have one or the other - see
//! that function's own doc comment.

use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::finance;
use crate::models::{
    EventMarketplaceLink, EventMarketplaceLinkInput, Marketplace, MarketplacePriceView, PriceCheck,
    PriceCheckInput, PriceCheckerSummary,
};
use rusqlite::{params, Connection, OptionalExtension, Row};
use tauri::State;

/// Undercut applied to `market_lowest_price_cents` to produce
/// `recommended_price_cents` - marko's own answer, via AskUserQuestion, when
/// asked how "recommended price" should work: "Mierne pod najnižšou trhovou
/// cenou" (slightly under the lowest market price). A plain named constant,
/// not a setting - marko asked for "jednoduchý transparentný výpočet ...
/// nie AI" (a simple transparent calculation, not AI), so keeping the whole
/// formula as one visible line here is deliberate; if he later wants this
/// tunable from the UI, this constant is the one thing that becomes a
/// parameter.
pub(crate) const RECOMMENDED_PRICE_UNDERCUT_PCT: f64 = 0.05;

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

fn map_price_check(row: &Row) -> rusqlite::Result<PriceCheck> {
    Ok(PriceCheck {
        id: row.get("id")?,
        event_id: row.get("event_id")?,
        marketplace_id: row.get("marketplace_id")?,
        lowest_price_cents: row.get("lowest_price_cents")?,
        average_price_cents: row.get("average_price_cents")?,
        highest_price_cents: row.get("highest_price_cents")?,
        listing_count: row.get("listing_count")?,
        currency: row.get("currency")?,
        median_price_cents: row.get("median_price_cents")?,
        checked_at: row.get("checked_at")?,
    })
}

// ---------------------------------------------------------------------------
// marketplaces - plain lookup table CRUD, same shape (and same "no logic
// beyond a straight insert/delete worth unit-testing on its own" reasoning)
// as lookups::list_platforms/create_platform/delete_platform. Marko manages
// this list himself from the app - adding a 4th/5th marketplace later never
// needs a new migration.
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_marketplaces(state: State<AppState>) -> AppResult<Vec<Marketplace>> {
    let conn = state.db.lock().unwrap();
    let mut stmt = conn.prepare("SELECT * FROM marketplaces ORDER BY name COLLATE NOCASE")?;
    let rows = stmt.query_map([], map_marketplace)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[tauri::command]
pub fn create_marketplace(state: State<AppState>, name: String) -> AppResult<Marketplace> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::Validation("Marketplace name cannot be empty".into()));
    }
    let conn = state.db.lock().unwrap();
    conn.execute("INSERT INTO marketplaces(name) VALUES (?1)", params![name])
        .map_err(|e| match &e {
            rusqlite::Error::SqliteFailure(_, Some(m)) if m.contains("UNIQUE") => {
                AppError::Validation(format!("Marketplace '{name}' already exists"))
            }
            _ => AppError::from(e),
        })?;
    let id = conn.last_insert_rowid();
    Ok(conn.query_row("SELECT * FROM marketplaces WHERE id = ?1", [id], map_marketplace)?)
}

/// Core logic behind `delete_marketplace` - split out for direct
/// unit-testability, same pattern as every other `_impl` function in this
/// module. 2.1.6: refuses to delete a marketplace that still has saved links
/// or price-check history - both foreign keys are `ON DELETE CASCADE`
/// (migrations/014_price_checker.sql), so before this guard existed a plain
/// delete would have silently wiped that history along with the row. No UI
/// currently calls this command at all (marko manages retirement via the
/// `active` flag instead - see this module's own doc comment), but the
/// command stays exposed for whenever one does, and it should never be able
/// to do by accident what marko explicitly asked NOT to happen to StubHub's
/// own history.
pub(crate) fn delete_marketplace_impl(conn: &Connection, id: i64) -> AppResult<()> {
    let history_count: i64 = conn.query_row(
        "SELECT (SELECT COUNT(*) FROM event_marketplace_links WHERE marketplace_id = ?1)
                + (SELECT COUNT(*) FROM price_checks WHERE marketplace_id = ?1)",
        [id],
        |r| r.get(0),
    )?;
    if history_count > 0 {
        return Err(AppError::Validation(
            "This marketplace still has saved links or price-check history, so it can't be deleted. \
             If you just don't want to use it for new checks anymore, it's fine to leave it as-is."
                .into(),
        ));
    }
    conn.execute("DELETE FROM marketplaces WHERE id = ?1", [id])?;
    Ok(())
}

#[tauri::command]
pub fn delete_marketplace(state: State<AppState>, id: i64) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    delete_marketplace_impl(&conn, id)
}

/// Shared guard for `save_event_marketplace_link_impl` and
/// `save_price_check_impl` below - 2.1.6: a retired marketplace (StubHub,
/// since Viagogo replaced it - see this module's own doc comment) must stay
/// strictly read-only for anything NEW, matching marko's own explicit
/// choice when asked ("StubHub stays visible read-only for events that
/// already have data against it; new checks go through Viagogo only").
///
/// MarketplaceCard (PriceChecker.tsx) is the primary defense - it hides the
/// "Check Prices"/"Auto-check" actions once `marketplaceActive` is false,
/// so a normal click can't reach either command for a retired marketplace
/// at all. This is the same second layer already applied to
/// `delete_marketplace_impl` above, for the same reason: the backend must
/// never be able to do what marko explicitly asked NOT to happen just
/// because a future UI change lets a button through it shouldn't.
///
/// Deliberately NOT extended to `open_price_scanner`/`scan_visible_prices`
/// (commands::price_checker_scanner) - those commands only ever take a raw
/// URL, never a marketplace_id, so they have nothing to check active
/// against, and opening a scanner window against a retired marketplace can
/// waste marko's own time but can never itself write a row - the actual
/// save afterward still goes through this same guard.
fn require_marketplace_active(conn: &Connection, marketplace_id: i64) -> AppResult<()> {
    let row: Option<(String, bool)> = conn
        .query_row(
            "SELECT name, active FROM marketplaces WHERE id = ?1",
            [marketplace_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    match row {
        None => Err(AppError::NotFound("Marketplace not found".into())),
        Some((_, true)) => Ok(()),
        Some((name, false)) => Err(AppError::Validation(format!(
            "{name} isn't used for new checks anymore - its saved history stays visible, but new links or price checks aren't accepted."
        ))),
    }
}

// ---------------------------------------------------------------------------
// event_marketplace_links - one saved URL per (event, marketplace).
// ---------------------------------------------------------------------------

/// Core logic behind `save_event_marketplace_link` - split out for direct
/// unit-testability (same "impl function + thin `#[tauri::command]` wrapper"
/// pattern used throughout this codebase). A blank/whitespace-only `url`
/// means "clear this marketplace's link" - deletes the row (if any) and
/// returns `None`, rather than storing an empty string; a non-blank `url`
/// upserts it via the table's own `UNIQUE(event_id, marketplace_id)`, so
/// saving over an existing link always updates it in place instead of
/// erroring or duplicating.
pub(crate) fn save_event_marketplace_link_impl(
    conn: &Connection,
    input: &EventMarketplaceLinkInput,
) -> AppResult<Option<EventMarketplaceLink>> {
    let url = input.url.trim();
    if url.is_empty() {
        conn.execute(
            "DELETE FROM event_marketplace_links WHERE event_id = ?1 AND marketplace_id = ?2",
            params![input.event_id, input.marketplace_id],
        )?;
        return Ok(None);
    }
    // 2.1.6: only guarded here, not above - clearing an existing link (the
    // blank-url branch just above) is always allowed even for a retired
    // marketplace, since removing data is never what "read-only" was meant
    // to prevent; only saving/changing to a NEW non-blank url is.
    require_marketplace_active(conn, input.marketplace_id)?;
    conn.execute(
        "INSERT INTO event_marketplace_links(event_id, marketplace_id, url) VALUES (?1, ?2, ?3)
         ON CONFLICT(event_id, marketplace_id) DO UPDATE SET
           url = excluded.url,
           updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')",
        params![input.event_id, input.marketplace_id, url],
    )?;
    Ok(Some(conn.query_row(
        "SELECT * FROM event_marketplace_links WHERE event_id = ?1 AND marketplace_id = ?2",
        params![input.event_id, input.marketplace_id],
        map_link,
    )?))
}

#[tauri::command]
pub fn save_event_marketplace_link(
    state: State<AppState>,
    input: EventMarketplaceLinkInput,
) -> AppResult<Option<EventMarketplaceLink>> {
    let conn = state.db.lock().unwrap();
    save_event_marketplace_link_impl(&conn, &input)
}

// ---------------------------------------------------------------------------
// price_checks - append-only history of manually-entered "Check Prices"
// results.
// ---------------------------------------------------------------------------

fn validate_price_check_input(input: &PriceCheckInput) -> AppResult<()> {
    if input.lowest_price_cents < 0 || input.average_price_cents < 0 || input.highest_price_cents < 0 {
        return Err(AppError::Validation("Prices cannot be negative".into()));
    }
    if input.listing_count < 0 {
        return Err(AppError::Validation("Listing count cannot be negative".into()));
    }
    if input.lowest_price_cents > input.average_price_cents {
        return Err(AppError::Validation(
            "Lowest price cannot be higher than the average price".into(),
        ));
    }
    if input.average_price_cents > input.highest_price_cents {
        return Err(AppError::Validation(
            "Average price cannot be higher than the highest price".into(),
        ));
    }
    // 2.1.9: median is optional (Visible Scanner sessions always supply it;
    // manual/pasted entries typically won't - see PriceCheckInput's own doc
    // comment), so only bother checking it against the other three when it's
    // actually present.
    if let Some(median) = input.median_price_cents {
        if median < 0 {
            return Err(AppError::Validation("Median price cannot be negative".into()));
        }
        if median < input.lowest_price_cents || median > input.highest_price_cents {
            return Err(AppError::Validation(
                "Median price must be between the lowest and highest price".into(),
            ));
        }
    }
    if input.currency.trim().is_empty() {
        return Err(AppError::Validation("Currency is required".into()));
    }
    Ok(())
}

/// Core logic behind `save_price_check` - split out for direct
/// unit-testability. Always a plain INSERT, never an update: see
/// migrations/014_price_checker.sql's doc comment on `price_checks` for why
/// this table is append-only (marko explicitly wants to see whether the
/// market moved up or down since the last check, not just the latest
/// snapshot).
pub(crate) fn save_price_check_impl(conn: &Connection, input: &PriceCheckInput) -> AppResult<PriceCheck> {
    validate_price_check_input(input)?;
    // 2.1.6: unlike the link table above, price_checks is append-only and
    // has no "clearing" concept - every call here adds a brand-new row, so
    // this guard applies unconditionally (see require_marketplace_active's
    // own doc comment).
    require_marketplace_active(conn, input.marketplace_id)?;
    let currency = input.currency.trim();
    conn.execute(
        "INSERT INTO price_checks
           (event_id, marketplace_id, lowest_price_cents, average_price_cents, highest_price_cents, listing_count, currency, median_price_cents)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            input.event_id,
            input.marketplace_id,
            input.lowest_price_cents,
            input.average_price_cents,
            input.highest_price_cents,
            input.listing_count,
            currency,
            input.median_price_cents,
        ],
    )?;
    let id = conn.last_insert_rowid();
    Ok(conn.query_row("SELECT * FROM price_checks WHERE id = ?1", [id], map_price_check)?)
}

#[tauri::command]
pub fn save_price_check(state: State<AppState>, input: PriceCheckInput) -> AppResult<PriceCheck> {
    let conn = state.db.lock().unwrap();
    save_price_check_impl(&conn, &input)
}

// ---------------------------------------------------------------------------
// The whole Price Checker page for one event, in one round trip.
// ---------------------------------------------------------------------------

/// Core logic behind `get_price_checker_summary` - see this module's own doc
/// comment for the overall design, and `PriceCheckerSummary`'s doc comment
/// (models.rs) for exactly what each field means and when it's `None`.
///
/// Every ACTIVE marketplace always appears in the result, even one marko
/// has never linked or checked for this event - so the page is always a
/// place to add data, not just a report of what's already filled in. A
/// retired (inactive) marketplace - StubHub as of 2.1.6 - appears too, but
/// ONLY for an event that already has a link or check history against it;
/// see the marketplace-selection query below and `Marketplace::active`'s
/// own doc comment (models.rs).
///
/// "My" figures (avg purchase cost / avg listing price) are computed over
/// the exact same ticket scope as Event Detail's own "Potential profit"
/// block - `available`/`listed` tickets only, i.e. not yet sold and not
/// cancelled - and are always returned even when marko's unsold inventory
/// for this event mixes currencies (`my_currency` is `None` in that case);
/// same "blended figure + a currency flag, let the UI decide whether to
/// show Mixed" convention `total_cost_cents`/`formatMoneyOrMixed` already
/// use everywhere else in this app. The market comparison below is
/// different: it genuinely needs ONE real currency to pick which
/// marketplace checks are even comparable, so it (and everything derived
/// from it - recommended price, expected profit, expected ROI) is `None`
/// whenever `my_currency` is `None`.
pub(crate) fn get_price_checker_summary_impl(conn: &Connection, event_id: i64) -> AppResult<PriceCheckerSummary> {
    let (event_name, event_date): (String, Option<String>) = conn
        .query_row("SELECT name, event_date FROM events WHERE id = ?1", [event_id], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .map_err(|_| AppError::NotFound(format!("Event #{event_id} not found")))?;

    // 2.1.6: every ACTIVE marketplace always appears (unchanged behavior),
    // plus any INACTIVE one (StubHub today) that this specific event already
    // has a saved link or check history against - so a retired marketplace
    // keeps showing its real past data exactly where marko left it, but
    // never gets offered to a fresh event as somewhere new to start. See
    // this module's own doc comment and Marketplace::active's doc comment
    // (models.rs).
    let marketplaces: Vec<Marketplace> = {
        let mut stmt = conn.prepare(
            "SELECT * FROM marketplaces
             WHERE active = 1
                OR EXISTS(SELECT 1 FROM event_marketplace_links WHERE event_id = ?1 AND marketplace_id = marketplaces.id)
                OR EXISTS(SELECT 1 FROM price_checks WHERE event_id = ?1 AND marketplace_id = marketplaces.id)
             ORDER BY name COLLATE NOCASE",
        )?;
        let rows = stmt.query_map([event_id], map_marketplace)?;
        rows.collect::<Result<Vec<_>, _>>()?
    };

    // ---- marko's own unsold inventory for this event ----------------------
    let (unsold_ticket_count, currency_count, min_currency, cost_sum_cents, listing_priced_count, listing_sum_cents): (
        i64,
        i64,
        Option<String>,
        i64,
        i64,
        i64,
    ) = conn.query_row(
        "SELECT
            COUNT(*),
            COUNT(DISTINCT currency),
            MIN(currency),
            COALESCE(SUM(purchase_cost_cents + purchase_fees_cents + other_costs_cents), 0),
            COUNT(CASE WHEN listing_price_cents IS NOT NULL THEN 1 END),
            COALESCE(SUM(listing_price_cents), 0)
         FROM tickets
         WHERE event_id = ?1 AND status IN ('available','listed')",
        [event_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
    )?;
    let my_currency = if currency_count <= 1 { min_currency } else { None };
    let missing_listing_price_count = unsold_ticket_count - listing_priced_count;
    let my_avg_purchase_cost_cents =
        finance::safe_ratio(cost_sum_cents, unsold_ticket_count).map(|v| v.round() as i64);
    let my_avg_listing_price_cents =
        finance::safe_ratio(listing_sum_cents, listing_priced_count).map(|v| v.round() as i64);

    // ---- per-marketplace link + full history, newest first ---------------
    let mut views: Vec<MarketplacePriceView> = Vec::with_capacity(marketplaces.len());
    for m in &marketplaces {
        let link: Option<EventMarketplaceLink> = conn
            .query_row(
                "SELECT * FROM event_marketplace_links WHERE event_id = ?1 AND marketplace_id = ?2",
                params![event_id, m.id],
                map_link,
            )
            .optional()?;
        let history: Vec<PriceCheck> = {
            let mut stmt = conn.prepare(
                "SELECT * FROM price_checks WHERE event_id = ?1 AND marketplace_id = ?2 ORDER BY id DESC",
            )?;
            let rows = stmt.query_map(params![event_id, m.id], map_price_check)?;
            rows.collect::<Result<Vec<_>, _>>()?
        };
        views.push(MarketplacePriceView {
            marketplace_id: m.id,
            marketplace_name: m.name.clone(),
            marketplace_active: m.active,
            link,
            history,
        });
    }

    // ---- market-wide comparison -------------------------------------------
    // Only across each marketplace's LATEST check (history[0]), and only
    // those that share marko's own unsold-inventory currency - see this
    // function's own doc comment and migrations/014_price_checker.sql for
    // why. A marketplace with no checks yet, or whose latest check is in a
    // different currency, simply doesn't contribute.
    let (market_lowest_price_cents, market_average_price_cents) = match &my_currency {
        Some(cur) => {
            let latest_matching: Vec<&PriceCheck> =
                views.iter().filter_map(|v| v.history.first()).filter(|c| &c.currency == cur).collect();
            if latest_matching.is_empty() {
                (None, None)
            } else {
                let lowest = latest_matching.iter().map(|c| c.lowest_price_cents).min();
                let avg_sum: i64 = latest_matching.iter().map(|c| c.average_price_cents).sum();
                let average = finance::safe_ratio(avg_sum, latest_matching.len() as i64).map(|v| v.round() as i64);
                (lowest, average)
            }
        }
        None => (None, None),
    };

    let recommended_price_cents = market_lowest_price_cents
        .map(|lowest| (lowest as f64 * (1.0 - RECOMMENDED_PRICE_UNDERCUT_PCT)).round() as i64);
    let expected_profit_cents = match (recommended_price_cents, my_avg_purchase_cost_cents) {
        (Some(recommended), Some(cost)) => Some(recommended - cost),
        _ => None,
    };
    let expected_roi = match (expected_profit_cents, my_avg_purchase_cost_cents) {
        (Some(profit), Some(cost)) => finance::safe_ratio(profit, cost),
        _ => None,
    };

    Ok(PriceCheckerSummary {
        event_id,
        event_name,
        event_date,
        marketplaces: views,
        my_currency,
        unsold_ticket_count,
        my_avg_purchase_cost_cents,
        my_avg_listing_price_cents,
        missing_listing_price_count,
        market_lowest_price_cents,
        market_average_price_cents,
        recommended_price_cents,
        expected_profit_cents,
        expected_roi,
    })
}

#[tauri::command]
pub fn get_price_checker_summary(state: State<AppState>, event_id: i64) -> AppResult<PriceCheckerSummary> {
    let conn = state.db.lock().unwrap();
    get_price_checker_summary_impl(&conn, event_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_conn;

    // migrations/014_price_checker.sql seeds exactly StubHub/Vivid Seats/
    // Ticombo in every freshly-migrated connection, including test_conn()'s.

    fn seed_event(conn: &Connection, name: &str) -> i64 {
        conn.execute("INSERT INTO events (name) VALUES (?1)", [name]).unwrap();
        conn.last_insert_rowid()
    }

    fn marketplace_id_by_name(conn: &Connection, name: &str) -> i64 {
        conn.query_row("SELECT id FROM marketplaces WHERE name = ?1", [name], |r| r.get(0)).unwrap()
    }

    fn seed_order_only(conn: &Connection, code_suffix: &str, event_id: i64) -> i64 {
        conn.execute(
            "INSERT INTO orders (code, event_id, purchase_date, quantity, currency)
             VALUES (?1, ?2, '2026-01-01', 1, 'EUR')",
            params![format!("ORD-{code_suffix}"), event_id],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    /// Same shape as dashboard.rs's own `seed_ticket` test helper - one
    /// order + one ticket, full control over status/currency/cost/listing
    /// price. `code_suffix` must be unique per call (code columns are UNIQUE).
    #[allow(clippy::too_many_arguments)]
    fn seed_ticket(
        conn: &Connection,
        code_suffix: &str,
        event_id: i64,
        status: &str,
        currency: &str,
        purchase_cost_cents: i64,
        listing_price_cents: Option<i64>,
    ) -> i64 {
        let order_id = seed_order_only(conn, &format!("t{code_suffix}"), event_id);
        conn.execute(
            "INSERT INTO tickets (code, event_id, order_id, purchase_cost_cents, listing_price_cents, currency, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                format!("TKT-{code_suffix}"),
                event_id,
                order_id,
                purchase_cost_cents,
                listing_price_cents,
                currency,
                status,
            ],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    /// Same idea as `seed_price_check` below - inserts a row directly via
    /// SQL, bypassing `save_event_marketplace_link_impl`'s own
    /// `require_marketplace_active` guard (2.1.6). Needed for any test that
    /// needs to represent a link saved back when a marketplace was still
    /// active (StubHub, before 2.1.6) - going through the real impl
    /// function can no longer produce that state for a retired marketplace,
    /// by design.
    fn seed_link(conn: &Connection, event_id: i64, marketplace_id: i64, url: &str) {
        conn.execute(
            "INSERT INTO event_marketplace_links(event_id, marketplace_id, url) VALUES (?1, ?2, ?3)",
            params![event_id, marketplace_id, url],
        )
        .unwrap();
    }

    #[allow(clippy::too_many_arguments)]
    fn seed_price_check(
        conn: &Connection,
        event_id: i64,
        marketplace_id: i64,
        lowest_price_cents: i64,
        average_price_cents: i64,
        highest_price_cents: i64,
        listing_count: i64,
        currency: &str,
    ) -> i64 {
        conn.execute(
            "INSERT INTO price_checks
               (event_id, marketplace_id, lowest_price_cents, average_price_cents, highest_price_cents, listing_count, currency)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![event_id, marketplace_id, lowest_price_cents, average_price_cents, highest_price_cents, listing_count, currency],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    // -- save_event_marketplace_link_impl ------------------------------------
    // 2.1.6: this whole block just needs SOME marketplace, not StubHub
    // specifically - switched to Vivid Seats (still active) now that
    // StubHub is retired and save_event_marketplace_link_impl refuses new
    // non-blank links against a retired one (require_marketplace_active).
    // StubHub itself still gets exercised, deliberately, by the dedicated
    // "2.1.6: StubHub retired" tests further down.

    #[test]
    fn saving_a_link_creates_it() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Coldplay Arena Show");
        let vivid_seats = marketplace_id_by_name(&conn, "Vivid Seats");
        let input = EventMarketplaceLinkInput { event_id, marketplace_id: vivid_seats, url: "https://vividseats.com/x".into() };

        let saved = save_event_marketplace_link_impl(&conn, &input).unwrap().unwrap();

        assert_eq!(saved.url, "https://vividseats.com/x");
        assert_eq!(saved.event_id, event_id);
        assert_eq!(saved.marketplace_id, vivid_seats);
    }

    #[test]
    fn saving_again_updates_the_existing_link_instead_of_duplicating_it() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Coldplay Arena Show");
        let vivid_seats = marketplace_id_by_name(&conn, "Vivid Seats");
        save_event_marketplace_link_impl(
            &conn,
            &EventMarketplaceLinkInput { event_id, marketplace_id: vivid_seats, url: "https://vividseats.com/old".into() },
        )
        .unwrap();

        let updated = save_event_marketplace_link_impl(
            &conn,
            &EventMarketplaceLinkInput { event_id, marketplace_id: vivid_seats, url: "https://vividseats.com/new".into() },
        )
        .unwrap()
        .unwrap();

        assert_eq!(updated.url, "https://vividseats.com/new");
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM event_marketplace_links WHERE event_id=?1 AND marketplace_id=?2",
                params![event_id, vivid_seats],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "must upsert, never duplicate");
    }

    #[test]
    fn a_blank_url_deletes_an_existing_link() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Coldplay Arena Show");
        let vivid_seats = marketplace_id_by_name(&conn, "Vivid Seats");
        save_event_marketplace_link_impl(
            &conn,
            &EventMarketplaceLinkInput { event_id, marketplace_id: vivid_seats, url: "https://vividseats.com/x".into() },
        )
        .unwrap();

        let result = save_event_marketplace_link_impl(
            &conn,
            &EventMarketplaceLinkInput { event_id, marketplace_id: vivid_seats, url: "   ".into() },
        )
        .unwrap();

        assert!(result.is_none());
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM event_marketplace_links WHERE event_id=?1 AND marketplace_id=?2",
                params![event_id, vivid_seats],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn a_blank_url_when_nothing_exists_yet_is_a_harmless_no_op() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Coldplay Arena Show");
        let vivid_seats = marketplace_id_by_name(&conn, "Vivid Seats");

        let result = save_event_marketplace_link_impl(
            &conn,
            &EventMarketplaceLinkInput { event_id, marketplace_id: vivid_seats, url: "".into() },
        )
        .unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn a_non_blank_url_against_a_retired_marketplace_is_refused() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Coldplay Arena Show");
        let stubhub = marketplace_id_by_name(&conn, "StubHub");

        let err = save_event_marketplace_link_impl(
            &conn,
            &EventMarketplaceLinkInput { event_id, marketplace_id: stubhub, url: "https://stubhub.com/x".into() },
        )
        .unwrap_err()
        .to_string();

        assert!(err.contains("StubHub"), "the message should name the retired marketplace, got: {err}");
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM event_marketplace_links WHERE event_id=?1 AND marketplace_id=?2",
                params![event_id, stubhub],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 0, "the refused link must never have been written");
    }

    #[test]
    fn clearing_a_retired_marketplaces_existing_link_is_still_allowed() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Coldplay Arena Show");
        let stubhub = marketplace_id_by_name(&conn, "StubHub");
        seed_link(&conn, event_id, stubhub, "https://stubhub.com/x");

        let result = save_event_marketplace_link_impl(
            &conn,
            &EventMarketplaceLinkInput { event_id, marketplace_id: stubhub, url: "   ".into() },
        );

        assert!(
            result.is_ok(),
            "removing data must never be blocked by the retired-marketplace guard, even though adding it now is"
        );
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM event_marketplace_links WHERE event_id=?1 AND marketplace_id=?2",
                params![event_id, stubhub],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    // -- save_price_check_impl -----------------------------------------------
    // 2.1.6: same switch as the block above, same reason - StubHub is
    // retired and save_price_check_impl now refuses it via
    // require_marketplace_active, so these generic tests move to Vivid
    // Seats; the guard itself gets its own dedicated tests right after.

    #[test]
    fn saving_a_valid_price_check_persists_it() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Coldplay Arena Show");
        let vivid_seats = marketplace_id_by_name(&conn, "Vivid Seats");
        let input = PriceCheckInput {
            event_id,
            marketplace_id: vivid_seats,
            lowest_price_cents: 5000,
            average_price_cents: 7000,
            highest_price_cents: 9000,
            listing_count: 12,
            currency: "EUR".into(),
            median_price_cents: None,
        };

        let saved = save_price_check_impl(&conn, &input).unwrap();

        assert_eq!(saved.lowest_price_cents, 5000);
        assert_eq!(saved.average_price_cents, 7000);
        assert_eq!(saved.highest_price_cents, 9000);
        assert_eq!(saved.listing_count, 12);
        assert_eq!(saved.currency, "EUR");
    }

    #[test]
    fn saving_twice_keeps_both_as_history_never_an_overwrite() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Coldplay Arena Show");
        let vivid_seats = marketplace_id_by_name(&conn, "Vivid Seats");
        let mk = |lowest: i64| PriceCheckInput {
            event_id,
            marketplace_id: vivid_seats,
            lowest_price_cents: lowest,
            average_price_cents: lowest + 1000,
            highest_price_cents: lowest + 2000,
            listing_count: 5,
            currency: "EUR".to_string(),
            median_price_cents: None,
        };

        save_price_check_impl(&conn, &mk(5000)).unwrap();
        save_price_check_impl(&conn, &mk(5500)).unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM price_checks WHERE event_id=?1 AND marketplace_id=?2",
                params![event_id, vivid_seats],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 2, "a later check must never overwrite an earlier one - marko explicitly asked to keep history");
    }

    #[test]
    fn rejects_a_lowest_price_above_the_average() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test");
        let vivid_seats = marketplace_id_by_name(&conn, "Vivid Seats");
        let input = PriceCheckInput {
            event_id,
            marketplace_id: vivid_seats,
            lowest_price_cents: 9000,
            average_price_cents: 7000,
            highest_price_cents: 9500,
            listing_count: 3,
            currency: "EUR".into(),
            median_price_cents: None,
        };
        assert!(save_price_check_impl(&conn, &input).is_err());
    }

    #[test]
    fn rejects_an_average_price_above_the_highest() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test");
        let vivid_seats = marketplace_id_by_name(&conn, "Vivid Seats");
        let input = PriceCheckInput {
            event_id,
            marketplace_id: vivid_seats,
            lowest_price_cents: 5000,
            average_price_cents: 9500,
            highest_price_cents: 9000,
            listing_count: 3,
            currency: "EUR".into(),
            median_price_cents: None,
        };
        assert!(save_price_check_impl(&conn, &input).is_err());
    }

    #[test]
    fn rejects_negative_prices_and_negative_listing_count() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test");
        let vivid_seats = marketplace_id_by_name(&conn, "Vivid Seats");
        let base = PriceCheckInput {
            event_id,
            marketplace_id: vivid_seats,
            lowest_price_cents: 1000,
            average_price_cents: 2000,
            highest_price_cents: 3000,
            listing_count: 3,
            currency: "EUR".into(),
            median_price_cents: None,
        };

        let mut negative_lowest = base.clone();
        negative_lowest.lowest_price_cents = -1;
        assert!(save_price_check_impl(&conn, &negative_lowest).is_err());

        let mut negative_count = base.clone();
        negative_count.listing_count = -1;
        assert!(save_price_check_impl(&conn, &negative_count).is_err());
    }

    #[test]
    fn rejects_a_blank_currency() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test");
        let vivid_seats = marketplace_id_by_name(&conn, "Vivid Seats");
        let input = PriceCheckInput {
            event_id,
            marketplace_id: vivid_seats,
            lowest_price_cents: 1000,
            average_price_cents: 2000,
            highest_price_cents: 3000,
            listing_count: 3,
            currency: "   ".into(),
            median_price_cents: None,
        };
        assert!(save_price_check_impl(&conn, &input).is_err());
    }

    // -- require_marketplace_active guard (2.1.6) ----------------------------

    #[test]
    fn a_price_check_against_a_retired_marketplace_is_refused() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test");
        let stubhub = marketplace_id_by_name(&conn, "StubHub");
        let input = PriceCheckInput {
            event_id,
            marketplace_id: stubhub,
            lowest_price_cents: 5000,
            average_price_cents: 6000,
            highest_price_cents: 7000,
            listing_count: 10,
            currency: "EUR".into(),
            median_price_cents: None,
        };

        let err = save_price_check_impl(&conn, &input).unwrap_err().to_string();

        assert!(err.contains("StubHub"), "the message should name the retired marketplace, got: {err}");
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM price_checks WHERE event_id = ?1", [event_id], |r| r.get(0)).unwrap();
        assert_eq!(count, 0, "the refused check must never have been written");
    }

    #[test]
    fn a_price_check_against_an_active_marketplace_still_succeeds() {
        // Guards against a too-broad guard - only StubHub (inactive) should
        // ever be refused, every other seeded marketplace must go through
        // exactly as before this whole 2.1.6 change.
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test");
        let ticombo = marketplace_id_by_name(&conn, "Ticombo");
        let input = PriceCheckInput {
            event_id,
            marketplace_id: ticombo,
            lowest_price_cents: 5000,
            average_price_cents: 6000,
            highest_price_cents: 7000,
            listing_count: 10,
            currency: "EUR".into(),
            median_price_cents: None,
        };
        assert!(save_price_check_impl(&conn, &input).is_ok());
    }

    #[test]
    fn a_price_check_against_an_unknown_marketplace_id_is_a_not_found_error() {
        // Not just is_err() - price_checks.marketplace_id is itself a
        // foreign key (and test_conn() runs with PRAGMA foreign_keys = ON),
        // so a plain INSERT against a nonexistent id would ALSO fail even
        // with require_marketplace_active deleted entirely, just with a
        // different, generic "FOREIGN KEY constraint failed" message
        // (see error.rs's own From<rusqlite::Error>). Asserting the exact
        // message proves this specific guard is what actually caught it.
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test");
        let input = PriceCheckInput {
            event_id,
            marketplace_id: 999_999,
            lowest_price_cents: 5000,
            average_price_cents: 6000,
            highest_price_cents: 7000,
            listing_count: 10,
            currency: "EUR".into(),
            median_price_cents: None,
        };

        let err = save_price_check_impl(&conn, &input).unwrap_err().to_string();

        assert_eq!(err, "Marketplace not found", "must be require_marketplace_active's own NotFound message");
    }

    // -- get_price_checker_summary_impl --------------------------------------

    #[test]
    fn errors_on_an_unknown_event() {
        let conn = test_conn();
        assert!(get_price_checker_summary_impl(&conn, 999_999).is_err());
    }

    #[test]
    fn a_fresh_event_with_nothing_yet_still_lists_every_active_marketplace_with_empty_data() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Coldplay Arena Show");

        let summary = get_price_checker_summary_impl(&conn, event_id).unwrap();

        assert_eq!(summary.event_name, "Coldplay Arena Show");
        // 2.1.6: StubHub is retired (inactive) and this event has never
        // touched it, so only the 3 ACTIVE marketplaces show - see the
        // dedicated StubHub-specific tests below for the retired/history
        // behavior itself.
        let names: Vec<&str> = summary.marketplaces.iter().map(|m| m.marketplace_name.as_str()).collect();
        assert_eq!(names, vec!["Ticombo", "Viagogo", "Vivid Seats"], "alphabetical, active-only, for a fresh event");
        for m in &summary.marketplaces {
            assert!(m.marketplace_active, "every marketplace shown to a fresh event must be active");
            assert!(m.link.is_none());
            assert!(m.history.is_empty());
        }
        assert_eq!(summary.unsold_ticket_count, 0);
        assert_eq!(summary.my_currency, None);
        assert_eq!(summary.my_avg_purchase_cost_cents, None);
        assert_eq!(summary.my_avg_listing_price_cents, None);
        assert_eq!(summary.missing_listing_price_count, 0);
        assert_eq!(summary.market_lowest_price_cents, None);
        assert_eq!(summary.market_average_price_cents, None);
        assert_eq!(summary.recommended_price_cents, None);
        assert_eq!(summary.expected_profit_cents, None);
        assert_eq!(summary.expected_roi, None);
    }

    #[test]
    fn my_averages_only_count_unsold_available_and_listed_tickets() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event");
        seed_ticket(&conn, "1", event_id, "available", "EUR", 1000, Some(1500));
        seed_ticket(&conn, "2", event_id, "listed", "EUR", 2000, None); // no listing price yet
        seed_ticket(&conn, "3", event_id, "sold", "EUR", 9999, Some(99999)); // must be excluded
        seed_ticket(&conn, "4", event_id, "cancelled", "EUR", 9999, Some(99999)); // must be excluded

        let summary = get_price_checker_summary_impl(&conn, event_id).unwrap();

        assert_eq!(summary.unsold_ticket_count, 2);
        assert_eq!(summary.my_currency.as_deref(), Some("EUR"));
        assert_eq!(summary.my_avg_purchase_cost_cents, Some((1000 + 2000) / 2));
        assert_eq!(summary.my_avg_listing_price_cents, Some(1500), "only the one unsold ticket with a listing price counts");
        assert_eq!(summary.missing_listing_price_count, 1);
    }

    #[test]
    fn mixed_currency_unsold_inventory_still_reports_a_blended_average_but_no_market_comparison() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event");
        seed_ticket(&conn, "1", event_id, "available", "EUR", 1000, Some(1500));
        seed_ticket(&conn, "2", event_id, "available", "USD", 1200, Some(1600));
        let stubhub = marketplace_id_by_name(&conn, "StubHub");
        seed_price_check(&conn, event_id, stubhub, 5000, 6000, 7000, 10, "EUR");

        let summary = get_price_checker_summary_impl(&conn, event_id).unwrap();

        // Same "always return the blended figure, flag it instead of hiding
        // it" convention as total_cost_cents/formatMoneyOrMixed elsewhere in
        // this app - see PriceCheckerSummary's own doc comment.
        assert_eq!(summary.my_currency, None);
        assert_eq!(summary.my_avg_purchase_cost_cents, Some((1000 + 1200) / 2));
        // But the market comparison genuinely needs ONE currency to filter
        // marketplace checks against, and there isn't one here.
        assert_eq!(summary.market_lowest_price_cents, None);
        assert_eq!(summary.market_average_price_cents, None);
        assert_eq!(summary.recommended_price_cents, None);
        assert_eq!(summary.expected_profit_cents, None);
        assert_eq!(summary.expected_roi, None);
    }

    #[test]
    fn market_comparison_uses_only_the_latest_check_per_marketplace() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event");
        seed_ticket(&conn, "1", event_id, "available", "EUR", 4000, Some(6000));
        let stubhub = marketplace_id_by_name(&conn, "StubHub");
        // Older check first (lower id -> sorts after the newer one).
        seed_price_check(&conn, event_id, stubhub, 5000, 6000, 7000, 10, "EUR");
        // Newer check: a real price drop marko should see reflected.
        seed_price_check(&conn, event_id, stubhub, 4500, 5500, 6500, 8, "EUR");

        let summary = get_price_checker_summary_impl(&conn, event_id).unwrap();

        let stubhub_view = summary.marketplaces.iter().find(|m| m.marketplace_name == "StubHub").unwrap();
        assert_eq!(stubhub_view.history.len(), 2);
        assert_eq!(stubhub_view.history[0].lowest_price_cents, 4500, "history[0] must be the newest check");
        assert_eq!(stubhub_view.history[1].lowest_price_cents, 5000);

        assert_eq!(summary.market_lowest_price_cents, Some(4500), "must use the latest check, not the older 5000");
    }

    #[test]
    fn market_comparison_ignores_a_marketplaces_latest_check_in_a_different_currency() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event");
        seed_ticket(&conn, "1", event_id, "available", "EUR", 4000, Some(6000));
        let stubhub = marketplace_id_by_name(&conn, "StubHub");
        let vivid = marketplace_id_by_name(&conn, "Vivid Seats");
        seed_price_check(&conn, event_id, stubhub, 4500, 5500, 6500, 8, "USD"); // wrong currency - excluded
        seed_price_check(&conn, event_id, vivid, 5200, 6200, 7200, 4, "EUR"); // matching - included

        let summary = get_price_checker_summary_impl(&conn, event_id).unwrap();

        assert_eq!(summary.market_lowest_price_cents, Some(5200), "the USD StubHub check must not be blended in");
        assert_eq!(summary.market_average_price_cents, Some(6200));
    }

    #[test]
    fn recommended_price_is_five_percent_under_the_market_lowest() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event");
        seed_ticket(&conn, "1", event_id, "available", "EUR", 4000, Some(6000));
        let stubhub = marketplace_id_by_name(&conn, "StubHub");
        seed_price_check(&conn, event_id, stubhub, 10000, 11000, 12000, 10, "EUR");

        let summary = get_price_checker_summary_impl(&conn, event_id).unwrap();

        assert_eq!(summary.market_lowest_price_cents, Some(10000));
        assert_eq!(summary.recommended_price_cents, Some(9500), "10000 undercut by RECOMMENDED_PRICE_UNDERCUT_PCT (5%)");
        assert_eq!(summary.expected_profit_cents, Some(9500 - 4000));
        let expected_roi = (9500.0 - 4000.0) / 4000.0;
        assert!((summary.expected_roi.unwrap() - expected_roi).abs() < 1e-9);
    }

    #[test]
    fn a_marketplace_with_a_saved_link_but_no_checks_yet_still_appears_with_the_link() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event");
        // 2.1.6: Vivid Seats, not StubHub - this test is about summary
        // assembly for an ordinary active marketplace, not retirement.
        let vivid_seats = marketplace_id_by_name(&conn, "Vivid Seats");
        save_event_marketplace_link_impl(
            &conn,
            &EventMarketplaceLinkInput { event_id, marketplace_id: vivid_seats, url: "https://vividseats.com/x".into() },
        )
        .unwrap();

        let summary = get_price_checker_summary_impl(&conn, event_id).unwrap();

        let view = summary.marketplaces.iter().find(|m| m.marketplace_name == "Vivid Seats").unwrap();
        assert_eq!(view.link.as_ref().map(|l| l.url.as_str()), Some("https://vividseats.com/x"));
        assert!(view.history.is_empty());
    }

    // -- 2.1.6: StubHub retired (active=false), Viagogo added -----------------

    #[test]
    fn a_retired_marketplace_is_excluded_from_a_fresh_event_with_no_history_against_it() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event");

        let summary = get_price_checker_summary_impl(&conn, event_id).unwrap();

        assert!(
            summary.marketplaces.iter().all(|m| m.marketplace_name != "StubHub"),
            "StubHub must not be offered to an event that never touched it"
        );
    }

    #[test]
    fn a_retired_marketplace_still_appears_for_an_event_with_an_existing_link() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event");
        let stubhub = marketplace_id_by_name(&conn, "StubHub");
        // 2.1.6: seeded directly via SQL, not save_event_marketplace_link_impl -
        // that link is being modeled as one saved back when StubHub was
        // still active, and the impl function itself now refuses to create
        // a brand-new one against a retired marketplace (see seed_link's
        // own doc comment and require_marketplace_active).
        seed_link(&conn, event_id, stubhub, "https://stubhub.com/x");

        let summary = get_price_checker_summary_impl(&conn, event_id).unwrap();

        let stubhub_view = summary
            .marketplaces
            .iter()
            .find(|m| m.marketplace_name == "StubHub")
            .expect("StubHub must still appear - this event has a real saved link against it");
        assert!(!stubhub_view.marketplace_active, "StubHub must be reported as retired, not active");
    }

    #[test]
    fn a_retired_marketplace_still_appears_for_an_event_with_existing_check_history() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event");
        let stubhub = marketplace_id_by_name(&conn, "StubHub");
        seed_price_check(&conn, event_id, stubhub, 5000, 6000, 7000, 10, "EUR");

        let summary = get_price_checker_summary_impl(&conn, event_id).unwrap();

        let stubhub_view = summary
            .marketplaces
            .iter()
            .find(|m| m.marketplace_name == "StubHub")
            .expect("StubHub must still appear - this event has real check history against it");
        assert_eq!(stubhub_view.history.len(), 1, "the old history itself must still be intact and readable");
    }

    #[test]
    fn a_retired_marketplaces_history_on_one_event_never_leaks_it_into_an_unrelated_event() {
        let conn = test_conn();
        let event_with_history = seed_event(&conn, "Has StubHub History");
        let unrelated_event = seed_event(&conn, "Never Touched StubHub");
        let stubhub = marketplace_id_by_name(&conn, "StubHub");
        seed_price_check(&conn, event_with_history, stubhub, 5000, 6000, 7000, 10, "EUR");

        let summary = get_price_checker_summary_impl(&conn, unrelated_event).unwrap();

        assert!(
            summary.marketplaces.iter().all(|m| m.marketplace_name != "StubHub"),
            "one event's StubHub history must not make StubHub appear for a different, unrelated event"
        );
    }

    #[test]
    fn viagogo_is_seeded_active_and_available_to_every_fresh_event() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event");

        let summary = get_price_checker_summary_impl(&conn, event_id).unwrap();

        let viagogo = summary
            .marketplaces
            .iter()
            .find(|m| m.marketplace_name == "Viagogo")
            .expect("Viagogo must be offered to every event, same as Vivid Seats/Ticombo");
        assert!(viagogo.marketplace_active);
    }

    // -- delete_marketplace_impl (2.1.6) ---------------------------------------

    #[test]
    fn delete_marketplace_refuses_when_a_link_exists() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event");
        let stubhub = marketplace_id_by_name(&conn, "StubHub");
        // 2.1.6: seeded directly via SQL - see seed_link's own doc comment.
        seed_link(&conn, event_id, stubhub, "https://stubhub.com/x");

        assert!(delete_marketplace_impl(&conn, stubhub).is_err(), "must refuse - deleting would cascade away the saved link");
        let still_there: i64 = conn.query_row("SELECT COUNT(*) FROM marketplaces WHERE id = ?1", [stubhub], |r| r.get(0)).unwrap();
        assert_eq!(still_there, 1, "the refused delete must not have happened at all");
    }

    #[test]
    fn delete_marketplace_refuses_when_check_history_exists() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event");
        let stubhub = marketplace_id_by_name(&conn, "StubHub");
        seed_price_check(&conn, event_id, stubhub, 5000, 6000, 7000, 10, "EUR");

        assert!(delete_marketplace_impl(&conn, stubhub).is_err(), "must refuse - deleting would cascade away real history");
    }

    #[test]
    fn delete_marketplace_succeeds_for_one_with_no_history_at_all() {
        let conn = test_conn();
        conn.execute("INSERT INTO marketplaces(name) VALUES ('Throwaway Test Marketplace')", []).unwrap();
        let id = conn.last_insert_rowid();

        assert!(delete_marketplace_impl(&conn, id).is_ok());
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM marketplaces WHERE id = ?1", [id], |r| r.get(0)).unwrap();
        assert_eq!(count, 0);
    }
}

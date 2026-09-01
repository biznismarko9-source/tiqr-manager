-- TIQR Manager - 019_price_checker_market_analysis
-- 2.2.0: marko's "PRICE CHECKER — MARKET ANALYSIS" spec - tier/level pricing,
-- section breakdown, comparable-market ranking and price recommendations on
-- top of the existing Visible Scanner (2.1.9), built without touching the
-- scanner's own lifecycle/session code at all. See
-- PRICE-CHECKER-MARKET-ANALYSIS-2.2-REPORT.md for the full design.
--
-- Live scan-session analysis (tier/section grouping, comparable ranking,
-- recommendations) is computed on demand from the already-accumulated
-- session listings (AppState::price_scanner_sessions) - same as the session
-- itself, it is NOT persisted here; it only exists in memory while the
-- scanner window is open. The one real schema addition is for the ALREADY
-- persisted side: `price_checks` (014_price_checker.sql) has always stored
-- only an event-wide aggregate (lowest/average/highest/median/count) per
-- saved check, never a per-tier breakdown - marko's own spec point 10
-- explicitly asks the saved history to also remember tier-level lowest/
-- median/count going forward ("Rozšír ju o: ... tier lowest, tier median,
-- listing count"), so a future trend view can show a tier's price moving
-- over time, not just the event-wide figure.
--
-- A separate child table, not new columns on `price_checks` - a check can
-- have zero, one, or many tiers, which a fixed set of extra columns can't
-- represent, and (same reasoning as `event_marketplace_links`/`price_checks`
-- themselves) this keeps `price_checks` itself completely unchanged in
-- shape, so every existing query/struct/test against it stays valid as-is.
CREATE TABLE IF NOT EXISTS price_check_tiers (
  id                   INTEGER PRIMARY KEY AUTOINCREMENT,
  -- Disposable derived data belonging entirely to one check, same
  -- ON DELETE CASCADE reasoning as event_marketplace_links: this table only
  -- ever exists to enrich a price_checks row, so it must not outlive it.
  price_check_id       INTEGER NOT NULL REFERENCES price_checks(id) ON DELETE CASCADE,
  -- Whatever the marketplace itself called it ("Level 100", "Tier 1",
  -- "Zone A", ...) or the literal string "Unclassified" when the Visible
  -- Scanner's tier detection couldn't confidently determine one - never
  -- invented/normalized into a different label (marko's own "NEVYMÝŠĽAJ
  -- tier mapping"). Free text, no CHECK constraint, same permissiveness as
  -- price_checks.currency.
  tier_name            TEXT NOT NULL,
  lowest_price_cents   INTEGER NOT NULL,
  median_price_cents   INTEGER NOT NULL,
  listing_count        INTEGER NOT NULL
);

-- Every read of this table is "give me this check's tier breakdown" (used
-- both when displaying one saved check and when assembling a marketplace's
-- full tier-comparison across its history) - this index covers that access
-- pattern directly, same spirit as idx_price_checks_event_marketplace.
CREATE INDEX IF NOT EXISTS idx_price_check_tiers_check ON price_check_tiers(price_check_id);

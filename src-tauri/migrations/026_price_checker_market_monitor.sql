-- TIQR Manager - 026_price_checker_market_monitor
-- 2.4.1: "Price Checker Live Market Monitor" - marko's replacement plan for
-- the previously-cancelled "Live Event Intelligence" direction (that build
-- was reviewed, then reverted in full before ever shipping - see
-- PROJECT_STATE/PROTECTED_AREAS.md's own entry on it, and CHANGELOG.md's
-- "2.4.0 (pre-release direction, never shipped) - Live Event Intelligence
-- Foundation" entry for the original, abandoned design). His own words: "Predchádzajúci nápad 'Live Event
-- Intelligence' RUŠÍME ÚPLNE" (that idea is cancelled completely) - every
-- online/live-market capability now lives directly inside PRICE CHECKER
-- instead of a separate section. Since nothing from migration 026 was ever
-- shipped to a real install under that old name, "026" is reused here
-- rather than left permanently skipped - see PROTECTED_AREAS.md's entry for
-- why that's safe in THIS specific case.
--
-- Reuses everything that already exists rather than reinventing it: the
-- `marketplaces` lookup table (unchanged - Viagogo/Vivid Seats/Ticombo,
-- 014/017/025) and `event_marketplace_links` (unchanged - one saved URL per
-- event+marketplace, already never re-entered per scan) are exactly what
-- marko's new spec asks for on that front. `price_checks`/`price_check_tiers`
-- (014/019) also stay completely untouched - that remains marko's own
-- manually-curated, explicitly-saved "Price History", exactly as it works
-- today. The tables below are new and ADDITIVE alongside it, for a
-- DIFFERENT purpose: an AUTOMATIC record of every scan (manual or, later,
-- Auto Monitor) the Visible Scanner actually completes, which is what
-- drives change detection and Market Alerts - marko was explicit these are
-- two different things ("existing Price History" must be preserved
-- unchanged, per his own spec's protected-areas list).
--
-- Four tables:

-- One row per successful/partial scan (commands::price_checker_scanner's
-- own ScannerStatus vocabulary - see that module's doc comment) - "market
-- grouping by Tier/Level where available" (marko's own Section 6 wording).
-- Split by CURRENCY, same rule as commands::price_checker_analysis's own
-- MarketAnalysisResult.byCurrency - a scan session mixing e.g. EUR and USD
-- listings produces one snapshot row PER currency actually present, never a
-- blended figure (marko's long-standing "## CURRENCY" rule, "never sum
-- EUR+USD+GBP"). NEVER overwritten or deleted by an ordinary scan - real,
-- permanent history, same append-only spirit as price_checks itself.
CREATE TABLE IF NOT EXISTS market_snapshots (
  id                   INTEGER PRIMARY KEY AUTOINCREMENT,
  event_id             INTEGER NOT NULL REFERENCES events(id) ON DELETE CASCADE,
  marketplace_id       INTEGER NOT NULL REFERENCES marketplaces(id) ON DELETE CASCADE,
  checked_at           TEXT NOT NULL,
  -- 'success' | 'partial' - mirrors ScannerStatus's own two "real data was
  -- found" values (never 'error'/'blocked'/'unable_to_read' - those never
  -- reach this table at all, see market_source_status below instead).
  scan_status          TEXT NOT NULL CHECK (scan_status IN ('success','partial')),
  listing_count         INTEGER NOT NULL,
  lowest_price_cents   INTEGER NOT NULL,
  median_price_cents   INTEGER NOT NULL,
  average_price_cents  INTEGER NOT NULL,
  highest_price_cents  INTEGER NOT NULL,
  currency             TEXT NOT NULL,
  is_demo              INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_market_snapshots_lookup ON market_snapshots(event_id, marketplace_id, id DESC);

-- Per-tier breakdown for one snapshot - marko's own HARD CONSTRAINT: Tier/
-- Level is the only market grouping; Section/Row/Seat stay metadata-only and
-- must never feed a price computation. Same shape as price_check_tiers
-- (019_price_checker_market_analysis.sql) and the same "Unclassified"
-- fallback convention (commands::price_checker_analysis::group_by_tier) for
-- a listing with no confidently-detected tier - never invented/normalized
-- into a different label.
CREATE TABLE IF NOT EXISTS market_snapshot_tiers (
  id                   INTEGER PRIMARY KEY AUTOINCREMENT,
  snapshot_id          INTEGER NOT NULL REFERENCES market_snapshots(id) ON DELETE CASCADE,
  tier_name            TEXT NOT NULL,
  lowest_price_cents   INTEGER NOT NULL,
  median_price_cents   INTEGER NOT NULL,
  listing_count        INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_market_snapshot_tiers_snapshot ON market_snapshot_tiers(snapshot_id);

-- One row per (event, marketplace) - the outcome of the LAST scan ATTEMPT,
-- success or failure, updated every time (never deleted). This is the
-- single source of truth behind marko's own Section 13 status enum
-- (CONNECTED/SCANNING/SUCCESS/FAILED/NOT CONNECTED - the first and last are
-- derived at read time from event_marketplace_links/this table's mere
-- presence, "SCANNING" is frontend-only live session state, so only
-- SUCCESS/FAILED are actually stored here) and Section 12's "always show
-- Last successful scan" cache/offline requirement: last_successful_scan_at
-- is only ever ADVANCED by a real success, so a run of failures afterward
-- can never make marko's last real data disappear or look stale in a
-- misleading way.
CREATE TABLE IF NOT EXISTS market_source_status (
  event_id                       INTEGER NOT NULL REFERENCES events(id) ON DELETE CASCADE,
  marketplace_id                 INTEGER NOT NULL REFERENCES marketplaces(id) ON DELETE CASCADE,
  last_scan_at                   TEXT NOT NULL,
  last_scan_ok                   INTEGER NOT NULL,
  -- Short, human-readable only - never a stack trace (marko's own explicit
  -- Section 13 wording). NULL exactly when last_scan_ok = 1.
  last_error_message             TEXT,
  last_successful_scan_at        TEXT,
  last_successful_listing_count  INTEGER,
  PRIMARY KEY (event_id, marketplace_id)
);

-- Append-only log of detected changes - marko's own Section 9/10: MARKET
-- DROP, MARKET RISE, NEW SUPPLY, SUPPLY DROP, SOURCE FAILURE. tier = NULL
-- means a whole-market (overall) alert; a real tier name means that ONE
-- tier's own price/supply changed. `message` is the one thing the UI ever
-- shows directly - fully pre-formatted, human-readable text (Section 13's
-- "no technical stack traces" rule applied here too), so the frontend never
-- has to reconstruct meaning from the raw previous/current columns itself.
-- Never deleted, never edited - a real, permanent log, same spirit as this
-- app's existing notification_log (013_notifications.sql).
CREATE TABLE IF NOT EXISTS market_alerts (
  id                       INTEGER PRIMARY KEY AUTOINCREMENT,
  event_id                 INTEGER NOT NULL REFERENCES events(id) ON DELETE CASCADE,
  marketplace_id           INTEGER NOT NULL REFERENCES marketplaces(id) ON DELETE CASCADE,
  alert_type               TEXT NOT NULL CHECK (alert_type IN ('market_drop','market_rise','new_supply','supply_drop','source_failure')),
  tier_name                TEXT,
  message                  TEXT NOT NULL,
  previous_price_cents     INTEGER,
  current_price_cents      INTEGER,
  previous_listing_count   INTEGER,
  current_listing_count    INTEGER,
  currency                 TEXT,
  created_at               TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_market_alerts_lookup ON market_alerts(event_id, marketplace_id, id DESC);

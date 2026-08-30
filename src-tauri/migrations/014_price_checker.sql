-- TIQR Manager - 014_price_checker
-- 2.0.81: marko's "Price Checker" request - save marketplace links per
-- event, manually record what the market is currently asking (no live
-- API/scraping: neither StubHub nor Vivid Seats offers a public read API to
-- an individual, Vivid Seats' API is partner/approved-seller only, and
-- StubHub actively blocks casual scraping - confirmed by research before
-- building this, not assumed. marko's own instruction was to fall back to
-- manual entry rather than bypass any site's protection, so that's what
-- this schema is built around), and compare it against marko's own cost.
--
-- Three tables, same shape/spirit as existing precedent in this codebase:

-- Lookup table, identical pattern to `platforms`/`suppliers`/
-- `event_categories` (001_initial_schema.sql / 012_event_categories.sql) -
-- marko manages this list himself from the app (list/create/delete), so
-- adding a 4th/5th marketplace later never needs a new migration.
CREATE TABLE IF NOT EXISTS marketplaces (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  name       TEXT NOT NULL UNIQUE,
  is_demo    INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

-- Seeded with marko's own 3, in the order he asked for them - same
-- "pre-fill the options already being requested" precedent as
-- 012_event_categories.sql's 6 seeded categories.
INSERT INTO marketplaces (name) VALUES ('StubHub'), ('Vivid Seats'), ('Ticombo');

-- One saved link per (event, marketplace). Saving with a non-blank URL
-- upserts this row; clearing the URL field deletes it entirely rather than
-- storing an empty string - see price_checker::save_event_marketplace_link_
-- impl. ON DELETE CASCADE on both foreign keys: a link is disposable,
-- re-enterable reference data (a saved URL), never financial history, so it
-- is fine for it to disappear along with the event or the marketplace it
-- pointed at, unlike orders/tickets/sales which this app protects deliberately.
CREATE TABLE IF NOT EXISTS event_marketplace_links (
  id             INTEGER PRIMARY KEY AUTOINCREMENT,
  event_id       INTEGER NOT NULL REFERENCES events(id) ON DELETE CASCADE,
  marketplace_id INTEGER NOT NULL REFERENCES marketplaces(id) ON DELETE CASCADE,
  url            TEXT NOT NULL,
  is_demo        INTEGER NOT NULL DEFAULT 0,
  created_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  UNIQUE(event_id, marketplace_id)
);

-- Append-only history of "Check Prices" entries - deliberately NOT one row
-- per (event, marketplace) overwritten in place. marko explicitly asked to
-- keep every past check so the app can show whether the market moved up or
-- down since last time, not just the latest snapshot - see
-- price_checker::get_price_checker_summary_impl, which returns each
-- marketplace's FULL history (newest first); the frontend derives "up or
-- down since last time" from history[0] vs history[1] itself.
CREATE TABLE IF NOT EXISTS price_checks (
  id                   INTEGER PRIMARY KEY AUTOINCREMENT,
  event_id             INTEGER NOT NULL REFERENCES events(id) ON DELETE CASCADE,
  marketplace_id       INTEGER NOT NULL REFERENCES marketplaces(id) ON DELETE CASCADE,
  lowest_price_cents   INTEGER NOT NULL,
  average_price_cents  INTEGER NOT NULL,
  highest_price_cents  INTEGER NOT NULL,
  listing_count        INTEGER NOT NULL,
  -- Free text, no CHECK constraint - same deliberate permissiveness as
  -- orders/tickets/sales.currency (see PROTECTED-AREAS-NOTES.md's currency
  -- architecture notes). Not blended across marketplaces that disagree on
  -- currency - the summary's market-wide figures only ever combine checks
  -- that share marko's own unsold-inventory currency for this event, same
  -- "never guess across currencies" rule the rest of this app already holds.
  currency             TEXT NOT NULL DEFAULT 'EUR',
  checked_at           TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  is_demo              INTEGER NOT NULL DEFAULT 0
);

-- Every read of this table is "give me this event's checks, newest first,
-- grouped by marketplace" - this index covers that access pattern directly.
CREATE INDEX IF NOT EXISTS idx_price_checks_event_marketplace ON price_checks(event_id, marketplace_id, id DESC);

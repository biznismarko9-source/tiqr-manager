-- TIQR Manager - 026_live_event_intelligence
-- 2.4.0: marko's "Live Event Intelligence Foundation" spec - an Event can now
-- optionally carry a connected ONLINE identity on exactly 3 marketplaces:
-- Viagogo, Vivid Seats, Ticombo ("Podporuj LEN tieto 3 marketplace...
-- NEPRIDÁVAJ StubHub, Seatriks ani ine"). Foundation work only - this table
-- stores WHERE an event lives online and whether a human has confirmed that;
-- it is never read by Price Checker, never used for pricing, and nothing
-- here is populated automatically.
--
-- Deliberately a brand-new, STANDALONE table - NOT a new column on `events`
-- (marko's own explicit "existujuce udaje o evente sa nesmu menit ani
-- spatne dopĺňať" - existing event data must not change or be backfilled -
-- a new nullable column would technically satisfy that, but a separate
-- table keeps every existing Event/EventWithStats query, struct and test
-- completely untouched, which is the stronger, safer reading of that
-- instruction) and NOT a foreign key onto the existing `marketplaces` table
-- (014_price_checker.sql). That table is a general, marko-MANAGED lookup
-- (he can add/rename/retire rows from Price Checker's own UI) shared by
-- Price Checker and Listings - StubHub/Seatriks both live there, retired
-- but kept for history (017_price_checker_viagogo.sql, 025_deactivate_
-- seatriks_price_checker.sql). Live Event Intelligence's 3 sources are the
-- opposite: a fixed, code-defined set marko explicitly said NOT to let grow
-- via that same mechanism. Reusing `marketplaces` would either let someone
-- "connect" an event to StubHub/Seatriks through this feature (explicitly
-- forbidden) or need a second special-case flag on that shared table just
-- for this feature's narrower rule. A separate `source` column with its own
-- CHECK constraint keeps the two concepts (and their very different
-- lifecycles) fully decoupled: deleting a row from `marketplaces` has zero
-- effect here, and vice versa - see commands::live_event_intelligence's own
-- module doc comment for the full reasoning, including how a genuine 4th
-- source would be added later (extend the CHECK constraint in a new forward-
-- only migration + one new frontend search-URL entry - no rework of the
-- other 3 sources' code or data).
--
-- event_id is ON DELETE CASCADE, same rule as every other per-event child
-- table (price_checks, event_marketplace_links - both 014_price_checker.sql).
--
-- UNIQUE(event_id, source) is the "marketplace najviac raz na event" (a given
-- marketplace at most once per event) rule from marko's own spec, enforced
-- in the schema itself rather than only in application code - same
-- technique 004_sales_active_unique.sql already established for a different
-- business rule.
--
-- `verified` and `active` are two INDEPENDENT flags, not one combined status:
--   verified - whether a human has actually looked at `url` in a real,
--     visible window and confirmed it (see save_confirmed_online_source_impl
--     in commands::live_event_intelligence.rs). Starts at 0 for a manually
--     entered link (marko typed a URL - the app has no way to know it's
--     right until he confirms it, e.g. via "Refresh") and is set to 1 only
--     by that one function, never anywhere else - marko's own "nikdy
--     neuložiť neoverené data ako potvrdené" (never save unverified data as
--     confirmed).
--   active - a soft "still connected" flag, same convention as
--     marketplaces.active/ticket_listings.status='removed': "Disconnect" in
--     the UI flips this to 0 without deleting the row (or its verified
--     state/history), "Reconnect" flips it back. Keeps a mistaken or
--     no-longer-wanted source from being silently destroyed.
-- Every fresh row is verified=0 (see the two save functions - a discovery
-- capture-and-confirm is the only path that ever inserts with verified=1
-- directly) and active=1 (DEFAULT below covers plain manual-connect inserts).
--
-- last_checked_at/last_checked_title are both nullable and both ONLY ever
-- written by save_confirmed_online_source_impl (the same function behind
-- both "Find Online Event" -> confirm and "Refresh" -> confirm - see that
-- module's doc comment for why these two flows share one function). A
-- manually-connected source that has never been refreshed keeps both NULL -
-- an honest "never actually checked", not a guessed/backfilled timestamp.
-- last_checked_title is exactly what was read from the page's own
-- `document.title` at that moment (never parsed/interpreted) - a quick
-- human-readable sanity check ("still looks like the right event, or does
-- the title now say sold out / cancelled?"), not a new source of truth.
--
-- No is_demo column - unlike every other table in this schema, this one is
-- never seeded with demo rows (nothing about "connect this event online"
-- makes sense as canned demo data), so the column would only ever be 0.
CREATE TABLE IF NOT EXISTS event_online_sources (
  id                 INTEGER PRIMARY KEY AUTOINCREMENT,
  event_id           INTEGER NOT NULL REFERENCES events(id) ON DELETE CASCADE,
  source             TEXT NOT NULL CHECK (source IN ('viagogo', 'vivid_seats', 'ticombo')),
  url                TEXT NOT NULL,
  external_event_id  TEXT,
  verified           INTEGER NOT NULL DEFAULT 0,
  active             INTEGER NOT NULL DEFAULT 1,
  last_checked_at    TEXT,
  last_checked_title TEXT,
  created_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  UNIQUE(event_id, source)
);
CREATE INDEX IF NOT EXISTS idx_event_online_sources_event ON event_online_sources(event_id);

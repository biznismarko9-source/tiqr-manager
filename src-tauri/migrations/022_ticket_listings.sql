-- TIQR Manager - 022_ticket_listings
-- 2.2.4: marko's own request to turn "Listings" from a read-only view of
-- Ticket.listing_price_cents/status into a real multi-marketplace listing
-- system - "jeden ticket moze byt zalistovany na viacerych marketplace"
-- (one ticket can be listed on several marketplaces at once: StubHub,
-- Vivid, Ticombo, each its own price/status/URL). Deliberately NOT new
-- columns on `tickets` (marko's own explicit instruction - "nechcem tieto
-- udaje davat priamo do tickets") - a separate child table, one ticket ->
-- many listings, same "child table with a FK back to its owner" shape as
-- every other 1-to-many relationship in this schema (orders -> tickets,
-- events -> price_checks).
--
-- Reuses the EXISTING `marketplaces` lookup table (014_price_checker.sql) -
-- the same list marko already manages from Price Checker - rather than
-- inventing a second, duplicate marketplace concept.
--
-- marketplace_id is ON DELETE CASCADE, matching event_marketplace_links/
-- price_checks (both 014_price_checker.sql) - the established rule for
-- every marketplace_id column in this schema (see PROJECT_STATE/
-- PROTECTED_AREAS.md's "2.2.0" entry: a new marketplace-referencing table
-- that ISN'T ON DELETE CASCADE would make a future full marketplace delete
-- either orphan rows or hard-fail). commands::price_checker::
-- delete_marketplace_impl's own existing guard query was extended in this
-- same release to also count THIS table before allowing a delete, so that
-- rule stays true in practice, not just in the schema - see that
-- function's own doc comment.
--
-- ticket_id is also ON DELETE CASCADE: a listing with no ticket behind it
-- makes no sense. Unlike sales.ticket_id (ON DELETE RESTRICT, protecting
-- real sale history), nothing currently deletes a ticket at all - there is
-- no delete_ticket command anywhere in this codebase today - so this is a
-- safety net for a path that does not exist yet, not a real, reachable risk.
--
-- listing_id is nullable: marko will mostly enter these by hand from a
-- marketplace's own dashboard, and may not always have/enter an external
-- id. UNIQUE(ticket_id, marketplace_id, listing_id) is the "ziadne
-- duplicity" (no duplicates) guard marko asked for - SQLite treats every
-- NULL as distinct from every other NULL in a UNIQUE index, so several
-- hand-entered listings for the same ticket+marketplace with no id yet can
-- still coexist, but the exact same (ticket, marketplace, listing_id) can
-- never be inserted twice once a real id is known.
--
-- price_cents stays INTEGER cents (never REAL), same rule as every money
-- column since 001_initial_schema.sql. `status` is a small, independent
-- lifecycle for the LISTING itself (active/sold/removed) - deliberately
-- NOT the same vocabulary as tickets.status (available/listed/sold/
-- cancelled): one ticket can now have several listings each in a different
-- state at once (still active on Vivid, sold on Ticombo), so folding this
-- into tickets.status would make that column ambiguous. This migration
-- does not touch tickets.status or tickets.listing_price_cents at all, and
-- nothing in commands::ticket_listings ever writes to them - marko's own
-- explicit instruction not to change existing tickets/inventory/sales/
-- refund logic.
--
-- is_demo follows the same convention every other table in this schema
-- already has (tickets/orders/events/sales/marketplaces/price_checks), for
-- consistency with any future demo-data tooling - it is not currently
-- settable through the new commands (always 0/real, same as
-- create_marketplace/create_finance_category, which don't expose it either).
CREATE TABLE IF NOT EXISTS ticket_listings (
  id             INTEGER PRIMARY KEY AUTOINCREMENT,
  ticket_id      INTEGER NOT NULL REFERENCES tickets(id) ON DELETE CASCADE,
  marketplace_id INTEGER NOT NULL REFERENCES marketplaces(id) ON DELETE CASCADE,
  listing_id     TEXT,
  listing_url    TEXT,
  price_cents    INTEGER NOT NULL CHECK (price_cents >= 0),
  currency       TEXT NOT NULL DEFAULT 'EUR',
  status         TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active','sold','removed')),
  is_demo        INTEGER NOT NULL DEFAULT 0,
  created_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  UNIQUE(ticket_id, marketplace_id, listing_id)
);
CREATE INDEX IF NOT EXISTS idx_ticket_listings_ticket ON ticket_listings(ticket_id);
CREATE INDEX IF NOT EXISTS idx_ticket_listings_marketplace ON ticket_listings(marketplace_id);

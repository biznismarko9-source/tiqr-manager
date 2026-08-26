-- TIQR Manager - 011_pulls_received
-- New standalone-but-linkable feature (2.0.17): "Pull received" - the mirror
-- direction of the existing `pulls` table (005_pulls.sql). `pulls` is marko
-- pulling tickets FOR someone else, charging his own fee, and those tickets
-- never become his own inventory. `pulls_received` is the opposite: SOMEONE
-- ELSE pulls tickets FOR marko (marko is the buyer here), marko pays THEM a
-- fee, and - unlike `pulls` - the tickets DO become marko's own inventory,
-- which is why this table (optionally) links to the resulting `orders` row
-- (marko's own 2.0.17 request: a Pulls-section toggle between pulls he did
-- FOR others and pulls he took FROM others, the latter also auto-filled from
-- the Orders & Sales sheet's existing `pull`/`who pulled`/`how much pull`
-- columns whenever `pull` = "yes" - see commands/orders_sheet_sync.rs's
-- module doc comment and its `maybe_link_pull_received`).
--
-- order_id is deliberately nullable, not NOT NULL: marko confirmed (via
-- AskUserQuestion) a received pull must be recordable standalone too, not
-- only when linked to a real order - "Aj samostatne (odporúčam)". The
-- partial unique index below only ever applies to sheet-sync rows (which
-- always ARE linked - see apply_sales_rows), so it never gets in the way of
-- any number of standalone manual rows sharing a NULL or even the same
-- order_id.
--
-- amount_cents (marko's fee to the puller) is deliberately NEVER summed into
-- FinanceSummary/CashflowSummary/Dashboard - marko confirmed (via
-- AskUserQuestion) this stays purely informational - "Len informatívne
-- (odporúčam)" - the exact same standalone-from-finance choice 005_pulls.sql
-- already made for the other direction's price_cents.
--
-- `source` distinguishes a manually-typed row from one auto-created by
-- Orders & Sales sheet sync. The partial UNIQUE index on (order_id) WHERE
-- source = 'sheet_sync' is a DB-level backstop (belt-and-suspenders on top of
-- an application-level check already made before every sync insert) so a
-- re-synced/partially-synced order - see orders_sheet_sync.rs's own
-- "creation-only" doc comment - can never end up with two linked rows.
--
-- event_name/event_date are free text, not a real Event FK, for the exact
-- same reason as `pulls.event_name`/`event_date` (005_pulls.sql) - fast
-- standalone entry when there's no linked order yet. A sheet-sync-created
-- row fills them in automatically from its linked order's own event, so
-- they're never blank there either.

INSERT OR IGNORE INTO counters(name, value) VALUES ('pull_received', 0);

CREATE TABLE IF NOT EXISTS pulls_received (
  id                 INTEGER PRIMARY KEY AUTOINCREMENT,
  code               TEXT NOT NULL UNIQUE,
  -- Who pulled the tickets FOR marko - the reverse of pulls.buyer_name. Free
  -- text by choice, same standalone-fast-entry rationale as buyer_name.
  puller_name        TEXT NOT NULL,
  event_name         TEXT NOT NULL,
  event_date         TEXT, -- ISO 8601 date, nullable (free text, not a real Event)
  quantity           INTEGER NOT NULL DEFAULT 1 CHECK (quantity > 0),
  -- The fee marko paid/owes the puller - informational only, see doc comment
  -- above. Never the ticket price itself - that's marko's own purchase,
  -- which already flows through orders/tickets normally when order_id links
  -- to one.
  amount_cents       INTEGER NOT NULL DEFAULT 0 CHECK (amount_cents >= 0),
  currency           TEXT NOT NULL DEFAULT 'EUR',
  more_info          TEXT,
  order_id           INTEGER REFERENCES orders(id) ON DELETE SET NULL,
  source             TEXT NOT NULL DEFAULT 'manual' CHECK (source IN ('manual','sheet_sync')),
  is_demo            INTEGER NOT NULL DEFAULT 0,
  created_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE INDEX IF NOT EXISTS idx_pulls_received_order ON pulls_received(order_id);
CREATE INDEX IF NOT EXISTS idx_pulls_received_source ON pulls_received(source);
CREATE INDEX IF NOT EXISTS idx_pulls_received_is_demo ON pulls_received(is_demo);
CREATE UNIQUE INDEX IF NOT EXISTS idx_pulls_received_one_sheet_sync_row_per_order
  ON pulls_received(order_id) WHERE source = 'sheet_sync';

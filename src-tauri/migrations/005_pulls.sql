-- TIQR Manager - 005_pulls
-- New standalone feature (1.9.7): "Pull" - buying tickets on behalf of
-- someone else for a fee. marko queues for the sale, adds the cheapest
-- tickets to the cart, the other person sends one-off card details to pay
-- with, and pays marko a fee (e.g. 15 EUR) once the tickets are
-- transferred to them.
--
-- Deliberately NOT linked to events/orders/tickets/sales - the pulled
-- tickets never become marko's own inventory (paid for with the other
-- person's card, transferred away, never resold by him), so there is
-- nothing in the existing schema to join against and no shared lifecycle
-- with a real Order. event_name/event_date are plain free text by choice
-- (marko: "voľný text", not a foreign key to `events`) - this list is meant
-- to be filled in as fast as his own spreadsheet was.
--
-- Also deliberately standalone from `finance.rs`/the Dashboard (marko:
-- "úplne samostatné") - price_cents (his fee) is never summed into
-- FinanceSummary/CashflowSummary/etc. anywhere. If that's wanted later, it's
-- a separate, additive change - nothing here blocks it.

INSERT OR IGNORE INTO counters(name, value) VALUES ('pull', 0);

CREATE TABLE IF NOT EXISTS pulls (
  id                 INTEGER PRIMARY KEY AUTOINCREMENT,
  code               TEXT NOT NULL UNIQUE,
  -- Who marko is pulling the tickets for (marko's choice - see
  -- REDESIGN-1.9.7-REPORT.md - this is the row's primary identity, not just
  -- a label, so it's required rather than folded into more_info).
  buyer_name         TEXT NOT NULL,
  event_name         TEXT NOT NULL,
  event_date         TEXT, -- ISO 8601 date, nullable (free text, not a real Event)
  quantity           INTEGER NOT NULL DEFAULT 1 CHECK (quantity > 0),
  platform_id        INTEGER REFERENCES platforms(id) ON DELETE SET NULL,
  seats              TEXT,
  more_info          TEXT,
  -- marko's OWN fee/reward for doing the pull (e.g. 15.00 EUR) - never the
  -- ticket price itself, which is paid by the other person's card and is
  -- not marko's money or expense.
  price_cents        INTEGER NOT NULL DEFAULT 0 CHECK (price_cents >= 0),
  currency           TEXT NOT NULL DEFAULT 'EUR',
  transfer_deadline  TEXT, -- ISO 8601 date, nullable - by when the tickets must be transferred
  transfer_done      INTEGER NOT NULL DEFAULT 0 CHECK (transfer_done IN (0,1)),
  -- Auto-stamped the moment transfer_done flips 0->1 (and cleared back to
  -- NULL if it's ever flipped back) - see set_pull_transfer_done_impl /
  -- update_pull_impl in commands/pulls.rs. Not a user-editable field.
  transfer_done_at   TEXT,
  is_demo            INTEGER NOT NULL DEFAULT 0,
  created_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE INDEX IF NOT EXISTS idx_pulls_platform ON pulls(platform_id);
CREATE INDEX IF NOT EXISTS idx_pulls_transfer_done ON pulls(transfer_done);
CREATE INDEX IF NOT EXISTS idx_pulls_transfer_deadline ON pulls(transfer_deadline);
CREATE INDEX IF NOT EXISTS idx_pulls_is_demo ON pulls(is_demo);

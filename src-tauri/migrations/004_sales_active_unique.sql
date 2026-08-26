-- 004_sales_active_unique
--
-- BUG FIX (critical): sales.ticket_id was UNIQUE across ALL rows forever
-- (see 001). refund_sale never deletes the sales row - refund history must
-- be preserved (002) - it only flips payment_status to 'refunded' and
-- returns the ticket to 'available'. Combined, this meant a refunded ticket
-- could never be sold again: any new INSERT INTO sales for that ticket_id
-- hit the old UNIQUE constraint and was rejected as "already sold", even
-- though the UI correctly showed the ticket as Available and the Refund
-- dialog promised it could be resold.
--
-- The fix: uniqueness must apply only among ACTIVE (non-refunded) sales,
-- not across all of history. "A ticket can never be sold twice" stays a
-- real, enforced database guarantee - it now means "a ticket can never have
-- two simultaneously ACTIVE sales", which is what was always intended.
--
-- SQLite has no ALTER TABLE for dropping an inline column constraint, so
-- this rebuilds the table following SQLite's own documented procedure for
-- schema changes ALTER TABLE can't express directly
-- (https://www.sqlite.org/lang_altertable.html#otheralter):
-- disable FK enforcement, do the rebuild inside one transaction (so it's
-- fully atomic - either the whole thing applies or none of it does), copy
-- every column of every existing row across unchanged (zero data loss, all
-- refund history preserved exactly as it was), then recreate the indexes
-- that lived on the old table (DROP TABLE removes them) plus the new
-- partial unique index that is the actual fix.

PRAGMA foreign_keys = OFF;

BEGIN TRANSACTION;

DROP TABLE IF EXISTS sales_new;

CREATE TABLE sales_new (
  id                  INTEGER PRIMARY KEY AUTOINCREMENT,
  code                TEXT NOT NULL UNIQUE,
  ticket_id           INTEGER NOT NULL REFERENCES tickets(id) ON DELETE RESTRICT,
  platform_id         INTEGER REFERENCES platforms(id) ON DELETE SET NULL,
  sale_date           TEXT NOT NULL,
  sale_price_cents    INTEGER NOT NULL DEFAULT 0 CHECK (sale_price_cents >= 0),
  selling_fees_cents  INTEGER NOT NULL DEFAULT 0 CHECK (selling_fees_cents >= 0),
  currency            TEXT NOT NULL DEFAULT 'EUR',
  payment_status      TEXT NOT NULL DEFAULT 'pending' CHECK (payment_status IN ('pending','paid','refunded')),
  buyer_reference     TEXT,
  notes               TEXT,
  is_demo             INTEGER NOT NULL DEFAULT 0,
  created_at          TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at          TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  refunded_at         TEXT,
  refund_reason       TEXT,
  batch_id            TEXT
);

INSERT INTO sales_new (
  id, code, ticket_id, platform_id, sale_date, sale_price_cents, selling_fees_cents,
  currency, payment_status, buyer_reference, notes, is_demo, created_at, updated_at,
  refunded_at, refund_reason, batch_id
)
SELECT
  id, code, ticket_id, platform_id, sale_date, sale_price_cents, selling_fees_cents,
  currency, payment_status, buyer_reference, notes, is_demo, created_at, updated_at,
  refunded_at, refund_reason, batch_id
FROM sales;

DROP TABLE sales;
ALTER TABLE sales_new RENAME TO sales;

CREATE INDEX IF NOT EXISTS idx_sales_platform ON sales(platform_id);
CREATE INDEX IF NOT EXISTS idx_sales_date ON sales(sale_date);
CREATE INDEX IF NOT EXISTS idx_sales_is_demo ON sales(is_demo);
CREATE INDEX IF NOT EXISTS idx_sales_batch ON sales(batch_id);

-- The actual fix: at most one ACTIVE (non-refunded) sale per ticket. A
-- refunded row is exempt, so it no longer blocks a future resale, while two
-- simultaneously-active sales of the same ticket remain impossible.
CREATE UNIQUE INDEX IF NOT EXISTS idx_sales_ticket_active_unique
  ON sales(ticket_id) WHERE payment_status != 'refunded';

COMMIT;

PRAGMA foreign_keys = ON;

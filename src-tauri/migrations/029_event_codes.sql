-- 029: event-derived codes (2.30.0)
--
-- Marko: "ked je to celine dion tak celine-001 ... bolo by to lepsie
-- zmapovatelne nez to ako to je teraz", and "nech je vsade rovnaky" - one
-- format everywhere rather than ORD-000091 in one place and #91 in another.
--
-- This migration only ADDS the column that makes the rewrite reversible and
-- keeps Google Sheets working. The rewrite itself happens in Rust
-- (`codes::backfill_event_codes`), because the prefix rule folds accents and
-- applies word rules that SQL cannot express - and having two different
-- implementations of that rule is exactly how the two machines would end up
-- minting two different codes for the same event.
--
-- `legacy_code` is the code this record had before the rewrite. It is what
-- lets an already-synced Google Sheet keep matching its rows: the sheet
-- stores the old code in its "TIQR ID" column, and the sync falls back to
-- this column when a TIQR ID matches no current code.
BEGIN TRANSACTION;

ALTER TABLE orders          ADD COLUMN legacy_code TEXT;
ALTER TABLE tickets         ADD COLUMN legacy_code TEXT;
ALTER TABLE sales           ADD COLUMN legacy_code TEXT;
ALTER TABLE pulls           ADD COLUMN legacy_code TEXT;
ALTER TABLE pulls_received  ADD COLUMN legacy_code TEXT;

-- Looked up on every Sheets row that fails to match a current code, so it is
-- worth an index even though it is only read on that fallback path.
CREATE INDEX IF NOT EXISTS idx_orders_legacy_code  ON orders(legacy_code);
CREATE INDEX IF NOT EXISTS idx_tickets_legacy_code ON tickets(legacy_code);
CREATE INDEX IF NOT EXISTS idx_sales_legacy_code   ON sales(legacy_code);
CREATE INDEX IF NOT EXISTS idx_pulls_legacy_code   ON pulls(legacy_code);

COMMIT;

-- 007_payments
-- Payments 2.0: a real payment ledger, replacing the binary
-- sales.payment_status/orders.payment_status ('paid' or not) as the source
-- of truth for how much has actually been received. Neither existing column
-- is touched or removed by this migration - see payments.rs for how they
-- keep working as a quick "record a full payment" shortcut on top of this
-- table, not a separate parallel source of truth.
--
-- One Sale/Order can have MANY payments (partial payments are the whole
-- point of this version) - never the reverse - so this is its own table,
-- not new columns bolted onto `sales`/`orders`.
--
-- sale_group_key / order_id: a payment belongs to exactly one of a sale
-- GROUP (not a single `sales` row) or an order - the CHECK below enforces
-- that. Sales are grouped by `COALESCE(batch_id, 'single:'||id)` everywhere
-- else in this app (see GROUP_KEY_EXPR in sales.rs) - sale_group_key stores
-- exactly that same value, not a raw `sales.id`. That matters: `batch_id`
-- itself never changes once a batch exists, even if the specific row that
-- happens to be the group's lowest surviving id changes (e.g. after that
-- row is deleted) - a payment tied to the (stable) group key survives that
-- kind of housekeeping instead of needing to be re-parented every time.
-- Orders don't have this grouping problem (one order = one row), so
-- order_id is a plain, ordinary foreign key.
CREATE TABLE IF NOT EXISTS payments (
  id                 INTEGER PRIMARY KEY AUTOINCREMENT,
  code               TEXT NOT NULL UNIQUE,
  sale_group_key     TEXT,
  order_id           INTEGER REFERENCES orders(id) ON DELETE RESTRICT,
  amount_cents       INTEGER NOT NULL CHECK (amount_cents > 0),
  currency           TEXT NOT NULL,
  payment_date       TEXT NOT NULL,
  -- Small, fixed set on purpose (marko: "don't add dozens of methods") - a
  -- CHECK enum, not a Lookup table, since unlike Platforms/Suppliers this
  -- isn't something a reseller customizes per install. 'other' pairs with
  -- method_other_note for a free-text description.
  method             TEXT NOT NULL DEFAULT 'other'
                      CHECK (method IN ('bank_transfer','card','revolut','cash','paypal','other')),
  method_other_note  TEXT,
  reference          TEXT,
  -- 1 = auto-created by the "Mark as Paid" shortcut (Sale Detail's bulk
  -- action / Order Edit's Payment status field) rather than a real payment
  -- someone manually entered with its own date/method/reference. Lets the
  -- shortcut's reverse direction ("Mark as Pending") safely undo ONLY what
  -- it itself created, never a genuine manually-entered payment - see
  -- payments.rs.
  is_shortcut        INTEGER NOT NULL DEFAULT 0,
  is_demo            INTEGER NOT NULL DEFAULT 0,
  created_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  CHECK (
    (sale_group_key IS NOT NULL AND order_id IS NULL)
    OR (sale_group_key IS NULL AND order_id IS NOT NULL)
  )
);
CREATE INDEX IF NOT EXISTS idx_payments_sale_group ON payments(sale_group_key);
CREATE INDEX IF NOT EXISTS idx_payments_order ON payments(order_id);
CREATE INDEX IF NOT EXISTS idx_payments_date ON payments(payment_date);
INSERT OR IGNORE INTO counters(name, value) VALUES ('payment', 0);

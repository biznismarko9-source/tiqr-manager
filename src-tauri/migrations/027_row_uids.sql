-- 027: a stable identity for every row that can travel between machines.
--
-- WHY THIS EXISTS
--
-- marko asked for the two machines to MERGE instead of one overwriting the
-- other ("ta novsia verzia vzdy sa syncne s tou starou ... cize nejaky order
-- naviac, sale, pull atd"). Merging is impossible without being able to say
-- "this row here and that row there are the same row" - and today nothing can
-- say that: every primary key in this app is a per-machine INTEGER
-- AUTOINCREMENT, so the Mac and the PC both mint id 5 for two completely
-- different orders. That is the single reason 2.12.0 shipped whole-file sync
-- instead of a merge (see PROTECTED_AREAS.md).
--
-- This migration adds that identity. It changes NO behaviour by itself: no
-- code reads `uid` yet, nothing syncs differently, every existing query still
-- returns exactly what it returned before.
--
-- THE TWO HALVES OF THE BACKFILL, AND WHY THEY DIFFER
--
-- Rows that ALREADY EXIST get 'legacy-<id>'. That is deliberate and it is the
-- whole trick: both machines' databases descend from the same file (that is
-- what whole-file sync did), so a row that exists on both has the same id on
-- both, and therefore gets the SAME uid on both, without either machine
-- talking to the other. Shared history becomes shared identity for free.
--
-- Rows created FROM NOW ON get a random 128-bit uid, assigned by the triggers
-- below. Two machines cannot collide on those.
--
-- THE ONE THING THAT MUST HAPPEN AFTER THIS SHIPS
--
-- If the two machines had DIVERGED before updating (each already holding rows
-- the other has never seen), their post-divergence rows carry colliding
-- 'legacy-N' uids, because both counted up from the same place. So there is
-- exactly one more whole-file sync to do: update both machines, then push from
-- whichever is current and pull on the other. After that one hand-off the
-- identities agree and nothing has to be overwritten ever again. This is
-- called out in the changelog rather than automated, because picking which
-- side is "current" is the one judgement no timer may make.
--
-- WHY TRIGGERS AND NOT RUST
--
-- The uid is assigned by an AFTER INSERT trigger per table, so not one of the
-- app's existing INSERT statements has to change - not the order/ticket
-- writer, not the CSV import, not the Sheets sync, not the AI import. A new
-- insert site added later gets a uid automatically too, which is the failure
-- mode worth designing out: a row with no uid is a row a merge cannot see.
--
-- WHAT IS DELIBERATELY NOT HERE
--
-- * marketplaces, and the price-checker/market_* tables: the first is seeded
--   identically by these very migrations, the rest are per-machine scan
--   history, not marko's business data.
-- * app_settings, app_secrets, counters, notification_log, sheet_sync_links:
--   bookkeeping - the same list db.rs's `is_bookkeeping_table` already keeps
--   out of the sync dirty-flag, for the same reason.
-- * Tombstones for deletes. An insert-merge cannot lose data; propagating a
--   DELETE across machines can. That needs its own design pass and its own
--   migration, and it is not smuggled in here.
--
-- WRAPPED IN ONE TRANSACTION, ON PURPOSE
--
-- This is 17 independent ALTER TABLEs, and `ALTER TABLE ADD COLUMN` has no
-- IF NOT EXISTS. The migration runner only records a version AFTER the whole
-- file succeeded, so a failure halfway through would leave some tables with
-- `uid` and some without, and the retry on next launch would then die on
-- "duplicate column name" - permanently. All-or-nothing removes that
-- possibility. Same BEGIN TRANSACTION/COMMIT shape migrations 004 and 020
-- already use.

BEGIN TRANSACTION;

-- events: an event.
ALTER TABLE events ADD COLUMN uid TEXT;
UPDATE events SET uid = 'legacy-' || id WHERE uid IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_events_uid ON events(uid);
CREATE TRIGGER IF NOT EXISTS events_uid_after_insert
AFTER INSERT ON events
WHEN NEW.uid IS NULL
BEGIN
    UPDATE events SET uid = lower(hex(randomblob(16))) WHERE id = NEW.id;
END;

-- orders: a purchase.
ALTER TABLE orders ADD COLUMN uid TEXT;
UPDATE orders SET uid = 'legacy-' || id WHERE uid IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_orders_uid ON orders(uid);
CREATE TRIGGER IF NOT EXISTS orders_uid_after_insert
AFTER INSERT ON orders
WHEN NEW.uid IS NULL
BEGIN
    UPDATE orders SET uid = lower(hex(randomblob(16))) WHERE id = NEW.id;
END;

-- tickets: a ticket.
ALTER TABLE tickets ADD COLUMN uid TEXT;
UPDATE tickets SET uid = 'legacy-' || id WHERE uid IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_tickets_uid ON tickets(uid);
CREATE TRIGGER IF NOT EXISTS tickets_uid_after_insert
AFTER INSERT ON tickets
WHEN NEW.uid IS NULL
BEGIN
    UPDATE tickets SET uid = lower(hex(randomblob(16))) WHERE id = NEW.id;
END;

-- sales: a sale line.
ALTER TABLE sales ADD COLUMN uid TEXT;
UPDATE sales SET uid = 'legacy-' || id WHERE uid IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_sales_uid ON sales(uid);
CREATE TRIGGER IF NOT EXISTS sales_uid_after_insert
AFTER INSERT ON sales
WHEN NEW.uid IS NULL
BEGIN
    UPDATE sales SET uid = lower(hex(randomblob(16))) WHERE id = NEW.id;
END;

-- pulls: a pull.
ALTER TABLE pulls ADD COLUMN uid TEXT;
UPDATE pulls SET uid = 'legacy-' || id WHERE uid IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_pulls_uid ON pulls(uid);
CREATE TRIGGER IF NOT EXISTS pulls_uid_after_insert
AFTER INSERT ON pulls
WHEN NEW.uid IS NULL
BEGIN
    UPDATE pulls SET uid = lower(hex(randomblob(16))) WHERE id = NEW.id;
END;

-- pulls_received: a received pull.
ALTER TABLE pulls_received ADD COLUMN uid TEXT;
UPDATE pulls_received SET uid = 'legacy-' || id WHERE uid IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_pulls_received_uid ON pulls_received(uid);
CREATE TRIGGER IF NOT EXISTS pulls_received_uid_after_insert
AFTER INSERT ON pulls_received
WHEN NEW.uid IS NULL
BEGIN
    UPDATE pulls_received SET uid = lower(hex(randomblob(16))) WHERE id = NEW.id;
END;

-- ticket_listings: a listing.
ALTER TABLE ticket_listings ADD COLUMN uid TEXT;
UPDATE ticket_listings SET uid = 'legacy-' || id WHERE uid IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_ticket_listings_uid ON ticket_listings(uid);
CREATE TRIGGER IF NOT EXISTS ticket_listings_uid_after_insert
AFTER INSERT ON ticket_listings
WHEN NEW.uid IS NULL
BEGIN
    UPDATE ticket_listings SET uid = lower(hex(randomblob(16))) WHERE id = NEW.id;
END;

-- event_marketplace_links: an event/marketplace link.
ALTER TABLE event_marketplace_links ADD COLUMN uid TEXT;
UPDATE event_marketplace_links SET uid = 'legacy-' || id WHERE uid IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_event_marketplace_links_uid ON event_marketplace_links(uid);
CREATE TRIGGER IF NOT EXISTS event_marketplace_links_uid_after_insert
AFTER INSERT ON event_marketplace_links
WHEN NEW.uid IS NULL
BEGIN
    UPDATE event_marketplace_links SET uid = lower(hex(randomblob(16))) WHERE id = NEW.id;
END;

-- payments: a payment.
ALTER TABLE payments ADD COLUMN uid TEXT;
UPDATE payments SET uid = 'legacy-' || id WHERE uid IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_payments_uid ON payments(uid);
CREATE TRIGGER IF NOT EXISTS payments_uid_after_insert
AFTER INSERT ON payments
WHEN NEW.uid IS NULL
BEGIN
    UPDATE payments SET uid = lower(hex(randomblob(16))) WHERE id = NEW.id;
END;

-- finance_entries: a finance entry.
ALTER TABLE finance_entries ADD COLUMN uid TEXT;
UPDATE finance_entries SET uid = 'legacy-' || id WHERE uid IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_finance_entries_uid ON finance_entries(uid);
CREATE TRIGGER IF NOT EXISTS finance_entries_uid_after_insert
AFTER INSERT ON finance_entries
WHEN NEW.uid IS NULL
BEGIN
    UPDATE finance_entries SET uid = lower(hex(randomblob(16))) WHERE id = NEW.id;
END;

-- accounts: an account.
ALTER TABLE accounts ADD COLUMN uid TEXT;
UPDATE accounts SET uid = 'legacy-' || id WHERE uid IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_accounts_uid ON accounts(uid);
CREATE TRIGGER IF NOT EXISTS accounts_uid_after_insert
AFTER INSERT ON accounts
WHEN NEW.uid IS NULL
BEGIN
    UPDATE accounts SET uid = lower(hex(randomblob(16))) WHERE id = NEW.id;
END;

-- transfers: a transfer.
ALTER TABLE transfers ADD COLUMN uid TEXT;
UPDATE transfers SET uid = 'legacy-' || id WHERE uid IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_transfers_uid ON transfers(uid);
CREATE TRIGGER IF NOT EXISTS transfers_uid_after_insert
AFTER INSERT ON transfers
WHEN NEW.uid IS NULL
BEGIN
    UPDATE transfers SET uid = lower(hex(randomblob(16))) WHERE id = NEW.id;
END;

-- recurring_expenses: a recurring expense.
ALTER TABLE recurring_expenses ADD COLUMN uid TEXT;
UPDATE recurring_expenses SET uid = 'legacy-' || id WHERE uid IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_recurring_expenses_uid ON recurring_expenses(uid);
CREATE TRIGGER IF NOT EXISTS recurring_expenses_uid_after_insert
AFTER INSERT ON recurring_expenses
WHEN NEW.uid IS NULL
BEGIN
    UPDATE recurring_expenses SET uid = lower(hex(randomblob(16))) WHERE id = NEW.id;
END;

-- platforms: a platform.
ALTER TABLE platforms ADD COLUMN uid TEXT;
UPDATE platforms SET uid = 'legacy-' || id WHERE uid IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_platforms_uid ON platforms(uid);
CREATE TRIGGER IF NOT EXISTS platforms_uid_after_insert
AFTER INSERT ON platforms
WHEN NEW.uid IS NULL
BEGIN
    UPDATE platforms SET uid = lower(hex(randomblob(16))) WHERE id = NEW.id;
END;

-- suppliers: a supplier.
ALTER TABLE suppliers ADD COLUMN uid TEXT;
UPDATE suppliers SET uid = 'legacy-' || id WHERE uid IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_suppliers_uid ON suppliers(uid);
CREATE TRIGGER IF NOT EXISTS suppliers_uid_after_insert
AFTER INSERT ON suppliers
WHEN NEW.uid IS NULL
BEGIN
    UPDATE suppliers SET uid = lower(hex(randomblob(16))) WHERE id = NEW.id;
END;

-- event_categories: an event category.
ALTER TABLE event_categories ADD COLUMN uid TEXT;
UPDATE event_categories SET uid = 'legacy-' || id WHERE uid IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_event_categories_uid ON event_categories(uid);
CREATE TRIGGER IF NOT EXISTS event_categories_uid_after_insert
AFTER INSERT ON event_categories
WHEN NEW.uid IS NULL
BEGIN
    UPDATE event_categories SET uid = lower(hex(randomblob(16))) WHERE id = NEW.id;
END;

-- finance_categories: a finance category.
ALTER TABLE finance_categories ADD COLUMN uid TEXT;
UPDATE finance_categories SET uid = 'legacy-' || id WHERE uid IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_finance_categories_uid ON finance_categories(uid);
CREATE TRIGGER IF NOT EXISTS finance_categories_uid_after_insert
AFTER INSERT ON finance_categories
WHEN NEW.uid IS NULL
BEGIN
    UPDATE finance_categories SET uid = lower(hex(randomblob(16))) WHERE id = NEW.id;
END;

COMMIT;

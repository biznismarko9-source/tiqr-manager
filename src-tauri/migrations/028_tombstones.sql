-- 028: tombstones - a deletion is a fact, and it has to travel too.
--
-- WHY THIS EXISTS
--
-- 2.16.0's merge can only ADD. That was deliberate and it is why it cannot
-- lose anything, but it left one honest hole, and marko named it: delete an
-- order on the Mac and it stays on the PC - and the next merge, seeing a
-- record the Mac "has never seen", copies it straight back. The deletion
-- undoes itself.
--
-- The reason a merge cannot simply notice the absence is that a row which is
-- merely NOT THERE is indistinguishable from one that has not arrived yet.
-- Absence carries no information. So the deletion has to leave something
-- behind, and that is what this table is.
--
-- WHY TRIGGERS AND NOT RUST
--
-- The same reasoning as 027's uid: a tombstone written by application code is
-- a tombstone that gets forgotten the first time someone deletes a row from a
-- path nobody thought about - a cascade, a bulk action, a cleanup. An AFTER
-- DELETE trigger per table cannot be bypassed.
--
-- Verified against real SQLite rather than assumed: an AFTER DELETE trigger
-- DOES fire for rows removed by ON DELETE CASCADE, with or without
-- `recursive_triggers`. That matters here because deleting an order cascades
-- to its tickets (001's `tickets.order_id ... ON DELETE CASCADE`), and
-- without it those tickets would leave no tombstone and could be copied back
-- as orphan-less rows.
--
-- WHAT A TOMBSTONE MEANS, EXACTLY
--
-- "The row with this uid was deleted on some machine." It does not record
-- WHICH machine, and it does not carry a version. So when a tombstone meets a
-- row that the other machine has meanwhile EDITED, the deletion wins. That is
-- a real decision, not an oversight: the alternative is resurrecting a record
-- its owner deliberately removed, and there is no third answer without
-- per-row history this app does not keep.
--
-- A tombstone is never removed. They are tiny - a table name and a uid - and
-- deleting one would let the record it describes come back on the next merge
-- from a machine that still has it.

BEGIN TRANSACTION;

CREATE TABLE IF NOT EXISTS deleted_rows (
  table_name TEXT NOT NULL,
  uid        TEXT NOT NULL,
  deleted_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  PRIMARY KEY (table_name, uid)
);
-- Deliberately a rowid table, NOT `WITHOUT ROWID`: SQLite's update hook -
-- which is how this app notices it has unsynced changes (db.rs) - is not
-- called for WITHOUT ROWID tables, so a tombstone arriving from the other
-- machine would not mark the database as needing a push and could sit here
-- unpropagated.

CREATE TRIGGER IF NOT EXISTS events_tombstone
AFTER DELETE ON events
WHEN OLD.uid IS NOT NULL
BEGIN
    INSERT OR REPLACE INTO deleted_rows(table_name, uid, deleted_at)
    VALUES ('events', OLD.uid, strftime('%Y-%m-%dT%H:%M:%fZ','now'));
END;

CREATE TRIGGER IF NOT EXISTS orders_tombstone
AFTER DELETE ON orders
WHEN OLD.uid IS NOT NULL
BEGIN
    INSERT OR REPLACE INTO deleted_rows(table_name, uid, deleted_at)
    VALUES ('orders', OLD.uid, strftime('%Y-%m-%dT%H:%M:%fZ','now'));
END;

CREATE TRIGGER IF NOT EXISTS tickets_tombstone
AFTER DELETE ON tickets
WHEN OLD.uid IS NOT NULL
BEGIN
    INSERT OR REPLACE INTO deleted_rows(table_name, uid, deleted_at)
    VALUES ('tickets', OLD.uid, strftime('%Y-%m-%dT%H:%M:%fZ','now'));
END;

CREATE TRIGGER IF NOT EXISTS sales_tombstone
AFTER DELETE ON sales
WHEN OLD.uid IS NOT NULL
BEGIN
    INSERT OR REPLACE INTO deleted_rows(table_name, uid, deleted_at)
    VALUES ('sales', OLD.uid, strftime('%Y-%m-%dT%H:%M:%fZ','now'));
END;

CREATE TRIGGER IF NOT EXISTS pulls_tombstone
AFTER DELETE ON pulls
WHEN OLD.uid IS NOT NULL
BEGIN
    INSERT OR REPLACE INTO deleted_rows(table_name, uid, deleted_at)
    VALUES ('pulls', OLD.uid, strftime('%Y-%m-%dT%H:%M:%fZ','now'));
END;

CREATE TRIGGER IF NOT EXISTS pulls_received_tombstone
AFTER DELETE ON pulls_received
WHEN OLD.uid IS NOT NULL
BEGIN
    INSERT OR REPLACE INTO deleted_rows(table_name, uid, deleted_at)
    VALUES ('pulls_received', OLD.uid, strftime('%Y-%m-%dT%H:%M:%fZ','now'));
END;

CREATE TRIGGER IF NOT EXISTS ticket_listings_tombstone
AFTER DELETE ON ticket_listings
WHEN OLD.uid IS NOT NULL
BEGIN
    INSERT OR REPLACE INTO deleted_rows(table_name, uid, deleted_at)
    VALUES ('ticket_listings', OLD.uid, strftime('%Y-%m-%dT%H:%M:%fZ','now'));
END;

CREATE TRIGGER IF NOT EXISTS event_marketplace_links_tombstone
AFTER DELETE ON event_marketplace_links
WHEN OLD.uid IS NOT NULL
BEGIN
    INSERT OR REPLACE INTO deleted_rows(table_name, uid, deleted_at)
    VALUES ('event_marketplace_links', OLD.uid, strftime('%Y-%m-%dT%H:%M:%fZ','now'));
END;

CREATE TRIGGER IF NOT EXISTS payments_tombstone
AFTER DELETE ON payments
WHEN OLD.uid IS NOT NULL
BEGIN
    INSERT OR REPLACE INTO deleted_rows(table_name, uid, deleted_at)
    VALUES ('payments', OLD.uid, strftime('%Y-%m-%dT%H:%M:%fZ','now'));
END;

CREATE TRIGGER IF NOT EXISTS finance_entries_tombstone
AFTER DELETE ON finance_entries
WHEN OLD.uid IS NOT NULL
BEGIN
    INSERT OR REPLACE INTO deleted_rows(table_name, uid, deleted_at)
    VALUES ('finance_entries', OLD.uid, strftime('%Y-%m-%dT%H:%M:%fZ','now'));
END;

CREATE TRIGGER IF NOT EXISTS accounts_tombstone
AFTER DELETE ON accounts
WHEN OLD.uid IS NOT NULL
BEGIN
    INSERT OR REPLACE INTO deleted_rows(table_name, uid, deleted_at)
    VALUES ('accounts', OLD.uid, strftime('%Y-%m-%dT%H:%M:%fZ','now'));
END;

CREATE TRIGGER IF NOT EXISTS transfers_tombstone
AFTER DELETE ON transfers
WHEN OLD.uid IS NOT NULL
BEGIN
    INSERT OR REPLACE INTO deleted_rows(table_name, uid, deleted_at)
    VALUES ('transfers', OLD.uid, strftime('%Y-%m-%dT%H:%M:%fZ','now'));
END;

CREATE TRIGGER IF NOT EXISTS recurring_expenses_tombstone
AFTER DELETE ON recurring_expenses
WHEN OLD.uid IS NOT NULL
BEGIN
    INSERT OR REPLACE INTO deleted_rows(table_name, uid, deleted_at)
    VALUES ('recurring_expenses', OLD.uid, strftime('%Y-%m-%dT%H:%M:%fZ','now'));
END;

CREATE TRIGGER IF NOT EXISTS platforms_tombstone
AFTER DELETE ON platforms
WHEN OLD.uid IS NOT NULL
BEGIN
    INSERT OR REPLACE INTO deleted_rows(table_name, uid, deleted_at)
    VALUES ('platforms', OLD.uid, strftime('%Y-%m-%dT%H:%M:%fZ','now'));
END;

CREATE TRIGGER IF NOT EXISTS suppliers_tombstone
AFTER DELETE ON suppliers
WHEN OLD.uid IS NOT NULL
BEGIN
    INSERT OR REPLACE INTO deleted_rows(table_name, uid, deleted_at)
    VALUES ('suppliers', OLD.uid, strftime('%Y-%m-%dT%H:%M:%fZ','now'));
END;

CREATE TRIGGER IF NOT EXISTS event_categories_tombstone
AFTER DELETE ON event_categories
WHEN OLD.uid IS NOT NULL
BEGIN
    INSERT OR REPLACE INTO deleted_rows(table_name, uid, deleted_at)
    VALUES ('event_categories', OLD.uid, strftime('%Y-%m-%dT%H:%M:%fZ','now'));
END;

CREATE TRIGGER IF NOT EXISTS finance_categories_tombstone
AFTER DELETE ON finance_categories
WHEN OLD.uid IS NOT NULL
BEGIN
    INSERT OR REPLACE INTO deleted_rows(table_name, uid, deleted_at)
    VALUES ('finance_categories', OLD.uid, strftime('%Y-%m-%dT%H:%M:%fZ','now'));
END;

COMMIT;

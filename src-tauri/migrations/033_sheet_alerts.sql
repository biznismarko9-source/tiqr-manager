-- 033: Sheet alerts - a reminder attached to a sheet, at a time.
--
-- WHY THIS EXISTS
--
-- marko, choosing the dark "Classic" grid: "nejake alert by som si tam chcel
-- nastavit casovo a tak". A reseller's sheet is full of things that are only
-- useful before a date - send the remaining codes on the 28th, chase a refund
-- on Friday - and a row that just sits there does not remind anybody.
--
-- WHY IT IS ITS OWN TABLE AND NOT A COLUMN ON note_rows
--
-- An alert is not a property of a row. It can point at a row, at one cell, or
-- at nothing but the sheet itself, and several can exist for the same row.
-- Putting a `remind_at` on note_rows would allow exactly one per row, and
-- would drag reminder state through every cell write.
--
-- `row_id` and `col_index` are BOTH optional. A sheet-level alert has neither;
-- a cell alert has both. `row_id` is a plain integer with no foreign key on
-- purpose: rows are deleted freely here, and an alert whose row is gone is
-- still a real reminder marko wrote. The UI shows it against the sheet.
--
-- WHY `notified` IS SEPARATE FROM `done`
--
-- `notified` means "the desktop notification for this has already been shown
-- on this machine" - it stops the same reminder firing every minute while it
-- is overdue. `done` means marko ticked it off. They are different questions
-- and one must not be inferred from the other.
--
-- NOTE ON `notified` AND SYNC: it travels with the row like every other
-- column, so a reminder already shown on the Mac will not fire again on the
-- PC after a merge. That is deliberate - one reminder, not two.
--
-- SYNC
--
-- All THREE pieces, per PROTECTED_AREAS ("a new synced table needs all THREE
-- pieces"): the `uid` column with the 027 insert trigger, the 028 delete
-- tombstone, and an entry in cloud_merge::MERGE_TABLES with sheet_id declared
-- as a foreign key into note_sheets. Miss one and a merge silently keeps only
-- the local copy.

CREATE TABLE IF NOT EXISTS sheet_alerts (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    uid         TEXT UNIQUE,
    sheet_id    INTEGER NOT NULL REFERENCES note_sheets(id) ON DELETE CASCADE,
    -- Optional anchor. Both NULL = the alert belongs to the sheet as a whole.
    row_id      INTEGER,
    col_index   INTEGER,
    title       TEXT NOT NULL DEFAULT '',
    note        TEXT NOT NULL DEFAULT '',
    -- Local wall-clock 'YYYY-MM-DDTHH:MM', not UTC: "remind me at 9:00" means
    -- 9:00 where marko is, on whichever machine he is at.
    remind_at   TEXT NOT NULL,
    done        INTEGER NOT NULL DEFAULT 0,
    notified    INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

-- The two questions ever asked of this table: "what is due?" and "what does
-- this sheet have?"
CREATE INDEX IF NOT EXISTS idx_sheet_alerts_due ON sheet_alerts(done, remind_at);
CREATE INDEX IF NOT EXISTS idx_sheet_alerts_sheet ON sheet_alerts(sheet_id);

CREATE TRIGGER IF NOT EXISTS sheet_alerts_uid_after_insert
AFTER INSERT ON sheet_alerts
WHEN NEW.uid IS NULL
BEGIN
    UPDATE sheet_alerts SET uid = lower(hex(randomblob(16))) WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS sheet_alerts_tombstone
AFTER DELETE ON sheet_alerts
WHEN OLD.uid IS NOT NULL
BEGIN
    INSERT OR REPLACE INTO deleted_rows(table_name, uid, deleted_at)
    VALUES ('sheet_alerts', OLD.uid, strftime('%Y-%m-%dT%H:%M:%fZ','now'));
END;

-- 2.55.0 also frees the grid itself: a column no longer has to be named, so
-- the header can be plain A / B / C and marko writes whatever he wants
-- wherever he wants ("chcem aby to bolo volne a vedel s tym pracovat"). That
-- needed no schema change at all - `columns_json` already holds a list of
-- strings and an empty string is a perfectly good "no label". The validation
-- that rejected an empty name lived in Rust and is gone.

-- 031: Notes - sheets with columns marko names himself.
--
-- WHY THIS EXISTS
--
-- marko: "chcem urobit miesto kde si viem zapisovat vsetky dolezite info,
-- urobit nieco podobne v apke ako su google sheets a tam si vediet vpisat
-- dolezite veci, datumy, atd". Asked which shape he wanted, he picked sheets
-- with his own columns over free-text pages, and standalone over notes
-- attached to an event or order.
--
-- WHY THE CELLS ARE JSON AND NOT A THIRD TABLE
--
-- The obvious "proper" schema is note_columns + note_cells, three tables and a
-- join per render. It would buy the ability to query one column across rows -
-- and nothing here ever will. This is a notepad: every cell is free text the
-- person typed, read only as part of its own row, and written a whole row at a
-- time. `cells_json` is a JSON array of strings positionally aligned with the
-- sheet's `columns_json`, which is the same shape the UI already holds.
--
-- Re-shaping on a column change is therefore a rewrite of the affected rows,
-- done in Rust inside one transaction (commands::notes::set_sheet_columns) -
-- never a partial update, so a row's cells can never drift out of alignment
-- with its sheet's columns.
--
-- SYNC
--
-- Both tables carry `uid` and get the same insert trigger migration 027 gave
-- every other syncing table, plus the delete tombstone from 028, and both are
-- listed in cloud_merge::MERGE_TABLES. Without all three a merge between the
-- two machines would silently keep only the local copy - which for the one
-- place marko keeps "vsetky dolezite info" would be the worst possible
-- failure. New tables need no `legacy-` backfill: nothing exists yet, so every
-- row here is born with a random uid on whichever machine made it.

CREATE TABLE IF NOT EXISTS note_sheets (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    uid          TEXT UNIQUE,
    name         TEXT NOT NULL,
    -- JSON array of column names, e.g. ["Event","Deadline","Note"].
    columns_json TEXT NOT NULL DEFAULT '[]',
    position     INTEGER NOT NULL DEFAULT 0,
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TABLE IF NOT EXISTS note_rows (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    uid        TEXT UNIQUE,
    sheet_id   INTEGER NOT NULL REFERENCES note_sheets(id) ON DELETE CASCADE,
    position   INTEGER NOT NULL DEFAULT 0,
    -- JSON array of strings, positionally aligned with the sheet's columns.
    cells_json TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE INDEX IF NOT EXISTS idx_note_rows_sheet ON note_rows(sheet_id, position);

-- Same identity mechanism as migration 027 gave every other syncing table.
CREATE TRIGGER IF NOT EXISTS note_sheets_uid_after_insert
AFTER INSERT ON note_sheets
WHEN NEW.uid IS NULL
BEGIN
    UPDATE note_sheets SET uid = lower(hex(randomblob(16))) WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS note_rows_uid_after_insert
AFTER INSERT ON note_rows
WHEN NEW.uid IS NULL
BEGIN
    UPDATE note_rows SET uid = lower(hex(randomblob(16))) WHERE id = NEW.id;
END;

-- Same delete tombstones as migration 028, so a sheet or row deleted on one
-- machine is deleted on the other instead of coming back at the next merge.
CREATE TRIGGER IF NOT EXISTS note_sheets_tombstone
AFTER DELETE ON note_sheets
WHEN OLD.uid IS NOT NULL
BEGIN
    INSERT OR REPLACE INTO deleted_rows(table_name, uid, deleted_at)
    VALUES ('note_sheets', OLD.uid, strftime('%Y-%m-%dT%H:%M:%fZ','now'));
END;

CREATE TRIGGER IF NOT EXISTS note_rows_tombstone
AFTER DELETE ON note_rows
WHEN OLD.uid IS NOT NULL
BEGIN
    INSERT OR REPLACE INTO deleted_rows(table_name, uid, deleted_at)
    VALUES ('note_rows', OLD.uid, strftime('%Y-%m-%dT%H:%M:%fZ','now'));
END;

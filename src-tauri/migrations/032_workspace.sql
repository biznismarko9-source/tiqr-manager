-- 032: Workspace - the operational memory. Notes, records, tasks and tables.
--
-- WHY THIS EXISTS
--
-- marko's brief: "miesto na ktore sa mozem spolahnut ... aky kod a co som komu
-- predal, aky je jeho nick, ake info treba mat ulozene ... plany, ulozit si
-- ucty, prehlad, najst vsetky jednoducho", then a full written spec for a
-- Workspace section: notes, structured records, tasks with due dates, custom
-- tables, tags, pins, archive, categories, checklists and one search over all
-- of it.
--
-- ONE TABLE FOR NOTES, RECORDS AND TASKS
--
-- They are the same object wearing three hats, and the spec is explicit that a
-- quick note must be able to GROW into a task or a record without being
-- retyped ("Do not force the user to know the final structure when creating
-- information"). Three tables would make that a migration between rows; one
-- table with a `kind` makes it a single UPDATE.
--
-- Fields, tags and checklists are JSON for the same reason the note tables use
-- it (migration 031): they are lists the person types, always read as part of
-- their own item, never queried across items.
--
-- THE TABLES KEEP THEIR 031 NAMES
--
-- `note_sheets`/`note_rows` become Workspace's tables. They are NOT renamed:
-- they already carry uids, tombstones and MERGE_TABLES entries, and existing
-- tombstones name them by string. A rename would buy a tidier name and risk
-- real data. The columns added below are what the spec asks for on top.
--
-- SYNC
--
-- Same three pieces every synced table needs (see PROTECTED_AREAS, 2.51.0):
-- `uid` + insert trigger, delete tombstone, and an entry in MERGE_TABLES.
-- This adds no sync TRIGGERS of its own kind and changes no sync code - it
-- follows the existing per-table pattern, which is what "persist exactly like
-- other user data" requires.

CREATE TABLE IF NOT EXISTS workspace_items (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    uid            TEXT UNIQUE,
    -- note | record | task. One row can change kind in place.
    kind           TEXT NOT NULL DEFAULT 'note',
    title          TEXT NOT NULL DEFAULT '',
    content        TEXT NOT NULL DEFAULT '',
    category       TEXT,
    -- ["oasis","followup"] - stored without the leading '#'.
    tags_json      TEXT NOT NULL DEFAULT '[]',
    -- [{"name":"Nick","value":"John123"}] - a record's own fields.
    fields_json    TEXT NOT NULL DEFAULT '[]',
    -- [{"text":"Send remaining 2","done":false}]
    checklist_json TEXT NOT NULL DEFAULT '[]',
    -- open | done, and only meaningful on a task.
    status         TEXT,
    due_date       TEXT,
    pinned         INTEGER NOT NULL DEFAULT 0,
    archived       INTEGER NOT NULL DEFAULT 0,
    created_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE INDEX IF NOT EXISTS idx_workspace_items_live ON workspace_items(archived, updated_at);
CREATE INDEX IF NOT EXISTS idx_workspace_items_due ON workspace_items(due_date) WHERE due_date IS NOT NULL;

CREATE TRIGGER IF NOT EXISTS workspace_items_uid_after_insert
AFTER INSERT ON workspace_items
WHEN NEW.uid IS NULL
BEGIN
    UPDATE workspace_items SET uid = lower(hex(randomblob(16))) WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS workspace_items_tombstone
AFTER DELETE ON workspace_items
WHEN OLD.uid IS NOT NULL
BEGIN
    INSERT OR REPLACE INTO deleted_rows(table_name, uid, deleted_at)
    VALUES ('workspace_items', OLD.uid, strftime('%Y-%m-%dT%H:%M:%fZ','now'));
END;

-- The 031 tables gain what the spec asks of a Workspace table: something to
-- say what it is for, and the same pin/archive/tag handles every other item
-- has. `column_types_json` is a list aligned with `columns_json` - "text",
-- "number", "date", "datetime", "check", "status", "url" or "money" - and is
-- allowed to be shorter, in which case the missing ones are plain text.
ALTER TABLE note_sheets ADD COLUMN description TEXT NOT NULL DEFAULT '';
ALTER TABLE note_sheets ADD COLUMN tags_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE note_sheets ADD COLUMN pinned INTEGER NOT NULL DEFAULT 0;
ALTER TABLE note_sheets ADD COLUMN archived INTEGER NOT NULL DEFAULT 0;
ALTER TABLE note_sheets ADD COLUMN column_types_json TEXT NOT NULL DEFAULT '[]';

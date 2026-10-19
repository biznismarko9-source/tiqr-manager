-- 046: live sync between computers (2.112.0).
--
-- marko: "ako by sme vedeli urobit nejaky sync ktory v realnom case po
-- zatvoreni apky a otvoreni v inom zariadeni by tam boli insta informacie",
-- "ze vsetko by ako keby fungovalo spolu".
--
-- Until now the two computers met through ONE FILE on Google Drive, only
-- when the app opened or closed, and the merge of that file only ever ADDED
-- rows and carried deletions - an EDIT to a record both had never travelled
-- at all. Live sync sends every change, row by row, through the account's
-- own Firestore (commands/live_sync.rs, src/lib/liveSync.ts), and the other
-- computer applies it as it arrives.
--
-- These are the bookkeeping tables it needs. The per-table triggers that
-- fill the outbox are NOT here: they are created by `live_sync::
-- install_triggers` from cloud_merge's MERGE_TABLES, so the list of tables
-- that travel has exactly one definition and a table added there is caught
-- here too.

-- One row: whether live sync is on for this database, the clock it stamps
-- changes with, and where reading the other computer's changes got to.
CREATE TABLE IF NOT EXISTS sync_control (
    id            INTEGER PRIMARY KEY CHECK (id = 1),
    enabled       INTEGER NOT NULL DEFAULT 0,
    -- 1 only inside `live_sync::apply`, so a change arriving from the other
    -- computer is never sent straight back to it.
    applying      INTEGER NOT NULL DEFAULT 0,
    -- A hybrid logical clock, in milliseconds: never behind the wall clock,
    -- never behind any change already seen from the other computer. Newer
    -- edit wins compares these.
    hlc           INTEGER NOT NULL DEFAULT 0,
    -- Firestore's commit time of the last change read, as seconds + nanos.
    cursor_s      INTEGER,
    cursor_ns     INTEGER,
    -- Whether marko (or the first start) decided on/off yet.
    decided_at    TEXT,
    seeded_at     TEXT,
    last_push_at  TEXT,
    last_pull_at  TEXT,
    last_error    TEXT,
    last_error_at TEXT
);
INSERT OR IGNORE INTO sync_control (id) VALUES (1);

-- What this computer changed and has not sent yet. One row per record, so
-- ten edits to the same order in a minute are sent once, as they stand.
-- `row_id` is this computer's own id (a negative pseudo-id for a deletion
-- that only exists as a tombstone); `uid` is filled for deletions, whose row
-- is gone by the time it is sent.
CREATE TABLE IF NOT EXISTS sync_outbox (
    table_name TEXT NOT NULL,
    row_id     INTEGER NOT NULL,
    uid        TEXT,
    op         TEXT NOT NULL,
    hlc        INTEGER NOT NULL,
    queued_at  TEXT NOT NULL,
    PRIMARY KEY (table_name, row_id)
);
CREATE INDEX IF NOT EXISTS idx_sync_outbox_hlc ON sync_outbox (hlc);

-- The version (hlc) each record had when it was last sent or applied - what
-- an arriving change is compared with.
CREATE TABLE IF NOT EXISTS sync_row_meta (
    table_name TEXT NOT NULL,
    uid        TEXT NOT NULL,
    hlc        INTEGER NOT NULL,
    device     TEXT,
    PRIMARY KEY (table_name, uid)
);

-- A lookup the two computers each created on their own ("Ticketmaster" with
-- two different uids) is the same row by name. The other computer's uid is
-- remembered here so its records keep pointing at this one.
CREATE TABLE IF NOT EXISTS sync_uid_alias (
    table_name TEXT NOT NULL,
    remote_uid TEXT NOT NULL,
    local_id   INTEGER NOT NULL,
    PRIMARY KEY (table_name, remote_uid)
);

-- Changes that arrived before the record they point at (a ticket before its
-- order). Kept, and applied the moment the parent is here.
CREATE TABLE IF NOT EXISTS sync_pending (
    doc_id      TEXT PRIMARY KEY,
    body        TEXT NOT NULL,
    received_at TEXT NOT NULL,
    reason      TEXT
);

-- 036: The notepad - notes, their sub-tabs, and their images.
--
-- WHY THIS EXISTS
--
-- marko picked design 02 out of ten and said what he liked about it:
--
--   "presne aj to ze si vies zaskrtnut, ze si vies pridat take veci ktore vies
--    pouzit, aj to ze si vies vybrat kde chces mat fotku, kde chces pisat, kde
--    ten text ma byt vacsi, aka farba"
--
-- and then, while it was being built:
--
--   "v jednej karte si vies pridat podkarty niekde dole ako to je v google
--    sheets ze napr budes mat poznamku oasis kody a vies si povyberat podkarty
--    a do kazdej si zapises o danom kode"
--
-- DO NOT CONFUSE THESE WITH THE SPREADSHEET TABLES
--
--   note_sheets / note_columns / note_rows   = the SPREADSHEET (migration 031)
--   notes / note_pages / note_images         = the NOTEPAD      (this one)
--
-- They are different features with unfortunately similar names. The
-- spreadsheet keeps its names because existing tombstones name it by string
-- (see PROTECTED_AREAS, 2.52.0).
--
-- WHY A NOTE'S BODY IS A LIST OF BLOCKS
--
-- "kde chces mat fotku, kde chces pisat, kde ten text ma byt vacsi, aka farba"
-- is a per-piece decision, not a per-note one. So `note_pages.blocks_json` is
-- an ordered JSON array and each entry carries its own type and formatting:
--
--   {"k":"text",  "t":"…", "s":"h1|h2|p|small", "c":"r|o|g|b|p|m", "b":1, "i":1}
--   {"k":"check", "t":"…", "d":1, …same formatting…}
--   {"k":"image", "id":12, "t":"caption"}
--   {"k":"rule"}
--
-- A table of blocks would be the "proper" schema and would buy nothing: a
-- block is only ever read as part of its page, written a whole page at a time,
-- and never queried across notes. An unknown `k` is skipped by the UI rather
-- than rejected, so a newer version cannot make an older one unable to open a
-- note.
--
-- WHY THE SUB-TABS ARE A TABLE AND NOT MORE JSON
--
-- Because they are what he selects between, they are reordered, renamed and
-- deleted individually, and a note with thirty codes would otherwise rewrite
-- every page's blocks on every keystroke in one of them.
--
-- WHY IMAGES ARE TEXT AND NOT A BLOB
--
-- `data_uri` holds `data:image/jpeg;base64,…`. Sync pushes the whole database
-- file to Drive and `cloud_merge` copies rows column by column, so a TEXT
-- column travels exactly like every other column with nothing new to verify -
-- a BLOB would be the one type on that path that had never been exercised.
--
-- The UI downscales and re-encodes to JPEG before it ever gets here (longest
-- edge 1400px), because the alternative is a 6MB phone screenshot inflating
-- the file that gets uploaded on every sync. They live in their own table so
-- that listing notes never drags the image data along with it.
--
-- SYNC
--
-- All three tables get all THREE pieces (PROTECTED_AREAS, 2.51.0): the `uid`
-- column with the 027 insert trigger, the 028 delete tombstone, and an entry
-- in cloud_merge::MERGE_TABLES.
--
-- WHY LINKS ARE SIX REAL FOREIGN KEYS AND NOT (kind, id)
--
-- marko: "neni to len pre kody ale aj do buducna na zapisovacky, priradovanie
-- eventu, pullu, inventaru, sellu atd" + "aj finance". So a note attaches to
-- an order, an event, a ticket, a sale, a pull or a finance entry - and to
-- several at once if that is the truth of it.
--
-- The tempting shape is one `(link_kind TEXT, link_id INTEGER)` pair. It would
-- be WRONG here, and silently: `cloud_merge` translates the other machine's
-- ids to this one's by NAMED COLUMN -> parent table (MERGE_TABLES `fks`). A
-- generic pair has no named parent, so after a merge a note would point at
-- whatever local record happened to hold that number - a different order, or a
-- sale. Six nullable columns each get translated properly.
--
-- Exactly one is non-null per row; several rows give several links. All six
-- parents are already in MERGE_TABLES, so `note_links` is listed last, after
-- every table it points at.
--
-- ON DELETE CASCADE on the link columns, not SET NULL: deleting an order
-- removes the LINK and leaves the note, which is the thing he would still
-- want. A dangling empty link row would be worse than no row.

CREATE TABLE IF NOT EXISTS notes (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    uid        TEXT UNIQUE,
    title      TEXT NOT NULL DEFAULT '',
    tag        TEXT NOT NULL DEFAULT '',
    pinned     INTEGER NOT NULL DEFAULT 0,
    archived   INTEGER NOT NULL DEFAULT 0,
    -- The date he attaches himself, 'YYYY-MM-DD'. Not a timestamp: it is a
    -- thing he writes, not a thing that happens.
    note_date  TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TABLE IF NOT EXISTS note_pages (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    uid         TEXT UNIQUE,
    note_id     INTEGER NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    position    INTEGER NOT NULL DEFAULT 0,
    name        TEXT NOT NULL DEFAULT '',
    blocks_json TEXT NOT NULL DEFAULT '[]',
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TABLE IF NOT EXISTS note_images (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    uid        TEXT UNIQUE,
    note_id    INTEGER NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    data_uri   TEXT NOT NULL,
    caption    TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TABLE IF NOT EXISTS note_links (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    uid              TEXT UNIQUE,
    note_id          INTEGER NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    -- Exactly one of these is non-null. Six columns rather than a (kind, id)
    -- pair so cloud_merge can translate each id across machines - see above.
    order_id         INTEGER REFERENCES orders(id) ON DELETE CASCADE,
    event_id         INTEGER REFERENCES events(id) ON DELETE CASCADE,
    ticket_id        INTEGER REFERENCES tickets(id) ON DELETE CASCADE,
    sale_id          INTEGER REFERENCES sales(id) ON DELETE CASCADE,
    pull_id          INTEGER REFERENCES pulls(id) ON DELETE CASCADE,
    finance_entry_id INTEGER REFERENCES finance_entries(id) ON DELETE CASCADE,
    created_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE INDEX IF NOT EXISTS idx_notes_live ON notes(archived, pinned, updated_at);
CREATE INDEX IF NOT EXISTS idx_note_links_note ON note_links(note_id);
CREATE INDEX IF NOT EXISTS idx_note_pages_note ON note_pages(note_id, position);
CREATE INDEX IF NOT EXISTS idx_note_images_note ON note_images(note_id);

CREATE TRIGGER IF NOT EXISTS notes_uid_after_insert
AFTER INSERT ON notes WHEN NEW.uid IS NULL
BEGIN
    UPDATE notes SET uid = lower(hex(randomblob(16))) WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS note_pages_uid_after_insert
AFTER INSERT ON note_pages WHEN NEW.uid IS NULL
BEGIN
    UPDATE note_pages SET uid = lower(hex(randomblob(16))) WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS note_images_uid_after_insert
AFTER INSERT ON note_images WHEN NEW.uid IS NULL
BEGIN
    UPDATE note_images SET uid = lower(hex(randomblob(16))) WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS note_links_uid_after_insert
AFTER INSERT ON note_links WHEN NEW.uid IS NULL
BEGIN
    UPDATE note_links SET uid = lower(hex(randomblob(16))) WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS notes_tombstone
AFTER DELETE ON notes WHEN OLD.uid IS NOT NULL
BEGIN
    INSERT OR REPLACE INTO deleted_rows(table_name, uid, deleted_at)
    VALUES ('notes', OLD.uid, strftime('%Y-%m-%dT%H:%M:%fZ','now'));
END;

CREATE TRIGGER IF NOT EXISTS note_pages_tombstone
AFTER DELETE ON note_pages WHEN OLD.uid IS NOT NULL
BEGIN
    INSERT OR REPLACE INTO deleted_rows(table_name, uid, deleted_at)
    VALUES ('note_pages', OLD.uid, strftime('%Y-%m-%dT%H:%M:%fZ','now'));
END;

CREATE TRIGGER IF NOT EXISTS note_images_tombstone
AFTER DELETE ON note_images WHEN OLD.uid IS NOT NULL
BEGIN
    INSERT OR REPLACE INTO deleted_rows(table_name, uid, deleted_at)
    VALUES ('note_images', OLD.uid, strftime('%Y-%m-%dT%H:%M:%fZ','now'));
END;

CREATE TRIGGER IF NOT EXISTS note_links_tombstone
AFTER DELETE ON note_links WHEN OLD.uid IS NOT NULL
BEGIN
    INSERT OR REPLACE INTO deleted_rows(table_name, uid, deleted_at)
    VALUES ('note_links', OLD.uid, strftime('%Y-%m-%dT%H:%M:%fZ','now'));
END;

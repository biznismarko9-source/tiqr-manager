-- 047: a picture for every event (2.116.0).
--
-- Two tables, and the difference between them is the whole design:
--
-- event_artworks   what was DECIDED for a subject by hand - a picture
--                  chosen, uploaded or confirmed, "no picture", or "back to
--                  automatic". One row per subject (`identity_key`, e.g.
--                  `name:coldplay`, or `event:<uid>` for a picture meant for
--                  one event only). It travels between the two computers
--                  (cloud_merge::MERGE_TABLES, so live sync and the Drive
--                  merge both carry it): a choice made on one computer shows
--                  on the other.
--
--                  The uid is not random: commands/event_artwork.rs derives
--                  it from `identity_key`, so both computers name the same
--                  subject's row the same way without talking to each other,
--                  and the newer choice wins like any other record. Rows are
--                  never deleted - "back to automatic" is a state, not a
--                  DELETE - because a deleted uid can never come back
--                  (migration 028: a deletion wins over any later edit).
--
-- artwork_lookups  what the picture services ANSWERED for one search
--                  (`query_key`, e.g. `artist:coldplay`), and the automatic
--                  pick when that answer was certain. It stays on this
--                  computer: an automatic pick is never sent anywhere, so it
--                  can never arrive on the other computer and replace a
--                  choice made there by hand. db.rs counts it as bookkeeping,
--                  so a search never marks the database as changed for the
--                  Drive backup either.
--
-- Additive only: two new tables, nothing existing is altered, and every
-- statement is IF NOT EXISTS, so a start interrupted half way through this
-- file simply runs it again.

CREATE TABLE IF NOT EXISTS event_artworks (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  uid           TEXT UNIQUE,
  identity_key  TEXT NOT NULL UNIQUE,
  -- The subject as a person reads it ("Coldplay"), for the picker.
  label         TEXT NOT NULL DEFAULT '',
  -- The search this subject belongs to ("artist:coldplay"), so the same
  -- artist under another event name is offered this picture first.
  query_key     TEXT NOT NULL DEFAULT '',
  state         TEXT NOT NULL DEFAULT 'set' CHECK (state IN ('set', 'none', 'auto')),
  -- data:image/...;base64,... - a small square-ish thumbnail, never a link:
  -- a picture that is stored cannot disappear from a server later.
  image         TEXT,
  fit           TEXT NOT NULL DEFAULT 'cover' CHECK (fit IN ('cover', 'contain')),
  -- Where it came from: deezer | wikipedia | upload | earlier (a picture
  -- already chosen for the same subject).
  source        TEXT NOT NULL DEFAULT '',
  source_id     TEXT NOT NULL DEFAULT '',
  source_title  TEXT NOT NULL DEFAULT '',
  source_page   TEXT NOT NULL DEFAULT '',
  image_url     TEXT NOT NULL DEFAULT '',
  confidence    REAL,
  created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

-- Only a safety net - the app always writes the derived uid itself.
CREATE TRIGGER IF NOT EXISTS event_artworks_uid_after_insert
AFTER INSERT ON event_artworks WHEN NEW.uid IS NULL
BEGIN
    UPDATE event_artworks SET uid = lower(hex(randomblob(16))) WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS event_artworks_tombstone
AFTER DELETE ON event_artworks WHEN OLD.uid IS NOT NULL
BEGIN
    INSERT OR REPLACE INTO deleted_rows(table_name, uid, deleted_at)
    VALUES ('event_artworks', OLD.uid, strftime('%Y-%m-%dT%H:%M:%fZ','now'));
END;

CREATE TABLE IF NOT EXISTS artwork_lookups (
  query_key   TEXT PRIMARY KEY,
  -- match: certain, `image` holds the automatic pick
  -- ask:   a likely answer that needs a yes before it is shown
  -- none:  nothing that could be trusted
  -- failed: the services could not be asked (offline, busy, an error)
  outcome     TEXT NOT NULL CHECK (outcome IN ('match', 'ask', 'none', 'failed')),
  level       TEXT NOT NULL DEFAULT '',
  -- JSON: what was found, with its source and how sure - no pictures.
  candidates  TEXT NOT NULL DEFAULT '[]',
  pick        TEXT,
  image       TEXT,
  fit         TEXT NOT NULL DEFAULT 'cover',
  error       TEXT,
  attempts    INTEGER NOT NULL DEFAULT 0,
  checked_at  TEXT NOT NULL,
  -- No new search before this (RFC 3339); NULL = whenever asked.
  retry_after TEXT
);

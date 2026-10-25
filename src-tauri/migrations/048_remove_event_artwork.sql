-- 048: the event pictures removed (2.116.1).
--
-- marko asked for the pictures to go completely, so what 047 stored goes
-- with them: every answer a picture service gave and every picture chosen by
-- hand. Nothing else is touched - events, orders, tickets and sales keep
-- every value they had.
--
-- artwork_lookups is dropped. It never left this computer.
--
-- event_artworks is emptied, not dropped. A computer still on 2.116.0 merges
-- a Drive copy table by table and stops on a table the copy does not have,
-- so the table stays here, empty, until both computers are past 2.116.0.
-- This version neither sends nor applies it (cloud_merge::MERGE_TABLES no
-- longer names it; live_sync::RETIRED_TABLES sets aside what a 2.116.0
-- computer still sends).
--
-- Its triggers go first: the delete below then leaves no tombstones and
-- queues nothing for live sync, so emptying this computer's table does not
-- reach into the other computer's. That one empties its own table when it
-- runs this same file.
--
-- Runs inside one transaction (db.rs apply_migrations), so a start stopped
-- half way leaves the pictures as they were and runs this again next time.

DROP TRIGGER IF EXISTS event_artworks_live_sync_ins;
DROP TRIGGER IF EXISTS event_artworks_live_sync_upd;
DROP TRIGGER IF EXISTS event_artworks_live_sync_del;
DROP TRIGGER IF EXISTS event_artworks_tombstone;
DROP TRIGGER IF EXISTS event_artworks_uid_after_insert;

DELETE FROM event_artworks;
DROP TABLE IF EXISTS artwork_lookups;

-- What live sync and the Drive merge still held about those rows.
DELETE FROM sync_outbox WHERE table_name = 'event_artworks';
DELETE FROM sync_row_meta WHERE table_name = 'event_artworks';
DELETE FROM sync_uid_alias WHERE table_name = 'event_artworks';
DELETE FROM sync_pending WHERE json_extract(body, '$.t') = 'event_artworks';
DELETE FROM deleted_rows WHERE table_name = 'event_artworks';

-- The on/off switch the pictures had in Settings.
DELETE FROM app_settings WHERE key = 'event_artwork_auto';

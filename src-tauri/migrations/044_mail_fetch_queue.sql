-- 044: the fetch queue - what is still waiting to be downloaded.
--
-- marko: "ano toto chcem aby sa vzdy po kazdom urobenom zobrazili".
--
-- WHY A QUEUE AND NOT JUST A SMALLER BATCH
--
-- The obvious way to make mail appear sooner is to shrink the batch: fetch
-- 40 instead of 300 and refresh the list more often. That does not work on
-- its own, because every run has to ask Gmail WHICH messages exist first -
-- `messages.list`, up to twenty pages of it - and that listing costs the
-- same whether the run then fetches 40 messages or 300. Shrinking the batch
-- would mean paying for the listing eight times as often, and the listing
-- is quota too.
--
-- So the listing happens ONCE and its result is parked here. After that,
-- every run is pure fetching: take a handful off the queue, download them,
-- store them, delete them from the queue. A batch becomes about a second of
-- work instead of six, and the screen fills in continuously.
--
-- A SECOND THING THIS BUYS, which matters more than it looks
--
-- The queue is on disk, so a long first sync SURVIVES. Close the app halfway
-- through a mailbox and the next run carries on from the same place instead
-- of starting the listing again. Before this, quitting mid-sync threw away
-- everything that had not been fetched yet.
--
-- The id is the PRIMARY KEY, so enqueuing the same message twice is a no-op
-- rather than a duplicate - the same reason `mail_messages` keys on it.
CREATE TABLE IF NOT EXISTS mail_fetch_queue (
    provider_message_id TEXT PRIMARY KEY,
    account_email       TEXT NOT NULL,
    queued_at           TEXT NOT NULL,
    -- 0 = starred, 1 = everything else. The queue drains by priority, so
    -- marko's starred mail arrives FIRST rather than behind a month of
    -- newsletters. "je to jedno ake stare su tie maily ktore mam oznacene
    -- tie musia byt vzdy" - always there, and first.
    --
    -- `MIN(priority)` on conflict, so a message that is both starred and
    -- inside the date window keeps the better one.
    priority            INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_mail_queue_order ON mail_fetch_queue (priority, queued_at);

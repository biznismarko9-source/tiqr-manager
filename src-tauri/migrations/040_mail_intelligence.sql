-- 040: Mail Intelligence - the mailbox TIQR watches, and what it made of it.
--
-- WHY THIS EXISTS
--
-- marko asked for TIQR to read his inbox and recognise the mail that matters:
-- sale confirmations, purchase confirmations, payouts, refunds, cancellations
-- and event updates - so that the work he does by hand today (open the mail,
-- find the event, type the sale) can be proposed for him.
--
-- This migration is PHASE 1 of that: it stores what was seen and what was
-- made of it. It deliberately stores NO link to a sale, order, ticket or
-- finance entry yet, because nothing in phase 1 writes one. Those columns
-- arrive with the phase that actually uses them, so no column here is ever a
-- promise the code does not keep.
--
-- WHAT IS DELIBERATELY NOT HERE
--
-- 1. No table for the mailbox CREDENTIAL. The Gmail refresh token lives in
--    `app_settings` under `mail_gmail_refresh_token`, exactly where and how
--    the Sheets sign-in already keeps `google_oauth_refresh_token`
--    (commands/google_auth.rs). One secret store, one convention, one place
--    to look. A second mechanism for the same kind of secret would be a
--    second thing to get wrong.
--
-- 2. No full message bodies. See `body_excerpt` below.
--
-- WHY THESE TABLES ARE NOT IN `MERGE_TABLES`
--
-- `commands/cloud_merge.rs::MERGE_TABLES` lists every table that travels
-- between marko's two machines. These three are deliberately NOT added to it,
-- and that is a decision, not an oversight:
--
--   * A mailbox connection belongs to ONE machine. The refresh token is
--     granted to that machine's sign-in; copying the sync checkpoint to the
--     other machine would make it skip mail it has never actually fetched.
--   * `last_history_id` is a Gmail-side cursor. Two machines sharing one
--     cursor is how mail silently stops arriving on one of them.
--   * The records this feature will eventually CREATE - sales, finance
--     entries - are already in `MERGE_TABLES` and travel normally. It is the
--     mailbox bookkeeping that stays local, not the business result.
--
-- Consequence worth stating plainly: each machine syncs the same mailbox
-- independently and will hold its own `mail_messages` rows for the same
-- Gmail messages. That is correct for phase 1 (nothing is written from them),
-- and phase 3 must deduplicate on the BUSINESS record, not on this table -
-- which is the same rule the spec asks for anyway.

-- ---------------------------------------------------------------------------
-- One row per message this app has ever seen.
-- ---------------------------------------------------------------------------
--
-- IDEMPOTENT INGESTION is the `UNIQUE` at the bottom, not application code.
-- Gmail's message id is immutable and stable across folders, labels and
-- re-reads, so "have I already stored this?" is a constraint the database
-- answers, and an `INSERT ... ON CONFLICT DO UPDATE` can be run as many times
-- as a retry needs without ever producing a second row. The same message
-- arriving again after a reconnect, a full resync, or a crash mid-batch lands
-- on the row that is already there.
--
-- `account_email` is part of the key on purpose: connecting a DIFFERENT
-- mailbox must not collide with the first one's ids.
CREATE TABLE IF NOT EXISTS mail_messages (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,

    -- 'gmail' today. IMAP is a second value here, not a second table - the
    -- provider abstraction is in Rust, and this column is what it writes.
    provider            TEXT NOT NULL,
    provider_message_id TEXT NOT NULL,
    provider_thread_id  TEXT,
    account_email       TEXT NOT NULL,

    -- Headers. `from_address` is the parsed address, `from_name` the display
    -- name when the header carried one. Both are stored as received: a
    -- display name is attacker-controlled text and is only ever RENDERED as
    -- text, never trusted as identity (see from_address for matching).
    from_address        TEXT NOT NULL,
    from_name           TEXT,
    subject             TEXT NOT NULL DEFAULT '',

    -- Gmail's own short preview. Cheap, already truncated by Google, and
    -- enough to recognise a message in a list without storing its body.
    snippet             TEXT,

    -- RFC3339 UTC, from the message's own internal date. Stored UTC because
    -- two machines in different places must sort the same list identically;
    -- the UI formats to local, same as every other timestamp in this app.
    received_at         TEXT NOT NULL,

    -- WHAT THE CLASSIFIER MADE OF IT.
    --
    -- `category` is one of the values commands/mail.rs::MailCategory accepts;
    -- an unrecognised message is 'other', never a guess. `confidence` is
    -- 'high' | 'medium' | 'low' and is what sends a message to the review
    -- queue rather than straight through - phase 1 never processes anything
    -- automatically, but the column is what phase 4 will gate on.
    category            TEXT NOT NULL DEFAULT 'other',
    confidence          TEXT NOT NULL DEFAULT 'low',

    -- The marketplace the mail appears to come from, when a deterministic
    -- rule recognised it. NULL means "not recognised", never a guess.
    detected_platform   TEXT,

    -- LIFECYCLE. See commands/mail.rs::MailStatus for the full set and the
    -- transitions. Phase 1 reaches: 'classified' and 'review_required'.
    status              TEXT NOT NULL DEFAULT 'detected',

    -- Why a human needs to look at this one. Plain English, written by this
    -- app, never provider text.
    review_reason       TEXT,

    -- A BOUNDED, PLAIN-TEXT excerpt - not the message.
    --
    -- The spec asks for minimal retention of original bodies, and this is
    -- that. Reasons it is an excerpt and not the full body:
    --   * The body is the sensitive part. A mailbox's worth of full bodies in
    --     a SQLite file that gets backed up to Drive is a liability that
    --     buys nothing phase 1 needs.
    --   * It is stored as TEXT that has already had HTML stripped in Rust, so
    --     nothing downstream is ever handed markup to render. The UI renders
    --     this as text and only as text.
    --   * It is capped (commands/mail.rs::BODY_EXCERPT_LIMIT). A cap in the
    --     writer is the only cap that holds; a column type is not one.
    -- `mail_retention_days` in app_settings clears these on a schedule.
    body_excerpt        TEXT,

    first_seen_at       TEXT NOT NULL,
    updated_at          TEXT NOT NULL,

    UNIQUE (provider, account_email, provider_message_id)
);

-- The list screen sorts newest-first and filters by status and category.
CREATE INDEX IF NOT EXISTS idx_mail_messages_received  ON mail_messages (received_at DESC);
CREATE INDEX IF NOT EXISTS idx_mail_messages_status    ON mail_messages (status);
CREATE INDEX IF NOT EXISTS idx_mail_messages_category  ON mail_messages (category);

-- ---------------------------------------------------------------------------
-- The audit trail: every step, every attempt, success or not.
-- ---------------------------------------------------------------------------
--
-- Append-only by convention - nothing in this app updates a row here. "What
-- did it do to this message, and why" has to survive the message being
-- re-processed, so each attempt adds a row rather than replacing one.
--
-- `detail` is for the app's OWN explanation ("sender matched a Viagogo
-- rule", "two events share this name"). It must never carry a credential, a
-- token, or a raw provider error body - commands/mail.rs::safe_detail is
-- what enforces that, and it is tested.
CREATE TABLE IF NOT EXISTS mail_processing_log (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    message_id  INTEGER NOT NULL REFERENCES mail_messages (id) ON DELETE CASCADE,
    at          TEXT NOT NULL,
    step        TEXT NOT NULL,
    outcome     TEXT NOT NULL,
    detail      TEXT
);

CREATE INDEX IF NOT EXISTS idx_mail_log_message ON mail_processing_log (message_id, at);

-- ---------------------------------------------------------------------------
-- The connection, and where the last sync got to. Exactly one row.
-- ---------------------------------------------------------------------------
--
-- `CHECK (id = 1)` is what makes "exactly one" a database fact rather than a
-- habit. There is one mailbox per installation in phase 1; when a second one
-- is wanted, this becomes a real table with a real key and the check comes
-- off in its own migration - which is a smaller change than un-picking code
-- that assumed a single row without saying so.
--
-- WHY THE CURSOR IS A TEXT COLUMN
--
-- Gmail's `historyId` is documented as an unsigned 64-bit value and Google
-- returns it as a JSON string. Storing it as TEXT keeps it exactly as Google
-- gave it, with no integer round-trip that could silently lose precision on
-- a value near the top of the range. It is only ever compared for equality
-- and handed back to Google verbatim, so it never needs to be a number.
CREATE TABLE IF NOT EXISTS mail_sync_state (
    id                INTEGER PRIMARY KEY CHECK (id = 1),
    provider          TEXT,
    account_email     TEXT,

    -- Gmail's incremental cursor. NULL means "no incremental point yet", so
    -- the next sync is a bounded backfill by date rather than a full history
    -- replay (see commands/mail.rs::sync_gmail).
    last_history_id   TEXT,

    last_sync_at      TEXT,
    last_sync_count   INTEGER NOT NULL DEFAULT 0,

    -- The last failure, kept so Settings can show "what went wrong" without
    -- the user having to catch a toast. Cleared on the next success, so a
    -- stale error never sits there looking current.
    last_error        TEXT,
    last_error_at     TEXT,

    -- marko's own switch. 1 = do not sync, even if a timer fires. This is the
    -- global pause the spec asks for and it is checked in the backend, not
    -- just hidden in the UI.
    paused            INTEGER NOT NULL DEFAULT 0,

    -- Guards against two syncs overlapping (the timer firing while a manual
    -- sync is still running). Set at the start of a run and cleared at the
    -- end, including on failure.
    sync_in_progress  INTEGER NOT NULL DEFAULT 0,
    sync_started_at   TEXT
);

-- The single row exists from the start, so every read is a plain SELECT with
-- no "does it exist yet" branch, and every write is an UPDATE.
INSERT OR IGNORE INTO mail_sync_state (id) VALUES (1);

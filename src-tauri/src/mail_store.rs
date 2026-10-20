//! 2.110.0 - the mail cache lives in its own file.
//!
//! marko: "urobit nejaky lock aby tie maily nemohli len tak zmiznut, potom to
//! urobit nejako jednoducho a ulozit na nejake miesto kde je to safe a kde to
//! zerie najmenej info" - and "nechcem 200 message hold limit ale minimalne
//! 100 000".
//!
//! ## Why a second file
//!
//! Everything cloud sync does, it does to the MAIN file whole: a push uploads
//! it, a pull replaces it, and every merge first copies it as a safety backup
//! (`backup::create_safety_backup`, which nothing ever prunes). A hundred
//! thousand messages inside it would make each of those about 100 MB, and a
//! pull would quietly swap his mailbox for the other machine's.
//!
//! So the mail lives in `<main>.mail.sqlite3` next to the main file, ATTACHED
//! to the live connection as the schema `mail`. The Online Backup API copies
//! only `main`, so pushes, merges, safety backups and restores never see this
//! file and nothing they do can reach it. It is per account for free: the
//! path is derived from the account's own main file.
//!
//! Every existing query keeps working unqualified. SQLite looks a table name
//! up in `main` first and then in attached databases, and after `ensure` the
//! cache tables no longer exist in `main`.
//!
//! ## The lock
//!
//! A trigger refuses every DELETE on `mail_messages` unless `mail_lock` says
//! unlocked, and the only thing that unlocks it is `delete_forever` - marko's
//! own "Delete forever" in Trash, inside one transaction that locks again
//! before it commits. So no sync, migration, merge or bug can remove a stored
//! message. The database refuses, whoever asks.
//!
//! ## Least space
//!
//! Headers and Gmail's short preview only - a body is fetched when a message
//! is opened and never stored (commands/mail.rs::get_mail_body). The file is
//! created with incremental auto-vacuum so space freed by the retention purge
//! goes back to the disk instead of sitting in the file.
//!
//! ## Rule for future migrations
//!
//! The numbered migrations in `migrations/` run on EVERY file, including a
//! downloaded copy from the other machine that has no `mail` attached. They
//! must never touch a cache table again. A change to this schema is a new
//! `user_version` step in `ensure`, not a numbered migration - `SCHEMA_V1`
//! (2.110.0), `SCHEMA_V2` (2.111.0) and `SCHEMA_V3` (2.113.0) so far, and a
//! released step is never edited, only followed by the next one.
use rusqlite::{Connection, OptionalExtension};
use std::path::{Path, PathBuf};

/// `tiqr-manager.sqlite3` -> `tiqr-manager.mail.sqlite3`, in the same folder.
pub fn mail_db_path(main: &Path) -> PathBuf {
    let stem = main
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "tiqr-manager".to_string());
    main.with_file_name(format!("{stem}.mail.sqlite3"))
}

/// Attaches the mail file to a live connection. Must run outside any
/// transaction (SQLite refuses ATTACH inside one).
pub fn attach(conn: &Connection, path: &Path) -> rusqlite::Result<()> {
    conn.execute("ATTACH DATABASE ?1 AS mail", [path.to_string_lossy().as_ref()])?;
    prepare_attached(conn)
}

/// Unit tests get the same layout as the app, in memory.
#[cfg(test)]
pub fn attach_in_memory(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch("ATTACH DATABASE ':memory:' AS mail;")?;
    prepare_attached(conn)
}

fn prepare_attached(conn: &Connection) -> rusqlite::Result<()> {
    // auto_vacuum only takes effect before the first table exists, so it is
    // set on a brand-new file and left alone on every later open.
    let tables: i64 = conn.query_row("SELECT COUNT(*) FROM mail.sqlite_master", [], |r| r.get(0))?;
    if tables == 0 {
        conn.execute_batch("PRAGMA mail.auto_vacuum = INCREMENTAL;")?;
    }
    let _mode: String = conn.query_row("PRAGMA mail.journal_mode = WAL", [], |r| r.get(0))?;
    Ok(())
}

pub fn is_attached(conn: &Connection) -> bool {
    let Ok(mut stmt) = conn.prepare("PRAGMA database_list") else {
        return false;
    };
    let Ok(rows) = stmt.query_map([], |r| r.get::<_, String>(1)) else {
        return false;
    };
    let found = rows.flatten().any(|name| name == "mail");
    found
}

/// Brings the mail file to the current schema and moves any cache table that
/// is still in `main` into it. Idempotent, and a no-op on a connection with no
/// `mail` attached - which is what keeps a downloaded or restored copy of the
/// main file from growing cache tables of its own.
///
/// Called at the end of every `db::run_migrations`, so a fresh install, an
/// upgrade and a restore of a pre-2.110 backup all pass through it.
pub fn ensure(conn: &Connection) -> rusqlite::Result<()> {
    if !is_attached(conn) {
        return Ok(());
    }
    let version: i64 = conn.query_row("PRAGMA mail.user_version", [], |r| r.get(0))?;
    if version < 1 {
        conn.execute_batch(SCHEMA_V1)?;
        conn.execute_batch("PRAGMA mail.user_version = 1;")?;
    }
    if version < 2 {
        // One transaction with the version bump, so a crash half way leaves
        // version 1 behind and the whole step runs again on the next start.
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(SCHEMA_V2)?;
        tx.execute_batch("PRAGMA mail.user_version = 2;")?;
        tx.commit()?;
    }
    if version < 3 {
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(SCHEMA_V3)?;
        tx.execute_batch("PRAGMA mail.user_version = 3;")?;
        tx.commit()?;
    }
    relocate(conn)?;
    Ok(())
}

/// The cache tables, children before parents - the order they are dropped in.
const CACHE_TABLES: &[&str] = &[
    "mail_processing_log",
    "mail_fetch_queue",
    "mail_labels",
    "mail_sync_state",
    "mail_messages",
];

fn main_has(conn: &Connection, table: &str) -> rusqlite::Result<bool> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM main.sqlite_master WHERE type = 'table' AND name = ?1)",
        [table],
        |r| r.get(0),
    )
}

fn columns(conn: &Connection, schema: &str, table: &str) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare(&format!("PRAGMA {schema}.table_info({table})"))?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(1))?;
    rows.collect()
}

/// Copies `main.<table>` into `mail.<table>` over the columns both have, ids
/// included, so log rows still point at the right message.
fn copy_table(conn: &Connection, table: &str, verb: &str) -> rusqlite::Result<()> {
    let theirs = columns(conn, "main", table)?;
    let ours = columns(conn, "mail", table)?;
    let shared: Vec<&String> = theirs.iter().filter(|c| ours.contains(c)).collect();
    if shared.is_empty() {
        return Ok(());
    }
    let list = shared.iter().map(|c| format!("\"{c}\"")).collect::<Vec<_>>().join(", ");
    conn.execute_batch(&format!(
        "{verb} INTO mail.{table} ({list}) SELECT {list} FROM main.{table};"
    ))
}

/// Moves the mail out of the main file, once.
///
/// When the mail file is still EMPTY this is the first start of 2.110.0 and
/// everything is copied across, ids and all. When it already holds mail, the
/// tables in `main` came from a restored old backup, and the cache here is
/// newer than they are - so they are dropped, not merged. Copying them would
/// put old rows under ids that now belong to different messages, and their
/// log rows would end up attached to the wrong mail.
///
/// The queue is never copied: the 2.110.0 engine walks every folder again from
/// the top, and already-stored messages cost it nothing.
fn relocate(conn: &Connection) -> rusqlite::Result<()> {
    let mut any = false;
    for t in CACHE_TABLES {
        any |= main_has(conn, t)?;
    }
    if !any {
        return Ok(());
    }
    let tx = conn.unchecked_transaction()?;
    let mail_empty: bool =
        tx.query_row("SELECT NOT EXISTS(SELECT 1 FROM mail.mail_messages)", [], |r| r.get(0))?;
    if mail_empty {
        if main_has(&tx, "mail_messages")? {
            copy_table(&tx, "mail_messages", "INSERT OR IGNORE")?;
        }
        if main_has(&tx, "mail_processing_log")? {
            copy_table(&tx, "mail_processing_log", "INSERT OR IGNORE")?;
        }
        if main_has(&tx, "mail_labels")? {
            copy_table(&tx, "mail_labels", "INSERT OR REPLACE")?;
        }
        if main_has(&tx, "mail_sync_state")? {
            copy_table(&tx, "mail_sync_state", "INSERT OR REPLACE")?;
            // The old engine's run claim and cursor mean nothing to the new
            // one. Pause, last error and last sync time are kept.
            tx.execute_batch(
                "UPDATE mail.mail_sync_state
                    SET sync_in_progress = 0, sync_started_at = NULL, last_history_id = NULL
                  WHERE id = 1;",
            )?;
        }
    }
    for t in CACHE_TABLES {
        if main_has(&tx, t)? {
            tx.execute_batch(&format!("DROP TABLE main.{t};"))?;
        }
    }
    tx.commit()?;
    // The pages the mail used are free now but still INSIDE the main file,
    // and a push uploads every page of it. Compacted once, here, so the
    // first upload after the move is the small file it should be. Best
    // effort: a file that cannot be compacted right now is still correct.
    let _ = conn.execute_batch("VACUUM main;");
    Ok(())
}

/// Runs `f` with the delete lock open, inside one transaction that closes it
/// again before committing. If `f` fails, the rollback restores the lock too.
pub fn with_delete_unlocked<T>(
    conn: &Connection,
    f: impl FnOnce(&Connection) -> rusqlite::Result<T>,
) -> rusqlite::Result<T> {
    let tx = conn.unchecked_transaction()?;
    tx.execute("UPDATE mail.mail_lock SET unlocked = 1 WHERE id = 1", [])?;
    let out = f(&tx)?;
    tx.execute("UPDATE mail.mail_lock SET unlocked = 0 WHERE id = 1", [])?;
    tx.commit()?;
    Ok(out)
}

/// Bytes the mail file takes on disk, for the settings pane.
pub fn size_bytes(conn: &Connection) -> i64 {
    let pages: Option<i64> = conn
        .query_row("PRAGMA mail.page_count", [], |r| r.get(0))
        .optional()
        .ok()
        .flatten();
    let size: Option<i64> = conn
        .query_row("PRAGMA mail.page_size", [], |r| r.get(0))
        .optional()
        .ok()
        .flatten();
    pages.unwrap_or(0) * size.unwrap_or(0)
}

/// Gives back space the retention purge freed. Cheap when there is nothing
/// to reclaim; a no-op on a file created before auto-vacuum was set.
pub fn reclaim(conn: &Connection) {
    let _ = conn.execute_batch("PRAGMA mail.incremental_vacuum(2000);");
}

/// The whole cache schema as of 2.110.0. Columns are exactly the ones
/// migrations 040-044 built in `main`, plus `finance` and `labels_at`; the
/// tables below the queue are new.
const SCHEMA_V1: &str = r#"
CREATE TABLE IF NOT EXISTS mail.mail_messages (
    id                     INTEGER PRIMARY KEY AUTOINCREMENT,
    provider               TEXT NOT NULL,
    provider_message_id    TEXT NOT NULL,
    provider_thread_id     TEXT,
    account_email          TEXT NOT NULL,
    from_address           TEXT NOT NULL,
    from_name              TEXT,
    subject                TEXT NOT NULL DEFAULT '',
    snippet                TEXT,
    received_at            TEXT NOT NULL,
    category               TEXT NOT NULL DEFAULT 'other',
    confidence             TEXT NOT NULL DEFAULT 'low',
    detected_platform      TEXT,
    status                 TEXT NOT NULL DEFAULT 'detected',
    review_reason          TEXT,
    body_excerpt           TEXT,
    first_seen_at          TEXT NOT NULL,
    updated_at             TEXT NOT NULL,
    is_unread              INTEGER NOT NULL DEFAULT 1,
    labels                 TEXT,
    is_starred             INTEGER NOT NULL DEFAULT 0,
    to_address             TEXT,
    reply_to               TEXT,
    from_raw               TEXT,
    otp_code               TEXT,
    extracted_amount_cents INTEGER,
    extracted_currency     TEXT,
    extracted_quantity     INTEGER,
    extracted_reference    TEXT,
    trashed_at             TEXT,
    extracted_unit_cents   INTEGER,
    extracted_basis        TEXT,
    created_kind           TEXT,
    created_id             INTEGER,
    created_at             TEXT,
    created_by             TEXT,
    -- 2.110.0: money mail (payouts, refunds, PayPal/Stripe/Revolut/Wise,
    -- invoices and payments), decided once in Rust when the message is
    -- stored. NULL only on rows that came from before 2.110.0, and the
    -- engine fills those in.
    finance                INTEGER,
    -- 2.110.0: when `labels` was last set from what Gmail itself said. A
    -- folder walk only clears a label it did not see on rows OLDER than the
    -- walk, so a star added mid-walk is never undone by it.
    labels_at              TEXT,
    UNIQUE (provider, account_email, provider_message_id)
);
CREATE INDEX IF NOT EXISTS mail.idx_mail_messages_received ON mail_messages (received_at DESC, id DESC);
CREATE INDEX IF NOT EXISTS mail.idx_mail_messages_pid      ON mail_messages (provider_message_id);
CREATE INDEX IF NOT EXISTS mail.idx_mail_messages_status   ON mail_messages (status);
CREATE INDEX IF NOT EXISTS mail.idx_mail_messages_category ON mail_messages (category);
CREATE INDEX IF NOT EXISTS mail.idx_mail_messages_unread   ON mail_messages (is_unread, received_at DESC);
CREATE INDEX IF NOT EXISTS mail.idx_mail_messages_starred  ON mail_messages (is_starred, received_at DESC);
CREATE INDEX IF NOT EXISTS mail.idx_mail_messages_otp      ON mail_messages (otp_code) WHERE otp_code IS NOT NULL;
CREATE INDEX IF NOT EXISTS mail.idx_mail_messages_trashed  ON mail_messages (trashed_at);
CREATE INDEX IF NOT EXISTS mail.idx_mail_messages_created  ON mail_messages (created_kind, created_id);
-- The few rows still waiting for their Finances flag, found without reading
-- the other 100 000 on every run.
CREATE INDEX IF NOT EXISTS mail.idx_mail_messages_finance_todo ON mail_messages (id) WHERE finance IS NULL;

CREATE TABLE IF NOT EXISTS mail.mail_processing_log (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    message_id  INTEGER NOT NULL REFERENCES mail_messages (id) ON DELETE CASCADE,
    at          TEXT NOT NULL,
    step        TEXT NOT NULL,
    outcome     TEXT NOT NULL,
    detail      TEXT
);
CREATE INDEX IF NOT EXISTS mail.idx_mail_log_message ON mail_processing_log (message_id, at);

CREATE TABLE IF NOT EXISTS mail.mail_sync_state (
    id                INTEGER PRIMARY KEY CHECK (id = 1),
    provider          TEXT,
    account_email     TEXT,
    last_history_id   TEXT,
    last_sync_at      TEXT,
    last_sync_count   INTEGER NOT NULL DEFAULT 0,
    last_error        TEXT,
    last_error_at     TEXT,
    paused            INTEGER NOT NULL DEFAULT 0,
    sync_in_progress  INTEGER NOT NULL DEFAULT 0,
    sync_started_at   TEXT,
    -- 2.110.0: when Gmail's change history was last read.
    history_checked_at TEXT,
    -- 2.110.0: when the labels list was last refreshed.
    labels_checked_at  TEXT
);
INSERT OR IGNORE INTO mail.mail_sync_state (id) VALUES (1);

CREATE TABLE IF NOT EXISTS mail.mail_labels (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    kind       TEXT NOT NULL DEFAULT 'user',
    updated_at TEXT NOT NULL,
    -- 2.110.0: the id exactly as Gmail spells it (`Label_8`). `id` is
    -- lower-cased to match `mail_messages.labels`; Gmail's own API wants
    -- this one when a label's mail is listed.
    gmail_id   TEXT
);

-- What is still waiting to be downloaded. `priority` is the folder's place
-- in marko's order (lower first) and `seq` is the order it was listed in, so
-- inside one folder the newest message comes first.
CREATE TABLE IF NOT EXISTS mail.mail_fetch_queue (
    provider_message_id TEXT PRIMARY KEY,
    account_email       TEXT NOT NULL,
    queued_at           TEXT NOT NULL,
    priority            INTEGER NOT NULL DEFAULT 50,
    seq                 INTEGER NOT NULL DEFAULT 0,
    attempts            INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS mail.idx_mail_queue_order ON mail_fetch_queue (priority, seq);

-- One row per folder the engine walks, newest to oldest.
CREATE TABLE IF NOT EXISTS mail.mail_sync_sections (
    key             TEXT PRIMARY KEY,
    -- What the screen calls it ("Starred", "OASIS") and its place in
    -- marko's order, so progress can be shown without re-planning.
    title           TEXT NOT NULL DEFAULT '',
    tier            INTEGER NOT NULL DEFAULT 50,
    -- What the folder was built from. When it changes (a new sender, an
    -- edited filter) the walk starts again from the top.
    spec            TEXT NOT NULL DEFAULT '',
    -- Gmail's page token: where the walk carries on. NULL = from the newest.
    page_token      TEXT,
    -- 1 once the walk reached the oldest message.
    done            INTEGER NOT NULL DEFAULT 0,
    -- Ids seen in the walk under way, and walks completed so far.
    listed          INTEGER NOT NULL DEFAULT 0,
    passes          INTEGER NOT NULL DEFAULT 0,
    pass_started_at TEXT,
    updated_at      TEXT
);

-- Which ids a walk of a label folder (Starred, Spam, Trash, his own labels)
-- has seen, so that at the end of the walk a label Gmail no longer has can
-- be taken off. Emptied when the walk completes.
CREATE TABLE IF NOT EXISTS mail.mail_section_seen (
    section             TEXT NOT NULL,
    provider_message_id TEXT NOT NULL,
    PRIMARY KEY (section, provider_message_id)
) WITHOUT ROWID;

-- Messages marko deleted forever. Only the id, so a walk never brings one
-- back - which is what "deleted" has to mean.
CREATE TABLE IF NOT EXISTS mail.mail_deleted (
    provider_message_id TEXT PRIMARY KEY,
    deleted_at          TEXT NOT NULL
) WITHOUT ROWID;

-- THE LOCK. See the module comment.
CREATE TABLE IF NOT EXISTS mail.mail_lock (
    id       INTEGER PRIMARY KEY CHECK (id = 1),
    unlocked INTEGER NOT NULL DEFAULT 0
);
INSERT OR IGNORE INTO mail.mail_lock (id, unlocked) VALUES (1, 0);
CREATE TRIGGER IF NOT EXISTS mail.mail_messages_locked
BEFORE DELETE ON mail_messages
WHEN (SELECT unlocked FROM mail_lock WHERE id = 1) IS NOT 1
BEGIN
    SELECT RAISE(ABORT, 'TIQR mail is locked: a message can only be removed with Delete forever in Trash');
END;
"#;

/// 2.111.0. The queue says where each id was found, how it is doing and when
/// it may be tried again: a message that keeps failing steps aside and is
/// tried later, instead of leaving the queue until its folder happens to be
/// walked again (which, for a finished folder, was only after Gmail's history
/// expired).
///
/// The lanes are also renumbered - new mail, Starred, labels, and only then
/// the rest (commands/mail_sync.rs) - so ids queued by 2.110.0 move to the new
/// numbers: Important was 10, Starred 20 and labels 30.
const SCHEMA_V2: &str = r#"
ALTER TABLE mail.mail_fetch_queue ADD COLUMN source TEXT NOT NULL DEFAULT '';
ALTER TABLE mail.mail_fetch_queue ADD COLUMN status TEXT NOT NULL DEFAULT 'queued';
ALTER TABLE mail.mail_fetch_queue ADD COLUMN last_error TEXT;
ALTER TABLE mail.mail_fetch_queue ADD COLUMN updated_at TEXT;
ALTER TABLE mail.mail_fetch_queue ADD COLUMN next_attempt_at TEXT;

UPDATE mail.mail_fetch_queue
   SET priority = CASE priority WHEN 10 THEN 30 WHEN 20 THEN 10 WHEN 30 THEN 20 ELSE priority END;
UPDATE mail.mail_sync_sections
   SET tier = CASE tier WHEN 10 THEN 30 WHEN 20 THEN 10 WHEN 30 THEN 20 ELSE tier END;
UPDATE mail.mail_fetch_queue
   SET source = CASE priority
                    WHEN 0 THEN 'new' WHEN 10 THEN 'starred' WHEN 20 THEN 'label'
                    WHEN 30 THEN 'important' WHEN 40 THEN 'finances' WHEN 50 THEN 'inbox'
                    WHEN 60 THEN 'promotions' WHEN 70 THEN 'spam' WHEN 80 THEN 'trash'
                    ELSE 'backlog' END,
       updated_at = queued_at
 WHERE source = '';

-- `seq` for the next id without reading the whole queue, and the few ids
-- waiting to be tried again without reading the rest.
CREATE INDEX IF NOT EXISTS mail.idx_mail_queue_seq ON mail_fetch_queue (seq);
CREATE INDEX IF NOT EXISTS mail.idx_mail_queue_retry ON mail_fetch_queue (next_attempt_at)
    WHERE next_attempt_at IS NOT NULL;

-- When every folder was last sent back to the top because Gmail's change
-- history was lost - at most once an hour (commands/mail_sync.rs).
ALTER TABLE mail.mail_sync_state ADD COLUMN walks_reset_at TEXT;
-- Runs in a row that Google rate-limited, and that reached nothing at all:
-- each one waits longer than the last, and the first good run resets both.
ALTER TABLE mail.mail_sync_state ADD COLUMN rate_limit_streak INTEGER NOT NULL DEFAULT 0;
ALTER TABLE mail.mail_sync_state ADD COLUMN trouble_streak INTEGER NOT NULL DEFAULT 0;
"#;

/// 2.113.0. What reading a message for a record rests on, where marko filed
/// it in TIQR, and what TIQR keeps once a message has been opened - its text,
/// its attachments and a translation.
///
/// marko: "aby som vedel dane maily presunut tak kde chcem", "pri kazdom
/// maile kde je nejaky attachment tak ten tiez dat do tiqr", "aj obrazky co
/// su v maily tak prekopirj do tiqr", and "AI nesmie rozhodovat naslepo -
/// kazdy navrh musi mat zdrojove udaje, mieru istoty a kontrolu duplicit".
const SCHEMA_V3: &str = r#"
-- The words each number was read from, as JSON (commands/mail.rs, `Reading`).
ALTER TABLE mail.mail_messages ADD COLUMN extracted_evidence TEXT;
-- The text of a message that could become a record, read from its body when
-- it arrived (at most a few thousand characters), so an event created later
-- can still be matched to it. NULL for every other message.
ALTER TABLE mail.mail_messages ADD COLUMN extract_text TEXT;
-- What Automatic decided about this message and why, kept for the screen.
ALTER TABLE mail.mail_messages ADD COLUMN auto_decision TEXT;
ALTER TABLE mail.mail_messages ADD COLUMN auto_reason TEXT;
-- Where marko moved it in TIQR - a folder key, one of his labels or one of
-- his own folders. NULL = where Gmail has it. TIQR's own, like read state:
-- Gmail is never changed.
ALTER TABLE mail.mail_messages ADD COLUMN moved_to TEXT;
ALTER TABLE mail.mail_messages ADD COLUMN moved_at TEXT;
-- 1 once an opened message was seen to carry attachments.
ALTER TABLE mail.mail_messages ADD COLUMN has_attachments INTEGER NOT NULL DEFAULT 0;
CREATE INDEX IF NOT EXISTS mail.idx_mail_messages_moved ON mail_messages (moved_to) WHERE moved_to IS NOT NULL;
CREATE INDEX IF NOT EXISTS mail.idx_mail_messages_thread ON mail_messages (provider_thread_id);

-- A message as it was shown when marko opened it: the cleaned, safe HTML
-- (images already inside it) and the plain text. Opening it again needs no
-- connection. Cleared with the rest of a message's text by the retention.
CREATE TABLE IF NOT EXISTS mail.mail_bodies (
    message_id INTEGER PRIMARY KEY REFERENCES mail_messages (id) ON DELETE CASCADE,
    html       TEXT,
    text       TEXT NOT NULL DEFAULT '',
    -- Where each picture in `html` came from, as JSON: what TIQR needs to
    -- fetch it again should its copy in the files folder be gone.
    images     TEXT,
    fetched_at TEXT NOT NULL
);

-- Every attachment of an opened message. `cached_file` is set once it was
-- downloaded into the attachments folder next to the mail file.
CREATE TABLE IF NOT EXISTS mail.mail_attachments (
    message_id          INTEGER NOT NULL REFERENCES mail_messages (id) ON DELETE CASCADE,
    part_id             TEXT NOT NULL,
    filename            TEXT NOT NULL,
    declared_mime       TEXT,
    size                INTEGER NOT NULL DEFAULT 0,
    gmail_attachment_id TEXT,
    content_id          TEXT,
    is_inline           INTEGER NOT NULL DEFAULT 0,
    sniffed_kind        TEXT,
    sha256              TEXT,
    cached_file         TEXT,
    fetched_at          TEXT,
    PRIMARY KEY (message_id, part_id)
) WITHOUT ROWID;

-- What AI made of a message on marko's click - a translation, a summary, a
-- reading of the numbers - kept, so the same question is never paid twice.
CREATE TABLE IF NOT EXISTS mail.mail_ai (
    message_id INTEGER NOT NULL REFERENCES mail_messages (id) ON DELETE CASCADE,
    task       TEXT NOT NULL,
    result     TEXT NOT NULL,
    model      TEXT,
    created_at TEXT NOT NULL,
    PRIMARY KEY (message_id, task)
) WITHOUT ROWID;
"#;

#[cfg(test)]
mod tests {
    use super::*;

    fn live() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        attach_in_memory(&conn).unwrap();
        crate::db::run_migrations(&conn).unwrap();
        conn
    }

    fn insert_message(conn: &Connection, pid: &str) -> i64 {
        conn.execute(
            "INSERT INTO mail_messages (provider, provider_message_id, account_email, from_address,
                                        received_at, first_seen_at, updated_at)
             VALUES ('gmail', ?1, 'me@gmail.com', 'a@b.com', '2026-10-01T00:00:00Z',
                     '2026-10-01T00:00:00Z', '2026-10-01T00:00:00Z')",
            [pid],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    /// Scenario 15: a mail file exactly as 2.110.0 left it - messages, a
    /// queue numbered the old way, walks - goes through schema 2 with nothing
    /// lost and every queued id still in its own lane.
    #[test]
    fn mail_from_2_110_survives_schema_2_and_every_queued_id_keeps_its_lane() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        attach_in_memory(&conn).unwrap();
        conn.execute_batch(SCHEMA_V1).unwrap();
        conn.execute_batch("PRAGMA mail.user_version = 1;").unwrap();
        insert_message(&conn, "kept1");
        insert_message(&conn, "kept2");
        conn.execute(
            "UPDATE mail.mail_messages SET labels = 'inbox,starred,label_3', is_starred = 1
              WHERE provider_message_id = 'kept1'",
            [],
        )
        .unwrap();
        // 2.110.0 numbered Important 10, Starred 20, labels 30.
        for (pid, prio, seq) in [("imp", 10, 1), ("star", 20, 2), ("lab", 30, 3), ("fin", 40, 4), ("new", 0, 5)] {
            conn.execute(
                "INSERT INTO mail.mail_fetch_queue (provider_message_id, account_email, queued_at, priority, seq, attempts)
                 VALUES (?1, 'me', '2026-10-17T00:00:00+00:00', ?2, ?3, 2)",
                rusqlite::params![pid, prio, seq],
            )
            .unwrap();
        }
        conn.execute_batch(
            "INSERT INTO mail.mail_sync_sections (key, title, tier, spec, page_token)
             VALUES ('important', 'Important', 10, '', 'P'), ('starred', 'Starred', 20, '', NULL),
                    ('label:label_3', 'OASIS', 30, '', 'Q');",
        )
        .unwrap();

        crate::db::run_migrations(&conn).unwrap();

        let version: i64 = conn.query_row("PRAGMA mail.user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(version, 3);
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM mail_messages", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 2, "every message is still here");
        let (labels, starred): (String, i64) = conn
            .query_row(
                "SELECT labels, is_starred FROM mail_messages WHERE provider_message_id = 'kept1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!((labels.as_str(), starred), ("inbox,starred,label_3", 1));

        let queue: Vec<(String, i64, String, String, i64)> = conn
            .prepare(
                "SELECT provider_message_id, priority, source, status, attempts FROM mail_fetch_queue ORDER BY seq",
            )
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        use crate::commands::mail_sync::{TIER_FINANCES, TIER_IMPORTANT, TIER_LABELS, TIER_NEW, TIER_STARRED};
        let q = |s: &str| queue.iter().find(|r| r.0 == s).cloned().unwrap();
        assert_eq!(q("imp").1, TIER_IMPORTANT);
        assert_eq!(q("star").1, TIER_STARRED);
        assert_eq!(q("lab").1, TIER_LABELS);
        assert_eq!(q("fin").1, TIER_FINANCES);
        assert_eq!(q("new").1, TIER_NEW);
        assert_eq!((q("star").2.as_str(), q("star").3.as_str(), q("star").4), ("starred", "queued", 2));
        let tiers: Vec<(String, i64)> = conn
            .prepare("SELECT key, tier FROM mail_sync_sections ORDER BY key")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(
            tiers,
            vec![("important".into(), TIER_IMPORTANT), ("label:label_3".into(), TIER_LABELS), ("starred".into(), TIER_STARRED)]
        );
        // Still locked, and running it again changes nothing.
        assert!(conn.execute("DELETE FROM mail_messages", []).is_err());
        ensure(&conn).unwrap();
        let again: i64 = conn.query_row("SELECT priority FROM mail_fetch_queue WHERE provider_message_id = 'star'", [], |r| r.get(0)).unwrap();
        assert_eq!(again, TIER_STARRED, "the renumbering ran once, not twice");
    }

    /// 2.113.0: a 2.111.0 mail file goes through schema 3 with every message
    /// kept, and what V3 adds goes when its message goes.
    #[test]
    fn mail_from_2_111_survives_schema_3_and_its_extras_follow_their_message() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        attach_in_memory(&conn).unwrap();
        conn.execute_batch(SCHEMA_V1).unwrap();
        conn.execute_batch(SCHEMA_V2).unwrap();
        conn.execute_batch("PRAGMA mail.user_version = 2;").unwrap();
        let kept = insert_message(&conn, "kept");
        let gone = insert_message(&conn, "gone");

        crate::db::run_migrations(&conn).unwrap();
        let version: i64 = conn.query_row("PRAGMA mail.user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(version, 3);
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM mail_messages", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 2);
        let moved: Option<String> =
            conn.query_row("SELECT moved_to FROM mail_messages WHERE id = ?1", [kept], |r| r.get(0)).unwrap();
        assert_eq!(moved, None, "nothing is moved by an upgrade");

        for id in [kept, gone] {
            conn.execute("INSERT INTO mail_bodies (message_id, text, fetched_at) VALUES (?1, 'hi', 'x')", [id]).unwrap();
            conn.execute(
                "INSERT INTO mail_attachments (message_id, part_id, filename) VALUES (?1, '1', 'ticket.pdf')",
                [id],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO mail_ai (message_id, task, result, created_at) VALUES (?1, 'translate:sk', 'Ahoj', 'x')",
                [id],
            )
            .unwrap();
        }
        with_delete_unlocked(&conn, |c| c.execute("DELETE FROM mail_messages WHERE id = ?1", [gone])).unwrap();
        for table in ["mail_bodies", "mail_attachments", "mail_ai"] {
            let left: Vec<i64> = conn
                .prepare(&format!("SELECT message_id FROM {table}"))
                .unwrap()
                .query_map([], |r| r.get(0))
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap();
            assert_eq!(left, vec![kept], "{table}");
        }
        // Running it again changes nothing.
        ensure(&conn).unwrap();
    }

    #[test]
    fn the_mail_file_sits_next_to_the_main_file() {
        let p = mail_db_path(Path::new("/data/users/abc/tiqr-manager.sqlite3"));
        assert_eq!(p, PathBuf::from("/data/users/abc/tiqr-manager.mail.sqlite3"));
    }

    #[test]
    fn after_migrating_the_cache_tables_exist_only_in_the_mail_file() {
        let conn = live();
        for t in CACHE_TABLES {
            assert!(!main_has(&conn, t).unwrap(), "{t} is still in main");
            let in_mail: bool = conn
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM mail.sqlite_master WHERE type='table' AND name=?1)",
                    [t],
                    |r| r.get(0),
                )
                .unwrap();
            assert!(in_mail, "{t} is missing from the mail file");
        }
        // Configuration stays in main, so it travels with the cloud file.
        assert!(main_has(&conn, "mail_rules").unwrap());
        assert!(main_has(&conn, "mail_senders").unwrap());
        // ...and unqualified SQL finds the moved tables.
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM mail_sync_state", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 1);
    }

    #[test]
    fn a_connection_without_the_mail_file_is_left_exactly_as_it_was() {
        // A downloaded copy from the other machine, or a backup being checked.
        let conn = Connection::open_in_memory().unwrap();
        crate::db::run_migrations(&conn).unwrap();
        assert!(main_has(&conn, "mail_messages").unwrap());
    }

    #[test]
    fn existing_mail_moves_across_with_its_ids_and_its_log() {
        // A 2.109 database: everything in main.
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        crate::db::run_migrations(&conn).unwrap();
        let id = insert_message(&conn, "old1");
        conn.execute(
            "INSERT INTO mail_processing_log (message_id, at, step, outcome) VALUES (?1, 'x', 'fetched', 'ok')",
            [id],
        )
        .unwrap();
        conn.execute("UPDATE mail_sync_state SET paused = 1, sync_in_progress = 1 WHERE id = 1", []).unwrap();

        // First start of 2.110.0.
        attach_in_memory(&conn).unwrap();
        crate::db::run_migrations(&conn).unwrap();

        let (moved_id, log): (i64, i64) = conn
            .query_row(
                "SELECT m.id, (SELECT COUNT(*) FROM mail.mail_processing_log l WHERE l.message_id = m.id)
                   FROM mail.mail_messages m WHERE provider_message_id = 'old1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(moved_id, id);
        assert_eq!(log, 1);
        let (paused, running): (i64, i64) = conn
            .query_row("SELECT paused, sync_in_progress FROM mail_sync_state WHERE id = 1", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!((paused, running), (1, 0), "his pause survives, a stale run claim does not");
        assert!(!main_has(&conn, "mail_messages").unwrap());
    }

    #[test]
    fn a_restored_old_backup_never_overwrites_the_mail_already_here() {
        let conn = live();
        insert_message(&conn, "current");
        // What a restore of a 2.109 backup leaves in main.
        conn.execute_batch(
            "CREATE TABLE main.mail_messages (id INTEGER PRIMARY KEY, provider TEXT, provider_message_id TEXT,
                                              account_email TEXT, from_address TEXT, received_at TEXT,
                                              first_seen_at TEXT, updated_at TEXT);
             INSERT INTO main.mail_messages VALUES (1, 'gmail', 'stale', 'me@gmail.com', 'x@y.z',
                                                    '2020-01-01T00:00:00Z', 'x', 'x');",
        )
        .unwrap();
        ensure(&conn).unwrap();
        let pids: Vec<String> = conn
            .prepare("SELECT provider_message_id FROM mail.mail_messages ORDER BY id")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(pids, vec!["current".to_string()]);
        assert!(!main_has(&conn, "mail_messages").unwrap());
    }

    #[test]
    fn the_lock_refuses_a_delete_from_anywhere() {
        let conn = live();
        insert_message(&conn, "kept");
        let err = conn.execute("DELETE FROM mail_messages", []).unwrap_err();
        assert!(err.to_string().contains("locked"), "{err}");
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM mail_messages", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 1);
    }

    #[test]
    fn delete_forever_opens_the_lock_only_for_itself() {
        let conn = live();
        let id = insert_message(&conn, "gone");
        insert_message(&conn, "kept");
        let n = with_delete_unlocked(&conn, |c| c.execute("DELETE FROM mail_messages WHERE id = ?1", [id]))
            .unwrap();
        assert_eq!(n, 1);
        // Locked again straight after.
        assert!(conn.execute("DELETE FROM mail_messages", []).is_err());
        let unlocked: i64 = conn.query_row("SELECT unlocked FROM mail_lock", [], |r| r.get(0)).unwrap();
        assert_eq!(unlocked, 0);
    }

    #[test]
    fn a_failed_delete_forever_leaves_the_lock_closed() {
        let conn = live();
        insert_message(&conn, "kept");
        let r: rusqlite::Result<usize> = with_delete_unlocked(&conn, |c| {
            c.execute("DELETE FROM mail_messages", [])?;
            c.execute("SELECT * FROM no_such_table", [])
        });
        assert!(r.is_err());
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM mail_messages", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 1, "the rollback restores the message");
        let unlocked: i64 = conn.query_row("SELECT unlocked FROM mail_lock", [], |r| r.get(0)).unwrap();
        assert_eq!(unlocked, 0, "...and the lock");
    }

    #[test]
    fn the_live_connection_creates_and_uses_the_mail_file_on_disk() {
        let dir = std::env::temp_dir().join(format!("tiqr-mail-store-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let main = dir.join("tiqr-manager.sqlite3");
        let _ = std::fs::remove_file(&main);
        let _ = std::fs::remove_file(mail_db_path(&main));
        {
            let conn = crate::db::open_live_connection(&main).unwrap();
            crate::db::run_migrations(&conn).unwrap();
            insert_message(&conn, "m1");
        }
        assert!(mail_db_path(&main).exists());
        // The main file on its own - what a push uploads - holds no mail.
        let plain = Connection::open(&main).unwrap();
        assert!(!main_has(&plain, "mail_messages").unwrap());
        drop(plain);
        // Opened again, the mail is still there.
        let conn = crate::db::open_live_connection(&main).unwrap();
        crate::db::run_migrations(&conn).unwrap();
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM mail_messages", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 1);
        drop(conn);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

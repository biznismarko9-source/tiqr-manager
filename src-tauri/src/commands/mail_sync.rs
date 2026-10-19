//! 2.110.0, reworked in 2.111.0 - the mail reading engine.
//!
//! marko: "labels a starred messages musi brat vsetky a je to jedno ake su
//! stare ale tie su hlavne", "chcem aby to citalo spravy od najmladsiej po
//! najstarsiu po tych castiach a samo" - and in 2.111.0 the order made exact:
//! new mail first, then EVERY starred message, then EVERY message in his
//! labels, and only then the ordinary history. What decides is what a message
//! IS, not how old it is: a starred mail from last year comes down before an
//! ordinary one from yesterday.
//!
//! ## The lanes
//!
//! | lane | what | found by |
//! |------|------|----------|
//! | 0 new | anything that arrived, any folder | Gmail's change history |
//! | 10 starred | every message with Gmail's `STARRED`, any age | a walk of `labelIds=STARRED` |
//! | 20 labels | every message in one of his own labels, any age | a walk per label |
//! | 30-80 backlog | Important (his watched senders), Finances, Inbox, Promotions, Spam, Trash | a walk per folder |
//!
//! System labels: `STARRED` is lane 10. `INBOX`, the `CATEGORY_*` tabs,
//! `SPAM` and `TRASH` are backlog folders. Gmail's own `IMPORTANT` marker,
//! `UNREAD` and the categories are kept on each message but rank nothing -
//! Gmail puts `IMPORTANT` on a large share of all mail. `SENT`, `DRAFT` and
//! `CHAT` are not read, unless a message is starred or in one of his labels.
//!
//! ## Finding, then downloading
//!
//! Finding a message (its id, from a listing or the history) and downloading
//! it (`messages.get`, metadata only) are separate steps, joined by the queue
//! `mail_fetch_queue`. Every run, about a second, chained by the app itself
//! (Layout.tsx) while there is work:
//!
//! 1. **The change history**, every `HISTORY_EVERY_SECS` - new ids go to lane
//!    0; a star or label added to a message not here yet moves it up to lane
//!    10 or 20; a change to a stored message is applied in place, with no
//!    download.
//! 2. **One page of one folder**: the first folder, in lane order, not walked
//!    to its oldest message yet - and only while fewer than `LOW_WATER` ids
//!    can be downloaded, so finding stays just ahead of downloading.
//! 3. **One batch from the queue**: lane first, then the order Gmail listed
//!    them in, which inside a folder is newest first.
//!
//! An id found by several walks is queued ONCE, at the best lane it has, and
//! an id already stored is never queued: a starred message in three labels is
//! downloaded once, and walking mail that is already here costs listing only.
//! The lane is chosen again for every batch (40 messages); a request already
//! in flight is never cut off.
//!
//! ## When something goes wrong
//!
//! * **Google says "slow down"** (429, or 403 `rateLimitExceeded`): every
//!   worker pauses, the batch stays queued, and the next run waits - longer
//!   for each such run in a row, with jitter, and never less than Google's
//!   `Retry-After`. Until 2.110.0 this 403 read "Google refused access" and the
//!   messages that failed were dropped.
//! * **No connection, or Google failing everything**: nothing counts against
//!   any message; the run says so and waits, longer each time in a row.
//! * **One message failing** while the rest of its batch comes down: it steps
//!   aside (30 s, doubling), and after `MAX_ATTEMPTS` it is parked and tried
//!   again every few hours. Never dropped - the queue is the only record that
//!   it still has to come down.
//! * **History lost** - expired (404, Gmail keeps about a week) or no longer
//!   accepted (400): a fresh starting point is taken and every folder is
//!   walked again from the top. Stored mail is untouched and costs listing
//!   only. At most once an hour (`RESCAN_GUARD_SECS`).
//! * **A crash**: the queue and every walk's page are on disk, and everything
//!   a run writes is one transaction (`run_once`), so the next start carries
//!   on from the same place.
use crate::commands::mail::{
    apply_rules, automation_mode, classify_fetched, clean_custom_query, create_from_mail_impl,
    domain_matches, domain_of, fetched_from_gmail, finance_of, gmail_client, load_rules, log_step,
    mail_scope, proposal_kind_for, purge_expired, run_rule_action, safe_detail, sync_interval_seconds,
    upsert_message, urlencode, watched_senders, Classification, FetchedMessage, MailCategory,
    MailProviderKind, RuleSubject, FINANCE_QUERY_WORDS, FINANCE_SENDERS, GMAIL_API, MAIL_ACCOUNT_EMAIL_KEY,
    MAIL_CUSTOM_QUERY_KEY, MAIL_REFRESH_TOKEN_KEY,
};
use crate::commands::notifications::DiscordDestination;
use crate::commands::sheets_sync::get_setting;
use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::google_oauth;
use crate::models::{MailLaneProgress, MailSyncProgress, MailSyncResult};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

// ---------------------------------------------------------------------------
// The lanes.
// ---------------------------------------------------------------------------

/// The queue drains lowest first: new mail, every starred message, every
/// message in his labels - and only then the backlog, which keeps marko's
/// 2.110.0 order inside it (Important, Finances, then the rest). The gaps
/// leave room for a lane without renumbering the others.
pub(crate) const TIER_NEW: i64 = 0;
pub(crate) const TIER_STARRED: i64 = 10;
pub(crate) const TIER_LABELS: i64 = 20;
pub(crate) const TIER_IMPORTANT: i64 = 30;
pub(crate) const TIER_FINANCES: i64 = 40;
pub(crate) const TIER_INBOX: i64 = 50;
pub(crate) const TIER_PROMOTIONS: i64 = 60;
pub(crate) const TIER_SPAM: i64 = 70;
pub(crate) const TIER_TRASH: i64 = 80;

/// From here down is the ordinary historical backlog.
pub(crate) const TIER_BACKLOG: i64 = TIER_IMPORTANT;

/// How many messages one run downloads - about a second of work.
const FETCH_BATCH: usize = 40;

/// Parallel downloads. FOUR, where 2.108.0 used six: each `messages.get`
/// costs 5 of the 250 quota units a user gets per second, so ~50 messages a
/// second is Google's ceiling - and six workers sat right on it, which is
/// where the 403s marko saw came from. Four stays under it with room.
///
/// Gmail's batch endpoint (`/batch/gmail/v1`) was considered and left out:
/// every call inside a batch is charged the same 5 units, so it cannot beat
/// that ceiling, and the connections here are already kept alive.
const FETCH_THREADS: usize = 4;

/// The next folder page is listed only while fewer than this many ids can be
/// downloaded. Finding runs a little ahead of downloading, never a whole
/// mailbox ahead.
const LOW_WATER: i64 = 200;

/// Gmail's label list is re-read this often, so a label made in Gmail shows
/// up without a restart.
const LABELS_EVERY_SECS: i64 = 300;

/// The change history - new mail - is read at most this often. While there
/// is work, runs come a second apart, so new mail never waits for a long
/// first read to finish; when everything is read, runs (and so history reads)
/// come on marko's interval.
const HISTORY_EVERY_SECS: i64 = 20;

/// A message that fails while the rest of its batch comes down is tried again
/// after this, doubling each time...
const RETRY_FIRST_MS: i64 = 30_000;

/// ...and after this many tries it is parked: still queued, tried again every
/// `PARKED_RETRY_MS`, and out of everyone else's way.
const MAX_ATTEMPTS: i64 = 5;
const PARKED_RETRY_MS: i64 = 6 * 3_600_000;

/// After Google asked to slow down: this long, doubling for each such run in
/// a row, up to `RATE_LIMIT_MAX_MS`.
const RATE_LIMIT_PAUSE_MS: i64 = 20_000;
const RATE_LIMIT_MAX_MS: i64 = 5 * 60_000;

/// The pause between chained runs while there is more to read.
const CHAIN_DELAY_MS: i64 = 250;

/// After a run that reached nothing at all because Gmail could not be
/// reached, or Google failed everything - a laptop that lost its wifi,
/// mostly: this long, doubling each time in a row, up to `TROUBLE_MAX_MS`,
/// which is short enough to notice the connection coming back.
const TROUBLE_DELAY_MS: i64 = 30_000;
const TROUBLE_MAX_MS: i64 = 2 * 60_000;

/// Every folder walked again after lost history at most this often.
const RESCAN_GUARD_SECS: i64 = 3600;

/// History pages read in one run. Beyond this the checkpoint moves to the
/// last record read and the rest is read next time.
const HISTORY_MAX_PAGES: usize = 40;

/// Rows from before 2.110.0 get their Finances flag this many at a time.
const FINANCE_BACKFILL_CHUNK: i64 = 2000;

/// `base` doubled `n` times, capped at `cap`, and then shortened by up to a
/// quarter at random - so two computers, or four workers, that were told to
/// wait at the same moment do not all come back at the same moment.
pub(crate) fn backoff_ms(base: i64, n: u32, cap: i64) -> i64 {
    let full = base.saturating_mul(1i64 << n.min(20)).min(cap).max(0);
    let jitter = (rand::random::<u32>() % 1000) as i64;
    full - full / 4 * jitter / 1000
}

// ---------------------------------------------------------------------------
// Folders.
// ---------------------------------------------------------------------------

/// One folder the engine walks.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Section {
    pub key: String,
    /// What the screen calls it.
    pub title: String,
    pub tier: i64,
    /// A Gmail search, when the folder is a search.
    pub q: Option<String>,
    /// A Gmail label id exactly as Gmail spells it, when the folder is a label.
    pub label_id: Option<String>,
    pub include_spam_trash: bool,
    /// The label (lower-case, as stored on messages) that this folder
    /// vouches for. Set on every row the walk lists; taken off, at the end of
    /// a walk, from older rows it did not list.
    pub flag: Option<String>,
}

impl Section {
    fn search(key: &str, title: &str, tier: i64, q: String) -> Self {
        Section {
            key: key.to_string(),
            title: title.to_string(),
            tier,
            q: Some(q),
            label_id: None,
            include_spam_trash: false,
            flag: None,
        }
    }

    fn label(key: String, title: String, tier: i64, gmail_id: &str, include_spam_trash: bool) -> Self {
        Section {
            key,
            title,
            tier,
            q: None,
            label_id: Some(gmail_id.to_string()),
            include_spam_trash,
            flag: Some(gmail_id.to_ascii_lowercase()),
        }
    }

    /// What the walk was built from. When this changes - a new sender, an
    /// edited filter - the walk starts again from the newest message.
    pub(crate) fn spec(&self) -> String {
        format!(
            "{}|{}|{}",
            self.q.as_deref().unwrap_or(""),
            self.label_id.as_deref().unwrap_or(""),
            self.include_spam_trash
        )
    }

    /// One page of this folder, 500 ids (Gmail's most), newest first. `api` is
    /// Gmail's base URL - a test points it at a fake Gmail.
    pub(crate) fn list_url(&self, api: &str, page: Option<&str>) -> String {
        let mut url = format!("{api}/messages?maxResults=500");
        if let Some(q) = &self.q {
            url.push_str(&format!("&q={}", urlencode(q)));
        }
        if let Some(l) = &self.label_id {
            url.push_str(&format!("&labelIds={}", urlencode(l)));
        }
        if self.include_spam_trash {
            url.push_str("&includeSpamTrash=true");
        }
        if let Some(p) = page {
            url.push_str(&format!("&pageToken={}", urlencode(p)));
        }
        url
    }
}

/// One of marko's own Gmail labels.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct UserLabel {
    /// Gmail's id, case and all (`Label_8`).
    pub gmail_id: String,
    pub name: String,
}

/// What the Finances folder asks Gmail for. Single words only, because a
/// phrase would need quoting and a quote is what `clean_custom_query` exists
/// to keep out of a query.
pub(crate) fn finance_query() -> String {
    let mut terms: Vec<String> = FINANCE_SENDERS.iter().map(|d| format!("from:{d}")).collect();
    terms.extend(FINANCE_QUERY_WORDS.iter().map(|w| format!("subject:{w}")));
    format!("({}) -category:promotions -in:chats", terms.join(" OR "))
}

/// The folders, in lane order: Starred, then his labels, and only then the
/// backlog - Important, Finances, the inbox, promotions, spam and trash.
///
/// Starred and labels are LABEL walks with no date and no filter in them at
/// all: "je to jedno ake su stare". A label walk lists every message in that
/// label wherever it sits - archived mail included, not only the inbox - but
/// not spam or trash, which is what Gmail's own label view does too. Spam and
/// Trash are walked so they can be looked in; what is in them is still never
/// trusted (commands/mail.rs::is_untrusted).
///
/// In the `known` scope only Starred, the labels, Important and Finances are
/// walked - the mail that matters, and none of the rest. An EMPTY sender list
/// there still means the whole mailbox (2.109.0: a list he emptied must not
/// quietly come back as the built-in one, and "watch nothing" is not a
/// mailbox).
pub(crate) fn plan_sections(scope: &str, senders: &[String], labels: &[UserLabel], custom: &str) -> Vec<Section> {
    let whole_mailbox = scope != "known" || senders.is_empty();
    let mut out = Vec::new();
    out.push(Section::label("starred".into(), "Starred".into(), TIER_STARRED, "STARRED", false));
    let mut sorted = labels.to_vec();
    sorted.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    for l in &sorted {
        out.push(Section::label(
            format!("label:{}", l.gmail_id.to_ascii_lowercase()),
            l.name.clone(),
            TIER_LABELS,
            &l.gmail_id,
            false,
        ));
    }
    if !senders.is_empty() {
        let terms = senders.iter().map(|d| format!("from:{d}")).collect::<Vec<_>>().join(" OR ");
        out.push(Section::search("important", "Important", TIER_IMPORTANT, format!("({terms}) -in:chats")));
    }
    out.push(Section::search("finances", "Finances", TIER_FINANCES, finance_query()));
    if whole_mailbox {
        // Everything received: not what he sent, not drafts, not chat logs,
        // not promotions - those have their own walk further down.
        let mut q = "-in:sent -in:drafts -in:chats -category:promotions".to_string();
        if !custom.is_empty() {
            // His own filter narrows the inbox walk only. Starred and labels
            // are never filtered - nothing he wrote for something else may
            // hide one.
            q.push(' ');
            q.push_str(custom);
        }
        out.push(Section::search("inbox", "Inbox", TIER_INBOX, q));
        out.push(Section::label("promotions".into(), "Promotions".into(), TIER_PROMOTIONS, "CATEGORY_PROMOTIONS", false));
        out.push(Section::label("spam".into(), "Spam".into(), TIER_SPAM, "SPAM", true));
        out.push(Section::label("trash".into(), "Trash".into(), TIER_TRASH, "TRASH", true));
    }
    out
}

/// His labels as the engine needs them, from what the last label refresh
/// stored.
fn user_labels(conn: &Connection) -> AppResult<Vec<UserLabel>> {
    let mut stmt = conn.prepare(
        "SELECT gmail_id, name FROM mail_labels WHERE kind = 'user' AND gmail_id IS NOT NULL ORDER BY name",
    )?;
    let rows = stmt.query_map([], |r| Ok(UserLabel { gmail_id: r.get(0)?, name: r.get(1)? }))?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Keeps one row per planned folder: new folders start from the top, a
/// folder whose definition changed starts again, a folder that no longer
/// exists (a deleted label) is forgotten.
fn sync_section_rows(conn: &Connection, sections: &[Section], now: &str) -> AppResult<()> {
    let existing: HashMap<String, String> = {
        let mut stmt = conn.prepare("SELECT key, spec FROM mail_sync_sections")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        rows.collect::<Result<_, _>>()?
    };
    for s in sections {
        match existing.get(&s.key) {
            None => {
                conn.execute(
                    "INSERT INTO mail_sync_sections (key, title, tier, spec, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![s.key, s.title, s.tier, s.spec(), now],
                )?;
            }
            Some(spec) if *spec != s.spec() => {
                conn.execute(
                    "UPDATE mail_sync_sections
                        SET title = ?2, tier = ?3, spec = ?4, page_token = NULL, done = 0, listed = 0,
                            pass_started_at = NULL, updated_at = ?5
                      WHERE key = ?1",
                    params![s.key, s.title, s.tier, s.spec(), now],
                )?;
                conn.execute("DELETE FROM mail_section_seen WHERE section = ?1", params![s.key])?;
            }
            Some(_) => {
                conn.execute(
                    "UPDATE mail_sync_sections SET title = ?2, tier = ?3 WHERE key = ?1 AND (title <> ?2 OR tier <> ?3)",
                    params![s.key, s.title, s.tier],
                )?;
            }
        }
    }
    let planned: HashSet<&str> = sections.iter().map(|s| s.key.as_str()).collect();
    for key in existing.keys().filter(|k| !planned.contains(k.as_str())) {
        conn.execute("DELETE FROM mail_sync_sections WHERE key = ?1", params![key])?;
        conn.execute("DELETE FROM mail_section_seen WHERE section = ?1", params![key])?;
    }
    Ok(())
}

/// The first folder in marko's order that is not walked to the end yet, and
/// the page the walk carries on from.
fn next_walk(conn: &Connection, sections: &[Section]) -> AppResult<Option<(Section, Option<String>)>> {
    for s in sections {
        let row: Option<(i64, Option<String>)> = conn
            .query_row(
                "SELECT done, page_token FROM mail_sync_sections WHERE key = ?1",
                params![s.key],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        if let Some((0, page)) = row {
            return Ok(Some((s.clone(), page)));
        }
    }
    Ok(None)
}

/// Every folder from the top again - after Gmail's history expired, or a
/// new mailbox was connected.
pub(crate) fn reset_walks(conn: &Connection) -> AppResult<()> {
    conn.execute(
        "UPDATE mail_sync_sections SET page_token = NULL, done = 0, listed = 0, pass_started_at = NULL",
        [],
    )?;
    conn.execute("DELETE FROM mail_section_seen", [])?;
    Ok(())
}

/// A folder Gmail refuses to list at all is marked read-to-the-end, so the
/// folders after it are not held up. Its definition changing (a fixed
/// filter, a label made again) starts it afresh through `sync_section_rows`.
fn skip_walk(conn: &Connection, key: &str, now: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE mail_sync_sections SET done = 1, page_token = NULL, updated_at = ?2 WHERE key = ?1",
        params![key, now],
    )?;
    conn.execute("DELETE FROM mail_section_seen WHERE section = ?1", params![key])?;
    Ok(())
}

fn restart_walk(conn: &Connection, key: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE mail_sync_sections SET page_token = NULL, done = 0, listed = 0, pass_started_at = NULL WHERE key = ?1",
        params![key],
    )?;
    conn.execute("DELETE FROM mail_section_seen WHERE section = ?1", params![key])?;
    Ok(())
}

// ---------------------------------------------------------------------------
// The queue.
// ---------------------------------------------------------------------------

pub(crate) fn queue_depth(conn: &Connection) -> i64 {
    conn.query_row("SELECT COUNT(*) FROM mail_fetch_queue", [], |r| r.get(0))
        .unwrap_or(0)
}

/// Queued ids that can be downloaded right now - not the ones waiting to be
/// tried again.
fn due_count(conn: &Connection, now: &str) -> AppResult<i64> {
    Ok(conn.query_row(
        "SELECT COUNT(*) FROM mail_fetch_queue WHERE next_attempt_at IS NULL OR next_attempt_at <= ?1",
        params![now],
        |r| r.get(0),
    )?)
}

/// Queues ids that are not stored yet, at `tier`, saying where they were
/// found (`source`: a folder's key, or `history`). Already-stored and
/// deleted-forever ids are skipped in SQL, which is what makes walking a
/// folder that is already here cost nothing but the listing.
///
/// An id that is already queued is never queued twice: it moves UP when it
/// was found again in a better lane - a message first met in the inbox walk
/// and then in Starred becomes a starred one - and is left alone otherwise.
/// Returns how many ids were queued or moved up.
pub(crate) fn enqueue(
    conn: &Connection,
    account: &str,
    ids: &[String],
    tier: i64,
    source: &str,
    now: &str,
) -> AppResult<usize> {
    if ids.is_empty() {
        return Ok(0);
    }
    let mut seq: i64 = conn.query_row("SELECT IFNULL(MAX(seq), 0) FROM mail_fetch_queue", [], |r| r.get(0))?;
    let mut stmt = conn.prepare(
        "INSERT INTO mail_fetch_queue (provider_message_id, account_email, queued_at, priority, seq, source, status, updated_at)
         SELECT ?1, ?2, ?3, ?4, ?5, ?6, 'queued', ?3
          WHERE NOT EXISTS (SELECT 1 FROM mail_messages WHERE provider_message_id = ?1)
            AND NOT EXISTS (SELECT 1 FROM mail_deleted WHERE provider_message_id = ?1)
         ON CONFLICT(provider_message_id) DO UPDATE
            SET priority = excluded.priority, source = excluded.source, updated_at = excluded.updated_at
          WHERE excluded.priority < mail_fetch_queue.priority",
    )?;
    let mut n = 0usize;
    for id in ids {
        seq += 1;
        n += stmt.execute(params![id, account, now, tier, seq, source])?;
    }
    Ok(n)
}

/// The next ids to download: the best lane first, then the order they were
/// found in - which inside one folder is Gmail's own, newest first - and the
/// id itself, so the same queue always gives the same batch. A message
/// waiting to be tried again is not offered before its time.
///
/// The date is not part of it because nothing knows it yet: a listing and the
/// history give an id and nothing more, and the date is what downloading the
/// message finds out.
pub(crate) fn peek(conn: &Connection, limit: usize, now: &str) -> AppResult<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT provider_message_id FROM mail_fetch_queue
          WHERE next_attempt_at IS NULL OR next_attempt_at <= ?2
          ORDER BY priority ASC, seq ASC, provider_message_id ASC
          LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit as i64, now], |r| r.get::<_, String>(0))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

fn dequeue(conn: &Connection, ids: &[String]) -> AppResult<()> {
    let mut stmt = conn.prepare("DELETE FROM mail_fetch_queue WHERE provider_message_id = ?1")?;
    for id in ids {
        stmt.execute(params![id])?;
    }
    Ok(())
}

/// One more failed try for each id, with what went wrong. The message steps
/// aside until `next_attempt_at` - `RETRY_FIRST_MS`, doubling - and after
/// `MAX_ATTEMPTS` it is parked and tried again every `PARKED_RETRY_MS`.
///
/// Never deleted. Until 2.111.0 the fifth failure removed the id, and it came
/// back only when its folder was walked again - for a finished folder, only
/// after Gmail's history expired. A starred message could go missing for that
/// long.
fn note_failures(conn: &Connection, failed: &[(String, String)], now: DateTime<Utc>) -> AppResult<()> {
    let mut read = conn.prepare("SELECT attempts FROM mail_fetch_queue WHERE provider_message_id = ?1")?;
    let mut write = conn.prepare(
        "UPDATE mail_fetch_queue
            SET attempts = ?2, status = ?3, last_error = ?4, next_attempt_at = ?5, updated_at = ?6
          WHERE provider_message_id = ?1",
    )?;
    for (pid, why) in failed {
        let Some(before) = read.query_row(params![pid], |r| r.get::<_, i64>(0)).optional()? else {
            continue;
        };
        let attempts = before + 1;
        let (status, wait_ms) = if attempts >= MAX_ATTEMPTS {
            ("parked", PARKED_RETRY_MS)
        } else {
            ("retrying", backoff_ms(RETRY_FIRST_MS, (attempts - 1) as u32, PARKED_RETRY_MS))
        };
        let at = (now + chrono::Duration::milliseconds(wait_ms)).to_rfc3339();
        write.execute(params![pid, attempts, status, safe_detail(why), at, now.to_rfc3339()])?;
    }
    Ok(())
}

/// How long until the first message waiting to be tried again is due, if
/// any is.
fn next_retry_in_ms(conn: &Connection, now: DateTime<Utc>) -> AppResult<Option<i64>> {
    let next: Option<String> = conn.query_row(
        "SELECT MIN(next_attempt_at) FROM mail_fetch_queue WHERE next_attempt_at > ?1",
        params![now.to_rfc3339()],
        |r| r.get(0),
    )?;
    Ok(next
        .and_then(|t| DateTime::parse_from_rfc3339(&t).ok())
        .map(|t| (t.with_timezone(&Utc) - now).num_milliseconds().max(0)))
}

// ---------------------------------------------------------------------------
// Labels on stored messages.
// ---------------------------------------------------------------------------

/// `labels` is a comma list; this is "has this exact label", without LIKE,
/// whose `_` wildcard would let `label_8` match `labelX8`.
pub(crate) fn has_label_sql(flag_param: &str) -> String {
    format!("instr(',' || IFNULL(labels, '') || ',', ',' || {flag_param} || ',') > 0")
}

fn add_flag(conn: &Connection, pid: &str, flag: &str, now: &str) -> AppResult<usize> {
    let mut n = conn.execute(
        &format!(
            "UPDATE mail_messages
                SET labels = CASE WHEN labels IS NULL OR labels = '' THEN ?2 ELSE labels || ',' || ?2 END,
                    labels_at = ?3
              WHERE provider_message_id = ?1 AND NOT {}",
            has_label_sql("?2")
        ),
        params![pid, flag, now],
    )?;
    if flag == "starred" {
        n += conn.execute(
            "UPDATE mail_messages SET is_starred = 1, labels_at = ?2 WHERE provider_message_id = ?1 AND is_starred = 0",
            params![pid, now],
        )?;
    }
    Ok(n)
}

fn remove_flag(conn: &Connection, pid: &str, flag: &str, now: &str) -> AppResult<usize> {
    let mut n = conn.execute(
        &format!(
            "UPDATE mail_messages
                SET labels = TRIM(REPLACE(',' || IFNULL(labels, '') || ',', ',' || ?2 || ',', ','), ','),
                    labels_at = ?3
              WHERE provider_message_id = ?1 AND {}",
            has_label_sql("?2")
        ),
        params![pid, flag, now],
    )?;
    if flag == "starred" {
        n += conn.execute(
            "UPDATE mail_messages SET is_starred = 0, labels_at = ?2 WHERE provider_message_id = ?1 AND is_starred = 1",
            params![pid, now],
        )?;
    }
    Ok(n)
}

/// The end of a walk of a label folder: rows that carry the label but were
/// not listed have lost it in Gmail, so it comes off here too.
///
/// Only rows OLDER than the walk. A row stored, or re-labelled from history,
/// after the walk began already carries what Gmail said at that moment, and
/// undoing it would undo a star marko added while the walk was running. And
/// for a folder that does not list spam and trash, rows sitting in spam or
/// trash are left alone - not being listed is exactly what they would do.
fn reconcile(conn: &Connection, section: &Section, pass_started_at: &str, now: &str) -> AppResult<usize> {
    let Some(flag) = &section.flag else {
        return Ok(0);
    };
    let carries = if flag == "starred" {
        format!("(is_starred = 1 OR {})", has_label_sql("?1"))
    } else {
        has_label_sql("?1")
    };
    let junk_guard = if section.include_spam_trash {
        String::new()
    } else {
        format!(
            "AND NOT {} AND NOT {} AND trashed_at IS NULL",
            has_label_sql("'spam'"),
            has_label_sql("'trash'")
        )
    };
    let sql = format!(
        "SELECT provider_message_id FROM mail_messages
          WHERE {carries}
            AND first_seen_at < ?2
            AND (labels_at IS NULL OR labels_at < ?2)
            {junk_guard}
            AND provider_message_id NOT IN (SELECT provider_message_id FROM mail_section_seen WHERE section = ?3)"
    );
    let stale: Vec<String> = {
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params![flag, pass_started_at, section.key], |r| r.get::<_, String>(0))?;
        rows.collect::<Result<_, _>>()?
    };
    let mut n = 0;
    for pid in &stale {
        n += remove_flag(conn, pid, flag, now)?;
    }
    Ok(n)
}

/// One page of one folder, applied. Returns how many stored messages changed.
fn apply_listing(
    conn: &Connection,
    account: &str,
    section: &Section,
    page_used: Option<&str>,
    ids: &[String],
    next: Option<&str>,
    now: &str,
) -> AppResult<usize> {
    if page_used.is_none() {
        // A walk starting from the newest message.
        conn.execute(
            "UPDATE mail_sync_sections SET pass_started_at = ?2, listed = 0 WHERE key = ?1",
            params![section.key, now],
        )?;
        conn.execute("DELETE FROM mail_section_seen WHERE section = ?1", params![section.key])?;
    }
    enqueue(conn, account, ids, section.tier, &section.key, now)?;
    let mut changed = 0usize;
    if let Some(flag) = &section.flag {
        let mut seen = conn.prepare(
            "INSERT OR IGNORE INTO mail_section_seen (section, provider_message_id) VALUES (?1, ?2)",
        )?;
        for id in ids {
            seen.execute(params![section.key, id])?;
            changed += add_flag(conn, id, flag, now)?;
        }
    }
    conn.execute(
        "UPDATE mail_sync_sections SET page_token = ?2, listed = listed + ?3, updated_at = ?4 WHERE key = ?1",
        params![section.key, next, ids.len() as i64, now],
    )?;
    if next.is_none() {
        let started: Option<String> = conn
            .query_row(
                "SELECT pass_started_at FROM mail_sync_sections WHERE key = ?1",
                params![section.key],
                |r| r.get(0),
            )
            .optional()?
            .flatten();
        if let Some(started) = started {
            changed += reconcile(conn, section, &started, now)?;
        }
        conn.execute(
            "UPDATE mail_sync_sections SET done = 1, passes = passes + 1 WHERE key = ?1",
            params![section.key],
        )?;
        conn.execute("DELETE FROM mail_section_seen WHERE section = ?1", params![section.key])?;
    }
    Ok(changed)
}

// ---------------------------------------------------------------------------
// Gmail's change history.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum HistoryEvent {
    Added { id: String, labels: Vec<String> },
    LabelsAdded { id: String, labels: Vec<String> },
    LabelsRemoved { id: String, labels: Vec<String> },
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct HistoryRead {
    pub events: Vec<HistoryEvent>,
    /// Where the next read starts.
    pub checkpoint: Option<String>,
}

fn str_of(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(|x| x.as_str()).map(|s| s.to_string())
}

fn lower_list(v: Option<&Value>) -> Vec<String> {
    v.and_then(|a| a.as_array())
        .map(|a| a.iter().filter_map(|l| l.as_str()).map(|l| l.to_ascii_lowercase()).collect())
        .unwrap_or_default()
}

/// One page of `users.history.list`: its events in order, the next page's
/// token, the mailbox's current history id, and the last record's own id.
/// Pure, so the shape can be tested from a fixture.
pub(crate) fn parse_history_page(v: &Value) -> (Vec<HistoryEvent>, Option<String>, Option<String>, Option<String>) {
    let mut events = Vec::new();
    let mut last_record: Option<String> = None;
    let empty = Vec::new();
    for rec in v.get("history").and_then(|h| h.as_array()).unwrap_or(&empty) {
        if let Some(id) = str_of(rec, "id") {
            last_record = Some(id);
        }
        let list = |key: &str| rec.get(key).and_then(|a| a.as_array()).cloned().unwrap_or_default();
        for a in list("messagesAdded") {
            if let Some(m) = a.get("message") {
                if let Some(id) = str_of(m, "id") {
                    events.push(HistoryEvent::Added { id, labels: lower_list(m.get("labelIds")) });
                }
            }
        }
        for a in list("labelsAdded") {
            if let Some(id) = a.get("message").and_then(|m| str_of(m, "id")) {
                events.push(HistoryEvent::LabelsAdded { id, labels: lower_list(a.get("labelIds")) });
            }
        }
        for a in list("labelsRemoved") {
            if let Some(id) = a.get("message").and_then(|m| str_of(m, "id")) {
                events.push(HistoryEvent::LabelsRemoved { id, labels: lower_list(a.get("labelIds")) });
            }
        }
    }
    (events, str_of(v, "nextPageToken"), str_of(v, "historyId"), last_record)
}

fn has(labels: &[String], l: &str) -> bool {
    labels.iter().any(|x| x == l)
}

fn is_user_label(l: &str) -> bool {
    l.starts_with("label_")
}

fn is_stored(conn: &Connection, pid: &str) -> AppResult<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM mail_messages WHERE provider_message_id = ?1)",
        params![pid],
        |r| r.get(0),
    )?)
}

/// Applies history events in order. Returns (stored messages changed, ids
/// queued or moved up).
///
/// * A new message goes to lane 0 whatever folder it landed in - a new
///   promotion or spam is still new, and one is a handful, not a backlog. Not
///   drafts, chat logs, or mail he only sent.
/// * A star or one of his labels added to a message that is not here yet
///   moves it to lane 10 or 20 - up from wherever a walk had queued it.
/// * Anything added to or taken off a STORED message - a star, a label, the
///   inbox, spam, trash - is applied to it in place. Nothing is downloaded
///   again: the change is all there is to know.
///
/// Repeating an event changes nothing: a flag is added only where missing and
/// taken off only where present, and an id is queued once.
///
/// TIQR's own read state is deliberately left alone (migration 041): Gmail's
/// UNREAD going on or off is not applied, so a message he marked read here
/// is not turned unread because his phone saw it differently.
pub(crate) fn apply_history(conn: &Connection, account: &str, events: &[HistoryEvent], now: &str) -> AppResult<(usize, usize)> {
    let mut changed = 0usize;
    let mut arrived: Vec<String> = Vec::new();
    let mut starred: Vec<String> = Vec::new();
    let mut labelled: Vec<String> = Vec::new();
    for ev in events {
        match ev {
            HistoryEvent::Added { id, labels } => {
                if has(labels, "draft") || has(labels, "chat") {
                    continue;
                }
                let kept = has(labels, "starred") || labels.iter().any(|l| is_user_label(l));
                if has(labels, "sent") && !has(labels, "inbox") && !kept {
                    continue;
                }
                arrived.push(id.clone());
            }
            HistoryEvent::LabelsAdded { id, labels } => {
                if is_stored(conn, id)? {
                    for l in labels.iter().filter(|l| *l != "unread") {
                        changed += add_flag(conn, id, l, now)?;
                    }
                } else if has(labels, "starred") {
                    // An old message he just starred in Gmail.
                    starred.push(id.clone());
                } else if labels.iter().any(|l| is_user_label(l)) {
                    labelled.push(id.clone());
                }
            }
            HistoryEvent::LabelsRemoved { id, labels } => {
                for l in labels.iter().filter(|l| *l != "unread") {
                    changed += remove_flag(conn, id, l, now)?;
                }
            }
        }
    }
    let mut queued = enqueue(conn, account, &arrived, TIER_NEW, "history", now)?;
    queued += enqueue(conn, account, &starred, TIER_STARRED, "history:starred", now)?;
    queued += enqueue(conn, account, &labelled, TIER_LABELS, "history:label", now)?;
    Ok((changed, queued))
}

// ---------------------------------------------------------------------------
// Gmail HTTP.
// ---------------------------------------------------------------------------

/// What went wrong talking to Gmail, read from the status AND the reason
/// code in Google's answer - because a 403 can mean four different things.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum GmailFailure {
    RateLimited { retry_after_secs: Option<u64> },
    Unauthorized,
    NoPermission,
    ApiDisabled,
    Forbidden,
    NotFound,
    BadRequest,
    Server(u16),
    Network,
    Unreadable,
}

impl GmailFailure {
    /// marko's words, not Google's. Google's own text is never shown.
    pub(crate) fn message(&self) -> String {
        match self {
            GmailFailure::RateLimited { .. } => {
                "Google asked TIQR to slow down for a moment. Nothing is lost - it carries on by itself.".to_string()
            }
            GmailFailure::Unauthorized => {
                "Google would not accept the saved sign-in. Reconnect the mailbox in Mail -> Settings.".to_string()
            }
            GmailFailure::NoPermission => "The mailbox was connected without permission to read mail. Reconnect it in Mail -> Settings and leave the mail permission ticked on Google's screen.".to_string(),
            GmailFailure::ApiDisabled => "The Gmail API is switched off for this app's Google Cloud project. That is fixed in the Google Cloud console, not in TIQR.".to_string(),
            GmailFailure::Forbidden => "Google refused the request (403). If this keeps happening, reconnect the mailbox in Mail -> Settings.".to_string(),
            GmailFailure::NotFound => "Gmail could not find that message any more.".to_string(),
            GmailFailure::BadRequest => "Google rejected a request as malformed. TIQR starts that folder again from the top.".to_string(),
            GmailFailure::Server(code) => format!("Google had a server error ({code}). It is temporary - TIQR tries again by itself."),
            GmailFailure::Network => "Could not reach Gmail. TIQR tries again by itself when the connection is back.".to_string(),
            GmailFailure::Unreadable => "Gmail sent an answer TIQR could not read. It tries again by itself.".to_string(),
        }
    }

    /// The mailbox cannot be read at all until marko does something.
    pub(crate) fn stops_the_run(&self) -> bool {
        matches!(
            self,
            GmailFailure::Unauthorized | GmailFailure::NoPermission | GmailFailure::ApiDisabled | GmailFailure::Forbidden
        )
    }

    fn is_rate_limit(&self) -> bool {
        matches!(self, GmailFailure::RateLimited { .. })
    }

    /// Google's own `Retry-After`, when it sent one with a rate limit.
    fn retry_after_secs(&self) -> Option<u64> {
        match self {
            GmailFailure::RateLimited { retry_after_secs } => *retry_after_secs,
            _ => None,
        }
    }

    /// No answer at all, or Google having a moment. Says nothing about the
    /// message or the mailbox, and goes away by itself.
    fn is_transient(&self) -> bool {
        matches!(self, GmailFailure::Network | GmailFailure::Server(_) | GmailFailure::Unreadable)
    }
}

/// Reads a non-2xx answer. PURE.
///
/// The body is searched for Google's REASON CODE only. It is never shown,
/// logged or stored - the same rule the module has always kept about
/// Google's error text, kept here too.
pub(crate) fn failure_from(code: u16, body: &str, retry_after_secs: Option<u64>) -> GmailFailure {
    let b = body.to_ascii_lowercase();
    let any = |needles: &[&str]| needles.iter().any(|n| b.contains(n));
    let rate = any(&[
        "ratelimitexceeded",
        "userratelimitexceeded",
        "quotaexceeded",
        "dailylimitexceeded",
        "resource_exhausted",
        "rate_limit_exceeded",
    ]);
    match code {
        429 => GmailFailure::RateLimited { retry_after_secs },
        403 if rate => GmailFailure::RateLimited { retry_after_secs },
        403 if any(&["insufficientpermissions", "access_token_scope_insufficient", "insufficient authentication scopes"]) => {
            GmailFailure::NoPermission
        }
        403 if any(&["accessnotconfigured", "service_disabled", "has not been used in project"]) => {
            GmailFailure::ApiDisabled
        }
        403 => GmailFailure::Forbidden,
        401 => GmailFailure::Unauthorized,
        404 => GmailFailure::NotFound,
        400 => GmailFailure::BadRequest,
        other => GmailFailure::Server(other),
    }
}

/// Every worker waits on this after any of them was told to slow down, so
/// four threads do not each find the limit for themselves.
static PAUSE_UNTIL_MS: AtomicU64 = AtomicU64::new(0);

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn pause_all_for(ms: u64) {
    PAUSE_UNTIL_MS.fetch_max(now_ms() + ms, Ordering::SeqCst);
}

fn wait_for_pause() {
    let until = PAUSE_UNTIL_MS.load(Ordering::SeqCst);
    let now = now_ms();
    if until > now {
        std::thread::sleep(Duration::from_millis((until - now).min(30_000)));
    }
}

/// Tries per request before the failure is handed back to the run.
const REQUEST_ATTEMPTS: u32 = 3;

/// One authenticated GET, with backoff for a rate limit and for Google's own
/// bad moments: exponential, with jitter, and never shorter than Google's
/// `Retry-After`. A rate limit that outlasts the tries is returned so the run
/// can stop and the app can wait; a timeout (the client's 30 s) counts as no
/// connection.
pub(crate) fn gmail_json(http: &reqwest::blocking::Client, token: &str, url: &str) -> Result<Value, GmailFailure> {
    let mut last = GmailFailure::Network;
    for attempt in 0..REQUEST_ATTEMPTS {
        let again = attempt + 1 < REQUEST_ATTEMPTS;
        wait_for_pause();
        let resp = match http.get(url).bearer_auth(token).send() {
            Ok(r) => r,
            Err(_) => {
                last = GmailFailure::Network;
                if again {
                    std::thread::sleep(Duration::from_millis(backoff_ms(400, attempt, 5_000) as u64));
                }
                continue;
            }
        };
        let code = resp.status().as_u16();
        if (200..=299).contains(&code) {
            return resp.json::<Value>().map_err(|_| GmailFailure::Unreadable);
        }
        let retry_after = resp
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.trim().parse::<u64>().ok());
        let body = resp.text().unwrap_or_default();
        match failure_from(code, &body, retry_after) {
            GmailFailure::RateLimited { retry_after_secs } => {
                // Every worker waits, not just this one - they share the quota.
                let wait = match retry_after_secs {
                    Some(s) => s.saturating_mul(1000).min(30_000) as i64,
                    None => backoff_ms(1000, attempt, 30_000),
                };
                pause_all_for(wait.clamp(0, 30_000) as u64);
                last = GmailFailure::RateLimited { retry_after_secs };
            }
            GmailFailure::Server(c) => {
                if again {
                    std::thread::sleep(Duration::from_millis(backoff_ms(400, attempt, 5_000) as u64));
                }
                last = GmailFailure::Server(c);
            }
            other => return Err(other),
        }
    }
    Err(last)
}

/// The access token, refreshed only when it is about to run out. It used to
/// be refreshed on every run; with runs chained a second apart that is a
/// token request a second for nothing.
static TOKEN: Mutex<Option<(u64, String, DateTime<Utc>)>> = Mutex::new(None);

fn token_key(refresh: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in refresh.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x1000_0000_01b3);
    }
    h
}

pub(crate) fn access_token(client: &google_oauth::OAuthClient, refresh: &str) -> AppResult<String> {
    let key = token_key(refresh);
    {
        let cached = TOKEN.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((k, tok, exp)) = cached.as_ref() {
            if *k == key && *exp > Utc::now() + chrono::Duration::seconds(60) {
                return Ok(tok.clone());
            }
        }
    }
    let (tok, exp) = google_oauth::refresh_access_token(client, refresh).map_err(|e| {
        AppError::External(safe_detail(&format!(
            "Google would not refresh the mailbox sign-in ({e}). If this keeps happening, reconnect the mailbox in Mail -> Settings."
        )))
    })?;
    *TOKEN.lock().unwrap_or_else(|e| e.into_inner()) = Some((key, tok.clone(), exp));
    Ok(tok)
}

pub(crate) fn forget_access_token() {
    *TOKEN.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

// ---------------------------------------------------------------------------
// One run.
// ---------------------------------------------------------------------------

/// Only one run at a time, in memory. The database flag of the same name is
/// for the screen; a crash cannot leave THIS one set, which is why a stale
/// claim can no longer block every later sync.
static RUNNING: AtomicBool = AtomicBool::new(false);

struct RunGuard;

impl RunGuard {
    fn claim() -> Option<RunGuard> {
        RUNNING
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .ok()
            .map(|_| RunGuard)
    }
}

impl Drop for RunGuard {
    fn drop(&mut self) {
        RUNNING.store(false, Ordering::SeqCst);
    }
}

pub(crate) fn is_running() -> bool {
    RUNNING.load(Ordering::SeqCst)
}

/// What a run will do, decided under the lock.
#[derive(Debug, Clone)]
struct Plan {
    refresh_token: String,
    account_email: String,
    known_scope: bool,
    senders: Vec<String>,
    need_profile: bool,
    labels_due: bool,
    history_from: Option<String>,
    listing: Option<(Section, Option<String>)>,
    batch: Vec<String>,
    interval_secs: i64,
    /// Gmail's base URL: `GMAIL_API`, or a fake Gmail in a test.
    api: String,
}

#[derive(Debug)]
enum Fetched {
    Got(Box<FetchedMessage>, Classification),
    Gone,
    Failed(GmailFailure),
    /// Not attempted: another worker was told to slow down first, or found
    /// the connection gone.
    Skipped,
}

/// What came back from Google, gathered with no lock held.
#[derive(Debug, Default)]
struct Net {
    profile_history: Option<String>,
    labels: Option<Vec<(String, String, String)>>,
    history: Option<Result<HistoryRead, GmailFailure>>,
    listing: Option<Result<(Vec<String>, Option<String>), GmailFailure>>,
    fetched: Vec<(String, Fetched)>,
    stop: Option<GmailFailure>,
    rate_limited: bool,
    /// The longest `Retry-After` Google sent with a rate limit this run.
    retry_after_secs: Option<u64>,
    /// The first failure that goes away by itself (`is_transient`). When
    /// nothing at all came down, the run says so and waits.
    trouble: Option<GmailFailure>,
}

fn due(last: Option<&str>, every_secs: i64, now: DateTime<Utc>) -> bool {
    match last.and_then(|s| DateTime::parse_from_rfc3339(s).ok()) {
        None => true,
        Some(t) => (now - t.with_timezone(&Utc)).num_seconds() >= every_secs,
    }
}

#[derive(Debug, Default)]
struct EngineState {
    paused: bool,
    last_history_id: Option<String>,
    history_checked_at: Option<String>,
    labels_checked_at: Option<String>,
}

fn engine_state(conn: &Connection) -> AppResult<EngineState> {
    Ok(conn
        .query_row(
            "SELECT paused, last_history_id, history_checked_at, labels_checked_at FROM mail_sync_state WHERE id = 1",
            [],
            |r| {
                Ok(EngineState {
                    paused: r.get::<_, i64>(0)? != 0,
                    last_history_id: r.get(1)?,
                    history_checked_at: r.get(2)?,
                    labels_checked_at: r.get(3)?,
                })
            },
        )
        .optional()?
        .unwrap_or_default())
}

fn make_plan(conn: &Connection, now: DateTime<Utc>) -> AppResult<Result<Plan, &'static str>> {
    let Some(refresh_token) = get_setting(conn, MAIL_REFRESH_TOKEN_KEY)? else {
        return Ok(Err("No mailbox is connected yet."));
    };
    let st = engine_state(conn)?;
    if st.paused {
        return Ok(Err("Mail sync is paused."));
    }
    let now_s = now.to_rfc3339();
    let account_email = get_setting(conn, MAIL_ACCOUNT_EMAIL_KEY)?.unwrap_or_default();
    let scope = mail_scope(conn)?;
    let senders = watched_senders(conn)?;
    let custom = clean_custom_query(&get_setting(conn, MAIL_CUSTOM_QUERY_KEY)?.unwrap_or_default());
    let labels = user_labels(conn)?;
    let sections = plan_sections(&scope, &senders, &labels, &custom);
    sync_section_rows(conn, &sections, &now_s)?;

    // 0 is "only when I press Sync"; a pressed Sync still reads history.
    let interval_secs = sync_interval_seconds(conn);
    let need_profile = st.last_history_id.is_none();
    let labels_due = due(st.labels_checked_at.as_deref(), LABELS_EVERY_SECS, now);
    let history_from = if need_profile {
        None
    } else if due(st.history_checked_at.as_deref(), HISTORY_EVERY_SECS, now) {
        st.last_history_id.clone()
    } else {
        None
    };
    // Measured against what can be downloaded now, so messages waiting to
    // be tried again never stop the walks from going on.
    let listing = if due_count(conn, &now_s)? < LOW_WATER { next_walk(conn, &sections)? } else { None };
    let batch = peek(conn, FETCH_BATCH, &now_s)?;
    Ok(Ok(Plan {
        refresh_token,
        account_email,
        known_scope: scope == "known" && !senders.is_empty(),
        senders,
        need_profile,
        labels_due,
        history_from,
        listing,
        batch,
        interval_secs,
        api: GMAIL_API.to_string(),
    }))
}

impl Net {
    fn note_trouble(&mut self, f: &GmailFailure) {
        if f.is_transient() && self.trouble.is_none() {
            self.trouble = Some(f.clone());
        }
    }

    fn note_rate_limit(&mut self, f: &GmailFailure) {
        self.rate_limited = true;
        if let Some(s) = f.retry_after_secs() {
            self.retry_after_secs = Some(self.retry_after_secs.map_or(s, |t| t.max(s)));
        }
    }

    fn stopped_by(mut self, f: GmailFailure) -> Net {
        if f.is_rate_limit() {
            self.note_rate_limit(&f);
        } else {
            self.stop = Some(f);
        }
        self
    }
}

fn parse_labels(v: &Value) -> Vec<(String, String, String)> {
    v.get("labels")
        .and_then(|l| l.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|l| {
                    Some((
                        str_of(l, "id")?,
                        str_of(l, "name")?,
                        str_of(l, "type").unwrap_or_else(|| "user".to_string()),
                    ))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn read_history(http: &reqwest::blocking::Client, token: &str, api: &str, from: &str) -> Result<HistoryRead, GmailFailure> {
    let mut out = HistoryRead::default();
    let mut page: Option<String> = None;
    for _ in 0..HISTORY_MAX_PAGES {
        let mut url = format!(
            "{api}/history?startHistoryId={}&maxResults=500\
             &historyTypes=messageAdded&historyTypes=labelAdded&historyTypes=labelRemoved",
            urlencode(from)
        );
        if let Some(p) = &page {
            url.push_str(&format!("&pageToken={}", urlencode(p)));
        }
        let v = gmail_json(http, token, &url)?;
        let (events, next, current, last_record) = parse_history_page(&v);
        out.events.extend(events);
        match next {
            Some(n) => {
                page = Some(n);
                // If the page cap ends the read here, carry on from the
                // last record actually read - never from "now", which would
                // skip what was not read.
                if let Some(r) = last_record {
                    out.checkpoint = Some(r);
                }
            }
            None => {
                out.checkpoint = current.or(out.checkpoint);
                return Ok(out);
            }
        }
    }
    Ok(out)
}

fn list_page(
    http: &reqwest::blocking::Client,
    token: &str,
    api: &str,
    section: &Section,
    page: Option<&str>,
) -> Result<(Vec<String>, Option<String>), GmailFailure> {
    let v = gmail_json(http, token, &section.list_url(api, page))?;
    let ids = v
        .get("messages")
        .and_then(|m| m.as_array())
        .map(|a| a.iter().filter_map(|m| str_of(m, "id")).collect())
        .unwrap_or_default();
    Ok((ids, str_of(&v, "nextPageToken")))
}

/// METADATA only - sender, subject, date, labels and Gmail's preview. The
/// body is fetched when a message is opened and never stored.
fn fetch_batch(http: &reqwest::blocking::Client, token: &str, api: &str, ids: &[String]) -> Vec<(String, Fetched)> {
    if ids.is_empty() {
        return Vec::new();
    }
    let stop = AtomicBool::new(false);
    let per = ids.len().div_ceil(FETCH_THREADS).max(1);
    std::thread::scope(|scope| {
        let handles: Vec<_> = ids
            .chunks(per)
            .map(|part| {
                let stop = &stop;
                scope.spawn(move || {
                    part.iter()
                        .map(|id| {
                            if stop.load(Ordering::SeqCst) {
                                return (id.clone(), Fetched::Skipped);
                            }
                            let url = format!(
                                "{api}/messages/{}?format=metadata\
                                 &metadataHeaders=From&metadataHeaders=Subject\
                                 &metadataHeaders=Date&metadataHeaders=To\
                                 &metadataHeaders=Reply-To",
                                urlencode(id)
                            );
                            let outcome = match gmail_json(http, token, &url) {
                                Ok(raw) => match fetched_from_gmail(&raw) {
                                    Some(m) => {
                                        let c = classify_fetched(&m);
                                        Fetched::Got(Box::new(m), c)
                                    }
                                    None => Fetched::Gone,
                                },
                                Err(GmailFailure::NotFound) => Fetched::Gone,
                                Err(f) => {
                                    // No connection: the rest of the batch
                                    // would only wait out the same retries.
                                    if f.is_rate_limit() || f.stops_the_run() || f == GmailFailure::Network {
                                        stop.store(true, Ordering::SeqCst);
                                    }
                                    Fetched::Failed(f)
                                }
                            };
                            (id.clone(), outcome)
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles.into_iter().flat_map(|h| h.join().unwrap_or_default()).collect()
    })
}

/// Everything that talks to Google, with no lock held. `token` is a live
/// access token (`access_token`); `http` is `gmail_client()`, or a test's.
fn network(plan: &Plan, http: &reqwest::blocking::Client, token: &str) -> AppResult<Net> {
    let api = plan.api.as_str();
    let mut net = Net::default();

    if plan.need_profile {
        // The starting point for the change history, taken before any walk
        // lists a page - so whatever arrives while a long first read is under
        // way is in the history.
        match gmail_json(http, token, &format!("{api}/profile")) {
            Ok(v) => net.profile_history = str_of(&v, "historyId"),
            Err(f) if f.stops_the_run() || f.is_rate_limit() => return Ok(net.stopped_by(f)),
            Err(f) => net.note_trouble(&f),
        }
    }
    if plan.labels_due {
        match gmail_json(http, token, &format!("{api}/labels")) {
            Ok(v) => net.labels = Some(parse_labels(&v)),
            Err(f) if f.stops_the_run() || f.is_rate_limit() => return Ok(net.stopped_by(f)),
            Err(f) => net.note_trouble(&f),
        }
    }
    if let Some(from) = &plan.history_from {
        let r = read_history(http, token, api, from);
        if let Err(f) = &r {
            if f.stops_the_run() || f.is_rate_limit() {
                return Ok(net.stopped_by(f.clone()));
            }
            net.note_trouble(f);
        }
        net.history = Some(r);
    }
    if let Some((section, page)) = &plan.listing {
        let r = list_page(http, token, api, section, page.as_deref());
        if let Err(f) = &r {
            if f.stops_the_run() || f.is_rate_limit() {
                return Ok(net.stopped_by(f.clone()));
            }
            net.note_trouble(f);
        }
        net.listing = Some(r);
    }
    net.fetched = fetch_batch(http, token, api, &plan.batch);
    let mut trouble = None;
    let mut limits = Vec::new();
    for (_, f) in &net.fetched {
        if let Fetched::Failed(f) = f {
            if f.is_rate_limit() {
                limits.push(f.clone());
            } else if f.stops_the_run() && net.stop.is_none() {
                net.stop = Some(f.clone());
            } else if trouble.is_none() {
                trouble = Some(f.clone());
            }
        }
    }
    for f in &limits {
        net.note_rate_limit(f);
    }
    if let Some(f) = trouble {
        net.note_trouble(&f);
    }
    Ok(net)
}

/// In the `known` scope only mail that matters is kept: a watched sender,
/// money mail, or anything he starred or labelled himself.
pub(crate) fn wanted_in_known_scope(m: &FetchedMessage, class: &Classification, senders: &[String]) -> bool {
    if m.is_starred || m.labels.split(',').any(is_user_label) {
        return true;
    }
    let watched = domain_of(&m.from_address)
        .map(|d| senders.iter().any(|s| domain_matches(&d, s)))
        .unwrap_or(false);
    watched || finance_of(&m.from_address, &m.subject, class.category.as_str(), &m.labels)
}

#[derive(Debug, Default)]
struct Stored {
    fetched: i64,
    stored_new: i64,
    duplicates: i64,
    review: i64,
    new_rows: Vec<(i64, String)>,
}

/// Stores what came down and settles each id in the queue: stored, gone or
/// refused ids leave it; a failure is noted against the message (see
/// `note_failures`) only when it was the message's own.
fn store_fetched(conn: &Connection, plan: &Plan, fetched: Vec<(String, Fetched)>, now: DateTime<Utc>) -> AppResult<Stored> {
    let rules = load_rules(conn)?;
    let answered = fetched.iter().any(|(_, f)| matches!(f, Fetched::Got(..) | Fetched::Gone));
    let mut out = Stored::default();
    let mut done: Vec<String> = Vec::new();
    let mut failed: Vec<(String, String)> = Vec::new();
    for (pid, f) in fetched {
        match f {
            Fetched::Got(msg, class) => {
                out.fetched += 1;
                if plan.known_scope && !wanted_in_known_scope(&msg, &class, &plan.senders) {
                    done.push(pid);
                    continue;
                }
                let (row_id, was_new) =
                    upsert_message(conn, MailProviderKind::Gmail, &plan.account_email, &msg, &class)?;
                if was_new {
                    out.stored_new += 1;
                    // Only what matters is logged. A "fetched" and a
                    // "classified: other" row for every one of 100 000
                    // messages was most of this file's size and told nobody
                    // anything - the message row already says when it came.
                    if class.category != MailCategory::Other {
                        out.review += 1;
                        log_step(conn, row_id, "classified", class.category.as_str(), class.review_reason.as_deref())?;
                    }
                    // marko's rules, on NEW mail only - never dragging back
                    // something he already dealt with.
                    if !rules.is_empty() {
                        let subj = RuleSubject {
                            from: &msg.from_address,
                            subject: &msg.subject,
                            preview: msg.snippet.as_deref().unwrap_or_default(),
                        };
                        if let Some(hit) = apply_rules(&rules, &subj) {
                            let _ = run_rule_action(conn, row_id, &hit);
                        }
                    }
                    out.new_rows.push((row_id, msg.subject.clone()));
                } else {
                    out.duplicates += 1;
                }
                done.push(pid);
            }
            Fetched::Gone | Fetched::Failed(GmailFailure::BadRequest) => done.push(pid),
            // Kept for the next run: nothing is wrong with the message.
            Fetched::Skipped => {}
            Fetched::Failed(f) if f.is_rate_limit() || f.stops_the_run() => {}
            // A failure counts against the MESSAGE only when Google answered
            // for others in the same batch - otherwise it was the connection
            // or Google, and counting it would push good mail aside every
            // time the laptop went offline for a few minutes.
            Fetched::Failed(GmailFailure::Network) => {}
            Fetched::Failed(_) if !answered => {}
            Fetched::Failed(f) => failed.push((pid, f.message())),
        }
    }
    dequeue(conn, &done)?;
    note_failures(conn, &failed, now)?;
    Ok(out)
}

/// A Discord message about a record Automatic created, sent once the run's
/// transaction is committed - never with it open.
type Notice = (DiscordDestination, String);

/// Only what marko has switched to `auto`, only for a proposal that is
/// already `ready`, and never twice - the 2.106.0 rules, unchanged. Each
/// record is made inside its own savepoint: one that fails half way leaves
/// nothing behind, and the rest of the run goes on.
fn run_automation(conn: &Connection, new_rows: &[(i64, String)]) -> Vec<Notice> {
    let mut notices = Vec::new();
    for (row_id, subject) in new_rows {
        let category: String = conn
            .query_row(
                "SELECT category FROM mail_messages WHERE id = ?1 AND created_id IS NULL",
                params![row_id],
                |r| r.get(0),
            )
            .unwrap_or_default();
        let Some(kind) = proposal_kind_for(&category) else {
            continue;
        };
        if automation_mode(conn, kind) != "auto" {
            continue;
        }
        if conn.execute_batch("SAVEPOINT mail_auto_create").is_err() {
            continue;
        }
        match create_from_mail_impl(conn, *row_id, "auto") {
            Ok((k, rid)) => {
                let _ = conn.execute_batch("RELEASE mail_auto_create");
                let dest = match k.as_str() {
                    "finance" => DiscordDestination::Finance,
                    "order" => DiscordDestination::Inventory,
                    _ => DiscordDestination::Sales,
                };
                notices.push((dest, format!("{k} #{rid} - {subject}")));
            }
            Err(e) => {
                let _ = conn.execute_batch("ROLLBACK TO mail_auto_create; RELEASE mail_auto_create");
                let _ = log_step(conn, *row_id, "auto_create", "skipped", Some(&e.to_string()));
            }
        }
    }
    notices
}

/// Labels exactly as Gmail has them now. A label deleted in Gmail leaves
/// the rail; its id stays on old messages, where it harms nothing.
fn save_labels(conn: &Connection, labels: &[(String, String, String)], now: &str) -> AppResult<()> {
    let mut stmt = conn.prepare(
        "INSERT INTO mail_labels (id, name, kind, updated_at, gmail_id) VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(id) DO UPDATE SET name = excluded.name, kind = excluded.kind,
                                       updated_at = excluded.updated_at, gmail_id = excluded.gmail_id",
    )?;
    let mut keep: Vec<String> = Vec::new();
    for (gmail_id, name, kind) in labels {
        let id = gmail_id.to_ascii_lowercase();
        stmt.execute(params![id, name, kind, now, gmail_id])?;
        keep.push(id);
    }
    if !keep.is_empty() {
        let marks = vec!["?"; keep.len()].join(",");
        let sql = format!("DELETE FROM mail_labels WHERE id NOT IN ({marks})");
        let refs: Vec<&dyn rusqlite::ToSql> = keep.iter().map(|k| k as &dyn rusqlite::ToSql).collect();
        conn.execute(&sql, refs.as_slice())?;
    }
    Ok(())
}

/// Rows from before 2.110.0 have no Finances flag yet; they get one a chunk
/// at a time, so the first start of a big mailbox is not one long freeze.
fn backfill_finance(conn: &Connection) -> AppResult<usize> {
    let rows: Vec<(i64, String, String, String, String)> = {
        let mut stmt = conn.prepare(
            "SELECT id, from_address, subject, category, IFNULL(labels, '') FROM mail_messages
              WHERE finance IS NULL LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![FINANCE_BACKFILL_CHUNK], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })?;
        rows.collect::<Result<_, _>>()?
    };
    let mut stmt = conn.prepare("UPDATE mail_messages SET finance = ?2 WHERE id = ?1")?;
    for (id, from, subject, category, labels) in &rows {
        stmt.execute(params![id, i64::from(finance_of(from, subject, category, labels))])?;
    }
    Ok(rows.len())
}

/// One folder walk, as the progress needs it.
struct WalkRow {
    title: String,
    tier: i64,
    done: bool,
    started: bool,
}

/// A lane's discovery from its walks: every one finished - "complete"; any
/// under way - "discovering"; none begun yet - "pending". A lane with no
/// walk at all (no labels) has nothing left to find.
fn lane_discovery<'a>(walks: impl Iterator<Item = &'a WalkRow>) -> String {
    let mut any = false;
    let mut all_done = true;
    let mut started = false;
    for w in walks {
        any = true;
        all_done &= w.done;
        started |= !w.done && w.started;
    }
    if !any || all_done {
        "complete".to_string()
    } else if started {
        "discovering".to_string()
    } else {
        "pending".to_string()
    }
}

/// Where the engine is, for the screen. Cheap enough for every run: the
/// queue is kept to a few hundred ids, the walks to one row per folder.
pub(crate) fn progress_of(conn: &Connection, now: DateTime<Utc>) -> MailSyncProgress {
    let now_s = now.to_rfc3339();
    // (queued, due now) per lane: 0 new, 1 starred, 2 labels, 3 backlog.
    let mut lanes = [(0i64, 0i64); 4];
    let mut first_due_tier: Option<i64> = None;
    if let Ok(mut stmt) = conn.prepare(
        "SELECT priority, COUNT(*),
                SUM(CASE WHEN next_attempt_at IS NULL OR next_attempt_at <= ?1 THEN 1 ELSE 0 END)
           FROM mail_fetch_queue GROUP BY priority ORDER BY priority",
    ) {
        if let Ok(rows) = stmt.query_map(params![now_s], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?))
        }) {
            for (tier, queued, due) in rows.flatten() {
                let lane = match tier {
                    t if t <= TIER_NEW => 0,
                    t if t <= TIER_STARRED => 1,
                    t if t <= TIER_LABELS => 2,
                    _ => 3,
                };
                lanes[lane].0 += queued;
                lanes[lane].1 += due;
                if due > 0 && first_due_tier.is_none() {
                    first_due_tier = Some(tier);
                }
            }
        }
    }
    let walks: Vec<WalkRow> = conn
        .prepare("SELECT title, tier, done, page_token IS NOT NULL OR pass_started_at IS NOT NULL FROM mail_sync_sections ORDER BY tier, title")
        .and_then(|mut s| {
            let rows = s.query_map([], |r| {
                Ok(WalkRow {
                    title: r.get(0)?,
                    tier: r.get(1)?,
                    done: r.get::<_, i64>(2)? != 0,
                    started: r.get::<_, i64>(3)? != 0,
                })
            })?;
            rows.collect::<Result<Vec<_>, _>>()
        })
        .unwrap_or_default();
    let (stored_total, last_success_at, anchored): (i64, Option<String>, bool) = (
        conn.query_row("SELECT COUNT(*) FROM mail_messages", [], |r| r.get(0)).unwrap_or(0),
        conn.query_row("SELECT last_sync_at FROM mail_sync_state WHERE id = 1", [], |r| r.get(0))
            .optional()
            .ok()
            .flatten()
            .flatten(),
        conn.query_row(
            "SELECT last_history_id IS NOT NULL FROM mail_sync_state WHERE id = 1",
            [],
            |r| r.get::<_, i64>(0),
        )
        .map(|v| v != 0)
        .unwrap_or(false),
    );

    // What is being worked on: the best lane with something to download now,
    // or with a folder still being walked - whichever comes first.
    let first_walk = walks.iter().find(|w| !w.done);
    let tier = match (first_due_tier, first_walk.map(|w| w.tier)) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    };
    let (phase, reading) = match tier {
        None => ("idle", None),
        Some(t) if t <= TIER_NEW => ("new", Some("New mail".to_string())),
        Some(t) if t <= TIER_STARRED => ("starred", Some("Starred".to_string())),
        // A label being walked is named; labelled mail being downloaded
        // may come from any of them, so it is just "Labels".
        Some(t) if t <= TIER_LABELS => {
            let walking = first_walk.filter(|w| w.tier == t).map(|w| w.title.clone());
            ("labels", Some(walking.unwrap_or_else(|| "Labels".to_string())))
        }
        // Each backlog lane is one folder.
        Some(t) => {
            let title = walks.iter().find(|w| w.tier == t).map(|w| w.title.clone());
            ("backlog", Some(title.unwrap_or_else(|| "History".to_string())))
        }
    };
    let lane = |i: usize, discovery: String| MailLaneProgress { waiting: lanes[i].0, discovery };
    MailSyncProgress {
        reading,
        waiting: lanes.iter().map(|l| l.0).sum(),
        folders_done: walks.iter().filter(|w| w.done).count() as i64,
        folders_total: walks.len() as i64,
        stored_total,
        phase: phase.to_string(),
        new_mail: lane(0, if anchored { "watching" } else { "pending" }.to_string()),
        starred: lane(1, lane_discovery(walks.iter().filter(|w| w.tier == TIER_STARRED))),
        labels: lane(2, lane_discovery(walks.iter().filter(|w| w.tier == TIER_LABELS))),
        backlog: lane(3, lane_discovery(walks.iter().filter(|w| w.tier >= TIER_BACKLOG))),
        due: lanes.iter().map(|l| l.1).sum(),
        retry_later: lanes.iter().map(|l| l.0 - l.1).sum(),
        last_success_at,
    }
}

fn record_failure(conn: &Connection, message: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE mail_sync_state
            SET sync_in_progress = 0, sync_started_at = NULL, last_error = ?1, last_error_at = ?2
          WHERE id = 1",
        params![safe_detail(message), Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

fn outcome(status: &str, message: String, at: &str, progress: MailSyncProgress) -> MailSyncResult {
    MailSyncResult {
        success: status == "synced",
        status: status.to_string(),
        message,
        synced_at: at.to_string(),
        more_waiting: progress.due > 0 || progress.folders_done < progress.folders_total,
        progress,
        ..Default::default()
    }
}

/// Gmail's change history can no longer be read from where this computer got
/// to: expired (404 - Gmail keeps about a week, and the app only runs while
/// it is open) or no longer accepted (400). A fresh starting point is taken
/// on the next run, and every folder is walked again from the top, so
/// whatever changed in between is found. Stored mail is not touched and costs
/// listing only; the queue is kept.
///
/// At most once an hour: only a request this app got wrong could fail like
/// this again straight away, and that must not keep sending every walk back
/// to the start. Returns whether the walks were sent back.
fn history_lost(conn: &Connection, now: DateTime<Utc>) -> AppResult<bool> {
    let now_s = now.to_rfc3339();
    conn.execute(
        "UPDATE mail_sync_state SET last_history_id = NULL, history_checked_at = ?1 WHERE id = 1",
        params![now_s],
    )?;
    let last: Option<String> = conn
        .query_row("SELECT walks_reset_at FROM mail_sync_state WHERE id = 1", [], |r| r.get(0))
        .optional()?
        .flatten();
    if !due(last.as_deref(), RESCAN_GUARD_SECS, now) {
        return Ok(false);
    }
    reset_walks(conn)?;
    conn.execute("UPDATE mail_sync_state SET walks_reset_at = ?1 WHERE id = 1", params![now_s])?;
    Ok(true)
}

/// One more run in a row that failed this way; returns the count BEFORE it.
fn bump_streak(conn: &Connection, column: &str) -> AppResult<u32> {
    let before: i64 = conn
        .query_row(&format!("SELECT {column} FROM mail_sync_state WHERE id = 1"), [], |r| r.get(0))
        .optional()?
        .unwrap_or(0);
    conn.execute(&format!("UPDATE mail_sync_state SET {column} = ?1 WHERE id = 1"), params![before + 1])?;
    Ok(before.clamp(0, 20) as u32)
}

/// What one run changed - all of it inside the caller's transaction - and
/// what is left to do once that is committed.
struct Applied {
    result: MailSyncResult,
    notices: Vec<Notice>,
    /// The retention purge ran, so freed space can be given back.
    reclaim: bool,
}

fn apply(conn: &Connection, plan: &Plan, net: Net, now: DateTime<Utc>) -> AppResult<Applied> {
    let now_s = now.to_rfc3339();
    let mut changed = false;

    if let Some(labels) = &net.labels {
        save_labels(conn, labels, &now_s)?;
        conn.execute("UPDATE mail_sync_state SET labels_checked_at = ?1 WHERE id = 1", params![now_s])?;
        changed = true;
    }
    if let Some(h) = &net.profile_history {
        // The checkpoint is taken BEFORE the first walk, so everything that
        // arrives while a long first read is under way is in the history.
        conn.execute(
            "UPDATE mail_sync_state SET last_history_id = ?1, history_checked_at = ?2 WHERE id = 1",
            params![h, now_s],
        )?;
    }
    let history_ran = net.history.is_some();
    let mut walks_reset = false;
    match &net.history {
        Some(Ok(read)) => {
            let (c, _) = apply_history(conn, &plan.account_email, &read.events, &now_s)?;
            changed |= c > 0;
            if let Some(h) = &read.checkpoint {
                conn.execute("UPDATE mail_sync_state SET last_history_id = ?1 WHERE id = 1", params![h])?;
            }
            conn.execute("UPDATE mail_sync_state SET history_checked_at = ?1 WHERE id = 1", params![now_s])?;
        }
        Some(Err(GmailFailure::NotFound | GmailFailure::BadRequest)) => {
            walks_reset = history_lost(conn, now)?;
        }
        _ => {}
    }
    let mut skipped_folder: Option<String> = None;
    // A page listed in the same run that sent every walk back to the top
    // belongs to the walk that was just thrown away; applying it would carry
    // that walk on from where it was.
    if !walks_reset {
        if let (Some((section, page_used)), Some(result)) = (&plan.listing, &net.listing) {
            match result {
                Ok((ids, next)) => {
                    changed |=
                        apply_listing(conn, &plan.account_email, section, page_used.as_deref(), ids, next.as_deref(), &now_s)?
                            > 0;
                }
                // A page token Gmail no longer accepts: that folder from the top.
                Err(GmailFailure::BadRequest | GmailFailure::NotFound) if page_used.is_some() => {
                    restart_walk(conn, &section.key)?
                }
                // The FIRST page refused: the folder itself is what Gmail will
                // not take (a label deleted a moment ago, a filter it cannot
                // read). Restarting would refuse again forever and hold up every
                // folder after it in the order - so it is set aside, and said.
                Err(GmailFailure::BadRequest | GmailFailure::NotFound) => {
                    skip_walk(conn, &section.key, &now_s)?;
                    skipped_folder = Some(section.title.clone());
                }
                Err(_) => {}
            }
        }
    }

    let stored = store_fetched(conn, plan, net.fetched, now)?;
    let notices = run_automation(conn, &stored.new_rows);
    backfill_finance(conn)?;
    if history_ran {
        purge_expired(conn)?;
    }
    let changed = changed || stored.stored_new > 0;
    let finish = |result: MailSyncResult| Applied { result, notices, reclaim: history_ran };

    // Something only marko can fix: said, and looked at again on his
    // interval.
    if let Some(stop) = &net.stop {
        if *stop == GmailFailure::Unauthorized {
            forget_access_token();
        }
        record_failure(conn, &stop.message())?;
        let mut r = outcome("failed", stop.message(), &now_s, progress_of(conn, now));
        r.stored_new = stored.stored_new;
        r.fetched = stored.fetched;
        r.next_delay_ms = (plan.interval_secs.max(60)) * 1000;
        r.changed = changed;
        return Ok(finish(r));
    }
    // Offline, or Google having a moment, and nothing at all came back:
    // said, and tried again after a pause that grows while it lasts - never
    // every quarter of a second.
    let reached = stored.fetched > 0
        || net.profile_history.is_some()
        || net.labels.is_some()
        || matches!(net.history, Some(Ok(_)))
        || matches!(net.listing, Some(Ok(_)));
    if !reached {
        if let Some(t) = &net.trouble {
            let streak = bump_streak(conn, "trouble_streak")?;
            record_failure(conn, &t.message())?;
            let mut r = outcome("failed", t.message(), &now_s, progress_of(conn, now));
            r.next_delay_ms = backoff_ms(TROUBLE_DELAY_MS, streak, TROUBLE_MAX_MS);
            r.changed = changed;
            return Ok(finish(r));
        }
    }
    let rate_streak = if net.rate_limited { Some(bump_streak(conn, "rate_limit_streak")?) } else { None };
    // A run Google turned away before anything came down is not a sync, so
    // "last synced" does not move for it.
    let synced = !net.rate_limited || stored.fetched > 0;
    conn.execute(
        "UPDATE mail_sync_state
            SET last_sync_at = CASE WHEN ?4 THEN ?1 ELSE last_sync_at END, last_sync_count = ?2,
                last_error = NULL, last_error_at = NULL, sync_in_progress = 0, sync_started_at = NULL,
                trouble_streak = 0, rate_limit_streak = CASE WHEN ?3 THEN rate_limit_streak ELSE 0 END
          WHERE id = 1",
        params![now_s, stored.stored_new, net.rate_limited, synced],
    )?;
    let progress = progress_of(conn, now);
    let more = progress.due > 0 || progress.folders_done < progress.folders_total;
    let message = if net.rate_limited {
        GmailFailure::RateLimited { retry_after_secs: None }.message()
    } else if let Some(title) = &skipped_folder {
        format!("Gmail would not list {title}, so it was set aside. If that is your own filter, check it in Mail -> Settings.")
    } else if more {
        format!(
            "{} new - reading {} - {} waiting",
            stored.stored_new,
            progress.reading.as_deref().unwrap_or("new mail"),
            progress.waiting
        )
    } else {
        format!("{} new. Everything is read.", stored.stored_new)
    };
    let next_delay_ms = if let Some(streak) = rate_streak {
        let google = net.retry_after_secs.map_or(0, |s| (s as i64).saturating_mul(1000));
        backoff_ms(RATE_LIMIT_PAUSE_MS, streak, RATE_LIMIT_MAX_MS).max(google)
    } else if more {
        CHAIN_DELAY_MS
    } else {
        // Everything read: new mail on his interval - sooner when a message
        // that failed is due again before it.
        let every = plan.interval_secs * 1000;
        match next_retry_in_ms(conn, now)? {
            Some(ms) if every > 0 => every.min(ms.max(CHAIN_DELAY_MS)),
            _ => every,
        }
    };
    let mut r = outcome("synced", message, &now_s, progress);
    r.fetched = stored.fetched;
    r.stored_new = stored.stored_new;
    r.duplicates_ignored = stored.duplicates;
    r.review_required = stored.review;
    r.rate_limited = net.rate_limited;
    r.next_delay_ms = next_delay_ms;
    r.changed = changed;
    Ok(finish(r))
}

fn skipped(message: &str, at: &str, progress: MailSyncProgress) -> MailSyncResult {
    let mut r = outcome("skipped", message.to_string(), at, progress);
    r.more_waiting = false;
    r
}

/// Runs `apply` in ONE transaction - a crash or an error half way leaves the
/// database exactly as the run found it, and the next run does the same work
/// again - then, with it committed, gives back freed space and sends the
/// Discord messages for anything Automatic created.
fn apply_committed(conn: &Connection, plan: &Plan, net: Net, now: DateTime<Utc>) -> AppResult<MailSyncResult> {
    let Applied { result, notices, reclaim } = {
        let tx = conn.unchecked_transaction()?;
        let applied = apply(&tx, plan, net, now)?;
        tx.commit()?;
        applied
    };
    if reclaim {
        crate::mail_store::reclaim(conn);
    }
    for (dest, text) in notices {
        crate::commands::notifications::notify_discord(conn, dest, "Created from email", &text, &[]);
    }
    Ok(result)
}

/// One run of the engine. See the module comment for what it does.
pub(crate) async fn run_once(state: &AppState) -> AppResult<MailSyncResult> {
    let now = Utc::now();
    let at = now.to_rfc3339();
    let Some(_guard) = RunGuard::claim() else {
        let conn = state.db.lock().unwrap();
        return Ok(skipped("A sync is already running.", &at, progress_of(&conn, now)));
    };
    let plan = {
        let conn = state.db.lock().unwrap();
        match make_plan(&conn, now)? {
            Ok(p) => {
                conn.execute(
                    "UPDATE mail_sync_state SET sync_in_progress = 1, sync_started_at = ?1 WHERE id = 1",
                    params![at],
                )?;
                p
            }
            Err(reason) => return Ok(skipped(reason, &at, progress_of(&conn, now))),
        }
    };
    let Some(client) = google_oauth::embedded_oauth_client() else {
        let conn = state.db.lock().unwrap();
        let msg = "This build has no Google client embedded.";
        record_failure(&conn, msg)?;
        return Ok(outcome("failed", msg.to_string(), &at, progress_of(&conn, now)));
    };
    let joined = tauri::async_runtime::spawn_blocking(move || {
        let net = access_token(&client, &plan.refresh_token)
            .and_then(|token| Ok((token, gmail_client()?)))
            .and_then(|(token, http)| network(&plan, &http, &token));
        (plan, net)
    })
    .await;
    let conn = state.db.lock().unwrap();
    match joined {
        Ok((plan, Ok(net))) => match apply_committed(&conn, &plan, net, now) {
            Ok(r) => Ok(r),
            Err(e) => {
                // Rolled back whole: nothing of this run is half-written,
                // and the next run does the same work again.
                let msg = safe_detail(&e.to_string());
                record_failure(&conn, &msg)?;
                let mut r = outcome("failed", msg, &at, progress_of(&conn, now));
                r.next_delay_ms = TROUBLE_DELAY_MS;
                Ok(r)
            }
        },
        Ok((plan, Err(e))) => {
            let msg = safe_detail(&e.to_string());
            record_failure(&conn, &msg)?;
            let mut r = outcome("failed", msg, &at, progress_of(&conn, now));
            r.next_delay_ms = plan.interval_secs.max(60) * 1000;
            Ok(r)
        }
        Err(_) => {
            let msg = "The sync did not finish cleanly.";
            record_failure(&conn, msg)?;
            let mut r = outcome("failed", msg.to_string(), &at, progress_of(&conn, now));
            r.next_delay_ms = TROUBLE_DELAY_MS;
            Ok(r)
        }
    }
}

/// A new mailbox, or the same one connected again: nothing about where the
/// old walks had got to applies.
pub(crate) fn reset_engine(conn: &Connection) -> AppResult<()> {
    reset_walks(conn)?;
    conn.execute("DELETE FROM mail_fetch_queue", [])?;
    conn.execute(
        "UPDATE mail_sync_state
            SET last_history_id = NULL, history_checked_at = NULL, labels_checked_at = NULL,
                walks_reset_at = NULL, rate_limit_streak = 0, trouble_streak = 0
          WHERE id = 1",
        [],
    )?;
    forget_access_token();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::sheets_sync::set_setting;
    use crate::db::test_conn;
    use serde_json::json;
    use std::io::{Read, Write};
    use std::sync::Arc;

    const NOW: &str = "2026-10-18T12:00:00+00:00";

    fn at(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    fn now() -> DateTime<Utc> {
        at(NOW)
    }

    fn later(secs: i64) -> DateTime<Utc> {
        now() + chrono::Duration::seconds(secs)
    }

    fn insert(conn: &Connection, pid: &str, labels: &str, starred: bool, first_seen: &str) -> i64 {
        conn.execute(
            "INSERT INTO mail_messages (provider, provider_message_id, account_email, from_address, subject,
                                        received_at, first_seen_at, updated_at, labels, is_starred)
             VALUES ('gmail', ?1, 'me@gmail.com', 'a@b.com', 's', '2026-10-01T00:00:00Z', ?2, ?2, ?3, ?4)",
            params![pid, first_seen, labels, i64::from(starred)],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    fn labels_of(conn: &Connection, pid: &str) -> (String, i64) {
        conn.query_row(
            "SELECT IFNULL(labels, ''), is_starred FROM mail_messages WHERE provider_message_id = ?1",
            params![pid],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap()
    }

    fn rows_for(conn: &Connection, pid: &str) -> i64 {
        conn.query_row("SELECT COUNT(*) FROM mail_messages WHERE provider_message_id = ?1", params![pid], |r| r.get(0))
            .unwrap()
    }

    /// (priority, source, status, attempts) of one queued id.
    fn queued(conn: &Connection, pid: &str) -> Option<(i64, String, String, i64)> {
        conn.query_row(
            "SELECT priority, source, status, attempts FROM mail_fetch_queue WHERE provider_message_id = ?1",
            params![pid],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()
        .unwrap()
    }

    fn label(id: &str, name: &str) -> UserLabel {
        UserLabel { gmail_id: id.to_string(), name: name.to_string() }
    }

    fn section(key: &str, labels: &[UserLabel]) -> Section {
        plan_sections("all", &["viagogo.com".to_string()], labels, "").into_iter().find(|s| s.key == key).unwrap()
    }

    fn starred_section() -> Section {
        section("starred", &[])
    }

    fn ids(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    fn fetched_as(id: &str, from: &str, subject: &str, labels: &str) -> FetchedMessage {
        FetchedMessage {
            provider_message_id: id.into(),
            provider_thread_id: None,
            from_address: from.into(),
            from_name: None,
            subject: subject.into(),
            snippet: Some(subject.into()),
            received_at: NOW.into(),
            body_text: String::new(),
            is_unread: true,
            labels: labels.into(),
            is_starred: labels.split(',').any(|l| l == "starred"),
            from_raw: from.into(),
            to_address: None,
            reply_to: None,
        }
    }

    fn fetched(from: &str, subject: &str, labels: &str) -> FetchedMessage {
        fetched_as("x", from, subject, labels)
    }

    fn got(id: &str, labels: &str) -> (String, Fetched) {
        let m = fetched_as(id, "a@b.com", "hello", labels);
        let c = classify_fetched(&m);
        (id.to_string(), Fetched::Got(Box::new(m), c))
    }

    fn plan_with(batch: &[&str]) -> Plan {
        Plan {
            refresh_token: "r".into(),
            account_email: "me@gmail.com".into(),
            known_scope: false,
            senders: Vec::new(),
            need_profile: false,
            labels_due: false,
            history_from: None,
            listing: None,
            batch: ids(batch),
            interval_secs: 60,
            api: GMAIL_API.to_string(),
        }
    }

    fn apply_now(conn: &Connection, plan: &Plan, net: Net, when: DateTime<Utc>) -> MailSyncResult {
        apply_committed(conn, plan, net, when).unwrap()
    }

    // ---- the lanes and the folders ----------------------------------------

    #[test]
    fn folders_are_walked_starred_then_labels_then_the_backlog() {
        let senders = vec!["viagogo.com".to_string()];
        let labels = vec![label("Label_2", "PULLS"), label("Label_1", "OASIS")];
        let keys: Vec<String> = plan_sections("all", &senders, &labels, "").into_iter().map(|s| s.key).collect();
        assert_eq!(
            keys,
            vec![
                "starred", "label:label_1", "label:label_2", "important", "finances", "inbox", "promotions",
                "spam", "trash"
            ]
        );
        let tiers: Vec<i64> = plan_sections("all", &senders, &labels, "").into_iter().map(|s| s.tier).collect();
        assert!(tiers.windows(2).all(|w| w[0] <= w[1]), "the order IS the lanes: {tiers:?}");
        assert!(TIER_NEW < TIER_STARRED && TIER_STARRED < TIER_LABELS && TIER_LABELS < TIER_BACKLOG);
        assert_eq!(TIER_BACKLOG, TIER_IMPORTANT, "Important is history, not a priority lane");
    }

    #[test]
    fn starred_and_labels_have_no_date_and_no_filter_in_them() {
        let labels = vec![label("Label_1", "OASIS")];
        for s in plan_sections("all", &["viagogo.com".to_string()], &labels, "-from:x.com has:attachment") {
            if s.key == "starred" || s.key.starts_with("label:") {
                assert!(s.q.is_none(), "{} must be a plain label walk: {:?}", s.key, s.q);
                assert!(s.flag.is_some());
                let url = s.list_url(GMAIL_API, None);
                assert!(!url.contains("after") && !url.contains("q="), "{url}");
            }
        }
        let starred = starred_section().list_url(GMAIL_API, None);
        assert!(starred.contains("labelIds=STARRED"), "Gmail's own starred state: {starred}");
        let oasis = section("label:label_1", &labels).list_url(GMAIL_API, None);
        assert!(oasis.contains("labelIds=Label_1"), "Gmail's id, case and all: {oasis}");
    }

    #[test]
    fn his_own_filter_narrows_the_inbox_and_nothing_else() {
        let sections = plan_sections("all", &[], &[], "has:attachment");
        let with_it: Vec<&str> = sections
            .iter()
            .filter(|s| s.q.as_deref().is_some_and(|q| q.contains("has:attachment")))
            .map(|s| s.key.as_str())
            .collect();
        assert_eq!(with_it, vec!["inbox"]);
    }

    #[test]
    fn the_inbox_is_everything_received_except_promotions() {
        let inbox = plan_sections("all", &[], &[], "").into_iter().find(|s| s.key == "inbox").unwrap();
        let q = inbox.q.unwrap();
        for part in ["-in:sent", "-in:drafts", "-in:chats", "-category:promotions"] {
            assert!(q.contains(part), "{q}");
        }
        assert!(!q.contains("after:"), "no date limit - newest to oldest, all of it: {q}");
        assert!(!q.contains("in:inbox"), "archived mail is history too: {q}");
    }

    #[test]
    fn the_known_scope_reads_only_the_mail_that_matters() {
        let keys: Vec<String> = plan_sections("known", &["viagogo.com".to_string()], &[label("Label_1", "A")], "")
            .into_iter()
            .map(|s| s.key)
            .collect();
        assert_eq!(keys, vec!["starred", "label:label_1", "important", "finances"]);
    }

    #[test]
    fn an_emptied_sender_list_still_means_the_whole_mailbox() {
        let keys: Vec<String> = plan_sections("known", &[], &[], "").into_iter().map(|s| s.key).collect();
        assert!(keys.contains(&"inbox".to_string()), "{keys:?}");
        assert!(!keys.contains(&"important".to_string()), "no senders, no Important walk: {keys:?}");
    }

    #[test]
    fn spam_and_trash_are_listed_with_spam_and_trash_included() {
        for s in plan_sections("all", &[], &[], "") {
            let url = s.list_url(GMAIL_API, None);
            if s.key == "spam" || s.key == "trash" {
                assert!(url.contains("includeSpamTrash=true"), "{url}");
            } else {
                assert!(!url.contains("includeSpamTrash"), "{url}");
            }
        }
    }

    #[test]
    fn a_query_is_encoded_for_the_url() {
        let s = plan_sections("all", &[], &[], "").into_iter().find(|s| s.key == "finances").unwrap();
        let url = s.list_url(GMAIL_API, Some("tok en"));
        assert!(url.starts_with(GMAIL_API), "{url}");
        assert!(!url.contains(' '), "{url}");
        assert!(url.contains("pageToken=tok+en"), "{url}");
        assert!(url.contains("from:paypal.com"), "{url}");
        assert!(url.contains("maxResults=500"), "Gmail's biggest page: {url}");
    }

    // ---- Google's answers -------------------------------------------------

    #[test]
    fn a_403_rate_limit_is_a_rate_limit_not_a_refusal() {
        let body = r#"{"error":{"code":403,"message":"User-rate limit exceeded.","errors":[{"domain":"usageLimits","reason":"userRateLimitExceeded"}],"status":"PERMISSION_DENIED"}}"#;
        let f = failure_from(403, body, None);
        assert_eq!(f, GmailFailure::RateLimited { retry_after_secs: None });
        assert!(!f.stops_the_run());
        assert!(!f.message().contains("refused"), "{}", f.message());
        let other = r#"{"error":{"errors":[{"reason":"rateLimitExceeded"}]}}"#;
        assert!(failure_from(403, other, Some(7)).is_rate_limit());
        assert_eq!(failure_from(429, "", Some(7)), GmailFailure::RateLimited { retry_after_secs: Some(7) });
        assert_eq!(failure_from(429, "", Some(7)).retry_after_secs(), Some(7));
    }

    #[test]
    fn the_three_kinds_of_real_403_are_told_apart() {
        let scope = r#"{"error":{"code":403,"message":"Request had insufficient authentication scopes.","errors":[{"reason":"insufficientPermissions"}],"details":[{"reason":"ACCESS_TOKEN_SCOPE_INSUFFICIENT"}]}}"#;
        assert_eq!(failure_from(403, scope, None), GmailFailure::NoPermission);
        // Note `usageLimits` as the domain here too - which is why the
        // REASON is read and not the domain.
        let off = r#"{"error":{"code":403,"message":"Gmail API has not been used in project 1 before or it is disabled.","errors":[{"domain":"usageLimits","reason":"accessNotConfigured"}],"details":[{"reason":"SERVICE_DISABLED"}]}}"#;
        assert_eq!(failure_from(403, off, None), GmailFailure::ApiDisabled);
        assert_eq!(failure_from(403, "{}", None), GmailFailure::Forbidden);
        for f in [GmailFailure::NoPermission, GmailFailure::ApiDisabled, GmailFailure::Forbidden, GmailFailure::Unauthorized] {
            assert!(f.stops_the_run(), "{f:?}");
        }
    }

    #[test]
    fn the_other_answers_map_to_what_they_mean() {
        assert_eq!(failure_from(401, "", None), GmailFailure::Unauthorized);
        assert_eq!(failure_from(404, "", None), GmailFailure::NotFound);
        assert_eq!(failure_from(400, "", None), GmailFailure::BadRequest);
        assert_eq!(failure_from(503, "", None), GmailFailure::Server(503));
    }

    #[test]
    fn no_message_ever_carries_googles_own_text() {
        let body = r#"{"error":{"message":"secret-ish detail ya29.TOKEN"}}"#;
        for code in [400u16, 401, 403, 404, 429, 500] {
            let msg = failure_from(code, body, None).message();
            assert!(!msg.contains("ya29"), "{msg}");
            assert!(!msg.contains("secret-ish"), "{msg}");
        }
    }

    #[test]
    fn backoff_doubles_stays_under_its_cap_and_is_never_shortened_by_more_than_a_quarter() {
        for _ in 0..200 {
            let a = backoff_ms(1000, 0, 60_000);
            let b = backoff_ms(1000, 3, 60_000);
            let c = backoff_ms(1000, 30, 60_000);
            assert!((750..=1000).contains(&a), "{a}");
            assert!((6000..=8000).contains(&b), "{b}");
            assert!((45_000..=60_000).contains(&c), "{c}");
        }
        // Jitter, not one fixed number for everyone.
        let spread: HashSet<i64> = (0..50).map(|_| backoff_ms(20_000, 0, 60_000)).collect();
        assert!(spread.len() > 1);
    }

    // ---- the queue --------------------------------------------------------

    #[test]
    fn the_queue_skips_what_is_stored_or_deleted_and_keeps_the_best_lane() {
        let conn = test_conn();
        insert(&conn, "stored", "inbox", false, NOW);
        conn.execute("INSERT INTO mail_deleted (provider_message_id, deleted_at) VALUES ('deleted', 'x')", [])
            .unwrap();
        enqueue(&conn, "me", &ids(&["i1", "i2", "stored", "deleted"]), TIER_INBOX, "inbox", NOW).unwrap();
        enqueue(&conn, "me", &ids(&["s1", "s2"]), TIER_STARRED, "starred", NOW).unwrap();
        // The same id from a better folder keeps the better place...
        enqueue(&conn, "me", &ids(&["i2"]), TIER_STARRED, "starred", NOW).unwrap();
        // ...and a worse one never takes it away.
        assert_eq!(enqueue(&conn, "me", &ids(&["s1"]), TIER_INBOX, "inbox", NOW).unwrap(), 0);
        assert_eq!(peek(&conn, 10, NOW).unwrap(), vec!["i2", "s1", "s2", "i1"]);
        assert_eq!(queue_depth(&conn), 4);
        assert_eq!(queued(&conn, "s1").unwrap().1, "starred");
    }

    #[test]
    fn inside_one_folder_the_newest_comes_first() {
        let conn = test_conn();
        // Gmail lists newest first, page after page.
        enqueue(&conn, "me", &ids(&["newest", "newer"]), TIER_INBOX, "inbox", NOW).unwrap();
        enqueue(&conn, "me", &ids(&["older", "oldest"]), TIER_INBOX, "inbox", NOW).unwrap();
        assert_eq!(peek(&conn, 10, NOW).unwrap(), vec!["newest", "newer", "older", "oldest"]);
        // The same queue gives the same batch every time.
        assert_eq!(peek(&conn, 3, NOW).unwrap(), peek(&conn, 3, NOW).unwrap());
    }

    /// Scenario 1.
    #[test]
    fn new_mail_comes_before_ordinary_history() {
        let conn = test_conn();
        let backlog: Vec<String> = (0..300).map(|i| format!("old{i}")).collect();
        enqueue(&conn, "me", &backlog, TIER_INBOX, "inbox", NOW).unwrap();
        let ev = vec![HistoryEvent::Added { id: "fresh".into(), labels: vec!["inbox".into(), "unread".into()] }];
        apply_history(&conn, "me", &ev, NOW).unwrap();
        let batch = peek(&conn, FETCH_BATCH, NOW).unwrap();
        assert_eq!(batch[0], "fresh", "found last, downloaded first");
        assert_eq!(queued(&conn, "fresh").unwrap().0, TIER_NEW);
    }

    /// Scenario 2.
    #[test]
    fn a_starred_mail_from_last_year_comes_before_yesterdays_ordinary_mail() {
        let conn = test_conn();
        let inbox = section("inbox", &[]);
        let starred = starred_section();
        sync_section_rows(&conn, &[starred.clone(), inbox.clone()], NOW).unwrap();
        // The inbox walk happens to meet yesterday's mail first...
        apply_listing(&conn, "me", &inbox, None, &ids(&["yesterday", "last_week"]), Some("p2"), NOW).unwrap();
        // ...and Starred meets the year-old one after it.
        apply_listing(&conn, "me", &starred, None, &ids(&["last_year"]), None, NOW).unwrap();
        assert_eq!(peek(&conn, 10, NOW).unwrap(), vec!["last_year", "yesterday", "last_week"]);
        assert_eq!(queued(&conn, "last_year").unwrap().0, TIER_STARRED);
    }

    /// Scenario 3.
    #[test]
    fn a_labelled_mail_from_last_year_comes_before_ordinary_history() {
        let conn = test_conn();
        let labels = vec![label("Label_7", "NFL MEXICO")];
        let inbox = section("inbox", &labels);
        let nfl = section("label:label_7", &labels);
        sync_section_rows(&conn, &[nfl.clone(), inbox.clone()], NOW).unwrap();
        apply_listing(&conn, "me", &inbox, None, &ids(&["yesterday"]), Some("p2"), NOW).unwrap();
        apply_listing(&conn, "me", &nfl, None, &ids(&["archived_last_year"]), None, NOW).unwrap();
        assert_eq!(peek(&conn, 10, NOW).unwrap(), vec!["archived_last_year", "yesterday"]);
        let (tier, source, ..) = queued(&conn, "archived_last_year").unwrap();
        assert_eq!((tier, source.as_str()), (TIER_LABELS, "label:label_7"));
    }

    /// Scenario 4.
    #[test]
    fn a_message_in_several_labels_is_queued_once_and_stored_once_with_all_of_them() {
        let conn = test_conn();
        let labels = vec![label("Label_1", "A"), label("Label_2", "B")];
        let a = section("label:label_1", &labels);
        let b = section("label:label_2", &labels);
        sync_section_rows(&conn, &[a.clone(), b.clone()], NOW).unwrap();
        apply_listing(&conn, "me", &a, None, &ids(&["m"]), Some("p2"), NOW).unwrap();
        apply_listing(&conn, "me", &b, None, &ids(&["m"]), Some("p2"), NOW).unwrap();
        assert_eq!(queue_depth(&conn), 1);

        let stored = store_fetched(&conn, &plan_with(&["m"]), vec![got("m", "inbox,label_1,label_2")], now()).unwrap();
        assert_eq!(stored.stored_new, 1);
        assert_eq!(queue_depth(&conn), 0);
        // Both walks meet it again: still one row, each label once.
        apply_listing(&conn, "me", &a, Some("p2"), &ids(&["m"]), None, NOW).unwrap();
        apply_listing(&conn, "me", &b, Some("p2"), &ids(&["m"]), None, NOW).unwrap();
        assert_eq!(rows_for(&conn, "m"), 1);
        let (labels, _) = labels_of(&conn, "m");
        assert_eq!(labels, "inbox,label_1,label_2");
        assert_eq!(queue_depth(&conn), 0, "stored mail is never queued again");
    }

    /// Scenario 5 (and see the end-to-end run below, which counts requests).
    #[test]
    fn starred_and_labelled_is_queued_once_in_the_better_lane() {
        let conn = test_conn();
        let labels = vec![label("Label_1", "A")];
        let starred = starred_section();
        let a = section("label:label_1", &labels);
        sync_section_rows(&conn, &[starred.clone(), a.clone()], NOW).unwrap();
        apply_listing(&conn, "me", &a, None, &ids(&["both"]), None, NOW).unwrap();
        apply_listing(&conn, "me", &starred, None, &ids(&["both"]), None, NOW).unwrap();
        assert_eq!(queue_depth(&conn), 1);
        assert_eq!(queued(&conn, "both").unwrap().0, TIER_STARRED);
        assert_eq!(peek(&conn, 10, NOW).unwrap(), vec!["both"], "offered once, so fetched once");
    }

    /// Scenario 6.
    #[test]
    fn a_queued_history_message_moves_up_when_starred_finds_it() {
        let conn = test_conn();
        let inbox = section("inbox", &[]);
        let starred = starred_section();
        sync_section_rows(&conn, &[starred.clone(), inbox.clone()], NOW).unwrap();
        apply_listing(&conn, "me", &inbox, None, &ids(&["m", "other"]), Some("p2"), NOW).unwrap();
        assert_eq!(queued(&conn, "m").unwrap().0, TIER_INBOX);
        apply_listing(&conn, "me", &starred, None, &ids(&["m"]), None, "2026-10-18T12:00:05+00:00").unwrap();
        let (tier, source, status, attempts) = queued(&conn, "m").unwrap();
        assert_eq!((tier, source.as_str(), status.as_str(), attempts), (TIER_STARRED, "starred", "queued", 0));
        assert_eq!(queue_depth(&conn), 2, "moved, not copied");
        // A message starred in Gmail while it waits moves up the same way.
        let ev = vec![HistoryEvent::LabelsAdded { id: "other".into(), labels: vec!["starred".into()] }];
        apply_history(&conn, "me", &ev, NOW).unwrap();
        assert_eq!(queued(&conn, "other").unwrap().0, TIER_STARRED);
        assert_eq!(queued(&conn, "other").unwrap().1, "history:starred");
    }

    /// Scenario 7.
    #[test]
    fn new_mail_is_downloaded_while_a_long_backlog_is_being_read() {
        let conn = test_conn();
        let backlog: Vec<String> = (0..500).map(|i| format!("h{i}")).collect();
        enqueue(&conn, "me", &backlog, TIER_INBOX, "inbox", NOW).unwrap();
        // A first batch of history is downloaded...
        let first = peek(&conn, FETCH_BATCH, NOW).unwrap();
        let got_first: Vec<_> = first.iter().map(|id| got(id, "inbox")).collect();
        store_fetched(&conn, &plan_with(&[]), got_first, now()).unwrap();
        // ...then new mail arrives, half way through the backlog.
        let ev = vec![HistoryEvent::Added { id: "arrived".into(), labels: vec!["inbox".into()] }];
        apply_history(&conn, "me", &ev, NOW).unwrap();
        let next = peek(&conn, FETCH_BATCH, NOW).unwrap();
        assert_eq!(next[0], "arrived", "the very next batch, not after 460 old ones");
        assert_eq!(queue_depth(&conn), 461);
    }

    /// Scenario 12.
    #[test]
    fn a_failing_message_steps_aside_and_is_never_dropped() {
        let conn = test_conn();
        enqueue(&conn, "me", &ids(&["bad", "good1", "good2"]), TIER_STARRED, "starred", NOW).unwrap();
        let plan = plan_with(&["bad", "good1", "good2"]);
        let batch = vec![
            ("bad".to_string(), Fetched::Failed(GmailFailure::Server(500))),
            got("good1", "inbox,starred"),
            got("good2", "inbox,starred"),
        ];
        store_fetched(&conn, &plan, batch, now()).unwrap();
        let (_, _, status, attempts) = queued(&conn, "bad").unwrap();
        assert_eq!((status.as_str(), attempts), ("retrying", 1));
        let last_error: String = conn
            .query_row("SELECT last_error FROM mail_fetch_queue WHERE provider_message_id = 'bad'", [], |r| r.get(0))
            .unwrap();
        assert!(last_error.contains("server error"), "{last_error}");
        // Out of the way at once: the next batch does not wait for it...
        enqueue(&conn, "me", &ids(&["good3"]), TIER_INBOX, "inbox", NOW).unwrap();
        assert_eq!(peek(&conn, 10, NOW).unwrap(), vec!["good3"]);
        // ...and back when its time comes.
        let later_s = later(60).to_rfc3339();
        assert_eq!(peek(&conn, 10, &later_s).unwrap(), vec!["bad", "good3"]);

        // Failing every time: parked, still queued, tried every few hours.
        let mut when = later(60);
        for _ in 1..MAX_ATTEMPTS {
            let batch = vec![("bad".to_string(), Fetched::Failed(GmailFailure::Server(500))), got("x", "inbox")];
            store_fetched(&conn, &plan, batch, when).unwrap();
            when += chrono::Duration::hours(7);
        }
        let (_, _, status, attempts) = queued(&conn, "bad").unwrap();
        assert_eq!((status.as_str(), attempts), ("parked", MAX_ATTEMPTS));
        let p = progress_of(&conn, when - chrono::Duration::hours(7));
        assert_eq!(p.retry_later, 1);
        assert!(peek(&conn, 10, &(when + chrono::Duration::hours(6)).to_rfc3339()).unwrap().contains(&"bad".to_string()));
    }

    #[test]
    fn nothing_but_messages_waiting_to_be_retried_does_not_keep_runs_spinning() {
        let conn = test_conn();
        let sections = plan_sections("all", &[], &[], "");
        sync_section_rows(&conn, &sections, NOW).unwrap();
        conn.execute("UPDATE mail_sync_sections SET done = 1", []).unwrap();
        enqueue(&conn, "me", &ids(&["bad"]), TIER_INBOX, "inbox", NOW).unwrap();
        let batch = vec![("bad".to_string(), Fetched::Failed(GmailFailure::Server(500))), got("ok", "inbox")];
        let net = Net { fetched: batch, ..Default::default() };
        let r = apply_now(&conn, &plan_with(&["bad", "ok"]), net, now());
        assert_eq!(r.status, "synced");
        assert!(!r.more_waiting, "only a message waiting for its retry time is left");
        assert!(r.next_delay_ms > CHAIN_DELAY_MS && r.next_delay_ms <= 60_000, "{}", r.next_delay_ms);
        assert_eq!(r.progress.due, 0);
        assert_eq!(r.progress.retry_later, 1);
    }

    #[test]
    fn an_hour_offline_pushes_nothing_aside() {
        let conn = test_conn();
        enqueue(&conn, "me", &ids(&["a", "b"]), TIER_STARRED, "starred", NOW).unwrap();
        let plan = plan_with(&["a", "b"]);
        for _ in 0..MAX_ATTEMPTS * 3 {
            let got = vec![
                ("a".to_string(), Fetched::Failed(GmailFailure::Network)),
                ("b".to_string(), Fetched::Skipped),
            ];
            store_fetched(&conn, &plan, got, now()).unwrap();
        }
        assert_eq!(peek(&conn, 10, NOW).unwrap(), vec!["a", "b"]);
        assert_eq!(queued(&conn, "a").unwrap().3, 0);
    }

    #[test]
    fn a_google_error_counts_against_a_message_only_while_google_answers_for_others() {
        let conn = test_conn();
        enqueue(&conn, "me", &ids(&["bad", "good"]), TIER_INBOX, "inbox", NOW).unwrap();
        let plan = plan_with(&["bad", "good"]);
        // Google failing for everything: not this message's fault.
        for _ in 0..MAX_ATTEMPTS {
            store_fetched(&conn, &plan, vec![("bad".into(), Fetched::Failed(GmailFailure::Server(503)))], now())
                .unwrap();
        }
        assert_eq!(queued(&conn, "bad").unwrap().3, 0);
        // Google fine for the other one: this message is the problem.
        store_fetched(
            &conn,
            &plan,
            vec![("bad".into(), Fetched::Failed(GmailFailure::Server(500))), got("good", "inbox")],
            now(),
        )
        .unwrap();
        assert_eq!(queued(&conn, "bad").unwrap().3, 1);
        assert!(queued(&conn, "good").is_none(), "the one that came down left the queue");
    }

    #[test]
    fn offline_says_so_and_waits_longer_each_time_instead_of_spinning() {
        let conn = test_conn();
        sync_section_rows(&conn, &plan_sections("all", &[], &[], ""), NOW).unwrap();
        enqueue(&conn, "me", &ids(&["a"]), TIER_STARRED, "starred", NOW).unwrap();
        let offline = || Net {
            fetched: vec![("a".into(), Fetched::Failed(GmailFailure::Network))],
            trouble: Some(GmailFailure::Network),
            ..Default::default()
        };
        let first = apply_now(&conn, &plan_with(&["a"]), offline(), now());
        assert_eq!(first.status, "failed");
        assert!(first.message.contains("Could not reach Gmail"), "{}", first.message);
        assert!((TROUBLE_DELAY_MS * 3 / 4..=TROUBLE_DELAY_MS).contains(&first.next_delay_ms), "{}", first.next_delay_ms);
        let second = apply_now(&conn, &plan_with(&["a"]), offline(), later(30));
        assert!(second.next_delay_ms > first.next_delay_ms, "{} then {}", first.next_delay_ms, second.next_delay_ms);
        for i in 0..10 {
            let r = apply_now(&conn, &plan_with(&["a"]), offline(), later(60 + i));
            assert!(r.next_delay_ms <= TROUBLE_MAX_MS, "{}", r.next_delay_ms);
        }
        assert_eq!(peek(&conn, 10, NOW).unwrap(), vec!["a"], "still queued");

        // A page listed before the connection went: that counts as reaching
        // Gmail, so the next run follows straight away - and the streak ends.
        let net = Net {
            listing: Some(Ok((ids(&["n1"]), Some("p2".into())))),
            fetched: vec![("a".into(), Fetched::Failed(GmailFailure::Network))],
            trouble: Some(GmailFailure::Network),
            ..Default::default()
        };
        let mut plan = plan_with(&["a"]);
        plan.listing = Some((starred_section(), None));
        let r = apply_now(&conn, &plan, net, later(200));
        assert_eq!(r.status, "synced");
        assert_eq!(r.next_delay_ms, CHAIN_DELAY_MS);
        let streak: i64 =
            conn.query_row("SELECT trouble_streak FROM mail_sync_state WHERE id = 1", [], |r| r.get(0)).unwrap();
        assert_eq!(streak, 0);
    }

    // ---- folder walks -----------------------------------------------------

    #[test]
    fn walking_starred_marks_mail_that_was_stored_before_it_was_starred() {
        let conn = test_conn();
        insert(&conn, "old", "inbox", false, "2026-01-01T00:00:00+00:00");
        let s = starred_section();
        sync_section_rows(&conn, &[s.clone()], NOW).unwrap();
        apply_listing(&conn, "me", &s, None, &ids(&["old", "new"]), Some("p2"), NOW).unwrap();
        let (labels, starred) = labels_of(&conn, "old");
        assert_eq!(starred, 1);
        assert!(labels.split(',').any(|l| l == "starred"), "{labels}");
        // Only what is missing is queued: the stored one is not downloaded again.
        assert_eq!(peek(&conn, 10, NOW).unwrap(), vec!["new"]);
    }

    #[test]
    fn the_end_of_a_walk_takes_off_a_star_gmail_no_longer_has() {
        let conn = test_conn();
        let before = "2026-01-01T00:00:00+00:00";
        insert(&conn, "still", "inbox,starred", true, before);
        insert(&conn, "unstarred", "inbox,starred", true, before);
        insert(&conn, "in_trash", "trash,starred", true, before);
        let s = starred_section();
        sync_section_rows(&conn, &[s.clone()], NOW).unwrap();
        apply_listing(&conn, "me", &s, None, &ids(&["still"]), Some("p2"), NOW).unwrap();
        // A message starred WHILE the walk ran (history set labels_at now).
        insert(&conn, "mid_walk", "inbox,starred", true, "2026-10-18T12:00:30+00:00");
        apply_listing(&conn, "me", &s, Some("p2"), &[], None, "2026-10-18T12:01:00+00:00").unwrap();

        assert_eq!(labels_of(&conn, "still").1, 1);
        let (labels, starred) = labels_of(&conn, "unstarred");
        assert_eq!(starred, 0);
        assert_eq!(labels, "inbox");
        assert_eq!(labels_of(&conn, "mid_walk").1, 1, "a star added mid-walk is not undone");
        assert_eq!(labels_of(&conn, "in_trash").1, 1, "trash is not listed by Starred, so not judged by it");
        let done: i64 = conn.query_row("SELECT done FROM mail_sync_sections WHERE key='starred'", [], |r| r.get(0)).unwrap();
        assert_eq!(done, 1);
    }

    #[test]
    fn walks_resume_where_they_stopped_and_follow_the_lanes() {
        let conn = test_conn();
        let sections = plan_sections("all", &["viagogo.com".into()], &[], "");
        sync_section_rows(&conn, &sections, NOW).unwrap();
        let (first, page) = next_walk(&conn, &sections).unwrap().unwrap();
        assert_eq!((first.key.as_str(), page), ("starred", None));
        apply_listing(&conn, "me", &first, None, &ids(&["a"]), Some("TOKEN2"), NOW).unwrap();
        let (again, page) = next_walk(&conn, &sections).unwrap().unwrap();
        assert_eq!((again.key.as_str(), page.as_deref()), ("starred", Some("TOKEN2")));
        apply_listing(&conn, "me", &again, Some("TOKEN2"), &[], None, NOW).unwrap();
        let (next, _) = next_walk(&conn, &sections).unwrap().unwrap();
        assert_eq!(next.key, "important", "no labels, so the backlog is next");
    }

    #[test]
    fn changing_a_folder_starts_its_walk_again_and_a_deleted_label_is_forgotten() {
        let conn = test_conn();
        let a = plan_sections("all", &[], &[label("Label_1", "A")], "");
        sync_section_rows(&conn, &a, NOW).unwrap();
        conn.execute("UPDATE mail_sync_sections SET done = 1, page_token = 'x'", []).unwrap();
        let b = plan_sections("all", &[], &[], "has:attachment");
        sync_section_rows(&conn, &b, NOW).unwrap();
        let rows: HashMap<String, (i64, Option<String>)> = conn
            .prepare("SELECT key, done, page_token FROM mail_sync_sections")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, (r.get(1)?, r.get(2)?))))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert!(!rows.contains_key("label:label_1"));
        assert_eq!(rows["inbox"], (0, None), "the filter changed, so the inbox starts again");
        assert_eq!(rows["starred"], (1, Some("x".into())), "nothing about Starred changed");
    }

    #[test]
    fn a_folder_gmail_refuses_is_set_aside_instead_of_holding_up_the_rest() {
        let conn = test_conn();
        let sections = plan_sections("all", &[], &[], "");
        sync_section_rows(&conn, &sections, NOW).unwrap();
        skip_walk(&conn, "starred", NOW).unwrap();
        let (next, _) = next_walk(&conn, &sections).unwrap().unwrap();
        assert_eq!(next.key, "finances");
        // ...and a definition that changes starts it again.
        let edited = plan_sections("all", &[], &[], "has:attachment");
        sync_section_rows(&conn, &edited, NOW).unwrap();
        skip_walk(&conn, "inbox", NOW).unwrap();
        let fixed = plan_sections("all", &[], &[], "has:pdf");
        sync_section_rows(&conn, &fixed, NOW).unwrap();
        let done: i64 = conn.query_row("SELECT done FROM mail_sync_sections WHERE key='inbox'", [], |r| r.get(0)).unwrap();
        assert_eq!(done, 0);
    }

    /// Scenario 9.
    #[test]
    fn a_restart_carries_on_with_the_same_queue_and_the_same_page() {
        let dir = std::env::temp_dir().join(format!("tiqr-mail-restart-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let main = dir.join("tiqr-manager.sqlite3");
        let _ = std::fs::remove_file(&main);
        let _ = std::fs::remove_file(crate::mail_store::mail_db_path(&main));
        let sections = plan_sections("all", &[], &[label("Label_1", "A")], "");
        let before = {
            let conn = crate::db::open_live_connection(&main).unwrap();
            crate::db::run_migrations(&conn).unwrap();
            sync_section_rows(&conn, &sections, NOW).unwrap();
            let starred = starred_section();
            apply_listing(&conn, "me", &starred, None, &ids(&["s1", "s2"]), Some("PAGE2"), NOW).unwrap();
            enqueue(&conn, "me", &ids(&["n1"]), TIER_NEW, "history", NOW).unwrap();
            peek(&conn, 10, NOW).unwrap()
        };
        // The app closes here - or crashes; nothing was in memory.
        let conn = crate::db::open_live_connection(&main).unwrap();
        crate::db::run_migrations(&conn).unwrap();
        assert_eq!(peek(&conn, 10, NOW).unwrap(), before);
        assert_eq!(before, vec!["n1", "s1", "s2"]);
        let (walk, page) = next_walk(&conn, &sections).unwrap().unwrap();
        assert_eq!((walk.key.as_str(), page.as_deref()), ("starred", Some("PAGE2")));
        drop(conn);
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ---- history ----------------------------------------------------------

    fn history_fixture() -> Value {
        json!({
            "history": [
                {"id": "101", "messagesAdded": [{"message": {"id": "n1", "threadId": "t", "labelIds": ["INBOX", "UNREAD"]}}]},
                {"id": "102", "labelsAdded": [{"message": {"id": "s1", "labelIds": ["INBOX", "STARRED"]}, "labelIds": ["STARRED"]}]},
                {"id": "103", "labelsRemoved": [{"message": {"id": "s2", "labelIds": ["INBOX"]}, "labelIds": ["STARRED", "UNREAD"]}]},
                {"id": "104", "messagesAdded": [{"message": {"id": "d1", "labelIds": ["DRAFT"]}}]},
                {"id": "105", "messagesAdded": [{"message": {"id": "sp", "labelIds": ["SPAM"]}}]}
            ],
            "nextPageToken": "NP",
            "historyId": "200"
        })
    }

    #[test]
    fn a_history_page_reads_in_order() {
        let (events, next, current, last) = parse_history_page(&history_fixture());
        assert_eq!(next.as_deref(), Some("NP"));
        assert_eq!(current.as_deref(), Some("200"));
        assert_eq!(last.as_deref(), Some("105"));
        assert_eq!(
            events[0],
            HistoryEvent::Added { id: "n1".into(), labels: vec!["inbox".into(), "unread".into()] }
        );
        assert_eq!(events[1], HistoryEvent::LabelsAdded { id: "s1".into(), labels: vec!["starred".into()] });
        assert_eq!(
            events[2],
            HistoryEvent::LabelsRemoved { id: "s2".into(), labels: vec!["starred".into(), "unread".into()] }
        );
        assert_eq!(events.len(), 5);
    }

    #[test]
    fn history_queues_new_mail_first_and_applies_stars_to_stored_mail() {
        let conn = test_conn();
        insert(&conn, "s1", "inbox", false, NOW);
        insert(&conn, "s2", "inbox,starred", true, NOW);
        conn.execute("UPDATE mail_messages SET is_unread = 0", []).unwrap();
        let (events, ..) = parse_history_page(&history_fixture());
        apply_history(&conn, "me", &events, NOW).unwrap();

        assert_eq!(labels_of(&conn, "s1").1, 1, "starred in Gmail, starred here");
        let (labels, starred) = labels_of(&conn, "s2");
        assert_eq!((labels.as_str(), starred), ("inbox", 0));
        let unread: i64 = conn.query_row("SELECT SUM(is_unread) FROM mail_messages", [], |r| r.get(0)).unwrap();
        assert_eq!(unread, 0, "TIQR's own read state is never taken from Gmail");
        // New mail first - spam too, it is new - and the draft never.
        assert_eq!(peek(&conn, 10, NOW).unwrap(), vec!["n1", "sp"]);
        assert_eq!(queued(&conn, "sp").unwrap().0, TIER_NEW);
    }

    /// Scenario 8.
    #[test]
    fn stars_and_labels_follow_gmail_both_ways_without_downloading_again() {
        let conn = test_conn();
        insert(&conn, "m", "inbox", false, NOW);
        let on = vec![
            HistoryEvent::LabelsAdded { id: "m".into(), labels: vec!["starred".into(), "label_7".into()] },
            HistoryEvent::LabelsRemoved { id: "m".into(), labels: vec!["inbox".into()] },
        ];
        apply_history(&conn, "me", &on, NOW).unwrap();
        assert_eq!(labels_of(&conn, "m"), ("starred,label_7".to_string(), 1), "starred, labelled, archived");
        let off = vec![
            HistoryEvent::LabelsRemoved { id: "m".into(), labels: vec!["starred".into(), "label_7".into()] },
            HistoryEvent::LabelsAdded { id: "m".into(), labels: vec!["trash".into()] },
        ];
        apply_history(&conn, "me", &off, NOW).unwrap();
        assert_eq!(labels_of(&conn, "m"), ("trash".to_string(), 0));
        assert_eq!(queue_depth(&conn), 0, "a change to stored mail is applied, never downloaded again");

        // Not stored yet: a label added moves it to the labels lane.
        let ev = vec![HistoryEvent::LabelsAdded { id: "elsewhere".into(), labels: vec!["label_7".into()] }];
        apply_history(&conn, "me", &ev, NOW).unwrap();
        assert_eq!(queued(&conn, "elsewhere").unwrap().0, TIER_LABELS);
    }

    #[test]
    fn the_same_history_twice_changes_nothing_the_second_time() {
        let conn = test_conn();
        insert(&conn, "s1", "inbox", false, NOW);
        let (events, ..) = parse_history_page(&history_fixture());
        apply_history(&conn, "me", &events, NOW).unwrap();
        let labels_once = labels_of(&conn, "s1");
        let queue_once = peek(&conn, 10, NOW).unwrap();
        let (changed, queued_again) = apply_history(&conn, "me", &events, NOW).unwrap();
        assert_eq!((changed, queued_again), (0, 0));
        assert_eq!(labels_of(&conn, "s1"), labels_once);
        assert_eq!(peek(&conn, 10, NOW).unwrap(), queue_once);
    }

    #[test]
    fn history_is_read_every_twenty_seconds_while_there_is_work() {
        let conn = test_conn();
        set_setting(&conn, MAIL_REFRESH_TOKEN_KEY, "r").unwrap();
        conn.execute(
            "UPDATE mail_sync_state SET last_history_id = '100', history_checked_at = ?1 WHERE id = 1",
            params![NOW],
        )
        .unwrap();
        // His interval is ten minutes; that is for when everything is read.
        set_setting(&conn, "mail_sync_interval_seconds", "600").unwrap();
        let soon = make_plan(&conn, later(5)).unwrap().unwrap();
        assert!(soon.history_from.is_none());
        let due_now = make_plan(&conn, later(HISTORY_EVERY_SECS)).unwrap().unwrap();
        assert_eq!(due_now.history_from.as_deref(), Some("100"));
    }

    /// Scenario 11.
    #[test]
    fn lost_history_walks_every_folder_again_and_keeps_every_message() {
        let conn = test_conn();
        set_setting(&conn, MAIL_REFRESH_TOKEN_KEY, "r").unwrap();
        let sections = plan_sections("all", &[], &[], "");
        sync_section_rows(&conn, &sections, NOW).unwrap();
        conn.execute("UPDATE mail_sync_sections SET done = 1, passes = 1", []).unwrap();
        conn.execute("UPDATE mail_sync_state SET last_history_id = '100' WHERE id = 1", []).unwrap();
        insert(&conn, "kept", "inbox,starred", true, NOW);
        enqueue(&conn, "me", &ids(&["waiting"]), TIER_STARRED, "starred", NOW).unwrap();

        // A page of the inbox walk came back in the same run as the 404.
        let mut plan = plan_with(&[]);
        plan.history_from = Some("100".into());
        plan.listing = Some((section("inbox", &[]), Some("OLD_PAGE".into())));
        let net = Net {
            history: Some(Err(GmailFailure::NotFound)),
            listing: Some(Ok((ids(&["x"]), Some("NEXT".into())))),
            ..Default::default()
        };
        let r = apply_now(&conn, &plan, net, now());
        assert_eq!(r.status, "synced");
        let anchor: Option<String> =
            conn.query_row("SELECT last_history_id FROM mail_sync_state WHERE id = 1", [], |r| r.get(0)).unwrap();
        assert!(anchor.is_none(), "a fresh starting point is taken next run");
        let not_done: i64 =
            conn.query_row("SELECT COUNT(*) FROM mail_sync_sections WHERE done = 0 AND page_token IS NULL", [], |r| r.get(0)).unwrap();
        assert_eq!(not_done, sections.len() as i64, "every folder from the top - the stale page was not applied");
        assert_eq!(rows_for(&conn, "kept"), 1);
        assert_eq!(labels_of(&conn, "kept").1, 1);
        assert!(queued(&conn, "waiting").is_some(), "the queue is kept");
        assert!(queued(&conn, "x").is_none());

        // The next run takes the fresh starting point.
        assert!(make_plan(&conn, later(1)).unwrap().unwrap().need_profile);

        // A cursor refused again straight away (400) does not send the walks
        // back a second time within the hour...
        conn.execute("UPDATE mail_sync_sections SET done = 1 WHERE key = 'starred'", []).unwrap();
        conn.execute("UPDATE mail_sync_state SET last_history_id = '300' WHERE id = 1", []).unwrap();
        let net = Net { history: Some(Err(GmailFailure::BadRequest)), ..Default::default() };
        apply_now(&conn, &plan_with(&[]), net, later(600));
        let starred_done: i64 =
            conn.query_row("SELECT done FROM mail_sync_sections WHERE key = 'starred'", [], |r| r.get(0)).unwrap();
        assert_eq!(starred_done, 1);
        // ...but does after it.
        conn.execute("UPDATE mail_sync_state SET last_history_id = '300' WHERE id = 1", []).unwrap();
        let net = Net { history: Some(Err(GmailFailure::BadRequest)), ..Default::default() };
        apply_now(&conn, &plan_with(&[]), net, later(RESCAN_GUARD_SECS + 10));
        let starred_done: i64 =
            conn.query_row("SELECT done FROM mail_sync_sections WHERE key = 'starred'", [], |r| r.get(0)).unwrap();
        assert_eq!(starred_done, 0);
    }

    // ---- what is kept, and what it creates --------------------------------

    #[test]
    fn the_known_scope_keeps_watched_senders_money_and_anything_he_marked() {
        let senders = vec!["viagogo.com".to_string()];
        let keep = |m: FetchedMessage| {
            let c = classify_fetched(&m);
            wanted_in_known_scope(&m, &c, &senders)
        };
        assert!(keep(fetched("noreply@email.viagogo.com", "Hello", "inbox")));
        assert!(keep(fetched("service@paypal.com", "You received money", "inbox")));
        assert!(keep(fetched("someone@x.com", "lunch", "inbox,starred")));
        assert!(keep(fetched("someone@x.com", "lunch", "inbox,label_5")));
        assert!(!keep(fetched("news@shop.com", "Big sale", "inbox")));
    }

    /// Scenario 13.
    #[test]
    fn syncing_the_same_mail_again_creates_no_second_message_and_no_second_record() {
        let conn = test_conn();
        set_setting(&conn, "mail_auto_finance", "auto").unwrap();
        let payout = || {
            let m = fetched_as("p1", "noreply@viagogo.com", "Your payout of 240,00 EUR is on the way", "inbox");
            let c = classify_fetched(&m);
            ("p1".to_string(), Fetched::Got(Box::new(m), c))
        };
        let count = |table: &str| -> i64 {
            conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0)).unwrap()
        };
        for round in 0..3 {
            enqueue(&conn, "me", &ids(&["p1"]), TIER_NEW, "history", NOW).unwrap();
            let net = Net { fetched: vec![payout()], ..Default::default() };
            let r = apply_now(&conn, &plan_with(&["p1"]), net, later(round));
            assert_eq!(r.stored_new, i64::from(round == 0), "round {round}");
            // The same event delivered again by the history.
            let ev = vec![HistoryEvent::Added { id: "p1".into(), labels: vec!["inbox".into()] }];
            apply_history(&conn, "me", &ev, NOW).unwrap();
            assert_eq!(queue_depth(&conn), 0, "stored mail is never queued again");
        }
        assert_eq!(count("mail_messages"), 1);
        assert_eq!(count("finance_entries"), 1, "Automatic created it once");
    }

    #[test]
    fn a_record_automatic_fails_to_create_leaves_nothing_half_written() {
        let conn = test_conn();
        set_setting(&conn, "mail_auto_finance", "auto").unwrap();
        let m = fetched_as("p1", "noreply@viagogo.com", "Your payout of 240,00 EUR is on the way", "inbox");
        let c = classify_fetched(&m);
        let (row, _) = upsert_message(&conn, MailProviderKind::Gmail, "me@gmail.com", &m, &c).unwrap();
        // The finance table refuses every insert: creation fails half way.
        conn.execute_batch(
            "CREATE TEMP TRIGGER no_finance BEFORE INSERT ON finance_entries BEGIN SELECT RAISE(ABORT, 'nope'); END;",
        )
        .unwrap();
        let notices = run_automation(&conn, &[(row, "x".into())]);
        assert!(notices.is_empty());
        let created: Option<i64> =
            conn.query_row("SELECT created_id FROM mail_messages WHERE id = ?1", params![row], |r| r.get(0)).unwrap();
        assert!(created.is_none(), "the message is not marked as having made a record it did not make");
    }

    /// Scenario 14 (the backend half; src/lib/mailRows.test.ts is the screen's).
    #[test]
    fn stored_mail_is_listed_while_a_long_read_is_under_way() {
        let conn = test_conn();
        for i in 0..30 {
            insert(&conn, &format!("here{i}"), "inbox", false, NOW);
        }
        let sections = plan_sections("all", &[], &[], "");
        sync_section_rows(&conn, &sections, NOW).unwrap();
        apply_listing(&conn, "me", &starred_section(), None, &ids(&["w1", "w2"]), Some("p2"), NOW).unwrap();
        conn.execute("UPDATE mail_sync_state SET sync_in_progress = 1 WHERE id = 1", []).unwrap();
        let page = crate::commands::mail::list_folder_impl(&conn, "inbox", None, None, 200).unwrap();
        assert_eq!(page.len(), 30, "everything already here, straight away");
        let p = progress_of(&conn, now());
        assert_eq!(p.stored_total, 30);
        assert_eq!(p.starred.waiting, 2);
        assert_eq!(p.starred.discovery, "discovering", "more starred mail may still be found");
    }

    // ---- progress ---------------------------------------------------------

    #[test]
    fn progress_counts_each_lane_and_never_guesses_a_total() {
        let conn = test_conn();
        let labels = vec![label("Label_1", "OASIS")];
        let sections = plan_sections("all", &[], &labels, "");
        sync_section_rows(&conn, &sections, NOW).unwrap();
        // Before anything: new mail is not followed yet, every lane is pending.
        let p = progress_of(&conn, now());
        assert_eq!(p.new_mail.discovery, "pending");
        assert_eq!(p.starred.discovery, "pending");
        assert_eq!(p.labels.discovery, "pending");
        assert_eq!(p.backlog.discovery, "pending");
        assert_eq!(p.phase, "starred");

        conn.execute("UPDATE mail_sync_state SET last_history_id = '1', last_sync_at = ?1 WHERE id = 1", params![NOW])
            .unwrap();
        apply_listing(&conn, "me", &starred_section(), None, &ids(&["s1", "s2"]), None, NOW).unwrap();
        let oasis = section("label:label_1", &labels);
        apply_listing(&conn, "me", &oasis, None, &ids(&["l1"]), Some("p2"), NOW).unwrap();
        enqueue(&conn, "me", &ids(&["n1"]), TIER_NEW, "history", NOW).unwrap();
        enqueue(&conn, "me", &ids(&["h1", "h2", "h3"]), TIER_INBOX, "inbox", NOW).unwrap();

        let p = progress_of(&conn, now());
        assert_eq!((p.new_mail.waiting, p.new_mail.discovery.as_str()), (1, "watching"));
        assert_eq!((p.starred.waiting, p.starred.discovery.as_str()), (2, "complete"));
        assert_eq!((p.labels.waiting, p.labels.discovery.as_str()), (1, "discovering"));
        assert_eq!((p.backlog.waiting, p.backlog.discovery.as_str()), (3, "pending"));
        assert_eq!((p.waiting, p.due, p.retry_later), (7, 7, 0));
        assert_eq!(p.phase, "new");
        assert_eq!(p.reading.as_deref(), Some("New mail"));
        assert_eq!(p.last_success_at.as_deref(), Some(NOW));
    }

    #[test]
    fn progress_names_the_folder_being_read() {
        let conn = test_conn();
        let sections = plan_sections("all", &[], &[], "");
        sync_section_rows(&conn, &sections, NOW).unwrap();
        conn.execute("UPDATE mail_sync_sections SET done = 1 WHERE key = 'starred'", []).unwrap();
        let p = progress_of(&conn, now());
        assert_eq!(p.reading.as_deref(), Some("Finances"));
        assert_eq!(p.phase, "backlog");
        assert_eq!(p.folders_done, 1);
        assert_eq!(p.folders_total, sections.len() as i64);
    }

    #[test]
    fn rate_limited_runs_wait_longer_each_time_and_never_less_than_google_said() {
        let conn = test_conn();
        let limited = |retry: Option<u64>| Net { rate_limited: true, retry_after_secs: retry, ..Default::default() };
        let first = apply_now(&conn, &plan_with(&[]), limited(None), now());
        assert!(first.rate_limited);
        assert!((RATE_LIMIT_PAUSE_MS * 3 / 4..=RATE_LIMIT_PAUSE_MS).contains(&first.next_delay_ms), "{}", first.next_delay_ms);
        let second = apply_now(&conn, &plan_with(&[]), limited(None), later(30));
        assert!(second.next_delay_ms > first.next_delay_ms, "{} then {}", first.next_delay_ms, second.next_delay_ms);
        let told = apply_now(&conn, &plan_with(&[]), limited(Some(900)), later(90));
        assert!(told.next_delay_ms >= 900_000, "Google's Retry-After wins: {}", told.next_delay_ms);
        // One good run and it starts gently again.
        apply_now(&conn, &plan_with(&[]), Net::default(), later(1000));
        let again = apply_now(&conn, &plan_with(&[]), limited(None), later(1001));
        assert!(again.next_delay_ms <= RATE_LIMIT_PAUSE_MS, "{}", again.next_delay_ms);
    }

    // ---- against a fake Gmail ---------------------------------------------
    //
    // A real HTTP server on 127.0.0.1, answering like Gmail does, so the
    // URLs, the JSON and the retries are exercised for real - with no
    // account and no credentials anywhere.

    type Answer = (u16, Vec<(&'static str, String)>, String);

    struct FakeGmail {
        api: String,
        seen: Arc<Mutex<Vec<String>>>,
    }

    impl FakeGmail {
        fn start(route: impl Fn(&str) -> Answer + Send + Sync + 'static) -> FakeGmail {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let api = format!("http://{}", listener.local_addr().unwrap());
            let seen = Arc::new(Mutex::new(Vec::new()));
            let route = Arc::new(route);
            let log = seen.clone();
            std::thread::spawn(move || {
                for stream in listener.incoming() {
                    let Ok(mut stream) = stream else { continue };
                    let route = route.clone();
                    let log = log.clone();
                    std::thread::spawn(move || {
                        let mut head = Vec::new();
                        let mut buf = [0u8; 4096];
                        while !head.windows(4).any(|w| w == b"\r\n\r\n") {
                            match stream.read(&mut buf) {
                                Ok(0) | Err(_) => return,
                                Ok(n) => head.extend_from_slice(&buf[..n]),
                            }
                        }
                        let text = String::from_utf8_lossy(&head).to_string();
                        let target = text.split_whitespace().nth(1).unwrap_or("/").to_string();
                        log.lock().unwrap().push(target.clone());
                        let (code, headers, body) = route(&target);
                        let mut out = format!(
                            "HTTP/1.1 {code} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n",
                            body.len()
                        );
                        for (k, v) in headers {
                            out.push_str(&format!("{k}: {v}\r\n"));
                        }
                        out.push_str("\r\n");
                        out.push_str(&body);
                        let _ = stream.write_all(out.as_bytes());
                    });
                }
            });
            FakeGmail { api, seen }
        }

        fn requests(&self) -> Vec<String> {
            self.seen.lock().unwrap().clone()
        }
    }

    fn ok(v: Value) -> Answer {
        (200, Vec::new(), v.to_string())
    }

    fn client() -> reqwest::blocking::Client {
        reqwest::blocking::Client::builder().no_proxy().timeout(Duration::from_secs(10)).build().unwrap()
    }

    fn listing(list: &[&str]) -> Answer {
        let messages: Vec<Value> = list.iter().map(|id| json!({"id": id, "threadId": format!("t-{id}")})).collect();
        ok(json!({"messages": messages, "resultSizeEstimate": list.len()}))
    }

    fn message(id: &str, subject: &str, labels: &[&str], days_old: i64) -> Answer {
        let ms = (now() - chrono::Duration::days(days_old)).timestamp_millis();
        ok(json!({
            "id": id, "threadId": format!("t-{id}"), "labelIds": labels, "snippet": subject,
            "internalDate": ms.to_string(),
            "payload": {"headers": [
                {"name": "From", "value": "Shop <news@shop.example>"},
                {"name": "Subject", "value": subject},
            ]}
        }))
    }

    /// Scenario 10.
    #[test]
    fn a_rate_limit_from_gmail_is_waited_out_and_the_request_tried_again() {
        let calls = Arc::new(AtomicU64::new(0));
        let n = calls.clone();
        let gmail = FakeGmail::start(move |_| {
            if n.fetch_add(1, Ordering::SeqCst) == 0 {
                let body = r#"{"error":{"code":403,"errors":[{"reason":"userRateLimitExceeded"}]}}"#;
                (403, vec![("Retry-After", "1".to_string())], body.to_string())
            } else {
                ok(json!({"emailAddress": "me@gmail.com", "historyId": "1"}))
            }
        });
        let started = std::time::Instant::now();
        let v = gmail_json(&client(), "t", &format!("{}/profile", gmail.api)).unwrap();
        assert_eq!(v["historyId"], "1");
        assert_eq!(gmail.requests().len(), 2, "asked again after the wait");
        assert!(started.elapsed() >= Duration::from_millis(900), "Google's Retry-After was waited out");
    }

    #[test]
    fn a_server_error_is_retried_and_one_that_lasts_is_handed_back() {
        let gmail = FakeGmail::start(|_| (503, Vec::new(), "{}".to_string()));
        let err = gmail_json(&client(), "t", &format!("{}/profile", gmail.api)).unwrap_err();
        assert_eq!(err, GmailFailure::Server(503));
        assert_eq!(gmail.requests().len(), REQUEST_ATTEMPTS as usize);
    }

    /// Scenarios 1, 2, 3, 5 and 7, end to end: plan, HTTP, store - the way
    /// `run_once` runs them, against a mailbox where the ordinary mail is the
    /// newest and the starred and labelled mail is a year old. The folders are
    /// big enough that each lane is still downloading when the next one is
    /// found, so it is the QUEUE that has to keep the order, not luck.
    #[test]
    fn a_whole_mailbox_comes_down_new_then_starred_then_labels_then_history() {
        let conn = test_conn();
        set_setting(&conn, MAIL_REFRESH_TOKEN_KEY, "r").unwrap();
        set_setting(&conn, MAIL_ACCOUNT_EMAIL_KEY, "me@gmail.com").unwrap();
        let arrived = Arc::new(AtomicBool::new(false));
        let flag = arrived.clone();
        let named = |prefix: &str, n: usize| -> Vec<String> { (0..n).map(|i| format!("{prefix}{i:03}")).collect() };
        let starred = named("s", 60);
        let labelled = named("lab", 150);
        let history = named("h", 120);
        let (st, lb, hi) = (starred.clone(), labelled.clone(), history.clone());
        let gmail = FakeGmail::start(move |target| {
            let path = target.split('?').next().unwrap_or("");
            let refs = |v: &[String]| -> Vec<String> { v.to_vec() };
            match path {
                "/profile" => ok(json!({"emailAddress": "me@gmail.com", "historyId": "100"})),
                "/labels" => ok(json!({"labels": [
                    {"id": "INBOX", "name": "INBOX", "type": "system"},
                    {"id": "STARRED", "name": "STARRED", "type": "system"},
                    {"id": "IMPORTANT", "name": "IMPORTANT", "type": "system"},
                    {"id": "Label_1", "name": "OASIS", "type": "user"}
                ]})),
                "/history" if flag.load(Ordering::SeqCst) && target.contains("startHistoryId=100") => ok(json!({
                    "history": [{"id": "101", "messagesAdded": [{"message": {"id": "brand_new", "labelIds": ["INBOX", "UNREAD"]}}]}],
                    "historyId": "101"
                })),
                "/history" => ok(json!({"historyId": if flag.load(Ordering::SeqCst) { "101" } else { "100" }})),
                "/messages" if target.contains("labelIds=STARRED") => {
                    let mut all = vec!["both".to_string()];
                    all.extend(refs(&st));
                    listing(&all.iter().map(|s| s.as_str()).collect::<Vec<_>>())
                }
                "/messages" if target.contains("labelIds=Label_1") => {
                    let mut all = vec!["both".to_string()];
                    all.extend(refs(&lb));
                    listing(&all.iter().map(|s| s.as_str()).collect::<Vec<_>>())
                }
                "/messages" if target.contains("q=-in:sent") => {
                    // Starred mail is in the inbox walk too. The labelled mail
                    // is in Promotions, which the inbox walk leaves out - so
                    // only the queue can keep it ahead of the history.
                    let mut all = refs(&hi);
                    all.extend(refs(&st));
                    all.push("both".into());
                    listing(&all.iter().map(|s| s.as_str()).collect::<Vec<_>>())
                }
                "/messages" if target.contains("labelIds=CATEGORY_PROMOTIONS") => {
                    listing(&lb.iter().map(|s| s.as_str()).collect::<Vec<_>>())
                }
                "/messages" => listing(&[]),
                p if p.starts_with("/messages/") => {
                    let id = &p["/messages/".len()..];
                    if id == "both" {
                        message(id, "starred and labelled", &["STARRED", "Label_1", "IMPORTANT"], 400)
                    } else if id == "brand_new" {
                        message(id, "just arrived", &["INBOX", "UNREAD"], 0)
                    } else if id.starts_with("lab") {
                        message(id, "a year-old label", &["Label_1", "CATEGORY_PROMOTIONS"], 370)
                    } else if id.starts_with('s') {
                        message(id, "a year-old star", &["STARRED"], 365)
                    } else {
                        message(id, "ordinary", &["INBOX"], 1)
                    }
                }
                _ => (404, Vec::new(), "{}".to_string()),
            }
        });

        let mut when = now();
        let mut results = Vec::new();
        for i in 0..60 {
            if i == 4 {
                arrived.store(true, Ordering::SeqCst);
                when += chrono::Duration::seconds(HISTORY_EVERY_SECS);
            }
            let mut plan = make_plan(&conn, when).unwrap().expect("connected");
            plan.api = gmail.api.clone();
            let net = network(&plan, &client(), "test-token").unwrap();
            let r = apply_committed(&conn, &plan, net, when).unwrap();
            let done = !r.more_waiting;
            results.push(r);
            when += chrono::Duration::seconds(1);
            if done {
                break;
            }
        }
        let messages: Vec<&String> = results.iter().map(|r| &r.message).collect();
        assert!(results.iter().all(|r| r.status == "synced"), "{messages:?}");
        assert!(!results.last().unwrap().more_waiting, "everything came down: {messages:?}");

        // The order things were downloaded in is the order they were stored.
        let order: Vec<String> = conn
            .prepare("SELECT provider_message_id FROM mail_messages ORDER BY id")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        let pos = |id: &str| order.iter().position(|x| x == id).unwrap_or_else(|| panic!("{id} never stored"));
        let first = |ids: &[String]| ids.iter().map(|id| pos(id)).min().unwrap();
        let last = |ids: &[String]| ids.iter().map(|id| pos(id)).max().unwrap();
        assert_eq!(order.len(), starred.len() + labelled.len() + history.len() + 2);
        assert!(pos("both") < first(&labelled), "starred and labelled counts as starred");
        assert!(last(&starred) < first(&labelled), "every star before any label");
        assert!(last(&labelled) < first(&history), "a year-old label before yesterday's ordinary mail");
        assert!(pos("brand_new") < last(&labelled), "new mail did not wait behind the labels still queued");
        assert!(pos("brand_new") < first(&history));

        // Each message was downloaded exactly once - "both" too.
        let gets: Vec<String> = gmail.requests().into_iter().filter(|t| t.starts_with("/messages/")).collect();
        let unique: HashSet<&String> = gets.iter().collect();
        assert_eq!(gets.len(), unique.len(), "no message downloaded twice");
        assert_eq!(gets.len(), order.len());
        assert_eq!(gets.iter().filter(|t| t.starts_with("/messages/both?")).count(), 1);
        assert!(gets.iter().all(|t| t.contains("format=metadata")), "metadata only, never bodies");

        // Every label kept, on one row; Gmail's IMPORTANT kept but ranks nothing.
        assert_eq!(rows_for(&conn, "both"), 1);
        let (labels, is_starred) = labels_of(&conn, "both");
        assert_eq!(is_starred, 1);
        for l in ["starred", "label_1", "important"] {
            assert!(labels.split(',').any(|x| x == l), "{l} in {labels}");
        }
        let oasis = crate::commands::mail::list_folder_impl(&conn, "label_1", None, None, 500).unwrap();
        assert_eq!(oasis.len(), labelled.len() + 1, "the label folder shows all of its messages");
    }
}

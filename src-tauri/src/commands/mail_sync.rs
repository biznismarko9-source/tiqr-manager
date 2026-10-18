//! 2.110.0 - the mail reading engine.
//!
//! marko, in his own order:
//!
//! * "najskor treba important, starred, labels, finance a az potom vsetko
//!   ostatne"
//! * "chcem aby to citalo spravy od najmladsiej po najstarsiu po tych
//!   castiach a samo, nie ze ja vzdy sync musim davat"
//! * "labels a starred messages musi brat vsetky a je to jedno ake su stare"
//!
//! ## What one run does
//!
//! A run is small - about a second - and the app chains them by itself
//! (Layout.tsx) while anything is left to read, then keeps checking for new
//! mail on marko's interval. Every run, in this order:
//!
//! 1. **New mail and changes**, from Gmail's own change history, once per
//!    interval. New messages jump the queue, and a star or a label added or
//!    taken off in Gmail is applied to the message already stored.
//! 2. **One page of one folder**, while the queue is running low: the first
//!    folder in marko's order that has not been walked to its oldest message
//!    yet. Ids already stored cost nothing; the rest are queued in that
//!    folder's place in the order.
//! 3. **One batch from the queue**: folder order first, newest first inside a
//!    folder.
//!
//! Every folder walks from its newest message to its oldest, with no date
//! limit, and where the walk has got to is kept on disk - close the app
//! halfway through and the next start carries on from the same page.
//!
//! ## History, and why it can expire without anything being lost
//!
//! Gmail keeps change history for about a week and answers 404 for anything
//! older. This app only runs while marko has it open, so after a holiday that
//! is the normal case. It costs one thing: every folder is walked again from
//! the top. Already-stored ids are skipped in SQL, so a walk over a mailbox
//! that is already here is listing only - a few minutes for 100 000
//! messages, not a re-download.
//!
//! ## When Google says "slow down"
//!
//! Gmail answers a rate limit with 429 OR with 403 (`userRateLimitExceeded`,
//! `rateLimitExceeded`). Until 2.110.0 a 403 was always reported as "Google
//! refused access", was never retried, and every message that failed to
//! download was dropped from the queue - so the more marko synced, the more
//! mail went missing. Now the reason in Google's answer is read (the reason code only,
//! never the text), a rate limit pauses every worker, the batch stays queued,
//! and the app is told when to carry on.
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
use crate::models::{MailSyncProgress, MailSyncResult};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

// ---------------------------------------------------------------------------
// marko's order.
// ---------------------------------------------------------------------------

/// The queue drains lowest first. The gaps are deliberate, so a folder can be
/// slotted in later without renumbering the rest.
pub(crate) const TIER_NEW: i64 = 0;
pub(crate) const TIER_IMPORTANT: i64 = 10;
pub(crate) const TIER_STARRED: i64 = 20;
pub(crate) const TIER_LABELS: i64 = 30;
pub(crate) const TIER_FINANCES: i64 = 40;
pub(crate) const TIER_INBOX: i64 = 50;
pub(crate) const TIER_PROMOTIONS: i64 = 60;
pub(crate) const TIER_SPAM: i64 = 70;
pub(crate) const TIER_TRASH: i64 = 80;

/// How many messages one run downloads - about a second of work.
const FETCH_BATCH: usize = 40;

/// Parallel downloads. FOUR, where 2.108.0 used six: each `messages.get`
/// costs 5 of the 250 quota units a user gets per second, so ~50 messages a
/// second is Google's ceiling - and six workers sat right on it, which is
/// where the 403s marko saw came from. Four stays under it with room.
const FETCH_THREADS: usize = 4;

/// The next folder page is listed only while fewer than this many ids wait.
/// Listing runs a little ahead of downloading, never a whole mailbox ahead.
const LOW_WATER: i64 = 200;

/// Gmail's label list is re-read this often, so a label made in Gmail shows
/// up without a restart.
const LABELS_EVERY_SECS: i64 = 300;

/// A message that fails this many times leaves the queue, and the next walk
/// of its folder offers it again. Only a failure while Google answered for
/// the rest of its batch counts - see `store_fetched`.
const MAX_ATTEMPTS: i64 = 5;

/// How long the app waits after Google asked it to slow down.
const RATE_LIMIT_PAUSE_MS: i64 = 20_000;

/// The pause between chained runs while there is more to read.
const CHAIN_DELAY_MS: i64 = 250;

/// How long the app waits when a run got nothing at all because Gmail could
/// not be reached, or Google had a server error - a laptop that lost its
/// wifi, mostly. Chaining runs a quarter of a second apart would only spin.
const TROUBLE_DELAY_MS: i64 = 30_000;

/// History pages read in one run. Beyond this the checkpoint moves to the
/// last record read and the rest is read next time.
const HISTORY_MAX_PAGES: usize = 40;

/// Rows from before 2.110.0 get their Finances flag this many at a time.
const FINANCE_BACKFILL_CHUNK: i64 = 2000;

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

    pub(crate) fn list_url(&self, page: Option<&str>) -> String {
        let mut url = format!("{GMAIL_API}/messages?maxResults=500");
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

/// The folders, in marko's order: Important, Starred, his labels, Finances,
/// and only then everything else - the inbox, promotions, spam and trash.
///
/// Starred and labels are LABEL walks with no date in them at all: "je to
/// jedno ake su stare". Spam and Trash are walked so they can be looked in;
/// what is in them is still never trusted (commands/mail.rs::is_untrusted).
///
/// In the `known` scope only the first four are walked - the mail that
/// matters, and none of the rest. An EMPTY sender list there still means the
/// whole mailbox (2.109.0: a list he emptied must not quietly come back as
/// the built-in one, and "watch nothing" is not a mailbox).
pub(crate) fn plan_sections(scope: &str, senders: &[String], labels: &[UserLabel], custom: &str) -> Vec<Section> {
    let whole_mailbox = scope != "known" || senders.is_empty();
    let mut out = Vec::new();
    if !senders.is_empty() {
        let terms = senders.iter().map(|d| format!("from:{d}")).collect::<Vec<_>>().join(" OR ");
        out.push(Section::search("important", "Important", TIER_IMPORTANT, format!("({terms}) -in:chats")));
    }
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

/// Queues ids that are not stored yet. Already-stored and deleted-forever
/// ids are skipped in SQL, which is what makes walking a folder that is
/// already here cost nothing but the listing. An id queued twice keeps the
/// better of its two places in the order.
pub(crate) fn enqueue(conn: &Connection, account: &str, ids: &[String], tier: i64, now: &str) -> AppResult<usize> {
    let mut seq: i64 = conn.query_row("SELECT IFNULL(MAX(seq), 0) FROM mail_fetch_queue", [], |r| r.get(0))?;
    let mut stmt = conn.prepare(
        "INSERT INTO mail_fetch_queue (provider_message_id, account_email, queued_at, priority, seq)
         SELECT ?1, ?2, ?3, ?4, ?5
          WHERE NOT EXISTS (SELECT 1 FROM mail_messages WHERE provider_message_id = ?1)
            AND NOT EXISTS (SELECT 1 FROM mail_deleted WHERE provider_message_id = ?1)
         ON CONFLICT(provider_message_id) DO UPDATE SET priority = MIN(priority, excluded.priority)",
    )?;
    let mut n = 0usize;
    for id in ids {
        seq += 1;
        n += stmt.execute(params![id, account, now, tier, seq])?;
    }
    Ok(n)
}

/// The next ids to download: marko's order first, then the order they were
/// listed in - which inside one folder is newest first.
pub(crate) fn peek(conn: &Connection, limit: usize) -> AppResult<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT provider_message_id FROM mail_fetch_queue ORDER BY priority ASC, seq ASC LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit as i64], |r| r.get::<_, String>(0))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

fn dequeue(conn: &Connection, ids: &[String]) -> AppResult<()> {
    let mut stmt = conn.prepare("DELETE FROM mail_fetch_queue WHERE provider_message_id = ?1")?;
    for id in ids {
        stmt.execute(params![id])?;
    }
    Ok(())
}

/// One more failed attempt. Past `MAX_ATTEMPTS` the id leaves the queue so
/// one bad message cannot hold a slot forever; the next walk of its folder
/// offers it again.
fn bump_attempts(conn: &Connection, ids: &[String]) -> AppResult<()> {
    let mut stmt = conn.prepare("UPDATE mail_fetch_queue SET attempts = attempts + 1 WHERE provider_message_id = ?1")?;
    for id in ids {
        stmt.execute(params![id])?;
    }
    conn.execute("DELETE FROM mail_fetch_queue WHERE attempts >= ?1", params![MAX_ATTEMPTS])?;
    Ok(())
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
    enqueue(conn, account, ids, section.tier, now)?;
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

/// Where a brand-new message goes in the queue. New mail is first - that is
/// what "real time" means - except junk, which keeps its own late place.
pub(crate) fn new_mail_tier(labels: &[String]) -> i64 {
    let kept = has(labels, "starred") || labels.iter().any(|l| is_user_label(l));
    if has(labels, "spam") {
        TIER_SPAM
    } else if has(labels, "trash") {
        TIER_TRASH
    } else if has(labels, "category_promotions") && !kept {
        TIER_PROMOTIONS
    } else {
        TIER_NEW
    }
}

fn is_stored(conn: &Connection, pid: &str) -> AppResult<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM mail_messages WHERE provider_message_id = ?1)",
        params![pid],
        |r| r.get(0),
    )?)
}

/// Applies history events in order. Returns (stored messages changed, ids
/// queued).
///
/// TIQR's own read state is deliberately left alone (migration 041): Gmail's
/// UNREAD going on or off is not applied, so a message he marked read here
/// is not turned unread because his phone saw it differently.
pub(crate) fn apply_history(conn: &Connection, account: &str, events: &[HistoryEvent], now: &str) -> AppResult<(usize, usize)> {
    let mut changed = 0usize;
    let mut queued = 0usize;
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
                queued += enqueue(conn, account, std::slice::from_ref(id), new_mail_tier(labels), now)?;
            }
            HistoryEvent::LabelsAdded { id, labels } => {
                if is_stored(conn, id)? {
                    for l in labels.iter().filter(|l| *l != "unread") {
                        changed += add_flag(conn, id, l, now)?;
                    }
                } else if has(labels, "starred") {
                    // An old message he just starred in Gmail.
                    queued += enqueue(conn, account, std::slice::from_ref(id), TIER_STARRED, now)?;
                } else if labels.iter().any(|l| is_user_label(l)) {
                    queued += enqueue(conn, account, std::slice::from_ref(id), TIER_LABELS, now)?;
                }
            }
            HistoryEvent::LabelsRemoved { id, labels } => {
                for l in labels.iter().filter(|l| *l != "unread") {
                    changed += remove_flag(conn, id, l, now)?;
                }
            }
        }
    }
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

/// One authenticated GET, with backoff for a rate limit and for Google's own
/// bad moments. Three attempts; a rate limit that outlasts them is returned
/// so the run can stop and the app can wait.
pub(crate) fn gmail_json(http: &reqwest::blocking::Client, token: &str, url: &str) -> Result<Value, GmailFailure> {
    let mut last = GmailFailure::Network;
    for attempt in 0..3u32 {
        wait_for_pause();
        let resp = match http.get(url).bearer_auth(token).send() {
            Ok(r) => r,
            Err(_) => {
                last = GmailFailure::Network;
                std::thread::sleep(Duration::from_millis(400u64 << attempt));
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
                let wait = retry_after_secs.map(|s| s.saturating_mul(1000)).unwrap_or(1000u64 << attempt);
                pause_all_for(wait.min(30_000));
                last = GmailFailure::RateLimited { retry_after_secs };
            }
            GmailFailure::Server(c) => {
                std::thread::sleep(Duration::from_millis(400u64 << attempt));
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

    // 0 is "only when I press Sync"; a pressed Sync still reads history,
    // just not more often than every half minute.
    let interval_secs = sync_interval_seconds(conn);
    let need_profile = st.last_history_id.is_none();
    let labels_due = due(st.labels_checked_at.as_deref(), LABELS_EVERY_SECS, now);
    let history_from = if need_profile {
        None
    } else if due(st.history_checked_at.as_deref(), interval_secs.max(30), now) {
        st.last_history_id.clone()
    } else {
        None
    };
    let listing = if queue_depth(conn) < LOW_WATER { next_walk(conn, &sections)? } else { None };
    let batch = peek(conn, FETCH_BATCH)?;
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
    }))
}

impl Net {
    fn note_trouble(&mut self, f: &GmailFailure) {
        if f.is_transient() && self.trouble.is_none() {
            self.trouble = Some(f.clone());
        }
    }

    fn stopped_by(mut self, f: GmailFailure) -> Net {
        if f.is_rate_limit() {
            self.rate_limited = true;
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

fn read_history(http: &reqwest::blocking::Client, token: &str, from: &str) -> Result<HistoryRead, GmailFailure> {
    let mut out = HistoryRead::default();
    let mut page: Option<String> = None;
    for _ in 0..HISTORY_MAX_PAGES {
        let mut url = format!(
            "{GMAIL_API}/history?startHistoryId={}&maxResults=500\
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
    section: &Section,
    page: Option<&str>,
) -> Result<(Vec<String>, Option<String>), GmailFailure> {
    let v = gmail_json(http, token, &section.list_url(page))?;
    let ids = v
        .get("messages")
        .and_then(|m| m.as_array())
        .map(|a| a.iter().filter_map(|m| str_of(m, "id")).collect())
        .unwrap_or_default();
    Ok((ids, str_of(&v, "nextPageToken")))
}

/// METADATA only - sender, subject, date, labels and Gmail's preview. The
/// body is fetched when a message is opened and never stored.
fn fetch_batch(http: &reqwest::blocking::Client, token: &str, ids: &[String]) -> Vec<(String, Fetched)> {
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
                                "{GMAIL_API}/messages/{}?format=metadata\
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

/// Everything that talks to Google, with no lock held.
fn network(plan: &Plan, client: &google_oauth::OAuthClient) -> AppResult<Net> {
    let token = access_token(client, &plan.refresh_token)?;
    let http = gmail_client()?;
    let mut net = Net::default();

    if plan.need_profile {
        match gmail_json(&http, &token, &format!("{GMAIL_API}/profile")) {
            Ok(v) => net.profile_history = str_of(&v, "historyId"),
            Err(f) => return Ok(net.stopped_by(f)),
        }
    }
    if plan.labels_due {
        match gmail_json(&http, &token, &format!("{GMAIL_API}/labels")) {
            Ok(v) => net.labels = Some(parse_labels(&v)),
            Err(f) if f.stops_the_run() || f.is_rate_limit() => return Ok(net.stopped_by(f)),
            Err(f) => net.note_trouble(&f),
        }
    }
    if let Some(from) = &plan.history_from {
        let r = read_history(&http, &token, from);
        if let Err(f) = &r {
            if f.stops_the_run() || f.is_rate_limit() {
                return Ok(net.stopped_by(f.clone()));
            }
            net.note_trouble(f);
        }
        net.history = Some(r);
    }
    if let Some((section, page)) = &plan.listing {
        let r = list_page(&http, &token, section, page.as_deref());
        if let Err(f) = &r {
            if f.stops_the_run() || f.is_rate_limit() {
                return Ok(net.stopped_by(f.clone()));
            }
            net.note_trouble(f);
        }
        net.listing = Some(r);
    }
    net.fetched = fetch_batch(&http, &token, &plan.batch);
    let mut trouble = None;
    for (_, f) in &net.fetched {
        if let Fetched::Failed(f) = f {
            if f.is_rate_limit() {
                net.rate_limited = true;
            } else if f.stops_the_run() && net.stop.is_none() {
                net.stop = Some(f.clone());
            } else if trouble.is_none() {
                trouble = Some(f.clone());
            }
        }
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

fn store_fetched(conn: &Connection, plan: &Plan, fetched: Vec<(String, Fetched)>, now: &str) -> AppResult<Stored> {
    let rules = load_rules(conn)?;
    let answered = fetched.iter().any(|(_, f)| matches!(f, Fetched::Got(..) | Fetched::Gone));
    let mut out = Stored::default();
    let mut done: Vec<String> = Vec::new();
    let mut failed: Vec<String> = Vec::new();
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
            // or Google, and counting it would push good mail out of the
            // queue every time the laptop went offline for a few minutes.
            Fetched::Failed(GmailFailure::Network) => {}
            Fetched::Failed(_) if !answered => {}
            Fetched::Failed(_) => failed.push(pid),
        }
    }
    let _ = now;
    dequeue(conn, &done)?;
    bump_attempts(conn, &failed)?;
    Ok(out)
}

/// Only what marko has switched to `auto`, only for a proposal that is
/// already `ready`, and never twice - the 2.106.0 rules, unchanged.
fn run_automation(conn: &Connection, new_rows: &[(i64, String)]) {
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
        match create_from_mail_impl(conn, *row_id, "auto") {
            Ok((k, rid)) => {
                let dest = match k.as_str() {
                    "finance" => DiscordDestination::Finance,
                    "order" => DiscordDestination::Inventory,
                    _ => DiscordDestination::Sales,
                };
                crate::commands::notifications::notify_discord(
                    conn,
                    dest,
                    "Created from email",
                    &format!("{k} #{rid} - {subject}"),
                    &[],
                );
            }
            Err(e) => {
                let _ = log_step(conn, *row_id, "auto_create", "skipped", Some(&e.to_string()));
            }
        }
    }
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

/// Where the engine is, for the screen.
pub(crate) fn progress_of(conn: &Connection) -> MailSyncProgress {
    let waiting = queue_depth(conn);
    let sections: Vec<(String, i64)> = conn
        .prepare("SELECT title, done FROM mail_sync_sections ORDER BY tier, title")
        .and_then(|mut s| {
            let rows = s.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
            rows.collect::<Result<Vec<_>, _>>()
        })
        .unwrap_or_default();
    let reading = sections.iter().find(|(_, done)| *done == 0).map(|(t, _)| t.clone());
    let stored_total: i64 = conn
        .query_row("SELECT COUNT(*) FROM mail_messages", [], |r| r.get(0))
        .unwrap_or(0);
    MailSyncProgress {
        reading: reading.or_else(|| (waiting > 0).then(|| "New mail".to_string())),
        waiting,
        folders_done: sections.iter().filter(|(_, d)| *d != 0).count() as i64,
        folders_total: sections.len() as i64,
        stored_total,
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
        more_waiting: progress.waiting > 0 || progress.folders_done < progress.folders_total,
        progress,
        ..Default::default()
    }
}

fn apply(conn: &Connection, plan: &Plan, net: Net, now: DateTime<Utc>) -> AppResult<MailSyncResult> {
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
    match &net.history {
        Some(Ok(read)) => {
            let (c, _) = apply_history(conn, &plan.account_email, &read.events, &now_s)?;
            changed |= c > 0;
            if let Some(h) = &read.checkpoint {
                conn.execute("UPDATE mail_sync_state SET last_history_id = ?1 WHERE id = 1", params![h])?;
            }
            conn.execute("UPDATE mail_sync_state SET history_checked_at = ?1 WHERE id = 1", params![now_s])?;
        }
        Some(Err(GmailFailure::NotFound)) => {
            // History expired. Every folder from the top - see the module
            // comment for why that costs listing and not downloading.
            reset_walks(conn)?;
            conn.execute(
                "UPDATE mail_sync_state SET last_history_id = NULL, history_checked_at = ?1 WHERE id = 1",
                params![now_s],
            )?;
        }
        _ => {}
    }
    let mut skipped_folder: Option<String> = None;
    if let (Some((section, page_used)), Some(result)) = (&plan.listing, &net.listing) {
        match result {
            Ok((ids, next)) => {
                changed |= apply_listing(conn, &plan.account_email, section, page_used.as_deref(), ids, next.as_deref(), &now_s)? > 0;
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

    let stored = store_fetched(conn, plan, net.fetched, &now_s)?;
    run_automation(conn, &stored.new_rows);
    backfill_finance(conn)?;
    if history_ran {
        purge_expired(conn)?;
        crate::mail_store::reclaim(conn);
    }

    let progress = progress_of(conn);
    let more = progress.waiting > 0 || progress.folders_done < progress.folders_total;
    if let Some(stop) = &net.stop {
        if *stop == GmailFailure::Unauthorized {
            forget_access_token();
        }
        record_failure(conn, &stop.message())?;
        let mut r = outcome("failed", stop.message(), &now_s, progress);
        r.stored_new = stored.stored_new;
        r.fetched = stored.fetched;
        r.next_delay_ms = (plan.interval_secs.max(60)) * 1000;
        r.changed = changed || stored.stored_new > 0;
        return Ok(r);
    }
    // Offline, or Google having a moment, and nothing at all came back:
    // said, and tried again in half a minute rather than every quarter of a
    // second.
    let reached = stored.fetched > 0
        || net.profile_history.is_some()
        || net.labels.is_some()
        || matches!(net.history, Some(Ok(_)))
        || matches!(net.listing, Some(Ok(_)));
    if !reached {
        if let Some(t) = &net.trouble {
            record_failure(conn, &t.message())?;
            let mut r = outcome("failed", t.message(), &now_s, progress);
            r.next_delay_ms = TROUBLE_DELAY_MS;
            r.changed = changed;
            return Ok(r);
        }
    }
    conn.execute(
        "UPDATE mail_sync_state
            SET last_sync_at = ?1, last_sync_count = ?2, last_error = NULL, last_error_at = NULL,
                sync_in_progress = 0, sync_started_at = NULL
          WHERE id = 1",
        params![now_s, stored.stored_new],
    )?;
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
    let next_delay_ms = if net.rate_limited {
        RATE_LIMIT_PAUSE_MS
    } else if more {
        CHAIN_DELAY_MS
    } else {
        plan.interval_secs * 1000
    };
    let mut r = outcome("synced", message, &now_s, progress);
    r.fetched = stored.fetched;
    r.stored_new = stored.stored_new;
    r.duplicates_ignored = stored.duplicates;
    r.review_required = stored.review;
    r.rate_limited = net.rate_limited;
    r.next_delay_ms = next_delay_ms;
    r.changed = changed || stored.stored_new > 0;
    Ok(r)
}

fn skipped(message: &str, at: &str, progress: MailSyncProgress) -> MailSyncResult {
    let mut r = outcome("skipped", message.to_string(), at, progress);
    r.more_waiting = false;
    r
}

/// One run of the engine. See the module comment for what it does.
pub(crate) async fn run_once(state: &AppState) -> AppResult<MailSyncResult> {
    let now = Utc::now();
    let at = now.to_rfc3339();
    let Some(_guard) = RunGuard::claim() else {
        let conn = state.db.lock().unwrap();
        return Ok(skipped("A sync is already running.", &at, progress_of(&conn)));
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
            Err(reason) => return Ok(skipped(reason, &at, progress_of(&conn))),
        }
    };
    let Some(client) = google_oauth::embedded_oauth_client() else {
        let conn = state.db.lock().unwrap();
        let msg = "This build has no Google client embedded.";
        record_failure(&conn, msg)?;
        return Ok(outcome("failed", msg.to_string(), &at, progress_of(&conn)));
    };
    let joined = tauri::async_runtime::spawn_blocking(move || {
        let net = network(&plan, &client);
        (plan, net)
    })
    .await;
    let conn = state.db.lock().unwrap();
    match joined {
        Ok((plan, Ok(net))) => apply(&conn, &plan, net, now),
        Ok((plan, Err(e))) => {
            let msg = safe_detail(&e.to_string());
            record_failure(&conn, &msg)?;
            let mut r = outcome("failed", msg, &at, progress_of(&conn));
            r.next_delay_ms = plan.interval_secs.max(60) * 1000;
            Ok(r)
        }
        Err(_) => {
            let msg = "The sync did not finish cleanly.";
            record_failure(&conn, msg)?;
            let mut r = outcome("failed", msg.to_string(), &at, progress_of(&conn));
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
        "UPDATE mail_sync_state SET last_history_id = NULL, history_checked_at = NULL, labels_checked_at = NULL WHERE id = 1",
        [],
    )?;
    forget_access_token();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_conn;
    use serde_json::json;

    const NOW: &str = "2026-10-18T12:00:00+00:00";

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

    fn label(id: &str, name: &str) -> UserLabel {
        UserLabel { gmail_id: id.to_string(), name: name.to_string() }
    }

    // ---- marko's order ----------------------------------------------------

    #[test]
    fn folders_are_read_in_markos_order() {
        let senders = vec!["viagogo.com".to_string()];
        let labels = vec![label("Label_2", "PULLS"), label("Label_1", "OASIS")];
        let keys: Vec<String> = plan_sections("all", &senders, &labels, "").into_iter().map(|s| s.key).collect();
        assert_eq!(
            keys,
            vec![
                "important", "starred", "label:label_1", "label:label_2", "finances", "inbox", "promotions",
                "spam", "trash"
            ]
        );
        let tiers: Vec<i64> = plan_sections("all", &senders, &labels, "").into_iter().map(|s| s.tier).collect();
        assert!(tiers.windows(2).all(|w| w[0] <= w[1]), "the order IS the tiers: {tiers:?}");
    }

    #[test]
    fn starred_and_labels_have_no_date_and_no_filter_in_them() {
        let labels = vec![label("Label_1", "OASIS")];
        for s in plan_sections("all", &["viagogo.com".to_string()], &labels, "-from:x.com has:attachment") {
            if s.key == "starred" || s.key.starts_with("label:") {
                assert!(s.q.is_none(), "{} must be a plain label walk: {:?}", s.key, s.q);
                assert!(s.flag.is_some());
                assert!(!s.list_url(None).contains("after"), "{}", s.list_url(None));
            }
        }
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
    }

    #[test]
    fn the_known_scope_reads_only_the_mail_that_matters() {
        let keys: Vec<String> = plan_sections("known", &["viagogo.com".to_string()], &[label("Label_1", "A")], "")
            .into_iter()
            .map(|s| s.key)
            .collect();
        assert_eq!(keys, vec!["important", "starred", "label:label_1", "finances"]);
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
            let url = s.list_url(None);
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
        let url = s.list_url(Some("tok en"));
        assert!(!url.contains(' '), "{url}");
        assert!(url.contains("pageToken=tok+en"), "{url}");
        assert!(url.contains("from:paypal.com"), "{url}");
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

    // ---- the queue --------------------------------------------------------

    #[test]
    fn the_queue_skips_what_is_stored_or_deleted_and_drains_in_markos_order() {
        let conn = test_conn();
        insert(&conn, "stored", "inbox", false, NOW);
        conn.execute("INSERT INTO mail_deleted (provider_message_id, deleted_at) VALUES ('deleted', 'x')", [])
            .unwrap();
        enqueue(&conn, "me", &["i1".into(), "i2".into(), "stored".into(), "deleted".into()], TIER_INBOX, NOW)
            .unwrap();
        enqueue(&conn, "me", &["s1".into(), "s2".into()], TIER_STARRED, NOW).unwrap();
        // The same id from a better folder keeps the better place.
        enqueue(&conn, "me", &["i2".into()], TIER_STARRED, NOW).unwrap();
        assert_eq!(peek(&conn, 10).unwrap(), vec!["i2", "s1", "s2", "i1"]);
        assert_eq!(queue_depth(&conn), 4);
    }

    #[test]
    fn inside_one_folder_the_newest_comes_first() {
        let conn = test_conn();
        // Gmail lists newest first, page after page.
        enqueue(&conn, "me", &["newest".into(), "newer".into()], TIER_INBOX, NOW).unwrap();
        enqueue(&conn, "me", &["older".into(), "oldest".into()], TIER_INBOX, NOW).unwrap();
        assert_eq!(peek(&conn, 10).unwrap(), vec!["newest", "newer", "older", "oldest"]);
    }

    #[test]
    fn a_message_that_keeps_failing_eventually_gives_its_slot_up() {
        let conn = test_conn();
        enqueue(&conn, "me", &["bad".into(), "good".into()], TIER_INBOX, NOW).unwrap();
        for _ in 0..MAX_ATTEMPTS {
            bump_attempts(&conn, &["bad".into()]).unwrap();
        }
        assert_eq!(peek(&conn, 10).unwrap(), vec!["good"]);
    }

    // ---- folder walks -----------------------------------------------------

    fn starred_section() -> Section {
        plan_sections("all", &[], &[], "").into_iter().find(|s| s.key == "starred").unwrap()
    }

    #[test]
    fn walking_starred_marks_mail_that_was_stored_before_it_was_starred() {
        let conn = test_conn();
        insert(&conn, "old", "inbox", false, "2026-01-01T00:00:00+00:00");
        let s = starred_section();
        sync_section_rows(&conn, &[s.clone()], NOW).unwrap();
        apply_listing(&conn, "me", &s, None, &["old".into(), "new".into()], Some("p2"), NOW).unwrap();
        let (labels, starred) = labels_of(&conn, "old");
        assert_eq!(starred, 1);
        assert!(labels.split(',').any(|l| l == "starred"), "{labels}");
        // The one not stored yet is queued at Starred's place.
        assert_eq!(peek(&conn, 10).unwrap(), vec!["new"]);
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
        apply_listing(&conn, "me", &s, None, &["still".into()], Some("p2"), NOW).unwrap();
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
    fn walks_resume_where_they_stopped_and_follow_markos_order() {
        let conn = test_conn();
        let sections = plan_sections("all", &["viagogo.com".into()], &[], "");
        sync_section_rows(&conn, &sections, NOW).unwrap();
        let (first, page) = next_walk(&conn, &sections).unwrap().unwrap();
        assert_eq!((first.key.as_str(), page), ("important", None));
        apply_listing(&conn, "me", &first, None, &["a".into()], Some("TOKEN2"), NOW).unwrap();
        let (again, page) = next_walk(&conn, &sections).unwrap().unwrap();
        assert_eq!((again.key.as_str(), page.as_deref()), ("important", Some("TOKEN2")));
        apply_listing(&conn, "me", &again, Some("TOKEN2"), &[], None, NOW).unwrap();
        let (next, _) = next_walk(&conn, &sections).unwrap().unwrap();
        assert_eq!(next.key, "starred");
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
        // New mail first, the draft never, spam at the back.
        assert_eq!(peek(&conn, 10).unwrap(), vec!["n1", "sp"]);
    }

    #[test]
    fn an_old_message_starred_in_gmail_is_fetched_even_though_no_walk_will_meet_it() {
        let conn = test_conn();
        let ev = vec![HistoryEvent::LabelsAdded { id: "ancient".into(), labels: vec!["starred".into()] }];
        apply_history(&conn, "me", &ev, NOW).unwrap();
        let tier: i64 = conn
            .query_row("SELECT priority FROM mail_fetch_queue WHERE provider_message_id = 'ancient'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(tier, TIER_STARRED);
    }

    #[test]
    fn junk_keeps_its_late_place_even_when_it_is_new() {
        assert_eq!(new_mail_tier(&["inbox".into()]), TIER_NEW);
        assert_eq!(new_mail_tier(&["spam".into()]), TIER_SPAM);
        assert_eq!(new_mail_tier(&["category_promotions".into()]), TIER_PROMOTIONS);
        assert_eq!(new_mail_tier(&["category_promotions".into(), "starred".into()]), TIER_NEW);
    }

    // ---- what is kept -----------------------------------------------------

    fn fetched(from: &str, subject: &str, labels: &str) -> FetchedMessage {
        FetchedMessage {
            provider_message_id: "x".into(),
            provider_thread_id: None,
            from_address: from.into(),
            from_name: None,
            subject: subject.into(),
            snippet: None,
            received_at: NOW.into(),
            body_text: String::new(),
            is_unread: true,
            labels: labels.into(),
            is_starred: labels.contains("starred"),
            from_raw: from.into(),
            to_address: None,
            reply_to: None,
        }
    }

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

    #[test]
    fn progress_names_the_folder_being_read() {
        let conn = test_conn();
        let sections = plan_sections("all", &[], &[], "");
        sync_section_rows(&conn, &sections, NOW).unwrap();
        conn.execute("UPDATE mail_sync_sections SET done = 1 WHERE key = 'starred'", []).unwrap();
        let p = progress_of(&conn);
        assert_eq!(p.reading.as_deref(), Some("Finances"));
        assert_eq!(p.folders_done, 1);
        assert_eq!(p.folders_total, sections.len() as i64);
    }

    // ---- when Gmail cannot be reached -------------------------------------

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
            batch: batch.iter().map(|s| s.to_string()).collect(),
            interval_secs: 60,
        }
    }

    fn attempts(conn: &Connection, pid: &str) -> i64 {
        conn.query_row(
            "SELECT attempts FROM mail_fetch_queue WHERE provider_message_id = ?1",
            params![pid],
            |r| r.get(0),
        )
        .unwrap()
    }

    #[test]
    fn an_hour_offline_pushes_nothing_out_of_the_queue() {
        let conn = test_conn();
        enqueue(&conn, "me", &["a".into(), "b".into()], TIER_STARRED, NOW).unwrap();
        let plan = plan_with(&["a", "b"]);
        for _ in 0..MAX_ATTEMPTS * 3 {
            let got = vec![
                ("a".to_string(), Fetched::Failed(GmailFailure::Network)),
                ("b".to_string(), Fetched::Skipped),
            ];
            store_fetched(&conn, &plan, got, NOW).unwrap();
        }
        assert_eq!(peek(&conn, 10).unwrap(), vec!["a", "b"]);
        assert_eq!(attempts(&conn, "a"), 0);
    }

    #[test]
    fn a_google_error_counts_against_a_message_only_while_google_answers_for_others() {
        let conn = test_conn();
        enqueue(&conn, "me", &["bad".into(), "good".into()], TIER_INBOX, NOW).unwrap();
        let plan = plan_with(&["bad", "good"]);
        // Google failing for everything: not this message's fault.
        for _ in 0..MAX_ATTEMPTS {
            store_fetched(&conn, &plan, vec![("bad".into(), Fetched::Failed(GmailFailure::Server(503)))], NOW)
                .unwrap();
        }
        assert_eq!(attempts(&conn, "bad"), 0);
        // Google fine for the other one: this message is the problem.
        let m = fetched("a@b.com", "hello", "inbox");
        let c = classify_fetched(&m);
        store_fetched(
            &conn,
            &plan,
            vec![
                ("bad".into(), Fetched::Failed(GmailFailure::Server(500))),
                ("good".into(), Fetched::Got(Box::new(m), c)),
            ],
            NOW,
        )
        .unwrap();
        assert_eq!(attempts(&conn, "bad"), 1);
        assert_eq!(peek(&conn, 10).unwrap(), vec!["bad"], "the one that came down left the queue");
    }

    #[test]
    fn offline_says_so_and_waits_instead_of_spinning() {
        let conn = test_conn();
        sync_section_rows(&conn, &plan_sections("all", &[], &[], ""), NOW).unwrap();
        enqueue(&conn, "me", &["a".into()], TIER_STARRED, NOW).unwrap();
        let now = DateTime::parse_from_rfc3339(NOW).unwrap().with_timezone(&Utc);
        let net = Net {
            fetched: vec![("a".into(), Fetched::Failed(GmailFailure::Network))],
            trouble: Some(GmailFailure::Network),
            ..Default::default()
        };
        let r = apply(&conn, &plan_with(&["a"]), net, now).unwrap();
        assert_eq!(r.status, "failed");
        assert_eq!(r.next_delay_ms, TROUBLE_DELAY_MS);
        assert!(r.message.contains("Could not reach Gmail"), "{}", r.message);
        assert_eq!(peek(&conn, 10).unwrap(), vec!["a"], "still queued");

        // A page listed before the connection went: that counts as reaching
        // Gmail, so the next run follows straight away.
        let starred = starred_section();
        let net = Net {
            listing: Some(Ok((vec!["n1".into()], Some("p2".into())))),
            fetched: vec![("a".into(), Fetched::Failed(GmailFailure::Network))],
            trouble: Some(GmailFailure::Network),
            ..Default::default()
        };
        let mut plan = plan_with(&["a"]);
        plan.listing = Some((starred, None));
        let r = apply(&conn, &plan, net, now).unwrap();
        assert_eq!(r.status, "synced");
        assert_eq!(r.next_delay_ms, CHAIN_DELAY_MS);
    }
}

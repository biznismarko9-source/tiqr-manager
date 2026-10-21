//! 2.115.0 - two computers, one Gmail reader: the Rust half.
//!
//! marko: "aj ten mail funguje na oboch zariadeniach rozne, chcem to spojit do
//! jedneho lebo vkuse to je ze google asked tiqr to slowed down" - and, asked
//! how: "najlepsie ak to vedia robit oba podla toho ktory prave runnuje, ak
//! naraz tak len jeden a ten to bude posielat tomu druhemu aby to bolo
//! obmedzene".
//!
//! Gmail lets one account read about 50 messages a second. One computer sits
//! just under that; two reading the same mailbox at once go over it, Google
//! slows both down, both wait, both start again together - round and round.
//! So while both run, ONE reads Gmail - the one holding the lease
//! (src/lib/mailShare.ts) - and sends the other what it stored, through the
//! account's own Firestore space, a batch at a time; the other reads nothing
//! from Gmail and stores what arrives. A computer running alone reads Gmail
//! itself, as before.
//!
//! This is what goes out (`export`), what is done with what comes in
//! (`import`), and the switch that stops the reader while another computer
//! reads for this one (`mail_sync::set_following`).
//!
//! What travels is what Gmail gave the computer that read it - sender,
//! subject, preview, date, labels, read and starred - and the text TIQR keeps
//! for a message that could become a record. Nothing TIQR decided about a
//! message travels: no category, no review, no record made, no OTP code as
//! such, nowhere marko moved it. The computer that receives a message reads
//! it itself, exactly as if it had come from Gmail - its rules, its
//! Automatic, its folders. And no Discord message: the computer that read it
//! from Gmail announced it already.
use crate::commands::mail::{
    apply_rules, classify_fetched, load_rules, run_rule_action, upsert_message, FetchedMessage, MailProviderKind,
    RuleSubject, MAIL_ACCOUNT_EMAIL_KEY,
};
use crate::commands::mail_sync;
use crate::commands::sheets_sync::{get_setting, set_setting};
use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::models::MailConnectionStatus;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tauri::State;

/// On unless marko turns it off - for both computers (shared_settings.rs).
pub(crate) const MAIL_SHARE_KEY: &str = "mail_share";

/// Rows read per query while a batch is filled.
const ROWS_PER_QUERY: i64 = 400;

/// What one batch may weigh, as JSON. Firestore takes 1 MiB a document and
/// this stays well under it, accents and all.
pub(crate) const MAX_BATCH_BYTES: usize = 600_000;

/// What travels for one message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SharedMail {
    /// Gmail's message id.
    pub id: String,
    pub thread: Option<String>,
    pub from_raw: Option<String>,
    pub from_address: String,
    pub from_name: Option<String>,
    pub to: Option<String>,
    pub reply_to: Option<String>,
    pub subject: String,
    pub snippet: Option<String>,
    pub received_at: String,
    /// Gmail's labels, lower-cased and comma separated.
    pub labels: String,
    pub unread: bool,
    pub starred: bool,
    /// When Gmail last said what `labels` says - kept as it came, so of two
    /// computers' views of a message the newer is always known.
    pub labels_at: String,
    /// The text kept to read figures from (`extract_text`), when there is one.
    pub text: Option<String>,
}

/// How far a receiving computer has got with one sender: everything up to
/// and including this (`labels_at`, row id) of the SENDER's.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ShareCursor {
    pub at: String,
    pub id: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareBatch {
    pub account: String,
    pub rows: Vec<SharedMail>,
    pub up_to: ShareCursor,
    /// More may follow straight away.
    pub more: bool,
}

#[derive(Debug, Serialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ShareImported {
    pub stored_new: usize,
    pub updated: usize,
    /// Everything received from that computer so far.
    pub received: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareInfo {
    pub enabled: bool,
    pub connected: bool,
    pub account: String,
    /// This computer's id (live_sync.rs), `None` for a database with no file.
    pub device: Option<String>,
    /// Mail is paused here: this computer neither reads for the other one
    /// nor gets mail from it.
    pub paused: bool,
}

pub(crate) fn share_enabled(conn: &Connection) -> bool {
    get_setting(conn, MAIL_SHARE_KEY).ok().flatten().as_deref() != Some("off")
}

fn account_of(conn: &Connection) -> AppResult<String> {
    Ok(get_setting(conn, MAIL_ACCOUNT_EMAIL_KEY)?.unwrap_or_default())
}

pub(crate) fn info(conn: &Connection) -> AppResult<ShareInfo> {
    Ok(ShareInfo {
        enabled: share_enabled(conn),
        connected: crate::secrets::get(conn, crate::commands::mail::MAIL_REFRESH_TOKEN_KEY)?.is_some(),
        account: account_of(conn)?,
        device: crate::commands::mail::this_device(conn),
        paused: conn
            .query_row("SELECT paused FROM mail_sync_state WHERE id = 1", [], |r| r.get::<_, i64>(0))
            .optional()?
            .unwrap_or(0)
            != 0,
    })
}

/// The next batch to send: this mailbox's messages after `after`, in
/// (`labels_at`, id) order - every message once, and again whenever Gmail
/// changes its labels - up to `max_bytes` of JSON.
pub(crate) fn export(conn: &Connection, after: &ShareCursor, max_bytes: usize) -> AppResult<ShareBatch> {
    let account = account_of(conn)?;
    let mut rows: Vec<SharedMail> = Vec::new();
    let mut up_to = after.clone();
    let mut bytes = 0usize;
    let mut stmt = conn.prepare(
        "SELECT provider_message_id, provider_thread_id, from_raw, from_address, from_name, to_address, reply_to,
                subject, snippet, received_at, IFNULL(labels, ''), is_unread, is_starred, labels_at, extract_text, id
           FROM mail_messages
          WHERE account_email = ?1 AND labels_at IS NOT NULL AND (labels_at, id) > (?2, ?3)
          ORDER BY labels_at, id
          LIMIT ?4",
    )?;
    loop {
        let page: Vec<(SharedMail, i64)> = stmt
            .query_map(params![account, up_to.at, up_to.id, ROWS_PER_QUERY], |r| {
                Ok((
                    SharedMail {
                        id: r.get(0)?,
                        thread: r.get(1)?,
                        from_raw: r.get(2)?,
                        from_address: r.get(3)?,
                        from_name: r.get(4)?,
                        to: r.get(5)?,
                        reply_to: r.get(6)?,
                        subject: r.get(7)?,
                        snippet: r.get(8)?,
                        received_at: r.get(9)?,
                        labels: r.get(10)?,
                        unread: r.get::<_, i64>(11)? != 0,
                        starred: r.get::<_, i64>(12)? != 0,
                        labels_at: r.get(13)?,
                        text: r.get(14)?,
                    },
                    r.get::<_, i64>(15)?,
                ))
            })?
            .collect::<Result<_, _>>()?;
        let full_page = page.len() as i64 == ROWS_PER_QUERY;
        for (mail, id) in page {
            let size = serde_json::to_string(&mail).map(|s| s.len()).unwrap_or(0);
            // Never an empty batch for one big message; never past the budget otherwise.
            if !rows.is_empty() && bytes + size > max_bytes {
                return Ok(ShareBatch { account, rows, up_to, more: true });
            }
            bytes += size;
            up_to = ShareCursor { at: mail.labels_at.clone(), id };
            rows.push(mail);
        }
        if !full_page {
            return Ok(ShareBatch { account, rows, up_to, more: false });
        }
    }
}

/// A Gmail id as Gmail writes them.
fn gmail_id(s: &str) -> bool {
    !s.is_empty() && s.len() <= 64 && s.chars().all(|c| c.is_ascii_alphanumeric())
}

fn a_time(s: &str) -> bool {
    DateTime::parse_from_rfc3339(s).is_ok()
}

/// A computer's id as live_sync.rs makes them.
fn a_device(s: &str) -> bool {
    s.len() == 32 && s.chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
}

fn labels_of(s: &str) -> HashSet<&str> {
    s.split(',').map(str::trim).filter(|l| !l.is_empty()).collect()
}

fn a_label(l: &str) -> bool {
    l.len() <= 64 && l.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

/// What the other computer sent is checked like anything else from outside:
/// a message with an id, a date or a label TIQR would not store from Gmail
/// itself is left out, and long text is cut.
fn checked(mut m: SharedMail) -> Option<SharedMail> {
    if !gmail_id(&m.id) || !a_time(&m.received_at) || !a_time(&m.labels_at) || !labels_of(&m.labels).iter().all(|l| a_label(l)) {
        return None;
    }
    if m.thread.as_deref().is_some_and(|t| !gmail_id(t)) {
        m.thread = None;
    }
    let cut = |s: &mut String, n: usize| {
        if s.chars().count() > n {
            *s = s.chars().take(n).collect();
        }
    };
    cut(&mut m.subject, 2_000);
    cut(&mut m.from_address, 320);
    for s in [&mut m.snippet, &mut m.from_name, &mut m.from_raw, &mut m.to, &mut m.reply_to].into_iter().flatten() {
        cut(s, 2_000);
    }
    if let Some(t) = m.text.as_mut() {
        cut(t, crate::commands::mail::EXTRACT_TEXT_CHARS);
    }
    Some(m)
}

/// Stores what another computer sent - new messages as if they had just come
/// from Gmail, and a newer view of a stored one's labels as a change Gmail
/// made - and remembers how far it got with that computer. One transaction:
/// a batch is in whole, or not at all and sent again.
pub(crate) fn import(conn: &Connection, sender: &str, account: &str, rows: Vec<SharedMail>, up_to: &ShareCursor) -> AppResult<ShareImported> {
    let ours = account_of(conn)?;
    if ours.is_empty() || !ours.eq_ignore_ascii_case(account) {
        return Err(AppError::Validation("That mail is from another mailbox than the one connected here.".into()));
    }
    if !a_device(sender) {
        return Err(AppError::Validation("That is not a computer TIQR knows.".into()));
    }
    let now = Utc::now().to_rfc3339();
    let mut out = ShareImported::default();
    let notices = {
        let tx = conn.unchecked_transaction()?;
        let rules = load_rules(&tx)?;
        let mut new_rows: Vec<(i64, String)> = Vec::new();
        let mut seen: Vec<String> = Vec::new();
        for m in rows.into_iter().filter_map(checked) {
            seen.push(m.id.clone());
            let stored: Option<(String, Option<String>)> = tx
                .query_row(
                    "SELECT IFNULL(labels, ''), labels_at FROM mail_messages WHERE provider_message_id = ?1",
                    params![m.id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()?;
            if let Some((labels, at)) = stored {
                // Only a NEWER view of Gmail's labels: the other computer may
                // have been off while this one saw a later change.
                if at.as_deref().is_some_and(|a| m.labels_at.as_str() <= a) {
                    continue;
                }
                let (have, want) = (labels_of(&labels), labels_of(&m.labels));
                let mut changed = 0;
                for l in want.difference(&have) {
                    changed += mail_sync::add_flag(&tx, &m.id, l, &m.labels_at, true)?;
                }
                for l in have.difference(&want) {
                    changed += mail_sync::remove_flag(&tx, &m.id, l, &m.labels_at, true)?;
                }
                tx.execute("UPDATE mail_messages SET labels_at = ?2 WHERE provider_message_id = ?1", params![m.id, m.labels_at])?;
                if changed > 0 {
                    out.updated += 1;
                }
                continue;
            }
            let msg = FetchedMessage {
                provider_message_id: m.id.clone(),
                provider_thread_id: m.thread.clone(),
                from_raw: m.from_raw.clone().unwrap_or_else(|| m.from_address.clone()),
                from_address: m.from_address.clone(),
                from_name: m.from_name.clone(),
                subject: m.subject.clone(),
                snippet: m.snippet.clone(),
                received_at: m.received_at.clone(),
                body_text: m.text.clone().unwrap_or_default(),
                is_unread: m.unread,
                labels: m.labels.clone(),
                is_starred: m.starred,
                to_address: m.to.clone(),
                reply_to: m.reply_to.clone(),
            };
            let class = classify_fetched(&msg);
            let (row_id, was_new) = upsert_message(&tx, MailProviderKind::Gmail, &ours, &msg, &class)?;
            if !was_new {
                continue;
            }
            tx.execute("UPDATE mail_messages SET labels_at = ?2 WHERE id = ?1", params![row_id, m.labels_at])?;
            out.stored_new += 1;
            // The rules set here, on new mail - as for mail read from Gmail here.
            if !rules.is_empty() {
                let subject = RuleSubject {
                    from: &msg.from_address,
                    subject: &msg.subject,
                    preview: msg.snippet.as_deref().unwrap_or_default(),
                };
                if let Some(hit) = apply_rules(&rules, &subject) {
                    let _ = run_rule_action(&tx, row_id, &hit);
                }
            }
            new_rows.push((row_id, msg.subject.clone()));
        }
        // Found here before, or by this computer's own walks: no need to
        // download any of it from Gmail any more.
        mail_sync::dequeue(&tx, &seen)?;
        let notices = mail_sync::run_automation(&tx, &new_rows);
        tx.execute(
            "INSERT INTO mail_share_cursors (sender, at, id, received, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(sender) DO UPDATE SET at = excluded.at, id = excluded.id,
                 received = received + excluded.received, updated_at = excluded.updated_at",
            params![sender, up_to.at, up_to.id, seen.len() as i64, now],
        )?;
        out.received = tx.query_row("SELECT received FROM mail_share_cursors WHERE sender = ?1", params![sender], |r| r.get(0))?;
        tx.commit()?;
        notices
    };
    // Only records Automatic made here - never "mail arrived", which the
    // computer that read it from Gmail announced.
    for (subject, occasion) in notices {
        crate::commands::discord::announce(conn, subject, occasion);
    }
    Ok(out)
}

/// Where this computer got to with `sender`. The first time, where it
/// starts - written down at once, so it never moves on by itself.
pub(crate) fn cursor(conn: &Connection, sender: &str) -> AppResult<(ShareCursor, i64)> {
    if !a_device(sender) {
        return Err(AppError::Validation("That is not a computer TIQR knows.".into()));
    }
    let read = || {
        conn.query_row("SELECT at, id, received FROM mail_share_cursors WHERE sender = ?1", params![sender], |r| {
            Ok((ShareCursor { at: r.get(0)?, id: r.get(1)? }, r.get(2)?))
        })
        .optional()
    };
    if let Some(got) = read()? {
        return Ok(got);
    }
    let start = first_cursor(conn);
    conn.execute(
        "INSERT OR IGNORE INTO mail_share_cursors (sender, at, id, received, updated_at) VALUES (?1, ?2, 0, 0, ?3)",
        params![sender, start.at, Utc::now().to_rfc3339()],
    )?;
    Ok(read()?.unwrap_or((start, 0)))
}

/// How long before this computer's last read of Gmail a computer it has never
/// followed starts sending - for label changes that run did not reach, and
/// for two clocks that do not quite agree.
const FIRST_CURSOR_BACK_DAYS: i64 = 7;

/// Where a computer starts with one it has never followed. When its own
/// reading has been through every folder - none set aside - nothing found
/// waits to be downloaded and new mail is followed from Gmail's history, it
/// has all that Gmail had at its last read; so the other computer sends what
/// changed from a week before that, not the whole mailbox once more. Anything
/// less, and it starts from the very beginning.
pub(crate) fn first_cursor(conn: &Connection) -> ShareCursor {
    let read_through = conn
        .query_row(
            "SELECT (SELECT COUNT(*) FROM mail_sync_sections) > 0
                AND NOT EXISTS (SELECT 1 FROM mail_sync_sections WHERE done = 0 OR set_aside_at IS NOT NULL)
                AND NOT EXISTS (SELECT 1 FROM mail_fetch_queue)
                AND (SELECT last_history_id IS NOT NULL FROM mail_sync_state WHERE id = 1)",
            [],
            |r| r.get::<_, Option<bool>>(0),
        )
        .ok()
        .flatten()
        .unwrap_or(false);
    let last = conn
        .query_row("SELECT last_sync_at FROM mail_sync_state WHERE id = 1", [], |r| r.get::<_, Option<String>>(0))
        .ok()
        .flatten()
        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok());
    match last {
        Some(at) if read_through => ShareCursor {
            at: (at.with_timezone(&Utc) - chrono::Duration::days(FIRST_CURSOR_BACK_DAYS)).to_rfc3339(),
            id: 0,
        },
        _ => ShareCursor::default(),
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareProgress {
    pub up_to: ShareCursor,
    pub received: i64,
}

#[tauri::command]
pub fn mail_share_info(state: State<AppState>) -> AppResult<ShareInfo> {
    let conn = state.db.lock().unwrap();
    info(&conn)
}

#[tauri::command]
pub fn mail_share_export(state: State<AppState>, after: ShareCursor, max_bytes: Option<usize>) -> AppResult<ShareBatch> {
    let conn = state.db.lock().unwrap();
    export(&conn, &after, max_bytes.unwrap_or(MAX_BATCH_BYTES).clamp(10_000, MAX_BATCH_BYTES))
}

/// Stores a batch, and tells the window the way a run does (`mail-sync`).
#[tauri::command]
pub fn mail_share_import(
    app: tauri::AppHandle,
    state: State<AppState>,
    sender: String,
    account: String,
    rows: Vec<SharedMail>,
    up_to: ShareCursor,
) -> AppResult<ShareImported> {
    use tauri::Emitter;
    let conn = state.db.lock().unwrap();
    let got = import(&conn, &sender, &account, rows, &up_to)?;
    let r = mail_sync::following_result(&conn, got.stored_new as i64, got.stored_new + got.updated > 0);
    let _ = app.emit(mail_sync::EVENT_MAIL_SYNC, &r);
    Ok(got)
}

#[tauri::command]
pub fn mail_share_cursor(state: State<AppState>, sender: String) -> AppResult<ShareProgress> {
    let conn = state.db.lock().unwrap();
    let (up_to, received) = cursor(&conn, &sender)?;
    Ok(ShareProgress { up_to, received })
}

/// The other computer reads Gmail for this one (`on`), renewed by the window
/// every minute - or no longer.
#[tauri::command]
pub fn mail_share_follow(on: bool) {
    mail_sync::set_following(on);
}

/// Mail -> Settings: share the reading between computers, or not - for both.
#[tauri::command]
pub fn set_mail_share(state: State<AppState>, on: bool) -> AppResult<MailConnectionStatus> {
    let conn = state.db.lock().unwrap();
    set_setting(&conn, MAIL_SHARE_KEY, if on { "on" } else { "off" })?;
    crate::commands::shared_settings::touch(&conn, &crate::commands::shared_settings::MAIL)?;
    if !on {
        mail_sync::set_following(false);
    }
    crate::commands::mail::status_impl(&conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_conn;

    const ME: &str = "biznismarko9@gmail.com";
    const MAC: &str = "0123456789abcdef0123456789abcdef";

    fn computer() -> Connection {
        let conn = test_conn();
        set_setting(&conn, MAIL_ACCOUNT_EMAIL_KEY, ME).unwrap();
        conn
    }

    /// A message stored as the engine stores one from Gmail, its labels as
    /// Gmail said them at `labels_at`.
    fn got(conn: &Connection, id: &str, from: &str, subject: &str, labels: &str, labels_at: &str) -> i64 {
        let msg = FetchedMessage {
            provider_message_id: id.into(),
            provider_thread_id: Some(format!("t{id}")),
            from_raw: format!("Shop <{from}>"),
            from_address: from.into(),
            from_name: Some("Shop".into()),
            subject: subject.into(),
            snippet: Some(format!("about {subject}")),
            received_at: "2026-10-01T10:00:00+00:00".into(),
            body_text: if subject.contains("sold") { "You sold 2 tickets for 412.50 EUR".into() } else { String::new() },
            is_unread: labels.contains("unread"),
            labels: labels.into(),
            is_starred: labels.contains("starred"),
            to_address: Some(ME.into()),
            reply_to: None,
        };
        let class = classify_fetched(&msg);
        let (row, _) = upsert_message(conn, MailProviderKind::Gmail, ME, &msg, &class).unwrap();
        conn.execute("UPDATE mail_messages SET labels_at = ?2 WHERE id = ?1", params![row, labels_at]).unwrap();
        row
    }

    fn ids(conn: &Connection) -> Vec<String> {
        conn.prepare("SELECT provider_message_id FROM mail_messages ORDER BY provider_message_id")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    fn row(conn: &Connection, id: &str) -> (String, i64, i64, String) {
        conn.query_row(
            "SELECT IFNULL(labels, ''), is_unread, is_starred, category FROM mail_messages WHERE provider_message_id = ?1",
            params![id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap()
    }

    #[test]
    fn the_other_computer_receives_everything_in_batches_and_reads_each_message_itself() {
        let mac = computer();
        for i in 0..40 {
            got(&mac, &format!("m{i:03}"), "news@shop.com", &format!("Weekly news {i}"), "inbox", &format!("2026-10-01T10:00:{i:02}+00:00"));
        }
        got(&mac, "sale1", "noreply@viagogo.com", "Congratulations, you sold 2 tickets", "inbox,unread", "2026-10-01T10:01:00+00:00");
        let windows = computer();
        // A rule there stars the newsletters - and runs on what arrives, as on mail from Gmail.
        windows
            .execute(
                "INSERT INTO mail_rules (id, position, enabled, field, match_kind, needle, action, created_at)
                 VALUES ('r1', 1, 1, 'subject', 'contains', 'weekly news 7', 'star', 'x')",
                [],
            )
            .unwrap();

        let mut cursor = ShareCursor::default();
        let mut batches = 0;
        loop {
            let b = export(&mac, &cursor, 4_000).unwrap();
            assert_eq!(b.account, ME);
            if b.rows.is_empty() {
                assert!(!b.more);
                break;
            }
            batches += 1;
            let got = import(&windows, MAC, &b.account, b.rows, &b.up_to).unwrap();
            assert_eq!(got.updated, 0);
            cursor = b.up_to;
            if !b.more {
                break;
            }
        }
        assert!(batches > 1, "a small budget sends several batches");
        assert_eq!(ids(&windows), ids(&mac), "every message arrived");
        assert_eq!(row(&windows, "sale1"), ("inbox,unread".into(), 1, 0, "sale_confirmation".into()), "read there as a sale");
        let text: Option<String> = windows
            .query_row("SELECT extract_text FROM mail_messages WHERE provider_message_id = 'sale1'", [], |r| r.get(0))
            .unwrap();
        assert!(text.unwrap_or_default().contains("412.50"), "the text figures are read from came too");
        assert_eq!(row(&windows, "m007").2, 1, "Windows' own rule ran on it");
        let (at, received) = super::cursor(&windows, MAC).unwrap();
        assert_eq!((at, received), (cursor.clone(), 41));
        // Nothing new: nothing to send.
        let b = export(&mac, &cursor, MAX_BATCH_BYTES).unwrap();
        assert!(b.rows.is_empty() && !b.more);
    }

    #[test]
    fn a_newer_view_of_gmails_labels_is_taken_and_an_older_one_never() {
        let mac = computer();
        let windows = computer();
        got(&windows, "m1", "news@shop.com", "Hello", "inbox,unread", "2026-10-02T09:00:00+00:00");
        got(&mac, "m1", "news@shop.com", "Hello", "inbox,starred", "2026-10-02T10:00:00+00:00");
        let b = export(&mac, &ShareCursor::default(), MAX_BATCH_BYTES).unwrap();
        let r = import(&windows, MAC, ME, b.rows, &b.up_to).unwrap();
        assert_eq!((r.stored_new, r.updated), (0, 1));
        let (labels, unread, starred, _) = row(&windows, "m1");
        assert_eq!((labels_of(&labels), unread, starred), (labels_of("inbox,starred"), 0, 1), "read and starred in Gmail since");

        // An older view - a computer that was off - changes nothing.
        let old = vec![SharedMail { labels: "inbox,unread".into(), labels_at: "2026-10-01T00:00:00+00:00".into(), ..b_row(&mac, "m1") }];
        let r = import(&windows, MAC, ME, old, &ShareCursor::default()).unwrap();
        assert_eq!(r.updated, 0);
        assert_eq!(row(&windows, "m1").2, 1);
    }

    fn b_row(conn: &Connection, id: &str) -> SharedMail {
        export(conn, &ShareCursor::default(), MAX_BATCH_BYTES).unwrap().rows.into_iter().find(|m| m.id == id).unwrap()
    }

    #[test]
    fn mail_of_another_mailbox_or_a_message_tiqr_would_not_take_from_gmail_is_never_stored() {
        let mac = computer();
        got(&mac, "ok1", "news@shop.com", "Fine", "inbox", "2026-10-02T10:00:00+00:00");
        let windows = computer();
        let good = b_row(&mac, "ok1");
        assert!(import(&windows, MAC, "someone.else@gmail.com", vec![good.clone()], &ShareCursor::default()).is_err());
        assert!(import(&windows, "../../etc", ME, vec![good.clone()], &ShareCursor::default()).is_err());
        let bad = vec![
            SharedMail { id: "x'; DROP TABLE mail_messages; --".into(), ..good.clone() },
            SharedMail { id: "baddate".into(), received_at: "yesterday".into(), ..good.clone() },
            SharedMail { id: "badlabel".into(), labels: "inbox,<b>".into(), ..good.clone() },
            good,
        ];
        let r = import(&windows, MAC, ME, bad, &ShareCursor::default()).unwrap();
        assert_eq!(r.stored_new, 1);
        assert_eq!(ids(&windows), vec!["ok1"]);
    }

    #[test]
    fn a_computer_that_read_its_whole_mailbox_is_sent_only_the_last_week_and_any_other_everything() {
        const WIN: &str = "fedcba9876543210fedcba9876543210";
        // Windows, 16,000 of 100,000 read: everything, from the very beginning.
        let windows = computer();
        windows
            .execute("INSERT INTO mail_sync_sections (key, title, tier, spec, done, page_token) VALUES ('inbox', 'Inbox', 10, '', 0, 'P')", [])
            .unwrap();
        windows.execute("UPDATE mail_sync_state SET last_sync_at = '2026-10-20T12:00:00+00:00', last_history_id = '9' WHERE id = 1", []).unwrap();
        assert_eq!(super::cursor(&windows, MAC).unwrap(), (ShareCursor::default(), 0));

        // The Mac, read to its oldest message: a week before its last read.
        let mac = computer();
        mac.execute("INSERT INTO mail_sync_sections (key, title, tier, spec, done) VALUES ('inbox', 'Inbox', 10, '', 1)", []).unwrap();
        mac.execute("UPDATE mail_sync_state SET last_sync_at = '2026-10-20T12:00:00+00:00', last_history_id = '9' WHERE id = 1", []).unwrap();
        let start = ShareCursor { at: "2026-10-13T12:00:00+00:00".into(), id: 0 };
        assert_eq!(first_cursor(&mac), start);
        assert_eq!(super::cursor(&mac, WIN).unwrap(), (start.clone(), 0));
        // Written down the first time: a later read of Gmail does not move it on.
        mac.execute("UPDATE mail_sync_state SET last_sync_at = '2026-10-21T08:00:00+00:00' WHERE id = 1", []).unwrap();
        assert_eq!(super::cursor(&mac, WIN).unwrap().0, start);

        // Not read through after all: a folder Gmail refused, a message still to download.
        mac.execute("UPDATE mail_sync_sections SET set_aside_at = '2026-10-20T11:00:00+00:00'", []).unwrap();
        assert_eq!(first_cursor(&mac), ShareCursor::default());
        mac.execute("UPDATE mail_sync_sections SET set_aside_at = NULL", []).unwrap();
        mac.execute("INSERT INTO mail_fetch_queue (provider_message_id, account_email, queued_at) VALUES ('q1', ?1, 'x')", params![ME]).unwrap();
        assert_eq!(first_cursor(&mac), ShareCursor::default());
        assert!(super::cursor(&mac, "../../etc").is_err(), "only a computer's id");
    }

    #[test]
    fn while_the_other_computer_reads_for_this_one_the_reader_stands_by() {
        mail_sync::set_following(true);
        assert!(mail_sync::following());
        mail_sync::set_following(false);
        assert!(!mail_sync::following());
        let conn = computer();
        assert!(share_enabled(&conn), "on unless turned off");
        set_setting(&conn, MAIL_SHARE_KEY, "off").unwrap();
        assert!(!share_enabled(&conn));
        let i = info(&conn).unwrap();
        assert_eq!((i.enabled, i.connected, i.account.as_str(), i.device, i.paused), (false, false, ME, None, false));
        conn.execute("UPDATE mail_sync_state SET paused = 1 WHERE id = 1", []).unwrap();
        assert!(info(&conn).unwrap().paused, "a paused computer steps out");
    }
}

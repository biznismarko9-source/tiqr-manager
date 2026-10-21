//! 2.110.0 - settings that are the same on both computers.
//!
//! marko: "nech sa to iste aj webhooks zobrazuju na 2 zariadeniach inych
//! naraz".
//!
//! The MAIL itself never travels: each computer reads it from Gmail itself,
//! into its own mail file (mail_store.rs). What has to travel is what makes
//! the two computers read it the same way - the Gmail connection, his rules,
//! his senders, his filter - and the webhooks, so a notification set up on
//! one is there on the other.
//!
//! ## Why newest-wins, and why per group
//!
//! The cloud merge (cloud_merge.rs) only ever ADDS rows, because "both
//! machines changed the same order" has two plausible answers. Settings are
//! different: there is no sensible merge of two webhook URLs, and a rule list
//! is meant as a whole - a Keep only protects what sits below it. So each
//! group carries the time it was last changed, and the newer group replaces
//! the older one whole, deletions included: disconnecting the mailbox on one
//! computer disconnects it on the other instead of being undone by the next
//! merge.
//!
//! ## What deliberately stays per computer
//!
//! * What TIQR may create by itself (`mail_auto_*`). Two computers both on
//!   Auto would each create the same order from the same email.
//! * Pause, and everything about where reading has got to - that is the
//!   mail file, which never travels.
use crate::commands::mail::{MAIL_ACCOUNT_EMAIL_KEY, MAIL_GRANTED_SCOPE_KEY, MAIL_REFRESH_TOKEN_KEY};
use crate::commands::sheets_sync::{delete_setting, get_setting, set_setting};
use crate::error::AppResult;
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use std::sync::atomic::Ordering;

pub(crate) struct Group {
    /// For the merge log and tests.
    pub name: &'static str,
    /// When this group was last changed on the computer that owns the file.
    pub stamp_key: &'static str,
    pub keys: &'static [&'static str],
    /// Tables in the MAIN file that belong to the group, replaced whole.
    pub tables: &'static [&'static str],
}

pub(crate) const MAIL: Group = Group {
    name: "mail",
    stamp_key: "shared_mail_at",
    // 2.113.0: no credential. The Gmail refresh token and its granted scope
    // live beside each computer's database (secrets.rs) and are never part
    // of a file that travels - each computer connects the mailbox itself.
    keys: &[
        "mail_gmail_account_email",
        "mail_scope",
        "mail_custom_query",
        "mail_retention_days",
        "mail_sync_interval_seconds",
        "mail_extra_senders",
        "mail_senders_seeded",
        // 2.113.0: which computer makes records from mail (mail.rs), the own
        // mail folders, and the reader's choices (mail_view.rs).
        "mail_records_computer",
        "mail_folders",
        "mail_translate_to",
        "mail_remote_images",
        // 2.113.1: muted senders (mail.rs).
        "mail_muted",
        // 2.114.0: folders not read at all (mail.rs).
        "mail_skip_folders",
        // 2.115.0: whether two computers share the reading (mail_share.rs),
        // and the Mail screen's choices (mail.rs) - not the reading speed,
        // which is each computer's own.
        "mail_share",
        "mail_mark_read",
        "mail_after_move",
        "mail_list_preview",
        "mail_list_tags",
        "mail_list_compact",
        "mail_start_folder",
        "mail_shortcuts",
    ],
    tables: &["mail_rules", "mail_senders"],
};

pub(crate) const NOTIFICATIONS: Group = Group {
    name: "notifications",
    stamp_key: "shared_notifications_at",
    keys: &["notification_config"],
    tables: &[],
};

pub(crate) const GROUPS: &[&Group] = &[&MAIL, &NOTIFICATIONS];

/// What an untouched group from before 2.110.0 is stamped with: older than
/// any real change, so the first real edit on either computer wins - and two
/// computers that were both already set up keep their own until he edits.
const BEFORE_ANY_CHANGE: &str = "1970-01-01T00:00:00+00:00";

/// A change to a group on THIS computer. Settings are bookkeeping to the
/// change detector (db.rs), so without the explicit flag a webhook edited
/// here would wait for some unrelated edit before it ever reached the other
/// computer.
pub(crate) fn touch(conn: &Connection, group: &Group) -> AppResult<()> {
    set_setting(conn, group.stamp_key, &Utc::now().to_rfc3339())?;
    crate::db::LOCAL_DIRTY.store(true, Ordering::Relaxed);
    Ok(())
}

/// The first start of 2.110.0 on a computer that already had a group set
/// up: it gets a stamp, so a computer that has NOTHING receives it - and the
/// stamp is `BEFORE_ANY_CHANGE`, so it can never overrule a real edit.
pub(crate) fn bootstrap(conn: &Connection) -> AppResult<()> {
    for g in GROUPS {
        if get_setting(conn, g.stamp_key)?.is_some() {
            continue;
        }
        let set_up = g.keys.iter().any(|k| matches!(get_setting(conn, k), Ok(Some(_))));
        if set_up {
            set_setting(conn, g.stamp_key, BEFORE_ANY_CHANGE)?;
            crate::db::LOCAL_DIRTY.store(true, Ordering::Relaxed);
        }
    }
    Ok(())
}

fn remote_setting(conn: &Connection, key: &str) -> AppResult<Option<String>> {
    Ok(conn
        .query_row("SELECT value FROM remote.app_settings WHERE key = ?1", params![key], |r| r.get(0))
        .optional()?)
}

fn remote_has_table(conn: &Connection, table: &str) -> AppResult<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM remote.sqlite_master WHERE type = 'table' AND name = ?1)",
        params![table],
        |r| r.get(0),
    )?)
}

fn columns(conn: &Connection, schema: &str, table: &str) -> AppResult<Vec<String>> {
    let mut stmt = conn.prepare(&format!("PRAGMA {schema}.table_info({table})"))?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(1))?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// `main.<table>` becomes exactly the other computer's - over the columns
/// both versions know, so a version apart still works.
fn replace_table(conn: &Connection, table: &str) -> AppResult<()> {
    let ours = columns(conn, "main", table)?;
    let theirs = columns(conn, "remote", table)?;
    let shared: Vec<String> = ours.into_iter().filter(|c| theirs.contains(c)).map(|c| format!("\"{c}\"")).collect();
    conn.execute(&format!("DELETE FROM main.{table}"), [])?;
    if !shared.is_empty() {
        let list = shared.join(", ");
        conn.execute(&format!("INSERT INTO main.{table} ({list}) SELECT {list} FROM remote.{table}"), [])?;
    }
    Ok(())
}

/// Called by the cloud merge, inside its transaction, with the other
/// computer's file attached as `remote`. Returns the groups taken from it.
pub(crate) fn merge_from_remote(conn: &Connection) -> AppResult<Vec<&'static str>> {
    let mut taken = Vec::new();
    let mut other_mailbox = false;
    for g in GROUPS {
        // A file from before 2.110.0 has no stamp, and says nothing.
        let Some(theirs) = remote_setting(conn, g.stamp_key)? else {
            continue;
        };
        let ours = get_setting(conn, g.stamp_key)?;
        if ours.as_deref().is_some_and(|o| o >= theirs.as_str()) {
            continue;
        }
        for key in g.keys {
            let theirs = remote_setting(conn, key)?;
            // 2.114.3: the mailbox address the way live sync takes it
            // (live_sync::settings_apply) - a computer with none never
            // clears it here, and only a DIFFERENT address is another
            // mailbox, whose sign-in this computer's is not.
            if *key == MAIL_ACCOUNT_EMAIL_KEY {
                let Some(theirs) = theirs.filter(|v| !v.is_empty()) else { continue };
                let ours = get_setting(conn, key)?;
                if ours.as_deref().is_some_and(|o| !o.is_empty() && !o.eq_ignore_ascii_case(&theirs)) {
                    crate::secrets::delete(conn, MAIL_REFRESH_TOKEN_KEY)?;
                    crate::secrets::delete(conn, MAIL_GRANTED_SCOPE_KEY)?;
                    other_mailbox = true;
                }
                set_setting(conn, key, &theirs)?;
                continue;
            }
            match theirs {
                Some(v) => set_setting(conn, key, &v)?,
                None => delete_setting(conn, key)?,
            }
        }
        for table in g.tables {
            if remote_has_table(conn, table)? {
                replace_table(conn, table)?;
            }
        }
        set_setting(conn, g.stamp_key, &theirs)?;
        taken.push(g.name);
    }
    // Another mailbox: where this computer's folder walks had got to no
    // longer means anything. 2.114.3: ONLY then, the same as live sync - a
    // newer mute, folder, rule or reader choice from the other computer used
    // to send every walk here back to the top and empty the queue too.
    if other_mailbox && taken.contains(&MAIL.name) && crate::mail_store::is_attached(conn) {
        crate::commands::mail_sync::reset_engine(conn)?;
    }
    Ok(taken)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_conn;

    /// The other computer's file, on disk, attached as `remote` the way the
    /// merge attaches it.
    fn with_remote(local: &Connection, setup: impl FnOnce(&Connection)) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "tiqr-shared-{}-{}.sqlite3",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        {
            let remote = Connection::open(&path).unwrap();
            crate::db::run_migrations(&remote).unwrap();
            setup(&remote);
        }
        local
            .execute("ATTACH DATABASE ?1 AS remote", [path.to_string_lossy().to_string()])
            .unwrap();
        path
    }

    fn done(local: &Connection, path: std::path::PathBuf) {
        local.execute("DETACH DATABASE remote", []).unwrap();
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn a_computer_with_nothing_set_up_receives_the_rules_and_webhooks_but_never_a_credential() {
        let local = test_conn();
        let path = with_remote(&local, |r| {
            // A file from before 2.113.0 still holds the token in app_settings.
            set_setting(r, "mail_gmail_refresh_token", "1//token").unwrap();
            set_setting(r, "mail_gmail_account_email", "biznismarko9@gmail.com").unwrap();
            set_setting(r, "notification_config", "{\"discord\":{\"enabled\":true}}").unwrap();
            r.execute(
                "INSERT INTO mail_rules (id, position, enabled, field, match_kind, needle, action, created_at)
                 VALUES ('r1', 1, 1, 'subject', 'contains', 'payout', 'review', 'x')",
                [],
            )
            .unwrap();
            bootstrap(r).unwrap();
        });
        let taken = merge_from_remote(&local).unwrap();
        assert_eq!(taken, vec!["mail", "notifications"]);
        // 2.113.0: the mailbox address travels, the credential does not.
        assert_eq!(get_setting(&local, "mail_gmail_account_email").unwrap().as_deref(), Some("biznismarko9@gmail.com"));
        assert_eq!(get_setting(&local, "mail_gmail_refresh_token").unwrap(), None);
        assert_eq!(crate::secrets::get(&local, crate::secrets::GMAIL_REFRESH_TOKEN).unwrap(), None);
        assert!(get_setting(&local, "notification_config").unwrap().unwrap().contains("discord"));
        let rules: i64 = local.query_row("SELECT COUNT(*) FROM main.mail_rules", [], |r| r.get(0)).unwrap();
        assert_eq!(rules, 1);
        done(&local, path);
    }

    #[test]
    fn the_newer_change_wins_and_an_older_one_is_left_alone() {
        let local = test_conn();
        set_setting(&local, "notification_config", "{\"mine\":1}").unwrap();
        set_setting(&local, NOTIFICATIONS.stamp_key, "2026-10-18T12:00:00+00:00").unwrap();
        let path = with_remote(&local, |r| {
            set_setting(r, "notification_config", "{\"theirs\":1}").unwrap();
            set_setting(r, NOTIFICATIONS.stamp_key, "2026-10-17T12:00:00+00:00").unwrap();
        });
        assert!(merge_from_remote(&local).unwrap().is_empty());
        assert_eq!(get_setting(&local, "notification_config").unwrap().as_deref(), Some("{\"mine\":1}"));
        done(&local, path);

        let path = with_remote(&local, |r| {
            set_setting(r, "notification_config", "{\"theirs\":2}").unwrap();
            set_setting(r, NOTIFICATIONS.stamp_key, "2026-10-19T08:00:00+00:00").unwrap();
        });
        assert_eq!(merge_from_remote(&local).unwrap(), vec!["notifications"]);
        assert_eq!(get_setting(&local, "notification_config").unwrap().as_deref(), Some("{\"theirs\":2}"));
        done(&local, path);
    }

    /// 2.113.0 reverses 2.110.0 here on purpose: the credential is each
    /// computer's own (secrets.rs), so a newer mail group from the other
    /// computer - even one disconnected there - leaves this sign-in alone.
    #[test]
    fn a_newer_mail_group_from_the_other_computer_never_takes_this_computers_sign_in() {
        let local = test_conn();
        crate::secrets::set(&local, crate::secrets::GMAIL_REFRESH_TOKEN, "1//mine").unwrap();
        set_setting(&local, MAIL.stamp_key, "2026-10-01T00:00:00+00:00").unwrap();
        let path = with_remote(&local, |r| {
            // Disconnected there, later: no token, a newer stamp.
            set_setting(r, MAIL.stamp_key, "2026-10-18T00:00:00+00:00").unwrap();
        });
        assert_eq!(merge_from_remote(&local).unwrap(), vec!["mail"]);
        assert_eq!(crate::secrets::get(&local, crate::secrets::GMAIL_REFRESH_TOKEN).unwrap().as_deref(), Some("1//mine"));
        done(&local, path);
    }

    /// 2.114.3: a newer mute, folder or rule from the other computer is the
    /// same mailbox, so where reading had got to here stays; only another
    /// mailbox starts reading again - and drops the sign-in of the old one.
    #[test]
    fn only_another_mailbox_sends_reading_back_to_the_start() {
        let local = test_conn();
        set_setting(&local, MAIL_ACCOUNT_EMAIL_KEY, "biznismarko9@gmail.com").unwrap();
        crate::secrets::set(&local, crate::secrets::GMAIL_REFRESH_TOKEN, "1//mine").unwrap();
        set_setting(&local, MAIL.stamp_key, "2026-10-01T00:00:00+00:00").unwrap();
        local
            .execute(
                "INSERT INTO mail_sync_sections (key, title, tier, spec, page_token) VALUES ('inbox', 'Inbox', 50, 'q', 'page-7')",
                [],
            )
            .unwrap();
        local
            .execute(
                "INSERT INTO mail_fetch_queue (provider_message_id, account_email, queued_at) VALUES ('m1', 'biznismarko9@gmail.com', 'x')",
                [],
            )
            .unwrap();
        let reading = |c: &Connection| -> (Option<String>, i64) {
            (
                c.query_row("SELECT page_token FROM mail_sync_sections WHERE key = 'inbox'", [], |r| r.get(0)).unwrap(),
                c.query_row("SELECT COUNT(*) FROM mail_fetch_queue", [], |r| r.get(0)).unwrap(),
            )
        };
        let token = |c: &Connection| crate::secrets::get(c, crate::secrets::GMAIL_REFRESH_TOKEN).unwrap();

        let path = with_remote(&local, |r| {
            set_setting(r, MAIL_ACCOUNT_EMAIL_KEY, "BiznisMarko9@gmail.com").unwrap();
            set_setting(r, "mail_muted", "[{\"pattern\":\"shop.example\"}]").unwrap();
            set_setting(r, MAIL.stamp_key, "2026-10-18T00:00:00+00:00").unwrap();
        });
        assert_eq!(merge_from_remote(&local).unwrap(), vec!["mail"]);
        assert!(get_setting(&local, "mail_muted").unwrap().unwrap().contains("shop.example"), "the newer setting is taken");
        assert_eq!(reading(&local), (Some("page-7".to_string()), 1), "same mailbox: the walk and the queue stay");
        assert_eq!(token(&local).as_deref(), Some("1//mine"));
        done(&local, path);

        // A computer with no address at all never clears this one's.
        let path = with_remote(&local, |r| set_setting(r, MAIL.stamp_key, "2026-10-18T06:00:00+00:00").unwrap());
        assert_eq!(merge_from_remote(&local).unwrap(), vec!["mail"]);
        assert_eq!(get_setting(&local, MAIL_ACCOUNT_EMAIL_KEY).unwrap().as_deref(), Some("BiznisMarko9@gmail.com"));
        assert_eq!(reading(&local), (Some("page-7".to_string()), 1));
        done(&local, path);

        let path = with_remote(&local, |r| {
            set_setting(r, MAIL_ACCOUNT_EMAIL_KEY, "someone.else@gmail.com").unwrap();
            set_setting(r, MAIL.stamp_key, "2026-10-19T00:00:00+00:00").unwrap();
        });
        assert_eq!(merge_from_remote(&local).unwrap(), vec!["mail"]);
        assert_eq!(reading(&local), (None, 0), "another mailbox: reading starts again");
        assert_eq!(token(&local), None, "this sign-in belongs to the old mailbox");
        done(&local, path);
    }

    #[test]
    fn two_computers_both_set_up_before_this_release_keep_their_own() {
        let local = test_conn();
        set_setting(&local, "notification_config", "{\"mine\":1}").unwrap();
        bootstrap(&local).unwrap();
        let path = with_remote(&local, |r| {
            set_setting(r, "notification_config", "{\"theirs\":1}").unwrap();
            bootstrap(r).unwrap();
        });
        assert!(merge_from_remote(&local).unwrap().is_empty());
        assert_eq!(get_setting(&local, "notification_config").unwrap().as_deref(), Some("{\"mine\":1}"));
        done(&local, path);
    }

    #[test]
    fn a_file_from_an_older_version_changes_nothing() {
        let local = test_conn();
        set_setting(&local, "notification_config", "{\"mine\":1}").unwrap();
        let path = with_remote(&local, |r| {
            set_setting(r, "notification_config", "{\"theirs\":1}").unwrap();
        });
        assert!(merge_from_remote(&local).unwrap().is_empty());
        done(&local, path);
    }

    #[test]
    fn what_tiqr_may_create_by_itself_never_travels() {
        for g in GROUPS {
            assert!(!g.keys.iter().any(|k| k.starts_with("mail_auto_")), "{}", g.name);
        }
    }
}

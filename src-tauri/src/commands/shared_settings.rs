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
    keys: &[
        "mail_gmail_refresh_token",
        "mail_gmail_account_email",
        "mail_gmail_granted_scope",
        "mail_scope",
        "mail_custom_query",
        "mail_retention_days",
        "mail_sync_interval_seconds",
        "mail_extra_senders",
        "mail_senders_seeded",
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
            match remote_setting(conn, key)? {
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
    if taken.contains(&MAIL.name) {
        // Another connection, or the same one again: where this computer's
        // folder walks had got to no longer means anything.
        if crate::mail_store::is_attached(conn) {
            crate::commands::mail_sync::reset_engine(conn)?;
        }
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
    fn a_computer_with_nothing_set_up_receives_the_mailbox_rules_and_webhooks() {
        let local = test_conn();
        let path = with_remote(&local, |r| {
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
        assert_eq!(get_setting(&local, "mail_gmail_refresh_token").unwrap().as_deref(), Some("1//token"));
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

    #[test]
    fn disconnecting_on_one_computer_disconnects_the_other_instead_of_coming_back() {
        let local = test_conn();
        set_setting(&local, "mail_gmail_refresh_token", "1//old").unwrap();
        set_setting(&local, MAIL.stamp_key, "2026-10-01T00:00:00+00:00").unwrap();
        let path = with_remote(&local, |r| {
            // Disconnected there, later: no token, a newer stamp.
            set_setting(r, MAIL.stamp_key, "2026-10-18T00:00:00+00:00").unwrap();
        });
        merge_from_remote(&local).unwrap();
        assert!(get_setting(&local, "mail_gmail_refresh_token").unwrap().is_none());
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

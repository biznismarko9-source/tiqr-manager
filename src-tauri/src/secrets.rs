//! 2.113.0 - credentials live next to the database, never in it.
//!
//! marko's spec: "OAuth tokeny a citlivé údaje nesmú skončiť v logoch ani v
//! nechcenom cloud syncu". Until now the Gmail and Google refresh tokens sat
//! in `app_settings` and the Anthropic key in `app_secrets` - tables of the
//! main database FILE. That file is what the Google Drive backup uploads,
//! what a restore point copies and what a Sync down hands to the other
//! computer, and `app_settings` is also what the generic `get_app_setting`
//! command returns to the screen verbatim.
//!
//! Now they are in `<db stem>.secrets.json` beside the database - the folder
//! the `.device` id (live_sync.rs) and the `.mail.sqlite3` file already live
//! in - readable only by this user on Unix. Nothing uploads, copies or
//! syncs that file, so each computer keeps its own sign-ins: a Sync down or a
//! restored backup brings records, never another computer's credentials, and
//! never takes this computer's away.
//!
//! An in-memory database (the tests) has no folder; its secrets live in a
//! TEMP table of that connection instead - gone with it, never on disk.
use crate::error::{AppError, AppResult};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// The Gmail connection (commands/mail.rs).
pub(crate) const GMAIL_REFRESH_TOKEN: &str = "mail_gmail_refresh_token";
pub(crate) const GMAIL_GRANTED_SCOPE: &str = "mail_gmail_granted_scope";
/// The Google sign-in Sheets and the Drive backup use (commands/google_auth.rs).
pub(crate) const GOOGLE_REFRESH_TOKEN: &str = "google_oauth_refresh_token";
/// commands/settings.rs.
pub(crate) const ANTHROPIC_API_KEY: &str = "anthropic_api_key";

/// What used to be in `app_settings`, and is moved out by `adopt_from_database`.
const FROM_APP_SETTINGS: &[&str] = &[GMAIL_REFRESH_TOKEN, GMAIL_GRANTED_SCOPE, GOOGLE_REFRESH_TOKEN];
/// What used to be in `app_secrets` (migration 017).
const FROM_APP_SECRETS: &[&str] = &[ANTHROPIC_API_KEY];

/// `<stem>.secrets.json` beside the main database file, or `None` for an
/// in-memory database.
fn file_for(conn: &Connection) -> AppResult<Option<PathBuf>> {
    let main: Option<String> = conn
        .query_row("SELECT file FROM pragma_database_list WHERE name = 'main'", [], |r| r.get(0))
        .optional()?;
    let Some(main) = main.filter(|f| !f.is_empty()) else { return Ok(None) };
    let path = PathBuf::from(main);
    let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "tiqr-manager".into());
    Ok(Some(path.with_file_name(format!("{stem}.secrets.json"))))
}

fn read_file(path: &PathBuf) -> AppResult<BTreeMap<String, String>> {
    match std::fs::read_to_string(path) {
        Ok(text) if text.trim().is_empty() => Ok(BTreeMap::new()),
        Ok(text) => serde_json::from_str(&text).map_err(|e| {
            AppError::Io(format!(
                "The file with this computer's sign-ins ({}) is damaged: {e}. Sign in again after removing it.",
                path.display()
            ))
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(BTreeMap::new()),
        Err(e) => Err(AppError::Io(format!("Couldn't read this computer's sign-ins: {e}"))),
    }
}

/// Written whole to a temporary file and renamed over the old one, so a
/// crash half way leaves the previous version, never half of one.
fn write_file(path: &PathBuf, values: &BTreeMap<String, String>) -> AppResult<()> {
    let tmp = path.with_extension("json.tmp");
    let text = serde_json::to_string_pretty(values).map_err(|e| AppError::Other(e.to_string()))?;
    std::fs::write(&tmp, text).map_err(|e| AppError::Io(format!("Couldn't save this computer's sign-ins: {e}")))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
    }
    std::fs::rename(&tmp, path).map_err(|e| AppError::Io(format!("Couldn't save this computer's sign-ins: {e}")))?;
    Ok(())
}

fn ensure_temp_table(conn: &Connection) -> AppResult<()> {
    conn.execute_batch("CREATE TEMP TABLE IF NOT EXISTS local_secrets (key TEXT PRIMARY KEY, value TEXT NOT NULL);")?;
    Ok(())
}

pub(crate) fn get(conn: &Connection, key: &str) -> AppResult<Option<String>> {
    match file_for(conn)? {
        Some(path) => Ok(read_file(&path)?.remove(key)),
        None => {
            ensure_temp_table(conn)?;
            Ok(conn
                .query_row("SELECT value FROM temp.local_secrets WHERE key = ?1", params![key], |r| r.get(0))
                .optional()?)
        }
    }
}

pub(crate) fn set(conn: &Connection, key: &str, value: &str) -> AppResult<()> {
    match file_for(conn)? {
        Some(path) => {
            let mut values = read_file(&path)?;
            values.insert(key.to_string(), value.to_string());
            write_file(&path, &values)
        }
        None => {
            ensure_temp_table(conn)?;
            conn.execute(
                "INSERT INTO temp.local_secrets (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value],
            )?;
            Ok(())
        }
    }
}

pub(crate) fn delete(conn: &Connection, key: &str) -> AppResult<()> {
    match file_for(conn)? {
        Some(path) => {
            let mut values = read_file(&path)?;
            if values.remove(key).is_some() {
                write_file(&path, &values)?;
            }
            Ok(())
        }
        None => {
            ensure_temp_table(conn)?;
            conn.execute("DELETE FROM temp.local_secrets WHERE key = ?1", params![key])?;
            Ok(())
        }
    }
}

/// Every credential still in the database file moves out of it - on every
/// start of the live connection, and after a restore (a backup from before
/// 2.113.0 still holds them). What this computer already has wins: a
/// restored copy never replaces this computer's own sign-in. The rows are
/// deleted with `secure_delete`, so the bytes are overwritten, not merely
/// unlinked - the next Drive backup must not carry them in a free page.
pub(crate) fn adopt_from_database(conn: &Connection) -> AppResult<usize> {
    let mut moved = 0;
    let mut found: Vec<(&str, &str, String)> = Vec::new();
    for key in FROM_APP_SETTINGS {
        let v: Option<String> = conn
            .query_row("SELECT value FROM main.app_settings WHERE key = ?1", params![key], |r| r.get(0))
            .optional()?;
        if let Some(v) = v {
            found.push(("app_settings", key, v));
        }
    }
    let has_secrets_table: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM main.sqlite_master WHERE type = 'table' AND name = 'app_secrets')",
        [],
        |r| r.get(0),
    )?;
    if has_secrets_table {
        for key in FROM_APP_SECRETS {
            let v: Option<String> = conn
                .query_row("SELECT value FROM main.app_secrets WHERE key = ?1", params![key], |r| r.get(0))
                .optional()?;
            if let Some(v) = v {
                found.push(("app_secrets", key, v));
            }
        }
    }
    if found.is_empty() {
        return Ok(0);
    }
    for (_, key, value) in &found {
        if !value.trim().is_empty() && get(conn, key)?.is_none() {
            set(conn, key, value)?;
            moved += 1;
        }
    }
    conn.execute_batch("PRAGMA secure_delete = ON;")?;
    let result = (|| -> AppResult<()> {
        for (table, key, _) in &found {
            conn.execute(&format!("DELETE FROM main.{table} WHERE key = ?1"), params![key])?;
        }
        Ok(())
    })();
    conn.execute_batch("PRAGMA secure_delete = OFF;")?;
    result?;
    Ok(moved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::sheets_sync::{get_setting, set_setting};

    fn file_db(name: &str) -> (Connection, PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "tiqr-secrets-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("tiqr-manager.sqlite3");
        let conn = Connection::open(&db).unwrap();
        crate::db::run_migrations(&conn).unwrap();
        (conn, dir)
    }

    #[test]
    fn credentials_live_beside_the_database_and_never_in_it() {
        let (conn, dir) = file_db("beside");
        set(&conn, GMAIL_REFRESH_TOKEN, "1//gmail").unwrap();
        set(&conn, GOOGLE_REFRESH_TOKEN, "1//google").unwrap();
        assert_eq!(get(&conn, GMAIL_REFRESH_TOKEN).unwrap().as_deref(), Some("1//gmail"));
        let file = dir.join("tiqr-manager.secrets.json");
        assert!(file.exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(&file).unwrap().permissions().mode() & 0o777, 0o600);
        }
        let db_bytes = std::fs::read(dir.join("tiqr-manager.sqlite3")).unwrap();
        let needle: &[u8] = b"1//gmail";
        assert!(!db_bytes.windows(needle.len()).any(|w| w == needle), "the token is not in the database file");
        delete(&conn, GMAIL_REFRESH_TOKEN).unwrap();
        assert_eq!(get(&conn, GMAIL_REFRESH_TOKEN).unwrap(), None);
        assert_eq!(get(&conn, GOOGLE_REFRESH_TOKEN).unwrap().as_deref(), Some("1//google"));
        drop(conn);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_older_database_gives_its_credentials_up_and_keeps_no_trace_of_them() {
        let (conn, dir) = file_db("adopt");
        set_setting(&conn, GMAIL_REFRESH_TOKEN, "1//old-gmail-token").unwrap();
        set_setting(&conn, GMAIL_GRANTED_SCOPE, "gmail.readonly").unwrap();
        set_setting(&conn, "mail_gmail_account_email", "me@gmail.com").unwrap();
        conn.execute("INSERT INTO app_secrets (key, value) VALUES (?1, 'sk-ant-old')", params![ANTHROPIC_API_KEY]).unwrap();
        assert_eq!(adopt_from_database(&conn).unwrap(), 3);
        assert_eq!(get(&conn, GMAIL_REFRESH_TOKEN).unwrap().as_deref(), Some("1//old-gmail-token"));
        assert_eq!(get(&conn, ANTHROPIC_API_KEY).unwrap().as_deref(), Some("sk-ant-old"));
        assert_eq!(get_setting(&conn, GMAIL_REFRESH_TOKEN).unwrap(), None);
        assert_eq!(get_setting(&conn, "mail_gmail_account_email").unwrap().as_deref(), Some("me@gmail.com"), "only credentials move");
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);").unwrap();
        let db_bytes = std::fs::read(dir.join("tiqr-manager.sqlite3")).unwrap();
        let needle: &[u8] = b"1//old-gmail-token";
        assert!(!db_bytes.windows(needle.len()).any(|w| w == needle), "overwritten, not just unlinked");
        // Again: nothing left to move.
        assert_eq!(adopt_from_database(&conn).unwrap(), 0);
        drop(conn);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_restored_copy_never_replaces_this_computers_own_sign_in() {
        let (conn, dir) = file_db("restore");
        set(&conn, GMAIL_REFRESH_TOKEN, "1//current").unwrap();
        set_setting(&conn, GMAIL_REFRESH_TOKEN, "1//from-the-backup").unwrap();
        assert_eq!(adopt_from_database(&conn).unwrap(), 0);
        assert_eq!(get(&conn, GMAIL_REFRESH_TOKEN).unwrap().as_deref(), Some("1//current"));
        assert_eq!(get_setting(&conn, GMAIL_REFRESH_TOKEN).unwrap(), None, "and the copy's is gone from the file");
        drop(conn);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_in_memory_database_keeps_them_in_the_connection_only() {
        let conn = crate::db::test_conn();
        set(&conn, GMAIL_REFRESH_TOKEN, "1//mem").unwrap();
        assert_eq!(get(&conn, GMAIL_REFRESH_TOKEN).unwrap().as_deref(), Some("1//mem"));
        assert_eq!(get_setting(&conn, GMAIL_REFRESH_TOKEN).unwrap(), None);
        let other = crate::db::test_conn();
        assert_eq!(get(&other, GMAIL_REFRESH_TOKEN).unwrap(), None);
    }

    #[test]
    fn a_damaged_file_is_said_out_loud_not_taken_for_empty() {
        let (conn, dir) = file_db("damaged");
        std::fs::write(dir.join("tiqr-manager.secrets.json"), "{not json").unwrap();
        let err = get(&conn, GMAIL_REFRESH_TOKEN).unwrap_err().to_string();
        assert!(err.contains("damaged"), "{err}");
        drop(conn);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

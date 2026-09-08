//! TIQR Cloud Sync (2.12.0) - one database, two machines.
//!
//! marko works on a Windows PC and a Mac and wants what he writes on one to
//! show up on the other. This module is the smallest thing that honestly
//! delivers that.
//!
//! ## What this is, and what it deliberately is not
//!
//! It syncs the WHOLE DATABASE AS ONE FILE, in one direction at a time:
//! push (upload this machine's database) or pull (download and restore the
//! other machine's). It does **not** merge row-level changes.
//!
//! That limit is the whole design, not a shortcut. Real row-level merging
//! would need globally-unique ids on every table (every primary key in this
//! app is a per-machine `INTEGER AUTOINCREMENT`, so two machines both mint
//! id 5), plus a conflict policy for invariants that genuinely have no
//! automatic answer - `insert_order_with_tickets` splits an order's cost
//! across its tickets to the exact cent, and `refund_sale_impl` is a
//! one-way atomic transition. Merging two machines' versions of those
//! without a human deciding would silently produce impossible states. See
//! `PROTECTED_AREAS.md`'s 2.12.0 entry.
//!
//! ## Why it cannot lose data silently
//!
//! Every upload records the Drive file's `version` at the moment it was
//! written. Before the next upload, that version is checked again: if the
//! remote changed in the meantime (the other machine pushed), the upload is
//! REFUSED and the caller is told. Overwriting anyway is possible, but only
//! as an explicit, separate decision (`force`).
//!
//! Every download goes through `backup::restore_database_impl`, which
//! validates the candidate file, takes a safety backup of the current
//! database first, and rolls back automatically if anything fails. Nothing
//! here bypasses that.
//!
//! ## Storage
//!
//! One file in the person's OWN Google Drive, created by this app under the
//! `drive.file` scope - the narrowest scope that works, granting access only
//! to files this app itself created and never to the rest of their Drive.
//! No new service, no server of ours, no hosting bill. It reuses the Google
//! sign-in and refresh-token plumbing `commands::google_auth` already has in
//! production for Sheets sync.
//!
//! ## Never automatic without consent
//!
//! Nothing in this module runs on a timer or on startup by itself. The
//! frontend decides when to call, and only ever after marko has switched
//! sync on. This app is still local-first: with sync off, or offline, every
//! command here simply reports that and the app works exactly as before.

use crate::commands::backup::{restore_database_impl, snapshot_db_to};
use crate::commands::google_auth::active_oauth_access_token;
use crate::commands::sheets_sync::{get_setting, set_setting};
use crate::db::AppState;
use crate::error::{AppError, AppResult};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::State;

pub(crate) const ENABLED_KEY: &str = "cloud_sync_enabled";
pub(crate) const FILE_ID_KEY: &str = "cloud_sync_file_id";
/// The Drive `version` this machine last read or wrote. The whole
/// lost-update guard hangs off this one value.
pub(crate) const REMOTE_VERSION_KEY: &str = "cloud_sync_remote_version";
pub(crate) const LAST_SYNC_KEY: &str = "cloud_sync_last_sync_at";

/// Name of the file in the person's Drive. Visible to them on purpose (an
/// `appDataFolder` file would be invisible and impossible to back up or
/// delete by hand), and distinctive enough not to be mistaken for anything
/// else they own.
const REMOTE_FILE_NAME: &str = "tiqr-manager-sync.sqlite3";

const DRIVE_FILES: &str = "https://www.googleapis.com/drive/v3/files";
const DRIVE_UPLOAD: &str = "https://www.googleapis.com/upload/drive/v3/files";
/// Everything this module ever needs from Drive about a file.
const FILE_FIELDS: &str = "id,name,version,modifiedTime,size";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DriveFile {
    id: String,
    /// Drive returns int64s as strings.
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    modified_time: Option<String>,
    #[serde(default)]
    size: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DriveFileList {
    #[serde(default)]
    files: Vec<DriveFile>,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CloudSyncStatus {
    pub enabled: bool,
    /// False when this build has no Google OAuth client, or nobody is signed
    /// in - the UI uses this to say "sign in with Google first" rather than
    /// showing a broken sync panel.
    pub signed_in: bool,
    /// None until the first push has created the remote file.
    pub file_id: Option<String>,
    pub last_sync_at: Option<String>,
    /// Drive's own last-modified timestamp for the remote file.
    pub remote_modified_at: Option<String>,
    pub remote_size_bytes: Option<i64>,
    /// True when the remote file has changed since this machine last pushed
    /// or pulled - i.e. the other machine has newer data. This is the single
    /// signal the UI needs to say "pull first".
    pub remote_newer: bool,
}

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(name)
}

fn http() -> AppResult<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        // Generous: a database upload over a slow connection is normal, and
        // a hung request must still eventually give up rather than wedge the
        // command thread forever.
        .timeout(std::time::Duration::from_secs(180))
        .build()
        .map_err(|e| AppError::External(format!("Couldn't start an HTTPS client: {e}")))
}

/// Turns any non-2xx Drive response into a message worth showing a person.
fn drive_error(context: &str, status: reqwest::StatusCode, body: &str) -> AppError {
    let hint = match status.as_u16() {
        // Two very different causes share this status. The Google Drive API
        // not being enabled on the OAuth client's Cloud project is a
        // one-time, project-wide setting and by far the more common of the
        // two on a fresh setup - Google's own message (passed through below)
        // carries the exact console URL, which the UI turns into a button.
        401 | 403 => " - either the Google Drive API is not switched on for this app's Google Cloud project (a one-time step, see the link below), or your sign-in needs renewing: Settings -> Integrations -> sign in with Google again and allow Drive access.",
        404 => " - the sync file no longer exists in your Drive. Turn sync off and on again to start a new one.",
        507 => " - your Google Drive is full.",
        _ => "",
    };
    // Drive's own error bodies are long JSON. 400 characters is enough to
    // keep Google's message AND the console URL it embeds when an API is not
    // enabled - the UI looks for that URL and offers it as a button, so
    // truncating it away would remove the one-click fix.
    let snippet: String = body.chars().take(400).collect();
    AppError::External(format!("{context} failed ({status}){hint} {snippet}"))
}

fn token(conn: &Connection) -> AppResult<String> {
    active_oauth_access_token(conn)?.ok_or_else(|| {
        AppError::Validation(
            "Cloud sync needs you signed in with Google - open Settings and sign in first.".to_string(),
        )
    })
}

/// Looks for an existing sync file this app created. Used only when the
/// stored file id is missing (first run on a second machine), so the second
/// machine adopts the first machine's file instead of creating a rival one.
fn find_remote_file(client: &reqwest::blocking::Client, access_token: &str) -> AppResult<Option<DriveFile>> {
    let query = format!("name = '{REMOTE_FILE_NAME}' and trashed = false");
    let q = utf8_percent_encode(&query, NON_ALPHANUMERIC);
    let fields = utf8_percent_encode("files(id,name,version,modifiedTime,size)", NON_ALPHANUMERIC);
    let url = format!("{DRIVE_FILES}?q={q}&spaces=drive&fields={fields}&pageSize=10");
    let resp = client
        .get(&url)
        .bearer_auth(access_token)
        .send()
        .map_err(|e| AppError::External(format!("Couldn't reach Google Drive: {e}")))?;
    let status = resp.status();
    let body = resp.text().unwrap_or_default();
    if !status.is_success() {
        return Err(drive_error("Looking for your sync file", status, &body));
    }
    let list: DriveFileList = serde_json::from_str(&body)
        .map_err(|e| AppError::External(format!("Google Drive returned something unexpected: {e}")))?;
    // Newest first, so a duplicate left behind by an old experiment can
    // never win over the file actually in use.
    let mut files = list.files;
    files.sort_by(|a, b| b.modified_time.cmp(&a.modified_time));
    Ok(files.into_iter().next())
}

fn get_remote_meta(
    client: &reqwest::blocking::Client,
    access_token: &str,
    file_id: &str,
) -> AppResult<DriveFile> {
    let id = utf8_percent_encode(file_id, NON_ALPHANUMERIC);
    let fields = utf8_percent_encode(FILE_FIELDS, NON_ALPHANUMERIC);
    let url = format!("{DRIVE_FILES}/{id}?fields={fields}");
    let resp = client
        .get(&url)
        .bearer_auth(access_token)
        .send()
        .map_err(|e| AppError::External(format!("Couldn't reach Google Drive: {e}")))?;
    let status = resp.status();
    let body = resp.text().unwrap_or_default();
    if !status.is_success() {
        return Err(drive_error("Checking your sync file", status, &body));
    }
    serde_json::from_str(&body)
        .map_err(|e| AppError::External(format!("Google Drive returned something unexpected: {e}")))
}

/// Creates the (empty) remote file. Content is written by `upload_content`
/// straight after - two simple requests instead of one hand-rolled
/// multipart/related body, which is far easier to get subtly wrong.
fn create_remote_file(client: &reqwest::blocking::Client, access_token: &str) -> AppResult<String> {
    let url = format!("{DRIVE_FILES}?fields=id");
    let resp = client
        .post(&url)
        .bearer_auth(access_token)
        .json(&serde_json::json!({
            "name": REMOTE_FILE_NAME,
            "description": "TIQR Manager cloud sync - one database snapshot, written by the app.",
            "mimeType": "application/x-sqlite3",
        }))
        .send()
        .map_err(|e| AppError::External(format!("Couldn't reach Google Drive: {e}")))?;
    let status = resp.status();
    let body = resp.text().unwrap_or_default();
    if !status.is_success() {
        return Err(drive_error("Creating your sync file", status, &body));
    }
    let created: DriveFile = serde_json::from_str(&body)
        .map_err(|e| AppError::External(format!("Google Drive returned something unexpected: {e}")))?;
    Ok(created.id)
}

fn upload_content(
    client: &reqwest::blocking::Client,
    access_token: &str,
    file_id: &str,
    bytes: Vec<u8>,
) -> AppResult<DriveFile> {
    let id = utf8_percent_encode(file_id, NON_ALPHANUMERIC);
    let fields = utf8_percent_encode(FILE_FIELDS, NON_ALPHANUMERIC);
    let url = format!("{DRIVE_UPLOAD}/{id}?uploadType=media&fields={fields}");
    let resp = client
        .patch(&url)
        .bearer_auth(access_token)
        .header(reqwest::header::CONTENT_TYPE, "application/x-sqlite3")
        .body(bytes)
        .send()
        .map_err(|e| AppError::External(format!("Couldn't upload to Google Drive: {e}")))?;
    let status = resp.status();
    let body = resp.text().unwrap_or_default();
    if !status.is_success() {
        return Err(drive_error("Uploading your data", status, &body));
    }
    serde_json::from_str(&body)
        .map_err(|e| AppError::External(format!("Google Drive returned something unexpected: {e}")))
}

/// Streams the remote file straight to `dest` rather than buffering the whole
/// database in memory first - `reqwest::blocking::Response` implements
/// `std::io::Read`, so this is a plain `io::copy`.
fn download_to_file(
    client: &reqwest::blocking::Client,
    access_token: &str,
    file_id: &str,
    dest: &std::path::Path,
) -> AppResult<()> {
    let id = utf8_percent_encode(file_id, NON_ALPHANUMERIC);
    let url = format!("{DRIVE_FILES}/{id}?alt=media");
    let mut resp = client
        .get(&url)
        .bearer_auth(access_token)
        .send()
        .map_err(|e| AppError::External(format!("Couldn't download from Google Drive: {e}")))?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().unwrap_or_default();
        return Err(drive_error("Downloading your data", status, &body));
    }
    let mut file = std::fs::File::create(dest)
        .map_err(|e| AppError::Other(format!("Couldn't create the download file: {e}")))?;
    std::io::copy(&mut resp, &mut file)
        .map_err(|e| AppError::External(format!("Couldn't save the downloaded file: {e}")))?;
    Ok(())
}

/// The lost-update guard, in one place.
///
/// `stored` is the Drive version this machine last read or wrote; `remote`
/// is what Drive reports now. They differing means the OTHER machine has
/// written since - so this machine's upload would destroy work it has never
/// seen.
pub(crate) fn remote_has_moved(stored: Option<&str>, remote: Option<&str>) -> bool {
    match (stored, remote) {
        // Never synced from here: anything already up there is unseen.
        (None, Some(_)) => true,
        (Some(s), Some(r)) => s != r,
        // Drive did not report a version. Refusing on the strength of a
        // missing field would block sync entirely, and this guard exists to
        // prevent silent loss, not to be a lock - so treat it as unchanged
        // and let the explicit push proceed.
        (_, None) => false,
    }
}

fn is_enabled(conn: &Connection) -> AppResult<bool> {
    Ok(get_setting(conn, ENABLED_KEY)?.as_deref() == Some("true"))
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Read-only. Never uploads, never downloads content - one small metadata
/// request, and not even that when sync is off or nobody is signed in.
#[tauri::command]
pub fn cloud_sync_status(state: State<AppState>) -> AppResult<CloudSyncStatus> {
    let conn = state.db.lock().unwrap();
    let enabled = is_enabled(&conn)?;
    let file_id = get_setting(&conn, FILE_ID_KEY)?;
    let stored_version = get_setting(&conn, REMOTE_VERSION_KEY)?;
    let last_sync_at = get_setting(&conn, LAST_SYNC_KEY)?;

    let signed_in = active_oauth_access_token(&conn).map(|t| t.is_some()).unwrap_or(false);
    let mut status = CloudSyncStatus {
        enabled,
        signed_in,
        file_id: file_id.clone(),
        last_sync_at,
        remote_modified_at: None,
        remote_size_bytes: None,
        remote_newer: false,
    };
    if !enabled || !signed_in {
        return Ok(status);
    }
    let Some(file_id) = file_id else {
        return Ok(status);
    };

    // Being offline is a normal state for this app, not an error worth
    // failing a status call over - the panel just shows what it knows
    // locally.
    let client = http()?;
    let access_token = token(&conn)?;
    if let Ok(meta) = get_remote_meta(&client, &access_token, &file_id) {
        status.remote_newer = remote_has_moved(stored_version.as_deref(), meta.version.as_deref());
        status.remote_modified_at = meta.modified_time;
        status.remote_size_bytes = meta.size.and_then(|s| s.parse::<i64>().ok());
    }
    Ok(status)
}

#[tauri::command]
pub fn set_cloud_sync_enabled(state: State<AppState>, enabled: bool) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    set_setting(&conn, ENABLED_KEY, if enabled { "true" } else { "false" })?;
    Ok(())
}

/// Uploads this machine's database.
///
/// Refuses when the remote has moved since this machine last synced, unless
/// `force` - see `remote_has_moved`. That refusal is the only thing standing
/// between "I forgot to sync" and losing the other machine's day of work.
#[tauri::command]
pub fn cloud_sync_push(state: State<AppState>, force: bool) -> AppResult<CloudSyncStatus> {
    let conn = state.db.lock().unwrap();
    if !is_enabled(&conn)? {
        return Err(AppError::Validation("Cloud sync is turned off.".to_string()));
    }
    let client = http()?;
    let access_token = token(&conn)?;

    // Adopt an existing file before creating one, so a second machine joins
    // the same sync instead of starting a rival copy.
    let file_id = match get_setting(&conn, FILE_ID_KEY)? {
        Some(id) => id,
        None => match find_remote_file(&client, &access_token)? {
            Some(found) => {
                set_setting(&conn, FILE_ID_KEY, &found.id)?;
                found.id
            }
            None => {
                let id = create_remote_file(&client, &access_token)?;
                set_setting(&conn, FILE_ID_KEY, &id)?;
                id
            }
        },
    };

    if !force {
        let stored = get_setting(&conn, REMOTE_VERSION_KEY)?;
        // A file this machine just created has no content yet; only guard
        // once there is something up there worth protecting.
        if let Ok(meta) = get_remote_meta(&client, &access_token, &file_id) {
            let has_content = meta.size.as_deref().and_then(|s| s.parse::<i64>().ok()).unwrap_or(0) > 0;
            if has_content && remote_has_moved(stored.as_deref(), meta.version.as_deref()) {
                return Err(AppError::Validation(
                    "The other machine has synced newer data since this one last did. Sync down first, or choose to overwrite."
                        .to_string(),
                ));
            }
        }
    }

    // Online Backup API, not a raw file copy - consistent even though the
    // live connection is in WAL mode and open right now.
    let snapshot = temp_path("tiqr-cloud-sync-upload.sqlite3");
    let _ = std::fs::remove_file(&snapshot);
    snapshot_db_to(&conn, &snapshot)?;
    let bytes = std::fs::read(&snapshot)
        .map_err(|e| AppError::Other(format!("Couldn't read the snapshot to upload: {e}")))?;
    let uploaded = upload_content(&client, &access_token, &file_id, bytes)?;
    let _ = std::fs::remove_file(&snapshot);

    if let Some(v) = uploaded.version.as_deref() {
        set_setting(&conn, REMOTE_VERSION_KEY, v)?;
    }
    set_setting(&conn, LAST_SYNC_KEY, &now_iso())?;

    Ok(CloudSyncStatus {
        enabled: true,
        signed_in: true,
        file_id: Some(file_id),
        last_sync_at: get_setting(&conn, LAST_SYNC_KEY)?,
        remote_modified_at: uploaded.modified_time,
        remote_size_bytes: uploaded.size.and_then(|s| s.parse::<i64>().ok()),
        remote_newer: false,
    })
}

/// Downloads the other machine's database and restores it over this one.
///
/// Destructive by nature, and routed entirely through
/// `backup::restore_database_impl` so it inherits that path's validation,
/// automatic safety backup and automatic rollback. The safety backup's
/// location is returned so the UI can tell marko exactly where his previous
/// data went.
#[tauri::command]
pub fn cloud_sync_pull(state: State<AppState>) -> AppResult<String> {
    let mut conn = state.db.lock().unwrap();
    if !is_enabled(&conn)? {
        return Err(AppError::Validation("Cloud sync is turned off.".to_string()));
    }
    let client = http()?;
    let access_token = token(&conn)?;

    let file_id = match get_setting(&conn, FILE_ID_KEY)? {
        Some(id) => id,
        None => find_remote_file(&client, &access_token)?
            .map(|f| f.id)
            .ok_or_else(|| {
                AppError::Validation(
                    "There is nothing in your Drive to sync down yet - sync up from your other machine first."
                        .to_string(),
                )
            })?,
    };
    set_setting(&conn, FILE_ID_KEY, &file_id)?;

    let meta = get_remote_meta(&client, &access_token, &file_id)?;
    let has_content = meta.size.as_deref().and_then(|s| s.parse::<i64>().ok()).unwrap_or(0) > 0;
    if !has_content {
        return Err(AppError::Validation(
            "The sync file in your Drive is empty - sync up from your other machine first.".to_string(),
        ));
    }

    let downloaded = temp_path("tiqr-cloud-sync-download.sqlite3");
    let _ = std::fs::remove_file(&downloaded);
    download_to_file(&client, &access_token, &file_id, &downloaded)?;

    // Same rule restore_database itself follows (2.0.72): the safety backup
    // goes next to the CURRENTLY ACTIVE per-account database, so it belongs
    // to the account that is signed in right now rather than to a shared
    // root.
    let db_path = state.db_path.lock().unwrap().clone();
    let safety_dir = db_path
        .parent()
        .ok_or_else(|| AppError::Other("Could not resolve app data directory".into()))?
        .to_path_buf();
    let outcome = restore_database_impl(&mut conn, &downloaded, &safety_dir)?;
    let _ = std::fs::remove_file(&downloaded);

    // Only recorded after a restore that actually succeeded - a failed pull
    // must not make this machine believe it is up to date.
    if let Some(v) = meta.version.as_deref() {
        set_setting(&conn, REMOTE_VERSION_KEY, v)?;
    }
    set_setting(&conn, LAST_SYNC_KEY, &now_iso())?;
    Ok(outcome.safety_backup_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_conn;

    #[test]
    fn a_machine_that_has_never_synced_treats_any_remote_file_as_newer() {
        assert!(remote_has_moved(None, Some("7")));
    }

    #[test]
    fn an_unchanged_remote_version_is_not_a_conflict() {
        assert!(!remote_has_moved(Some("7"), Some("7")));
    }

    #[test]
    fn a_changed_remote_version_is_a_conflict() {
        assert!(remote_has_moved(Some("7"), Some("8")));
        // Drive versions only ever increase, but the guard is deliberately
        // "different", not "greater" - an unexpected value still means this
        // machine is not looking at what it last wrote.
        assert!(remote_has_moved(Some("8"), Some("7")));
    }

    #[test]
    fn a_missing_remote_version_never_blocks_an_explicit_push() {
        // This guard exists to prevent silent loss, not to be a lock. If
        // Drive does not report a version, refusing would make sync
        // impossible rather than safe.
        assert!(!remote_has_moved(Some("7"), None));
        assert!(!remote_has_moved(None, None));
    }

    #[test]
    fn sync_is_off_until_it_is_explicitly_turned_on() {
        let conn = test_conn();
        assert!(!is_enabled(&conn).unwrap());
        set_setting(&conn, ENABLED_KEY, "true").unwrap();
        assert!(is_enabled(&conn).unwrap());
        set_setting(&conn, ENABLED_KEY, "false").unwrap();
        assert!(!is_enabled(&conn).unwrap());
    }

    #[test]
    fn a_fresh_database_reports_nothing_synced_rather_than_guessing() {
        let conn = test_conn();
        assert!(get_setting(&conn, FILE_ID_KEY).unwrap().is_none());
        assert!(get_setting(&conn, REMOTE_VERSION_KEY).unwrap().is_none());
        assert!(get_setting(&conn, LAST_SYNC_KEY).unwrap().is_none());
    }
}

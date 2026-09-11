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
//! ## Automatic, but never a guess (2.14.0)
//!
//! Until 2.14.0 nothing here ran on a timer or on startup by itself. marko
//! asked for the hand-off between his two machines to stop needing a click,
//! so the frontend now calls `cloud_sync_auto` when the app opens and on a
//! quiet timer while he works.
//!
//! That command is READ-ONLY: it decides and reports. The frontend then
//! calls the same `cloud_sync_push` / `cloud_sync_pull` the buttons have
//! always called, so nothing destructive got a second code path, and every
//! download still goes through `restore_database_impl` with its validation,
//! safety backup and rollback.
//!
//! The policy itself is `decide_auto` - one pure function, the whole table
//! on one screen. Until 2.16.0 two cases stopped and asked (both sides
//! changed; a machine that had never synced finding data in Drive), because
//! whole-file sync had to pick a winner. Both now answer `Merge` and hand
//! off to `commands::cloud_merge`, which adds the two sides together
//! instead. Every case is answerable without a human, and is answered.
//!
//! With sync off, or offline, every command here still simply reports that
//! and the app works exactly as before. That part has not changed.

use crate::commands::backup::{restore_database_impl, snapshot_db_to};
use crate::commands::google_auth::active_oauth_access_token;
use crate::commands::sheets_sync::{get_setting, set_setting};
use crate::db::AppState;
use crate::error::{AppError, AppResult};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use tauri::State;

pub(crate) const ENABLED_KEY: &str = "cloud_sync_enabled";
pub(crate) const FILE_ID_KEY: &str = "cloud_sync_file_id";
/// The Drive `version` this machine last read or wrote. The whole
/// lost-update guard hangs off this one value.
pub(crate) const REMOTE_VERSION_KEY: &str = "cloud_sync_remote_version";
pub(crate) const LAST_SYNC_KEY: &str = "cloud_sync_last_sync_at";
/// Persisted copy of `db::LOCAL_DIRTY`. Survives a restart; the atomic does
/// not. See `local_dirty`.
pub(crate) const DIRTY_KEY: &str = "cloud_sync_local_dirty";

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
pub(crate) struct DriveFile {
    pub(crate) id: String,
    /// Drive returns int64s as strings.
    #[serde(default)]
    pub(crate) version: Option<String>,
    #[serde(default)]
    pub(crate) modified_time: Option<String>,
    #[serde(default)]
    pub(crate) size: Option<String>,
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

pub(crate) fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(name)
}

pub(crate) fn http() -> AppResult<reqwest::blocking::Client> {
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

pub(crate) fn token(conn: &Connection) -> AppResult<String> {
    active_oauth_access_token(conn)?.ok_or_else(|| {
        AppError::Validation(
            "Cloud sync needs you signed in with Google - open Settings and sign in first.".to_string(),
        )
    })
}

/// Looks for an existing sync file this app created. Used only when the
/// stored file id is missing (first run on a second machine), so the second
/// machine adopts the first machine's file instead of creating a rival one.
pub(crate) fn find_remote_file(client: &reqwest::blocking::Client, access_token: &str) -> AppResult<Option<DriveFile>> {
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

pub(crate) fn get_remote_meta(
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
pub(crate) fn download_to_file(
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

pub(crate) fn is_enabled(conn: &Connection) -> AppResult<bool> {
    Ok(get_setting(conn, ENABLED_KEY)?.as_deref() == Some("true"))
}

pub(crate) fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Read-only. Never uploads, never downloads content - one small metadata
/// request, and not even that when sync is off or nobody is signed in.
#[tauri::command(async)]
pub fn cloud_sync_status(state: State<'_, AppState>) -> AppResult<CloudSyncStatus> {
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
#[tauri::command(async)]
pub fn cloud_sync_push(state: State<'_, AppState>, force: bool) -> AppResult<CloudSyncStatus> {
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
    // Only now, with the bytes accepted by Drive. Nothing marko wrote can be
    // lost in the gap: every write goes through this same lock, which this
    // command has held since before the snapshot was taken.
    mark_local_clean(&conn)?;

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
#[tauri::command(async)]
pub fn cloud_sync_pull(state: State<'_, AppState>) -> AppResult<String> {
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
    // The restore itself is a write, and this machine's contents are now
    // exactly what is in Drive - so the flag is cleared here rather than
    // left set by the very operation that made the two sides agree.
    mark_local_clean(&conn)?;
    Ok(outcome.safety_backup_path)
}

// ---------------------------------------------------------------------------
// Automatic sync (2.14.0)
// ---------------------------------------------------------------------------

/// What the frontend should do next. Serialized camelCase, so TypeScript sees
/// `"off" | "offline" | "idle" | "push" | "pull" | "merge"`.
#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CloudSyncAutoAction {
    /// Sync is switched off, or nobody is signed in. Do nothing, say nothing.
    Off,
    /// Drive could not be reached. Normal for a local-first app - do nothing
    /// and try again on the next tick. Never treated as "the remote is
    /// unchanged", because a push on that assumption is exactly how the
    /// other machine's day disappears.
    Offline,
    /// Both sides already agree.
    Idle,
    /// This machine holds work the other one has not seen: call
    /// `cloud_sync_push` (never with `force`).
    Push,
    /// The other machine holds work this one has not seen, and this one has
    /// nothing unsent: call `cloud_sync_pull`, then relaunch.
    Pull,
    /// Both sides hold something the other has not seen. Call
    /// `cloud_merge_pull`, which adds them together instead of picking a
    /// winner - see `commands::cloud_merge`. Until 2.16.0 this was `Ask`,
    /// because with whole-file sync there was genuinely nothing to do but
    /// make marko choose which machine's day to keep.
    Merge,
}

/// The result of one automatic check. Carries the two facts the decision was
/// made from, so the UI can explain itself without asking again.
#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CloudSyncAutoPlan {
    pub action: CloudSyncAutoAction,
    /// One plain sentence, safe to show as-is.
    pub reason: String,
    pub local_dirty: bool,
    pub remote_newer: bool,
}

/// The whole automatic-sync policy, as one pure function.
///
/// Deliberately free of Drive, the database and Tauri: this is the part that
/// decides whether a machine's data gets replaced, so it has to fit on one
/// screen and be testable without a network.
///
/// Two situations used to stop and ask, because whole-file sync had to pick a
/// winner and a winner picked by a timer is a coin toss with marko's work:
/// both sides changed, and a machine that has never synced finding data
/// already in Drive. **Since 2.16.0 neither asks.** Both now answer `Merge`,
/// and `commands::cloud_merge` adds the two sides together - a record only
/// one machine has is copied, a record both have is left alone. There is
/// nothing to choose between when nothing is thrown away.
pub(crate) fn decide_auto(
    enabled: bool,
    signed_in: bool,
    remote_reachable: bool,
    remote_has_content: bool,
    ever_synced: bool,
    local_dirty: bool,
    remote_newer: bool,
) -> (CloudSyncAutoAction, &'static str) {
    use CloudSyncAutoAction::*;
    if !enabled || !signed_in {
        return (Off, "Cloud sync is off.");
    }
    if !remote_reachable {
        return (Offline, "Google Drive couldn't be reached - nothing was changed.");
    }
    if !remote_has_content {
        // Nothing up there yet, so an upload cannot overwrite anything.
        return if local_dirty || !ever_synced {
            (Push, "Drive is still empty - this machine's data goes up first.")
        } else {
            (Idle, "Everything is already in sync.")
        };
    }
    if !ever_synced {
        return (
            Merge,
            "First sync on this machine - your data and what's in Drive get added together.",
        );
    }
    match (local_dirty, remote_newer) {
        (true, true) => (
            Merge,
            "Both machines have changes - they get added together, nothing is dropped.",
        ),
        (true, false) => (Push, "This machine has changes the other one hasn't seen."),
        (false, true) => (
            Pull,
            "The other machine has newer data and this one has nothing unsent.",
        ),
        (false, false) => (Idle, "Everything is already in sync."),
    }
}

/// The in-memory write flag and its persisted copy, combined.
///
/// The atomic (`db::LOCAL_DIRTY`) dies with the process. The persisted copy
/// is what lets a machine that was closed before it could push still know,
/// next launch, that it is holding unsent work - which is precisely the case
/// where an automatic pull would otherwise overwrite it.
fn combine_dirty(atomic: bool, persisted: Option<&str>) -> bool {
    atomic || persisted == Some("true")
}

/// True when this machine has writes the remote copy has not seen.
///
/// Writing the flag through on the way past is the point: the atomic is
/// volatile, and the window between "marko edited something" and "the app
/// was closed" is exactly where the persisted copy earns its keep.
pub(crate) fn local_dirty(conn: &Connection) -> AppResult<bool> {
    let atomic = crate::db::LOCAL_DIRTY.load(Ordering::Relaxed);
    if atomic {
        set_setting(conn, DIRTY_KEY, "true")?;
    }
    let persisted = get_setting(conn, DIRTY_KEY)?;
    Ok(combine_dirty(atomic, persisted.as_deref()))
}

/// Called only after an upload or a restore has actually succeeded - never
/// before, never on the strength of having tried.
pub(crate) fn mark_local_clean(conn: &Connection) -> AppResult<()> {
    set_setting(conn, DIRTY_KEY, "false")?;
    crate::db::LOCAL_DIRTY.store(false, Ordering::Relaxed);
    Ok(())
}

/// Best-effort flush on the way out of the process (see `lib.rs`'s
/// `ExitRequested` block). Closes the last gap in the persisted flag: an
/// edit made between two automatic checks, followed immediately by a close.
/// Failure is ignored on purpose - nothing may delay an ordinary exit.
pub fn flush_local_dirty(conn: &Connection) {
    if crate::db::LOCAL_DIRTY.load(Ordering::Relaxed) {
        let _ = set_setting(conn, DIRTY_KEY, "true");
    }
}

/// Decides what should happen, and does none of it.
///
/// READ-ONLY except for the dirty flag: one Drive metadata request, no
/// upload, no download, no restore. The frontend acts on the answer by
/// calling the same `cloud_sync_push` / `cloud_sync_pull` the buttons have
/// always called, so automatic sync introduced no second destructive path -
/// and it cannot take those locks itself anyway, since both of them lock
/// `state.db` and this command is already holding it.
#[tauri::command(async)]
pub fn cloud_sync_auto(state: State<'_, AppState>) -> AppResult<CloudSyncAutoPlan> {
    let conn = state.db.lock().unwrap();

    let enabled = is_enabled(&conn)?;
    // ONE token call, where `cloud_sync_status` makes two. That is a
    // deliberate difference: status runs when marko opens a panel, this runs
    // on a timer for as long as the app is open, and
    // `active_oauth_access_token` refreshes over the network whenever the
    // cached token has expired.
    let token_result = active_oauth_access_token(&conn);
    let signed_in = match &token_result {
        Ok(t) => t.is_some(),
        // A refresh that FAILED is a network problem, not a signed-out
        // account. Calling it `Off` would tell marko sync is switched off
        // when it is switched on and simply out of reach.
        Err(_) => true,
    };
    let plan = |action: CloudSyncAutoAction, reason: &str, dirty: bool, newer: bool| CloudSyncAutoPlan {
        action,
        reason: reason.to_string(),
        local_dirty: dirty,
        remote_newer: newer,
    };
    if !enabled || !signed_in {
        // Deliberately before the dirty flag is even read: with sync off,
        // this command writes nothing at all.
        let (action, reason) = decide_auto(enabled, signed_in, false, false, false, false, false);
        return Ok(plan(action, reason, false, false));
    }

    let dirty = local_dirty(&conn)?;
    let ever_synced = get_setting(&conn, LAST_SYNC_KEY)?.is_some();
    let stored_version = get_setting(&conn, REMOTE_VERSION_KEY)?;

    let client = http()?;
    // `Ok(None)` cannot reach here - it is `signed_in == false`, returned
    // above. So the only way this fails is a refresh that could not complete,
    // which is exactly the offline case.
    let Ok(Some(access_token)) = token_result else {
        let (action, reason) = decide_auto(true, true, false, false, ever_synced, dirty, false);
        return Ok(plan(action, reason, dirty, false));
    };

    // No stored file id means this machine has never synced. Looking the
    // file up (rather than assuming there is none) is what lets a second
    // machine notice the first one's data instead of quietly pushing over
    // it. Nothing is stored here - `cloud_sync_push`/`_pull` adopt the file
    // themselves when marko actually chooses a direction.
    let found = match get_setting(&conn, FILE_ID_KEY)? {
        Some(id) => get_remote_meta(&client, &access_token, &id).map(Some),
        None => find_remote_file(&client, &access_token),
    };
    let (remote_reachable, meta) = match found {
        Ok(m) => (true, m),
        Err(_) => (false, None),
    };
    let remote_has_content = meta
        .as_ref()
        .map(|m| m.size.as_deref().and_then(|s| s.parse::<i64>().ok()).unwrap_or(0) > 0)
        .unwrap_or(false);
    let remote_newer = meta
        .as_ref()
        .map(|m| remote_has_moved(stored_version.as_deref(), m.version.as_deref()))
        .unwrap_or(false);

    let (action, reason) = decide_auto(
        true,
        true,
        remote_reachable,
        remote_has_content,
        ever_synced,
        dirty,
        remote_newer,
    );
    Ok(plan(action, reason, dirty, remote_newer))
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

    // --- the automatic policy (2.14.0) ---------------------------------
    //
    // Argument order throughout: enabled, signed_in, remote_reachable,
    // remote_has_content, ever_synced, local_dirty, remote_newer.

    fn act(
        enabled: bool,
        signed_in: bool,
        reachable: bool,
        has_content: bool,
        ever: bool,
        dirty: bool,
        newer: bool,
    ) -> CloudSyncAutoAction {
        decide_auto(enabled, signed_in, reachable, has_content, ever, dirty, newer).0
    }

    #[test]
    fn nothing_happens_automatically_while_sync_is_off_or_signed_out() {
        // Even with every other condition screaming "push".
        assert_eq!(act(false, true, true, true, true, true, false), CloudSyncAutoAction::Off);
        assert_eq!(act(true, false, true, true, true, true, false), CloudSyncAutoAction::Off);
    }

    #[test]
    fn an_unreachable_drive_is_never_mistaken_for_an_unchanged_one() {
        // The dangerous bug this rules out: treating "no answer from Drive"
        // as "the remote hasn't moved" and pushing over the other machine.
        assert_eq!(act(true, true, false, true, true, true, false), CloudSyncAutoAction::Offline);
        assert_eq!(act(true, true, false, true, true, false, true), CloudSyncAutoAction::Offline);
    }

    #[test]
    fn an_empty_drive_takes_this_machines_data_without_asking() {
        // An upload into an empty file cannot overwrite anything, so there
        // is nothing to ask about - including on a machine that has never
        // synced, which is how the very first sync gets started.
        assert_eq!(act(true, true, true, false, false, false, false), CloudSyncAutoAction::Push);
        assert_eq!(act(true, true, true, false, true, true, false), CloudSyncAutoAction::Push);
    }

    #[test]
    fn an_empty_drive_with_nothing_new_stays_quiet() {
        // Already pushed once, nothing written since: no re-upload on every
        // tick just because the remote happens to read as empty.
        assert_eq!(act(true, true, true, false, true, false, false), CloudSyncAutoAction::Idle);
    }

    #[test]
    fn a_machine_that_has_never_synced_merges_rather_than_picking_a_side() {
        // 2.16.0. Nothing on this side can tell whether that file is this
        // machine's own data or the other one's - and now it does not have
        // to, because adding them together loses neither.
        assert_eq!(act(true, true, true, true, false, false, true), CloudSyncAutoAction::Merge);
        assert_eq!(act(true, true, true, true, false, true, true), CloudSyncAutoAction::Merge);
    }

    #[test]
    fn one_side_changing_is_answerable_without_a_human() {
        assert_eq!(act(true, true, true, true, true, true, false), CloudSyncAutoAction::Push);
        assert_eq!(act(true, true, true, true, true, false, true), CloudSyncAutoAction::Pull);
    }

    #[test]
    fn both_sides_changing_merges_and_never_picks() {
        // This was the whole complaint: "ked ma jedna strana nieco ine a
        // druha a das sync tak sa to zachova len z jednej strany". A winner
        // picked by a timer was a coin toss with marko's work; now there is
        // no winner to pick.
        assert_eq!(act(true, true, true, true, true, true, true), CloudSyncAutoAction::Merge);
    }

    #[test]
    fn two_machines_that_agree_do_nothing_at_all() {
        assert_eq!(act(true, true, true, true, true, false, false), CloudSyncAutoAction::Idle);
    }

    #[test]
    fn every_automatic_outcome_explains_itself() {
        // The reason is shown to marko as-is, so no branch may return a
        // placeholder.
        for plan in [
            decide_auto(false, true, true, true, true, true, false),
            decide_auto(true, true, false, true, true, true, false),
            decide_auto(true, true, true, false, false, false, false),
            decide_auto(true, true, true, false, true, false, false),
            decide_auto(true, true, true, true, false, false, true),
            decide_auto(true, true, true, true, true, true, false),
            decide_auto(true, true, true, true, true, false, true),
            decide_auto(true, true, true, true, true, true, true),
        ] {
            assert!(plan.1.len() > 10, "empty reason for {:?}", plan.0);
        }
    }

    #[test]
    fn unsent_work_survives_the_app_being_closed() {
        // The whole point of persisting the flag: the atomic is gone after a
        // restart, and a machine that forgets it has unsent work is a
        // machine that will let the next startup pull over it.
        assert!(combine_dirty(false, Some("true")));
        assert!(combine_dirty(true, None));
        assert!(combine_dirty(true, Some("false")));
    }

    #[test]
    fn a_machine_that_has_written_nothing_is_not_dirty() {
        assert!(!combine_dirty(false, None));
        assert!(!combine_dirty(false, Some("false")));
    }

    #[test]
    fn a_persisted_dirty_flag_is_believed_on_the_next_launch() {
        // Reads the true direction only - `db::LOCAL_DIRTY` is one static
        // shared by every test in this binary, so asserting it is false
        // would be asserting something about the other tests.
        let conn = test_conn();
        set_setting(&conn, DIRTY_KEY, "true").unwrap();
        assert!(local_dirty(&conn).unwrap());
    }

    #[test]
    fn a_successful_sync_clears_the_persisted_flag() {
        let conn = test_conn();
        set_setting(&conn, DIRTY_KEY, "true").unwrap();
        mark_local_clean(&conn).unwrap();
        assert_eq!(get_setting(&conn, DIRTY_KEY).unwrap().as_deref(), Some("false"));
    }
}

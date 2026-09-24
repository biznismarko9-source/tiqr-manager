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

use crate::commands::backup::{restore_database_impl, snapshot_db_to, validate_tiqr_backup};
use crate::commands::google_auth::active_oauth_access_token;
use crate::commands::sheets_sync::{get_setting, set_setting};
use crate::db::AppState;
use crate::error::{AppError, AppResult};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
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
    /// 2.18.0. One of: `off`, `syncing`, `conflict`, `offline`, `failed`,
    /// `localChanges`, `cloudChanges`, `synced`. Decided by `summarize_state`
    /// so the panel never re-derives it from the booleans and drifts.
    pub state: String,
    /// This machine holds writes Drive has not seen.
    pub local_changes: bool,
    /// How the last sync failed, in one sentence. `None` when the last one
    /// worked - a stale error left on screen is worse than none.
    pub last_error: Option<String>,
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
/// Which of the three very different 403s Drive can answer with this is.
///
/// 2.49.1: they used to share one sentence that led with "the Drive API is not
/// switched on", which sent marko to the Google Cloud Console for a problem
/// that was not there. His body said `insufficientPermissions` / "Request had
/// insufficient authentication scopes" - a permission that was never granted,
/// fixed only by signing in again and ticking it.
fn forbidden_hint(body: &str) -> &'static str {
    let b = body.to_ascii_lowercase();
    if b.contains("insufficientpermissions")
        || b.contains("insufficient authentication scopes")
        || b.contains("insufficient permission")
    {
        return " - your Google sign-in does not include permission for Drive, so this can never succeed as it stands. Settings -> Integrations -> sign in with Google again, and on Google's screen leave EVERY permission ticked (each line has its own tick box).";
    }
    if b.contains("accessnotconfigured") || b.contains("service_disabled") || b.contains("has not been used in project") {
        return " - the Google Drive API is not switched on for this app's Google Cloud project. That is a one-time step; the link below opens the exact page.";
    }
    " - Google refused the request. If you have just changed your Google account or its permissions, sign in again from Settings -> Integrations."
}

/// Turns any non-2xx Drive response into a message worth showing a person.
fn drive_error(context: &str, status: reqwest::StatusCode, body: &str) -> AppError {
    let hint = match status.as_u16() {
        401 => " - your sign-in has expired: Settings -> Integrations -> sign in with Google again.",
        403 => forbidden_hint(body),
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
// Cloud versions (2.20.0)
// ---------------------------------------------------------------------------
//
// marko: the restore points are all on this machine, and the one copy that
// isn't - the file in Drive - gets overwritten by every sync. So a bad day
// that got pushed had nowhere to be undone from.
//
// It turns out there was already a history there. Google Drive keeps
// REVISIONS of a file, and every upload this app has ever made created one.
// So this is point-in-time recovery from off the machine with no new storage,
// no new service and not one byte of new infrastructure - just an endpoint
// that was never called.
//
// Verified against Google's own reference for `revisions.list` rather than
// assumed: `https://www.googleapis.com/auth/drive.file` IS among the accepted
// scopes, which is the narrow scope this app already holds. No new consent
// screen, and it works retroactively on revisions already up there.
//
// Deliberately NOT setting `keepForever` on uploads: Drive prunes revision
// history on its own schedule, and pinning every sync would multiply marko's
// Drive usage by the number of syncs. So this shows what Drive has kept, and
// says as much.

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DriveRevision {
    id: String,
    #[serde(default)]
    modified_time: Option<String>,
    #[serde(default)]
    size: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DriveRevisionList {
    #[serde(default)]
    revisions: Vec<DriveRevision>,
}

/// One earlier version of the sync file, as Drive still holds it.
#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CloudRevision {
    pub id: String,
    pub modified_at: Option<String>,
    pub size_bytes: Option<i64>,
    /// True for the version currently live in Drive - restoring that one is
    /// an ordinary sync down, not a trip backwards.
    pub is_current: bool,
}

fn fetch_revisions(
    client: &reqwest::blocking::Client,
    access_token: &str,
    file_id: &str,
) -> AppResult<Vec<DriveRevision>> {
    let id = utf8_percent_encode(file_id, NON_ALPHANUMERIC);
    let fields = utf8_percent_encode("revisions(id,modifiedTime,size)", NON_ALPHANUMERIC);
    let url = format!("{DRIVE_FILES}/{id}/revisions?fields={fields}&pageSize=100");
    let resp = client
        .get(&url)
        .bearer_auth(access_token)
        .send()
        .map_err(|e| AppError::External(format!("Couldn't reach Google Drive: {e}")))?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().unwrap_or_default();
        return Err(drive_error("Listing earlier cloud versions", status, &body));
    }
    let parsed: DriveRevisionList = resp
        .json()
        .map_err(|e| AppError::External(format!("Google Drive returned something unexpected: {e}")))?;
    Ok(parsed.revisions)
}

fn download_revision_to_file(
    client: &reqwest::blocking::Client,
    access_token: &str,
    file_id: &str,
    revision_id: &str,
    dest: &std::path::Path,
) -> AppResult<()> {
    let id = utf8_percent_encode(file_id, NON_ALPHANUMERIC);
    let rev = utf8_percent_encode(revision_id, NON_ALPHANUMERIC);
    let url = format!("{DRIVE_FILES}/{id}/revisions/{rev}?alt=media");
    let mut resp = client
        .get(&url)
        .bearer_auth(access_token)
        .send()
        .map_err(|e| AppError::External(format!("Couldn't download from Google Drive: {e}")))?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().unwrap_or_default();
        return Err(drive_error("Downloading an earlier cloud version", status, &body));
    }
    let mut file = std::fs::File::create(dest)
        .map_err(|e| AppError::Other(format!("Couldn't create the download file: {e}")))?;
    std::io::copy(&mut resp, &mut file)
        .map_err(|e| AppError::External(format!("Couldn't save the downloaded file: {e}")))?;
    Ok(())
}

/// Earlier versions of the sync file, newest first. Read-only: one metadata
/// request, no file content.
#[tauri::command(async)]
pub fn cloud_sync_revisions(state: State<'_, AppState>) -> AppResult<Vec<CloudRevision>> {
    let conn = state.db.lock().unwrap();
    if !is_enabled(&conn)? {
        return Ok(Vec::new());
    }
    let Some(file_id) = get_setting(&conn, FILE_ID_KEY)? else {
        return Ok(Vec::new());
    };
    let client = http()?;
    let access_token = token(&conn)?;
    let mut revisions = retrying(|| fetch_revisions(&client, &access_token, &file_id))?;
    // Drive returns oldest first; the last one is what is live right now.
    revisions.reverse();
    Ok(revisions
        .into_iter()
        .enumerate()
        .map(|(i, r)| CloudRevision {
            id: r.id,
            modified_at: r.modified_time,
            size_bytes: r.size.and_then(|s| s.parse::<i64>().ok()),
            is_current: i == 0,
        })
        .collect())
}

/// Replaces this machine's database with an earlier version from Drive.
///
/// Destructive, and routed through exactly the same `restore_database_impl`
/// as every other restore: validation, an automatic safety backup, automatic
/// rollback. The caller relaunches afterwards.
#[tauri::command(async)]
pub fn cloud_sync_restore_revision(state: State<'_, AppState>, revision_id: String) -> AppResult<String> {
    let Some(_guard) = SyncGuard::acquire() else { return Err(busy_error()) };
    let mut conn = state.db.lock().unwrap();
    let db_path = state.db_path.lock().unwrap().clone();
    let outcome = restore_revision_inner(&mut conn, db_path, &revision_id);
    record_outcome(&conn, &outcome);
    outcome
}

fn restore_revision_inner(
    conn: &mut Connection,
    db_path: std::path::PathBuf,
    revision_id: &str,
) -> AppResult<String> {
    if !is_enabled(conn)? {
        return Err(AppError::Validation("Cloud sync is turned off.".to_string()));
    }
    let file_id = get_setting(conn, FILE_ID_KEY)?.ok_or_else(|| {
        AppError::Validation("There is nothing in your Drive to restore from yet.".to_string())
    })?;
    let client = http()?;
    let access_token = token(conn)?;

    let downloaded = temp_path("tiqr-cloud-revision.sqlite3");
    let _ = std::fs::remove_file(&downloaded);
    retrying(|| download_revision_to_file(&client, &access_token, &file_id, revision_id, &downloaded))?;
    validate_tiqr_backup(&downloaded)?;

    let safety_dir = db_path
        .parent()
        .ok_or_else(|| AppError::Other("Could not resolve app data directory".into()))?
        .to_path_buf();
    let outcome = restore_database_impl(conn, &downloaded, &safety_dir)?;
    let _ = std::fs::remove_file(&downloaded);

    // Record the version that is CURRENTLY live in Drive, not the one just
    // restored. That is deliberate and it is what makes the restore travel:
    // this machine has seen the newer copy and is choosing not to keep it, so
    // there is no conflict to report - and the restore left the database
    // dirty, so automatic sync pushes the older data up and the other machine
    // gets the same rescue. The pushed-hash is deliberately left stale for the
    // same reason: it must not match, or the upload would be skipped.
    if let Ok(meta) = retrying(|| get_remote_meta(&client, &access_token, &file_id)) {
        if let Some(v) = meta.version.as_deref() {
            set_setting(conn, REMOTE_VERSION_KEY, v)?;
        }
    }
    set_setting(conn, LAST_SYNC_KEY, &now_iso())?;
    Ok(outcome.safety_backup_path)
}

// ---------------------------------------------------------------------------
// One sync at a time, bounded retries, and a hash that stops pointless uploads
// (2.18.0)
// ---------------------------------------------------------------------------

/// Hash of the bytes this machine last successfully uploaded. What turns "the
/// database was written to" into "the database actually differs from the copy
/// in Drive" - see `content_hash`.
pub(crate) const PUSHED_HASH_KEY: &str = "cloud_sync_pushed_hash";
/// Last failure, in one sentence, so the panel can still explain itself long
/// after the toast is gone.
pub(crate) const LAST_ERROR_KEY: &str = "cloud_sync_last_error";
/// Set when a merge could not decide something on its own. Cleared only by a
/// merge that comes back clean - not by an ordinary push, which would hide it
/// while the two machines still disagree.
pub(crate) const CONFLICT_KEY: &str = "cloud_sync_conflict";

/// True while any sync operation is running, anywhere in the app.
///
/// Before 2.18.0 the only guards were per-screen: `Layout.tsx` had a ref for
/// its timer and Settings disabled its own buttons. Neither knew about the
/// other, so the 5-minute tick could start an upload in the middle of a
/// hand-pressed Sync - and 2.17.0 made that genuinely concurrent by moving
/// these commands off the main thread. Two uploads racing decide the winner
/// by whichever HTTP request finishes last, and then store a `version` for a
/// file the other one has already replaced.
static SYNC_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

/// Held for the length of one sync operation. Releases on drop, including on
/// an early `?` return and on a panic, which is the entire reason it is a
/// guard rather than two stores.
pub(crate) struct SyncGuard;

impl SyncGuard {
    /// `None` when another sync is already running. Never blocks: a sync that
    /// waits its turn would just queue up behind the timer forever.
    pub(crate) fn acquire() -> Option<SyncGuard> {
        SYNC_IN_PROGRESS
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .ok()
            .map(|_| SyncGuard)
    }
}

impl Drop for SyncGuard {
    fn drop(&mut self) {
        SYNC_IN_PROGRESS.store(false, Ordering::SeqCst);
    }
}

pub(crate) fn sync_in_progress() -> bool {
    SYNC_IN_PROGRESS.load(Ordering::SeqCst)
}

fn busy_error() -> AppError {
    AppError::Validation("A sync is already running - wait for it to finish.".to_string())
}

/// How long to wait before each retry. Two entries = three attempts total,
/// and then it gives up. Never an endless retry: this app is local-first, so
/// "we'll try again in five minutes" is a perfectly good outcome and a loop
/// that never stops is worse than a failure that says so.
const RETRY_DELAYS_MS: &[u64] = &[1_000, 3_000];

/// Whether an error is a hiccup or an answer.
///
/// Only `External` is ever retried - that is this codebase's own class for
/// "an outside service failed" (see error.rs). A rejected token, a missing
/// file or a refused permission are answers: retrying them twice more just
/// makes the user wait longer for the same message.
fn is_worth_retrying(e: &AppError) -> bool {
    let AppError::External(msg) = e else { return false };
    let lower = msg.to_lowercase();
    !(lower.contains("(401")
        || lower.contains("(403")
        || lower.contains("(404")
        || lower.contains("invalid_grant")
        || lower.contains("sign in"))
}

/// Runs `op`, and on a transient failure runs it again after a pause.
///
/// The sleep is only safe because 2.17.0 moved these commands off the main
/// thread with `#[tauri::command(async)]`. On the main thread this would
/// freeze the window for four seconds - exactly the bug that release fixed.
pub(crate) fn retrying<T>(mut op: impl FnMut() -> AppResult<T>) -> AppResult<T> {
    let mut attempt = 0usize;
    loop {
        match op() {
            Ok(value) => return Ok(value),
            Err(e) => {
                if attempt >= RETRY_DELAYS_MS.len() || !is_worth_retrying(&e) {
                    return Err(e);
                }
                std::thread::sleep(std::time::Duration::from_millis(RETRY_DELAYS_MS[attempt]));
                attempt += 1;
            }
        }
    }
}

/// Identifies a snapshot's contents well enough to answer one question: are
/// these the same bytes this machine last uploaded?
///
/// FNV-1a with the byte length appended. Deliberately not a cryptographic
/// hash and not a new dependency - nothing here defends against a forged
/// database, it only avoids pushing several megabytes that Drive already has.
///
/// The failure modes are lopsided in the safe direction. The same data can
/// produce different bytes (SQLite is free to lay pages out differently), so
/// the common wrong answer is "changed" when it hasn't - which costs one
/// upload. The opposite, a collision saying "unchanged" when it changed, is a
/// 1-in-2^64 event that would cost one delayed hand-off, and the next edit
/// undoes it.
fn content_hash(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}-{}", bytes.len())
}

/// One sentence, safe to put on a settings panel. Long technical detail is
/// not more helpful here, it just stops the sentence being read at all.
fn short_error(e: &AppError) -> String {
    let msg = e.to_string();
    let first = msg.lines().next().unwrap_or(&msg).trim().to_string();
    if first.chars().count() > 180 {
        let cut: String = first.chars().take(177).collect();
        format!("{cut}...")
    } else {
        first
    }
}

/// Records how a sync operation ended, so the panel can say so later.
pub(crate) fn record_outcome(conn: &Connection, outcome: &AppResult<impl Sized>) {
    match outcome {
        Ok(_) => {
            let _ = set_setting(conn, LAST_ERROR_KEY, "");
        }
        Err(e) => {
            let _ = set_setting(conn, LAST_ERROR_KEY, &short_error(e));
        }
    }
}

/// The one short word the UI shows, decided in one place.
///
/// A pure function for the same reason `decide_auto` is one: this is what
/// marko reads to know whether his two machines agree, and it has to be
/// checkable without a network, a database or a running app.
pub(crate) fn summarize_state(
    enabled: bool,
    signed_in: bool,
    syncing: bool,
    conflict: bool,
    reachable: bool,
    failed: bool,
    local_changes: bool,
    remote_newer: bool,
) -> &'static str {
    if !enabled || !signed_in {
        return "off";
    }
    if syncing {
        return "syncing";
    }
    // Before offline on purpose: a recorded conflict is a fact about the data
    // itself and stays true whether or not there is a signal right now.
    if conflict {
        return "conflict";
    }
    if !reachable {
        return "offline";
    }
    if failed {
        return "failed";
    }
    if local_changes && remote_newer {
        return "conflict";
    }
    if local_changes {
        return "localChanges";
    }
    if remote_newer {
        return "cloudChanges";
    }
    "synced"
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
    let syncing = sync_in_progress();
    let conflict = get_setting(&conn, CONFLICT_KEY)?.as_deref() == Some("true");
    let last_error = get_setting(&conn, LAST_ERROR_KEY)?.filter(|e| !e.is_empty());
    // Reading the flag is a write when the in-memory one is set - see
    // `local_dirty`. Harmless here and deliberately the same call the auto
    // check makes, so the panel and the timer can never disagree about
    // whether this machine has unsent work.
    let local_changes = if enabled && signed_in { local_dirty(&conn)? } else { false };

    let mut status = CloudSyncStatus {
        enabled,
        signed_in,
        file_id: file_id.clone(),
        last_sync_at,
        remote_modified_at: None,
        remote_size_bytes: None,
        remote_newer: false,
        state: String::new(),
        local_changes,
        last_error: last_error.clone(),
    };
    let finish = |mut st: CloudSyncStatus, reachable: bool| {
        st.state = summarize_state(
            st.enabled,
            st.signed_in,
            syncing,
            conflict,
            reachable,
            st.last_error.is_some(),
            st.local_changes,
            st.remote_newer,
        )
        .to_string();
        st
    };
    if !enabled || !signed_in {
        return Ok(finish(status, true));
    }
    let Some(file_id) = file_id else {
        // Nothing uploaded yet. Not offline, just never synced - the state
        // falls out as localChanges or synced from what is known locally.
        return Ok(finish(status, true));
    };

    // Being offline is a normal state for this app, not an error worth
    // failing a status call over - the panel says "offline" and shows what it
    // knows locally. One attempt only: this runs whenever the panel opens,
    // and making that wait four seconds for a retry would be worse than the
    // honest answer.
    let client = http()?;
    let mut reachable = false;
    if let Ok(access_token) = token(&conn) {
        if let Ok(meta) = get_remote_meta(&client, &access_token, &file_id) {
            reachable = true;
            status.remote_newer = remote_has_moved(stored_version.as_deref(), meta.version.as_deref());
            status.remote_modified_at = meta.modified_time;
            status.remote_size_bytes = meta.size.and_then(|s| s.parse::<i64>().ok());
        }
    }
    Ok(finish(status, reachable))
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
    // 2.18.0: before anything else. Two uploads racing decide the winner by
    // whichever request happens to finish last, and then record a `version`
    // for a file the other one already replaced.
    let Some(_guard) = SyncGuard::acquire() else { return Err(busy_error()) };
    let conn = state.db.lock().unwrap();
    let outcome = push_inner(&conn, force);
    record_outcome(&conn, &outcome);
    outcome
}

fn push_inner(conn: &Connection, force: bool) -> AppResult<CloudSyncStatus> {
    if !is_enabled(conn)? {
        return Err(AppError::Validation("Cloud sync is turned off.".to_string()));
    }
    let client = http()?;
    let access_token = token(conn)?;

    // Adopt an existing file before creating one, so a second machine joins
    // the same sync instead of starting a rival copy.
    let file_id = match get_setting(conn, FILE_ID_KEY)? {
        Some(id) => id,
        None => match retrying(|| find_remote_file(&client, &access_token))? {
            Some(found) => {
                set_setting(conn, FILE_ID_KEY, &found.id)?;
                found.id
            }
            None => {
                let id = retrying(|| create_remote_file(&client, &access_token))?;
                set_setting(conn, FILE_ID_KEY, &id)?;
                id
            }
        },
    };

    let stored_version = get_setting(conn, REMOTE_VERSION_KEY)?;
    let meta = retrying(|| get_remote_meta(&client, &access_token, &file_id)).ok();
    let remote_moved = meta
        .as_ref()
        .map(|m| {
            let has_content = m.size.as_deref().and_then(|s| s.parse::<i64>().ok()).unwrap_or(0) > 0;
            // A file this machine just created has no content yet; only guard
            // once there is something up there worth protecting.
            has_content && remote_has_moved(stored_version.as_deref(), m.version.as_deref())
        })
        .unwrap_or(false);

    if !force && remote_moved {
        return Err(AppError::Validation(
            "The other machine has synced newer data since this one last did. Sync down first, or choose to overwrite."
                .to_string(),
        ));
    }

    // Online Backup API, not a raw file copy - consistent even though the
    // live connection is in WAL mode and open right now.
    let snapshot = temp_path("tiqr-cloud-sync-upload.sqlite3");
    let _ = std::fs::remove_file(&snapshot);
    snapshot_db_to(conn, &snapshot)?;
    let bytes = std::fs::read(&snapshot)
        .map_err(|e| AppError::Other(format!("Couldn't read the snapshot to upload: {e}")))?;
    let _ = std::fs::remove_file(&snapshot);

    // 2.18.0: the database being WRITTEN to is not the same thing as the
    // database DIFFERING from the copy in Drive. Running the app's own
    // migrations on launch marks it dirty, so before this every app update
    // pushed several megabytes that Drive already had, byte for byte. Taking
    // the snapshot is the cheap half of a push; the upload is the slow half,
    // and this is what skips it.
    let hash = content_hash(&bytes);
    let already_there = !remote_moved
        && get_setting(conn, PUSHED_HASH_KEY)?.as_deref() == Some(hash.as_str())
        && meta.is_some();
    if already_there {
        set_setting(conn, LAST_SYNC_KEY, &now_iso())?;
        mark_local_clean(conn)?;
        let m = meta.expect("checked by already_there");
        return Ok(CloudSyncStatus {
            enabled: true,
            signed_in: true,
            file_id: Some(file_id),
            last_sync_at: get_setting(conn, LAST_SYNC_KEY)?,
            remote_modified_at: m.modified_time,
            remote_size_bytes: m.size.and_then(|s| s.parse::<i64>().ok()),
            remote_newer: false,
            state: "synced".to_string(),
            local_changes: false,
            last_error: None,
        });
    }

    let uploaded = retrying(|| upload_content(&client, &access_token, &file_id, bytes.clone()))?;

    if let Some(v) = uploaded.version.as_deref() {
        set_setting(conn, REMOTE_VERSION_KEY, v)?;
    }
    set_setting(conn, PUSHED_HASH_KEY, &hash)?;
    set_setting(conn, LAST_SYNC_KEY, &now_iso())?;
    // Only now, with the bytes accepted by Drive. Nothing marko wrote can be
    // lost in the gap: every write goes through this same lock, which this
    // command has held since before the snapshot was taken.
    mark_local_clean(conn)?;

    Ok(CloudSyncStatus {
        enabled: true,
        signed_in: true,
        file_id: Some(file_id),
        last_sync_at: get_setting(conn, LAST_SYNC_KEY)?,
        remote_modified_at: uploaded.modified_time,
        remote_size_bytes: uploaded.size.and_then(|s| s.parse::<i64>().ok()),
        remote_newer: false,
        state: "synced".to_string(),
        local_changes: false,
        last_error: None,
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
    let Some(_guard) = SyncGuard::acquire() else { return Err(busy_error()) };
    let mut conn = state.db.lock().unwrap();
    let db_path = state.db_path.lock().unwrap().clone();
    let outcome = pull_inner(&mut conn, db_path);
    record_outcome(&conn, &outcome);
    outcome
}

/// The destructive half, kept separate only so the command above can record
/// how it ended. Every safety property lives here and is unchanged: validation,
/// the automatic safety backup, and the rollback all come from
/// `restore_database_impl`.
fn pull_inner(conn: &mut Connection, db_path: std::path::PathBuf) -> AppResult<String> {
    if !is_enabled(conn)? {
        return Err(AppError::Validation("Cloud sync is turned off.".to_string()));
    }
    let client = http()?;
    let access_token = token(conn)?;

    let file_id = match get_setting(conn, FILE_ID_KEY)? {
        Some(id) => id,
        None => retrying(|| find_remote_file(&client, &access_token))?
            .map(|f| f.id)
            .ok_or_else(|| {
                AppError::Validation(
                    "There is nothing in your Drive to sync down yet - sync up from your other machine first."
                        .to_string(),
                )
            })?,
    };
    set_setting(conn, FILE_ID_KEY, &file_id)?;

    let meta = retrying(|| get_remote_meta(&client, &access_token, &file_id))?;
    let has_content = meta.size.as_deref().and_then(|s| s.parse::<i64>().ok()).unwrap_or(0) > 0;
    if !has_content {
        return Err(AppError::Validation(
            "The sync file in your Drive is empty - sync up from your other machine first.".to_string(),
        ));
    }

    let downloaded = temp_path("tiqr-cloud-sync-download.sqlite3");
    let _ = std::fs::remove_file(&downloaded);
    retrying(|| download_to_file(&client, &access_token, &file_id, &downloaded))?;

    // Same rule restore_database itself follows (2.0.72): the safety backup
    // goes next to the CURRENTLY ACTIVE per-account database, so it belongs
    // to the account that is signed in right now rather than to a shared
    // root.
    let safety_dir = db_path
        .parent()
        .ok_or_else(|| AppError::Other("Could not resolve app data directory".into()))?
        .to_path_buf();
    let outcome = restore_database_impl(conn, &downloaded, &safety_dir)?;
    let _ = std::fs::remove_file(&downloaded);

    // Only recorded after a restore that actually succeeded - a failed pull
    // must not make this machine believe it is up to date.
    if let Some(v) = meta.version.as_deref() {
        set_setting(conn, REMOTE_VERSION_KEY, v)?;
    }
    set_setting(conn, LAST_SYNC_KEY, &now_iso())?;
    // This machine now IS the remote copy byte for byte, so the next push has
    // nothing to send - recording the hash of what was downloaded is what
    // lets it skip the upload instead of shipping it straight back.
    if let Ok(bytes) = std::fs::read(&downloaded) {
        let _ = set_setting(conn, PUSHED_HASH_KEY, &content_hash(&bytes));
    }
    // The restore itself is a write, and this machine's contents are now
    // exactly what is in Drive - so the flag is cleared here rather than
    // left set by the very operation that made the two sides agree.
    mark_local_clean(conn)?;
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
    // 2.18.0: the timer must never decide anything while a hand-pressed sync
    // is mid-flight - it would read a half-written state and then act on it.
    // Idle rather than an error: this fires every five minutes on its own, so
    // "nothing to do right now" is the truthful answer, not a failure.
    if sync_in_progress() {
        return Ok(CloudSyncAutoPlan {
            action: CloudSyncAutoAction::Idle,
            reason: "A sync is already running.".to_string(),
            local_dirty: false,
            remote_newer: false,
        });
    }

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
        Some(id) => retrying(|| get_remote_meta(&client, &access_token, &id)).map(Some),
        None => retrying(|| find_remote_file(&client, &access_token)),
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

    // --- one sync at a time, retries, change detection, state (2.18.0) ---

    #[test]
    fn a_second_sync_cannot_start_while_one_is_running() {
        // The race 2.17.0 created by moving these commands off the main
        // thread: the 5-minute timer firing into a hand-pressed Sync. Two
        // uploads decide the winner by whichever request finishes last.
        let first = SyncGuard::acquire().expect("nothing should be running");
        assert!(sync_in_progress());
        assert!(SyncGuard::acquire().is_none(), "a second sync must be refused, not queued");
        drop(first);
        assert!(!sync_in_progress());
        // And the lock is genuinely free again afterwards - a guard that
        // leaked would wedge every sync until the app restarted.
        let again = SyncGuard::acquire().expect("the guard must release on drop");
        drop(again);
    }

    #[test]
    fn an_authentication_failure_is_an_answer_and_is_never_retried() {
        // Retrying a rejected token twice more just makes marko wait longer
        // for the same message.
        for msg in ["Drive metadata failed (401) unauthorized", "failed (403) forbidden", "invalid_grant"] {
            assert!(!is_worth_retrying(&AppError::External(msg.to_string())), "{msg}");
        }
        // Nor is anything that was never a network problem.
        assert!(!is_worth_retrying(&AppError::Validation("Cloud sync is turned off.".into())));
        assert!(!is_worth_retrying(&AppError::Db("disk image is malformed".into())));
    }

    #[test]
    fn a_dropped_connection_is_worth_one_more_go() {
        assert!(is_worth_retrying(&AppError::External(
            "Couldn't reach Google Drive: connection reset".into()
        )));
        assert!(is_worth_retrying(&AppError::External("failed (503) unavailable".into())));
        assert!(is_worth_retrying(&AppError::External("failed (429) too many requests".into())));
    }

    #[test]
    fn retrying_stops_rather_than_going_on_forever() {
        use std::cell::Cell;
        let attempts = Cell::new(0);
        let result: AppResult<()> = retrying(|| {
            attempts.set(attempts.get() + 1);
            Err(AppError::External("failed (503) unavailable".into()))
        });
        assert!(result.is_err());
        assert_eq!(
            attempts.get(),
            RETRY_DELAYS_MS.len() + 1,
            "one attempt per delay, plus the first - and then it gives up"
        );
    }

    #[test]
    fn a_failure_that_is_not_worth_retrying_is_returned_immediately() {
        use std::cell::Cell;
        let attempts = Cell::new(0);
        let result: AppResult<()> = retrying(|| {
            attempts.set(attempts.get() + 1);
            Err(AppError::Validation("Cloud sync is turned off.".into()))
        });
        assert!(result.is_err());
        assert_eq!(attempts.get(), 1);
    }

    #[test]
    fn identical_bytes_hash_the_same_and_one_changed_byte_does_not() {
        // The whole question this answers: "are these the bytes I already
        // uploaded?" A wrong "changed" costs one upload; a wrong "unchanged"
        // is a 1-in-2^64 collision.
        let a = vec![1u8, 2, 3, 4, 5];
        let mut b = a.clone();
        assert_eq!(content_hash(&a), content_hash(&b));
        b[3] = 9;
        assert_ne!(content_hash(&a), content_hash(&b));
        // Length is part of it, so a truncated file is never mistaken for the
        // whole one.
        assert_ne!(content_hash(&a), content_hash(&a[..4]));
        assert_eq!(content_hash(&[]), content_hash(&[]));
    }

    // --- the one word the panel shows -----------------------------------
    //
    // Argument order: enabled, signed_in, syncing, conflict, reachable,
    // failed, local_changes, remote_newer.

    #[test]
    fn sync_that_is_off_or_signed_out_says_so_before_anything_else() {
        assert_eq!(summarize_state(false, true, false, true, true, true, true, true), "off");
        assert_eq!(summarize_state(true, false, false, true, true, true, true, true), "off");
    }

    #[test]
    fn a_sync_in_flight_outranks_everything_it_is_about_to_change() {
        assert_eq!(summarize_state(true, true, true, true, false, true, true, true), "syncing");
    }

    #[test]
    fn a_recorded_conflict_survives_being_offline() {
        // It is a fact about the data, not about the signal - hiding it
        // behind "offline" is how it would never get fixed.
        assert_eq!(summarize_state(true, true, false, true, false, false, false, false), "conflict");
    }

    #[test]
    fn both_sides_changed_reads_as_a_conflict_too() {
        assert_eq!(summarize_state(true, true, false, false, true, false, true, true), "conflict");
    }

    #[test]
    fn unreachable_drive_reads_as_offline_not_as_a_failure() {
        assert_eq!(summarize_state(true, true, false, false, false, false, false, false), "offline");
    }

    #[test]
    fn the_last_failure_is_shown_until_something_works() {
        assert_eq!(summarize_state(true, true, false, false, true, true, false, false), "failed");
    }

    #[test]
    fn one_side_changing_names_which_side() {
        assert_eq!(summarize_state(true, true, false, false, true, false, true, false), "localChanges");
        assert_eq!(summarize_state(true, true, false, false, true, false, false, true), "cloudChanges");
    }

    #[test]
    fn two_machines_that_agree_say_synced() {
        assert_eq!(summarize_state(true, true, false, false, true, false, false, false), "synced");
    }

    #[test]
    fn a_short_error_stays_short_enough_to_read() {
        let long = AppError::External("x".repeat(400));
        assert!(short_error(&long).chars().count() <= 180);
        assert!(short_error(&long).ends_with("..."));
        // And a multi-line failure is cut to its first line, not pasted whole.
        let multi = AppError::Other("Couldn't reach Drive\nbacktrace nonsense".into());
        assert_eq!(short_error(&multi), "Couldn't reach Drive");
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

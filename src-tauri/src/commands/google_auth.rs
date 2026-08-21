//! Tauri commands for "Sign in with Google" (2.0.5) - see google_oauth.rs's
//! module doc comment for the full flow and why it exists alongside, not
//! instead of, the shared service account (google_sheets.rs). This file
//! owns the storage side (one signed-in account per *installation* - not per
//! data source, since it is a property of who is running this particular
//! copy of the app, orthogonal to which spreadsheet is connected for Pulls)
//! and the three commands Settings calls; google_oauth.rs owns the actual
//! protocol work and has no database dependency of its own.
//!
//! Stored in the same app_settings key/value store every other setting
//! already uses (see commands/sheets_sync.rs) - the same local-SQLite-file
//! trust boundary this app already relies on for everything else it keeps
//! (buyer names, prices, the embedded service account's own private key at
//! rest in the compiled binary), not a new category of risk.

use crate::commands::sheets_sync::{delete_setting, get_setting, set_setting};
use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::google_oauth::{self, OAuthClient};
use crate::google_sheets;
use rusqlite::Connection;
use serde::Serialize;
use tauri::State;

const REFRESH_TOKEN_KEY: &str = "google_oauth_refresh_token";
const EMAIL_KEY: &str = "google_oauth_email";

/// What Settings shows for the installation-wide Google sign-in state.
#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GoogleSignInStatus {
    /// Whether this build was compiled with an OAuth client embedded (false
    /// on a plain local dev build) - same convention as
    /// SheetsConnectionStatus::sync_available on the service-account side.
    pub sign_in_available: bool,
    pub signed_in_email: Option<String>,
}

fn sign_in_status_impl(conn: &Connection) -> AppResult<GoogleSignInStatus> {
    Ok(GoogleSignInStatus {
        sign_in_available: google_oauth::embedded_oauth_client().is_some(),
        signed_in_email: get_setting(conn, EMAIL_KEY)?,
    })
}

#[tauri::command]
pub fn get_google_sign_in_status(state: State<AppState>) -> AppResult<GoogleSignInStatus> {
    let conn = state.db.lock().unwrap();
    sign_in_status_impl(&conn)
}

fn google_sign_out_impl(conn: &Connection) -> AppResult<()> {
    delete_setting(conn, REFRESH_TOKEN_KEY)?;
    delete_setting(conn, EMAIL_KEY)?;
    Ok(())
}

#[tauri::command]
pub fn google_sign_out(state: State<AppState>) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    google_sign_out_impl(&conn)
}

fn require_oauth_client() -> AppResult<OAuthClient> {
    google_oauth::embedded_oauth_client().ok_or_else(|| {
        AppError::External("Google sign-in isn't available in this build (no OAuth client configured).".to_string())
    })
}

// Not unit-tested beyond `require_oauth_client` above: the rest of this
// command opens a real browser window and blocks on a real human completing
// a real Google sign-in in it (google_oauth::run_sign_in) - there is nothing
// meaningful left to fake in an automated test once past the availability
// check, and this sandbox cannot reach Google's endpoints regardless (see
// google_oauth.rs's module doc comment). Reviewed by hand instead,
// mirroring pulls_sheet_sync.rs's split between a fully-tested offline core
// (apply_pull_rows) and an untested network-calling shell (sync_pulls_impl).
#[tauri::command]
pub fn start_google_sign_in(state: State<AppState>, app: tauri::AppHandle) -> AppResult<GoogleSignInStatus> {
    let client = require_oauth_client()?;
    let account = google_oauth::run_sign_in(&client, &app)?;
    let conn = state.db.lock().unwrap();
    set_setting(&conn, REFRESH_TOKEN_KEY, &account.refresh_token)?;
    set_setting(&conn, EMAIL_KEY, &account.email)?;
    sign_in_status_impl(&conn)
}

/// Returns a ready-to-use access token for the signed-in Google account, or
/// `Ok(None)` if nobody is signed in *in a way this build can act on* - no
/// refresh token stored, or this particular build has no OAuth client
/// embedded (see google_oauth::embedded_oauth_client's doc comment; a stored
/// refresh token from a build that did have one is simply inert here, not
/// an error). Every sheet-touching command
/// (commands::pulls_sheet_sync::{sync_pulls_impl, create_pulls_sheet_impl},
/// commands::sheets_sync::test_sheets_connection_impl) consults this first
/// and only falls back to the shared service account when it comes back
/// `None`.
///
/// A refresh token that *is* present but fails to actually refresh (Testing
/// mode's 7-day expiry, or the person revoked access in their own Google
/// account) is deliberately `Err`, not a silent `None` - falling back to the
/// service account in that case would use a different identity than the one
/// the person thinks they are using, with no explanation. Surfacing it
/// instead ("please sign in again") is the honest failure mode.
pub(crate) fn active_oauth_access_token(conn: &Connection) -> AppResult<Option<String>> {
    let Some(refresh_token) = get_setting(conn, REFRESH_TOKEN_KEY)? else {
        return Ok(None);
    };
    let Some(client) = google_oauth::embedded_oauth_client() else {
        return Ok(None);
    };
    let (access_token, _expires_at) = google_oauth::refresh_access_token(&client, &refresh_token)?;
    Ok(Some(access_token))
}

/// Which credential actually produced a token - callers use this to decide
/// whether a step the service-account path still needs (currently: sharing
/// a newly-created sheet by e-mail, see
/// commands::pulls_sheet_sync::create_pulls_sheet_impl) can be skipped
/// entirely, because a signed-in person's own new sheet already belongs to
/// them the moment it is created.
#[derive(Debug)]
pub(crate) enum GoogleCredential {
    OAuth { access_token: String },
    ServiceAccount { access_token: String },
}

impl GoogleCredential {
    pub(crate) fn access_token(&self) -> &str {
        match self {
            GoogleCredential::OAuth { access_token } | GoogleCredential::ServiceAccount { access_token } => access_token,
        }
    }

    pub(crate) fn is_oauth(&self) -> bool {
        matches!(self, GoogleCredential::OAuth { .. })
    }
}

/// The single place every sheet-touching command resolves "what bearer
/// token should this call use": the signed-in person's own OAuth token when
/// one is active, the shared service account otherwise - never both, and
/// OAuth always wins when it is genuinely usable (see
/// `active_oauth_access_token`'s doc comment for what "usable" means; a
/// build with neither configured falls through to the error below).
///
/// `need_drive` should be `true` only when falling back to the service
/// account for the one flow that still needs Drive access there (creating
/// *and sharing* a brand-new sheet - see
/// google_sheets::SHEETS_AND_DRIVE_SCOPE's doc comment); every other caller
/// passes `false`. The OAuth path never needs Drive access regardless of
/// this flag (see google_oauth::OAUTH_SCOPE's doc comment) - a signed-in
/// person's own new sheet needs no separate *share* step at all.
pub(crate) fn resolve_google_credential(conn: &Connection, need_drive: bool) -> AppResult<GoogleCredential> {
    if let Some(access_token) = active_oauth_access_token(conn)? {
        return Ok(GoogleCredential::OAuth { access_token });
    }
    let account = google_sheets::embedded_service_account().ok_or_else(|| {
        AppError::External(
            "Google Sheets isn't available in this build - sign in with Google, or use a build configured with a service account.".to_string(),
        )
    })?;
    let scope = if need_drive { google_sheets::SHEETS_AND_DRIVE_SCOPE } else { google_sheets::SHEETS_SCOPE };
    let access_token = google_sheets::fetch_access_token(&account, scope)?;
    Ok(GoogleCredential::ServiceAccount { access_token })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_conn;

    #[test]
    fn nobody_signed_in_reports_none_not_an_error() {
        let conn = test_conn();
        let status = sign_in_status_impl(&conn).unwrap();
        assert_eq!(status.signed_in_email, None);
        // This test suite never has GOOGLE_OAUTH_CLIENT_ID set (build.rs
        // falls back to an empty embed), so this must hold here too.
        assert!(!status.sign_in_available);
    }

    #[test]
    fn signed_in_email_round_trips_through_app_settings() {
        let conn = test_conn();
        set_setting(&conn, EMAIL_KEY, "marko@example.com").unwrap();
        let status = sign_in_status_impl(&conn).unwrap();
        assert_eq!(status.signed_in_email, Some("marko@example.com".to_string()));
    }

    #[test]
    fn sign_out_forgets_both_the_refresh_token_and_the_email() {
        let conn = test_conn();
        set_setting(&conn, REFRESH_TOKEN_KEY, "fake-refresh-token").unwrap();
        set_setting(&conn, EMAIL_KEY, "marko@example.com").unwrap();

        google_sign_out_impl(&conn).unwrap();

        assert_eq!(get_setting(&conn, REFRESH_TOKEN_KEY).unwrap(), None);
        let status = sign_in_status_impl(&conn).unwrap();
        assert_eq!(status.signed_in_email, None);
    }

    #[test]
    fn require_oauth_client_fails_cleanly_when_none_is_embedded() {
        let err = require_oauth_client().unwrap_err();
        assert!(err.to_string().contains("isn't available in this build"));
    }

    #[test]
    fn active_oauth_access_token_is_none_when_nobody_is_signed_in() {
        let conn = test_conn();
        assert_eq!(active_oauth_access_token(&conn).unwrap(), None);
    }

    #[test]
    fn resolve_google_credential_fails_cleanly_when_nothing_is_configured() {
        // Neither OAuth nor the service account is embedded in this test
        // build (both fall back to "not configured" - see
        // embedded_oauth_client's and embedded_service_account's doc
        // comments), so this must hold here, without ever attempting a
        // network call.
        let conn = test_conn();
        let err = resolve_google_credential(&conn, false).unwrap_err();
        assert!(err.to_string().contains("isn't available in this build"));
    }

    #[test]
    fn active_oauth_access_token_is_none_when_a_refresh_token_exists_but_this_build_has_no_client() {
        // Simulates a database that already has a signed-in account (e.g.
        // copied from a real build) opened by a build with no OAuth client
        // embedded (any local cargo test/build, per this module's doc
        // comment) - must stay a quiet None, never an error, exactly like
        // google_sheets::embedded_service_account's "not configured"
        // convention.
        let conn = test_conn();
        set_setting(&conn, REFRESH_TOKEN_KEY, "fake-refresh-token").unwrap();
        assert_eq!(active_oauth_access_token(&conn).unwrap(), None);
    }
}

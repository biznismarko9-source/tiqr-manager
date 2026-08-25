//! Tauri commands for "Continue with Google" (2.0.46) - the Welcome
//! screen's app sign-in button (lib/auth.tsx's `loginWithGoogle`), NOT the
//! existing "Sign in with Google" in Settings (commands::google_auth). See
//! google_oauth.rs's module doc comment ("also reused for...") for the full
//! reasoning on why these are two separate flows sharing one underlying
//! OAuth implementation rather than one flow serving both purposes.
//!
//! This module is deliberately much smaller than commands::google_auth:
//! there is nothing to persist here. Once this app hands Firebase the ID
//! token `start_firebase_google_sign_in` produces
//! (`GoogleAuthProvider.credential(idToken)` +
//! `signInWithCredential(auth, credential)`, done on the frontend in
//! lib/auth.tsx), Firebase's own JS SDK owns that session end to end -
//! nothing about who is signed into the app itself lives in this app's own
//! SQLite database, matching this whole feature's founding principle that
//! the account/identity layer stays completely separate from the app's own
//! (local-only) data.

use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::google_oauth::{self, OAuthClient};
use serde::Serialize;
use tauri::State;

/// What `start_firebase_google_sign_in` hands back to the frontend - just
/// enough to complete the Firebase side of the sign-in
/// (`GoogleAuthProvider.credential(idToken)`), nothing more.
#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FirebaseGoogleSignInResult {
    pub id_token: String,
}

fn require_firebase_oauth_client() -> AppResult<OAuthClient> {
    google_oauth::embedded_firebase_oauth_client().ok_or_else(|| {
        AppError::External("Google sign-in isn't available in this build (no OAuth client configured).".to_string())
    })
}

/// Lets the Welcome screen show "Continue with Google" as a normal, enabled
/// button only when this build can actually complete the flow - same
/// "never silently fake it" honesty as
/// commands::google_auth::GoogleSignInStatus::sign_in_available, just
/// returned as a plain bool since there is no signed-in-email state to
/// report alongside it here (see this module's own doc comment).
#[tauri::command]
pub fn firebase_google_sign_in_available() -> AppResult<bool> {
    Ok(google_oauth::embedded_firebase_oauth_client().is_some())
}

// Not unit-tested beyond `require_firebase_oauth_client` above, for exactly
// the same reason as commands::google_auth::start_google_sign_in: the rest
// of this opens a real browser window and blocks on a real human completing
// a real Google sign-in in it. Mirrors that command's `async` +
// `spawn_blocking` shape precisely (see its own doc comment for why plain,
// non-`async` Tauri commands would block the main thread and make
// `cancel_firebase_google_sign_in` unreachable in time) - reviewed by hand
// instead.
#[tauri::command]
pub async fn start_firebase_google_sign_in(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> AppResult<FirebaseGoogleSignInResult> {
    let client = require_firebase_oauth_client()?;

    let cancel_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    *state.firebase_oauth_cancel_flag.lock().unwrap() = Some(cancel_flag.clone());

    let cancel_for_task = cancel_flag.clone();
    let app_for_task = app.clone();
    let result: AppResult<google_oauth::SignedInAccount> = tauri::async_runtime::spawn_blocking(move || {
        google_oauth::run_sign_in(&client, google_oauth::FIREBASE_SIGN_IN_SCOPE, &app_for_task, &cancel_for_task)
    })
    .await
    .map_err(|e| AppError::External(format!("the sign-in task did not complete cleanly: {e}")))?;

    *state.firebase_oauth_cancel_flag.lock().unwrap() = None;
    let account = result?;

    Ok(FirebaseGoogleSignInResult { id_token: account.id_token })
}

/// "Cancel" shown on the Welcome screen while waiting on the browser - same
/// UX fix as commands::google_auth::cancel_google_sign_in (2.0.12), reusing
/// its actual logic (`cancel_google_sign_in_impl` is generic over any
/// cancel-flag slot) against this flow's own, separate flag.
#[tauri::command]
pub fn cancel_firebase_google_sign_in(state: State<AppState>) -> AppResult<()> {
    super::google_auth::cancel_google_sign_in_impl(&state.firebase_oauth_cancel_flag);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn require_firebase_oauth_client_fails_cleanly_when_none_is_embedded() {
        // This test suite never has FIREBASE_GOOGLE_OAUTH_CLIENT_ID set
        // (build.rs falls back to an empty embed), so this must hold here.
        let err = require_firebase_oauth_client().unwrap_err();
        assert!(err.to_string().contains("isn't available in this build"));
    }

    #[test]
    fn firebase_google_sign_in_available_is_false_on_a_plain_local_build() {
        assert!(!firebase_google_sign_in_available().unwrap());
    }

    // cancel_firebase_google_sign_in itself is a thin State<AppState> wrapper
    // around commands::google_auth::cancel_google_sign_in_impl, which is
    // already exhaustively tested (safe no-op, flips the flag, never touches
    // an unrelated one) in commands/google_auth.rs's own test module -
    // re-testing that same pure function's behavior again here would only be
    // testing Rust's function-call semantics, not anything specific to this
    // module. Which AppState field the wrapper passes in is reviewed by hand
    // (db.rs gives the two flows separate slots - see its own doc comment),
    // matching this codebase's established boundary for OAuth-adjacent code
    // that needs a running Tauri app to exercise the wiring itself.
}

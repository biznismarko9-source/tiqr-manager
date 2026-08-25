//! "Sign in with Google" (2.0.5) - a per-person alternative to the shared
//! service account (google_sheets.rs). The service account is one fixed
//! machine identity embedded in every copy of this app; every sheet anyone
//! connects or creates through it is technically reachable by that same
//! identity, which is a fine tradeoff for a small, personally-distributed
//! group (see google_sheets.rs's module doc comment) but not real per-person
//! isolation. This module gives each person who runs their own copy of the
//! app a way to sign in with their OWN Google account instead, so pulling
//! and pushing sheet data happens as *them* - genuinely separate identities,
//! not one shared key.
//!
//! ## 2.0.46: also reused for "Continue with Google" app sign-in
//!
//! The generic parts of this file (PKCE, the loopback listener, building the
//! authorization URL) are shared with a SECOND, unrelated flow:
//! commands::firebase_google_auth, the "Continue with Google" button on the
//! Welcome screen (see lib/auth.tsx). That flow signs a person into the app
//! ITSELF (Firebase Authentication - who is using this copy of the app),
//! completely separate from this module's original purpose (Google identity
//! acting on Sheets calls). Different Google Cloud project (marko's Firebase
//! project, not the one this module's own OAuth client lives in), different
//! scope (`FIREBASE_SIGN_IN_SCOPE`, identity only - no Sheets/Drive access),
//! different embedded client (`embedded_firebase_oauth_client`), different
//! Tauri commands, and no database persistence here at all - Firebase's own
//! JS SDK owns that flow's session once it has the ID token this module
//! hands back. `run_sign_in`/`build_authorization_url` take `scope` as a
//! plain parameter specifically so both flows share one well-tested
//! implementation instead of two near-identical copies.
//!
//! ## The flow (RFC 8252 "OAuth 2.0 for Native Apps", PKCE / RFC 7636)
//!
//! 1. Generate a random `code_verifier` and its SHA-256 `code_challenge`
//!    (PKCE) plus a random `state` (CSRF protection) - `generate_pkce_pair`/
//!    `generate_state`.
//! 2. Start a listener on `127.0.0.1` on an OS-assigned free port
//!    (`bind_loopback_listener`) - no async runtime, no extra HTTP server
//!    crate: this only ever needs to read exactly one raw GET request off
//!    exactly one connection, which a few lines of `std::net::TcpListener`
//!    handles directly (`accept_one_redirect`), matching how every other
//!    network-touching piece of this app avoids pulling in machinery it
//!    does not need (see google_sheets.rs's own module doc comment).
//! 3. Open the person's own default browser (tauri-plugin-opener) to
//!    Google's consent page (`build_authorization_url`), with that port's
//!    address as the `redirect_uri`. Google explicitly supports an
//!    arbitrary, unregistered port here for "Desktop app" OAuth clients -
//!    the "loopback IP address" flow - nothing to pre-register beyond the
//!    client ID itself.
//! 4. The person signs in and consents in their own real browser, under
//!    their own real Google account - never inside this app, never on a
//!    screen this app draws or controls.
//! 5. Google redirects that browser back to the local port with an
//!    authorization `code`; the listener from step 2 catches it, and this
//!    app exchanges the code for an access + refresh token pair directly
//!    with Google, server to server (`exchange_code_for_tokens`) - the
//!    browser's only role was steps 3-4.
//! 6. The refresh token (plus the signed-in email) is handed back to the
//!    caller to store - see commands::google_auth for where and why that is
//!    the same trust boundary as everything else this app already keeps
//!    locally, not a new one.
//!
//! ## Scope - deliberately narrower than the service account's
//!
//! `OAUTH_SCOPE` below has no `drive.file`, unlike
//! google_sheets::SHEETS_AND_DRIVE_SCOPE: "create a new sheet" needs no
//! separate *share* step once the person is signed in as themselves - it is
//! already their own file the moment the Sheets API creates it, so there is
//! nothing for a Drive scope to do here. `openid email` (both standard,
//! non-sensitive scopes) is added instead, purely so this app can show
//! "Signed in as ..." - it never reads anything else about the person's
//! Google account.
//!
//! ## What can and can't be verified in this sandbox
//!
//! Same limitation as google_sheets.rs (see its module doc comment):
//! `accounts.google.com`/`oauth2.googleapis.com` are unreachable from here,
//! so `exchange_code_for_tokens`/`refresh_access_token`/`fetch_email` can't
//! be exercised end to end. Fully offline and unit-tested below: PKCE
//! generation, the authorization URL it builds, and the loopback listener's
//! raw-HTTP-request parsing (success, declined-consent, and malformed
//! cases) - the parts that do not require an actual network round trip.

use crate::error::{AppError, AppResult};
use crate::google_sheets::parse_json_response;
use chrono::{DateTime, Utc};
use percent_encoding::{percent_decode_str, utf8_percent_encode, NON_ALPHANUMERIC};
use rand::RngCore;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

const AUTH_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
const USERINFO_ENDPOINT: &str = "https://www.googleapis.com/oauth2/v3/userinfo";

/// See this module's doc comment ("Scope") for why this is narrower than
/// google_sheets::SHEETS_AND_DRIVE_SCOPE.
pub const OAUTH_SCOPE: &str = "https://www.googleapis.com/auth/spreadsheets openid email";

/// 2.0.46: scope for the SEPARATE "Continue with Google" app sign-in button
/// (commands::firebase_google_auth) - identity only, no Sheets/Drive access
/// at all. `profile` (not requested by OAUTH_SCOPE above) is what lets
/// Firebase fill in a real display name automatically once this app hands
/// it the resulting ID token - see lib/auth.tsx's `toAuthUser`.
pub const FIREBASE_SIGN_IN_SCOPE: &str = "openid email profile";

/// How long `run_sign_in` waits for the person to finish in their browser
/// before giving up - generous on purpose (they might be typing a password,
/// picking an account, or reading the consent screen), but bounded so a
/// closed tab or an abandoned attempt doesn't hang the app forever.
const SIGN_IN_TIMEOUT: Duration = Duration::from_secs(300);

const EMBEDDED_OAUTH_CLIENT_ID: &str = include_str!(concat!(env!("OUT_DIR"), "/google_oauth_client_id.txt"));
const EMBEDDED_OAUTH_CLIENT_SECRET: &str = include_str!(concat!(env!("OUT_DIR"), "/google_oauth_client_secret.txt"));

/// The OAuth client this build was compiled with - see build.rs's doc
/// comment. `client_secret` is `None` when empty: Google documents a Desktop
/// app's client_secret as not needing to be kept confidential (PKCE is what
/// actually secures this flow), so shipping without one is a valid,
/// supported configuration, not a broken one.
#[derive(Debug, Clone)]
pub struct OAuthClient {
    pub client_id: String,
    pub client_secret: Option<String>,
}

/// Returns the OAuth client this build was compiled with, or `None` if this
/// is a local/dev build that never had GOOGLE_OAUTH_CLIENT_ID injected -
/// same "not configured in this build" convention as
/// google_sheets::embedded_service_account.
pub fn embedded_oauth_client() -> Option<OAuthClient> {
    let client_id = EMBEDDED_OAUTH_CLIENT_ID.trim();
    if client_id.is_empty() {
        return None;
    }
    let secret = EMBEDDED_OAUTH_CLIENT_SECRET.trim();
    Some(OAuthClient {
        client_id: client_id.to_string(),
        client_secret: if secret.is_empty() { None } else { Some(secret.to_string()) },
    })
}

const EMBEDDED_FIREBASE_OAUTH_CLIENT_ID: &str =
    include_str!(concat!(env!("OUT_DIR"), "/firebase_google_oauth_client_id.txt"));
const EMBEDDED_FIREBASE_OAUTH_CLIENT_SECRET: &str =
    include_str!(concat!(env!("OUT_DIR"), "/firebase_google_oauth_client_secret.txt"));

/// 2.0.46: same "not configured in this build" convention as
/// `embedded_oauth_client` above, for the separate OAuth client the
/// "Continue with Google" app sign-in button uses - see this module's own
/// doc comment for why these are two distinct clients, not one reused.
pub fn embedded_firebase_oauth_client() -> Option<OAuthClient> {
    let client_id = EMBEDDED_FIREBASE_OAUTH_CLIENT_ID.trim();
    if client_id.is_empty() {
        return None;
    }
    let secret = EMBEDDED_FIREBASE_OAUTH_CLIENT_SECRET.trim();
    Some(OAuthClient {
        client_id: client_id.to_string(),
        client_secret: if secret.is_empty() { None } else { Some(secret.to_string()) },
    })
}

// ---------------------------------------------------------------------------
// PKCE (RFC 7636) - pure and offline, hence fully unit-tested below.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct PkcePair {
    pub verifier: String,
    pub challenge: String,
}

fn base64_url_encode(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn random_url_safe_token(num_bytes: usize) -> String {
    let mut bytes = vec![0u8; num_bytes];
    rand::rng().fill_bytes(&mut bytes);
    base64_url_encode(&bytes)
}

/// A fresh PKCE pair: 32 random bytes base64url-encoded (43 characters,
/// within RFC 7636's required 43-128 character range) as the `verifier`,
/// and its SHA-256 hash (also base64url) as the `challenge` sent up front -
/// Google only ever sees the challenge until the final token exchange, which
/// is what proves this app (not an attacker who merely observed the
/// redirect) is the one completing the sign-in it started.
pub fn generate_pkce_pair() -> PkcePair {
    let verifier = random_url_safe_token(32);
    let challenge = base64_url_encode(&Sha256::digest(verifier.as_bytes()));
    PkcePair { verifier, challenge }
}

/// A random CSRF `state` value - proves the redirect this app's listener
/// receives actually answers the authorization request *this* run started,
/// not a stale or forged one.
pub fn generate_state() -> String {
    random_url_safe_token(16)
}

fn build_authorization_url(client: &OAuthClient, redirect_uri: &str, scope: &str, pkce: &PkcePair, state: &str) -> String {
    let params: &[(&str, &str)] = &[
        ("client_id", &client.client_id),
        ("redirect_uri", redirect_uri),
        ("response_type", "code"),
        ("scope", scope),
        ("code_challenge", &pkce.challenge),
        ("code_challenge_method", "S256"),
        ("state", state),
        // offline -> a refresh token is issued, not just a short-lived
        // access token; consent -> Google issues a fresh refresh token even
        // if this person already signed in before (otherwise a repeat
        // sign-in can silently come back with no refresh_token at all).
        ("access_type", "offline"),
        ("prompt", "consent"),
    ];
    let query = params
        .iter()
        .map(|(k, v)| format!("{k}={}", utf8_percent_encode(v, NON_ALPHANUMERIC)))
        .collect::<Vec<_>>()
        .join("&");
    format!("{AUTH_ENDPOINT}?{query}")
}

// ---------------------------------------------------------------------------
// The loopback listener - catches exactly one redirect, then is done.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq)]
struct RedirectResult {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

fn bind_loopback_listener() -> AppResult<TcpListener> {
    TcpListener::bind("127.0.0.1:0")
        .map_err(|e| AppError::External(format!("could not open a local port for Google sign-in: {e}")))
}

fn redirect_uri_for(listener: &TcpListener) -> AppResult<String> {
    let port = listener
        .local_addr()
        .map_err(|e| AppError::External(format!("could not read the local sign-in port: {e}")))?
        .port();
    Ok(format!("http://127.0.0.1:{port}"))
}

/// Parses just the request line of a raw HTTP request, e.g.
/// `GET /?code=4/0Adeu...&state=abc123 HTTP/1.1` (or `?error=access_denied`
/// if the person declined consent) - the only thing this app ever needs out
/// of Google's redirect, so nothing beyond that first line is even read.
fn parse_redirect_request_line(line: &str) -> RedirectResult {
    let path_and_query = line.split_whitespace().nth(1).unwrap_or("");
    let query = path_and_query.splitn(2, '?').nth(1).unwrap_or("");
    let mut result = RedirectResult::default();
    for pair in query.split('&').filter(|p| !p.is_empty()) {
        let mut parts = pair.splitn(2, '=');
        let key = parts.next().unwrap_or("");
        let raw_value = parts.next().unwrap_or("");
        let value = percent_decode_str(raw_value).decode_utf8_lossy().into_owned();
        match key {
            "code" => result.code = Some(value),
            "state" => result.state = Some(value),
            "error" => result.error = Some(value),
            _ => {}
        }
    }
    result
}

/// Blocks until exactly one connection arrives on `listener` (Google's
/// redirect, once the person finishes in their browser), `timeout` elapses
/// first, or `cancel` is flipped to `true` from elsewhere (see
/// commands::google_auth::cancel_google_sign_in - the "Cancel" button shown
/// while Settings reads "Waiting for you to finish in your browser...") -
/// then answers a real connection with a plain "you can close this tab" page
/// and shuts down. This listener only ever serves this one request.
///
/// The `cancel` check exists because a closed browser tab, or picking
/// "use another account" and never finishing, leaves this otherwise blocked
/// for the full `timeout` (a generous 5 minutes - see `SIGN_IN_TIMEOUT`)
/// with no way back into the app: marko's own report was that "Sign in with
/// Google", left uncompleted, made the whole sign-in card look frozen until
/// he restarted the app. `cancel` is checked on the same 200ms poll this
/// loop already runs for the timeout check, so a cancellation is noticed
/// within one polling interval, not the full wait.
fn accept_one_redirect(listener: &TcpListener, timeout: Duration, cancel: &AtomicBool) -> AppResult<RedirectResult> {
    listener
        .set_nonblocking(true)
        .map_err(|e| AppError::External(format!("could not configure the local sign-in listener: {e}")))?;
    let start = std::time::Instant::now();
    let mut stream = loop {
        match listener.accept() {
            Ok((stream, _)) => break stream,
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if cancel.load(Ordering::Relaxed) {
                    return Err(AppError::Validation("Google sign-in was cancelled.".to_string()));
                }
                if start.elapsed() > timeout {
                    return Err(AppError::External(
                        "Timed out waiting for Google sign-in - the browser window may have been closed before finishing. Please try again.".to_string(),
                    ));
                }
                std::thread::sleep(Duration::from_millis(200));
            }
            Err(e) => return Err(AppError::External(format!("local sign-in listener failed: {e}"))),
        }
    };
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .map_err(|e| AppError::External(format!("could not configure the local sign-in connection: {e}")))?;

    let request_line = {
        let mut reader = BufReader::new(&stream);
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .map_err(|e| AppError::External(format!("could not read Google's redirect: {e}")))?;
        line
    };
    let result = parse_redirect_request_line(&request_line);

    let body = "You can close this tab and return to TIQR Manager.";
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    // Best-effort: whether or not the browser is still there to receive this
    // response, this app already has everything it needs from the request
    // line above, so a failed write here is not itself an error.
    let _ = stream.write_all(response.as_bytes());

    Ok(result)
}

// ---------------------------------------------------------------------------
// Token exchange / refresh / identity - real network calls, see this
// module's doc comment for why they can't be exercised in this sandbox.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    expires_in: i64,
    /// Present whenever `openid` is in the requested scope (both
    /// OAUTH_SCOPE and FIREBASE_SIGN_IN_SCOPE include it) - a signed JWT
    /// asserting who signed in, which is exactly what
    /// commands::firebase_google_auth needs to hand Firebase
    /// (GoogleAuthProvider.credential(idToken) + signInWithCredential).
    /// `#[serde(default)]` defensively, same reasoning as refresh_token
    /// above, even though it is expected every time this scope is used.
    #[serde(default)]
    id_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UserinfoResponse {
    email: String,
}

fn exchange_code_for_tokens(client: &OAuthClient, code: &str, verifier: &str, redirect_uri: &str) -> AppResult<TokenResponse> {
    let http = reqwest::blocking::Client::new();
    let mut form: Vec<(&str, &str)> = vec![
        ("client_id", &client.client_id),
        ("code", code),
        ("code_verifier", verifier),
        ("grant_type", "authorization_code"),
        ("redirect_uri", redirect_uri),
    ];
    if let Some(secret) = &client.client_secret {
        form.push(("client_secret", secret));
    }
    let resp = http
        .post(TOKEN_ENDPOINT)
        .form(&form)
        .send()
        .map_err(|e| AppError::External(format!("could not reach Google to finish signing in: {e}")))?;
    parse_json_response(resp)
}

/// Exchanges a stored refresh token for a fresh access token - call this
/// whenever the cached access token has expired (or is close enough to).
/// The refresh token itself is long-lived and is not consumed by this call.
pub fn refresh_access_token(client: &OAuthClient, refresh_token: &str) -> AppResult<(String, DateTime<Utc>)> {
    let http = reqwest::blocking::Client::new();
    let mut form: Vec<(&str, &str)> = vec![
        ("client_id", &client.client_id),
        ("refresh_token", refresh_token),
        ("grant_type", "refresh_token"),
    ];
    if let Some(secret) = &client.client_secret {
        form.push(("client_secret", secret));
    }
    let resp = http
        .post(TOKEN_ENDPOINT)
        .form(&form)
        .send()
        .map_err(|e| AppError::External(format!("could not reach Google to refresh sign-in: {e}")))?;
    let parsed: TokenResponse = parse_json_response(resp)?;
    Ok((parsed.access_token, Utc::now() + chrono::Duration::seconds(parsed.expires_in)))
}

fn fetch_email(access_token: &str) -> AppResult<String> {
    let http = reqwest::blocking::Client::new();
    let resp = http
        .get(USERINFO_ENDPOINT)
        .bearer_auth(access_token)
        .send()
        .map_err(|e| AppError::External(format!("could not read the signed-in Google account: {e}")))?;
    let parsed: UserinfoResponse = parse_json_response(resp)?;
    Ok(parsed.email)
}

/// What one completed `run_sign_in` call produces: a signed-in person's
/// e-mail and the long-lived refresh token needed to act as them afterward.
/// Deliberately does not also carry the access token the token exchange
/// happened to return: every caller in this app fetches a fresh one via
/// `refresh_access_token` right before it is actually needed (see
/// commands::google_auth::active_oauth_access_token's doc comment for why
/// "always refresh, never cache" is the simpler and preferred choice here)
/// rather than separately tracking an access token's expiry from the moment
/// sign-in finished to whenever it is first used.
#[derive(Debug, Clone)]
pub struct SignedInAccount {
    pub email: String,
    pub refresh_token: String,
    /// 2.0.46: only actually read by commands::firebase_google_auth (the
    /// Sheets sign-in path, commands::google_auth, has no use for it and
    /// simply ignores it) - see this module's own doc comment for why one
    /// shared `run_sign_in` serves both flows rather than two copies.
    pub id_token: String,
}

/// The pure, offline part of turning a raw token-endpoint response into
/// what this app actually needs - split out from `run_sign_in` specifically
/// so it can be unit-tested without a real network round trip (see this
/// module's doc comment, "What can and can't be verified in this sandbox").
/// Both `refresh_token` and `id_token` are contractually present whenever
/// `openid` is in the requested scope and `access_type=offline` was sent
/// (see `build_authorization_url`) - Google not honoring that is treated as
/// an external failure, not a `None`/silent skip, so a caller never
/// half-completes a sign-in with a `SignedInAccount` missing a field it
/// actually needs.
fn signed_in_account_from_tokens(tokens: TokenResponse, email: String) -> AppResult<SignedInAccount> {
    let refresh_token = tokens.refresh_token.ok_or_else(|| {
        AppError::External("Google did not return a long-lived sign-in - please try signing in again.".to_string())
    })?;
    let id_token = tokens
        .id_token
        .ok_or_else(|| AppError::External("Google did not return a sign-in token - please try signing in again.".to_string()))?;
    Ok(SignedInAccount { email, refresh_token, id_token })
}

/// Runs one full "Sign in with Google" round trip end to end: opens the
/// person's browser, waits for them to finish in it, exchanges the result
/// for tokens, and looks up which account they signed in as. Blocking, with
/// a generous timeout (`SIGN_IN_TIMEOUT`) - matches how every other command
/// in this app is a plain synchronous function (see db.rs's AppState doc
/// comment), no async runtime needed here either. `cancel` is a fresh flag
/// the caller creates for this one attempt (see
/// commands::google_auth::start_google_sign_in, or
/// commands::firebase_google_auth::start_firebase_google_sign_in for the
/// other caller - see this module's own doc comment for why there are two)
/// - flipping it to `true` from elsewhere unblocks `accept_one_redirect`
/// below promptly instead of leaving this waiting out the full timeout.
/// `scope` is the one thing that actually differs between those two
/// callers (`OAUTH_SCOPE` vs `FIREBASE_SIGN_IN_SCOPE`) - `client` differs
/// too, but is already a parameter for other reasons.
pub fn run_sign_in(client: &OAuthClient, scope: &str, app: &tauri::AppHandle, cancel: &AtomicBool) -> AppResult<SignedInAccount> {
    use tauri_plugin_opener::OpenerExt;

    let listener = bind_loopback_listener()?;
    let uri = redirect_uri_for(&listener)?;
    let pkce = generate_pkce_pair();
    let state = generate_state();
    let auth_url = build_authorization_url(client, &uri, scope, &pkce, &state);

    app.opener()
        .open_url(auth_url, None::<&str>)
        .map_err(|e| AppError::External(format!("could not open the browser for Google sign-in: {e}")))?;

    let redirect = accept_one_redirect(&listener, SIGN_IN_TIMEOUT, cancel)?;

    if let Some(err) = redirect.error {
        return Err(AppError::Validation(format!("Google sign-in was not completed: {err}")));
    }
    let code = redirect
        .code
        .ok_or_else(|| AppError::External("Google's redirect did not include an authorization code.".to_string()))?;
    if redirect.state.as_deref() != Some(state.as_str()) {
        return Err(AppError::External(
            "Google sign-in response did not match this request (possible interference) - please try again.".to_string(),
        ));
    }

    let tokens = exchange_code_for_tokens(client, &code, &pkce.verifier, &uri)?;
    let email = fetch_email(&tokens.access_token)?;

    signed_in_account_from_tokens(tokens, email)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_client() -> OAuthClient {
        OAuthClient { client_id: "test-client-id.apps.googleusercontent.com".to_string(), client_secret: None }
    }

    #[test]
    fn embedded_oauth_client_is_none_on_a_plain_local_build() {
        // This test suite never has GOOGLE_OAUTH_CLIENT_ID set (build.rs
        // falls back to an empty embed - see its doc comment), so this must
        // hold in exactly the environment this test actually runs in.
        assert!(embedded_oauth_client().is_none(), "a local cargo test/build must never have a real OAuth client embedded");
    }

    #[test]
    fn pkce_verifier_is_within_rfc_7636s_required_length_range() {
        let pair = generate_pkce_pair();
        assert!(pair.verifier.len() >= 43 && pair.verifier.len() <= 128, "got length {}", pair.verifier.len());
    }

    #[test]
    fn pkce_verifier_and_challenge_use_only_url_safe_characters() {
        let pair = generate_pkce_pair();
        let is_url_safe = |s: &str| s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
        assert!(is_url_safe(&pair.verifier));
        assert!(is_url_safe(&pair.challenge));
    }

    #[test]
    fn pkce_challenge_is_deterministic_given_the_same_verifier() {
        // The whole point of PKCE: Google independently recomputes the
        // challenge from the verifier this app reveals at the very end, so
        // this transform must be pure and reproducible.
        let pair = generate_pkce_pair();
        let recomputed = base64_url_encode(&Sha256::digest(pair.verifier.as_bytes()));
        assert_eq!(pair.challenge, recomputed);
    }

    #[test]
    fn two_pkce_pairs_are_never_the_same() {
        let a = generate_pkce_pair();
        let b = generate_pkce_pair();
        assert_ne!(a.verifier, b.verifier, "reusing a verifier would defeat the point of PKCE");
    }

    #[test]
    fn two_states_are_never_the_same() {
        assert_ne!(generate_state(), generate_state());
    }

    #[test]
    fn authorization_url_carries_every_required_param_and_the_exact_redirect_uri() {
        let client = test_client();
        let pkce = generate_pkce_pair();
        let url = build_authorization_url(&client, "http://127.0.0.1:54321", OAUTH_SCOPE, &pkce, "the-state-value");

        assert!(url.starts_with(AUTH_ENDPOINT));
        // NON_ALPHANUMERIC (same choice google_sheets.rs already makes for
        // its own query values) percent-encodes '-' and '.' too, so a
        // realistic client ID like "test-client-id.apps..." does not survive
        // as a literal substring - decode it back out instead of assuming
        // which characters happened to pass through unencoded.
        assert!(url.contains("client_id="));
        assert_eq!(decoded_query_param(&url, "client_id"), client.client_id);
        assert_eq!(decoded_query_param(&url, "state"), "the-state-value");
        assert_eq!(decoded_query_param(&url, "code_challenge"), pkce.challenge);
        assert_eq!(decoded_query_param(&url, "scope"), OAUTH_SCOPE);
        assert!(url.contains("response_type=code"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("access_type=offline"));
        assert!(url.contains("prompt=consent"));
        // The redirect_uri must survive round-trip intact - Google rejects
        // the whole request if this does not byte-for-byte match what is
        // sent to the token endpoint later.
        assert_eq!(decoded_query_param(&url, "redirect_uri"), "http://127.0.0.1:54321");
    }

    #[test]
    fn authorization_url_uses_whichever_scope_the_caller_passes_in() {
        // 2.0.46: proves `scope` is actually threaded through, not just
        // accepted and ignored - the two real callers pass different scopes
        // (OAUTH_SCOPE for Sheets sign-in, FIREBASE_SIGN_IN_SCOPE for the
        // "Continue with Google" app sign-in button) and must never cross.
        let client = test_client();
        let pkce = generate_pkce_pair();
        let url = build_authorization_url(&client, "http://127.0.0.1:54321", FIREBASE_SIGN_IN_SCOPE, &pkce, "s");
        assert_eq!(decoded_query_param(&url, "scope"), FIREBASE_SIGN_IN_SCOPE);
        assert_ne!(FIREBASE_SIGN_IN_SCOPE, OAUTH_SCOPE, "the two scopes must actually differ for this test to mean anything");
    }

    #[test]
    fn embedded_firebase_oauth_client_is_none_on_a_plain_local_build() {
        // Same reasoning as embedded_oauth_client_is_none_on_a_plain_local_build
        // above - this test suite never has FIREBASE_GOOGLE_OAUTH_CLIENT_ID set.
        assert!(
            embedded_firebase_oauth_client().is_none(),
            "a local cargo test/build must never have a real Firebase OAuth client embedded"
        );
    }

    fn token_response(access_token: &str, refresh_token: Option<&str>, id_token: Option<&str>) -> TokenResponse {
        TokenResponse {
            access_token: access_token.to_string(),
            refresh_token: refresh_token.map(str::to_string),
            expires_in: 3600,
            id_token: id_token.map(str::to_string),
        }
    }

    #[test]
    fn signed_in_account_from_tokens_succeeds_when_both_refresh_and_id_tokens_are_present() {
        let tokens = token_response("access", Some("refresh"), Some("id"));
        let account = signed_in_account_from_tokens(tokens, "marko@example.com".to_string()).unwrap();
        assert_eq!(account.email, "marko@example.com");
        assert_eq!(account.refresh_token, "refresh");
        assert_eq!(account.id_token, "id");
    }

    #[test]
    fn signed_in_account_from_tokens_fails_cleanly_when_google_omits_the_refresh_token() {
        let tokens = token_response("access", None, Some("id"));
        let err = signed_in_account_from_tokens(tokens, "marko@example.com".to_string()).unwrap_err();
        assert!(err.to_string().contains("long-lived sign-in"));
    }

    #[test]
    fn signed_in_account_from_tokens_fails_cleanly_when_google_omits_the_id_token() {
        // 2.0.46: the case commands::firebase_google_auth actually depends
        // on - without this, `run_sign_in` would happily return a
        // SignedInAccount whose id_token is empty, and the Google sign-in
        // button would silently hand Firebase a token that always fails.
        let tokens = token_response("access", Some("refresh"), None);
        let err = signed_in_account_from_tokens(tokens, "marko@example.com".to_string()).unwrap_err();
        assert!(err.to_string().contains("sign-in token"));
    }

    /// Test-only helper: pulls one query parameter's value back out of a
    /// built authorization URL and percent-decodes it, so assertions can
    /// check the real value survived round-trip rather than guessing which
    /// characters NON_ALPHANUMERIC happened to leave unencoded.
    fn decoded_query_param(url: &str, key: &str) -> String {
        let query = url.splitn(2, '?').nth(1).expect("built URL must have a query string");
        for pair in query.split('&') {
            if let Some(raw_value) = pair.strip_prefix(&format!("{key}=")) {
                return percent_decode_str(raw_value).decode_utf8_lossy().into_owned();
            }
        }
        panic!("query param '{key}' not found in {url}");
    }

    #[test]
    fn redirect_request_line_with_a_successful_consent_yields_code_and_state() {
        let line = "GET /?code=4%2F0Adeu-abc123&state=xyz789 HTTP/1.1\r\n";
        let result = parse_redirect_request_line(line);
        assert_eq!(result.code, Some("4/0Adeu-abc123".to_string()), "percent-encoding must be decoded");
        assert_eq!(result.state, Some("xyz789".to_string()));
        assert_eq!(result.error, None);
    }

    #[test]
    fn redirect_request_line_with_declined_consent_yields_an_error_and_no_code() {
        let line = "GET /?error=access_denied&state=xyz789 HTTP/1.1\r\n";
        let result = parse_redirect_request_line(line);
        assert_eq!(result.error, Some("access_denied".to_string()));
        assert_eq!(result.code, None);
    }

    #[test]
    fn redirect_request_line_with_no_query_string_yields_nothing_rather_than_panicking() {
        let line = "GET / HTTP/1.1\r\n";
        let result = parse_redirect_request_line(line);
        assert_eq!(result, RedirectResult::default());
    }

    #[test]
    fn redirect_request_line_completely_malformed_yields_nothing_rather_than_panicking() {
        for line in ["", "not even close to an http request", "\r\n"] {
            let result = parse_redirect_request_line(line);
            assert_eq!(result, RedirectResult::default(), "input {line:?} must not panic");
        }
    }

    #[test]
    fn loopback_listener_binds_to_127_0_0_1_on_a_real_free_port() {
        // Fully offline (127.0.0.1 never leaves this machine) - this is the
        // one piece of the real flow that *can* be exercised end to end
        // without reaching Google: bind, read the port back out, connect to
        // it, and confirm accept_one_redirect parses what was sent.
        let listener = bind_loopback_listener().expect("binding a loopback listener must succeed in any test environment");
        let uri = redirect_uri_for(&listener).unwrap();
        assert!(uri.starts_with("http://127.0.0.1:"));

        let addr = listener.local_addr().unwrap();
        let no_cancel = AtomicBool::new(false);
        let handle = std::thread::spawn(move || accept_one_redirect(&listener, Duration::from_secs(5), &no_cancel));

        // Give the listener a moment to actually be polling accept() before
        // this test connects to it.
        std::thread::sleep(Duration::from_millis(50));
        {
            use std::io::Write as _;
            let mut client = std::net::TcpStream::connect(addr).expect("connecting to our own loopback listener must succeed");
            client.write_all(b"GET /?code=abc&state=def HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n").unwrap();
        }

        let result = handle.join().unwrap().expect("accept_one_redirect must succeed for a well-formed request");
        assert_eq!(result.code, Some("abc".to_string()));
        assert_eq!(result.state, Some("def".to_string()));
    }

    #[test]
    fn accept_one_redirect_times_out_instead_of_hanging_forever_if_nothing_ever_connects() {
        let listener = bind_loopback_listener().unwrap();
        let no_cancel = AtomicBool::new(false);
        let result = accept_one_redirect(&listener, Duration::from_millis(150), &no_cancel);
        assert!(result.is_err(), "no connection ever arriving must time out, not hang");
    }

    #[test]
    fn accept_one_redirect_is_interrupted_promptly_when_cancelled_instead_of_waiting_out_the_full_timeout() {
        // 2.0.12: marko's own report - closing the browser tab (or picking
        // "use another account" and never finishing) before Google's
        // redirect ever arrives used to leave this blocked for the full,
        // generous SIGN_IN_TIMEOUT (5 minutes) with no way back into the
        // app. Proves the actual fix: flipping `cancel` unblocks this within
        // about one polling interval (200ms), not anywhere near a real
        // multi-minute timeout - using one here (300s) that would fail this
        // test outright (via the 2s assertion below) if cancellation were
        // silently not being checked.
        let listener = bind_loopback_listener().unwrap();
        let cancel = std::sync::Arc::new(AtomicBool::new(false));
        let cancel_for_thread = cancel.clone();
        let handle =
            std::thread::spawn(move || accept_one_redirect(&listener, Duration::from_secs(300), &cancel_for_thread));

        // Give the listener a moment to actually start polling accept()
        // before this test flips the flag it should be watching.
        std::thread::sleep(Duration::from_millis(50));
        cancel.store(true, Ordering::Relaxed);

        let start = std::time::Instant::now();
        let result = handle.join().unwrap();
        assert!(result.is_err(), "a cancelled wait must return an error, never a successful RedirectResult");
        assert!(
            start.elapsed() < Duration::from_secs(2),
            "cancellation must be noticed within a polling interval or two, not anywhere near the 300s timeout - took {:?}",
            start.elapsed()
        );
    }
}

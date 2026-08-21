//! Minimal, hand-rolled Google Sheets v4 REST client, authenticated as a
//! service account (the OAuth2 "JWT bearer" flow Google documents at
//! <https://developers.google.com/identity/protocols/oauth2/service-account>).
//!
//! Deliberately NOT the generated `google-sheets4` client or the `yup-oauth2`
//! crate: both are built around the full Sheets API surface (we only ever
//! need three operations) and their own async runtime, while every command
//! in this app is a plain synchronous function over a mutex-guarded
//! `Connection` (see db.rs's `AppState` doc comment). Pulling in an async
//! stack just for this one feature would be a real architectural mismatch,
//! not just extra weight - so this module uses `reqwest`'s *blocking*
//! client, matching how every other command already runs.
//!
//! ## Why a service account, not per-user OAuth login
//!
//! A user-facing "Sign in with Google" flow was the first design here, but
//! it does not actually give what marko asked for ("appka niekomu, aby
//! stačilo kliknúť pripojiť tabuľku, žiadne ďalšie kroky"): while an OAuth
//! app is in Google's "Testing" publishing state, only individually
//! allow-listed test users can sign in at all, and every refresh token
//! expires after 7 days regardless of who's using it - moving out of
//! Testing requires Google's app-verification review (a privacy policy,
//! owned domain, and several days' turnaround). A service account has
//! neither limitation: connecting a sheet is just "share it with this fixed
//! e-mail address, then paste the sheet's URL/ID into Settings" - an action
//! every Google Sheets user already knows how to do, with no login screen,
//! no per-user allow-listing, and no Google review to wait on.
//!
//! The real tradeoff, and the reason this key is never committed to git
//! (see build.rs's doc comment): a service account is *one shared identity*
//! embedded in every copy of the app, not a separate identity per user like
//! OAuth would give. Anyone holding the raw key could ask the Sheets/Drive
//! API which files have been shared with it and read/write every one of
//! them - not just the sheet a single person connected. That's an
//! acceptable tradeoff for an app used by marko and people he personally
//! shares it with, over a public GitHub repo whose *source* must stay
//! secret-free either way; it would need revisiting (real per-user OAuth,
//! which is a contained, isolated swap behind this module's public
//! functions) if this app is ever distributed to a large, untrusted
//! audience.
//!
//! ## What can and can't be verified in this sandbox
//!
//! This sandbox's network proxy allow-lists a short list of package
//! registries and blocks everything else, including `googleapis.com` -
//! confirmed directly (`curl -v` to `www.googleapis.com` gets a 403 from the
//! proxy's own CONNECT tunnel, before ever reaching Google). So
//! `fetch_access_token`/`get_values`/`update_values`/`append_values` below
//! cannot be exercised end-to-end here, exactly the same category of
//! limitation this project has hit before with `cargo`/`npm` in earlier
//! sandboxes (see REDESIGN-1.8.2-REPORT.md section 9, for one). What *can*
//! be, and is, unit-tested without any network: `build_signed_jwt`'s
//! structure and RS256 signature are fully self-verified offline (see the
//! tests below) using a throwaway test keypair - never the real production
//! key, which never appears in a test.

use crate::error::{AppError, AppResult};
use chrono::{DateTime, Utc};
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde::{Deserialize, Serialize};

pub const SHEETS_SCOPE: &str = "https://www.googleapis.com/auth/spreadsheets";

/// The embedded service account key, written by build.rs from the
/// GOOGLE_SERVICE_ACCOUNT_JSON GitHub Actions secret at real build time.
/// Empty on any local `cargo build`/`cargo test` - see build.rs's doc
/// comment for why that's fine.
const EMBEDDED_SERVICE_ACCOUNT_JSON: &str =
    include_str!(concat!(env!("OUT_DIR"), "/google_service_account.json"));

/// The fields this app actually reads out of the JSON key file Google
/// generates (Cloud Console -> IAM & Admin -> Service Accounts -> Keys ->
/// Add key -> JSON). `serde` silently ignores every other field in that
/// file (project_id, private_key_id, client_id, ...) rather than erroring.
#[derive(Debug, Clone, Deserialize)]
pub struct ServiceAccountKey {
    pub client_email: String,
    pub private_key: String,
    #[serde(default = "default_token_uri")]
    pub token_uri: String,
}

fn default_token_uri() -> String {
    "https://oauth2.googleapis.com/token".to_string()
}

/// Returns the service account this build was compiled with, or `None` if
/// this is a local/dev build that never had the secret injected. Settings
/// uses this to show "Google Sheets sync isn't available in this build"
/// instead of a confusing runtime error when someone runs a plain `cargo
/// build`/`npm run tauri dev` copy.
pub fn embedded_service_account() -> Option<ServiceAccountKey> {
    if EMBEDDED_SERVICE_ACCOUNT_JSON.trim().is_empty() {
        return None;
    }
    serde_json::from_str(EMBEDDED_SERVICE_ACCOUNT_JSON).ok()
}

#[derive(Serialize)]
struct JwtClaims<'a> {
    iss: &'a str,
    scope: &'a str,
    aud: &'a str,
    iat: i64,
    exp: i64,
}

/// Builds and RS256-signs the JWT assertion Google's token endpoint expects
/// from a service account (the "self-signed JWT" step of the JWT-bearer
/// flow - RFC 7523). Pure and entirely offline: no network call happens
/// here, which is what makes it unit-testable in this sandbox even though
/// the token exchange that consumes it (`fetch_access_token`) is not.
pub fn build_signed_jwt(key: &ServiceAccountKey, scope: &str, now: DateTime<Utc>) -> AppResult<String> {
    let claims = JwtClaims {
        iss: &key.client_email,
        scope,
        aud: &key.token_uri,
        iat: now.timestamp(),
        // Google rejects any JWT bearer assertion valid for longer than 1h.
        exp: now.timestamp() + 3600,
    };
    let encoding_key = EncodingKey::from_rsa_pem(key.private_key.as_bytes())
        .map_err(|e| AppError::External(format!("service account private key is not valid PEM: {e}")))?;
    jsonwebtoken::encode(&Header::new(Algorithm::RS256), &claims, &encoding_key)
        .map_err(|e| AppError::External(format!("failed to sign the Google sign-in request: {e}")))
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

/// Exchanges a freshly-signed JWT for a short-lived (~1h) OAuth access
/// token. A real network call to `oauth2.googleapis.com` - see the module
/// doc comment for why this can't be exercised in this sandbox.
pub fn fetch_access_token(key: &ServiceAccountKey, scope: &str) -> AppResult<String> {
    let jwt = build_signed_jwt(key, scope, Utc::now())?;
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(&key.token_uri)
        .form(&[
            ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
            ("assertion", jwt.as_str()),
        ])
        .send()
        .map_err(|e| AppError::External(format!("could not reach Google to sign in: {e}")))?;
    parse_json_response::<TokenResponse>(resp).map(|t| t.access_token)
}

/// A Sheets API `ValueRange` - a rectangular block of cell values. Every
/// cell is a plain string on this side of the wire deliberately: the app
/// always parses/validates values itself afterward (same principle CSV
/// import already follows), never trusts Sheets' own type guessing about
/// what a cell "means".
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ValueRange {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<String>,
    #[serde(default)]
    pub values: Vec<Vec<String>>,
}

fn sheets_values_url(spreadsheet_id: &str, range: &str) -> String {
    let encoded_id = utf8_percent_encode(spreadsheet_id, NON_ALPHANUMERIC);
    let encoded_range = utf8_percent_encode(range, NON_ALPHANUMERIC);
    format!("https://sheets.googleapis.com/v4/spreadsheets/{encoded_id}/values/{encoded_range}")
}

/// Reads every cell currently in `range` (e.g. `"Pulls!A1:Z"`) as strings.
pub fn get_values(token: &str, spreadsheet_id: &str, range: &str) -> AppResult<ValueRange> {
    let client = reqwest::blocking::Client::new();
    let url = format!("{}?valueRenderOption=UNFORMATTED_VALUE", sheets_values_url(spreadsheet_id, range));
    let resp = client
        .get(&url)
        .bearer_auth(token)
        .send()
        .map_err(|e| AppError::External(format!("could not reach Google Sheets: {e}")))?;
    parse_json_response(resp)
}

/// Overwrites exactly `range` with `values` - never grows or shifts
/// anything outside it. `valueInputOption=RAW` on purpose: Sheets is never
/// allowed to reinterpret what the app sends (e.g. quietly reformatting a
/// date string), the same "never let anything guess" principle `money.rs`
/// and CSV import already hold everywhere else in this app.
///
/// Used by commands::pulls_sheet_sync (2.0.3) for exactly two things: adding
/// the app's own "TIQR ID" marker column header the first time a sheet is
/// synced, and writing that marker into a newly-created row afterward -
/// never anything else, and never any of a row's real data (sheet -> app
/// only in this pass, see that module's doc comment).
pub fn update_values(token: &str, spreadsheet_id: &str, range: &str, values: &[Vec<String>]) -> AppResult<()> {
    let client = reqwest::blocking::Client::new();
    let url = format!("{}?valueInputOption=RAW", sheets_values_url(spreadsheet_id, range));
    let body = ValueRange { range: None, values: values.to_vec() };
    let resp = client
        .put(&url)
        .bearer_auth(token)
        .json(&body)
        .send()
        .map_err(|e| AppError::External(format!("could not reach Google Sheets: {e}")))?;
    parse_json_response::<serde_json::Value>(resp).map(|_| ())
}

/// Appends `values` as new rows immediately after the existing table in
/// `range` (e.g. `"Pulls!A1"` - Sheets finds the real end of the table
/// itself via `insertDataOption=INSERT_ROWS`), rather than overwriting
/// anything. `valueInputOption=RAW` for the same reason as `update_values`.
///
/// Still not called anywhere (hence `#[allow(dead_code)]`): sync is
/// sheet -> app only as of 2.0.3 (commands::pulls_sheet_sync), so nothing
/// ever appends a *new* row to the sheet yet - that's the app -> sheet push
/// direction, deliberately a separate, later step. Kept ready for it.
#[allow(dead_code)]
pub fn append_values(token: &str, spreadsheet_id: &str, range: &str, values: &[Vec<String>]) -> AppResult<()> {
    let client = reqwest::blocking::Client::new();
    let url = format!(
        "{}:append?valueInputOption=RAW&insertDataOption=INSERT_ROWS",
        sheets_values_url(spreadsheet_id, range)
    );
    let body = ValueRange { range: None, values: values.to_vec() };
    let resp = client
        .post(&url)
        .bearer_auth(token)
        .json(&body)
        .send()
        .map_err(|e| AppError::External(format!("could not reach Google Sheets: {e}")))?;
    parse_json_response::<serde_json::Value>(resp).map(|_| ())
}

fn parse_json_response<T: serde::de::DeserializeOwned>(resp: reqwest::blocking::Response) -> AppResult<T> {
    let status = resp.status();
    let body = resp
        .text()
        .map_err(|e| AppError::External(format!("could not read Google's response: {e}")))?;
    if !status.is_success() {
        return Err(AppError::External(format!(
            "Google Sheets rejected the request ({status}): {body}"
        )));
    }
    serde_json::from_str(&body)
        .map_err(|e| AppError::External(format!("unexpected response from Google Sheets: {e} (body: {body})")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    /// Generates a fresh, disposable 2048-bit RSA test keypair via the
    /// `openssl` CLI (present in this sandbox and on any normal dev
    /// machine) - never the real production key, which must never appear in
    /// a test. Returns (private_key_pem, public_key_pem).
    fn generate_test_keypair() -> (String, String) {
        let priv_out = Command::new("openssl")
            .args(["genrsa", "2048"])
            .output()
            .expect("openssl must be available to run this test");
        assert!(priv_out.status.success(), "openssl genrsa failed: {}", String::from_utf8_lossy(&priv_out.stderr));
        let private_pem = String::from_utf8(priv_out.stdout).unwrap();

        let mut child = Command::new("openssl")
            .args(["rsa", "-pubout"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("failed to spawn openssl rsa -pubout");
        use std::io::Write;
        child.stdin.take().unwrap().write_all(private_pem.as_bytes()).unwrap();
        let pub_out = child.wait_with_output().expect("openssl rsa -pubout failed to run");
        assert!(pub_out.status.success(), "openssl rsa -pubout failed: {}", String::from_utf8_lossy(&pub_out.stderr));
        let public_pem = String::from_utf8(pub_out.stdout).unwrap();

        (private_pem, public_pem)
    }

    fn test_key(private_pem: &str) -> ServiceAccountKey {
        ServiceAccountKey {
            client_email: "tiqr-sync@example-project.iam.gserviceaccount.com".to_string(),
            private_key: private_pem.to_string(),
            token_uri: default_token_uri(),
        }
    }

    #[test]
    fn build_signed_jwt_produces_three_dot_separated_base64url_segments() {
        let (priv_pem, _pub_pem) = generate_test_keypair();
        let key = test_key(&priv_pem);
        let now = "2026-08-21T12:00:00Z".parse::<DateTime<Utc>>().unwrap();

        let jwt = build_signed_jwt(&key, SHEETS_SCOPE, now).expect("signing must succeed with a valid key");

        let parts: Vec<&str> = jwt.split('.').collect();
        assert_eq!(parts.len(), 3, "a JWT must have exactly header.claims.signature");
        for part in &parts {
            assert!(!part.is_empty());
        }
    }

    #[test]
    fn build_signed_jwt_header_and_claims_are_exactly_what_google_documents() {
        let (priv_pem, _pub_pem) = generate_test_keypair();
        let key = test_key(&priv_pem);
        let now = "2026-08-21T12:00:00Z".parse::<DateTime<Utc>>().unwrap();

        let jwt = build_signed_jwt(&key, SHEETS_SCOPE, now).unwrap();
        let parts: Vec<&str> = jwt.split('.').collect();

        let header_bytes = base64_url_decode(parts[0]);
        let header: serde_json::Value = serde_json::from_slice(&header_bytes).unwrap();
        assert_eq!(header["alg"], "RS256");
        assert_eq!(header["typ"], "JWT");

        let claims_bytes = base64_url_decode(parts[1]);
        let claims: serde_json::Value = serde_json::from_slice(&claims_bytes).unwrap();
        assert_eq!(claims["iss"], "tiqr-sync@example-project.iam.gserviceaccount.com");
        assert_eq!(claims["scope"], SHEETS_SCOPE);
        assert_eq!(claims["aud"], "https://oauth2.googleapis.com/token");
        assert_eq!(claims["iat"], now.timestamp());
        assert_eq!(
            claims["exp"],
            now.timestamp() + 3600,
            "Google rejects assertions valid for longer than exactly 1 hour"
        );
    }

    #[test]
    fn build_signed_jwt_signature_actually_verifies_against_the_matching_public_key() {
        // The strongest offline check available: sign with the private key,
        // then independently verify with the *public* half of the same
        // keypair using jsonwebtoken's own RS256 verifier. This proves the
        // signature this app produces is one any real RS256 verifier
        // (including Google's) would accept - not just that the code ran
        // without panicking.
        let (priv_pem, pub_pem) = generate_test_keypair();
        let key = test_key(&priv_pem);
        let now = Utc::now();

        let jwt = build_signed_jwt(&key, SHEETS_SCOPE, now).unwrap();

        let decoding_key = jsonwebtoken::DecodingKey::from_rsa_pem(pub_pem.as_bytes())
            .expect("the public key openssl derived must itself be valid PEM");
        let mut validation = jsonwebtoken::Validation::new(Algorithm::RS256);
        validation.set_audience(&["https://oauth2.googleapis.com/token"]);
        // This app's claim set has no `sub`, which jsonwebtoken doesn't
        // require by default; nothing extra to relax here.
        let decoded = jsonwebtoken::decode::<serde_json::Value>(&jwt, &decoding_key, &validation)
            .expect("a JWT signed with the private key must verify against its own matching public key");
        assert_eq!(decoded.claims["scope"], SHEETS_SCOPE);
    }

    #[test]
    fn build_signed_jwt_fails_cleanly_on_a_malformed_key_instead_of_panicking() {
        let key = test_key("this is not a PEM private key");
        let err = build_signed_jwt(&key, SHEETS_SCOPE, Utc::now());
        assert!(err.is_err(), "a malformed key must be reported as an error, never panic");
    }

    #[test]
    fn a_realistic_service_account_json_parses_into_exactly_the_fields_this_app_uses() {
        // Same shape Google's Cloud Console actually generates (see the
        // module doc comment) - fake values throughout, never the real key,
        // which must never appear in a test. Proves `ServiceAccountKey`'s
        // `Deserialize` impl tolerates every field Google includes that this
        // app doesn't use, and correctly ignores none of the ones it does.
        let json = r#"{
            "type": "service_account",
            "project_id": "example-project",
            "private_key_id": "deadbeef",
            "private_key": "-----BEGIN PRIVATE KEY-----\nFAKE\n-----END PRIVATE KEY-----\n",
            "client_email": "tiqr-sync@example-project.iam.gserviceaccount.com",
            "client_id": "123456789",
            "auth_uri": "https://accounts.google.com/o/oauth2/auth",
            "token_uri": "https://oauth2.googleapis.com/token",
            "auth_provider_x509_cert_url": "https://www.googleapis.com/oauth2/v1/certs",
            "client_x509_cert_url": "https://www.googleapis.com/robot/v1/metadata/x509/fake",
            "universe_domain": "googleapis.com"
        }"#;
        let key: ServiceAccountKey = serde_json::from_str(json).expect("a real Google-shaped key file must parse");
        assert_eq!(key.client_email, "tiqr-sync@example-project.iam.gserviceaccount.com");
        assert_eq!(key.private_key, "-----BEGIN PRIVATE KEY-----\nFAKE\n-----END PRIVATE KEY-----\n");
        assert_eq!(key.token_uri, "https://oauth2.googleapis.com/token");
    }

    #[test]
    fn embedded_service_account_is_none_on_a_plain_local_build() {
        // This test suite never has GOOGLE_SERVICE_ACCOUNT_JSON set (build.rs
        // falls back to an empty embed - see its doc comment), so this must
        // hold in exactly the environment this test actually runs in.
        assert!(
            embedded_service_account().is_none(),
            "a local cargo test/build must never have a real key embedded"
        );
    }

    #[test]
    fn sheets_values_url_percent_encodes_spaces_and_diacritics_in_the_range() {
        let url = sheets_values_url("abc123", "Ťahy 2026!A1:Z");
        assert!(!url.contains(' '), "a raw space would produce an invalid URL");
        assert!(url.starts_with("https://sheets.googleapis.com/v4/spreadsheets/abc123/values/"));
    }

    fn base64_url_decode(s: &str) -> Vec<u8> {
        use base64::Engine;
        base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(s).expect("valid base64url")
    }
}

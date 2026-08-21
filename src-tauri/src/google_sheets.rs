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

/// Combined scope for the one flow that needs both APIs: auto-creating a
/// brand-new sheet (Sheets API) and sharing it with whoever asked for it
/// (Drive API) - see commands::pulls_sheet_sync::create_pulls_sheet. Every
/// other flow here only ever reads/writes an *existing* sheet's values, so
/// it keeps using the narrower `SHEETS_SCOPE` above, unchanged. `drive.file`
/// (never the much broader `drive` scope) on purpose: this service account
/// only ever needs to touch files it created itself, never anything else in
/// anyone's Drive. Requires the Google Drive API to be enabled on the same
/// GCP project the service account belongs to (Cloud Console -> APIs &
/// Services -> Library -> Google Drive API -> Enable) - a one-time step,
/// same category as enabling the Sheets API itself.
pub const SHEETS_AND_DRIVE_SCOPE: &str =
    "https://www.googleapis.com/auth/spreadsheets https://www.googleapis.com/auth/drive.file";

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
///
/// Getting there takes one extra step, though (since 2.0.7): with
/// `valueRenderOption=UNFORMATTED_VALUE` below, Google sends a cell's *own*
/// JSON type, not always a string - a cell holding the number `50` comes
/// back as the JSON number `50`, and a cell someone typed a real date into
/// (as opposed to typing the date as text) comes back as its underlying
/// serial-date number. Deserializing `values` straight into
/// `Vec<Vec<String>>` used to hard-fail the instant any cell held a
/// non-string JSON value - see REDESIGN-2.0.7-REPORT.md for the exact error
/// this produced the first time a real user typed a normal number into a
/// normal cell. `deserialize_cell_grid` below is what still lets this field
/// be `Vec<Vec<String>>` (nothing downstream has to change) while actually
/// tolerating what Sheets really sends.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ValueRange {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<String>,
    #[serde(default, deserialize_with = "deserialize_cell_grid")]
    pub values: Vec<Vec<String>>,
}

/// Turns one already-JSON-parsed Sheets cell into the exact text this app
/// treats every cell as (see `ValueRange`'s doc comment above). Numbers go
/// through `serde_json::Number`'s own `Display`, which prints the *shortest
/// decimal that round-trips back to the same value* - so `50` stringifies
/// to `"50"` and `12.5` to `"12.5"`, never a floating-point artifact like
/// `"12.499999999999998"`. That is what makes it safe to feed straight into
/// `money::parse_decimal_to_cents` afterward without that function - or
/// anything else downstream - ever having to touch a float itself.
fn cell_json_to_string(value: serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s,
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => if b { "TRUE" } else { "FALSE" }.to_string(),
        serde_json::Value::Null => String::new(),
        // A Sheets cell is always one of the four above in practice - kept
        // total rather than assuming that, so a surprising response can
        // never panic this far from the actual HTTP boundary.
        other => other.to_string(),
    }
}

fn deserialize_cell_grid<'de, D>(deserializer: D) -> Result<Vec<Vec<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let grid: Vec<Vec<serde_json::Value>> = Deserialize::deserialize(deserializer)?;
    Ok(grid.into_iter().map(|row| row.into_iter().map(cell_json_to_string).collect()).collect())
}

/// Formats a sheet/tab name and a plain cell range (e.g. `"A1:Z"`, `"B12"`)
/// into one valid A1-notation range string, e.g. `'My Tab'!A1:Z`. Always
/// wraps the tab name in single quotes and doubles any embedded single quote
/// (the standard A1-notation escape - identical to Excel's own range syntax)
/// rather than only quoting when it looks "necessary": a bare single-word
/// name like `Pulls` is exactly as valid quoted as unquoted, so always
/// quoting means this never has to guess which names need it.
///
/// This is not optional for a real sheet: an *unquoted* tab name is only
/// valid A1 syntax when it is a single bare word - the instant it contains a
/// space or most punctuation, Google rejects the whole request with
/// "Unable to parse range: ...". That is exactly what happened with a real
/// tab named "Tiqr manager event + order" (see
/// REDESIGN-2.0.9-REPORT.md) - `sheets_values_url` below still correctly
/// percent-encodes whatever range string it's given for the URL itself, but
/// percent-encoding an already-invalid, unquoted range does not make it
/// valid A1 notation once Google decodes and parses it server-side.
pub(crate) fn a1_range(sheet_tab: &str, cell_range: &str) -> String {
    format!("'{}'!{cell_range}", sheet_tab.replace('\'', "''"))
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

/// The (partial) shape of the Sheets API's `spreadsheets.create` response
/// this app actually reads. Sheets returns a great deal more (every sheet's
/// full grid properties, default formatting, ...) - `serde` silently ignores
/// all of it, same principle as `ServiceAccountKey` above.
#[derive(Debug, Deserialize)]
struct CreateSpreadsheetResponse {
    #[serde(rename = "spreadsheetId")]
    spreadsheet_id: String,
    #[serde(rename = "spreadsheetUrl")]
    spreadsheet_url: String,
}

/// What `create_spreadsheet` hands back to its caller - just the two things
/// commands::pulls_sheet_sync needs: the ID to connect to (same shape as if
/// the user had pasted an existing sheet's ID) and the URL to show them.
#[derive(Debug, Clone)]
pub struct CreatedSpreadsheet {
    pub spreadsheet_id: String,
    pub spreadsheet_url: String,
}

/// Creates a brand-new spreadsheet titled `title`, with a single sheet
/// (tab) named `sheet_tab`, and returns its ID/URL. Requires `token` to have
/// been fetched with `SHEETS_AND_DRIVE_SCOPE` (create itself only needs the
/// Sheets scope, but the caller's very next step is always `share_file`,
/// which does need Drive - see that constant's doc comment).
///
/// The service account becomes the sole owner of the new file, exactly like
/// any file it creates via the Sheets API always has been - `share_file`
/// below is what makes it visible/editable to an actual person afterward.
pub fn create_spreadsheet(token: &str, title: &str, sheet_tab: &str) -> AppResult<CreatedSpreadsheet> {
    let client = reqwest::blocking::Client::new();
    let body = serde_json::json!({
        "properties": { "title": title },
        "sheets": [{ "properties": { "title": sheet_tab } }],
    });
    let resp = client
        .post("https://sheets.googleapis.com/v4/spreadsheets")
        .bearer_auth(token)
        .json(&body)
        .send()
        .map_err(|e| AppError::External(format!("could not reach Google Sheets: {e}")))?;
    let parsed: CreateSpreadsheetResponse = parse_json_response(resp)?;
    Ok(CreatedSpreadsheet {
        spreadsheet_id: parsed.spreadsheet_id,
        spreadsheet_url: parsed.spreadsheet_url,
    })
}

/// Shares Drive file `file_id` with `email` as an editor ("writer"), and
/// asks Drive to send that person its normal "X shared a file with you"
/// notification e-mail so they actually notice the new sheet exists.
/// Requires `token` to have been fetched with `SHEETS_AND_DRIVE_SCOPE`
/// (`drive.file`) - see that constant's doc comment for why this app never
/// requests the broader `drive` scope.
pub fn share_file(token: &str, file_id: &str, email: &str) -> AppResult<()> {
    let client = reqwest::blocking::Client::new();
    let encoded_id = utf8_percent_encode(file_id, NON_ALPHANUMERIC);
    let url = format!(
        "https://www.googleapis.com/drive/v3/files/{encoded_id}/permissions?sendNotificationEmail=true"
    );
    let body = serde_json::json!({
        "role": "writer",
        "type": "user",
        "emailAddress": email,
    });
    let resp = client
        .post(&url)
        .bearer_auth(token)
        .json(&body)
        .send()
        .map_err(|e| AppError::External(format!("could not reach Google Drive: {e}")))?;
    parse_json_response::<serde_json::Value>(resp).map(|_| ())
}

// `pub(crate)` since 2.0.5: google_oauth.rs talks to a different set of
// Google endpoints (accounts.google.com/oauth2.googleapis.com's user-facing
// token endpoint, not this module's service-account one) but the shape of
// "is this a 2xx JSON response, and if not, surface Google's own error body"
// is identical - reused rather than re-written a second time.
pub(crate) fn parse_json_response<T: serde::de::DeserializeOwned>(resp: reqwest::blocking::Response) -> AppResult<T> {
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

    #[test]
    fn a1_range_quotes_a_tab_name_containing_spaces_and_punctuation() {
        // The exact real tab name that used to make Google reject every
        // request with "Unable to parse range" - see REDESIGN-2.0.9-REPORT.md.
        assert_eq!(a1_range("Tiqr manager event + order", "A1:Z"), "'Tiqr manager event + order'!A1:Z");
    }

    #[test]
    fn a1_range_quotes_a_plain_single_word_tab_name_too() {
        // Quoting is always valid, even when not strictly required - this
        // function never has to guess which names need it.
        assert_eq!(a1_range("Pulls", "A1:A1"), "'Pulls'!A1:A1");
    }

    #[test]
    fn a1_range_doubles_an_embedded_single_quote() {
        assert_eq!(a1_range("Marko's Tickets", "A1:Z"), "'Marko''s Tickets'!A1:Z");
    }

    #[test]
    fn cell_json_to_string_stringifies_every_json_scalar_type_exactly() {
        assert_eq!(cell_json_to_string(serde_json::json!("ticketmaster")), "ticketmaster");
        assert_eq!(cell_json_to_string(serde_json::json!(50)), "50");
        assert_eq!(cell_json_to_string(serde_json::json!(12.5)), "12.5", "must be the shortest round-tripping decimal, never a float artifact");
        assert_eq!(cell_json_to_string(serde_json::json!(true)), "TRUE");
        assert_eq!(cell_json_to_string(serde_json::json!(false)), "FALSE");
        assert_eq!(cell_json_to_string(serde_json::Value::Null), "");
    }

    #[test]
    fn value_range_deserializes_a_real_row_with_mixed_cell_types_without_erroring() {
        // The exact shape of response that used to hard-fail sync entirely
        // the moment a normal user typed a normal number - or a real date -
        // into a sheet cell. See REDESIGN-2.0.7-REPORT.md.
        let json = r#"{
            "range": "Pulls!A1:Z1000",
            "majorDimension": "ROWS",
            "values": [
                ["pull", "Event name", "event date", "Ks", "Platform", "More info", "Section", "Row", "Seats", "Transfer", "Price"],
                ["sojky", "England vs Spain", 46291, 8, "ticketmaster", "", 410, 25, "11-18", true, 50]
            ]
        }"#;
        let parsed: ValueRange = serde_json::from_str(json).expect("must no longer fail on numeric/boolean cells");
        assert_eq!(parsed.values[1][0], "sojky");
        assert_eq!(parsed.values[1][2], "46291", "the raw serial-date number, unconverted at this layer - see pulls_sheet_sync::parse_sheet_date");
        assert_eq!(parsed.values[1][3], "8");
        assert_eq!(parsed.values[1][6], "410");
        assert_eq!(parsed.values[1][9], "TRUE");
        assert_eq!(parsed.values[1][10], "50");
    }

    fn base64_url_decode(s: &str) -> Vec<u8> {
        use base64::Engine;
        base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(s).expect("valid base64url")
    }
}

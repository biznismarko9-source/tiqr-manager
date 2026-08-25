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

/// Same as `update_values`, but with `valueInputOption=USER_ENTERED` - the
/// one deliberate exception to this module's "RAW only, Sheets never
/// reinterprets what the app sends" rule (see `update_values`'s own doc
/// comment). `USER_ENTERED` is what makes a string like `"=O2*I2"` become a
/// real, live formula instead of literal text - exactly as if marko had
/// typed it into that cell himself.
///
/// 2.0.19: used for exactly one thing - commands::orders_sheet_sync's
/// Revenue/Profit columns. Marko confirmed via AskUserQuestion these must be
/// live Sheets formulas (recalculating instantly even when he hand-edits
/// another cell in that row, without waiting for the next sync/push) rather
/// than a number the app computes and pushes - see that module's own doc
/// comment for the full reasoning. Never used for anything else: every other
/// cell this app writes must stay exactly the literal text handed to it.
pub fn update_values_as_formulas(token: &str, spreadsheet_id: &str, range: &str, values: &[Vec<String>]) -> AppResult<()> {
    let client = reqwest::blocking::Client::new();
    let url = format!("{}?valueInputOption=USER_ENTERED", sheets_values_url(spreadsheet_id, range));
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
/// 2.0.18: the app -> sheet push direction (commands::pulls_sheet_sync::
/// push_pulls_impl, commands::orders_sheet_sync's own push) - a brand-new
/// local-only Pull/Order becomes a new row via this, appended after
/// whatever the sheet already has.
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

/// The (partial) shape of the Sheets API's `spreadsheets.get` response this
/// app reads for `get_spreadsheet_sheet_titles`/`get_sheet_structure_metadata`
/// below - `fields=` on that call already asks Google to omit everything else
/// (every sheet's full grid contents/formatting), so there is little left
/// for `serde` to ignore here, unlike the other response shapes in this
/// module.
#[derive(Debug, Deserialize)]
struct SpreadsheetMetadata {
    sheets: Vec<SheetMetadataEntry>,
}

#[derive(Debug, Deserialize)]
struct SheetMetadataEntry {
    properties: SheetMetadataProperties,
    // 2.0.22: added alongside `properties`, same "one shared struct,
    // tolerate whichever subset of fields THIS particular fields= query
    // actually asked for" principle the 2.0.20 fix established for
    // `sheet_id` right below - only `get_sheet_structure_metadata`'s own
    // query actually requests `conditionalFormats`, so `#[serde(default)]`
    // is required here too: `get_spreadsheet_sheet_titles`'s response
    // genuinely never carries this field at all, not even as an empty array.
    #[serde(default, rename = "conditionalFormats")]
    conditional_formats: Vec<ConditionalFormatEntry>,
}

/// The (partial) shape of one existing `ConditionalFormatRule` as
/// `get_sheet_structure_metadata` below reads it back - just enough to work
/// out which single column (if any) it targets, nothing about its actual
/// condition/color (this app never needs to read those back, only decide
/// whether a rule is a deletion candidate - see
/// `conditional_format_indices_to_replace`).
#[derive(Debug, Deserialize)]
struct ConditionalFormatEntry {
    #[serde(default)]
    ranges: Vec<ConditionalFormatGridRange>,
}

#[derive(Debug, Deserialize)]
struct ConditionalFormatGridRange {
    #[serde(rename = "startColumnIndex")]
    start_column_index: Option<i64>,
    #[serde(rename = "endColumnIndex")]
    end_column_index: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct SheetMetadataProperties {
    title: String,
    // 2.0.19: added alongside `title` (was title-only) - see
    // `get_sheet_structure_metadata`'s own doc comment for why this is
    // needed, and why it is a completely different thing from the tab's
    // name.
    //
    // `Option`, NOT a plain `i64` - both `get_spreadsheet_sheet_titles` and
    // `get_sheet_structure_metadata` deserialize into this SAME struct, but
    // only the second one's `fields=` query actually asks Google for
    // `sheetId`; the first's response genuinely never has it. 2.0.20 bug
    // fix: this was a required `i64` for one release, which broke
    // `get_spreadsheet_sheet_titles` - i.e. every "paste a URL" tab
    // auto-detect, on both the Pulls and Orders & Sales cards - with a hard
    // "missing field `sheetId`" on every real spreadsheet, since that call's
    // own response never carries it. `get_sheet_structure_metadata` below is
    // the one place that actually needs the number, and is responsible for
    // treating a bare `None` as an error - `get_spreadsheet_sheet_titles`
    // never looks at this field at all. (2.0.19 originally introduced this
    // field for a narrower `get_sheet_numeric_id` function that only
    // returned the ID; 2.0.22 folded that into `get_sheet_structure_metadata`
    // below, which every caller that needs the ID also needs the
    // conditional-format info from, and removed the narrower function.)
    #[serde(rename = "sheetId")]
    sheet_id: Option<i64>,
    // 2.0.41: added alongside `sheet_id` above, same reasoning - only
    // `get_sheet_structure_metadata`'s own `fields=` query actually asks for
    // `gridProperties`, so this must tolerate a response that never carries
    // it at all (get_spreadsheet_sheet_titles's own query never will).
    #[serde(default, rename = "gridProperties")]
    grid_properties: Option<GridPropertiesEntry>,
}

/// A sheet's own actual grid size, as opposed to how much of it has real
/// content - both `Option` because Google omits `gridProperties` entirely
/// from a response whose `fields=` query didn't ask for it (never sends it
/// as `null`), same convention as every other field in this struct.
#[derive(Debug, Deserialize)]
struct GridPropertiesEntry {
    #[serde(rename = "rowCount")]
    row_count: Option<i64>,
    #[serde(rename = "columnCount")]
    column_count: Option<i64>,
}

/// Returns the exact title of every tab in spreadsheet `spreadsheet_id`, in
/// the order Google itself returns them (left to right, as shown in the
/// spreadsheet's own tab bar). 2.0.13: exists purely so a failed connection
/// test can tell marko exactly what to type, instead of describing the
/// problem in the abstract - see
/// commands::sheets_sync::test_sheets_connection_impl for the one caller.
///
/// `fields=sheets.properties.title` asks Google to send back only this one
/// field per sheet rather than each tab's full grid contents/formatting/
/// conditional-formatting rules/etc. - the same "ask for less, get a
/// smaller and cheaper response" principle `valueRenderOption` already
/// applies to `get_values`, just via the Sheets API's separate `fields`
/// partial-response mechanism this time.
pub fn get_spreadsheet_sheet_titles(token: &str, spreadsheet_id: &str) -> AppResult<Vec<String>> {
    let client = reqwest::blocking::Client::new();
    let encoded_id = utf8_percent_encode(spreadsheet_id, NON_ALPHANUMERIC);
    let url = format!("https://sheets.googleapis.com/v4/spreadsheets/{encoded_id}?fields=sheets.properties.title");
    let resp = client
        .get(&url)
        .bearer_auth(token)
        .send()
        .map_err(|e| AppError::External(format!("could not reach Google Sheets: {e}")))?;
    let parsed: SpreadsheetMetadata = parse_json_response(resp)?;
    Ok(parsed.sheets.into_iter().map(|s| s.properties.title).collect())
}

/// Sends a `spreadsheets.batchUpdate` request - the Sheets API's mechanism
/// for structural/formatting changes (data validation rules, cell
/// formatting, etc.) that the plain `values.*` endpoints above can't express
/// at all. `requests` is the API's own array of one-request-per-change
/// objects (see `set_data_validation_request` below for the only shape this
/// app currently builds) sent through untouched - Google applies every
/// request in the array atomically, all or nothing.
///
/// 2.0.19: added for commands::orders_sheet_sync's dropdown-setup step.
pub fn batch_update(token: &str, spreadsheet_id: &str, requests: Vec<serde_json::Value>) -> AppResult<()> {
    let client = reqwest::blocking::Client::new();
    let encoded_id = utf8_percent_encode(spreadsheet_id, NON_ALPHANUMERIC);
    let url = format!("https://sheets.googleapis.com/v4/spreadsheets/{encoded_id}:batchUpdate");
    let body = serde_json::json!({ "requests": requests });
    let resp = client
        .post(&url)
        .bearer_auth(token)
        .json(&body)
        .send()
        .map_err(|e| AppError::External(format!("could not reach Google Sheets: {e}")))?;
    parse_json_response::<serde_json::Value>(resp).map(|_| ())
}

/// Builds one `setDataValidation` request restricting `sheet_id`'s column
/// `col_index` (0-based) to a dropdown of exactly `values`, across rows
/// `start_row..end_row` (both 0-based, `end_row` exclusive - i.e. the same
/// convention Google's own API uses, NOT the 1-based row numbers everywhere
/// else in this app).
///
/// `strict: false` ("Show a warning" in the Sheets UI, not "Reject input")
/// is deliberate: marko explicitly wants a value he or the sheet itself adds
/// that isn't in `values` YET to still be accepted, never blocked - see
/// commands::orders_sheet_sync's module doc comment for why growing the
/// list, not enforcing it, is the whole point of this feature.
pub fn set_data_validation_request(sheet_id: i64, start_row: i64, end_row: i64, col_index: i64, values: &[String]) -> serde_json::Value {
    serde_json::json!({
        "setDataValidation": {
            "range": {
                "sheetId": sheet_id,
                "startRowIndex": start_row,
                "endRowIndex": end_row,
                "startColumnIndex": col_index,
                "endColumnIndex": col_index + 1
            },
            "rule": {
                "condition": {
                    "type": "ONE_OF_LIST",
                    "values": values.iter().map(|v| serde_json::json!({ "userEnteredValue": v })).collect::<Vec<_>>()
                },
                "showCustomUi": true,
                "strict": false
            }
        }
    })
}

/// Everything `ensure_orders_sheet_structure`/`ensure_pulls_sheet_structure`
/// need from ONE network round-trip, whether a particular refresh needs
/// dropdowns, color-coding, or both: the tab's internal numeric `sheetId` -
/// NOT the same thing as the tab's name/title, and required by every
/// `batchUpdate` request (`set_data_validation_request`,
/// `add_conditional_format_color_request`/`delete_conditional_format_rule_
/// request` below) since that whole endpoint addresses sheets by this ID,
/// never by name, unlike every other endpoint in this module - plus, for
/// every conditional-format rule ALREADY on the sheet, the single column it
/// targets, so a caller can work out exactly which of ITS OWN
/// previously-added color rules need deleting before adding fresh ones (see
/// `conditional_format_indices_to_replace` below), without ever touching a
/// rule on a column it doesn't manage.
///
/// 2.0.19: introduced (as a narrower `get_sheet_numeric_id`, sheetId only)
/// for commands::orders_sheet_sync's dropdown-setup step. 2.0.22: folded in
/// the conditional-format lookup too, for the Status/Delivery status/Payout
/// status/Transfer color-coding feature (marko's own request) - every caller
/// that needs the ID also needs this now, so one shared fetch replaces what
/// would otherwise be two. Errs clearly (rather than silently picking the
/// first tab) if `sheet_tab` isn't found - the same "never guess" rule as
/// everywhere else in this module. A rule whose `ranges` isn't exactly one
/// single-column range - anything this app itself could not have created,
/// e.g. something marko added by hand covering multiple columns, or an
/// unbounded range - maps to `None`, so it is never treated as "ours" and
/// never becomes a deletion candidate.
pub struct SheetStructureMetadata {
    pub sheet_id: i64,
    pub conditional_format_columns: Vec<Option<i64>>,
    // 2.0.41: the sheet's own CURRENT grid size - see
    // `grow_grid_request_if_needed` below for why a caller needs this at
    // all. `Option` for the same "this response might not carry it" reason
    // `sheet_id` above is - in practice always `Some` for THIS function's own
    // `fields=` query below (which always asks for both), but never assumed.
    pub row_count: Option<i64>,
    pub column_count: Option<i64>,
}

pub fn get_sheet_structure_metadata(token: &str, spreadsheet_id: &str, sheet_tab: &str) -> AppResult<SheetStructureMetadata> {
    let client = reqwest::blocking::Client::new();
    let encoded_id = utf8_percent_encode(spreadsheet_id, NON_ALPHANUMERIC);
    let url = format!(
        "https://sheets.googleapis.com/v4/spreadsheets/{encoded_id}?fields=sheets.properties.title,sheets.properties.sheetId,sheets.properties.gridProperties.rowCount,sheets.properties.gridProperties.columnCount,sheets.conditionalFormats.ranges.startColumnIndex,sheets.conditionalFormats.ranges.endColumnIndex"
    );
    let resp = client
        .get(&url)
        .bearer_auth(token)
        .send()
        .map_err(|e| AppError::External(format!("could not reach Google Sheets: {e}")))?;
    let parsed: SpreadsheetMetadata = parse_json_response(resp)?;
    let entry = parsed
        .sheets
        .into_iter()
        .find(|s| s.properties.title == sheet_tab)
        .ok_or_else(|| AppError::Validation(format!("Tab \"{sheet_tab}\" was not found in this spreadsheet.")))?;
    let sheet_id = entry.properties.sheet_id.ok_or_else(|| {
        AppError::External(format!("Google Sheets did not return an internal ID for tab \"{sheet_tab}\"."))
    })?;
    let row_count = entry.properties.grid_properties.as_ref().and_then(|g| g.row_count);
    let column_count = entry.properties.grid_properties.as_ref().and_then(|g| g.column_count);
    let conditional_format_columns = entry.conditional_formats.iter().map(|f| single_column_index(&f.ranges)).collect();
    Ok(SheetStructureMetadata { sheet_id, conditional_format_columns, row_count, column_count })
}

/// Pure core of `get_sheet_structure_metadata`'s conditional-format reading:
/// `Some(column)` only when `ranges` is exactly one range that covers a
/// single whole column (both `startColumnIndex`/`endColumnIndex` present,
/// exactly one column apart) - the only shape
/// `add_conditional_format_color_request` below ever produces. Anything else
/// - more than one range, no ranges at all, or a range missing either bound
/// (Google reports an unbounded side as simply absent, not zero) - is `None`,
/// so a rule shaped like that (marko's own manual conditional formatting
/// almost certainly is) is never mistaken for one this app created.
fn single_column_index(ranges: &[ConditionalFormatGridRange]) -> Option<i64> {
    match ranges {
        [r] => match (r.start_column_index, r.end_column_index) {
            (Some(start), Some(end)) if end == start + 1 => Some(start),
            _ => None,
        },
        _ => None,
    }
}

/// Given the sheet's existing conditional-format rules (`existing_columns`,
/// one entry per rule in Sheets' own index order - see
/// `get_sheet_structure_metadata` above) and the column indices a refresh is
/// about to (re-)color (`managed_columns`), returns exactly which existing
/// indices must be deleted first: every rule whose column is one about to be
/// managed, and ONLY those - a rule on any other column (something marko
/// added himself, or simply a column this particular refresh doesn't color)
/// is never touched. Returned in descending order: `deleteConditionalFormatRule`
/// requests inside one `batchUpdate` apply in array order and each delete
/// shifts every later index down by one, so deleting highest-first keeps
/// every remaining index in the list valid for the next delete in the same
/// call.
pub fn conditional_format_indices_to_replace(existing_columns: &[Option<i64>], managed_columns: &[i64]) -> Vec<i64> {
    let mut indices: Vec<i64> = existing_columns
        .iter()
        .enumerate()
        .filter_map(|(i, col)| col.filter(|c| managed_columns.contains(c)).map(|_| i as i64))
        .collect();
    indices.sort_unstable_by(|a, b| b.cmp(a));
    indices
}

/// Builds one `addConditionalFormatRule` request: cells in `sheet_id`'s
/// column `col_index` (0-based) across rows `start_row..end_row` (both
/// 0-based, `end_row` exclusive - same convention as
/// `set_data_validation_request` above) that exactly equal `text_value` get
/// `color` (red, green, blue - each 0.0-1.0, the Sheets API's own `Color`
/// shape) as their background.
///
/// `index: 0` always - correct regardless of how many rules already exist on
/// the sheet (Sheets simply inserts at the front, shifting everything else
/// down by one); safe here specifically because every rule this app ever
/// creates has a condition that can never match the same cell as another
/// rule it creates in the same batch (a cell holds exactly one exact text
/// value), so which one ends up "first" in the list is never visible in the
/// result.
pub fn add_conditional_format_color_request(
    sheet_id: i64,
    start_row: i64,
    end_row: i64,
    col_index: i64,
    text_value: &str,
    color: (f64, f64, f64),
) -> serde_json::Value {
    let (red, green, blue) = color;
    serde_json::json!({
        "addConditionalFormatRule": {
            "rule": {
                "ranges": [{
                    "sheetId": sheet_id,
                    "startRowIndex": start_row,
                    "endRowIndex": end_row,
                    "startColumnIndex": col_index,
                    "endColumnIndex": col_index + 1
                }],
                "booleanRule": {
                    "condition": {
                        "type": "TEXT_EQ",
                        "values": [{ "userEnteredValue": text_value }]
                    },
                    "format": {
                        "backgroundColor": { "red": red, "green": green, "blue": blue }
                    }
                }
            },
            "index": 0
        }
    })
}

/// Builds one `deleteConditionalFormatRule` request removing the rule
/// currently at `index` (0-based, Sheets' own index order) on `sheet_id`.
/// See `conditional_format_indices_to_replace` above for how a caller works
/// out which indices are safe to pass here.
pub fn delete_conditional_format_rule_request(sheet_id: i64, index: i64) -> serde_json::Value {
    serde_json::json!({
        "deleteConditionalFormatRule": {
            "sheetId": sheet_id,
            "index": index
        }
    })
}

/// Builds one `repeatCell` request bolding the text of a vertical range,
/// with an optional flat background color - 2.0.40, for the small
/// summary "widgets" this app writes to the side of a sheet
/// (commands::pulls_sheet_sync's Total price, commands::orders_sheet_sync's
/// Summary block). Unlike `add_conditional_format_color_request` above,
/// this is a plain, unconditional cell format (not a rule keyed on the
/// cell's text) - the right tool here since every cell in these ranges
/// should always look this way, not only when it happens to equal some
/// specific value.
///
/// Deliberately does NOT also set a currency number format - that's
/// `currency_number_format_request` below, a separate `repeatCell` request,
/// since it targets a different range within these widgets (the numbers
/// themselves, not their labels - see each call site). 2.0.40 originally
/// left currency formatting out entirely here on the theory that a correct
/// pattern would need knowing the connected spreadsheet's own locale, which
/// nothing here tracks - that turned out to be overly cautious; see
/// `currency_number_format_request`'s own doc comment for the correction.
pub fn bold_header_request(sheet_id: i64, start_row: i64, end_row: i64, col_index: i64, background: Option<(f64, f64, f64)>) -> serde_json::Value {
    let mut format = serde_json::json!({ "textFormat": { "bold": true } });
    if let Some((red, green, blue)) = background {
        format["backgroundColor"] = serde_json::json!({ "red": red, "green": green, "blue": blue });
    }
    serde_json::json!({
        "repeatCell": {
            "range": {
                "sheetId": sheet_id,
                "startRowIndex": start_row,
                "endRowIndex": end_row,
                "startColumnIndex": col_index,
                "endColumnIndex": col_index + 1
            },
            "cell": { "userEnteredFormat": format },
            "fields": "userEnteredFormat(textFormat,backgroundColor)"
        }
    })
}

/// Builds one `repeatCell` request applying plain Euro currency display
/// formatting to a vertical range - 2.0.42, for the same small summary
/// "widgets" `bold_header_request` above styles (commands::pulls_sheet_sync's
/// Total price, commands::orders_sheet_sync's Summary block) - see each call
/// site for exactly which range within a widget this targets (always the
/// number cells, never the label cells `bold_header_request` bolds).
///
/// `bold_header_request`'s own doc comment used to say a currency format
/// here wasn't safe without knowing the connected spreadsheet's own locale -
/// that turned out to be overly cautious. Per Google's own Sheets API
/// documentation (see "Date and number formats"), a `numberFormat.pattern`
/// string is a locale-INVARIANT pattern language: `,` always means "group
/// here" and `.` always means "decimal point here" *in the pattern itself*,
/// and Sheets renders that same pattern using whatever punctuation the
/// spreadsheet's own locale actually uses - the exact pattern below reads as
/// "1 234,50 €" on a Slovak-locale sheet and "1,234.50 €" on a US-locale one,
/// with zero locale-detection needed on this app's part. The one exception
/// is LITERAL characters embedded in a pattern (anything that isn't itself
/// pattern grammar) - those are never translated, which is exactly why the
/// "€" below always renders as "€": marko asked for EUR specifically ("daj
/// do eur"), not "whatever currency my connection happens to be", so this is
/// a fixed literal, not derived from the connection's own currency code.
pub fn currency_number_format_request(sheet_id: i64, start_row: i64, end_row: i64, col_index: i64) -> serde_json::Value {
    serde_json::json!({
        "repeatCell": {
            "range": {
                "sheetId": sheet_id,
                "startRowIndex": start_row,
                "endRowIndex": end_row,
                "startColumnIndex": col_index,
                "endColumnIndex": col_index + 1
            },
            "cell": {
                "userEnteredFormat": {
                    "numberFormat": { "type": "CURRENCY", "pattern": "#,##0.00 €" }
                }
            },
            "fields": "userEnteredFormat.numberFormat"
        }
    })
}

/// Builds an `updateSheetProperties` request that grows `sheet_id`'s own
/// grid to at least `needed_rows` rows / `needed_columns` columns - `None`
/// when the known CURRENT size (`current_rows`/`current_columns`, straight
/// from `SheetStructureMetadata`) already covers what's needed, so this
/// never fires a no-op request. Also `None` when either current size isn't
/// known at all - a missing `gridProperties` in the metadata response means
/// "don't touch it", never "assume it's small enough to need growing"; same
/// fail-safe, never-guess principle every other `Option` field this module
/// reads back from a `fields=`-restricted response already follows. Only
/// ever GROWS a dimension that's currently too small for what the caller is
/// about to reference in the same batchUpdate call - a sheet marko made
/// larger himself is always left exactly as he sized it.
///
/// 2.0.41: added after `batch_update`'s own atomic all-or-nothing behavior
/// (see that function's own doc comment) took down an ENTIRE Orders & Sales
/// structure refresh - not just the new Summary block, but the pre-existing
/// dropdowns and Status/Payout-status color-coding too - the first time
/// `ensure_orders_sheet_structure` sent a `bold_header_request` for the
/// Summary block's header cell at column AB (28th column) against marko's
/// real sheet, which - like most real Google Sheets - still had Google's own
/// default 26-column (A-Z) grid, never having had a reason to grow past it
/// before. Google's `repeatCell`/other structural requests are validated
/// against the sheet's CURRENT declared size and rejected outright
/// (`400 INVALID_ARGUMENT`, "exceeds grid limits") if a range falls outside
/// it - unlike the plain `values.*` endpoints, which grow the grid
/// automatically as needed. The fix: call this FIRST, and if it returns
/// `Some`, put that request at the FRONT of the same `requests` array a
/// caller is about to hand to `batch_update` - Sheets applies every request
/// in one `batchUpdate` call in array order, so the grid is already the
/// right size by the time any later request in that same call references a
/// cell inside it.
pub fn grow_grid_request_if_needed(
    sheet_id: i64,
    current_rows: Option<i64>,
    current_columns: Option<i64>,
    needed_rows: i64,
    needed_columns: i64,
) -> Option<serde_json::Value> {
    let new_rows = current_rows?.max(needed_rows);
    let new_columns = current_columns?.max(needed_columns);
    if new_rows == current_rows? && new_columns == current_columns? {
        return None;
    }
    Some(serde_json::json!({
        "updateSheetProperties": {
            "properties": {
                "sheetId": sheet_id,
                "gridProperties": { "rowCount": new_rows, "columnCount": new_columns }
            },
            "fields": "gridProperties.rowCount,gridProperties.columnCount"
        }
    }))
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

/// Builds the human-facing message for a non-2xx Google API response.
/// Pulled out of `parse_json_response` below purely so this is directly unit
/// testable: nothing in this crate can construct a real
/// `reqwest::blocking::Response` without an actual HTTP round trip, which
/// this sandbox can't make either way (see this module's doc comment).
///
/// 2.0.12: appends a clarifying hint whenever Google's own message is
/// "Unable to parse range: ..." - marko's own report, hit for a manually
/// (not app-)created sheet: this exact error is misleading, since Google
/// returns the identical wording both for a genuinely malformed A1 range
/// (the bug `a1_range` above already fixed in 2.0.9) AND for a
/// syntactically-fine range whose sheet/tab name simply is not found in that
/// spreadsheet - the far more likely case once quoting is handled correctly.
/// A person pasting an existing sheet's URL by hand has no way to tell those
/// two apart from Google's wording alone, especially since "sheet" is
/// genuinely ambiguous (the whole spreadsheet *file*, vs. the specific
/// *tab* inside it Google's API actually means) - "Test connection"/"Sync"
/// failing with this exact error right after "Save" reported success (Save
/// never made a network call at all until 2.0.12 - see
/// commands::sheets_sync::set_sheets_connection_impl) is the single most
/// common way that confusion has actually surfaced.
fn describe_error_response(status: reqwest::StatusCode, body: &str) -> String {
    let mut message = format!("Google Sheets rejected the request ({status}): {body}");
    if body.contains("Unable to parse range") {
        message.push_str(
            " - this usually means the exact tab name was not found in that spreadsheet, not a \
             syntax problem. Check the tab label at the bottom of the Google Sheet itself (not the \
             spreadsheet file's own name, which can read very similarly) - it must match the \
             \"Sheet/tab name\" field exactly, including capitalization and spacing.",
        );
    } else if status == reqwest::StatusCode::FORBIDDEN && body.contains("PERMISSION_DENIED") {
        // 2.0.13: marko's own report - hit for a manually-pasted sheet that
        // was never actually shared with whichever Google identity this
        // request used. The two identities need this in different ways
        // (see commands::google_auth::resolve_google_credential's doc
        // comment): signed in with his own Google account, the sheet needs
        // to already be his or shared with THAT account; not signed in
        // (the shared service account), it needs to be shared with the
        // service account's own e-mail specifically - Settings shows
        // exactly which one applies and the exact address to share with.
        message.push_str(
            " - this usually means the Google identity this request used does not have access to \
             that spreadsheet yet. Check Settings -> Integrations for the exact e-mail to share it \
             with (it differs depending on whether you're signed in with your own Google account or \
             using the app's shared one), then share the spreadsheet with that address (Editor \
             access) in Google Sheets itself.",
        );
    }
    message
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
        return Err(AppError::External(describe_error_response(status, &body)));
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
    fn describe_error_response_adds_a_tab_name_hint_for_unable_to_parse_range() {
        // The exact real error marko reported (2.0.12) for a manually
        // connected sheet whose tab name did not actually exist - proves the
        // clarifying hint fires for the real-world case, not a hypothetical.
        let status = reqwest::StatusCode::BAD_REQUEST;
        let body = r#"{ "error": { "code": 400, "message": "Unable to parse range: 'TIQR Manager - Pulls'!A1:A1", "status": "INVALID_ARGUMENT" } }"#;
        let message = describe_error_response(status, body);
        assert!(message.contains("Unable to parse range"), "must still include Google's own raw message: {message}");
        assert!(message.to_lowercase().contains("tab"), "must add a clarifying hint mentioning the tab name: {message}");
    }

    #[test]
    fn describe_error_response_adds_a_sharing_hint_for_permission_denied() {
        // The exact real error marko reported (2.0.13) for a manually
        // connected sheet never shared with the identity making the request.
        let status = reqwest::StatusCode::FORBIDDEN;
        let body = r#"{ "error": { "code": 403, "message": "The caller does not have permission", "status": "PERMISSION_DENIED" } }"#;
        let message = describe_error_response(status, body);
        assert!(message.contains("permission"), "must still include Google's own raw message: {message}");
        assert!(message.to_lowercase().contains("share"), "must add a clarifying hint about sharing the sheet: {message}");
        assert!(!message.to_lowercase().contains("tab label"), "a permission error must not get the range-parsing hint: {message}");
    }

    #[test]
    fn describe_error_response_adds_no_hint_for_an_unrelated_error() {
        let status = reqwest::StatusCode::NOT_FOUND;
        let body = r#"{ "error": { "code": 404, "message": "Requested entity was not found.", "status": "NOT_FOUND" } }"#;
        let message = describe_error_response(status, body);
        assert!(message.contains("not found"), "must still include Google's own raw message: {message}");
        assert!(!message.to_lowercase().contains("tab label"), "must not add an irrelevant hint to a different kind of error: {message}");
        assert!(!message.to_lowercase().contains("share the spreadsheet"), "must not add an irrelevant hint to a different kind of error: {message}");
    }

    #[test]
    fn spreadsheet_metadata_deserializes_the_real_shape_spreadsheets_get_returns() {
        // The exact shape `?fields=sheets.properties.title,sheets.properties.
        // sheetId` produces - a real spreadsheet has 1+ sheets, each carrying
        // (among many other fields this app never asks for) its own tab
        // title and its own numeric grid ID.
        let json = r#"{
            "sheets": [
                { "properties": { "title": "Objednávky", "sheetId": 0 } },
                { "properties": { "title": "Predaje 2026", "sheetId": 1234567890 } }
            ]
        }"#;
        let parsed: SpreadsheetMetadata = serde_json::from_str(json).expect("must parse a real spreadsheets.get response");
        let titles: Vec<String> = parsed.sheets.iter().map(|s| s.properties.title.clone()).collect();
        assert_eq!(titles, vec!["Objednávky".to_string(), "Predaje 2026".to_string()]);
        let ids: Vec<Option<i64>> = parsed.sheets.iter().map(|s| s.properties.sheet_id).collect();
        assert_eq!(ids, vec![Some(0), Some(1234567890)]);
    }

    #[test]
    fn spreadsheet_metadata_also_parses_the_title_only_shape_get_spreadsheet_sheet_titles_actually_receives() {
        // 2.0.20 regression test for the exact bug marko hit: `?fields=
        // sheets.properties.title` (get_spreadsheet_sheet_titles's own
        // query - no sheetId requested at all) genuinely never carries this
        // field - Google omits it entirely rather than sending `null`. Both
        // functions share this one struct, so it must tolerate a response
        // from EITHER `fields=` query, not just the one `sheetId`-aware
        // caller (get_sheet_structure_metadata) happens to send. Real error
        // this reproduces: "missing field `sheetId`" on every single "paste a
        // URL" tab auto-detect, the very first time marko connected a sheet
        // after 2.0.19 shipped.
        let json = r#"{ "sheets": [ { "properties": { "title": "Pulls" } } ] }"#;
        let parsed: SpreadsheetMetadata =
            serde_json::from_str(json).expect("a response with no sheetId at all must still parse");
        assert_eq!(parsed.sheets[0].properties.title, "Pulls");
        assert_eq!(parsed.sheets[0].properties.sheet_id, None);
    }

    #[test]
    fn spreadsheet_metadata_finds_the_matching_tab_by_title_not_position() {
        // Same real-shape JSON as the test above, parsed the same way
        // `get_sheet_structure_metadata` itself does - the tab it wants is
        // second in the list, so this also guards against accidentally
        // returning "just the first sheet" instead of actually matching the
        // title.
        let json = r#"{
            "sheets": [
                { "properties": { "title": "Objednávky", "sheetId": 0 } },
                { "properties": { "title": "Predaje 2026", "sheetId": 987654321 } }
            ]
        }"#;
        let parsed: SpreadsheetMetadata = serde_json::from_str(json).unwrap();
        let found = parsed.sheets.into_iter().find(|s| s.properties.title == "Predaje 2026").and_then(|s| s.properties.sheet_id);
        assert_eq!(found, Some(987654321));
    }

    #[test]
    fn spreadsheet_metadata_parses_a_real_conditional_formats_shape_with_existing_rules() {
        // The exact shape `?fields=...sheets.conditionalFormats.ranges.
        // startColumnIndex,sheets.conditionalFormats.ranges.endColumnIndex`
        // produces on a sheet that already has 2.0.19/2.0.21's own dropdowns
        // plus one earlier color rule on the Status column (index 17).
        let json = r#"{
            "sheets": [
                {
                    "properties": { "title": "Orders", "sheetId": 42 },
                    "conditionalFormats": [
                        { "ranges": [ { "sheetId": 42, "startRowIndex": 1, "endRowIndex": 501, "startColumnIndex": 17, "endColumnIndex": 18 } ] },
                        { "ranges": [ { "sheetId": 42, "startRowIndex": 1, "endRowIndex": 501, "startColumnIndex": 17, "endColumnIndex": 18 } ] }
                    ]
                }
            ]
        }"#;
        let parsed: SpreadsheetMetadata = serde_json::from_str(json).expect("must parse a real conditionalFormats shape");
        let columns: Vec<Option<i64>> =
            parsed.sheets[0].conditional_formats.iter().map(|f| single_column_index(&f.ranges)).collect();
        assert_eq!(columns, vec![Some(17), Some(17)]);
    }

    #[test]
    fn spreadsheet_metadata_defaults_conditional_formats_to_empty_when_the_field_is_entirely_absent() {
        // get_spreadsheet_sheet_titles's/a freshly-created sheet's own
        // response shape - no conditionalFormats key at all, not even an
        // empty array. Same #[serde(default)] tolerance sheet_id already
        // needed (2.0.20).
        let json = r#"{ "sheets": [ { "properties": { "title": "Pulls" } } ] }"#;
        let parsed: SpreadsheetMetadata = serde_json::from_str(json).expect("must parse with no conditionalFormats key at all");
        assert!(parsed.sheets[0].conditional_formats.is_empty());
    }

    #[test]
    fn spreadsheet_metadata_parses_a_real_grid_properties_shape() {
        // The exact shape `?fields=...sheets.properties.gridProperties.
        // rowCount,sheets.properties.gridProperties.columnCount` produces on
        // a perfectly ordinary real sheet that has never been resized past
        // Google's own default (1000 data rows + 1 header = 1001, 26
        // columns A-Z) - this is the shape that, before 2.0.41, this app had
        // no way to read at all.
        let json = r#"{
            "sheets": [
                { "properties": { "title": "Orders", "sheetId": 0, "gridProperties": { "rowCount": 1001, "columnCount": 26 } } }
            ]
        }"#;
        let parsed: SpreadsheetMetadata = serde_json::from_str(json).expect("must parse a real gridProperties shape");
        let props = &parsed.sheets[0].properties;
        assert_eq!(props.grid_properties.as_ref().and_then(|g| g.row_count), Some(1001));
        assert_eq!(props.grid_properties.as_ref().and_then(|g| g.column_count), Some(26));
    }

    #[test]
    fn spreadsheet_metadata_defaults_grid_properties_to_none_when_the_field_is_entirely_absent() {
        // get_spreadsheet_sheet_titles's own query never asks for
        // gridProperties at all - same "Option, tolerate absence, never
        // guess" convention sheet_id/conditional_formats already need.
        let json = r#"{ "sheets": [ { "properties": { "title": "Pulls" } } ] }"#;
        let parsed: SpreadsheetMetadata = serde_json::from_str(json).expect("must parse with no gridProperties key at all");
        assert!(parsed.sheets[0].properties.grid_properties.is_none());
    }

    #[test]
    fn grow_grid_request_if_needed_is_none_when_the_current_grid_already_covers_what_is_needed() {
        assert_eq!(grow_grid_request_if_needed(0, Some(1001), Some(26), 500, 25), None);
    }

    #[test]
    fn grow_grid_request_if_needed_is_none_exactly_at_the_boundary_needed_equal_to_current() {
        // needed_columns == current_columns must count as "fits" - matches
        // the Sheets API's own exclusive-endColumnIndex convention
        // (bold_header_request's `endColumnIndex: col_index + 1`, i.e. a
        // 26-column grid already fully covers column index 25 / column Z).
        assert_eq!(grow_grid_request_if_needed(0, Some(1001), Some(26), 1001, 26), None);
    }

    #[test]
    fn grow_grid_request_if_needed_grows_only_columns_when_only_columns_are_too_small() {
        // The real incident: a Summary block header cell at column index 27
        // (AB, the 28th column) against a real sheet's default 26-column
        // grid - needed_columns = 28.
        let req = grow_grid_request_if_needed(555, Some(1001), Some(26), 500, 28).expect("columns are too small, must grow");
        assert_eq!(
            req,
            serde_json::json!({
                "updateSheetProperties": {
                    "properties": { "sheetId": 555, "gridProperties": { "rowCount": 1001, "columnCount": 28 } },
                    "fields": "gridProperties.rowCount,gridProperties.columnCount"
                }
            })
        );
    }

    #[test]
    fn grow_grid_request_if_needed_grows_only_rows_when_only_rows_are_too_small() {
        // A sheet with far more real data than Google's own default 1000
        // rows - dropdowns/colors need to reach further down than the grid
        // currently goes, columns are untouched.
        let req = grow_grid_request_if_needed(555, Some(1001), Some(26), 1502, 20).expect("rows are too small, must grow");
        assert_eq!(
            req,
            serde_json::json!({
                "updateSheetProperties": {
                    "properties": { "sheetId": 555, "gridProperties": { "rowCount": 1502, "columnCount": 26 } },
                    "fields": "gridProperties.rowCount,gridProperties.columnCount"
                }
            })
        );
    }

    #[test]
    fn grow_grid_request_if_needed_grows_both_dimensions_at_once_when_both_are_too_small() {
        let req = grow_grid_request_if_needed(1, Some(100), Some(10), 200, 30).expect("both too small, must grow both");
        assert_eq!(
            req,
            serde_json::json!({
                "updateSheetProperties": {
                    "properties": { "sheetId": 1, "gridProperties": { "rowCount": 200, "columnCount": 30 } },
                    "fields": "gridProperties.rowCount,gridProperties.columnCount"
                }
            })
        );
    }

    #[test]
    fn grow_grid_request_if_needed_never_shrinks_a_dimension_marko_made_larger_himself() {
        // His own real sheet already has 2000 rows (he added them by hand) -
        // a refresh that only actually needs 501 must never shrink it back.
        assert_eq!(grow_grid_request_if_needed(1, Some(2000), Some(26), 501, 25), None);
    }

    #[test]
    fn grow_grid_request_if_needed_is_none_when_current_row_count_is_unknown() {
        // Fail-safe: never guess a resize when the CURRENT size can't be
        // read back at all - same principle as every other Option field in
        // this module.
        assert_eq!(grow_grid_request_if_needed(1, None, Some(26), 500, 28), None);
    }

    #[test]
    fn grow_grid_request_if_needed_is_none_when_current_column_count_is_unknown() {
        assert_eq!(grow_grid_request_if_needed(1, Some(1001), None, 500, 28), None);
    }

    #[test]
    fn single_column_index_recognizes_exactly_the_shape_this_app_itself_creates() {
        let one_column = vec![ConditionalFormatGridRange { start_column_index: Some(4), end_column_index: Some(5) }];
        assert_eq!(single_column_index(&one_column), Some(4));
    }

    #[test]
    fn single_column_index_is_none_for_a_range_spanning_more_than_one_column() {
        // Not a shape this app ever creates - almost certainly something
        // marko added by hand - must never be mistaken for "ours".
        let multi_column = vec![ConditionalFormatGridRange { start_column_index: Some(0), end_column_index: Some(3) }];
        assert_eq!(single_column_index(&multi_column), None);
    }

    #[test]
    fn single_column_index_is_none_for_more_than_one_range_or_zero_ranges() {
        let two_ranges = vec![
            ConditionalFormatGridRange { start_column_index: Some(1), end_column_index: Some(2) },
            ConditionalFormatGridRange { start_column_index: Some(3), end_column_index: Some(4) },
        ];
        assert_eq!(single_column_index(&two_ranges), None);
        assert_eq!(single_column_index(&[]), None);
    }

    #[test]
    fn single_column_index_is_none_when_a_bound_is_unset_ie_an_unbounded_range() {
        // Google reports an unbounded side of a range as simply absent, not
        // a sentinel like 0 or -1 - a rule marko set covering "the whole
        // column" via the Sheets UI shortcut looks exactly like this.
        let unbounded_end = vec![ConditionalFormatGridRange { start_column_index: Some(2), end_column_index: None }];
        assert_eq!(single_column_index(&unbounded_end), None);
    }

    #[test]
    fn conditional_format_indices_to_replace_finds_only_rules_on_managed_columns_descending() {
        // Index 0 -> column 4 (managed), index 1 -> column 9 (NOT managed,
        // e.g. something marko added himself), index 2 -> column 4 again
        // (managed) - must return exactly [2, 0], never touching index 1.
        let existing = vec![Some(4), Some(9), Some(4)];
        assert_eq!(conditional_format_indices_to_replace(&existing, &[4, 17]), vec![2, 0]);
    }

    #[test]
    fn conditional_format_indices_to_replace_skips_rules_that_are_not_single_column_ie_none() {
        let existing = vec![None, Some(4)];
        assert_eq!(conditional_format_indices_to_replace(&existing, &[4]), vec![1]);
    }

    #[test]
    fn conditional_format_indices_to_replace_is_empty_when_nothing_existing_matches() {
        let existing = vec![Some(9), None];
        assert!(conditional_format_indices_to_replace(&existing, &[4, 17]).is_empty());
    }

    #[test]
    fn add_conditional_format_color_request_builds_the_exact_shape_google_documents() {
        let req = add_conditional_format_color_request(555, 1, 501, 17, "Sold", (0.71, 0.88, 0.80));
        assert_eq!(
            req,
            serde_json::json!({
                "addConditionalFormatRule": {
                    "rule": {
                        "ranges": [{
                            "sheetId": 555,
                            "startRowIndex": 1,
                            "endRowIndex": 501,
                            "startColumnIndex": 17,
                            "endColumnIndex": 18
                        }],
                        "booleanRule": {
                            "condition": {
                                "type": "TEXT_EQ",
                                "values": [{ "userEnteredValue": "Sold" }]
                            },
                            "format": {
                                "backgroundColor": { "red": 0.71, "green": 0.88, "blue": 0.80 }
                            }
                        }
                    },
                    "index": 0
                }
            })
        );
    }

    #[test]
    fn bold_header_request_builds_the_exact_shape_google_documents() {
        let req = bold_header_request(555, 0, 1, 27, Some((0.85, 0.88, 0.95)));
        assert_eq!(
            req,
            serde_json::json!({
                "repeatCell": {
                    "range": {
                        "sheetId": 555,
                        "startRowIndex": 0,
                        "endRowIndex": 1,
                        "startColumnIndex": 27,
                        "endColumnIndex": 28
                    },
                    "cell": {
                        "userEnteredFormat": {
                            "textFormat": { "bold": true },
                            "backgroundColor": { "red": 0.85, "green": 0.88, "blue": 0.95 }
                        }
                    },
                    "fields": "userEnteredFormat(textFormat,backgroundColor)"
                }
            })
        );
    }

    #[test]
    fn bold_header_request_omits_background_color_entirely_when_none_is_given() {
        let req = bold_header_request(555, 0, 1, 27, None);
        assert_eq!(
            req,
            serde_json::json!({
                "repeatCell": {
                    "range": {
                        "sheetId": 555,
                        "startRowIndex": 0,
                        "endRowIndex": 1,
                        "startColumnIndex": 27,
                        "endColumnIndex": 28
                    },
                    "cell": { "userEnteredFormat": { "textFormat": { "bold": true } } },
                    "fields": "userEnteredFormat(textFormat,backgroundColor)"
                }
            })
        );
    }

    #[test]
    fn currency_number_format_request_builds_the_exact_shape_google_documents() {
        let req = currency_number_format_request(555, 1, 4, 28);
        assert_eq!(
            req,
            serde_json::json!({
                "repeatCell": {
                    "range": {
                        "sheetId": 555,
                        "startRowIndex": 1,
                        "endRowIndex": 4,
                        "startColumnIndex": 28,
                        "endColumnIndex": 29
                    },
                    "cell": {
                        "userEnteredFormat": {
                            "numberFormat": { "type": "CURRENCY", "pattern": "#,##0.00 €" }
                        }
                    },
                    "fields": "userEnteredFormat.numberFormat"
                }
            })
        );
    }

    #[test]
    fn currency_number_format_request_pattern_uses_only_locale_invariant_grouping_and_decimal_tokens() {
        // 2.0.42: the whole point of this pattern (see its own doc comment)
        // is that "," and "." in a numberFormat pattern are pattern-grammar
        // tokens, not literal punctuation - Sheets renders them per the
        // spreadsheet's own locale. The only literal character allowed in
        // this pattern is the currency symbol itself and the space next to
        // it - if this ever grows a second literal character (e.g. someone
        // "fixes" it to hardcode a thousands separator), that would silently
        // reintroduce the exact class of bug this version fixed.
        let req = currency_number_format_request(1, 0, 1, 0);
        let pattern = req["repeatCell"]["cell"]["userEnteredFormat"]["numberFormat"]["pattern"].as_str().unwrap();
        assert_eq!(pattern, "#,##0.00 €");
        assert_eq!(pattern.matches('€').count(), 1);
    }

    #[test]
    fn delete_conditional_format_rule_request_builds_the_exact_shape_google_documents() {
        let req = delete_conditional_format_rule_request(555, 2);
        assert_eq!(req, serde_json::json!({ "deleteConditionalFormatRule": { "sheetId": 555, "index": 2 } }));
    }

    #[test]
    fn set_data_validation_request_builds_the_exact_shape_google_documents() {
        let req = set_data_validation_request(555, 1, 501, 13, &["Listed".to_string(), "Unlisted".to_string(), "Sold".to_string()]);
        assert_eq!(
            req,
            serde_json::json!({
                "setDataValidation": {
                    "range": {
                        "sheetId": 555,
                        "startRowIndex": 1,
                        "endRowIndex": 501,
                        "startColumnIndex": 13,
                        "endColumnIndex": 14
                    },
                    "rule": {
                        "condition": {
                            "type": "ONE_OF_LIST",
                            "values": [
                                { "userEnteredValue": "Listed" },
                                { "userEnteredValue": "Unlisted" },
                                { "userEnteredValue": "Sold" }
                            ]
                        },
                        "showCustomUi": true,
                        "strict": false
                    }
                }
            })
        );
    }

    #[test]
    fn set_data_validation_request_end_column_index_is_exactly_one_past_the_start() {
        let req = set_data_validation_request(1, 1, 10, 4, &["Yes".to_string(), "No".to_string()]);
        assert_eq!(req["setDataValidation"]["range"]["startColumnIndex"], 4);
        assert_eq!(req["setDataValidation"]["range"]["endColumnIndex"], 5);
    }

    #[test]
    fn set_data_validation_request_is_never_strict_so_a_new_value_is_never_blocked() {
        // marko explicitly wants a value added in the sheet or the app to
        // keep working even before this rule has been refreshed to include
        // it - "strict" (Sheets' "Reject input") would defeat that.
        let req = set_data_validation_request(1, 1, 10, 0, &["A".to_string()]);
        assert_eq!(req["setDataValidation"]["rule"]["strict"], false);
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

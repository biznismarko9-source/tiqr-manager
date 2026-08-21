//! Tauri commands for the Google Sheets connection itself (Settings ->
//! Integrations): is sync available in this build, which spreadsheet is
//! linked for a given data source, and a manual "Test connection" check.
//!
//! Deliberately just the *connection*, not yet the actual sync/import logic
//! for any specific data source (Pulls first, Tickets/Orders later) - that
//! needs the exact column mapping of marko's real sheet nailed down first
//! (see the 2.0.2 report), and building it against a guessed mapping would
//! risk exactly the kind of rework this project's whole history has tried
//! to avoid. Everything in this file, by contrast, is fully generic across
//! any future data source and safe to build ahead of that.
//!
//! `data_source` is always a plain string key (e.g. `"pulls"`) - see
//! models.rs's Google Sheets section doc comment for why.

use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::google_sheets;
use crate::models::{
    SheetsConnectionConfig, SheetsConnectionStatus, SheetsConnectionTestResult, SpreadsheetTabsResult,
};
use rusqlite::{params, Connection, OptionalExtension};
use tauri::State;

/// The only currencies a connected sheet's rows can be synced as (2.0.3:
/// marko's Pulls tracker has no currency column of its own - see
/// `SheetsConnectionConfig::currency`'s doc comment). Deliberately a short,
/// explicit allow-list rather than accepting any free-text currency code:
/// this is money, and this app never lets a value it can't vouch for reach
/// `price_cents`/`currency` (same principle as `money.rs`).
pub const ALLOWED_CURRENCIES: &[&str] = &["EUR", "USD", "GBP"];

// `pub(crate)` (not just `fn`) on this group since 2.0.3: commands::
// pulls_sheet_sync reuses the exact same app_settings key naming and
// load/save behavior for the connection itself and its last-synced stamp,
// rather than a second, easily-drifting copy of this scheme.
pub(crate) fn connection_key(data_source: &str) -> String {
    format!("sheets_connection:{data_source}")
}

pub(crate) fn last_synced_key(data_source: &str) -> String {
    format!("sheets_last_synced:{data_source}")
}

// `pub(crate)` on this trio since 2.0.5: commands::google_auth reuses the
// same generic app_settings key/value store for the signed-in Google
// account (a different concept entirely - one per installation, not one per
// data source - but the same underlying table and the same tested
// read/write/delete behavior, not worth a second copy of three SQL
// statements).
pub(crate) fn get_setting(conn: &Connection, key: &str) -> AppResult<Option<String>> {
    Ok(conn
        .query_row("SELECT value FROM app_settings WHERE key = ?1", params![key], |r| r.get(0))
        .optional()?)
}

pub(crate) fn set_setting(conn: &Connection, key: &str, value: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

pub(crate) fn delete_setting(conn: &Connection, key: &str) -> AppResult<()> {
    conn.execute("DELETE FROM app_settings WHERE key = ?1", params![key])?;
    Ok(())
}

pub(crate) fn load_connection(conn: &Connection, data_source: &str) -> AppResult<Option<SheetsConnectionConfig>> {
    match get_setting(conn, &connection_key(data_source))? {
        None => Ok(None),
        Some(json) => serde_json::from_str(&json)
            .map(Some)
            .map_err(|e| AppError::Other(format!("stored Google Sheets connection is corrupt: {e}"))),
    }
}

/// Accepts either a bare spreadsheet ID or a full URL pasted straight out of
/// the browser's address bar (`https://docs.google.com/spreadsheets/d/
/// <ID>/edit#gid=0`) - so connecting a sheet is really just "copy the URL,
/// paste it here", nothing to manually extract by hand.
pub fn extract_spreadsheet_id(input: &str) -> Option<String> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }
    if let Some(after) = input.split("/spreadsheets/d/").nth(1) {
        let id = after.split(['/', '?', '#']).next().unwrap_or("");
        if !id.is_empty() {
            return Some(id.to_string());
        }
    }
    // Not a URL - accept it as a bare ID only if it looks like one (Sheets
    // IDs are letters/digits/-/_ , never contain a slash or whitespace), so
    // a mistakenly pasted sentence fails clearly instead of being "saved"
    // as a bogus ID that only breaks later at sync time.
    let looks_like_id = !input.contains(['/', ' ', '\t', '\n']) && input.len() > 10;
    if looks_like_id {
        Some(input.to_string())
    } else {
        None
    }
}

fn get_sheets_connection_status_impl(conn: &Connection, data_source: &str) -> AppResult<SheetsConnectionStatus> {
    let account = google_sheets::embedded_service_account();
    Ok(SheetsConnectionStatus {
        sync_available: account.is_some(),
        service_account_email: account.map(|a| a.client_email),
        connection: load_connection(conn, data_source)?,
        last_synced_at: get_setting(conn, &last_synced_key(data_source))?,
    })
}

#[tauri::command]
pub fn get_sheets_connection_status(state: State<AppState>, data_source: String) -> AppResult<SheetsConnectionStatus> {
    let conn = state.db.lock().unwrap();
    get_sheets_connection_status_impl(&conn, &data_source)
}

// `pub(crate)` since 2.0.4: commands::pulls_sheet_sync::create_pulls_sheet_impl
// reuses this directly as the single source of truth for persisting a
// connection, once it has just created+shared a brand-new spreadsheet itself
// - rather than a second, easily-drifting copy of the same validate-and-save
// logic. `spreadsheet_url_or_id` accepts a bare ID there too, same as a
// pasted URL would: `extract_spreadsheet_id` already handles both.
pub(crate) fn set_sheets_connection_impl(
    conn: &Connection,
    data_source: &str,
    spreadsheet_url_or_id: &str,
    sheet_tab: &str,
    currency: &str,
) -> AppResult<SheetsConnectionConfig> {
    let spreadsheet_id = extract_spreadsheet_id(spreadsheet_url_or_id)
        .ok_or_else(|| AppError::Validation("That doesn't look like a Google Sheets URL or ID".to_string()))?;
    let sheet_tab = sheet_tab.trim();
    if sheet_tab.is_empty() {
        return Err(AppError::Validation("Sheet/tab name is required".to_string()));
    }
    let currency_upper = currency.trim().to_uppercase();
    if !ALLOWED_CURRENCIES.contains(&currency_upper.as_str()) {
        return Err(AppError::Validation(format!(
            "Currency must be one of {} - got '{currency}'",
            ALLOWED_CURRENCIES.join(", ")
        )));
    }
    let config = SheetsConnectionConfig { spreadsheet_id, sheet_tab: sheet_tab.to_string(), currency: currency_upper };
    let json = serde_json::to_string(&config).map_err(|e| AppError::Other(e.to_string()))?;
    set_setting(conn, &connection_key(data_source), &json)?;
    Ok(config)
}

#[tauri::command]
pub fn set_sheets_connection(
    state: State<AppState>,
    data_source: String,
    spreadsheet_url_or_id: String,
    sheet_tab: String,
    currency: String,
) -> AppResult<SheetsConnectionConfig> {
    let conn = state.db.lock().unwrap();
    set_sheets_connection_impl(&conn, &data_source, &spreadsheet_url_or_id, &sheet_tab, &currency)
}

fn clear_sheets_connection_impl(conn: &Connection, data_source: &str) -> AppResult<()> {
    delete_setting(conn, &connection_key(data_source))?;
    delete_setting(conn, &last_synced_key(data_source))?;
    conn.execute("DELETE FROM sheet_sync_links WHERE data_source = ?1", params![data_source])?;
    Ok(())
}

/// Disconnects a data source entirely - forgets the linked spreadsheet AND
/// every row-matching link for it (see migrations/008_sheet_sync.sql). A
/// later reconnect (even to the very same sheet) therefore starts from a
/// clean slate rather than silently reusing stale markers from before.
#[tauri::command]
pub fn clear_sheets_connection(state: State<AppState>, data_source: String) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    clear_sheets_connection_impl(&conn, &data_source)
}

/// Whether an error message looks like the Sheets API's "the tab name in
/// this range does not exist in that spreadsheet" case - worth enriching
/// with the spreadsheet's actual tab titles (see `format_with_real_tab_titles`
/// below) rather than leaving marko to guess. 2.0.13: marko's own report -
/// the plain-English hint google_sheets::describe_error_response already
/// adds ("check the tab label...") kept the error recurring anyway, because
/// knowing a mismatch is LIKELY still doesn't say what the right name IS.
fn looks_like_a_missing_tab_error(message: &str) -> bool {
    message.contains("Unable to parse range")
}

/// Appends the spreadsheet's real, exact tab titles to `message` - turning
/// "the tab name probably doesn't exist" into a straight copy-paste of what
/// it should actually be. Pure and unit-tested on its own; the network call
/// that produces `titles` lives in `test_sheets_connection_impl` below, the
/// same "pure logic vs. untested network shell" split every other command
/// in this app already uses (see e.g. pulls_sheet_sync.rs's own module
/// comment). Leaves `message` untouched when `titles` is empty - a
/// spreadsheet reporting zero sheets is not itself useful information to
/// hand back, and never worth pretending otherwise.
fn format_with_real_tab_titles(message: &str, titles: &[String]) -> String {
    if titles.is_empty() {
        return message.to_string();
    }
    let quoted = titles.iter().map(|t| format!("\"{t}\"")).collect::<Vec<_>>().join(", ");
    format!(
        "{message} The tabs that actually exist in this spreadsheet are: {quoted}. Update \"Sheet/tab name\" to match one of these exactly."
    )
}

fn test_sheets_connection_impl(conn: &Connection, data_source: &str) -> AppResult<SheetsConnectionTestResult> {
    let Some(connection) = load_connection(conn, data_source)? else {
        return Ok(SheetsConnectionTestResult { ok: false, message: "No spreadsheet is connected yet.".to_string() });
    };

    // 2.0.5: the signed-in person's own OAuth token when there is one, the
    // shared service account otherwise - see
    // commands::google_auth::resolve_google_credential's doc comment. Same
    // "never propagate AppError, always a readable ok:false" convention this
    // function already used for embedded_service_account/fetch_access_token
    // before this credential resolution moved behind that one shared call.
    let credential = match crate::commands::google_auth::resolve_google_credential(conn, false) {
        Ok(c) => c,
        Err(e) => return Ok(SheetsConnectionTestResult { ok: false, message: e.to_string() }),
    };
    let token = credential.access_token();
    let range = google_sheets::a1_range(&connection.sheet_tab, "A1:A1");
    match google_sheets::get_values(token, &connection.spreadsheet_id, &range) {
        Ok(_) => Ok(SheetsConnectionTestResult {
            ok: true,
            message: format!("Connected - the app can read \"{}\".", connection.sheet_tab),
        }),
        Err(e) => {
            let mut message = e.to_string();
            // 2.0.13: best-effort only - a spreadsheet this credential can't
            // even read the metadata of (e.g. the exact same permission
            // problem that made the original call fail) just keeps the
            // original message, never a second, more confusing failure
            // layered on top of the first.
            if looks_like_a_missing_tab_error(&message) {
                if let Ok(titles) = google_sheets::get_spreadsheet_sheet_titles(token, &connection.spreadsheet_id) {
                    message = format_with_real_tab_titles(&message, &titles);
                }
            }
            Ok(SheetsConnectionTestResult { ok: false, message })
        }
    }
}

/// Manual "Test connection" button - never runs on its own, only when the
/// user clicks it. Always returns `Ok` with `ok:false` and a human-readable
/// reason for every *expected* failure (not configured, not connected,
/// sheet not shared, network unreachable) rather than propagating an
/// `AppError` - the frontend shows one thing either way, it never needs a
/// separate error-vs-failed-result branch for what is, to the person
/// looking at the screen, the same "it didn't work, here's why" message.
#[tauri::command]
pub fn test_sheets_connection(state: State<AppState>, data_source: String) -> AppResult<SheetsConnectionTestResult> {
    let conn = state.db.lock().unwrap();
    test_sheets_connection_impl(&conn, &data_source)
}

/// Best-effort lookup of a pasted spreadsheet's real tab names, so the
/// frontend can offer them as a dropdown instead of asking marko to type the
/// exact tab name by hand (2.0.14: his own report - even with
/// `format_with_real_tab_titles` above telling him the exact right answer,
/// still having to hand-type it afterwards remained the actual recurring
/// failure - "nejde tam manualne dat svoju tabulku, skus zmenit styl akoto
/// funguje"). Deliberately takes the raw URL/ID text directly, not a saved
/// `data_source` connection - this runs BEFORE anything is saved, as the user
/// is still filling in the "Spreadsheet URL or ID" field. Same "never
/// propagate AppError, always a readable ok:false" convention as
/// `test_sheets_connection_impl`: an incomplete paste, no signed-in/no
/// service account, or a not-yet-shared sheet are all expected, ordinary
/// states while the form is being filled in, not errors to surface loudly.
fn detect_spreadsheet_tabs_impl(conn: &Connection, spreadsheet_url_or_id: &str) -> AppResult<SpreadsheetTabsResult> {
    let Some(spreadsheet_id) = extract_spreadsheet_id(spreadsheet_url_or_id) else {
        return Ok(SpreadsheetTabsResult {
            ok: false,
            tabs: vec![],
            message: "That doesn't look like a Google Sheets URL or ID yet.".to_string(),
        });
    };
    let credential = match crate::commands::google_auth::resolve_google_credential(conn, false) {
        Ok(c) => c,
        Err(e) => return Ok(SpreadsheetTabsResult { ok: false, tabs: vec![], message: e.to_string() }),
    };
    match google_sheets::get_spreadsheet_sheet_titles(credential.access_token(), &spreadsheet_id) {
        Ok(tabs) if tabs.is_empty() => Ok(SpreadsheetTabsResult {
            ok: false,
            tabs: vec![],
            message: "Google reported no tabs at all in that spreadsheet.".to_string(),
        }),
        Ok(tabs) => Ok(SpreadsheetTabsResult { ok: true, tabs, message: String::new() }),
        Err(e) => Ok(SpreadsheetTabsResult { ok: false, tabs: vec![], message: e.to_string() }),
    }
}

/// Called from Settings as soon as the "Spreadsheet URL or ID" field loses
/// focus (and once on load for an already-connected sheet) - see
/// `detect_spreadsheet_tabs_impl`'s doc comment for why this exists.
#[tauri::command]
pub fn detect_spreadsheet_tabs(
    state: State<AppState>,
    spreadsheet_url_or_id: String,
) -> AppResult<SpreadsheetTabsResult> {
    let conn = state.db.lock().unwrap();
    detect_spreadsheet_tabs_impl(&conn, &spreadsheet_url_or_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_conn;

    #[test]
    fn extract_spreadsheet_id_pulls_the_id_out_of_a_pasted_browser_url() {
        let url = "https://docs.google.com/spreadsheets/d/1AbC-XyZ_9900/edit#gid=1234567";
        assert_eq!(extract_spreadsheet_id(url), Some("1AbC-XyZ_9900".to_string()));
    }

    #[test]
    fn extract_spreadsheet_id_pulls_the_id_out_of_a_url_with_no_trailing_fragment() {
        let url = "https://docs.google.com/spreadsheets/d/1AbC-XyZ_9900";
        assert_eq!(extract_spreadsheet_id(url), Some("1AbC-XyZ_9900".to_string()));
    }

    #[test]
    fn extract_spreadsheet_id_accepts_a_bare_id_pasted_directly() {
        assert_eq!(extract_spreadsheet_id("1AbC-XyZ_9900"), Some("1AbC-XyZ_9900".to_string()));
    }

    #[test]
    fn extract_spreadsheet_id_rejects_obvious_non_ids_instead_of_saving_garbage() {
        assert_eq!(extract_spreadsheet_id(""), None);
        assert_eq!(extract_spreadsheet_id("   "), None);
        assert_eq!(extract_spreadsheet_id("not a url or id"), None);
        assert_eq!(extract_spreadsheet_id("short"), None);
    }

    #[test]
    fn set_then_get_connection_round_trips_through_app_settings() {
        let conn = test_conn();
        let saved = set_sheets_connection_impl(
            &conn,
            "pulls",
            "https://docs.google.com/spreadsheets/d/1AbC-XyZ_9900/edit",
            "Pulls",
            "EUR",
        )
        .expect("a valid URL, tab name and currency must be accepted");
        assert_eq!(saved.spreadsheet_id, "1AbC-XyZ_9900");
        assert_eq!(saved.sheet_tab, "Pulls");
        assert_eq!(saved.currency, "EUR");

        let status = get_sheets_connection_status_impl(&conn, "pulls").unwrap();
        assert_eq!(status.connection, Some(saved));
    }

    #[test]
    fn set_sheets_connection_rejects_an_empty_tab_name() {
        let conn = test_conn();
        let result = set_sheets_connection_impl(&conn, "pulls", "1AbC-XyZ_9900", "   ", "EUR");
        assert!(result.is_err(), "an empty tab name must be rejected before anything is saved");
    }

    #[test]
    fn set_sheets_connection_accepts_only_eur_usd_gbp() {
        let conn = test_conn();
        for ok in ["EUR", "USD", "GBP", "eur", "usd", "gbp"] {
            assert!(
                set_sheets_connection_impl(&conn, "pulls", "1AbC-XyZ_9900", "Pulls", ok).is_ok(),
                "'{ok}' must be accepted"
            );
        }
        for bad in ["CZK", "PLN", "", "   ", "EURO"] {
            assert!(
                set_sheets_connection_impl(&conn, "pulls", "1AbC-XyZ_9900", "Pulls", bad).is_err(),
                "'{bad}' must be rejected"
            );
        }
    }

    #[test]
    fn set_sheets_connection_stores_currency_uppercased_regardless_of_input_case() {
        let conn = test_conn();
        let saved = set_sheets_connection_impl(&conn, "pulls", "1AbC-XyZ_9900", "Pulls", "gbp").unwrap();
        assert_eq!(saved.currency, "GBP");
    }

    #[test]
    fn a_data_source_with_no_connection_yet_reports_none_not_an_error() {
        let conn = test_conn();
        let status = get_sheets_connection_status_impl(&conn, "pulls").unwrap();
        assert_eq!(status.connection, None);
        assert_eq!(status.last_synced_at, None);
    }

    #[test]
    fn two_data_sources_keep_completely_independent_connections() {
        let conn = test_conn();
        set_sheets_connection_impl(&conn, "pulls", "1PullsSheetId000", "Pulls", "EUR").unwrap();
        set_sheets_connection_impl(&conn, "tickets", "1TicketsSheetId0", "Tickets", "USD").unwrap();

        let pulls_status = get_sheets_connection_status_impl(&conn, "pulls").unwrap();
        let tickets_status = get_sheets_connection_status_impl(&conn, "tickets").unwrap();
        assert_eq!(pulls_status.connection.unwrap().spreadsheet_id, "1PullsSheetId000");
        assert_eq!(tickets_status.connection.unwrap().spreadsheet_id, "1TicketsSheetId0");
    }

    #[test]
    fn clear_sheets_connection_forgets_the_connection_and_every_sync_link() {
        let conn = test_conn();
        set_sheets_connection_impl(&conn, "pulls", "1AbC-XyZ_9900", "Pulls", "EUR").unwrap();
        conn.execute(
            "INSERT INTO sheet_sync_links (data_source, local_id, sheet_marker, last_synced_snapshot, last_synced_at)
             VALUES ('pulls', 1, 'PULL-000001', '{}', strftime('%Y-%m-%dT%H:%M:%fZ','now'))",
            [],
        )
        .unwrap();

        clear_sheets_connection_impl(&conn, "pulls").unwrap();

        let status = get_sheets_connection_status_impl(&conn, "pulls").unwrap();
        assert_eq!(status.connection, None);
        let links: i64 = conn
            .query_row("SELECT COUNT(*) FROM sheet_sync_links WHERE data_source='pulls'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(links, 0, "disconnecting must forget row-matching links too, so a later reconnect starts clean");
    }

    #[test]
    fn clear_sheets_connection_never_touches_a_different_data_sources_links() {
        let conn = test_conn();
        set_sheets_connection_impl(&conn, "pulls", "1AbC-XyZ_9900", "Pulls", "EUR").unwrap();
        set_sheets_connection_impl(&conn, "tickets", "1TicketsSheetId0", "Tickets", "USD").unwrap();
        conn.execute(
            "INSERT INTO sheet_sync_links (data_source, local_id, sheet_marker, last_synced_snapshot, last_synced_at)
             VALUES ('tickets', 1, 'ORD-000001', '{}', strftime('%Y-%m-%dT%H:%M:%fZ','now'))",
            [],
        )
        .unwrap();

        clear_sheets_connection_impl(&conn, "pulls").unwrap();

        let tickets_status = get_sheets_connection_status_impl(&conn, "tickets").unwrap();
        assert!(tickets_status.connection.is_some(), "clearing pulls must never disconnect tickets");
        let links: i64 = conn
            .query_row("SELECT COUNT(*) FROM sheet_sync_links WHERE data_source='tickets'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(links, 1);
    }

    #[test]
    fn looks_like_a_missing_tab_error_matches_the_real_google_wording() {
        assert!(looks_like_a_missing_tab_error(
            "Google Sheets rejected the request (400 Bad Request): { \"error\": { \"message\": \"Unable to parse range: 'TIQR Manager - Orders'!A1:A1\" } }"
        ));
        assert!(!looks_like_a_missing_tab_error(
            "Google Sheets rejected the request (403 Forbidden): { \"error\": { \"message\": \"The caller does not have permission\" } }"
        ));
    }

    #[test]
    fn format_with_real_tab_titles_lists_every_tab_quoted_and_comma_separated() {
        let message = format_with_real_tab_titles("original error", &["Objednávky".to_string(), "Predaje".to_string()]);
        assert!(message.starts_with("original error"), "must keep Google's own message intact: {message}");
        assert!(message.contains("\"Objednávky\", \"Predaje\""), "{message}");
    }

    #[test]
    fn format_with_real_tab_titles_leaves_the_message_alone_when_there_are_no_titles() {
        let message = format_with_real_tab_titles("original error", &[]);
        assert_eq!(message, "original error", "no titles to suggest is not itself useful information to append");
    }

    #[test]
    fn test_connection_reports_a_clear_reason_instead_of_an_error_when_nothing_is_connected_yet() {
        // No embedded service account either, in this test build (see
        // google_sheets.rs's embedded_service_account_is_none_on_a_plain_local_build
        // test) - either missing piece must produce a readable ok:false, never
        // an AppError the frontend would have to special-case.
        let conn = test_conn();
        let result = test_sheets_connection_impl(&conn, "pulls").unwrap();
        assert!(!result.ok);
        assert!(!result.message.is_empty());
    }

    #[test]
    fn detect_spreadsheet_tabs_reports_a_clear_reason_for_input_that_is_not_a_url_or_id() {
        let conn = test_conn();
        let result = detect_spreadsheet_tabs_impl(&conn, "not a url or id").unwrap();
        assert!(!result.ok);
        assert!(result.tabs.is_empty());
        assert!(!result.message.is_empty());
    }

    #[test]
    fn detect_spreadsheet_tabs_reports_a_clear_reason_when_no_credential_is_available() {
        // Same guarantee this whole test suite relies on elsewhere (see
        // test_connection_reports_a_clear_reason_instead_of_an_error_when_nothing_is_connected_yet
        // above): no embedded service account and nobody signed in, so this
        // never makes a real network call - it fails fast on credential
        // resolution alone.
        let conn = test_conn();
        let result = detect_spreadsheet_tabs_impl(&conn, "1AbC-XyZ_9900").unwrap();
        assert!(!result.ok);
        assert!(result.tabs.is_empty());
        assert!(!result.message.is_empty());
    }
}

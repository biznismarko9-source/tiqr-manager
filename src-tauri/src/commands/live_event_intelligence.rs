//! Live Event Intelligence (2.4.0) - marko's "Live Event Intelligence
//! Foundation" spec: an Event can now optionally carry a confirmed ONLINE
//! identity on exactly 3 marketplaces - Viagogo, Vivid Seats, Ticombo
//! ("Podporuj LEN tieto 3 marketplace pre teraz... NEPRIDÁVAJ StubHub,
//! Seatriks ani ine"). Foundation work only: this module finds/stores WHERE
//! an event lives online and whether a human confirmed that. It never reads
//! prices or listings, never feeds Price Checker or any pricing decision,
//! and Section/Row/Seat are never referenced anywhere in it.
//!
//! ## Why this reuses the Visible Scanner's TECHNIQUE, not its code
//! Price Checker's 2.1.9 rewrite (`commands::price_checker_scanner`)
//! already solved "let marko look at and interact with a real marketplace
//! page himself, safely": a real, VISIBLE `WebviewWindow` the human drives
//! (searches, scrolls, solves any CAPTCHA), with the backend only ever
//! reading what's CURRENTLY rendered, on demand, when explicitly asked.
//! That module is explicitly protected (PROJECT_STATE/PROTECTED_AREAS.md)
//! and this task must not touch it - so this module reimplements the same
//! small, hard-won pattern (`WebviewWindowBuilder` built off a plain OS
//! thread - building on the command's own calling thread deadlocks on
//! Windows, the same constraint `price_checker_scanner.rs` already hit -
//! plus `eval_with_callback` with a bounded timeout) against its OWN state
//! (`db::LiveIntelSession`, keyed in `AppState::live_intel_sessions`)
//! rather than sharing `price_scanner_sessions`/`ScannerSession`. The two
//! features can never affect each other, and a change to one's session
//! handling can never silently change the other's.
//!
//! What this module reads off a page is deliberately far smaller than the
//! Scanner's own extraction: only `document.title` + `location.href` -
//! never prices, never listing counts, never anything selector-based.
//! Nothing here can break when a marketplace changes its page layout, and
//! nothing here reads as scraping structured data at scale.
//!
//! ## Flow (marko's own spec)
//! Event Workspace -> "Find Online Event" -> pick one of the 3 sources ->
//! a normal, VISIBLE window opens on a best-effort search URL for that
//! marketplace, built on the FRONTEND from the event's own name/city (this
//! module never constructs or knows about search URLs - it only ever opens
//! whatever URL it's given, exactly like `open_price_scanner`) -> marko
//! searches/navigates himself, exactly like any normal browser tab ->
//! "Capture this page" (`capture_live_event_page`) reads the CURRENTLY
//! loaded page's title + URL, once, and hands it back to the frontend as
//! one candidate -> marko can capture again after navigating, to try
//! another candidate -> "Use this one" on a candidate calls
//! `save_confirmed_online_source`, the only function that ever writes
//! `verified = true`.
//!
//! "Refresh" on an already-connected source is the EXACT same
//! window+capture+confirm sequence, just opened at the already-saved url
//! instead of a fresh search - see `save_confirmed_online_source_impl`'s
//! own doc comment for why one function deliberately covers both. This
//! also means a manually-connected (`verified = false`) source becomes
//! verified the first time marko runs a successful Refresh on it and
//! confirms what he sees - no separate "mark as verified" action needed.
//!
//! "Connect manually" skips the window entirely - marko pastes a URL he
//! already has, saved via `connect_online_source_manually` with
//! `verified = false` (the app never looked at the page, so it cannot
//! confirm anything itself - only a later Refresh can).
//!
//! ## What's deliberately NOT here (same safety envelope as
//! `price_checker_scanner.rs`, and the same one this project applied when
//! marko separately asked about an auto-listing bot for Ticketmaster this
//! same release cycle and was told no)
//! No CAPTCHA/anti-bot bypass, no automated form-fill/submit anywhere, no
//! headless/backend HTTP requests to any of the 3 marketplaces at all - the
//! ONLY networking this module ever does is opening a real, visible browser
//! window that a human drives, exactly like `price_checker_scanner.rs`. No
//! automatic candidate selection and no automatic event creation - every
//! save requires an explicit user action naming a specific, already-
//! existing `event_id`; an unclear/no capture simply means nothing is
//! saved, never a best-guess. No pricing logic of any kind - Section/Row/
//! Seat are never referenced, and nothing in `commands::price_checker` or
//! `commands::price_checker_scanner` ever reads this module's table.
//!
//! ## Everything actually persisted lives in `event_online_sources`
//! See `migrations/026_live_event_intelligence.sql` for the full column-by-
//! column reasoning - in particular why this is a standalone table (not a
//! new column on `events`, and not a foreign key onto the existing,
//! marko-managed `marketplaces` table Price Checker/Listings share).

use crate::db::{AppState, LiveIntelSession};
use crate::error::{AppError, AppResult};
use crate::models::{
    EventOnlineSource, EventOnlineSourceActiveInput, EventOnlineSourceConfirmInput, EventOnlineSourceManualInput,
    LiveIntelCapturePayload, LiveIntelWindowClosedPayload, LiveIntelWindowErrorPayload, LiveIntelWindowOpenedPayload,
};
use rusqlite::{params, Connection, Row};
use std::sync::mpsc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};

/// The exact 3 marketplaces this feature ever supports - matches
/// `migrations/026_live_event_intelligence.sql`'s CHECK constraint exactly.
/// Marko's own explicit "Podporuj LEN tieto 3... NEPRIDÁVAJ StubHub,
/// Seatriks ani ine". Adding a real 4th source later means extending BOTH
/// this list AND the migration's CHECK constraint (a new forward-only
/// migration) together, plus one new frontend search-URL entry - no rework
/// of the other 3 sources' own code or data either way. Checked by
/// `validate_source` below so a caller can never write an unsupported value
/// through this module even though the DB's own CHECK constraint would
/// catch it too (defense in depth, same style as `require_marketplace_
/// active` in commands::price_checker.rs).
const SUPPORTED_SOURCES: &[&str] = &["viagogo", "vivid_seats", "ticombo"];

fn validate_source(source: &str) -> AppResult<()> {
    if SUPPORTED_SOURCES.contains(&source) {
        Ok(())
    } else {
        Err(AppError::Validation(format!(
            "Unsupported source '{source}' - Live Event Intelligence only supports {}",
            SUPPORTED_SOURCES.join(", ")
        )))
    }
}

const EVENT_WINDOW_OPENED: &str = "live-intel-window-opened";
const EVENT_WINDOW_ERROR: &str = "live-intel-window-error";
const EVENT_CAPTURE_RESULT: &str = "live-intel-capture-result";
const EVENT_WINDOW_CLOSED: &str = "live-intel-window-closed";

/// Same bounded-wait budget as `price_checker_scanner::SCAN_EVAL_TIMEOUT`,
/// for the same reason: reading the CURRENTLY VISIBLE page's title/URL
/// should be near-instant (no network wait involved - the page is already
/// loaded), so this is generous headroom, not a retry budget. A single
/// attempt, no retry loop, matching this project's established "no
/// autonomous retry" rule for anything touching these 3 marketplaces - if
/// this trips, marko just clicks the capture button again himself.
const CAPTURE_EVAL_TIMEOUT: Duration = Duration::from_secs(10);

/// Reads `document.title` + `location.href` of whatever's CURRENTLY
/// loaded - nothing else, no DOM scan, no selectors. Deliberately a plain
/// JS string, not an object it JSON.stringifies itself:
/// `eval_with_callback` JSON-encodes the script's own completion value
/// exactly once before handing it to the Rust callback, so returning a
/// plain string here (rather than an object, which would need a SECOND
/// decode - see `price_checker_scanner.rs`'s own `parse_scan_js_payload`
/// doc comment for that gotcha) keeps `parse_capture_result` below to a
/// single decode. `String.fromCharCode(31)` (the ASCII "unit separator")
/// joins the two fields - the same separator convention `models::
/// SeatEntry::parse_aggregate` already uses, chosen because a real page
/// title or URL containing it is not a realistic concern.
const CAPTURE_SCRIPT: &str = "document.title + String.fromCharCode(31) + location.href";

fn window_label_for(request_id: u64) -> String {
    format!("live-intel-{request_id}")
}

/// Same guard as `price_checker_scanner::parse_scanner_url` - only a plain
/// http(s) link is ever handed to `WebviewWindowBuilder`.
fn parse_intel_url(raw: &str) -> AppResult<tauri::Url> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation("No URL to open".into()));
    }
    let url = tauri::Url::parse(trimmed).map_err(|_| AppError::Validation("That doesn't look like a valid URL".into()))?;
    if url.scheme() != "http" && url.scheme() != "https" {
        return Err(AppError::Validation("Only http:// or https:// links can be opened".into()));
    }
    Ok(url)
}

/// Unwraps `eval_with_callback`'s single JSON-encoding layer (see
/// `CAPTURE_SCRIPT`'s own doc comment) and splits the two fields back apart.
fn parse_capture_result(raw: &str) -> Result<(String, String), String> {
    let decoded: String = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    decoded
        .split_once('\u{1f}')
        .map(|(title, url)| (title.to_string(), url.to_string()))
        .ok_or_else(|| "Unexpected response from the page".to_string())
}

fn finish_session(app: &AppHandle, request_id: u64) {
    let removed = match app.try_state::<AppState>() {
        Some(st) => st.live_intel_sessions.lock().unwrap().remove(&request_id),
        None => None,
    };
    if removed.is_some() {
        let _ = app.emit(EVENT_WINDOW_CLOSED, LiveIntelWindowClosedPayload { request_id });
    }
}

fn emit_window_error(app: &AppHandle, request_id: u64, message: &str) {
    let _ = app.emit(EVENT_WINDOW_ERROR, LiveIntelWindowErrorPayload { request_id, message: message.to_string() });
}

// ---------------------------------------------------------------------------
// Visible window session commands - "Find Online Event" and "Refresh" both
// go through these 3; see this module's own doc comment for the full flow.
// None of these are `async` - exactly like their Price Checker Scanner
// counterparts, the window build/eval happens on a spawned plain OS thread,
// so the command function itself returns almost immediately and never
// blocks Tauri's single IPC thread. This is what makes "a network hiccup
// must never freeze the app" true here without needing `spawn_blocking`.
// ---------------------------------------------------------------------------

/// Opens a real, visible browser window at `url` - a freshly constructed
/// search-results page for "Find Online Event", or the already-saved
/// source url for "Refresh". The real outcome arrives via
/// `live-intel-window-opened`/`live-intel-window-error`.
#[tauri::command]
pub fn open_live_event_window(app: AppHandle, state: State<AppState>, request_id: u64, event_id: i64, source: String, url: String) -> AppResult<()> {
    validate_source(&source)?;
    let parsed_url = parse_intel_url(&url)?;
    let label = window_label_for(request_id);

    {
        let mut sessions = state.live_intel_sessions.lock().unwrap();
        if sessions.contains_key(&request_id) {
            return Err(AppError::Validation("A window with this id is already open".into()));
        }
        // Same guard as price_checker_scanner::insert_new_session: refuses a
        // SECOND window for the same (event, source) pair rather than
        // silently allowing two windows the UI has no way to tell apart -
        // close the existing one first.
        if sessions.values().any(|s| s.event_id == event_id && s.source == source) {
            return Err(AppError::Validation(
                "A window is already open for this marketplace on this event - close it before opening another.".into(),
            ));
        }
        sessions.insert(request_id, LiveIntelSession { window_label: label.clone(), event_id, source });
    }

    let handle = app.clone();
    std::thread::spawn(move || {
        let build_result = tauri::WebviewWindowBuilder::new(&handle, &label, tauri::WebviewUrl::External(parsed_url))
            .title("TIQR Manager - Live Event Intelligence")
            .inner_size(1280.0, 900.0)
            .visible(true)
            .build();

        match build_result {
            Ok(window) => {
                let handle_for_close = handle.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { .. } = event {
                        // marko closed the window himself (native close
                        // button) - same cleanup either way, see
                        // finish_session's own doc comment above.
                        finish_session(&handle_for_close, request_id);
                    }
                });
                let _ = handle.emit(EVENT_WINDOW_OPENED, LiveIntelWindowOpenedPayload { request_id });
            }
            Err(e) => {
                if let Some(st) = handle.try_state::<AppState>() {
                    st.live_intel_sessions.lock().unwrap().remove(&request_id);
                }
                emit_window_error(&handle, request_id, &format!("Could not open the window: {e}"));
            }
        }
    });

    Ok(())
}

/// Reads whatever's CURRENTLY rendered in the window - title + URL, once.
/// Same "returns fast, the real result follows as an event" contract as
/// `price_checker_scanner::scan_visible_prices`.
#[tauri::command]
pub fn capture_live_event_page(app: AppHandle, state: State<AppState>, request_id: u64) -> AppResult<()> {
    let window_label = {
        let sessions = state.live_intel_sessions.lock().unwrap();
        sessions
            .get(&request_id)
            .ok_or_else(|| AppError::NotFound("Window session not found - it may have been closed".into()))?
            .window_label
            .clone()
    };

    let handle = app.clone();
    std::thread::spawn(move || {
        let window = match handle.get_webview_window(&window_label) {
            Some(w) => w,
            None => {
                emit_window_error(&handle, request_id, "The window is no longer open.");
                return;
            }
        };

        let (tx, rx) = mpsc::channel::<String>();
        if let Err(e) = window.eval_with_callback(CAPTURE_SCRIPT, move |result: String| {
            let _ = tx.send(result);
        }) {
            emit_window_error(&handle, request_id, &format!("Could not read the page: {e}"));
            return;
        }

        let raw = match rx.recv_timeout(CAPTURE_EVAL_TIMEOUT) {
            Ok(r) => r,
            Err(_) => {
                emit_window_error(&handle, request_id, "The page didn't respond in time - try again.");
                return;
            }
        };

        match parse_capture_result(&raw) {
            Ok((title, url)) => {
                let _ = handle.emit(EVENT_CAPTURE_RESULT, LiveIntelCapturePayload { request_id, title, url });
            }
            Err(e) => emit_window_error(&handle, request_id, &format!("Got an unreadable response from the page: {e}")),
        }
    });

    Ok(())
}

/// Ends a window session - "Close" in the UI. `close_window: false` just
/// forgets the session's bookkeeping (the browser window stays open,
/// marko's choice); `true` also closes it. Idempotent - closing an
/// already-gone session is a harmless no-op, same as `close_price_scanner`.
#[tauri::command]
pub fn close_live_event_window(app: AppHandle, state: State<AppState>, request_id: u64, close_window: bool) -> AppResult<()> {
    let window_label = {
        let sessions = state.live_intel_sessions.lock().unwrap();
        sessions.get(&request_id).map(|s| s.window_label.clone())
    };
    if close_window {
        if let Some(label) = &window_label {
            if let Some(window) = app.get_webview_window(label) {
                let _ = window.close();
            }
        }
    }
    finish_session(&app, request_id);
    Ok(())
}

// ---------------------------------------------------------------------------
// event_online_sources - the real, persisted data. Plain, fast, local DB
// commands - no network involved in any of these at all (the 3 window
// commands above are the ONLY thing in this module that ever touches the
// network, and even they never block the IPC thread - see their own
// section comment).
// ---------------------------------------------------------------------------

fn map_source(row: &Row) -> rusqlite::Result<EventOnlineSource> {
    Ok(EventOnlineSource {
        id: row.get("id")?,
        event_id: row.get("event_id")?,
        source: row.get("source")?,
        url: row.get("url")?,
        external_event_id: row.get("external_event_id")?,
        verified: row.get("verified")?,
        active: row.get("active")?,
        last_checked_at: row.get("last_checked_at")?,
        last_checked_title: row.get("last_checked_title")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

/// Every source ever saved for this event, regardless of `active` - the UI
/// decides how to group/show active vs. disconnected ones; nothing here is
/// ever silently hidden. Ordered by `source` for a stable, predictable list
/// (there are only ever 3 possible values, so this also happens to sort
/// them alphabetically: ticombo, viagogo, vivid_seats).
pub(crate) fn list_event_online_sources_impl(conn: &Connection, event_id: i64) -> AppResult<Vec<EventOnlineSource>> {
    let mut stmt = conn.prepare("SELECT * FROM event_online_sources WHERE event_id = ?1 ORDER BY source")?;
    let rows = stmt.query_map(params![event_id], map_source)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[tauri::command]
pub fn list_event_online_sources(state: State<AppState>, event_id: i64) -> AppResult<Vec<EventOnlineSource>> {
    let conn = state.db.lock().unwrap();
    list_event_online_sources_impl(&conn, event_id)
}

fn fetch_source(conn: &Connection, event_id: i64, source: &str) -> AppResult<EventOnlineSource> {
    Ok(conn.query_row(
        "SELECT * FROM event_online_sources WHERE event_id = ?1 AND source = ?2",
        params![event_id, source],
        map_source,
    )?)
}

/// "Connect manually" - always saves `verified = false` (see
/// `EventOnlineSource::verified`'s own doc comment - the app never looked
/// at this page) and `active = true`. Upserts on (event_id, source), same
/// "re-saving the same marketplace updates it in place, never duplicates"
/// convention as `price_checker::save_event_marketplace_link_impl` -
/// re-connecting manually always resets `verified` back to false, even if
/// the previous row happened to be verified, since a new/edited URL has not
/// itself been looked at yet.
pub(crate) fn connect_online_source_manually_impl(conn: &Connection, input: &EventOnlineSourceManualInput) -> AppResult<EventOnlineSource> {
    validate_source(&input.source)?;
    let url = input.url.trim();
    if url.is_empty() {
        return Err(AppError::Validation("Enter a URL first".into()));
    }
    let external_id = input.external_event_id.as_deref().map(str::trim).filter(|s| !s.is_empty());
    conn.execute(
        "INSERT INTO event_online_sources(event_id, source, url, external_event_id, verified, active)
         VALUES (?1, ?2, ?3, ?4, 0, 1)
         ON CONFLICT(event_id, source) DO UPDATE SET
           url = excluded.url,
           external_event_id = excluded.external_event_id,
           verified = 0,
           active = 1,
           updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')",
        params![input.event_id, input.source, url, external_id],
    )?;
    fetch_source(conn, input.event_id, &input.source)
}

#[tauri::command]
pub fn connect_online_source_manually(state: State<AppState>, input: EventOnlineSourceManualInput) -> AppResult<EventOnlineSource> {
    let conn = state.db.lock().unwrap();
    connect_online_source_manually_impl(&conn, &input)
}

/// The ONE function that ever writes `verified = true` - called after a
/// human has looked at `url` in the real visible window and explicitly
/// confirmed it (either a "Find Online Event" candidate, or a "Refresh" on
/// an already-saved source - deliberately the same function for both, since
/// both are "marko just looked at this exact page and says it's right").
/// Always also sets `active = true` and stamps `last_checked_at`/
/// `last_checked_title` to right now - the app just looked at this exact
/// page a moment ago, so "last checked" genuinely means now, not a guess.
/// Upserts on (event_id, source): confirming a NEW candidate for a source
/// that already has a (possibly unverified, possibly different-url) row
/// replaces it rather than creating a duplicate, matching marko's
/// "marketplace najviac raz na event" (at most once per event) rule.
pub(crate) fn save_confirmed_online_source_impl(conn: &Connection, input: &EventOnlineSourceConfirmInput) -> AppResult<EventOnlineSource> {
    validate_source(&input.source)?;
    let url = input.url.trim();
    if url.is_empty() {
        return Err(AppError::Validation("No URL to confirm".into()));
    }
    conn.execute(
        "INSERT INTO event_online_sources(event_id, source, url, verified, active, last_checked_at, last_checked_title)
         VALUES (?1, ?2, ?3, 1, 1, strftime('%Y-%m-%dT%H:%M:%fZ','now'), ?4)
         ON CONFLICT(event_id, source) DO UPDATE SET
           url = excluded.url,
           verified = 1,
           active = 1,
           last_checked_at = excluded.last_checked_at,
           last_checked_title = excluded.last_checked_title,
           updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')",
        params![input.event_id, input.source, url, input.title],
    )?;
    fetch_source(conn, input.event_id, &input.source)
}

#[tauri::command]
pub fn save_confirmed_online_source(state: State<AppState>, input: EventOnlineSourceConfirmInput) -> AppResult<EventOnlineSource> {
    let conn = state.db.lock().unwrap();
    save_confirmed_online_source_impl(&conn, &input)
}

/// "Disconnect"/"Reconnect" - a soft flag flip, same convention as
/// `Marketplace::active`/`TicketListing::status == "removed"`. Never
/// deletes the row and never touches `verified`/`last_checked_*`/history.
/// Errors if this event has no saved row for that source yet, rather than
/// silently creating a placeholder one - there is nothing sensible to
/// disconnect/reconnect if nothing was ever connected.
pub(crate) fn set_online_source_active_impl(conn: &Connection, input: &EventOnlineSourceActiveInput) -> AppResult<EventOnlineSource> {
    validate_source(&input.source)?;
    let changed = conn.execute(
        "UPDATE event_online_sources SET active = ?1, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
         WHERE event_id = ?2 AND source = ?3",
        params![input.active, input.event_id, input.source],
    )?;
    if changed == 0 {
        return Err(AppError::NotFound("This event has no saved source for that marketplace yet".into()));
    }
    fetch_source(conn, input.event_id, &input.source)
}

#[tauri::command]
pub fn set_online_source_active(state: State<AppState>, input: EventOnlineSourceActiveInput) -> AppResult<EventOnlineSource> {
    let conn = state.db.lock().unwrap();
    set_online_source_active_impl(&conn, &input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_conn;
    use crate::models::{EventOnlineSourceActiveInput, EventOnlineSourceConfirmInput, EventOnlineSourceManualInput};

    fn seed_event(conn: &Connection, name: &str) -> i64 {
        conn.execute("INSERT INTO events (name) VALUES (?1)", [name]).unwrap();
        conn.last_insert_rowid()
    }

    // -- no sources / empty state ---------------------------------------

    #[test]
    fn a_fresh_event_has_no_online_sources() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event");

        let sources = list_event_online_sources_impl(&conn, event_id).unwrap();

        assert!(sources.is_empty(), "a brand-new event must start with zero online sources, never an invented one");
    }

    // -- manual connect ----------------------------------------------------

    #[test]
    fn connect_manually_saves_unverified_and_active() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event");

        let saved = connect_online_source_manually_impl(
            &conn,
            &EventOnlineSourceManualInput { event_id, source: "viagogo".into(), url: "https://www.viagogo.com/x".into(), external_event_id: None },
        )
        .unwrap();

        assert_eq!(saved.source, "viagogo");
        assert_eq!(saved.url, "https://www.viagogo.com/x");
        assert!(!saved.verified, "a manually-typed URL must never be saved as verified - the app never looked at it");
        assert!(saved.active, "a fresh connect must be active");
        assert!(saved.last_checked_at.is_none(), "manual connect must never invent a last-checked timestamp");
        assert!(saved.external_event_id.is_none());
    }

    #[test]
    fn connect_manually_rejects_an_unsupported_source() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event");

        // marko's own explicit "NEPRIDÁVAJ StubHub, Seatriks ani ine" -
        // this must stay refused even though both are real, active rows in
        // the general `marketplaces` table.
        let stubhub = connect_online_source_manually_impl(
            &conn,
            &EventOnlineSourceManualInput { event_id, source: "stubhub".into(), url: "https://stubhub.com/x".into(), external_event_id: None },
        );
        let seatriks = connect_online_source_manually_impl(
            &conn,
            &EventOnlineSourceManualInput { event_id, source: "seatriks".into(), url: "https://seatriks.com/x".into(), external_event_id: None },
        );

        assert!(stubhub.is_err(), "StubHub must never be a valid Live Event Intelligence source");
        assert!(seatriks.is_err(), "Seatriks must never be a valid Live Event Intelligence source");
        assert!(list_event_online_sources_impl(&conn, event_id).unwrap().is_empty(), "a refused connect must not have saved anything");
    }

    #[test]
    fn connect_manually_rejects_a_blank_url() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event");

        let result = connect_online_source_manually_impl(
            &conn,
            &EventOnlineSourceManualInput { event_id, source: "ticombo".into(), url: "   ".into(), external_event_id: None },
        );

        assert!(result.is_err(), "a blank URL must be refused, never saved as an empty/placeholder row");
    }

    #[test]
    fn connect_manually_with_an_external_id_saves_it_trimmed() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event");

        let saved = connect_online_source_manually_impl(
            &conn,
            &EventOnlineSourceManualInput {
                event_id,
                source: "ticombo".into(),
                url: "https://www.ticombo.com/x".into(),
                external_event_id: Some("  E-12345  ".into()),
            },
        )
        .unwrap();

        assert_eq!(saved.external_event_id, Some("E-12345".to_string()));
    }

    // -- duplicate-marketplace prevention / upsert ---------------------------

    #[test]
    fn reconnecting_the_same_source_updates_in_place_rather_than_duplicating() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event");

        connect_online_source_manually_impl(
            &conn,
            &EventOnlineSourceManualInput { event_id, source: "viagogo".into(), url: "https://www.viagogo.com/old".into(), external_event_id: None },
        )
        .unwrap();
        let second = connect_online_source_manually_impl(
            &conn,
            &EventOnlineSourceManualInput { event_id, source: "viagogo".into(), url: "https://www.viagogo.com/new".into(), external_event_id: None },
        )
        .unwrap();

        let all = list_event_online_sources_impl(&conn, event_id).unwrap();
        assert_eq!(all.len(), 1, "the same (event, source) pair must never produce two rows");
        assert_eq!(second.url, "https://www.viagogo.com/new");
    }

    #[test]
    fn confirming_a_new_candidate_resets_a_previously_saved_urls_verification_correctly() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event");

        save_confirmed_online_source_impl(
            &conn,
            &EventOnlineSourceConfirmInput { event_id, source: "viagogo".into(), url: "https://www.viagogo.com/first-pick".into(), title: Some("First".into()) },
        )
        .unwrap();
        let updated = save_confirmed_online_source_impl(
            &conn,
            &EventOnlineSourceConfirmInput { event_id, source: "viagogo".into(), url: "https://www.viagogo.com/better-pick".into(), title: Some("Better".into()) },
        )
        .unwrap();

        let all = list_event_online_sources_impl(&conn, event_id).unwrap();
        assert_eq!(all.len(), 1, "picking a different candidate for the same source must replace it, not add a second row");
        assert_eq!(updated.url, "https://www.viagogo.com/better-pick");
        assert!(updated.verified);
    }

    // -- multiple sources per event -----------------------------------------

    #[test]
    fn one_event_can_have_all_3_sources_connected_at_once() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event");

        for source in SUPPORTED_SOURCES {
            connect_online_source_manually_impl(
                &conn,
                &EventOnlineSourceManualInput { event_id, source: source.to_string(), url: format!("https://example.com/{source}"), external_event_id: None },
            )
            .unwrap();
        }

        let all = list_event_online_sources_impl(&conn, event_id).unwrap();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn one_events_sources_never_leak_into_a_different_event() {
        let conn = test_conn();
        let event_a = seed_event(&conn, "Event A");
        let event_b = seed_event(&conn, "Event B");
        connect_online_source_manually_impl(
            &conn,
            &EventOnlineSourceManualInput { event_id: event_a, source: "viagogo".into(), url: "https://www.viagogo.com/a".into(), external_event_id: None },
        )
        .unwrap();

        assert_eq!(list_event_online_sources_impl(&conn, event_a).unwrap().len(), 1);
        assert!(list_event_online_sources_impl(&conn, event_b).unwrap().is_empty(), "Event B must not see Event A's source");
    }

    // -- discovery confirmation / verified state -----------------------------

    #[test]
    fn confirming_a_captured_candidate_marks_it_verified_with_a_checked_timestamp() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event");

        let saved = save_confirmed_online_source_impl(
            &conn,
            &EventOnlineSourceConfirmInput { event_id, source: "vivid_seats".into(), url: "https://www.vividseats.com/real-event".into(), title: Some("Real Event - Vivid Seats".into()) },
        )
        .unwrap();

        assert!(saved.verified, "an explicitly confirmed candidate must be saved as verified");
        assert!(saved.active);
        assert!(saved.last_checked_at.is_some(), "confirming a live capture must stamp last_checked_at to now");
        assert_eq!(saved.last_checked_title, Some("Real Event - Vivid Seats".to_string()));
    }

    #[test]
    fn an_unconfirmed_manual_entry_stays_unverified_until_a_refresh_confirms_it() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event");

        let manual = connect_online_source_manually_impl(
            &conn,
            &EventOnlineSourceManualInput { event_id, source: "ticombo".into(), url: "https://www.ticombo.com/x".into(), external_event_id: None },
        )
        .unwrap();
        assert!(!manual.verified, "must stay unverified right after a manual connect");

        // "Refresh" -> open the saved url in the visible window -> capture
        // -> confirm: this is the SAME function a "Find Online Event"
        // confirmation uses. See save_confirmed_online_source_impl's own
        // doc comment for why one function covers both flows.
        let refreshed = save_confirmed_online_source_impl(
            &conn,
            &EventOnlineSourceConfirmInput { event_id, source: "ticombo".into(), url: manual.url.clone(), title: Some("Ticombo Event Page".into()) },
        )
        .unwrap();

        assert!(refreshed.verified, "a successful Refresh-and-confirm must flip a manual entry to verified");
        assert_eq!(refreshed.url, manual.url);
    }

    #[test]
    fn refreshing_an_already_verified_source_updates_last_checked_without_losing_verification() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event");
        let first = save_confirmed_online_source_impl(
            &conn,
            &EventOnlineSourceConfirmInput { event_id, source: "viagogo".into(), url: "https://www.viagogo.com/x".into(), title: Some("Old Title".into()) },
        )
        .unwrap();

        let refreshed = save_confirmed_online_source_impl(
            &conn,
            &EventOnlineSourceConfirmInput { event_id, source: "viagogo".into(), url: "https://www.viagogo.com/x".into(), title: Some("Still The Right Event".into()) },
        )
        .unwrap();

        assert!(refreshed.verified);
        assert_eq!(refreshed.last_checked_title, Some("Still The Right Event".to_string()));
        assert_ne!(first.last_checked_title, refreshed.last_checked_title, "sanity: the refresh really did update the captured title");
    }

    // -- active/inactive (disconnect/reconnect) ------------------------------

    #[test]
    fn disconnect_then_reconnect_preserves_verified_state_and_history() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event");
        save_confirmed_online_source_impl(
            &conn,
            &EventOnlineSourceConfirmInput { event_id, source: "viagogo".into(), url: "https://www.viagogo.com/x".into(), title: Some("T".into()) },
        )
        .unwrap();

        let disconnected =
            set_online_source_active_impl(&conn, &EventOnlineSourceActiveInput { event_id, source: "viagogo".into(), active: false }).unwrap();
        assert!(!disconnected.active);
        assert!(disconnected.verified, "disconnecting must never clear verified");

        let reconnected =
            set_online_source_active_impl(&conn, &EventOnlineSourceActiveInput { event_id, source: "viagogo".into(), active: true }).unwrap();
        assert!(reconnected.active);
        assert!(reconnected.verified);
        assert_eq!(reconnected.url, "https://www.viagogo.com/x", "reconnecting must not lose the saved url");
    }

    #[test]
    fn disconnecting_a_source_that_was_never_connected_is_an_error_not_a_silent_create() {
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event");

        let result = set_online_source_active_impl(&conn, &EventOnlineSourceActiveInput { event_id, source: "ticombo".into(), active: false });

        assert!(result.is_err());
        assert!(list_event_online_sources_impl(&conn, event_id).unwrap().is_empty(), "must not have created a placeholder row");
    }

    // -- offline / cache-only usage -------------------------------------------

    #[test]
    fn saved_sources_are_listable_with_zero_network_involved() {
        // Every function this test calls is a plain local DB read/write -
        // this stands in for marko's own "app musí fungovať plne offline s
        // poslednymi ulozenymi datami" (the app must keep working fully
        // offline using last-saved data): nothing about listing or reading
        // previously-confirmed sources ever requires a live connection.
        let conn = test_conn();
        let event_id = seed_event(&conn, "Test Event");
        save_confirmed_online_source_impl(
            &conn,
            &EventOnlineSourceConfirmInput { event_id, source: "viagogo".into(), url: "https://www.viagogo.com/x".into(), title: Some("T".into()) },
        )
        .unwrap();

        let sources = list_event_online_sources_impl(&conn, event_id).unwrap();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].url, "https://www.viagogo.com/x");
    }

    // -- capture-result parsing (the runtime window/eval/timeout behavior
    // itself needs a real WebviewWindow to exercise, same documented
    // limitation as price_checker_scanner.rs's own module comment - "not
    // unit-testable alone" - but the parsing/error-shape logic downstream
    // of a timeout or a malformed eval result is exactly what these test.) -

    #[test]
    fn parse_capture_result_splits_title_and_url_correctly() {
        let raw = serde_json::to_string("My Event Page\u{1f}https://example.com/e/123").unwrap();

        let (title, url) = parse_capture_result(&raw).unwrap();

        assert_eq!(title, "My Event Page");
        assert_eq!(url, "https://example.com/e/123");
    }

    #[test]
    fn parse_capture_result_rejects_a_missing_separator_instead_of_guessing() {
        let raw = serde_json::to_string("no separator here at all").unwrap();

        assert!(parse_capture_result(&raw).is_err());
    }

    #[test]
    fn parse_capture_result_rejects_malformed_json_with_a_message_not_a_panic() {
        let result = parse_capture_result("{ not valid json");

        assert!(result.is_err());
    }

    #[test]
    fn parse_intel_url_rejects_blank_and_non_http_schemes() {
        assert!(parse_intel_url("").is_err());
        assert!(parse_intel_url("   ").is_err());
        assert!(parse_intel_url("javascript:alert(1)").is_err());
        assert!(parse_intel_url("file:///etc/passwd").is_err());
        assert!(parse_intel_url("https://www.viagogo.com/Search?q=x").is_ok());
        assert!(parse_intel_url("http://example.com").is_ok());
    }
}

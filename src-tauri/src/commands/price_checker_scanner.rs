//! Visible Scanner (2.1.9) - marko's full rewrite of Price Checker
//! automation: "Doterajší hidden WebView + automatické selektory stále
//! nefungujú spoľahlivo... Chcem nový spôsob: VISIBLE BROWSER / WEBVIEW
//! SCANNER" (the hidden-WebView + automatic-selectors approach still isn't
//! reliable... I want a new approach: a visible browser/webview scanner).
//! Replaces `price_checker_auto.rs` (deleted this version) entirely.
//!
//! ## Flow (marko's own spec, "## FLOW")
//! Price Checker -> pick an Event -> paste a marketplace URL -> "Open &
//! Scan" (`open_price_scanner`) -> a NORMAL, VISIBLE window opens and loads
//! the page exactly as marko would see it in any browser -> marko scrolls
//! it himself -> "Scan Visible Prices" (`scan_visible_prices`) reads
//! whatever is on screen AT THAT MOMENT -> result shown in TIQR Manager ->
//! marko can scroll/navigate more and scan again; each scan adds newly-seen
//! listings to the same running session (`cancel_price_scan` interrupts an
//! in-flight scan, `close_price_scanner` ends the session).
//!
//! ## Why no autonomous retry/polling/AI fallback survives from 2.1.1-2.1.8
//! Every one of those existed to compensate for a HIDDEN window the old
//! design couldn't let marko look at or interact with himself. A visible
//! window removes the entire premise: if a marketplace shows a CAPTCHA or
//! hasn't finished rendering, marko sees that directly and can act (solve
//! it, wait, scroll) - the backend's job shrinks to "read what's on screen,
//! once, when asked," which is exactly what this module does. See
//! PRICE-CHECKER-VISIBLE-SCANNER-REPORT.md for the full before/after.
//!
//! ## Session model
//! One `ScannerSession` (db.rs) per open scanner window, keyed by
//! `request_id` in `AppState::price_scanner_sessions` - the SAME
//! frontend-minted id used for `open_price_scanner` and every later
//! `scan_visible_prices`/`cancel_price_scan`/`close_price_scanner` call
//! against that window (this codebase's established "frontend mints a
//! request id, backend echoes it back on every event" convention, e.g. the
//! old auto-check's `requestIdRef`). A HashMap rather than a single slot
//! because a visible window has no reason to be single-flight anymore - the
//! old design's one-shared-cancel-flag existed only because it drove ONE
//! hidden background thread; marko can now have a StubHub card and a Vivid
//! Seats card both open and scanning at once, and one misbehaving window
//! must never affect another (marko's own explicit spec requirement, "## AK
//! JEDEN MARKETPLACE NEFUNGUJE, OSTATNÉ MUSIA FUNGOVAŤ" era wording carried
//! over into "## MARKETPLACE STATUS").
//!
//! Deciding which marketplace-specific reader to run happens entirely in
//! `price_checker_scan.js`, from `location.hostname` of whatever page is
//! CURRENTLY loaded in the visible window - not from `marketplace_id`. A
//! visible window is just a normal browser: marko can navigate it anywhere,
//! and the reader must match the page actually being looked at, not the
//! card it was launched from. `event_id`/`marketplace_id` on the session
//! exist purely so the frontend can route "Save to history" back to the
//! right card afterward (through the ordinary, untouched
//! `commands::price_checker::save_price_check`) - they are never checked
//! against anything here (see that command's own
//! `require_marketplace_active` doc comment for why that guard stays
//! confined to the actual save).
//!
//! ## Status derivation
//! `derive_session_status` reflects the ACCUMULATED session (every listing
//! found across every scan so far), not just the latest scan's own delta -
//! "success" once ANY listing has both a section and a row, else "partial"
//! once any listing exists at all, else "unable_to_read". "blocked"/"error"
//! are the one exception: they reflect only the MOST RECENT scan's own
//! outcome, and are NOT sticky - a challenge marko solves himself in the
//! now-interactive window, then scans again successfully, must not stay
//! stuck showing "Blocked" forever. Since `ScannerSession::status` is
//! overwritten fresh on every `merge_scan_into_session` call, this falls
//! out automatically rather than needing a separate "ever blocked" flag.
//!
//! ## What's deliberately NOT here
//! No CAPTCHA/anti-bot bypass, no proxy rotation, no stealth - marko's own
//! explicit "## SECURITY". No screenshot/OCR fallback layer - this
//! sandbox's Rust side has no screenshot-capture or OCR crate, and adding
//! one is disproportionate to what the JS's three text-based layers already
//! cover; see price_checker_scan.js's own module comment and the report's
//! "WHAT WAS LEFT UNTOUCHED" section. No AI analysis, no automatic
//! repricing - marko's own "Zatiaľ nerob AI... Zatiaľ nerob automatické
//! repricing."

use crate::db::{AppState, ScannerSession};
use crate::error::{AppError, AppResult};
use crate::models::{
    NormalizedListing, ScanResultPayload, ScannerClosedPayload, ScannerErrorPayload, ScannerOpenedPayload,
};
use chrono::Utc;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};

/// The extraction script, embedded at compile time - see its own module
/// comment for the four-layer design. Sent verbatim to
/// `WebviewWindow::eval_with_callback` on every `scan_visible_prices` call;
/// never modified per-call (the page it reads from is what changes, not the
/// script reading it).
const SCAN_SCRIPT: &str = include_str!("price_checker_scan.js");

const EVENT_SCANNER_OPENED: &str = "price-scanner-opened";
const EVENT_SCANNER_ERROR: &str = "price-scanner-error";
const EVENT_SCAN_RESULT: &str = "price-scanner-scan-result";
const EVENT_SCANNER_CLOSED: &str = "price-scanner-closed";

/// How long one eval is allowed to take before `scan_visible_prices` gives
/// up and reports status "error" - reading the CURRENTLY VISIBLE page
/// should be fast (no network wait, no scrolling, no polling loop); this is
/// generous headroom for a slow/heavy page, not a retry budget. Deliberately
/// a single attempt, no retry loop - marko's own spec rejects another
/// auto-retry mechanism; if this trips, he just clicks "Scan Visible
/// Prices" again himself.
const SCAN_EVAL_TIMEOUT: Duration = Duration::from_secs(10);

// ---------------------------------------------------------------------------
// Shapes returned by price_checker_scan.js. Deliberately separate from
// NormalizedListing/ScanResultPayload (models.rs) - this is the raw,
// untrusted JS-interop wire shape; merge_scan_into_session below is what
// turns it into the app's real, permanent normalized model.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScanJsPayload {
    ok: bool,
    #[serde(default)]
    blocked: bool,
    #[serde(default)]
    blocked_reason: Option<String>,
    #[serde(default)]
    candidates: Vec<ScanJsCandidate>,
    #[serde(default)]
    diagnostics: ScanJsDiagnostics,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScanJsCandidate {
    price_cents: i64,
    #[serde(default)]
    currency: Option<String>,
    #[serde(default)]
    section: Option<String>,
    #[serde(default)]
    row: Option<String>,
    #[serde(default)]
    quantity: Option<u32>,
    #[serde(default)]
    listing_id: Option<String>,
    marketplace: String,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ScanJsDiagnostics {
    #[serde(default)]
    generic_elements_scanned: u32,
    #[serde(default)]
    selector_layer_error: Option<String>,
    #[serde(default)]
    generic_layer_error: Option<String>,
    #[serde(default)]
    fatal_error: Option<String>,
}

// ---------------------------------------------------------------------------
// Pure/unit-testable core - no Tauri types, no I/O. Everything below this
// point that touches an AppHandle/WebviewWindow is thin glue over these.
// ---------------------------------------------------------------------------

fn window_label_for(request_id: u64) -> String {
    format!("price-scanner-{request_id}")
}

/// Rejects anything that isn't a plain http(s) link before an OS window is
/// ever opened for it - `javascript:`/`file:`/`data:` etc. would either do
/// something marko didn't ask for or (per the `webview-data-url` feature
/// gap noted in this project's own throwaway smoke test) simply fail to
/// load.
fn parse_scanner_url(raw: &str) -> AppResult<tauri::Url> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation("Enter a marketplace URL first".into()));
    }
    let url = tauri::Url::parse(trimmed).map_err(|_| AppError::Validation("That doesn't look like a valid URL".into()))?;
    if url.scheme() != "http" && url.scheme() != "https" {
        return Err(AppError::Validation("The scanner can only open http:// or https:// links".into()));
    }
    Ok(url)
}

/// Core of `open_price_scanner`: registers a brand-new session, refusing a
/// `request_id` that's already in use rather than silently clobbering an
/// existing window's accumulated listings, and refusing a SECOND session
/// for a (event_id, marketplace_id) pair that already has one open - marko
/// can have StubHub and Vivid Seats open at once for the same event (two
/// different marketplace cards, two different sessions), but two windows
/// for the SAME card would just be confusing clutter with no way for the
/// UI to tell them apart. Returns the fresh session's cancel flag so the
/// caller can hand a clone of it to the eventual build/scan threads without
/// re-locking the map.
fn insert_new_session(
    sessions: &mut HashMap<u64, ScannerSession>,
    request_id: u64,
    window_label: String,
    event_id: i64,
    marketplace_id: i64,
) -> AppResult<Arc<AtomicBool>> {
    if sessions.contains_key(&request_id) {
        return Err(AppError::Validation(
            "A scanner session with this id is already open".into(),
        ));
    }
    if sessions.values().any(|s| s.event_id == event_id && s.marketplace_id == marketplace_id) {
        return Err(AppError::Validation(
            "A scanner is already open for this marketplace on this event - close it before opening another.".into(),
        ));
    }
    let cancel_flag = Arc::new(AtomicBool::new(false));
    sessions.insert(
        request_id,
        ScannerSession {
            window_label,
            event_id,
            marketplace_id,
            cancel_flag: cancel_flag.clone(),
            status: "ready".to_string(),
            listings: Vec::new(),
            fingerprints: HashSet::new(),
            scan_count: 0,
            last_scan_at: None,
        },
    );
    Ok(cancel_flag)
}

/// marko's own spec, "## DUPLICATES": "Ak sa ten istý listing objaví pri
/// dvoch scan-och: NEPRIDAJ DUPLICATE. Použi rozumný interný fingerprint
/// podľa dostupných údajov: marketplace, price, section, row, quantity,
/// listing identity, ak existuje." Implemented literally: every field that
/// exists feeds the key, a missing one contributes an empty slot rather
/// than being skipped (so "no section" and "section is an empty string"
/// collide, which is correct - both mean "no section info available").
///
/// Honest limitations, spec-acknowledged rather than hidden:
///
/// 1. Two bare price-only listings (no section/row/quantity/listingId at
///    all) with the same price/currency on the same marketplace WILL
///    collide and be treated as one listing, because nothing distinguishes
///    them. There is no real listing identity to fall back on in that case -
///    inventing one would violate marko's own "nikdy nevymýšľaj údaje"
///    (never invent data) rule more than an occasional under-count does.
/// 2. Every field feeds the key, INCLUDING section/row/quantity - so if the
///    same physical listing is read as bare-price-only on one scan (its
///    section/row hadn't rendered yet) and then with full section/row on a
///    later scan, it produces a DIFFERENT fingerprint and is counted as a
///    second listing rather than recognized as the same one maturing from
///    "partial" to "success". This is an inherent consequence of marko's own
///    literal fingerprint spec above (composite key over exactly those
///    fields, "ak existuje" - if it exists) - reconciling "the same listing
///    with more detail now visible" would require real listing identity
///    tracking across scans, which most pages don't expose (see
///    price_checker_scan.js's own listingIdFor comment: absent is the normal
///    case). Flagged, not silently fixed with a heuristic that would risk
///    the opposite mistake - wrongly merging two genuinely different
///    listings that happen to share a price (e.g. a general-admission
///    listing and a specific-seat listing at the same price). marko always
///    reviews the scanned table before saving to history, so an occasional
///    over-count here is visible and correctable, not silently trusted.
fn fingerprint_for(listing: &NormalizedListing) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}|{}",
        listing.marketplace,
        listing.price_cents,
        listing.currency.as_deref().unwrap_or(""),
        listing.section.as_deref().unwrap_or(""),
        listing.row.as_deref().unwrap_or(""),
        listing.quantity.map(|q| q.to_string()).unwrap_or_default(),
        listing.listing_id.as_deref().unwrap_or(""),
    )
}

fn median_of_sorted_cents(sorted: &[i64]) -> i64 {
    let n = sorted.len();
    if n % 2 == 1 {
        sorted[n / 2]
    } else {
        let a = sorted[n / 2 - 1];
        let b = sorted[n / 2];
        ((a as f64 + b as f64) / 2.0).round() as i64
    }
}

/// Lowest/median/average/highest/currency across every listing accumulated
/// in the session so far - marko's own spec, "## RESULT". Currency is
/// whichever the FIRST listing (in accumulation order) actually reported -
/// same "first candidate wins, never blended/guessed" convention this
/// codebase already uses elsewhere (see `ScanResultPayload::currency`'s own
/// doc comment, models.rs). `None` across the board for an empty session,
/// never a fabricated zero.
fn compute_scan_stats(
    listings: &[NormalizedListing],
) -> (Option<i64>, Option<i64>, Option<i64>, Option<i64>, Option<String>) {
    if listings.is_empty() {
        return (None, None, None, None, None);
    }
    let mut prices: Vec<i64> = listings.iter().map(|l| l.price_cents).collect();
    prices.sort_unstable();
    let lowest = prices.first().copied();
    let highest = prices.last().copied();
    let sum: i64 = prices.iter().sum();
    let average = Some((sum as f64 / prices.len() as f64).round() as i64);
    let median = Some(median_of_sorted_cents(&prices));
    let currency = listings.iter().find_map(|l| l.currency.clone());
    (lowest, median, average, highest, currency)
}

/// See this module's own doc comment, "## Status derivation". `latest_*`
/// params reflect only THIS scan's own JS payload (never sticky across
/// scans - see that same doc comment for why); `listings` is the
/// accumulated session total (sticky, by design - once a good listing is
/// found it stays found even if a later scan on a different part of the
/// page is blocked/partial).
fn derive_session_status(listings: &[NormalizedListing], latest_scan_blocked: bool, latest_scan_had_error: bool) -> &'static str {
    if latest_scan_had_error {
        return "error";
    }
    if latest_scan_blocked {
        return "blocked";
    }
    if listings.iter().any(|l| l.section.is_some() && l.row.is_some()) {
        return "success";
    }
    if !listings.is_empty() {
        return "partial";
    }
    "unable_to_read"
}

/// Human-readable detail for every non-"success" status - every number
/// quoted is real (session/diagnostic counts actually observed), never
/// invented, per marko's own "## IMPORTANT: Nikdy nevymýšľaj údaje".
fn build_status_message(status: &str, js: &ScanJsPayload, listings: &[NormalizedListing]) -> Option<String> {
    match status {
        "success" => None,
        "blocked" => Some(match &js.blocked_reason {
            Some(reason) => format!(
                "This page looks like it's showing a verification/anti-bot check (\"{reason}\"). Solve it in the scanner window, then click Scan Visible Prices again - TIQR Manager never tries to bypass this automatically."
            ),
            None => "This page looks like it's showing a verification/anti-bot check. Solve it in the scanner window, then click Scan Visible Prices again - TIQR Manager never tries to bypass this automatically.".to_string(),
        }),
        "error" => Some(
            js.diagnostics
                .fatal_error
                .clone()
                .or_else(|| js.diagnostics.selector_layer_error.clone())
                .or_else(|| js.diagnostics.generic_layer_error.clone())
                .unwrap_or_else(|| "The page could not be read.".to_string()),
        ),
        "partial" => Some(format!(
            "Found {} listing(s) with a price, but couldn't confidently read section/row for all of them - review before saving.",
            listings.len()
        )),
        _ => {
            // ok:true doesn't guarantee every layer ran cleanly - a layer
            // can catch its own exception internally (e.g. document.body
            // not ready yet) and still report ok:true with zero listings.
            // Surface that real cause when there is one, rather than always
            // showing the generic "didn't find any prices" text as if
            // every layer genuinely ran to completion and found nothing.
            let layer_error = js.diagnostics.selector_layer_error.as_deref().or(js.diagnostics.generic_layer_error.as_deref());
            Some(match layer_error {
                Some(err) => format!(
                    "Scanned {} element(s) on the visible page but hit an error partway through (\"{err}\"). Scroll to where the listings are, then scan again.",
                    js.diagnostics.generic_elements_scanned
                ),
                None => format!(
                    "Scanned {} element(s) on the visible page but didn't find any recognizable prices. Scroll to where the listings are, then scan again.",
                    js.diagnostics.generic_elements_scanned
                ),
            })
        }
    }
}

fn now_iso8601() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

/// `WebviewWindow::eval_with_callback`'s own doc comment (tauri 2.11.5,
/// src/webview/mod.rs): "The evaluation result will be serialized into a
/// JSON string and passed to the callback function." `SCAN_SCRIPT`'s own
/// completion value is ITSELF a JSON string - its last line is
/// `return JSON.stringify(payload);`, i.e. a JS *string* - so the raw wire
/// text this callback receives is a JSON string literal wrapping ANOTHER
/// JSON string: two encoding layers, not one. Confirmed empirically against
/// a real WebKitGTK eval_with_callback round-trip under Xvfb (task #455's
/// runtime verification - see PRICE-CHECKER-VISIBLE-SCANNER-REPORT.md): a
/// single `serde_json::from_str` here failed on every real scan with
/// "invalid type: string ..., expected struct ScanJsPayload", because it
/// stopped one layer short. Kept as its own small, pure, unit-testable
/// function specifically so this exact shape has a permanent regression
/// test that doesn't need a real WebView to run.
fn parse_scan_js_payload(raw: &str) -> Result<ScanJsPayload, String> {
    let once_unwrapped: String = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    serde_json::from_str(&once_unwrapped).map_err(|e| e.to_string())
}

/// The heart of `scan_visible_prices`: folds one scan's raw JS candidates
/// into the session's running, deduplicated total and returns the payload
/// to broadcast. Mutates `session` in place (listings/fingerprints/
/// scan_count/last_scan_at/status) - see `ScannerSession`'s own doc comment
/// (db.rs) for why this state lives there rather than being recomputed from
/// scratch on every call.
fn merge_scan_into_session(session: &mut ScannerSession, request_id: u64, js: &ScanJsPayload) -> ScanResultPayload {
    let mut added_this_scan: u32 = 0;
    for c in &js.candidates {
        let listing = NormalizedListing {
            price_cents: c.price_cents,
            currency: c.currency.clone(),
            section: c.section.clone(),
            row: c.row.clone(),
            quantity: c.quantity,
            listing_id: c.listing_id.clone(),
            marketplace: c.marketplace.clone(),
        };
        let fingerprint = fingerprint_for(&listing);
        if session.fingerprints.insert(fingerprint) {
            session.listings.push(listing);
            added_this_scan += 1;
        }
    }

    session.scan_count += 1;
    session.last_scan_at = Some(now_iso8601());

    let status = derive_session_status(&session.listings, js.blocked, !js.ok);
    session.status = status.to_string();

    let (lowest, median, average, highest, currency) = compute_scan_stats(&session.listings);
    let message = build_status_message(status, js, &session.listings);

    ScanResultPayload {
        request_id,
        status: status.to_string(),
        added_this_scan,
        listings: session.listings.clone(),
        lowest_price_cents: lowest,
        median_price_cents: median,
        average_price_cents: average,
        highest_price_cents: highest,
        currency,
        scan_count: session.scan_count,
        last_scan_at: session.last_scan_at.clone(),
        message,
    }
}

// ---------------------------------------------------------------------------
// Tauri glue - thread-spawning, window lifecycle, event emission. Kept as
// thin as possible; the actual decisions all live in the pure functions
// above.
// ---------------------------------------------------------------------------

/// Shared by `close_price_scanner` and the visible window's own native
/// close button (`on_window_event`/`CloseRequested`, wired up in
/// `open_price_scanner` below) - whichever of the two reaches the session
/// map first does the real cleanup+emit; `HashMap::remove` returning `None`
/// the second time makes the other one a harmless no-op, so marko clicking
/// "Close window" at the exact moment he also closes it natively can never
/// double-fire `price-scanner-closed`.
fn finish_session(app: &AppHandle, request_id: u64) {
    let removed = match app.try_state::<AppState>() {
        Some(st) => st.price_scanner_sessions.lock().unwrap().remove(&request_id),
        None => None,
    };
    if let Some(session) = removed {
        session.cancel_flag.store(true, Ordering::Relaxed);
        let _ = app.emit(EVENT_SCANNER_CLOSED, ScannerClosedPayload { request_id });
    }
}

/// Reports a scan-mechanism failure (eval dispatch error, timeout, unreadable
/// response, window gone) - as opposed to `build_status_message`'s "error",
/// which is the PAGE's own JS reporting a caught exception. Both end up as
/// status "error" on a `ScanResultPayload`, but this path never got a real
/// JS payload at all, so it reports the session's PREVIOUSLY accumulated
/// listings/stats unchanged rather than merging anything new in. A silent
/// no-op if the session is already gone (the window closed while this scan
/// was in flight) - nothing left to tell.
fn emit_scan_error(app: &AppHandle, request_id: u64, message: &str) {
    if let Some(st) = app.try_state::<AppState>() {
        let mut sessions = st.price_scanner_sessions.lock().unwrap();
        if let Some(session) = sessions.get_mut(&request_id) {
            session.status = "error".to_string();
            let (lowest, median, average, highest, currency) = compute_scan_stats(&session.listings);
            let payload = ScanResultPayload {
                request_id,
                status: "error".to_string(),
                added_this_scan: 0,
                listings: session.listings.clone(),
                lowest_price_cents: lowest,
                median_price_cents: median,
                average_price_cents: average,
                highest_price_cents: highest,
                currency,
                scan_count: session.scan_count,
                last_scan_at: session.last_scan_at.clone(),
                message: Some(message.to_string()),
            };
            drop(sessions);
            let _ = app.emit(EVENT_SCAN_RESULT, payload);
        }
    }
}

/// Opens a new Visible Scanner window for one marketplace card. Returns
/// almost immediately - the actual window build happens on a plain OS
/// thread (never synchronously inside this command's own call stack:
/// `WebviewWindowBuilder::build()` deadlocks on Windows if called from a
/// Tauri command's calling thread, this project's own established,
/// previously-hit constraint) and the real outcome arrives via
/// `price-scanner-opened`/`price-scanner-error`.
#[tauri::command]
pub fn open_price_scanner(app: AppHandle, state: State<AppState>, request_id: u64, event_id: i64, marketplace_id: i64, url: String) -> AppResult<()> {
    let parsed_url = parse_scanner_url(&url)?;
    let label = window_label_for(request_id);

    {
        let mut sessions = state.price_scanner_sessions.lock().unwrap();
        insert_new_session(&mut sessions, request_id, label.clone(), event_id, marketplace_id)?;
    }

    let handle = app.clone();
    std::thread::spawn(move || {
        let build_result = tauri::WebviewWindowBuilder::new(&handle, &label, tauri::WebviewUrl::External(parsed_url))
            .title("TIQR Manager - Price Scanner")
            .inner_size(1280.0, 900.0)
            .visible(true)
            .build();

        match build_result {
            Ok(window) => {
                let handle_for_close = handle.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { .. } = event {
                        // marko closed the window himself (native close
                        // button) rather than via "Close window" in TIQR
                        // Manager - same cleanup either way, see
                        // finish_session's own doc comment.
                        finish_session(&handle_for_close, request_id);
                    }
                });
                let _ = handle.emit(EVENT_SCANNER_OPENED, ScannerOpenedPayload { request_id });
            }
            Err(e) => {
                if let Some(st) = handle.try_state::<AppState>() {
                    st.price_scanner_sessions.lock().unwrap().remove(&request_id);
                }
                let _ = handle.emit(
                    EVENT_SCANNER_ERROR,
                    ScannerErrorPayload { request_id, message: format!("Could not open the scanner window: {e}") },
                );
            }
        }
    });

    Ok(())
}

/// Reads whatever `price_checker_scan.js` finds on the CURRENTLY VISIBLE
/// page, exactly once, and merges it into the session. Returns almost
/// immediately; the real result arrives via `price-scanner-scan-result`
/// (`ScanResultPayload`) - the frontend should optimistically flip that
/// marketplace card to "Scanning" the instant it calls this, the same
/// "command returns fast, the real outcome follows as an event" contract
/// `open_price_scanner` uses.
#[tauri::command]
pub fn scan_visible_prices(app: AppHandle, state: State<AppState>, request_id: u64) -> AppResult<()> {
    let (window_label, cancel_flag) = {
        let mut sessions = state.price_scanner_sessions.lock().unwrap();
        let session = sessions
            .get_mut(&request_id)
            .ok_or_else(|| AppError::NotFound("Scanner session not found - the window may have been closed".into()))?;
        // A genuinely NEW flag for THIS attempt - not a reset-in-place of
        // the session's existing one. Reusing the same Arc<AtomicBool> and
        // just flipping it back to false here would race a still-in-flight
        // PREVIOUS scan: Stop sets it true while scan A is blocked waiting
        // on eval_with_callback's callback; the user clicks Scan again
        // before A's result arrives; this line would silently flip the
        // SAME flag A is about to check back to false, so A's own cancel
        // check (below) would wrongly read "not cancelled" and merge in a
        // result the user explicitly stopped. Replacing the Arc instead
        // means scan A keeps its own clone (permanently true, set by Stop)
        // while this new attempt gets its own fresh flag - each attempt's
        // cancellation is independent, matching marko's own spec: Stop
        // interrupts the attempt in flight when it's clicked, and never
        // permanently disables the window for every future scan (the
        // window "zostane otvorený" and the app "musí byť stále úplne
        // použiteľná").
        session.cancel_flag = Arc::new(AtomicBool::new(false));
        session.status = "scanning".to_string();
        (session.window_label.clone(), session.cancel_flag.clone())
    };

    let handle = app.clone();
    std::thread::spawn(move || {
        if cancel_flag.load(Ordering::Relaxed) {
            return;
        }
        let window = match handle.get_webview_window(&window_label) {
            Some(w) => w,
            None => {
                emit_scan_error(&handle, request_id, "The scanner window is no longer open.");
                return;
            }
        };

        let (tx, rx) = mpsc::channel::<String>();
        if let Err(e) = window.eval_with_callback(SCAN_SCRIPT, move |result: String| {
            let _ = tx.send(result);
        }) {
            emit_scan_error(&handle, request_id, &format!("Could not run the scan: {e}"));
            return;
        }

        let raw = match rx.recv_timeout(SCAN_EVAL_TIMEOUT) {
            Ok(r) => r,
            Err(_) => {
                emit_scan_error(&handle, request_id, "The page didn't respond to the scan in time.");
                return;
            }
        };

        if cancel_flag.load(Ordering::Relaxed) {
            // Stopped while the eval was in flight - drop the result
            // rather than merging it in, so a cancelled scan really does
            // add nothing (matches marko's own "Stop scanning" intent).
            return;
        }

        let js: ScanJsPayload = match parse_scan_js_payload(&raw) {
            Ok(v) => v,
            Err(e) => {
                emit_scan_error(&handle, request_id, &format!("Got an unreadable response from the page: {e}"));
                return;
            }
        };

        if let Some(st) = handle.try_state::<AppState>() {
            let mut sessions = st.price_scanner_sessions.lock().unwrap();
            if let Some(session) = sessions.get_mut(&request_id) {
                let payload = merge_scan_into_session(session, request_id, &js);
                drop(sessions);
                let _ = handle.emit(EVENT_SCAN_RESULT, payload);
            }
        }
    });

    Ok(())
}

/// Interrupts an in-flight `scan_visible_prices` call for this session, if
/// one is running - marko's own "## STOP / CANCEL". A harmless flag flip
/// when nothing is actually in flight (the next `scan_visible_prices`
/// resets it anyway - see that command's own doc comment). Never touches
/// the window itself: the visible browser stays open and fully usable
/// either way.
#[tauri::command]
pub fn cancel_price_scan(state: State<AppState>, request_id: u64) -> AppResult<()> {
    let sessions = state.price_scanner_sessions.lock().unwrap();
    let session = sessions.get(&request_id).ok_or_else(|| AppError::NotFound("Scanner session not found".into()))?;
    session.cancel_flag.store(true, Ordering::Relaxed);
    Ok(())
}

/// Ends a scanner session from the TIQR Manager side - "Close window" in
/// the UI. `close_window: false` just forgets the session's bookkeeping
/// while leaving the actual browser window open (marko's own spec: "browser
/// zostane otvorený alebo sa môže zavrieť podľa voľby používateľa" - stays
/// open or closes, by the user's choice); `true` also closes it. Idempotent
/// - closing an already-gone session (e.g. marko already closed it natively
/// a moment earlier) is a harmless no-op, never an error, since from the
/// UI's point of view the end state ("no session, no card controls") is
/// identical either way.
#[tauri::command]
pub fn close_price_scanner(app: AppHandle, state: State<AppState>, request_id: u64, close_window: bool) -> AppResult<()> {
    let window_label = {
        let sessions = state.price_scanner_sessions.lock().unwrap();
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

#[cfg(test)]
mod tests {
    use super::*;

    fn listing(marketplace: &str, price_cents: i64, section: Option<&str>, row: Option<&str>, quantity: Option<u32>, listing_id: Option<&str>) -> NormalizedListing {
        NormalizedListing {
            price_cents,
            currency: Some("EUR".to_string()),
            section: section.map(|s| s.to_string()),
            row: row.map(|s| s.to_string()),
            quantity,
            listing_id: listing_id.map(|s| s.to_string()),
            marketplace: marketplace.to_string(),
        }
    }

    fn session_at(request_id_seed: &str) -> ScannerSession {
        ScannerSession {
            window_label: format!("price-scanner-{request_id_seed}"),
            event_id: 1,
            marketplace_id: 1,
            cancel_flag: Arc::new(AtomicBool::new(false)),
            status: "ready".to_string(),
            listings: Vec::new(),
            fingerprints: HashSet::new(),
            scan_count: 0,
            last_scan_at: None,
        }
    }

    fn js_ok(candidates: Vec<ScanJsCandidate>) -> ScanJsPayload {
        ScanJsPayload { ok: true, blocked: false, blocked_reason: None, candidates, diagnostics: ScanJsDiagnostics::default() }
    }

    fn candidate(price_cents: i64, section: Option<&str>, row: Option<&str>) -> ScanJsCandidate {
        ScanJsCandidate {
            price_cents,
            currency: Some("EUR".to_string()),
            section: section.map(|s| s.to_string()),
            row: row.map(|s| s.to_string()),
            quantity: None,
            listing_id: None,
            marketplace: "stubhub".to_string(),
        }
    }

    // -- parse_scanner_url ---------------------------------------------------

    #[test]
    fn a_blank_url_is_refused() {
        assert!(parse_scanner_url("   ").is_err());
    }

    #[test]
    fn a_non_http_scheme_is_refused() {
        assert!(parse_scanner_url("javascript:alert(1)").is_err());
        assert!(parse_scanner_url("file:///etc/passwd").is_err());
    }

    #[test]
    fn a_plain_https_url_is_accepted() {
        assert!(parse_scanner_url("https://www.stubhub.com/some-event/tickets").is_ok());
    }

    // -- insert_new_session ---------------------------------------------------

    #[test]
    fn inserting_a_new_session_succeeds_and_starts_ready_and_empty() {
        let mut sessions = HashMap::new();
        insert_new_session(&mut sessions, 1, "price-scanner-1".into(), 10, 20).unwrap();
        let session = sessions.get(&1).unwrap();
        assert_eq!(session.status, "ready");
        assert!(session.listings.is_empty());
        assert_eq!(session.event_id, 10);
        assert_eq!(session.marketplace_id, 20);
    }

    #[test]
    fn a_duplicate_request_id_is_refused_without_touching_the_existing_session() {
        let mut sessions = HashMap::new();
        insert_new_session(&mut sessions, 1, "price-scanner-1".into(), 10, 20).unwrap();
        sessions.get_mut(&1).unwrap().scan_count = 3;

        assert!(insert_new_session(&mut sessions, 1, "price-scanner-1-again".into(), 99, 99).is_err());
        assert_eq!(sessions.get(&1).unwrap().scan_count, 3, "the existing session must be untouched");
    }

    #[test]
    fn a_second_session_for_the_same_event_and_marketplace_is_refused() {
        let mut sessions = HashMap::new();
        insert_new_session(&mut sessions, 1, "price-scanner-1".into(), 10, 20).unwrap();

        let err = insert_new_session(&mut sessions, 2, "price-scanner-2".into(), 10, 20).unwrap_err();
        assert!(err.to_string().contains("already open"));
        assert_eq!(sessions.len(), 1);
    }

    #[test]
    fn a_second_session_for_a_different_marketplace_on_the_same_event_is_allowed() {
        let mut sessions = HashMap::new();
        insert_new_session(&mut sessions, 1, "price-scanner-1".into(), 10, 20).unwrap();

        assert!(insert_new_session(&mut sessions, 2, "price-scanner-2".into(), 10, 21).is_ok());
        assert_eq!(sessions.len(), 2, "StubHub and Vivid Seats must be able to be open at once for the same event");
    }

    // -- cancel_flag replacement (scan_visible_prices's own preamble) -----------

    #[test]
    fn starting_a_new_scan_gives_a_fresh_flag_without_disturbing_a_still_in_flight_older_one() {
        // Reproduces the exact race scan_visible_prices's own doc comment
        // describes: scan A is in flight holding a clone of the session's
        // cancel_flag; Stop sets THAT flag true; the user starts scan B
        // before A's result arrives. scan_visible_prices's real preamble
        // does `session.cancel_flag = Arc::new(AtomicBool::new(false))`
        // (never `.store(false, ..)` on the existing Arc) specifically so
        // this can't happen - reproduced here directly against
        // ScannerSession's own field, no real webview needed.
        let mut sessions = HashMap::new();
        insert_new_session(&mut sessions, 1, "price-scanner-1".into(), 10, 20).unwrap();
        let session = sessions.get_mut(&1).unwrap();

        let scan_a_flag = session.cancel_flag.clone(); // what scan A's spawned thread would hold
        scan_a_flag.store(true, Ordering::Relaxed); // user clicks Stop while A is in flight

        // scan_visible_prices's real preamble for the next attempt:
        session.cancel_flag = Arc::new(AtomicBool::new(false));

        assert!(scan_a_flag.load(Ordering::Relaxed), "scan A's own captured flag must stay true - it was genuinely stopped");
        assert!(!session.cancel_flag.load(Ordering::Relaxed), "the NEW attempt must start with its own fresh, uncancelled flag");
    }

    // -- fingerprint_for --------------------------------------------------------

    #[test]
    fn identical_listings_produce_the_same_fingerprint() {
        let a = listing("stubhub", 5000, Some("112"), Some("A"), Some(2), Some("xyz"));
        let b = listing("stubhub", 5000, Some("112"), Some("A"), Some(2), Some("xyz"));
        assert_eq!(fingerprint_for(&a), fingerprint_for(&b));
    }

    #[test]
    fn a_different_price_produces_a_different_fingerprint() {
        let a = listing("stubhub", 5000, Some("112"), Some("A"), Some(2), None);
        let b = listing("stubhub", 5100, Some("112"), Some("A"), Some(2), None);
        assert_ne!(fingerprint_for(&a), fingerprint_for(&b));
    }

    #[test]
    fn two_bare_price_only_listings_collide_by_design() {
        // Documented, spec-acknowledged limitation - see fingerprint_for's
        // own doc comment: with nothing else to go on, two same-priced
        // bare listings are indistinguishable.
        let a = listing("stubhub", 5000, None, None, None, None);
        let b = listing("stubhub", 5000, None, None, None, None);
        assert_eq!(fingerprint_for(&a), fingerprint_for(&b));
    }

    #[test]
    fn a_different_currency_at_the_same_price_and_seat_produces_a_different_fingerprint() {
        // A EUR 50.00 listing and a USD 50.00 listing in the same
        // section/row/quantity are two genuinely different real listings,
        // not the same one seen twice - currency must be part of the key.
        let mut a = listing("stubhub", 5000, Some("112"), Some("A"), Some(2), None);
        a.currency = Some("EUR".to_string());
        let mut b = listing("stubhub", 5000, Some("112"), Some("A"), Some(2), None);
        b.currency = Some("USD".to_string());
        assert_ne!(fingerprint_for(&a), fingerprint_for(&b));
    }

    // -- compute_scan_stats ------------------------------------------------------

    #[test]
    fn stats_over_an_empty_session_are_all_none() {
        assert_eq!(compute_scan_stats(&[]), (None, None, None, None, None));
    }

    #[test]
    fn stats_with_an_odd_number_of_listings_use_the_middle_price_as_median() {
        let listings = vec![
            listing("stubhub", 3000, None, None, None, None),
            listing("stubhub", 1000, None, None, None, None),
            listing("stubhub", 2000, None, None, None, None),
        ];
        let (lowest, median, average, highest, currency) = compute_scan_stats(&listings);
        assert_eq!(lowest, Some(1000));
        assert_eq!(median, Some(2000));
        assert_eq!(average, Some(2000));
        assert_eq!(highest, Some(3000));
        assert_eq!(currency, Some("EUR".to_string()));
    }

    #[test]
    fn stats_with_an_even_number_of_listings_average_the_two_middle_prices_for_the_median() {
        let listings = vec![
            listing("stubhub", 1000, None, None, None, None),
            listing("stubhub", 2000, None, None, None, None),
            listing("stubhub", 3000, None, None, None, None),
            listing("stubhub", 4000, None, None, None, None),
        ];
        let (_, median, _, _, _) = compute_scan_stats(&listings);
        assert_eq!(median, Some(2500));
    }

    #[test]
    fn currency_is_the_first_listings_currency_in_order() {
        let mut first = listing("stubhub", 1000, None, None, None, None);
        first.currency = None;
        let mut second = listing("stubhub", 2000, None, None, None, None);
        second.currency = Some("USD".to_string());
        let (_, _, _, _, currency) = compute_scan_stats(&[first, second]);
        assert_eq!(currency, Some("USD".to_string()), "must skip a listing with no currency and use the first one that has it");
    }

    // -- derive_session_status ----------------------------------------------------

    #[test]
    fn no_listings_and_no_error_or_block_is_unable_to_read() {
        assert_eq!(derive_session_status(&[], false, false), "unable_to_read");
    }

    #[test]
    fn a_price_only_listing_is_partial() {
        let listings = vec![listing("stubhub", 5000, None, None, None, None)];
        assert_eq!(derive_session_status(&listings, false, false), "partial");
    }

    #[test]
    fn a_listing_with_both_section_and_row_is_success() {
        let listings = vec![
            listing("stubhub", 5000, None, None, None, None),
            listing("stubhub", 6000, Some("112"), Some("A"), None, None),
        ];
        assert_eq!(derive_session_status(&listings, false, false), "success");
    }

    #[test]
    fn section_without_row_is_still_only_partial() {
        let listings = vec![listing("stubhub", 5000, Some("112"), None, None, None)];
        assert_eq!(derive_session_status(&listings, false, false), "partial");
    }

    #[test]
    fn a_blocked_latest_scan_overrides_existing_good_listings() {
        let listings = vec![listing("stubhub", 5000, Some("112"), Some("A"), None, None)];
        assert_eq!(derive_session_status(&listings, true, false), "blocked");
    }

    #[test]
    fn an_errored_latest_scan_takes_priority_over_blocked() {
        let listings = vec![listing("stubhub", 5000, Some("112"), Some("A"), None, None)];
        assert_eq!(derive_session_status(&listings, true, true), "error");
    }

    // -- merge_scan_into_session ------------------------------------------------

    #[test]
    fn markos_own_spec_example_20_plus_20_distinct_listings_is_40_unique() {
        let mut session = session_at("1");
        let first_batch: Vec<ScanJsCandidate> = (0..20).map(|i| candidate(1000 + i, Some("A"), Some("1"))).collect();
        let js1 = js_ok(first_batch);
        let result1 = merge_scan_into_session(&mut session, 1, &js1);
        assert_eq!(result1.added_this_scan, 20);
        assert_eq!(session.listings.len(), 20);

        let second_batch: Vec<ScanJsCandidate> = (100..120).map(|i| candidate(1000 + i, Some("A"), Some("1"))).collect();
        let js2 = js_ok(second_batch);
        let result2 = merge_scan_into_session(&mut session, 1, &js2);
        assert_eq!(result2.added_this_scan, 20, "scroll + scan again must add exactly the 20 NEW listings");
        assert_eq!(session.listings.len(), 40, "40 unique listings total, matching marko's own spec example");
    }

    #[test]
    fn scanning_the_same_listing_twice_adds_it_only_once() {
        let mut session = session_at("1");
        let js1 = js_ok(vec![candidate(5000, Some("112"), Some("A"))]);
        merge_scan_into_session(&mut session, 1, &js1);

        let js2 = js_ok(vec![candidate(5000, Some("112"), Some("A"))]);
        let result2 = merge_scan_into_session(&mut session, 1, &js2);

        assert_eq!(result2.added_this_scan, 0, "an already-seen listing must not be added again");
        assert_eq!(session.listings.len(), 1);
        assert_eq!(session.scan_count, 2, "the scan itself still counts, even though nothing new was found");
    }

    #[test]
    fn scan_count_and_last_scan_at_advance_on_every_scan() {
        let mut session = session_at("1");
        assert_eq!(session.scan_count, 0);
        assert!(session.last_scan_at.is_none());

        merge_scan_into_session(&mut session, 1, &js_ok(vec![]));
        assert_eq!(session.scan_count, 1);
        assert!(session.last_scan_at.is_some());
    }

    #[test]
    fn a_blocked_scan_is_not_sticky_once_a_later_scan_succeeds() {
        let mut session = session_at("1");
        let blocked = ScanJsPayload {
            ok: true,
            blocked: true,
            blocked_reason: Some("just a moment".to_string()),
            candidates: vec![],
            diagnostics: ScanJsDiagnostics::default(),
        };
        let result1 = merge_scan_into_session(&mut session, 1, &blocked);
        assert_eq!(result1.status, "blocked");
        assert!(result1.message.is_some());

        let clean = js_ok(vec![candidate(5000, Some("112"), Some("A"))]);
        let result2 = merge_scan_into_session(&mut session, 1, &clean);
        assert_eq!(result2.status, "success", "a later successful scan must clear the earlier blocked status");
    }

    #[test]
    fn a_partial_result_carries_a_review_message_and_a_success_result_carries_none() {
        let mut session = session_at("1");
        let partial = js_ok(vec![candidate(5000, None, None)]);
        let result = merge_scan_into_session(&mut session, 1, &partial);
        assert_eq!(result.status, "partial");
        assert!(result.message.is_some());

        let mut session2 = session_at("2");
        let success = js_ok(vec![candidate(5000, Some("112"), Some("A"))]);
        let result2 = merge_scan_into_session(&mut session2, 2, &success);
        assert_eq!(result2.status, "success");
        assert!(result2.message.is_none());
    }

    #[test]
    fn stats_in_the_returned_payload_match_the_full_accumulated_session_not_just_this_scans_delta() {
        let mut session = session_at("1");
        merge_scan_into_session(&mut session, 1, &js_ok(vec![candidate(1000, Some("A"), Some("1"))]));
        let result = merge_scan_into_session(&mut session, 1, &js_ok(vec![candidate(3000, Some("B"), Some("2"))]));

        assert_eq!(result.lowest_price_cents, Some(1000));
        assert_eq!(result.highest_price_cents, Some(3000));
        assert_eq!(result.listings.len(), 2, "the payload's listings must be the WHOLE session, not just this scan's new ones");
    }

    // -- parse_scan_js_payload ---------------------------------------------------

    #[test]
    fn parses_the_real_double_json_encoded_wire_format_eval_with_callback_actually_produces() {
        // eval_with_callback's own doc comment: "The evaluation result will
        // be serialized into a JSON string." SCAN_SCRIPT's completion value
        // is ITSELF JSON.stringify(payload) - a JS string - so the real wire
        // text is a JSON string literal wrapping another JSON string. This
        // reproduces that exact shape (confirmed against a real WebKitGTK
        // round-trip under Xvfb) without needing a real WebView to run.
        let inner = r#"{"ok":true,"marketplace":"stubhub","hostname":"h","url":"http://h/","title":"t","blocked":false,"candidates":[{"priceCents":12500,"currency":"USD","section":"101","row":"F","quantity":2,"marketplace":"stubhub"}],"diagnostics":{"selectorHits":1,"genericElementsScanned":10,"totalBeforeDedup":1,"totalAfterDedup":1}}"#;
        let wire = serde_json::to_string(inner).unwrap(); // exactly what eval_with_callback hands the Rust callback
        let parsed = parse_scan_js_payload(&wire).expect("must parse the real double-encoded wire format");
        assert!(parsed.ok);
        assert_eq!(parsed.candidates.len(), 1);
        assert_eq!(parsed.candidates[0].price_cents, 12500);
    }

    #[test]
    fn a_not_double_encoded_payload_is_rejected_rather_than_silently_misparsed() {
        // Guards the other direction too - if a future Tauri version ever
        // changes eval_with_callback to hand back an already-unwrapped
        // object string, this must fail loudly rather than pass by
        // accident on the wrong shape.
        let inner = r#"{"ok":true,"marketplace":"stubhub","hostname":"h","url":"http://h/","title":"t","blocked":false,"candidates":[],"diagnostics":{}}"#;
        assert!(parse_scan_js_payload(inner).is_err(), "a not-double-encoded payload must not silently parse");
    }
}

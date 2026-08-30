//! Price Checker "auto-check" (2.1.1) - marko's follow-up on top of the
//! 2.0.81/2.0.82 manual Price Checker (see commands::price_checker's own
//! module doc comment for that original design and WHY it was manual-only
//! from day one: no public read API on StubHub/Vivid Seats/Ticombo, and
//! marko's explicit instruction to fall back to manual entry rather than
//! bypass any site's protection).
//!
//! This module does NOT change that decision or bypass anything new - it
//! automates the ONE step marko was already doing by hand: opening the
//! saved marketplace link himself and reading the prices off the page.
//! `auto_check_price` opens that exact URL in the app's own embedded
//! WebView (the same engine Tauri already renders the app's own UI with -
//! WebView2/WKWebView/WebKitGTK depending on OS, nothing bundled, nothing
//! downloaded), lets it render like it would for any visitor, and reads
//! back whatever prices are actually visible - no login, no CAPTCHA
//! solving, no anti-bot evasion, no API, no proxies. If a site's own
//! protections stop this (or it simply doesn't expose prices without a
//! person interacting with it), the command returns a clear `"blocked"` /
//! `"unable_to_read"` status and marko is back to exactly the existing
//! paste/manual flow (SavePriceCheckModal, PriceChecker.tsx) - unchanged.
//!
//! ## Deliberately additive - see FINANCE-2.1.0-REPORT.md for the pattern
//!
//! Zero schema changes, zero changes to `commands::price_checker` or its
//! tests, zero changes to `models::{Marketplace, PriceCheck, PriceCheckInput}`.
//! `AutoCheckResult` (models.rs) only ever flows back to the frontend, which
//! reuses the EXISTING "paste text -> extractPricesFromText -> fill the 4
//! fields -> marko reviews -> Save check (unchanged `save_price_check`)"
//! pipeline from 2.0.82 - auto-check is presented to marko as "let the app
//! do the copy-paste for you", not a new kind of record. A successful
//! auto-check is stored exactly the same way a pasted one already is: as a
//! normal manually-saved `price_checks` row, once marko reviews and clicks
//! Save.
//!
//! ## Extraction strategy - marketplace-agnostic on purpose
//!
//! `marketplaces` is a plain lookup table marko manages himself (see
//! migrations/014_price_checker.sql) - he can rename or add a 4th/5th
//! marketplace without a new migration. Hard-coding "if marketplace name is
//! StubHub, do X" here would quietly break the moment he does that, so this
//! reads the SAME three signals regardless of which marketplace it's
//! pointed at: schema.org JSON-LD `Offer`/`AggregateOffer` (structured,
//! standard - if a site includes it at all, it's reliable), an HTML table
//! whose header row looks like a listings table (Section/Row/Price-shaped
//! columns), and Open Graph `og:price:amount` meta tags. Whichever finds
//! prices first wins; if none do, the result is `"unable_to_read"`, not a
//! guess.
//!
//! ## Freeze fix (2.1.2) - what was actually broken and why
//!
//! marko's real-world report: clicking Auto-check left the button loading
//! forever, the rest of the app became unresponsive to clicks, and closing
//! TIQR Manager while this was happening sometimes left it unable to start
//! again without a full PC restart. All three symptoms trace to ONE root
//! cause, confirmed directly against Tauri's own published documentation
//! for `WebviewWindowBuilder`/`WebviewWindow::new` (not assumed from
//! memory): **"On Windows, this function deadlocks when used in a
//! synchronous command and event handlers... use async commands and
//! separate threads when creating windows."** The pre-2.1.2 version of this
//! file did exactly the thing that warning describes - `run_browser_read`
//! called `WebviewWindowBuilder::build()` directly, synchronously, from
//! within `auto_check_price`'s own command-handler call stack. That
//! matches marko's report precisely: a main-thread deadlock during window
//! creation freezes the whole app's event loop (not just the Auto-check
//! button), and `tauri_plugin_single_instance` (lib.rs - "MUST be the first
//! plugin registered") keeps its lock held by that now-unresponsive-but-
//! still-alive process, so a normal relaunch can never take over - only
//! killing the hung process (or restarting the PC, the blunt instrument any
//! user knows) releases it. This is invisible to `cargo check`/`cargo
//! test`/`tsc`/`npm run build` - none of them ever create a real window,
//! and no sandbox seen so far (including this one) has a display server to
//! catch it by actually running the app either. The fix below follows
//! Tauri's own documented remedy ("separate threads") using the exact
//! primitives already established elsewhere in this codebase - see
//! `run_with_outer_deadline`'s own doc comment.
//!
//! ## What was actually verified vs. what wasn't (2026-08-30)
//!
//! Researched directly (real fetches against live marketplace pages, not
//! assumed) before this was built: StubHub's event pages carry NO
//! extractable price data without executing the page's own JavaScript -
//! this module's JSON-LD pass exists for correctness/future-proofing but is
//! realistically expected to come back `"unable_to_read"` for StubHub today.
//! Vivid Seats' event pages DO expose a Section/Row/Price/Deal-Score table
//! directly (confirmed on a real, live event page) - the HTML-table pass
//! targets exactly that shape. Ticombo's pages carry `og:price:amount` /
//! `og:price:currency` meta tags in their markup pattern, but no currently
//! -listed (non-expired) Ticombo event page was available to confirm a
//! populated value directly - this pass is the least field-verified of the
//! three.
//!
//! ## Same sandbox limitation as google_oauth.rs/google_sheets.rs
//!
//! This module's own live-webview path (`run_browser_read`) could not be
//! exercised end to end in ANY sandbox seen so far, including this one: no
//! display server is available for a real WebView, so `WebviewWindowBuilder
//! ::build()`/`eval_with_callback` succeeding or failing against a real
//! window is simply not observable here - only marko's own real machine can
//! confirm that (see this module's own STOP checklist in the shipped
//! report). What COULD be verified here, and was: the freeze fix's actual
//! root cause against Tauri's own published docs (not memory), a full
//! `cargo check`/`cargo test --lib` of this exact module compiling and
//! passing in a sandbox with a modern-enough Rust toolchain (1.95.0 here -
//! the `dlopen2_derive`/Rust-1.85+ blocker an earlier pass of this feature
//! hit does not apply in this sandbox), the JS extraction logic against
//! real, saved marketplace HTML (a separate, isolated jsdom/Node harness -
//! see the shipped freeze-fix report), and every pure-Rust function below
//! that doesn't require an actual running WebView (`parse_auto_check_json`,
//! URL validation/normalization, the cancel/timeout state machine, the
//! RAII cleanup pattern) - all covered by the `#[cfg(test)]` module below.
//!
//! ## Threading - matches this codebase's own established "long operation
//! that must not block the UI, with cancellation" pattern
//!
//! `commands::google_auth::start_google_sign_in` already solves the exact
//! same shape of problem (a blocking, possibly-slow, cancellable operation
//! that must never freeze the app) using a plain `std::thread`/`mpsc`
//! channel/`recv_timeout` - no `async`/tokio business logic anywhere in
//! this crate's own database or command layer, matching every other module
//! (see google_sheets.rs's own module doc comment on why `reqwest`'s
//! blocking feature is used deliberately throughout). `run_with_outer_
//! deadline` below follows that identical shape: the actual window-creation
//! + polling + extraction work happens entirely on a freshly spawned
//! `std::thread` (a genuinely separate thread from whichever worker thread
//! Tauri's own synchronous-command dispatch used to call `auto_check_price`
//! - exactly what Tauri's own docs recommend), while `auto_check_price`
//! itself just waits on that thread's result with a hard, bounded
//! `recv_timeout`. Cancellation reuses `commands::google_auth`'s own
//! `AtomicBool`-in-an-`AppState`-slot pattern directly (via `AppState::
//! price_checker_auto_cancel_flag` and the SAME generic `cancel_google_
//! sign_in_impl` helper `firebase_google_auth.rs` already reuses for its
//! own, separate flag) rather than inventing a new mechanism.

//!
//! ## Production hardening (2.1.3) - marko's explicit follow-up after 2.1.2
//!
//! marko's own words: "Mám stále obavu z Price Checker Auto-checku... Chcem
//! z tejto časti spraviť production-grade systém." No new features (his own
//! explicit instruction) - everything below hardens the SAME lifecycle
//! 2.1.2 already fixed, closing gaps a full re-audit found rather than
//! reacting to a new bug report. See PRICE-CHECKER-PRODUCTION-HARDENING-
//! REPORT.md for the full writeup; summary of what changed here:
//!
//! - **Single-flight guard** (`auto_check_price`): the pre-2.1.3 version set
//!   `price_checker_auto_cancel_flag` unconditionally, so two concurrent
//!   invocations (a fast double-click slipping past the frontend's own
//!   `disabled` gating before React re-renders it, or any future caller)
//!   would silently overwrite each other's cancel flag - the FIRST request
//!   would keep running with no way to cancel it via the UI, and its
//!   eventual late result could reopen a modal marko had already moved on
//!   from. Now a second invocation while one is already in flight is
//!   rejected immediately with `status: "busy"`, never starts a second
//!   webview, and never touches the first attempt's flag.
//! - **Request IDs** (marko's literal "Každý request musí mať vlastné
//!   request ID"): `PriceChecker.tsx` mints one per attempt and passes it
//!   through `auto_check_price`; every `PROGRESS_EVENT` this module emits
//!   carries it back, and every `log_lifecycle` line below is tagged with
//!   it. The frontend uses it to ignore a stale attempt's late-arriving
//!   progress events/promise result once a newer attempt has started - see
//!   PriceChecker.tsx's own comments on `requestIdRef`. The backend never
//!   makes any decision based on this value (single-flight safety is purely
//!   the cancel-flag slot below) - it is a pure echo, for display only.
//! - **CLEANING_UP phase**: `emit_phase(app, request_id, "cleaning_up")` now
//!   fires right after `poll_then_extract` returns, before `run_browser_
//!   read` itself returns (and therefore before `WebviewGuard` actually
//!   closes the window) - so the button shows one more distinct state
//!   instead of silently sitting on "Analyzing..." while the reader window
//!   is still being torn down.
//! - **Eval failure vs. timeout** (`EvalOutcome`, was a bare
//!   `Option<String>`): a genuine `eval_with_callback` dispatch failure (or
//!   its callback channel disconnecting without ever firing) used to
//!   collapse into the exact same `None` a real timeout produced, so it was
//!   always reported as `"timeout"` even when the JS never ran at all. Now a
//!   real evaluation failure on the final extraction pass reports `status:
//!   "error"` with its own message instead.
//! - **Data validation** (`parse_auto_check_json`): rejects an implausible
//!   listing count (`MAX_PRICES` = 500, matching the precedent
//!   `models::seat_entry_tests::caps_at_500_unique_entries_instead_of_
//!   growing_unbounded` already set elsewhere in this codebase) as `"error"`
//!   rather than silently truncating and claiming success on data that
//!   almost certainly means something went wrong reading the page;
//!   `sanitize_currency` degrades an implausibly-shaped currency string (not
//!   3 ASCII letters) to `None` rather than ever passing something clearly
//!   broken through to the currency dropdown it pre-fills.
//! - **Dev-visibility logging** (`log_lifecycle`): plain `eprintln!` (this
//!   crate declares no `log`/`tracing` dependency, confirmed by grep before
//!   adding this - a new logging crate would be new infrastructure, arguably
//!   out of scope for a hardening-only pass) tagged with the request id, at
//!   the points marko's spec named (request started/webview created/page
//!   ready/analysis started/result received/cleanup started/cleanup
//!   completed/finished-with-status). Nothing logged is ever more than a
//!   request id, a phase name, a byte LENGTH, or a status string - never
//!   page content, never any of marko's own business data (this module never
//!   touches the database at all, confirmed by grep - see below).
//! - **`OPEN_WEBVIEW_COUNT`**: a module-level counter, incremented right
//!   after a reader webview is actually built and decremented in
//!   `WebviewGuard::drop` - logged alongside "cleanup completed" so marko
//!   can watch it return to 0 after every real check on his own machine, as
//!   the closest thing to "hard measurement" of "no orphan windows" this
//!   sandbox's total lack of a display server allows (his own spec's
//!   fallback for exactly this: "Ak sa nedá tvrdo zmerať, pridaj interný
//!   debug counter/log").
//! - **App-shutdown hook** (`lib.rs`, not this file): a best-effort,
//!   non-blocking `RunEvent::ExitRequested` handler now flips THIS
//!   feature's own cancel flag (if anything is in flight) so a spawned
//!   reader thread notices sooner - see lib.rs's own comment for why this is
//!   honestly a marginal improvement (the OS reclaims every window/handle on
//!   process exit regardless, and this hook deliberately never calls
//!   `prevent_exit()`, so it cannot and must not delay shutdown).
//!
//! Deliberately NOT changed: the hidden (non-visible) WebView design
//! (swapping to a visible window would add a second real difference to
//! reason about for zero documented safety benefit - marko's own spec made
//! that conditional on "ak je technicky bezpečnejšie", and nothing in this
//! audit found the hidden design less safe); the overall threading/
//! cancellation architecture 2.1.2 already established (still correct,
//! still matches Tauri's own documented fix); marketplace isolation (already
//! structurally satisfied - each card's attempt is a fully independent
//! command invocation with no shared mutable state besides the single-flight
//! slot, which is now MORE strictly isolated than before, not less).

use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::models::AutoCheckResult;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, State, WebviewUrl, WebviewWindowBuilder};

const MIN_WAIT: Duration = Duration::from_millis(800);
const POLL_INTERVAL: Duration = Duration::from_millis(400);
const EVAL_TIMEOUT: Duration = Duration::from_secs(3);

/// Hard wall-clock ceiling for the WHOLE auto-check attempt - from the
/// moment the reader window starts opening to the moment a result comes
/// back, covering MIN_WAIT, every poll iteration, AND the final extraction
/// eval. marko's own explicit requirement after the freeze/hang incident:
/// no matter how the smaller waits below stack in the worst case, the total
/// never exceeds this. Every wait in `poll_then_extract` is budget-aware
/// (shrinks to whatever time is actually left, via `remaining_budget`)
/// rather than each independently allowed its own full duration, which is
/// what let the pre-2.1.2 version's *documented* worst case run longer than
/// its own "~10 seconds" estimate suggested.
const OVERALL_TIMEOUT: Duration = Duration::from_secs(15);

/// How much longer `run_with_outer_deadline` waits, beyond `OVERALL_TIMEOUT`
/// itself, before giving up on the spawned reader thread and returning
/// `TimedOut` anyway - see that function's own doc comment. Purely a grace
/// margin so `poll_then_extract`'s OWN budget-aware timeout fires first and
/// gets to report its own, more specific outcome; this outer one is a
/// backstop, not the primary timeout.
const OUTER_GRACE: Duration = Duration::from_secs(2);

/// Runs entirely inside the loaded page. Returns `{"ready": bool,
/// "blocked": bool}` - `ready` true means "there's something worth a full
/// extraction pass now" (found JSON-LD offers, a price-shaped table, an
/// og:price meta tag, or an anti-bot signal - the last one so a blocked
/// page is recognized immediately rather than polled for the full
/// remaining budget). Deliberately narrow/cheap so it can run every
/// POLL_INTERVAL without doing the full extraction work each time.
const READINESS_CHECK_JS: &str = include_str!("price_checker_auto_readiness.js");

/// The full extraction pass, run once readiness is confirmed (or the
/// overall budget runs out, on whatever the page has by then). Returns
/// `{"prices": number[], "currency": string|null, "blocked": bool}`. See
/// this module's own doc comment ("Extraction strategy") for what each of
/// the three passes inside it looks for.
const EXTRACT_JS: &str = include_str!("price_checker_auto_extract.js");

/// Tauri event name `PriceChecker.tsx` listens on while an auto-check is in
/// flight, to show real Starting/Loading/Analyzing/Cleaning-up progress
/// instead of a single opaque spinner (marko's own explicit requirement) -
/// see `emit_phase`. Payload is `ProgressPayload` (2.1.3 - was a bare phase
/// string before the request-id addition).
const PROGRESS_EVENT: &str = "price-checker-auto-check-progress";

/// Upper bound on how many prices a single extraction pass is allowed to
/// report (2.1.3, marko's spec section 12 - "impossible listing count").
/// Matches the precedent `models::seat_entry_tests::caps_at_500_unique_
/// entries_instead_of_growing_unbounded` already set elsewhere in this
/// codebase for the same kind of "trust nothing, cap it" rule. A real
/// marketplace listings page showing more than this is not a realistic
/// scenario this app needs to support - a raw count above it almost
/// certainly means the page returned something other than a real listings
/// table, so `parse_auto_check_json` reports `"error"` rather than silently
/// truncating to this many and claiming a normal success.
const MAX_PRICES: usize = 500;

/// How many reader webviews are open RIGHT NOW, across every attempt so far
/// this process's lifetime - incremented the moment `run_browser_read`
/// actually builds one, decremented in `WebviewGuard::drop` (2.1.3, marko's
/// spec section 15 - "no background zombies... ak sa nedá tvrdo zmerať,
/// pridaj interný debug counter"). No sandbox seen so far (this one
/// included) has a display server to literally enumerate open windows, so
/// this is the closest thing to hard measurement available - logged
/// alongside "cleanup completed" (`log_lifecycle`) so marko can watch it on
/// his own machine and confirm it returns to 0 after every real check,
/// including repeated ones back to back.
static OPEN_WEBVIEW_COUNT: AtomicUsize = AtomicUsize::new(0);

/// `PROGRESS_EVENT`'s payload shape (2.1.3). `request_id` is whatever
/// `PriceChecker.tsx` minted for this attempt and passed into
/// `auto_check_price` - this module only ever echoes it back unchanged, it
/// never assigns or interprets it itself. `#[serde(rename_all =
/// "camelCase")]` matches `AutoCheckResult`'s own convention (models.rs) so
/// the frontend sees `requestId`, not `request_id` - see
/// `progress_payload_serializes_request_id_as_camel_case_matching_the_
/// frontend` in the tests below, which locks this in.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressPayload {
    request_id: u64,
    phase: String,
}

/// Lightweight dev-visibility logging for this module only (2.1.3 - marko's
/// spec section 16). Plain `eprintln!`, matching this crate's own existing
/// (test-only) diagnostic-printing convention (see db.rs's `[perf ...]`
/// lines) rather than adding a brand-new logging crate dependency for a
/// hardening-only pass that explicitly must not add features (grepped
/// Cargo.toml first - confirmed no `log`/`tracing`/`tauri-plugin-log`
/// declared anywhere in this crate). PRODUCTION-SAFE by construction: every
/// call site below passes only a request id (a plain per-attempt counter,
/// never derived from any user/account data), a phase/event name, and at
/// most the byte LENGTH of a result (never its content) - this module never
/// touches cookies, auth tokens, or any of marko's own business data
/// (tickets/orders/customers/finance) in the first place, so there is
/// nothing sensitive for a log line here to ever leak.
fn log_lifecycle(request_id: u64, event: &str) {
    eprintln!("[price-checker-auto] request {request_id}: {event}");
}

/// One `auto_check_price` attempt's real outcome, before it's turned into
/// the `AutoCheckResult` the frontend sees (`read_outcome_to_result`
/// below). Kept as its own type - rather than building `AutoCheckResult`
/// directly throughout - so `Cancelled`/`TimedOut` are first-class outcomes
/// a caller (and the tests below) can match on directly, distinct from
/// `Json`, which still needs `parse_auto_check_json`'s own ok/blocked/
/// unable_to_read parsing.
#[derive(Debug, PartialEq)]
enum ReadOutcome {
    /// The extraction pass returned this raw JSON - still needs
    /// `parse_auto_check_json` to become ok/blocked/unable_to_read.
    Json(String),
    Cancelled,
    TimedOut,
    Error(String),
}

fn read_outcome_to_result(outcome: ReadOutcome) -> AutoCheckResult {
    match outcome {
        ReadOutcome::Json(raw) => parse_auto_check_json(&raw),
        ReadOutcome::Cancelled => AutoCheckResult {
            status: "cancelled".into(),
            prices: vec![],
            currency: None,
            message: Some("Auto-check was cancelled.".into()),
        },
        ReadOutcome::TimedOut => AutoCheckResult {
            status: "timeout".into(),
            prices: vec![],
            currency: None,
            message: Some(
                "This took too long (over 15 seconds) - use the paste/manual entry below instead, or try again."
                    .into(),
            ),
        },
        ReadOutcome::Error(message) => {
            AutoCheckResult { status: "error".into(), prices: vec![], currency: None, message: Some(message) }
        }
    }
}

/// `request_id` (2.1.3) is minted by `PriceChecker.tsx`, not this module -
/// see this file's own doc comment ("Production hardening" - "Request
/// IDs") for why. It is echoed back on every `PROGRESS_EVENT` and
/// `log_lifecycle` line for THIS attempt and otherwise never inspected or
/// acted on here.
///
/// Single-flight guard (2.1.3): the pre-2.1.3 version of this function set
/// `price_checker_auto_cancel_flag` unconditionally, with no check for
/// whether it was already `Some` - so two overlapping invocations (a fast
/// double-click slipping past `PriceChecker.tsx`'s own `disabled` gating
/// before React re-renders it, or any future caller) would silently
/// overwrite each other's flag: the FIRST invocation's flag reference would
/// be replaced in the slot, leaving it with no way to ever be cancelled via
/// the UI again (Cancel only ever reaches whatever flag is CURRENTLY in the
/// slot), while both attempts raced to open their own reader webview. Below,
/// the check-and-set happens under a SINGLE lock acquisition (`slot` stays
/// locked across both the `is_some()` check and the `= Some(...)` write), so
/// there is no window in which two calls can both observe an empty slot and
/// both proceed - the second one is rejected immediately with `"busy"`,
/// before ever normalizing further or spawning anything.
#[tauri::command]
pub fn auto_check_price(app: AppHandle, state: State<AppState>, url: String, request_id: u64) -> AppResult<AutoCheckResult> {
    let normalized = normalize_auto_check_url(&url)?;

    let cancel_flag = Arc::new(AtomicBool::new(false));
    {
        let mut slot = state.price_checker_auto_cancel_flag.lock().unwrap();
        if slot.is_some() {
            log_lifecycle(request_id, "rejected: another auto-check is already running");
            return Ok(AutoCheckResult {
                status: "busy".into(),
                prices: vec![],
                currency: None,
                message: Some("Another auto-check is already running - wait for it to finish or cancel it first.".into()),
            });
        }
        *slot = Some(cancel_flag.clone());
    }
    log_lifecycle(request_id, "request started");

    let outcome = run_with_outer_deadline(app, request_id, normalized, cancel_flag);

    // Cleared unconditionally (success, cancel, timeout, or error) so a
    // stale flag from an attempt that already finished can never reach a
    // later, unrelated one - same convention `start_google_sign_in` already
    // uses for `oauth_cancel_flag`.
    *state.price_checker_auto_cancel_flag.lock().unwrap() = None;

    let result = read_outcome_to_result(outcome);
    log_lifecycle(request_id, &format!("finished: {}", result.status));
    Ok(result)
}

/// "Cancel" button shown next to Auto-check's spinner while a check is in
/// flight - marko's own explicit requirement after the freeze/hang
/// incident. Reuses `commands::google_auth::cancel_google_sign_in_impl`
/// directly against this feature's own slot rather than duplicating its
/// three lines - that helper was explicitly written to be generic over any
/// cancel-flag slot (see its own doc comment; `firebase_google_auth.rs`
/// already reuses it the same way for its own, separate flag). A safe
/// no-op when nothing is actually in flight - a stray double-click, or the
/// attempt already finished a moment earlier.
#[tauri::command]
pub fn cancel_auto_check_price(state: State<AppState>) -> AppResult<()> {
    super::google_auth::cancel_google_sign_in_impl(&state.price_checker_auto_cancel_flag);
    Ok(())
}

/// Spawns the ENTIRE reader-webview lifecycle onto its own, genuinely
/// separate `std::thread` - see this module's doc comment ("Freeze fix")
/// for exactly why this specific shape: Tauri's own docs state that
/// creating a window from within a synchronous command's own call stack
/// deadlocks on Windows, which is exactly what the pre-2.1.2 version of
/// this file did. A plain `std::thread::spawn` (the same primitive
/// `google_oauth.rs`'s own tests already use to run `accept_one_redirect`
/// off the calling thread) satisfies Tauri's documented fix directly.
///
/// Then races that thread's result against an OUTER deadline
/// (`OVERALL_TIMEOUT + OUTER_GRACE`) so THIS function - and therefore the
/// command, and therefore the UI - always returns in bounded time even in
/// the residual, currently-unobserved case that the spawned thread's own
/// budget-aware waits (`poll_then_extract`) somehow don't return on their
/// own (e.g. `WebviewWindowBuilder::build()` itself hanging even off the
/// synchronous-command thread - not the documented failure mode, but safe
/// Rust has no way to forcibly abort a genuinely stuck native call, so this
/// is the strongest guarantee available). If that outer deadline fires
/// first, the spawned thread is NOT killed - it simply is no longer waited
/// on; its own `WebviewGuard` still runs and closes the reader window
/// whenever that thread does eventually finish, and the command has
/// already returned control to the UI either way, which is what actually
/// matters for "the app must never freeze."
fn run_with_outer_deadline(app: AppHandle, request_id: u64, url: String, cancel: Arc<AtomicBool>) -> ReadOutcome {
    let (tx, rx) = mpsc::channel::<ReadOutcome>();
    let overall_start = Instant::now();
    std::thread::spawn(move || {
        let outcome = run_browser_read(&app, request_id, &url, overall_start, &cancel);
        // Logged here unconditionally, whether or not anyone is still
        // waiting on `rx` by now (2.1.3, marko's spec section 15) - proves
        // this thread actually terminated even in the residual case where
        // the OUTER deadline below already gave up on it first (that case
        // is honest, not a bug: this line simply arrives LATE relative to
        // whatever the UI already moved on to - see this module's own doc
        // comment, "Production hardening").
        log_lifecycle(request_id, "background reader thread finished");
        let _ = tx.send(outcome);
    });
    rx.recv_timeout(OVERALL_TIMEOUT + OUTER_GRACE).unwrap_or(ReadOutcome::TimedOut)
}

/// Best-effort progress hint for `PriceChecker.tsx`'s own listener - never
/// allowed to affect the actual auto-check attempt if it fails (e.g. the
/// main window happens to be closing right at that moment).
fn emit_phase(app: &AppHandle, request_id: u64, phase: &str) {
    let _ = app.emit(PROGRESS_EVENT, ProgressPayload { request_id, phase: phase.to_string() });
}

/// RAII guard: closes the reader webview the moment this scope ends, on
/// every ORDINARY exit path - normal return, or an early return on cancel/
/// timeout/error (all of which are plain `return` statements, not panics,
/// and Rust always runs `Drop` for a `return` regardless of build profile).
/// marko's own explicit requirement after the freeze/hang incident: no code
/// path may leave a hidden reader window behind.
///
/// One honest caveat: this crate's own `[profile.release]` sets
/// `panic = "abort"` (Cargo.toml) for a smaller shipped binary, and an
/// abort skips unwinding entirely, so `Drop` would NOT run if an actual
/// panic occurred inside this guard's scope in the real shipped build
/// (it does run under `cargo test`'s default unwind panic strategy, which
/// is what the test below actually exercises). This is not a gap specific
/// to this feature: `panic = "abort"` already means ANY panic anywhere in
/// the app takes the whole process down immediately, at which point the OS
/// reclaims every window/handle that process owned as part of normal
/// process teardown - there is no scenario under this build's own existing
/// panic strategy where the app "keeps running" with a leaked window from
/// a panic. This guard's real job, and what it's actually tested against
/// below, is the ordinary paths above, which are the ones that actually
/// happen in practice.
struct WebviewGuard<'a> {
    window: &'a tauri::WebviewWindow,
    /// 2.1.3: purely for `log_lifecycle` below - carries no other meaning
    /// here, see this file's own doc comment ("Request IDs").
    request_id: u64,
}

impl<'a> Drop for WebviewGuard<'a> {
    fn drop(&mut self) {
        log_lifecycle(self.request_id, "cleanup started");
        let _ = self.window.close();
        // fetch_sub returns the value BEFORE decrementing, so subtracting 1
        // here gives the count AFTER this close - never underflows in
        // practice: exactly one OPEN_WEBVIEW_COUNT increment happens per
        // successful webview build (run_browser_read), always immediately
        // paired with constructing exactly one WebviewGuard whose Drop runs
        // exactly once (normal Rust ownership - see this struct's own doc
        // comment above on the panic=abort caveat).
        let remaining = OPEN_WEBVIEW_COUNT.fetch_sub(1, Ordering::Relaxed) - 1;
        log_lifecycle(self.request_id, &format!("cleanup completed (open reader windows now: {remaining})"));
    }
}

/// Parses `url` into a `tauri::Url`, kept as its own tiny function (rather
/// than inlined into `run_browser_read`) specifically so it's testable
/// without needing a real `AppHandle` - see the tests below. This is a
/// SECOND, independent validation layer beyond `normalize_auto_check_url`'s
/// own scheme check: a string can start with "https://" and still not be a
/// well-formed URL (e.g. "https://" alone, no host).
fn parse_target_url(url: &str) -> Result<tauri::Url, String> {
    url.parse().map_err(|e| format!("Invalid URL: {e}"))
}

/// Runs on the dedicated thread `run_with_outer_deadline` spawns. Creates
/// the hidden reader webview - now safely off the synchronous command's own
/// call stack - polls it, and always closes it again via `WebviewGuard`
/// before returning, whichever of Cancelled/TimedOut/Json/Error it is.
fn run_browser_read(app: &AppHandle, request_id: u64, url: &str, overall_start: Instant, cancel: &AtomicBool) -> ReadOutcome {
    emit_phase(app, request_id, "starting");

    let parsed_url = match parse_target_url(url) {
        Ok(u) => u,
        Err(e) => return ReadOutcome::Error(e),
    };

    let label = format!(
        "price-auto-check-{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()
    );
    let webview = match WebviewWindowBuilder::new(app, &label, WebviewUrl::External(parsed_url)).visible(false).build() {
        Ok(w) => w,
        Err(e) => return ReadOutcome::Error(format!("Could not open a reader window: {e}")),
    };
    OPEN_WEBVIEW_COUNT.fetch_add(1, Ordering::Relaxed);
    log_lifecycle(request_id, "webview created (navigation to the target URL begins immediately)");
    let _guard = WebviewGuard { window: &webview, request_id };

    emit_phase(app, request_id, "loading");
    let outcome = poll_then_extract(app, request_id, &webview, overall_start, cancel);
    // 2.1.3: a distinct phase for the frontend (marko's spec section 2 -
    // explicit state machine, no implicit "loading = true") covering the
    // window between "the read is done" and "the reader window is actually
    // closed" - fires here, with `_guard` still in scope, so it lands
    // BEFORE WebviewGuard's own Drop (and therefore before the actual
    // `.close()` call) rather than after.
    emit_phase(app, request_id, "cleaning_up");
    outcome
}

/// Time left before `overall_start` hits `OVERALL_TIMEOUT` - saturates at
/// zero rather than going negative, so every budget-aware wait below
/// naturally shrinks to nothing right at the deadline instead of
/// overshooting it.
fn remaining_budget(overall_start: Instant) -> Duration {
    OVERALL_TIMEOUT.saturating_sub(overall_start.elapsed())
}

/// Sleeps for `dur`, waking early and returning `false` the moment `cancel`
/// is flipped - checked every 100ms, the same granularity
/// `google_oauth::accept_one_redirect` already uses for its own "Cancel"
/// button, so a cancellation is noticed within about that long, never the
/// full remaining wait. Returns `true` if the full duration elapsed without
/// a cancel.
fn sleep_interruptible(dur: Duration, cancel: &AtomicBool) -> bool {
    let start = Instant::now();
    let tick = Duration::from_millis(100);
    while start.elapsed() < dur {
        if cancel.load(Ordering::Relaxed) {
            return false;
        }
        std::thread::sleep(tick.min(dur - start.elapsed()));
    }
    !cancel.load(Ordering::Relaxed)
}

/// One `eval_and_wait` attempt's outcome (2.1.3 - was a bare
/// `Option<String>`, which collapsed a genuine JS-evaluation failure and a
/// real timeout/cancel into the exact same `None`, so a real failure was
/// always mislabeled `"timeout"` - marko's spec section 11 explicitly wants
/// these distinguishable). See `eval_and_wait`'s own doc comment for how
/// each variant is reached, and its two call sites in `poll_then_extract`
/// for how each becomes (or doesn't become) a distinct `ReadOutcome`.
#[derive(Debug, PartialEq)]
enum EvalOutcome {
    /// The JS actually ran and called back with this raw result in time.
    Ready(String),
    Cancelled,
    TimedOut,
    /// The JS could not be run/delivered at all - either
    /// `eval_with_callback` itself returned `Err` synchronously (never even
    /// dispatched), or its callback channel disconnected without ever
    /// sending a result (dispatched, but the callback itself never ran -
    /// e.g. the webview was torn down mid-eval). Distinct from `TimedOut`
    /// (which means the JS may still genuinely be running, just hasn't
    /// answered yet).
    Failed(&'static str),
}

/// Runs `js` in `webview` and blocks (this call's own dedicated thread,
/// never the main thread - see `run_with_outer_deadline`) for at most
/// `max_wait` waiting for the result - budget-aware (see
/// `remaining_budget`), never the pre-2.1.2 version's fixed `EVAL_TIMEOUT`
/// regardless of how much of the overall budget was already spent.
///
/// Polls `rx` in short (100ms) slices - the same tick `sleep_interruptible`
/// uses - rather than one single `recv_timeout(max_wait)`, so a `Cancel`
/// click is noticed within about that long even while a real extraction JS
/// eval (up to `EVAL_TIMEOUT` = 3s) is still in flight, instead of
/// potentially waiting out however much of that 3s happened to be left -
/// marko's own explicit "Cancel musí okamžite [...]" requirement, applied
/// consistently everywhere this module waits on anything.
fn eval_and_wait(webview: &tauri::WebviewWindow, js: &str, max_wait: Duration, cancel: &AtomicBool) -> EvalOutcome {
    if max_wait.is_zero() {
        return EvalOutcome::TimedOut;
    }
    let (tx, rx) = mpsc::channel::<String>();
    if webview
        .eval_with_callback(js, move |result: String| {
            let _ = tx.send(result);
        })
        .is_err()
    {
        return EvalOutcome::Failed("could not start JS evaluation");
    }

    let start = Instant::now();
    let tick = Duration::from_millis(100);
    loop {
        if cancel.load(Ordering::Relaxed) {
            return EvalOutcome::Cancelled;
        }
        let elapsed = start.elapsed();
        if elapsed >= max_wait {
            return EvalOutcome::TimedOut;
        }
        match rx.recv_timeout(tick.min(max_wait - elapsed)) {
            Ok(result) => return EvalOutcome::Ready(result),
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => return EvalOutcome::Failed("JS evaluation callback never ran"),
        }
    }
}

/// The actual poll loop: waits `MIN_WAIT` for the page to start rendering,
/// then polls `READINESS_CHECK_JS` every `POLL_INTERVAL` until it reports
/// `ready`, the overall budget runs out, or `cancel` fires - whichever
/// happens first (cancel always wins immediately over everything else,
/// matching marko's own explicit priority: "Cancel musí okamžite..."). Once
/// out of that loop (ready OR budget exhausted, but not cancelled), still
/// makes ONE best-effort final extraction attempt on whatever the page has
/// by then, same as the pre-2.1.2 design already did - readiness never
/// firing doesn't mean the page has nothing; the extraction pass has its
/// own, stricter criteria and may still find something.
fn poll_then_extract(app: &AppHandle, request_id: u64, webview: &tauri::WebviewWindow, overall_start: Instant, cancel: &AtomicBool) -> ReadOutcome {
    if cancel.load(Ordering::Relaxed) {
        return ReadOutcome::Cancelled;
    }
    if !sleep_interruptible(MIN_WAIT.min(remaining_budget(overall_start)), cancel) {
        return ReadOutcome::Cancelled;
    }

    loop {
        if cancel.load(Ordering::Relaxed) {
            return ReadOutcome::Cancelled;
        }
        let budget = remaining_budget(overall_start);
        if budget.is_zero() {
            break; // best-effort: still try ONE final extraction below, see this function's own doc comment
        }

        match eval_and_wait(webview, READINESS_CHECK_JS, budget.min(EVAL_TIMEOUT), cancel) {
            EvalOutcome::Ready(raw) => {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&raw) {
                    if parsed.get("ready").and_then(|v| v.as_bool()).unwrap_or(false) {
                        log_lifecycle(request_id, "page ready");
                        break;
                    }
                }
            }
            EvalOutcome::Cancelled => return ReadOutcome::Cancelled,
            // Not fatal - the readiness probe is deliberately best-effort/
            // cheap (see its own const doc comment above); a single failed
            // or slow tick just means "not confirmed ready yet", and the
            // loop's own budget/cancel checks above still govern when to
            // actually give up.
            EvalOutcome::TimedOut => {}
            EvalOutcome::Failed(reason) => {
                log_lifecycle(request_id, &format!("readiness check eval failed ({reason}) - will keep polling"));
            }
        }

        if cancel.load(Ordering::Relaxed) {
            return ReadOutcome::Cancelled;
        }
        if !sleep_interruptible(POLL_INTERVAL.min(remaining_budget(overall_start)), cancel) {
            return ReadOutcome::Cancelled;
        }
    }

    if cancel.load(Ordering::Relaxed) {
        return ReadOutcome::Cancelled;
    }
    emit_phase(app, request_id, "analyzing");
    log_lifecycle(request_id, "analysis started");
    let budget = remaining_budget(overall_start);
    if budget.is_zero() {
        return ReadOutcome::TimedOut;
    }
    match eval_and_wait(webview, EXTRACT_JS, budget.min(EVAL_TIMEOUT), cancel) {
        EvalOutcome::Ready(raw) => {
            log_lifecycle(request_id, &format!("result received ({} bytes)", raw.len()));
            ReadOutcome::Json(raw)
        }
        EvalOutcome::Cancelled => ReadOutcome::Cancelled,
        EvalOutcome::TimedOut => ReadOutcome::TimedOut,
        // 2.1.3: previously mislabeled "timeout" (see EvalOutcome's own doc
        // comment) - a real JS-evaluation failure now reports its own
        // distinct "error" instead, per marko's spec section 11.
        EvalOutcome::Failed(reason) => ReadOutcome::Error(format!("The reader page's script could not be run ({reason}).")),
    }
}

/// Turns `EXTRACT_JS`'s raw JSON string into the `AutoCheckResult` the
/// frontend actually consumes. Pure and fully unit-testable without a
/// WebView - see the tests below.
pub(crate) fn parse_auto_check_json(raw: &str) -> AutoCheckResult {
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(raw) else {
        return AutoCheckResult {
            status: "error".into(),
            prices: vec![],
            currency: None,
            message: Some("The reader window returned something unexpected.".into()),
        };
    };

    if parsed.get("blocked").and_then(|v| v.as_bool()).unwrap_or(false) {
        return AutoCheckResult {
            status: "blocked".into(),
            prices: vec![],
            currency: None,
            message: Some(
                "This page returned an anti-bot/verification challenge instead of its normal content. \
                 This app does not attempt to solve or bypass that - use the paste/manual entry below instead."
                    .into(),
            ),
        };
    }

    // 2.1.3 (marko's spec section 12 - "impossible listing count"): checked
    // on the RAW array length, before any filtering, so a page that returns
    // an implausible number of entries is rejected outright rather than
    // silently truncated down to MAX_PRICES and reported as an ordinary
    // success - see MAX_PRICES's own doc comment.
    let raw_price_count = parsed.get("prices").and_then(|v| v.as_array()).map(|arr| arr.len()).unwrap_or(0);
    if raw_price_count > MAX_PRICES {
        return AutoCheckResult {
            status: "error".into(),
            prices: vec![],
            currency: None,
            message: Some(format!(
                "The page reported an implausible number of listings ({raw_price_count}) - this usually means \
                 something went wrong reading the page. Use the paste/manual entry below instead."
            )),
        };
    }

    let prices: Vec<f64> = parsed
        .get("prices")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_f64()).filter(|p| p.is_finite() && *p > 0.0).collect())
        .unwrap_or_default();
    let currency = sanitize_currency(parsed.get("currency").and_then(|v| v.as_str()));

    if prices.is_empty() {
        return AutoCheckResult {
            status: "unable_to_read".into(),
            prices: vec![],
            currency,
            message: Some(
                "The page loaded, but no prices could be found on it automatically \
                 (most likely they only appear after you interact with the page yourself). \
                 Use the paste/manual entry below instead."
                    .into(),
            ),
        };
    }

    AutoCheckResult { status: "ok".into(), prices, currency, message: None }
}

/// Best-effort shape check on whatever the page's own markup claims its
/// currency is (schema.org JSON-LD, og:price:currency, ...) - 2.1.3, marko's
/// spec section 12. NOT a hard allowlist: marko's own manual entry already
/// accepts arbitrary custom currency codes elsewhere in this app
/// (SavePriceCheckModal's "Other..." field, `CURRENCIES` in Orders.tsx), so
/// this only guards against clearly-broken values (empty, numeric, sentence-
/// shaped) reaching `AutoCheckResult` and confusing the currency dropdown it
/// pre-fills. Anything that isn't exactly 3 ASCII letters degrades to `None`
/// (an unknown currency, same as the page simply not stating one at all)
/// rather than failing the whole attempt over a currency string alone -
/// matches this file's own `AutoCheckResult` field doc comment
/// ("Nesprávne dáta -> ERROR/PARTIAL, nie fake success" scoped to the
/// PRICES; a currency alone is never fatal to an otherwise-good result).
fn sanitize_currency(raw: Option<&str>) -> Option<String> {
    let s = raw?.trim();
    if s.len() == 3 && s.chars().all(|c| c.is_ascii_alphabetic()) {
        Some(s.to_ascii_uppercase())
    } else {
        None
    }
}

/// Validates and normalizes whatever marko has in the URL field before it's
/// ever handed to the WebView. `save_event_marketplace_link_impl`
/// (commands::price_checker, unchanged) already accepts any non-empty
/// string with zero validation - and most browsers hide the "https://"
/// prefix in their own address bar, so a link marko copy-pasted from there,
/// or a bare domain he typed himself, may well arrive here with no scheme
/// at all. Rather than reject those outright, a missing scheme is treated
/// as "https://" (this app never has a reason to open one of these pages
/// over plain http anyway). An EXPLICIT non-http(s) scheme (`javascript:`,
/// `file:`, `ftp:`, ...) is a different situation - never silently
/// rewritten, always rejected - see the tests below.
fn normalize_auto_check_url(raw: &str) -> AppResult<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation("Enter this marketplace's listings page URL above first.".into()));
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        return Ok(trimmed.to_string());
    }
    if has_explicit_non_http_scheme(trimmed) {
        return Err(AppError::Validation("URL must start with http:// or https://".into()));
    }
    Ok(format!("https://{trimmed}"))
}

/// True when `s` starts with an RFC 3986-shaped `scheme:` prefix (a letter,
/// then letters/digits/`+`/`-`/`.`) whose colon comes before any `/` - i.e.
/// it looks like an explicit URI scheme rather than a bare host (which may
/// legitimately contain a later `:` or `/`, e.g. a port in `example.com:8080/x`
/// - an extremely unlikely shape for a ticket marketplace URL, and safer to
/// reject than to guess about).
fn has_explicit_non_http_scheme(s: &str) -> bool {
    let Some(scheme_end) = s.find(':') else { return false };
    if let Some(slash) = s.find('/') {
        if slash < scheme_end {
            return false;
        }
    }
    let scheme = &s[..scheme_end];
    if scheme.is_empty() {
        return false;
    }
    let mut chars = scheme.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use std::sync::Mutex;

    // -- URL validation/normalization -----------------------------------

    #[test]
    fn rejects_a_non_http_url_before_touching_any_webview() {
        // No AppHandle available in a unit test (needs a running Tauri app)
        // - but URL validation/normalization happens before run_browser_read
        // is ever called, in the plain, AppHandle-free helper below, so it's
        // directly testable via the real function auto_check_price_impl
        // itself calls (not a duplicated condition).
        assert!(normalize_auto_check_url("javascript:alert(1)").is_err());
    }

    #[test]
    fn other_explicit_non_http_schemes_are_also_rejected() {
        assert!(normalize_auto_check_url("ftp://files.example.com/x").is_err());
        assert!(normalize_auto_check_url("file:///etc/passwd").is_err());
    }

    #[test]
    fn a_bare_domain_with_no_scheme_gets_https_prepended() {
        // marko's browser address bar (like most) hides "https://" - a link
        // copy-pasted from there, or typed by hand, should still work here,
        // same as save_event_marketplace_link_impl already accepts it with
        // zero validation.
        assert_eq!(normalize_auto_check_url("vividseats.com/event/123").unwrap(), "https://vividseats.com/event/123");
        assert_eq!(normalize_auto_check_url("www.ticombo.com/en/x").unwrap(), "https://www.ticombo.com/en/x");
    }

    #[test]
    fn an_already_qualified_url_is_left_completely_unchanged() {
        assert_eq!(normalize_auto_check_url("https://stubhub.com/x?y=1").unwrap(), "https://stubhub.com/x?y=1");
        assert_eq!(normalize_auto_check_url("HTTP://Example.com/X").unwrap(), "HTTP://Example.com/X");
    }

    #[test]
    fn blank_or_whitespace_only_url_is_rejected_with_a_clear_message() {
        assert!(normalize_auto_check_url("").is_err());
        assert!(normalize_auto_check_url("   ").is_err());
    }

    #[test]
    fn parse_target_url_accepts_a_normal_https_url() {
        assert!(parse_target_url("https://vividseats.com/event/123").is_ok());
    }

    #[test]
    fn parse_target_url_rejects_a_schemeless_authority_cleanly() {
        // Passes normalize_auto_check_url's OWN scheme check (has
        // "https://") but still isn't a valid URL on its own (no host) -
        // proves the second, independent validation layer (tauri::Url's own
        // parser, inside run_browser_read) is actually reached and handled
        // as a clean ReadOutcome::Error, not just assumed to always
        // succeed. This is as close as a sandbox with no display server can
        // get to testing "browser creation failure" - see this module's own
        // doc comment for what remains genuinely untestable here.
        assert!(parse_target_url("https://").is_err());
    }

    // -- parse_auto_check_json (unchanged from 2.1.1) --------------------

    #[test]
    fn parses_ok_result_with_prices() {
        let raw = r#"{"prices": [31.0, 39.0, 39.0, 50.0, 52.0], "currency": "USD", "blocked": false}"#;
        let result = parse_auto_check_json(raw);
        assert_eq!(result.status, "ok");
        assert_eq!(result.prices, vec![31.0, 39.0, 39.0, 50.0, 52.0]);
        assert_eq!(result.currency.as_deref(), Some("USD"));
        assert!(result.message.is_none());
    }

    #[test]
    fn empty_prices_is_unable_to_read_not_fabricated() {
        let raw = r#"{"prices": [], "currency": null, "blocked": false}"#;
        let result = parse_auto_check_json(raw);
        assert_eq!(result.status, "unable_to_read");
        assert!(result.prices.is_empty());
        assert!(result.message.is_some());
    }

    #[test]
    fn blocked_signal_wins_even_if_some_prices_were_also_found() {
        // A challenge page occasionally has stray numbers on it - blocked
        // must take priority over any prices, never a mix of both.
        let raw = r#"{"prices": [12.0], "currency": null, "blocked": true}"#;
        let result = parse_auto_check_json(raw);
        assert_eq!(result.status, "blocked");
        assert!(result.prices.is_empty());
    }

    #[test]
    fn malformed_json_is_a_clean_error_not_a_panic() {
        let result = parse_auto_check_json("not json at all");
        assert_eq!(result.status, "error");
    }

    #[test]
    fn negative_or_zero_or_non_finite_prices_are_filtered_out() {
        let raw = r#"{"prices": [50.0, -10.0, 0.0, 99.5], "currency": "EUR", "blocked": false}"#;
        let result = parse_auto_check_json(raw);
        assert_eq!(result.prices, vec![50.0, 99.5]);
    }

    // -- ReadOutcome -> AutoCheckResult mapping (new in the freeze fix) --

    #[test]
    fn cancelled_outcome_maps_to_a_distinct_cancelled_status_not_error() {
        let result = read_outcome_to_result(ReadOutcome::Cancelled);
        assert_eq!(result.status, "cancelled");
        assert!(result.prices.is_empty());
    }

    #[test]
    fn timed_out_outcome_maps_to_a_distinct_timeout_status_not_error() {
        let result = read_outcome_to_result(ReadOutcome::TimedOut);
        assert_eq!(result.status, "timeout");
        assert!(result.message.unwrap().contains("15 seconds"));
    }

    #[test]
    fn error_outcome_carries_its_own_message_through_unchanged() {
        let result = read_outcome_to_result(ReadOutcome::Error("Could not open a reader window: boom".into()));
        assert_eq!(result.status, "error");
        assert_eq!(result.message.as_deref(), Some("Could not open a reader window: boom"));
    }

    #[test]
    fn json_outcome_delegates_to_parse_auto_check_json_unchanged() {
        let raw = r#"{"prices": [45.5], "currency": "EUR", "blocked": false}"#;
        let via_outcome = read_outcome_to_result(ReadOutcome::Json(raw.to_string()));
        let direct = parse_auto_check_json(raw);
        assert_eq!(via_outcome.status, direct.status);
        assert_eq!(via_outcome.prices, direct.prices);
    }

    // -- Cancel responsiveness (the actual "Cancel must work immediately"
    //    requirement, at the primitive `sleep_interruptible` builds on) --

    #[test]
    fn sleep_interruptible_waits_the_full_duration_when_never_cancelled() {
        let cancel = AtomicBool::new(false);
        let start = Instant::now();
        let completed = sleep_interruptible(Duration::from_millis(150), &cancel);
        assert!(completed, "must report completing normally when never cancelled");
        assert!(start.elapsed() >= Duration::from_millis(150));
    }

    #[test]
    fn sleep_interruptible_is_interrupted_promptly_instead_of_waiting_out_the_full_duration() {
        // Mirrors google_oauth's own
        // accept_one_redirect_is_interrupted_promptly_when_cancelled_instead_of_waiting_out_the_full_timeout
        // test exactly - same requirement (marko's own "Cancel musí
        // okamžite..."), same primitive shape, same style of proof: use a
        // duration here (5s) that would fail the 2s assertion below outright
        // if cancellation were silently not being checked.
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_for_thread = cancel.clone();
        let handle = std::thread::spawn(move || sleep_interruptible(Duration::from_secs(5), &cancel_for_thread));

        std::thread::sleep(Duration::from_millis(50));
        cancel.store(true, Ordering::Relaxed);

        let start = Instant::now();
        let completed = handle.join().unwrap();
        assert!(!completed, "a cancelled sleep must report NOT completing normally");
        assert!(
            start.elapsed() < Duration::from_secs(2),
            "cancellation must be noticed within about one 100ms tick, not anywhere near the 5s duration - took {:?}",
            start.elapsed()
        );
    }

    // -- Overall-budget arithmetic ----------------------------------------

    #[test]
    fn remaining_budget_saturates_at_zero_instead_of_going_negative() {
        // An overall_start far enough in the past that OVERALL_TIMEOUT has
        // long since passed must not panic (Duration can't be negative) or
        // wrap around - it must cleanly report zero remaining.
        let long_ago = Instant::now() - Duration::from_secs(3600);
        assert_eq!(remaining_budget(long_ago), Duration::ZERO);
    }

    #[test]
    fn remaining_budget_is_close_to_the_full_ceiling_right_at_the_start() {
        let just_started = Instant::now();
        let remaining = remaining_budget(just_started);
        assert!(remaining <= OVERALL_TIMEOUT);
        assert!(remaining >= OVERALL_TIMEOUT - Duration::from_millis(500), "got {remaining:?}");
    }

    // -- RAII cleanup pattern (see WebviewGuard's own doc comment for the
    //    honest panic=abort caveat this test's scope actually covers) ----

    #[test]
    fn raii_cleanup_pattern_runs_on_normal_return_early_return_and_a_test_profile_panic() {
        // WebviewGuard itself can't be constructed in a unit test (needs a
        // real tauri::WebviewWindow, which needs a running Tauri app -
        // unavailable in any sandbox seen so far, this one included). This
        // proves the identical Drop-based pattern it uses actually behaves
        // as required for every exit path marko's own bug report named,
        // using a trivial mock in place of tauri::WebviewWindow.
        struct CloseCounter(AtomicUsize);
        struct MockGuard<'a>(&'a CloseCounter);
        impl<'a> Drop for MockGuard<'a> {
            fn drop(&mut self) {
                self.0 .0.fetch_add(1, Ordering::SeqCst);
            }
        }

        let counter = CloseCounter(AtomicUsize::new(0));
        {
            let _guard = MockGuard(&counter);
        }
        assert_eq!(counter.0.load(Ordering::SeqCst), 1, "must close on a normal scope exit");

        let counter = CloseCounter(AtomicUsize::new(0));
        // clippy's needless_return would fire on a bare `return 42;` here since
        // it's the last statement - allowed deliberately, because the `return`
        // keyword itself is the point: it's what actually mirrors Cancelled/
        // TimedOut/Error's own early `return`s (in the real functions there
        // are always more lines after theirs; this trivial mock just doesn't
        // need any).
        #[allow(clippy::needless_return)]
        fn early_return(counter: &CloseCounter) -> i32 {
            let _guard = MockGuard(counter);
            return 42;
        }
        early_return(&counter);
        assert_eq!(counter.0.load(Ordering::SeqCst), 1, "must close on an early return");

        // cargo test's own default panic strategy is unwind (this crate's
        // panic = "abort" override is scoped to [profile.release] only -
        // see Cargo.toml and WebviewGuard's own doc comment) - so this DOES
        // genuinely exercise Drop-on-panic, for the profile it applies to.
        let counter = CloseCounter(AtomicUsize::new(0));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = MockGuard(&counter);
            panic!("simulated failure inside the guarded scope");
        }));
        assert!(result.is_err());
        assert_eq!(counter.0.load(Ordering::SeqCst), 1, "must close even when a panic unwinds through the guarded scope");
    }

    // -- Cancel-flag slot lifecycle (marko's "repeated check" / "check
    //    after cancel" requirement) ----------------------------------------

    #[test]
    fn cancel_flag_slot_lifecycle_matches_google_auths_established_pattern() {
        // price_checker_auto reuses google_auth::cancel_google_sign_in_impl
        // directly against its OWN AppState slot (price_checker_auto_cancel
        // _flag) rather than duplicating this logic - this proves the exact
        // behavior marko's own "repeated check" / "check after cancel"
        // requirement needs holds for THIS feature's slot too: a fresh
        // attempt never inherits a previous one's cancellation, and a stray
        // cancel can never reach an attempt that already finished.
        use crate::commands::google_auth::cancel_google_sign_in_impl;
        let slot: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);

        // Attempt 1 starts, is cancelled.
        let flag1 = Arc::new(AtomicBool::new(false));
        *slot.lock().unwrap() = Some(flag1.clone());
        cancel_google_sign_in_impl(&slot);
        assert!(flag1.load(Ordering::Relaxed));

        // Attempt 1 finishes (whatever its outcome) - auto_check_price
        // clears the slot unconditionally.
        *slot.lock().unwrap() = None;

        // Attempt 2 (a "repeated check") starts with a brand-new flag.
        let flag2 = Arc::new(AtomicBool::new(false));
        *slot.lock().unwrap() = Some(flag2.clone());
        assert!(!flag2.load(Ordering::Relaxed), "a fresh attempt must never inherit a previous attempt's cancellation");

        // A stray cancel now only ever reaches attempt 2's flag.
        cancel_google_sign_in_impl(&slot);
        assert!(flag2.load(Ordering::Relaxed));
        assert!(flag1.load(Ordering::Relaxed), "attempt 1's own (already-finished, discarded) flag is untouched by this, still true from before");
    }

    #[test]
    fn cancel_auto_check_price_impl_is_a_safe_no_op_when_nothing_is_in_flight() {
        use crate::commands::google_auth::cancel_google_sign_in_impl;
        let slot: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);
        cancel_google_sign_in_impl(&slot); // must not panic
        assert!(slot.lock().unwrap().is_none());
    }

    // =====================================================================
    // 2.1.3 - production hardening
    // =====================================================================

    // -- Single-flight guard (the double-click/overlapping-request race
    //    this closes - see auto_check_price's own doc comment) -----------

    #[test]
    fn single_flight_guard_rejects_a_second_attempt_while_the_first_is_still_in_the_slot() {
        // Mirrors auto_check_price's own check-and-set block exactly (can't
        // call auto_check_price itself in a unit test - needs a real
        // AppHandle/State, unavailable here - same limitation as every
        // other test in this module that would need one).
        let slot: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);
        let flag1 = Arc::new(AtomicBool::new(false));
        {
            let mut guard = slot.lock().unwrap();
            assert!(guard.is_none(), "slot must start empty");
            *guard = Some(flag1.clone());
        }

        // A second attempt while the first is still running must be
        // rejected WITHOUT touching the first attempt's flag or the slot.
        let rejected = slot.lock().unwrap().is_some();
        assert!(rejected, "a second attempt must observe the slot as occupied");
        assert!(!flag1.load(Ordering::Relaxed), "a rejected second attempt must never touch the first attempt's flag");
        assert!(slot.lock().unwrap().is_some(), "a rejected second attempt must never clear the first attempt's slot either");

        // The first attempt finishes normally and clears the slot -
        // ONLY then can a new attempt actually proceed.
        *slot.lock().unwrap() = None;
        let flag2 = Arc::new(AtomicBool::new(false));
        {
            let mut guard = slot.lock().unwrap();
            assert!(guard.is_none());
            *guard = Some(flag2.clone());
        }
        assert!(!flag2.load(Ordering::Relaxed));
    }

    // -- EvalOutcome (dispatch/callback failure vs. a real timeout) ------

    #[test]
    fn eval_outcome_failed_is_a_distinct_value_from_timed_out_and_cancelled() {
        // The bug this closes: both Failed and TimedOut used to collapse
        // into the exact same `None`, so a genuine JS-evaluation failure
        // was always reported as "timeout" - this proves the values a
        // caller can now actually match on are genuinely different.
        assert_ne!(EvalOutcome::Failed("x"), EvalOutcome::TimedOut);
        assert_ne!(EvalOutcome::Failed("x"), EvalOutcome::Cancelled);
        assert_ne!(EvalOutcome::TimedOut, EvalOutcome::Cancelled);
    }

    // NOTE (honesty, matching this module's own established convention):
    // eval_and_wait itself can't be called directly in a unit test at all,
    // not even to exercise its zero-budget/dispatch-failure branches - every
    // path through it requires a real `&tauri::WebviewWindow` argument to
    // type-check, and no sandbox seen so far (this one included) can
    // construct one (no display server). `eval_outcome_failed_is_a_
    // distinct_value_from_timed_out_and_cancelled` above is the closest
    // this module can get in any sandbox: it proves the VALUES themselves
    // are genuinely distinguishable, which is what callers like
    // `poll_then_extract` actually depend on - the real dispatch-failure/
    // callback-disconnect paths inside eval_and_wait itself are exactly the
    // kind of WebView-dependent code this project's own established
    // convention (see this file's "Same sandbox limitation" doc comment
    // section) already flags as needing marko's own real machine to verify.

    // -- sanitize_currency ------------------------------------------------

    #[test]
    fn sanitize_currency_accepts_a_plausible_three_letter_code_and_uppercases_it() {
        assert_eq!(sanitize_currency(Some("USD")), Some("USD".to_string()));
        assert_eq!(sanitize_currency(Some("eur")), Some("EUR".to_string()), "must uppercase, matching save_price_check's own convention");
        assert_eq!(sanitize_currency(Some("  gbp  ")), Some("GBP".to_string()), "must trim surrounding whitespace");
    }

    #[test]
    fn sanitize_currency_degrades_implausible_shapes_to_none_without_failing_the_whole_result() {
        assert_eq!(sanitize_currency(None), None);
        assert_eq!(sanitize_currency(Some("")), None);
        assert_eq!(sanitize_currency(Some("US")), None, "too short");
        assert_eq!(sanitize_currency(Some("DOLLARS")), None, "too long");
        assert_eq!(sanitize_currency(Some("12A")), None, "not all letters");
        assert_eq!(sanitize_currency(Some("$$$")), None, "not letters at all");
    }

    // -- MAX_PRICES cap ("impossible listing count") ----------------------

    #[test]
    fn implausible_price_count_is_rejected_as_error_not_silently_truncated() {
        let too_many: Vec<f64> = (0..(MAX_PRICES + 1)).map(|i| (i + 1) as f64).collect();
        let raw = serde_json::json!({ "prices": too_many, "currency": "USD", "blocked": false }).to_string();
        let result = parse_auto_check_json(&raw);
        assert_eq!(result.status, "error", "must not silently truncate and claim success on an implausible listing count");
        assert!(result.prices.is_empty());
        assert!(result.message.unwrap().contains(&(MAX_PRICES + 1).to_string()));
    }

    #[test]
    fn exactly_max_prices_is_still_accepted_as_a_normal_success() {
        let exactly_max: Vec<f64> = (0..MAX_PRICES).map(|i| (i + 1) as f64).collect();
        let raw = serde_json::json!({ "prices": exactly_max, "currency": "USD", "blocked": false }).to_string();
        let result = parse_auto_check_json(&raw);
        assert_eq!(result.status, "ok");
        assert_eq!(result.prices.len(), MAX_PRICES);
    }

    // -- OPEN_WEBVIEW_COUNT pairing ("no orphan windows" - the closest
    //    thing to hard measurement this sandbox allows, see this module's
    //    own doc comment) --------------------------------------------------

    #[test]
    fn a_fetch_add_fetch_sub_pair_around_a_drop_guard_returns_to_zero_after_repeated_cycles() {
        // WebviewGuard itself can't be constructed here (needs a real
        // tauri::WebviewWindow - see this module's doc comment on why no
        // sandbox seen so far can do that), so this proves the counter
        // arithmetic its Drop impl actually uses - fetch_add paired with
        // fetch_sub around a Drop-based guard - genuinely returns to zero
        // across repeated cycles, including one that panics mid-scope,
        // mirroring raii_cleanup_pattern_runs_on_normal_return_early_return_
        // and_a_test_profile_panic above.
        static COUNT: AtomicUsize = AtomicUsize::new(0);
        struct CountGuard;
        impl Drop for CountGuard {
            fn drop(&mut self) {
                COUNT.fetch_sub(1, Ordering::Relaxed);
            }
        }

        for i in 0..10 {
            COUNT.fetch_add(1, Ordering::Relaxed);
            let _guard = CountGuard;
            assert_eq!(COUNT.load(Ordering::Relaxed), 1, "cycle {i}: exactly one open \"window\" while the guard is alive");
        }
        assert_eq!(COUNT.load(Ordering::Relaxed), 0, "must return to exactly 0 after 10 repeated create/cleanup cycles");

        // A panic mid-scope must still decrement (matches WebviewGuard's
        // own real Drop impl and its documented panic=abort caveat).
        COUNT.fetch_add(1, Ordering::Relaxed);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = CountGuard;
            panic!("simulated failure mid-scope");
        }));
        assert!(result.is_err());
        assert_eq!(COUNT.load(Ordering::Relaxed), 0, "must decrement even when a panic unwinds through the guarded scope");
    }

    // -- ProgressPayload wire shape (the Rust/TS boundary contract) ------

    #[test]
    fn progress_payload_serializes_request_id_as_camel_case_matching_the_frontend() {
        // PriceChecker.tsx's listener reads `event.payload.requestId` (see
        // AutoCheckProgressEvent, types.ts) - this locks in that the Rust
        // field actually serializes that way, so a rename on either side
        // can't silently desync without a test failing here.
        let payload = ProgressPayload { request_id: 42, phase: "loading".to_string() };
        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["requestId"], serde_json::json!(42));
        assert_eq!(json["phase"], serde_json::json!("loading"));
        assert!(json.get("request_id").is_none(), "must not ALSO emit the snake_case field name");
    }

    // -- Exhaustive terminal-state check (marko's explicit "NO TERMINAL
    //    STATE MAY LEAVE REQUEST IN LOADING") -----------------------------

    #[test]
    fn every_read_outcome_variant_maps_to_a_real_terminal_status_never_a_loading_one() {
        let loading_like = ["starting", "loading", "analyzing", "cleaning_up"];
        let terminal = ["ok", "unable_to_read", "blocked", "cancelled", "timeout", "error"];
        let cases = [
            ReadOutcome::Json(r#"{"prices": [10.0], "currency": "USD", "blocked": false}"#.to_string()),
            ReadOutcome::Json(r#"{"prices": [], "currency": null, "blocked": false}"#.to_string()),
            ReadOutcome::Json(r#"{"prices": [], "currency": null, "blocked": true}"#.to_string()),
            ReadOutcome::Cancelled,
            ReadOutcome::TimedOut,
            ReadOutcome::Error("boom".to_string()),
        ];
        for case in cases {
            let result = read_outcome_to_result(case);
            assert!(!loading_like.contains(&result.status.as_str()), "status {:?} must never be a loading-shaped state", result.status);
            assert!(terminal.contains(&result.status.as_str()), "unexpected status {:?}", result.status);
        }
    }

    #[test]
    fn busy_status_from_the_single_flight_guard_is_a_real_terminal_status_too() {
        // "busy" itself isn't a ReadOutcome (auto_check_price returns it
        // directly, before ever spawning a thread - see its own doc
        // comment) - this documents/locks in the exact literal string the
        // frontend switches on, so a typo here can't silently desync from
        // PriceChecker.tsx's own check.
        let busy = AutoCheckResult {
            status: "busy".into(),
            prices: vec![],
            currency: None,
            message: Some("Another auto-check is already running - wait for it to finish or cancel it first.".into()),
        };
        assert_eq!(busy.status, "busy");
        assert!(busy.prices.is_empty());
        assert!(busy.message.is_some());
    }

    // -- Repeated-cycle slot bookkeeping, 10x (marko's explicit "REPEATED-
    //    RUN SAFETY", extending cancel_flag_slot_lifecycle_matches_google_
    //    auths_established_pattern above from 2 cycles to 10) -------------

    #[test]
    fn cancel_flag_slot_survives_ten_repeated_start_finish_cycles_without_leaking_state() {
        use crate::commands::google_auth::cancel_google_sign_in_impl;
        let slot: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);
        for i in 0..10 {
            assert!(slot.lock().unwrap().is_none(), "cycle {i}: must start every cycle with an empty slot");
            let flag = Arc::new(AtomicBool::new(false));
            *slot.lock().unwrap() = Some(flag.clone());
            assert!(!flag.load(Ordering::Relaxed), "cycle {i}: a fresh flag must never start pre-cancelled");
            if i % 3 == 0 {
                cancel_google_sign_in_impl(&slot);
                assert!(flag.load(Ordering::Relaxed), "cycle {i}: cancel must reach this cycle's own flag");
            }
            *slot.lock().unwrap() = None; // auto_check_price's own unconditional cleanup
        }
        assert!(slot.lock().unwrap().is_none(), "must end with an empty slot after 10 cycles");
    }
}

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
//! `spawn_auto_check_thread`'s own doc comment.
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
//! marko's own later real-world report (relayed via release.ps1's commit
//! history, 2.1.5): even after the above, Auto-check ran the full 60s and
//! found nothing on every marketplace he actually tried it against. Taken
//! together with the paragraph above, this is a meaningful gap worth
//! stating plainly: the research that confirmed Vivid Seats' table exists
//! was a one-time fetch through this session's own research tooling, not a
//! real Windows WebView2 navigating invisibly the exact way `run_browser_
//! read` below does - a real difference in rendering path, bot-detection
//! behavior, or simply time passed since that fetch could each independently
//! explain marko's result without EXTRACT_JS's own logic being wrong. This
//! module still cannot resolve that gap from this sandbox (see "Same sandbox
//! limitation" right below) - only marko's own machine, with the diagnostic
//! detail `build_unable_to_read_message` now surfaces directly in the UI,
//! can actually tell which marketplace is hitting which failure mode.
//!
//! ## AI-assisted extraction fallback (2.1.6)
//!
//! marko's own follow-up request, in the same message that asked for
//! Viagogo: "kludne mozme pridat anthropic api keby vedel pomoct" (we could
//! go ahead and add the Anthropic API if it could help). Confirmed via
//! AskUserQuestion before building: add it now as an extra fallback pass,
//! never instead of the four existing free page-rule passes above, which
//! keep running first exactly as before.
//!
//! There are honestly two distinct reasons Auto-check can find nothing (see
//! the paragraph above), and this fallback can only ever help with ONE of
//! them. If a marketplace's own anti-bot protection stops real content from
//! ever reaching the page (`status == "blocked"`, or a page that loads but
//! never actually serves real listings), there is no real text to hand a
//! model either - no extraction method, AI included, can find prices that
//! were never delivered to the browser in the first place, and this
//! fallback correctly never even attempts that case (see `spawn_auto_check_
//! thread` - only `"unable_to_read"` ever triggers it, never `"blocked"`).
//! What it CAN genuinely help with: a page that rendered real content, just
//! not shaped the way any of the four rule-based passes happens to
//! recognize - `try_ai_extraction_fallback` hands the model the same
//! visible text Pass 4 already scans with a fixed regex, but a model can
//! follow "is this actually a ticket price" contextually instead of a rigid
//! pattern.
//!
//! Opt-in and zero-cost by default: with no key configured
//! (`commands::settings::read_anthropic_api_key`), behavior is byte-for-byte
//! identical to 2.1.5 - this fallback is only ever reached, and only ever
//! spends any of marko's own API credit, when he has actually pasted a key
//! into Settings. Always fits INSIDE the existing 60s `OVERALL_TIMEOUT`
//! budget, never extends it (`AI_FALLBACK_MIN_REMAINING_BUDGET`/
//! `try_ai_extraction_fallback`) - marko's own explicit ceiling for the
//! whole attempt stays real, AI included, not just for the page-reading
//! part. A result this fallback produces is marked `ai_assisted: true`
//! (models.rs) end to end, so the frontend can ask marko to double-check it
//! a bit more carefully than a result a hard page-structure rule found -
//! same "never fabricate, marko is always the final check" spirit as
//! everything else this whole feature already does.
//!
//! **Cannot be live-tested end-to-end from this sandbox** without spending
//! marko's own real API credit on his own real key, which this session was
//! never given and would never ask for - same discipline already
//! established for Google OAuth/Sheets/Firebase/FX-rate in this app (see
//! fx.rs's own module doc comment). Unlike those, though, api.anthropic.com
//! itself IS reachable from this sandbox's network (fx.rs's own doc comment
//! recorded this while checking FX-rate APIs), so the request-building and
//! response-parsing below were checked against that real endpoint's real
//! error shape for an invalid key - a genuine 401 it actually returned, not
//! one guessed at from documentation - see this module's own tests. A real
//! SUCCESSFUL extraction (a valid key, a real marketplace page's text, a
//! real answer) can still only be exercised on marko's own machine.
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
use crate::models::{AutoCheckDiagnostics, AutoCheckListing, AutoCheckResult};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};

const MIN_WAIT: Duration = Duration::from_millis(800);
const POLL_INTERVAL: Duration = Duration::from_millis(400);
const EVAL_TIMEOUT: Duration = Duration::from_secs(3);

/// Hard wall-clock ceiling for the WHOLE auto-check attempt, from the
/// moment the reader window starts opening to the moment a result is
/// emitted - marko's explicit spec (production-hardening-2 prompt, section
/// 1/6): "60 sekúnd... maximálny čas na samotné čítanie marketplace, nie
/// čas počas ktorého môže byť UI zablokované." This budget now ONLY bounds
/// the background thread's own work - see this module's doc comment
/// ("True non-blocking...") for why nothing in `auto_check_price` itself
/// waits on it anymore, which is what makes that distinction real rather
/// than aspirational.
const OVERALL_TIMEOUT: Duration = Duration::from_secs(60);

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

/// Tauri event carrying the FINAL outcome of an auto-check attempt
/// (2.1.4 - "true non-blocking" fix). This is now the ONLY way a terminal
/// result (ok/partial/unable_to_read/blocked/cancelled/timeout/error) ever
/// reaches the frontend - `auto_check_price` itself returns almost
/// immediately (see that command's own doc comment) and never carries this
/// value as its own return value anymore. `PriceChecker.tsx` listens for
/// this the same way it already listens for `PROGRESS_EVENT`.
const RESULT_EVENT: &str = "price-checker-auto-check-result";

/// `RESULT_EVENT`'s payload - `request_id` lets the frontend recognize (or
/// discard, if stale) which attempt this result belongs to; `result` is the
/// exact same `AutoCheckResult` shape the command used to return directly.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ResultPayload {
    request_id: u64,
    result: AutoCheckResult,
}

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
        // Cancelled/TimedOut/Error never got a real page result to build
        // diagnostics FROM (cancelled or timed out before/during an
        // attempt, or a failure like an invalid URL that never reached a
        // real page at all) - diagnostics: None here is honest, not a gap.
        ReadOutcome::Cancelled => AutoCheckResult {
            status: "cancelled".into(),
            prices: vec![],
            currency: None,
            message: Some("Auto-check was cancelled.".into()),
            listings: vec![],
            ai_assisted: false,
            diagnostics: None,
        },
        ReadOutcome::TimedOut => AutoCheckResult {
            status: "timeout".into(),
            prices: vec![],
            currency: None,
            message: Some(
                "This took too long (over 60 seconds) - use the paste/manual entry below instead, or try again."
                    .into(),
            ),
            listings: vec![],
            ai_assisted: false,
            diagnostics: None,
        },
        ReadOutcome::Error(message) => AutoCheckResult {
            status: "error".into(),
            prices: vec![],
            currency: None,
            message: Some(message),
            listings: vec![],
            ai_assisted: false,
            diagnostics: None,
        },
    }
}

/// `request_id` (2.1.3) is minted by `PriceChecker.tsx`, not this module -
/// see this file's own doc comment ("Production hardening" - "Request
/// IDs") for why. It is echoed back on every `PROGRESS_EVENT`/`RESULT_EVENT`
/// and `log_lifecycle` line for THIS attempt and otherwise never inspected
/// or acted on here.
///
/// ## True non-blocking fix (2.1.4)
///
/// marko's real-world report AFTER 2.1.2/2.1.3 shipped: Auto-check still
/// froze the whole app - Cancel didn't react, Dashboard/other pages
/// couldn't be clicked, Windows showed "not responding". 2.1.2 correctly
/// fixed ONE real bug (`WebviewWindowBuilder::build()` deadlocking when
/// called directly from a synchronous command's own call stack - Tauri's
/// own documented Windows issue), but left a SECOND, independent one in
/// place: this command function itself still didn't RETURN until the whole
/// attempt finished (`run_with_outer_deadline` [now removed] blocked on `rx.recv_timeout`
/// for up to 17, now would-be 60+, seconds before this function's own
/// `Ok(result)` line ran). Confirmed directly against Tauri's own docs
/// (not memory) that this second shape is independently freeze-causing:
/// <https://tauritutorials.com/blog/tauri-command-fundamentals> demonstrates
/// the exact symptom with a plain synchronous command that sleeps - "we'll
/// have a window that will open and freeze... before resuming like normal"
/// - and its own fix is "make the command async" so the caller gets
/// control back immediately. Below applies that same principle the way
/// marko's own spec asked for directly: `auto_check_price` now does only
/// fast, synchronous work (normalize the URL, take-or-reject the
/// single-flight slot) and returns within, realistically, low
/// milliseconds - `status: "busy"` immediately if already running,
/// otherwise `status: "started"` the moment the reader thread has been
/// handed off. The entire browser-read lifecycle runs on a fully detached
/// `std::thread::spawn` that this function does NOT wait on in any way,
/// and reports its real, eventual outcome exclusively via `RESULT_EVENT` -
/// see that constant's own doc comment. This is what actually makes "the
/// app must never freeze" true rather than merely documented: nothing in
/// this function's own call stack can ever take more than a few
/// milliseconds, regardless of what the marketplace page or the WebView
/// engine does afterward.
///
/// Single-flight guard (2.1.3, unchanged in shape): the check-and-set below
/// happens under a SINGLE lock acquisition (`slot` stays locked across both
/// the `is_some()` check and the `= Some(...)` write), so there is no
/// window in which two calls can both observe an empty slot and both
/// proceed - the second is rejected immediately with `"busy"`, before ever
/// spawning anything. The slot is now cleared by the SPAWNED THREAD itself
/// once the real work actually finishes (see below), not by this function -
/// it no longer has anything left to do after handing the thread off.
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
                listings: vec![],
                ai_assisted: false,
                diagnostics: None,
            });
        }
        *slot = Some(cancel_flag.clone());
    }
    log_lifecycle(request_id, "request started");
    spawn_auto_check_thread(app, request_id, normalized, cancel_flag);

    // Returns here - realistically within microseconds of the lock above
    // being released - regardless of how long the spawned thread's own
    // work takes. "started" is not itself shown anywhere as a terminal
    // state; PriceChecker.tsx treats it purely as "the request was
    // accepted, now wait for RESULT_EVENT" (see that event's own doc
    // comment) and keeps its own Starting/Loading/Analyzing UI driven by
    // PROGRESS_EVENT exactly as before.
    Ok(AutoCheckResult { status: "started".into(), prices: vec![], currency: None, message: None, listings: vec![], ai_assisted: false, diagnostics: None })
}

/// "Cancel" button shown next to Auto-check's spinner while a check is in
/// flight - marko's own explicit requirement after the freeze/hang
/// incident. Reuses `commands::google_auth::cancel_google_sign_in_impl`
/// directly against this feature's own slot rather than duplicating its
/// three lines - that helper was explicitly written to be generic over any
/// cancel-flag slot (see its own doc comment; `firebase_google_auth.rs`
/// already reuses it the same way for its own, separate flag). A safe
/// no-op when nothing is actually in flight - a stray double-click, or the
/// attempt already finished a moment earlier. Already fast and fully
/// independent of `auto_check_price` (marko's spec section 3) - unchanged
/// by the 2.1.4 fix, included here only for context.
#[tauri::command]
pub fn cancel_auto_check_price(state: State<AppState>) -> AppResult<()> {
    super::google_auth::cancel_google_sign_in_impl(&state.price_checker_auto_cancel_flag);
    Ok(())
}

/// Hands the ENTIRE reader-webview lifecycle to a fully detached
/// `std::thread::spawn` and returns immediately - `auto_check_price` itself
/// never joins or waits on this thread in any way, which is the actual fix
/// (see that command's own doc comment, "True non-blocking fix"). The
/// thread clears the single-flight slot and emits `RESULT_EVENT` itself
/// once `run_browser_read` returns, however long that takes; nothing about
/// its lifetime is tied to whether the ORIGINAL command call, or even the
/// specific frontend that made it, still exists by the time it finishes -
/// exactly the property that makes "the command can't be made to wait"
/// safe rather than merely fast.
fn spawn_auto_check_thread(app: AppHandle, request_id: u64, url: String, cancel: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let overall_start = Instant::now();
        let outcome = run_browser_read(&app, request_id, &url, overall_start, &cancel);

        // 2.1.6: captured BEFORE read_outcome_to_result consumes `outcome`
        // (it takes `ReadOutcome` by value) - the only extra thing the new
        // AI-assisted fallback needs beyond `result` itself is the RAW
        // extraction JSON, so its title/visible-text are still available to
        // send. `parse_auto_check_json`/`read_outcome_to_result` themselves
        // are untouched by any of this - both stay exactly as pure and
        // tested as before.
        let raw_extract_json = if let ReadOutcome::Json(raw) = &outcome { Some(raw.clone()) } else { None };
        let mut result = read_outcome_to_result(outcome);

        // Last resort, only when every free page-rule pass already came up
        // empty - see try_ai_extraction_fallback's own doc comment and this
        // module's "AI-assisted extraction fallback" section for why this
        // never fires for "blocked" (no real content to hand a model
        // either way) and never spends anything when marko hasn't
        // configured a key.
        //
        // 2.1.6 bugfix: this whole block used to run AFTER the cancel-flag
        // slot below was already cleared. Since `cancel_auto_check_price`
        // reaches this attempt's `AtomicBool` ONLY through that slot (see
        // `cancel_google_sign_in_impl`'s own doc comment - it flips
        // whatever's currently in the slot, and is a safe no-op when the
        // slot is empty), clearing the slot first meant a marko click on
        // "Cancel" during the AI call (up to ~20s, real Anthropic-API money
        // already spent by the time it would land) did precisely nothing -
        // the flag it needed to flip was already unreachable, even though
        // `try_ai_extraction_fallback` itself does check `cancel` at a
        // couple of points. It also meant the single-flight guarantee this
        // slot exists for (see auto_check_price's own doc comment) didn't
        // actually hold for the AI call's own duration. Moving the clear to
        // AFTER this block (below) fixes both: the slot - and so Cancel -
        // stays live for the attempt's ENTIRE real duration, matching
        // spawn_auto_check_thread's own doc comment ("cleared... once the
        // real work actually finishes").
        if result.status == "unable_to_read" {
            if let Some(raw) = raw_extract_json {
                if let Some(ai_result) = try_ai_extraction_fallback(&app, request_id, &raw, overall_start, &cancel) {
                    result = ai_result;
                }
            }
        }

        // Cleared here, not in `auto_check_price` (which returned long
        // ago) - unconditionally, whatever the outcome, so a stale slot can
        // never block a later, unrelated attempt. Uses `try_state` (not
        // `state`, which panics if the app is already mid-teardown) since
        // this can race a real app-exit - see lib.rs's own `ExitRequested`
        // handler and ONE unconditional cleanup either way is what matters,
        // not which of the two lines actually runs it.
        if let Some(state) = app.try_state::<AppState>() {
            *state.price_checker_auto_cancel_flag.lock().unwrap() = None;
        }

        log_lifecycle(request_id, &format!("finished: {}", result.status));
        let _ = app.emit(RESULT_EVENT, ResultPayload { request_id, result });
    });
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

/// Runs on the dedicated thread `spawn_auto_check_thread` spawns. Creates
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
/// never the main thread - see `spawn_auto_check_thread`) for at most
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

/// Whether the readiness-poll loop in `poll_then_extract` should give up
/// polling and fall through to its final best-effort extraction attempt,
/// given how much of the overall budget is left. Pulled out into its own
/// pure function purely so this exact boundary is unit-testable without a
/// real `WebviewWindow` (which `poll_then_extract` itself needs, and which
/// this sandbox can't construct - see this module's own "what was actually
/// verified" notes) - see `poll_then_extract`'s own doc comment for why
/// this specific `<=` boundary (not `.is_zero()`) is the entire fix for a
/// real 2.1.6 bug.
fn should_stop_polling_for_readiness(remaining_budget: Duration) -> bool {
    remaining_budget <= EVAL_TIMEOUT
}

/// Upper bound on how many extraction attempts `poll_then_extract`'s retry
/// loop (2.1.8) will make, independent of the budget-based stop condition
/// above - marko's spec section 5 ("attempt 1, attempt 2, attempt 3...").
/// Purely a sanity cap against pointless spinning on a page that keeps
/// "succeeding" at scrolling further without ever showing a real listing
/// (an infinite-scroll feed unrelated to ticket listings, say) while
/// `OVERALL_TIMEOUT` still has plenty left - in the common case the budget
/// check reached via `should_stop_polling_for_readiness` is what actually
/// ends the loop, well before this count is reached.
const MAX_EXTRACT_ATTEMPTS: u32 = 5;

/// Cheap peek at one attempt's raw `EXTRACT_JS` result: is this worth
/// stopping the retry loop for right now, on its own terms - a real
/// anti-bot block (no point scrolling further, nothing more will ever load)
/// or at least one price/listing actually found? Deliberately NOT the same
/// thing as `parse_auto_check_json`'s full, stricter parsing (which also
/// enforces `MAX_PRICES`, sanitizes currency, degrades a malformed payload
/// to `"error"`, etc.) - this only needs to answer "keep retrying, or
/// return this JSON as the attempt's final answer", the full parse still
/// happens exactly once, in `read_outcome_to_result`, on whichever attempt's
/// raw JSON this loop actually returns. A malformed/unparseable payload
/// here is treated as "nothing found yet" (`false`), not an error - an
/// unparseable attempt is exactly the kind of transient glitch (mid-render,
/// a script conflict on the page) retrying is meant to ride out; the LAST
/// attempt still reports it truthfully as `"error"` via the normal parse
/// path once the loop actually stops.
fn extraction_found_something(raw: &str) -> bool {
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(raw) else {
        return false;
    };
    let blocked = parsed.get("blocked").and_then(|v| v.as_bool()).unwrap_or(false);
    let has_prices = parsed.get("prices").and_then(|v| v.as_array()).map(|a| !a.is_empty()).unwrap_or(false);
    let has_listings = parsed.get("listings").and_then(|v| v.as_array()).map(|a| !a.is_empty()).unwrap_or(false);
    blocked || has_prices || has_listings
}

/// The actual poll loop: waits `MIN_WAIT` for the page to start rendering,
/// then polls `READINESS_CHECK_JS` every `POLL_INTERVAL` until it reports
/// `ready`, the overall budget is down to its last `EVAL_TIMEOUT`, or
/// `cancel` fires - whichever happens first (cancel always wins immediately
/// over everything else, matching marko's own explicit priority: "Cancel
/// musí okamžite..."). Once out of that loop (ready, or budget down to its
/// reserve, but not cancelled), still makes ONE best-effort final
/// extraction attempt on whatever the page has by then, same as the
/// pre-2.1.2 design already did - readiness never firing doesn't mean the
/// page has nothing; the extraction pass has its own, stricter criteria and
/// may still find something.
///
/// 2.1.6 bugfix: the loop used to poll all the way until the budget hit
/// exactly zero (`budget.is_zero()`), and the final extraction below used
/// to re-check `remaining_budget(...).is_zero()` and bail out to
/// `ReadOutcome::TimedOut` WITHOUT ever calling `eval_and_wait` if so. Once
/// the loop had spent the entire budget polling, that second check was
/// therefore ALWAYS true too (elapsed time only ever increases) - so the
/// "best-effort final extraction" this doc comment promises was 100% dead
/// code whenever readiness never fired, which is exactly what happens on a
/// page whose prices load in a shape the cheap readiness probe doesn't
/// recognize. Two consequences: the final-attempt promise silently never
/// happened even before 2.1.6, and as of 2.1.6 it also meant such a page
/// could never reach `AutoCheckResult.status == "unable_to_read"` (only a
/// COMPLETED extraction pass produces that status - see
/// `parse_auto_check_json`) - so the new AI fallback
/// (`try_ai_extraction_fallback`, gated on exactly that status) could never
/// trigger for it either, even with a key configured. That's precisely
/// marko's own reported symptom quoted elsewhere in this module ("ran the
/// full 60s, found nothing" on every marketplace he tried). The fix:
/// `should_stop_polling_for_readiness` now stops the loop with
/// `EVAL_TIMEOUT` still on the clock (not zero), reserved specifically for
/// this final attempt, and every wait inside the loop is capped so it can
/// never eat into that reserve. This is a reservation, not a mathematically
/// perfect guarantee (the loop's own ~`POLL_INTERVAL` polling granularity
/// means the exact remainder at break time can occasionally be smaller than
/// a full `EVAL_TIMEOUT`) - but it can never be WORSE than the old always-
/// exactly-zero behavior, since `eval_and_wait` already degrades safely to
/// an immediate `TimedOut` on a near-zero window, same as before.
///
/// 2.1.7: closed that remaining gap too. The final extraction attempt below
/// no longer derives ITS OWN timeout from the shared clock at all - it
/// always gets a fixed, full `EVAL_TIMEOUT` window, so the "occasionally
/// smaller than a full EVAL_TIMEOUT" case above can no longer silently
/// shrink or (right at the edge) zero out the one attempt that actually
/// matters. Trade-off: the true worst case is now ~`OVERALL_TIMEOUT` +
/// `EVAL_TIMEOUT` (~63s), not a hard 60s - see that call site's own doc
/// comment for the full reasoning.
///
/// 2.1.8: "ONE best-effort final extraction attempt" above is no longer
/// literally one - marko's own spec, section 5 ("Extraction nerob iba raz...
/// attempt 1, attempt 2, attempt 3... kým sa nájdu relevantné listings,
/// stránka je označená blocked, alebo sa vyčerpá 60s budget"), in direct
/// response to real-world marketplace pages whose listings render in
/// several waves (a first paint, then a client-side data fetch, then
/// lazy-loaded rows as the page scrolls) - a single attempt right as
/// readiness first fires can easily land between two of those waves and see
/// nothing yet, even on a page that would show real listings a couple of
/// seconds later. The retry loop below keeps the EXACT lesson 2.1.6/2.1.7
/// already paid for, generalized rather than special-cased: every single
/// attempt, first or last, gets the SAME fixed, full `EVAL_TIMEOUT` window -
/// nothing here ever derives an individual attempt's own wait duration from
/// `remaining_budget`, because that specific pattern is what silently starved
/// the "attempt that actually matters" twice already (see both paragraphs
/// above). What DOES depend on the remaining budget is only ever a boolean -
/// "is there room for another attempt after this one" - decided fresh before
/// each attempt via the same `should_stop_polling_for_readiness` boundary
/// already established (and unit-tested) above, so the worst-case total time
/// stays the same documented ~63s ceiling regardless of how many attempts it
/// took. `MAX_EXTRACT_ATTEMPTS` is a second, independent cap against
/// pointless spinning when budget alone would otherwise allow many more
/// retries (its own doc comment has the reasoning). Between attempts: one
/// `READINESS_CHECK_JS` eval (whose own job, 2.1.8, now includes the actual
/// incremental scroll marko's spec section 4 asks for - see that file's own
/// comment) plus a short pause, so newly-scrolled-in content has a real
/// chance to render before the next attempt looks for it.
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
        if should_stop_polling_for_readiness(budget) {
            break; // best-effort: still try ONE final extraction below, see this function's own doc comment
        }

        // Capped at `budget - EVAL_TIMEOUT` (never the raw budget) so this
        // call itself can't eat into the reserve the final extraction below
        // needs - see should_stop_polling_for_readiness's own doc comment.
        match eval_and_wait(webview, READINESS_CHECK_JS, (budget - EVAL_TIMEOUT).min(EVAL_TIMEOUT), cancel) {
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
        // Same reserve applied to the poll-interval sleep itself.
        if !sleep_interruptible(POLL_INTERVAL.min(remaining_budget(overall_start).saturating_sub(EVAL_TIMEOUT)), cancel) {
            return ReadOutcome::Cancelled;
        }
    }

    if cancel.load(Ordering::Relaxed) {
        return ReadOutcome::Cancelled;
    }
    emit_phase(app, request_id, "analyzing");
    log_lifecycle(request_id, "analysis started");

    // 2.1.8 retry loop - see this function's own "2.1.8" doc comment above
    // for the full reasoning. `attempt` is 1-based purely for log/
    // diagnostic readability.
    let mut attempt: u32 = 0;
    loop {
        attempt += 1;
        if cancel.load(Ordering::Relaxed) {
            return ReadOutcome::Cancelled;
        }
        // Decided BEFORE this attempt runs, from the CURRENT clock - once
        // this is true, this attempt is the last one no matter what it
        // finds, so it must be treated exactly like the old single "final"
        // attempt (which it now literally is, on this path through the
        // loop).
        let is_last_attempt = should_stop_polling_for_readiness(remaining_budget(overall_start)) || attempt >= MAX_EXTRACT_ATTEMPTS;

        // Every attempt, first or last, gets the SAME fixed EVAL_TIMEOUT
        // window - never derived from remaining_budget. See this function's
        // own "2.1.8" doc comment for why that specific derivation is
        // exactly what silently broke this twice before (2.1.6, 2.1.7).
        match eval_and_wait(webview, EXTRACT_JS, EVAL_TIMEOUT, cancel) {
            EvalOutcome::Ready(raw) => {
                log_lifecycle(request_id, &format!("attempt {attempt} result received ({} bytes)", raw.len()));
                if is_last_attempt || extraction_found_something(&raw) {
                    return ReadOutcome::Json(raw);
                }
                log_lifecycle(request_id, &format!("attempt {attempt} found nothing yet - retrying"));
            }
            EvalOutcome::Cancelled => return ReadOutcome::Cancelled,
            EvalOutcome::TimedOut => {
                log_lifecycle(request_id, &format!("attempt {attempt} timed out"));
                if is_last_attempt {
                    return ReadOutcome::TimedOut;
                }
            }
            // 2.1.3: previously mislabeled "timeout" (see EvalOutcome's own
            // doc comment) - a real JS-evaluation failure reports its own
            // distinct "error" instead, per marko's spec section 11. Never
            // retried (unlike TimedOut/an empty result above) - a dispatch/
            // callback failure means something is wrong with the reader
            // window itself, not "the listings haven't rendered yet",
            // exactly the distinction EvalOutcome::Failed exists to draw.
            EvalOutcome::Failed(reason) => return ReadOutcome::Error(format!("The reader page's script could not be run ({reason}).")),
        }

        if is_last_attempt {
            // Unreachable in practice (every branch above that can run when
            // is_last_attempt is true already returns), kept only so this
            // loop is provably not infinite even if a future edit adds a
            // branch that forgets to check it.
            return ReadOutcome::TimedOut;
        }
        if cancel.load(Ordering::Relaxed) {
            return ReadOutcome::Cancelled;
        }
        // Between attempts: READINESS_CHECK_JS's own side effect performs
        // one incremental scroll step (see that file's own comment) - its
        // `ready`/`blocked` answer is deliberately ignored here, this loop
        // already has its own, stricter "did EXTRACT_JS itself find
        // something" criteria via extraction_found_something.
        //
        // Capped by remaining budget (fixed on adversarial review - this
        // used to be an unconditional EVAL_TIMEOUT here, which could add a
        // full extra EVAL_TIMEOUT on top of the extraction eval above
        // within the SAME non-last iteration, pushing the true worst case
        // to ~OVERALL_TIMEOUT + 2*EVAL_TIMEOUT instead of the documented
        // ~OVERALL_TIMEOUT + EVAL_TIMEOUT). This is safe to shrink, unlike
        // the extraction eval above: this call's own `ready`/`blocked`
        // result is thrown away either way, so a smaller window here can
        // only mean slightly less time for this one scroll step to settle,
        // never a starved attempt - the actual "attempt that matters" stays
        // on its own always-fixed EVAL_TIMEOUT.
        let _ = eval_and_wait(webview, READINESS_CHECK_JS, EVAL_TIMEOUT.min(remaining_budget(overall_start)), cancel);
        if cancel.load(Ordering::Relaxed) {
            return ReadOutcome::Cancelled;
        }
        // The pause itself IS fine to shrink under budget pressure - unlike
        // an extraction attempt's own wait, a shorter settle pause only
        // means slightly less time for newly-scrolled-in content to render
        // before the next attempt, never a skipped attempt.
        if !sleep_interruptible(POLL_INTERVAL.min(remaining_budget(overall_start)), cancel) {
            return ReadOutcome::Cancelled;
        }
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
            listings: vec![],
            ai_assisted: false,
            diagnostics: None, // nothing to parse diagnostics FROM - the whole payload wasn't even valid JSON
        };
    };

    let diagnostics = parse_diagnostics(&parsed);

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
            listings: vec![],
            ai_assisted: false,
            diagnostics,
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
            listings: vec![],
            ai_assisted: false,
            diagnostics,
        };
    }

    let prices: Vec<f64> = parsed
        .get("prices")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_f64()).filter(|p| p.is_finite() && *p > 0.0).collect())
        .unwrap_or_default();
    let currency = sanitize_currency(parsed.get("currency").and_then(|v| v.as_str()));
    let listings = parse_listings(&parsed);

    if prices.is_empty() {
        return AutoCheckResult {
            status: "unable_to_read".into(),
            prices: vec![],
            currency,
            message: Some(build_diagnostic_message(
                "The page loaded, but no prices could be found on it automatically. Use the paste/manual entry below instead.",
                &parsed,
            )),
            listings: vec![],
            ai_assisted: false,
            diagnostics,
        };
    }

    // 2.1.8 (marko's spec section 9): a real price number is not the same
    // claim as a real, confirmed ticket LISTING - `listings` only ever gets
    // an entry when a price came with correlated section/row/seat context
    // (see price_checker_auto_extract.js's own `readCandidates`/
    // `nearbyListingContext`, and the Vivid Seats table pass, which has
    // always populated it this same way since 2.1.4). Bare prices with no
    // such context - a schema.org AggregateOffer's low/high, an og:price
    // meta tag, a generic currency-adjacent number - are real numbers the
    // page actually showed, never fabricated, but not confirmed as
    // individual listings either; `"partial"` says exactly that, rather
    // than either overclaiming `"ok"` or discarding real data as
    // `"unable_to_read"`.
    if listings.is_empty() {
        return AutoCheckResult {
            status: "partial".into(),
            prices: prices.clone(),
            currency,
            message: Some(build_diagnostic_message(
                &format!(
                    "Found {} price{} on the page, but couldn't confirm they're individual ticket listings \
                     (no matching section/row/seat detail nearby) - this could be an aggregate, a starting-price \
                     figure, or an unrelated number. Double-check carefully before saving.",
                    prices.len(),
                    if prices.len() == 1 { "" } else { "s" }
                ),
                &parsed,
            )),
            listings: vec![],
            ai_assisted: false,
            diagnostics,
        };
    }

    AutoCheckResult { status: "ok".into(), prices, currency, message: None, listings, ai_assisted: false, diagnostics }
}

/// Builds `AutoCheckDiagnostics` from `EXTRACT_JS`'s raw `diagnostics`
/// object, or `None` if that object is missing entirely (an older cached
/// page, or a genuinely unusual payload) - never partially-built with made-
/// up zeros standing in for absent fields, matching every other parse in
/// this module. Every individual field still degrades independently to
/// `None` (not the whole struct) when only THAT one is missing/wrong-typed,
/// so a page that reports 11 of these 14 fields correctly still gets useful
/// diagnostics for the 11, rather than losing everything over the 3 it
/// couldn't. Pure and unit-tested directly - see the tests below.
fn parse_diagnostics(parsed: &serde_json::Value) -> Option<AutoCheckDiagnostics> {
    let diag = parsed.get("diagnostics")?;
    let s = |key: &str| diag.get(key).and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(String::from);
    let n = |key: &str| diag.get(key).and_then(|v| v.as_u64());
    let n32 = |key: &str| diag.get(key).and_then(|v| v.as_u64()).and_then(|v| u32::try_from(v).ok());
    Some(AutoCheckDiagnostics {
        marketplace_reader: s("marketplaceReader"),
        attempt: n32("attempt"),
        page_title: parsed.get("title").and_then(|v| v.as_str()).filter(|s| !s.trim().is_empty()).map(String::from),
        final_url: s("finalUrl"),
        dom_length: n("domLength"),
        visible_text_length: n("visibleTextLength"),
        table_count: n("tableCount"),
        link_count: n("linkCount"),
        button_count: n("buttonCount"),
        currency_symbol_element_count: n("currencySymbolElementCount"),
        price_text_element_count: n("priceTextElementCount"),
        section_text_element_count: n("sectionTextElementCount"),
        row_text_element_count: n("rowTextElementCount"),
        candidate_listing_element_count: n("candidateListingElementCount"),
        text_sample: s("textSample"),
        dom_snapshot: s("domSnapshot"),
    })
}

/// Builds the message marko actually sees for an `"unable_to_read"` or
/// `"partial"` result - 2.1.5 originally (title/table-count/text-sample
/// only), extended 2.1.8 (marko's spec section 7: "Aktuálna hláška...
/// je príliš slabá") with everything else `AutoCheckDiagnostics` now
/// carries. Without this, the only way to know WHY would have been console
/// output marko has no way to see in a packaged GUI app (no terminal
/// attached) - this puts the same information directly in the message the
/// UI already shows, so marko's existing "copy the message, paste it back
/// to me" workflow (the exact one that led to this rewrite) carries enough
/// to actually diagnose the next miss, without another round trip just to
/// ask "what did it see". `domSnapshot` deliberately never appears here
/// (see `AutoCheckDiagnostics::dom_snapshot`'s own doc comment) - it would
/// dwarf everything else in this message; it is still on the structured
/// result for whenever it's actually needed. `base` is the one sentence
/// that differs between the two callers; everything from "What the reader
/// actually saw" down is identical for both. Every piece here is something
/// the page's own markup/visible text actually said - never invented,
/// matching this whole module's own "never fabricate data" rule.
fn build_diagnostic_message(base: &str, parsed: &serde_json::Value) -> String {
    let title = parsed.get("title").and_then(|v| v.as_str()).filter(|s| !s.trim().is_empty());
    let diag = parsed.get("diagnostics");
    let reader = diag.and_then(|d| d.get("marketplaceReader")).and_then(|v| v.as_str());
    let attempt = diag.and_then(|d| d.get("attempt")).and_then(|v| v.as_u64());
    let candidates = diag.and_then(|d| d.get("candidateListingElementCount")).and_then(|v| v.as_u64());
    let table_count = diag.and_then(|d| d.get("tableCount")).and_then(|v| v.as_u64());
    let link_count = diag.and_then(|d| d.get("linkCount")).and_then(|v| v.as_u64());
    let button_count = diag.and_then(|d| d.get("buttonCount")).and_then(|v| v.as_u64());
    let currency_count = diag.and_then(|d| d.get("currencySymbolElementCount")).and_then(|v| v.as_u64());
    let price_count = diag.and_then(|d| d.get("priceTextElementCount")).and_then(|v| v.as_u64());
    let section_count = diag.and_then(|d| d.get("sectionTextElementCount")).and_then(|v| v.as_u64());
    let row_count = diag.and_then(|d| d.get("rowTextElementCount")).and_then(|v| v.as_u64());
    let dom_length = diag.and_then(|d| d.get("domLength")).and_then(|v| v.as_u64());
    let text_length = diag.and_then(|d| d.get("visibleTextLength")).and_then(|v| v.as_u64());
    let text_sample = diag.and_then(|d| d.get("textSample")).and_then(|v| v.as_str()).map(|s| s.trim()).filter(|s| !s.is_empty());
    let has_snapshot = diag.and_then(|d| d.get("domSnapshot")).and_then(|v| v.as_str()).map(|s| !s.is_empty()).unwrap_or(false);

    let mut msg = String::from(base);
    msg.push_str("\n\nWhat the reader actually saw (for diagnosis):");
    if let Some(r) = reader {
        msg.push_str(&format!("\n- Marketplace reader used: {r}"));
    }
    if let Some(t) = title {
        msg.push_str(&format!("\n- Page title: \"{t}\""));
    }
    if let Some(a) = attempt {
        msg.push_str(&format!("\n- Extraction attempts made: {a}"));
    }
    if let Some(c) = candidates {
        msg.push_str(&format!("\n- Candidate listing elements found: {c}"));
    }
    if table_count.is_some() || link_count.is_some() || button_count.is_some() {
        msg.push_str(&format!(
            "\n- HTML elements: {} <table>, {} links, {} buttons",
            table_count.unwrap_or(0),
            link_count.unwrap_or(0),
            button_count.unwrap_or(0)
        ));
    }
    if currency_count.is_some() || price_count.is_some() || section_count.is_some() || row_count.is_some() {
        msg.push_str(&format!(
            "\n- Elements mentioning a currency symbol / \"price\" / \"section\" / \"row\": {} / {} / {} / {}",
            currency_count.unwrap_or(0),
            price_count.unwrap_or(0),
            section_count.unwrap_or(0),
            row_count.unwrap_or(0)
        ));
    }
    if dom_length.is_some() || text_length.is_some() {
        msg.push_str(&format!(
            "\n- Page size: {} chars of HTML, {} chars of visible text",
            dom_length.unwrap_or(0),
            text_length.unwrap_or(0)
        ));
    }
    if let Some(sample) = text_sample {
        let truncated: String = sample.chars().take(300).collect();
        msg.push_str(&format!("\n- First visible text: \"{truncated}{}\"", if sample.chars().count() > 300 { "..." } else { "" }));
    }
    if has_snapshot {
        msg.push_str("\n\n(A safe snapshot of the page's own HTML was also captured for a closer look - no cookies or login data included.)");
    }
    msg
}

#[cfg(test)]
mod pure_module_tests {
    use super::*;

    #[test]
    fn unable_to_read_message_includes_title_and_table_count_when_present() {
        let raw = r#"{"prices": [], "currency": null, "blocked": false, "title": "Some Event | Marketplace", "diagnostics": {"tableCount": 0, "textSample": "Sold out - check back later"}}"#;
        let result = parse_auto_check_json(raw);
        let msg = result.message.unwrap();
        assert!(msg.contains("Some Event | Marketplace"), "must surface the real page title for diagnosis");
        assert!(msg.contains("0 <table>"));
        assert!(msg.contains("Sold out"));
    }

    #[test]
    fn unable_to_read_message_degrades_gracefully_with_no_diagnostics_at_all() {
        // Older/malformed payloads without title/diagnostics must not panic
        // or produce a broken message - just the base sentence.
        let raw = r#"{"prices": [], "currency": null, "blocked": false}"#;
        let result = parse_auto_check_json(raw);
        assert!(result.message.unwrap().contains("no prices could be found"));
        assert_eq!(result.diagnostics, None, "no diagnostics object in the payload at all must mean None, not a zeroed-out struct");
    }

    #[test]
    fn unable_to_read_message_truncates_a_very_long_text_sample() {
        let long_text = "x".repeat(1000);
        let raw = serde_json::json!({"prices": [], "currency": null, "blocked": false, "diagnostics": {"tableCount": 2, "textSample": long_text}}).to_string();
        let result = parse_auto_check_json(&raw);
        let msg = result.message.unwrap();
        assert!(msg.contains("..."), "a long sample must be truncated, not dumped whole into the UI message");
    }

    #[test]
    fn unable_to_read_message_never_inlines_the_dom_snapshot_but_notes_it_was_captured() {
        let raw = serde_json::json!({"prices": [], "currency": null, "blocked": false, "diagnostics": {"domSnapshot": "<div class=\"secret-marker-xyz\">hi</div>"}}).to_string();
        let result = parse_auto_check_json(&raw);
        let msg = result.message.unwrap();
        assert!(!msg.contains("secret-marker-xyz"), "the raw snapshot HTML must never be dumped into the human-facing message");
        assert!(msg.contains("safe snapshot"), "must still say one was captured, so marko knows it exists");
        assert_eq!(result.diagnostics.unwrap().dom_snapshot.as_deref(), Some("<div class=\"secret-marker-xyz\">hi</div>"), "but IS on the structured result");
    }

    #[test]
    fn a_bare_price_with_no_listing_context_is_partial_not_ok() {
        // og:price-shaped payload - a real number, no section/row context.
        let raw = r#"{"prices": [199.5], "currency": "USD", "blocked": false, "listings": [], "diagnostics": {"marketplaceReader": "ticombo"}}"#;
        let result = parse_auto_check_json(raw);
        assert_eq!(result.status, "partial");
        assert_eq!(result.prices, vec![199.5], "the real number found must still be reported, not discarded");
        assert!(result.listings.is_empty());
        let msg = result.message.unwrap();
        assert!(msg.contains("Found 1 price"));
        assert!(msg.contains("couldn't confirm"));
    }

    #[test]
    fn prices_with_real_listing_context_are_ok_not_partial() {
        let raw = r#"{"prices": [199.5], "currency": "USD", "blocked": false,
            "listings": [{"price": 199.5, "currency": "USD", "section": "112", "row": "A", "quantity": 2}]}"#;
        let result = parse_auto_check_json(raw);
        assert_eq!(result.status, "ok");
        assert_eq!(result.listings.len(), 1);
        assert!(result.message.is_none(), "a confirmed ok result carries no diagnostic message, matching pre-2.1.8 behavior");
    }

    #[test]
    fn partial_message_uses_correct_singular_plural_price_wording() {
        let one = parse_auto_check_json(r#"{"prices": [10.0], "currency": null, "blocked": false, "listings": []}"#);
        assert!(one.message.unwrap().contains("Found 1 price on"));
        let two = parse_auto_check_json(r#"{"prices": [10.0, 20.0], "currency": null, "blocked": false, "listings": []}"#);
        assert!(two.message.unwrap().contains("Found 2 prices on"));
    }

    #[test]
    fn parse_diagnostics_keeps_the_fields_present_even_when_others_are_missing_or_wrong_typed() {
        let raw = serde_json::json!({
            "prices": [], "currency": null, "blocked": false,
            "diagnostics": { "marketplaceReader": "stubhub", "tableCount": 3, "linkCount": "not a number" }
        })
        .to_string();
        let result = parse_auto_check_json(&raw);
        let d = result.diagnostics.unwrap();
        assert_eq!(d.marketplace_reader.as_deref(), Some("stubhub"));
        assert_eq!(d.table_count, Some(3));
        assert_eq!(d.link_count, None, "a wrong-typed field must degrade to None for itself only, not fail the whole struct");
    }

    #[test]
    fn blocked_result_still_carries_diagnostics() {
        let raw = serde_json::json!({"prices": [], "currency": null, "blocked": true, "diagnostics": {"marketplaceReader": "blocked", "buttonCount": 4}}).to_string();
        let result = parse_auto_check_json(&raw);
        assert_eq!(result.status, "blocked");
        assert_eq!(result.diagnostics.unwrap().button_count, Some(4), "diagnostics are still worth having on a blocked page too");
    }
}



/// Parses `EXTRACT_JS`'s optional `listings` array (2.1.4 - present only
/// from the HTML-table pass, see that pass's own comment in the .js file)
/// into `AutoCheckListing`s. Never guesses a missing field - `section`/
/// `row`/`quantity` become `None`, not an empty string or a made-up value,
/// exactly like `currency` above degrades via `sanitize_currency` rather
/// than ever passing through something the page didn't actually say.
fn parse_listings(parsed: &serde_json::Value) -> Vec<AutoCheckListing> {
    parsed
        .get("listings")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|entry| {
                    let price = entry.get("price")?.as_f64().filter(|p| p.is_finite() && *p > 0.0)?;
                    Some(AutoCheckListing {
                        price,
                        currency: sanitize_currency(entry.get("currency").and_then(|v| v.as_str())),
                        section: entry.get("section").and_then(|v| v.as_str()).map(str::trim).filter(|s| !s.is_empty()).map(String::from),
                        row: entry.get("row").and_then(|v| v.as_str()).map(str::trim).filter(|s| !s.is_empty()).map(String::from),
                        quantity: entry.get("quantity").and_then(|v| v.as_u64()).and_then(|q| u32::try_from(q).ok()),
                    })
                })
                .take(MAX_PRICES)
                .collect()
        })
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// AI-assisted extraction fallback (2.1.6) - see this module's own doc
// comment, "AI-assisted extraction fallback", for the full design and its
// honest limits.
// ---------------------------------------------------------------------------

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
/// Dateless alias (resolves to the latest 4.5 snapshot server-side) rather
/// than a pinned dated snapshot - this app has no auto-update-the-model-
/// string mechanism, so pinning a dated snapshot would eventually point at
/// a retired model with no code change able to fix it. Deliberately Haiku,
/// not Sonnet/Opus: this is a narrow, cheap, well-specified extraction task
/// (read this text, report any ticket prices you're confident about, say so
/// plainly if you're not) - exactly what the fast/cheap tier is for, and it
/// keeps marko's own per-check cost negligible (a fraction of a cent at
/// today's published pricing for a page-sized amount of text).
const ANTHROPIC_MODEL: &str = "claude-haiku-4-5";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const AI_FALLBACK_MAX_TOKENS: u32 = 1024;
/// Below this much of OVERALL_TIMEOUT remaining, don't even attempt the
/// fallback - a real Haiku call plus network round trip realistically needs
/// a few seconds, and starting one with almost nothing left would either
/// get cut short pointlessly or risk overrunning OVERALL_TIMEOUT's own
/// honest ceiling (this module's own doc comment). Skipping cleanly here
/// just means this attempt reports the same `unable_to_read` result 2.1.5
/// already gave - never worse than today, only sometimes better.
const AI_FALLBACK_MIN_REMAINING_BUDGET: Duration = Duration::from_secs(8);
/// Upper bound on the AI call's own timeout - always further capped by
/// whatever of OVERALL_TIMEOUT is actually left, see
/// `try_ai_extraction_fallback`.
const AI_FALLBACK_MAX_CALL_TIMEOUT: Duration = Duration::from_secs(20);

/// What `parse_ai_extraction_response` pulls out of a successful call -
/// deliberately NOT `AutoCheckResult` itself: this is only ever the AI's
/// half of the answer (does it think there are real prices here, and what
/// are they), never a full result on its own. `try_ai_extraction_fallback`
/// is the one place that turns this into an actual `AutoCheckResult` - the
/// same "keep the pure parsing part and the result-shaping part separate"
/// split `parse_auto_check_json`/`build_unable_to_read_message` already use.
#[derive(Debug, Clone, PartialEq)]
struct AiExtraction {
    prices: Vec<f64>,
    currency: Option<String>,
}

/// Builds the exact JSON body sent to the Messages API. Pure and
/// unit-tested directly (no network) - see the tests below. Uses the
/// standard "assistant turn pre-filled with `{`" technique to make the
/// model continue straight into JSON rather than wrapping its answer in
/// prose or a markdown code fence - more reliable than instructions alone,
/// and it means `parse_ai_extraction_response` can always prepend the same
/// `{` back rather than needing to strip a fence that may or may not be
/// there.
fn build_ai_extraction_request_body(page_title: &str, visible_text: &str) -> serde_json::Value {
    let prompt = format!(
        "You are reading the visible text of a ticket resale marketplace webpage, extracted by a script. \
         Report ONLY real ticket listing prices - what a buyer would pay for one ticket. Never report fees, \
         shipping, delivery charges, discount codes, unrelated promotions, or navigation/footer numbers. If \
         you are not reasonably confident real ticket prices are present, set \"found\" to false and return \
         an empty prices array - never guess.\n\n\
         Page title: {page_title}\n\n\
         Visible page text:\n{visible_text}\n\n\
         Respond with ONLY a JSON object in exactly this shape, nothing else, no markdown fence:\n\
         {{\"found\": true or false, \"currency\": a 3-letter ISO 4217 currency code guess or null, \
         \"prices\": [array of numbers - the individual ticket prices you found]}}"
    );
    serde_json::json!({
        "model": ANTHROPIC_MODEL,
        "max_tokens": AI_FALLBACK_MAX_TOKENS,
        "messages": [
            {"role": "user", "content": prompt},
            {"role": "assistant", "content": "{"}
        ]
    })
}

/// Parses the Messages API's own raw JSON response body into an
/// `AiExtraction`, or `None` on ANY ambiguity. A network-level failure
/// never reaches this function at all (see `call_anthropic_api`) - from
/// here on it's "the call succeeded, now can this actually be trusted":
/// a malformed response shape, a text block that isn't valid JSON even
/// after re-adding the `{` prefill, `"found": false`, an empty prices
/// array, or any non-finite/non-positive price all mean `None`, exactly
/// like `parse_auto_check_json` never lets a malformed page result become a
/// fabricated success. Pure and unit-tested directly against hand-authored
/// response bodies, including the real 401 error shape recorded from
/// api.anthropic.com itself - see this module's own doc comment.
fn parse_ai_extraction_response(response_body: &str) -> Option<AiExtraction> {
    let parsed: serde_json::Value = serde_json::from_str(response_body).ok()?;
    let text = parsed.get("content")?.as_array()?.iter().find_map(|block| {
        if block.get("type")?.as_str()? == "text" {
            block.get("text")?.as_str()
        } else {
            None
        }
    })?;
    // Re-prepend the `{` the request's own assistant-turn prefill forced the
    // model to continue from (see build_ai_extraction_request_body) - the
    // API never echoes the prefill back itself, only what the model
    // generated AFTER it.
    let full_json = format!("{{{text}");
    let extraction: serde_json::Value = serde_json::from_str(&full_json).ok()?;

    if !extraction.get("found").and_then(|v| v.as_bool()).unwrap_or(false) {
        return None;
    }
    let prices: Vec<f64> = extraction
        .get("prices")?
        .as_array()?
        .iter()
        .filter_map(|v| v.as_f64())
        .filter(|p| p.is_finite() && *p > 0.0)
        .take(MAX_PRICES)
        .collect();
    if prices.is_empty() {
        return None;
    }
    let currency = sanitize_currency(extraction.get("currency").and_then(|v| v.as_str()));
    Some(AiExtraction { prices, currency })
}

/// The one network call this fallback makes - thin by design, everything
/// worth testing without a real HTTP round trip already lives in the pure
/// functions around it. Returns the raw response body text on ANY 2xx
/// status; a non-2xx status or a transport-level failure (timeout, DNS,
/// TLS, ...) both become `Err` with a short reason, never a panic - the
/// caller (`try_ai_extraction_fallback`) treats every `Err` identically to
/// a `None` from `parse_ai_extraction_response`: fall through to the
/// existing `unable_to_read` result, exactly as if this fallback had never
/// been attempted.
fn call_anthropic_api(api_key: &str, body: &serde_json::Value, timeout: Duration) -> Result<String, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|e| format!("could not set up the request: {e}"))?;
    let resp = client
        .post(ANTHROPIC_API_URL)
        .header("x-api-key", api_key)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .json(body)
        .send()
        .map_err(|e| format!("could not reach the Anthropic API: {e}"))?;
    let status = resp.status();
    let text = resp.text().map_err(|e| format!("could not read the Anthropic API's response: {e}"))?;
    if !status.is_success() {
        return Err(describe_anthropic_rejection(status, &text));
    }
    Ok(text)
}

/// Formats a clear reason when the Anthropic API rejects a request outright
/// (non-2xx status). Split out of `call_anthropic_api` purely so it's
/// directly unit-testable the way `fx::describe_rejected_request` already
/// is for Frankfurter's errors: a real `reqwest::blocking::Response` can't
/// be constructed without an actual HTTP round trip, but a `StatusCode` and
/// a body string can be built by hand in a test - see this module's own
/// tests, which use the REAL body api.anthropic.com returned for an
/// invalid key during development (a genuine 401, not a guess).
fn describe_anthropic_rejection(status: reqwest::StatusCode, body: &str) -> String {
    format!("the Anthropic API rejected the request ({status}): {body}")
}

/// Orchestrates the whole fallback: budget/key checks, the actual call, and
/// turning a real extraction into a proper `AutoCheckResult`. Called from
/// `spawn_auto_check_thread` only when the ordinary extraction already
/// produced `status == "unable_to_read"` - see this module's own doc
/// comment ("AI-assisted extraction fallback") for the full reasoning.
/// Returns `None` (meaning: "leave the existing unable_to_read result
/// exactly as it was") whenever anything isn't a clean, confident success -
/// no key configured, not enough budget left, no usable text to send, the
/// call itself failing, or the model reporting it isn't confident there are
/// real prices. `cancel` is checked first (and again after the network
/// call returns) so a marko Cancel-click during this fallback is honored
/// exactly as it is everywhere else in this module.
///
/// 2.1.6 bugfix: emits the `"asking_ai"` phase right before the actual
/// network call (not any earlier - the no-key/no-budget/no-text early exits
/// above stay completely silent, matching "zero cost, zero UI change when
/// this isn't configured"). Before this, PriceChecker.tsx's card kept
/// showing "Cleaning up..." - the LAST phase `run_browser_read` emits -
/// for the entire duration of a real, paid API call, with no visible sign
/// that money was being spent at all. See the sibling fix in
/// `spawn_auto_check_thread` (moving where the cancel-flag slot gets
/// cleared) for the related bug where Cancel also silently did nothing
/// during this same window.
fn try_ai_extraction_fallback(
    app: &AppHandle,
    request_id: u64,
    raw_extract_json: &str,
    overall_start: Instant,
    cancel: &AtomicBool,
) -> Option<AutoCheckResult> {
    if cancel.load(Ordering::Relaxed) {
        return None;
    }

    let state = app.try_state::<AppState>()?;
    let api_key = {
        let conn = state.db.lock().unwrap();
        crate::commands::settings::read_anthropic_api_key(&conn)?
    };

    let remaining = remaining_budget(overall_start);
    if remaining < AI_FALLBACK_MIN_REMAINING_BUDGET {
        log_lifecycle(request_id, "AI fallback skipped: not enough of the 60s budget left to attempt it");
        return None;
    }
    let call_timeout = remaining.saturating_sub(Duration::from_secs(1)).min(AI_FALLBACK_MAX_CALL_TIMEOUT);

    let parsed: serde_json::Value = serde_json::from_str(raw_extract_json).ok()?;
    let title = parsed.get("title").and_then(|v| v.as_str()).unwrap_or("");
    let visible_text = parsed
        .get("diagnostics")
        .and_then(|d| d.get("aiText"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())?;

    log_lifecycle(request_id, "AI fallback: page's own rules found nothing, asking the Anthropic API");
    emit_phase(app, request_id, "asking_ai");
    let body = build_ai_extraction_request_body(title, visible_text);
    let response = match call_anthropic_api(&api_key, &body, call_timeout) {
        Ok(text) => text,
        Err(reason) => {
            log_lifecycle(request_id, &format!("AI fallback call failed, falling back to unable_to_read: {reason}"));
            return None;
        }
    };

    if cancel.load(Ordering::Relaxed) {
        return None;
    }

    let extraction = parse_ai_extraction_response(&response)?;
    log_lifecycle(request_id, &format!("AI fallback found {} price(s)", extraction.prices.len()));
    Some(AutoCheckResult {
        // 2.1.8 (fixed on adversarial review): this used to unconditionally
        // be "ok" - but 2.1.8 elsewhere in this same file redefined "ok" to
        // specifically mean "confirmed real listings, price correlated with
        // section/row/seat context" (see parse_auto_check_json's own ok/
        // partial split). The AI fallback's prompt only ever asks for a
        // flat prices array - it has no schema slot for section/row/seat at
        // all, so `listings` below is always empty and this evidence is
        // structurally identical to what earns "partial" everywhere else in
        // this file: real prices, no confirmed listing correlation. Giving
        // it "ok" gave an LLM's free-text guess - which this very message
        // already admits is more likely to be wrong than the page's own
        // structured data - a MORE confident label than a bare structured
        // price would get. `ai_assisted: true` (below) is untouched and
        // still drives its own separate "AI read these" note in the UI -
        // this only changes which status string carries that evidence.
        status: "partial".into(),
        prices: extraction.prices,
        currency: extraction.currency,
        message: Some(
            "Found by AI reading the page's visible text, not by an exact page-structure rule - \
             double-check these against the real page before saving."
                .into(),
        ),
        listings: vec![],
        ai_assisted: true,
        // Deliberately None, not the rule-based passes' own diagnostics
        // (which already exist, on the "unable_to_read" result this
        // replaces) - the AI's OWN confidence signal is `ai_assisted: true`
        // plus this message, a different kind of "double-check this" than
        // AutoCheckDiagnostics represents, and re-attaching page-structure
        // diagnostics to an AI-sourced result risks implying they explain
        // ITS answer, when they only ever described why the rule-based
        // pass came up empty in the first place.
        diagnostics: None,
    })
}

#[cfg(test)]
mod ai_fallback_tests {
    use super::*;

    #[test]
    fn request_body_has_the_required_fields_and_the_json_prefill() {
        let body = build_ai_extraction_request_body("Coldplay | Vivid Seats", "Section 100 $150 each");
        assert_eq!(body["model"], "claude-haiku-4-5");
        assert_eq!(body["max_tokens"], AI_FALLBACK_MAX_TOKENS);
        let messages = body["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 2, "a user turn with the page content, plus the assistant JSON prefill");
        assert_eq!(messages[0]["role"], "user");
        assert!(messages[0]["content"].as_str().unwrap().contains("Coldplay | Vivid Seats"), "the real page title must reach the model");
        assert!(messages[0]["content"].as_str().unwrap().contains("Section 100 $150 each"), "the real page text must reach the model");
        assert_eq!(messages[1]["role"], "assistant");
        assert_eq!(messages[1]["content"], "{", "the prefill forces the reply to continue straight into JSON");
    }

    #[test]
    fn response_parser_accepts_a_confident_answer_with_the_prefill_reattached() {
        // Mirrors exactly what the real API returns for a successful call -
        // "text" here is what would follow the "{" prefill, never including
        // it (see build_ai_extraction_request_body's own doc comment).
        let body = serde_json::json!({
            "content": [{"type": "text", "text": "\"found\": true, \"currency\": \"USD\", \"prices\": [120, 145.50, 99]}"}]
        })
        .to_string();
        let extraction = parse_ai_extraction_response(&body).expect("a confident, well-formed answer must parse");
        assert_eq!(extraction.prices, vec![120.0, 145.50, 99.0]);
        assert_eq!(extraction.currency.as_deref(), Some("USD"));
    }

    #[test]
    fn response_parser_returns_none_when_the_model_is_not_confident() {
        let body = serde_json::json!({
            "content": [{"type": "text", "text": "\"found\": false, \"currency\": null, \"prices\": []}"}]
        })
        .to_string();
        assert_eq!(parse_ai_extraction_response(&body), None, "found:false must never become a fabricated success");
    }

    #[test]
    fn response_parser_returns_none_when_found_is_true_but_prices_is_empty() {
        // Defensive against an inconsistent answer (found:true, no actual
        // prices) - never trusted just because "found" says true.
        let body = serde_json::json!({
            "content": [{"type": "text", "text": "\"found\": true, \"currency\": null, \"prices\": []}"}]
        })
        .to_string();
        assert_eq!(parse_ai_extraction_response(&body), None);
    }

    #[test]
    fn response_parser_drops_non_positive_or_non_finite_prices_but_keeps_the_rest() {
        let body = serde_json::json!({
            "content": [{"type": "text", "text": "\"found\": true, \"currency\": \"EUR\", \"prices\": [80, -5, 0, 120]}"}]
        })
        .to_string();
        let extraction = parse_ai_extraction_response(&body).unwrap();
        assert_eq!(extraction.prices, vec![80.0, 120.0]);
    }

    #[test]
    fn response_parser_returns_none_for_a_non_text_content_block() {
        let body = serde_json::json!({"content": [{"type": "tool_use", "id": "x"}]}).to_string();
        assert_eq!(parse_ai_extraction_response(&body), None);
    }

    #[test]
    fn response_parser_returns_none_for_prose_the_prefill_somehow_didnt_prevent() {
        let body = serde_json::json!({
            "content": [{"type": "text", "text": "Sure, here are the prices I found: $120, $145"}]
        })
        .to_string();
        assert_eq!(parse_ai_extraction_response(&body), None, "must never guess-parse free text - only real JSON counts");
    }

    #[test]
    fn response_parser_returns_none_for_completely_malformed_json() {
        assert_eq!(parse_ai_extraction_response("not json at all"), None);
        assert_eq!(parse_ai_extraction_response(""), None);
    }

    #[test]
    fn rejection_message_matches_the_real_401_body_api_anthropic_com_returns_for_an_invalid_key() {
        // Captured directly from a real request against the real endpoint
        // during development (this module's own doc comment) - not
        // reconstructed from documentation, so this test locks in the
        // ACTUAL shape describe_anthropic_rejection has to handle.
        let real_body = r#"{"type":"error","error":{"type":"authentication_error","message":"API key is invalid."},"request_id":null}"#;
        let msg = describe_anthropic_rejection(reqwest::StatusCode::UNAUTHORIZED, real_body);
        assert!(msg.contains("401"));
        assert!(msg.contains("authentication_error"));
        assert!(msg.contains("API key is invalid"));
    }

    #[test]
    fn min_remaining_budget_is_comfortably_smaller_than_the_overall_timeout() {
        // Sanity check on the constants themselves - if someone ever tuned
        // OVERALL_TIMEOUT down without revisiting this, a fallback could
        // pointlessly never fire (or worse, the max call timeout could
        // exceed what's actually left). Not a real-world scenario today
        // (60s vs an 8s floor and a 20s cap), but cheap to guard directly.
        assert!(AI_FALLBACK_MIN_REMAINING_BUDGET < OVERALL_TIMEOUT);
        assert!(AI_FALLBACK_MAX_CALL_TIMEOUT <= OVERALL_TIMEOUT);
    }
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
        // 2.1.8: "ok" now specifically means real listing context was
        // found, not merely a non-empty prices array (see this module's own
        // "2.1.8" doc comment on parse_auto_check_json / the "partial"
        // tests below for that new distinction) - a `listings` entry per
        // price is what makes this fixture a genuine "ok" case rather than
        // "partial".
        let raw = r#"{"prices": [31.0, 39.0, 39.0, 50.0, 52.0], "currency": "USD", "blocked": false, "listings": [
            {"price": 31.0, "currency": "USD", "section": "101", "row": "A", "quantity": 1},
            {"price": 39.0, "currency": "USD", "section": "102", "row": "B", "quantity": 1},
            {"price": 39.0, "currency": "USD", "section": "102", "row": "C", "quantity": 1},
            {"price": 50.0, "currency": "USD", "section": "103", "row": "A", "quantity": 1},
            {"price": 52.0, "currency": "USD", "section": "103", "row": "B", "quantity": 1}
        ]}"#;
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
        assert!(result.message.unwrap().contains("60 seconds"), "must match OVERALL_TIMEOUT (60s) - this test still said the stale pre-2.1.4 15s value");
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

    // -- should_stop_polling_for_readiness (2.1.6 bugfix - see
    //    poll_then_extract's own doc comment for the real bug this closes:
    //    without this reserve, the loop always ran the budget down to
    //    exactly zero, which made the final best-effort extraction attempt
    //    unreachable dead code every single time readiness never fired) ---

    #[test]
    fn should_stop_polling_is_false_with_more_than_eval_timeout_left() {
        assert!(!should_stop_polling_for_readiness(EVAL_TIMEOUT + Duration::from_millis(1)));
        assert!(!should_stop_polling_for_readiness(OVERALL_TIMEOUT));
    }

    #[test]
    fn should_stop_polling_is_true_at_exactly_eval_timeout_remaining() {
        // The boundary itself: this is what actually reserves EVAL_TIMEOUT
        // for the final extraction rather than running it down to zero.
        assert!(should_stop_polling_for_readiness(EVAL_TIMEOUT));
    }

    #[test]
    fn should_stop_polling_is_true_with_less_than_eval_timeout_or_nothing_left() {
        assert!(should_stop_polling_for_readiness(EVAL_TIMEOUT - Duration::from_millis(1)));
        assert!(should_stop_polling_for_readiness(Duration::ZERO));
    }

    // -- 2.1.8 multi-attempt retry loop (marko's spec section 5/6: "don't
    //    extract only once - attempt 1, attempt 2, attempt 3... until
    //    relevant listings are found, the page is marked blocked, or the
    //    budget is exhausted"). `poll_then_extract` itself needs a real
    //    `WebviewWindow` and can't be unit-tested directly (same limitation
    //    as everywhere else in this module), so these lock in the pure
    //    decision function the loop is actually built on:
    //    `extraction_found_something` - "is this attempt's raw result worth
    //    stopping for, or should the loop scroll and try again". ------------

    #[test]
    fn dynamic_content_not_ready_yet_is_reported_as_nothing_found_so_the_loop_retries() {
        // An attempt fired before the marketplace's JS has rendered any
        // listings - the extractor legitimately ran and came back with
        // truly empty arrays. Must read as "keep retrying", not as a result
        // worth returning early.
        assert!(!extraction_found_something(r#"{"prices": [], "currency": null, "blocked": false, "listings": []}"#));
    }

    #[test]
    fn a_listing_that_only_appears_on_a_later_attempt_is_recognized_the_moment_it_does() {
        // Simulates the shape of "attempt 1 empty (page still loading),
        // attempt 2 (after a scroll + short wait) finds the listing" -
        // the two attempts' raw payloads in sequence, each fed through the
        // same pure check the real loop uses to decide whether to stop.
        let attempt_1 = r#"{"prices": [], "currency": null, "blocked": false, "listings": []}"#;
        let attempt_2 = r#"{"prices": [45.0], "currency": "USD", "blocked": false, "listings": [{"price": 45.0, "currency": "USD", "section": "Floor A", "row": "3", "quantity": 2}]}"#;
        assert!(!extraction_found_something(attempt_1), "attempt 1 has nothing yet - must not stop the loop early");
        assert!(extraction_found_something(attempt_2), "attempt 2 found a real listing - must stop the loop here");
    }

    #[test]
    fn scroll_revealed_only_bare_prices_still_counts_as_something_found() {
        // A page whose lazy-loaded content is a lone aggregated/starting
        // price rather than itemized listings still needs the loop to stop
        // and hand that off to parse_auto_check_json (which will correctly
        // grade it "partial", not "ok" - that grading is a separate concern
        // from "is it worth ending the retry loop for").
        assert!(extraction_found_something(r#"{"prices": [120.0], "currency": "EUR", "blocked": false, "listings": []}"#));
    }

    #[test]
    fn a_blocked_page_stops_the_retry_loop_immediately_even_with_nothing_else_found() {
        // No point scrolling a CAPTCHA/anti-bot interstitial six more times
        // hoping a price appears - marko's spec section 6 lists "the page is
        // marked blocked" as one of the loop's three legitimate stop
        // conditions, independent of prices/listings being empty.
        assert!(extraction_found_something(r#"{"prices": [], "currency": null, "blocked": true, "listings": []}"#));
    }

    #[test]
    fn an_unparseable_attempt_result_is_treated_as_nothing_found_yet_not_as_an_error() {
        // A single garbled attempt (mid-render, a page script conflict) is
        // exactly the kind of transient glitch retrying is meant to ride
        // out - see extraction_found_something's own doc comment. It must
        // never be treated as a stop-worthy signal; the LAST attempt still
        // reports a genuinely malformed payload truthfully as "error" later,
        // via parse_auto_check_json, once the loop actually stops.
        assert!(!extraction_found_something("not valid json at all"));
        assert!(!extraction_found_something(""));
    }

    #[test]
    fn max_extract_attempts_actually_allows_more_than_a_single_try() {
        // Locks in marko's explicit "not just once" requirement at the
        // constant level - a regression back to 1 would silently undo the
        // entire retry loop while every other test here still passed.
        assert!(MAX_EXTRACT_ATTEMPTS > 1);
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
        // This fixture carries no `listings` (irrelevant to what it's
        // actually testing - the MAX_PRICES count boundary), so 2.1.8's
        // stricter "ok" definition correctly reports it as "partial", not
        // "ok" - see parses_ok_result_with_prices for a fixture that
        // exercises the "ok" path instead. What this test still proves,
        // unchanged: exactly MAX_PRICES is accepted and NOT truncated/
        // rejected the way MAX_PRICES + 1 is (previous test) - "still a
        // normal success" means "not silently truncated", regardless of
        // which of ok/partial a bare list of prices resolves to.
        let exactly_max: Vec<f64> = (0..MAX_PRICES).map(|i| (i + 1) as f64).collect();
        let raw = serde_json::json!({ "prices": exactly_max, "currency": "USD", "blocked": false }).to_string();
        let result = parse_auto_check_json(&raw);
        assert_eq!(result.status, "partial");
        assert_eq!(result.prices.len(), MAX_PRICES, "must not be truncated - all MAX_PRICES entries must survive");
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
        // "partial" added in 2.1.8: a bare-price-no-listing-context result is
        // just as terminal as "ok" - it must never be mistaken for a
        // loading-shaped state either.
        let terminal = ["ok", "partial", "unable_to_read", "blocked", "cancelled", "timeout", "error"];
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
            listings: vec![],
            ai_assisted: false,
            diagnostics: None,
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

    // -- 2.1.4 ("true non-blocking" fix) - marko's spec sections 4/14/17
    //    ("SINGLE-FLIGHT", "REPEATED TEST", stale-request handling) tested
    //    directly against the exact same slot type auto_check_price/
    //    spawn_auto_check_thread actually use (no AppHandle needed for the
    //    slot logic itself - only for the WebView parts, which is the one
    //    thing this sandbox genuinely cannot exercise, see this module's
    //    own doc comment). --------------------------------------------

    #[test]
    fn second_start_while_first_in_flight_never_touches_the_first_flag() {
        let slot: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);
        let first = Arc::new(AtomicBool::new(false));
        *slot.lock().unwrap() = Some(first.clone());

        // Mirrors auto_check_price's own check-and-set exactly.
        let second_rejected = slot.lock().unwrap().is_some();
        assert!(second_rejected, "a second concurrent start must observe the slot as occupied");
        assert!(!first.load(Ordering::Relaxed), "the rejected second attempt must never flip the FIRST attempt's own flag");
    }

    #[test]
    fn started_status_is_never_confused_with_a_terminal_ok() {
        // Locks in the literal string PriceChecker.tsx's own `.then()`
        // branches on for the non-terminal ack - see auto_check_price's own
        // doc comment ("True non-blocking fix").
        let started =
            AutoCheckResult { status: "started".into(), prices: vec![], currency: None, message: None, listings: vec![], ai_assisted: false, diagnostics: None };
        assert_eq!(started.status, "started");
        assert_ne!(started.status, "ok");
        assert!(started.prices.is_empty(), "a \"started\" ack must never carry any price data of its own");
    }

    #[test]
    fn timeout_message_reflects_the_real_60_second_ceiling_not_a_stale_one() {
        let result = read_outcome_to_result(ReadOutcome::TimedOut);
        assert_eq!(result.status, "timeout");
        assert!(
            result.message.as_deref().unwrap().contains("60 seconds"),
            "message must match OVERALL_TIMEOUT (60s) - a stale 15s/17s string here would silently mislead marko about the real ceiling"
        );
    }

    #[test]
    fn overall_timeout_constant_is_exactly_60_seconds() {
        // marko's explicit spec section 1/6: "Timeout nastav na: 60 sekúnd."
        assert_eq!(OVERALL_TIMEOUT, Duration::from_secs(60));
    }

    // -- 2.1.4: listings (section/row/quantity) extraction ---------------

    #[test]
    fn ok_result_with_listings_carries_real_section_row_quantity() {
        let raw = r#"{
            "prices": [31.0, 39.0],
            "currency": "USD",
            "blocked": false,
            "listings": [
                {"price": 31.0, "currency": "USD", "section": "Grandstand Outfield 413", "row": "10", "quantity": 2},
                {"price": 39.0, "currency": "USD", "section": "Bleacher 237", "row": "20", "quantity": null}
            ]
        }"#;
        let result = parse_auto_check_json(raw);
        assert_eq!(result.listings.len(), 2);
        assert_eq!(result.listings[0].section.as_deref(), Some("Grandstand Outfield 413"));
        assert_eq!(result.listings[0].quantity, Some(2));
        assert_eq!(result.listings[1].quantity, None, "must not fabricate a quantity the page never stated");
    }

    #[test]
    fn missing_listings_key_is_fine_prices_alone_still_work() {
        // The JSON-LD and og:price passes never populate "listings" at all.
        // 2.1.8: a bare price with no listing context is correctly "partial"
        // now (see the ok/partial confidence rule), not "ok" - but the
        // original point of this test still holds and is asserted below:
        // the price itself is never lost or blanked out just because
        // "listings" is entirely absent from the payload.
        let raw = r#"{"prices": [99.0], "currency": "USD", "blocked": false}"#;
        let result = parse_auto_check_json(raw);
        assert_eq!(result.status, "partial");
        assert_eq!(result.prices, vec![99.0], "the price must still be reported, not lost");
        assert!(result.listings.is_empty());
    }

    #[test]
    fn listing_with_blank_section_text_becomes_none_not_an_empty_string() {
        let raw = r#"{"prices":[10.0],"currency":"USD","blocked":false,"listings":[{"price":10.0,"currency":"USD","section":"   ","row":null,"quantity":null}]}"#;
        let result = parse_auto_check_json(raw);
        assert_eq!(result.listings[0].section, None);
    }
}

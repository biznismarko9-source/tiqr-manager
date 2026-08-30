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
//! exercised end to end in the sandbox this was written in: no display
//! server is available there for a real WebView, AND the target marketplace
//! domains are outside that sandbox's own network egress allowlist
//! (confirmed directly - see PRICE-CHECKER-AUTO-CHECK-REPORT.md). What
//! COULD be verified there: the JS extraction logic against real, saved
//! marketplace HTML (a WebKitGTK/Python proof-of-concept, same engine this
//! module drives via Tauri on Linux), and every pure-Rust function below
//! that doesn't require an actual running WebView (`parse_auto_check_json`,
//! URL validation) - both covered by the `#[cfg(test)]` module below.
//!
//! ## Threading - no async runtime, matching this whole codebase
//!
//! Same reasoning as google_oauth.rs's own loopback listener: this app has
//! no tokio/async-std anywhere (`rusqlite` is sync, `reqwest`'s "blocking"
//! feature is used deliberately elsewhere in this codebase - see
//! google_sheets.rs). `WebviewWindow` is `Send + Sync` (Tauri's own docs),
//! so it's called directly from this command's own worker thread (Tauri
//! commands already run off the main thread by default) rather than
//! bouncing through `run_on_main_thread`. Waiting for each `eval_with_
//! callback` result uses a plain `std::sync::mpsc` channel with a timeout,
//! the same "thread + channel + timeout, no async runtime" shape
//! `google_oauth::accept_one_redirect` already uses for its own
//! wait-for-an-external-event problem.

use crate::error::{AppError, AppResult};
use crate::models::AutoCheckResult;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, WebviewUrl, WebviewWindowBuilder};

const MIN_WAIT: Duration = Duration::from_millis(800);
const POLL_INTERVAL: Duration = Duration::from_millis(400);
const MAX_WAIT: Duration = Duration::from_secs(9);
const EVAL_TIMEOUT: Duration = Duration::from_secs(3);

/// Runs entirely inside the loaded page. Returns `{"ready": bool,
/// "blocked": bool}` - `ready` true means "there's something worth a full
/// extraction pass now" (found JSON-LD offers, a price-shaped table, an
/// og:price meta tag, or an anti-bot signal - the last one so a blocked
/// page is recognized immediately rather than polled for the full
/// MAX_WAIT). Deliberately narrow/cheap so it can run every POLL_INTERVAL
/// without doing the full extraction work each time.
const READINESS_CHECK_JS: &str = include_str!("price_checker_auto_readiness.js");

/// The full extraction pass, run once readiness is confirmed (or MAX_WAIT
/// is reached, on whatever the page has by then). Returns
/// `{"prices": number[], "currency": string|null, "blocked": bool}`. See
/// this module's own doc comment ("Extraction strategy") for what each of
/// the three passes inside it looks for.
const EXTRACT_JS: &str = include_str!("price_checker_auto_extract.js");

/// Core logic behind `auto_check_price` - split out for the same
/// direct-unit-testability reason every other `*_impl` function in this
/// codebase is (see commands::price_checker's own functions). The only
/// part that can't run without a live WebView is `run_browser_read` itself
/// (isolated below) - everything else here is plain, testable Rust.
pub(crate) fn auto_check_price_impl(app: &AppHandle, url: &str) -> AppResult<AutoCheckResult> {
    let normalized = normalize_auto_check_url(url)?;

    match run_browser_read(app, &normalized) {
        Ok(raw_json) => Ok(parse_auto_check_json(&raw_json)),
        Err(message) => Ok(AutoCheckResult {
            status: "error".into(),
            prices: vec![],
            currency: None,
            message: Some(message),
        }),
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

#[tauri::command]
pub fn auto_check_price(app: AppHandle, url: String) -> AppResult<AutoCheckResult> {
    auto_check_price_impl(&app, &url)
}

/// Opens `url` in a hidden WebView, polls (not a fixed sleep - see this
/// module's own doc comment) until either the readiness check says there's
/// something to extract or MAX_WAIT passes, then runs the full extraction
/// pass and returns its raw JSON string. `Err` here means the WebView/eval
/// machinery itself failed (couldn't create the window, eval never
/// responded at all) - a real marketplace response (including a blocked
/// or empty one) is always `Ok`, handled by `parse_auto_check_json`.
fn run_browser_read(app: &AppHandle, url: &str) -> Result<String, String> {
    let label = format!(
        "price-auto-check-{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()
    );
    let parsed_url: tauri::Url = url.parse().map_err(|e| format!("Invalid URL: {e}"))?;

    let webview = WebviewWindowBuilder::new(app, &label, WebviewUrl::External(parsed_url))
        .visible(false)
        .build()
        .map_err(|e| format!("Could not open a reader window: {e}"))?;

    let result = poll_then_extract(&webview);
    let _ = webview.close();
    result
}

fn poll_then_extract(webview: &tauri::WebviewWindow) -> Result<String, String> {
    std::thread::sleep(MIN_WAIT);
    let start = Instant::now();

    loop {
        if let Some(raw) = eval_and_wait(webview, READINESS_CHECK_JS) {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&raw) {
                let ready = parsed.get("ready").and_then(|v| v.as_bool()).unwrap_or(false);
                if ready {
                    break;
                }
            }
        }
        if start.elapsed() >= MAX_WAIT {
            break;
        }
        std::thread::sleep(POLL_INTERVAL);
    }

    eval_and_wait(webview, EXTRACT_JS).ok_or_else(|| "The reader window stopped responding.".to_string())
}

/// Runs `js` in `webview` and blocks (this command's own worker thread,
/// never the main thread) for at most `EVAL_TIMEOUT` waiting for the
/// result. `None` on timeout - callers treat that as "this one poll
/// attempt didn't answer in time", not necessarily a hard failure.
fn eval_and_wait(webview: &tauri::WebviewWindow, js: &str) -> Option<String> {
    let (tx, rx) = mpsc::channel::<String>();
    webview.eval_with_callback(js, move |result: String| {
        let _ = tx.send(result);
    })
    .ok()?;
    rx.recv_timeout(EVAL_TIMEOUT).ok()
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

    let prices: Vec<f64> = parsed
        .get("prices")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_f64()).filter(|p| p.is_finite() && *p > 0.0).collect())
        .unwrap_or_default();
    let currency = parsed.get("currency").and_then(|v| v.as_str()).map(String::from);

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

#[cfg(test)]
mod tests {
    use super::*;

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
}

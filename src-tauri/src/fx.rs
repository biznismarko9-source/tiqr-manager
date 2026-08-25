//! Live currency-conversion rate lookup - Frankfurter
//! (<https://frankfurter.dev>), a free, no-API-key exchange-rate service
//! that tracks the European Central Bank's own reference rates. Added in
//! 2.0.50 for the "Convert to EUR" action on the New Order form: marko
//! wanted a foreign-currency purchase (GBP, USD, ...) converted to its EUR
//! equivalent at today's real rate in a couple of clicks, instead of
//! looking the rate up himself and doing the math by hand. See
//! commands::currency for the thin Tauri command wrapper that actually
//! calls this, and Orders.tsx's `OrderFormModal` for the one place in the
//! UI that uses it today.
//!
//! Deliberately just a rate lookup + a cents-rounding helper, not a general
//! "currency" subsystem - this app's currency/amount fields are otherwise
//! set once at order-creation time and never rescaled afterward (see
//! PROTECTED-AREAS-NOTES.md's 2.0.50 section for why this feature is
//! scoped to the New Order form specifically, and not to editing an
//! existing order/ticket/sale's currency).
//!
//! **Cannot be live-tested from this sandbox**: outbound network access
//! here is a narrow allowlist and api.frankfurter.dev is not on it
//! (confirmed via direct `curl` reachability testing before this was
//! built - api.anthropic.com reachable, seven different FX-rate APIs
//! including this one all unreachable). Verified instead by (a) fetching
//! the real, live endpoint through the research WebFetch tool, which runs
//! on different infrastructure than this sandbox's own network and
//! confirmed both the exact URL (`api.frankfurter.app`'s classic
//! `/latest?from=..&to=..` shape 302-redirects to
//! `api.frankfurter.dev/v1/latest?from=..&to=..`) and the real JSON
//! response shape used below, and (b) unit tests against that confirmed
//! shape. The actual live call can only be exercised for real on marko's
//! own machine - same discipline already established for Google
//! OAuth/Sheets/Firebase in this app.

use crate::error::{AppError, AppResult};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde::Deserialize;
use std::collections::HashMap;

const FRANKFURTER_URL: &str = "https://api.frankfurter.dev/v1/latest";

#[derive(Debug, Deserialize)]
struct FrankfurterResponse {
    date: String,
    rates: HashMap<String, f64>,
}

/// A single currency-pair rate, as of the date Frankfurter last published it.
#[derive(Debug, Clone, PartialEq)]
pub struct RateQuote {
    /// How many units of `to` one unit of `from` is worth right now.
    pub rate: f64,
    /// The date this rate was published for, e.g. "2026-08-25" - shown to
    /// marko so a conversion is never a silent black box (same "always show
    /// what happened" spirit as every other money computation in this app).
    pub date: String,
}

/// Fetches today's reference rate to convert 1 unit of `from` into `to`
/// (e.g. `from="GBP", to="EUR"` -> how many EUR one GBP is worth right now).
/// `from`/`to` are trimmed and normalized to uppercase before use (both for
/// the same-currency shortcut below and for the actual request/lookup) -
/// beyond that, no validation is done here. Frankfurter itself rejects a
/// code it doesn't recognise with a clear error body, which is passed
/// straight through rather than guessed at (same "surface the real
/// service's own message" approach `google_sheets::describe_error_response`
/// already uses for Google's errors).
pub fn fetch_rate(from: &str, to: &str) -> AppResult<RateQuote> {
    let from = from.trim().to_ascii_uppercase();
    let to = to.trim().to_ascii_uppercase();

    if from == to {
        // No real request needed, and nothing downstream should ever hit
        // this from the UI (the "Convert to EUR" button is only shown when
        // the order's currency isn't already EUR) - kept as a defensive
        // shortcut for any future caller, not a case this app's own UI
        // triggers today.
        return Ok(RateQuote {
            rate: 1.0,
            date: chrono::Local::now().format("%Y-%m-%d").to_string(),
        });
    }

    // Built by hand, same as every other outbound URL in this app
    // (google_sheets.rs's `sheets_values_url` etc.) rather than reqwest's
    // own `.query()` builder - that method needs the `query` cargo feature,
    // which this crate deliberately doesn't enable (reqwest is already a
    // dependency purely for the existing hand-rolled Google REST client, see
    // Cargo.toml's comment above the `reqwest` line - no reason to widen it
    // just for two extra query params here when percent-encoding is already
    // a dependency for exactly this purpose).
    let encoded_from = utf8_percent_encode(&from, NON_ALPHANUMERIC);
    let encoded_to = utf8_percent_encode(&to, NON_ALPHANUMERIC);
    let url = format!("{FRANKFURTER_URL}?from={encoded_from}&to={encoded_to}");

    let client = reqwest::blocking::Client::new();
    let resp = client
        .get(&url)
        .send()
        .map_err(|e| AppError::External(format!("could not reach the exchange-rate service: {e}")))?;

    let status = resp.status();
    let body = resp.text().map_err(|e| {
        AppError::External(format!("could not read the exchange-rate service's response: {e}"))
    })?;
    if !status.is_success() {
        return Err(AppError::External(describe_rejected_request(status, &body)));
    }

    parse_rate_response(&body, &to)
}

/// Formats a clear message when Frankfurter rejects a request outright
/// (non-2xx status). Split out of `fetch_rate` purely so it's directly
/// unit-testable the way `google_sheets::describe_error_response` already
/// is for Google's errors: a real `reqwest::blocking::Response` can't be
/// constructed without an actual HTTP round trip (which this sandbox can't
/// make either - see this module's doc comment), but a `StatusCode` and a
/// body string can be built by hand in a test, so keeping this as a plain
/// function taking exactly those two things keeps it testable despite that.
fn describe_rejected_request(status: reqwest::StatusCode, body: &str) -> String {
    format!("the exchange-rate service rejected the request ({status}): {body}")
}

/// Parses a successful (2xx) response body and pulls out the rate for `to`.
/// Split out of `fetch_rate` for the same testability reason as
/// `describe_rejected_request` above - this one needs no `Response` at all,
/// just the body text, so both its success path and its two failure paths
/// (malformed JSON; valid JSON that simply has no rate for `to`) can be
/// tested directly against a hand-authored string.
fn parse_rate_response(body: &str, to: &str) -> AppResult<RateQuote> {
    let parsed: FrankfurterResponse = serde_json::from_str(body).map_err(|e| {
        AppError::External(format!(
            "could not understand the exchange-rate service's response: {e} (body: {body})"
        ))
    })?;
    let rate = parsed.rates.get(to).copied().ok_or_else(|| {
        AppError::External(format!(
            "the exchange-rate service did not return a rate for '{to}' (response: {body})"
        ))
    })?;
    Ok(RateQuote { rate, date: parsed.date })
}

/// Converts an integer-cents amount by `rate`, rounding to the nearest whole
/// cent. `f64::round()` rounds half-way cases away from zero, the same rule
/// `money::round_decimal_to_cents` uses (the plain rounding a non-technical
/// user expects) - conceptually the same convention, though not a byte-for-
/// byte equivalent guarantee: money.rs decides ties by inspecting exact
/// decimal-string digits (no floating point involved at all), while this
/// multiplies by a float rate first, so an amount landing *exactly* on a
/// half-cent tie is only reliably decided the same way when it arrives at
/// that tie via decimal text, not via float multiplication (see this
/// module's own tests for why that distinction matters and how it's tested
/// around instead of over-claimed). In practice this is a non-issue for any
/// realistic ticket-reselling amount - never left as a float and never
/// truncated either way.
pub fn convert_cents(cents: i64, rate: f64) -> i64 {
    (cents as f64 * rate).round() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fetch_rate_short_circuits_to_1_0_when_from_and_to_are_the_same_currency() {
        // The only part of fetch_rate this sandbox can exercise without real
        // network access - proves the shortcut works and, just as
        // importantly, that it never reaches the `reqwest` call below it.
        let q = fetch_rate("EUR", "EUR").unwrap();
        assert_eq!(q.rate, 1.0);
        let q2 = fetch_rate("gbp", "GBP").unwrap();
        assert_eq!(q2.rate, 1.0, "case-insensitive - a free-typed lowercase code must still short-circuit");
    }

    /// A real response body fetched live via WebFetch (which runs on
    /// different infrastructure than this sandbox, so it could actually
    /// reach the API) from `https://api.frankfurter.dev/v1/latest?from=GBP&to=EUR`
    /// on 2026-08-25 - `{"amount":1.0,"base":"GBP","date":"2026-08-25","rates":{"EUR":1.1689}}`.
    /// Proves `FrankfurterResponse` actually parses the real, current shape
    /// of this API, not a guessed/remembered one - the closest thing to an
    /// integration test this sandbox can run for a network call it can't
    /// make itself.
    #[test]
    fn frankfurter_response_parses_a_real_captured_response_body() {
        let body = r#"{"amount":1.0,"base":"GBP","date":"2026-08-25","rates":{"EUR":1.1689}}"#;
        let parsed: FrankfurterResponse = serde_json::from_str(body).unwrap();
        assert_eq!(parsed.date, "2026-08-25");
        assert_eq!(parsed.rates.get("EUR").copied(), Some(1.1689));
    }

    #[test]
    fn convert_cents_applies_marko_s_own_worked_example() {
        // "mam 20 GBP, podla aktualneho kurzu je teraz 23,38 eur" - his own
        // example from the request, using the real rate captured above
        // (1 GBP = 1.1689 EUR): 2000 cents * 1.1689 = 2337.8 -> rounds to
        // 2338 cents = 23.38 EUR, matching what he described almost exactly
        // (his own "23.38" was presumably from a slightly different moment's
        // rate - the point is the rounding behaviour, not an exact rate match).
        assert_eq!(convert_cents(2000, 1.1689), 2338);
    }

    #[test]
    fn convert_cents_rounds_to_the_nearest_cent() {
        // Deliberately NOT testing an exact X.5 boundary here (e.g.
        // "100 cents * a rate that lands on exactly 100.5"): a rate like
        // 1.005 has no exact f64 representation, so whether the product
        // lands a hair above or below the true half-cent boundary depends
        // on binary floating-point rounding, not on the half-away-from-zero
        // rule `f64::round()` implements - that rule is real and does apply
        // to a genuine tie, it's just not reliably constructible from an
        // arbitrary float rate the way it is from money.rs's own exact
        // decimal-string parsing. Clearly-above/below cases are robust
        // regardless of that imprecision, which is what actually matters
        // for a real Frankfurter rate like 1.1689.
        assert_eq!(convert_cents(100, 1.006), 101, "100.6 -> clearly rounds up");
        assert_eq!(convert_cents(100, 1.004), 100, "100.4 -> clearly rounds down");
        assert_eq!(convert_cents(0, 1.5), 0);
    }

    #[test]
    fn convert_cents_handles_a_rate_below_1_without_losing_a_cent() {
        // EUR -> USD-style direction (rate < 1), e.g. converting toward a
        // stronger currency - same rounding rule applies either way.
        assert_eq!(convert_cents(1000, 0.856), 856);
    }

    #[test]
    fn describe_rejected_request_surfaces_the_real_status_and_body() {
        let msg = describe_rejected_request(reqwest::StatusCode::NOT_FOUND, "currency code unknown");
        assert!(msg.contains("404"));
        assert!(msg.contains("currency code unknown"));
    }

    #[test]
    fn parse_rate_response_reads_the_real_captured_shape() {
        // Same real body captured via WebFetch as
        // frankfurter_response_parses_a_real_captured_response_body above -
        // this test proves the whole parse_rate_response path (not just
        // FrankfurterResponse's own deserialization) ends in the right
        // RateQuote.
        let body = r#"{"amount":1.0,"base":"GBP","date":"2026-08-25","rates":{"EUR":1.1689}}"#;
        let quote = parse_rate_response(body, "EUR").unwrap();
        assert_eq!(quote.rate, 1.1689);
        assert_eq!(quote.date, "2026-08-25");
    }

    #[test]
    fn parse_rate_response_errors_clearly_on_malformed_json() {
        let err = parse_rate_response("not json at all", "EUR").unwrap_err();
        assert!(err.to_string().contains("could not understand"));
    }

    #[test]
    fn parse_rate_response_errors_clearly_when_the_requested_currency_is_missing() {
        // A syntactically valid Frankfurter-shaped body that just doesn't
        // happen to contain the currency being asked for - e.g. a typo'd
        // free-typed currency code that Frankfurter still accepted as a
        // valid `to` for some OTHER currency but not this one. Must not
        // panic on a missing HashMap key, and must say which code is
        // missing rather than a generic failure.
        let body = r#"{"amount":1.0,"base":"GBP","date":"2026-08-25","rates":{"USD":1.27}}"#;
        let err = parse_rate_response(body, "EUR").unwrap_err();
        assert!(err.to_string().contains("EUR"));
    }
}

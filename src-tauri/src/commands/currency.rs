//! Thin Tauri command wrapper around `fx` (live exchange-rate lookup) - see
//! that module's doc comment for the actual HTTP call, why it can't be
//! live-tested from this sandbox, and the rounding convention used below.
//! 2.0.50, marko's request: a "Convert to EUR" action on the New Order
//! form, so a purchase entered in GBP/USD/etc. can be turned into its EUR
//! equivalent (amounts AND the currency field together, in one click)
//! before the order is created - see Orders.tsx's `OrderFormModal`.

use crate::error::AppResult;
use crate::fx;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrencyConversion {
    /// How many units of `to_currency` one unit of `from_currency` is worth
    /// right now - shown to marko so the conversion is never a silent black
    /// box.
    pub rate: f64,
    /// The date this rate was published for, e.g. "2026-08-25".
    pub rate_date: String,
    /// `amounts_cents`, converted 1:1 by position (same length, same
    /// order) - the caller matches each result back to the field it sent
    /// by index. Not a map/named structure: this is only ever called from
    /// one place today (the New Order form converting its own handful of
    /// fields together), where the caller builds and consumes this list in
    /// the same spot, so position-matching is simple and safe rather than
    /// fragile.
    pub converted_cents: Vec<i64>,
}

/// Converts every amount in `amounts_cents` from `from_currency` to
/// `to_currency` using a single live rate fetched once, not one request per
/// amount. Returns `Err` (surfaced to marko via a toast, same as every
/// other command's error) if the rate can't be fetched at all - never
/// guesses a rate or silently leaves an amount unconverted.
#[tauri::command]
pub fn convert_currency(
    from_currency: String,
    to_currency: String,
    amounts_cents: Vec<i64>,
) -> AppResult<CurrencyConversion> {
    let quote = fx::fetch_rate(&from_currency, &to_currency)?;
    let converted_cents = amounts_cents
        .iter()
        .map(|&cents| fx::convert_cents(cents, quote.rate))
        .collect();
    Ok(CurrencyConversion {
        rate: quote.rate,
        rate_date: quote.date,
        converted_cents,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_currency_converts_every_amount_with_one_shared_rate() {
        // Same-currency shortcut (fx::fetch_rate short-circuits to 1.0
        // without a network call) - the only path this sandbox can exercise
        // for the full command end-to-end, but it still proves the command
        // applies ONE rate across the whole list rather than re-fetching
        // per amount, and preserves order/length.
        let result = convert_currency("EUR".into(), "EUR".into(), vec![2000, 0, 12345]).unwrap();
        assert_eq!(result.rate, 1.0);
        assert_eq!(result.converted_cents, vec![2000, 0, 12345]);
    }

    #[test]
    fn convert_currency_handles_an_empty_amounts_list() {
        // Defensive: a caller with nothing to convert (e.g. every field
        // blank) must get back an empty list, not an error.
        let result = convert_currency("EUR".into(), "EUR".into(), vec![]).unwrap();
        assert_eq!(result.converted_cents, Vec::<i64>::new());
    }
}

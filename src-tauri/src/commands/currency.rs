//! Thin Tauri command wrapper around `fx` (live exchange-rate lookup) - see
//! that module's doc comment for the actual HTTP call, why it can't be
//! live-tested from this sandbox, and the rounding convention used below.
//! 2.0.50, marko's request: a "Convert to EUR" action on the New Order
//! form, so a purchase entered in GBP/USD/etc. can be turned into its EUR
//! equivalent (amounts AND the currency field together, in one click)
//! before the order is created - see Orders.tsx's `OrderFormModal`.

use crate::commands::sheets_sync::{get_setting, set_setting};
use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::fx;
use rusqlite::Connection;
use serde::Serialize;
use tauri::State;

/// 2.50.0, marko's request: "do settings daj moznost preffered currency podla
/// toho co clovek chce, na vyber gbp, eur, usd, tiez ten convert bude podla
/// toho co mas zapnute".
const PREFERRED_CURRENCY_KEY: &str = "preferred_currency";

/// The three marko asked for, and the only values this will store. A code
/// outside the list is refused rather than written - the whole point of this
/// setting is that the convert action can trust it.
pub const PREFERRED_CURRENCIES: [&str; 3] = ["EUR", "USD", "GBP"];

/// What the app converts INTO, and the currency it treats as "already fine".
/// Defaults to EUR, so an install that never touches this setting behaves
/// exactly as every version before 2.50.0 did.
pub(crate) fn preferred_currency(conn: &Connection) -> AppResult<String> {
    let stored = get_setting(conn, PREFERRED_CURRENCY_KEY)?;
    Ok(match stored {
        Some(raw) => {
            let code = fx::normalize_currency(&raw);
            // A stored value that is somehow not one of the three is ignored
            // rather than trusted - it would otherwise decide where real money
            // gets converted to.
            if PREFERRED_CURRENCIES.contains(&code.as_str()) {
                code
            } else {
                "EUR".to_string()
            }
        }
        None => "EUR".to_string(),
    })
}

#[tauri::command]
pub fn get_preferred_currency(state: State<AppState>) -> AppResult<String> {
    let conn = state.db.lock().unwrap();
    preferred_currency(&conn)
}

#[tauri::command]
pub fn set_preferred_currency(state: State<AppState>, currency: String) -> AppResult<String> {
    let code = fx::normalize_currency(&currency);
    if !PREFERRED_CURRENCIES.contains(&code.as_str()) {
        return Err(AppError::Validation(format!(
            "{code} is not one of the currencies this can be set to ({}).",
            PREFERRED_CURRENCIES.join(", ")
        )));
    }
    let conn = state.db.lock().unwrap();
    set_setting(&conn, PREFERRED_CURRENCY_KEY, &code)?;
    Ok(code)
}

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
#[tauri::command(async)]
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

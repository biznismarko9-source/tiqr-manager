//! All money is stored and computed as INTEGER cents. Never as f64/REAL.
//! These helpers are the only place decimal strings are parsed/formatted.

/// Parses a human-entered decimal amount (e.g. "12.3", "12,34", "12") into
/// integer cents. Rejects more than 2 decimal digits rather than silently
/// rounding, so bad CSV data is caught instead of quietly losing precision.
pub fn parse_decimal_to_cents(input: &str) -> Result<i64, String> {
    let s = input.trim();
    if s.is_empty() {
        return Err("amount is empty".to_string());
    }
    let neg = s.starts_with('-');
    let unsigned = s.trim_start_matches('-').replace(',', ".");
    let mut parts = unsigned.splitn(2, '.');
    let whole = parts.next().unwrap_or("");
    let frac = parts.next().unwrap_or("");

    if frac.len() > 2 {
        return Err(format!("'{input}' has more than 2 decimal places"));
    }
    if whole.is_empty() || !whole.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("'{input}' is not a valid amount"));
    }
    if !frac.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("'{input}' is not a valid amount"));
    }

    let whole_val: i64 = whole
        .parse()
        .map_err(|_| format!("'{input}' is not a valid amount"))?;
    let frac_padded = format!("{frac:0<2}");
    let frac_val: i64 = frac_padded
        .parse()
        .map_err(|_| format!("'{input}' is not a valid amount"))?;

    let total = whole_val * 100 + frac_val;
    Ok(if neg { -total } else { total })
}

/// Formats integer cents back into a plain decimal string, e.g. 1234 -> "12.34".
pub fn format_cents(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let abs = cents.unsigned_abs();
    format!("{sign}{}.{:02}", abs / 100, abs % 100)
}

/// 2.0.42: a lenient sibling of `parse_decimal_to_cents` for the one place
/// this app now deliberately tolerates more than 2 decimal digits instead of
/// rejecting them - commands::orders_sheet_sync's reconciliation of
/// automated order rows, where marko's own automation sometimes computes a
/// Price Per Ticket as Total Purchase Price / Number of Tickets and doesn't
/// land on a whole cent (e.g. "96.6825"). Rounds to the nearest cent
/// (round-half-up, away from zero - the plain convention non-technical users
/// expect, not banker's rounding) rather than silently truncating or
/// refusing the value outright. Still rejects anything that isn't a plain
/// decimal number at all (letters, more than one '.', empty) - this widens
/// what counts as *imprecise*, never what counts as *not a number*.
///
/// Looking at only the 3rd fractional digit to decide rounding direction is
/// deliberate, not a shortcut: for any decimal value, whether it's above,
/// below, or exactly at the X.XX5 boundary is fully determined by that one
/// digit alone (e.g. ".684xx" is always < ".685" no matter what "xx" is,
/// and ".685xx" is always >= ".685") - so digits past the 3rd can never
/// change which way this rounds and are safe to ignore.
pub fn round_decimal_to_cents(input: &str) -> Result<i64, String> {
    let s = input.trim();
    if s.is_empty() {
        return Err("amount is empty".to_string());
    }
    let neg = s.starts_with('-');
    let unsigned = s.trim_start_matches('-').replace(',', ".");
    let mut parts = unsigned.splitn(2, '.');
    let whole = parts.next().unwrap_or("");
    let frac = parts.next().unwrap_or("");

    if whole.is_empty() || !whole.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("'{input}' is not a valid amount"));
    }
    if !frac.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("'{input}' is not a valid amount"));
    }

    let whole_val: i64 = whole.parse().map_err(|_| format!("'{input}' is not a valid amount"))?;
    let digit_at = |i: usize| frac.as_bytes().get(i).map(|b| (b - b'0') as i64).unwrap_or(0);
    let mut frac_val = digit_at(0) * 10 + digit_at(1);
    let mut whole_val = whole_val;
    if digit_at(2) >= 5 {
        frac_val += 1;
        if frac_val == 100 {
            frac_val = 0;
            whole_val += 1;
        }
    }

    let total = whole_val * 100 + frac_val;
    Ok(if neg { -total } else { total })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_amounts() {
        assert_eq!(parse_decimal_to_cents("12.34").unwrap(), 1234);
        assert_eq!(parse_decimal_to_cents("12").unwrap(), 1200);
        assert_eq!(parse_decimal_to_cents("12.3").unwrap(), 1230);
        assert_eq!(parse_decimal_to_cents("0.05").unwrap(), 5);
        assert_eq!(parse_decimal_to_cents("12,34").unwrap(), 1234);
        assert_eq!(parse_decimal_to_cents("-5.00").unwrap(), -500);
    }

    #[test]
    fn rejects_bad_amounts() {
        assert!(parse_decimal_to_cents("").is_err());
        assert!(parse_decimal_to_cents("abc").is_err());
        assert!(parse_decimal_to_cents("12.345").is_err());
    }

    #[test]
    fn formats_round_trip() {
        assert_eq!(format_cents(1234), "12.34");
        assert_eq!(format_cents(5), "0.05");
        assert_eq!(format_cents(-500), "-5.00");
    }

    #[test]
    fn round_decimal_to_cents_agrees_with_the_strict_parser_on_plain_amounts() {
        assert_eq!(round_decimal_to_cents("12.34").unwrap(), 1234);
        assert_eq!(round_decimal_to_cents("12").unwrap(), 1200);
        assert_eq!(round_decimal_to_cents("12,34").unwrap(), 1234);
        assert_eq!(round_decimal_to_cents("-5.00").unwrap(), -500);
    }

    #[test]
    fn round_decimal_to_cents_rounds_marko_s_real_example_down() {
        // 386.73 / 4 tickets = 96.6825 - the exact value from marko's own
        // screenshot (a Price Per Ticket cell rejected for having 4 decimal
        // places). .6825 is below the .685 halfway point, so this rounds
        // down to 96.68, not up.
        assert_eq!(round_decimal_to_cents("96.6825").unwrap(), 9668);
    }

    #[test]
    fn round_decimal_to_cents_rounds_half_up_away_from_zero() {
        assert_eq!(round_decimal_to_cents("0.685").unwrap(), 69);
        assert_eq!(round_decimal_to_cents("0.6849999").unwrap(), 68, "below the halfway point regardless of trailing digits");
        assert_eq!(round_decimal_to_cents("0.6850001").unwrap(), 69, "at/above the halfway point regardless of trailing digits");
        assert_eq!(round_decimal_to_cents("-0.685").unwrap(), -69, "away from zero, not toward it");
    }

    #[test]
    fn round_decimal_to_cents_carries_a_rounded_up_fraction_into_the_whole_part() {
        assert_eq!(round_decimal_to_cents("0.996").unwrap(), 100, "must carry: 99 cents rounding up is 100, not 1.00");
        assert_eq!(round_decimal_to_cents("2.999").unwrap(), 300);
    }

    #[test]
    fn round_decimal_to_cents_still_rejects_genuinely_invalid_text() {
        assert!(round_decimal_to_cents("").is_err());
        assert!(round_decimal_to_cents("abc").is_err());
        assert!(round_decimal_to_cents("1.2.3").is_err());
    }
}

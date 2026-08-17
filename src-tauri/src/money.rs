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
}

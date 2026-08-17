//! Single shared financial calculation module.
//! Dashboard, Events, Orders and Sales all call the SAME functions here so
//! revenue / cost / profit / margin / ROI can never drift between screens.
//!
//! Definitions:
//!   total cost   = purchase cost + purchase fees + other costs, for ALL purchased tickets in scope
//!   COGS         = the same cost, but only for tickets that have actually been SOLD in scope
//!   revenue      = sum of sale prices in scope
//!   selling fees = sum of selling fees in scope
//!   profit       = revenue - COGS - selling fees   (profit is only meaningful against realized sales)
//!   margin       = profit / revenue
//!   roi          = profit / COGS
//! Division by zero never panics or throws: both ratios return None ("N/A") when their denominator is 0.

use serde::Serialize;

#[derive(Debug, Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct FinanceSummary {
    pub purchased_tickets: i64,
    pub available_tickets: i64,
    pub listed_tickets: i64,
    pub sold_tickets: i64,
    pub cancelled_tickets: i64,
    pub total_cost_cents: i64,
    pub cogs_cents: i64,
    pub revenue_cents: i64,
    pub selling_fees_cents: i64,
    pub profit_cents: i64,
    pub margin: Option<f64>,
    pub roi: Option<f64>,
    /// Some(code) when every ticket/sale contributing to this summary shares
    /// one currency; None when they don't. Never guess - a None here means
    /// the caller must not blend the amounts above into a single total.
    pub currency: Option<String>,
}

/// Safe division: None instead of NaN/Infinity when the denominator is 0.
pub fn safe_ratio(numerator: i64, denominator: i64) -> Option<f64> {
    if denominator == 0 {
        None
    } else {
        Some(numerator as f64 / denominator as f64)
    }
}

pub fn profit_cents(revenue_cents: i64, cost_cents: i64, fees_cents: i64) -> i64 {
    revenue_cents - cost_cents - fees_cents
}

#[allow(clippy::too_many_arguments)]
pub fn compute_summary(
    purchased_tickets: i64,
    available_tickets: i64,
    listed_tickets: i64,
    sold_tickets: i64,
    cancelled_tickets: i64,
    total_cost_cents: i64,
    cogs_cents: i64,
    revenue_cents: i64,
    selling_fees_cents: i64,
    currency: Option<String>,
) -> FinanceSummary {
    let profit = profit_cents(revenue_cents, cogs_cents, selling_fees_cents);
    FinanceSummary {
        purchased_tickets,
        available_tickets,
        listed_tickets,
        sold_tickets,
        cancelled_tickets,
        total_cost_cents,
        cogs_cents,
        revenue_cents,
        selling_fees_cents,
        profit_cents: profit,
        margin: safe_ratio(profit, revenue_cents),
        roi: safe_ratio(profit, cogs_cents),
        currency,
    }
}

/// Splits `total_cents` into `n` integer parts that sum EXACTLY back to
/// `total_cents` (no float rounding leakage). The first `total % n` parts
/// get one extra cent. Used to allocate order-level fees across the
/// individual tickets it generates.
pub fn allocate_cents(total_cents: i64, n: i64) -> Vec<i64> {
    if n <= 0 {
        return vec![];
    }
    let base = total_cents / n;
    let remainder = total_cents % n;
    (0..n)
        .map(|i| if i < remainder { base + 1 } else { base })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocation_sums_exactly() {
        let parts = allocate_cents(1000, 3);
        assert_eq!(parts.iter().sum::<i64>(), 1000);
        assert_eq!(parts, vec![334, 333, 333]);
    }

    #[test]
    fn allocation_zero_total() {
        let parts = allocate_cents(0, 5);
        assert_eq!(parts, vec![0, 0, 0, 0, 0]);
    }

    #[test]
    fn ratios_are_safe() {
        assert_eq!(safe_ratio(10, 0), None);
        assert_eq!(safe_ratio(10, 2), Some(5.0));
    }

    #[test]
    fn ratios_can_be_negative_without_panicking() {
        // A loss (negative numerator) against a real denominator is a normal,
        // valid ratio - it must NOT be clamped to zero or None.
        assert_eq!(safe_ratio(-500, 1000), Some(-0.5));
    }

    #[test]
    fn profit_handles_zero_revenue_zero_cost_zero_fees() {
        // A brand new event with nothing purchased and nothing sold yet.
        assert_eq!(profit_cents(0, 0, 0), 0);
    }

    #[test]
    fn profit_goes_negative_when_sold_at_a_loss() {
        // Sold for 1000, cost 1200, fees 50 -> a real loss, not clamped to 0.
        assert_eq!(profit_cents(1000, 1200, 50), -250);
    }

    #[test]
    fn compute_summary_is_all_zero_for_a_fresh_event_with_nothing_in_it() {
        let s = compute_summary(0, 0, 0, 0, 0, 0, 0, 0, 0, None);
        assert_eq!(s.profit_cents, 0);
        assert_eq!(s.margin, None); // 0 revenue -> N/A, not 0.0 or NaN
        assert_eq!(s.roi, None); // 0 cogs -> N/A, not 0.0 or NaN
        assert_eq!(s.currency, None);
    }

    #[test]
    fn compute_summary_zero_cost_but_real_revenue_gives_100_percent_margin_and_no_roi() {
        // e.g. a free/comped ticket resold for pure profit: cogs is 0 but
        // revenue isn't, so margin is defined (100%) while ROI (profit/cogs)
        // is undefined (division by zero) and must be None, not infinity.
        let s = compute_summary(1, 0, 0, 1, 0, 0, 0, 5000, 0, Some("EUR".into()));
        assert_eq!(s.profit_cents, 5000);
        assert_eq!(s.margin, Some(1.0));
        assert_eq!(s.roi, None);
    }

    #[test]
    fn compute_summary_reflects_a_loss_with_a_negative_margin_and_roi() {
        let s = compute_summary(1, 0, 0, 1, 0, 12000, 12000, 10000, 500, Some("EUR".into()));
        assert_eq!(s.profit_cents, -2500); // 10000 - 12000 - 500
        assert_eq!(s.margin, Some(-0.25));
        assert!((s.roi.unwrap() - (-2500.0 / 12000.0)).abs() < 1e-9);
    }

    #[test]
    fn compute_summary_handles_large_amounts_without_overflow_or_rounding_drift() {
        // ~ 10 million EUR in cogs/revenue - far beyond any real reseller's
        // scale, but i64 cents must still be exact here (no float creeping in).
        let cogs = 1_000_000_000_00i64; // 1,000,000,000.00 EUR
        let revenue = 1_200_000_000_00i64; // 1,200,000,000.00 EUR
        let s = compute_summary(1, 0, 0, 1, 0, cogs, cogs, revenue, 0, Some("EUR".into()));
        assert_eq!(s.profit_cents, revenue - cogs);
        assert!(s.margin.unwrap() > 0.0);
    }

    #[test]
    fn allocation_handles_a_large_ticket_count_exactly() {
        // Guards the exact-sum invariant at the upper end of a realistic bulk
        // order (matches the "large amounts" edge case from the financial audit).
        let parts = allocate_cents(999_999, 9973); // an awkward, non-round split
        assert_eq!(parts.len(), 9973);
        assert_eq!(parts.iter().sum::<i64>(), 999_999);
    }
}

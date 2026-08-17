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
}

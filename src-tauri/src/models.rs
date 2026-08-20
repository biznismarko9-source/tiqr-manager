use crate::finance::FinanceSummary;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Platform {
    pub id: i64,
    pub name: String,
    pub kind: String,
    pub is_demo: bool,
    pub created_at: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Supplier {
    pub id: i64,
    pub name: String,
    pub contact: Option<String>,
    pub is_demo: bool,
    pub created_at: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub id: i64,
    pub name: String,
    pub artist_team: Option<String>,
    pub venue: Option<String>,
    pub city: Option<String>,
    pub country: Option<String>,
    pub event_date: Option<String>,
    pub category: Option<String>,
    pub status: String,
    pub notes: Option<String>,
    pub is_demo: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EventInput {
    pub name: String,
    pub artist_team: Option<String>,
    pub venue: Option<String>,
    pub city: Option<String>,
    pub country: Option<String>,
    pub event_date: Option<String>,
    pub category: Option<String>,
    pub status: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EventWithStats {
    #[serde(flatten)]
    pub event: Event,
    pub stats: FinanceSummary,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    pub id: i64,
    pub code: String,
    pub event_id: i64,
    pub event_name: String,
    pub supplier_id: Option<i64>,
    pub supplier_name: Option<String>,
    pub platform_id: Option<i64>,
    pub platform_name: Option<String>,
    pub purchase_date: String,
    pub quantity: i64,
    pub unit_price_cents: i64,
    pub fees_cents: i64,
    pub other_costs_cents: i64,
    pub total_cost_cents: i64,
    pub currency: String,
    pub payment_status: String,
    pub notes: Option<String>,
    pub is_demo: bool,
    pub created_at: String,
    pub updated_at: String,
    pub sold_count: i64,
    pub available_count: i64,
    pub listed_count: i64,
    pub cancelled_count: i64,
}

/// Sales-side rollup for one order, computed only from that order's tickets
/// that were actually sold and never refunded - i.e. "realized" numbers, the
/// same convention `finance.rs` and every other screen already uses. Loaded
/// separately from `Order` (only when Order Detail is opened) so the main
/// order list never pays for this extra join.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OrderSalesSummary {
    pub revenue_cents: i64,
    pub selling_fees_cents: i64,
    pub cogs_cents: i64,
    pub profit_cents: i64,
    pub margin: Option<f64>,
    pub roi: Option<f64>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OrderInput {
    pub event_id: i64,
    pub supplier_id: Option<i64>,
    pub platform_id: Option<i64>,
    pub purchase_date: String,
    pub quantity: i64,
    pub unit_price_cents: i64,
    pub fees_cents: i64,
    pub other_costs_cents: i64,
    pub currency: String,
    pub payment_status: Option<String>,
    pub notes: Option<String>,
    pub ticket_type: Option<String>,
    pub section: Option<String>,
    pub row_label: Option<String>,
    /// Individual seat labels, one per generated ticket, in order. When
    /// provided (non-empty) its length must equal `quantity` - each ticket
    /// gets `seats[i]`. Leave empty/absent to generate tickets without a
    /// seat number (unchanged default behaviour).
    pub seats: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OrderEditInput {
    pub supplier_id: Option<i64>,
    pub platform_id: Option<i64>,
    pub purchase_date: String,
    pub currency: String,
    pub payment_status: String,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Ticket {
    pub id: i64,
    pub code: String,
    pub event_id: i64,
    pub event_name: String,
    pub order_id: i64,
    pub order_code: String,
    pub section: Option<String>,
    pub row_label: Option<String>,
    pub seat: Option<String>,
    pub ticket_type: Option<String>,
    pub purchase_cost_cents: i64,
    pub purchase_fees_cents: i64,
    pub other_costs_cents: i64,
    pub total_cost_cents: i64,
    pub listing_price_cents: Option<i64>,
    pub currency: String,
    pub status: String,
    pub notes: Option<String>,
    pub is_demo: bool,
    pub created_at: String,
    pub updated_at: String,
    pub sale_price_cents: Option<i64>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TicketUpdateInput {
    pub section: Option<String>,
    pub row_label: Option<String>,
    pub seat: Option<String>,
    pub ticket_type: Option<String>,
    pub listing_price_cents: Option<i64>,
    pub status: Option<String>,
    pub notes: Option<String>,
}

/// Closed set of ticket fields `bulk_update_tickets` is allowed to change.
/// Deliberately has NO `Status` variant - see `bulk_update_tickets_impl`
/// (tickets.rs) for why a naive bulk status change is unsafe. Being a closed
/// enum rather than a free-form column-name string means there is no code
/// path that could ever compile a bulk UPDATE against a column outside this
/// list, in particular never against `tickets.status`.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum BulkTicketField {
    Section,
    RowLabel,
    Seat,
    ListingPriceCents,
}

/// Input for `bulk_update_tickets`: set one field to one value across many
/// tickets in a single all-or-nothing transaction. `text_value` is used for
/// Section/RowLabel/Seat; `cents_value` is used for ListingPriceCents.
/// Leaving the relevant one `None` clears that field.
///
/// 1.9.1: TicketType was removed from this set - marko found it confusing to
/// have "ticket type" changeable both here AND per-ticket, and asked for it
/// to be a single one-time choice made when creating the order instead (see
/// `OrderInput.ticket_type`, unchanged - it already copies onto every
/// generated ticket). It's still editable per-ticket afterwards via the
/// single-ticket `TicketUpdateInput`/`TicketEditModal` if one ticket needs
/// correcting - only the bulk path was removed.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BulkTicketUpdateInput {
    pub ticket_ids: Vec<i64>,
    pub field: BulkTicketField,
    pub text_value: Option<String>,
    pub cents_value: Option<i64>,
}

/// Input for `bulk_update_ticket_status` (1.9.3): set many tickets'
/// `status` in one all-or-nothing transaction. Deliberately a plain
/// `String` rather than a closed enum here (unlike `BulkTicketField` above) -
/// the safety guarantee for this endpoint isn't "which column can be
/// written", it's "which values `status` may take", and that's enforced by
/// `bulk_update_ticket_status_impl`'s own validation (available/listed/
/// cancelled only - never `sold`, see that function's doc comment in
/// tickets.rs for the full reasoning).
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BulkTicketStatusInput {
    pub ticket_ids: Vec<i64>,
    pub status: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Sale {
    pub id: i64,
    pub code: String,
    pub ticket_id: i64,
    pub ticket_code: String,
    pub section: Option<String>,
    pub row_label: Option<String>,
    pub seat: Option<String>,
    pub event_id: i64,
    pub event_name: String,
    /// 1.8.0: the ticket's own order, so Sale Detail can link straight to
    /// Order Detail without a second round trip. Every ticket belongs to
    /// exactly one order (tickets.order_id is NOT NULL - see migration 001),
    /// so this is never optional, unlike event_id/currency on `SaleGroup`
    /// which CAN be "Mixed" once several lines are aggregated together.
    pub order_id: i64,
    pub order_code: String,
    pub platform_id: Option<i64>,
    pub platform_name: Option<String>,
    pub sale_date: String,
    pub sale_price_cents: i64,
    pub selling_fees_cents: i64,
    pub currency: String,
    pub payment_status: String,
    pub buyer_reference: Option<String>,
    pub notes: Option<String>,
    pub is_demo: bool,
    pub created_at: String,
    pub updated_at: String,
    pub cost_cents: i64,
    pub profit_cents: i64,
    pub margin: Option<f64>,
    pub roi: Option<f64>,
    /// Set together, only by the dedicated refund action - never by a plain edit.
    pub refunded_at: Option<String>,
    pub refund_reason: Option<String>,
    /// NULL for an ordinary single-ticket sale. Shared by every row that was
    /// submitted together in one multi-ticket "New sale" action - see
    /// migration 003. Used only to group rows in the UI; never changes what
    /// a single `sales` row means (still exactly one ticket).
    pub batch_id: Option<String>,
}

/// One row in the Sales screen's main (grouped) list: everything submitted
/// together in one sale action - a single ticket, or a multi-ticket batch -
/// collapsed into one summary row. `id`/`code` are the group's representative
/// (lowest) sale id/code, used to open Sale Detail, which then loads the
/// individual `Sale` rows on demand (mirrors Order/Order Detail).
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SaleGroup {
    pub id: i64,
    pub code: String,
    pub batch_id: Option<String>,
    pub ticket_count: i64,
    /// Some only when every ticket in this group shares one event; None
    /// means mixed (a batch can span events - ticket selection isn't
    /// restricted to one event) and the UI should show "Mixed events".
    pub event_id: Option<i64>,
    pub event_name: Option<String>,
    pub sale_date: String,
    pub platform_id: Option<i64>,
    pub platform_name: Option<String>,
    /// Some only when every line shares one currency - same mixed-safety
    /// convention as `FinanceSummary.currency`.
    pub currency: Option<String>,
    /// Revenue/fees/cost/profit below are summed EXCLUDING refunded lines
    /// (they are never "realized"), matching the site-wide convention.
    pub revenue_cents: i64,
    pub selling_fees_cents: i64,
    pub cost_cents: i64,
    pub profit_cents: i64,
    pub margin: Option<f64>,
    pub roi: Option<f64>,
    /// Some(status) only when every line shares one payment status; None
    /// means mixed (e.g. one ticket in the batch was refunded later while
    /// the rest are still paid) - the UI should show "Mixed" rather than a
    /// single, misleading badge.
    pub payment_status: Option<String>,
    pub refunded_count: i64,
    pub is_demo: bool,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SaleInput {
    pub ticket_id: i64,
    pub platform_id: Option<i64>,
    pub sale_date: String,
    pub sale_price_cents: i64,
    pub selling_fees_cents: i64,
    pub payment_status: Option<String>,
    pub buyer_reference: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SaleBatchLineInput {
    pub ticket_id: i64,
    pub sale_price_cents: i64,
    pub selling_fees_cents: i64,
}

/// Records one sale "transaction" that can cover several tickets at once
/// (e.g. selling a block of 4 seats to the same buyer). Every ticket still
/// gets its own `sales` row internally - one row per ticket is what keeps
/// revenue/cost/profit/margin/ROI exact per seat - but all lines here share
/// buyer/platform/date/payment details and are inserted atomically, so a
/// single "New sale" action in the UI can hand over many tickets in one go.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SaleBatchInput {
    pub lines: Vec<SaleBatchLineInput>,
    pub platform_id: Option<i64>,
    pub sale_date: String,
    pub payment_status: Option<String>,
    pub buyer_reference: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SaleEditInput {
    pub platform_id: Option<i64>,
    pub sale_date: String,
    pub sale_price_cents: i64,
    pub selling_fees_cents: i64,
    pub payment_status: String,
    pub buyer_reference: Option<String>,
    pub notes: Option<String>,
}

/// Input for `bulk_update_sale_payment_status` (1.9.2): set many sales'
/// `payment_status` in one all-or-nothing transaction. Deliberately narrower
/// than `BulkTicketUpdateInput` in two ways: it only ever touches
/// `sales.payment_status` (never `tickets.status`), and `payment_status` here
/// is restricted by the impl to "pending"/"paid" only - refunding is a
/// separate, dedicated, one-way action (`refund_sale_impl`) and stays that
/// way; a refunded sale can never be reached through this path, in either
/// direction (can't be set TO refunded, and if it's already refunded the
/// whole batch is rejected - see sales.rs tests).
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BulkSalePaymentStatusInput {
    pub sale_ids: Vec<i64>,
    pub payment_status: String,
}

/// Dashboard "Inventory & Potential Profit" block - deliberately separate
/// from `FinanceSummary` (realized numbers). Never mixed into `inventory`
/// or `period` above, and never labelled "profit" alone - only "Potential
/// Profit" - so it can never be mistaken for realized profit.
///
/// Scope: tickets currently `available` or `listed` (i.e. NOT YET sold, and
/// not cancelled) - this is "current inventory" in the same sense as the
/// existing `inventory` FinanceSummary block, so it is intentionally NOT
/// affected by the Dashboard's period filter (see dashboard.rs).
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InventoryPotential {
    /// Purchase cost (+fees +other costs) of every ticket still available or
    /// listed - money already spent on stock that hasn't sold yet.
    pub inventory_cost_cents: i64,
    /// Sum of `listing_price_cents` for available/listed tickets that HAVE a
    /// listing price set (tickets with no listing price contribute 0, they
    /// are counted separately in `DashboardAlerts.missing_listing_price_count`).
    pub listing_value_cents: i64,
    /// `listing_value_cents - inventory_cost_cents`. Only meaningful for the
    /// tickets already counted in `listing_value_cents` - unpriced inventory
    /// still counts against `inventory_cost_cents` but contributes no
    /// estimated revenue here, so this number gets more accurate as more
    /// inventory is priced (see Attention: Missing Listing Price).
    pub potential_profit_cents: i64,
    /// Some(code) only when every available/listed ticket shares one
    /// currency; None means mixed - same "never blend, show Mixed" contract
    /// as `FinanceSummary.currency` / `SaleGroup.currency` (BUG #6). Reuses
    /// this dashboard's own primary_currency/mixed_currencies rather than a
    /// second, separate currency-mix check.
    pub currency: Option<String>,
}

/// One row in the Dashboard's "Upcoming Events" alert - deliberately a small,
/// focused DTO (not the full `EventWithStats`/`FinanceSummary`) since the
/// alert only ever needs enough to show and link to the event.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpcomingEventAlert {
    pub id: i64,
    pub name: String,
    pub event_date: String,
    /// Tickets still available or listed for this event - inventory that is
    /// still relevant to sell before the event happens.
    pub relevant_inventory: i64,
}

/// Dashboard "Attention" section. Deliberately simple, transparent counts -
/// no scoring, no new alert/notification engine, no persisted state. Always
/// computed from current data, never affected by the Dashboard's period
/// filter (these are "right now" facts, not activity within a date range).
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DashboardAlerts {
    /// Orders whose payment_status is 'unpaid' or 'partial' - i.e. money
    /// still owed to a supplier. (This is `orders.payment_status`, the
    /// purchase side - a different field from `sales.payment_status`.)
    pub unpaid_orders_count: i64,
    /// Tickets currently `available` or `listed` with no `listing_price_cents`
    /// set - inventory that cannot yet generate a sale because it has no price.
    pub missing_listing_price_count: i64,
    /// Total count of `status='upcoming'` events, with an `event_date`
    /// within the next `UPCOMING_EVENT_WINDOW_DAYS` (see dashboard.rs), that
    /// still have available/listed ticket inventory.
    pub upcoming_events_count: i64,
    /// The soonest of the above, up to `UPCOMING_EVENTS_CAP` - same
    /// "capped list + separate total count" convention already used for
    /// Recent Events/Orders/Sales elsewhere on this dashboard.
    pub upcoming_events: Vec<UpcomingEventAlert>,
    /// 1.8.3 (section 13, Payments visibility): sales whose payment_status
    /// is 'pending' - money not yet collected from the buyer. The sales-side
    /// mirror of `unpaid_orders_count` (money not yet paid to a supplier).
    /// Deliberately NOT period-filtered, same "right now" rule as every
    /// other field on this struct - and deliberately just a count/amount,
    /// not a new payments module (no transactions/reconciliation/invoices).
    pub pending_sales_count: i64,
    /// SUM(sale_price_cents) of the sales counted in `pending_sales_count`,
    /// scoped to `primary_currency` like every other money total on this
    /// dashboard. Legitimately 0 (not "missing") when the count is 0.
    pub pending_sales_amount_cents: i64,
    /// Some(code) unless the database has more than one currency
    /// (`mixed_currencies`) - same "null = mixed, never blend" convention as
    /// `InventoryPotential.currency`. Reuses the dashboard's own
    /// primary_currency/mixed_currencies signal (computed from tickets,
    /// whose currency `sales.currency` always copies at creation time)
    /// rather than a second, narrower check scoped to just pending sales.
    pub pending_sales_currency: Option<String>,
}

/// Dashboard "Cashflow" section (1.9.0). A small, transparent snapshot of
/// what was sold vs. what has actually been collected from buyers vs. what
/// they still owe - built entirely from the existing `sales.payment_status`
/// field (pending/paid/refunded), no new payment/transaction table. Not
/// period-filtered - like `DashboardAlerts`, this is a "right now" fact, all
/// time, so it never disagrees with `alerts.pending_sales_amount_cents`
/// (which is exactly `outstanding_cents` below, just also surfaced there).
///
/// The four numbers are related by one invariant that always holds (for the
/// same currency scope): `revenue_cents == paid_cents + outstanding_cents`,
/// since every non-refunded sale is either 'paid' or 'pending' - never both,
/// never neither. Refunded sales are excluded from all four fields, same as
/// every other realized-money figure in this app.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CashflowSummary {
    /// All-time realized revenue - identical figure to `DashboardData.
    /// inventory.revenue_cents`, repeated here so this section is
    /// self-contained and doesn't require cross-referencing another block.
    pub revenue_cents: i64,
    /// All-time realized profit - identical figure to `DashboardData.
    /// inventory.profit_cents`.
    pub profit_cents: i64,
    /// SUM(sale_price_cents) of sales with payment_status = 'paid' - money
    /// actually collected from buyers.
    pub paid_cents: i64,
    /// SUM(sale_price_cents) of sales with payment_status = 'pending' -
    /// money sold but not yet collected. Same query/value as
    /// `DashboardAlerts.pending_sales_amount_cents`.
    pub outstanding_cents: i64,
    /// Some(code) unless the database has more than one currency
    /// (`mixed_currencies`) - same "null = mixed, never blend" convention as
    /// every other money block on this dashboard.
    pub currency: Option<String>,
}

/// One bucket of the Dashboard revenue/profit-over-time chart (1.6.0). Same
/// scope and realized-only/refund-excluded rule as `DashboardData.period`
/// (see dashboard.rs) - just broken out by date instead of collapsed into
/// one total, so the chart and the StatCards above it can never disagree.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RevenueTimeSeriesPoint {
    /// The earliest real sale_date that fell into this bucket - always a
    /// concrete calendar date (never one of period_bounds()'s sentinel
    /// dates), used as this point's display date/label.
    pub bucket_start: String,
    pub revenue_cents: i64,
    pub selling_fees_cents: i64,
    pub cogs_cents: i64,
    pub profit_cents: i64,
    /// COUNT(*) of (non-refunded) sales lines in this bucket - i.e. tickets
    /// sold, same definition as `FinanceSummary.sold_tickets` (1.7.5, added
    /// for the Dashboard chart's "Sales" metric). Independent of the money
    /// fields above - a bucket's revenue doesn't tell you how many tickets
    /// made it up (one expensive ticket vs. several cheap ones), so this is
    /// its own real COUNT(*), not derived from revenue_cents.
    pub sold_tickets: i64,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DashboardData {
    pub inventory: FinanceSummary,
    pub period: FinanceSummary,
    pub period_from: String,
    pub period_to: String,
    pub recent_orders: Vec<Order>,
    pub recent_sales: Vec<Sale>,
    pub recent_events: Vec<EventWithStats>,
    /// The currency all dashboard totals below are computed in. Always a
    /// concrete code (defaults to EUR) - never a blended sum of currencies.
    pub primary_currency: String,
    /// True when the database also contains data in other currencies, which
    /// is therefore excluded from the totals above. The UI should warn.
    pub mixed_currencies: bool,
    /// Inventory Cost / Listing Value / Potential Profit - see
    /// `InventoryPotential` doc comment. Kept and labelled separately from
    /// `inventory`/`period` (realized) by design.
    pub inventory_potential: InventoryPotential,
    /// Attention/alerts - see `DashboardAlerts` doc comment.
    pub alerts: DashboardAlerts,
    /// Cashflow snapshot (1.9.0) - see `CashflowSummary` doc comment.
    pub cashflow: CashflowSummary,
    /// Revenue/profit chart data - see `RevenueTimeSeriesPoint` doc comment.
    pub revenue_time_series: Vec<RevenueTimeSeriesPoint>,
    /// "day" | "week" | "month" - the bucket width `revenue_time_series`
    /// used, chosen from the period's span (see dashboard.rs::
    /// time_series_granularity), so the frontend can label ticks
    /// appropriately without re-deriving the same span logic itself.
    pub time_series_granularity: String,
}

// ---------------------------------------------------------------------------
// Pull (1.9.7) - buying tickets on someone else's behalf for a fee. See
// migrations/005_pulls.sql for the full rationale (why this is standalone,
// not linked to events/orders/tickets/sales/finance.rs). `commands/pulls.rs`
// has the impl+wrapper functions.
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Pull {
    pub id: i64,
    pub code: String,
    pub buyer_name: String,
    pub event_name: String,
    pub event_date: Option<String>,
    pub quantity: i64,
    pub platform_id: Option<i64>,
    pub platform_name: Option<String>,
    pub seats: Option<String>,
    pub more_info: Option<String>,
    /// marko's own fee for doing the pull - never the ticket price (paid by
    /// the other person's card, not marko's money).
    pub price_cents: i64,
    pub currency: String,
    pub transfer_deadline: Option<String>,
    pub transfer_done: bool,
    pub transfer_done_at: Option<String>,
    pub is_demo: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Input for `create_pull`. `transfer_done`/`transfer_done_at` deliberately
/// aren't here - a brand new pull always starts not-transferred; use
/// `set_pull_transfer_done` (or edit it afterwards) once it's actually done.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PullInput {
    pub buyer_name: String,
    pub event_name: String,
    pub event_date: Option<String>,
    pub quantity: i64,
    pub platform_id: Option<i64>,
    pub seats: Option<String>,
    pub more_info: Option<String>,
    pub price_cents: i64,
    pub currency: String,
    pub transfer_deadline: Option<String>,
}

/// Input for `update_pull` - the full edit form. Unlike `PullInput`, this
/// DOES include `transfer_done` (so a mistaken checkbox click, or backfilling
/// older data, can be corrected here too) - see `update_pull_impl`'s doc
/// comment for how `transfer_done_at` is kept consistent with it either way.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PullEditInput {
    pub buyer_name: String,
    pub event_name: String,
    pub event_date: Option<String>,
    pub quantity: i64,
    pub platform_id: Option<i64>,
    pub seats: Option<String>,
    pub more_info: Option<String>,
    pub price_cents: i64,
    pub currency: String,
    pub transfer_deadline: Option<String>,
    pub transfer_done: bool,
}

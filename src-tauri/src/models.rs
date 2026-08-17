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

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Sale {
    pub id: i64,
    pub code: String,
    pub ticket_id: i64,
    pub ticket_code: String,
    pub event_id: i64,
    pub event_name: String,
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
}

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

/// 2.0.28: shared result shape for every `bulk_delete_*` command (pulls,
/// pulls received, orders, events, sale groups - see each command's own doc
/// comment for why deletion specifically does NOT follow this codebase's
/// usual "validate every id first, then write all, any single failure means
/// nothing happens" bulk-write convention (`bulk_update_tickets_impl` et al).
/// Deletion safety is a genuine per-row business rule (sold tickets, sale
/// history, an event's linked orders), not a referential-integrity
/// precondition on the whole batch - so each selected id is judged on its
/// own merits: everything safe to delete is removed together in ONE
/// transaction (still fully atomic - a crash mid-way can never leave a
/// partial delete on disk), and anything that isn't is reported back with a
/// plain-English reason instead of silently vanishing from the selection or
/// blocking the rows that WERE safe.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BulkDeleteResult {
    pub deleted_ids: Vec<i64>,
    pub skipped: Vec<BulkDeleteSkip>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BulkDeleteSkip {
    pub id: i64,
    pub reason: String,
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
    /// 2.0.27: legacy free-text mirror of `category_id`'s name - see
    /// migrations/012_event_categories.sql's doc comment for why this stays
    /// around and stays written (never DROP a shipped column; csv_export.rs's
    /// Events export still reads this directly).
    pub category: Option<String>,
    /// 2.0.27: the real lookup FK - see `EventCategory`. `None` means no
    /// category, same as `category` being `None` did before this version.
    pub category_id: Option<i64>,
    /// 2.0.27: the resolved category's `color_slot`, joined in alongside
    /// `category_id` (see commands::events::STATS_SQL/PLAIN_SELECT_SQL) so
    /// the Events list can render EventCategoryBadge.tsx without a second
    /// round trip - same convention as Order.category_color_slot/
    /// SaleGroup.category_color_slot. `category` above already mirrors the
    /// category's NAME, so only color_slot is needed here (unlike Order/
    /// SaleGroup, which have no such text mirror and so need category_name
    /// too).
    pub category_color_slot: Option<i64>,
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
    /// 2.0.27: no longer settable directly - `commands::events` derives this
    /// from `category_id` (looking up the category's own name) so the two
    /// never drift apart. Kept on the struct (rather than removed) only
    /// because it's `#[derive(Deserialize)]` and dropping it would be a
    /// breaking wire-format change for no benefit; any value sent here is
    /// ignored. See `EventCategory`. `#[allow(dead_code)]` because nothing
    /// ever reads it back out on purpose - see above.
    #[allow(dead_code)]
    pub category: Option<String>,
    pub category_id: Option<i64>,
    pub status: Option<String>,
    pub notes: Option<String>,
}

/// 2.0.27: a managed event category (Settings -> Lookups, "like Platforms" -
/// marko's own words) - replaces the old hardcoded CATEGORY_OPTIONS array in
/// Events.tsx. See migrations/012_event_categories.sql for the full
/// rationale, in particular why `color_slot` is a plain integer (an index
/// into a fixed palette the frontend owns) rather than a hex string.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EventCategory {
    pub id: i64,
    pub name: String,
    /// Index into the frontend's fixed categorical palette (EventCategoryBadge.tsx),
    /// assigned once at creation and never recomputed - see the migration's
    /// doc comment for why. Not bounded here; the frontend wraps it (`% palette.length`)
    /// so it never fails to render even past the palette's own length.
    pub color_slot: i64,
    pub is_demo: bool,
    pub created_at: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EventWithStats {
    #[serde(flatten)]
    pub event: Event,
    pub stats: FinanceSummary,
}

/// Result of one "Detect categories" run (commands::events::
/// detect_event_categories, 2.0.63) - the retroactive, one-click sibling of
/// the automatic detection `commands::orders_sheet_sync::resolve_or_create_
/// event` already runs on every brand-new event a sheet sync creates. Only
/// ever touches events that currently have `category_id IS NULL`, so
/// running this again is always safe - see ai_categorize.rs's module doc
/// comment for the full free-rules-then-AI design this reports on.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CategoryDetectionResult {
    /// How many uncategorized events this run looked at.
    pub checked: i64,
    /// Of those, how many got a category from a free keyword rule (no AI
    /// involved).
    pub categorized_by_rule: i64,
    /// Of those, how many got a category only because the AI fallback
    /// actually recognized the team/artist/performer - always 0 when
    /// `ai_configured` is false.
    pub categorized_by_ai: i64,
    /// Left exactly as they were (still uncategorized) - either nothing
    /// recognized them, or `ai_configured` is false so only the free rules
    /// ever ran.
    pub left_uncategorized: i64,
    /// Whether this build has an Anthropic key embedded at all (see
    /// ai_categorize::embedded_anthropic_api_key) - the frontend uses this
    /// to explain a non-zero `left_uncategorized` accurately, rather than
    /// implying the AI tried and failed when it was never consulted.
    pub ai_configured: bool,
}

/// 2.0.38: one ticket's seat location, as shown on the Orders/Tickets/Sales
/// list screens' new "Seats" column (`Order.seats`/`SaleGroup.seats` below).
/// Deliberately the same three nullable fields `Sale`/`Ticket` already carry
/// inline (section/row_label/seat) - kept as a real struct here instead,
/// since this is the first place the app needs a whole LIST of them per row.
/// Formatting this into a compact display string (grouping same section/row,
/// collapsing contiguous seat numbers into a range) happens entirely on the
/// frontend (`formatSeatsSummary` in format.ts) - this struct only carries
/// the raw per-ticket data, unmodified, same "aggregation in SQL, formatting
/// in the UI" split every other field on these two screens already follows.
#[derive(Debug, Serialize, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct SeatEntry {
    pub section: Option<String>,
    pub row_label: Option<String>,
    pub seat: Option<String>,
}

impl SeatEntry {
    /// Parses the `GROUP_CONCAT(... , char(30))` aggregate produced by
    /// orders.rs's `BASE_SQL` / sales.rs's `GROUP_BASE_SELECT`. Each ticket is
    /// encoded as `section\x1Frow_label\x1Fseat` (0x1F, the ASCII "unit
    /// separator" - never typed by a real user), records joined by 0x1E (the
    /// "record separator"). Deliberately NOT done with SQL's own
    /// `GROUP_CONCAT(DISTINCT ...)`: SQLite rejects a custom separator
    /// combined with DISTINCT ("DISTINCT aggregates must have exactly one
    /// argument", confirmed empirically before writing this), and a plain
    /// comma-joined DISTINCT would be ambiguous against real section/seat
    /// values that might themselves contain a comma. Deduplicating here in
    /// Rust instead sidesteps both problems.
    ///
    /// `None` (the column is NULL - no joined ticket rows at all) and `Some("")`
    /// both yield an empty Vec. A record whose three fields are all empty
    /// (a ticket with no section/row/seat at all) still yields one
    /// `SeatEntry { None, None, None }` - correctly distinct from "no
    /// tickets" - the frontend renders that as "General admission", same
    /// convention `formatSeatLocation` already uses for an individual ticket.
    pub fn parse_aggregate(raw: Option<&str>) -> Vec<SeatEntry> {
        const FIELD_SEP: char = '\u{1f}';
        const RECORD_SEP: char = '\u{1e}';
        // Safety cap, same reasoning as this app's other LIST_CAP constants -
        // protects response size against a pathological (thousands-of-tickets)
        // order/sale group; no realistic one gets remotely close.
        const MAX_ENTRIES: usize = 500;

        let Some(raw) = raw else { return Vec::new() };
        if raw.is_empty() {
            return Vec::new();
        }
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for record in raw.split(RECORD_SEP) {
            let mut parts = record.split(FIELD_SEP);
            let non_empty = |s: &str| if s.is_empty() { None } else { Some(s.to_string()) };
            let entry = SeatEntry {
                section: non_empty(parts.next().unwrap_or("")),
                row_label: non_empty(parts.next().unwrap_or("")),
                seat: non_empty(parts.next().unwrap_or("")),
            };
            if seen.insert(entry.clone()) {
                out.push(entry);
                if out.len() >= MAX_ENTRIES {
                    break;
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod seat_entry_tests {
    use super::SeatEntry;

    #[test]
    fn none_input_is_no_seats_at_all() {
        assert_eq!(SeatEntry::parse_aggregate(None), Vec::new());
    }

    #[test]
    fn empty_string_is_also_no_seats() {
        assert_eq!(SeatEntry::parse_aggregate(Some("")), Vec::new());
    }

    #[test]
    fn one_fully_blank_record_is_one_general_admission_entry_not_zero() {
        let raw = "\u{1f}\u{1f}";
        assert_eq!(
            SeatEntry::parse_aggregate(Some(raw)),
            vec![SeatEntry { section: None, row_label: None, seat: None }]
        );
    }

    #[test]
    fn parses_a_normal_multi_ticket_order() {
        let raw = "204\u{1f}AA\u{1f}128\u{1e}204\u{1f}AA\u{1f}129\u{1e}204\u{1f}AA\u{1f}130";
        assert_eq!(
            SeatEntry::parse_aggregate(Some(raw)),
            vec![
                SeatEntry { section: Some("204".into()), row_label: Some("AA".into()), seat: Some("128".into()) },
                SeatEntry { section: Some("204".into()), row_label: Some("AA".into()), seat: Some("129".into()) },
                SeatEntry { section: Some("204".into()), row_label: Some("AA".into()), seat: Some("130".into()) },
            ]
        );
    }

    #[test]
    fn duplicate_records_collapse_to_one_preserving_first_seen_order() {
        let raw = "204\u{1f}AA\u{1f}128\u{1e}210\u{1f}BB\u{1f}5\u{1e}204\u{1f}AA\u{1f}128";
        assert_eq!(
            SeatEntry::parse_aggregate(Some(raw)),
            vec![
                SeatEntry { section: Some("204".into()), row_label: Some("AA".into()), seat: Some("128".into()) },
                SeatEntry { section: Some("210".into()), row_label: Some("BB".into()), seat: Some("5".into()) },
            ]
        );
    }

    #[test]
    fn general_admission_ticket_mixed_in_with_seated_ones_keeps_both_kinds() {
        let raw = "\u{1f}\u{1f}\u{1e}204\u{1f}AA\u{1f}128";
        assert_eq!(
            SeatEntry::parse_aggregate(Some(raw)),
            vec![
                SeatEntry { section: None, row_label: None, seat: None },
                SeatEntry { section: Some("204".into()), row_label: Some("AA".into()), seat: Some("128".into()) },
            ]
        );
    }

    #[test]
    fn caps_at_500_unique_entries_instead_of_growing_unbounded() {
        let raw = (0..600)
            .map(|i| format!("204\u{1f}AA\u{1f}{i}"))
            .collect::<Vec<_>>()
            .join("\u{1e}");
        assert_eq!(SeatEntry::parse_aggregate(Some(&raw)).len(), 500);
    }
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    pub id: i64,
    pub code: String,
    pub event_id: i64,
    pub event_name: String,
    /// 2.0.27: the order's event's category, resolved here (same convention
    /// as `event_name`/`platform_name`) so the Orders list can filter/badge
    /// without a second round trip. `None` on both when the event has no
    /// category set.
    pub category_id: Option<i64>,
    pub category_name: Option<String>,
    pub category_color_slot: Option<i64>,
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
    /// 2.0.66: how many of this order's SOLD tickets (out of `sold_count`)
    /// have `Ticket.delivery_status = 'Delivered'` - see the new "Completed"
    /// indicator (Orders/Sales/Pulls, REDESIGN-2.0.66-REPORT.md). Scoped to
    /// sold tickets only, same spirit as `sold_count` itself excluding
    /// available/listed stock - an available ticket has nothing to deliver
    /// yet, so it shouldn't count against this.
    pub delivered_count: i64,
    /// 2.0.66: how many of this order's SOLD tickets (out of `sold_count`)
    /// have a CURRENT (non-refunded) sale whose `payment_status = 'paid'` -
    /// see `delivered_count` above for why this is scoped to sold tickets
    /// only. A refunded ticket reverts to `status='available'` (see
    /// `refund_sale_impl`), so it's already excluded from `sold_count` and
    /// therefore from this count too - it isn't double-penalized here.
    pub paid_count: i64,
    /// 2.0.38: every ticket in this order's own seat location (deduplicated,
    /// order not significant - the frontend sorts/groups for display). NOT
    /// filtered by ticket status - matches `sold_count`/`available_count`/etc.
    /// above, which are the order's true complete counts; a cancelled
    /// ticket's seat is still part of "what this order covers."
    pub seats: Vec<SeatEntry>,
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
    /// 2.2.7: seating/pricing tier/level - see `Ticket::tier`'s own doc
    /// comment (models.rs) for why this is a separate field from
    /// `ticket_type` above. Set once here, copied onto every ticket this
    /// order generates - same "set once at creation, editable per-ticket
    /// afterwards via TicketUpdateInput" convention already established by
    /// `ticket_type`/`section`/`row_label` above. There is no separate "Add
    /// Ticket" flow in this app; tickets only ever come from an order.
    pub tier: Option<String>,
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

/// Result of converting one EXISTING order's currency to EUR (2.0.51) - see
/// `commands::orders::convert_order_currency_impl` for the actual atomic
/// ticket+sale+order conversion this reports on. Unlike 2.0.50's
/// `CurrencyConversion` (a stateless "here's what these numbers would become"
/// preview for the New Order form, nothing saved yet), this always describes
/// a conversion that has ALREADY been committed to the database by the time
/// it's returned - `ticketsConverted`/`salesConverted` are real counts of
/// rows actually rewritten, not an estimate.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OrderCurrencyConversion {
    pub order_id: i64,
    pub order_code: String,
    pub from_currency: String,
    pub to_currency: String,
    pub rate: f64,
    pub rate_date: String,
    pub tickets_converted: i64,
    pub sales_converted: i64,
    /// 2.0.53: was this order ever linked to a Google Sheet at all (via
    /// Order sync or Order push)? `false` for the common case - manually
    /// entered or CSV-imported orders, or a sheet connection that was never
    /// set up - and `sheet_push_error` is then always `None` too, since
    /// there was never anywhere to push to. `true` means this conversion
    /// also attempted to update that sheet row's Currency/Price Per Ticket/
    /// Total Purchase Price cells - see `sheet_push_error` for whether that
    /// attempt actually succeeded.
    pub linked_to_sheet: bool,
    /// `None` when `linked_to_sheet` is `false` (nothing was attempted), or
    /// when it's `true` and the push succeeded. `Some(message)` means the
    /// local conversion above still fully happened and is already saved -
    /// only the follow-up push to the actual Google Sheet failed (network,
    /// permissions, the row having since moved or been deleted, etc.) and
    /// the sheet itself is now out of sync with the app until this is
    /// retried or fixed by hand.
    pub sheet_push_error: Option<String>,
}

/// What the single-order `convert_order_currency` command returns - the
/// summary above, plus the order's own freshly-refetched record, so Order
/// Detail can update its Currency/cost fields immediately without a second
/// round trip.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OrderCurrencyConversionResult {
    pub order: Order,
    pub conversion: OrderCurrencyConversion,
}

/// Result of the bulk "Convert to EUR" action on the Dashboard's
/// mixed-currency banner (2.0.51) - every order actually converted, plus any
/// that were skipped and why. Reuses `BulkDeleteSkip`'s existing `{id,
/// reason}` shape (same convention as every other bulk action in this app)
/// rather than a new, near-identical type. Conversion judges each order on
/// its own merits and commits everything safe in its own transaction, same
/// per-item philosophy as `bulk_delete_orders_impl` - see `BulkDeleteResult`'s
/// doc comment above for the full reasoning. A skip here should be rare (see
/// `convert_order_currency_impl`'s own currency-consistency guard and the
/// per-currency rate-fetch failure path), but the shape stays honest about
/// partial success rather than silently stopping at the first problem.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BulkCurrencyConversionResult {
    pub converted: Vec<OrderCurrencyConversion>,
    pub skipped: Vec<BulkDeleteSkip>,
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
    /// 2.2.7: seating/pricing TIER or LEVEL (e.g. "VIP", "Lower Bowl",
    /// "Level 200", "Category 1") - free text, deliberately a SEPARATE field
    /// from `ticket_type` below, which is a DELIVERY method (E-ticket/PDF/
    /// Mobile transfer/Physical/Will call) and has nothing to do with
    /// seating/pricing category. See migration 024's own doc comment for the
    /// full reasoning and the two prior places this exact mix-up was already
    /// flagged before this column existed.
    pub tier: Option<String>,
    pub seat: Option<String>,
    pub ticket_type: Option<String>,
    pub purchase_cost_cents: i64,
    pub purchase_fees_cents: i64,
    pub other_costs_cents: i64,
    pub total_cost_cents: i64,
    pub listing_price_cents: Option<i64>,
    pub currency: String,
    pub status: String,
    /// 2.0.10: marko's own free-text "Status"/"Delivery status" tracking,
    /// distinct from `status` above - see migration 010's doc comment for
    /// why these aren't folded into that field or into a CHECK enum.
    pub resale_status: Option<String>,
    pub delivery_status: Option<String>,
    pub notes: Option<String>,
    pub is_demo: bool,
    pub created_at: String,
    pub updated_at: String,
    pub sale_price_cents: Option<i64>,
    /// 2.0.68: the ACTIVE (non-refunded) sale's payment_status, via the same
    /// `LEFT JOIN sales sa ON sa.ticket_id = t.id AND sa.payment_status !=
    /// 'refunded'` this struct's `sale_price_cents` already reuses (see
    /// BASE_SQL's own doc comment in commands/tickets.rs) - no new JOIN, and
    /// the same "at most one active sale per ticket" guarantee applies. None
    /// for a never-sold ticket, or one whose only sale was refunded - same
    /// cases where `sale_price_cents` is already None. Lets Order Detail show
    /// a per-ticket "Payout status" column without needing `Sale[]` at all.
    pub sale_payment_status: Option<String>,
    /// 2.0.69: the same active sale's own `id` - same join, same None cases
    /// as `sale_payment_status` right above. Lets Order Detail's inline
    /// Payout-status edit call the existing `bulk_update_sale_payment_status`
    /// (sale-id-based, unchanged since before this feature) directly, with
    /// no new "set payment status by ticket id" endpoint needed.
    pub sale_id: Option<i64>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TicketUpdateInput {
    pub section: Option<String>,
    pub row_label: Option<String>,
    /// 2.2.7 - see `Ticket::tier`'s own doc comment.
    pub tier: Option<String>,
    pub seat: Option<String>,
    pub ticket_type: Option<String>,
    pub listing_price_cents: Option<i64>,
    pub status: Option<String>,
    /// 2.0.10 - see `Ticket::resale_status`/`Ticket::delivery_status`.
    pub resale_status: Option<String>,
    pub delivery_status: Option<String>,
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

/// 2.0.69: input for the new direct, single-ticket-friendly command exposing
/// `bulk_update_ticket_delivery_status_impl` (2.0.67) - that function has
/// existed since 2.0.67 but was, until now, only ever reached indirectly via
/// the Orders/Sales LIST pages' own order/sale-group-scoped bulk actions
/// (`BulkOrdersDeliveryStatusInput`/`BulkSaleGroupsDeliveryStatusInput`
/// above), neither of which fits "change just THIS one row's delivery
/// status inline" - those resolve a whole order/sale-group down to tickets,
/// not a single already-known ticket id. `ticket_ids` still takes a Vec (not
/// a single id) so a future bulk selection elsewhere in the app - not just
/// Sale/Order Detail's inline edit - could reuse this same endpoint; today's
/// callers always pass exactly one.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BulkTicketDeliveryStatusInput {
    pub ticket_ids: Vec<i64>,
    pub delivery_status: String,
}

/// 2.0.69: same shape and reasoning as `BulkTicketDeliveryStatusInput` above,
/// for the brand-new `bulk_update_ticket_resale_status_impl` - marko's
/// report wanted Status (his own manual Listed/Unlisted/Sold) editable
/// inline on Sale Detail's table, and no endpoint at any raw-ticket-id level
/// existed yet for `resale_status` (only the single-ticket `TicketUpdateInput`
/// full-record editor did). Validated the same closed set the single-ticket
/// editor's own `<Select>` offers - `RESALE_STATUS_OPTIONS` in Tickets.tsx -
/// see `bulk_update_ticket_resale_status_impl`'s own doc comment.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BulkTicketResaleStatusInput {
    pub ticket_ids: Vec<i64>,
    pub resale_status: String,
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
    /// 2.0.66: the ticket's OWN current status - almost always "sold" (a
    /// `Sale` row only exists for a ticket that was sold), EXCEPT after a
    /// refund, which reverts the ticket to "available" (see
    /// `refund_sale_impl`) while this historical `Sale` row stays as-is.
    /// Powers the new "Completed" indicator's per-line breakdown on Sale
    /// Detail - see REDESIGN-2.0.66-REPORT.md.
    pub ticket_status: String,
    /// 2.0.66: the ticket's own `delivery_status` - see
    /// `Ticket::delivery_status`'s doc comment for why this is free-text.
    pub ticket_delivery_status: Option<String>,
    /// 2.0.68: the ticket's own `resale_status` (marko's manual Listed/
    /// Unlisted/Sold tracking - see `Ticket::resale_status`'s doc comment).
    /// Deliberately separate from `ticket_status` above: that's the real,
    /// system-managed enum, this is marko's own free-text sheet mirror -
    /// Sale Detail shows both as distinct badges, never one replacing the
    /// other (marko's report: "ako je status tak bude status ... listed,
    /// unlisted, sold").
    pub ticket_resale_status: Option<String>,
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
    /// 2.0.57: true when this sale's OWN currency differs from its OWN
    /// ticket's purchase currency (only possible since `SaleBatchInput`
    /// gained an explicit `currency` override - before that this was always
    /// false by construction). `cost_cents`/`profit_cents` below are still
    /// populated as real numbers either way (each is meaningful on its own,
    /// in its own currency), but `margin`/`roi` are forced to `None` when
    /// this is true (see `map_sale`) and the frontend shows "Mixed" instead
    /// of `profit_cents`/`cost_cents` for this row - never blend two
    /// currencies into one number, the same rule `SaleGroup.currency`
    /// already enforces at the batch level.
    pub currency_mismatch: bool,
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
    /// 2.0.27: the shared event's category - same "Some only when every
    /// line's event agrees" rule as `event_id`/`event_name` right above
    /// (derived from the very same single-event check), since a category is
    /// itself just an attribute of that one event. All three are `None`
    /// together whenever `event_id` is `None`.
    pub category_id: Option<i64>,
    pub category_name: Option<String>,
    pub category_color_slot: Option<i64>,
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
    /// 2.0.66: how many of this group's `ticket_count` tickets currently have
    /// `Ticket.status = 'sold'`. Normally equals `ticket_count` (a `Sale` row
    /// only exists for a sold ticket) - lower only when a line was refunded
    /// (its ticket reverts to "available", see `refund_sale_impl`), which is
    /// exactly the case `refunded_count` above already flags. Feeds the new
    /// "Completed" indicator, see REDESIGN-2.0.66-REPORT.md.
    pub sold_count: i64,
    /// 2.0.66: how many of this group's `ticket_count` tickets have
    /// `Ticket.delivery_status = 'Delivered'`. Not refund-filtered, same
    /// "not filtered by refund status" convention `ticket_count`/`seats`
    /// already follow above.
    pub delivered_count: i64,
    /// 2.0.66: how many of this group's OWN sale lines have
    /// `payment_status = 'paid'` (each line's own status, not derived from
    /// `payment_status` above - that field collapses to `None` the moment
    /// lines disagree, which is too coarse for "3 of 5 paid").
    pub paid_count: i64,
    pub is_demo: bool,
    /// 2.0.38: the seat location of every ticket THIS SALE GROUP actually
    /// sold (deduplicated). Unlike `Order.seats` above, this is naturally
    /// already scoped to just this group's own lines (the same `JOIN tickets
    /// t ON t.id = s.ticket_id` every other field here is computed from) -
    /// not filtered by refund status, matching `ticket_count` right above
    /// (a refunded line is still one of the tickets this sale covers).
    pub seats: Vec<SeatEntry>,
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
    /// 2.0.57: explicit sale currency for this whole batch, chosen once in
    /// the New Sale form - with an optional "Convert to EUR" step using the
    /// same live-rate helper the New Order form already has (see fx.rs).
    /// `None` preserves the app's original behaviour of silently copying
    /// each line's own ticket's purchase currency: kept for
    /// `orders_sheet_sync::apply_sales_rows` (the Sales tab of Google Sheets
    /// sync has no currency column of its own, exactly like Pulls - see
    /// `money::format_cents_for_sheet`'s doc comment) and every pre-2.0.57
    /// test, none of which send this field at all.
    /// `Some(code)` applies that ONE currency to every line in the batch -
    /// the sale currency is a single value for the whole "New sale"
    /// transaction (one buyer pays once), the same way Quick-fill
    /// price/fees already apply to every selected ticket at once. See
    /// `create_sales_batch_impl` for how this interacts with a ticket's own
    /// (possibly different) purchase currency.
    pub currency: Option<String>,
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

/// 2.0.67: input for the new Orders-list bulk 'Mark Delivered/Not delivered'
/// action - see `commands::orders::bulk_set_orders_delivery_status_impl`'s
/// doc comment for exactly how `order_ids` resolves down to the tickets this
/// actually touches (only each order's SOLD tickets). `delivery_status` is
/// validated the same way the single-ticket editor and
/// `bulk_update_ticket_delivery_status_impl` already do - only 'Delivered'/
/// 'Not delivered' ever reach the database through this path.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BulkOrdersDeliveryStatusInput {
    pub order_ids: Vec<i64>,
    pub delivery_status: String,
}

/// 2.0.67: input for the new Orders-list bulk 'Mark Paid/Pending' action -
/// see `commands::orders::bulk_set_orders_payment_status_impl`'s doc comment
/// for exactly how `order_ids` resolves down to the sales this actually
/// touches (only each order's CURRENT, non-refunded sale per sold ticket).
/// Same "pending/paid only, never refunded" restriction as
/// `BulkSalePaymentStatusInput` above - refunding stays its own dedicated,
/// one-way action.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BulkOrdersPaymentStatusInput {
    pub order_ids: Vec<i64>,
    pub payment_status: String,
}

/// 2.0.67: input for the new Sales-list bulk 'Mark Delivered/Not delivered'
/// action - `group_ids` are the same SaleGroup anchor ids
/// `bulk_delete_sale_groups`/the Sales list's own selection already use (see
/// `commands::sales::resolve_sale_groups_ticket_ids`'s doc comment for the
/// batch_id expansion this goes through).
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BulkSaleGroupsDeliveryStatusInput {
    pub group_ids: Vec<i64>,
    pub delivery_status: String,
}

/// 2.0.67: input for the new Sales-list bulk 'Mark Paid/Pending' action -
/// same `group_ids` selection as `BulkSaleGroupsDeliveryStatusInput` above,
/// but resolved down to payable (non-refunded) sale ids instead of tickets -
/// see `commands::sales::resolve_sale_groups_payable_sale_ids`.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BulkSaleGroupsPaymentStatusInput {
    pub group_ids: Vec<i64>,
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
    /// Ticket-scoped on purpose: feeds the Overview "Potential Profit"
    /// sentence, which is genuinely about how many individual tickets are
    /// dragging that estimate down. For the Activity tab's own "Missing
    /// listing price" card, see `missing_listing_price_orders_count` below.
    pub missing_listing_price_count: i64,
    /// 2.0.48: the same unpriced available/listed tickets as
    /// `missing_listing_price_count` above, but counting each ORDER once no
    /// matter how many of its tickets are missing a price - marko reads the
    /// Activity alert in terms of "how many orders do I need to go price",
    /// not a raw ticket tally that can look scarier than the real workload
    /// (one 8-ticket order with no price is one thing to go fix, not 8).
    pub missing_listing_price_orders_count: i64,
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
    /// 2.0.79: pulls (Given - see Pull's own doc comment) not yet
    /// `transfer_done`, whose `event_date` is already past or within
    /// `dashboard::PULLS_WARNING_WINDOW_DAYS` days from now - i.e. exactly
    /// the same "not transferred + event date approaching or past" condition
    /// Pulls.tsx's own `WARNING_WINDOW_DAYS`/"Deadline" column warning
    /// already flags there (unbounded on the overdue side, same as that
    /// warning - a pull doesn't stop needing attention just because its
    /// event date has passed and it's still untransferred). Replaces
    /// `unpaid_orders_count` on the Dashboard's Activity tab specifically
    /// (marko's own request) - `unpaid_orders_count` itself is unchanged and
    /// still used by the outbound-notifications feature
    /// (commands/notifications.rs), which marko did not ask to change.
    pub pulls_needing_transfer_count: i64,
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

/// One row of the Dashboard's "Sales by platform" widget (2.0.47 - see
/// REDESIGN-2.0.47-REPORT.md / DIR-001 "signature idea" #02). Same period/
/// currency/event/platform scope and refund-exclusion rule as `period`
/// above - deliberately reuses the exact same filters, just grouped by
/// platform instead of collapsed into one total or broken out by date (see
/// `RevenueTimeSeriesPoint` right above for the date-bucketed sibling of
/// this same query shape). Mirrors Eventbrite's "Sales by Source" pattern
/// found in research: revenue AND ticket count per channel, ordered by
/// revenue so the platform actually earning the most sorts first.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PlatformSales {
    pub platform_id: Option<i64>,
    /// None only when `platform_id` is None (a sale with no platform set on
    /// it) - the frontend shows "No platform" for that row, the same label
    /// already used for orders with no platform elsewhere in this app.
    pub platform_name: Option<String>,
    pub sold_tickets: i64,
    pub revenue_cents: i64,
    pub profit_cents: i64,
}

/// One non-EUR currency currently present on at least one order, with how
/// many orders are in it (2.0.51) - powers the Dashboard mixed-currency
/// banner's "Convert to EUR" picker (a specific currency, or all at once -
/// see `commands::orders::convert_currencies_to_eur`). Deliberately
/// ORDER-scoped, unlike the ticket-scoped `mixed_currencies`/
/// `primary_currency` below: conversion itself always operates order by
/// order (see `convert_order_currency_impl`'s doc comment for why), so this
/// is "how many orders would this button actually touch", not a second,
/// redundant view of the same ticket-level mix those two fields already
/// describe.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CurrencyOrderCount {
    pub currency: String,
    pub order_count: i64,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DashboardData {
    pub inventory: FinanceSummary,
    pub period: FinanceSummary,
    /// The equal-length window immediately preceding `period_from..
    /// period_to`, used for the Dashboard KPI cards' "vs previous period"
    /// trend (2.0.47, DIR-001). None when there's no sensible previous
    /// period - "All time", or a Custom range with no explicit start - see
    /// `previous_period_bounds` in dashboard.rs. Never affects `period`
    /// itself; purely additional, read-only comparison data.
    pub previous_period: Option<FinanceSummary>,
    pub period_from: String,
    pub period_to: String,
    pub recent_orders: Vec<Order>,
    /// 2.0.54: SaleGroup (one row per sale action - a single ticket, or a
    /// multi-ticket batch), not Sale (one row per ticket) - see
    /// sales::fetch_recent_groups's own doc comment for why this changed.
    pub recent_sales: Vec<SaleGroup>,
    pub recent_events: Vec<EventWithStats>,
    /// The currency all dashboard totals below are computed in. Always a
    /// concrete code (defaults to EUR) - never a blended sum of currencies.
    pub primary_currency: String,
    /// True when the database also contains data in other currencies, which
    /// is therefore excluded from the totals above. The UI should warn.
    pub mixed_currencies: bool,
    /// 2.0.51: every non-EUR currency actually present on an order, with its
    /// order count - see `CurrencyOrderCount`. Empty whenever every order is
    /// already EUR (including whenever `mixed_currencies` above is false).
    pub non_eur_order_currencies: Vec<CurrencyOrderCount>,
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
    /// "Sales by platform" widget (2.0.47) - see `PlatformSales` doc
    /// comment. Ordered by `revenue_cents` descending.
    pub sales_by_platform: Vec<PlatformSales>,
}

// ---------------------------------------------------------------------------
// Pull (1.9.7, seat fields reshaped in 1.9.8) - buying tickets on someone
// else's behalf for a fee. See migrations/005_pulls.sql for the full
// rationale (why this is standalone, not linked to
// events/orders/tickets/sales/finance.rs) and migrations/006_pulls_seat_
// fields.sql for why `seats` (one free-text field) became `section`/
// `row_label`/`seat` (three fields, mirroring `Ticket`'s own shape).
// `commands/pulls.rs` has the impl+wrapper functions.
//
// `transfer_deadline` (on `Pull` only) is a 1.9.7 leftover: 1.9.8 replaced
// the manual deadline field with an automatic "3 days before the event"
// warning computed client-side from `event_date`, so nothing in the app
// sets this column any more (`PullInput`/`PullEditInput` below no longer
// have it) - it's kept readable here only so old data already in the column
// isn't hidden, not because anything still writes it.
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
    pub section: Option<String>,
    pub row_label: Option<String>,
    pub seat: Option<String>,
    pub more_info: Option<String>,
    /// marko's own fee for doing the pull - never the ticket price (paid by
    /// the other person's card, not marko's money).
    pub price_cents: i64,
    pub currency: String,
    /// 1.9.8: no longer settable from the UI - see this section's doc
    /// comment above. Still readable for any pre-1.9.8 pull that has one.
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
/// No `transfer_deadline` either as of 1.9.8 - see the section doc comment
/// above.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PullInput {
    pub buyer_name: String,
    pub event_name: String,
    pub event_date: Option<String>,
    pub quantity: i64,
    pub platform_id: Option<i64>,
    pub section: Option<String>,
    pub row_label: Option<String>,
    pub seat: Option<String>,
    pub more_info: Option<String>,
    pub price_cents: i64,
    pub currency: String,
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
    pub section: Option<String>,
    pub row_label: Option<String>,
    pub seat: Option<String>,
    pub more_info: Option<String>,
    pub price_cents: i64,
    pub currency: String,
    pub transfer_done: bool,
}

// ---------------------------------------------------------------------------
// Pull received (2.0.17): the mirror direction of Pull above - someone ELSE
// pulls tickets FOR marko (marko pays them a fee) instead of marko pulling
// for someone else. See migrations/011_pulls_received.sql for the full
// schema rationale, and commands/pulls_received.rs for the CRUD logic.
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PullReceived {
    pub id: i64,
    pub code: String,
    pub puller_name: String,
    pub event_name: String,
    pub event_date: Option<String>,
    pub quantity: i64,
    /// marko's fee to the puller - informational only, never summed into
    /// FinanceSummary/CashflowSummary/Dashboard anywhere (marko confirmed
    /// via AskUserQuestion - the same standalone-from-finance choice
    /// `Pull.price_cents` already made for the other direction).
    pub amount_cents: i64,
    pub currency: String,
    pub more_info: Option<String>,
    /// Which Order these pulled tickets became, if any. Nullable: marko
    /// confirmed (via AskUserQuestion) a received pull must also work fully
    /// standalone, with no order to link to at all.
    pub order_id: Option<i64>,
    /// Convenience join of `orders.code` for display, e.g. "ORD-000042" -
    /// `None` whenever `order_id` is `None`. Same LEFT JOIN pattern as
    /// `Pull.platform_name`.
    pub order_code: Option<String>,
    /// `"manual"` (typed directly in the app) or `"sheet_sync"` (auto-created
    /// by Orders & Sales sheet sync when a synced row's `pull` column says
    /// "yes" - see commands/orders_sheet_sync.rs::maybe_link_pull_received).
    /// Not user-editable after creation - see `PullReceivedEditInput`'s doc
    /// comment.
    pub source: String,
    pub is_demo: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Input for `create_pull_received`. `source` deliberately isn't here as
/// free user input - see `create_pull_received_impl`/
/// `create_pull_received_with_source` in commands/pulls_received.rs for how
/// each creation path (the manual command vs. sheet sync) supplies its own
/// fixed `source` value. `order_id` IS user-settable here - an optional
/// manual link to an existing order.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PullReceivedInput {
    pub puller_name: String,
    pub event_name: String,
    pub event_date: Option<String>,
    pub quantity: i64,
    pub amount_cents: i64,
    pub currency: String,
    pub more_info: Option<String>,
    pub order_id: Option<i64>,
}

/// Input for `update_pull_received` - the full edit form. Same fields as
/// `PullReceivedInput` (including `order_id`, so marko can link or unlink a
/// standalone row to/from an order later); deliberately does NOT include
/// `source` - whether a row started out manual or sheet-sync-created is
/// provenance, not something an edit form should be able to rewrite, same
/// as `is_demo` never appearing in any *EditInput* in this codebase.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PullReceivedEditInput {
    pub puller_name: String,
    pub event_name: String,
    pub event_date: Option<String>,
    pub quantity: i64,
    pub amount_cents: i64,
    pub currency: String,
    pub more_info: Option<String>,
    pub order_id: Option<i64>,
}

// ---------------------------------------------------------------------------
// Google Sheets sync (Settings -> Integrations, 2.0.2+). See
// google_sheets.rs and commands/sheets_sync.rs for the full mechanism.
// Deliberately data-source-agnostic (a plain `data_source` string, e.g.
// "pulls" today) so a second connected sheet never needs its own struct.
// ---------------------------------------------------------------------------

/// Which spreadsheet+tab (if any) is linked for one data source. Stored as
/// JSON under an `app_settings` key (see commands/settings.rs's existing
/// generic key/value store) - this is configuration, not business data,
/// exactly what that table already exists for.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SheetsConnectionConfig {
    pub spreadsheet_id: String,
    pub sheet_tab: String,
    /// 2.0.3: the sheet has no currency column of its own (marko's Pulls
    /// tracker never had one) - one currency applies to every row synced
    /// from this sheet instead. Restricted to EUR/USD/GBP (marko: "menu
    /// mozes dat na EUR, USD a GBP, nic ine") - see
    /// commands/sheets_sync.rs::ALLOWED_CURRENCIES.
    pub currency: String,
}

/// What Settings shows for one data source's "Integrations" card.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SheetsConnectionStatus {
    /// Whether this build was even compiled with a service account embedded
    /// (false on a plain local dev build - see
    /// `google_sheets::embedded_service_account`'s doc comment).
    pub sync_available: bool,
    /// The address to show ("share your sheet with this address") when
    /// `sync_available` is true.
    pub service_account_email: Option<String>,
    pub connection: Option<SheetsConnectionConfig>,
    pub last_synced_at: Option<String>,
    /// 2.0.18: the "Push to sheet" direction's own separate timestamp - kept
    /// apart from `last_synced_at` (which only ever means "last pulled sheet
    /// -> app") precisely so the two directions never get blurred together in
    /// the UI. `None` until the first successful push for this data source.
    pub last_pushed_at: Option<String>,
}

/// Result of a manual "Test connection" click. `message` is deliberately kept
/// short and glanceable (2.0.15: marko's own request - the previous version
/// echoed Google's raw JSON error body directly into `message`, which was too
/// much to read at a glance); `hint` carries the concrete next step (e.g.
/// which e-mail to share the sheet with) as a separate line, for the small
/// set of failures the app can recognize and explain plainly. `hint` is
/// `None` both on success and for a failure the app doesn't recognize -
/// there, `message` itself keeps the full underlying detail (see
/// commands/sheets_sync.rs::test_sheets_connection_impl), since a vague short
/// headline would be worse than the real error for something unrecognized.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SheetsConnectionTestResult {
    pub ok: bool,
    pub message: String,
    pub hint: Option<String>,
}

/// Result of a best-effort attempt to detect a pasted spreadsheet's real tab
/// names (2.0.14), so Settings can offer them as a dropdown instead of
/// requiring marko to type the exact tab name by hand - see
/// commands/sheets_sync.rs::detect_spreadsheet_tabs_impl's doc comment for
/// why. Same "always a readable ok:false, never a thrown error" convention as
/// `SheetsConnectionTestResult` - the frontend calls this eagerly (e.g. as
/// soon as the URL field loses focus), and a half-typed or not-yet-shared
/// URL is the normal case, not something worth surfacing as a hard failure.
/// `tabs` is always empty when `ok` is false; `message` is empty when `ok` is
/// true (nothing to explain). Same short-`message`-plus-optional-`hint` split
/// as `SheetsConnectionTestResult` (2.0.15) and for the same reason - see its
/// doc comment.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SpreadsheetTabsResult {
    pub ok: bool,
    pub tabs: Vec<String>,
    pub message: String,
    pub hint: Option<String>,
}

/// Result of "Create a new sheet for me" (2.0.4) - the auto-create-and-share
/// flow that replaces a Google sign-in window with the same service account
/// every other connection uses (see commands/pulls_sheet_sync.rs::
/// create_pulls_sheet_impl and google_sheets.rs's `SHEETS_AND_DRIVE_SCOPE`
/// doc comment for why this needs no login popup). `connection` is already
/// persisted by the time this is returned - Settings just reloads its status
/// from it like any other connection change. `spreadsheet_url` is shown as
/// selectable text, not a clickable link (see Settings.tsx).
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CreatedSheetResult {
    pub connection: SheetsConnectionConfig,
    pub spreadsheet_url: String,
}

// ---------------------------------------------------------------------------
// Pulls <-> Google Sheet sync (2.0.3). See commands/pulls_sheet_sync.rs for
// the full column mapping and matching/conflict rules. Sheet -> app only in
// this pass - see that file's module doc comment for why.
// ---------------------------------------------------------------------------

/// One row-level problem from a sync run - either a parse/validation error
/// (that row was skipped) or a genuine two-sided conflict (that row was left
/// untouched on both sides, never guessed at - see
/// pulls_sheet_sync.rs::apply_pull_rows's doc comment). `row_number` is the
/// sheet's own row number (header = row 1), so it points at exactly the row
/// marko would scroll to in Google Sheets.
#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SheetSyncIssue {
    pub row_number: i64,
    pub message: String,
}

/// Result of one "Sync now" run, shown as-is in Settings -> Integrations.
/// Named generically (not e.g. `PullsSyncResult`) since 2.0.8, matching
/// `SheetSyncIssue` right above it - this shape was never actually specific
/// to Pulls, it's just "how many rows did what" plus row-level issues, and
/// commands::orders_sheet_sync (2.0.8) reuses it verbatim for Orders/Tickets
/// sync rather than a second, identically-shaped struct.
#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SheetSyncResult {
    pub created: i64,
    pub updated: i64,
    pub unchanged: i64,
    pub conflicts: Vec<SheetSyncIssue>,
    pub errors: Vec<SheetSyncIssue>,
    /// 2.0.42: rows that were NOT skipped and NOT flagged as a conflict -
    /// they were saved successfully, but only after
    /// commands::orders_sheet_sync::reconcile_order_pricing corrected a
    /// small, sensible-to-explain gap in their pricing (see that function's
    /// own doc comment). Always empty for every sync/push path other than
    /// Orders sync - kept on this shared struct rather than a second,
    /// Orders-only result type, same reasoning as this struct's own doc
    /// comment above for why `SheetSyncIssue`/`SheetSyncResult` are shared
    /// verbatim in the first place.
    pub corrected: Vec<SheetSyncIssue>,
    pub synced_at: String,
}

// ---------------------------------------------------------------------------
// Outbound notifications (2.0.76; email channel removed again in 2.0.77 -
// marko decided against it. The mobile-push channel was Pushover in 2.0.76-
// 2.0.77, replaced by ntfy in 2.0.78 because Pushover's API always needs
// BOTH a user key AND an application token, and marko wanted a channel that
// needs only ONE thing from him - see notifications.rs's module doc
// comment) - desktop/ntfy, built on top of the same 4 categories
// `DashboardAlerts` above already tracks. See commands/notifications.rs for
// the actual logic; these are just the DTOs, same split every other feature
// in this file already follows (dashboard.rs stays logic-only, its structs
// live here).
// ---------------------------------------------------------------------------

/// The whole notification configuration, stored as ONE JSON blob under
/// app_settings["notification_config"] via commands::sheets_sync's existing
/// get_setting/set_setting helpers (already reused once by google_auth.rs
/// for its own, separate secret - this is the second, independent reuse).
/// Round-trips through serde on both sides: Rust struct <-> JSON text <->
/// the `app_settings` table.
///
/// `#[serde(default)]` on every level here defensively tolerates a stored
/// blob from a slightly older shape (e.g. a field added in a later version,
/// or - 2.0.77 - the whole `email` field this struct used to carry, or -
/// 2.0.78 - the `pushover` field it carried before that) failing to
/// deserialize a whole config just because one field no longer matches -
/// the same defensive posture this codebase already takes with external API
/// response shapes (see google_sheets.rs's `conditional_formats`/`grid_
/// properties`), applied here to a value THIS app itself writes and later
/// reads back across app updates.
///
/// This is the ONLY place a real secret (the ntfy topic) is ever held as a
/// plain field - it must never be sent to the frontend as-is. See
/// `NotificationStatus` (safe to send) and `NotificationConfigInput` (safe
/// to receive) below.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct NotificationConfig {
    pub desktop_enabled: bool,
    pub ntfy: NtfyChannelConfig,
}

/// 2.0.78: ntfy (https://ntfy.sh) replaced Pushover as this app's mobile-
/// push channel - see notifications.rs's module doc comment for the full
/// reasoning. `topic` is the ONE thing a person needs: a name they made up
/// themselves, entered both here and in the free ntfy app on their phone.
/// ntfy's public server needs no signup and no application-level credential
/// on either side, so - unlike Pushover - there is nothing for this app to
/// ship an embedded build secret for any more. Treated as a secret (never
/// echoed back, see `NotificationStatus` below) because on ntfy's public
/// server the topic name IS the entire access control - anyone who knows it
/// can publish to it or read it.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct NtfyChannelConfig {
    pub enabled: bool,
    pub topic: String,
}

/// What Settings -> Notifications actually receives from the GET status
/// command - same shape as `NotificationConfig`, but the secret field is
/// replaced by a `*_set: bool` presence flag. Precedented by
/// `GoogleSignInStatus` (google_auth.rs), which returns `sign_in_available:
/// bool` + an email but never the OAuth refresh token itself - same idea,
/// second implementation.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NotificationStatus {
    pub desktop_enabled: bool,
    pub ntfy_enabled: bool,
    pub ntfy_topic_set: bool,
}

/// What Settings -> Notifications sends to the SET command. The secret
/// field is `Option<String>`: `None` means "leave whatever is already
/// stored untouched", `Some(value)` overwrites it (including with an empty
/// string, if that's ever genuinely what's submitted). The frontend never
/// pre-fills the secret field from `NotificationStatus` (which only ever
/// carries a presence boolean, never the value) - leaving it blank is
/// submitted as `None`, which is exactly "leave unchanged".
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NotificationConfigInput {
    pub desktop_enabled: bool,
    pub ntfy_enabled: bool,
    pub ntfy_topic: Option<String>,
}

/// Result of a "Send test" click in Settings -> Notifications (one per
/// channel) - unlike the silent periodic check, a test click should say
/// plainly whether it worked.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NotificationTestResult {
    pub success: bool,
    pub message: String,
}

// ---------------------------------------------------------------------------
// Price Checker (2.0.81) - see commands::price_checker's own module doc
// comment and migrations/014_price_checker.sql for the full design.
// ---------------------------------------------------------------------------

/// A marketplace marko can save a link/price checks against for an event
/// (Vivid Seats/Ticombo/Viagogo, seeded - same "Platforms"-style lookup
/// pattern as `Platform`/`Supplier`/`EventCategory`, so he can add more of
/// his own later without a new migration.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Marketplace {
    pub id: i64,
    pub name: String,
    /// 2.1.6 (migrations/017_price_checker_viagogo.sql): whether marko can
    /// start something NEW here - a fresh link, a fresh check. `false` only
    /// for StubHub, retired at his own request in favor of Viagogo but kept
    /// exactly as-is (never deleted/renamed) so every past check against it
    /// stays real, readable history - see
    /// `commands::price_checker::get_price_checker_summary_impl`'s own doc
    /// comment for exactly how this changes which marketplaces an event's
    /// page shows.
    pub active: bool,
    pub is_demo: bool,
    pub created_at: String,
}

/// One saved marketplace URL for one event.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EventMarketplaceLink {
    pub id: i64,
    pub event_id: i64,
    pub marketplace_id: i64,
    pub url: String,
    pub created_at: String,
    pub updated_at: String,
}

/// `url` blank/whitespace-only means "clear this marketplace's link" - see
/// `commands::price_checker::save_event_marketplace_link_impl`.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EventMarketplaceLinkInput {
    pub event_id: i64,
    pub marketplace_id: i64,
    pub url: String,
}

/// One "Check Prices" entry - hand-typed off a marketplace's own listings
/// page, pasted, or reviewed from a Visible Scanner session (2.1.9). Always
/// the same shape regardless of source - see `commands::price_checker_
/// scanner`'s own module doc comment for why the scanner deliberately funnels
/// into this exact table rather than getting its own. Append-only (see the
/// migration's own doc comment) - never updated or overwritten by a later
/// check for the same event/marketplace.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PriceCheck {
    pub id: i64,
    pub event_id: i64,
    pub marketplace_id: i64,
    pub lowest_price_cents: i64,
    pub average_price_cents: i64,
    pub highest_price_cents: i64,
    pub listing_count: i64,
    pub currency: String,
    /// 2.1.9 (migrations/018_price_checker_scanner.sql). `None` for every
    /// check saved before this version, or for a fully hand-typed entry with
    /// no underlying list of individual prices to compute a real median
    /// from - never backfilled/guessed from lowest/average/highest, which
    /// would not be a real median (marko's own "never invent data" rule).
    pub median_price_cents: Option<i64>,
    pub checked_at: String,
    /// 2.2.0 (migrations/019_price_checker_market_analysis.sql) - empty for
    /// every check saved before this version, or a manual/pasted entry with
    /// no per-tier detail to give (same "None/empty means genuinely not
    /// computed, never backfilled" rule as `median_price_cents`). Attached
    /// separately from the row's own columns - see
    /// `commands::price_checker::fetch_tier_breakdown`'s own doc comment for
    /// why `map_price_check` alone can't populate this field.
    pub tier_breakdown: Vec<TierBreakdownRecord>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PriceCheckInput {
    pub event_id: i64,
    pub marketplace_id: i64,
    pub lowest_price_cents: i64,
    pub average_price_cents: i64,
    pub highest_price_cents: i64,
    pub listing_count: i64,
    pub currency: String,
    /// See `PriceCheck::median_price_cents` - optional for the same reason.
    pub median_price_cents: Option<i64>,
    /// 2.2.0: optional per-tier breakdown to persist alongside this check -
    /// see `TierBreakdownInput`'s own doc comment. `#[serde(default)]` so a
    /// manual/pasted entry (no scan session behind it at all) can omit this
    /// key entirely from the frontend's JSON payload rather than having to
    /// send an explicit empty array.
    #[serde(default)]
    pub tier_breakdown: Vec<TierBreakdownInput>,
}

/// One marketplace's row in the Price Checker page for one event: its saved
/// link (if any) plus its full check history, newest first (`history[0]` is
/// the latest check, `history[1]` the one before it - marko explicitly
/// wants to see whether the price moved up or down since last time, so the
/// frontend derives that delta straight from these two rather than the
/// backend baking in a single "trend" value). Present for every ACTIVE
/// marketplace, even one marko has never linked or checked yet for this
/// event (`link` is `None`, `history` is empty) - so the page always shows
/// all of his active marketplaces as a place to add data, not just the ones
/// already filled in. A RETIRED marketplace (`active: false` - StubHub as
/// of 2.1.6) additionally appears whenever THIS event already has a link or
/// history against it, so past data is never hidden - see
/// `commands::price_checker::get_price_checker_summary_impl`'s own doc
/// comment for the exact rule.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MarketplacePriceView {
    pub marketplace_id: i64,
    pub marketplace_name: String,
    /// 2.1.6: mirrors `Marketplace::active` for this marketplace - lets the
    /// frontend show a "retired" note (e.g. next to StubHub) without a
    /// separate lookup.
    pub marketplace_active: bool,
    pub link: Option<EventMarketplaceLink>,
    pub history: Vec<PriceCheck>,
}

/// The whole Price Checker page for one event in a single round trip: every
/// marketplace's link + history, marko's own unsold inventory for this
/// event (cost/listing price, from the exact same ticket scope Event
/// Detail's own "Potential profit" block already uses - available/listed,
/// never sold/cancelled), and the derived market comparison
/// (lowest/average/recommended/expected profit/ROI). See
/// `commands::price_checker::get_price_checker_summary_impl`'s own doc
/// comment for exactly how each derived field is computed and why.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PriceCheckerSummary {
    pub event_id: i64,
    pub event_name: String,
    pub event_date: Option<String>,
    pub marketplaces: Vec<MarketplacePriceView>,
    /// `None` when marko's own unsold tickets for this event don't all
    /// share one currency (or there are none at all) - same signal
    /// `EventWithStats.stats.currency`/`FinanceSummary.currency` already use
    /// everywhere else in this app.
    pub my_currency: Option<String>,
    pub unsold_ticket_count: i64,
    /// Average of (purchase cost + purchase fees + other costs) across
    /// `unsold_ticket_count` tickets. `None` only when that count is 0 - NOT
    /// suppressed when `my_currency` is `None`. Same "always return the
    /// blended figure, let the currency flag decide whether the UI shows it
    /// or shows Mixed" convention as `total_cost_cents` elsewhere (see
    /// `format.ts`'s `formatMoneyOrMixed`) - unlike a RATIO (margin/ROI
    /// below), a blended money amount can't silently masquerade as a real
    /// one, so it doesn't need backend-level suppression, just the same
    /// currency flag every other money amount in this app already carries.
    pub my_avg_purchase_cost_cents: Option<i64>,
    /// Same shape as `my_avg_purchase_cost_cents`, but only over the subset
    /// of unsold tickets that actually have a `listing_price_cents` set -
    /// `None` when none of them do (not the same thing as `my_currency`
    /// being `None`).
    pub my_avg_listing_price_cents: Option<i64>,
    pub missing_listing_price_count: i64,
    /// The two fields below (and everything derived from them further down)
    /// are the one place this summary DOES require a real, single
    /// `my_currency` - unlike the two averages above, comparing against the
    /// market means picking exactly one currency to match each
    /// marketplace's latest check against, and this app never guesses that.
    /// `None` whenever `my_currency` is `None`, OR when it is `Some` but no
    /// marketplace's latest check happens to share it yet.
    pub market_lowest_price_cents: Option<i64>,
    pub market_average_price_cents: Option<i64>,
    /// `market_lowest_price_cents` reduced by
    /// `commands::price_checker::RECOMMENDED_PRICE_UNDERCUT_PCT` - marko's
    /// own answer for how this should work ("Mierne pod najnižšou trhovou
    /// cenou" - slightly under the lowest market price), kept as one plain
    /// transparent formula rather than AI, per his explicit request.
    pub recommended_price_cents: Option<i64>,
    /// `recommended_price_cents - my_avg_purchase_cost_cents` - `None`
    /// unless both are `Some` (which, per the note above, already implies a
    /// real shared `my_currency`).
    pub expected_profit_cents: Option<i64>,
    /// `expected_profit_cents / my_avg_purchase_cost_cents` via
    /// `finance::safe_ratio` - `None` under the same conditions as
    /// `expected_profit_cents`, plus the ordinary zero-cost case
    /// `safe_ratio` itself already guards against.
    pub expected_roi: Option<f64>,
}

/// One deduplicated listing accumulated by a Visible Scanner session
/// (2.1.9) - see `commands::price_checker_scanner`'s own module doc comment
/// for the full design. Unlike the old hidden auto-check's separate
/// `prices`/`listings` arrays, this is the ONE shape every scan result
/// becomes: a bare price with no confirmed context is simply a
/// `NormalizedListing` whose `section`/`row`/`quantity`/`listing_id` are all
/// `None` - `commands::price_checker_scanner::derive_session_status` is what
/// turns "does this session have any WITH context" into success vs. partial,
/// not a separate field here. Never fabricated: every field is exactly what
/// the page's own DOM/text actually had, or `None`.
#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedListing {
    pub price_cents: i64,
    pub currency: Option<String>,
    pub section: Option<String>,
    pub row: Option<String>,
    /// 2.2.0 (Market Analysis): the tier/level label exactly as the page
    /// itself showed it ("Level 100", "Tier 1", ...), best-effort detected
    /// by `tierFor` (price_checker_scan.js) - inline text within the
    /// listing's own container, or the nearest preceding heading-shaped
    /// element above it when the page renders tier as a group header
    /// instead (the more common real-world shape). `None` when neither
    /// found anything - `commands::price_checker_analysis::group_by_tier`
    /// is what turns that into the literal displayed string "Unclassified",
    /// never guessed here or anywhere upstream of it.
    pub tier: Option<String>,
    pub quantity: Option<u32>,
    /// Best-effort element `id`/`data-*id*` attribute, when the page's own
    /// markup has one - the strongest fingerprint component when present
    /// (see `commands::price_checker_scanner::fingerprint_for`), but most
    /// real listing elements won't have one, so this staying `None` is the
    /// normal case, not a failure.
    pub listing_id: Option<String>,
    /// Which reader produced this - `"stubhub"` / `"vividseats"` /
    /// `"ticombo"` / `"generic"`, mirrors the old auto-check's
    /// `AutoCheckDiagnostics::marketplace_reader`. Constant across every
    /// listing in one scan session today (one session always targets one
    /// marketplace card), carried per-listing anyway so a future combined
    /// view across sessions (marko's spec's "Market Overview") never needs a
    /// join to know which marketplace each row came from.
    pub marketplace: String,
}

/// `scan_visible_prices`'s result, delivered via the
/// `price-scanner-scan-result` event (2.1.9) - the accumulated state of the
/// WHOLE session after merging this scan's findings in, not just this scan's
/// own delta (`added_this_scan` is the delta, everything else is the running
/// total) - see `commands::price_checker_scanner`'s own module doc comment.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScanResultPayload {
    pub request_id: u64,
    /// "success" | "partial" | "unable_to_read" | "blocked" | "error" - see
    /// `commands::price_checker_scanner::derive_session_status`.
    pub status: String,
    /// How many NEW, previously-unseen listings THIS scan added - lets the
    /// frontend say "found 6 new listings" rather than just showing the
    /// (potentially unchanged) running total, matching marko's own spec
    /// example ("Scan 1 -> 20 listings, scroll, Scan 2 -> ďalších 20").
    pub added_this_scan: u32,
    pub listings: Vec<NormalizedListing>,
    pub lowest_price_cents: Option<i64>,
    pub median_price_cents: Option<i64>,
    pub average_price_cents: Option<i64>,
    pub highest_price_cents: Option<i64>,
    /// First non-`None` currency seen across `listings`, in insertion order -
    /// `None` only when `listings` is empty. Same "first candidate's
    /// currency wins" convention the old extraction JS already used.
    pub currency: Option<String>,
    pub scan_count: u32,
    pub last_scan_at: Option<String>,
    /// Human-readable detail for a non-"success" status - `None` when
    /// `status` is "success" (nothing to explain). Built the same way the
    /// old auto-check's diagnostic message was: real counts/text the page
    /// actually had, never invented.
    pub message: Option<String>,
}

/// `open_price_scanner`'s async outcome, delivered via the
/// `price-scanner-opened` / `price-scanner-error` events (2.1.9) - mirrors
/// the old auto-check's own "command returns almost immediately, the real
/// outcome arrives later via an event" pattern (see that design's own
/// history in PROTECTED-AREAS-NOTES.md, "an extraction attempt's own eval
/// timeout..." - the freeze-avoidance half of that lesson, not the timeout
/// half, still applies here: `WebviewWindowBuilder::build()` must never be
/// called synchronously from a command's own call stack, Tauri's own
/// documented Windows deadlock).
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScannerOpenedPayload {
    pub request_id: u64,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScannerErrorPayload {
    pub request_id: u64,
    pub message: String,
}

/// Fires when a scanner window closes - either marko clicked "Close window"
/// (`close_price_scanner`) or he closed it directly himself (the window's own
/// native close button, detected via `on_window_event`/`CloseRequested` -
/// see `commands::price_checker_scanner::open_price_scanner`). The frontend
/// already has every listing this session ever found (each `scan_visible_
/// prices` call already delivered them via `ScanResultPayload` above), so
/// nothing is lost - this only tells the card to stop offering Scan/Stop for
/// a session that no longer has a window behind it.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScannerClosedPayload {
    pub request_id: u64,
}

// --- Price Checker Market Analysis (2.2.0) ----------------------------------
// Built entirely on top of the Visible Scanner above - reads a session's
// already-accumulated `NormalizedListing`s (or a saved `PriceCheck`'s tier
// breakdown), never touches the scanner's own commands/session/lifecycle
// code. See `commands::price_checker_analysis`'s own module doc comment for
// the full design and PRICE-CHECKER-MARKET-ANALYSIS-2.2-REPORT.md for what's
// verified vs. not. Status-like classifications here follow this whole
// feature's existing convention (`ScannerSession::status`/
// `derive_session_status`) - a plain, literal, lower_snake_case `String`,
// not a new Rust enum type.

/// Lowest/median/average/highest/count over a group of listings that all
/// share one currency - the same 4 stats `commands::price_checker_scanner::
/// compute_scan_stats` already computes session-wide, reused here per
/// tier/section/currency group instead (see `price_stats_for`,
/// commands::price_checker_analysis).
#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PriceStats {
    pub lowest_price_cents: i64,
    pub median_price_cents: i64,
    pub average_price_cents: i64,
    pub highest_price_cents: i64,
    pub listing_count: i64,
}

/// One section's stats within one tier - marko's own spec, "## MAP /
/// SECTION ANALYSIS".
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SectionBreakdown {
    /// The section label exactly as read from the page - listings with no
    /// section at all are simply not grouped here; they still count toward
    /// the tier's own `TierBreakdown::stats`.
    pub section: String,
    pub stats: PriceStats,
}

/// One tier/level's full breakdown - marko's own spec, "## TIER PRICING" +
/// "## MAP / SECTION ANALYSIS". `tier` is exactly what the page called it
/// ("Level 100", "Tier 1", ...), or the literal string `"Unclassified"` for
/// every listing `tierFor` (price_checker_scan.js) couldn't confidently
/// place - `commands::price_checker_analysis::group_by_tier` is the only
/// place that string is written, see its own doc comment.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TierBreakdown {
    pub tier: String,
    pub stats: PriceStats,
    /// Sorted by `stats.lowest_price_cents` ascending - marko's own spec
    /// example lists sections lowest-to-highest within a tier.
    pub sections: Vec<SectionBreakdown>,
}

/// One listing ranked against a specific reference ticket/spec - marko's own
/// spec, "## COMPARABLE MARKET". `level` is one of "exact_comparable" |
/// "close_comparable" | "tier_comparable" | "general_market", decided by
/// `classify_comparable` (commands::price_checker_analysis) purely from
/// marko's own literal priority list ("same section, same tier, nearby
/// sections in same tier, same quantity, nearby rows") - a plain field
/// comparison against `reference`, independent of `data_quality` below.
/// `level` and `data_quality` are deliberately two separate, honest facts
/// about the same listing, not one gating the other: a listing can be
/// `"exact_comparable"` (its `section` matches the reference's) while its
/// own `data_quality` is still `"partial"` (no tier/row/quantity confirmed
/// beyond that section) - suppressing a genuine section match just because
/// other fields are missing would contradict marko's own priority order,
/// where section match is checked first, before tier. `data_quality` is one
/// of "strong_comparable" | "section_comparable" | "tier_comparable" |
/// "partial" - marko's own spec, "## DATA QUALITY", decided by
/// `data_quality_for` (same module) purely from which fields THIS listing
/// actually has, independent of any reference ticket.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RankedComparable {
    pub listing: NormalizedListing,
    pub level: String,
    pub data_quality: String,
}

/// What to compare a scan session's listings against - marko's own spec
/// example ("Section 112, Row 8, Quantity 4"). Every seat-shape field is
/// optional: the less marko provides, the less specific a match
/// `rank_comparable` can honestly claim. `currency` is NOT optional, unlike
/// the others - marko's own "## CURRENCY" is explicit that EUR/USD/GBP must
/// never be blended, and a comparable ranking that mixed listings from more
/// than one currency into a single lowest/median would do exactly that; the
/// caller picks which of the scan session's `by_currency` groups (see
/// `MarketAnalysisResult`) to compare the reference ticket against, same as
/// how the rest of this feature is already split per currency rather than
/// guessing which one the reference ticket "must" mean. This mirrors
/// `commands::price_checker::get_price_checker_summary_impl`'s own
/// `my_currency` requirement for its market comparison - not something
/// marko's spec spelled out for this specific command, but the same rule
/// applied consistently; flagged as a design decision in
/// PRICE-CHECKER-MARKET-ANALYSIS-2.2-REPORT.md.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ComparableReferenceInput {
    pub request_id: u64,
    pub section: Option<String>,
    pub tier: Option<String>,
    pub row: Option<String>,
    pub quantity: Option<u32>,
    pub currency: String,
}

/// One group of marko's own unsold tickets for this event - marko's own
/// spec, "## YOUR TICKETS" + "## PRICE RECOMMENDATION". One row per
/// (section, row) group of available/listed tickets (same scope
/// `commands::price_checker::get_price_checker_summary_impl` already uses),
/// grouped rather than one row per physical ticket, since several identical
/// unsold tickets in the same section/row are one real pricing decision, not
/// several.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct YourTicketGroup {
    /// Still always `None` as of 2.2.7 - unchanged by this field's own
    /// original reasoning below, which no longer fully holds: `tickets` DOES
    /// now have a real `tier` column (migration 024, `Ticket::tier`), added
    /// for the Event Workspace/Inventory Intelligence task, not for Market
    /// Analysis. Wiring that real, already-stored value into "Your Tickets"
    /// grouping here is a deliberate, NOT-YET-DONE follow-up (marko's own
    /// 2.2.7 instruction: "pripravit data tak, aby ich neskor vedel pouzivat
    /// Market Analysis" - prepare the data, don't wire it in yet) - out of
    /// scope for a ticket-metadata task that explicitly excluded this
    /// module. Original reasoning, kept for context: checked against the
    /// real `tickets` schema before writing this (marko's own spec, point
    /// #18: investigate first, never invent a field that doesn't exist) -
    /// `tickets` had `section`/`row_label`/`seat` but no tier/level column at
    /// all; the one field that could be mistaken for it, `ticket_type`, is
    /// actually a DELIVERY method (`TICKET_TYPES` in Orders.tsx: "E-ticket"/
    /// "PDF"/"Mobile transfer"/"Physical"/"Will call"), not a seating tier -
    /// grouping "Your Tickets" by it would silently produce nonsense groups.
    /// Kept as `Option<String>` (matching `NormalizedListing::tier` and
    /// `ComparableReferenceInput::tier`) so a real tier source could be wired
    /// in later without a breaking shape change - see
    /// PRICE-CHECKER-MARKET-ANALYSIS-2.2-REPORT.md's UNAVAILABLE DATA
    /// section.
    pub tier: Option<String>,
    pub section: Option<String>,
    pub row: Option<String>,
    pub quantity: i64,
    pub currency: String,
    pub avg_cost_cents: i64,
    /// `None` when none of the tickets in this group have a listing price
    /// set yet.
    pub avg_listing_price_cents: Option<i64>,
    /// `None` when the CURRENT scan session has no listings at all in this
    /// group's own currency - never blended from a different currency's
    /// figures (marko's own "## CURRENCY": never sum/blend EUR+USD+GBP).
    pub recommendation: Option<PriceRecommendation>,
}

/// A single ticket group's price recommendation - marko's own spec, "##
/// PRICE RECOMMENDATION": every number here is a plain, transparent
/// calculation (see `recommend_price`, commands::price_checker_analysis),
/// never AI - reuses `commands::price_checker::RECOMMENDED_PRICE_UNDERCUT_
/// PCT`, the same constant the manual/history-based Price Checker summary
/// already uses, rather than a second, different formula.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PriceRecommendation {
    pub comparable_lowest_price_cents: i64,
    pub comparable_median_price_cents: i64,
    pub market_average_price_cents: i64,
    /// `comparable_lowest_price_cents` reduced by
    /// `RECOMMENDED_PRICE_UNDERCUT_PCT`.
    pub recommended_price_cents: i64,
    /// `recommended_price_cents - avg_cost_cents` for this group.
    pub expected_profit_cents: i64,
    pub expected_roi: Option<f64>,
    /// Which comparable pool `comparable_lowest_price_cents`/`_median_` were
    /// actually computed from - "Same section" | "Close match (same tier)" |
    /// "Same tier" | "General market", the human-readable form of
    /// `RankedComparable::level`'s 4 values in the same priority order
    /// (`commands::price_checker_analysis::rank_comparable` always picks the
    /// narrowest non-empty pool) - marko's own spec, "Based on:".
    pub based_on: String,
    /// "High" | "Medium" | "Low" - see `recommendation_confidence`
    /// (commands::price_checker_analysis) for the exact rule.
    pub confidence: String,
}

/// Everything derived from ONE currency's worth of a scan session's
/// listings - marko's own spec, "## MARKET OVERVIEW" + "## TIER PRICING" +
/// "## MAP / SECTION ANALYSIS", all computed together in one pass over the
/// same listings (see `commands::price_checker_analysis::
/// compute_market_analysis`, "## PERFORMANCE" - normalize once, derive
/// everything else from that, never re-read the page per computation).
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CurrencyMarketAnalysis {
    pub currency: String,
    pub overall: PriceStats,
    /// Sorted by `stats.lowest_price_cents` ascending - marko's own spec
    /// example lists Level 100/200/500 lowest-to-highest.
    pub tiers: Vec<TierBreakdown>,
}

/// `compute_market_analysis`'s whole result - one `CurrencyMarketAnalysis`
/// per currency actually present in the session's listings, since prices in
/// different currencies are never blended into one figure (marko's own
/// spec, "## CURRENCY": "EUR + USD + GBP nikdy nesčítavaj ... alebo rozdeľ
/// podľa meny" - this IS that split). `your_tickets` lives at this top
/// level, not inside one `CurrencyMarketAnalysis`, because marko's own
/// inventory has its own currency independent of what has been scanned so
/// far - see `YourTicketGroup::recommendation`'s own doc comment for what
/// happens when nothing scanned yet matches it.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MarketAnalysisResult {
    pub request_id: u64,
    pub by_currency: Vec<CurrencyMarketAnalysis>,
    /// True when the session's listings span more than one currency - lets
    /// the frontend show an explicit "Mixed - split by currency below" note
    /// rather than the user having to notice `by_currency.len() > 1` itself.
    pub mixed_currencies: bool,
    /// How many of the session's listings have a price but no currency at
    /// all (a money-shaped amount was found with no symbol/code attached).
    /// These contribute to NO `by_currency` entry - grouping them under a
    /// guessed currency would fabricate data - so this plain count is how
    /// the UI stays honest about them instead of silently dropping them.
    pub uncurrencied_listing_count: i64,
    pub your_tickets: Vec<YourTicketGroup>,
}

/// One saved tier breakdown row for a `PriceCheck` (2.2.0,
/// migrations/019_price_checker_market_analysis.sql) - marko's own spec,
/// "## PRICE HISTORY".
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TierBreakdownRecord {
    pub tier: String,
    pub lowest_price_cents: i64,
    pub median_price_cents: i64,
    pub listing_count: i64,
}

/// What `save_price_check` additionally accepts to populate
/// `price_check_tiers` - empty by default (`#[serde(default)]` on
/// `PriceCheckInput::tier_breakdown`) so every existing caller (manual/
/// pasted entries, which have no per-tier breakdown to give) is completely
/// unaffected; see `commands::price_checker::save_price_check_impl`.
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct TierBreakdownInput {
    pub tier: String,
    pub lowest_price_cents: i64,
    pub median_price_cents: i64,
    pub listing_count: i64,
}

// --- Finance (2.0.83) -------------------------------------------------------
// marko's personal + business money tracker - see
// migrations/015_finance.sql's doc comment for the full design rationale.
// Lives in commands::finance_entries, deliberately NOT named
// `commands::finance` - that name is already the shared ticket-business P&L
// calculation module (`crate::finance`, imported at the top of this file)
// and is a completely different concept from this manual personal/business
// ledger; picking a visibly different name keeps the two from ever being
// confused with each other.

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FinanceCategory {
    pub id: i64,
    pub name: String,
    /// 'expense' | 'income' | 'both' - which of Finance's two entry-type
    /// lists (mirrors FinanceEntry::entry_type) this category shows up in.
    /// Same convention as `Platform::kind` ('purchase'/'sale'/'both').
    pub kind: String,
    /// Same convention as `EventCategory::color_slot` - a fixed palette
    /// index (FinanceCategoryBadge.tsx) assigned once at creation via
    /// MAX(color_slot)+1, never recomputed.
    pub color_slot: i64,
    pub is_demo: bool,
    pub created_at: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FinanceEntry {
    pub id: i64,
    /// 'income' | 'expense'.
    pub entry_type: String,
    pub entry_date: String,
    pub amount_cents: i64,
    pub currency: String,
    /// 'personal' | 'business' - see migrations/015_finance.sql's
    /// `finance_entries.scope` comment for why this is just a label on an
    /// otherwise-identical entry, never a link into orders/tickets/sales.
    pub scope: String,
    pub category_id: Option<i64>,
    /// Denormalized from a LEFT JOIN, same convention as `Ticket::event_name`/
    /// `Ticket::order_code` - lets the frontend list/badge a category without
    /// a second round trip. `None` exactly when `category_id` is `None`
    /// (no category picked) or points at a category since deleted (ON DELETE
    /// SET NULL already clears `category_id` itself in that case too).
    pub category_name: Option<String>,
    pub category_color_slot: Option<i64>,
    /// 2.1.0: which `Account` this entry was recorded against - optional,
    /// same "denormalized name alongside the id, None exactly when unset or
    /// pointing at a since-deleted row" convention as `category_id`/
    /// `category_name` right above (accounts.rs's own `ON DELETE SET NULL`
    /// is the whole story here too - see migrations/016_finance_v2.sql).
    pub account_id: Option<i64>,
    pub account_name: Option<String>,
    /// 2.2.1: which `Order` this entry represents the recorded cost of -
    /// optional, same "denormalized label alongside the id, None exactly
    /// when unset or pointing at a since-deleted row" convention as
    /// `account_id`/`account_name` right above (`ON DELETE SET NULL`, see
    /// migrations/021_finance_entry_order_link.sql). Deliberately NOT a
    /// replacement for `orders.total_cost_cents` - that stays the order's
    /// own protected number (see PROTECTED_AREAS.md); this is just a
    /// reference to it so Finance and Orders can be cross-checked.
    pub order_id: Option<i64>,
    pub order_code: Option<String>,
    pub place: Option<String>,
    pub note: Option<String>,
    pub is_demo: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Input for both `create_finance_entry` and `update_finance_entry` - same
/// "one struct, not a fistful of flat arguments" convention as `OrderInput`
/// above, used here for the same reason (this has as many fields as an
/// order's own input does).
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FinanceEntryInput {
    pub entry_type: String,
    pub entry_date: String,
    pub amount_cents: i64,
    pub currency: String,
    pub scope: String,
    pub category_id: Option<i64>,
    /// 2.1.0: optional, same as `category_id` - "Account" is never a
    /// required field on an entry (marko's own point 4: "má možnosť
    /// Account", has the OPTION of an account), so every entry created
    /// before this version, and any entry created without one, is simply
    /// `None`.
    pub account_id: Option<i64>,
    /// 2.2.1: optional, same as `account_id` - most entries still have no
    /// order at all (personal spending, business income, anything not a
    /// ticket purchase).
    pub order_id: Option<i64>,
    pub place: Option<String>,
    pub note: Option<String>,
}

// --- Finance 2.1 (Accounts / Transfers / Recurring / Forecast) -------------
// marko's own "FINANCE 2.1" spec - see FINANCE-2.1.0-REPORT.md for the full
// rationale, and migrations/016_finance_v2.sql's doc comment for the schema
// and FK-delete design these structs mirror. Lives in three new modules,
// same "impl function + thin #[tauri::command] wrapper" pattern as
// commands::finance_entries: commands::finance_accounts (Account CRUD +
// balances + Transfer CRUD), commands::finance_recurring (RecurringExpense
// CRUD + Create/Skip/Pause/Resume), commands::finance_forecast (read-only).

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: i64,
    pub name: String,
    /// 'bank' | 'revolut' | 'paypal' | 'cash' | 'credit_card' | 'other' -
    /// only ever picks an icon/preset label (Accounts.tsx); `name` is
    /// always marko's own free text, never restricted by this.
    pub account_type: String,
    pub currency: String,
    pub opening_balance_cents: i64,
    /// Computed fresh every call as opening_balance_cents + this account's
    /// own finance_entries (income/expense) + transfers (in/out) - ONE
    /// aggregate query across every account at once (see
    /// commands::finance_accounts::list_accounts), never stored, never a
    /// per-account query. "Balance účtu musí vychádzať z jeho transakcií"
    /// (marko's own point 3) - this is the whole implementation of that.
    pub current_balance_cents: i64,
    pub is_active: bool,
    pub is_demo: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AccountInput {
    pub name: String,
    pub account_type: String,
    pub currency: String,
    pub opening_balance_cents: i64,
    pub is_active: bool,
}

/// One movement of marko's own money between two of his own accounts -
/// deliberately NOT a `FinanceEntry` (never income/expense, never touches
/// finance_entries at all) - see migrations/016_finance_v2.sql's header
/// comment for why this is its own table.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Transfer {
    pub id: i64,
    pub transfer_date: String,
    pub from_account_id: i64,
    /// Denormalized from a JOIN, same convention as
    /// `FinanceEntry::category_name` - always `Some` in practice since
    /// accounts referenced by a transfer can never be deleted (`ON DELETE
    /// RESTRICT`, see the migration), but typed as `Option` rather than a
    /// bare `String` so a display bug elsewhere can never panic on this.
    pub from_account_name: Option<String>,
    pub to_account_id: i64,
    pub to_account_name: Option<String>,
    pub amount_cents: i64,
    /// Always equal to both accounts' own currency - derived server-side,
    /// never taken from client input (see `TransferInput` below and
    /// commands::finance_accounts::create_transfer_impl).
    pub currency: String,
    pub note: Option<String>,
    pub is_demo: bool,
    pub created_at: String,
}

/// Deliberately has NO `currency` field - v1 disallows cross-currency
/// transfers entirely (marko's own preferred "simpler, safer" option, point
/// 6), so the currency is never a real choice: `create_transfer_impl`
/// derives it from `from_account`/`to_account` after confirming they match,
/// rather than trusting (and having to double-check) a client-supplied
/// value that could disagree with the accounts themselves.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TransferInput {
    pub transfer_date: String,
    pub from_account_id: i64,
    pub to_account_id: i64,
    pub amount_cents: i64,
    pub note: Option<String>,
}

/// A scheduled TEMPLATE, not a transaction - see
/// migrations/016_finance_v2.sql's header comment and
/// commands::finance_recurring's module doc comment for why the actual
/// `FinanceEntry` row is only ever created through an explicit user action.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RecurringExpense {
    pub id: i64,
    pub name: String,
    pub amount_cents: i64,
    pub currency: String,
    /// 'personal' | 'business' - same convention as `FinanceEntry::scope`;
    /// carried on the template so the `FinanceEntry` a "Create" action
    /// produces always has a valid scope without asking again.
    pub scope: String,
    pub category_id: Option<i64>,
    pub category_name: Option<String>,
    pub category_color_slot: Option<i64>,
    pub account_id: Option<i64>,
    pub account_name: Option<String>,
    /// 'weekly' | 'monthly' | 'quarterly' | 'yearly'.
    pub frequency: String,
    pub start_date: String,
    /// The next occurrence this template will produce - only ever advanced
    /// by an explicit Create/Skip action, never automatically. A value in
    /// the past (on an still-active template) means "overdue", shown as
    /// such by the frontend rather than silently caught up.
    pub next_date: String,
    pub is_active: bool,
    pub note: Option<String>,
    pub is_demo: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Input for both create and edit. Deliberately excludes `next_date` and
/// `is_active` - those are runtime state managed only by their own actions
/// (create_from_recurring/skip advance `next_date`; pause/resume flip
/// `is_active`), never by a generic field edit, so editing e.g. `frequency`
/// on an existing template can never silently jump or reset its schedule.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RecurringExpenseInput {
    pub name: String,
    pub amount_cents: i64,
    pub currency: String,
    pub scope: String,
    pub category_id: Option<i64>,
    pub account_id: Option<i64>,
    pub frequency: String,
    pub start_date: String,
    pub note: Option<String>,
}

/// Result of `commands::finance_forecast::get_cashflow_forecast` - a simple,
/// non-AI projection built only from data already in the app (marko's own
/// point 9/10: current balances, known pending amounts, scheduled recurring
/// expenses, already-logged future entries - never guessed sales, market
/// prices, or inventory profit).
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CashflowForecast {
    /// `false` when there are no active EUR accounts to project a balance
    /// from - the frontend shows "Forecast unavailable / limited data"
    /// (marko's own explicit requirement) rather than a forecast built on
    /// nothing. Every cents field below is `0` and not meaningful when this
    /// is `false`.
    pub available: bool,
    pub current_balance_cents: i64,
    /// Future-dated EUR income already logged (`FinanceEntry`) plus pending
    /// ticket-business sales (`sales.payment_status = 'pending'`, the exact
    /// same "right-now fact" concept dashboard.rs's own
    /// `pending_sales_amount_cents` already uses) - never an unknown/
    /// estimated future sale.
    pub expected_income_cents: i64,
    /// Active recurring templates whose `next_date` falls inside the
    /// forecast window.
    pub recurring_expenses_cents: i64,
    /// Future-dated one-off EUR expense `FinanceEntry` rows inside the
    /// forecast window.
    pub upcoming_expenses_cents: i64,
    /// `current_balance_cents + expected_income_cents - recurring_expenses_cents - upcoming_expenses_cents`.
    pub forecast_balance_cents: i64,
    /// The forecast window, in days, `expected_income`/`recurring_expenses`/
    /// `upcoming_expenses` above were computed over - see
    /// finance_forecast.rs's module doc comment for why 30.
    pub window_days: i64,
    /// `true` when non-EUR pending sales and/or non-EUR future
    /// `FinanceEntry` rows exist and were deliberately excluded (never
    /// guessed at with an invented FX rate) - the frontend shows a small
    /// disclosure note when this is `true`.
    pub excludes_non_eur_data: bool,
}

/// Result of `commands::finance_recurring::create_from_recurring` - the one
/// command that touches two tables at once (see that module's own doc
/// comment for why it runs inside an explicit transaction), so its result
/// carries both the newly-created entry and the template's own new
/// `next_date` in one response rather than making the frontend re-fetch
/// either.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CreateFromRecurringResult {
    pub recurring: RecurringExpense,
    pub entry: FinanceEntry,
}

// --- Ticket Listings (2.2.4) ------------------------------------------------
// See commands::ticket_listings's own module doc comment and migrations/
// 022_ticket_listings.sql for the full design. One ticket can now have many
// of these (one per marketplace it's listed on), which is exactly why this
// isn't just more columns on `Ticket` - marko's own explicit instruction.

/// One (ticket, marketplace) listing - a ticket can have several of these at
/// once, one per marketplace it's currently posted on.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TicketListing {
    pub id: i64,
    pub ticket_id: i64,
    /// Denormalized from a JOIN onto `tickets`, same "id + display fields
    /// alongside it" convention as `FinanceEntry::order_code`/
    /// `Ticket::event_name` - lets the Event Workspace Listings tab render a
    /// row without a second fetch back to `tickets`.
    pub ticket_code: String,
    pub ticket_section: Option<String>,
    pub ticket_row_label: Option<String>,
    pub ticket_seat: Option<String>,
    pub marketplace_id: i64,
    /// Denormalized from a JOIN onto `marketplaces`, same convention as
    /// `FinanceEntry::category_name`/`account_name`.
    pub marketplace_name: String,
    /// The marketplace's OWN id for this listing, if marko has entered one -
    /// optional, since this is manual entry and he may not always have/type
    /// one in. Part of the no-duplicates guard - see the migration's own
    /// doc comment.
    pub listing_id: Option<String>,
    pub listing_url: Option<String>,
    pub price_cents: i64,
    pub currency: String,
    /// 'active' | 'sold' | 'removed' - the listing's OWN lifecycle,
    /// deliberately separate from `Ticket.status` (see the migration's own
    /// doc comment for why the two must never be conflated).
    pub status: String,
    pub is_demo: bool,
    pub created_at: String,
    /// Doubles as "last checked" for now (marko's own "updated_at / last
    /// checked" - one field, not two, until this app ever automates
    /// re-checking a listing's live price, which is explicitly out of scope
    /// this release).
    pub updated_at: String,
}

/// Input for both `create_ticket_listing` and `update_ticket_listing` - same
/// "one struct, not flat arguments" convention as `FinanceEntryInput`/
/// `OrderInput`. An edit always resubmits the same `ticket_id` it started
/// with (the UI never offers to re-parent a listing to a different ticket) -
/// same "round-trip a field the form doesn't expose" spirit as
/// `FinanceEntryInput.order_id`.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TicketListingInput {
    pub ticket_id: i64,
    pub marketplace_id: i64,
    pub listing_id: Option<String>,
    pub listing_url: Option<String>,
    pub price_cents: i64,
    pub currency: String,
    pub status: String,
}

// --- Ticket Listings bulk actions (2.2.5) -----------------------------------
// marko's follow-up: the Listings tab's new multi-select table needs "edit
// status"/"edit price"/"delete" across many selected listings at once - see
// commands/ticket_listings.rs's own doc comment for the all-or-nothing
// transaction design these two inputs feed. Bulk delete needs no dedicated
// input struct - `bulk_delete_ticket_listings` takes a plain `Vec<i64>`,
// same as the existing `bulk_delete_sale_groups`.

/// Input for `bulk_update_ticket_listings_status`: set many listings'
/// `status` (active/sold/removed) in one all-or-nothing transaction. A plain
/// `String` rather than a closed enum - same reasoning as
/// `BulkTicketStatusInput` above: the safety guarantee here is "which values
/// are allowed", enforced by `bulk_update_ticket_listings_status_impl`'s own
/// validation, not "which column can be written".
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BulkTicketListingsStatusInput {
    pub ids: Vec<i64>,
    pub status: String,
}

/// Input for `bulk_update_ticket_listings_price`: set many listings'
/// `price_cents` to the same amount in one all-or-nothing transaction.
/// Currency is never part of this input - it's never written by this
/// action, and the impl rejects the whole batch if the selected listings
/// don't already agree on one (see that function's own doc comment).
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BulkTicketListingsPriceInput {
    pub ids: Vec<i64>,
    pub price_cents: i64,
}

// --- Inventory Intelligence (2.2.6) -----------------------------------------
// marko's request: a compact "Inventory Intelligence" block on the Event
// Workspace's Overview tab, above the existing Orders/Tickets tables - KPIs,
// an aging breakdown, an attention list, and breakdowns by section/
// marketplace. See commands::inventory_intelligence's own module doc comment
// for the full design and exactly which existing computations each field
// reuses (finance::compute_summary's definitions, ListingsTab's own "Listed
// value" definition, SalesTab's own "Potential Profit" definition,
// commands::price_checker::get_price_checker_summary_impl for the market
// comparison) rather than a second, competing implementation of any of them.
//
// Every clickable grouping below carries its own `ticketIds` - the frontend
// filters the Overview tab's ALREADY-FETCHED `tickets` list by id membership
// to show "just these tickets", rather than re-deriving any of these same
// predicates a second time in TypeScript.

/// The 6 headline numbers marko asked for. Three different scopes are in
/// play here (all tickets / unsold tickets only / active listings only), so
/// - same rule as every other money aggregate in this app - each carries its
/// OWN currency flag rather than assuming they all share one.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InventoryIntelligenceKpis {
    /// Every ticket ever purchased for this event, regardless of status -
    /// the exact same count as `EventWithStats.stats.purchasedTickets`
    /// (`FinanceSummary`), computed independently here from the same
    /// underlying rows (not a second query joined back to that struct) but
    /// mathematically identical by construction - see this module's tests.
    pub total_tickets: i64,
    /// Sum of `total_cost_cents` across ALL tickets (same scope as
    /// `total_tickets` above) - identical figure to the existing Overview
    /// "Total cost" stat card, just re-surfaced here under the name marko
    /// asked for.
    pub total_invested_cents: i64,
    pub currency: Option<String>,
    /// Sum of `price_cents` across this event's ACTIVE `ticket_listings`
    /// rows only - the exact same definition ListingsTab's own "Listed
    /// value" summary card already uses (see EventDetail.tsx), computed
    /// here independently since that one has only ever existed client-side.
    pub current_listed_value_cents: i64,
    pub current_listed_value_currency: Option<String>,
    /// Unsold tickets' (available+listed) listing value minus their cost -
    /// byte-for-byte the same formula SalesTab's existing "Potential Profit"
    /// card already computes from `tickets.listingPriceCents` (the LEGACY
    /// single-price field, not `ticket_listings`) - reused here as-is so the
    /// two numbers can never disagree. See this struct's own field above for
    /// why "current listed value" and this card's own internal listing
    /// value are legitimately two different numbers (real per-marketplace
    /// listings vs. the older single-price field) - flagged in
    /// PROTECTED_AREAS.md.
    pub potential_profit_cents: i64,
    pub potential_profit_currency: Option<String>,
    /// sold / total (both counting ALL tickets, cancelled included) - `None`
    /// only when there are no tickets at all. A judgment call: could instead
    /// exclude cancelled tickets from the denominator (arguably more
    /// "correct" as a sell-through definition) but this keeps it consistent
    /// with `total_tickets` above, shown right next to it - see
    /// PROTECTED_AREAS.md.
    pub sell_through_pct: Option<f64>,
    /// total_invested_cents / total_tickets, rounded to the nearest cent -
    /// `None` only when there are no tickets at all. Same
    /// safe_ratio(...).round() idiom already used for `avg_listing_price_
    /// cents`/`my_avg_listing_price_cents` elsewhere in this codebase.
    pub average_ticket_cost_cents: Option<i64>,
}

/// One "days since purchased, still unsold" bucket. Always exactly 4 of
/// these, in a fixed order (0-7 / 8-30 / 31-60 / 61+) - marko's own list,
/// with the 8-30/30-60 overlap at day 30 resolved to 31-60 for the second
/// bucket so every unsold ticket lands in exactly one bucket, never two.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AgingBucket {
    pub key: String,
    pub label: String,
    pub ticket_count: i64,
    pub ticket_ids: Vec<i64>,
}

/// One attention row. Always exactly 4 of these, in a fixed order (event
/// soon / missing listing price / no active listing / outside market
/// price) - present even when `count` is 0, so the block can honestly show
/// "all clear" rather than omitting a row silently.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AttentionItem {
    /// One of: "event_soon", "missing_listing_price", "no_active_listing",
    /// "outside_market_price" - the frontend owns the actual display copy
    /// for each (same "backend sends a stable key, frontend owns the
    /// label" split already used for e.g. `BulkTicketField`).
    pub key: String,
    pub count: i64,
    pub ticket_ids: Vec<i64>,
    /// `false` ONLY for "outside_market_price" when this event has no
    /// Price Checker / Market Analysis data yet - marko's own explicit "iba
    /// ak uz existuju data" (only if that data already exists). `count`/
    /// `ticketIds` are always empty when this is `false`; the frontend
    /// shows this row as "not available yet", never as a misleading "0
    /// problems". Always `true` for the other 3 keys.
    pub available: bool,
}

/// One row of a breakdown (by section, or by marketplace). The same shape
/// serves both - `totalCents` means "total cost" for the section breakdown
/// and "total active listing value" for the marketplace breakdown; see
/// commands::inventory_intelligence for which is which.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InventoryBreakdownGroup {
    pub label: String,
    pub ticket_count: i64,
    pub ticket_ids: Vec<i64>,
    pub total_cents: i64,
    pub currency: Option<String>,
}

/// Everything the Event Workspace Overview's "Inventory Intelligence" block
/// needs, in one round trip. See commands::inventory_intelligence's module
/// doc comment for the full design.
///
/// 2.2.7: now includes a "by tier" breakdown - `tickets.tier` (migration
/// 024) fixed the gap this struct's own doc comment used to describe here
/// (no tier/level column anywhere in this schema). Same shape/clickability
/// as `breakdown_by_section`, grouping unsold tickets by their real, already-
/// stored `tier` value (blank/NULL -> "Unknown", per marko's own explicit
/// instruction - deliberately a DIFFERENT label from section's own "No
/// section", since he asked for "Unknown" specifically here). No fallback/
/// invented data either way: before 2.2.7 the field didn't exist and this
/// breakdown was omitted entirely (a plain-text UI note said so); now it
/// exists and groups by whatever marko has actually entered - a ticket
/// nobody has tiered yet lands in "Unknown", it does not disappear or get
/// guessed at.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InventoryIntelligence {
    pub kpis: InventoryIntelligenceKpis,
    pub aging: Vec<AgingBucket>,
    pub attention: Vec<AttentionItem>,
    pub breakdown_by_tier: Vec<InventoryBreakdownGroup>,
    pub breakdown_by_section: Vec<InventoryBreakdownGroup>,
    pub breakdown_by_marketplace: Vec<InventoryBreakdownGroup>,
    /// Every unsold (available+listed) ticket id - the click target for
    /// "Potential profit" and for "event soon" when it's non-zero, so both
    /// share one definition of "unsold" rather than two ad hoc filters.
    pub unsold_ticket_ids: Vec<i64>,
    /// Every sold ticket id - the click target for "Sell-through %".
    pub sold_ticket_ids: Vec<i64>,
}

/// One row in the Dashboard's global "Attention Center" (2.2.8) - see
/// `commands::attention_center`'s module doc comment for the full design.
/// Unlike `AttentionItem` above (per-event, always exactly 4 rows even at
/// count 0), this is a flat list of INDIVIDUAL things needing a look across
/// EVERY event - only present when they actually apply. An empty `Vec` from
/// the command means genuinely nothing needs attention right now, not a
/// hidden zero.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AttentionCenterItem {
    /// Stable dedup id: `"{category}:{eventId}"` for the one event-level
    /// category (`event_soon`), `"{category}:{ticketId}"` for the 4
    /// ticket-level ones. Never reused across categories, so the same
    /// ticket can legitimately appear more than once under DIFFERENT
    /// reasons (marko's own explicit allowance - "rôzne dôvody môžu byť
    /// samostatné položky") while never appearing twice under the SAME one.
    pub key: String,
    /// One of: "event_soon", "missing_listing_price", "no_active_listing",
    /// "outside_market_price", "sold_undelivered" - same "backend sends a
    /// stable key, frontend owns the display copy" split `AttentionItem.key`
    /// already uses.
    pub category: String,
    /// One of "critical" / "attention" / "info" - see
    /// `commands::attention_center`'s module doc comment for the exact
    /// mapping (a new judgment call this task makes, marko's spec named the
    /// 3 tiers but not which category goes where) and why.
    pub priority: String,
    pub event_id: i64,
    pub event_name: String,
    pub event_date: Option<String>,
    /// `None` only for "event_soon", which is deliberately aggregated per
    /// EVENT rather than per ticket (one row per soon-event, not one per
    /// unsold ticket on it) - see this module's doc comment for why. Real
    /// for the other 4 categories, which are inherently per-ticket.
    pub ticket_id: Option<i64>,
    pub ticket_code: Option<String>,
    /// Human-readable, backend-owned (unlike `AttentionItem.key`, this one
    /// has no fixed enum of frontend copy to select from - the exact count/
    /// wording varies per row, e.g. "3 unsold tickets - event date
    /// approaching").
    pub reason: String,
    /// A basic supporting value, where one exists and is already real data -
    /// currently only populated for "outside_market_price" (that ticket's
    /// own, already-entered listing price - never a suggested/computed one).
    /// `None` for every other category rather than a fabricated number.
    pub amount_cents: Option<i64>,
    pub currency: Option<String>,
}

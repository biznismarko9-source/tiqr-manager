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

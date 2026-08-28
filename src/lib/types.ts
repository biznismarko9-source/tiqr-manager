export type TicketStatus = "available" | "listed" | "sold" | "cancelled";
export type EventStatus = "upcoming" | "completed" | "cancelled";
export type OrderPaymentStatus = "unpaid" | "partial" | "paid";
export type SalePaymentStatus = "pending" | "paid" | "refunded";

export interface FinanceSummary {
  purchasedTickets: number;
  availableTickets: number;
  listedTickets: number;
  soldTickets: number;
  cancelledTickets: number;
  totalCostCents: number;
  cogsCents: number;
  revenueCents: number;
  sellingFeesCents: number;
  profitCents: number;
  margin: number | null;
  roi: number | null;
  /** Set only when every contributing ticket/sale shares one currency. Null means mixed - do not show the totals above as a single money amount. */
  currency: string | null;
}

export interface EventRecord {
  id: number;
  name: string;
  artistTeam: string | null;
  venue: string | null;
  city: string | null;
  country: string | null;
  eventDate: string | null;
  /** 2.0.27: legacy free-text mirror of categoryId's own name - kept in sync
   * by the backend (see commands::events::resolve_category_name), still used
   * directly by CSV export. Prefer categoryId for anything new. */
  category: string | null;
  categoryId: number | null;
  /** 2.0.27: the resolved category's colorSlot, joined in server-side so
   * EventCategoryBadge.tsx can render without a second fetch - see Event's
   * matching doc comment (models.rs). Null exactly when categoryId is null. */
  categoryColorSlot: number | null;
  status: EventStatus;
  notes: string | null;
  isDemo: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface EventWithStats extends EventRecord {
  stats: FinanceSummary;
}

export interface EventInput {
  name: string;
  artistTeam?: string | null;
  venue?: string | null;
  city?: string | null;
  country?: string | null;
  eventDate?: string | null;
  categoryId?: number | null;
  status?: EventStatus | null;
  notes?: string | null;
}

/** 2.0.27: a managed event category (Settings -> Lookups, "like Platforms").
 * See EventCategoryBadge.tsx for how `colorSlot` becomes an actual color. */
export interface EventCategory {
  id: number;
  name: string;
  colorSlot: number;
  isDemo: boolean;
  createdAt: string;
}

/** 2.0.28: shared result shape for every `bulkDelete*` call (pulls, pulls
 * received, orders, events, sale groups) - mirrors the backend's
 * `BulkDeleteResult`/`BulkDeleteSkip` (models.rs). Deletion is judged per
 * item, not all-or-nothing: `deletedIds` is everything that was actually
 * removed, `skipped` is everything that wasn't, each with a plain-English
 * reason (e.g. "This order has sold tickets and cannot be deleted.") to show
 * the user, not just log silently. See BulkDeleteBar (components/ui.tsx) for
 * the shared selection-mode toolbar this pairs with. */
export interface BulkDeleteResult {
  deletedIds: number[];
  skipped: { id: number; reason: string }[];
}

/** 2.0.63: result of one "Detect categories" run (models.rs' `CategoryDetectionResult`) -
 * the retroactive, one-click sibling of the automatic detection that already runs on every
 * brand-new event a sheet sync creates. Only ever touches events that had no category yet,
 * so running this again is always safe. `aiConfigured` says whether this build even has an
 * Anthropic key embedded - when false, `leftUncategorized` only ever reflects what the free
 * keyword rules couldn't recognize, not a failed AI attempt. */
export interface CategoryDetectionResult {
  checked: number;
  categorizedByRule: number;
  categorizedByAi: number;
  leftUncategorized: number;
  aiConfigured: boolean;
}

/** 2.0.38: one ticket's seat location, as returned in the new `seats` list on
 * OrderRecord/SaleGroup below (mirrors the backend's `SeatEntry`, models.rs).
 * Formatting a whole list of these into the compact "Seats" column string
 * (grouping same section/row, collapsing contiguous seat numbers into a
 * range) is `formatSeatsSummary` in format.ts. */
export interface SeatEntry {
  section: string | null;
  rowLabel: string | null;
  seat: string | null;
}

export interface OrderRecord {
  id: number;
  code: string;
  eventId: number;
  eventName: string;
  /** 2.0.27: the event's category, denormalized here the same way eventName
   * already is - lets the Orders list filter/badge by category without a
   * second round trip. All three are null together when the event has no
   * category set. */
  categoryId: number | null;
  categoryName: string | null;
  categoryColorSlot: number | null;
  supplierId: number | null;
  supplierName: string | null;
  platformId: number | null;
  platformName: string | null;
  purchaseDate: string;
  quantity: number;
  unitPriceCents: number;
  feesCents: number;
  otherCostsCents: number;
  totalCostCents: number;
  currency: string;
  paymentStatus: OrderPaymentStatus;
  notes: string | null;
  isDemo: boolean;
  createdAt: string;
  updatedAt: string;
  soldCount: number;
  availableCount: number;
  listedCount: number;
  cancelledCount: number;
  /** 2.0.66: how many of this order's SOLD tickets (out of soldCount) have
   * deliveryStatus "Delivered" - see the new "Completed" indicator
   * (completionStatus in lib/completion.ts). */
  deliveredCount: number;
  /** 2.0.66: how many of this order's SOLD tickets (out of soldCount) have a
   * current (non-refunded) sale with paymentStatus "paid". */
  paidCount: number;
  /** 2.0.38: every ticket in this order's own seat location (not filtered by
   * ticket status - includes cancelled tickets' seats too, same "true
   * complete count" convention soldCount/availableCount/etc. above follow). */
  seats: SeatEntry[];
}

/** Sales-side rollup for one order (Order Detail's "ORDER SUMMARY"), loaded
 * separately from OrderRecord - only when Order Detail is opened. Realized
 * numbers only: unsold and refunded tickets are excluded, same convention as
 * everywhere else in the app. */
export interface OrderSalesSummary {
  revenueCents: number;
  sellingFeesCents: number;
  cogsCents: number;
  profitCents: number;
  margin: number | null;
  roi: number | null;
}

export interface OrderInput {
  eventId: number;
  supplierId?: number | null;
  platformId?: number | null;
  purchaseDate: string;
  quantity: number;
  unitPriceCents: number;
  feesCents: number;
  otherCostsCents: number;
  currency: string;
  paymentStatus?: OrderPaymentStatus | null;
  notes?: string | null;
  ticketType?: string | null;
  section?: string | null;
  rowLabel?: string | null;
  /** One seat label per generated ticket, in order. Length must equal quantity if provided. */
  seats?: string[] | null;
}

export interface OrderEditInput {
  supplierId?: number | null;
  platformId?: number | null;
  purchaseDate: string;
  currency: string;
  paymentStatus: OrderPaymentStatus;
  notes?: string | null;
}

/** 2.0.67: input for the Orders-list bulk "Mark Delivered/Not delivered"
 * action - `orderIds` are whole orders (from the list's own selection
 * checkboxes, the same ones bulk-delete already uses); the backend resolves
 * each order down to just its SOLD tickets before writing anything. See
 * `bulk_set_orders_delivery_status_impl` (orders.rs) for the exact contract. */
export interface BulkOrdersDeliveryStatusInput {
  orderIds: number[];
  deliveryStatus: "Delivered" | "Not delivered";
}

/** 2.0.67: input for the Orders-list bulk "Mark Paid/Pending" action -
 * `orderIds` are whole orders; the backend resolves each order down to its
 * current (non-refunded) sale per sold ticket before writing anything, the
 * same way `bulkUpdateSalePaymentStatus` already restricts to pending/paid
 * only. See `bulk_set_orders_payment_status_impl` (orders.rs). */
export interface BulkOrdersPaymentStatusInput {
  orderIds: number[];
  paymentStatus: "pending" | "paid";
}

export interface Ticket {
  id: number;
  code: string;
  eventId: number;
  eventName: string;
  orderId: number;
  orderCode: string;
  section: string | null;
  rowLabel: string | null;
  seat: string | null;
  ticketType: string | null;
  purchaseCostCents: number;
  purchaseFeesCents: number;
  otherCostsCents: number;
  totalCostCents: number;
  listingPriceCents: number | null;
  currency: string;
  status: TicketStatus;
  /** 2.0.10: marko's own free-text "Status"/"Delivery status" tracking from
   * his sheet (Sales sync) - distinct from `status` above, which the app
   * manages automatically. See migration 010's doc comment. */
  resaleStatus: string | null;
  deliveryStatus: string | null;
  notes: string | null;
  isDemo: boolean;
  createdAt: string;
  updatedAt: string;
  salePriceCents: number | null;
  /** 2.0.68: the ticket's ACTIVE (non-refunded) sale's paymentStatus, via the
   * same join salePriceCents already comes from - null for a never-sold
   * ticket or one whose only sale was refunded, same cases where
   * salePriceCents is already null. Powers Order Detail's Payout status
   * column without needing a separate Sale[] fetch. */
  salePaymentStatus: SalePaymentStatus | null;
  /** 2.0.69: the same active sale's own id - null in the same cases
   * salePaymentStatus is. Lets Order Detail's inline Payout-status edit call
   * `bulkUpdateSalePaymentStatus` (sale-id-based) directly. */
  saleId: number | null;
}

export interface TicketUpdateInput {
  section?: string | null;
  rowLabel?: string | null;
  seat?: string | null;
  ticketType?: string | null;
  listingPriceCents?: number | null;
  status?: TicketStatus | null;
  resaleStatus?: string | null;
  deliveryStatus?: string | null;
  notes?: string | null;
}

/** Closed set of ticket fields `bulkUpdateTickets` can change - deliberately
 * excludes status. See `bulk_update_tickets_impl` (tickets.rs) and
 * `BulkTicketEditBar.tsx` for why.
 *
 * 1.9.1: "ticketType" was removed from this set - it's now a one-time choice
 * made when creating the order instead (see `OrderInput.ticketType`), still
 * editable per-ticket afterwards via `TicketUpdateInput` if needed. */
export type BulkTicketField = "section" | "rowLabel" | "seat" | "listingPriceCents";

/** Input for `bulkUpdateTickets` (1.8.3) - set one field to one value across
 * many tickets in a single all-or-nothing transaction. `textValue` is used
 * for section/rowLabel/seat; `centsValue` is used for listingPriceCents. */
export interface BulkTicketUpdateInput {
  ticketIds: number[];
  field: BulkTicketField;
  textValue?: string | null;
  centsValue?: number | null;
}

/** Input for `bulkUpdateTicketStatus` (1.9.3): set many tickets' `status` in
 * one all-or-nothing transaction. Only "available" | "listed" | "cancelled"
 * are accepted - "sold" is deliberately unreachable here, both as a target
 * and as a starting point being moved away from, since it must always
 * correspond to an active sale. See `bulk_update_ticket_status_impl`
 * (tickets.rs) for the full reasoning. Replaces `BulkTicketUpdateInput` on
 * Order Detail, which now only offers this narrow status action. */
export interface BulkTicketStatusInput {
  ticketIds: number[];
  status: "available" | "listed" | "cancelled";
}

/** Input for `bulkUpdateTicketDeliveryStatus` (2.0.69) - a direct, raw-
 * ticket-id endpoint around the same write `bulkSetOrdersDeliveryStatus`/
 * `bulkSetSaleGroupsDeliveryStatus` already use internally, for Sale/Order
 * Detail's inline Delivery status edit (one row = one already-known ticket
 * id, nothing to resolve from an order/sale-group selection). `ticketIds`
 * still takes an array so a future bulk selection could reuse this same
 * endpoint - today's callers always pass exactly one id. */
export interface BulkTicketDeliveryStatusInput {
  ticketIds: number[];
  deliveryStatus: "Delivered" | "Not delivered";
}

/** Input for `bulkUpdateTicketResaleStatus` (2.0.69) - same shape/reasoning
 * as `BulkTicketDeliveryStatusInput` above, for marko's own manual Listed/
 * Unlisted/Sold tracking. Powers Sale Detail's inline Status edit. */
export interface BulkTicketResaleStatusInput {
  ticketIds: number[];
  resaleStatus: "Listed" | "Unlisted" | "Sold";
}

export interface Sale {
  id: number;
  code: string;
  ticketId: number;
  ticketCode: string;
  section: string | null;
  rowLabel: string | null;
  seat: string | null;
  /** 2.0.66: the ticket's own current status - almost always "sold" (a Sale
   * only exists for a sold ticket), EXCEPT after a refund, which reverts the
   * ticket to "available" while this historical Sale row stays as-is. Powers
   * the new "Completed" indicator's per-line breakdown on Sale Detail. */
  ticketStatus: TicketStatus;
  /** 2.0.66: the ticket's own deliveryStatus (see Ticket.deliveryStatus). */
  ticketDeliveryStatus: string | null;
  /** 2.0.68: the ticket's own manual resaleStatus (Listed/Unlisted/Sold -
   * see Ticket.resaleStatus). Deliberately separate from ticketStatus above:
   * that's the real system-managed enum, this is marko's own free-text sheet
   * mirror - Sale Detail shows both as distinct badges. */
  ticketResaleStatus: string | null;
  eventId: number;
  eventName: string;
  /** The ticket's own order - every ticket belongs to exactly one order, so
   * this is never null/Mixed (unlike SaleGroup's fields below, which CAN be
   * Mixed once several lines are aggregated). Powers Sale Detail's
   * Ticket -> Order Detail link (1.8.0). */
  orderId: number;
  orderCode: string;
  platformId: number | null;
  platformName: string | null;
  saleDate: string;
  salePriceCents: number;
  sellingFeesCents: number;
  currency: string;
  /** 2.0.57: true when this sale's own currency differs from its own
   * ticket's purchase currency (only possible once a New Sale explicitly
   * overrides the currency - see SaleBatchInput.currency). When true,
   * `margin`/`roi` are already `null`, and `costCents`/`profitCents` should
   * be shown as "Mixed" (e.g. via formatMoneyOrMixed) rather than a real
   * number - they'd otherwise silently subtract two different currencies. */
  currencyMismatch: boolean;
  paymentStatus: SalePaymentStatus;
  buyerReference: string | null;
  notes: string | null;
  isDemo: boolean;
  createdAt: string;
  updatedAt: string;
  costCents: number;
  profitCents: number;
  margin: number | null;
  roi: number | null;
  refundedAt: string | null;
  refundReason: string | null;
  /** NULL for an ordinary single-ticket sale; shared by every row submitted
   * together in one multi-ticket "New sale" action. Grouping only - never
   * changes what one `sales` row means. */
  batchId: string | null;
}

/** One row in the Sales screen's main (grouped) list - everything submitted
 * together in one sale action (a single ticket, or a multi-ticket batch)
 * collapsed into a summary row. `id`/`code` are the group's representative
 * sale, used to open Sale Detail (which loads the individual lines). */
export interface SaleGroup {
  id: number;
  code: string;
  batchId: string | null;
  ticketCount: number;
  /** Null means the group's tickets span more than one event ("Mixed events"). */
  eventId: number | null;
  eventName: string | null;
  /** 2.0.27: the group's shared event category - same "Some only when every
   * line's event agrees" rule as eventId/eventName above (null on a "Mixed
   * events" group, and also null when the one shared event has no category). */
  categoryId: number | null;
  categoryName: string | null;
  categoryColorSlot: number | null;
  saleDate: string;
  platformId: number | null;
  platformName: string | null;
  /** Null means the group's lines aren't all one currency - show "Mixed", never a blended amount. */
  currency: string | null;
  /** Revenue/fees/cost/profit exclude refunded lines - never "realized". */
  revenueCents: number;
  sellingFeesCents: number;
  costCents: number;
  profitCents: number;
  margin: number | null;
  roi: number | null;
  /** Null means the lines don't all share one payment status (e.g. one of
   * several tickets was refunded later) - show "Mixed", not a single badge. */
  paymentStatus: SalePaymentStatus | null;
  refundedCount: number;
  /** 2.0.66: how many of this group's ticketCount tickets currently have
   * status "sold". Normally equals ticketCount - lower only when a line was
   * refunded (its ticket reverts to "available"), the same case
   * refundedCount above already flags. Feeds the new "Completed" indicator
   * (completionStatus in lib/completion.ts). */
  soldCount: number;
  /** 2.0.66: how many of this group's ticketCount tickets have
   * deliveryStatus "Delivered". */
  deliveredCount: number;
  /** 2.0.66: how many of this group's OWN sale lines have paymentStatus
   * "paid" (each line's own status - paymentStatus above collapses to null
   * the moment lines disagree, which is too coarse for "3 of 5 paid"). */
  paidCount: number;
  isDemo: boolean;
  /** 2.0.38: the seat location of every ticket THIS SALE GROUP actually sold
   * (not filtered by refund status - a refunded line is still one of the
   * tickets this group covers, same convention ticketCount above follows). */
  seats: SeatEntry[];
}

export interface SaleInput {
  ticketId: number;
  platformId?: number | null;
  saleDate: string;
  salePriceCents: number;
  sellingFeesCents: number;
  paymentStatus?: SalePaymentStatus | null;
  buyerReference?: string | null;
  notes?: string | null;
}

export interface SaleBatchLineInput {
  ticketId: number;
  salePriceCents: number;
  sellingFeesCents: number;
}

export interface SaleBatchInput {
  lines: SaleBatchLineInput[];
  platformId?: number | null;
  saleDate: string;
  paymentStatus?: SalePaymentStatus | null;
  buyerReference?: string | null;
  notes?: string | null;
  /** 2.0.57: the one currency this whole sale is recorded in, picked in the
   * New Sale form - independent of whatever currency the ticket(s) being
   * sold were themselves bought in. Backend accepts this as optional
   * (older/other callers like Sheets sync omit it and keep the original
   * per-ticket-derived behaviour), but the New Sale UI always sends it. */
  currency: string;
}

export interface SaleEditInput {
  platformId?: number | null;
  saleDate: string;
  salePriceCents: number;
  sellingFeesCents: number;
  paymentStatus: SalePaymentStatus;
  buyerReference?: string | null;
  notes?: string | null;
}

/** Input for `bulkUpdateSalePaymentStatus` (1.9.2) - sets many sales'
 * paymentStatus to "pending" or "paid" in one all-or-nothing transaction.
 * Powers Sale Detail's small "Mark as Paid" / "Mark as Pending" action,
 * which replaced the old general Bulk Ticket Edit bar there. Refunding stays
 * its own dedicated action (`refundSale`) - this never accepts "refunded" as
 * a target, and a batch containing an already-refunded sale is rejected
 * entirely rather than applied to the rest. See `bulk_update_sale_payment_
 * status_impl` (sales.rs) for the exact contract. */
export interface BulkSalePaymentStatusInput {
  saleIds: number[];
  paymentStatus: "pending" | "paid";
}

/** 2.0.67: input for the Sales-list bulk "Mark Delivered/Not delivered"
 * action - `groupIds` are the same SaleGroup anchor ids the list's own
 * selection checkboxes (and bulk-delete) already use. The backend expands
 * each group to every ticket across all its lines, INCLUDING a refunded
 * line's ticket (delivery status is a ticket-level fact, independent of
 * refund). See `bulk_set_sale_groups_delivery_status_impl` (sales.rs). */
export interface BulkSaleGroupsDeliveryStatusInput {
  groupIds: number[];
  deliveryStatus: "Delivered" | "Not delivered";
}

/** 2.0.67: input for the Sales-list bulk "Mark Paid/Pending" action - same
 * `groupIds` selection as `BulkSaleGroupsDeliveryStatusInput` above, but the
 * backend excludes any already-refunded line before writing, so a group with
 * one refunded line still gets its other lines marked paid. See
 * `bulk_set_sale_groups_payment_status_impl` (sales.rs). */
export interface BulkSaleGroupsPaymentStatusInput {
  groupIds: number[];
  paymentStatus: "pending" | "paid";
}

/** 1.9.3: "purchase" platforms populate Order forms, "sale" platforms
 * populate Sale forms, "both" populates either. Was already the schema's
 * design from the very first migration - this round just started exposing
 * it in the UI (Settings -> Lookups' split Purchase/Selling lists). */
export interface Platform {
  id: number;
  name: string;
  kind: "purchase" | "sale" | "both";
  isDemo: boolean;
  createdAt: string;
}

export interface Supplier {
  id: number;
  name: string;
  contact: string | null;
  isDemo: boolean;
  createdAt: string;
}

/** Dashboard "Inventory & Potential Profit" block - deliberately separate
 * from the realized `inventory`/`period` FinanceSummary blocks above. Scope
 * is tickets currently `available` or `listed` (not yet sold, not
 * cancelled) - a "current state" snapshot, not affected by the Dashboard's
 * period filter. Never call/show `potentialProfitCents` as if it were
 * realized profit. */
export interface InventoryPotential {
  /** Purchase cost (+fees +other costs) of every ticket still unsold. */
  inventoryCostCents: number;
  /** Sum of listing prices for unsold tickets that HAVE a listing price set
   * (tickets missing one contribute 0 here - see alerts.missingListingPriceCount). */
  listingValueCents: number;
  /** listingValueCents - inventoryCostCents. Potential, not realized. */
  potentialProfitCents: number;
  /** Null means the current available/listed tickets aren't all one
   * currency (or the dashboard overall is mixed) - show "Mixed", never a
   * blended amount. Same convention as FinanceSummary.currency. */
  currency: string | null;
}

/** One row in the Dashboard's "Upcoming Events" alert. */
export interface UpcomingEventAlert {
  id: number;
  name: string;
  eventDate: string;
  /** Tickets still available or listed for this event. */
  relevantInventory: number;
}

/** Dashboard "Attention" section - simple, transparent counts. Never
 * period-filtered (these are "right now" facts). */
export interface DashboardAlerts {
  /** Orders with payment_status 'unpaid' or 'partial' (purchase side - money owed to a supplier). */
  unpaidOrdersCount: number;
  /** Available/listed tickets with no listing price set. Ticket-scoped -
   * feeds the Overview "Potential Profit" sentence. For the Activity tab's
   * own alert card, see missingListingPriceOrdersCount below. */
  missingListingPriceCount: number;
  /** 2.0.48: same unpriced tickets as missingListingPriceCount, but counting
   * each order once no matter how many of its tickets are unpriced - what
   * the Activity tab's "Missing listing price" card actually shows. */
  missingListingPriceOrdersCount: number;
  /** Total 'upcoming' events within the alert window that still have available/listed inventory. */
  upcomingEventsCount: number;
  /** The soonest of the above, capped (same convention as Recent Events/Orders/Sales). */
  upcomingEvents: UpcomingEventAlert[];
  /** 1.8.3: sales with payment_status 'pending' - money not yet collected
   * from the buyer (sales-side mirror of unpaidOrdersCount). Not period-filtered. */
  pendingSalesCount: number;
  /** Sum of salePriceCents for the sales counted in pendingSalesCount, scoped to primaryCurrency. */
  pendingSalesAmountCents: number;
  /** Null means mixed currencies - same convention as InventoryPotential.currency. */
  pendingSalesCurrency: string | null;
}

/** Dashboard "Cashflow" section (1.9.0) - what was sold vs. what has
 * actually been collected from buyers vs. what they still owe, built
 * entirely from the existing sales payment status. Not period-filtered -
 * same "right now" convention as DashboardAlerts. `revenueCents` always
 * equals `paidCents + outstandingCents` for the same currency scope (every
 * non-refunded sale is exactly one or the other); refunded sales are
 * excluded from all four fields. */
export interface CashflowSummary {
  /** Same figure as DashboardData.inventory.revenueCents, repeated here so this section is self-contained. */
  revenueCents: number;
  /** Same figure as DashboardData.inventory.profitCents. */
  profitCents: number;
  /** Money actually collected from buyers (sales with paymentStatus 'paid'). */
  paidCents: number;
  /** Money sold but not yet collected (sales with paymentStatus 'pending') - same value as alerts.pendingSalesAmountCents. */
  outstandingCents: number;
  /** Null means mixed currencies - same convention as every other money block here. */
  currency: string | null;
}

/** One bucket of the Dashboard revenue/profit-over-time chart (1.6.0). Same
 * scope and realized-only/refund-excluded rule as DashboardData.period -
 * just broken out by date instead of collapsed into one total. */
export interface RevenueTimeSeriesPoint {
  /** The earliest real sale date in this bucket - always a concrete
   * calendar date, never one of the backend's "no bound" sentinel dates. */
  bucketStart: string;
  revenueCents: number;
  sellingFeesCents: number;
  cogsCents: number;
  /** COUNT(*) of (non-refunded) sales lines in this bucket - i.e. tickets
   * sold. Same definition as the "Tickets sold" StatCard, just broken out
   * per bucket - powers the Dashboard chart's "Sales" metric (1.7.5). */
  soldTickets: number;
  profitCents: number;
}

/** Which Dashboard tab is active (1.9.3) - replaces the 1.9.2 "Customize"
 * show/hide-per-section panel (that `DashboardWidgets` type is gone; nothing
 * on the dashboard can be hidden any more, only navigated to). Persisted
 * client-side only, via the existing generic app-settings key/value
 * mechanism (getAppSetting/setAppSetting) under the key "dashboardTab" as a
 * plain string - no new backend command, no migration. See Dashboard.tsx's
 * useDashboardTab for the load/persist logic. */
export type DashboardTab = "overview" | "financials" | "activity";

/** One row of the Dashboard's "Sales by platform" widget (2.0.47, DIR-001 -
 * see REDESIGN-2.0.47-REPORT.md). Same period/currency/event/platform scope
 * as `DashboardData.period`, just grouped by platform instead of collapsed
 * into one total. Mirrors src-tauri/src/models.rs's `PlatformSales`. */
export interface PlatformSales {
  platformId: number | null;
  /** null only when platformId is null (a sale with no platform set) - shown as "No platform". */
  platformName: string | null;
  soldTickets: number;
  revenueCents: number;
  profitCents: number;
}

export interface DashboardData {
  inventory: FinanceSummary;
  period: FinanceSummary;
  /** The equal-length window immediately preceding periodFrom..periodTo,
   * used for the Dashboard KPI cards' "vs previous period" trend (2.0.47,
   * DIR-001 - see computeTrend/computeTrendPoints in lib/format.ts). null
   * when there's no sensible previous period ("All time", or a Custom range
   * with no explicit start) - see previous_period_bounds in dashboard.rs. */
  previousPeriod: FinanceSummary | null;
  periodFrom: string;
  periodTo: string;
  recentOrders: OrderRecord[];
  /** 2.0.54: SaleGroup (one row per sale action - a single ticket, or a
   * multi-ticket batch), not Sale (one row per ticket) - a 4-ticket batch
   * used to show as 4 identical entries here. */
  recentSales: SaleGroup[];
  recentEvents: EventWithStats[];
  /** The single currency every total on this dashboard is computed in. */
  primaryCurrency: string;
  /** True when the database also has data in other currencies, excluded from the totals above. */
  mixedCurrencies: boolean;
  /** 2.0.51: every non-EUR currency actually present on an order, with its
   * order count - see CurrencyOrderCount. Empty whenever every order is
   * already EUR (including whenever mixedCurrencies above is false). Powers
   * the mixed-currency banner's per-currency/"Convert all" bulk action. */
  nonEurOrderCurrencies: CurrencyOrderCount[];
  /** Inventory Cost / Listing Value / Potential Profit - see InventoryPotential. */
  inventoryPotential: InventoryPotential;
  /** Attention/alerts - see DashboardAlerts. */
  alerts: DashboardAlerts;
  /** Cashflow snapshot (1.9.0) - see CashflowSummary. */
  cashflow: CashflowSummary;
  /** Revenue/profit chart data - see RevenueTimeSeriesPoint. Same period/
   * currency/event/platform scope as `period` above. */
  revenueTimeSeries: RevenueTimeSeriesPoint[];
  /** "day" | "week" | "month" - the bucket width revenueTimeSeries used. */
  timeSeriesGranularity: string;
  /** "Sales by platform" widget (2.0.47) - see PlatformSales. Ordered by revenueCents descending. */
  salesByPlatform: PlatformSales[];
}

export interface CsvPreviewRow {
  rowNumber: number;
  values: Record<string, string>;
  errors: string[];
}
export interface CsvPreview {
  headers: string[];
  rows: CsvPreviewRow[];
  validCount: number;
  errorCount: number;
}
export interface CsvImportResult {
  importedOrders: number;
  importedTickets: number;
  errors: string[];
}

export interface AppInfo {
  version: string;
  dbPath: string;
}

/** 2.0.72: result of switching the live database connection to a specific
 * account's file - see api.ts's `switchActiveDatabase` and
 * src-tauri/src/commands/database.rs's own doc comment for the full design. */
export interface DatabaseSwitchOutcome {
  dbPath: string;
  /** True the first time this exact file is ever switched to - lets the
   * frontend know for certain it's showing a brand-new, empty workspace
   * rather than inferring that from an empty ticket list, which would also
   * be (mis)read as "the switch silently failed." */
  isNew: boolean;
}

export interface RestoreOutcome {
  safetyBackupPath: string;
}

// ---------------------------------------------------------------------------
// Pull (1.9.7) - buying tickets on someone else's behalf for a fee. See
// src-tauri/migrations/005_pulls.sql for the full feature rationale.
// Deliberately not linked to Event/Order/Ticket/Sale - see that file's
// comment for why. 1.9.8 (src-tauri/migrations/006_pulls_seat_fields.sql)
// reshaped the old single `seats` free-text field into `section`/`rowLabel`/
// `seat` (same shape as `Ticket`'s own fields - see formatSeatLocation in
// format.ts), and dropped the manual transfer-deadline input in favor of an
// automatic warning computed client-side from `eventDate` (see Pulls.tsx).
// ---------------------------------------------------------------------------

export interface Pull {
  id: number;
  code: string;
  buyerName: string;
  eventName: string;
  eventDate: string | null;
  quantity: number;
  platformId: number | null;
  platformName: string | null;
  section: string | null;
  rowLabel: string | null;
  seat: string | null;
  moreInfo: string | null;
  /** marko's own fee for doing the pull - never the ticket price itself (paid by the other person's card, not marko's money). */
  priceCents: number;
  currency: string;
  /** 1.9.8: no longer settable from the UI - replaced by an automatic
   * "N days before the event" warning computed from `eventDate` (see
   * Pulls.tsx). Kept readable here only in case an older pull still has one. */
  transferDeadline: string | null;
  transferDone: boolean;
  transferDoneAt: string | null;
  isDemo: boolean;
  createdAt: string;
  updatedAt: string;
}

/** Input for `createPull`. `transferDone`/`transferDoneAt` deliberately
 * aren't here - a brand new pull always starts not-transferred; use
 * `setPullTransferDone` (or edit it afterwards) once it's actually done.
 * No `transferDeadline` either as of 1.9.8 - see `Pull`'s comment above. */
export interface PullInput {
  buyerName: string;
  eventName: string;
  eventDate?: string | null;
  quantity: number;
  platformId?: number | null;
  section?: string | null;
  rowLabel?: string | null;
  seat?: string | null;
  moreInfo?: string | null;
  priceCents: number;
  currency: string;
}

/** Input for `updatePull` - the full edit form. Unlike `PullInput`, this DOES
 * include `transferDone` (so a mistaken checkbox click, or backfilling older
 * data, can be corrected here too). */
export interface PullEditInput {
  buyerName: string;
  eventName: string;
  eventDate?: string | null;
  quantity: number;
  platformId?: number | null;
  section?: string | null;
  rowLabel?: string | null;
  seat?: string | null;
  moreInfo?: string | null;
  priceCents: number;
  currency: string;
  transferDone: boolean;
}

// ---------------------------------------------------------------------------
// Pull received (2.0.17) - the mirror direction of Pull above: someone ELSE
// pulls tickets FOR marko (marko pays them a fee) instead of marko pulling
// for someone else. See src-tauri/migrations/011_pulls_received.sql.
// ---------------------------------------------------------------------------

export interface PullReceived {
  id: number;
  code: string;
  pullerName: string;
  eventName: string;
  eventDate: string | null;
  quantity: number;
  /** marko's fee to the puller - informational only, never affects Profit/Revenue anywhere. */
  amountCents: number;
  currency: string;
  moreInfo: string | null;
  /** Which Order these pulled tickets became, if any - null when this row is standalone. */
  orderId: number | null;
  /** The linked order's own code (e.g. "ORD-000042"), for display only - null whenever orderId is null. */
  orderCode: string | null;
  /** "manual" (typed in the app) or "sheet_sync" (auto-created by Orders & Sales sheet sync). */
  source: "manual" | "sheet_sync";
  isDemo: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface PullReceivedInput {
  pullerName: string;
  eventName: string;
  eventDate?: string | null;
  quantity: number;
  amountCents: number;
  currency: string;
  moreInfo?: string | null;
  orderId?: number | null;
}

/** Input for `updatePullReceived` - the full edit form. Deliberately has no
 * `source` field - whether a row started out manual or sheet-sync-created is
 * provenance, not editable from the form. */
export interface PullReceivedEditInput {
  pullerName: string;
  eventName: string;
  eventDate?: string | null;
  quantity: number;
  amountCents: number;
  currency: string;
  moreInfo?: string | null;
  orderId?: number | null;
}

// ---------------------------------------------------------------------------
// Google Sheets sync (Settings -> Integrations, 2.0.2+). `dataSource` is
// always a plain string key ("pulls" today, "tickets" planned next) - see
// models.rs's matching Rust section for why.
// ---------------------------------------------------------------------------

export interface SheetsConnectionConfig {
  spreadsheetId: string;
  sheetTab: string;
  /** 2.0.3: one currency applies to every row synced from this sheet - see
   * models.rs's matching field doc comment for why. One of CURRENCY_OPTIONS. */
  currency: string;
}

export const CURRENCY_OPTIONS = ["EUR", "USD", "GBP"] as const;

export interface SheetsConnectionStatus {
  syncAvailable: boolean;
  serviceAccountEmail?: string | null;
  connection?: SheetsConnectionConfig | null;
  lastSyncedAt?: string | null;
  /** 2.0.18: the "Push to sheet" direction's own separate timestamp - see
   * SheetsConnectionStatus::last_pushed_at's doc comment (models.rs). */
  lastPushedAt?: string | null;
}

/** `message` is deliberately short and glanceable (2.0.15) - for a failure
 * the app recognizes (not shared, wrong tab name) it's a plain headline like
 * "Can't access this spreadsheet yet.", with `hint` carrying the concrete
 * next step (e.g. the exact e-mail to share it with) as its own line. `hint`
 * is `null`/absent both on success and for a failure the app doesn't
 * recognize - there, `message` itself keeps the full underlying detail, see
 * commands/sheets_sync.rs::test_sheets_connection_impl. */
export interface SheetsConnectionTestResult {
  ok: boolean;
  message: string;
  hint?: string | null;
}

/** Result of a best-effort attempt to detect a pasted spreadsheet's real tab
 * names (2.0.14), so Settings can offer them as a dropdown instead of
 * requiring the exact tab name to be typed by hand - see
 * commands/sheets_sync.rs::detect_spreadsheet_tabs_impl's doc comment for
 * why. `tabs` is always empty when `ok` is false; `message` is empty when
 * `ok` is true. Same short-`message`-plus-optional-`hint` split as
 * SheetsConnectionTestResult (2.0.15) and for the same reason. */
export interface SpreadsheetTabsResult {
  ok: boolean;
  tabs: string[];
  message: string;
  hint?: string | null;
}

/** Installation-wide "Sign in with Google" state (2.0.5) - one signed-in
 * account per copy of the app, orthogonal to which spreadsheet is connected
 * for any given data source. See commands/google_auth.rs's module doc
 * comment. */
export interface GoogleSignInStatus {
  /** Whether this build was compiled with an OAuth client embedded - same
   * convention as SheetsConnectionStatus.syncAvailable on the
   * service-account side. */
  signInAvailable: boolean;
  signedInEmail?: string | null;
}

/** 2.0.46: what `startFirebaseGoogleSignIn` hands back - the "Continue with
 * Google" APP sign-in button on the Welcome screen, a completely separate
 * flow from GoogleSignInStatus above (which is only about Sheets access -
 * see commands/firebase_google_auth.rs's module doc comment). Just enough
 * to finish the Firebase side of the sign-in (`lib/auth.tsx`'s
 * `loginWithGoogle`: `GoogleAuthProvider.credential(idToken)` +
 * `signInWithCredential`) - nothing about this flow is persisted anywhere
 * in this app's own database. */
export interface FirebaseGoogleSignInResult {
  idToken: string;
}

/** Result of "Create a new sheet for me" (2.0.4) - the auto-create-and-share
 * alternative to pasting an existing sheet's URL, no Google sign-in window.
 * `connection` is already persisted by the time this returns. `spreadsheetUrl`
 * is shown as selectable text, not a clickable link (no shell-opener
 * dependency in this app - see Settings.tsx). */
export interface CreatedSheetResult {
  connection: SheetsConnectionConfig;
  spreadsheetUrl: string;
}

// ---------------------------------------------------------------------------
// Sheet <-> app row sync (2.0.3, generalized 2.0.8). Sheet -> app only - see
// commands/pulls_sheet_sync.rs's and commands/orders_sheet_sync.rs's module
// doc comments.
// ---------------------------------------------------------------------------

export interface SheetSyncIssue {
  rowNumber: number;
  message: string;
}

/** Result of one "Sync now" run, shown as-is in Settings -> Integrations.
 * Named generically (not e.g. "PullsSyncResult") since 2.0.8 - this shape was
 * never actually specific to Pulls, and commands::orders_sheet_sync reuses it
 * verbatim for Orders/Tickets sync. */
export interface SheetSyncResult {
  created: number;
  updated: number;
  unchanged: number;
  conflicts: SheetSyncIssue[];
  errors: SheetSyncIssue[];
  /** 2.0.42: rows saved successfully after the app auto-corrected a small,
   * sensible pricing gap (Total Purchase Price vs Number of Tickets x Price
   * Per Ticket, or an over-precise Price Per Ticket) - never a row that was
   * skipped. Always empty outside Orders sync. */
  corrected: SheetSyncIssue[];
  syncedAt: string;
}

/** Result of converting a list of cents amounts from one currency to
 * another at today's live rate - see commands::currency::convert_currency
 * (Rust) / fx.rs for the actual Frankfurter API call this comes from.
 * 2.0.50: powers the "Convert to EUR" action on the New Order form.
 * `convertedCents` lines up with the `amountsCents` list sent to
 * `api.convertCurrency` by position (same length, same order) - callers
 * match results back to fields by index, not by name. */
export interface CurrencyConversion {
  rate: number;
  rateDate: string;
  convertedCents: number[];
}

/** One non-EUR currency actually present on an order, with how many orders
 * hold it (2.0.51) - mirrors the backend's `CurrencyOrderCount` (models.rs).
 * Powers the Dashboard mixed-currency banner's per-currency "Convert to EUR"
 * buttons. */
export interface CurrencyOrderCount {
  currency: string;
  orderCount: number;
}

/** Result of converting one EXISTING order's currency to EUR (2.0.51) - see
 * `api.convertOrderCurrency` / commands::orders::convert_order_currency_impl
 * (Rust). Unlike `CurrencyConversion` above (2.0.50's stateless "here's what
 * these numbers would become" preview for the New Order form, nothing saved
 * yet), this always describes a conversion that has ALREADY been committed
 * to the database - `ticketsConverted`/`salesConverted` are real counts of
 * rows actually rewritten, not an estimate. */
export interface OrderCurrencyConversion {
  orderId: number;
  orderCode: string;
  fromCurrency: string;
  toCurrency: string;
  rate: number;
  rateDate: string;
  ticketsConverted: number;
  salesConverted: number;
  /** 2.0.53: true when this order was ever linked to a Google Sheet (Order
   * sync or Order push) - false for the common case (manually entered,
   * CSV-imported, or Sheets never connected), in which case sheetPushError
   * is always null too, since nothing was attempted. */
  linkedToSheet: boolean;
  /** null when linkedToSheet is false, or when it's true and the push to
   * the sheet succeeded. A message means the conversion above is still
   * fully saved - only the follow-up update to the actual Google Sheet row
   * failed, so the sheet itself is out of sync with the app until this is
   * retried or fixed by hand there. */
  sheetPushError: string | null;
}

/** What `api.convertOrderCurrency` returns - the summary above, plus the
 * order's own freshly-refetched record, so Order Detail can update its
 * Currency/cost fields immediately without a second round trip. */
export interface OrderCurrencyConversionResult {
  order: OrderRecord;
  conversion: OrderCurrencyConversion;
}

/** Result of the bulk "Convert to EUR" action on the Dashboard's
 * mixed-currency banner (2.0.51) - every order actually converted, plus any
 * that were skipped and why (reuses the same `{id, reason}` shape as
 * BulkDeleteResult above). */
export interface BulkCurrencyConversionResult {
  converted: OrderCurrencyConversion[];
  skipped: { id: number; reason: string }[];
}

// ---------------------------------------------------------------------------
// Outbound notifications (2.0.76) - desktop, email, Pushover. Settings ->
// Notifications; the periodic check itself runs from Layout.tsx. Mirrors
// src-tauri/src/models.rs's NotificationStatus/NotificationConfigInput/
// NotificationTestResult exactly (serde's rename_all = "camelCase") - see
// that file's own doc comments for the full design (never echoing a secret
// back, Option<String> = "leave unchanged" on the input side, etc).
// ---------------------------------------------------------------------------

/** What Settings -> Notifications loads to show the current state. Secret
 * fields (the SMTP password, the Pushover keys) are never included - only
 * whether one is currently stored (`*Set`). Never pre-fill a secret input
 * from this - see NotificationConfigInput below. */
export interface NotificationStatus {
  desktopEnabled: boolean;
  emailEnabled: boolean;
  emailSmtpHost: string;
  emailSmtpPort: number;
  emailSmtpUsername: string;
  emailSmtpPasswordSet: boolean;
  emailFromAddress: string;
  emailToAddress: string;
  pushoverEnabled: boolean;
  pushoverUserKeySet: boolean;
  pushoverApiTokenSet: boolean;
}

/** What Settings -> Notifications submits to `setNotificationConfig`. Every
 * secret field is optional/nullable: omit it (or send null) to leave
 * whatever is already stored untouched - exactly what a secret input the
 * user left blank means, since it's never pre-filled with a real value to
 * begin with. Send a real string only when the user actually typed a new
 * one. */
export interface NotificationConfigInput {
  desktopEnabled: boolean;
  emailEnabled: boolean;
  emailSmtpHost: string;
  emailSmtpPort: number;
  emailSmtpUsername: string;
  emailSmtpPassword?: string | null;
  emailFromAddress: string;
  emailToAddress: string;
  pushoverEnabled: boolean;
  pushoverUserKey?: string | null;
  pushoverApiToken?: string | null;
}

/** Result of one "Send test" click (one per channel) - unlike the silent
 * periodic check, a test click says plainly whether it worked. */
export interface NotificationTestResult {
  success: boolean;
  message: string;
}

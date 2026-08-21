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
  category: string | null;
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
  category?: string | null;
  status?: EventStatus | null;
  notes?: string | null;
}

export interface OrderRecord {
  id: number;
  code: string;
  eventId: number;
  eventName: string;
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
  notes: string | null;
  isDemo: boolean;
  createdAt: string;
  updatedAt: string;
  salePriceCents: number | null;
}

export interface TicketUpdateInput {
  section?: string | null;
  rowLabel?: string | null;
  seat?: string | null;
  ticketType?: string | null;
  listingPriceCents?: number | null;
  status?: TicketStatus | null;
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

export interface Sale {
  id: number;
  code: string;
  ticketId: number;
  ticketCode: string;
  section: string | null;
  rowLabel: string | null;
  seat: string | null;
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
  isDemo: boolean;
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
  /** Available/listed tickets with no listing price set. */
  missingListingPriceCount: number;
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

export interface DashboardData {
  inventory: FinanceSummary;
  period: FinanceSummary;
  periodFrom: string;
  periodTo: string;
  recentOrders: OrderRecord[];
  recentSales: Sale[];
  recentEvents: EventWithStats[];
  /** The single currency every total on this dashboard is computed in. */
  primaryCurrency: string;
  /** True when the database also has data in other currencies, excluded from the totals above. */
  mixedCurrencies: boolean;
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
}

export interface SheetsConnectionTestResult {
  ok: boolean;
  message: string;
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
  syncedAt: string;
}

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

export interface Platform {
  id: number;
  name: string;
  kind: string;
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

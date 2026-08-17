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
  eventId: number;
  eventName: string;
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

export interface DashboardData {
  inventory: FinanceSummary;
  period: FinanceSummary;
  periodFrom: string;
  periodTo: string;
  recentOrders: OrderRecord[];
  recentSales: Sale[];
  recentEvents: EventWithStats[];
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

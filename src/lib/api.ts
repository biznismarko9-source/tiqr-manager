import { invoke } from "@tauri-apps/api/core";
import type {
  AppInfo,
  BulkTicketUpdateInput,
  CsvImportResult,
  CsvPreview,
  DashboardData,
  EventInput,
  EventRecord,
  EventWithStats,
  OrderEditInput,
  OrderInput,
  OrderRecord,
  OrderSalesSummary,
  Platform,
  RestoreOutcome,
  Sale,
  SaleBatchInput,
  SaleEditInput,
  SaleGroup,
  SaleInput,
  Supplier,
  Ticket,
  TicketUpdateInput,
} from "./types";

export const api = {
  // Events
  listEvents: (search?: string) => invoke<EventWithStats[]>("list_events", { search }),
  getEvent: (id: number) => invoke<EventWithStats>("get_event", { id }),
  createEvent: (input: EventInput) => invoke<EventRecord>("create_event", { input }),
  updateEvent: (id: number, input: EventInput) => invoke<EventRecord>("update_event", { id, input }),
  deleteEvent: (id: number) => invoke<void>("delete_event", { id }),

  // Orders
  listOrders: (params: {
    search?: string;
    eventId?: number;
    orderId?: number;
    supplierId?: number;
    platformId?: number;
    status?: string;
    section?: string;
    dateFrom?: string;
    dateTo?: string;
  } = {}) => invoke<OrderRecord[]>("list_orders", params),
  getOrder: (id: number) => invoke<OrderRecord>("get_order", { id }),
  getOrderSalesSummary: (id: number) => invoke<OrderSalesSummary>("get_order_sales_summary", { id }),
  createOrder: (input: OrderInput) => invoke<OrderRecord>("create_order", { input }),
  updateOrder: (id: number, input: OrderEditInput) => invoke<OrderRecord>("update_order", { id, input }),
  deleteOrder: (id: number) => invoke<void>("delete_order", { id }),

  // Tickets
  listTickets: (params: {
    search?: string;
    status?: string;
    eventId?: number;
    orderId?: number;
    sortBy?: string;
    sortDir?: string;
  }) => invoke<Ticket[]>("list_tickets", params),
  getTicket: (id: number) => invoke<Ticket>("get_ticket", { id }),
  updateTicket: (id: number, input: TicketUpdateInput) => invoke<Ticket>("update_ticket", { id, input }),
  /** 1.8.3: set one field to one value across many tickets at once, in a single all-or-nothing transaction. */
  bulkUpdateTickets: (input: BulkTicketUpdateInput) => invoke<Ticket[]>("bulk_update_tickets", { input }),

  // Sales
  listSales: (params: {
    search?: string;
    eventId?: number;
    platformId?: number;
    paymentStatus?: string;
    dateFrom?: string;
    dateTo?: string;
  }) => invoke<Sale[]>("list_sales", params),
  listSaleGroups: (params: {
    search?: string;
    eventId?: number;
    platformId?: number;
    paymentStatus?: string;
    dateFrom?: string;
    dateTo?: string;
    refundStatus?: string;
    /** 1.8.0 */
    currency?: string;
    /** 1.8.0: "oldest" | "revenue_desc" | "revenue_asc" | "profit_desc" | "profit_asc" | "tickets_desc" - omit/undefined for the default (newest first). */
    sortBy?: string;
  }) => invoke<SaleGroup[]>("list_sale_groups", params),
  /** 1.8.0: distinct currencies actually present in sales data, for the Sales screen's Currency filter. */
  listSaleCurrencies: () => invoke<string[]>("list_sale_currencies"),
  listSalesByGroup: (id: number) => invoke<Sale[]>("list_sales_by_group", { id }),
  getSale: (id: number) => invoke<Sale>("get_sale", { id }),
  createSale: (input: SaleInput) => invoke<Sale>("create_sale", { input }),
  createSalesBatch: (input: SaleBatchInput) => invoke<Sale[]>("create_sales_batch", { input }),
  updateSale: (id: number, input: SaleEditInput) => invoke<Sale>("update_sale", { id, input }),
  refundSale: (id: number, reason?: string | null) => invoke<Sale>("refund_sale", { id, reason: reason || null }),
  deleteSale: (id: number) => invoke<void>("delete_sale", { id }),
  deleteSaleGroup: (id: number) => invoke<number>("delete_sale_group", { id }),

  // Lookups
  listPlatforms: () => invoke<Platform[]>("list_platforms"),
  createPlatform: (name: string, kind?: string) => invoke<Platform>("create_platform", { name, kind }),
  deletePlatform: (id: number) => invoke<void>("delete_platform", { id }),
  listSuppliers: () => invoke<Supplier[]>("list_suppliers"),
  createSupplier: (name: string, contact?: string) => invoke<Supplier>("create_supplier", { name, contact }),
  deleteSupplier: (id: number) => invoke<void>("delete_supplier", { id }),

  // Dashboard
  getDashboard: (params: {
    period?: string;
    from?: string;
    to?: string;
    eventId?: number;
    platformId?: number;
  }) => invoke<DashboardData>("get_dashboard", params),

  // CSV import
  previewOrdersCsv: (path: string) => invoke<CsvPreview>("preview_orders_csv", { path }),
  importOrdersCsv: (path: string) => invoke<CsvImportResult>("import_orders_csv", { path }),

  // CSV export
  exportEventsCsv: (path: string) => invoke<number>("export_events_csv", { path }),
  /** 1.9.1: exports exactly the given event ids - powers the Settings -> Data export picker. */
  exportEventsCsvSelected: (path: string, ids: number[]) => invoke<number>("export_events_csv_selected", { path, ids }),
  exportOrdersCsv: (path: string) => invoke<number>("export_orders_csv", { path }),
  /** 1.9.1: exports exactly the given order ids - powers the Settings -> Data export picker. */
  exportOrdersCsvSelected: (path: string, ids: number[]) => invoke<number>("export_orders_csv_selected", { path, ids }),
  exportTicketsCsv: (path: string, status?: string, eventId?: number) =>
    invoke<number>("export_tickets_csv", { path, status, eventId }),
  /** 1.9.1: exports exactly the given ticket ids regardless of status - powers both the Tickets and Inventory Settings -> Data export pickers. */
  exportTicketsCsvSelected: (path: string, ids: number[]) => invoke<number>("export_tickets_csv_selected", { path, ids }),
  exportSalesCsv: (path: string) => invoke<number>("export_sales_csv", { path }),
  /** 1.8.0: exports exactly the sale groups whose (representative) ids are given - every line in each, not just its representative row. */
  exportSalesCsvSelected: (path: string, ids: number[]) => invoke<number>("export_sales_csv_selected", { path, ids }),
  exportInventoryCsv: (path: string, eventId?: number) =>
    invoke<number>("export_inventory_csv", { path, eventId }),
  /** 1.8.3: downloadable header template matching the CSV import's recognized columns. */
  exportOrdersCsvTemplate: (path: string) => invoke<void>("export_orders_csv_template", { path }),

  // Backup / restore
  backupDatabase: (destPath: string) => invoke<void>("backup_database", { destPath }),
  validateBackupFile: (srcPath: string) => invoke<void>("validate_backup_file", { srcPath }),
  restoreDatabase: (srcPath: string) => invoke<RestoreOutcome>("restore_database", { srcPath }),

  // Misc
  getAppInfo: () => invoke<AppInfo>("get_app_info"),

  // Settings (generic key/value, e.g. theme preference)
  getAppSetting: (key: string) => invoke<string | null>("get_app_setting", { key }),
  setAppSetting: (key: string, value: string) => invoke<void>("set_app_setting", { key, value }),
};

export function errMsg(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  try {
    return JSON.stringify(e);
  } catch {
    return String(e);
  }
}

import { invoke } from "@tauri-apps/api/core";
import type {
  AppInfo,
  CsvImportResult,
  CsvPreview,
  DashboardData,
  EventInput,
  EventRecord,
  EventWithStats,
  OrderEditInput,
  OrderInput,
  OrderRecord,
  Platform,
  Sale,
  SaleBatchInput,
  SaleEditInput,
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
  listOrders: (search?: string, eventId?: number) =>
    invoke<OrderRecord[]>("list_orders", { search, eventId }),
  getOrder: (id: number) => invoke<OrderRecord>("get_order", { id }),
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

  // Sales
  listSales: (params: {
    search?: string;
    eventId?: number;
    platformId?: number;
    paymentStatus?: string;
    dateFrom?: string;
    dateTo?: string;
  }) => invoke<Sale[]>("list_sales", params),
  getSale: (id: number) => invoke<Sale>("get_sale", { id }),
  createSale: (input: SaleInput) => invoke<Sale>("create_sale", { input }),
  createSalesBatch: (input: SaleBatchInput) => invoke<Sale[]>("create_sales_batch", { input }),
  updateSale: (id: number, input: SaleEditInput) => invoke<Sale>("update_sale", { id, input }),
  deleteSale: (id: number) => invoke<void>("delete_sale", { id }),

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
  exportOrdersCsv: (path: string) => invoke<number>("export_orders_csv", { path }),
  exportTicketsCsv: (path: string, status?: string, eventId?: number) =>
    invoke<number>("export_tickets_csv", { path, status, eventId }),
  exportSalesCsv: (path: string) => invoke<number>("export_sales_csv", { path }),
  exportInventoryCsv: (path: string, eventId?: number) =>
    invoke<number>("export_inventory_csv", { path, eventId }),

  // Backup / restore
  backupDatabase: (destPath: string) => invoke<void>("backup_database", { destPath }),
  restoreDatabase: (srcPath: string) => invoke<void>("restore_database", { srcPath }),

  // Demo data
  clearDemoData: () => invoke<void>("clear_demo_data"),
  resetDemoData: () => invoke<void>("reset_demo_data"),

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

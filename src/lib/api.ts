import { invoke } from "@tauri-apps/api/core";
import type {
  AppInfo,
  BulkDeleteResult,
  BulkSalePaymentStatusInput,
  BulkTicketStatusInput,
  BulkTicketUpdateInput,
  CreatedSheetResult,
  CsvImportResult,
  CsvPreview,
  CurrencyConversion,
  DashboardData,
  EventCategory,
  EventInput,
  EventRecord,
  EventWithStats,
  FirebaseGoogleSignInResult,
  GoogleSignInStatus,
  OrderEditInput,
  OrderInput,
  OrderRecord,
  OrderSalesSummary,
  Platform,
  Pull,
  PullEditInput,
  PullInput,
  PullReceived,
  PullReceivedEditInput,
  PullReceivedInput,
  RestoreOutcome,
  Sale,
  SaleBatchInput,
  SaleEditInput,
  SaleGroup,
  SaleInput,
  SheetsConnectionConfig,
  SheetsConnectionStatus,
  SheetsConnectionTestResult,
  SpreadsheetTabsResult,
  SheetSyncResult,
  Supplier,
  Ticket,
  TicketUpdateInput,
} from "./types";

export const api = {
  // Events
  listEvents: (params: { search?: string; categoryId?: number } = {}) =>
    invoke<EventWithStats[]>("list_events", params),
  getEvent: (id: number) => invoke<EventWithStats>("get_event", { id }),
  createEvent: (input: EventInput) => invoke<EventRecord>("create_event", { input }),
  updateEvent: (id: number, input: EventInput) => invoke<EventRecord>("update_event", { id, input }),
  deleteEvent: (id: number) => invoke<void>("delete_event", { id }),
  /** 2.0.28: bulk delete for the Events list's "Delete" selection mode - see BulkDeleteResult's doc comment (types.ts). */
  bulkDeleteEvents: (ids: number[]) => invoke<BulkDeleteResult>("bulk_delete_events", { ids }),

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
    /** 2.0.27 */
    categoryId?: number;
  } = {}) => invoke<OrderRecord[]>("list_orders", params),
  getOrder: (id: number) => invoke<OrderRecord>("get_order", { id }),
  getOrderSalesSummary: (id: number) => invoke<OrderSalesSummary>("get_order_sales_summary", { id }),
  createOrder: (input: OrderInput) => invoke<OrderRecord>("create_order", { input }),
  updateOrder: (id: number, input: OrderEditInput) => invoke<OrderRecord>("update_order", { id, input }),
  deleteOrder: (id: number) => invoke<void>("delete_order", { id }),
  /** 2.0.28: bulk delete for the Orders list's "Delete" selection mode - see BulkDeleteResult's doc comment (types.ts). */
  bulkDeleteOrders: (ids: number[]) => invoke<BulkDeleteResult>("bulk_delete_orders", { ids }),

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
  bulkUpdateTicketStatus: (input: BulkTicketStatusInput) => invoke<Ticket[]>("bulk_update_ticket_status", { input }),
  /** 2.0.19: every ticket type marko can currently pick - the 5 built-in
   * defaults, plus any other value already used on a real ticket (typed via
   * "Other..." in the app, or synced in from a sheet cell). Powers the "New
   * order" form's Ticket Type field - was a hardcoded array before 2.0.19. */
  listTicketTypes: () => invoke<string[]>("list_ticket_types"),

  // Pulls (1.9.7) - buying tickets on someone else's behalf for a fee.
  // Deliberately standalone - see src-tauri/migrations/005_pulls.sql.
  listPulls: (params: { search?: string; transferDone?: boolean } = {}) => invoke<Pull[]>("list_pulls", params),
  getPull: (id: number) => invoke<Pull>("get_pull", { id }),
  createPull: (input: PullInput) => invoke<Pull>("create_pull", { input }),
  updatePull: (id: number, input: PullEditInput) => invoke<Pull>("update_pull", { id, input }),
  deletePull: (id: number) => invoke<void>("delete_pull", { id }),
  /** 2.0.28: bulk delete for the Pulls (Given) list's "Delete" selection mode - see BulkDeleteResult's doc comment (types.ts). */
  bulkDeletePulls: (ids: number[]) => invoke<BulkDeleteResult>("bulk_delete_pulls", { ids }),
  /** Dedicated quick-action for the Pulls list's inline "Done" checkbox - see set_pull_transfer_done_impl (pulls.rs). */
  setPullTransferDone: (id: number, done: boolean) => invoke<Pull>("set_pull_transfer_done", { id, done }),

  // Pulls received (2.0.17) - the mirror direction: pulls marko TOOK from
  // other people, instead of pulls he did FOR them. Can be typed manually
  // and/or auto-linked from Orders & Sales sheet sync ("pull" = "yes") - see
  // src-tauri/migrations/011_pulls_received.sql.
  listPullsReceived: (params: { search?: string } = {}) => invoke<PullReceived[]>("list_pulls_received", params),
  getPullReceived: (id: number) => invoke<PullReceived>("get_pull_received", { id }),
  createPullReceived: (input: PullReceivedInput) => invoke<PullReceived>("create_pull_received", { input }),
  updatePullReceived: (id: number, input: PullReceivedEditInput) => invoke<PullReceived>("update_pull_received", { id, input }),
  deletePullReceived: (id: number) => invoke<void>("delete_pull_received", { id }),
  /** 2.0.28: bulk delete for the Pulls (Received) list's "Delete" selection mode - see BulkDeleteResult's doc comment (types.ts). */
  bulkDeletePullsReceived: (ids: number[]) => invoke<BulkDeleteResult>("bulk_delete_pulls_received", { ids }),
  /** 2.0.24: Order Detail's own lean "Add pull info" action - event name/
   * date/quantity/currency are all derived server-side from the order
   * itself, never sent from here. See commands::pulls_received's module doc
   * comment for how this differs from `createPullReceived` above. */
  linkPullReceivedToOrder: (orderId: number, pullerName: string, amountCents: number) =>
    invoke<PullReceived>("link_pull_received_to_order", { orderId, pullerName, amountCents }),
  /** 2.0.24: every pulls_received row linked to one order - see that
   * command's own doc comment for why this is a list, not just one. */
  listPullsReceivedForOrder: (orderId: number) => invoke<PullReceived[]>("list_pulls_received_for_order", { orderId }),

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
    /** 2.0.27 */
    categoryId?: number;
  }) => invoke<SaleGroup[]>("list_sale_groups", params),
  /** 1.8.0: distinct currencies actually present in sales data, for the Sales screen's Currency filter. */
  listSaleCurrencies: () => invoke<string[]>("list_sale_currencies"),
  listSalesByGroup: (id: number) => invoke<Sale[]>("list_sales_by_group", { id }),
  getSale: (id: number) => invoke<Sale>("get_sale", { id }),
  createSale: (input: SaleInput) => invoke<Sale>("create_sale", { input }),
  createSalesBatch: (input: SaleBatchInput) => invoke<Sale[]>("create_sales_batch", { input }),
  updateSale: (id: number, input: SaleEditInput) => invoke<Sale>("update_sale", { id, input }),
  /** 1.9.2: sets many sales' paymentStatus to "pending"/"paid" at once, in a single all-or-nothing transaction. Powers Sale Detail's "Mark as Paid"/"Mark as Pending" action. */
  bulkUpdateSalePaymentStatus: (input: BulkSalePaymentStatusInput) =>
    invoke<Sale[]>("bulk_update_sale_payment_status", { input }),
  refundSale: (id: number, reason?: string | null) => invoke<Sale>("refund_sale", { id, reason: reason || null }),
  deleteSale: (id: number) => invoke<void>("delete_sale", { id }),
  deleteSaleGroup: (id: number) => invoke<number>("delete_sale_group", { id }),
  /** 2.0.28: bulk delete for the Sales list's "Delete" selection mode (one selected id = one sale group/batch, same as list_sale_groups already returns) - see BulkDeleteResult's doc comment (types.ts). */
  bulkDeleteSaleGroups: (ids: number[]) => invoke<BulkDeleteResult>("bulk_delete_sale_groups", { ids }),

  // Lookups
  listPlatforms: () => invoke<Platform[]>("list_platforms"),
  createPlatform: (name: string, kind?: "purchase" | "sale" | "both") => invoke<Platform>("create_platform", { name, kind }),
  deletePlatform: (id: number) => invoke<void>("delete_platform", { id }),
  updatePlatformKind: (id: number, kind: "purchase" | "sale" | "both") =>
    invoke<Platform>("update_platform_kind", { id, kind }),
  listSuppliers: () => invoke<Supplier[]>("list_suppliers"),
  createSupplier: (name: string, contact?: string) => invoke<Supplier>("create_supplier", { name, contact }),
  deleteSupplier: (id: number) => invoke<void>("delete_supplier", { id }),
  /** 2.0.27: managed event categories (Settings -> Lookups, "like Platforms") -
   * see EventCategory's doc comment (types.ts). */
  listEventCategories: () => invoke<EventCategory[]>("list_event_categories"),
  createEventCategory: (name: string) => invoke<EventCategory>("create_event_category", { name }),
  deleteEventCategory: (id: number) => invoke<void>("delete_event_category", { id }),

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

  // Google Sheets sync (Settings -> Integrations, 2.0.2+). `dataSource` is a
  // plain string key ("pulls" today) - see sheets_sync.rs's module doc
  // comment. Connection setup/test only for now - no row sync yet.
  getSheetsConnectionStatus: (dataSource: string) =>
    invoke<SheetsConnectionStatus>("get_sheets_connection_status", { dataSource }),
  setSheetsConnection: (dataSource: string, spreadsheetUrlOrId: string, sheetTab: string, currency: string) =>
    invoke<SheetsConnectionConfig>("set_sheets_connection", { dataSource, spreadsheetUrlOrId, sheetTab, currency }),
  clearSheetsConnection: (dataSource: string) => invoke<void>("clear_sheets_connection", { dataSource }),
  testSheetsConnection: (dataSource: string) => invoke<SheetsConnectionTestResult>("test_sheets_connection", { dataSource }),
  /** 2.0.14: best-effort lookup of a pasted spreadsheet's real tab names, so
   * Settings can offer them as a dropdown instead of requiring the exact tab
   * name to be typed by hand - see
   * commands/sheets_sync.rs::detect_spreadsheet_tabs_impl's doc comment.
   * Takes the raw URL/ID text directly (not a saved `dataSource` connection)
   * since this runs while the form is still being filled in. Never throws
   * for an expected non-result (incomplete paste, not shared yet) - check
   * `.ok` on the result instead. */
  detectSpreadsheetTabs: (spreadsheetUrlOrId: string) =>
    invoke<SpreadsheetTabsResult>("detect_spreadsheet_tabs", { spreadsheetUrlOrId }),
  /** 2.0.3: reads the connected sheet and creates/updates matching Pulls -
   * sheet -> app only, see commands/pulls_sheet_sync.rs. */
  syncPulls: () => invoke<SheetSyncResult>("sync_pulls"),
  /** 2.0.18: the other direction - pushes app-only/app-changed Pulls out to
   * the connected sheet (new rows appended, changed rows updated cell by
   * cell). Sits next to syncPulls as a separate button, never runs as part
   * of it - see commands/pulls_sheet_sync.rs::push_pulls_impl. */
  pushPulls: () => invoke<SheetSyncResult>("push_pulls"),
  /** 2.0.4: auto-creates a brand-new Pulls sheet, shares it with `email`, and
   * connects it - no Google sign-in window. Sits next to setSheetsConnection
   * as a second way to arrive at a connection, not a replacement for it. */
  createPullsSheet: (email: string, currency: string) =>
    invoke<CreatedSheetResult>("create_pulls_sheet", { email, currency }),
  /** 2.0.20: "Update sheet" - for a sheet/tab already connected by pasting
   * its URL/ID (not createPullsSheet's brand-new-sheet flow) that turns out
   * to have no header row yet, e.g. a blank tab. Writes the header row only
   * when the sheet is currently empty; a sheet that already has a header is
   * left completely untouched and this just reports `unchanged` - always a
   * safe click. See commands/pulls_sheet_sync.rs::setup_pulls_sheet_impl. */
  setupPullsSheet: () => invoke<SheetSyncResult>("setup_pulls_sheet"),
  /** 2.0.8: reads the connected sheet and creates a new order (with its
   * tickets) for every row it hasn't seen before - creation-only, sheet ->
   * app only, see commands/orders_sheet_sync.rs. */
  syncOrders: () => invoke<SheetSyncResult>("sync_orders"),
  /** 2.0.18: pushes brand-new local orders out to the connected sheet as new
   * rows - append-only, an order that already has a "TIQR ID" is never
   * revisited (editing its purchase-side numbers after tickets exist would
   * touch protected cost-allocation logic - see push_orders_impl's own doc
   * comment). Sits next to syncOrders as a separate button. */
  pushOrders: () => invoke<SheetSyncResult>("push_orders"),
  /** 2.0.10: reads the SAME connected sheet's second batch of columns and
   * records a sale (with the right platform/date/payout status) for every
   * ticket that isn't sold yet on a row already created by syncOrders -
   * creation-only, sheet -> app only, see commands/orders_sheet_sync.rs. */
  syncSales: () => invoke<SheetSyncResult>("sync_sales"),
  /** 2.0.18: fills in the Sales-sync batch of columns (Site Listed ...
   * how much pull) for an already-linked order, but only when every one of
   * those cells is still completely blank on the sheet - never overwrites
   * anything already there. Needs every ticket on the order sold uniformly
   * (same platform/price/date/status) to have one clean value to push - see
   * push_sales_impl's own doc comment for exactly what counts. Sits next to
   * syncSales as a separate button. */
  pushSales: () => invoke<SheetSyncResult>("push_sales"),
  /** 2.0.9: auto-creates a brand-new Orders & Sales sheet, shares it with
   * `email`, and connects it - no Google sign-in window. Mirrors
   * createPullsSheet exactly; sits next to setSheetsConnection as a second
   * way to arrive at a connection, not a replacement for it. */
  createOrdersSheet: (email: string, currency: string) =>
    invoke<CreatedSheetResult>("create_orders_sheet", { email, currency }),
  /** 2.0.20: "Update sheet" - mirrors setupPullsSheet exactly (writes the
   * header row only when the connected sheet/tab is currently empty), but
   * for Orders & Sales this ALSO always (re-)applies the dropdowns and
   * Revenue/Profit formulas (the same structure Order sync/Sales sync/Push
   * orders/Push sales already keep up to date) right away, rather than
   * waiting for one of those four buttons to be clicked next. See
   * commands/orders_sheet_sync.rs::setup_orders_sheet_impl. */
  setupOrdersSheet: () => invoke<SheetSyncResult>("setup_orders_sheet"),
  /** 2.0.5: installation-wide "Sign in with Google" - see
   * commands/google_auth.rs's module doc comment. `startGoogleSignIn` opens
   * the system browser and blocks (up to 5 minutes) until the person
   * finishes there, it times out, or `cancelGoogleSignIn` interrupts it
   * (2.0.12) - closing the browser tab or picking "use another account" and
   * never finishing used to leave this pending for the full 5 minutes with
   * no way back into the app. */
  getGoogleSignInStatus: () => invoke<GoogleSignInStatus>("get_google_sign_in_status"),
  startGoogleSignIn: () => invoke<GoogleSignInStatus>("start_google_sign_in"),
  /** 2.0.12: the "Cancel" button shown while startGoogleSignIn's own promise
   * is still pending - a safe no-op if nothing is actually in flight (e.g. a
   * stray double-click). Does not itself resolve startGoogleSignIn's promise;
   * that happens moments later, on its own, once the backend notices the flag
   * this sets. */
  cancelGoogleSignIn: () => invoke<void>("cancel_google_sign_in"),
  googleSignOut: () => invoke<void>("google_sign_out"),
  /** 2.0.46: "Continue with Google" on the Welcome screen - signing into the
   * APP itself (Firebase Authentication), a completely separate flow from
   * the Sheets-access one just above despite the similar shape. See
   * commands/firebase_google_auth.rs's module doc comment.
   * `startFirebaseGoogleSignIn` opens the system browser and blocks (up to
   * 5 minutes) until the person finishes there, it times out, or
   * `cancelFirebaseGoogleSignIn` interrupts it - same UX as
   * `cancelGoogleSignIn` above, own separate in-flight attempt. */
  firebaseGoogleSignInAvailable: () => invoke<boolean>("firebase_google_sign_in_available"),
  startFirebaseGoogleSignIn: () => invoke<FirebaseGoogleSignInResult>("start_firebase_google_sign_in"),
  cancelFirebaseGoogleSignIn: () => invoke<void>("cancel_firebase_google_sign_in"),
  // Currency conversion
  /** 2.0.50: "Convert to EUR" on the New Order form - fetches one live rate
   * (Frankfurter, ECB reference rates) and converts every amount in
   * `amountsCents` by it in a single round trip. See
   * commands/currency.rs::convert_currency / fx.rs for the actual call -
   * cannot be exercised against the real network from this dev sandbox,
   * only on a real machine. */
  convertCurrency: (fromCurrency: string, toCurrency: string, amountsCents: number[]) =>
    invoke<CurrencyConversion>("convert_currency", { fromCurrency, toCurrency, amountsCents }),
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

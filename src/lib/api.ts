import { invoke } from "@tauri-apps/api/core";
import type {
  Account,
  AccountInput,
  AppInfo,
  BulkCurrencyConversionResult,
  BulkDeleteResult,
  BulkOrdersDeliveryStatusInput,
  BulkOrdersPaymentStatusInput,
  BulkSaleGroupsDeliveryStatusInput,
  BulkSaleGroupsPaymentStatusInput,
  BulkSalePaymentStatusInput,
  BulkTicketDeliveryStatusInput,
  BulkTicketResaleStatusInput,
  BulkTicketStatusInput,
  BulkTicketUpdateInput,
  CashflowForecast,
  CategoryDetectionResult,
  ComparableReferenceInput,
  CreatedSheetResult,
  CreateFromRecurringResult,
  CsvImportResult,
  CsvPreview,
  CurrencyConversion,
  DashboardData,
  DatabaseSwitchOutcome,
  EventCategory,
  EventInput,
  EventMarketplaceLink,
  EventMarketplaceLinkInput,
  EventRecord,
  EventWithStats,
  FinanceCategory,
  FinanceEntry,
  FinanceEntryInput,
  FirebaseGoogleSignInResult,
  GoogleSignInStatus,
  MarketAnalysisResult,
  Marketplace,
  NotificationConfigInput,
  NotificationStatus,
  NotificationTestResult,
  OrderCurrencyConversionResult,
  OrderEditInput,
  OrderInput,
  OrderRecord,
  OrderSalesSummary,
  Platform,
  PriceCheck,
  PriceCheckInput,
  PriceCheckerSummary,
  Pull,
  PullEditInput,
  PullInput,
  PullReceived,
  PullReceivedEditInput,
  PullReceivedInput,
  RankedComparable,
  RecurringExpense,
  RecurringExpenseInput,
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
  Transfer,
  TransferInput,
} from "./types";

export const api = {
  // Events
  listEvents: (params: { search?: string; categoryId?: number; dateFrom?: string; dateTo?: string } = {}) =>
    invoke<EventWithStats[]>("list_events", params),
  getEvent: (id: number) => invoke<EventWithStats>("get_event", { id }),
  createEvent: (input: EventInput) => invoke<EventRecord>("create_event", { input }),
  updateEvent: (id: number, input: EventInput) => invoke<EventRecord>("update_event", { id, input }),
  deleteEvent: (id: number) => invoke<void>("delete_event", { id }),
  /** 2.0.28: bulk delete for the Events list's "Delete" selection mode - see BulkDeleteResult's doc comment (types.ts). */
  bulkDeleteEvents: (ids: number[]) => invoke<BulkDeleteResult>("bulk_delete_events", { ids }),
  /** 2.0.63: retroactively tries to categorize every event that currently has no category, using
   * the same free-keyword-rules-then-AI logic that already runs automatically on brand-new events
   * created by a sheet sync (see ai_categorize.rs). Only ever touches events with no category yet -
   * see CategoryDetectionResult's doc comment (types.ts) - so this is always safe to run again. */
  detectEventCategories: () => invoke<CategoryDetectionResult>("detect_event_categories"),

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
  /** 2.0.67: Orders-list bulk "Mark Delivered/Not delivered" - only touches
   * each selected order's SOLD tickets. Returns how many tickets were
   * actually changed. See BulkOrdersDeliveryStatusInput's doc comment. */
  bulkSetOrdersDeliveryStatus: (input: BulkOrdersDeliveryStatusInput) =>
    invoke<number>("bulk_set_orders_delivery_status", { input }),
  /** 2.0.67: Orders-list bulk "Mark Paid/Pending" - only touches each
   * selected order's current (non-refunded) sale per sold ticket. Returns how
   * many sales were actually changed. See BulkOrdersPaymentStatusInput's doc
   * comment. */
  bulkSetOrdersPaymentStatus: (input: BulkOrdersPaymentStatusInput) =>
    invoke<number>("bulk_set_orders_payment_status", { input }),
  /** 2.0.51: converts an EXISTING order's currency to EUR - Order Detail's
   * "Convert to EUR" button next to the Currency field, shown whenever the
   * order's currency isn't already EUR (any currency, not just GBP - and
   * works for Sheets-imported orders too, since they're created the same way
   * as manual ones). Fetches one live rate and rewrites the order, every one
   * of its tickets, and every sale tied to those tickets (refunded/historical
   * included) atomically - see commands/orders.rs::convert_order_currency_impl. */
  convertOrderCurrency: (id: number) => invoke<OrderCurrencyConversionResult>("convert_order_currency", { id }),
  /** 2.0.51: the Dashboard mixed-currency banner's bulk action - converts
   * every order in `currencies` to EUR, or every non-EUR order at all when
   * `currencies` is omitted/empty (marko's own "or all" option). Fetches one
   * live rate per distinct currency actually being converted, not one per
   * order, and judges each order on its own merits (one bad order never
   * blocks the rest) - see commands/orders.rs::convert_currencies_to_eur. */
  convertCurrenciesToEur: (currencies?: string[]) =>
    invoke<BulkCurrencyConversionResult>("convert_currencies_to_eur", { currencies: currencies ?? null }),

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
  /** 2.0.69: direct, raw-ticket-id endpoints for Sale/Order Detail's inline
   * status-badge edit - see BulkTicketDeliveryStatusInput/
   * BulkTicketResaleStatusInput's own doc comments in types.ts. */
  bulkUpdateTicketDeliveryStatus: (input: BulkTicketDeliveryStatusInput) =>
    invoke<Ticket[]>("bulk_update_ticket_delivery_status", { input }),
  bulkUpdateTicketResaleStatus: (input: BulkTicketResaleStatusInput) =>
    invoke<Ticket[]>("bulk_update_ticket_resale_status", { input }),
  /** 2.0.19: every ticket type marko can currently pick - the 5 built-in
   * defaults, plus any other value already used on a real ticket (typed via
   * "Other..." in the app, or synced in from a sheet cell). Powers the "New
   * order" form's Ticket Type field - was a hardcoded array before 2.0.19. */
  listTicketTypes: () => invoke<string[]>("list_ticket_types"),

  // Pulls (1.9.7) - buying tickets on someone else's behalf for a fee.
  // Deliberately standalone - see src-tauri/migrations/005_pulls.sql.
  listPulls: (
    params: { search?: string; transferDone?: boolean; platformId?: number; dateFrom?: string; dateTo?: string } = {},
  ) => invoke<Pull[]>("list_pulls", params),
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
  listPullsReceived: (params: { search?: string; dateFrom?: string; dateTo?: string } = {}) =>
    invoke<PullReceived[]>("list_pulls_received", params),
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
  /** 2.0.67: Sales-list bulk "Mark Delivered/Not delivered" - expands each
   * selected group to every ticket across all its lines (refunded lines
   * included). Returns how many tickets were actually changed. See
   * BulkSaleGroupsDeliveryStatusInput's doc comment. */
  bulkSetSaleGroupsDeliveryStatus: (input: BulkSaleGroupsDeliveryStatusInput) =>
    invoke<number>("bulk_set_sale_groups_delivery_status", { input }),
  /** 2.0.67: Sales-list bulk "Mark Paid/Pending" - expands each selected
   * group to its payable (non-refunded) sale ids. Returns how many sales were
   * actually changed. See BulkSaleGroupsPaymentStatusInput's doc comment. */
  bulkSetSaleGroupsPaymentStatus: (input: BulkSaleGroupsPaymentStatusInput) =>
    invoke<number>("bulk_set_sale_groups_payment_status", { input }),

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

  // Finance (2.0.83)
  listFinanceCategories: () => invoke<FinanceCategory[]>("list_finance_categories"),
  createFinanceCategory: (name: string, kind: string) => invoke<FinanceCategory>("create_finance_category", { name, kind }),
  deleteFinanceCategory: (id: number) => invoke<void>("delete_finance_category", { id }),
  listFinanceEntries: () => invoke<FinanceEntry[]>("list_finance_entries"),
  listFinanceEntriesForOrder: (orderId: number) => invoke<FinanceEntry[]>("list_finance_entries_for_order", { orderId }),
  createFinanceEntry: (input: FinanceEntryInput) => invoke<FinanceEntry>("create_finance_entry", { input }),
  updateFinanceEntry: (id: number, input: FinanceEntryInput) => invoke<FinanceEntry>("update_finance_entry", { id, input }),
  deleteFinanceEntry: (id: number) => invoke<void>("delete_finance_entry", { id }),

  // Finance 2.1: Accounts / Transfers / Recurring Expenses / Forecast
  listAccounts: () => invoke<Account[]>("list_accounts"),
  createAccount: (input: AccountInput) => invoke<Account>("create_account", { input }),
  updateAccount: (id: number, input: AccountInput) => invoke<Account>("update_account", { id, input }),
  deleteAccount: (id: number) => invoke<void>("delete_account", { id }),
  listTransfers: () => invoke<Transfer[]>("list_transfers"),
  createTransfer: (input: TransferInput) => invoke<Transfer>("create_transfer", { input }),
  deleteTransfer: (id: number) => invoke<void>("delete_transfer", { id }),
  listRecurringExpenses: () => invoke<RecurringExpense[]>("list_recurring_expenses"),
  createRecurringExpense: (input: RecurringExpenseInput) => invoke<RecurringExpense>("create_recurring_expense", { input }),
  updateRecurringExpense: (id: number, input: RecurringExpenseInput) =>
    invoke<RecurringExpense>("update_recurring_expense", { id, input }),
  deleteRecurringExpense: (id: number) => invoke<void>("delete_recurring_expense", { id }),
  pauseRecurringExpense: (id: number) => invoke<RecurringExpense>("pause_recurring_expense", { id }),
  resumeRecurringExpense: (id: number) => invoke<RecurringExpense>("resume_recurring_expense", { id }),
  skipRecurringExpense: (id: number) => invoke<RecurringExpense>("skip_recurring_expense", { id }),
  createFromRecurring: (id: number) => invoke<CreateFromRecurringResult>("create_from_recurring", { id }),
  getCashflowForecast: () => invoke<CashflowForecast>("get_cashflow_forecast"),

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
  /** 2.0.72: swaps the live database connection to the given account's file -
   * called once by lib/auth.tsx right after Firebase confirms who's signed in
   * AND that they're approved. `legacy` decides which file: `true` reuses the
   * one original shared file (any account that already existed before this
   * feature shipped), `false` gets its own brand-new per-account file. See
   * commands/database.rs's own doc comment for the full design. */
  switchActiveDatabase: (uid: string, legacy: boolean) =>
    invoke<DatabaseSwitchOutcome>("switch_active_database", { uid, legacy }),

  // Settings (generic key/value, e.g. theme preference)
  getAppSetting: (key: string) => invoke<string | null>("get_app_setting", { key }),
  setAppSetting: (key: string, value: string) => invoke<void>("set_app_setting", { key, value }),

  // Anthropic API key (2.1.6, Settings -> AI-assisted price reading) - see
  // commands::settings's own "Anthropic API key" doc comment (Rust) for why
  // this is deliberately NOT the generic getAppSetting/setAppSetting above:
  // the real key value is never returned to the frontend at all, only
  // whether one is currently configured. Same "presence flag, never the
  // value" convention the ntfy notification topic already uses.
  getAnthropicApiKeyConfigured: () => invoke<boolean>("get_anthropic_api_key_configured"),
  setAnthropicApiKey: (key: string) => invoke<void>("set_anthropic_api_key", { key }),

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
  /** 2.0.60: marko's own request after one real sale didn't make it into the
   * sheet via `pushSales`, for a reason that couldn't be pinned down (the
   * order was already linked, every ticket sold at once at one identical
   * price, target cells were blank beforehand - by pushSales' own rule that
   * should already have been enough). Same "is this order even ready"
   * requirements as pushSales (still needs one uniform sale across every
   * ticket), but drops its "only if every target cell is still blank" rule
   * and instead corrects whichever cell currently disagrees with what the
   * app has - unlike every other sync/push action, this one CAN overwrite a
   * cell that already has something in it, so the "Fix sync" button confirms
   * before calling this. Sits next to pushSales as a third, separate
   * action. */
  forcePushSales: () => invoke<SheetSyncResult>("force_push_sales"),
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

  // Outbound notifications (2.0.76; email channel removed again in 2.0.77;
  // mobile-push channel switched from Pushover to ntfy in 2.0.78 - see
  // notifications.rs's module doc comment) - desktop, ntfy. Settings ->
  // Notifications; checkAndSendNotifications is the periodic check, called
  // from Layout.tsx every 30 minutes - see that file's own comment.
  getNotificationStatus: () => invoke<NotificationStatus>("get_notification_status"),
  setNotificationConfig: (input: NotificationConfigInput) =>
    invoke<NotificationStatus>("set_notification_config", { input }),
  testDesktopNotification: () => invoke<NotificationTestResult>("test_desktop_notification"),
  testNtfyNotification: () => invoke<NotificationTestResult>("test_ntfy_notification"),
  /** Silent by design (mirrors checkForUpdate's own "never surfaces an
   * error" contract) - a single disabled/misconfigured channel, or being
   * fully offline, must never interrupt the app with an error toast. See
   * commands/notifications.rs::check_and_send_notifications's doc comment. */
  checkAndSendNotifications: () => invoke<void>("check_and_send_notifications"),

  // Price Checker (2.0.81) - compare marko's own unsold inventory for one
  // event against StubHub/Vivid Seats/Ticombo. Manual entry only, no live
  // API/scraping - see commands/price_checker.rs's module doc comment (Rust)
  // for the full reasoning.
  listMarketplaces: () => invoke<Marketplace[]>("list_marketplaces"),
  /** Settings-style managed list (like Platforms/Suppliers/Event categories) - lets marko add a 4th/5th marketplace later with no code change. */
  createMarketplace: (name: string) => invoke<Marketplace>("create_marketplace", { name }),
  deleteMarketplace: (id: number) => invoke<void>("delete_marketplace", { id }),
  /** Saves (non-blank url) or clears (blank url) one event's link for one marketplace - see EventMarketplaceLinkInput's own doc comment (types.ts). */
  saveEventMarketplaceLink: (input: EventMarketplaceLinkInput) =>
    invoke<EventMarketplaceLink | null>("save_event_marketplace_link", { input }),
  /** Records one manually-typed "Check Prices" entry - always a new row, appended to history, never an overwrite (see PriceCheck's own doc comment). */
  savePriceCheck: (input: PriceCheckInput) => invoke<PriceCheck>("save_price_check", { input }),
  /** The whole Price Checker page for one event (every marketplace's link + full history, marko's own unsold-inventory figures, and the derived market comparison) in a single round trip. */
  getPriceCheckerSummary: (eventId: number) => invoke<PriceCheckerSummary>("get_price_checker_summary", { eventId }),
  // Visible Scanner (2.1.9) - marko's own full rewrite of price-check
  // automation: a NORMAL, VISIBLE window marko scrolls himself, scanned
  // on-demand - see commands/price_checker_scanner.rs's module doc comment
  // (Rust) for the full design and why the old hidden-WebView auto-check
  // (auto_check_price/cancel_auto_check_price) is gone entirely, not just
  // renamed. All four commands here return almost immediately - the real
  // outcome of each always arrives later via one of the
  // `price-scanner-opened`/`price-scanner-error`/`price-scanner-scan-result`/
  // `price-scanner-closed` Tauri events (PriceChecker.tsx listens for all
  // four), matching this app's own established "command returns fast, an
  // event carries the real result" pattern.
  /** Opens a new, real, visible browser window on `url` for one marketplace
   * card. `requestId` is whatever PriceChecker.tsx minted for this session
   * (its own `requestIdRef`) - the backend uses it as the session's key for
   * every later `scanVisiblePrices`/`cancelPriceScan`/`closePriceScanner`
   * call against this same window, and echoes it back on every event so the
   * listener knows which card it's for. Rejects immediately (before any
   * window opens) if `url` isn't a plain http(s) link, or if this exact
   * (eventId, marketplaceId) pair already has a session open. */
  openPriceScanner: (requestId: number, eventId: number, marketplaceId: number, url: string) =>
    invoke<void>("open_price_scanner", { requestId, eventId, marketplaceId, url }),
  /** Reads whatever's CURRENTLY VISIBLE in the scanner window, once - never
   * auto-scrolls or retries on its own (marko scrolls/navigates the window
   * himself between scans). The result merges into the session's running,
   * deduplicated total - see `ScanResultPayload`'s own doc comment
   * (types.ts) for why it's the whole session, not just this scan's delta. */
  scanVisiblePrices: (requestId: number) => invoke<void>("scan_visible_prices", { requestId }),
  /** "Stop scanning" - interrupts a `scanVisiblePrices` call currently in
   * flight for this session, if any. A harmless no-op otherwise (e.g. a
   * stray click after the scan already finished). Never touches the window
   * itself - it stays open and fully usable either way, and marko can start
   * a fresh scan right away. */
  cancelPriceScan: (requestId: number) => invoke<void>("cancel_price_scan", { requestId }),
  /** Ends a scanner session - "Close" in the UI. `closeWindow: true` also
   * closes the real browser window; `false` only forgets TIQR Manager's own
   * bookkeeping and leaves the window open (marko's own spec: the browser
   * "zostane otvorený alebo sa môže zavrieť podľa voľby používateľa"). Safe
   * to call even if the window was already closed natively a moment
   * earlier - never errors, since the end state is the same either way. */
  closePriceScanner: (requestId: number, closeWindow: boolean) =>
    invoke<void>("close_price_scanner", { requestId, closeWindow }),
  // Market Analysis (2.2.0) - built entirely on top of the Visible Scanner
  // above, never touches its commands/session/lifecycle. See
  // commands/price_checker_analysis.rs's module doc comment (Rust) for the
  // full design.
  /** Tier/section breakdown, per-currency market stats, and a "Your
   * Tickets" pricing panel for a scanner session's already-accumulated
   * listings - one round trip, matching marko's own "## PERFORMANCE"
   * requirement (see `MarketAnalysisResult`'s own doc comment, types.ts).
   * `requestId` is the SAME id `openPriceScanner` used for this card's
   * session - rejects with "not found" if that session's window has since
   * been closed, same as `scanVisiblePrices`. */
  computeMarketAnalysis: (requestId: number, eventId: number) =>
    invoke<MarketAnalysisResult>("compute_market_analysis", { requestId, eventId }),
  /** Ranks a scanner session's listings against ONE specific reference
   * ticket (marko's own spec example: Section 112 / Row 8 / Quantity 4) -
   * see `ComparableReferenceInput`'s own doc comment (types.ts) for why
   * `currency` is required, not optional. Pure in-memory on the backend;
   * safe to call repeatedly as marko edits the reference fields. */
  computeComparableMarket: (input: ComparableReferenceInput) =>
    invoke<RankedComparable[]>("compute_comparable_market", { input }),
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

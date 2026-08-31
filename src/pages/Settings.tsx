import { useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { open, save } from "@tauri-apps/plugin-dialog";
import { relaunch } from "@tauri-apps/plugin-process";
import { api, errMsg } from "../lib/api";
import {
  type AppInfo,
  type CreatedSheetResult,
  type CsvPreview,
  type EventCategory,
  type FinanceCategory,
  type GoogleSignInStatus,
  type NotificationConfigInput,
  type NotificationStatus,
  type NotificationTestResult,
  type Platform,
  type SheetsConnectionStatus,
  type SheetsConnectionTestResult,
  type SheetSyncResult,
  type SpreadsheetTabsResult,
} from "../lib/types";
import {
  Badge,
  Button,
  Card,
  CHECKBOX_CLASS,
  ConfirmDialog,
  Field,
  Input,
  Modal,
  ModalFooter,
  PageHeader,
  Select,
  Spinner,
} from "../components/ui";
import {
  eventsExportConfig,
  inventoryExportConfig,
  ordersExportConfig,
  salesExportConfig,
  ticketsExportConfig,
  ExportPickerModal,
  type ExportPickerConfig,
} from "../components/ExportPickerModal";
import { EventCategorySwatch } from "../components/EventCategoryBadge";
import { FinanceCategorySwatch } from "../components/FinanceCategoryBadge";
import {
  IconAlertTriangle,
  IconArrowLeft,
  IconBell,
  IconChevronDown,
  IconDatabase,
  IconDownload,
  IconInfo,
  IconLink,
  IconLogOut,
  IconPlus,
  IconSun,
  IconTag,
  IconTrash,
  IconUpload,
  IconUser,
} from "../components/icons";
import { useToast } from "../lib/toast";
import { checkForUpdate, installUpdate, type Update, type UpdateProgress } from "../lib/updater";
import { UpdateOverlay } from "../components/UpdateOverlay";
import { useTheme, type ThemeMode } from "../lib/theme";
import { useAuth } from "../lib/auth";
import { firebaseAuthErrorMessage } from "../lib/firebaseErrors";

const THEME_OPTIONS: { key: ThemeMode; label: string }[] = [
  { key: "light", label: "Light" },
  { key: "system", label: "System" },
  { key: "dark", label: "Dark" },
];

// 1.8.2: Settings Home - one card per category, each a real route
// (settings/:section, added in App.tsx) rather than a scroll-to-anchor, so
// it's stable on refresh via HashRouter with no new routing infrastructure.
// Existing features were only re-grouped under these 4 - nothing here is new
// functionality, see REDESIGN-1.8.2-REPORT.md section 4. No explicit array
// type annotation - TS infers `icon` as the shared icon-component type from
// the 4 actual values, which keeps this safe from any structural-typing
// mismatch that a hand-written narrower prop type could risk introducing.
// 2.0.2: Integrations - connect Pulls (Tickets later) to a Google Sheet. See
// SheetsConnectionCard below and REDESIGN-2.0.2-REPORT.md.
const SECTIONS = [
  { key: "lookups", title: "Lookups", description: "Platforms and other lookup lists used across orders and sales.", icon: IconTag },
  { key: "data", title: "Data", description: "Import CSV, export CSV, backup and restore your database.", icon: IconDatabase },
  {
    key: "integrations",
    title: "Integrations",
    // 2.1.6: was just "Connect Pulls, Orders and Tickets to a Google
    // Sheet." - extended (not split into a new section) once the Anthropic
    // API key card landed here too, since it's the same idea (connect an
    // optional external service) rather than its own new top-level concern.
    description: "Connect Pulls, Orders and Tickets to a Google Sheet, or add an Anthropic API key for AI-assisted price reading.",
    icon: IconLink,
  },
  // 2.0.76: desktop/mobile-push alerts for the same 4 things the Dashboard's
  // bell already tracks - see NotificationsCard below and REDESIGN-2.0.76-
  // REPORT.md. 2.0.77 removed the email channel this shipped with at
  // marko's own request; 2.0.78 switched the mobile-push channel from
  // Pushover to ntfy - see NotificationsCard's own doc comment.
  { key: "notifications", title: "Notifications", description: "Desktop and ntfy alerts for the things that need your attention.", icon: IconBell },
  { key: "appearance", title: "Appearance", description: "Light, system or dark theme.", icon: IconSun },
  { key: "software", title: "Software", description: "Check for updates and see your current version.", icon: IconDownload },
  // 2.0.44: your name/email/sign-in + Log out - see the profile widget at
  // the bottom of the sidebar (Layout.tsx), whose "Account settings" item
  // links straight to /settings/account.
  { key: "account", title: "Account", description: "Your name, email and sign-in.", icon: IconUser },
];

export default function Settings() {
  const { section } = useParams();
  const toast = useToast();
  const { user, updateName, logout } = useAuth();
  // 2.0.44: local draft of the name field on the Account section - synced
  // from `user` whenever it changes rather than only read once on mount,
  // since Settings.tsx's route ("settings" vs "settings/:section") can
  // re-render this same component instance without necessarily remounting it.
  const [accountName, setAccountName] = useState(user?.name ?? "");
  useEffect(() => {
    setAccountName(user?.name ?? "");
  }, [user?.name]);
  const [savingName, setSavingName] = useState(false);
  const [loggingOut, setLoggingOut] = useState(false);
  const [themeMode, setThemeMode] = useTheme();
  const [platforms, setPlatforms] = useState<Platform[]>([]);
  // 2.0.27: managed event categories (marko's request - "like Platforms").
  const [categories, setCategories] = useState<EventCategory[]>([]);
  // 2.0.83: managed Finance categories (Finance.tsx) - same "like Platforms"
  // pattern, split into Expense/Income lists the same way Platforms splits
  // into Purchase/Selling (see FinanceCategoryList below).
  const [financeCategories, setFinanceCategories] = useState<FinanceCategory[]>([]);
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null);
  const [importOpen, setImportOpen] = useState(false);
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [confirmRestorePath, setConfirmRestorePath] = useState<string | null>(null);
  const [confirmDeletePlatform, setConfirmDeletePlatform] = useState<Platform | null>(null);
  const [confirmDeleteCategory, setConfirmDeleteCategory] = useState<EventCategory | null>(null);
  const [confirmDeleteFinanceCategory, setConfirmDeleteFinanceCategory] = useState<FinanceCategory | null>(null);
  const [deletingLookup, setDeletingLookup] = useState(false);
  // 1.9.1: which entity's export picker is open, if any - see
  // ExportPickerModal.tsx. Replaces the old "click = instant whole-file
  // download" Export CSV buttons with "click = pick exactly which records"
  // (marko's request), reusing whichever *ExportConfig matches the button.
  const [exportConfig, setExportConfig] = useState<ExportPickerConfig<any> | null>(null);

  const [updateChecking, setUpdateChecking] = useState(false);
  const [updateChecked, setUpdateChecked] = useState(false);
  const [available, setAvailable] = useState<Update | null>(null);
  const [updateError, setUpdateError] = useState<string | null>(null);
  const [installing, setInstalling] = useState(false);
  const [installProgress, setInstallProgress] = useState<UpdateProgress | null>(null);

  // 2.0.5: lifted up here (rather than fetched independently inside
  // SheetsConnectionCard too) so a sign-in/sign-out in GoogleSignInCard is
  // reflected in SheetsConnectionCard immediately, with exactly one fetch of
  // this status per Settings visit rather than one per card.
  const [googleStatus, setGoogleStatus] = useState<GoogleSignInStatus | null>(null);

  const reload = () => {
    api.listPlatforms().then(setPlatforms).catch((e) => toast.error(errMsg(e)));
    api.listEventCategories().then(setCategories).catch((e) => toast.error(errMsg(e)));
    api.listFinanceCategories().then(setFinanceCategories).catch((e) => toast.error(errMsg(e)));
    api.getAppInfo().then(setAppInfo).catch(() => {});
  };

  useEffect(reload, []);

  // 1.9.3: platforms are now added straight into one of the two lists
  // (Purchase/Selling) below, so `addPlatform` needs to know which `kind` to
  // create with - see PlatformList, which owns its own input state and
  // calls this with its own fixed `kind`.
  const addPlatform = async (name: string, kind: "purchase" | "sale") => {
    try {
      await api.createPlatform(name, kind);
      reload();
    } catch (e) {
      toast.error(errMsg(e));
    }
  };

  // 1.9.3: re-tags an existing platform (e.g. a pre-1.9.3 platform, which
  // defaulted to "both" and so still shows in both lists) between Purchase/
  // Sale/Both - see PlatformList's per-row Select.
  const changePlatformKind = async (p: Platform, kind: "purchase" | "sale" | "both") => {
    try {
      await api.updatePlatformKind(p.id, kind);
      reload();
    } catch (e) {
      toast.error(errMsg(e));
    }
  };

  // 2.0.27: one list, not two (categories have no Purchase/Selling "kind"
  // split the way platforms do) - see EventCategoryList below.
  const addCategory = async (name: string) => {
    try {
      await api.createEventCategory(name);
      reload();
    } catch (e) {
      toast.error(errMsg(e));
    }
  };
  // 2.0.83: Finance categories - one list backed by `kind` ('expense'/
  // 'income'), same split-into-two-lists-by-kind idea as `addPlatform`
  // above, so FinanceCategoryList's own two instances (Expense/Income) just
  // pass their own fixed `kind` through, same as PlatformList does.
  const addFinanceCategory = async (name: string, kind: "expense" | "income") => {
    try {
      await api.createFinanceCategory(name, kind);
      reload();
    } catch (e) {
      toast.error(errMsg(e));
    }
  };
  // 1.8.3 (section 10): only one CSV import exists (orders + tickets
  // together, see csv_import.rs) so only one template is offered - see
  // export_orders_csv_template's doc comment (csv_export.rs).
  const doDownloadTemplate = async () => {
    const path = await save({
      defaultPath: "tiqr-orders-import-template.csv",
      filters: [{ name: "CSV", extensions: ["csv"] }],
    });
    if (!path) return;
    setBusyAction("template");
    try {
      await api.exportOrdersCsvTemplate(path);
      toast.success(`Template saved to ${path}`);
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setBusyAction(null);
    }
  };

  const doBackup = async () => {
    const stamp = new Date().toISOString().slice(0, 10);
    const path = await save({
      defaultPath: `tiqr-manager-backup-${stamp}.sqlite3`,
      filters: [{ name: "SQLite Database", extensions: ["sqlite3", "db", "sqlite"] }],
    });
    if (!path) return;
    setBusyAction("backup");
    try {
      await api.backupDatabase(path);
      toast.success(`Backup saved to ${path}`);
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setBusyAction(null);
    }
  };

  const pickRestoreFile = async () => {
    const path = await open({
      multiple: false,
      filters: [{ name: "SQLite Database", extensions: ["sqlite3", "db", "sqlite"] }],
    });
    if (!path || Array.isArray(path)) return;
    // Validate before showing the "this will replace your data" confirmation,
    // so a file that's not a TIQR Manager backup is rejected with a clear
    // error right away instead of behind a scary confirm dialog.
    setBusyAction("restore");
    try {
      await api.validateBackupFile(path);
      setConfirmRestorePath(path);
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setBusyAction(null);
    }
  };

  const doRestore = async () => {
    if (!confirmRestorePath) return;
    setBusyAction("restore");
    try {
      const result = await api.restoreDatabase(confirmRestorePath);
      toast.success(
        `Backup restored. Your previous data was automatically saved to ${result.safetyBackupPath}. Relaunching...`,
      );
      setTimeout(() => relaunch(), 700);
    } catch (e) {
      toast.error(errMsg(e));
      setBusyAction(null);
      setConfirmRestorePath(null);
    }
  };

  const doCheckForUpdate = async () => {
    setUpdateChecking(true);
    setUpdateError(null);
    try {
      const update = await checkForUpdate();
      setAvailable(update);
      setUpdateChecked(true);
    } catch (e) {
      setUpdateError(errMsg(e) || "Could not check for updates - are you online?");
    } finally {
      setUpdateChecking(false);
    }
  };

  const doInstallUpdate = async () => {
    if (!available) return;
    setInstalling(true);
    setUpdateError(null);
    try {
      await installUpdate(available, setInstallProgress);
      // installUpdate relaunches the app on success; if we're still here
      // after a moment something odd happened, but there's nothing more to do.
    } catch (e) {
      setUpdateError(errMsg(e) || "Update failed to install");
      setInstalling(false);
    }
  };

  const activeSection = section ? SECTIONS.find((s) => s.key === section) : undefined;

  return (
    <div>
      {!activeSection ? (
        <>
          <PageHeader title="Settings" subtitle="Lookups, data, appearance and software." />
          {/* 1.8.2: Settings Home - every category visible at once, no
              scrolling needed to find one (see REDESIGN-1.8.2-REPORT.md
              section 4).
              2.0.48: was a 4-column grid (row-major reading order: Lookups,
              Data, Integrations, Appearance, then Software, Account on a
              second row) - marko found that hard to read as a sequence, so
              this is now one column, top to bottom. SECTIONS' order itself
              is unchanged - it already read Lookups -> Data -> Integrations
              -> Appearance -> Software -> Account (roughly: set up your
              reference data, bring in/manage your data, connect optional
              external tools, personal preference, maintenance, account/
              sign-in), the grid layout was the only thing making that read
              as scattered instead of sequential. Capped at max-w-2xl - a
              list of rows reads better narrower than the old grid did, and
              it keeps every row's text at a comfortable line length on a
              wide window. */}
          <div className="flex flex-col gap-2 lg:max-w-2xl">
            {SECTIONS.map((s) => (
              <Link
                key={s.key}
                to={`/settings/${s.key}`}
                className="card flex items-center gap-4 p-4 text-left transition-colors hover:border-brand-300 dark:hover:border-brand-700 hover:bg-slate-50 dark:hover:bg-slate-800/60"
              >
                <s.icon className="h-6 w-6 shrink-0 text-brand-600 dark:text-brand-400" />
                <span className="min-w-0 flex-1">
                  <h3 className="text-sm font-semibold text-slate-800 dark:text-slate-200">{s.title}</h3>
                  <p className="mt-0.5 text-xs text-slate-400 dark:text-slate-500">{s.description}</p>
                </span>
                <IconChevronDown className="h-4 w-4 shrink-0 -rotate-90 text-slate-300 dark:text-slate-600" />
              </Link>
            ))}
          </div>
        </>
      ) : (
        <>
          <Link
            to="/settings"
            className="mb-3 inline-flex items-center gap-1 text-sm text-slate-500 dark:text-slate-400 hover:text-slate-800 dark:hover:text-slate-200"
          >
            <IconArrowLeft className="h-4 w-4" /> Back to Settings
          </Link>
          <PageHeader title={activeSection.title} subtitle={activeSection.description} />

          {/* 1.8.2: same Card content as before 1.8.2, just re-grouped one
              category per route instead of one long scrolling page - see
              REDESIGN-1.8.2-REPORT.md section 5. Nothing below this point
              in each branch is new functionality. */}
          {section === "appearance" && (
            <Card className="p-3">
              <div className="flex flex-wrap items-center justify-between gap-2">
                <h3 className="text-sm font-semibold text-slate-800 dark:text-slate-200">Theme</h3>
                <div className="inline-flex rounded-lg border border-slate-200 dark:border-slate-800 p-0.5">
                  {THEME_OPTIONS.map((o) => (
                    <button
                      key={o.key}
                      onClick={() => setThemeMode(o.key)}
                      className={`rounded-md px-2.5 py-1 text-xs font-medium transition-colors ${
                        themeMode === o.key
                          ? "bg-brand-600 text-white"
                          : "text-slate-500 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-100"
                      }`}
                    >
                      {o.label}
                    </button>
                  ))}
                </div>
              </div>
            </Card>
          )}

          {section === "lookups" && (
            <Card className="p-5 lg:max-w-3xl">
              {/* 1.9.3: split into two lists - marko didn't want "where you
                  bought it" and "where you sold it" sharing one pool any
                  more. Backed by the same `platforms` table either way (its
                  `kind` column has existed since the first migration, so no
                  schema change was needed) - a platform tagged "Both" simply
                  appears in both lists below, via PlatformList's own filter.
                  2.0.73: the explanation of what Purchase/Selling/Both mean
                  used to be a permanent paragraph here, every time - same
                  "keep every feature, just don't show it when you don't need
                  it" simplification already applied to Settings ->
                  Integrations (REDESIGN-2.0.65-REPORT.md). It's now on the
                  heading's own hover hint (InfoHint below) instead - nothing
                  explained before is explained any less, it's just not
                  permanently on screen. */}
              <div className="mb-4 flex items-center gap-1.5">
                <h3 className="text-sm font-semibold text-slate-800 dark:text-slate-200">Platforms</h3>
                <InfoHint text={`Purchase platforms show up when recording an order; Selling platforms show up when recording a sale. Tag a platform "Both" if you use it for either. Not hardcoded — add as many as you like.`} />
              </div>
              <div className="grid grid-cols-1 gap-5 sm:grid-cols-2">
                <PlatformList
                  heading="Purchase platforms"
                  kind="purchase"
                  platforms={platforms}
                  onAdd={addPlatform}
                  onDelete={setConfirmDeletePlatform}
                  onChangeKind={changePlatformKind}
                />
                <PlatformList
                  heading="Selling platforms"
                  kind="sale"
                  platforms={platforms}
                  onAdd={addPlatform}
                  onDelete={setConfirmDeletePlatform}
                  onChangeKind={changePlatformKind}
                />
              </div>

              {/* 2.0.27: marko's request - filter and color-code Events/
                  Orders/Sales by category (football, concert, etc.). One
                  list, not a Purchase/Selling pair - a category has no
                  "kind" split. Each category's color is assigned
                  automatically the first time it's added (see
                  EventCategoryBadge.tsx) - nothing to pick here. */}
              <div className="mt-6 border-t border-slate-100 pt-5 dark:border-slate-800">
                <div className="mb-4 flex items-center gap-1.5">
                  <h3 className="text-sm font-semibold text-slate-800 dark:text-slate-200">Event categories</h3>
                  <InfoHint text="Tag events (football, concert, etc.) to filter and color-code them on Events, Orders and Sales. Not hardcoded — add as many as you like, each gets its own color automatically." />
                </div>
                <div className="sm:max-w-sm">
                  <EventCategoryList categories={categories} onAdd={addCategory} onDelete={setConfirmDeleteCategory} />
                </div>
              </div>

              {/* 2.0.83: Finance categories (Finance.tsx) - same
                  Expense/Income split as Platforms' Purchase/Selling split
                  above, backed by the same one `finance_categories` table
                  either way (a category's `kind` decides which list(s) it
                  shows in). */}
              <div className="mt-6 border-t border-slate-100 pt-5 dark:border-slate-800">
                <div className="mb-4 flex items-center gap-1.5">
                  <h3 className="text-sm font-semibold text-slate-800 dark:text-slate-200">Finance categories</h3>
                  <InfoHint text="Categories for the Finance section's entries (personal and business money). Each gets its own color automatically - not hardcoded, add as many as you like." />
                </div>
                <div className="grid grid-cols-1 gap-5 sm:grid-cols-2">
                  <FinanceCategoryList
                    heading="Expense categories"
                    kind="expense"
                    categories={financeCategories}
                    onAdd={addFinanceCategory}
                    onDelete={setConfirmDeleteFinanceCategory}
                  />
                  <FinanceCategoryList
                    heading="Income categories"
                    kind="income"
                    categories={financeCategories}
                    onAdd={addFinanceCategory}
                    onDelete={setConfirmDeleteFinanceCategory}
                  />
                </div>
              </div>
            </Card>
          )}

          {section === "data" && (
            // 1.9.5: marko wants these stacked instead of side-by-side -
            // was `grid-cols-1 lg:grid-cols-2` (Import/Export paired on wide
            // screens, Backup spanning below); now a single column always,
            // so all three cards read top-to-bottom regardless of width.
            <div className="grid grid-cols-1 gap-4">
              <Card className="p-5">
                <h3 className="mb-1 text-sm font-semibold text-slate-800 dark:text-slate-200">Import orders from CSV</h3>
                {/* 1.9.2 (section 8): shortened from a single dense paragraph
                    to just the 3 things marko asked to keep - the required
                    columns, the seats note, and the all-or-nothing note.
                    Import itself (Preview -> Validate -> Confirm -> Import,
                    still transactional/all-or-nothing) and Download template
                    below are unchanged - this is a text-only simplification. */}
                <div className="mb-3 space-y-1 text-xs text-slate-400 dark:text-slate-500">
                  <p>
                    <span className="font-medium text-slate-500 dark:text-slate-400">Required format:</span> event,
                    purchase_date, supplier, platform, quantity, unit_price, fees, other_costs, currency,
                    payment_status, ticket_type, section, row, seats, notes.
                  </p>
                  <p>
                    "seats" is a comma-separated list matching quantity (e.g. "11,12,13,14") - leave it out to import
                    without seat numbers.
                  </p>
                  <p>Import is all-or-nothing.</p>
                </div>
                <div className="flex flex-wrap gap-2">
                  <Button variant="primary" onClick={() => setImportOpen(true)}>
                    <IconUpload className="h-4 w-4" /> Choose CSV &amp; preview
                  </Button>
                  <Button variant="secondary" disabled={busyAction === "template"} onClick={doDownloadTemplate}>
                    {busyAction === "template" ? <Spinner className="h-4 w-4" /> : <IconDownload className="h-4 w-4" />}
                    Download template
                  </Button>
                </div>
              </Card>

              <Card className="p-5">
                <h3 className="mb-1 text-sm font-semibold text-slate-800 dark:text-slate-200">Export CSV</h3>
                <p className="mb-3 text-xs text-slate-400 dark:text-slate-500">
                  Save part of your data as a CSV file - each button opens a picker so you can choose exactly which
                  records to include (one, several, or all).
                </p>
                <div className="flex flex-wrap gap-2">
                  {[
                    { label: "Events", key: "events", config: eventsExportConfig },
                    { label: "Orders", key: "orders", config: ordersExportConfig },
                    { label: "Tickets", key: "tickets", config: ticketsExportConfig },
                    { label: "Sales", key: "sales", config: salesExportConfig },
                    { label: "Inventory", key: "inventory", config: inventoryExportConfig },
                  ].map((x) => (
                    <Button key={x.key} variant="secondary" onClick={() => setExportConfig(x.config)}>
                      <IconDownload className="h-4 w-4" />
                      {x.label}
                    </Button>
                  ))}
                </div>
              </Card>

              <Card className="p-5">
                <h3 className="mb-1 text-sm font-semibold text-slate-800 dark:text-slate-200">Backup &amp; restore</h3>
                <p className="mb-3 text-xs text-slate-400 dark:text-slate-500">
                  Your database lives only on this device. Back it up regularly, especially before big imports.
                </p>
                <div className="flex flex-wrap gap-2">
                  <Button variant="secondary" disabled={busyAction === "backup"} onClick={doBackup}>
                    {busyAction === "backup" ? <Spinner className="h-4 w-4" /> : <IconDatabase className="h-4 w-4" />}
                    Backup database...
                  </Button>
                  <Button variant="secondary" disabled={busyAction === "restore"} onClick={pickRestoreFile}>
                    {busyAction === "restore" ? <Spinner className="h-4 w-4" /> : <IconUpload className="h-4 w-4" />}
                    Restore from backup...
                  </Button>
                </div>
                {appInfo && (
                  <p className="mt-4 break-all text-xs text-slate-400 dark:text-slate-500">
                    {/* 2.0.72: signed-in email shown alongside the path now
                        that different accounts can have entirely different
                        files - see lib/auth.tsx's `switchDatabaseFor`. */}
                    Database file ({user?.email}): <span className="font-mono">{appInfo.dbPath}</span>
                  </p>
                )}
              </Card>
            </div>
          )}

          {section === "integrations" && (
            <div className="grid grid-cols-1 gap-4 lg:max-w-6xl">
              <GoogleSignInCard onChange={setGoogleStatus} />
              {/* 2.0.16: Pulls and Orders & Sales side by side (marko's own
                  request) now that Sign in with Google always sits above
                  both, full width - was a single stacked column. */}
              <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
                <SheetsConnectionCard
                  dataSource="pulls"
                  label="Pulls"
                  googleStatus={googleStatus}
                  onSync={api.syncPulls}
                  syncDescription={`"Sync now" reads the sheet and creates/updates matching pulls in the app - it never writes your data back to the sheet yet, except its own row IDs.`}
                  onPush={api.pushPulls}
                  pushDescription={`"Push to sheet" is the other direction: brand-new pulls you added in the app become new rows, and changes to an already-synced pull are written back cell by cell - but only when the sheet itself hasn't changed that row since the last sync (if it has, the row is reported so you can "Sync now" first, then push again).`}
                  onSetup={api.setupPullsSheet}
                  setupDescription={`For a sheet you connected above that's still completely blank: writes the correct header row for you, exactly as "Create a new sheet for me" below would - use this instead when you already have the specific sheet/tab you want to keep using, just empty. Also (re-)applies the Platform/Transfer dropdowns below right away, same as Sync now/Push to sheet already do.`}
                  onCreate={api.createPullsSheet}
                />
                {/* 2.0.8: one row = one order (marko's own choice) - creates the
                    order and all its tickets from the sheet's first batch of
                    columns. 2.0.9: "Create a new sheet for me" added
                    (onCreate) - mirrors Pulls' own from-scratch setup, for
                    anyone who doesn't already have a real sheet like marko's.
                    2.0.10: "Sales sync" (secondarySync) reads the sheet's
                    SECOND batch of columns (same rows, same connection - marko
                    only ever connects this sheet once) and records a sale for
                    every ticket an already-synced row's order hasn't sold yet -
                    see commands/orders_sheet_sync.rs's module doc comment.
                    2.0.12: renamed from "Orders & Tickets" to "Orders & Sales"
                    (marko's own request) - purely the on-screen label; the
                    data_source key ("orders"), and every already-connected
                    sheet, are untouched. */}
                <SheetsConnectionCard
                  dataSource="orders"
                  label="Orders & Sales"
                  googleStatus={googleStatus}
                  onSync={api.syncOrders}
                  syncLabel="Order sync"
                  syncDescription={`"Order sync" reads the sheet and creates a new order (with its tickets) for every row it hasn't seen before - it never edits an order once created, and never writes your data back to the sheet except its own row IDs. Add new rows any time and sync again.`}
                  secondarySync={{
                    label: "Sales sync",
                    description:
                      "Reads the SAME sheet's second batch of columns and records a sale for every ticket that isn't sold yet on a row Order sync already created - creation-only, same as Order sync: once a ticket has an active sale, later syncs leave it completely alone.",
                    run: api.syncSales,
                  }}
                  onPush={api.pushOrders}
                  pushLabel="Push orders"
                  pushDescription="Adds brand-new orders you created in the app as new rows in the sheet - append-only, an order that's already in the sheet is never edited here again (its costs are already split across its tickets, so that stays a change you make by hand)."
                  secondaryPush={{
                    label: "Push sales",
                    description:
                      "Fills in the SAME row's Site Listed/Payout/Status/Payout status/paid-by columns (and pull/who pulled/how much pull, from a linked received pull) once every ticket on that order has sold the same way - but only into cells that are still completely blank, so it never overwrites anything already in the sheet. One exception: once every ticket on an order has been refunded in the app, this clears that same row's sale columns back to blank instead, since a refunded ticket is no longer an active sale - keeps the Summary block below from counting it forever.",
                    run: api.pushSales,
                  }}
                  forcePush={{
                    label: "Fix sync",
                    description:
                      'For a sale that should have pushed already (e.g. via "Push sales" above) but didn\'t. Unlike that button, this one CAN overwrite a cell that already has something in it, replacing it with what the app currently has for that order - but only cells whose current text actually disagrees with a value the app actually has; a cell the app has no data for (e.g. no platform recorded) is always left exactly as-is, never blanked, and an already-correct cell is left alone too - so clicking this again is always safe. Also clears a refunded order\'s stale row the same way "Push sales" does (see that button\'s own description). Never touches Total Purchase Price, currency, or the sheet\'s dropdowns/formulas - only "Push sales"\' own columns.',
                    confirmMessage:
                      'This can overwrite Site Listed / Payout / Status / Delivery status / Payout status / sale date / paid-by / pull cells that already have something in them, replacing it with what the app currently has for that order (blanking them if the order has since been fully refunded) - unlike "Push sales", which only ever fills in blank cells. It will never blank a cell the app itself has no data for, and never touches Total Purchase Price, currency, or the sheet\'s dropdowns/formulas. Use this when a sale (or received pull) you know is correct in the app didn\'t make it into the sheet. Continue?',
                    run: api.forcePushSales,
                  }}
                  onSetup={api.setupOrdersSheet}
                  setupDescription="For a sheet you connected above that's still completely blank: writes the correct header row for you, then immediately sets up its dropdowns and Revenue/Profit formulas - the same structure Order sync/Sales sync/Push orders/Push sales already keep up to date, applied right away instead of waiting for one of those."
                  onCreate={api.createOrdersSheet}
                />
              </div>

              {/* 2.1.6: standalone - not a Google Sheets thing at all, but
                  "integrations" is the closest existing home (connecting an
                  optional external service) and a whole new top-level
                  section for one API-key field felt like more navigation
                  than marko needs. See AnthropicApiKeyCard's own doc
                  comment. */}
              <AnthropicApiKeyCard />
            </div>
          )}

          {section === "notifications" && (
            <div className="lg:max-w-2xl">
              <NotificationsCard />
            </div>
          )}

          {section === "software" && (
            <Card className="p-5 lg:max-w-xl">
              <h3 className="mb-1 text-sm font-semibold text-slate-800 dark:text-slate-200">Software updates</h3>
              <p className="mb-3 text-xs text-slate-400 dark:text-slate-500">
                Checks GitHub for a newer signed release. Nothing downloads until you approve it, and everything
                still works fully offline either way.
              </p>

              {installing ? (
                // 2.0.39: full-screen branded takeover (marko asked for a
                // real "screen" here, not just this card's own progress bar)
                // - see UpdateOverlay.tsx. Still driven by the exact same
                // `installing`/`available`/`installProgress` state as before,
                // untouched - only the presentation moved out of this card.
                <UpdateOverlay version={available?.version} progress={installProgress} />
              ) : available ? (
                <div>
                  <p className="mb-1 text-sm font-medium text-slate-800 dark:text-slate-200">Version {available.version} is available</p>
                  {available.body && <p className="mb-3 whitespace-pre-line text-xs text-slate-500 dark:text-slate-400">{available.body}</p>}
                  <Button variant="primary" onClick={doInstallUpdate}>
                    <IconDownload className="h-4 w-4" /> Download &amp; install
                  </Button>
                </div>
              ) : (
                <div className="flex flex-wrap items-center gap-3">
                  <Button variant="secondary" disabled={updateChecking} onClick={doCheckForUpdate}>
                    {updateChecking ? <Spinner className="h-4 w-4" /> : null}
                    {updateChecking ? "Checking..." : "Check for updates"}
                  </Button>
                  {updateChecked && !updateError && <span className="text-xs text-slate-400 dark:text-slate-500">You're on the latest version.</span>}
                </div>
              )}
              {updateError && <p className="mt-3 text-xs text-red-600 dark:text-red-400">{updateError}</p>}
              {appInfo && <p className="mt-4 text-xs text-slate-400 dark:text-slate-500">TIQR Manager v{appInfo.version}</p>}
            </Card>
          )}

          {/* Real Firebase auth as of 2.0.45 (email/password) and 2.0.46
              (Google) - see lib/auth.tsx's own doc comment. (This comment
              used to say "placeholder auth, not real Firebase yet" - that
              was true back in 2.0.44 Phase 1 and went stale; caught while
              touching this section for 2.0.48.) */}
          {section === "account" && (
            <Card className="p-5 lg:max-w-xl">
              <h3 className="mb-1 text-sm font-semibold text-slate-800 dark:text-slate-200">Your profile</h3>
              {/* 2.0.48: this caption used to sit under the sidebar profile
                  widget (Layout.tsx) instead - marko wanted it off the
                  sidebar (shown on every page, all the time) and moved to
                  the one place someone would actually go looking for it. */}
              <p className="mb-4 text-xs text-slate-400 dark:text-slate-500">Local-first &middot; your data stays on this device</p>
              <div className="space-y-3">
                <Field label="Name">
                  <Input value={accountName} onChange={(e) => setAccountName(e.target.value)} placeholder="Your name" />
                </Field>
                <Field
                  label="Email"
                  hint={user?.provider === "google" ? "Signed in with Google" : "Signed in with email and password"}
                >
                  <Input value={user?.email ?? ""} disabled />
                </Field>
                <Button
                  variant="primary"
                  disabled={!accountName.trim() || accountName.trim() === user?.name || savingName}
                  onClick={async () => {
                    setSavingName(true);
                    try {
                      await updateName(accountName.trim());
                      toast.success("Profile updated.");
                    } catch (err) {
                      toast.error(firebaseAuthErrorMessage(err));
                    } finally {
                      setSavingName(false);
                    }
                  }}
                >
                  {savingName ? <Spinner className="h-4 w-4" /> : "Save"}
                </Button>
              </div>

              <div className="mt-6 border-t border-slate-100 pt-4 dark:border-slate-800">
                <Button
                  variant="danger"
                  disabled={loggingOut}
                  onClick={async () => {
                    setLoggingOut(true);
                    try {
                      await logout();
                    } catch {
                      toast.error("Couldn't log out - try again.");
                      setLoggingOut(false);
                    }
                  }}
                >
                  {loggingOut ? <Spinner className="h-4 w-4" /> : <IconLogOut className="h-4 w-4" />} Log out
                </Button>
              </div>
            </Card>
          )}
        </>
      )}

      <CsvImportModal open={importOpen} onClose={() => setImportOpen(false)} onImported={reload} />

      <ExportPickerModal open={!!exportConfig} config={exportConfig} onClose={() => setExportConfig(null)} />

      <ConfirmDialog
        open={!!confirmRestorePath}
        title="Restore this backup?"
        message={
          <>
            {/* 2.0.72: names the signed-in account explicitly, now that more
                than one account's data can exist on this computer - a second
                account restoring a backup file that happens to belong to a
                different account would otherwise be an easy mistake to make,
                since this dialog used to only ever mean "your one and only
                database." */}
            <b>{user?.email ?? "This account"}'s current data will be replaced</b> with the contents of{" "}
            <span className="break-all font-mono text-xs">{confirmRestorePath}</span>. The app will relaunch
            automatically. This cannot be undone — make sure this backup file actually belongs to{" "}
            {user?.email ?? "this account"} before continuing.
          </>
        }
        confirmLabel="Restore & relaunch"
        danger
        busy={busyAction === "restore"}
        onCancel={() => setConfirmRestorePath(null)}
        onConfirm={doRestore}
      />

      <ConfirmDialog
        open={!!confirmDeletePlatform}
        title="Remove this platform?"
        message={
          <>
            Removes <b>{confirmDeletePlatform?.name}</b> from the platform list. Any existing orders/sales that
            used it keep their cost/revenue amounts - they just lose the platform label.
          </>
        }
        confirmLabel="Remove platform"
        danger
        busy={deletingLookup}
        onCancel={() => setConfirmDeletePlatform(null)}
        onConfirm={async () => {
          if (!confirmDeletePlatform) return;
          setDeletingLookup(true);
          try {
            await api.deletePlatform(confirmDeletePlatform.id);
            setConfirmDeletePlatform(null);
            reload();
          } catch (e) {
            toast.error(errMsg(e));
          } finally {
            setDeletingLookup(false);
          }
        }}
      />

      <ConfirmDialog
        open={!!confirmDeleteCategory}
        title="Remove this category?"
        message={
          <>
            Removes <b>{confirmDeleteCategory?.name}</b> from the category list. Any events tagged with it lose the
            category label - their orders, tickets, sales and money figures are completely unaffected.
          </>
        }
        confirmLabel="Remove category"
        danger
        busy={deletingLookup}
        onCancel={() => setConfirmDeleteCategory(null)}
        onConfirm={async () => {
          if (!confirmDeleteCategory) return;
          setDeletingLookup(true);
          try {
            await api.deleteEventCategory(confirmDeleteCategory.id);
            setConfirmDeleteCategory(null);
            reload();
          } catch (e) {
            toast.error(errMsg(e));
          } finally {
            setDeletingLookup(false);
          }
        }}
      />

      <ConfirmDialog
        open={!!confirmDeleteFinanceCategory}
        title="Remove this category?"
        message={
          <>
            Removes <b>{confirmDeleteFinanceCategory?.name}</b> from the Finance category list. Any entries using it
            keep their amount and date - they just lose the category label.
          </>
        }
        confirmLabel="Remove category"
        danger
        busy={deletingLookup}
        onCancel={() => setConfirmDeleteFinanceCategory(null)}
        onConfirm={async () => {
          if (!confirmDeleteFinanceCategory) return;
          setDeletingLookup(true);
          try {
            await api.deleteFinanceCategory(confirmDeleteFinanceCategory.id);
            setConfirmDeleteFinanceCategory(null);
            reload();
          } catch (e) {
            toast.error(errMsg(e));
          } finally {
            setDeletingLookup(false);
          }
        }}
      />

    </div>
  );
}

/** 1.9.3: one of the two platform lists (Purchase / Selling) on the Lookups
 * page - same list backs both, filtered by `kind` (a "both"-tagged platform
 * appears in both, by design). Owns its own "add" input state since the two
 * lists are otherwise independent of each other. The per-row Select lets an
 * existing platform be re-tagged after the fact (mainly useful right after
 * upgrading: every platform created before 1.9.3 defaulted to "both" and so
 * starts out in both lists, until re-tagged here). */
/** 2.0.73: a small "(i)" glyph that carries a longer explanation as a native
 * hover tooltip (plain `title` attribute - every browser/webview already
 * knows how to show this, no extra tooltip library needed for one line of
 * text). Lets a heading stay explained without a permanent paragraph of text
 * underneath it - see Settings -> Lookups for where this replaced one. */
function InfoHint({ text }: { text: string }) {
  return (
    <span title={text} className="inline-flex cursor-help text-slate-300 dark:text-slate-600">
      <IconInfo className="h-3.5 w-3.5" />
    </span>
  );
}

function PlatformList({
  heading,
  kind,
  platforms,
  onAdd,
  onDelete,
  onChangeKind,
}: {
  heading: string;
  kind: "purchase" | "sale";
  platforms: Platform[];
  onAdd: (name: string, kind: "purchase" | "sale") => void;
  onDelete: (platform: Platform) => void;
  onChangeKind: (platform: Platform, kind: "purchase" | "sale" | "both") => void;
}) {
  const [value, setValue] = useState("");
  const visible = platforms.filter((p) => p.kind === kind || p.kind === "both");

  const add = () => {
    if (!value.trim()) return;
    onAdd(value.trim(), kind);
    setValue("");
  };

  return (
    <div>
      <h4 className="mb-2 text-xs font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">{heading}</h4>
      <div className="mb-2 flex gap-2">
        <Input
          placeholder="e.g. Ticketmaster"
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && add()}
        />
        <Button onClick={add}>Add</Button>
      </div>
      <ul className="max-h-56 divide-y divide-slate-100 dark:divide-slate-800 overflow-y-auto rounded-lg border border-slate-100 dark:border-slate-800">
        {visible.length === 0 && <li className="p-3 text-sm text-slate-400 dark:text-slate-500">No platforms yet</li>}
        {visible.map((p) => (
          <li key={p.id} className="flex items-center justify-between gap-2 px-3 py-2 text-sm">
            <span className="truncate">{p.name}</span>
            <div className="flex shrink-0 items-center gap-2">
              <div className="w-[112px]">
                <Select value={p.kind} onChange={(e) => onChangeKind(p, e.target.value as "purchase" | "sale" | "both")}>
                  <option value="purchase">Purchase only</option>
                  <option value="sale">Selling only</option>
                  <option value="both">Both</option>
                </Select>
              </div>
              <button
                className="text-slate-300 dark:text-slate-600 hover:text-red-600 dark:hover:text-red-400"
                title="Remove"
                onClick={() => onDelete(p)}
              >
                <IconTrash className="h-4 w-4" />
              </button>
            </div>
          </li>
        ))}
      </ul>
    </div>
  );
}

/** 2.0.27: Settings -> Lookups' Event Categories list - one list (unlike
 * PlatformList above, a category has no Purchase/Selling "kind" split).
 * Shows each category's already-assigned color as a small swatch next to
 * its name (a full EventCategoryBadge here would just repeat the name
 * that's already the row's own text, e.g. "Concert [Concert]") - see
 * EventCategorySwatch (EventCategoryBadge.tsx). */
function EventCategoryList({
  categories,
  onAdd,
  onDelete,
}: {
  categories: EventCategory[];
  onAdd: (name: string) => void;
  onDelete: (category: EventCategory) => void;
}) {
  const [value, setValue] = useState("");

  const add = () => {
    if (!value.trim()) return;
    onAdd(value.trim());
    setValue("");
  };

  return (
    <div>
      <div className="mb-2 flex gap-2">
        <Input
          placeholder="e.g. Football"
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && add()}
        />
        <Button onClick={add}>Add</Button>
      </div>
      <ul className="max-h-56 divide-y divide-slate-100 dark:divide-slate-800 overflow-y-auto rounded-lg border border-slate-100 dark:border-slate-800">
        {categories.length === 0 && <li className="p-3 text-sm text-slate-400 dark:text-slate-500">No categories yet</li>}
        {categories.map((c) => (
          <li key={c.id} className="flex items-center justify-between gap-2 px-3 py-2 text-sm">
            <span className="flex min-w-0 items-center gap-2">
              <EventCategorySwatch colorSlot={c.colorSlot} />
              <span className="truncate">{c.name}</span>
            </span>
            <button
              className="shrink-0 text-slate-300 dark:text-slate-600 hover:text-red-600 dark:hover:text-red-400"
              title="Remove"
              onClick={() => onDelete(c)}
            >
              <IconTrash className="h-4 w-4" />
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}

/** 2.0.83: one of the two Finance category lists (Expense/Income) - same
 * "one list filtered by kind, own add-input state" shape as `PlatformList`
 * above (kind = 'purchase'/'sale' there, 'expense'/'income' here), with
 * `EventCategoryList`'s swatch-next-to-name display (a category's color is
 * assigned automatically, nothing to pick here either). A category tagged
 * 'both' would appear in both lists, same as a 'both' platform does -
 * nothing in the starter set (migrations/015_finance.sql) uses it, and this
 * form itself only ever creates 'expense'/'income' (not 'both') to keep the
 * two-list UI simple - the 'both' kind stays fully supported by the schema
 * and backend either way. */
function FinanceCategoryList({
  heading,
  kind,
  categories,
  onAdd,
  onDelete,
}: {
  heading: string;
  kind: "expense" | "income";
  categories: FinanceCategory[];
  onAdd: (name: string, kind: "expense" | "income") => void;
  onDelete: (category: FinanceCategory) => void;
}) {
  const [value, setValue] = useState("");
  const visible = categories.filter((c) => c.kind === kind || c.kind === "both");

  const add = () => {
    if (!value.trim()) return;
    onAdd(value.trim(), kind);
    setValue("");
  };

  return (
    <div>
      <h4 className="mb-2 text-xs font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">{heading}</h4>
      <div className="mb-2 flex gap-2">
        <Input
          placeholder="e.g. Doprava"
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && add()}
        />
        <Button onClick={add}>Add</Button>
      </div>
      <ul className="max-h-56 divide-y divide-slate-100 dark:divide-slate-800 overflow-y-auto rounded-lg border border-slate-100 dark:border-slate-800">
        {visible.length === 0 && <li className="p-3 text-sm text-slate-400 dark:text-slate-500">No categories yet</li>}
        {visible.map((c) => (
          <li key={c.id} className="flex items-center justify-between gap-2 px-3 py-2 text-sm">
            <span className="flex min-w-0 items-center gap-2">
              <FinanceCategorySwatch colorSlot={c.colorSlot} />
              <span className="truncate">{c.name}</span>
            </span>
            <button
              className="shrink-0 text-slate-300 dark:text-slate-600 hover:text-red-600 dark:hover:text-red-400"
              title="Remove"
              onClick={() => onDelete(c)}
            >
              <IconTrash className="h-4 w-4" />
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}

// 2.0.2: Settings -> Integrations card for one data source ("pulls" today;
// "tickets" will reuse this same component once its row-sync logic exists -
// the connection commands underneath are already fully generic per data
// source, see sheets_sync.rs). Deliberately only sets up and tests the
// connection - no row import/export UI yet, see REDESIGN-2.0.2-REPORT.md for
// why that's a separate, later pass.
/// 2.0.5: installation-wide "Sign in with Google" - one signed-in account
/// per copy of the app (not per data source), sitting above the
/// per-data-source cards below. Purely additive: signing in changes *which*
/// credential SheetsConnectionCard's Connect/Create/Sync now/Test calls use
/// underneath (see commands::google_auth::resolve_google_credential's doc
/// comment) - the shared service account keeps working unchanged for
/// anyone who never signs in.
function GoogleSignInCard({ onChange }: { onChange: (status: GoogleSignInStatus) => void }) {
  const toast = useToast();
  const [status, setStatus] = useState<GoogleSignInStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState<"in" | "out" | null>(null);

  const reload = () => {
    setLoading(true);
    api
      .getGoogleSignInStatus()
      .then((s) => {
        setStatus(s);
        onChange(s);
      })
      .catch((e) => toast.error(errMsg(e)))
      .finally(() => setLoading(false));
  };

  useEffect(reload, []);

  const doSignIn = async () => {
    setBusy("in");
    try {
      const result = await api.startGoogleSignIn();
      setStatus(result);
      onChange(result);
      toast.success(`Signed in as ${result.signedInEmail}`);
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setBusy(null);
    }
  };

  // 2.0.12: marko's own report - closing the browser tab (or picking "use
  // another account" and never finishing there) left this card stuck reading
  // "Waiting for you to finish in your browser..." for the full 5-minute
  // timeout with no way back into the app, which read as a frozen app and
  // needing a restart. Does not itself clear `busy`/show a result - doSignIn's
  // own `finally` above does that once its now-interrupted promise actually
  // settles, moments after the backend notices the flag this sets (see
  // accept_one_redirect's doc comment) - so the button/spinner flips back to
  // normal exactly once.
  const doCancelSignIn = async () => {
    try {
      await api.cancelGoogleSignIn();
    } catch (e) {
      toast.error(errMsg(e));
    }
  };

  const doSignOut = async () => {
    setBusy("out");
    try {
      await api.googleSignOut();
      toast.success("Signed out");
      reload();
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setBusy(null);
    }
  };

  if (loading) {
    return (
      <Card className="p-5">
        <div className="flex items-center gap-2 text-sm text-slate-400 dark:text-slate-500">
          <Spinner className="h-4 w-4" /> Loading...
        </div>
      </Card>
    );
  }

  const signedIn = !!status?.signedInEmail;

  return (
    <Card className="p-5">
      <div className="mb-1 flex flex-wrap items-center gap-2">
        <h3 className="text-sm font-semibold text-slate-800 dark:text-slate-200">Sign in with Google</h3>
        {signedIn && <Badge tone="sold">Signed in</Badge>}
      </div>

      {!status?.signInAvailable ? (
        <p className="text-xs text-slate-400 dark:text-slate-500">Google sign-in isn&apos;t available in this build.</p>
      ) : signedIn ? (
        <>
          <p className="mb-3 text-xs text-slate-400 dark:text-slate-500">
            Connecting or creating a sheet below now uses your own Google account (
            <span className="break-all font-mono text-slate-500 dark:text-slate-400">{status.signedInEmail}</span>)
            instead of the app&apos;s shared account - no separate sharing step needed for a sheet you create.
          </p>
          <Button variant="ghost" disabled={busy === "out"} onClick={doSignOut}>
            {busy === "out" ? <Spinner className="h-4 w-4" /> : null}
            Sign out
          </Button>
        </>
      ) : (
        <>
          <p className="mb-3 text-xs text-slate-400 dark:text-slate-500">
            Optional. Sign in with your own Google account so connecting or creating a Pulls sheet below uses your
            identity instead of the app&apos;s shared one. Opens your own browser - nothing happens inside the app
            itself, and nobody but you sees your Google password.
          </p>
          <div className="flex flex-wrap items-center gap-2">
            <Button variant="primary" disabled={busy === "in"} onClick={doSignIn}>
              {busy === "in" ? <Spinner className="h-4 w-4" /> : <IconLink className="h-4 w-4" />}
              {busy === "in" ? "Waiting for you to finish in your browser..." : "Sign in with Google"}
            </Button>
            {busy === "in" && (
              <Button variant="ghost" onClick={doCancelSignIn}>
                Cancel
              </Button>
            )}
          </div>
        </>
      )}
    </Card>
  );
}

// 2.1.6: optional Anthropic API key, powering Price Checker's new
// AI-assisted extraction fallback (commands/price_checker_auto.rs's
// try_ai_extraction_fallback) - used only as a LAST RESORT, when
// Auto-check's own 4 free rule-based passes can't recognize a page's
// prices at all. Nothing about Price Checker changes without a key saved
// here; every existing free pass keeps working exactly as before, and nothing
// is ever sent to Anthropic unless this is configured.
//
// Same "collapse once configured, secret never round-trips to the frontend"
// shape as NotificationsCard's ntfy topic (see that component's own doc
// comment) - `getAnthropicApiKeyConfigured` only ever returns a bool, never
// the key itself, and the field always starts blank. UNLIKE ntfy's topic
// field, though, `setAnthropicApiKey` takes a plain (not `Option<String>`)
// string where blank always means "clear" (settings.rs) - it has no way to
// mean "leave whatever's already saved alone". So this can't reuse ntfy's
// "leave the field blank on Save to keep the existing value" convention -
// blank+Save would delete it here, not skip it. Instead: a configured key
// collapses to a summary row with separate "Change key"/"Remove" actions,
// and only "Remove" ever submits a blank key - Save in the open form always
// sends whatever's actually typed, and is disabled/no-ops on blank input.
function AnthropicApiKeyCard() {
  const toast = useToast();
  const [configured, setConfigured] = useState<boolean | null>(null);
  const [editing, setEditing] = useState(false);
  const [key, setKey] = useState("");
  const [saving, setSaving] = useState(false);
  const [removing, setRemoving] = useState(false);

  const load = () => {
    api
      .getAnthropicApiKeyConfigured()
      .then((c) => {
        setConfigured(c);
        setEditing(!c);
      })
      .catch((e) => toast.error(errMsg(e)));
  };
  useEffect(load, []);

  const doSave = async () => {
    if (!key.trim()) {
      toast.error("Paste your Anthropic API key first, or use Remove below to clear a saved one.");
      return;
    }
    setSaving(true);
    try {
      await api.setAnthropicApiKey(key.trim());
      setKey("");
      setConfigured(true);
      setEditing(false);
      toast.success("Anthropic API key saved");
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setSaving(false);
    }
  };

  const doRemove = async () => {
    setRemoving(true);
    try {
      await api.setAnthropicApiKey("");
      setKey("");
      setConfigured(false);
      setEditing(true);
      toast.success("Anthropic API key removed");
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setRemoving(false);
    }
  };

  if (configured === null) {
    return (
      <Card className="p-5">
        <div className="flex items-center gap-2 text-sm text-slate-400 dark:text-slate-500">
          <Spinner className="h-4 w-4" /> Loading...
        </div>
      </Card>
    );
  }

  return (
    <Card className="p-5">
      <div className="mb-1 flex flex-wrap items-center gap-2">
        <h3 className="text-sm font-semibold text-slate-800 dark:text-slate-200">AI-assisted price reading</h3>
        <Badge tone={configured ? "sold" : "available"}>{configured ? "On" : "Off"}</Badge>
      </div>
      <p className="mb-4 text-xs text-slate-400 dark:text-slate-500">
        Optional. When Price Checker&apos;s Auto-check can&apos;t recognize a page&apos;s prices on its own, it can
        ask Claude (Anthropic&apos;s AI) to read them instead - only as a last resort, for pages the free method
        already failed on. AI-derived results are always shown clearly marked in Price Checker, so you can
        double-check them before saving.
      </p>

      {configured && !editing ? (
        <div className="flex flex-wrap items-center justify-between gap-3 rounded-lg border border-slate-200 bg-slate-50 px-3 py-2 dark:border-slate-800 dark:bg-slate-800/40">
          <p className="text-xs text-slate-500 dark:text-slate-400">A key is saved.</p>
          <div className="flex items-center gap-2">
            <Button variant="ghost" onClick={() => setEditing(true)}>
              Change key
            </Button>
            <Button variant="ghost" disabled={removing} onClick={doRemove}>
              {removing ? <Spinner className="h-4 w-4" /> : null}
              Remove
            </Button>
          </div>
        </div>
      ) : (
        <div className="space-y-3">
          <div className="max-w-sm">
            <Field label="Anthropic API key">
              <Input type="password" autoComplete="off" placeholder="sk-ant-..." value={key} onChange={(e) => setKey(e.target.value)} />
            </Field>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <Button variant="primary" disabled={saving} onClick={doSave}>
              {saving ? <Spinner className="h-4 w-4" /> : null}
              Save
            </Button>
            {configured && (
              <Button
                variant="ghost"
                onClick={() => {
                  setKey("");
                  setEditing(false);
                }}
              >
                Cancel
              </Button>
            )}
          </div>
        </div>
      )}
    </Card>
  );
}

// 2.0.10: pulled out of SheetsConnectionCard so a card with two sync actions
// (Orders & Sales: Order sync / Sales sync) can render both results without
// duplicating this conflicts/errors markup twice.
function SyncResultView({ result }: { result: SheetSyncResult }) {
  return (
    <div className="rounded-lg border border-slate-200 dark:border-slate-800 p-3">
      <p className="text-xs text-slate-600 dark:text-slate-300">
        Created <b>{result.created}</b>, updated <b>{result.updated}</b>, unchanged <b>{result.unchanged}</b>.
      </p>
      {result.conflicts.length > 0 && (
        <div className="mt-2 max-h-40 overflow-y-auto">
          <p className="text-xs font-medium text-amber-700 dark:text-amber-400">
            {result.conflicts.length} row{result.conflicts.length === 1 ? "" : "s"} need
            {result.conflicts.length === 1 ? "s" : ""} your attention:
          </p>
          {result.conflicts.map((c, i) => (
            <p key={i} className="mt-0.5 text-xs text-amber-700 dark:text-amber-400">
              Row {c.rowNumber}: {c.message}
            </p>
          ))}
        </div>
      )}
      {result.errors.length > 0 && (
        <div className="mt-2 max-h-40 overflow-y-auto">
          <p className="text-xs font-medium text-red-600 dark:text-red-400">
            {result.errors.length} row{result.errors.length === 1 ? "" : "s"} skipped:
          </p>
          {result.errors.map((e, i) => (
            <p key={i} className="mt-0.5 text-xs text-red-600 dark:text-red-400">
              Row {e.rowNumber}: {e.message}
            </p>
          ))}
        </div>
      )}
      {result.corrected.length > 0 && (
        <div className="mt-2 max-h-40 overflow-y-auto">
          <p className="text-xs font-medium text-emerald-700 dark:text-emerald-400">
            {result.corrected.length} row{result.corrected.length === 1 ? "" : "s"} auto-corrected - no action needed:
          </p>
          {result.corrected.map((c, i) => (
            <p key={i} className="mt-0.5 text-xs text-emerald-700 dark:text-emerald-400">
              Row {c.rowNumber}: {c.message}
            </p>
          ))}
        </div>
      )}
    </div>
  );
}

// 2.0.16: marko's own request - Currency is no longer a per-connection
// choice; it never really needed to be. Orders & Sales rows already carry
// and use their own currency column automatically (see orders_sheet_sync.rs
// - unaffected by this change); Pulls has no currency column at all, so this
// fixed value is now simply what the app assumes wherever a sheet doesn't
// say otherwise - Pulls always, Orders & Sales only for a row whose currency
// cell is left blank. Change this one constant (nowhere else) if that
// assumption ever needs to be anything other than EUR.
const FIXED_CONNECTION_CURRENCY = "EUR";

function SheetsConnectionCard({
  dataSource,
  label,
  onSync,
  syncLabel,
  syncDescription,
  secondarySync,
  onPush,
  pushLabel,
  pushDescription,
  secondaryPush,
  forcePush,
  onSetup,
  setupDescription,
  onCreate,
  googleStatus,
}: {
  dataSource: string;
  label: string;
  /** The "Sync now" call for this data source, e.g. api.syncPulls /
   * api.syncOrders. 2.0.3: only Pulls had this at first; 2.0.8 generalized it
   * to a prop (was a hardcoded api.syncPulls() call) so a second data source
   * could bring its own sync function without this component needing to
   * know which one it is. Omit for a data source with no sync logic yet
   * (connection-only) - the card still lets you connect/test but hides this
   * button entirely. */
  onSync?: () => Promise<SheetSyncResult>;
  /** Button text for `onSync` - defaults to "Sync now" (Pulls, and every
   * other single-action card, omits this). 2.0.10: Orders & Sales sets
   * this to "Order sync" now that it has a second action (`secondarySync`)
   * on the same card, so the two buttons read clearly side by side. */
  syncLabel?: string;
  /** Shown next to the sync button explaining exactly what it does for this
   * data source - each one behaves differently (Pulls creates+updates;
   * Orders v1 only ever creates, see commands/orders_sheet_sync.rs's module
   * doc comment) so one shared sentence would misdescribe at least one of
   * them. Required whenever `onSync` is set. */
  syncDescription?: string;
  /** 2.0.10: an optional SECOND sync action on the same card/connection -
   * marko's own request, so Orders & Sales (one physical sheet, two
   * batches of columns) never needs a second, separate connection just to
   * offer "Sales sync" alongside "Order sync" on the Orders & Sales card.
   * Fully independent of `onSync`/`syncLabel`/`syncDescription` - own
   * button, own description, own result block - so this stays a plain,
   * generic "up to two named sync actions" card rather than anything
   * Orders-specific. Omit entirely for a card with only one sync action
   * (e.g. Pulls). */
  secondarySync?: { label: string; description: string; run: () => Promise<SheetSyncResult> };
  /** 2.0.18: the app -> sheet direction, e.g. api.pushPulls / api.pushOrders -
   * a separate button next to `onSync`, never combined with it (marko's own
   * choice: "Dve tlačidlá" - two buttons - rather than one that does both
   * directions). Omit for a data source with no push logic yet. */
  onPush?: () => Promise<SheetSyncResult>;
  /** Button text for `onPush` - defaults to "Push to sheet". */
  pushLabel?: string;
  /** Shown next to the push button, same one-per-data-source rationale as
   * `syncDescription`. Required whenever `onPush` is set. */
  pushDescription?: string;
  /** 2.0.18: the push direction's own second action, mirroring
   * `secondarySync` exactly - Orders & Sales gets "Push sales" alongside
   * "Push orders" here, same as "Sales sync" sits alongside "Order sync"
   * above. */
  secondaryPush?: { label: string; description: string; run: () => Promise<SheetSyncResult> };
  /** 2.0.60 - marko's own request: a real sale made via the Dashboard's "New
   * sale" shortcut didn't make it into the sheet through "Push sales" above,
   * for a reason that couldn't be pinned down from the information available
   * (the order was already linked, every ticket sold at once at one
   * identical price, and the target cells were blank beforehand - by "Push
   * sales"' own "only if every cell is still blank" rule, it should already
   * have written). Rather than keep guessing against marko's real
   * spreadsheet, this is a third, separate action that drops that rule: it
   * corrects whichever of the same columns currently disagree with what the
   * app knows, cell by cell, so it can also repair a row "Push sales" missed
   * for any similar reason in the future. Unlike every other sync/push
   * action on this card, this ONE can overwrite something already in the
   * sheet, so the card gates it behind a confirmation dialog
   * (`confirmMessage`) before running it. Omit for a data source with no
   * such repair action (currently only Orders & Sales has one). */
  forcePush?: { label: string; description: string; confirmMessage: string; run: () => Promise<SheetSyncResult> };
  /** 2.0.20: "Update sheet" - e.g. api.setupPullsSheet / api.setupOrdersSheet.
   * For a sheet/tab that's already connected (pasted URL/ID, not "Create a
   * new sheet for me" below) but turns out to have no header row yet - marko
   * hit exactly this after a real bug (see google_sheets.rs's
   * SpreadsheetMetadata doc comment) and asked for a way to bring it up to
   * the correct shape without disconnecting and starting over. Writes the
   * header only when the sheet is currently empty; always a safe click
   * otherwise. Omit for a data source with nothing to set up beyond the
   * header itself (none currently - both Pulls and Orders & Sales pass
   * this). */
  onSetup?: () => Promise<SheetSyncResult>;
  /** Shown next to the "Update sheet" button, same one-per-data-source
   * rationale as `syncDescription`/`pushDescription`. Required whenever
   * `onSetup` is set. */
  setupDescription?: string;
  /** "Create a new sheet for me", e.g. api.createPullsSheet /
   * api.createOrdersSheet. 2.0.8: generalized to a prop alongside onSync,
   * but kept independent of it - not every data source necessarily gets an
   * auto-create-a-blank-sheet flow. Omit to hide that whole section. */
  onCreate?: (email: string, currency: string) => Promise<CreatedSheetResult>;
  /** 2.0.5: from GoogleSignInCard via the parent Settings component - see
   * that state's own comment for why it is fetched once up there rather
   * than a second time in here. */
  googleStatus: GoogleSignInStatus | null;
}) {
  const toast = useToast();
  const [status, setStatus] = useState<SheetsConnectionStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [spreadsheetInput, setSpreadsheetInput] = useState("");
  const [sheetTab, setSheetTab] = useState("");
  const [createEmail, setCreateEmail] = useState("");
  const [createdUrl, setCreatedUrl] = useState<string | null>(null);
  const [busy, setBusy] = useState<
    "save" | "test" | "sync" | "sync2" | "push" | "push2" | "forcePush" | "setup" | "create" | "disconnect" | null
  >(null);
  const [testResult, setTestResult] = useState<SheetsConnectionTestResult | null>(null);
  const [syncResult, setSyncResult] = useState<SheetSyncResult | null>(null);
  // 2.0.10: result of `secondarySync`, kept fully separate from `syncResult`
  // above rather than one shared slot - so running both actions back to back
  // (e.g. Order sync then Sales sync) shows both outcomes at once instead of
  // the second overwriting the first.
  const [secondarySyncResult, setSecondarySyncResult] = useState<SheetSyncResult | null>(null);
  // 2.0.18: same "own slot per action" reasoning as secondarySyncResult
  // above, now for the push direction.
  const [pushResult, setPushResult] = useState<SheetSyncResult | null>(null);
  const [secondaryPushResult, setSecondaryPushResult] = useState<SheetSyncResult | null>(null);
  // 2.0.60: same "own slot per action" reasoning as secondaryPushResult
  // above, now for the "Fix sync" repair action - see `forcePush`'s own
  // comment.
  const [forcePushResult, setForcePushResult] = useState<SheetSyncResult | null>(null);
  // 2.0.20: same "own slot per action" reasoning as pushResult above, now for
  // the "Update sheet" button.
  const [setupResult, setSetupResult] = useState<SheetSyncResult | null>(null);
  const [confirmDisconnect, setConfirmDisconnect] = useState(false);
  // 2.0.60: gates `forcePush` behind a confirmation dialog, unlike every
  // other action on this card - see that prop's own comment for why.
  const [confirmForcePush, setConfirmForcePush] = useState(false);
  // 2.0.14: "Sheet/tab name" used to be free-text only, and marko's own
  // reports (twice) showed that typing the exact tab name by hand - even
  // once told what it should be - was itself the recurring failure, not just
  // a lack of explanation. `detectedTabs` holds the spreadsheet's real tab
  // names once known (null = not detected yet, or detection failed/isn't
  // possible right now - see `detectMessage`); when set and non-empty the
  // field below renders as a dropdown of real tabs instead of an <Input>.
  // `manualTabEntry` is an explicit escape hatch back to free text (e.g. the
  // tab doesn't exist yet, or the user just prefers typing it).
  const [detectedTabs, setDetectedTabs] = useState<string[] | null>(null);
  const [detecting, setDetecting] = useState(false);
  const [detectMessage, setDetectMessage] = useState<string | null>(null);
  // 2.0.15: the concrete next step (e.g. the exact e-mail to share with),
  // shown as its own line below `detectMessage` - see
  // SpreadsheetTabsResult's doc comment for why this is a separate field
  // rather than one long string.
  const [detectHint, setDetectHint] = useState<string | null>(null);
  const [manualTabEntry, setManualTabEntry] = useState(false);
  // 2.0.65: marko's own report - too many buttons visible at once, no way to
  // tell what they do without hovering each one, and the setup instructions/
  // input fields stayed on screen forever even once already connected and
  // working. Once connected, the connection itself (URL/tab fields, the
  // paste/share instructions, Save/Update sheet) collapses into one compact
  // summary line - `editingConnection` reopens it, e.g. to point this data
  // source at a different sheet. Not connected yet, there's nothing to
  // collapse, so the full form always shows regardless of this flag (see
  // `showConnectionForm` below). Every button this replaces still exists,
  // unchanged - see REDESIGN-2.0.65-REPORT.md.
  const [editingConnection, setEditingConnection] = useState(false);

  const oauthEmail = googleStatus?.signedInEmail ?? null;

  // 2.0.14: runs the spreadsheet through detect_spreadsheet_tabs - called on
  // blur of the URL/ID field below, and once on load whenever a connection
  // already exists (so reopening Settings on a sheet that was saved with the
  // wrong tab name shows the real options immediately, with no retyping of
  // the URL needed). Deliberately swallows network errors into
  // `detectMessage` rather than toasting - an incomplete paste or a
  // not-yet-shared sheet is the ordinary state of a half-filled form, not a
  // failure worth interrupting the user over.
  const detectTabs = async (spreadsheetUrlOrId: string) => {
    const trimmed = spreadsheetUrlOrId.trim();
    if (!trimmed) {
      setDetectedTabs(null);
      setDetectMessage(null);
      setDetectHint(null);
      return;
    }
    setDetecting(true);
    try {
      const result: SpreadsheetTabsResult = await api.detectSpreadsheetTabs(trimmed);
      if (result.ok) {
        setDetectedTabs(result.tabs);
        setDetectMessage(null);
        setDetectHint(null);
        setManualTabEntry(false);
        // Keep the current value if it's actually one of the real tabs;
        // otherwise default to the first real one - this is exactly what
        // auto-corrects marko's reported mistake (spreadsheet file name
        // typed into the tab field) without him retyping anything.
        setSheetTab((current) => (result.tabs.includes(current) ? current : result.tabs[0]));
      } else {
        setDetectedTabs(null);
        setDetectMessage(result.message);
        setDetectHint(result.hint ?? null);
      }
    } catch (e) {
      setDetectedTabs(null);
      setDetectMessage(errMsg(e));
      setDetectHint(null);
    } finally {
      setDetecting(false);
    }
  };

  const reload = () => {
    setLoading(true);
    api
      .getSheetsConnectionStatus(dataSource)
      .then((s) => {
        setStatus(s);
        setSpreadsheetInput(s.connection?.spreadsheetId ?? "");
        setSheetTab(s.connection?.sheetTab ?? "");
        if (s.connection?.spreadsheetId) {
          detectTabs(s.connection.spreadsheetId);
        } else {
          setDetectedTabs(null);
          setDetectMessage(null);
          setDetectHint(null);
          setManualTabEntry(false);
        }
      })
      .catch((e) => toast.error(errMsg(e)))
      .finally(() => setLoading(false));
  };

  useEffect(reload, [dataSource]);

  const doConnect = async () => {
    setBusy("save");
    setTestResult(null);
    setCreatedUrl(null);
    try {
      await api.setSheetsConnection(dataSource, spreadsheetInput, sheetTab, FIXED_CONNECTION_CURRENCY);
      reload();
      // 2.0.12: immediately run the same check "Test connection" always ran,
      // right here - marko's own report. Saving used to report success
      // purely because the input LOOKED valid (a well-formed URL/ID, a
      // non-empty tab name) with no network call at all, so a tab name that
      // does not actually exist in the spreadsheet (a very easy mistake -
      // "sheet" is ambiguous between the spreadsheet FILE's own name and the
      // specific TAB inside it Google actually means, see
      // google_sheets::describe_error_response's doc comment) only surfaced
      // later, confusingly, on the first Sync click. This closes that gap.
      const result = await api.testSheetsConnection(dataSource);
      setTestResult(result);
      if (result.ok) {
        toast.success("Sheet connected");
        // 2.0.65: collapse back to the compact summary now that the
        // connection actually works - stay open on a failed test (below) so
        // marko can see the fields and the error together and fix it.
        setEditingConnection(false);
      } else {
        toast.error(`Saved, but the connection test failed: ${result.message}`);
      }
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setBusy(null);
    }
  };

  // 2.0.4: the alternative to pasting an existing sheet's URL above - creates
  // a brand-new sheet and connects it. 2.0.5: signed in with Google
  // (oauthEmail set), the new sheet is already the signed-in person's own -
  // no separate share step, no email field shown, so `email` here is just
  // `oauthEmail` itself (still a real address, trivially satisfies the
  // backend's validation, and is ignored server-side on that path anyway -
  // see create_pulls_sheet_impl's doc comment). Not signed in, this is
  // `createEmail` from the field below, exactly as in 2.0.4.
  const doCreate = async () => {
    if (!onCreate) return;
    setBusy("create");
    setCreatedUrl(null);
    try {
      const result = await onCreate(oauthEmail ?? createEmail, FIXED_CONNECTION_CURRENCY);
      setCreatedUrl(result.spreadsheetUrl);
      setCreateEmail("");
      toast.success(oauthEmail ? "New sheet created in your Google Drive" : "New sheet created and shared");
      reload();
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setBusy(null);
    }
  };

  const doTest = async () => {
    setBusy("test");
    setTestResult(null);
    try {
      setTestResult(await api.testSheetsConnection(dataSource));
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setBusy(null);
    }
  };

  const doSync = async () => {
    if (!onSync) return;
    setBusy("sync");
    setSyncResult(null);
    try {
      const result = await onSync();
      setSyncResult(result);
      toast.success(`Synced: ${result.created} created, ${result.updated} updated, ${result.unchanged} unchanged`);
      reload();
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setBusy(null);
    }
  };

  // 2.0.10 - see `secondarySync` prop's own comment.
  const doSecondarySync = async () => {
    if (!secondarySync) return;
    setBusy("sync2");
    setSecondarySyncResult(null);
    try {
      const result = await secondarySync.run();
      setSecondarySyncResult(result);
      toast.success(
        `${secondarySync.label}: ${result.created} created, ${result.updated} updated, ${result.unchanged} unchanged`,
      );
      reload();
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setBusy(null);
    }
  };

  // 2.0.18 - see `onPush` prop's own comment.
  const doPush = async () => {
    if (!onPush) return;
    setBusy("push");
    setPushResult(null);
    try {
      const result = await onPush();
      setPushResult(result);
      toast.success(`Pushed: ${result.created} added, ${result.updated} updated, ${result.unchanged} unchanged`);
      reload();
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setBusy(null);
    }
  };

  // 2.0.18 - see `secondaryPush` prop's own comment.
  const doSecondaryPush = async () => {
    if (!secondaryPush) return;
    setBusy("push2");
    setSecondaryPushResult(null);
    try {
      const result = await secondaryPush.run();
      setSecondaryPushResult(result);
      toast.success(
        `${secondaryPush.label}: ${result.created} added, ${result.updated} updated, ${result.unchanged} unchanged`,
      );
      reload();
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setBusy(null);
    }
  };

  // 2.0.60 - see `forcePush` prop's own comment. Unlike doPush/
  // doSecondaryPush, the button itself never calls this directly - it opens
  // the confirm dialog below first, and only that dialog's onConfirm calls
  // this, since this is the one action on this card that can overwrite a
  // cell that already has something in it.
  const doForcePush = async () => {
    if (!forcePush) return;
    setBusy("forcePush");
    setForcePushResult(null);
    try {
      const result = await forcePush.run();
      setForcePushResult(result);
      toast.success(
        `${forcePush.label}: ${result.created} added, ${result.updated} updated, ${result.unchanged} unchanged`,
      );
      reload();
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setBusy(null);
    }
  };

  // 2.0.20 - see `onSetup` prop's own comment.
  const doSetup = async () => {
    if (!onSetup) return;
    setBusy("setup");
    setSetupResult(null);
    try {
      const result = await onSetup();
      setSetupResult(result);
      toast.success(result.created > 0 ? "Sheet header written - it's now set up correctly" : "Sheet already set up correctly");
      reload();
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setBusy(null);
    }
  };

  const doDisconnect = async () => {
    setBusy("disconnect");
    try {
      await api.clearSheetsConnection(dataSource);
      setConfirmDisconnect(false);
      setTestResult(null);
      setSyncResult(null);
      setSecondarySyncResult(null);
      setPushResult(null);
      setSecondaryPushResult(null);
      setSetupResult(null);
      setCreatedUrl(null);
      toast.success("Sheet disconnected");
      reload();
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setBusy(null);
    }
  };

  if (loading) {
    return (
      <Card className="p-5">
        <div className="flex items-center gap-2 text-sm text-slate-400 dark:text-slate-500">
          <Spinner className="h-4 w-4" /> Loading...
        </div>
      </Card>
    );
  }

  const connected = !!status?.connection;

  return (
    <Card className="p-5">
      <div className="mb-1 flex flex-wrap items-center gap-2">
        <h3 className="text-sm font-semibold text-slate-800 dark:text-slate-200">{label}</h3>
        <Badge tone={connected ? "sold" : "available"}>{connected ? "Connected" : "Not connected"}</Badge>
      </div>

      {!status?.syncAvailable ? (
        <p className="text-xs text-slate-400 dark:text-slate-500">Google Sheets sync isn&apos;t available in this build.</p>
      ) : (
        <>
          {/* 2.0.65: once a connection already works, none of this setup
              chrome (instructions, the URL/tab fields, Save/Update sheet)
              needs to sit on screen permanently - marko's own report ("vela
              tlacitok...zbytocne vela textu"). Nothing here was removed, it
              just collapses into the compact summary bar below once
              `connected`, and "Change connection" reopens exactly this same
              block. Not yet connected, there's nothing to collapse, so this
              always shows. */}
          {(!connected || editingConnection) && (
            <>
              {/* 2.0.26: was up to 5 stacked paragraphs here (main + one per
                  action) - marko's own report ("zminimalizovať túto časť...
                  menej textu"). Nothing was deleted: every action's detailed
                  explanation (syncDescription/pushDescription/secondarySync.
                  description/secondaryPush.description/setupDescription) still
                  exists, word for word - it just moved onto that action's own
                  button as a native `title` tooltip (see the button row below)
                  instead of being force-displayed at all times. One short line
                  stays here since it's an instruction for the fields right
                  below, not a per-action explanation. */}
              <p className="mb-4 text-xs text-slate-400 dark:text-slate-500">
                Paste the sheet&apos;s URL (or just its ID) and the exact tab name, then connect.
                {!onSync &&
                  ` Reading and writing ${label.toLowerCase()} rows comes in a future update - this only sets up and tests the connection itself.`}
                {oauthEmail && " Uses your own signed-in Google account above, not the app's shared one."}
              </p>

              <div className="grid grid-cols-1 gap-3">
                <Field label="Spreadsheet URL or ID">
                  <Input
                    placeholder="https://docs.google.com/spreadsheets/d/..."
                    value={spreadsheetInput}
                    onChange={(e) => {
                      setSpreadsheetInput(e.target.value);
                      // A different spreadsheet invalidates any previously
                      // detected tab list - never leave a stale dropdown from
                      // the sheet that used to be pasted here.
                      setDetectedTabs(null);
                      setDetectMessage(null);
                      setDetectHint(null);
                    }}
                    onBlur={() => detectTabs(spreadsheetInput)}
                  />
                </Field>
                <Field
                  label="Sheet/tab name"
                  hint={
                    detectedTabs && detectedTabs.length > 0 && !manualTabEntry
                      ? "Detected directly from the spreadsheet - pick the tab this data source should use."
                      : "The tab at the bottom of the Google Sheet (Google calls this a 'sheet') - not the spreadsheet file's own name, which can look very similar. Must match exactly, including capitalization and spacing."
                  }
                >
                  {detecting ? (
                    <div className="input flex items-center gap-2 text-xs text-slate-400 dark:text-slate-500">
                      <Spinner className="h-4 w-4" /> Detecting tabs...
                    </div>
                  ) : detectedTabs && detectedTabs.length > 0 && !manualTabEntry ? (
                    <>
                      <Select value={sheetTab} onChange={(e) => setSheetTab(e.target.value)}>
                        {detectedTabs.map((t) => (
                          <option key={t} value={t}>
                            {t}
                          </option>
                        ))}
                      </Select>
                      <button
                        type="button"
                        className="mt-1 text-xs text-slate-500 underline decoration-dotted underline-offset-2 hover:text-slate-700 dark:text-slate-400 dark:hover:text-slate-200"
                        onClick={() => setManualTabEntry(true)}
                      >
                        Type it in manually instead
                      </button>
                    </>
                  ) : (
                    <>
                      <Input
                        placeholder={`e.g. ${label}`}
                        value={sheetTab}
                        onChange={(e) => setSheetTab(e.target.value)}
                      />
                      {detectMessage && (
                        <div className="mt-1">
                          <p className="text-xs text-amber-600 dark:text-amber-400">{detectMessage}</p>
                          {detectHint && <p className="mt-0.5 text-xs text-slate-500 dark:text-slate-400">{detectHint}</p>}
                        </div>
                      )}
                      {detectedTabs && detectedTabs.length > 0 && (
                        <button
                          type="button"
                          className="mt-1 text-xs text-slate-500 underline decoration-dotted underline-offset-2 hover:text-slate-700 dark:text-slate-400 dark:hover:text-slate-200"
                          onClick={() => setManualTabEntry(false)}
                        >
                          Pick from detected tabs instead
                        </button>
                      )}
                    </>
                  )}
                </Field>
              </div>

              {/* 2.0.26: marko's own report - buttons used to be one long
                  flex-wrap row that wrapped wherever width happened to run out,
                  mixing connection/sync/push together with no visual order. Now
                  3 short rows, grouped by what the button actually DOES (same
                  grouping he described): connect/verify/disconnect the sheet
                  itself, then read the sheet INTO the app, then send the app's
                  own data OUT to the sheet - each row only renders if this data
                  source actually has a button for that group (e.g. a
                  connection-only future data source with no onSync/onPush would
                  show just the first row). Every action's detailed explanation
                  lives in its own `title` (hover) now - see the comment above
                  this component's description paragraph. */}
              <div className="mt-3 flex flex-wrap items-center gap-2">
                <Button
                  variant="primary"
                  disabled={busy === "save" || !spreadsheetInput.trim() || !sheetTab.trim()}
                  onClick={doConnect}
                >
                  {busy === "save" ? <Spinner className="h-4 w-4" /> : <IconLink className="h-4 w-4" />}
                  {connected ? "Save" : "Connect"}
                </Button>
                {connected && (
                  <>
                    <Button variant="secondary" disabled={busy === "test"} onClick={doTest}>
                      {busy === "test" ? <Spinner className="h-4 w-4" /> : null}
                      Test connection
                    </Button>
                    {onSetup && (
                      <Button variant="secondary" disabled={busy === "setup"} onClick={doSetup} title={setupDescription}>
                        {busy === "setup" ? <Spinner className="h-4 w-4" /> : null}
                        {busy === "setup" ? "Updating..." : "Update sheet"}
                      </Button>
                    )}
                    <Button variant="ghost" onClick={() => setEditingConnection(false)}>
                      Done
                    </Button>
                    <Button
                      variant="ghost"
                      className="ml-auto"
                      disabled={busy === "disconnect"}
                      onClick={() => setConfirmDisconnect(true)}
                    >
                      Disconnect
                    </Button>
                  </>
                )}
              </div>
            </>
          )}

          {/* 2.0.65: the compact, default view once a connection already
              works - one line instead of the whole form above. Every
              action that isn't Sync/Push (Save, Test, Update sheet,
              Disconnect) is still one click away via "Change connection",
              not removed. */}
          {connected && !editingConnection && (
            <div className="mb-1 flex flex-wrap items-center justify-between gap-3 rounded-lg border border-slate-200 bg-slate-50 px-3 py-2 dark:border-slate-800 dark:bg-slate-800/40">
              <p className="min-w-0 truncate text-xs text-slate-500 dark:text-slate-400">
                Tab <span className="font-medium text-slate-700 dark:text-slate-200">&quot;{sheetTab}&quot;</span>
              </p>
              <div className="flex shrink-0 items-center gap-2">
                <Button variant="ghost" onClick={() => setEditingConnection(true)}>
                  Change connection
                </Button>
                <Button variant="secondary" disabled={busy === "test"} onClick={doTest}>
                  {busy === "test" ? <Spinner className="h-4 w-4" /> : null}
                  Test connection
                </Button>
              </div>
            </div>
          )}

          {connected && (onSync || onPush) && (
            <div className="mt-3 space-y-3">
              {onSync && (
                <div>
                  {/* 2.0.65: short captions + a direction icon (unused
                      elsewhere on this card until now) so marko can tell
                      what a row does at a glance, not just on hover - his
                      own report ("aby si vedel naco sluzia..a nemusel to
                      hladat"). */}
                  <p className="mb-1 flex items-center gap-1.5 text-xs font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">
                    <IconDownload className="h-3.5 w-3.5" /> Import from sheet
                  </p>
                  <div className="flex flex-wrap items-center gap-2">
                    <Button variant="secondary" disabled={busy === "sync"} onClick={doSync} title={syncDescription}>
                      {busy === "sync" ? <Spinner className="h-4 w-4" /> : null}
                      {busy === "sync" ? "Syncing..." : (syncLabel ?? "Sync now")}
                    </Button>
                    {secondarySync && (
                      <Button
                        variant="secondary"
                        disabled={busy === "sync2"}
                        onClick={doSecondarySync}
                        title={secondarySync.description}
                      >
                        {busy === "sync2" ? <Spinner className="h-4 w-4" /> : null}
                        {busy === "sync2" ? "Syncing..." : secondarySync.label}
                      </Button>
                    )}
                  </div>
                </div>
              )}

              {onPush && (
                <div>
                  <p className="mb-1 flex items-center gap-1.5 text-xs font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">
                    <IconUpload className="h-3.5 w-3.5" /> Send to sheet
                  </p>
                  <div className="flex flex-wrap items-center gap-2">
                    <Button variant="secondary" disabled={busy === "push"} onClick={doPush} title={pushDescription}>
                      {busy === "push" ? <Spinner className="h-4 w-4" /> : null}
                      {busy === "push" ? "Pushing..." : (pushLabel ?? "Push to sheet")}
                    </Button>
                    {secondaryPush && (
                      <Button
                        variant="secondary"
                        disabled={busy === "push2"}
                        onClick={doSecondaryPush}
                        title={secondaryPush.description}
                      >
                        {busy === "push2" ? <Spinner className="h-4 w-4" /> : null}
                        {busy === "push2" ? "Pushing..." : secondaryPush.label}
                      </Button>
                    )}
                    {forcePush && (
                      <Button
                        variant="ghost"
                        // 2.0.65: marko's own report asked every button to be
                        // understandable at a glance - this is the one action
                        // on the whole card that can overwrite something
                        // already in the sheet (see `forcePush`'s own prop
                        // comment), so it gets a visibly different treatment
                        // from its siblings, not just a hover tooltip. `!`
                        // (Tailwind's important-modifier) guarantees this
                        // wins over the ghost variant's own default text
                        // color regardless of utility class generation order.
                        className="!text-amber-700 hover:!bg-amber-50 dark:!text-amber-500 dark:hover:!bg-amber-500/10"
                        disabled={busy === "forcePush"}
                        onClick={() => setConfirmForcePush(true)}
                        title={forcePush.description}
                      >
                        {busy === "forcePush" ? (
                          <Spinner className="h-4 w-4" />
                        ) : (
                          <IconAlertTriangle className="h-4 w-4" />
                        )}
                        {busy === "forcePush" ? "Fixing..." : forcePush.label}
                      </Button>
                    )}
                  </div>
                </div>
              )}
            </div>
          )}

          {!connected && onCreate && (
            <>
              <div className="my-4 flex items-center gap-3">
                <div className="h-px flex-1 bg-slate-200 dark:bg-slate-800" />
                <span className="text-xs text-slate-400 dark:text-slate-500">or</span>
                <div className="h-px flex-1 bg-slate-200 dark:bg-slate-800" />
              </div>
              <p className="mb-3 text-xs text-slate-400 dark:text-slate-500">
                {oauthEmail
                  ? "Don't have a sheet yet? The app can create one for you - already set up with the right columns - directly in your own Google Drive."
                  : "Don't have a sheet yet? The app can create one for you - already set up with the right columns - and share it with your Google account. No Google sign-in window."}
              </p>
              <div className="flex flex-wrap items-end gap-3">
                {!oauthEmail && (
                  <div className="min-w-[240px] flex-1">
                    <Field label="Your email (to share the new sheet with)">
                      <Input
                        type="email"
                        placeholder="you@example.com"
                        value={createEmail}
                        onChange={(e) => setCreateEmail(e.target.value)}
                      />
                    </Field>
                  </div>
                )}
                <Button
                  variant="secondary"
                  disabled={busy === "create" || (!oauthEmail && !createEmail.trim())}
                  onClick={doCreate}
                >
                  {busy === "create" ? <Spinner className="h-4 w-4" /> : <IconPlus className="h-4 w-4" />}
                  {busy === "create" ? "Creating..." : "Create a new sheet for me"}
                </Button>
              </div>
            </>
          )}

          {testResult && (
            <div className="mt-3">
              <p className={`text-xs ${testResult.ok ? "text-emerald-600 dark:text-emerald-400" : "text-red-600 dark:text-red-400"}`}>
                {testResult.message}
              </p>
              {testResult.hint && <p className="mt-0.5 text-xs text-slate-500 dark:text-slate-400">{testResult.hint}</p>}
            </div>
          )}

          {syncResult && (
            <div className="mt-3">
              {/* 2.0.10: only labeled when there's a second action to tell it
                  apart from - Pulls' single-action card looks exactly as it
                  always did. */}
              {secondarySync && (
                <p className="mb-1 text-xs font-semibold text-slate-500 dark:text-slate-400">
                  {syncLabel ?? "Sync now"} result
                </p>
              )}
              <SyncResultView result={syncResult} />
            </div>
          )}

          {secondarySyncResult && (
            <div className="mt-3">
              <p className="mb-1 text-xs font-semibold text-slate-500 dark:text-slate-400">{secondarySync?.label} result</p>
              <SyncResultView result={secondarySyncResult} />
            </div>
          )}

          {pushResult && (
            <div className="mt-3">
              {secondaryPush && (
                <p className="mb-1 text-xs font-semibold text-slate-500 dark:text-slate-400">
                  {pushLabel ?? "Push to sheet"} result
                </p>
              )}
              <SyncResultView result={pushResult} />
            </div>
          )}

          {secondaryPushResult && (
            <div className="mt-3">
              <p className="mb-1 text-xs font-semibold text-slate-500 dark:text-slate-400">{secondaryPush?.label} result</p>
              <SyncResultView result={secondaryPushResult} />
            </div>
          )}

          {forcePushResult && (
            <div className="mt-3">
              <p className="mb-1 text-xs font-semibold text-slate-500 dark:text-slate-400">{forcePush?.label} result</p>
              <SyncResultView result={forcePushResult} />
            </div>
          )}

          {setupResult && (
            <div className="rounded-lg border border-slate-200 p-3 dark:border-slate-800 mt-3">
              <p className="mb-1 text-xs font-semibold text-slate-500 dark:text-slate-400">Update sheet result</p>
              <p className="text-xs text-slate-600 dark:text-slate-300">
                {setupResult.created > 0
                  ? "Header row was missing, so it was just written - the sheet is now set up correctly."
                  : "The header row was already there - nothing needed to change."}
              </p>
              {setupResult.errors.length > 0 && (
                <div className="mt-2 max-h-40 overflow-y-auto">
                  <p className="text-xs font-medium text-red-600 dark:text-red-400">
                    {setupResult.errors.length} thing{setupResult.errors.length === 1 ? "" : "s"} didn&apos;t go through:
                  </p>
                  {setupResult.errors.map((e, i) => (
                    <p key={i} className="mt-0.5 text-xs text-red-600 dark:text-red-400">
                      {e.message}
                    </p>
                  ))}
                </div>
              )}
            </div>
          )}

          {createdUrl && (
            <div className="mt-3 rounded-lg border border-emerald-200 bg-emerald-50 p-3 dark:border-emerald-900 dark:bg-emerald-950/40">
              <p className="text-xs font-medium text-emerald-700 dark:text-emerald-400">
                New sheet created and shared - tap to select the link, then copy it to open in your browser:
              </p>
              <p className="mt-1 select-all break-all font-mono text-xs text-emerald-800 dark:text-emerald-300">
                {createdUrl}
              </p>
            </div>
          )}

          {/* 2.0.65: same "collapse once connected" treatment as the setup
              form above - this is instructional text for CONNECTING, not
              information marko needs staring at a working connection.
              "Change connection" brings it back, same as the fields. */}
          {(!connected || editingConnection) &&
            (oauthEmail ? (
              <p className="mt-4 text-xs text-slate-400 dark:text-slate-500">
                Pasting an existing sheet&apos;s URL above needs that sheet to already be yours or shared with{" "}
                <span className="break-all font-mono text-slate-500 dark:text-slate-400">{oauthEmail}</span> (Editor
                access) - the same as sharing with any other collaborator in Google Sheets.
              </p>
            ) : (
              status?.serviceAccountEmail && (
                <p className="mt-4 text-xs text-slate-400 dark:text-slate-500">
                  Share the sheet with{" "}
                  <span className="break-all font-mono text-slate-500 dark:text-slate-400">{status.serviceAccountEmail}</span>{" "}
                  (Editor access) so the app can read and write it.
                </p>
              )
            ))}

          {status?.lastPushedAt && (
            <p className="mt-1 text-xs text-slate-400 dark:text-slate-500">
              Last pushed: {new Date(status.lastPushedAt).toLocaleString()}
            </p>
          )}

          {status?.lastSyncedAt && (
            <p className="mt-1 text-xs text-slate-400 dark:text-slate-500">
              Last synced: {new Date(status.lastSyncedAt).toLocaleString()}
            </p>
          )}
        </>
      )}

      {forcePush && (
        <ConfirmDialog
          open={confirmForcePush}
          title={`${forcePush.label}?`}
          message={<>{forcePush.confirmMessage}</>}
          confirmLabel={forcePush.label}
          danger
          busy={busy === "forcePush"}
          onCancel={() => setConfirmForcePush(false)}
          onConfirm={async () => {
            setConfirmForcePush(false);
            await doForcePush();
          }}
        />
      )}

      <ConfirmDialog
        open={confirmDisconnect}
        title={`Disconnect ${label}?`}
        message={
          <>
            The app forgets this sheet connection. Nothing in your {label.toLowerCase()} data is deleted - you can
            reconnect (even to the same sheet) any time.
          </>
        }
        confirmLabel="Disconnect"
        danger
        busy={busy === "disconnect"}
        onCancel={() => setConfirmDisconnect(false)}
        onConfirm={doDisconnect}
      />
    </Card>
  );
}

// 1.8.3 (section 9): a small clickable "count" chip for the import preview's
// summary row - Valid/Invalid, filtering the preview table below when
// clicked. Deliberately reuses the same tone convention as ui.tsx's Badge
// (emerald=good, red=problem) rather than inventing new colors.
function ImportSummaryChip({
  label,
  tone,
  active,
  onClick,
}: {
  label: string;
  tone: "neutral" | "valid" | "error";
  active: boolean;
  onClick: () => void;
}) {
  const toneCls =
    tone === "valid"
      ? "bg-emerald-50 text-emerald-700 ring-emerald-200 dark:bg-emerald-500/10 dark:text-emerald-400 dark:ring-emerald-500/30"
      : tone === "error"
        ? "bg-red-50 text-red-700 ring-red-200 dark:bg-red-500/10 dark:text-red-400 dark:ring-red-500/30"
        : "bg-slate-100 text-slate-700 ring-slate-300 dark:bg-slate-800 dark:text-slate-300 dark:ring-slate-600";
  return (
    <button
      type="button"
      onClick={onClick}
      className={`rounded-full px-3 py-1 text-xs font-medium ring-1 ring-inset transition-opacity ${toneCls} ${active ? "" : "opacity-60 hover:opacity-100"}`}
    >
      {label}
    </button>
  );
}

function CsvImportModal({
  open: isOpen,
  onClose,
  onImported,
}: {
  open: boolean;
  onClose: () => void;
  onImported: () => void;
}) {
  const toast = useToast();
  const [path, setPath] = useState<string | null>(null);
  const [preview, setPreview] = useState<CsvPreview | null>(null);
  const [loading, setLoading] = useState(false);
  const [importing, setImporting] = useState(false);
  // 1.8.3 (section 9): which preview rows are shown - set by clicking a
  // summary chip below. Preview-only; the actual import always covers every
  // row regardless of this filter (see confirmImport, unaffected).
  const [filterMode, setFilterMode] = useState<"all" | "valid" | "errors">("all");

  useEffect(() => {
    if (!isOpen) {
      setPath(null);
      setPreview(null);
      setFilterMode("all");
    }
  }, [isOpen]);

  const pickFile = async () => {
    const p = await open({ multiple: false, filters: [{ name: "CSV", extensions: ["csv"] }] });
    if (!p || Array.isArray(p)) return;
    setPath(p);
    setLoading(true);
    setPreview(null);
    setFilterMode("all");
    try {
      const res = await api.previewOrdersCsv(p);
      setPreview(res);
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setLoading(false);
    }
  };

  const visibleRows = preview
    ? preview.rows.filter((r) =>
        filterMode === "valid" ? r.errors.length === 0 : filterMode === "errors" ? r.errors.length > 0 : true,
      )
    : [];

  const confirmImport = async () => {
    if (!path) return;
    setImporting(true);
    try {
      const res = await api.importOrdersCsv(path);
      if (res.errors.length > 0) {
        toast.error(`Import failed: ${res.errors[0]}${res.errors.length > 1 ? ` (+${res.errors.length - 1} more)` : ""}`);
      } else {
        toast.success(`Imported ${res.importedOrders} orders (${res.importedTickets} tickets)`);
        onImported();
        onClose();
      }
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setImporting(false);
    }
  };

  return (
    <Modal open={isOpen} onClose={onClose} title="Import orders from CSV" width="max-w-3xl">
      {!path && (
        <div className="flex flex-col items-center gap-3 rounded-lg border border-dashed border-slate-300 dark:border-slate-700 py-10">
          <IconUpload className="h-8 w-8 text-slate-300 dark:text-slate-600" />
          <p className="text-sm text-slate-500 dark:text-slate-400">Choose a CSV file to preview before importing.</p>
          <Button variant="primary" onClick={pickFile}>
            Choose CSV file...
          </Button>
        </div>
      )}

      {loading && (
        <div className="flex items-center justify-center gap-2 py-10 text-sm text-slate-400 dark:text-slate-500">
          <Spinner className="h-4 w-4" /> Reading file...
        </div>
      )}

      {preview && !loading && (
        <div>
          <div className="mb-2 flex flex-wrap items-center gap-2">
            <ImportSummaryChip
              label={`${preview.rows.length} row${preview.rows.length === 1 ? "" : "s"}`}
              tone="neutral"
              active={filterMode === "all"}
              onClick={() => setFilterMode("all")}
            />
            <ImportSummaryChip
              label={`✓ ${preview.validCount} valid`}
              tone="valid"
              active={filterMode === "valid"}
              onClick={() => setFilterMode("valid")}
            />
            {preview.errorCount > 0 && (
              <ImportSummaryChip
                label={`✕ ${preview.errorCount} error${preview.errorCount === 1 ? "" : "s"}`}
                tone="error"
                active={filterMode === "errors"}
                onClick={() => setFilterMode("errors")}
              />
            )}
            <button className="ml-auto text-xs font-medium text-brand-600 dark:text-brand-400 hover:underline" onClick={pickFile}>
              Choose a different file
            </button>
          </div>
          <p className="mb-3 text-xs text-slate-400 dark:text-slate-500">
            Click a count above to filter the preview below.{" "}
            {/* 1.8.3 (section 9): the spec asked for a Duplicates count too, but
                this app has no reliable way to detect one - every imported row
                gets a freshly generated order/ticket code with nothing that ties
                it back to CSV content, so there's no signal to compare against.
                Saying so here rather than showing a count that would just be
                guessing. */}
            Duplicate rows aren&apos;t flagged - imported orders always get a fresh code, so there&apos;s no reliable
            way to tell whether a row matches something you already imported. Review the file itself if you&apos;re
            re-importing.
          </p>

          <div className="max-h-80 overflow-auto rounded-lg border border-slate-200 dark:border-slate-800">
            <table className="w-full min-w-[600px] border-collapse text-xs">
              <thead className="sticky top-0 border-b border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-800/60">
                <tr>
                  <th className="th">#</th>
                  {preview.headers.map((h) => (
                    <th key={h} className="th">
                      {h}
                    </th>
                  ))}
                  <th className="th">Issues</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
                {visibleRows.length === 0 ? (
                  <tr>
                    <td className="td text-center text-slate-400 dark:text-slate-500" colSpan={preview.headers.length + 2}>
                      No rows match this filter
                    </td>
                  </tr>
                ) : (
                  visibleRows.slice(0, 100).map((r) => (
                    <tr key={r.rowNumber} className={r.errors.length > 0 ? "bg-red-50 dark:bg-red-500/10" : ""}>
                      <td className="td">{r.rowNumber}</td>
                      {preview.headers.map((h) => (
                        <td key={h} className="td whitespace-nowrap">
                          {r.values[h] ?? ""}
                        </td>
                      ))}
                      <td className="td text-red-600 dark:text-red-400">{r.errors.join("; ")}</td>
                    </tr>
                  ))
                )}
              </tbody>
            </table>
            {visibleRows.length > 100 && (
              <p className="border-t border-slate-100 dark:border-slate-800 p-2 text-center text-xs text-slate-400 dark:text-slate-500">
                Showing first 100 of {visibleRows.length} matching rows
              </p>
            )}
          </div>

          {preview.errorCount > 0 && (
            <p className="mt-3 text-sm text-red-600 dark:text-red-400">
              Fix the highlighted rows in your CSV and re-choose the file. Nothing is imported until every row is
              valid.
            </p>
          )}
        </div>
      )}

      <ModalFooter>
        <Button variant="secondary" onClick={onClose} disabled={importing}>
          Cancel
        </Button>
        <Button
          variant="primary"
          onClick={confirmImport}
          disabled={!preview || preview.errorCount > 0 || importing || preview.rows.length === 0}
        >
          {importing ? "Importing..." : `Import ${preview?.validCount ?? ""} orders`}
        </Button>
      </ModalFooter>
    </Modal>
  );
}

// 2.0.76: Settings -> Notifications. Desktop/mobile-push alerts for the
// same 4 categories AlertBell/AttentionSection (Dashboard.tsx) already
// track - no new detection logic here, this only configures where those
// same facts also get sent. See commands/notifications.rs's module doc
// comment for the full backend design.
//
// 2.0.77: the email channel this shipped with in 2.0.76 was removed at
// marko's own request ("email zatial odstranme").
//
// 2.0.78: the mobile-push channel switched from Pushover to ntfy
// (https://ntfy.sh) - Pushover always needs both a personal user key AND a
// separate application token with no way to derive one from the other, so
// even the 2.0.77 "just paste your user key" version still required a
// one-time app registration from marko. ntfy's public server needs neither:
// the only thing to configure is a single self-chosen "topic" string, and
// there is nothing to register anywhere. See notifications.rs's module doc
// comment for the full reasoning, including the trade-off this implies
// (the topic name is the entire access control on ntfy's public server, so
// it must be a private, hard-to-guess phrase - treated as a secret here
// exactly like the old Pushover key was).
//
// Same "collapse once configured" visual convention SheetsConnectionCard
// uses (2.0.65) - `editing` reopens the full form, e.g. to change the ntfy
// topic. Not configured yet (no channel enabled), there's nothing to
// collapse, so the full form always shows.
//
// The ntfy topic is NEVER pre-filled from `NotificationStatus` - it only
// ever carries a `ntfyTopicSet: bool` presence flag, never the actual value
// (see that type's own doc comment, types.ts). The field always starts
// blank with a placeholder explaining that; leaving it blank on Save means
// "keep whatever is already stored", exactly what
// `NotificationConfigInput.ntfyTopic`'s `Option<String>` expects.
function NotificationsCard() {
  const toast = useToast();
  const [status, setStatus] = useState<NotificationStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [editing, setEditing] = useState(false);
  const [saving, setSaving] = useState(false);
  const [testBusy, setTestBusy] = useState<"desktop" | "ntfy" | null>(null);
  const [testResults, setTestResults] = useState<Partial<Record<"desktop" | "ntfy", NotificationTestResult>>>({});

  const [desktopEnabled, setDesktopEnabled] = useState(false);
  const [ntfyEnabled, setNtfyEnabled] = useState(false);
  const [ntfyTopic, setNtfyTopic] = useState("");

  // Applies a freshly loaded/saved status to the form - `desktopEnabled`/
  // `ntfyEnabled` mirror it exactly; the secret field always resets to
  // blank (see this component's own doc comment above for why).
  const applyStatus = (s: NotificationStatus) => {
    setStatus(s);
    setDesktopEnabled(s.desktopEnabled);
    setNtfyEnabled(s.ntfyEnabled);
    setNtfyTopic("");
  };

  useEffect(() => {
    setLoading(true);
    api
      .getNotificationStatus()
      .then(applyStatus)
      .catch((e) => toast.error(errMsg(e)))
      .finally(() => setLoading(false));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const configured = desktopEnabled || ntfyEnabled;

  const doSave = async () => {
    setSaving(true);
    try {
      const input: NotificationConfigInput = {
        desktopEnabled,
        ntfyEnabled,
        ntfyTopic: ntfyTopic.trim() ? ntfyTopic : null,
      };
      const result = await api.setNotificationConfig(input);
      applyStatus(result);
      setTestResults({});
      toast.success("Notification settings saved");
      setEditing(false);
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setSaving(false);
    }
  };

  // Every "Send test" button acts on whatever is currently SAVED, same as
  // SheetsConnectionCard's own "Test connection" - a click here never uses
  // unsaved edits still sitting in the form (see the hint text next to the
  // Save button below).
  const doTest = async (channel: "desktop" | "ntfy") => {
    setTestBusy(channel);
    try {
      const result = await (channel === "desktop" ? api.testDesktopNotification() : api.testNtfyNotification());
      setTestResults((r) => ({ ...r, [channel]: result }));
      if (!result.success) toast.error(result.message);
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setTestBusy(null);
    }
  };

  if (loading) {
    return (
      <Card className="p-5">
        <div className="flex items-center gap-2 text-sm text-slate-400 dark:text-slate-500">
          <Spinner className="h-4 w-4" /> Loading...
        </div>
      </Card>
    );
  }

  const TestResultLine = ({ channel }: { channel: "desktop" | "ntfy" }) => {
    const result = testResults[channel];
    if (!result) return null;
    return (
      <span className={`text-xs ${result.success ? "text-emerald-600 dark:text-emerald-400" : "text-red-600 dark:text-red-400"}`}>
        {result.message}
      </span>
    );
  };

  return (
    <Card className="p-5">
      <div className="mb-1 flex flex-wrap items-center gap-2">
        <h3 className="text-sm font-semibold text-slate-800 dark:text-slate-200">Notifications</h3>
        <Badge tone={configured ? "sold" : "available"}>{configured ? "Enabled" : "Off"}</Badge>
      </div>
      <p className="mb-4 text-xs text-slate-400 dark:text-slate-500">
        Get notified about the same things the Dashboard&apos;s bell already tracks - unpaid orders, pending sales,
        missing listing prices, and events coming up soon - even when you&apos;re not looking at the app. Each
        channel sends at most one notification per category per day, and only while TIQR Manager is running.
      </p>

      {configured && !editing ? (
        <div className="mb-1 flex flex-wrap items-center justify-between gap-3 rounded-lg border border-slate-200 bg-slate-50 px-3 py-2 dark:border-slate-800 dark:bg-slate-800/40">
          <p className="min-w-0 truncate text-xs text-slate-500 dark:text-slate-400">
            {[desktopEnabled && "Desktop", ntfyEnabled && "ntfy"].filter(Boolean).join(", ")}
          </p>
          <Button variant="ghost" onClick={() => setEditing(true)}>
            Change settings
          </Button>
        </div>
      ) : (
        <div className="space-y-5">
          <div>
            <label className="flex items-center gap-2 text-sm font-medium text-slate-700 dark:text-slate-200">
              <input
                type="checkbox"
                className={CHECKBOX_CLASS}
                checked={desktopEnabled}
                onChange={(e) => setDesktopEnabled(e.target.checked)}
              />
              Desktop notifications
            </label>
            <p className="mt-1 text-xs text-slate-400 dark:text-slate-500">
              Shows a system notification while TIQR Manager is open. No setup needed.
            </p>
            <div className="mt-2 flex flex-wrap items-center gap-2">
              <Button variant="secondary" disabled={testBusy === "desktop"} onClick={() => doTest("desktop")}>
                {testBusy === "desktop" ? <Spinner className="h-4 w-4" /> : null}
                Send test
              </Button>
              <TestResultLine channel="desktop" />
            </div>
          </div>

          <div className="border-t border-slate-200 pt-4 dark:border-slate-800">
            <label className="flex items-center gap-2 text-sm font-medium text-slate-700 dark:text-slate-200">
              <input
                type="checkbox"
                className={CHECKBOX_CLASS}
                checked={ntfyEnabled}
                onChange={(e) => setNtfyEnabled(e.target.checked)}
              />
              ntfy (mobile push)
            </label>
            <p className="mt-1 text-xs text-slate-400 dark:text-slate-500">
              Needs the free ntfy app on your phone (search &quot;ntfy&quot; on the App Store/Google Play) - no
              account or sign-up either there or here. Pick your own private, hard-to-guess phrase below (this is
              your &quot;topic&quot; - anyone who knows it can see your notifications, so don&apos;t use something
              obvious), then subscribe to that exact same phrase in the app.
            </p>
            <div className="mt-2 max-w-sm">
              <Field
                label="Topic"
                hint={status?.ntfyTopicSet ? "Already saved - leave blank to keep it." : undefined}
              >
                <Input
                  type="password"
                  autoComplete="off"
                  placeholder={status?.ntfyTopicSet ? "Leave blank to keep the current topic" : "e.g. tiqr-marko-8k2f"}
                  value={ntfyTopic}
                  onChange={(e) => setNtfyTopic(e.target.value)}
                />
              </Field>
            </div>
            <div className="mt-2 flex flex-wrap items-center gap-2">
              <Button variant="secondary" disabled={testBusy === "ntfy"} onClick={() => doTest("ntfy")}>
                {testBusy === "ntfy" ? <Spinner className="h-4 w-4" /> : null}
                Send test
              </Button>
              <TestResultLine channel="ntfy" />
            </div>
          </div>

          <div className="flex flex-wrap items-center gap-2 border-t border-slate-200 pt-4 dark:border-slate-800">
            <Button variant="primary" disabled={saving} onClick={doSave}>
              {saving ? <Spinner className="h-4 w-4" /> : null}
              Save
            </Button>
            {configured && (
              <Button
                variant="ghost"
                onClick={() => {
                  if (status) applyStatus(status);
                  setEditing(false);
                }}
              >
                Cancel
              </Button>
            )}
            <span className="text-xs text-slate-400 dark:text-slate-500">
              &quot;Send test&quot; uses whatever is currently saved - save first if you just changed something.
            </span>
          </div>
        </div>
      )}
    </Card>
  );
}

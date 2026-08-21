import { useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { open, save } from "@tauri-apps/plugin-dialog";
import { relaunch } from "@tauri-apps/plugin-process";
import { api, errMsg } from "../lib/api";
import {
  CURRENCY_OPTIONS,
  type AppInfo,
  type CreatedSheetResult,
  type CsvPreview,
  type GoogleSignInStatus,
  type Platform,
  type SheetsConnectionStatus,
  type SheetsConnectionTestResult,
  type SheetSyncResult,
} from "../lib/types";
import {
  Badge,
  Button,
  Card,
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
import {
  IconArrowLeft,
  IconDatabase,
  IconDownload,
  IconLink,
  IconPlus,
  IconSun,
  IconTag,
  IconTrash,
  IconUpload,
} from "../components/icons";
import { useToast } from "../lib/toast";
import { checkForUpdate, installUpdate, type Update, type UpdateProgress } from "../lib/updater";
import { useTheme, type ThemeMode } from "../lib/theme";

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
  { key: "integrations", title: "Integrations", description: "Connect Pulls, Orders and Tickets to a Google Sheet.", icon: IconLink },
  { key: "appearance", title: "Appearance", description: "Light, system or dark theme.", icon: IconSun },
  { key: "software", title: "Software", description: "Check for updates and see your current version.", icon: IconDownload },
];

export default function Settings() {
  const { section } = useParams();
  const toast = useToast();
  const [themeMode, setThemeMode] = useTheme();
  const [platforms, setPlatforms] = useState<Platform[]>([]);
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null);
  const [importOpen, setImportOpen] = useState(false);
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [confirmRestorePath, setConfirmRestorePath] = useState<string | null>(null);
  const [confirmDeletePlatform, setConfirmDeletePlatform] = useState<Platform | null>(null);
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
              section 4). `lg:` is effectively always active on this app's
              1080px-minimum window (see Sales.tsx's layout comment), so this
              reads as one row of 4 on every real window size. */}
          <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-4">
            {SECTIONS.map((s) => (
              <Link
                key={s.key}
                to={`/settings/${s.key}`}
                className="card block p-5 text-left transition-colors hover:border-brand-300 dark:hover:border-brand-700 hover:bg-slate-50 dark:hover:bg-slate-800/60"
              >
                <s.icon className="h-6 w-6 text-brand-600 dark:text-brand-400" />
                <h3 className="mt-3 text-sm font-semibold text-slate-800 dark:text-slate-200">{s.title}</h3>
                <p className="mt-1 text-xs text-slate-400 dark:text-slate-500">{s.description}</p>
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
              <h3 className="mb-1 text-sm font-semibold text-slate-800 dark:text-slate-200">Platforms</h3>
              {/* 1.9.3: split into two lists - marko didn't want "where you
                  bought it" and "where you sold it" sharing one pool any
                  more. Backed by the same `platforms` table either way (its
                  `kind` column has existed since the first migration, so no
                  schema change was needed) - a platform tagged "Both" simply
                  appears in both lists below, via PlatformList's own filter. */}
              <p className="mb-4 text-xs text-slate-400 dark:text-slate-500">
                Purchase platforms show up when recording an order; Selling platforms show up when recording a sale.
                Tag a platform "Both" if you use it for either. Not hardcoded — add as many as you like.
              </p>
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
                    Database file: <span className="font-mono">{appInfo.dbPath}</span>
                  </p>
                )}
              </Card>
            </div>
          )}

          {section === "integrations" && (
            <div className="grid grid-cols-1 gap-4 lg:max-w-3xl">
              <GoogleSignInCard onChange={setGoogleStatus} />
              <SheetsConnectionCard
                dataSource="pulls"
                label="Pulls"
                googleStatus={googleStatus}
                onSync={api.syncPulls}
                syncDescription={`"Sync now" reads the sheet and creates/updates matching pulls in the app - it never writes your data back to the sheet yet, except its own row IDs.`}
                onCreate={api.createPullsSheet}
                currencyHint="Applies to every row synced from this sheet - it has no currency column of its own."
              />
              {/* 2.0.8: one row = one order (marko's own choice) - creates the
                  order and all its tickets from the sheet's first batch of
                  columns; the sheet's second batch (Sales) is a later,
                  separate sync against these same rows - see
                  commands/orders_sheet_sync.rs's module doc comment. No
                  "Create a new sheet for me" here yet (onCreate omitted) -
                  marko already has a real sheet, unlike Pulls' original
                  from-scratch setup. */}
              <SheetsConnectionCard
                dataSource="orders"
                label="Orders & Tickets"
                googleStatus={googleStatus}
                onSync={api.syncOrders}
                syncDescription={`"Sync now" reads the sheet and creates a new order (with its tickets) for every row it hasn't seen before - it never edits an order once created, and never writes your data back to the sheet except its own row IDs. Add new rows any time and sync again.`}
                currencyHint="Used only when a row's own currency cell is blank - a row with its own currency uses that instead."
              />
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
                <div>
                  <p className="mb-2 text-sm text-slate-600 dark:text-slate-400">
                    Installing {available?.version}
                    {installProgress?.total ? ` - ${Math.round((installProgress.downloaded / installProgress.total) * 100)}%` : "..."}
                  </p>
                  <div className="h-1.5 w-full overflow-hidden rounded-full bg-slate-100 dark:bg-slate-800">
                    <div
                      className="h-full rounded-full bg-brand-600 transition-all"
                      style={{
                        width: installProgress?.total
                          ? `${Math.min(100, Math.round((installProgress.downloaded / installProgress.total) * 100))}%`
                          : "30%",
                      }}
                    />
                  </div>
                  <p className="mt-2 text-xs text-slate-400 dark:text-slate-500">The app will relaunch automatically once this finishes.</p>
                </div>
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
        </>
      )}

      <CsvImportModal open={importOpen} onClose={() => setImportOpen(false)} onImported={reload} />

      <ExportPickerModal open={!!exportConfig} config={exportConfig} onClose={() => setExportConfig(null)} />

      <ConfirmDialog
        open={!!confirmRestorePath}
        title="Restore this backup?"
        message={
          <>
            Your <b>current data will be replaced</b> with the contents of{" "}
            <span className="break-all font-mono text-xs">{confirmRestorePath}</span>. The app will relaunch
            automatically. This cannot be undone — back up your current data first if unsure.
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
          <Button variant="primary" disabled={busy === "in"} onClick={doSignIn}>
            {busy === "in" ? <Spinner className="h-4 w-4" /> : <IconLink className="h-4 w-4" />}
            {busy === "in" ? "Waiting for you to finish in your browser..." : "Sign in with Google"}
          </Button>
        </>
      )}
    </Card>
  );
}

function SheetsConnectionCard({
  dataSource,
  label,
  onSync,
  syncDescription,
  onCreate,
  currencyHint,
  googleStatus,
}: {
  dataSource: string;
  label: string;
  /** The "Sync now" call for this data source, e.g. api.syncPulls /
   * api.syncOrders. 2.0.3: only Pulls had this at first; 2.0.8 generalized it
   * to a prop (was a hardcoded api.syncPulls() call) so a second data source
   * could bring its own sync function without this component needing to
   * know which one it is. Omit for a data source with no sync logic yet
   * (connection-only, like Sales before its own sync ships) - the card still
   * lets you connect/test but hides "Sync now" entirely. */
  onSync?: () => Promise<SheetSyncResult>;
  /** Shown next to "Sync now" explaining exactly what it does for this data
   * source - each one behaves differently (Pulls creates+updates; Orders v1
   * only ever creates, see commands/orders_sheet_sync.rs's module doc
   * comment) so one shared sentence would misdescribe at least one of them.
   * Required whenever `onSync` is set. */
  syncDescription?: string;
  /** "Create a new sheet for me", e.g. api.createPullsSheet. 2.0.8:
   * generalized to a prop alongside onSync, but kept independent of it -
   * Orders has real sync logic but, unlike Pulls' original from-scratch
   * setup, no auto-create-a-blank-sheet flow yet (marko already has a real
   * sheet). Omit to hide that whole section. */
  onCreate?: (email: string, currency: string) => Promise<CreatedSheetResult>;
  /** Currency field's hint text - differs because Pulls' sheet has no
   * currency column of its own (one currency applies to every row) while
   * Orders' does (per-row, this is just the fallback for a blank cell). */
  currencyHint: string;
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
  const [currency, setCurrency] = useState<string>(CURRENCY_OPTIONS[0]);
  const [createEmail, setCreateEmail] = useState("");
  const [createdUrl, setCreatedUrl] = useState<string | null>(null);
  const [busy, setBusy] = useState<"save" | "test" | "sync" | "create" | "disconnect" | null>(null);
  const [testResult, setTestResult] = useState<SheetsConnectionTestResult | null>(null);
  const [syncResult, setSyncResult] = useState<SheetSyncResult | null>(null);
  const [confirmDisconnect, setConfirmDisconnect] = useState(false);

  const oauthEmail = googleStatus?.signedInEmail ?? null;

  const reload = () => {
    setLoading(true);
    api
      .getSheetsConnectionStatus(dataSource)
      .then((s) => {
        setStatus(s);
        setSpreadsheetInput(s.connection?.spreadsheetId ?? "");
        setSheetTab(s.connection?.sheetTab ?? "");
        setCurrency(s.connection?.currency ?? CURRENCY_OPTIONS[0]);
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
      await api.setSheetsConnection(dataSource, spreadsheetInput, sheetTab, currency);
      toast.success("Sheet connected");
      reload();
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
  // `createEmail` from the field below, exactly as in 2.0.4. Reuses the same
  // `currency` state as the paste-URL form either way, so there is only ever
  // one currency selector on this card.
  const doCreate = async () => {
    if (!onCreate) return;
    setBusy("create");
    setCreatedUrl(null);
    try {
      const result = await onCreate(oauthEmail ?? createEmail, currency);
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

  const doDisconnect = async () => {
    setBusy("disconnect");
    try {
      await api.clearSheetsConnection(dataSource);
      setConfirmDisconnect(false);
      setTestResult(null);
      setSyncResult(null);
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
          <p className="mb-4 text-xs text-slate-400 dark:text-slate-500">
            Paste the sheet&apos;s URL (or just its ID) and the exact tab name, then connect.{" "}
            {onSync
              ? syncDescription
              : `Reading and writing ${label.toLowerCase()} rows comes in a future update - this only sets up and tests the connection itself.`}
            {oauthEmail && " Uses your own signed-in Google account above, not the app's shared one."}
          </p>

          <div className="grid grid-cols-1 gap-3">
            <Field label="Spreadsheet URL or ID">
              <Input
                placeholder="https://docs.google.com/spreadsheets/d/..."
                value={spreadsheetInput}
                onChange={(e) => setSpreadsheetInput(e.target.value)}
              />
            </Field>
            <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
              <Field label="Sheet/tab name">
                <Input placeholder={`e.g. ${label}`} value={sheetTab} onChange={(e) => setSheetTab(e.target.value)} />
              </Field>
              <Field label="Currency" hint={currencyHint}>
                <Select value={currency} onChange={(e) => setCurrency(e.target.value)}>
                  {CURRENCY_OPTIONS.map((c) => (
                    <option key={c} value={c}>
                      {c}
                    </option>
                  ))}
                </Select>
              </Field>
            </div>
          </div>

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
                {onSync && (
                  <Button variant="secondary" disabled={busy === "sync"} onClick={doSync}>
                    {busy === "sync" ? <Spinner className="h-4 w-4" /> : null}
                    {busy === "sync" ? "Syncing..." : "Sync now"}
                  </Button>
                )}
                <Button variant="ghost" disabled={busy === "disconnect"} onClick={() => setConfirmDisconnect(true)}>
                  Disconnect
                </Button>
              </>
            )}
          </div>

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
            <p
              className={`mt-3 text-xs ${testResult.ok ? "text-emerald-600 dark:text-emerald-400" : "text-red-600 dark:text-red-400"}`}
            >
              {testResult.message}
            </p>
          )}

          {syncResult && (
            <div className="mt-3 rounded-lg border border-slate-200 dark:border-slate-800 p-3">
              <p className="text-xs text-slate-600 dark:text-slate-300">
                Created <b>{syncResult.created}</b>, updated <b>{syncResult.updated}</b>, unchanged{" "}
                <b>{syncResult.unchanged}</b>.
              </p>
              {syncResult.conflicts.length > 0 && (
                <div className="mt-2 max-h-40 overflow-y-auto">
                  <p className="text-xs font-medium text-amber-700 dark:text-amber-400">
                    {syncResult.conflicts.length} row{syncResult.conflicts.length === 1 ? "" : "s"} need
                    {syncResult.conflicts.length === 1 ? "s" : ""} your attention - both the sheet and the app changed
                    them since the last sync:
                  </p>
                  {syncResult.conflicts.map((c, i) => (
                    <p key={i} className="mt-0.5 text-xs text-amber-700 dark:text-amber-400">
                      Row {c.rowNumber}: {c.message}
                    </p>
                  ))}
                </div>
              )}
              {syncResult.errors.length > 0 && (
                <div className="mt-2 max-h-40 overflow-y-auto">
                  <p className="text-xs font-medium text-red-600 dark:text-red-400">
                    {syncResult.errors.length} row{syncResult.errors.length === 1 ? "" : "s"} skipped:
                  </p>
                  {syncResult.errors.map((e, i) => (
                    <p key={i} className="mt-0.5 text-xs text-red-600 dark:text-red-400">
                      Row {e.rowNumber}: {e.message}
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

          {oauthEmail ? (
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
          )}

          {status?.lastSyncedAt && (
            <p className="mt-1 text-xs text-slate-400 dark:text-slate-500">
              Last synced: {new Date(status.lastSyncedAt).toLocaleString()}
            </p>
          )}
        </>
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

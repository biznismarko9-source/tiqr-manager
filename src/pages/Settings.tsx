import { useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { open, save } from "@tauri-apps/plugin-dialog";
import { relaunch } from "@tauri-apps/plugin-process";
import { api, errMsg } from "../lib/api";
import type { AppInfo, CsvPreview, Platform } from "../lib/types";
import {
  Button,
  Card,
  ConfirmDialog,
  Input,
  Modal,
  ModalFooter,
  PageHeader,
  Spinner,
} from "../components/ui";
import { IconArrowLeft, IconDatabase, IconDownload, IconSun, IconTag, IconTrash, IconUpload } from "../components/icons";
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
const SECTIONS = [
  { key: "lookups", title: "Lookups", description: "Platforms and other lookup lists used across orders and sales.", icon: IconTag },
  { key: "data", title: "Data", description: "Import CSV, export CSV, backup and restore your database.", icon: IconDatabase },
  { key: "appearance", title: "Appearance", description: "Light, system or dark theme.", icon: IconSun },
  { key: "software", title: "Software", description: "Check for updates and see your current version.", icon: IconDownload },
];

export default function Settings() {
  const { section } = useParams();
  const toast = useToast();
  const [themeMode, setThemeMode] = useTheme();
  const [platforms, setPlatforms] = useState<Platform[]>([]);
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null);
  const [newPlatform, setNewPlatform] = useState("");
  const [importOpen, setImportOpen] = useState(false);
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [confirmRestorePath, setConfirmRestorePath] = useState<string | null>(null);
  const [confirmDeletePlatform, setConfirmDeletePlatform] = useState<Platform | null>(null);
  const [deletingLookup, setDeletingLookup] = useState(false);

  const [updateChecking, setUpdateChecking] = useState(false);
  const [updateChecked, setUpdateChecked] = useState(false);
  const [available, setAvailable] = useState<Update | null>(null);
  const [updateError, setUpdateError] = useState<string | null>(null);
  const [installing, setInstalling] = useState(false);
  const [installProgress, setInstallProgress] = useState<UpdateProgress | null>(null);

  const reload = () => {
    api.listPlatforms().then(setPlatforms).catch((e) => toast.error(errMsg(e)));
    api.getAppInfo().then(setAppInfo).catch(() => {});
  };

  useEffect(reload, []);

  const addPlatform = async () => {
    if (!newPlatform.trim()) return;
    try {
      await api.createPlatform(newPlatform.trim());
      setNewPlatform("");
      reload();
    } catch (e) {
      toast.error(errMsg(e));
    }
  };
  const doExport = async (
    label: string,
    fileSuggestion: string,
    fn: (path: string) => Promise<number>,
  ) => {
    const path = await save({
      defaultPath: fileSuggestion,
      filters: [{ name: "CSV", extensions: ["csv"] }],
    });
    if (!path) return;
    setBusyAction(label);
    try {
      const count = await fn(path);
      toast.success(`Exported ${count} rows to ${path}`);
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
            <Card className="p-5 lg:max-w-xl">
              <h3 className="mb-1 text-sm font-semibold text-slate-800 dark:text-slate-200">Platforms</h3>
              <p className="mb-3 text-xs text-slate-400 dark:text-slate-500">Used when recording orders and sales. Not hardcoded — add as many as you like.</p>
              <div className="mb-3 flex gap-2">
                <Input
                  placeholder="e.g. Ticketmaster"
                  value={newPlatform}
                  onChange={(e) => setNewPlatform(e.target.value)}
                  onKeyDown={(e) => e.key === "Enter" && addPlatform()}
                />
                <Button onClick={addPlatform}>Add</Button>
              </div>
              <ul className="max-h-56 divide-y divide-slate-100 dark:divide-slate-800 overflow-y-auto rounded-lg border border-slate-100 dark:border-slate-800">
                {platforms.length === 0 && <li className="p-3 text-sm text-slate-400 dark:text-slate-500">No platforms yet</li>}
                {platforms.map((p) => (
                  <li key={p.id} className="flex items-center justify-between px-3 py-2 text-sm">
                    <span>{p.name}</span>
                    <button
                      className="text-slate-300 dark:text-slate-600 hover:text-red-600 dark:hover:text-red-400"
                      title="Remove"
                      onClick={() => setConfirmDeletePlatform(p)}
                    >
                      <IconTrash className="h-4 w-4" />
                    </button>
                  </li>
                ))}
              </ul>
            </Card>
          )}

          {section === "data" && (
            <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
              <Card className="p-5">
                <h3 className="mb-1 text-sm font-semibold text-slate-800 dark:text-slate-200">Import orders from CSV</h3>
                <p className="mb-3 text-xs text-slate-400 dark:text-slate-500">
                  Bulk-add orders (and their tickets) from a spreadsheet. Columns: event, purchase_date, supplier,
                  platform, quantity, unit_price, fees, other_costs, currency, payment_status, ticket_type, section,
                  row, seats, notes. "seats" is a comma-separated list matching quantity (e.g. "11,12,13,14") - leave it
                  out to import without seat numbers. Everything imports in one all-or-nothing transaction.
                </p>
                <Button variant="primary" onClick={() => setImportOpen(true)}>
                  <IconUpload className="h-4 w-4" /> Choose CSV &amp; preview
                </Button>
              </Card>

              <Card className="p-5">
                <h3 className="mb-1 text-sm font-semibold text-slate-800 dark:text-slate-200">Export CSV</h3>
                <p className="mb-3 text-xs text-slate-400 dark:text-slate-500">Save any part of your data as a CSV file.</p>
                <div className="flex flex-wrap gap-2">
                  {[
                    { label: "Events", key: "events", fn: api.exportEventsCsv, file: "events.csv" },
                    { label: "Orders", key: "orders", fn: api.exportOrdersCsv, file: "orders.csv" },
                    { label: "Tickets", key: "tickets", fn: (p: string) => api.exportTicketsCsv(p), file: "tickets.csv" },
                    { label: "Sales", key: "sales", fn: api.exportSalesCsv, file: "sales.csv" },
                    { label: "Inventory", key: "inventory", fn: (p: string) => api.exportInventoryCsv(p), file: "inventory.csv" },
                  ].map((x) => (
                    <Button
                      key={x.key}
                      variant="secondary"
                      disabled={busyAction === x.key}
                      onClick={() => doExport(x.key, x.file, x.fn)}
                    >
                      {busyAction === x.key ? <Spinner className="h-4 w-4" /> : <IconDownload className="h-4 w-4" />}
                      {x.label}
                    </Button>
                  ))}
                </div>
              </Card>

              <Card className="p-5 lg:col-span-2">
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

  useEffect(() => {
    if (!isOpen) {
      setPath(null);
      setPreview(null);
    }
  }, [isOpen]);

  const pickFile = async () => {
    const p = await open({ multiple: false, filters: [{ name: "CSV", extensions: ["csv"] }] });
    if (!p || Array.isArray(p)) return;
    setPath(p);
    setLoading(true);
    setPreview(null);
    try {
      const res = await api.previewOrdersCsv(p);
      setPreview(res);
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setLoading(false);
    }
  };

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
          <div className="mb-3 flex items-center justify-between">
            <p className="text-sm text-slate-600 dark:text-slate-400">
              <span className="font-medium text-emerald-600 dark:text-emerald-400">{preview.validCount} valid</span>
              {preview.errorCount > 0 && (
                <>
                  {" "}
                  &middot; <span className="font-medium text-red-600 dark:text-red-400">{preview.errorCount} with errors</span>
                </>
              )}{" "}
              of {preview.rows.length} rows
            </p>
            <button className="text-xs font-medium text-brand-600 dark:text-brand-400 hover:underline" onClick={pickFile}>
              Choose a different file
            </button>
          </div>

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
                {preview.rows.slice(0, 100).map((r) => (
                  <tr key={r.rowNumber} className={r.errors.length > 0 ? "bg-red-50 dark:bg-red-500/10" : ""}>
                    <td className="td">{r.rowNumber}</td>
                    {preview.headers.map((h) => (
                      <td key={h} className="td whitespace-nowrap">
                        {r.values[h] ?? ""}
                      </td>
                    ))}
                    <td className="td text-red-600 dark:text-red-400">{r.errors.join("; ")}</td>
                  </tr>
                ))}
              </tbody>
            </table>
            {preview.rows.length > 100 && (
              <p className="border-t border-slate-100 dark:border-slate-800 p-2 text-center text-xs text-slate-400 dark:text-slate-500">
                Showing first 100 of {preview.rows.length} rows
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

import { useEffect, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { relaunch } from "@tauri-apps/plugin-process";
import { api, errMsg } from "../lib/api";
import type { AppInfo, CsvPreview, Platform, Supplier } from "../lib/types";
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
  Spinner,
} from "../components/ui";
import { IconDatabase, IconDownload, IconTrash, IconUpload } from "../components/icons";
import { useToast } from "../lib/toast";

export default function Settings() {
  const toast = useToast();
  const [platforms, setPlatforms] = useState<Platform[]>([]);
  const [suppliers, setSuppliers] = useState<Supplier[]>([]);
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null);
  const [newPlatform, setNewPlatform] = useState("");
  const [newSupplier, setNewSupplier] = useState("");
  const [importOpen, setImportOpen] = useState(false);
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [confirmRestorePath, setConfirmRestorePath] = useState<string | null>(null);
  const [confirmClearDemo, setConfirmClearDemo] = useState(false);
  const [confirmResetDemo, setConfirmResetDemo] = useState(false);

  const reload = () => {
    api.listPlatforms().then(setPlatforms).catch((e) => toast.error(errMsg(e)));
    api.listSuppliers().then(setSuppliers).catch((e) => toast.error(errMsg(e)));
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
  const addSupplier = async () => {
    if (!newSupplier.trim()) return;
    try {
      await api.createSupplier(newSupplier.trim());
      setNewSupplier("");
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
    setConfirmRestorePath(path);
  };

  const doRestore = async () => {
    if (!confirmRestorePath) return;
    setBusyAction("restore");
    try {
      await api.restoreDatabase(confirmRestorePath);
      toast.success("Backup restored. Relaunching...");
      setTimeout(() => relaunch(), 700);
    } catch (e) {
      toast.error(errMsg(e));
      setBusyAction(null);
      setConfirmRestorePath(null);
    }
  };

  return (
    <div>
      <PageHeader title="Settings" subtitle="Lookups, data import/export, backups and demo data." />

      <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
        <Card className="p-5">
          <h2 className="mb-1 text-sm font-semibold text-slate-800">Platforms</h2>
          <p className="mb-3 text-xs text-slate-400">Used when recording orders and sales. Not hardcoded — add as many as you like.</p>
          <div className="mb-3 flex gap-2">
            <Input
              placeholder="e.g. Ticketmaster"
              value={newPlatform}
              onChange={(e) => setNewPlatform(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && addPlatform()}
            />
            <Button onClick={addPlatform}>Add</Button>
          </div>
          <ul className="max-h-56 divide-y divide-slate-100 overflow-y-auto rounded-lg border border-slate-100">
            {platforms.length === 0 && <li className="p-3 text-sm text-slate-400">No platforms yet</li>}
            {platforms.map((p) => (
              <li key={p.id} className="flex items-center justify-between px-3 py-2 text-sm">
                <span>{p.name}</span>
                <button
                  className="text-slate-300 hover:text-red-600"
                  title="Remove"
                  onClick={async () => {
                    try {
                      await api.deletePlatform(p.id);
                      reload();
                    } catch (e) {
                      toast.error(errMsg(e));
                    }
                  }}
                >
                  <IconTrash className="h-4 w-4" />
                </button>
              </li>
            ))}
          </ul>
        </Card>

        <Card className="p-5">
          <h2 className="mb-1 text-sm font-semibold text-slate-800">Suppliers</h2>
          <p className="mb-3 text-xs text-slate-400">Who you buy tickets from.</p>
          <div className="mb-3 flex gap-2">
            <Input
              placeholder="e.g. John from Discord"
              value={newSupplier}
              onChange={(e) => setNewSupplier(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && addSupplier()}
            />
            <Button onClick={addSupplier}>Add</Button>
          </div>
          <ul className="max-h-56 divide-y divide-slate-100 overflow-y-auto rounded-lg border border-slate-100">
            {suppliers.length === 0 && <li className="p-3 text-sm text-slate-400">No suppliers yet</li>}
            {suppliers.map((s) => (
              <li key={s.id} className="flex items-center justify-between px-3 py-2 text-sm">
                <span>
                  {s.name} {s.isDemo && <Badge tone="demo">demo</Badge>}
                </span>
                <button
                  className="text-slate-300 hover:text-red-600"
                  title="Remove"
                  onClick={async () => {
                    try {
                      await api.deleteSupplier(s.id);
                      reload();
                    } catch (e) {
                      toast.error(errMsg(e));
                    }
                  }}
                >
                  <IconTrash className="h-4 w-4" />
                </button>
              </li>
            ))}
          </ul>
        </Card>

        <Card className="p-5">
          <h2 className="mb-1 text-sm font-semibold text-slate-800">Import orders from CSV</h2>
          <p className="mb-3 text-xs text-slate-400">
            Bulk-add orders (and their tickets) from a spreadsheet. Columns: event, purchase_date, supplier,
            platform, quantity, unit_price, fees, other_costs, currency, payment_status, ticket_type, section,
            notes. Everything imports in one all-or-nothing transaction.
          </p>
          <Button variant="primary" onClick={() => setImportOpen(true)}>
            <IconUpload className="h-4 w-4" /> Choose CSV &amp; preview
          </Button>
        </Card>

        <Card className="p-5">
          <h2 className="mb-1 text-sm font-semibold text-slate-800">Export CSV</h2>
          <p className="mb-3 text-xs text-slate-400">Save any part of your data as a CSV file.</p>
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

        <Card className="p-5">
          <h2 className="mb-1 text-sm font-semibold text-slate-800">Backup &amp; restore</h2>
          <p className="mb-3 text-xs text-slate-400">
            Your database lives only on this device. Back it up regularly, especially before big imports.
          </p>
          <div className="flex flex-wrap gap-2">
            <Button variant="secondary" disabled={busyAction === "backup"} onClick={doBackup}>
              {busyAction === "backup" ? <Spinner className="h-4 w-4" /> : <IconDatabase className="h-4 w-4" />}
              Backup database...
            </Button>
            <Button variant="secondary" onClick={pickRestoreFile}>
              <IconUpload className="h-4 w-4" /> Restore from backup...
            </Button>
          </div>
          {appInfo && (
            <p className="mt-4 break-all text-xs text-slate-400">
              Database file: <span className="font-mono">{appInfo.dbPath}</span>
            </p>
          )}
        </Card>

        <Card className="p-5">
          <h2 className="mb-1 text-sm font-semibold text-slate-800">Demo data</h2>
          <p className="mb-3 text-xs text-slate-400">
            The sample events/orders/tickets/sales that ship with TIQR Manager so you're not staring at an
            empty app. Clear them once you start entering your own data.
          </p>
          <div className="flex flex-wrap gap-2">
            <Button variant="secondary" onClick={() => setConfirmClearDemo(true)}>
              Clear demo data
            </Button>
            <Button variant="secondary" onClick={() => setConfirmResetDemo(true)}>
              Reset demo data
            </Button>
          </div>
          {appInfo && <p className="mt-4 text-xs text-slate-400">TIQR Manager v{appInfo.version}</p>}
        </Card>
      </div>

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
        open={confirmClearDemo}
        title="Clear demo data?"
        message="Removes every demo event, order, ticket and sale. Your own real data is never touched. This cannot be undone."
        confirmLabel="Clear demo data"
        danger
        onCancel={() => setConfirmClearDemo(false)}
        onConfirm={async () => {
          try {
            await api.clearDemoData();
            toast.success("Demo data cleared");
          } catch (e) {
            toast.error(errMsg(e));
          } finally {
            setConfirmClearDemo(false);
          }
        }}
      />

      <ConfirmDialog
        open={confirmResetDemo}
        title="Reset demo data?"
        message="Removes existing demo records and re-creates a fresh set of sample events, orders, tickets and sales. Your own real data is never touched."
        confirmLabel="Reset demo data"
        onCancel={() => setConfirmResetDemo(false)}
        onConfirm={async () => {
          try {
            await api.resetDemoData();
            toast.success("Demo data reset");
          } catch (e) {
            toast.error(errMsg(e));
          } finally {
            setConfirmResetDemo(false);
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
        <div className="flex flex-col items-center gap-3 rounded-lg border border-dashed border-slate-300 py-10">
          <IconUpload className="h-8 w-8 text-slate-300" />
          <p className="text-sm text-slate-500">Choose a CSV file to preview before importing.</p>
          <Button variant="primary" onClick={pickFile}>
            Choose CSV file...
          </Button>
        </div>
      )}

      {loading && (
        <div className="flex items-center justify-center gap-2 py-10 text-sm text-slate-400">
          <Spinner className="h-4 w-4" /> Reading file...
        </div>
      )}

      {preview && !loading && (
        <div>
          <div className="mb-3 flex items-center justify-between">
            <p className="text-sm text-slate-600">
              <span className="font-medium text-emerald-600">{preview.validCount} valid</span>
              {preview.errorCount > 0 && (
                <>
                  {" "}
                  &middot; <span className="font-medium text-red-600">{preview.errorCount} with errors</span>
                </>
              )}{" "}
              of {preview.rows.length} rows
            </p>
            <button className="text-xs font-medium text-brand-600 hover:underline" onClick={pickFile}>
              Choose a different file
            </button>
          </div>

          <div className="max-h-80 overflow-auto rounded-lg border border-slate-200">
            <table className="w-full min-w-[600px] border-collapse text-xs">
              <thead className="sticky top-0 border-b border-slate-200 bg-slate-50">
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
              <tbody className="divide-y divide-slate-100">
                {preview.rows.slice(0, 100).map((r) => (
                  <tr key={r.rowNumber} className={r.errors.length > 0 ? "bg-red-50" : ""}>
                    <td className="td">{r.rowNumber}</td>
                    {preview.headers.map((h) => (
                      <td key={h} className="td whitespace-nowrap">
                        {r.values[h] ?? ""}
                      </td>
                    ))}
                    <td className="td text-red-600">{r.errors.join("; ")}</td>
                  </tr>
                ))}
              </tbody>
            </table>
            {preview.rows.length > 100 && (
              <p className="border-t border-slate-100 p-2 text-center text-xs text-slate-400">
                Showing first 100 of {preview.rows.length} rows
              </p>
            )}
          </div>

          {preview.errorCount > 0 && (
            <p className="mt-3 text-sm text-red-600">
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

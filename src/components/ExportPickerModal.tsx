import { useEffect, useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { api, errMsg } from "../lib/api";
import type { EventWithStats, OrderRecord, SaleGroup, Ticket } from "../lib/types";
import { todayIso } from "../lib/format";
import { Button, CHECKBOX_CLASS, Input, LoadingBlock, Modal, ModalFooter } from "./ui";
import { IconSearch } from "./icons";
import { useToast } from "../lib/toast";

/**
 * 1.9.1: describes one entity's "export CSV" flow for the shared picker
 * below - Settings.tsx builds one of these per button (Events/Orders/
 * Tickets/Sales/Inventory) instead of the old "click = instant whole-file
 * download" behaviour. marko's request: "urcite je to dobra feature ale
 * urobit ju nejak ina... dal by som na vyber po kliknuti napr na export a
 * tam mi ukaze presne ktory sale chcem vybrat... tak by som to urobil so
 * vsetkymi exportami co su tam" - i.e. every export in Settings -> Data
 * should show a picker (search + checkboxes, one/several/all), not just
 * Sales. This is that one shared engine, reusing each entity's own already
 * search-capable list command (`api.listEvents`/`listOrders`/`listTickets`/
 * `listSaleGroups`) and its own already-existing "export selected ids"
 * command - no new list-fetching commands were needed.
 */
export interface ExportPickerConfig<T> {
  /** Shown as the modal title, e.g. "Export events". */
  title: string;
  searchPlaceholder: string;
  /** Re-queries the entity's own existing search-capable list API. */
  fetchItems: (search: string) => Promise<T[]>;
  getId: (item: T) => number;
  /** Primary (bold) + optional secondary (muted) line shown per row. */
  renderItem: (item: T) => { primary: string; secondary?: string };
  /** Calls the entity's own *CsvSelected api function; resolves to the exported row count. */
  exportSelected: (path: string, ids: number[]) => Promise<number>;
  /** Used for the save dialog's default filename: "events" -> tiqr-events-selected-2026-08-19.csv */
  fileStem: string;
}

/** Config builders for the 5 Settings -> Data exports - kept here (not
 * inlined in Settings.tsx) since they're pure wiring with no UI of their
 * own, and Inventory is deliberately just Tickets with a status filter
 * baked into `fetchItems` (its export command is the same
 * `exportTicketsCsvSelected` - the picker only ever offers available/listed
 * tickets to choose from, so the backend doesn't need a second, separate
 * "selected inventory" command). */
export const eventsExportConfig: ExportPickerConfig<EventWithStats> = {
  title: "Export events",
  searchPlaceholder: "Search events...",
  fetchItems: (search) => api.listEvents(search || undefined),
  getId: (e) => e.id,
  renderItem: (e) => ({
    primary: e.name,
    secondary: [e.venue, e.eventDate].filter(Boolean).join(" · ") || undefined,
  }),
  exportSelected: (path, ids) => api.exportEventsCsvSelected(path, ids),
  fileStem: "events",
};

export const ordersExportConfig: ExportPickerConfig<OrderRecord> = {
  title: "Export orders",
  searchPlaceholder: "Search orders...",
  fetchItems: (search) => api.listOrders({ search: search || undefined }),
  getId: (o) => o.id,
  renderItem: (o) => ({ primary: o.code, secondary: `${o.eventName} · ${o.purchaseDate}` }),
  exportSelected: (path, ids) => api.exportOrdersCsvSelected(path, ids),
  fileStem: "orders",
};

export const ticketsExportConfig: ExportPickerConfig<Ticket> = {
  title: "Export tickets",
  searchPlaceholder: "Search tickets...",
  fetchItems: (search) => api.listTickets({ search: search || undefined }),
  getId: (t) => t.id,
  renderItem: (t) => ({ primary: t.code, secondary: `${t.eventName} · ${t.orderCode} · ${t.status}` }),
  exportSelected: (path, ids) => api.exportTicketsCsvSelected(path, ids),
  fileStem: "tickets",
};

export const inventoryExportConfig: ExportPickerConfig<Ticket> = {
  title: "Export inventory",
  searchPlaceholder: "Search inventory...",
  // "Inventory" = current stock only (available + listed) - same restriction
  // export_inventory_csv itself already applies, mirrored here so the picker
  // never even offers a sold/cancelled ticket to select.
  fetchItems: (search) => api.listTickets({ search: search || undefined, status: "available,listed" }),
  getId: (t) => t.id,
  renderItem: (t) => ({ primary: t.code, secondary: `${t.eventName} · ${t.orderCode}` }),
  exportSelected: (path, ids) => api.exportTicketsCsvSelected(path, ids),
  fileStem: "inventory",
};

export const salesExportConfig: ExportPickerConfig<SaleGroup> = {
  title: "Export sales",
  searchPlaceholder: "Search sales...",
  fetchItems: (search) => api.listSaleGroups({ search: search || undefined }),
  getId: (g) => g.id,
  renderItem: (g) => ({ primary: g.code, secondary: `${g.eventName ?? "Mixed events"} · ${g.saleDate}` }),
  exportSelected: (path, ids) => api.exportSalesCsvSelected(path, ids),
  fileStem: "sales",
};

export function ExportPickerModal<T>({
  open,
  config,
  onClose,
}: {
  open: boolean;
  /** Null while closed/between clicks - kept simple by always rendering one
   * instance in Settings.tsx and swapping which config it holds. */
  config: ExportPickerConfig<T> | null;
  onClose: () => void;
}) {
  const toast = useToast();
  const [search, setSearch] = useState("");
  const [items, setItems] = useState<T[] | null>(null);
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [exporting, setExporting] = useState(false);

  useEffect(() => {
    if (!open) return;
    setSearch("");
    setItems(null);
    setSelected(new Set());
  }, [open, config]);

  useEffect(() => {
    if (!open || !config) return;
    const t = setTimeout(() => {
      config
        .fetchItems(search)
        .then(setItems)
        .catch((e) => toast.error(errMsg(e)));
    }, 200);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, config, search]);

  if (!config) return null;

  const allSelected = items !== null && items.length > 0 && items.every((it) => selected.has(config.getId(it)));
  const toggleAll = () => {
    if (!items) return;
    setSelected(allSelected ? new Set() : new Set(items.map(config.getId)));
  };
  const toggleOne = (id: number) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const doExport = async () => {
    const path = await save({
      defaultPath: `tiqr-${config.fileStem}-selected-${todayIso()}.csv`,
      filters: [{ name: "CSV", extensions: ["csv"] }],
    });
    if (!path || Array.isArray(path)) return;
    setExporting(true);
    try {
      const ids = Array.from(selected);
      const count = await config.exportSelected(path, ids);
      toast.success(`Exported ${count} row${count === 1 ? "" : "s"} to ${path}`);
      onClose();
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setExporting(false);
    }
  };

  return (
    <Modal open={open} onClose={onClose} title={config.title} width="max-w-lg">
      <div className="relative mb-3">
        <IconSearch className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-400 dark:text-slate-500" />
        <Input
          autoFocus
          placeholder={config.searchPlaceholder}
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="pl-9"
        />
      </div>

      <div className="mb-2 flex items-center justify-between">
        <label className="flex items-center gap-2 text-xs font-medium text-slate-500 dark:text-slate-400">
          <input
            type="checkbox"
            className={CHECKBOX_CLASS}
            checked={allSelected}
            onChange={toggleAll}
            disabled={!items || items.length === 0}
          />
          Select all {items && items.length > 0 ? `(${items.length} shown)` : ""}
        </label>
        <span className="text-xs text-slate-400 dark:text-slate-500">Selected: {selected.size}</span>
      </div>

      <div className="max-h-72 divide-y divide-slate-100 dark:divide-slate-800 overflow-y-auto rounded-lg border border-slate-200 dark:border-slate-800">
        {items === null ? (
          <div className="p-6">
            <LoadingBlock />
          </div>
        ) : items.length === 0 ? (
          <p className="p-4 text-center text-sm text-slate-400 dark:text-slate-500">
            {search ? "No matches" : "Nothing to export yet"}
          </p>
        ) : (
          items.map((it) => {
            const id = config.getId(it);
            const { primary, secondary } = config.renderItem(it);
            return (
              <label
                key={id}
                className="flex cursor-pointer items-center gap-3 px-3 py-2 hover:bg-slate-50 dark:hover:bg-slate-800/60"
              >
                <input
                  type="checkbox"
                  className={CHECKBOX_CLASS}
                  checked={selected.has(id)}
                  onChange={() => toggleOne(id)}
                />
                <span className="min-w-0 flex-1">
                  <span className="block truncate text-sm text-slate-800 dark:text-slate-200">{primary}</span>
                  {secondary && (
                    <span className="block truncate text-xs text-slate-400 dark:text-slate-500">{secondary}</span>
                  )}
                </span>
              </label>
            );
          })
        )}
      </div>

      <ModalFooter>
        <Button variant="secondary" onClick={onClose} disabled={exporting}>
          Cancel
        </Button>
        <Button variant="primary" onClick={doExport} disabled={exporting || selected.size === 0}>
          {exporting ? "Exporting..." : selected.size > 0 ? `Export ${selected.size}` : "Export"}
        </Button>
      </ModalFooter>
    </Modal>
  );
}

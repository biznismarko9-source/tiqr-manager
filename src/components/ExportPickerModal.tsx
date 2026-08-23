import { useEffect, useMemo, useRef, useState, type InputHTMLAttributes } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { api, errMsg } from "../lib/api";
import type { EventWithStats, OrderRecord, SaleGroup, Ticket } from "../lib/types";
import { formatSeatLocation, todayIso } from "../lib/format";
import { Button, CHECKBOX_CLASS, Input, LoadingBlock, Modal, ModalFooter } from "./ui";
import { IconChevronDown, IconSearch } from "./icons";
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
  /** 1.9.2 (sections 9/10): when set, items are grouped into an expandable
   * Order -> Tickets tree instead of a flat list - used by the Tickets/
   * Inventory configs only (Events/Orders/Sales stay flat/unchanged, zero
   * regression risk for those three). Returns which group `item` belongs to;
   * items sharing the same `key` are shown together under one expandable
   * header (`primary`/`secondary` describe the GROUP - e.g. the order code
   * and event name - not the item itself).
   *
   * This is client-side grouping over the SAME already-search-capable flat
   * list `fetchItems` returns - no second grouping engine and no new backend
   * query: search still runs as one `list_tickets` call (same as before),
   * and its results are simply bucketed by order for display. A search term
   * that matches a ticket's own field (code/section/seat/row) returns just
   * that ticket, so its group shows only it; a term that matches the order
   * code or event name returns every ticket on that order (since every one
   * of its rows carries the same order code/event name), so its group shows
   * the whole order - "the matching ticket highlighted, or the whole order,
   * depending on what matched" falls out of the existing search semantics
   * automatically, nothing extra to build. */
  groupBy?: (item: T) => { key: number; primary: string; secondary?: string };
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
  fetchItems: (search) => api.listEvents({ search: search || undefined }),
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
  searchPlaceholder: "Search by ticket, order, or event...",
  fetchItems: (search) => api.listTickets({ search: search || undefined }),
  getId: (t) => t.id,
  // 1.9.2 (section 9): Event/Order used to be repeated on every row here -
  // now shown once per group header instead (see groupBy below), so a
  // ticket's own row shows what's actually specific to IT: seat location and
  // status. formatSeatLocation is the same helper Sale/Order Detail already
  // use for this.
  renderItem: (t) => ({ primary: t.code, secondary: `${formatSeatLocation(t.section, t.rowLabel, t.seat)} · ${t.status}` }),
  exportSelected: (path, ids) => api.exportTicketsCsvSelected(path, ids),
  fileStem: "tickets",
  groupBy: (t) => ({ key: t.orderId, primary: t.orderCode, secondary: t.eventName }),
};

export const inventoryExportConfig: ExportPickerConfig<Ticket> = {
  title: "Export inventory",
  searchPlaceholder: "Search by ticket, order, or event...",
  // "Inventory" = current stock only (available + listed) - same restriction
  // export_inventory_csv itself already applies, mirrored here so the picker
  // never even offers a sold/cancelled ticket to select. Unchanged by the
  // 1.9.2 grouping work below - still exactly this one status filter, no
  // other rule.
  fetchItems: (search) => api.listTickets({ search: search || undefined, status: "available,listed" }),
  getId: (t) => t.id,
  renderItem: (t) => ({ primary: t.code, secondary: `${formatSeatLocation(t.section, t.rowLabel, t.seat)} · ${t.status}` }),
  exportSelected: (path, ids) => api.exportTicketsCsvSelected(path, ids),
  fileStem: "inventory",
  groupBy: (t) => ({ key: t.orderId, primary: t.orderCode, secondary: t.eventName }),
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

/** A checkbox that can also show "some but not all" (indeterminate) - not a
 * plain JSX prop on `<input type="checkbox">`, so it has to be set
 * imperatively on the DOM node via a ref, same as every other framework's
 * checkbox component has to do. Used by the group header checkboxes below
 * (section 9: "whole-order selection" / "select all tickets in an order"). */
function TriStateCheckbox({
  indeterminate,
  className,
  ...rest
}: { indeterminate: boolean } & Omit<InputHTMLAttributes<HTMLInputElement>, "type">) {
  const ref = useRef<HTMLInputElement>(null);
  useEffect(() => {
    if (ref.current) ref.current.indeterminate = indeterminate;
  }, [indeterminate]);
  return <input ref={ref} type="checkbox" className={className ?? CHECKBOX_CLASS} {...rest} />;
}

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
  // 1.9.2 (section 9): which group keys are expanded - grouped mode
  // (Tickets/Inventory) only, unused otherwise. Keyed by the group's own
  // `key` (orderId for Tickets/Inventory), not by any item id.
  const [expanded, setExpanded] = useState<Set<number>>(new Set());

  useEffect(() => {
    if (!open) return;
    setSearch("");
    setItems(null);
    setSelected(new Set());
    setExpanded(new Set());
  }, [open, config]);

  useEffect(() => {
    if (!open || !config) return;
    const t = setTimeout(() => {
      config
        .fetchItems(search)
        .then((res) => {
          setItems(res);
          if (config.groupBy) {
            const groupBy = config.groupBy;
            if (search.trim() !== "") {
              // Every returned item already matched the search server-side
              // (list_tickets' own WHERE clause - unchanged) - auto-expand
              // every group that came back so whatever matched is visible
              // without an extra click (section 9: "search ... surfaces the
              // right Order group, with just the matching ticket
              // highlighted, or the whole order, depending on what
              // matched" - see groupBy's doc comment above for why that
              // distinction falls out of the existing search semantics for
              // free, with no extra logic needed here).
              setExpanded(new Set(res.map((it) => groupBy(it).key)));
            } else {
              // Browsing with no search: collapsed by default - the whole
              // point of grouping is not showing every ticket on every order
              // at once. "Select all" below still works on the full result
              // set regardless of what's expanded.
              setExpanded(new Set());
            }
          }
        })
        .catch((e) => toast.error(errMsg(e)));
    }, 200);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, config, search]);

  // 1.9.2 (section 9): buckets the flat `items` array by config.groupBy -
  // purely a client-side reshape of data already fetched in ONE query above
  // (section 13: never N queries for N tickets). `null` in flat mode
  // (Events/Orders/Sales configs have no groupBy), so the render below can
  // branch on it directly.
  const groups = useMemo(() => {
    if (!config?.groupBy || !items) return null;
    const groupBy = config.groupBy;
    const order: number[] = [];
    const byKey = new Map<number, { key: number; primary: string; secondary?: string; items: T[] }>();
    for (const it of items) {
      const g = groupBy(it);
      let bucket = byKey.get(g.key);
      if (!bucket) {
        bucket = { key: g.key, primary: g.primary, secondary: g.secondary, items: [] };
        byKey.set(g.key, bucket);
        order.push(g.key);
      }
      bucket.items.push(it);
    }
    return order.map((key) => byKey.get(key)!);
  }, [config, items]);

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
  // 1.9.2 (section 9): "whole-order selection" / "select all tickets in an
  // order" - one group header checkbox toggles every ticket currently in
  // that group. `currentlyAllSelected` is passed in rather than recomputed
  // here so the caller (which already has it, to render the checkbox's own
  // checked state) and this stay in agreement about which direction to
  // toggle.
  const toggleGroup = (ids: number[], currentlyAllSelected: boolean) => {
    setSelected((prev) => {
      const next = new Set(prev);
      ids.forEach((id) => (currentlyAllSelected ? next.delete(id) : next.add(id)));
      return next;
    });
  };
  const toggleExpanded = (key: number) => {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  };
  // 1.9.2 (section 9): "N tickets / M orders" selected-count line - grouped
  // mode only. A group counts as "touched" as soon as any one of its
  // tickets is selected, same rule the group header checkboxes below use for
  // their own indeterminate state.
  const selectedGroupCount = groups
    ? groups.filter((g) => g.items.some((it) => selected.has(config.getId(it)))).length
    : 0;

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
          {items && items.length > 0
            ? groups
              ? `Select all (${items.length} ticket${items.length === 1 ? "" : "s"} in ${groups.length} order${groups.length === 1 ? "" : "s"})`
              : `Select all (${items.length} shown)`
            : "Select all"}
        </label>
        {/* 1.9.2 (section 9): "N tickets / M orders" in grouped mode (Tickets/
            Inventory) instead of a plain count - selectedGroupCount is
            derived from the same `selected` Set flat mode already uses, so
            this can never disagree with the per-group checkboxes below. */}
        <span className="text-xs text-slate-400 dark:text-slate-500">
          {groups
            ? `Selected: ${selected.size} ticket${selected.size === 1 ? "" : "s"} / ${selectedGroupCount} order${selectedGroupCount === 1 ? "" : "s"}`
            : `Selected: ${selected.size}`}
        </span>
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
        ) : groups ? (
          // 1.9.2 (section 9): expandable Order -> Tickets tree - Tickets/
          // Inventory only (config.groupBy set). Events/Orders/Sales fall
          // through to the plain flat list below, completely unchanged.
          groups.map((g) => {
            const groupIds = g.items.map(config.getId);
            const selectedInGroup = groupIds.filter((id) => selected.has(id));
            const allInGroupSelected = groupIds.length > 0 && selectedInGroup.length === groupIds.length;
            const someInGroupSelected = selectedInGroup.length > 0 && !allInGroupSelected;
            const isExpanded = expanded.has(g.key);
            return (
              <div key={g.key}>
                <div className="flex items-center gap-2 bg-slate-50/60 px-3 py-2 dark:bg-slate-800/40">
                  <TriStateCheckbox
                    checked={allInGroupSelected}
                    indeterminate={someInGroupSelected}
                    onChange={() => toggleGroup(groupIds, allInGroupSelected)}
                    aria-label={`Select every ticket in order ${g.primary}`}
                  />
                  <button
                    type="button"
                    className="flex min-w-0 flex-1 items-center gap-2 text-left"
                    onClick={() => toggleExpanded(g.key)}
                    aria-expanded={isExpanded}
                  >
                    <IconChevronDown
                      className={`h-3.5 w-3.5 shrink-0 text-slate-400 transition-transform dark:text-slate-500 ${isExpanded ? "" : "-rotate-90"}`}
                    />
                    <span className="min-w-0 flex-1">
                      <span className="block truncate text-sm font-medium text-slate-800 dark:text-slate-200">{g.primary}</span>
                      {g.secondary && (
                        <span className="block truncate text-xs text-slate-400 dark:text-slate-500">{g.secondary}</span>
                      )}
                    </span>
                    <span className="shrink-0 text-xs tabular-nums text-slate-400 dark:text-slate-500">
                      {selectedInGroup.length > 0 ? `${selectedInGroup.length}/${groupIds.length}` : groupIds.length} ticket
                      {groupIds.length === 1 ? "" : "s"}
                    </span>
                  </button>
                </div>
                {isExpanded && (
                  <div className="divide-y divide-slate-50 dark:divide-slate-800/60">
                    {g.items.map((it) => {
                      const id = config.getId(it);
                      const { primary, secondary } = config.renderItem(it);
                      return (
                        <label
                          key={id}
                          className="flex cursor-pointer items-center gap-3 py-2 pl-9 pr-3 hover:bg-slate-50 dark:hover:bg-slate-800/60"
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
                    })}
                  </div>
                )}
              </div>
            );
          })
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

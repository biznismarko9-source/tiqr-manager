import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { Link, useLocation, useNavigate } from "react-router-dom";
import { api, errMsg } from "../lib/api";
import type { EventCategory, EventWithStats, OrderInput, OrderPaymentStatus, Platform } from "../lib/types";
import { centsToDecimalString, decimalStringToCents, formatDateNumeric, formatMoney, formatSeatsSummary, summarizeBulkDeleteSkips, todayIso } from "../lib/format";
import {
  Badge,
  Button,
  BulkDeleteBar,
  CHECKBOX_CLASS,
  ConfirmDialog,
  EmptyState,
  Field,
  Input,
  LoadingBlock,
  Modal,
  ModalFooter,
  PageHeader,
  Select,
  TabSwitcher,
  Textarea,
} from "../components/ui";
import { BulkCompletionBar } from "../components/BulkCompletionBar";
import { EventCategoryBadge } from "../components/EventCategoryBadge";
import { LookupSelect } from "../components/LookupSelect";
import { IconPackage, IconPlus, IconSearch, IconTrash } from "../components/icons";
import { useToast } from "../lib/toast";
import { useListTab } from "../lib/useListTab";
import { useNarrowTables } from "../lib/useNarrowTables";
import type { OrderRecord } from "../lib/types";
import { inventoryStatus } from "./Tickets";
import { completionStatus } from "../lib/completion";

// 2.0.66: the new "Completed" indicator (see REDESIGN-2.0.66-REPORT.md) -
// deliberately separate from inventoryStatus()/ORDER_TABS above, which keep
// their existing, ticket-count-only meaning (2.0.60) unchanged. "Sold" here
// reuses the exact same soldout definition inventoryStatus() already uses
// (nothing left in available/listed - a fully cancelled order counts as
// nothing outstanding, same as that tab). Delivered/Paid are scoped to SOLD
// tickets only (see Order.deliveredCount/paidCount's own doc comments,
// models.rs) - an order with 0 sold tickets fails the Sold check already, so
// the vacuous 0-of-0 comparisons for the other two never hide anything.
export function orderCompletionChecks(o: OrderRecord) {
  return [
    { label: "Sold", done: o.availableCount === 0 && o.listedCount === 0 },
    { label: "Delivered", done: o.deliveredCount === o.soldCount },
    { label: "Paid", done: o.paidCount === o.soldCount },
  ];
}

// 2.0.60 correction (marko, after trying 2.0.59): "Active" vs "Paid" here
// was originally bucketed by an order's own Payment status field (whether
// *marko* has paid *his supplier* for the tickets) - marko clarified that's
// the wrong signal entirely. What actually means "done" for this page is
// whether the order's tickets have been sold on to a buyer - the same
// signal Tickets' own Active/Completed tabs already use (inventoryStatus,
// imported from Tickets.tsx above), not a second, different notion of
// "done" for the same ticket counts. So: Paid = inventoryStatus(o).key is
// "soldout" or "cancelled" (nothing left to sell), Active = still has
// available/listed stock - Payment status is completely irrelevant to which
// tab an order lands in now, it's just still shown as its own column, as
// before. A fully-cancelled order lands in "Paid" here for the same reason
// it lands in "Completed" on Tickets: there's nothing further to do with
// it, so leaving it in "Active" would misrepresent it as still needing
// action - flag this to marko if he'd rather it stayed in Active instead.
const ORDER_TABS: { key: "active" | "paid"; label: string }[] = [
  { key: "active", label: "Active" },
  { key: "paid", label: "Paid" },
];

// Exported (2.0.81) so PriceChecker.tsx's own currency picker (the "Check
// Prices" entry form) can reuse this exact same list instead of maintaining
// a second copy that could quietly drift from this one.
export const CURRENCIES = ["EUR", "USD", "GBP", "CHF", "CZK", "PLN", "HUF", "SEK", "NOK", "DKK", "RON", "TRY", "BGN"];

// 1.9.1: preset ticket-type options for the New Order form's "Ticket type"
// field (see submit() below) - the most common delivery formats in ticket
// resale. Free-form via "Other..." (same toggle pattern as Currency, right
// below) covers anything not listed here. This replaces "ticket type" as a
// bulk-edit field (removed from BulkTicketEditBar per marko's request) -
// it's now a one-time choice made here at order creation, copied onto every
// generated ticket (see OrderInput.ticketType / insert_order_with_tickets),
// same as Section/Row already are.
//
// 2.0.19: this is now only the FALLBACK shown until `api.listTicketTypes()`
// resolves (and the safety net if that call ever fails) - the real,
// growable list comes from the backend (`known_ticket_type_names` in
// commands/tickets.rs), seeded with exactly these same 5 values, so a value
// typed via "Other..." here (or found in a synced sheet cell) shows up as a
// real option next time, everywhere this list is used - both here and in
// the Orders & Sales sheet's own Ticket Type dropdown.
const TICKET_TYPES = ["E-ticket", "PDF", "Mobile transfer", "Physical", "Will call"];

// 1.8.3 (section 8): remembers the last-used search for this app session
// only, same convention as Sales.tsx's `lastFilters` / Tickets.tsx's
// `lastTicketsFilters`, so returning here (in particular via Order Detail's
// context-aware Back link) finds the same search instead of starting blank.
let lastOrdersSearch: string | null = null;
// 2.0.27: same session-only "remember the last filter" convention, now for
// the category filter added alongside search.
let lastOrdersCategoryId: number | "" = "";
// 2.0.65: same again, for the new Platform and date-range filters -
// list_orders_impl has supported platform_id/date_from/date_to for a while
// (Tickets.tsx already sends all three against this same command), this
// page just never sent them.
let lastOrdersPlatformId: number | "" = "";
let lastOrdersDateFrom = "";
let lastOrdersDateTo = "";
// 2.0.34: same convention again, for the new Sort control below - marko
// asked for a way to sort Orders/Events/Sales by date "so nothing gets
// lost". Sales already had this (SORT_LABELS in Sales.tsx); Orders didn't
// have any user-facing sort at all before this - the list was always
// exactly `ORDER BY o.purchase_date DESC` from orders.rs::list_orders_impl,
// with no way to flip it. Sorted client-side rather than adding a backend
// sort_by param: list_orders_impl already returns every matching order up
// to LIST_CAP (5,000, see the banner above) in one response, so the full
// result set is already in memory - sorting it here is exactly as complete
// as a server-side sort would be, without touching orders.rs at all.
let lastOrdersSortBy: string = "";

// 2.0.65: relabeled from "Newest/Oldest first" to "Soonest/Furthest first",
// default flipped from descending to ascending - same app-wide change as
// Tickets/Sales/Events/Pulls, see REDESIGN-2.0.65-REPORT.md.
const ORDER_SORT_LABELS: Record<string, string> = {
  "": "Soonest first",
  furthest: "Furthest first",
};

/** Turns the free-form "Seats" input into one label per ticket.
 * Accepts a numeric range ("12-15" -> 12,13,14,15, either direction) or a
 * comma-separated list ("12, 14, 16A"). Blank input -> no seats assigned. */
function parseSeats(raw: string): string[] {
  const trimmed = raw.trim();
  if (!trimmed) return [];
  const rangeMatch = trimmed.match(/^(\d+)\s*-\s*(\d+)$/);
  if (rangeMatch) {
    const start = parseInt(rangeMatch[1], 10);
    const end = parseInt(rangeMatch[2], 10);
    const step = start <= end ? 1 : -1;
    const out: string[] = [];
    for (let n = start; step > 0 ? n <= end : n >= end; n += step) out.push(String(n));
    return out;
  }
  return trimmed
    .split(",")
    .map((s) => s.trim())
    .filter((s) => s.length > 0);
}

export default function Orders() {
  const toast = useToast();
  const isNarrow = useNarrowTables();
  const location = useLocation();
  const navigate = useNavigate();
  const [orders, setOrders] = useState<OrderRecord[] | null>(null);
  const [categories, setCategories] = useState<EventCategory[]>([]);
  const [platforms, setPlatforms] = useState<Platform[]>([]);
  const [search, setSearch] = useState(lastOrdersSearch ?? "");
  // 2.0.27: category filter (marko's request - filter Events/Orders/Sales by
  // category). Deliberately just this one new filter - not also an Event
  // filter, which nobody asked for here.
  const [categoryId, setCategoryId] = useState<number | "">(lastOrdersCategoryId);
  // 2.0.65: Platform + date-range filters, matching Tickets/Sales/Pulls -
  // list_orders_impl already accepts all three, this page just never sent
  // them.
  const [platformId, setPlatformId] = useState<number | "">(lastOrdersPlatformId);
  const [dateFrom, setDateFrom] = useState(lastOrdersDateFrom);
  const [dateTo, setDateTo] = useState(lastOrdersDateTo);
  const [sortBy, setSortBy] = useState(lastOrdersSortBy);
  // 2.0.59: see ORDER_TABS above.
  const [tab, setTab] = useListTab("ordersTab", ["active", "paid"] as const);
  const [modalOpen, setModalOpen] = useState(false);
  const [presetEventId, setPresetEventId] = useState<number | undefined>(undefined);
  // 2.0.28: bulk-delete selection mode - marko's own request. No checkbox
  // column sitting there all the time; the "Delete" toggle button below
  // reveals it, and it disappears again the moment you confirm or cancel.
  const [selectionMode, setSelectionMode] = useState(false);
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [confirmBulkDelete, setConfirmBulkDelete] = useState(false);
  const [bulkDeleting, setBulkDeleting] = useState(false);

  useEffect(() => {
    lastOrdersSearch = search;
  }, [search]);

  useEffect(() => {
    lastOrdersCategoryId = categoryId;
  }, [categoryId]);

  useEffect(() => {
    lastOrdersPlatformId = platformId;
  }, [platformId]);

  useEffect(() => {
    lastOrdersDateFrom = dateFrom;
  }, [dateFrom]);

  useEffect(() => {
    lastOrdersDateTo = dateTo;
  }, [dateTo]);

  useEffect(() => {
    lastOrdersSortBy = sortBy;
  }, [sortBy]);

  useEffect(() => {
    api.listEventCategories().then(setCategories).catch(() => {});
    api.listPlatforms().then(setPlatforms).catch(() => {});
  }, []);

  const load = () => {
    api
      .listOrders({
        search: search || undefined,
        categoryId: categoryId || undefined,
        platformId: platformId || undefined,
        dateFrom: dateFrom || undefined,
        dateTo: dateTo || undefined,
      })
      .then(setOrders)
      .catch((e) => toast.error(errMsg(e)));
  };

  useEffect(() => {
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const t = setTimeout(load, 250);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [search, categoryId, platformId, dateFrom, dateTo]);

  // 2.0.34: `orders` itself is left exactly as the backend returned it
  // (purchase_date DESC) - sorting happens only here, on a derived copy, so
  // every other reference to `orders` above (allSelected, the >= 5000
  // banner, the empty-state checks) keeps working on the real fetched list
  // regardless of display order. 2.0.65: default flipped to ascending
  // ("Soonest first") - see ORDER_SORT_LABELS above - so now "furthest" is
  // the no-op pass-through of the backend's own order, and the default
  // reverses it. No risk of a stable-sort quirk changing same-day ordering
  // either way, since neither branch re-sorts - both just choose a
  // direction over the same backend order.
  const sortedOrders = useMemo(() => {
    if (!orders) return [];
    if (sortBy === "furthest") return orders;
    return [...orders].reverse();
  }, [orders, sortBy]);

  // 2.0.59: tab split happens client-side, after sorting, on data the page
  // already fetched - no new backend query, same "filter what's already in
  // memory" approach the sort above already uses.
  // 2.0.60: bucketed by inventoryStatus (ticket sold/cancelled counts), not
  // paymentStatus - see the ORDER_TABS comment above for why.
  const visibleOrders = useMemo(
    () => sortedOrders.filter((o) => (inventoryStatus(o).key === "active") === (tab === "active")),
    [sortedOrders, tab],
  );

  const toggleOne = (id: number) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  // 2.0.59: scoped to visibleOrders (the current tab), not every order ever
  // fetched - "select all" should only ever select what you can actually
  // see, so it can never bulk-delete something the other tab is hiding.
  const allSelected = visibleOrders.length > 0 && visibleOrders.every((o) => selected.has(o.id));
  const toggleSelectAll = () => {
    setSelected(allSelected ? new Set() : new Set(visibleOrders.map((o) => o.id)));
  };

  const exitSelectionMode = () => {
    setSelectionMode(false);
    setSelected(new Set());
  };

  const confirmDeleteSelected = async () => {
    setBulkDeleting(true);
    try {
      const result = await api.bulkDeleteOrders(Array.from(selected));
      if (result.deletedIds.length > 0) {
        toast.success(`${result.deletedIds.length} order${result.deletedIds.length === 1 ? "" : "s"} deleted`);
      }
      if (result.skipped.length > 0) {
        toast.error(`${result.skipped.length} skipped: ${summarizeBulkDeleteSkips(result.skipped)}`);
      }
      setConfirmBulkDelete(false);
      exitSelectionMode();
      load();
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setBulkDeleting(false);
    }
  };

  useEffect(() => {
    // 1.8.3 (section 11): `openCreate` (no event preset) additionally lets
    // the Dashboard's "New Order" Quick Action open this same modal without
    // pinning it to one event - purely additive, presetEventId's own
    // behavior below is unchanged.
    const state = location.state as { presetEventId?: number; openCreate?: boolean } | null;
    if (state?.presetEventId || state?.openCreate) {
      setPresetEventId(state.presetEventId);
      setModalOpen(true);
      navigate(location.pathname, { replace: true, state: null });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [location.state]);

  return (
    <div>
      <PageHeader
        title="Orders"
        subtitle="Ticket purchases. Each order automatically generates one ticket per unit."
        actions={
          <div className="flex items-center gap-2">
            {!selectionMode && orders && orders.length > 0 && (
              <Button variant="secondary" onClick={() => setSelectionMode(true)}>
                <IconTrash className="h-4 w-4" /> Delete
              </Button>
            )}
            <Button
              variant="primary"
              onClick={() => {
                setPresetEventId(undefined);
                setModalOpen(true);
              }}
            >
              <IconPlus className="h-4 w-4" /> New Order
            </Button>
          </div>
        }
      />

      <TabSwitcher tabs={ORDER_TABS} active={tab} onChange={setTab} />

      <div className="mb-4 flex flex-wrap items-end gap-3">
        <div className="w-64">
          <span className="label">Search</span>
          <div className="relative">
            <IconSearch className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-400 dark:text-slate-500" />
            <Input
              placeholder="Search orders..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="pl-9"
            />
          </div>
        </div>
        <div className="w-52">
          <span className="label">Category</span>
          <Select value={categoryId} onChange={(e) => setCategoryId(e.target.value ? Number(e.target.value) : "")}>
            <option value="">All categories</option>
            {categories.map((c) => (
              <option key={c.id} value={c.id}>
                {c.name}
              </option>
            ))}
          </Select>
        </div>
        <div className="w-40">
          <span className="label">Platform</span>
          <Select value={platformId} onChange={(e) => setPlatformId(e.target.value ? Number(e.target.value) : "")}>
            <option value="">All platforms</option>
            {/* Orders is purchase-side, same purchase/both scoping as every
                other purchase-side Platform picker in the app (see
                Tickets.tsx's own comment for the full reasoning). */}
            {platforms
              .filter((p) => p.kind === "purchase" || p.kind === "both")
              .map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
          </Select>
        </div>
        <div className="w-36">
          <span className="label">From</span>
          <Input type="date" value={dateFrom} onChange={(e) => setDateFrom(e.target.value)} />
        </div>
        <div className="w-36">
          <span className="label">To</span>
          <Input type="date" value={dateTo} onChange={(e) => setDateTo(e.target.value)} />
        </div>
        <div className="w-44">
          <span className="label">Sort</span>
          <Select value={sortBy} onChange={(e) => setSortBy(e.target.value)} aria-label="Sort orders">
            {Object.entries(ORDER_SORT_LABELS).map(([value, label]) => (
              <option key={value || "soonest"} value={value}>
                {label}
              </option>
            ))}
          </Select>
        </div>
      </div>

      {selectionMode && (
        <>
          {/* 2.0.67: marko's own request - mark Delivered/Paid on many orders
           * at once, right from this list, next to the existing bulk-delete
           * bar below (same selection, same checkboxes). See
           * BulkCompletionBar's own doc comment for why applying one action
           * here never clears the selection - marko can mark a batch
           * Delivered and then, still selected, also mark it Paid. */}
          <BulkCompletionBar
            count={selected.size}
            itemLabel="order"
            onSetDelivered={async (delivered) => {
              const updated = await api.bulkSetOrdersDeliveryStatus({
                orderIds: Array.from(selected),
                deliveryStatus: delivered ? "Delivered" : "Not delivered",
              });
              load();
              return updated;
            }}
            onSetPaid={async (paid) => {
              const updated = await api.bulkSetOrdersPaymentStatus({
                orderIds: Array.from(selected),
                paymentStatus: paid ? "paid" : "pending",
              });
              load();
              return updated;
            }}
            onClear={() => setSelected(new Set())}
          />
          <BulkDeleteBar
            count={selected.size}
            itemLabel="order"
            busy={bulkDeleting}
            onConfirm={() => setConfirmBulkDelete(true)}
            onCancel={exitSelectionMode}
          />
        </>
      )}

      {orders && orders.length >= 5000 && (
        <div className="mb-4 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-800 dark:border-amber-500/30 dark:bg-amber-500/10 dark:text-amber-400">
          Showing the most recent 5,000 orders that match your filters. Narrow the search or event filter to see the
          rest.
        </div>
      )}

      {orders === null ? (
        <LoadingBlock />
      ) : orders.length === 0 ? (
        <EmptyState
          icon={<IconPackage className="h-8 w-8" />}
          title="No orders yet"
          description="Record a ticket purchase to automatically generate its individual tickets."
          action={
            <Button variant="primary" onClick={() => setModalOpen(true)}>
              <IconPlus className="h-4 w-4" /> New Order
            </Button>
          }
        />
      ) : visibleOrders.length === 0 ? (
        // 2.0.59: orders exist, just none in the active tab.
        <EmptyState
          icon={<IconPackage className="h-8 w-8" />}
          title={tab === "active" ? "No active orders" : "No paid orders yet"}
          description={
            tab === "active"
              ? "Every order is fully sold or cancelled. Switch to the Paid tab to see them."
              : "Orders move here once every ticket in them is sold or cancelled."
          }
        />
      ) : (
        // 2.0.35: same proportional-percentage model Sales.tsx now uses -
        // see that file's colgroup comment for the full history and the
        // honest narrow-window tradeoff (applies here identically: Event
        // is better off at every width than it used to be, the fixed-
        // content columns are the ones trading some of their old
        // guaranteed floor for the table growing with the window). Order
        // also went 92px -> 120px (basis for its new percentage) in the
        // same pass - the same truncating-10-char-code bug as Sale's own
        // 2.0.33 fix ("ORD-000001" didn't fit in 92px either), flagged in
        // the 2.0.33 report's FOUND BUT NOT TOUCHED section and folded in
        // now rather than left for a separate round (Tickets.tsx has the
        // identical column/bug - see that file's own comment).
        // 2.0.37: same shift as Sales.tsx made - min-w-[1400px] plus a
        // single percentage set couldn't stop a horizontal scrollbar below
        // 1400px wide (only stop columns shrinking below their floor), and
        // marko reported exactly that scrollbar on this table. Now two full
        // percentage sets switched by the same shared useNarrowTables()
        // breakpoint as every other table: Notes and Platform hide below
        // 1690px (never Order/Event/Date/the money columns), everything
        // else grows a little and switches to the smaller
        // .th-c-narrow/.td-c-narrow. See Sales.tsx's own colgroup comment
        // and PROTECTED-AREAS-NOTES.md (2.0.37 section) for the full
        // reasoning and verification.
        // 2.0.38: the 2.0.37 percentages above were STILL wrong for Order's
        // own code column - allocated against the header text ("Order" = 5
        // chars) rather than real codes, the exact same class of bug as the
        // 2.0.33 note above, just reintroduced - see PROTECTED-AREAS-NOTES.md
        // (2.0.38 section). Recomputed against real measured content this
        // time, added the new Seats column (marko's request - shows every
        // seat across the whole order, hides below the breakpoint like
        // Notes/Platform), and switched Date to formatDateNumeric
        // ("11.09.2026", full 4-digit year - marko's explicit format
        // request) in place of the old locale-dependent formatDate call.
        // Shared breakpoint moved to 1649px (was 1690px) - see
        // useNarrowTables.ts.
        <div className="overflow-x-auto rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-sm">
          <table className="w-full table-fixed border-collapse">
            {isNarrow ? (
              <colgroup>
                {selectionMode && <col className="w-8" />}
                <col className="w-[10.488%]" />
                <col className="w-[39.659%]" />
                <col className="w-[8.659%]" />
                <col className="w-[4.39%]" />
                <col className="w-[8.293%]" />
                <col className="w-[11.22%]" />
                <col className="w-[8.293%]" />
                {/* 2.0.66: new "Completed" column - width is my own estimate
                    (not measured against real content like the rest of this
                    colgroup), taken entirely from Event's share above. Flag
                    to marko if this looks visually off. */}
                <col className="w-[9%]" />
              </colgroup>
            ) : (
              <colgroup>
                {selectionMode && <col className="w-8" />}
                <col className="w-[8.133%]" />
                <col className="w-[16.838%]" />
                <col className="w-[6.86%]" />
                <col className="w-[9.194%]" />
                <col className="w-[16.124%]" />
                <col className="w-[11.74%]" />
                <col className="w-[3.678%]" />
                <col className="w-[6.506%]" />
                <col className="w-[8.699%]" />
                <col className="w-[5.728%]" />
                {/* 2.0.66: see the narrow colgroup's identical comment above. */}
                <col className="w-[6.5%]" />
              </colgroup>
            )}
            <thead className="border-b border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-800/60">
              <tr>
                {selectionMode && (
                  <th className={isNarrow ? "th-c-narrow" : "th-c"}>
                    <input
                      type="checkbox"
                      className={CHECKBOX_CLASS}
                      checked={allSelected}
                      onChange={toggleSelectAll}
                      aria-label="Select all orders"
                    />
                  </th>
                )}
                <th className={isNarrow ? "th-c-narrow" : "th-c"}>Order</th>
                <th className={isNarrow ? "th-c-narrow" : "th-c"}>Event</th>
                <th className={isNarrow ? "th-c-narrow" : "th-c"}>Date</th>
                {!isNarrow && <th className="th-c">Seats</th>}
                {!isNarrow && <th className="th-c">Notes</th>}
                {!isNarrow && <th className="th-c">Platform</th>}
                <th className={`${isNarrow ? "th-c-narrow" : "th-c"} text-right`}>Qty</th>
                <th className={`${isNarrow ? "th-c-narrow" : "th-c"} text-right`}>Sold</th>
                <th className={`${isNarrow ? "th-c-narrow" : "th-c"} text-right`}>Total cost</th>
                <th className={isNarrow ? "th-c-narrow" : "th-c"}>Payment</th>
                <th className={isNarrow ? "th-c-narrow" : "th-c"}>Completed</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
              {visibleOrders.map((o) => (
                <tr
                  key={o.id}
                  className="cursor-pointer hover:bg-slate-50 dark:hover:bg-slate-800/60"
                  onClick={(e) => {
                    // 2.0.28: excludes the new checkbox too (its own onChange
                    // handles it), and while selectionMode is on, a row click
                    // toggles selection instead of navigating away.
                    if ((e.target as HTMLElement).closest("a, input")) return;
                    if (selectionMode) {
                      toggleOne(o.id);
                      return;
                    }
                    navigate(`/orders/${o.id}`, { state: { from: location.pathname } });
                  }}
                >
                  {selectionMode && (
                    <td className={isNarrow ? "td-c-narrow" : "td-c"}>
                      <input
                        type="checkbox"
                        className={CHECKBOX_CLASS}
                        checked={selected.has(o.id)}
                        onChange={() => toggleOne(o.id)}
                        aria-label={`Select order ${o.code}`}
                      />
                    </td>
                  )}
                  <td className={`${isNarrow ? "td-c-narrow" : "td-c"} truncate`} title={o.code}>
                    <Link
                      to={`/orders/${o.id}`}
                      state={{ from: location.pathname }}
                      className="font-medium text-slate-900 dark:text-slate-100 hover:text-brand-700 dark:hover:text-brand-400"
                    >
                      {o.code}
                    </Link>
                  </td>
                  <td className={isNarrow ? "td-c-narrow" : "td-c"} title={o.eventName}>
                    {/* 1.9.1: plain text, not a <Link> to Event Detail -
                        marko asked to remove every "this reference jumps me
                        to a different section" link across Orders/Tickets/
                        Sales. The Order code Link/row-click above is
                        unaffected - opening this exact order's own detail
                        page isn't "being thrown elsewhere". */}
                    {/* 2.0.27: category badge sits inline with the event
                        name - this is already this table's tightest column
                        at the smallest supported window (see the 1.9.10
                        comment above), so the badge is shrink-0 and the name
                        gives way via truncate rather than the other way
                        round; falls back to this table's existing
                        overflow-x-auto scroll at the extreme minimum, same
                        as that comment already documents for Event alone. */}
                    <div className="flex items-center gap-1.5">
                      <span className="truncate">{o.eventName}</span>
                      {o.categoryName && o.categoryColorSlot !== null && (
                        <span className="shrink-0">
                          <EventCategoryBadge name={o.categoryName} colorSlot={o.categoryColorSlot} />
                        </span>
                      )}
                    </div>
                  </td>
                  <td className={`${isNarrow ? "td-c-narrow" : "td-c"} whitespace-nowrap`}>{formatDateNumeric(o.purchaseDate)}</td>
                  {!isNarrow && (
                    <td className="td-c truncate" title={formatSeatsSummary(o.seats)}>
                      {formatSeatsSummary(o.seats)}
                    </td>
                  )}
                  {!isNarrow && (
                    <td className="td-c truncate text-slate-500 dark:text-slate-400" title={o.notes ?? undefined}>
                      {o.notes || "-"}
                    </td>
                  )}
                  {!isNarrow && (
                    <td className="td-c truncate" title={o.platformName ?? undefined}>{o.platformName ?? "-"}</td>
                  )}
                  <td className={`${isNarrow ? "td-c-narrow" : "td-c"} text-right tabular-nums whitespace-nowrap`}>{o.quantity}</td>
                  <td className={`${isNarrow ? "td-c-narrow" : "td-c"} text-right tabular-nums whitespace-nowrap`}>
                    {o.soldCount}/{o.quantity}
                  </td>
                  <td className={`${isNarrow ? "td-c-narrow" : "td-c"} text-right tabular-nums whitespace-nowrap`}>{formatMoney(o.totalCostCents, o.currency)}</td>
                  <td className={isNarrow ? "td-c-narrow" : "td-c"}>
                    <Badge tone={o.paymentStatus}>{o.paymentStatus}</Badge>
                  </td>
                  <td className={isNarrow ? "td-c-narrow" : "td-c"}>
                    {(() => {
                      const c = completionStatus(orderCompletionChecks(o));
                      return (
                        <Badge tone={c.tone} title={c.title}>
                          {c.label}
                        </Badge>
                      );
                    })()}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      <OrderFormModal
        open={modalOpen}
        presetEventId={presetEventId}
        onClose={() => setModalOpen(false)}
        onCreated={(order) => {
          setModalOpen(false);
          load();
          navigate(`/orders/${order.id}`, { state: { from: location.pathname } });
        }}
      />

      <ConfirmDialog
        open={confirmBulkDelete}
        title={`Delete ${selected.size} selected order${selected.size === 1 ? "" : "s"}?`}
        message="Orders with sold tickets or any sales history (including refunds) will be skipped automatically. This cannot be undone."
        confirmLabel="Delete selected"
        danger
        busy={bulkDeleting}
        onCancel={() => setConfirmBulkDelete(false)}
        onConfirm={confirmDeleteSelected}
      />
    </div>
  );
}

/** 1.9.2 (section 6): purely visual grouping for New Order's fields - wraps a
 * labelled cluster in its own mini 2-column grid, separated from the group
 * above it by a thin top border (the first group in a form has none, via
 * `first:border-t-0`). Local to this file, not promoted to ui.tsx - New
 * Order is the only form in the app asking for this kind of sectioning so
 * far; nothing else needs it yet. `title` is optional so the same component
 * can wrap Payment status/Notes at the bottom (deliberately left out of
 * marko's 4 named groups - EVENT/TICKETS/PURCHASE/SUMMARY - but still
 * visually separated from the group above them). */
function FormGroup({ title, children }: { title?: string; children: ReactNode }) {
  return (
    <div className="border-t border-slate-200 pt-4 first:border-t-0 first:pt-0 dark:border-slate-800">
      {title && (
        <p className="mb-3 text-xs font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">{title}</p>
      )}
      <div className="grid grid-cols-2 gap-4">{children}</div>
    </div>
  );
}

function OrderFormModal({
  open,
  presetEventId,
  onClose,
  onCreated,
}: {
  open: boolean;
  presetEventId?: number;
  onClose: () => void;
  onCreated: (order: OrderRecord) => void;
}) {
  const toast = useToast();
  const [events, setEvents] = useState<EventWithStats[]>([]);
  const [platforms, setPlatforms] = useState<Platform[]>([]);
  // 2.0.19: real, growable list - see TICKET_TYPES's own comment above.
  const [ticketTypeOptions, setTicketTypeOptions] = useState<string[]>(TICKET_TYPES);

  const [eventId, setEventId] = useState<number | "">("");
  const [platformId, setPlatformId] = useState<number | null>(null);
  const [purchaseDate, setPurchaseDate] = useState(todayIso());
  const [quantity, setQuantity] = useState("1");
  const [unitPrice, setUnitPrice] = useState("");
  const [unitFees, setUnitFees] = useState("0");
  const [otherCosts, setOtherCosts] = useState("0");
  // 2.0.25: "Did you buy this through a pull?" - marko's own request, moved
  // here from Order Detail (see that page's 2.0.24 "Received pulls" section,
  // still there for editing/adding one later) since he wants this recorded
  // at the moment he creates the order, not as a separate later step. Only
  // asks for the 2 things this form can't already derive - who pulled, and
  // the fee - exactly like Order Detail's own "Add pull info" action.
  const [pulled, setPulled] = useState(false);
  const [pullerName, setPullerName] = useState("");
  const [pullFee, setPullFee] = useState("0");
  const [currency, setCurrency] = useState("EUR");
  const [customCurrency, setCustomCurrency] = useState(false);
  // 2.0.50: "Convert to EUR" button next to the Currency field - own loading
  // flag (not reusing `saving`, which is for the final Create submit) so the
  // button can show "Converting..." and disable itself without touching any
  // other part of the form.
  const [convertingCurrency, setConvertingCurrency] = useState(false);
  // 2.0.50, review fix: this modal never actually unmounts when it's closed
  // (only the inner <Modal> stops rendering - see its own component), so
  // this component's state, including anything an in-flight
  // api.convertCurrency call is about to write into, survives a close and a
  // later reopen for a DIFFERENT order. Bumped in the reset effect below on
  // every open; convertToEur captures the value at the moment it starts and
  // refuses to apply a result/error/loading-state change if the token has
  // since moved on - otherwise a slow conversion request abandoned by
  // closing the modal could land later and silently overwrite whatever the
  // next order's form now contains.
  const conversionToken = useRef(0);
  // 2.0.70: defaults to "paid" now (was "unpaid") - marko's own request: most
  // of his orders are already paid by the time he enters them, so this is
  // the safer default if he forgets to touch the field at all, not just the
  // more common one. Still freely editable either way - see the Payment
  // status Select below (Paid listed first, to match).
  const [paymentStatus, setPaymentStatus] = useState<OrderPaymentStatus>("paid");
  const [ticketType, setTicketType] = useState("");
  const [customTicketType, setCustomTicketType] = useState(false);
  const [section, setSection] = useState("");
  const [rowLabel, setRowLabel] = useState("");
  const [tier, setTier] = useState("");
  const [seatsRaw, setSeatsRaw] = useState("");
  const [notes, setNotes] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!open) return;
    // 2.0.50, review fix: invalidates any convertToEur call still in flight
    // from a previous time this same modal instance was open (it doesn't
    // unmount on close, so a slow request could otherwise resolve after
    // this reset and clobber the fresh fields below with stale numbers from
    // a different order) - see conversionToken's own comment above.
    conversionToken.current += 1;
    setConvertingCurrency(false);
    api.listEvents().then(setEvents).catch(() => {});
    api.listPlatforms().then(setPlatforms).catch(() => {});
    api.listTicketTypes().then(setTicketTypeOptions).catch(() => {});
    setEventId(presetEventId ?? "");
    setPlatformId(null);
    setPurchaseDate(todayIso());
    setQuantity("1");
    setUnitPrice("");
    setUnitFees("0");
    setOtherCosts("0");
    setPulled(false);
    setPullerName("");
    setPullFee("0");
    setCurrency("EUR");
    setCustomCurrency(false);
    setPaymentStatus("paid");
    setTicketType("");
    setCustomTicketType(false);
    setSection("");
    setRowLabel("");
    setTier("");
    setSeatsRaw("");
    setNotes("");
    setError(null);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, presetEventId]);

  const qNum = parseInt(quantity, 10) || 0;

  // 1.9.2 (section 7): was a single `totalPreviewCents` number - broken into
  // named subtotals here only so the new SUMMARY bar below can show a
  // breakdown ("4 tickets / Purchase: X / Fees: X / Total: X"). The formula
  // itself (fees entered per ticket, same as unit price, so both are
  // multiplied by quantity; other costs stays a plain order-wide total) is
  // untouched - purchaseCents + feesCents + otherCents is mathematically
  // identical to the old totalPreviewCents value.
  // 2.0.25: `pullFeeCents` folds into `totalCents` the exact same way
  // `otherCents` already does - marko's own worked example ("buy tickets for
  // 200, buy the pull for 20, total purchase becomes 220"). Kept as its own
  // named field (not just added straight into `otherCents`) purely so the
  // SUMMARY bar below can show it as its own line - `submit()` further down
  // is what actually combines it into the one `otherCostsCents` this form
  // has always sent the backend, so nothing about OrderInput's shape or
  // insert_order_with_tickets's existing, tested cost allocation changes.
  const summary = useMemo(() => {
    const up = decimalStringToCents(unitPrice) ?? 0;
    const f = decimalStringToCents(unitFees) ?? 0;
    const oc = decimalStringToCents(otherCosts) ?? 0;
    const pf = pulled ? decimalStringToCents(pullFee) ?? 0 : 0;
    const purchaseCents = qNum * up;
    const feesCents = qNum * f;
    const otherCents = oc;
    const pullFeeCents = pf;
    return {
      purchaseCents,
      feesCents,
      otherCents,
      pullFeeCents,
      totalCents: purchaseCents + feesCents + otherCents + pullFeeCents,
    };
  }, [qNum, unitPrice, unitFees, otherCosts, pulled, pullFee]);

  /** 2.0.50, marko's own request/example: "mam 20gbp, podla aktualneho
   * kurzu je teraz 23,38 eur tak musi to automaticky vediet a urobit" - a
   * foreign-currency purchase converted to EUR at today's real rate in a
   * couple of clicks, rather than looking the rate up and doing the math
   * by hand. Converts every price field this form has - always Pull fee
   * too, regardless of whether "This order was pulled by someone else" is
   * currently checked (review fix: converting it is harmless while it's
   * still the default "0", and skipping it only when unchecked left a real
   * gap - check it, convert, uncheck, re-check later and the field would
   * silently still hold its old un-converted number under a now-EUR
   * label) - with ONE shared rate/round trip, then switches the field
   * values AND the currency to EUR together - so the form is left in a
   * fully consistent EUR state, never a mix of an EUR currency label with
   * amounts that are still secretly GBP/USD numbers.
   * Deliberately only touches this form's own local state before Create is
   * pressed - never an existing order (see fx.rs's own doc comment for why
   * this feature is scoped to order-creation time, not editing afterward). */
  const convertToEur = async () => {
    if (currency === "EUR" || convertingCurrency) return;
    setError(null);

    const fields: Array<[string, (v: string) => void, string]> = [
      [unitPrice, setUnitPrice, "Unit purchase price"],
      [unitFees, setUnitFees, "Unit purchase fees"],
      [otherCosts, setOtherCosts, "Other costs"],
      [pullFee, setPullFee, "Pull fee"],
    ];
    // Review fix: validate BEFORE calling out, exactly like submit() does
    // for these same fields further down. decimalStringToCents returns null
    // for genuinely invalid text (not just empty, which is 0) - silently
    // falling back to 0 here would overwrite whatever marko actually typed
    // with "0.00", and submit()'s own check couldn't catch it afterward
    // since 0 is a perfectly valid amount as far as it's concerned.
    const parsedAmounts = fields.map(([value]) => decimalStringToCents(value));
    const invalidIndex = parsedAmounts.findIndex((cents) => cents === null);
    if (invalidIndex !== -1) {
      setError(`${fields[invalidIndex][2]} is not a valid amount - fix it before converting to EUR`);
      return;
    }

    // Captured AFTER validation (an invalid field must never even show the
    // "Converting..." state) - see conversionToken's own comment for why
    // this guards every state change below.
    const myToken = conversionToken.current;
    setConvertingCurrency(true);
    try {
      const amountsCents = parsedAmounts as number[];
      const result = await api.convertCurrency(currency, "EUR", amountsCents);
      if (conversionToken.current !== myToken) return; // stale - this modal moved on to something else while the request was in flight
      if (result.convertedCents.length !== fields.length) {
        throw new Error("Currency conversion returned an unexpected number of amounts");
      }
      fields.forEach(([, setValue], i) => setValue(centsToDecimalString(result.convertedCents[i])));
      const fromCurrency = currency;
      setCurrency("EUR");
      setCustomCurrency(false);
      toast.success(
        `Converted at today's rate: 1 ${fromCurrency} = ${result.rate.toFixed(4)} EUR (rate as of ${formatDateNumeric(result.rateDate)})`,
      );
    } catch (e) {
      if (conversionToken.current !== myToken) return; // a stale request's error no longer matters either
      toast.error(errMsg(e));
    } finally {
      if (conversionToken.current === myToken) setConvertingCurrency(false);
    }
  };

  const submit = async () => {
    setError(null);
    const q = parseInt(quantity, 10);
    const upCents = decimalStringToCents(unitPrice);
    const unitFeesCents = decimalStringToCents(unitFees);
    const otherCents = decimalStringToCents(otherCosts);
    // 2.0.25: only parsed/validated at all when the checkbox is on - an
    // untouched "0" left over from before unchecking it must never block
    // submit.
    const pullFeeCents = pulled ? decimalStringToCents(pullFee) : 0;
    const seats = parseSeats(seatsRaw);

    if (!eventId) return setError("Please select an event");
    if (!Number.isFinite(q) || q < 1) return setError("Quantity must be at least 1");
    if (upCents === null) return setError("Unit price is not a valid amount");
    if (unitFeesCents === null) return setError("Fees is not a valid amount");
    if (otherCents === null) return setError("Other costs is not a valid amount");
    if (pulled && !pullerName.trim()) return setError("Who pulled is required");
    if (pullFeeCents === null) return setError("Pull fee is not a valid amount");
    if (!purchaseDate) return setError("Purchase date is required");
    if (seats.length > 0 && seats.length !== q) {
      return setError(`You entered ${seats.length} seat(s) but quantity is ${q} - provide one seat per ticket, or clear the Seats field`);
    }

    const input: OrderInput = {
      eventId: Number(eventId),
      // 1.7.4: Supplier is no longer collected on this form (see LookupSelect
      // removal below) - marko flagged it as clutter he never uses when
      // quickly creating an order. Always sending null here, rather than
      // omitting the key, keeps the intent explicit for the next reader.
      // Existing/CSV-imported orders that already have a supplier keep it -
      // this form just never sets one going forward.
      supplierId: null,
      platformId,
      purchaseDate,
      quantity: q,
      unitPriceCents: upCents,
      // Fees are entered per ticket in this form (same as unit price), but
      // the backend's OrderInput.feesCents has always meant - and still
      // means - the order-wide total (it splits that total evenly across
      // tickets via the existing, tested allocate_cents). Multiplying here
      // keeps that backend contract 100% unchanged: the total sent is an
      // exact multiple of quantity, so allocate_cents hands every ticket
      // back exactly this per-unit amount with zero remainder to distribute.
      feesCents: unitFeesCents * q,
      // 2.0.25: the pull fee (if any) is simply added on top of whatever
      // marko typed into "Other costs" - see the `summary` useMemo's own
      // comment above for why this is the ONLY change here: otherCostsCents
      // already meant "one order-wide total, split evenly across every
      // ticket via allocate_cents" before this, and the pull fee is exactly
      // that same kind of cost, so it needs no new backend field, no new
      // migration, and doesn't touch insert_order_with_tickets's actual
      // allocation logic at all - only what value this form sends it.
      otherCostsCents: otherCents + pullFeeCents,
      currency,
      paymentStatus,
      notes: notes || null,
      ticketType: ticketType.trim() || null,
      section: section || null,
      rowLabel: rowLabel || null,
      tier: tier || null,
      seats: seats.length > 0 ? seats : null,
    };

    setSaving(true);
    try {
      const created = await api.createOrder(input);
      toast.success(`Order ${created.code} created with ${created.quantity} tickets`);
      // 2.0.25: reuses the exact same linking action Order Detail's own
      // "Add pull info" (2.0.24) already calls - see
      // commands::pulls_received::link_pull_received_to_order's doc comment.
      // A failure here is deliberately never fatal to order creation itself
      // (the order and its tickets are already fully real at this point) -
      // it surfaces as its own toast instead, and marko can still add the
      // pull by hand from Order Detail's own section if this one step failed.
      if (pulled && pullerName.trim()) {
        try {
          await api.linkPullReceivedToOrder(created.id, pullerName.trim(), pullFeeCents);
        } catch (e) {
          toast.error(`Order created, but the pull could not be linked: ${errMsg(e)}`);
        }
      }
      onCreated(created);
    } catch (e) {
      setError(errMsg(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal open={open} onClose={onClose} title="New order" width="max-w-2xl">
      <div className="flex flex-col gap-4">
        <FormGroup title="Event">
          <div className="col-span-2">
            <Field label="Event" required>
              <Select value={eventId} onChange={(e) => setEventId(e.target.value ? Number(e.target.value) : "")}>
                <option value="">Select an event...</option>
                {events.map((ev) => (
                  <option key={ev.id} value={ev.id}>
                    {ev.name} {ev.eventDate ? `(${ev.eventDate})` : ""}
                  </option>
                ))}
              </Select>
            </Field>
          </div>
          <Field label="Purchase date" required>
            <Input type="date" value={purchaseDate} onChange={(e) => setPurchaseDate(e.target.value)} />
          </Field>
        </FormGroup>

        <FormGroup title="Tickets">
          <Field label="Quantity" required hint="One ticket record is generated per unit">
            <Input
              type="number"
              min={1}
              step={1}
              value={quantity}
              onChange={(e) => setQuantity(e.target.value)}
            />
          </Field>
          {/* 1.9.1: replaces "ticket type" as a bulk-edit field on Sale/Order
              Detail (removed per marko's request) - set once here, copied onto
              every ticket this order generates. Same Select + "Other..."
              freeform toggle pattern as Currency below. 1.9.2 (section 6):
              now paired half-width with Quantity (same layout Currency
              already uses next to Unit price) instead of its own full-width
              row - part of this round's form simplification. */}
          <div>
            <div className="flex items-center justify-between">
              <span className="label mb-1">Ticket type</span>
              <button
                type="button"
                className="mb-1 text-xs font-medium text-brand-600 dark:text-brand-400 hover:underline"
                onClick={() => setCustomTicketType((c) => !c)}
              >
                {customTicketType ? "Choose from list" : "Other..."}
              </button>
            </div>
            {customTicketType ? (
              <Input
                autoFocus
                placeholder="e.g. Will call"
                value={ticketType}
                onChange={(e) => setTicketType(e.target.value)}
              />
            ) : (
              <Select value={ticketType} onChange={(e) => setTicketType(e.target.value)}>
                <option value="">Not specified</option>
                {/* Same "keep a custom value visible" fallback as Currency below -
                    without this, toggling back from "Other..." after typing a
                    value not in the list would show a blank-looking select
                    even though `ticketType` state is still correct. */}
                {(ticketType && !ticketTypeOptions.includes(ticketType)
                  ? [ticketType, ...ticketTypeOptions]
                  : ticketTypeOptions
                ).map((t) => (
                  <option key={t} value={t}>
                    {t}
                  </option>
                ))}
              </Select>
            )}
          </div>

          <Field label="Section">
            <Input value={section} onChange={(e) => setSection(e.target.value)} />
          </Field>
          <Field label="Row">
            <Input value={rowLabel} onChange={(e) => setRowLabel(e.target.value)} />
          </Field>
          <Field label="Tier / Level">
            <Input value={tier} onChange={(e) => setTier(e.target.value)} />
          </Field>

          <div className="col-span-2">
            <Field
              label="Seats"
              hint={qNum > 1 ? `One per ticket, e.g. "12-${11 + qNum}" or "12,13,14" - optional` : 'Optional, e.g. "12"'}
            >
              <Input
                placeholder={qNum > 1 ? "12-15" : "12"}
                value={seatsRaw}
                onChange={(e) => setSeatsRaw(e.target.value)}
              />
            </Field>
          </div>
        </FormGroup>

        <FormGroup title="Purchase">
          {/* 1.7.4: Supplier used to sit here next to Platform - removed as
              clutter marko never uses on this form (still settable per-order
              via Edit on Order Detail, and CSV import still recognizes a
              "supplier" column, so nothing already using it breaks - 1.9.2
              section 6 reconfirms Supplier stays out of this form). Platform
              takes the full width on its own row instead of leaving an empty
              half-row gap. */}
          <div className="col-span-2">
            <LookupSelect
              label="Platform"
              // 1.9.3: Orders only offers "purchase"/"both" platforms now -
              // marko split the shared platform pool into where-you-bought
              // vs where-you-sold lists (see Settings -> Lookups). onCreate
              // right below was already tagging new platforms "purchase"
              // from here even before 1.9.3; only this display filter is new.
              options={platforms.filter((p) => p.kind === "purchase" || p.kind === "both")}
              value={platformId}
              onChange={setPlatformId}
              onCreate={async (name) => {
                const p = await api.createPlatform(name, "purchase");
                setPlatforms((prev) => [...prev, p]);
                return p;
              }}
            />
          </div>

          <Field label={`Unit purchase price (${currency})`} required>
            <Input
              inputMode="decimal"
              placeholder="0.00"
              value={unitPrice}
              onChange={(e) => setUnitPrice(e.target.value)}
              // 2.0.50, review fix: without this, editing a field while a
              // conversion request is in flight gets silently thrown away
              // the moment that request lands and overwrites every field
              // with its own (now stale) converted numbers.
              disabled={convertingCurrency}
            />
          </Field>
          <div>
            {/* 1.7.3: this used to be an <input list="currency-list"> +
                <datalist> - CURRENCIES already had 13 options (EUR/GBP/USD
                included) but a plain text input pre-filled with "EUR" gives no
                visible sign that anything else is pickable, so marko never
                found them. A real <Select> makes every option visible on
                click; "Other..." (same toggle pattern as LookupSelect's
                "+ New") still allows typing any currency code freely. */}
            <div className="flex items-center justify-between">
              <span className="label mb-1">Currency</span>
              <div className="flex items-center gap-3">
                {/* 2.0.50: only relevant once there's actually a foreign
                    currency to convert away from - hidden entirely for the
                    default EUR case rather than showing a button that would
                    just error out or no-op. */}
                {currency !== "EUR" && (
                  <button
                    type="button"
                    className="mb-1 text-xs font-medium text-brand-600 dark:text-brand-400 hover:underline disabled:opacity-50 disabled:no-underline"
                    onClick={convertToEur}
                    disabled={convertingCurrency}
                  >
                    {convertingCurrency ? "Converting..." : "Convert to EUR"}
                  </button>
                )}
                <button
                  type="button"
                  className="mb-1 text-xs font-medium text-brand-600 dark:text-brand-400 hover:underline disabled:opacity-50 disabled:no-underline"
                  onClick={() => setCustomCurrency((c) => !c)}
                  // 2.0.50, review fix: switching currency mid-flight would
                  // make a landing conversion result apply to the WRONG
                  // "from" currency (it was fetched for whatever currency
                  // was selected when Convert was clicked).
                  disabled={convertingCurrency}
                >
                  {customCurrency ? "Choose from list" : "Other..."}
                </button>
              </div>
            </div>
            {customCurrency ? (
              <Input
                autoFocus
                placeholder="e.g. AED"
                value={currency}
                onChange={(e) => setCurrency(e.target.value.toUpperCase())}
                disabled={convertingCurrency}
              />
            ) : (
              <Select value={currency} onChange={(e) => setCurrency(e.target.value)} disabled={convertingCurrency}>
                {(CURRENCIES.includes(currency) ? CURRENCIES : [currency, ...CURRENCIES]).map((c) => (
                  <option key={c} value={c}>
                    {c}
                  </option>
                ))}
              </Select>
            )}
          </div>

          <Field label={`Unit purchase fees (${currency})`}>
            <Input inputMode="decimal" value={unitFees} onChange={(e) => setUnitFees(e.target.value)} disabled={convertingCurrency} />
          </Field>
          <Field label="Other costs (total)" hint="Split evenly across all tickets">
            <Input inputMode="decimal" value={otherCosts} onChange={(e) => setOtherCosts(e.target.value)} disabled={convertingCurrency} />
          </Field>

          {/* 2.0.25: marko's own request - record a pull right when creating
              the order, instead of a separate trip to Order Detail
              afterwards (that page's own "Received pulls" section, added in
              2.0.24, still exists for editing one or adding one later).
              Collapsed behind a checkbox rather than always-visible fields,
              same "only ask when relevant" pattern as Currency/Ticket type's
              own "Other..." toggles above - most orders were never pulled. */}
          <div className="col-span-2">
            <label className="flex items-center gap-2">
              <input
                type="checkbox"
                className={CHECKBOX_CLASS}
                checked={pulled}
                onChange={(e) => setPulled(e.target.checked)}
              />
              <span className="text-sm text-slate-700 dark:text-slate-300">This order was pulled by someone else</span>
            </label>
          </div>
          {pulled && (
            <>
              <Field label="Who pulled" required hint="Who pulled these tickets for you">
                <Input autoFocus value={pullerName} onChange={(e) => setPullerName(e.target.value)} />
              </Field>
              <Field label={`Pull fee (${currency})`} hint="What you paid them - added to the total purchase price below">
                <Input
                  inputMode="decimal"
                  placeholder="0.00"
                  value={pullFee}
                  onChange={(e) => setPullFee(e.target.value)}
                  disabled={convertingCurrency}
                />
              </Field>
            </>
          )}
        </FormGroup>

        {/* Payment status/Notes deliberately aren't one of marko's 4 named
            groups (EVENT/TICKETS/PURCHASE/SUMMARY) - FormGroup without a
            title still gives them the same divider separation as a named
            group, just no heading text. */}
        <FormGroup>
          <div className="col-span-2">
            <Field label="Payment status">
              <Select value={paymentStatus} onChange={(e) => setPaymentStatus(e.target.value as OrderPaymentStatus)}>
                <option value="paid">Paid</option>
                <option value="partial">Partial</option>
                <option value="unpaid">Unpaid</option>
              </Select>
            </Field>
          </div>
          <div className="col-span-2">
            <Field label="Notes">
              <Textarea rows={2} value={notes} onChange={(e) => setNotes(e.target.value)} />
            </Field>
          </div>
        </FormGroup>
      </div>

      {/* 1.9.2 (section 7): was a single "Total cost (preview)" line - kept
          the exact same underlying calculation (see the `summary` useMemo
          above; purchaseCents + feesCents + otherCents is mathematically
          identical to the old totalPreviewCents value) and only improved the
          presentation, breaking it into the ticket count and each cost
          component the way marko's brief spelled out, e.g. "4 tickets ·
          Purchase: EUR200.00 · Fees: EUR8.00 · Total: EUR208.00". */}
      <div className="mt-4">
        <p className="mb-2 text-xs font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">Summary</p>
        <div className="flex flex-wrap items-center gap-x-2 gap-y-1 rounded-lg bg-slate-50 dark:bg-slate-800/60 px-4 py-3 text-sm text-slate-500 dark:text-slate-400">
          <span>
            {qNum} ticket{qNum === 1 ? "" : "s"}
          </span>
          <span className="text-slate-300 dark:text-slate-600">&middot;</span>
          <span>Purchase: {formatMoney(summary.purchaseCents, currency)}</span>
          <span className="text-slate-300 dark:text-slate-600">&middot;</span>
          <span>Fees: {formatMoney(summary.feesCents, currency)}</span>
          {summary.otherCents !== 0 && (
            <>
              <span className="text-slate-300 dark:text-slate-600">&middot;</span>
              <span>Other costs: {formatMoney(summary.otherCents, currency)}</span>
            </>
          )}
          {summary.pullFeeCents !== 0 && (
            <>
              <span className="text-slate-300 dark:text-slate-600">&middot;</span>
              <span>Pull fee: {formatMoney(summary.pullFeeCents, currency)}</span>
            </>
          )}
          <span className="text-slate-300 dark:text-slate-600">&middot;</span>
          <span className="font-semibold text-slate-900 dark:text-slate-100">
            Total: {formatMoney(summary.totalCents, currency)}
          </span>
        </div>
      </div>

      {error && <p className="mt-3 text-sm text-red-600 dark:text-red-400">{error}</p>}
      <ModalFooter>
        <Button variant="secondary" onClick={onClose} disabled={saving}>
          Cancel
        </Button>
        <Button variant="primary" onClick={submit} disabled={saving}>
          {saving ? "Creating..." : "Create order"}
        </Button>
      </ModalFooter>
    </Modal>
  );
}

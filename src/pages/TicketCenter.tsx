import { useEffect, useMemo, useState } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { api, errMsg } from "../lib/api";
import type { EventWithStats, OrderRecord } from "../lib/types";
import { formatDateNumeric } from "../lib/format";
import { Badge, Card, EmptyState, Input, LoadingBlock, PageHeader, Select } from "../components/ui";
import { EventCategoryBadge } from "../components/EventCategoryBadge";
import { IconSearch } from "../components/icons";
import { orderCompletionChecks } from "./Orders";
import { completionStatus } from "../lib/completion";
import { useToast } from "../lib/toast";
import { useNarrowTables } from "../lib/useNarrowTables";

// 2.5.1: "Ticket Center" - marko's own explicit rework. Until this version it
// was a Finance subtab with two sub-pages, Control Center (2.4.3) and
// Fulfillment Center (2.2.12), both built around a flat list of individual
// TICKETS (or sale batches). Marko's own words: it should work on ORDERS
// instead, which you open to see what needs doing with which tickets - not a
// second place that edits tickets directly. This is now a completely
// different page built on completely different data, not a restyle of the
// old ones (both deleted this version - see PROTECTED_AREAS.md's 2.5.1
// entry for the one thing that's now orphaned as a result).
//
// It deliberately reuses, rather than re-derives, everything it needs:
// - The exact same `OrderRecord[]` Orders.tsx already loads via
//   `api.listOrders` (availableCount/listedCount/soldCount/deliveredCount/
//   paidCount are already computed there, in SQL, per order) - no new
//   backend command, no new query, no new migration.
// - The exact same `orderCompletionChecks`/`completionStatus` pair Orders.tsx
//   and OrderDetail.tsx already use for their own "Completed" badge -
//   imported from Orders.tsx rather than re-implemented, so this page can
//   never disagree with either of them about what "done" means for an order.
// - Clicking a row goes straight to the existing OrderDetail page
//   (`/orders/:id`), which already lists every ticket in that order with its
//   own Status/Delivery status/Payout status - each independently editable
//   right there. That page already IS "what needs to be done with which
//   tickets" - nothing about it needed to change for this rework beyond
//   recognizing Ticket Center as a 4th place that can link into it (see that
//   file's own 2.5.1 comment on `backTo`).
//
// The 4 tiles below intentionally fold BOTH old subtabs' concerns into one
// set of order-level lenses instead of duplicating Control Center's full
// 12-filter toolbar (most of which - tier/section/row/marketplace - doesn't
// even make sense once the grain is "one row per order" instead of "one row
// per ticket"): Needs listing covers what Control Center used to called
// "Unlisted" stock, Needs payment/Needs delivery cover exactly what
// Fulfillment Center's own Awaiting Payment/Awaiting Delivery tiles did -
// just recomputed from OrderRecord's own counts instead of a SaleGroup's.
// This page's own universe is always "pending" orders (same "pending" idea
// as Fulfillment Center's own `pending` filter) - a fully wrapped-up order
// has nothing left to do, and stays easy to find on Orders.tsx's own
// Active/Completed tabs instead.

type CategoryKey = "pending" | "listing" | "payment" | "delivery";

const CATEGORIES: { key: CategoryKey; title: string; subtext: string }[] = [
  { key: "pending", title: "Needs attention", subtext: "Every order not yet fully sold, paid, and delivered" },
  { key: "listing", title: "Needs listing", subtext: "Still has tickets sitting available, not listed for sale" },
  { key: "payment", title: "Needs payment", subtext: "Sold tickets still waiting on the buyer's payment" },
  { key: "delivery", title: "Needs delivery", subtext: "Sold tickets still waiting on delivery" },
];

function matchesCategory(o: OrderRecord, key: CategoryKey): boolean {
  if (key === "listing") return o.availableCount > 0;
  if (key === "payment") return o.soldCount > 0 && o.paidCount !== o.soldCount;
  if (key === "delivery") return o.soldCount > 0 && o.deliveredCount !== o.soldCount;
  return true; // "pending" - this page's own universe (see `pending` below) already is this set
}

/** Buyer-side payment, scoped to this order's SOLD tickets only - "-" when
 * nothing is sold yet (nothing to pay). Same tone vocabulary Orders.tsx's own
 * `paymentStatus` Badge already uses ("paid"/"pending"), plus "mixed" for a
 * partially-paid batch, the same tone the old Fulfillment Center's delivery
 * column used for its own partial case. */
function buyerPaymentCell(o: OrderRecord) {
  if (o.soldCount === 0) return <span className="text-slate-400 dark:text-slate-500">-</span>;
  if (o.paidCount === o.soldCount) return <Badge tone="paid">Paid</Badge>;
  if (o.paidCount === 0) return <Badge tone="pending">Pending</Badge>;
  return (
    <Badge tone="mixed" title={`${o.paidCount} of ${o.soldCount} sold tickets paid`}>
      {o.paidCount}/{o.soldCount} paid
    </Badge>
  );
}

/** Same shape as buyerPaymentCell, for deliveredCount/soldCount - identical
 * tone choices to the old Fulfillment Center's own deliveryStatusBadge. */
function deliveryCell(o: OrderRecord) {
  if (o.soldCount === 0) return <span className="text-slate-400 dark:text-slate-500">-</span>;
  if (o.deliveredCount === o.soldCount) return <Badge tone="delivered">Delivered</Badge>;
  if (o.deliveredCount === 0) return <Badge tone="not delivered">Not delivered</Badge>;
  return (
    <Badge tone="mixed" title={`${o.deliveredCount} of ${o.soldCount} sold tickets delivered`}>
      {o.deliveredCount}/{o.soldCount} delivered
    </Badge>
  );
}

/** Same visual language as the old Fulfillment Center's own category tiles
 * (StatCard-style number + always-one-selected), just ported to this page's
 * own 4 categories. */
function CategoryCard({
  title,
  subtext,
  count,
  selected,
  onSelect,
}: {
  title: string;
  subtext: string;
  count: number;
  selected: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onSelect}
      className={`rounded-xl border p-4 text-left transition-colors ${
        selected
          ? "border-brand-500 bg-brand-50/60 dark:border-brand-500 dark:bg-brand-500/10"
          : "border-slate-200 bg-white hover:bg-slate-50 dark:border-slate-800 dark:bg-slate-900 dark:hover:bg-slate-800/60"
      }`}
    >
      <p className="text-xs font-medium uppercase tracking-wide text-slate-400 dark:text-slate-500">{title}</p>
      <p className="mt-1.5 text-2xl font-semibold tabular-nums text-slate-900 dark:text-slate-100">{count}</p>
      <p className="mt-1 text-xs text-slate-400 dark:text-slate-500">{subtext}</p>
    </button>
  );
}

// Session-only filter memory, same convention as every other filterable list
// in this app (e.g. TicketControlCenter's own now-removed
// `lastControlCenterFilters`).
interface TicketCenterFilterState {
  search: string;
  eventId: number | "";
  category: CategoryKey;
}
let lastTicketCenterFilters: TicketCenterFilterState | null = null;

export default function TicketCenter() {
  const toast = useToast();
  const navigate = useNavigate();
  const location = useLocation();
  const isNarrow = useNarrowTables();
  const cached = lastTicketCenterFilters;

  const [orders, setOrders] = useState<OrderRecord[] | null>(null);
  const [events, setEvents] = useState<EventWithStats[]>([]);
  const [search, setSearch] = useState(cached?.search ?? "");
  const [eventId, setEventId] = useState<number | "">(cached?.eventId ?? "");
  const [category, setCategory] = useState<CategoryKey>(cached?.category ?? "pending");

  useEffect(() => {
    api.listEvents().then(setEvents).catch(() => {});
  }, []);

  useEffect(() => {
    lastTicketCenterFilters = { search, eventId, category };
  }, [search, eventId, category]);

  useEffect(() => {
    const t = setTimeout(() => {
      api
        .listOrders({ search: search || undefined, eventId: eventId || undefined })
        .then(setOrders)
        .catch((e) => toast.error(errMsg(e)));
    }, 200);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [search, eventId]);

  // This page's own universe: orders not yet fully sold+paid+delivered - the
  // exact same completionStatus/orderCompletionChecks pair Orders.tsx's and
  // OrderDetail.tsx's "Completed" badge already use (imported, not
  // reimplemented), so an order can never read "done" here while still
  // showing as outstanding everywhere else, or vice versa.
  const pending = useMemo(
    () => (orders ? orders.filter((o) => completionStatus(orderCompletionChecks(o)).tone === "pending") : null),
    [orders],
  );

  const counts = useMemo(() => {
    if (!pending) return null;
    return {
      pending: pending.length,
      listing: pending.filter((o) => matchesCategory(o, "listing")).length,
      payment: pending.filter((o) => matchesCategory(o, "payment")).length,
      delivery: pending.filter((o) => matchesCategory(o, "delivery")).length,
    };
  }, [pending]);

  const visible = useMemo(() => (pending ? pending.filter((o) => matchesCategory(o, category)) : []), [pending, category]);
  const activeCategory = CATEGORIES.find((c) => c.key === category)!;

  const openOrder = (o: OrderRecord) => navigate(`/orders/${o.id}`, { state: { from: location.pathname } });

  return (
    <div>
      <PageHeader
        title="Ticket Center"
        subtitle="Orders that still need something done - open one to see exactly which tickets and what's outstanding."
      />

      <div className="mb-4 flex flex-wrap items-end gap-3">
        <div className="w-64">
          <span className="label">Search</span>
          <div className="relative">
            <IconSearch className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-400 dark:text-slate-500" />
            <Input placeholder="Order or event..." value={search} onChange={(e) => setSearch(e.target.value)} className="pl-9" />
          </div>
        </div>
        <div className="w-56">
          <span className="label">Event</span>
          <Select value={eventId} onChange={(e) => setEventId(e.target.value ? Number(e.target.value) : "")}>
            <option value="">All events</option>
            {events.map((ev) => (
              <option key={ev.id} value={ev.id}>
                {ev.name}
              </option>
            ))}
          </Select>
        </div>
      </div>

      {!pending || !counts ? (
        <LoadingBlock label="Loading ticket center..." />
      ) : (
        <>
          <div className="mb-6 grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-4">
            {CATEGORIES.map((c) => (
              <CategoryCard
                key={c.key}
                title={c.title}
                subtext={c.subtext}
                count={counts[c.key]}
                selected={category === c.key}
                onSelect={() => setCategory(c.key)}
              />
            ))}
          </div>

          {visible.length === 0 ? (
            <Card className="p-4">
              <EmptyState
                title={counts.pending === 0 ? "Nothing needs attention right now" : `Nothing in "${activeCategory.title}" right now`}
                description={
                  counts.pending === 0
                    ? "Every order is fully sold, paid, and delivered (or has nothing outstanding for its event)."
                    : "Try one of the other categories above."
                }
              />
            </Card>
          ) : (
            <div className="overflow-x-auto rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-sm">
              <table className="w-full table-fixed border-collapse">
                {isNarrow ? (
                  <colgroup>
                    <col className="w-[30%]" />
                    <col className="w-[16%]" />
                    <col className="w-[22%]" />
                    <col className="w-[16%]" />
                    <col className="w-[16%]" />
                  </colgroup>
                ) : (
                  <colgroup>
                    <col className="w-[26%]" />
                    <col className="w-[13%]" />
                    <col className="w-[8%]" />
                    <col className="w-[16%]" />
                    <col className="w-[13%]" />
                    <col className="w-[13%]" />
                    <col className="w-[11%]" />
                  </colgroup>
                )}
                <thead className="border-b border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-800/60">
                  <tr>
                    <th className={isNarrow ? "th-c-narrow" : "th-c"}>Event</th>
                    <th className={isNarrow ? "th-c-narrow" : "th-c"}>Order</th>
                    {!isNarrow && <th className="th-c text-right">Qty</th>}
                    <th className={isNarrow ? "th-c-narrow" : "th-c"}>Stock</th>
                    <th className={isNarrow ? "th-c-narrow" : "th-c"}>Payment</th>
                    {!isNarrow && <th className="th-c">Delivery</th>}
                    <th className={isNarrow ? "th-c-narrow" : "th-c"}>Completed</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
                  {visible.map((o) => {
                    const c = completionStatus(orderCompletionChecks(o));
                    const cellCls = isNarrow ? "td-c-narrow" : "td-c";
                    return (
                      <tr key={o.id} onClick={() => openOrder(o)} className="cursor-pointer hover:bg-slate-50 dark:hover:bg-slate-800/60">
                        <td className={cellCls} title={o.eventName}>
                          <div className="flex items-center gap-1.5">
                            <span className="truncate font-medium text-slate-900 dark:text-slate-100">{o.eventName}</span>
                            {o.categoryName && o.categoryColorSlot !== null && (
                              <span className="shrink-0">
                                <EventCategoryBadge name={o.categoryName} colorSlot={o.categoryColorSlot} />
                              </span>
                            )}
                          </div>
                          {o.eventDate && <div className="text-xs text-slate-400 dark:text-slate-500">{formatDateNumeric(o.eventDate)}</div>}
                        </td>
                        <td className={`${cellCls} truncate`} title={o.code}>
                          {o.code}
                        </td>
                        {!isNarrow && <td className="td-c text-right tabular-nums whitespace-nowrap">{o.quantity}</td>}
                        <td className={cellCls}>
                          <div className="flex flex-wrap gap-x-2 text-xs tabular-nums text-slate-500 dark:text-slate-400">
                            <span title="Available">{o.availableCount} avail</span>
                            <span title="Listed">{o.listedCount} listed</span>
                            <span title="Sold" className="font-medium text-slate-700 dark:text-slate-300">
                              {o.soldCount} sold
                            </span>
                          </div>
                        </td>
                        <td className={cellCls}>{buyerPaymentCell(o)}</td>
                        {!isNarrow && <td className="td-c">{deliveryCell(o)}</td>}
                        <td className={cellCls}>
                          <Badge tone={c.tone} title={c.title}>
                            {c.label}
                          </Badge>
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          )}
          <p className="mt-3 text-xs text-slate-400 dark:text-slate-500">
            {visible.length} order{visible.length === 1 ? "" : "s"} - open one to see and update its individual tickets.
          </p>
        </>
      )}
    </div>
  );
}

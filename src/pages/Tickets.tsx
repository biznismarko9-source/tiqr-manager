import { useEffect, useMemo, useState } from "react";
import { Link, useLocation, useSearchParams } from "react-router-dom";
import { api, errMsg } from "../lib/api";
import type { EventCategory, EventWithStats, OrderRecord, Platform, Ticket, TicketStatus, TicketUpdateInput } from "../lib/types";
import { formatDateNumeric, formatMoney, formatSeatsSummary } from "../lib/format";
import {
  Badge,
  Button,
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
import { IconBoxes, IconSearch } from "../components/icons";
import { useToast } from "../lib/toast";
import { useListTab } from "../lib/useListTab";
import { useNarrowTables } from "../lib/useNarrowTables";

// 2.0.59: "Active" vs "Completed" tabs (marko's request - see
// REDESIGN-2.0.59-REPORT.md). Reuses inventoryStatus() below - the exact
// same per-order bucketing already shown as this table's own Status badge -
// rather than inventing a second notion of "done" for the same data. Only
// rendered on Tickets itself (guarded by !lockedStatus, same as the old
// Status filter it replaces) - Inventory is already locked to
// available/listed tickets only, so it can never have anything "Completed"
// to move out of the way (see the 2.0.59 report for the full reasoning).
const TICKETS_TABS: { key: "active" | "completed"; label: string }[] = [
  { key: "active", label: "Active" },
  { key: "completed", label: "Completed" },
];

export default function Tickets() {
  return (
    <TicketsView
      title="Tickets"
      subtitle="Every order you've purchased, grouped with its tickets."
      // 1.9.3: marko asked for the Order/Event links back on this page -
      // the 1.9.1 "no cross-section navigation" rule turned out to be too
      // broad here. Inventory already had this exception (see
      // allowCrossLinks' doc comment below); Tickets now gets the exact
      // same treatment. Sales stays link-free - it wasn't part of this ask.
      allowCrossLinks
    />
  );
}

/** An order's inventory status, derived purely from its ticket counts (there
 * is no separate DB column for this - see the report). "Active" means it
 * still has stock that could be listed/sold; "Sold out" means every ticket
 * has been sold or cancelled; "Cancelled" means the whole order was voided.
 * Exported (2.0.60) - Orders.tsx's own Active/Paid tabs reuse this exact
 * bucketing instead of a second, different notion of "done" - see that
 * file's own comment for why. */
export function inventoryStatus(o: OrderRecord): { key: string; label: string } {
  if (o.cancelledCount === o.quantity && o.quantity > 0) return { key: "cancelled", label: "Cancelled" };
  if (o.availableCount + o.listedCount > 0) return { key: "active", label: "Active" };
  return { key: "soldout", label: "Sold out" };
}

// 1.8.3 (section 8 of the brief): remembers each page's last-used filters
// for this app session only, same module-level/session-only convention
// Sales.tsx already established in 1.8.0 (see its own `lastFilters`). Keyed
// by pathname (not a single shared value) because this one component backs
// TWO different pages - Tickets ("/tickets") and Inventory ("/inventory") -
// which must never leak each other's search/filters into one another.
interface TicketsFilterState {
  search: string;
  eventId: number | "";
  platformId: number | "";
  // 2.0.27 equivalent for this page - category filter, added 2.0.65 to
  // match Orders/Events/Sales, which have all had one since 2.0.27.
  categoryId: number | "";
  dateFrom: string;
  dateTo: string;
  sortBy: string;
}
const lastTicketsFilters = new Map<string, TicketsFilterState>();

// 2.0.37: marko asked for the same Sort control Sales/Orders/Events already
// have, added here too (and to both of Pulls.tsx's tabs), sorted client-side
// by purchase date for exactly the same reason Orders.tsx's own version is
// client-side (listOrders already returns the full matching result set in
// one response, up to LIST_CAP, so sorting what's already in memory is
// exactly as complete as a backend sort would be). Keyed into the existing
// per-pathname lastTicketsFilters map (not a bare module variable like
// Orders.tsx's simpler lastOrdersSortBy) so Tickets and Inventory - two
// different pages sharing this one component - keep their own separate sort
// preference, same as every other filter on this page already does.
//
// 2.0.65: relabeled from "Newest/Oldest first" to "Soonest/Furthest first",
// and the default flipped from descending to ascending - same app-wide
// change as Orders/Sales/Events/Pulls, see REDESIGN-2.0.65-REPORT.md.
const TICKET_SORT_LABELS: Record<string, string> = {
  "": "Soonest first",
  furthest: "Furthest first",
};

/** Shared list view, reused (pre-filtered) by the Inventory page. Groups
 * tickets by their order - one row per order, not per ticket - so the list
 * stays fast and readable no matter how many individual tickets an order
 * generated. Clicking a row opens the existing Order Detail page, which
 * loads that order's individual tickets (and a sales summary) on demand. */
export function TicketsView({
  title,
  subtitle,
  lockedStatus,
  allowCrossLinks = false,
}: {
  title: string;
  subtitle: string;
  lockedStatus?: string;
  /** 1.9.2 (section 1) carved out Inventory as the one page allowed to keep
   * Order->Order Detail / Event->Event Detail as clickable links, as an
   * exception to the 1.9.1 "no cross-section navigation links" rule. 1.9.3
   * extended the same exception to Tickets itself - marko asked for the
   * links back there too. Sales (which shares none of this component) stays
   * link-free; it was never part of either ask. Defaults to false only as a
   * safe fallback for any future caller that doesn't specify it. */
  allowCrossLinks?: boolean;
}) {
  const toast = useToast();
  const isNarrow = useNarrowTables();
  const location = useLocation();
  const [params] = useSearchParams();
  const cached = lastTicketsFilters.get(location.pathname);
  const [orders, setOrders] = useState<OrderRecord[] | null>(null);
  const [events, setEvents] = useState<EventWithStats[]>([]);
  const [platforms, setPlatforms] = useState<Platform[]>([]);
  const [categories, setCategories] = useState<EventCategory[]>([]);
  const [search, setSearch] = useState(params.get("code") ?? cached?.search ?? "");
  const [eventId, setEventId] = useState<number | "">(cached?.eventId ?? "");
  const [platformId, setPlatformId] = useState<number | "">(cached?.platformId ?? "");
  // 2.0.65: Category filter (matches Orders/Events/Sales). Replaces the old
  // Section search box, which marko asked to be removed - see this page's
  // own report for why (freeform section codes made it the least useful
  // filter here, and the least consistent with every other list page).
  const [categoryId, setCategoryId] = useState<number | "">(cached?.categoryId ?? "");
  const [dateFrom, setDateFrom] = useState(cached?.dateFrom ?? "");
  const [dateTo, setDateTo] = useState(cached?.dateTo ?? "");
  const [sortBy, setSortBy] = useState(cached?.sortBy ?? "");
  // 2.0.59: see TICKETS_TABS above. Called unconditionally (React hooks
  // can't be conditional), but only ever rendered/applied when !lockedStatus
  // - Inventory keeps its own fixed lockedStatus untouched, exactly as
  // before.
  const [tab, setTab] = useListTab("ticketsTab", ["active", "completed"] as const);

  useEffect(() => {
    api.listEvents().then(setEvents).catch(() => {});
    api.listPlatforms().then(setPlatforms).catch(() => {});
    api.listEventCategories().then(setCategories).catch(() => {});
  }, []);

  // 1.8.3 (section 8): persist this page's own filters (see
  // lastTicketsFilters above) so returning here - in particular via Order
  // Detail's now context-aware Back link - finds the same search/filters
  // instead of a blank slate.
  useEffect(() => {
    lastTicketsFilters.set(location.pathname, { search, eventId, platformId, categoryId, dateFrom, dateTo, sortBy });
  }, [location.pathname, search, eventId, platformId, categoryId, dateFrom, dateTo, sortBy]);

  const load = () => {
    api
      .listOrders({
        search: search || undefined,
        eventId: eventId || undefined,
        platformId: platformId || undefined,
        // 2.0.59: lockedStatus directly - Inventory's own fixed filter,
        // completely unchanged. The old user-adjustable `status` dropdown
        // this used to also come from is gone on Tickets itself; the new
        // Active/Completed tabs below replace it, filtering client-side
        // instead (see visibleOrders), same as Events/Orders/Sales' own new
        // tabs.
        status: lockedStatus || undefined,
        categoryId: categoryId || undefined,
        dateFrom: dateFrom || undefined,
        dateTo: dateTo || undefined,
      })
      .then(setOrders)
      .catch((e) => toast.error(errMsg(e)));
  };

  useEffect(() => {
    const t = setTimeout(load, 200);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [search, eventId, platformId, categoryId, dateFrom, dateTo]);

  // 2.0.37: same client-side sort convention as Orders.tsx's own
  // sortedOrders - `orders` itself stays exactly as the backend returned it
  // (purchase_date DESC) so the >= 5000 banner/every other reference above
  // keeps working regardless of display order; only the table's own render
  // switches to this derived, optionally-reversed copy. 2.0.65: default
  // flipped to ascending ("Soonest first") - see TICKET_SORT_LABELS above -
  // so now it's "furthest" that's the backend's own order verbatim, and the
  // default that reverses it.
  const sortedOrders = useMemo(() => {
    if (!orders) return [];
    if (sortBy === "furthest") return orders;
    return [...orders].reverse();
  }, [orders, sortBy]);

  // 2.0.59: tab split happens client-side, after sorting, on data the page
  // already fetched - reuses inventoryStatus() above (the exact same
  // bucketing already shown as this table's own Status badge) rather than a
  // second notion of "done". lockedStatus (Inventory) skips this entirely -
  // it shows every order matching its own fixed backend filter, unchanged.
  const visibleOrders = useMemo(() => {
    if (lockedStatus) return sortedOrders;
    return sortedOrders.filter((o) => (inventoryStatus(o).key === "active") === (tab === "active"));
  }, [sortedOrders, tab, lockedStatus]);

  // 2.0.59: scoped to visibleOrders (the current tab on Tickets; every
  // fetched order on Inventory, same as before) so the caption always
  // describes what's actually on screen.
  const summary = useMemo(() => {
    if (!orders) return null;
    const totalTickets = visibleOrders.reduce((sum, o) => sum + o.quantity, 0);
    const availableTickets = visibleOrders.reduce((sum, o) => sum + o.availableCount + o.listedCount, 0);
    return { orderCount: visibleOrders.length, totalTickets, availableTickets };
  }, [orders, visibleOrders]);

  return (
    <div>
      <PageHeader title={title} subtitle={subtitle} />

      {/* 2.0.59: replaces the old Status dropdown below (Tickets only -
          Inventory never showed it either, same !lockedStatus guard). See
          TICKETS_TABS above. */}
      {!lockedStatus && <TabSwitcher tabs={TICKETS_TABS} active={tab} onChange={setTab} />}

      {/* 2.0.32: max-w-[1400px] added, matching the table below it, so the
          "N orders · N tickets · N still sellable" summary caption (ml-auto
          below) sat above the table instead of floating off to the real
          window's right edge. 2.0.37: removed again - the table below is
          now a pure-percentage, always-fills-the-window layout (same
          reasoning as Sales.tsx's 2.0.35 change), so there's no longer a
          narrower table edge for this row to match; both now fill the real
          window width together, same as Sales/Orders/Events already do. */}
      <div className="mb-4 flex flex-wrap items-end gap-3">
        <div className="w-52">
          <span className="label">Search</span>
          <div className="relative">
            <IconSearch className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-400 dark:text-slate-500" />
            <Input
              placeholder="Order code, event..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="pl-9"
            />
          </div>
        </div>
        <div className="w-48">
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
        <div className="w-40">
          <span className="label">Platform</span>
          {/* 1.9.4: marko - Tickets/Inventory is purchased stock, so only
              purchase-side platforms make sense to filter by here (a
              sale-only platform could never match any ticket's own
              purchase platform anyway). Same purchase/both scoping as the
              New Order and Edit Order Platform pickers. The Sales list
              filter used to be the one deliberate exception (left
              unscoped) - as of 1.9.5 it's scoped too (to sale+both, see
              its own comment in Sales.tsx), so every Platform picker in
              the app now follows this same purchase/sale split. */}
          <Select value={platformId} onChange={(e) => setPlatformId(e.target.value ? Number(e.target.value) : "")}>
            <option value="">All platforms</option>
            {platforms
              .filter((p) => p.kind === "purchase" || p.kind === "both")
              .map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
          </Select>
        </div>
        <div className="w-40">
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
            {Object.entries(TICKET_SORT_LABELS).map(([value, label]) => (
              <option key={value || "soonest"} value={value}>
                {label}
              </option>
            ))}
          </Select>
        </div>
        {summary && (
          <p className="ml-auto text-xs text-slate-400 dark:text-slate-500">
            {summary.orderCount} orders &middot; {summary.totalTickets} tickets &middot; {summary.availableTickets} still
            sellable
          </p>
        )}
      </div>

      {orders && orders.length >= 5000 && (
        <div className="mb-4 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-800 dark:border-amber-500/30 dark:bg-amber-500/10 dark:text-amber-400">
          Showing the most recent 5,000 orders that match your filters. Narrow the search, status, or event filter to
          see the rest.
        </div>
      )}

      {orders === null ? (
        <LoadingBlock />
      ) : orders.length === 0 ? (
        <EmptyState icon={<IconBoxes className="h-8 w-8" />} title="No orders match these filters" />
      ) : visibleOrders.length === 0 ? (
        // 2.0.59: orders match the filters above, just none in the active
        // tab (Tickets only - lockedStatus/Inventory never hits this, since
        // visibleOrders equals sortedOrders there).
        <EmptyState
          icon={<IconBoxes className="h-8 w-8" />}
          title={tab === "active" ? "No active stock" : "Nothing completed yet"}
          description={
            tab === "active"
              ? "Every matching order is sold out or cancelled. Switch to the Completed tab to see them."
              : "Orders move here once every ticket is sold or the order is cancelled."
          }
        />
      ) : (
        // 1.8.3 table-UX audit: table-layout:fixed + <colgroup> (see
        // Sales.tsx for the full rationale) instead of the old
        // min-w-[1000px]+overflow-x-auto pattern, which could scroll
        // horizontally on this app's smallest supported window.
        // 1.9.1: this table used to whole-row-navigate to Order Detail, with
        // the Order code and Event name cells also individually linking
        // there - marko asked to remove every "this reference jumps me to a
        // different section" link across Orders/Tickets/Sales, and singled
        // out Tickets by name, so this list became a plain read-only
        // overview: no row click, no cell links. Open a specific order from
        // the Orders page instead.
        // 1.9.2 (section 1) carved out Inventory as an exception to the
        // 1.9.1 "no cross-section navigation" rule; 1.9.3 extended the same
        // Order->Order Detail / Event->Event Detail links to Tickets itself
        // (see allowCrossLinks' doc comment above) - marko wanted them back
        // here specifically, not the old blanket removal. Sales stays
        // link-free either way.
        //
        // 1.9.5: marko pointed out Order should reliably behave like Sale
        // code on Sales or Order code on Orders - your own record's link,
        // not something that depends on a toggle. Order Detail IS this
        // table's own "detail page" (it's literally what a Tickets/
        // Inventory row groups into), so the Order cell below is now an
        // unconditional Link regardless of allowCrossLinks - same pattern
        // as every other list's own-record column. Event stays gated by
        // allowCrossLinks below it: jumping to Event Detail is a genuine
        // cross-section link (a different sidebar item entirely), not this
        // table's own record, so it keeps the toggle Sales still opts out
        // of. In practice allowCrossLinks is true for both current callers
        // (Tickets and Inventory), so this doesn't change what's visible
        // today - it just makes Order's own link independent of that flag
        // rather than accidentally riding along with it.
        // marko clarified in 1.9.6 that this alone wasn't the full ask -
        // the link working reliably was right, but landing on a page that
        // still reads as "Order" felt like being thrown elsewhere either
        // way. See OrderDetail.tsx's detailLabel for the rest of that fix.
        //
        // 1.9.3: Supplier column removed from this table - marko pointed
        // out it's effectively always empty at this order-grouped level of
        // detail and doesn't belong here (same treatment Orders.tsx's list
        // got in 1.9.2). 1.9.4 went further: the Supplier filter that used
        // to sit above this table and the Supplier field on Order Detail's
        // Edit Order form are both gone too - neither Tickets/Inventory nor
        // Order Detail can view or set an order's supplier anymore. CSV
        // import can still set supplier_id (it resolves/creates a supplier
        // by name from its own "supplier" column - see csv_import.rs), and
        // CSV export still includes it. supplier_id itself and the data
        // model are otherwise fully untouched - see the 1.9.4 report.
        // 2.0.37: switched from a fixed-px colgroup (one absorbing Event
        // column, constant widths regardless of window size) to the same
        // pure-percentage, two-mode model Sales.tsx/Events.tsx use -
        // max-w-[1400px] is gone from this wrapper for the same reason it
        // left theirs: a pure-percentage table has no single column that
        // runs away on a wide window, so there's nothing left to cap. Below
        // the shared useNarrowTables() breakpoint (1690px window, same for
        // every table in the app), Purchase date hides (Order's own
        // truncating title-tooltip pattern, pre-existing since 2.0.33/
        // 2.0.34, is untouched) and everything else grows a little and
        // switches to the smaller .th-c-narrow/.td-c-narrow. Verified
        // (Playwright, real Intl.NumberFormat/date data across en-US/
        // sk-SK/de-DE, not just header text) to fit without scrolling or
        // wrapping all the way down to 1080px, this app's enforced minimum
        // window width - see PROTECTED-AREAS-NOTES.md, 2.0.37 section.
        // 2.0.38: Order's own code column was STILL under-measured (sized
        // against the header, not real "ORD-1234567"-shaped codes - see
        // PROTECTED-AREAS-NOTES.md's 2.0.38 section for the full root
        // cause, shared with Orders.tsx/Sales.tsx). Recomputed every
        // column against real rendered content, added the new Seats column
        // (marko's request - every seat across the whole order, hides below
        // the breakpoint alongside Purchase date), and switched Purchase
        // date to formatDateNumeric ("11.09.2026") in place of the old
        // locale-dependent formatDate call. Shared breakpoint moved to
        // 1649px (was 1690px) - see useNarrowTables.ts.
        <div className="overflow-x-auto rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-sm">
          <table className="w-full table-fixed border-collapse">
            {isNarrow ? (
              <colgroup>
                <col className="w-[10.488%]" />
                <col className="w-[47.439%]" />
                <col className="w-[5.976%]" />
                <col className="w-[9.512%]" />
                <col className="w-[5.244%]" />
                <col className="w-[11.22%]" />
                <col className="w-[10.122%]" />
              </colgroup>
            ) : (
              <colgroup>
                <col className="w-[8.133%]" />
                <col className="w-[43.564%]" />
                <col className="w-[9.335%]" />
                <col className="w-[9.194%]" />
                <col className="w-[4.314%]" />
                <col className="w-[6.506%]" />
                <col className="w-[3.819%]" />
                <col className="w-[8.699%]" />
                <col className="w-[6.436%]" />
              </colgroup>
            )}
            <thead className="border-b border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-800/60">
              <tr>
                <th className={isNarrow ? "th-c-narrow" : "th-c"}>Order</th>
                <th className={isNarrow ? "th-c-narrow" : "th-c"}>Event</th>
                {!isNarrow && <th className="th-c">Purchase date</th>}
                {!isNarrow && <th className="th-c">Seats</th>}
                <th className={`${isNarrow ? "th-c-narrow" : "th-c"} text-right`}>Total</th>
                <th className={`${isNarrow ? "th-c-narrow" : "th-c"} text-right`}>Available</th>
                <th className={`${isNarrow ? "th-c-narrow" : "th-c"} text-right`}>Sold</th>
                <th className={`${isNarrow ? "th-c-narrow" : "th-c"} text-right`}>Total cost</th>
                <th className={isNarrow ? "th-c-narrow" : "th-c"}>Status</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
              {visibleOrders.map((o) => {
                const inv = inventoryStatus(o);
                return (
                  <tr key={o.id} className="hover:bg-slate-50 dark:hover:bg-slate-800/60">
                    <td className={`${isNarrow ? "td-c-narrow" : "td-c"} truncate font-medium text-slate-900 dark:text-slate-100`} title={o.code}>
                      <Link to={`/orders/${o.id}`} className="hover:underline">
                        {o.code}
                      </Link>
                    </td>
                    <td className={`${isNarrow ? "td-c-narrow" : "td-c"} truncate`} title={o.eventName}>
                      {allowCrossLinks ? (
                        <Link to={`/events/${o.eventId}`} className="hover:underline">
                          {o.eventName}
                        </Link>
                      ) : (
                        o.eventName
                      )}
                    </td>
                    {!isNarrow && <td className="td-c whitespace-nowrap">{formatDateNumeric(o.purchaseDate)}</td>}
                    {!isNarrow && (
                      <td className="td-c truncate" title={formatSeatsSummary(o.seats)}>
                        {formatSeatsSummary(o.seats)}
                      </td>
                    )}
                    <td className={`${isNarrow ? "td-c-narrow" : "td-c"} text-right tabular-nums whitespace-nowrap`}>{o.quantity}</td>
                    <td className={`${isNarrow ? "td-c-narrow" : "td-c"} text-right tabular-nums whitespace-nowrap`}>{o.availableCount + o.listedCount}</td>
                    <td className={`${isNarrow ? "td-c-narrow" : "td-c"} text-right tabular-nums whitespace-nowrap`}>{o.soldCount}</td>
                    <td className={`${isNarrow ? "td-c-narrow" : "td-c"} text-right tabular-nums whitespace-nowrap`}>{formatMoney(o.totalCostCents, o.currency)}</td>
                    <td className={isNarrow ? "td-c-narrow" : "td-c"}>
                      <Badge tone={inv.key}>{inv.label}</Badge>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

// 2.0.19 (marko's own request): Resale status/Delivery status were plain
// free-text fields until now - fixed, closed lists ("na výber tieto
// možnosti", not growable like Ticket Type/Site Listed), matching exactly
// the Orders & Sales sheet's own Status/Delivery status dropdowns
// (commands/orders_sheet_sync.rs's STATUS_OPTIONS/DELIVERY_STATUS_OPTIONS).
const RESALE_STATUS_OPTIONS = ["Listed", "Unlisted", "Sold"];
const DELIVERY_STATUS_OPTIONS = ["Delivered", "Not delivered"];

export function TicketEditModal({
  open,
  ticket,
  onClose,
  onSaved,
}: {
  open: boolean;
  ticket: Ticket | null;
  onClose: () => void;
  onSaved: () => void;
}) {
  const toast = useToast();
  const [section, setSection] = useState("");
  const [rowLabel, setRowLabel] = useState("");
  const [seat, setSeat] = useState("");
  const [ticketType, setTicketType] = useState("");
  const [listingPrice, setListingPrice] = useState("");
  const [status, setStatus] = useState<TicketStatus>("available");
  const [resaleStatus, setResaleStatus] = useState("");
  const [deliveryStatus, setDeliveryStatus] = useState("");
  const [notes, setNotes] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!ticket) return;
    setSection(ticket.section ?? "");
    setRowLabel(ticket.rowLabel ?? "");
    setSeat(ticket.seat ?? "");
    setTicketType(ticket.ticketType ?? "");
    setListingPrice(ticket.listingPriceCents != null ? (ticket.listingPriceCents / 100).toFixed(2) : "");
    setStatus(ticket.status);
    setResaleStatus(ticket.resaleStatus ?? "");
    setDeliveryStatus(ticket.deliveryStatus ?? "");
    setNotes(ticket.notes ?? "");
    setError(null);
  }, [ticket]);

  if (!ticket) return null;
  const locked = ticket.status === "sold";

  const submit = async () => {
    setError(null);
    let listingCents: number | null = null;
    if (listingPrice.trim() !== "") {
      const s = listingPrice.trim().replace(",", ".");
      if (!/^\d+(\.\d{1,2})?$/.test(s)) {
        setError("Listing price is not a valid amount");
        return;
      }
      listingCents = Math.round(parseFloat(s) * 100);
    }
    const input: TicketUpdateInput = {
      section: section || null,
      rowLabel: rowLabel || null,
      seat: seat || null,
      ticketType: ticketType || null,
      listingPriceCents: listingCents,
      status: locked ? undefined : status,
      resaleStatus: resaleStatus || null,
      deliveryStatus: deliveryStatus || null,
      notes: notes || null,
    };
    setSaving(true);
    try {
      await api.updateTicket(ticket.id, input);
      toast.success("Ticket updated");
      onSaved();
    } catch (e) {
      setError(errMsg(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal open={open} onClose={onClose} title={`Edit ${ticket.code}`}>
      <div className="grid grid-cols-2 gap-4">
        <Field label="Section">
          <Input value={section} onChange={(e) => setSection(e.target.value)} />
        </Field>
        <Field label="Ticket type">
          <Input value={ticketType} onChange={(e) => setTicketType(e.target.value)} />
        </Field>
        <Field label="Row">
          <Input value={rowLabel} onChange={(e) => setRowLabel(e.target.value)} />
        </Field>
        <Field label="Seat">
          <Input value={seat} onChange={(e) => setSeat(e.target.value)} />
        </Field>
        <Field label={`Listing price (${ticket.currency})`}>
          <Input inputMode="decimal" placeholder="0.00" value={listingPrice} onChange={(e) => setListingPrice(e.target.value)} />
        </Field>
        <Field label="Status">
          {locked ? (
            <div>
              <div className="input flex items-center bg-slate-50 dark:bg-slate-800/60 text-slate-500 dark:text-slate-400">Sold</div>
              <p className="mt-1 text-xs text-slate-400 dark:text-slate-500">Refund or delete the sale on the Sales screen to make this available again.</p>
            </div>
          ) : (
            <Select value={status} onChange={(e) => setStatus(e.target.value as TicketStatus)}>
              <option value="available">Available</option>
              <option value="listed">Listed</option>
              <option value="cancelled">Cancelled</option>
            </Select>
          )}
        </Field>
        <Field label="Resale status" hint="From a Sales sync, or set by hand - separate from Status above.">
          <Select value={resaleStatus} onChange={(e) => setResaleStatus(e.target.value)}>
            <option value="">Not specified</option>
            {/* A value from before 2.0.19 (free text) or synced from an
                unexpected sheet cell stays visible instead of looking
                blank/cleared - same fallback pattern Orders.tsx's Ticket
                type field already uses. */}
            {resaleStatus && !RESALE_STATUS_OPTIONS.includes(resaleStatus) && <option value={resaleStatus}>{resaleStatus}</option>}
            {RESALE_STATUS_OPTIONS.map((s) => (
              <option key={s} value={s}>
                {s}
              </option>
            ))}
          </Select>
        </Field>
        <Field label="Delivery status">
          <Select value={deliveryStatus} onChange={(e) => setDeliveryStatus(e.target.value)}>
            <option value="">Not specified</option>
            {deliveryStatus && !DELIVERY_STATUS_OPTIONS.includes(deliveryStatus) && (
              <option value={deliveryStatus}>{deliveryStatus}</option>
            )}
            {DELIVERY_STATUS_OPTIONS.map((s) => (
              <option key={s} value={s}>
                {s}
              </option>
            ))}
          </Select>
        </Field>
        <div className="col-span-2">
          <Field label="Notes">
            <Textarea rows={2} value={notes} onChange={(e) => setNotes(e.target.value)} />
          </Field>
        </div>
      </div>
      <div className="mt-3 grid grid-cols-3 gap-3 rounded-lg bg-slate-50 dark:bg-slate-800/60 px-4 py-3 text-xs text-slate-500 dark:text-slate-400">
        <div>Purchase cost: {formatMoney(ticket.purchaseCostCents, ticket.currency)}</div>
        <div>Fees: {formatMoney(ticket.purchaseFeesCents, ticket.currency)}</div>
        <div>Other: {formatMoney(ticket.otherCostsCents, ticket.currency)}</div>
      </div>
      {error && <p className="mt-3 text-sm text-red-600 dark:text-red-400">{error}</p>}
      <ModalFooter>
        <Button variant="secondary" onClick={onClose} disabled={saving}>
          Cancel
        </Button>
        <Button variant="primary" onClick={submit} disabled={saving}>
          {saving ? "Saving..." : "Save changes"}
        </Button>
      </ModalFooter>
    </Modal>
  );
}

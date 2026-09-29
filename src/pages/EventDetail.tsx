import { useCallback, useEffect, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { api, errMsg } from "../lib/api";
import type { EventWithStats, FinanceEntry, OrderRecord, SaleGroup, Ticket } from "../lib/types";
import { formatDate, formatMoney, formatMoneyOrMixed, formatPercentOrMixed, formatSeatLocation } from "../lib/format";
import { Badge, Button, Card, ConfirmDialog, EmptyState, LoadingBlock, StatCard, TabSwitcher } from "../components/ui";
import { FinanceCategoryBadge } from "../components/FinanceCategoryBadge";
import { IconArrowLeft, IconPencil, IconPlus, IconTrash } from "../components/icons";
import { useToast } from "../lib/toast";
import { EventFormModal } from "./Events";

// 2.2.4: marko's second follow-up on the Event Workspace. Two independent
// changes bundled into one release:
//
// 1) Tab consolidation - "Overview Inventory spoj do jedneho" (merge these
//    two into one) and "Sales Market Finance spoj do jedneho" (a looser
//    grouping, resolved below) - landing on exactly the 4 tabs marko's own
//    message named at the end: Overview | Listings | Sales | Finance.
//    - Overview absorbed Inventory: the Orders/Tickets tables that used to
//      have their own tab are now appended below Overview's own stat cards,
//      completely unchanged otherwise.
//    - Sales absorbed Market: "Market vs. mine" + "Potential Profit" (the
//      former Market tab's entire content) now live below the Sales table,
//      completely unchanged otherwise.
//    - Finance was named as its own surviving tab in marko's own final list
//      that release ("...sales a finance") and stayed untouched.
//    (2.2.5 note: the paragraph above describes 2.2.4's own reasoning for
//    keeping Finance separate - superseded by 2.2.5 below, which merges it
//    into Sales after all. Left here as real history, not rewritten.)
//
// 2) Listings rebuilt into a real system. REMOVED FROM THE UI in 2.64.0 -
//    the table and commands remain, see this file's WorkspaceTab note. Doc
//    comment and commands/ticket_listings.rs (Rust) for the full design.
//    Replaces 2.2.3's read-only view of Ticket.listingPriceCents/status
//    (which explicitly could not show marketplace/URL/last-checked, because
//    none of that data existed anywhere) with real per-marketplace listing
//    rows - one ticket can now have several at once.
//
// 2.2.5: marko's third follow-up on this same page, two more independent
// pieces:
//
// 1) Further tab consolidation - "Sales Market Finance spoj do jedneho" (the
//    2.2.4 entry above) merged Market into Sales but kept Finance separate,
//    since marko's own 2.2.4 message explicitly listed "sales" AND
//    "finance" as two of the 4 surviving tabs. This round he asked again,
//    unambiguously this time - "sales a finance daj dokopy" (put sales and
//    finance together) - with no companion list keeping them apart. Down to
//    3 tabs: Overview | Listings | Sales. JUDGMENT CALL (flagged here and in
//    REDESIGN-2.2.5-REPORT.md): marko named "Sales" first in that sentence,
//    so - same "first-named tab survives, its content absorbs the rest"
//    convention already used for Overview/Inventory and Sales/Market above -
//    Sales is the surviving name; Finance's entries table now renders at the
//    bottom of `SalesTab`, below the Market section 2.2.4 already put there.
//    The standalone Finance SECTION of the app (`/finance`, its own 4-tab
//    page) is completely unrelated and untouched - this is only about this
//    one event-scoped tab.
//
// 2) Listings made genuinely manageable at volume - filters (status/
//    marketplace), search, multi-select with bulk status/price/delete, and
//    an order-browse ticket picker for Add Listing (mirroring New Sale's own
//    "pick an order, then pick tickets from it" flow, replacing the old flat
//    "every ticket in the event in one dropdown" picker marko found opaque).
//    The UI this describes was removed in 2.64.0; commands/ticket_listings.rs
//    still holds the design.
//
// 2.2.6 added an "Inventory Intelligence" block here - KPIs, aging,
// attention and breakdowns, with clicking a row filtering the Tickets table
// below. 2.35.1 removed it at marko's request, along with that filter, which
// this block was the only thing that could switch on. The backend it called,
// `commands/inventory_intelligence.rs` / `get_inventory_intelligence`, is
// still there and still registered - nothing was deleted on that side.
/* 2.64.0: marko asked for Listings to go. The tab and its whole UI are
   gone from this file. The `ticket_listings` TABLE, its Rust commands and
   the "Listing price" column on Order Detail all stay - same call as the
   Sheets removal in 2.61.0: he asked for the section, not for the data. */
type WorkspaceTab = "overview" | "sales";

const WORKSPACE_TABS: { key: WorkspaceTab; label: string }[] = [
  { key: "overview", label: "Overview" },
  { key: "sales", label: "Sales" },
];

export default function EventDetail() {
  const { id } = useParams();
  const eventId = Number(id);
  const navigate = useNavigate();
  const toast = useToast();

  const [tab, setTab] = useState<WorkspaceTab>("overview");
  const [event, setEvent] = useState<EventWithStats | null>(null);
  const [orders, setOrders] = useState<OrderRecord[] | null>(null);
  const [tickets, setTickets] = useState<Ticket[] | null>(null);
  const [editOpen, setEditOpen] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [deleting, setDeleting] = useState(false);

  const load = useCallback(() => {
    api.getEvent(eventId).then(setEvent).catch((e) => toast.error(errMsg(e)));
    api.listOrders({ eventId }).then(setOrders).catch((e) => toast.error(errMsg(e)));
    api
      .listTickets({ eventId, sortBy: "created", sortDir: "desc" })
      .then(setTickets)
      .catch((e) => toast.error(errMsg(e)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [eventId]);

  useEffect(() => {
    load();
  }, [load]);

  if (!event) return <LoadingBlock />;

  return (
    <div>
      <Link to="/events" className="mb-3 inline-flex items-center gap-1 text-sm text-slate-500 dark:text-slate-400 hover:text-slate-800 dark:hover:text-slate-200">
        <IconArrowLeft className="h-4 w-4" /> Back to events
      </Link>

      <div className="mb-5 flex flex-wrap items-start justify-between gap-3">
        <div>
          <div className="flex items-center gap-2">
            <h1 className="text-xl font-semibold text-slate-900 dark:text-slate-100">{event.name}</h1>
            <Badge tone={event.status}>{event.status}</Badge>
          </div>
          <p className="mt-1 text-sm text-slate-500 dark:text-slate-400">
            {[event.venue, event.city, event.country].filter(Boolean).join(" &middot; ")}
            {event.eventDate ? ` &middot; ${formatDate(event.eventDate)}` : ""}
          </p>
        </div>
        <div className="flex gap-2">
          <Button variant="secondary" onClick={() => setEditOpen(true)}>
            <IconPencil className="h-4 w-4" /> Edit
          </Button>
          <Button variant="danger" onClick={() => setConfirmDelete(true)}>
            <IconTrash className="h-4 w-4" /> Delete
          </Button>
        </div>
      </div>

      <TabSwitcher tabs={WORKSPACE_TABS} active={tab} onChange={setTab} className="mb-4" />

      {tab === "overview" && <OverviewTab event={event} orders={orders} tickets={tickets} navigate={navigate} onSwitchTab={setTab} />}
      {tab === "sales" && <SalesTab event={event} tickets={tickets} orders={orders} navigate={navigate} />}

      <EventFormModal
        open={editOpen}
        initial={event}
        onClose={() => setEditOpen(false)}
        onSaved={() => {
          setEditOpen(false);
          load();
        }}
      />

      <ConfirmDialog
        open={confirmDelete}
        title="Delete this event?"
        message="This can only be done if the event has no orders or tickets linked to it. This cannot be undone."
        confirmLabel="Delete event"
        danger
        busy={deleting}
        onCancel={() => setConfirmDelete(false)}
        onConfirm={async () => {
          setDeleting(true);
          try {
            await api.deleteEvent(eventId);
            toast.success("Event deleted");
            navigate("/events");
          } catch (e) {
            toast.error(errMsg(e));
            setConfirmDelete(false);
          } finally {
            setDeleting(false);
          }
        }}
      />
    </div>
  );
}

// ---------------------------------------------------------------------------
// Overview - marko's own stat list (tickets, sold, available, total cost,
// revenue, profit, margin/ROI) unchanged from 2.2.2/2.2.3, plus (2.2.4) the
// Orders + Tickets tables that used to be their own "Inventory" tab -
// "Overview Inventory spoj do jedneho" (merge these two into one): the
// second-named tab (Inventory) is removed, its content moved into the
// first-named one that remains (Overview). Both halves are otherwise
// completely unchanged, just relocated into one function.
// ---------------------------------------------------------------------------
function OverviewTab({
  event,
  orders,
  tickets,
  navigate,
  onSwitchTab,
}: {
  event: EventWithStats;
  orders: OrderRecord[] | null;
  tickets: Ticket[] | null;
  navigate: ReturnType<typeof useNavigate>;
  onSwitchTab: (tab: WorkspaceTab) => void;
}) {
  const s = event.stats;

  // 2.35.1: the ticket-highlight filter went with Inventory Intelligence -
  // that block held every caller of it, so with the block gone the filter
  // could never be switched on and the "Showing: ..." banner could never
  // appear. The Tickets table below simply lists every ticket again.

  return (
    <div>
      <div className="summary-bar">
        <StatCard label="Tickets" value={String(s.purchasedTickets)} />
        <StatCard label="Sold" value={String(s.soldTickets)} sub={`${s.cancelledTickets} cancelled`} />
        <StatCard label="Available" value={String(s.availableTickets)} sub={`${s.listedTickets} listed`} />
        <StatCard label="Total cost" value={formatMoneyOrMixed(s.totalCostCents, s.currency)} />
        <StatCard label="Revenue" value={formatMoneyOrMixed(s.revenueCents, s.currency)} />
      </div>
      {s.currency === null && (
        <p className="mb-3 text-xs text-amber-700 dark:text-amber-400">
          This event has tickets in more than one currency, so these numbers can&apos;t be combined into one here. Check
          individual orders and sales instead.
        </p>
      )}
      <div className="summary-bar">
        <StatCard
          label="Profit"
          value={formatMoneyOrMixed(s.profitCents, s.currency)}
          tone={s.profitCents > 0 ? "positive" : s.profitCents < 0 ? "negative" : "default"}
        />
        {/* 2.39.0: Margin removed app-wide at marko's request. */}
        <StatCard label="ROI" value={formatPercentOrMixed(s.roi, s.currency)} />
      </div>

      {event.notes && (
        <Card className="mb-6 p-4">
          <p className="mb-1 section-title">Notes</p>
          <p className="whitespace-pre-wrap text-sm text-slate-700 dark:text-slate-300">{event.notes}</p>
        </Card>
      )}

      {/* 2.35.1: "Inventory Intelligence" removed at marko's request. The
          Rust command (`get_inventory_intelligence`) and its api.ts method are
          untouched and still registered - only this screen's block is gone. */}
      {/* 2.41.0: the Orders table that used to sit here is gone - marko:
          "v events ked to prekliknes tak tam mas aj orders a tickets, mali
          by tam byt len tickets". An order is how stock was BOUGHT; when
          you open an event you want to know what you HAVE on it, and every
          ticket already names its order. `orders` is still loaded - the
          Listings tab below takes it as a prop - and /orders still lists
          them all. Only this second table is gone. The "New order for this
          event" action moved onto the Tickets heading below, so the way in
          is where you are already looking. */}
      <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
        <h2 className="text-sm font-semibold text-slate-800 dark:text-slate-200">Tickets ({tickets?.length ?? 0})</h2>
        <Button variant="secondary" onClick={() => navigate("/orders", { state: { presetEventId: event.id } })}>
          <IconPlus className="h-4 w-4" /> New order for this event
        </Button>
      </div>
      {tickets === null ? (
        <LoadingBlock />
      ) : tickets.length === 0 ? (
        <EmptyState title="No tickets for this event yet" />
      ) : (
        // 2.2.3: max-w-[1400px] removed - see the Orders table above.
        <div className="table-shell table-shell-compact">
          <table className="w-full min-w-[700px] border-collapse">
            <thead>
              <tr>
                <th className="th">Ticket</th>
                <th className="th">Seat</th>
                <th className="th text-right">Cost</th>
                <th className="th text-right">Listing price</th>
                <th className="th">Status</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
              {tickets.map((t) => (
                <tr key={t.id}>
                  <td className="td">
                    <Link to={`/orders?code=${encodeURIComponent(t.code)}`} className="font-medium text-slate-900 dark:text-slate-100 hover:text-brand-700 dark:hover:text-brand-400">
                      {t.code}
                    </Link>
                  </td>
                  <td className="td text-slate-500 dark:text-slate-400">
                    {formatSeatLocation(t.section, t.rowLabel, t.seat)}
                  </td>
                  <td className="td text-right tabular-nums">{formatMoney(t.totalCostCents, t.currency)}</td>
                  <td className="td text-right tabular-nums">
                    {t.listingPriceCents != null ? formatMoney(t.listingPriceCents, t.currency) : "-"}
                  </td>
                  <td className="td">
                    <Badge tone={t.status}>{t.status}</Badge>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Sales - list_sale_groups({ eventId }) (Sales.tsx's own Event filter,
// reused), unchanged from 2.2.2, plus (2.2.4) the former Market tab's
// entire content appended below - "Market vs. mine" (get_price_checker_
// summary) and "Potential Profit" (this page's own unsold-inventory
// estimate) - and (2.2.5) the former Finance tab's entire content appended
// after that - see this file's own top-of-file doc comment for why both
// ended up here. Every section loads and renders independently (one slow
// fetch never blocks another), same "each tab fetches its own data"
// convention as before.
// ---------------------------------------------------------------------------
function SalesTab({
  event,
  tickets,
  orders,
  navigate,
}: {
  event: EventWithStats;
  tickets: Ticket[] | null;
  orders: OrderRecord[] | null;
  navigate: ReturnType<typeof useNavigate>;
}) {
  const toast = useToast();
  const [groups, setGroups] = useState<SaleGroup[] | null>(null);
  // 2.2.5: former FinanceTab state/effect, moved here verbatim - see that
  // function's own removed doc comment (still in CHANGELOG/git history) for
  // why this fetches per-order rather than one event-scoped query.
  const [financeEntries, setFinanceEntries] = useState<FinanceEntry[] | null>(null);

  useEffect(() => {
    setGroups(null);
    api
      .listSaleGroups({ eventId: event.id })
      .then(setGroups)
      .catch((e) => toast.error(errMsg(e)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [event.id]);

  useEffect(() => {
    if (orders === null) return;
    if (orders.length === 0) {
      setFinanceEntries([]);
      return;
    }
    Promise.all(orders.map((o) => api.listFinanceEntriesForOrder(o.id)))
      .then((lists) => setFinanceEntries(lists.flat().sort((a, b) => (a.entryDate < b.entryDate ? 1 : a.entryDate > b.entryDate ? -1 : b.id - a.id))))
      .catch((e) => toast.error(errMsg(e)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [orders]);

  // 1.8.3 (section 14) / 1.9.10: unchanged from this page's previous
  // single-tab version - see git history there for the original reasoning.
  const unsoldTickets = (tickets ?? []).filter((t) => t.status === "available" || t.status === "listed");
  const potentialInventoryCostCents = unsoldTickets.reduce((sum, t) => sum + t.totalCostCents, 0);
  const potentialListingValueCents = unsoldTickets.reduce((sum, t) => sum + (t.listingPriceCents ?? 0), 0);
  const potentialProfitCents = potentialListingValueCents - potentialInventoryCostCents;
  const unsoldCurrencies = Array.from(new Set(unsoldTickets.map((t) => t.currency)));
  const potentialCurrency = unsoldCurrencies.length <= 1 ? (unsoldCurrencies[0] ?? event.stats.currency) : null;
  const missingListingPriceCount = unsoldTickets.filter((t) => t.listingPriceCents == null).length;

  return (
    <div>
      <div className="mb-3 flex items-center justify-between">
        <h2 className="text-sm font-semibold text-slate-800 dark:text-slate-200">Sales ({groups?.length ?? 0})</h2>
        <Link to="/sales" className="text-sm font-medium text-brand-600 dark:text-brand-400 hover:underline">
          Open in Sales &rarr;
        </Link>
      </div>
      {groups === null ? (
        <LoadingBlock />
      ) : groups.length === 0 ? (
        <EmptyState title="No sales for this event yet" />
      ) : (
        // 2.2.3: no max-w cap - see Overview's Orders table's own comment.
        <div className="table-shell table-shell-compact mb-8">
          <table className="w-full min-w-[700px] border-collapse">
            <thead>
              <tr>
                <th className="th">Sale</th>
                <th className="th">Date</th>
                <th className="th">Platform</th>
                <th className="th text-right">Qty</th>
                <th className="th text-right">Revenue</th>
                <th className="th text-right">Profit</th>
                <th className="th">Payment</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
              {groups.map((g) => (
                <tr key={g.id}>
                  <td className="td">
                    <Link to={`/sales/${g.id}`} className="font-medium text-slate-900 dark:text-slate-100 hover:text-brand-700 dark:hover:text-brand-400">
                      {g.code}
                    </Link>
                  </td>
                  <td className="td">{formatDate(g.saleDate)}</td>
                  <td className="td text-slate-500 dark:text-slate-400">{g.platformName ?? "-"}</td>
                  <td className="td text-right tabular-nums">{g.ticketCount}</td>
                  <td className="td text-right tabular-nums">{formatMoneyOrMixed(g.revenueCents, g.currency)}</td>
                  <td
                    className={`td text-right tabular-nums font-medium ${
                      g.currency === null ? "" : g.profitCents > 0 ? "text-emerald-600 dark:text-emerald-400" : g.profitCents < 0 ? "text-red-600 dark:text-red-400" : ""
                    }`}
                  >
                    {formatMoneyOrMixed(g.profitCents, g.currency)}
                  </td>
                  <td className="td">
                    {g.paymentStatus ? <Badge tone={g.paymentStatus}>{g.paymentStatus}</Badge> : <Badge tone="mixed">Mixed</Badge>}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {/* 2.33.0: the "Market vs. mine" card and the Price Checker link that
          sat here are gone with Price Checker itself. Potential Profit below
          is this page's own calculation and stays. */}
      <div className="rounded-xl border border-slate-200 dark:border-slate-800 bg-slate-50/60 dark:bg-slate-800/30 p-4">
        <p className="mb-1 section-title">Potential Profit</p>
        <p className="mb-3 text-xs text-slate-500 dark:text-slate-400">
          This event&apos;s unsold stock (available + listed), not yet sold. This is an estimate, not realized profit.
        </p>
        <div className="summary-bar">
          <StatCard label="Inventory cost" value={formatMoneyOrMixed(potentialInventoryCostCents, potentialCurrency)} sub="What unsold tickets cost you" />
          <StatCard label="Listing value" value={formatMoneyOrMixed(potentialListingValueCents, potentialCurrency)} sub="Unsold tickets that have a listing price" />
          <StatCard label="Potential profit" value={formatMoneyOrMixed(potentialProfitCents, potentialCurrency)} sub="Listing value minus inventory cost" />
        </div>
        {missingListingPriceCount > 0 && (
          <p className="mt-3 text-xs text-slate-500 dark:text-slate-400">
            {missingListingPriceCount} unsold ticket{missingListingPriceCount === 1 ? "" : "s"} still{" "}
            {missingListingPriceCount === 1 ? "has" : "have"} no listing price, so potential profit understates what full inventory
            could be worth once priced.
          </p>
        )}
      </div>

      {/* 2.2.5: former FinanceTab, folded in below Market - "sales a finance
          daj dokopy" (see this file's own top-of-file doc comment for the
          Sales-survives judgment call). Content/logic is otherwise
          byte-for-byte what FinanceTab already rendered - only the
          orders/loading source changed from a dedicated prop to this tab's
          own already-fetched orders. */}
      <div className="mt-8 mb-3 flex items-center justify-between">
        <h2 className="text-sm font-semibold text-slate-800 dark:text-slate-200">Finance ({financeEntries?.length ?? 0})</h2>
        <Link to="/finance" className="text-sm font-medium text-brand-600 dark:text-brand-400 hover:underline">
          Open in Finance &rarr;
        </Link>
      </div>
      {orders === null || financeEntries === null ? (
        <LoadingBlock />
      ) : orders.length === 0 ? (
        <EmptyState title="No orders for this event yet" description="Record a purchase first, then you can link Finance entries to it." />
      ) : financeEntries.length === 0 ? (
        <EmptyState
          title="Nothing recorded in Finance for this event yet"
          description={`Open one of this event's orders and use "Record in Finance" there.`}
        />
      ) : (
        // 2.2.3: no max-w cap - see Overview's Orders table's own comment.
        <div className="table-shell table-shell-compact">
          <table className="w-full min-w-[600px] border-collapse">
            <thead>
              <tr>
                <th className="th">Date</th>
                <th className="th">Order</th>
                <th className="th">Category</th>
                <th className="th text-right">Amount</th>
                <th className="th">Note</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
              {financeEntries.map((e) => (
                <tr key={e.id}>
                  <td className="td">{formatDate(e.entryDate)}</td>
                  <td className="td">
                    {e.orderId && e.orderCode ? (
                      <Link to={`/orders/${e.orderId}`} className="font-medium text-slate-900 dark:text-slate-100 hover:text-brand-700 dark:hover:text-brand-400">
                        {e.orderCode}
                      </Link>
                    ) : (
                      "-"
                    )}
                  </td>
                  <td className="td">
                    {e.categoryName ? <FinanceCategoryBadge name={e.categoryName} colorSlot={e.categoryColorSlot ?? 0} /> : "-"}
                  </td>
                  <td
                    className={`td text-right tabular-nums font-medium ${
                      e.entryType === "income" ? "text-emerald-600 dark:text-emerald-400" : "text-slate-900 dark:text-slate-100"
                    }`}
                  >
                    {e.entryType === "income" ? "+" : "-"}
                    {formatMoney(e.amountCents, e.currency)}
                  </td>
                  <td className="td max-w-[220px] truncate text-slate-500 dark:text-slate-400" title={e.note ?? undefined}>
                    {e.note ?? "-"}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

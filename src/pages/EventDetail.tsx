import { useCallback, useEffect, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { api, errMsg } from "../lib/api";
import type {
  EventWithStats,
  FinanceEntry,
  Marketplace,
  OrderRecord,
  PriceCheckerSummary,
  SaleGroup,
  Ticket,
  TicketListing,
  TicketListingInput,
} from "../lib/types";
import {
  centsToDecimalString,
  decimalStringToCents,
  formatDate,
  formatDateTime,
  formatMoney,
  formatMoneyOrMixed,
  formatPercent,
  formatPercentOrMixed,
} from "../lib/format";
import {
  Badge,
  Button,
  Card,
  ConfirmDialog,
  EmptyState,
  Field,
  Input,
  LoadingBlock,
  Modal,
  ModalFooter,
  Select,
  Spinner,
  StatCard,
  TabSwitcher,
} from "../components/ui";
import { FinanceCategoryBadge } from "../components/FinanceCategoryBadge";
import { IconArrowLeft, IconLink, IconPencil, IconPlus, IconTrash } from "../components/icons";
import { useToast } from "../lib/toast";
import { EventFormModal } from "./Events";
import { CURRENCIES } from "./Orders";

// 2.2.4: marko's second follow-up on the Event Workspace. Two independent
// changes bundled into one release:
//
// 1) Tab consolidation - "Overview Inventory spoj do jedneho" (merge these
//    two into one) and "Sales Market Finance spoj do jedneho" (a looser
//    grouping, resolved below) - landing on exactly the 4 tabs marko's own
//    message names at the end: Overview | Listings | Sales | Finance.
//    - Overview absorbed Inventory: the Orders/Tickets tables that used to
//      have their own tab are now appended below Overview's own stat cards,
//      completely unchanged otherwise.
//    - Sales absorbed Market: "Market vs. mine" + "Potential Profit" (the
//      former Market tab's entire content) now live below the Sales table,
//      completely unchanged otherwise.
//    - Finance was named as its own surviving tab in marko's own final list
//      ("...sales a finance") and is untouched - not folded into anything.
//    JUDGMENT CALL (flagged here and in REDESIGN-2.2.4-REPORT.md): marko's
//    own sentence grouped "Sales Market Finance" together, but his
//    immediately-following list of the 4 tabs that should remain explicitly
//    keeps "sales" AND "finance" as two separate names - so Market (the one
//    name that disappears from that list) was folded into Sales, not
//    Finance. Market's content (what could I get if I sold now) reads as a
//    Sales concern more than a Finance ledger one, which is the other
//    reason this direction was chosen. Easy to move if marko meant it the
//    other way.
//
// 2) Listings rebuilt into a real system - see ListingsTab's own doc
//    comment and commands/ticket_listings.rs (Rust) for the full design.
//    Replaces 2.2.3's read-only view of Ticket.listingPriceCents/status
//    (which explicitly could not show marketplace/URL/last-checked, because
//    none of that data existed anywhere) with real per-marketplace listing
//    rows - one ticket can now have several at once.
type WorkspaceTab = "overview" | "listings" | "sales" | "finance";

const WORKSPACE_TABS: { key: WorkspaceTab; label: string }[] = [
  { key: "overview", label: "Overview" },
  { key: "listings", label: "Listings" },
  { key: "sales", label: "Sales" },
  { key: "finance", label: "Finance" },
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

      <TabSwitcher tabs={WORKSPACE_TABS} active={tab} onChange={setTab} />

      {tab === "overview" && <OverviewTab event={event} orders={orders} tickets={tickets} navigate={navigate} />}
      {tab === "listings" && <ListingsTab eventId={eventId} tickets={tickets} />}
      {tab === "sales" && <SalesTab event={event} tickets={tickets} navigate={navigate} />}
      {tab === "finance" && <FinanceTab orders={orders} />}

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
}: {
  event: EventWithStats;
  orders: OrderRecord[] | null;
  tickets: Ticket[] | null;
  navigate: ReturnType<typeof useNavigate>;
}) {
  const s = event.stats;
  return (
    <div>
      <div className="mb-3 grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-5">
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
      <div className="mb-6 grid grid-cols-2 gap-3 sm:grid-cols-3">
        <StatCard
          label="Profit"
          value={formatMoneyOrMixed(s.profitCents, s.currency)}
          tone={s.profitCents > 0 ? "positive" : s.profitCents < 0 ? "negative" : "default"}
        />
        <StatCard label="Margin" value={formatPercentOrMixed(s.margin, s.currency)} />
        <StatCard label="ROI" value={formatPercentOrMixed(s.roi, s.currency)} />
      </div>

      {event.notes && (
        <Card className="mb-6 p-4">
          <p className="mb-1 text-xs font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">Notes</p>
          <p className="whitespace-pre-wrap text-sm text-slate-700 dark:text-slate-300">{event.notes}</p>
        </Card>
      )}

      <div className="mb-3 flex items-center justify-between">
        <h2 className="text-sm font-semibold text-slate-800 dark:text-slate-200">Orders ({orders?.length ?? 0})</h2>
        <Button variant="secondary" onClick={() => navigate("/orders", { state: { presetEventId: event.id } })}>
          <IconPlus className="h-4 w-4" /> New order for this event
        </Button>
      </div>
      {orders === null ? (
        <LoadingBlock />
      ) : orders.length === 0 ? (
        <EmptyState title="No orders for this event yet" />
      ) : (
        // 2.2.3: max-w-[1400px] removed - marko noticed these tables
        // stopped short of the window edge on a wide screen (the same
        // "visible empty space on both sides" complaint that got the page
        // shell itself de-capped back in 2.0.31 - see Layout.tsx's own
        // comment). No colgroup/percentage-width system here unlike
        // Sales.tsx's own table (2.0.35+) - if a specific column ends up
        // looking oddly stretched on an ultra-wide window, that's the
        // next thing to fix, same iterative path Sales.tsx took.
        <div className="mb-8 overflow-x-auto rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-sm">
          <table className="w-full min-w-[700px] border-collapse">
            <thead className="border-b border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-800/60">
              <tr>
                <th className="th">Order</th>
                <th className="th">Purchase date</th>
                <th className="th text-right">Qty</th>
                <th className="th text-right">Total cost</th>
                <th className="th">Payment</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
              {orders.map((o) => (
                <tr key={o.id} className="hover:bg-slate-50 dark:hover:bg-slate-800/60">
                  <td className="td">
                    <Link to={`/orders/${o.id}`} className="font-medium text-slate-900 dark:text-slate-100 hover:text-brand-700 dark:hover:text-brand-400">
                      {o.code}
                    </Link>
                  </td>
                  <td className="td">{formatDate(o.purchaseDate)}</td>
                  <td className="td text-right tabular-nums">
                    {o.quantity} <span className="text-slate-400 dark:text-slate-500">({o.soldCount} sold)</span>
                  </td>
                  <td className="td text-right tabular-nums">{formatMoney(o.totalCostCents, o.currency)}</td>
                  <td className="td">
                    <Badge tone={o.paymentStatus}>{o.paymentStatus}</Badge>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      <h2 className="mb-3 text-sm font-semibold text-slate-800 dark:text-slate-200">Tickets ({tickets?.length ?? 0})</h2>
      {tickets === null ? (
        <LoadingBlock />
      ) : tickets.length === 0 ? (
        <EmptyState title="No tickets for this event yet" />
      ) : (
        // 2.2.3: max-w-[1400px] removed - see the Orders table above.
        <div className="overflow-x-auto rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-sm">
          <table className="w-full min-w-[700px] border-collapse">
            <thead className="border-b border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-800/60">
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
                <tr key={t.id} className="hover:bg-slate-50 dark:hover:bg-slate-800/60">
                  <td className="td">
                    <Link to={`/tickets?code=${encodeURIComponent(t.code)}`} className="font-medium text-slate-900 dark:text-slate-100 hover:text-brand-700 dark:hover:text-brand-400">
                      {t.code}
                    </Link>
                  </td>
                  <td className="td text-slate-500 dark:text-slate-400">
                    {[t.section, t.rowLabel, t.seat].filter(Boolean).join(" / ") || "-"}
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
// estimate). See this file's own top-of-file doc comment for why Market's
// content landed here rather than in Finance. Both sections load and render
// independently (one slow fetch never blocks the other), same "each tab
// fetches its own data" convention as before.
// ---------------------------------------------------------------------------
function SalesTab({
  event,
  tickets,
  navigate,
}: {
  event: EventWithStats;
  tickets: Ticket[] | null;
  navigate: ReturnType<typeof useNavigate>;
}) {
  const toast = useToast();
  const [groups, setGroups] = useState<SaleGroup[] | null>(null);
  const [summary, setSummary] = useState<PriceCheckerSummary | null>(null);

  useEffect(() => {
    setGroups(null);
    api
      .listSaleGroups({ eventId: event.id })
      .then(setGroups)
      .catch((e) => toast.error(errMsg(e)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [event.id]);

  useEffect(() => {
    api
      .getPriceCheckerSummary(event.id)
      .then(setSummary)
      .catch((e) => toast.error(errMsg(e)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [event.id]);

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
        <div className="mb-8 overflow-x-auto rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-sm">
          <table className="w-full min-w-[700px] border-collapse">
            <thead className="border-b border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-800/60">
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
                <tr key={g.id} className="hover:bg-slate-50 dark:hover:bg-slate-800/60">
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

      <div className="mb-3 flex items-center justify-between">
        <h2 className="text-sm font-semibold text-slate-800 dark:text-slate-200">Market</h2>
        <button
          type="button"
          onClick={() => navigate("/price-checker", { state: { presetEventId: event.id } })}
          className="text-sm font-medium text-brand-600 dark:text-brand-400 hover:underline"
        >
          Open in Price Checker &rarr;
        </button>
      </div>

      {summary && summary.marketLowestPriceCents !== null && (
        <Card className="mb-6 p-4">
          <p className="mb-3 text-xs font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">Market vs. mine</p>
          <div className="grid grid-cols-2 gap-3 sm:grid-cols-5">
            <StatCard label="Market lowest" value={formatMoney(summary.marketLowestPriceCents, summary.myCurrency ?? "EUR")} />
            <StatCard label="Market average" value={formatMoney(summary.marketAveragePriceCents, summary.myCurrency ?? "EUR")} />
            <StatCard
              label="Recommended price"
              value={formatMoney(summary.recommendedPriceCents, summary.myCurrency ?? "EUR")}
              sub="5% below the lowest market price"
            />
            <StatCard
              label="Expected profit"
              value={formatMoney(summary.expectedProfitCents, summary.myCurrency ?? "EUR")}
              tone={
                summary.expectedProfitCents == null ? "default" : summary.expectedProfitCents > 0 ? "positive" : summary.expectedProfitCents < 0 ? "negative" : "default"
              }
            />
            <StatCard label="Expected ROI" value={formatPercent(summary.expectedRoi)} />
          </div>
        </Card>
      )}

      <div className="rounded-xl border border-slate-200 dark:border-slate-800 bg-slate-50/60 dark:bg-slate-800/30 p-4">
        <p className="mb-1 text-xs font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">Potential Profit</p>
        <p className="mb-3 text-xs text-slate-400 dark:text-slate-500">
          This event&apos;s unsold stock (available + listed), not yet sold. This is an estimate, not realized profit.
        </p>
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
          <StatCard label="Inventory cost" value={formatMoneyOrMixed(potentialInventoryCostCents, potentialCurrency)} sub="What unsold tickets cost you" />
          <StatCard label="Listing value" value={formatMoneyOrMixed(potentialListingValueCents, potentialCurrency)} sub="Unsold tickets that have a listing price" />
          <StatCard label="Potential profit" value={formatMoneyOrMixed(potentialProfitCents, potentialCurrency)} sub="Listing value minus inventory cost" />
        </div>
        {missingListingPriceCount > 0 && (
          <p className="mt-3 text-xs text-slate-400 dark:text-slate-500">
            {missingListingPriceCount} unsold ticket{missingListingPriceCount === 1 ? "" : "s"} still{" "}
            {missingListingPriceCount === 1 ? "has" : "have"} no listing price, so potential profit understates what full inventory
            could be worth once priced.
          </p>
        )}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Finance - pulls every Finance entry linked to one of this event's own
// Orders (list_finance_entries_for_order, 2.2.1), merged client-side.
// Completely unchanged in 2.2.4 - marko's own final tab list keeps this as
// its own surviving tab, not folded into anything (see this file's own
// top-of-file doc comment).
// ---------------------------------------------------------------------------
function FinanceTab({ orders }: { orders: OrderRecord[] | null }) {
  const toast = useToast();
  const [entries, setEntries] = useState<FinanceEntry[] | null>(null);

  useEffect(() => {
    if (orders === null) return;
    if (orders.length === 0) {
      setEntries([]);
      return;
    }
    Promise.all(orders.map((o) => api.listFinanceEntriesForOrder(o.id)))
      .then((lists) => setEntries(lists.flat().sort((a, b) => (a.entryDate < b.entryDate ? 1 : a.entryDate > b.entryDate ? -1 : b.id - a.id))))
      .catch((e) => toast.error(errMsg(e)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [orders]);

  if (orders === null || entries === null) return <LoadingBlock />;

  return (
    <div>
      <div className="mb-3 flex items-center justify-between">
        <h2 className="text-sm font-semibold text-slate-800 dark:text-slate-200">Finance ({entries.length})</h2>
        <Link to="/finance" className="text-sm font-medium text-brand-600 dark:text-brand-400 hover:underline">
          Open in Finance &rarr;
        </Link>
      </div>
      {orders.length === 0 ? (
        <EmptyState title="No orders for this event yet" description="Record a purchase first, then you can link Finance entries to it." />
      ) : entries.length === 0 ? (
        <EmptyState
          title="Nothing recorded in Finance for this event yet"
          description={`Open one of this event's orders and use "Record in Finance" there.`}
        />
      ) : (
        // 2.2.3: no max-w cap - see Overview's Orders table's own comment.
        <div className="overflow-x-auto rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-sm">
          <table className="w-full min-w-[600px] border-collapse">
            <thead className="border-b border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-800/60">
              <tr>
                <th className="th">Date</th>
                <th className="th">Order</th>
                <th className="th">Category</th>
                <th className="th text-right">Amount</th>
                <th className="th">Note</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
              {entries.map((e) => (
                <tr key={e.id} className="hover:bg-slate-50 dark:hover:bg-slate-800/60">
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

// ---------------------------------------------------------------------------
// Listings (2.2.4 rebuild) - a REAL multi-marketplace listing system, on top
// of the new `ticket_listings` table (see migrations/022_ticket_listings.sql
// and commands/ticket_listings.rs). Replaces 2.2.3's read-only view of
// Ticket.listingPriceCents/status (which could only show one implied
// "listing" per ticket, and explicitly could not show marketplace/URL/last
// checked because none of that data existed anywhere). Now: one ticket can
// have several real listings, each tied to a real marketplace (the same
// list Price Checker manages), with its own price/status/URL/timestamp.
//
// Deliberately still manual-entry only - no automatic listing creation, no
// marketplace API, no repricing (marko's own explicit "Dôležité" list this
// release). The table below shows EVERY listing regardless of status (not
// just active ones) - marko's own field list explicitly asks for a
// "status" column, which is only meaningful if a listing can be shown in a
// state OTHER than active (sold/removed); the four summary numbers above it
// count active listings only, matching "počet aktívnych listingov".
// ---------------------------------------------------------------------------
function ListingsTab({ eventId, tickets }: { eventId: number; tickets: Ticket[] | null }) {
  const toast = useToast();
  const [listings, setListings] = useState<TicketListing[] | null>(null);
  const [marketplaces, setMarketplaces] = useState<Marketplace[] | null>(null);
  const [formOpen, setFormOpen] = useState(false);
  const [editing, setEditing] = useState<TicketListing | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<TicketListing | null>(null);
  const [deleting, setDeleting] = useState(false);

  const load = useCallback(() => {
    api
      .listTicketListingsForEvent(eventId)
      .then(setListings)
      .catch((e) => toast.error(errMsg(e)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [eventId]);

  useEffect(() => {
    load();
  }, [load]);

  useEffect(() => {
    api
      .listMarketplaces()
      .then(setMarketplaces)
      .catch((e) => toast.error(errMsg(e)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  if (listings === null || tickets === null) return <LoadingBlock />;

  // Summary counts/values are scoped to ACTIVE listings only - "počet
  // aktívnych listingov"/"listed value"/"lowest"/"highest" are all about
  // what's currently for sale, not sold/removed history.
  const active = listings.filter((l) => l.status === "active");
  const activeValueCents = active.reduce((sum, l) => sum + l.priceCents, 0);
  const activePrices = active.map((l) => l.priceCents);
  const lowestCents = activePrices.length > 0 ? Math.min(...activePrices) : null;
  const highestCents = activePrices.length > 0 ? Math.max(...activePrices) : null;
  const activeCurrencies = Array.from(new Set(active.map((l) => l.currency)));
  const activeCurrency = activeCurrencies.length <= 1 ? (activeCurrencies[0] ?? null) : null;

  return (
    <div>
      <div className="mb-3 grid grid-cols-2 gap-3 sm:grid-cols-4">
        <StatCard label="Active listings" value={String(active.length)} />
        <StatCard label="Listed value" value={formatMoneyOrMixed(activeValueCents, activeCurrency)} />
        <StatCard label="Lowest price" value={lowestCents !== null ? formatMoneyOrMixed(lowestCents, activeCurrency) : "-"} />
        <StatCard label="Highest price" value={highestCents !== null ? formatMoneyOrMixed(highestCents, activeCurrency) : "-"} />
      </div>

      <div className="mb-3 flex items-center justify-between">
        <h2 className="text-sm font-semibold text-slate-800 dark:text-slate-200">Listings ({listings.length})</h2>
        <Button
          variant="secondary"
          disabled={tickets.length === 0 || marketplaces === null}
          onClick={() => {
            setEditing(null);
            setFormOpen(true);
          }}
        >
          <IconPlus className="h-4 w-4" /> Add listing
        </Button>
      </div>

      {listings.length === 0 ? (
        <EmptyState
          title="No listings for this event yet"
          description={
            tickets.length === 0
              ? "Add a ticket to this event first, then list it on a marketplace here."
              : `Click "Add listing" to record where a ticket is posted for sale.`
          }
        />
      ) : (
        <div className="overflow-x-auto rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-sm">
          <table className="w-full min-w-[760px] border-collapse">
            <thead className="border-b border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-800/60">
              <tr>
                <th className="th">Ticket</th>
                <th className="th">Marketplace</th>
                <th className="th text-right">Price</th>
                <th className="th">Status</th>
                <th className="th">URL</th>
                <th className="th">Last updated</th>
                <th className="th" aria-label="Actions"></th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
              {listings.map((l) => (
                <tr key={l.id} className="hover:bg-slate-50 dark:hover:bg-slate-800/60">
                  <td className="td">
                    <Link
                      to={`/tickets?code=${encodeURIComponent(l.ticketCode)}`}
                      className="font-medium text-slate-900 dark:text-slate-100 hover:text-brand-700 dark:hover:text-brand-400"
                    >
                      {l.ticketCode}
                    </Link>
                    {[l.ticketSection, l.ticketRowLabel, l.ticketSeat].filter(Boolean).length > 0 && (
                      <div className="text-xs text-slate-400 dark:text-slate-500">
                        {[l.ticketSection, l.ticketRowLabel, l.ticketSeat].filter(Boolean).join(" / ")}
                      </div>
                    )}
                  </td>
                  <td className="td text-slate-700 dark:text-slate-300">{l.marketplaceName}</td>
                  <td className="td text-right tabular-nums">{formatMoney(l.priceCents, l.currency)}</td>
                  <td className="td">
                    <Badge tone={l.status}>{l.status}</Badge>
                  </td>
                  <td className="td">
                    {l.listingUrl ? (
                      <a
                        href={l.listingUrl}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="inline-flex items-center gap-1 text-brand-600 dark:text-brand-400 hover:underline"
                      >
                        <IconLink className="h-3.5 w-3.5" /> Open
                      </a>
                    ) : (
                      <span className="text-slate-400 dark:text-slate-500">-</span>
                    )}
                  </td>
                  <td className="td text-slate-500 dark:text-slate-400">{formatDateTime(l.updatedAt)}</td>
                  <td className="td">
                    <div className="flex justify-end gap-1">
                      <button
                        type="button"
                        onClick={() => {
                          setEditing(l);
                          setFormOpen(true);
                        }}
                        className="rounded-md p-1.5 text-slate-400 hover:bg-slate-100 hover:text-slate-700 dark:hover:bg-slate-800 dark:hover:text-slate-200"
                        aria-label="Edit listing"
                      >
                        <IconPencil className="h-4 w-4" />
                      </button>
                      <button
                        type="button"
                        onClick={() => setDeleteTarget(l)}
                        className="rounded-md p-1.5 text-slate-400 hover:bg-red-50 hover:text-red-600 dark:hover:bg-red-500/10 dark:hover:text-red-400"
                        aria-label="Delete listing"
                      >
                        <IconTrash className="h-4 w-4" />
                      </button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      <TicketListingFormModal
        open={formOpen}
        initial={editing}
        eventTickets={tickets}
        marketplaces={marketplaces ?? []}
        onClose={() => setFormOpen(false)}
        onSaved={load}
      />

      <ConfirmDialog
        open={deleteTarget !== null}
        title="Delete this listing?"
        message={
          deleteTarget ? `This removes the ${deleteTarget.marketplaceName} listing for ticket ${deleteTarget.ticketCode}. This cannot be undone.` : ""
        }
        confirmLabel="Delete listing"
        danger
        busy={deleting}
        onCancel={() => setDeleteTarget(null)}
        onConfirm={async () => {
          if (!deleteTarget) return;
          setDeleting(true);
          try {
            await api.deleteTicketListing(deleteTarget.id);
            toast.success("Listing deleted.");
            setDeleteTarget(null);
            load();
          } catch (e) {
            toast.error(errMsg(e));
          } finally {
            setDeleting(false);
          }
        }}
      />
    </div>
  );
}

// One (ticket, marketplace) listing's create/edit form. The ticket a listing
// belongs to is only pickable when CREATING - editing shows it as plain
// text, same "round-trip a field the form doesn't expose" spirit as
// Transactions.tsx's own order-linked entries (there is no UI anywhere to
// re-parent a listing to a different ticket).
function TicketListingFormModal({
  open,
  initial,
  eventTickets,
  marketplaces,
  onClose,
  onSaved,
}: {
  open: boolean;
  initial: TicketListing | null;
  eventTickets: Ticket[];
  marketplaces: Marketplace[];
  onClose: () => void;
  onSaved: () => void;
}) {
  const toast = useToast();
  const [ticketId, setTicketId] = useState("");
  const [marketplaceId, setMarketplaceId] = useState("");
  const [listingIdText, setListingIdText] = useState("");
  const [listingUrl, setListingUrl] = useState("");
  const [price, setPrice] = useState("");
  const [currency, setCurrency] = useState("EUR");
  const [status, setStatus] = useState<"active" | "sold" | "removed">("active");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    setTicketId(initial ? String(initial.ticketId) : "");
    setMarketplaceId(initial ? String(initial.marketplaceId) : "");
    setListingIdText(initial?.listingId ?? "");
    setListingUrl(initial?.listingUrl ?? "");
    setPrice(initial ? centsToDecimalString(initial.priceCents) : "");
    setCurrency(initial?.currency ?? "EUR");
    setStatus(initial?.status ?? "active");
    setError(null);
  }, [open, initial]);

  const ticketLabel = (t: Ticket) => {
    const seat = [t.section, t.rowLabel, t.seat].filter(Boolean).join(" / ");
    return seat ? `${t.code} (${seat})` : t.code;
  };

  const submit = async () => {
    if (!ticketId) {
      setError("Pick a ticket.");
      return;
    }
    if (!marketplaceId) {
      setError("Pick a marketplace.");
      return;
    }
    const cents = decimalStringToCents(price);
    if (cents === null || cents < 0) {
      setError("Enter a valid price.");
      return;
    }
    setSaving(true);
    setError(null);
    const input: TicketListingInput = {
      ticketId: Number(ticketId),
      marketplaceId: Number(marketplaceId),
      listingId: listingIdText.trim() || null,
      listingUrl: listingUrl.trim() || null,
      priceCents: cents,
      currency,
      status,
    };
    try {
      if (initial) {
        await api.updateTicketListing(initial.id, input);
        toast.success("Listing updated.");
      } else {
        await api.createTicketListing(input);
        toast.success("Listing added.");
      }
      onSaved();
      onClose();
    } catch (e) {
      setError(errMsg(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal open={open} onClose={onClose} title={initial ? "Edit listing" : "Add listing"}>
      <div className="space-y-3">
        <Field label="Ticket" required>
          {initial ? (
            <p className="rounded-lg bg-slate-50 px-3 py-2 text-sm text-slate-600 dark:bg-slate-800/60 dark:text-slate-300">
              {initial.ticketCode}
              {[initial.ticketSection, initial.ticketRowLabel, initial.ticketSeat].filter(Boolean).length > 0 && (
                <span className="text-slate-400 dark:text-slate-500">
                  {" "}
                  ({[initial.ticketSection, initial.ticketRowLabel, initial.ticketSeat].filter(Boolean).join(" / ")})
                </span>
              )}
            </p>
          ) : (
            <Select
              value={ticketId}
              onChange={(e) => {
                const nextTicketId = e.target.value;
                setTicketId(nextTicketId);
                const picked = eventTickets.find((t) => String(t.id) === nextTicketId);
                if (picked) setCurrency(picked.currency);
              }}
            >
              <option value="">Pick a ticket...</option>
              {eventTickets.map((t) => (
                <option key={t.id} value={t.id}>
                  {ticketLabel(t)}
                </option>
              ))}
            </Select>
          )}
        </Field>

        <Field label="Marketplace" required hint={marketplaces.length === 0 ? "Add a marketplace in Price Checker first." : undefined}>
          <Select value={marketplaceId} onChange={(e) => setMarketplaceId(e.target.value)}>
            <option value="">Pick a marketplace...</option>
            {marketplaces.map((m) => (
              <option key={m.id} value={m.id}>
                {m.name}
              </option>
            ))}
          </Select>
        </Field>

        <div className="grid grid-cols-[1fr_110px] gap-2">
          <Field label="Price" required>
            <Input inputMode="decimal" placeholder="0.00" value={price} onChange={(e) => setPrice(e.target.value)} />
          </Field>
          <Field label="Currency">
            <Select value={currency} onChange={(e) => setCurrency(e.target.value)}>
              {(CURRENCIES.includes(currency) ? CURRENCIES : [currency, ...CURRENCIES]).map((c) => (
                <option key={c} value={c}>
                  {c}
                </option>
              ))}
            </Select>
          </Field>
        </div>

        <Field label="Status">
          <div className="flex rounded-lg border border-slate-200 dark:border-slate-800 p-1">
            {(["active", "sold", "removed"] as const).map((s) => (
              <button
                key={s}
                type="button"
                onClick={() => setStatus(s)}
                className={`flex-1 rounded-md px-2.5 py-1.5 text-xs font-medium capitalize transition-colors ${
                  status === s ? "bg-brand-600 text-white" : "text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800"
                }`}
              >
                {s}
              </button>
            ))}
          </div>
        </Field>

        <Field label="Listing ID" hint="The marketplace's own id for this listing, if you have one.">
          <Input value={listingIdText} onChange={(e) => setListingIdText(e.target.value)} />
        </Field>

        <Field label="Listing URL">
          <Input type="url" placeholder="https://..." value={listingUrl} onChange={(e) => setListingUrl(e.target.value)} />
        </Field>

        {error && <p className="text-xs text-red-600 dark:text-red-400">{error}</p>}
      </div>
      <ModalFooter>
        <Button variant="secondary" onClick={onClose} disabled={saving}>
          Cancel
        </Button>
        <Button variant="primary" onClick={submit} disabled={saving}>
          {saving ? <Spinner className="h-4 w-4" /> : null}
          {initial ? "Save changes" : "Add listing"}
        </Button>
      </ModalFooter>
    </Modal>
  );
}

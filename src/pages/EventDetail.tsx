import { useCallback, useEffect, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { api, errMsg } from "../lib/api";
import type { EventWithStats, FinanceEntry, OrderRecord, PriceCheckerSummary, SaleGroup, Ticket } from "../lib/types";
import { formatDate, formatMoney, formatMoneyOrMixed, formatPercent, formatPercentOrMixed } from "../lib/format";
import {
  Badge,
  Button,
  Card,
  ConfirmDialog,
  EmptyState,
  LoadingBlock,
  StatCard,
  TabSwitcher,
} from "../components/ui";
import { FinanceCategoryBadge } from "../components/FinanceCategoryBadge";
import { IconArrowLeft, IconPencil, IconPlus, IconTrash } from "../components/icons";
import { useToast } from "../lib/toast";
import { EventFormModal } from "./Events";

// 2.2.3: "Event Workspace" - marko's own request to turn this page into one
// central place for a single Event, with everything else it touches
// organized under tabs instead of one long scroll. Overview is the only tab
// with genuinely new content (the same stats this page already showed,
// trimmed to exactly marko's own list); Inventory is literally the Orders +
// Tickets tables this page already had, just relocated; Sales/Market/Finance
// each pull from an already-existing command (list_sale_groups,
// get_price_checker_summary, list_finance_entries_for_order x N) rather than
// inventing anything new. 2.2.3 (second pass): Tasks removed (marko decided
// against it before it ever had a spec); Listings added between Inventory
// and Sales - a read-only look at the Ticket rows already filtered to
// status === "listed", reusing `tickets` this page already loads. No
// marketplace/listing-URL/last-checked columns - see ListingsTab's own doc
// comment for why (that data doesn't exist anywhere in this schema today).
type WorkspaceTab = "overview" | "inventory" | "listings" | "sales" | "market" | "finance";

const WORKSPACE_TABS: { key: WorkspaceTab; label: string }[] = [
  { key: "overview", label: "Overview" },
  { key: "inventory", label: "Inventory" },
  { key: "listings", label: "Listings" },
  { key: "sales", label: "Sales" },
  { key: "market", label: "Market" },
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

      {tab === "overview" && <OverviewTab event={event} />}
      {tab === "inventory" && <InventoryTab event={event} orders={orders} tickets={tickets} navigate={navigate} />}
      {tab === "listings" && <ListingsTab tickets={tickets} />}
      {tab === "sales" && <SalesTab eventId={eventId} />}
      {tab === "market" && <MarketTab event={event} tickets={tickets} navigate={navigate} />}
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
// Overview - exactly marko's own list (2.2.3): tickets, sold, available,
// total cost, revenue, profit, margin/ROI. Nothing else - the rest of what
// this page used to show up top now lives in its own tab.
// ---------------------------------------------------------------------------
function OverviewTab({ event }: { event: EventWithStats }) {
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
      <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
        <StatCard
          label="Profit"
          value={formatMoneyOrMixed(s.profitCents, s.currency)}
          tone={s.profitCents > 0 ? "positive" : s.profitCents < 0 ? "negative" : "default"}
        />
        <StatCard label="Margin" value={formatPercentOrMixed(s.margin, s.currency)} />
        <StatCard label="ROI" value={formatPercentOrMixed(s.roi, s.currency)} />
      </div>

      {event.notes && (
        <Card className="mt-6 p-4">
          <p className="mb-1 text-xs font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">Notes</p>
          <p className="whitespace-pre-wrap text-sm text-slate-700 dark:text-slate-300">{event.notes}</p>
        </Card>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Inventory - the Orders + Tickets tables this page already had, unchanged,
// just relocated under their own tab instead of always being on screen.
// ---------------------------------------------------------------------------
function InventoryTab({
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
  return (
    <div>
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
// Sales - new tab, but not new logic: list_sale_groups already accepts
// eventId (Sales.tsx's own Event filter uses the exact same call). No
// pending/completed tabs, search, or bulk actions here - this is a compact
// summary, not a re-implementation of Sales.tsx; "Open in Sales" underneath
// gets marko to the real thing for anything more than a glance.
// ---------------------------------------------------------------------------
function SalesTab({ eventId }: { eventId: number }) {
  const toast = useToast();
  const [groups, setGroups] = useState<SaleGroup[] | null>(null);

  useEffect(() => {
    setGroups(null);
    api
      .listSaleGroups({ eventId })
      .then(setGroups)
      .catch((e) => toast.error(errMsg(e)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [eventId]);

  if (groups === null) return <LoadingBlock />;

  return (
    <div>
      <div className="mb-3 flex items-center justify-between">
        <h2 className="text-sm font-semibold text-slate-800 dark:text-slate-200">Sales ({groups.length})</h2>
        <Link to="/sales" className="text-sm font-medium text-brand-600 dark:text-brand-400 hover:underline">
          Open in Sales &rarr;
        </Link>
      </div>
      {groups.length === 0 ? (
        <EmptyState title="No sales for this event yet" />
      ) : (
        // 2.2.3: no max-w cap - see the Inventory tables' own comment.
        <div className="overflow-x-auto rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-sm">
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
    </div>
  );
}

// ---------------------------------------------------------------------------
// Market - the "Potential Profit" block this page already had (unchanged
// calculation, just moved here since it's fundamentally about this event's
// position against the market), plus the same "Market vs. mine" summary
// PriceChecker.tsx itself shows once at least one price check is saved
// (get_price_checker_summary, unchanged). "Open in Price Checker" is where
// marko actually adds marketplaces/scans - not reimplemented here.
// ---------------------------------------------------------------------------
function MarketTab({
  event,
  tickets,
  navigate,
}: {
  event: EventWithStats;
  tickets: Ticket[] | null;
  navigate: ReturnType<typeof useNavigate>;
}) {
  const toast = useToast();
  const [summary, setSummary] = useState<PriceCheckerSummary | null>(null);

  useEffect(() => {
    api
      .getPriceCheckerSummary(event.id)
      .then(setSummary)
      .catch((e) => toast.error(errMsg(e)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [event.id]);

  // 1.8.3 (section 14) / 1.9.10: unchanged from this page's previous single-
  // tab version - see git history there for the original reasoning.
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
// Orders (list_finance_entries_for_order, 2.2.1), merged client-side. No new
// backend command - an event has at most a handful of orders, so N small
// queries stays cheap and avoids adding a second way to query the same data.
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
        // 2.2.3: no max-w cap - see the Inventory tables' own comment.
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
// Listings - a read-only look at this event's ACTIVE listings, i.e. its
// Tickets already filtered to status === "listed" (reuses the `tickets`
// this page already loads for Inventory - no new fetch). Deliberately shows
// only what genuinely exists on a Ticket today: the ticket itself, its
// listing price, currency, and status. Marketplace, listing URL, and last
// updated/checked were all explicitly asked for, but none of the three
// exist anywhere in this schema - a Ticket has never had a "which platform
// is this listed on", a listing URL, or a listing-specific timestamp column
// (checked migrations 001 and 010, the only two that ever touched
// `tickets`, plus Price Checker's own "Your Tickets" - the closest existing
// thing to a listings view - which doesn't have them either). Per marko's
// own instruction not to invent data we don't have, this tab does not show
// those 3 columns or a clickable URL - see REDESIGN-2.2.3-REPORT.md for the
// same explanation in Slovak, and the note in the empty/summary area below.
// ---------------------------------------------------------------------------
function ListingsTab({ tickets }: { tickets: Ticket[] | null }) {
  if (tickets === null) return <LoadingBlock />;

  const listed = tickets.filter((t) => t.status === "listed");
  const priced = listed.filter((t) => t.listingPriceCents != null);
  const listedValueCents = priced.reduce((sum, t) => sum + (t.listingPriceCents ?? 0), 0);
  const prices = priced.map((t) => t.listingPriceCents as number);
  const lowestCents = prices.length > 0 ? Math.min(...prices) : null;
  const highestCents = prices.length > 0 ? Math.max(...prices) : null;
  const currencies = Array.from(new Set(listed.map((t) => t.currency)));
  const listingsCurrency = currencies.length <= 1 ? (currencies[0] ?? null) : null;

  return (
    <div>
      <div className="mb-3 grid grid-cols-2 gap-3 sm:grid-cols-4">
        <StatCard label="Active listings" value={String(listed.length)} />
        <StatCard label="Listed value" value={formatMoneyOrMixed(listedValueCents, listingsCurrency)} />
        <StatCard label="Lowest price" value={lowestCents !== null ? formatMoneyOrMixed(lowestCents, listingsCurrency) : "-"} />
        <StatCard label="Highest price" value={highestCents !== null ? formatMoneyOrMixed(highestCents, listingsCurrency) : "-"} />
      </div>
      <p className="mb-6 text-xs text-slate-400 dark:text-slate-500">
        Marketplace, listing URL and last checked date aren&apos;t tracked in TIQR yet, so they&apos;re not shown here - let
        me know if you want to start recording those and I&apos;ll add them properly instead of guessing.
      </p>

      <h2 className="mb-3 text-sm font-semibold text-slate-800 dark:text-slate-200">Listings ({listed.length})</h2>
      {listed.length === 0 ? (
        <EmptyState title="No active listings for this event yet" description={`Set a listing price and mark a ticket "Listed" in Inventory to see it here.`} />
      ) : (
        <div className="overflow-x-auto rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-sm">
          <table className="w-full min-w-[600px] border-collapse">
            <thead className="border-b border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-800/60">
              <tr>
                <th className="th">Ticket</th>
                <th className="th">Seat</th>
                <th className="th text-right">Listing price</th>
                <th className="th">Currency</th>
                <th className="th">Status</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
              {listed.map((t) => (
                <tr key={t.id} className="hover:bg-slate-50 dark:hover:bg-slate-800/60">
                  <td className="td">
                    <Link to={`/tickets?code=${encodeURIComponent(t.code)}`} className="font-medium text-slate-900 dark:text-slate-100 hover:text-brand-700 dark:hover:text-brand-400">
                      {t.code}
                    </Link>
                  </td>
                  <td className="td text-slate-500 dark:text-slate-400">
                    {[t.section, t.rowLabel, t.seat].filter(Boolean).join(" / ") || "-"}
                  </td>
                  <td className="td text-right tabular-nums">
                    {t.listingPriceCents != null ? formatMoney(t.listingPriceCents, t.currency) : "-"}
                  </td>
                  <td className="td text-slate-500 dark:text-slate-400">{t.currency}</td>
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

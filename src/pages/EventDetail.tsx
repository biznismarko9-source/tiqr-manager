import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { api, errMsg } from "../lib/api";
import type {
  AttentionCenterItem,
  EventWithStats,
  FinanceEntry,
  InventoryIntelligence,
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
  formatSeatLocation,
} from "../lib/format";
import {
  Badge,
  Button,
  Card,
  CHECKBOX_CLASS,
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
import {
  IconArrowLeft,
  IconChevronDown,
  IconLink,
  IconPencil,
  IconPlus,
  IconSearch,
  IconTrash,
  IconX,
} from "../components/icons";
import { useToast } from "../lib/toast";
import {
  computeEventLifecyclePhase,
  EVENT_LIFECYCLE_PHASES,
  EventFormModal,
  LIFECYCLE_PHASE_DOT,
} from "./Events";
import { CURRENCIES } from "./Orders";
import { isSaleGroupDone } from "./Sales";

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
// 2) Listings rebuilt into a real system - see ListingsTab's own doc
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
//    See ListingsTab's own doc comment below for the full design.
//
// 2.2.6: "Inventory Intelligence" - a compact block on Overview, above the
// Orders/Tickets tables, with KPIs/aging/attention/breakdowns. See
// InventoryIntelligenceBlock's own doc comment below for the full design and
// commands/inventory_intelligence.rs for which existing definition each
// number reuses (nothing here is a second implementation of Potential
// Profit, Listed value, or the Price Checker market comparison - all three
// already exist elsewhere on this page/app). Clicking a KPI/aging/attention/
// breakdown row filters THIS tab's own already-rendered Tickets table down
// to the matching tickets (by id, computed once on the backend) rather than
// navigating away - Overview already shows that table right below.
type WorkspaceTab = "overview" | "listings" | "sales";

const WORKSPACE_TABS: { key: WorkspaceTab; label: string }[] = [
  { key: "overview", label: "Overview" },
  { key: "listings", label: "Listings" },
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

      <TabSwitcher tabs={WORKSPACE_TABS} active={tab} onChange={setTab} />

      {tab === "overview" && <OverviewTab event={event} orders={orders} tickets={tickets} navigate={navigate} onSwitchTab={setTab} />}
      {tab === "listings" && <ListingsTab eventId={eventId} tickets={tickets} orders={orders} />}
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

  // 2.2.6: set by InventoryIntelligenceBlock when a KPI/aging/attention/
  // breakdown row is clicked - filters the Tickets table below down to just
  // those ticket ids, with a small banner explaining what's shown and a way
  // to clear it. `null` (the default) shows every ticket, unchanged from
  // before this feature.
  const [highlight, setHighlight] = useState<{ ids: number[]; label: string } | null>(null);
  const ticketsAnchorRef = useRef<HTMLDivElement>(null);
  const applyHighlight = (ids: number[] | null, label: string | null) => {
    setHighlight(ids && label ? { ids, label } : null);
    ticketsAnchorRef.current?.scrollIntoView({ behavior: "smooth", block: "start" });
  };
  const highlightedIds = highlight ? new Set(highlight.ids) : null;
  const visibleTickets = highlightedIds ? (tickets ?? []).filter((t) => highlightedIds.has(t.id)) : tickets;

  return (
    <div>
      <EventLifecycleBlock event={event} onSwitchTab={onSwitchTab} />

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

      <InventoryIntelligenceBlock eventId={event.id} onSwitchTab={onSwitchTab} onHighlight={applyHighlight} />

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

      <div ref={ticketsAnchorRef} className="mb-3 flex flex-wrap items-center justify-between gap-2">
        <h2 className="text-sm font-semibold text-slate-800 dark:text-slate-200">
          Tickets ({visibleTickets?.length ?? 0}
          {highlight ? ` of ${tickets?.length ?? 0}` : ""})
        </h2>
        {highlight && (
          <p className="text-xs text-slate-500 dark:text-slate-400">
            Showing: <span className="font-medium text-slate-700 dark:text-slate-300">{highlight.label}</span>{" "}
            <button type="button" className="font-medium text-brand-600 dark:text-brand-400 hover:underline" onClick={() => applyHighlight(null, null)}>
              Clear filter
            </button>
          </p>
        )}
      </div>
      {tickets === null ? (
        <LoadingBlock />
      ) : tickets.length === 0 ? (
        <EmptyState title="No tickets for this event yet" />
      ) : visibleTickets && visibleTickets.length === 0 ? (
        <EmptyState title="No tickets match this filter" description="Clear the filter above to see every ticket again." />
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
              {(visibleTickets ?? []).map((t) => (
                <tr key={t.id} className="hover:bg-slate-50 dark:hover:bg-slate-800/60">
                  <td className="td">
                    <Link to={`/tickets?code=${encodeURIComponent(t.code)}`} className="font-medium text-slate-900 dark:text-slate-100 hover:text-brand-700 dark:hover:text-brand-400">
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
// Event Lifecycle (2.3.0) - "what stage is this event at", shown at the very
// top of Overview per marko's own request ("Current phase, jednoduchý
// progress/lifecycle indicator, základný operational summary"). The phase
// itself (computeEventLifecyclePhase) is a pure function of `event` alone -
// see Events.tsx for the full rule and why POST EVENT was folded into
// COMPLETED - so it costs nothing extra to compute here. This block's own
// two fetches are for the two numbers that pure function deliberately does
// NOT fold in (pending fulfillment, Next Actions) - both reuse existing,
// already-shipped endpoints, called the same "fetch independently, keyed on
// eventId" way InventoryIntelligenceBlock below already does:
//   - list_sale_groups, with its existing eventId filter (Sales.tsx/
//     FulfillmentCenter.tsx already call this exact command) - counts this
//     event's own sale batches that Fulfillment Center's own isSaleGroupDone
//     would call NOT done yet ("pending fulfillment").
//   - get_attention_center, the Dashboard's own GLOBAL Attention Center,
//     filtered client-side to this one event's items. 2.2.9 removed this
//     block's OWN per-event attention list in favor of that global view
//     ("fully superseded... which already covers every one of these same
//     categories") - this is not a reversal of that call: NEXT ACTIONS below
//     is a compact, lifecycle-scoped restatement (per marko's explicit new
//     instruction: "Použi už existujúce Attention Center... dáta"), not a
//     second copy of the Dashboard's own fuller per-order rows, and nothing
//     is computed here that get_attention_center doesn't already compute.
// No new business logic anywhere in this block - every predicate (is a sale
// group done, does an attention item belong to this event) already existed
// before 2.3.0.
// ---------------------------------------------------------------------------

// 2.3.0: fixed per-category presentation for the 4 ticket-grouped Attention
// Center categories - a short noun phrase plus which tab "fix this" points
// to. Deliberately excludes event_soon (shown separately below via its own
// already-correct `reason` text - it has no ticketIds to count, and Overview
// is already the page it would point back to). This is presentation only
// (labels + tab targets), not a new rule for WHEN something needs attention -
// that determination is still made entirely by attention_center.rs.
const NEXT_ACTION_CONFIG: { category: AttentionCenterItem["category"]; noun: (n: number) => string; tab: WorkspaceTab }[] = [
  { category: "no_active_listing", noun: (n) => `${n} ticket${n === 1 ? "" : "s"} not listed`, tab: "listings" },
  { category: "missing_listing_price", noun: (n) => `${n} ticket${n === 1 ? "" : "s"} missing a listing price`, tab: "listings" },
  { category: "sold_undelivered", noun: (n) => `${n} sold ticket${n === 1 ? "" : "s"} not delivered`, tab: "sales" },
  { category: "outside_market_price", noun: (n) => `${n} ticket${n === 1 ? "" : "s"} priced outside the market range`, tab: "sales" },
];

function EventLifecycleBlock({ event, onSwitchTab }: { event: EventWithStats; onSwitchTab: (tab: WorkspaceTab) => void }) {
  const toast = useToast();
  const [saleGroups, setSaleGroups] = useState<SaleGroup[] | null>(null);
  const [attentionItems, setAttentionItems] = useState<AttentionCenterItem[] | null>(null);

  useEffect(() => {
    setSaleGroups(null);
    setAttentionItems(null);
    api.listSaleGroups({ eventId: event.id }).then(setSaleGroups).catch((e) => toast.error(errMsg(e)));
    api.getAttentionCenter().then(setAttentionItems).catch((e) => toast.error(errMsg(e)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [event.id]);

  const phase = computeEventLifecyclePhase(event);
  const phaseLabel = EVENT_LIFECYCLE_PHASES.find((p) => p.key === phase)?.label ?? phase;
  const s = event.stats;

  // 2.3.0: this event's own sale batches not yet fully paid+delivered+
  // completed - exactly Fulfillment Center's "Pending Sales" definition
  // (isSaleGroupDone, imported from Sales.tsx), scoped to this one event.
  // Deliberately NOT gated by this event's own phase - a COMPLETED event can
  // still have pending fulfillment, same reasoning sold_undelivered below
  // already applies globally (see this block's own doc comment above).
  const pendingFulfillment = saleGroups ? saleGroups.filter((g) => !isSaleGroupDone(g)).length : null;

  const eventAttentionItems = (attentionItems ?? []).filter((i) => i.eventId === event.id);
  const eventSoon = eventAttentionItems.find((i) => i.category === "event_soon") ?? null;
  const nextActions: { key: string; label: string; onClick: (() => void) | null }[] = [];
  if (eventSoon) {
    // Already exactly the right granularity (one per event) and already
    // correctly worded by the backend - shown verbatim, same as Dashboard's
    // own AttentionCenterRow does for every category's `reason` text.
    nextActions.push({ key: "event_soon", label: eventSoon.reason, onClick: null });
  }
  NEXT_ACTION_CONFIG.forEach(({ category, noun, tab }) => {
    const ticketCount = eventAttentionItems
      .filter((i) => i.category === category)
      .reduce((sum, i) => sum + i.ticketIds.length, 0);
    if (ticketCount > 0) {
      nextActions.push({ key: category, label: noun(ticketCount), onClick: () => onSwitchTab(tab) });
    }
  });

  const loading = saleGroups === null || attentionItems === null;

  return (
    <Card className="mb-6 p-4">
      <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
        <p className="text-xs font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">Event Lifecycle</p>
        <div className="flex items-center gap-2">
          <span className={`h-2.5 w-2.5 rounded-full ${LIFECYCLE_PHASE_DOT[phase]}`} />
          <span className="text-base font-semibold text-slate-900 dark:text-slate-100">{phaseLabel}</span>
        </div>
      </div>

      {/* Simple progress indicator - shows WHICH stage this event is
          currently at among the fixed sequence, not a claim that it passed
          through every earlier one (an event can jump straight from
          UPCOMING to COMPLETED - e.g. cancelled on day one with nothing
          ever bought). See computeEventLifecyclePhase's own comment. */}
      <div className="mb-4 flex items-center gap-1">
        {EVENT_LIFECYCLE_PHASES.map((p) => (
          <div
            key={p.key}
            title={p.label}
            className={`h-1.5 flex-1 rounded-full ${p.key === phase ? LIFECYCLE_PHASE_DOT[phase] : "bg-slate-200 dark:bg-slate-700"}`}
          />
        ))}
      </div>

      <p className="mb-4 text-sm text-slate-600 dark:text-slate-300">
        <span className="tabular-nums">{s.purchasedTickets}</span> tickets &middot; <span className="tabular-nums">{s.listedTickets}</span>{" "}
        listed &middot; <span className="tabular-nums">{s.soldTickets}</span> sold &middot;{" "}
        {pendingFulfillment === null ? (
          <Spinner className="inline-block h-3 w-3" />
        ) : (
          <span className="tabular-nums">{pendingFulfillment}</span>
        )}{" "}
        pending fulfillment
      </p>

      <p className="mb-2 text-xs font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">Next Actions</p>
      {loading ? (
        <LoadingBlock />
      ) : nextActions.length === 0 ? (
        <p className="text-sm text-slate-400 dark:text-slate-500">Nothing needs attention for this event right now.</p>
      ) : (
        <ul className="space-y-1.5">
          {nextActions.map((a) =>
            a.onClick ? (
              <li key={a.key}>
                <button type="button" onClick={a.onClick} className="text-sm text-brand-600 hover:underline dark:text-brand-400">
                  {a.label}
                </button>
              </li>
            ) : (
              <li key={a.key} className="text-sm text-slate-600 dark:text-slate-300">
                {a.label}
              </li>
            ),
          )}
        </ul>
      )}
    </Card>
  );
}

// ---------------------------------------------------------------------------
// Inventory Intelligence (2.2.6) - a compact block on Overview, above the
// Orders/Tickets tables, built from a single new backend command
// (`getInventoryIntelligence`) that reuses existing definitions rather than
// inventing new ones - see commands/inventory_intelligence.rs's own doc
// comment for exactly which existing computation each number below comes
// from (finance::compute_summary's own scope for Total tickets/Total
// invested, ListingsTab's own "Listed value" definition for Current listed
// value, SalesTab's own "Potential Profit" definition unchanged, and
// commands::price_checker::get_price_checker_summary_impl - the same
// function SalesTab's "Market vs. mine" card already calls - for the
// outside-market-price attention item).
//
// Fetches independently, keyed on eventId, same "each piece of the Event
// Workspace fetches its own data" convention as Sales/Listings above (2.2.2's
// own doc comment). Every clickable row calls `onHighlight(ticketIds, label)`
// (filters Overview's own Tickets table, see OverviewTab above) or
// `onSwitchTab("listings")` for Current listed value, which is fundamentally
// about `ticket_listings` rows rather than raw tickets.
//
// No "by tier" breakdown - flagged explicitly, in the UI itself, rather than
// invented: see InventoryIntelligence's own doc comment (lib/types.ts).
//
// 2.2.9: this block's own per-event "Attention" list (event_soon/missing
// listing price/no active listing/outside market price rows) was removed -
// marko's own request, now fully superseded by the Dashboard's GLOBAL
// Attention Center (2.2.8, reworked in 2.2.9 to group by order - see
// commands/attention_center.rs), which already covers every one of these
// same categories across every event from one place. The backend command
// this block still calls below (`getInventoryIntelligence`) is UNCHANGED -
// attention_center.rs calls its underlying impl function directly and still
// depends on it - only this page's own rendering of its `.attention` field
// was deleted; KPIs/Aging/By tier/section/marketplace below are untouched.
// ---------------------------------------------------------------------------

/** Small local stand-in for `StatCard` that's actually clickable - `StatCard`
 * itself has no `onClick`, and this block's whole point is that every number
 * drills into something, so a plain non-interactive card would be the wrong
 * primitive here. Copies the exact same `.card` look every other card on
 * this page already uses (see index.css) rather than inventing a new style.
 * `disabled` renders a plain, unclickable version (used for a zero-count
 * bucket - nothing to show). */
function ClickableStat({ label, value, sub, onClick, disabled }: { label: string; value: string; sub?: string; onClick: () => void; disabled?: boolean }) {
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={onClick}
      className="card p-3 text-left transition hover:ring-2 hover:ring-brand-200 disabled:cursor-default disabled:opacity-60 disabled:hover:ring-0 dark:hover:ring-brand-900"
    >
      <p className="text-xs font-medium uppercase tracking-wide text-slate-400 dark:text-slate-500">{label}</p>
      <p className="mt-1 text-lg font-semibold tabular-nums text-slate-900 dark:text-slate-100">{value}</p>
      {sub && <p className="mt-0.5 text-xs text-slate-400 dark:text-slate-500">{sub}</p>}
    </button>
  );
}

function InventoryIntelligenceBlock({
  eventId,
  onSwitchTab,
  onHighlight,
}: {
  eventId: number;
  onSwitchTab: (tab: WorkspaceTab) => void;
  onHighlight: (ids: number[] | null, label: string | null) => void;
}) {
  const toast = useToast();
  const [data, setData] = useState<InventoryIntelligence | null>(null);

  useEffect(() => {
    setData(null);
    api
      .getInventoryIntelligence(eventId)
      .then(setData)
      .catch((e) => toast.error(errMsg(e)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [eventId]);

  if (data === null) {
    return (
      <Card className="mb-6 p-4">
        <LoadingBlock />
      </Card>
    );
  }

  const { kpis, aging, breakdownByTier, breakdownBySection, breakdownByMarketplace, unsoldTicketIds, soldTicketIds } = data;
  const clearFilter = () => onHighlight(null, null);

  return (
    <Card className="mb-6 p-4">
      <p className="mb-3 text-xs font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">Inventory Intelligence</p>

      <div className="mb-4 grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-6">
        <ClickableStat label="Total tickets" value={String(kpis.totalTickets)} onClick={clearFilter} />
        <ClickableStat label="Total invested" value={formatMoneyOrMixed(kpis.totalInvestedCents, kpis.currency)} onClick={clearFilter} />
        <ClickableStat
          label="Current listed value"
          value={formatMoneyOrMixed(kpis.currentListedValueCents, kpis.currentListedValueCurrency)}
          sub="Active listings"
          onClick={() => onSwitchTab("listings")}
        />
        <ClickableStat
          label="Potential profit"
          value={formatMoneyOrMixed(kpis.potentialProfitCents, kpis.potentialProfitCurrency)}
          sub="Unsold inventory"
          onClick={() => onHighlight(unsoldTicketIds, "Unsold tickets (potential profit)")}
        />
        <ClickableStat
          label="Sell-through"
          value={formatPercent(kpis.sellThroughPct)}
          onClick={() => onHighlight(soldTicketIds, "Sold tickets")}
        />
        <ClickableStat
          label="Avg. ticket cost"
          value={kpis.averageTicketCostCents != null ? formatMoneyOrMixed(kpis.averageTicketCostCents, kpis.currency) : "-"}
          onClick={clearFilter}
        />
      </div>

      <p className="mb-2 text-xs font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">Aging (unsold tickets)</p>
      <div className="mb-4 grid grid-cols-2 gap-3 sm:grid-cols-4">
        {aging.map((b) => (
          <ClickableStat
            key={b.key}
            label={b.label}
            value={String(b.ticketCount)}
            disabled={b.ticketCount === 0}
            onClick={() => onHighlight(b.ticketIds, `Aging: ${b.label}`)}
          />
        ))}
      </div>

      <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
        <div>
          {/* 2.2.7: tickets.tier (migration 024) - see InventoryIntelligence's
              own doc comment (types.ts) for why this used to say "not
              tracked yet" here. Blank/null groups as "Unknown" (backend-
              computed, not a frontend fallback) - deliberately different
              wording from the section breakdown's own "No section" below. */}
          <p className="mb-2 text-xs font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">By tier</p>
          {breakdownByTier.length === 0 ? (
            <p className="text-xs text-slate-400 dark:text-slate-500">No unsold tickets.</p>
          ) : (
            <ul className="divide-y divide-slate-100 rounded-lg border border-slate-200 dark:divide-slate-800 dark:border-slate-800">
              {breakdownByTier.map((g) => (
                <li key={g.label}>
                  <button
                    type="button"
                    className="flex w-full items-center justify-between px-3 py-2 text-left text-sm hover:bg-slate-50 dark:hover:bg-slate-800/60"
                    onClick={() => onHighlight(g.ticketIds, `Tier: ${g.label}`)}
                  >
                    <span className="text-slate-700 dark:text-slate-300">{g.label}</span>
                    <span className="tabular-nums text-slate-500 dark:text-slate-400">
                      {g.ticketCount} &middot; {formatMoneyOrMixed(g.totalCents, g.currency)}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
        <div>
          <p className="mb-2 text-xs font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">By section</p>
          {breakdownBySection.length === 0 ? (
            <p className="text-xs text-slate-400 dark:text-slate-500">No unsold tickets.</p>
          ) : (
            <ul className="divide-y divide-slate-100 rounded-lg border border-slate-200 dark:divide-slate-800 dark:border-slate-800">
              {breakdownBySection.map((g) => (
                <li key={g.label}>
                  <button
                    type="button"
                    className="flex w-full items-center justify-between px-3 py-2 text-left text-sm hover:bg-slate-50 dark:hover:bg-slate-800/60"
                    onClick={() => onHighlight(g.ticketIds, `Section: ${g.label}`)}
                  >
                    <span className="text-slate-700 dark:text-slate-300">{g.label}</span>
                    <span className="tabular-nums text-slate-500 dark:text-slate-400">
                      {g.ticketCount} &middot; {formatMoneyOrMixed(g.totalCents, g.currency)}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
        <div>
          <p className="mb-2 text-xs font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">By marketplace</p>
          {breakdownByMarketplace.length === 0 ? (
            <p className="text-xs text-slate-400 dark:text-slate-500">No active listings.</p>
          ) : (
            <ul className="divide-y divide-slate-100 rounded-lg border border-slate-200 dark:divide-slate-800 dark:border-slate-800">
              {breakdownByMarketplace.map((g) => (
                <li key={g.label}>
                  <button
                    type="button"
                    className="flex w-full items-center justify-between px-3 py-2 text-left text-sm hover:bg-slate-50 dark:hover:bg-slate-800/60"
                    onClick={() => onHighlight(g.ticketIds, `Marketplace: ${g.label}`)}
                  >
                    <span className="text-slate-700 dark:text-slate-300">{g.label}</span>
                    <span className="tabular-nums text-slate-500 dark:text-slate-400">
                      {g.ticketCount} &middot; {formatMoneyOrMixed(g.totalCents, g.currency)}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
    </Card>
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
  const [summary, setSummary] = useState<PriceCheckerSummary | null>(null);
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
    api
      .getPriceCheckerSummary(event.id)
      .then(setSummary)
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
              {financeEntries.map((e) => (
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
// Listings (2.2.4 rebuild into a real system; 2.2.5 made genuinely
// manageable at volume) - on top of the `ticket_listings` table (see
// migrations/022_ticket_listings.sql and commands/ticket_listings.rs). One
// ticket can have several real listings, each tied to a real marketplace
// (the same list Price Checker manages), with its own price/status/URL/
// timestamp.
//
// Deliberately still manual-entry only - no automatic listing creation, no
// marketplace API, no repricing (marko's own explicit "Dôležité" list, both
// releases). The table below shows EVERY listing matching the current
// filters regardless of status (not just active ones) - marko's own field
// list explicitly asks for a "status" column, which is only meaningful if a
// listing can be shown in a state OTHER than active (sold/removed); the four
// summary numbers above it count active listings only, matching "počet
// aktívnych listingov" - and are NEVER affected by the status/marketplace/
// search filters below, same "summary is a fact about the data, filters are
// a view onto it" separation Sales.tsx's own cashTotals already follows.
//
// 2.2.5: marko's "vylepšiť Listings tak, aby sa dali reálne pohodlne riadiť"
// (make this genuinely convenient to manage) - four additions, entirely
// client-side except the three new bulk commands:
// - Status filter (All/Active/Sold/Removed) + Marketplace filter + Search
//   (ticket code/seat/marketplace/listing id/URL) - pure `.filter()` over
//   the one already-fetched `listings` array, same "small enough dataset,
//   no server round trip needed" reasoning as this tab's own summary cards.
// - Checkboxes are always visible in this table (no separate "selection
//   mode" toggle like Sales.tsx/Orders.tsx use for their own, usually much
//   longer, lists) - one event's listings is a small enough table that the
//   extra click a mode toggle would add isn't worth it. Small, reversible UI
//   call, flagged in REDESIGN-2.2.5-REPORT.md.
// - Select all/deselect all applies to the currently VISIBLE (filtered +
//   searched) rows only, same convention as Sales.tsx's own
//   `allSelected`/`toggleSelectAll` - "select all" should only ever select
//   what's actually on screen.
// - `ListingsBulkBar` (below) shows only while `selected.size > 0` (marko's
//   own explicit "Bulk actions zobraz iba keď je niečo vybrané") and covers
//   Edit status / Edit price / Delete, all backed by the new all-or-nothing
//   bulk commands in ticket_listings.rs.
// ---------------------------------------------------------------------------
type ListingStatusFilter = "all" | "active" | "sold" | "removed";

const LISTING_STATUS_TABS: { key: ListingStatusFilter; label: string }[] = [
  { key: "all", label: "All" },
  { key: "active", label: "Active" },
  { key: "sold", label: "Sold" },
  { key: "removed", label: "Removed" },
];

function ListingsTab({
  eventId,
  tickets,
  orders,
}: {
  eventId: number;
  tickets: Ticket[] | null;
  orders: OrderRecord[] | null;
}) {
  const toast = useToast();
  const [listings, setListings] = useState<TicketListing[] | null>(null);
  const [marketplaces, setMarketplaces] = useState<Marketplace[] | null>(null);
  const [formOpen, setFormOpen] = useState(false);
  const [editing, setEditing] = useState<TicketListing | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<TicketListing | null>(null);
  const [deleting, setDeleting] = useState(false);

  // 2.2.5: filters/search/selection - all client-side, see this section's
  // own doc comment above.
  const [statusFilter, setStatusFilter] = useState<ListingStatusFilter>("all");
  const [marketplaceFilter, setMarketplaceFilter] = useState<number | "">("");
  const [search, setSearch] = useState("");
  const [selected, setSelected] = useState<Set<number>>(new Set());

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

  // Keeps the selection consistent with whatever `listings` actually holds -
  // runs after every load (initial fetch, or a reload following any create/
  // edit/delete/bulk action), so a row that's gone (deleted, bulk-deleted)
  // can never linger in `selected` and inflate the bulk bar's count.
  useEffect(() => {
    if (listings === null) return;
    setSelected((prev) => {
      const validIds = new Set(listings.map((l) => l.id));
      const next = new Set(Array.from(prev).filter((id) => validIds.has(id)));
      return next.size === prev.size ? prev : next;
    });
  }, [listings]);

  if (listings === null || tickets === null) return <LoadingBlock />;

  // Summary counts/values are scoped to ACTIVE listings only, and are NEVER
  // affected by the filters below - see this section's own doc comment.
  const active = listings.filter((l) => l.status === "active");
  const activeValueCents = active.reduce((sum, l) => sum + l.priceCents, 0);
  const activePrices = active.map((l) => l.priceCents);
  const lowestCents = activePrices.length > 0 ? Math.min(...activePrices) : null;
  const highestCents = activePrices.length > 0 ? Math.max(...activePrices) : null;
  const activeCurrencies = Array.from(new Set(active.map((l) => l.currency)));
  const activeCurrency = activeCurrencies.length <= 1 ? (activeCurrencies[0] ?? null) : null;

  const searchNeedle = search.trim().toLowerCase();
  const visibleListings = listings.filter((l) => {
    if (statusFilter !== "all" && l.status !== statusFilter) return false;
    if (marketplaceFilter !== "" && l.marketplaceId !== marketplaceFilter) return false;
    if (searchNeedle === "") return true;
    const haystack = [l.ticketCode, l.ticketSection, l.ticketRowLabel, l.ticketSeat, l.marketplaceName, l.listingId, l.listingUrl]
      .filter(Boolean)
      .join(" ")
      .toLowerCase();
    return haystack.includes(searchNeedle);
  });

  const selectedListings = listings.filter((l) => selected.has(l.id));
  const allVisibleSelected = visibleListings.length > 0 && visibleListings.every((l) => selected.has(l.id));
  const toggleSelectAll = () => {
    setSelected(allVisibleSelected ? new Set() : new Set(visibleListings.map((l) => l.id)));
  };
  const toggleOne = (id: number) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const hasActiveFilters = statusFilter !== "all" || marketplaceFilter !== "" || search !== "";
  const clearFilters = () => {
    setStatusFilter("all");
    setMarketplaceFilter("");
    setSearch("");
  };

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
          disabled={tickets.length === 0 || marketplaces === null || orders === null}
          onClick={() => {
            setEditing(null);
            setFormOpen(true);
          }}
        >
          <IconPlus className="h-4 w-4" /> Add listing
        </Button>
      </div>

      {listings.length > 0 && (
        <div className="mb-3 flex flex-wrap items-end gap-3">
          <div>
            <span className="label">Status</span>
            <TabSwitcher tabs={LISTING_STATUS_TABS} active={statusFilter} onChange={setStatusFilter} />
          </div>
          <div className="w-48">
            <span className="label">Marketplace</span>
            <Select value={marketplaceFilter} onChange={(e) => setMarketplaceFilter(e.target.value ? Number(e.target.value) : "")}>
              <option value="">All marketplaces</option>
              {(marketplaces ?? []).map((m) => (
                <option key={m.id} value={m.id}>
                  {m.name}
                </option>
              ))}
            </Select>
          </div>
          <div className="w-56">
            <span className="label">Search</span>
            <div className="relative">
              <IconSearch className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-400 dark:text-slate-500" />
              <Input
                placeholder="Ticket, marketplace, listing id/URL..."
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                className="pl-9"
              />
            </div>
          </div>
        </div>
      )}

      <ListingsBulkBar selectedListings={selectedListings} onClear={() => setSelected(new Set())} onApplied={load} />

      {listings.length === 0 ? (
        <EmptyState
          title="No listings for this event yet"
          description={
            tickets.length === 0
              ? "Add a ticket to this event first, then list it on a marketplace here."
              : `Click "Add listing" to record where a ticket is posted for sale.`
          }
        />
      ) : visibleListings.length === 0 ? (
        <EmptyState
          title="No listings match these filters"
          description="Try a different status, marketplace or search term."
          action={
            <Button variant="secondary" onClick={clearFilters}>
              Clear filters
            </Button>
          }
        />
      ) : (
        <div className="overflow-x-auto rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-sm">
          <table className="w-full min-w-[820px] border-collapse">
            <thead className="border-b border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-800/60">
              <tr>
                <th className="th w-8">
                  <input
                    type="checkbox"
                    className={CHECKBOX_CLASS}
                    checked={allVisibleSelected}
                    onChange={toggleSelectAll}
                    aria-label="Select all listings"
                  />
                </th>
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
              {visibleListings.map((l) => (
                <tr key={l.id} className="hover:bg-slate-50 dark:hover:bg-slate-800/60">
                  <td className="td">
                    <input
                      type="checkbox"
                      className={CHECKBOX_CLASS}
                      checked={selected.has(l.id)}
                      onChange={() => toggleOne(l.id)}
                      aria-label={`Select listing for ${l.ticketCode}`}
                    />
                  </td>
                  <td className="td">
                    <Link
                      to={`/tickets?code=${encodeURIComponent(l.ticketCode)}`}
                      className="font-medium text-slate-900 dark:text-slate-100 hover:text-brand-700 dark:hover:text-brand-400"
                    >
                      {l.ticketCode}
                    </Link>
                    {[l.ticketSection, l.ticketRowLabel, l.ticketSeat].filter(Boolean).length > 0 && (
                      <div className="text-xs text-slate-400 dark:text-slate-500">
                        {formatSeatLocation(l.ticketSection, l.ticketRowLabel, l.ticketSeat)}
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
        eventOrders={orders ?? []}
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

// ---------------------------------------------------------------------------
// ListingsBulkBar (2.2.5) - "Bulk actions zobraz iba keď je niečo vybrané"
// (only show bulk actions while something is selected): renders nothing at
// selectedListings.length === 0. Modeled on this codebase's existing bulk
// bars (BulkTicketEditBar/BulkCompletionBar/BulkDeleteBar - see those for
// the precedent), but purpose-built here as three explicit actions (marko
// asked for these three by name) rather than BulkTicketEditBar's generic
// "pick a field" abstraction - status and price have different enough input
// shapes (a 3-way picker vs. a currency-aware amount) that one shared field
// picker would add indirection without saving anything.
// ---------------------------------------------------------------------------
function ListingsBulkBar({
  selectedListings,
  onClear,
  onApplied,
}: {
  selectedListings: TicketListing[];
  onClear: () => void;
  /** Called after any bulk action succeeds - the caller just reloads its
   * list rather than this bar trying to merge partial updates in. */
  onApplied: () => void;
}) {
  const toast = useToast();
  const [statusModalOpen, setStatusModalOpen] = useState(false);
  const [priceModalOpen, setPriceModalOpen] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [bulkStatus, setBulkStatus] = useState<"active" | "sold" | "removed">("active");
  const [bulkPrice, setBulkPrice] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (selectedListings.length === 0) return null;

  const count = selectedListings.length;
  const ids = selectedListings.map((l) => l.id);
  // Mixed-currency safety (marko's own explicit requirement): bulk price
  // edit is only offered when every selected listing already agrees on one
  // currency - see bulk_update_ticket_listings_price_impl's own doc comment
  // for why a bare number can never be applied across differing currencies.
  // The backend enforces this too (defense in depth); this is just what
  // makes the UI honest about it up front rather than erroring after a click.
  const distinctCurrencies = Array.from(new Set(selectedListings.map((l) => l.currency)));
  const uniformCurrency = distinctCurrencies.length === 1 ? distinctCurrencies[0] : null;

  const submitStatus = async () => {
    setBusy(true);
    setError(null);
    try {
      const updated = await api.bulkUpdateTicketListingsStatus({ ids, status: bulkStatus });
      toast.success(`${updated.length} listing${updated.length === 1 ? "" : "s"} marked ${bulkStatus}`);
      setStatusModalOpen(false);
      onApplied();
    } catch (e) {
      setError(errMsg(e));
    } finally {
      setBusy(false);
    }
  };

  const submitPrice = async () => {
    setError(null);
    const cents = decimalStringToCents(bulkPrice);
    if (cents === null || cents < 0) {
      setError("Enter a valid price.");
      return;
    }
    setBusy(true);
    try {
      const updated = await api.bulkUpdateTicketListingsPrice({ ids, priceCents: cents });
      toast.success(`${updated.length} listing${updated.length === 1 ? "" : "s"} updated to ${formatMoney(cents, uniformCurrency ?? "EUR")}`);
      setPriceModalOpen(false);
      onApplied();
    } catch (e) {
      setError(errMsg(e));
    } finally {
      setBusy(false);
    }
  };

  const submitDelete = async () => {
    setBusy(true);
    try {
      const deletedCount = await api.bulkDeleteTicketListings(ids);
      toast.success(`${deletedCount} listing${deletedCount === 1 ? "" : "s"} deleted`);
      setConfirmDelete(false);
      onApplied();
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      <div className="mb-4 flex flex-wrap items-center gap-3 rounded-lg bg-brand-50 dark:bg-brand-500/10 px-4 py-2.5 text-sm ring-1 ring-inset ring-brand-200 dark:ring-brand-500/30">
        <span className="font-medium text-brand-800 dark:text-brand-300">
          Selected: {count} listing{count === 1 ? "" : "s"}
        </span>
        <Button
          variant="secondary"
          onClick={() => {
            setBulkStatus("active");
            setError(null);
            setStatusModalOpen(true);
          }}
        >
          Edit status
        </Button>
        <Button
          variant="secondary"
          disabled={uniformCurrency === null}
          title={uniformCurrency === null ? "Selected listings use different currencies - narrow your selection to one currency first." : undefined}
          onClick={() => {
            setBulkPrice("");
            setError(null);
            setPriceModalOpen(true);
          }}
        >
          Edit price
        </Button>
        <Button variant="danger" onClick={() => setConfirmDelete(true)}>
          Delete
        </Button>
        <button type="button" className="ml-auto text-xs font-medium text-brand-700 dark:text-brand-400 hover:underline" onClick={onClear}>
          Clear selection
        </button>
      </div>
      {uniformCurrency === null && (
        <p className="-mt-3 mb-4 text-xs text-amber-700 dark:text-amber-400">
          Selected listings use more than one currency, so bulk price editing is unavailable for this selection.
        </p>
      )}

      <Modal
        open={statusModalOpen}
        onClose={() => setStatusModalOpen(false)}
        title={`Set status for ${count} listing${count === 1 ? "" : "s"}`}
        width="max-w-sm"
      >
        <Field label="Status">
          <div className="flex rounded-lg border border-slate-200 dark:border-slate-800 p-1">
            {(["active", "sold", "removed"] as const).map((s) => (
              <button
                key={s}
                type="button"
                onClick={() => setBulkStatus(s)}
                className={`flex-1 rounded-md px-2.5 py-1.5 text-xs font-medium capitalize transition-colors ${
                  bulkStatus === s ? "bg-brand-600 text-white" : "text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800"
                }`}
              >
                {s}
              </button>
            ))}
          </div>
        </Field>
        {error && <p className="mt-3 text-xs text-red-600 dark:text-red-400">{error}</p>}
        <ModalFooter>
          <Button variant="secondary" onClick={() => setStatusModalOpen(false)} disabled={busy}>
            Cancel
          </Button>
          <Button variant="primary" onClick={submitStatus} disabled={busy}>
            {busy ? <Spinner className="h-4 w-4" /> : null}
            Apply to {count}
          </Button>
        </ModalFooter>
      </Modal>

      <Modal
        open={priceModalOpen}
        onClose={() => setPriceModalOpen(false)}
        title={`Set price for ${count} listing${count === 1 ? "" : "s"}`}
        width="max-w-sm"
      >
        <Field label={`New price${uniformCurrency ? ` (${uniformCurrency})` : ""}`} required>
          <Input autoFocus inputMode="decimal" placeholder="0.00" value={bulkPrice} onChange={(e) => setBulkPrice(e.target.value)} />
        </Field>
        <p className="mt-2 text-xs text-slate-400 dark:text-slate-500">
          Currency stays {uniformCurrency} for every selected listing - only the amount changes.
        </p>
        {error && <p className="mt-3 text-xs text-red-600 dark:text-red-400">{error}</p>}
        <ModalFooter>
          <Button variant="secondary" onClick={() => setPriceModalOpen(false)} disabled={busy}>
            Cancel
          </Button>
          <Button variant="primary" onClick={submitPrice} disabled={busy}>
            {busy ? <Spinner className="h-4 w-4" /> : null}
            Apply to {count}
          </Button>
        </ModalFooter>
      </Modal>

      <ConfirmDialog
        open={confirmDelete}
        title={`Delete ${count} selected listing${count === 1 ? "" : "s"}?`}
        message="This cannot be undone."
        confirmLabel="Delete selected"
        danger
        busy={busy}
        onCancel={() => setConfirmDelete(false)}
        onConfirm={submitDelete}
      />
    </>
  );
}

// One (ticket, marketplace) listing's create/edit form. Editing a listing
// shows its ticket as plain text - same "round-trip a field the form
// doesn't expose" spirit as Transactions.tsx's own order-linked entries
// (there is no UI anywhere to re-parent a listing to a different ticket).
//
// 2.2.5: marko's own complaint about CREATING a listing - "teraz to je
// uplne nepriehladne" (right now it's completely opaque), a flat dropdown
// of every ticket in the event with no context - fixed by mirroring
// Sales.tsx's own New Sale flow: browse this event's orders (searchable),
// open one, pick tickets from just that order, repeat across as many
// orders as needed, then fill in the shared details. Unlike New Sale, this
// needs no live fetch to do it - `eventTickets`/`eventOrders` are already
// this whole event's own tickets/orders (small, already loaded by
// ListingsTab), so the picker below is pure client-side filtering, not a
// second round trip to the backend.
//
// "vybrat dany pocet listkov" (pick a given number of tickets) is taken
// literally - marko can select several tickets at once here, same as New
// Sale, and this then creates one listing per selected ticket on the one
// chosen marketplace (marketplace/currency/status shared; price defaults
// from a "Quick-fill" but stays editable per ticket, exact same UX as New
// Sale's own price/fees grid). Listing ID/URL are each marketplace
// posting's OWN external identifier, so a shared value across several
// tickets would be meaningless - offered only when exactly one ticket is
// selected; for a batch, add them afterward via Edit on each created
// listing. Creating a batch is NOT all-or-nothing (that stricter guarantee
// is reserved for the bulk actions on EXISTING listings, per marko's own
// explicit wording this release) - a failure partway through still keeps
// whatever succeeded and reports exactly what didn't, letting the ticket
// picker retry just the failures.
function TicketListingFormModal({
  open,
  initial,
  eventTickets,
  eventOrders,
  marketplaces,
  onClose,
  onSaved,
}: {
  open: boolean;
  initial: TicketListing | null;
  eventTickets: Ticket[];
  eventOrders: OrderRecord[];
  marketplaces: Marketplace[];
  onClose: () => void;
  onSaved: () => void;
}) {
  const toast = useToast();
  const [marketplaceId, setMarketplaceId] = useState("");
  const [listingIdText, setListingIdText] = useState("");
  const [listingUrl, setListingUrl] = useState("");
  const [price, setPrice] = useState(""); // edit mode only
  const [currency, setCurrency] = useState("EUR");
  const [status, setStatus] = useState<"active" | "sold" | "removed">("active");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Create-mode ticket picker (see this component's own doc comment above).
  const [pickerStep, setPickerStep] = useState<"pick" | "details">("pick");
  const [orderQuery, setOrderQuery] = useState("");
  const [activeOrder, setActiveOrder] = useState<OrderRecord | null>(null);
  const [selectedTickets, setSelectedTickets] = useState<Ticket[]>([]);
  const [prices, setPrices] = useState<Record<number, string>>({});
  const [bulkPriceQuickFill, setBulkPriceQuickFill] = useState("");
  // Same "don't stomp a deliberate manual choice on a later recompute"
  // guard as Sales.tsx's own `currencyTouched` - marko can go "+ Change
  // tickets" back to picking, add another ticket, and return to details
  // without losing a currency he already picked by hand.
  const currencyTouched = useRef(false);

  useEffect(() => {
    if (!open) return;
    setError(null);
    if (initial) {
      setMarketplaceId(String(initial.marketplaceId));
      setListingIdText(initial.listingId ?? "");
      setListingUrl(initial.listingUrl ?? "");
      setPrice(centsToDecimalString(initial.priceCents));
      setCurrency(initial.currency);
      setStatus(initial.status);
      return;
    }
    setPickerStep("pick");
    setOrderQuery("");
    setActiveOrder(null);
    setSelectedTickets([]);
    setPrices({});
    setBulkPriceQuickFill("");
    setMarketplaceId("");
    setListingIdText("");
    setListingUrl("");
    setCurrency("EUR");
    setStatus("active");
    currencyTouched.current = false;
  }, [open, initial]);

  const ticketLabel = (t: Ticket) => {
    const hasSeat = [t.section, t.rowLabel, t.seat].some(Boolean);
    return hasSeat ? `${t.code} (${formatSeatLocation(t.section, t.rowLabel, t.seat)})` : t.code;
  };

  const ticketsByOrder = useMemo(() => {
    const map = new Map<number, Ticket[]>();
    for (const t of eventTickets) {
      const list = map.get(t.orderId) ?? [];
      list.push(t);
      map.set(t.orderId, list);
    }
    return map;
  }, [eventTickets]);

  const orderQueryNeedle = orderQuery.trim().toLowerCase();
  const orderOptions = eventOrders.filter((o) => {
    const orderTickets = ticketsByOrder.get(o.id) ?? [];
    if (orderTickets.length === 0) return false;
    if (orderQueryNeedle === "") return true;
    const haystack = [o.code, o.platformName, ...orderTickets.map((t) => t.code)].filter(Boolean).join(" ").toLowerCase();
    return haystack.includes(orderQueryNeedle);
  });
  const visibleOrderTickets = activeOrder
    ? (ticketsByOrder.get(activeOrder.id) ?? []).filter((t) => !selectedTickets.some((s) => s.id === t.id))
    : [];

  const addTicket = (t: Ticket) => {
    setSelectedTickets((prev) => (prev.some((s) => s.id === t.id) ? prev : [...prev, t]));
    setPrices((prev) => (prev[t.id] !== undefined ? prev : { ...prev, [t.id]: "" }));
  };
  const removeTicket = (id: number) => {
    setSelectedTickets((prev) => prev.filter((t) => t.id !== id));
    setPrices((prev) => {
      const next = { ...prev };
      delete next[id];
      return next;
    });
  };

  // Same default-currency reasoning as Sales.tsx's own `goToDetails`: the
  // selection's own uniform purchase currency if they all agree, else EUR -
  // always still freely editable afterward.
  const goToDetails = () => {
    if (!currencyTouched.current) {
      const first = selectedTickets[0]?.currency;
      const uniform = first && selectedTickets.every((t) => t.currency === first) ? first : "EUR";
      setCurrency(uniform);
    }
    setError(null);
    setPickerStep("details");
  };

  const applyBulkPriceToAll = () => {
    if (!bulkPriceQuickFill.trim()) return;
    setPrices((prev) => {
      const next = { ...prev };
      for (const t of selectedTickets) next[t.id] = bulkPriceQuickFill;
      return next;
    });
  };

  const submitEdit = async () => {
    if (!initial) return;
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
      ticketId: initial.ticketId,
      marketplaceId: Number(marketplaceId),
      listingId: listingIdText.trim() || null,
      listingUrl: listingUrl.trim() || null,
      priceCents: cents,
      currency,
      status,
    };
    try {
      await api.updateTicketListing(initial.id, input);
      toast.success("Listing updated.");
      onSaved();
      onClose();
    } catch (e) {
      setError(errMsg(e));
    } finally {
      setSaving(false);
    }
  };

  // Not all-or-nothing by design - see this component's own doc comment for
  // why that stricter guarantee is reserved for the bulk actions on
  // EXISTING listings instead.
  const submitCreateBatch = async () => {
    if (!marketplaceId) {
      setError("Pick a marketplace.");
      return;
    }
    if (selectedTickets.length === 0) {
      setError("Pick at least one ticket.");
      return;
    }
    const parsed: { ticket: Ticket; cents: number }[] = [];
    for (const t of selectedTickets) {
      const cents = decimalStringToCents(prices[t.id] ?? "");
      if (cents === null || cents < 0) {
        setError(`Enter a valid price for ${t.code}.`);
        return;
      }
      parsed.push({ ticket: t, cents });
    }
    setSaving(true);
    setError(null);
    const singleTicket = selectedTickets.length === 1;
    const succeededIds: number[] = [];
    const failed: { code: string; reason: string }[] = [];
    for (const { ticket, cents } of parsed) {
      const input: TicketListingInput = {
        ticketId: ticket.id,
        marketplaceId: Number(marketplaceId),
        listingId: singleTicket ? listingIdText.trim() || null : null,
        listingUrl: singleTicket ? listingUrl.trim() || null : null,
        priceCents: cents,
        currency,
        status,
      };
      try {
        await api.createTicketListing(input);
        succeededIds.push(ticket.id);
      } catch (e) {
        failed.push({ code: ticket.code, reason: errMsg(e) });
      }
    }
    setSaving(false);
    if (succeededIds.length > 0) onSaved();
    if (failed.length === 0) {
      toast.success(`${succeededIds.length} listing${succeededIds.length === 1 ? "" : "s"} added.`);
      onClose();
    } else {
      // Keep only the failures selected so marko can fix and retry them
      // without re-picking everything that already succeeded.
      setSelectedTickets((prev) => prev.filter((t) => !succeededIds.includes(t.id)));
      setError(
        `${succeededIds.length} of ${parsed.length} listing${parsed.length === 1 ? "" : "s"} added. Failed: ${failed
          .map((f) => `${f.code} (${f.reason})`)
          .join("; ")}`,
      );
    }
  };

  const showPicker = !initial && pickerStep === "pick";

  return (
    <Modal open={open} onClose={onClose} title={initial ? "Edit listing" : "Add listing"} width={initial ? undefined : "max-w-2xl"}>
      {showPicker ? (
        <div>
          {!activeOrder ? (
            <>
              <Field label="Find an order to list tickets from" required hint="Open an order, then pick which of its tickets to list.">
                <Input
                  autoFocus
                  placeholder="Search by order code, platform, ticket code..."
                  value={orderQuery}
                  onChange={(e) => setOrderQuery(e.target.value)}
                />
              </Field>
              <div className="mt-3 max-h-64 divide-y divide-slate-100 dark:divide-slate-800 overflow-y-auto rounded-lg border border-slate-200 dark:border-slate-800">
                {orderOptions.length === 0 ? (
                  <p className="p-4 text-center text-sm text-slate-400 dark:text-slate-500">
                    {eventOrders.length === 0
                      ? "This event has no orders yet"
                      : orderQuery
                        ? "No matching orders with tickets"
                        : "Start typing to search this event's orders"}
                  </p>
                ) : (
                  orderOptions.map((o) => {
                    const ticketCount = (ticketsByOrder.get(o.id) ?? []).length;
                    return (
                      <button
                        key={o.id}
                        type="button"
                        className="flex w-full items-center justify-between gap-3 px-3 py-2 text-left hover:bg-slate-50 dark:hover:bg-slate-800/60"
                        onClick={() => setActiveOrder(o)}
                      >
                        <span className="min-w-0">
                          <span className="block truncate text-sm font-medium text-slate-800 dark:text-slate-200">{o.code}</span>
                          <span className="block truncate text-xs text-slate-400 dark:text-slate-500">
                            {o.platformName ?? "No platform"} · {formatDate(o.purchaseDate)}
                          </span>
                        </span>
                        <span className="flex shrink-0 items-center gap-2">
                          <span className="whitespace-nowrap rounded-full bg-slate-100 dark:bg-slate-800 px-2 py-0.5 text-xs font-medium text-slate-600 dark:text-slate-300">
                            {ticketCount} ticket{ticketCount === 1 ? "" : "s"}
                          </span>
                          <IconChevronDown className="h-4 w-4 -rotate-90 text-slate-400 dark:text-slate-500" />
                        </span>
                      </button>
                    );
                  })
                )}
              </div>
            </>
          ) : (
            <>
              <div className="mb-2 flex items-center justify-between gap-2">
                <button
                  type="button"
                  className="inline-flex shrink-0 items-center gap-1 text-xs font-medium text-brand-600 dark:text-brand-400 hover:underline"
                  onClick={() => setActiveOrder(null)}
                >
                  <IconArrowLeft className="h-3.5 w-3.5" /> Back to orders
                </button>
                <span className="min-w-0 truncate text-xs text-slate-400 dark:text-slate-500">{activeOrder.code}</span>
              </div>
              <div className="max-h-64 divide-y divide-slate-100 dark:divide-slate-800 overflow-y-auto rounded-lg border border-slate-200 dark:border-slate-800">
                {visibleOrderTickets.length === 0 ? (
                  <p className="p-4 text-center text-sm text-slate-400 dark:text-slate-500">Every ticket from this order is already selected</p>
                ) : (
                  visibleOrderTickets.map((t) => (
                    <button
                      key={t.id}
                      type="button"
                      className="flex w-full items-center justify-between gap-3 px-3 py-2 text-left hover:bg-slate-50 dark:hover:bg-slate-800/60"
                      onClick={() => addTicket(t)}
                    >
                      <span className="min-w-0">
                        <span className="block truncate text-sm font-medium text-slate-800 dark:text-slate-200">{t.code}</span>
                        <span className="block truncate text-xs text-slate-400 dark:text-slate-500">
                          {formatSeatLocation(t.section, t.rowLabel, t.seat)}
                        </span>
                      </span>
                      <span className="flex shrink-0 items-center gap-2">
                        <Badge tone={t.status}>{t.status}</Badge>
                        <IconPlus className="h-4 w-4 text-brand-600 dark:text-brand-400" />
                      </span>
                    </button>
                  ))
                )}
              </div>
            </>
          )}

          {selectedTickets.length > 0 && (
            <div className="mt-4">
              <p className="label mb-1.5">Selected ({selectedTickets.length})</p>
              <div className="flex flex-wrap gap-1.5">
                {selectedTickets.map((t) => (
                  <span
                    key={t.id}
                    className="inline-flex items-center gap-1 rounded-full bg-brand-50 dark:bg-brand-500/10 py-1 pl-2.5 pr-1.5 text-xs font-medium text-brand-700 dark:text-brand-400 ring-1 ring-inset ring-brand-200 dark:ring-brand-500/30"
                  >
                    {t.code}
                    <button
                      type="button"
                      onClick={() => removeTicket(t.id)}
                      className="rounded-full p-0.5 hover:bg-brand-100 dark:hover:bg-brand-500/20"
                      aria-label={`Remove ${t.code}`}
                    >
                      <IconX className="h-3 w-3" />
                    </button>
                  </span>
                ))}
              </div>
            </div>
          )}

          {error && <p className="mt-3 text-xs text-red-600 dark:text-red-400">{error}</p>}
        </div>
      ) : (
        <div className="space-y-3">
          {!initial && (
            <div className="mb-1 flex items-center justify-between">
              <p className="label mb-0">
                Listing {selectedTickets.length} ticket{selectedTickets.length === 1 ? "" : "s"}
              </p>
              <button type="button" className="text-xs font-medium text-brand-600 dark:text-brand-400 hover:underline" onClick={() => setPickerStep("pick")}>
                + Change tickets
              </button>
            </div>
          )}

          <Field label="Ticket" required>
            {initial ? (
              <p className="rounded-lg bg-slate-50 px-3 py-2 text-sm text-slate-600 dark:bg-slate-800/60 dark:text-slate-300">
                {initial.ticketCode}
                {[initial.ticketSection, initial.ticketRowLabel, initial.ticketSeat].filter(Boolean).length > 0 && (
                  <span className="text-slate-400 dark:text-slate-500">
                    {" "}
                    ({formatSeatLocation(initial.ticketSection, initial.ticketRowLabel, initial.ticketSeat)})
                  </span>
                )}
              </p>
            ) : (
              <div className="flex flex-wrap gap-x-3 gap-y-1 rounded-lg bg-slate-50 px-3 py-2 dark:bg-slate-800/60">
                {selectedTickets.map((t) => (
                  <span key={t.id} className="text-sm text-slate-600 dark:text-slate-300">
                    {ticketLabel(t)}
                  </span>
                ))}
              </div>
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

          {!initial && selectedTickets.length > 1 ? (
            <>
              <div className="flex flex-wrap items-end gap-2 rounded-lg bg-slate-50 dark:bg-slate-800/60 p-3">
                <div className="w-20">
                  <span className="label">Currency</span>
                  <Select
                    value={currency}
                    onChange={(e) => {
                      currencyTouched.current = true;
                      setCurrency(e.target.value);
                    }}
                  >
                    {(CURRENCIES.includes(currency) ? CURRENCIES : [currency, ...CURRENCIES]).map((c) => (
                      <option key={c} value={c}>
                        {c}
                      </option>
                    ))}
                  </Select>
                </div>
                <div className="w-28">
                  <span className="label">Quick-fill price</span>
                  <Input inputMode="decimal" placeholder="0.00" value={bulkPriceQuickFill} onChange={(e) => setBulkPriceQuickFill(e.target.value)} />
                </div>
                <Button type="button" variant="secondary" disabled={!bulkPriceQuickFill.trim()} onClick={applyBulkPriceToAll}>
                  Apply to all
                </Button>
                <p className="w-full text-xs text-slate-400 dark:text-slate-500">
                  Applying overwrites any price already entered below for every selected ticket.
                </p>
              </div>

              <div className="max-h-52 divide-y divide-slate-100 dark:divide-slate-800 overflow-y-auto rounded-lg border border-slate-200 dark:border-slate-800">
                {selectedTickets.map((t) => (
                  <div key={t.id} className="flex items-center gap-2 px-3 py-2">
                    <div className="min-w-0 flex-1">
                      <p className="truncate text-sm font-medium text-slate-800 dark:text-slate-200">{t.code}</p>
                      <p className="truncate text-xs text-slate-400 dark:text-slate-500">
                        {formatSeatLocation(t.section, t.rowLabel, t.seat)}
                      </p>
                    </div>
                    <span className="w-9 shrink-0 text-center text-xs font-medium text-slate-400 dark:text-slate-500">{currency}</span>
                    <div className="w-24 shrink-0">
                      <Input
                        inputMode="decimal"
                        placeholder="0.00"
                        value={prices[t.id] ?? ""}
                        onChange={(e) => setPrices((prev) => ({ ...prev, [t.id]: e.target.value }))}
                      />
                    </div>
                  </div>
                ))}
              </div>
            </>
          ) : (
            <div className="grid grid-cols-[1fr_110px] gap-2">
              <Field label="Price" required>
                <Input
                  inputMode="decimal"
                  placeholder="0.00"
                  value={initial ? price : (prices[selectedTickets[0]?.id] ?? "")}
                  onChange={(e) =>
                    initial ? setPrice(e.target.value) : setPrices((prev) => ({ ...prev, [selectedTickets[0]?.id]: e.target.value }))
                  }
                />
              </Field>
              <Field label="Currency">
                <Select
                  value={currency}
                  onChange={(e) => {
                    currencyTouched.current = true;
                    setCurrency(e.target.value);
                  }}
                >
                  {(CURRENCIES.includes(currency) ? CURRENCIES : [currency, ...CURRENCIES]).map((c) => (
                    <option key={c} value={c}>
                      {c}
                    </option>
                  ))}
                </Select>
              </Field>
            </div>
          )}

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

          {(initial || selectedTickets.length === 1) && (
            <>
              <Field label="Listing ID" hint="The marketplace's own id for this listing, if you have one.">
                <Input value={listingIdText} onChange={(e) => setListingIdText(e.target.value)} />
              </Field>
              <Field label="Listing URL">
                <Input type="url" placeholder="https://..." value={listingUrl} onChange={(e) => setListingUrl(e.target.value)} />
              </Field>
            </>
          )}
          {!initial && selectedTickets.length > 1 && (
            <p className="text-xs text-slate-400 dark:text-slate-500">
              Listing ID/URL aren&apos;t set here for a multi-ticket batch - each marketplace posting has its own, so add them
              afterward by editing each created listing.
            </p>
          )}

          {error && <p className="text-xs text-red-600 dark:text-red-400">{error}</p>}
        </div>
      )}

      <ModalFooter>
        {showPicker ? (
          <>
            <Button variant="secondary" onClick={onClose}>
              Cancel
            </Button>
            <Button variant="primary" disabled={selectedTickets.length === 0} onClick={goToDetails}>
              Continue with {selectedTickets.length} ticket{selectedTickets.length === 1 ? "" : "s"}
            </Button>
          </>
        ) : (
          <>
            <Button variant="secondary" onClick={onClose} disabled={saving}>
              Cancel
            </Button>
            <Button variant="primary" onClick={initial ? submitEdit : submitCreateBatch} disabled={saving}>
              {saving ? <Spinner className="h-4 w-4" /> : null}
              {initial ? "Save changes" : `Add ${selectedTickets.length} listing${selectedTickets.length === 1 ? "" : "s"}`}
            </Button>
          </>
        )}
      </ModalFooter>
    </Modal>
  );
}

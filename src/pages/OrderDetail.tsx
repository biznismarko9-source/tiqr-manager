import { useCallback, useEffect, useState } from "react";
import { Link, useLocation, useNavigate, useParams } from "react-router-dom";
import { api, errMsg } from "../lib/api";
import type {
  OrderEditInput,
  OrderPaymentStatus,
  OrderRecord,
  OrderSalesSummary,
  Platform,
  PullReceived,
  Ticket,
} from "../lib/types";
import { decimalStringToCents, formatDate, formatDateNumeric, formatMoney, formatSeatLocation } from "../lib/format";
import {
  Badge,
  Button,
  CHECKBOX_CLASS,
  Card,
  ConfirmDialog,
  EmptyState,
  Field,
  Input,
  LoadingBlock,
  Modal,
  ModalFooter,
  Select,
  Textarea,
} from "../components/ui";
import { LookupSelect } from "../components/LookupSelect";
import { IconArrowLeft, IconLink, IconPencil, IconPlus, IconTrash } from "../components/icons";
import { useToast } from "../lib/toast";
import { useNarrowTables } from "../lib/useNarrowTables";
import { TicketEditModal } from "./Tickets";
import { PullReceivedFormModal } from "./Pulls";

export default function OrderDetail() {
  const { id } = useParams();
  const orderId = Number(id);
  const navigate = useNavigate();
  const location = useLocation();
  const toast = useToast();
  const isNarrow = useNarrowTables();

  // 1.8.3 (section 8): if the user arrived from Orders - which passes
  // state={{ from: location.pathname }} on its link into Order Detail (see
  // Orders.tsx) - Back returns to that exact page (which itself remembers
  // its last search, see lastOrdersSearch) instead of always landing on the
  // plain Orders list. Allowlisted rather than trusting state.from blindly,
  // and falls back to the pre-1.8.3 default when absent (e.g. a direct link
  // or a page refresh).
  // 1.9.1 removed Tickets/Inventory's links into this page entirely; 1.9.2
  // (Inventory) and 1.9.3 (Tickets) brought them back, and 1.9.5 made the
  // Order-code link on both unconditional - so in practice all three
  // (Orders, Tickets, Inventory) are live entry points again, not just a
  // fallback-labeling relic.
  const cameFrom = (location.state as { from?: string } | null)?.from;
  const backTo = cameFrom && ["/tickets", "/inventory", "/orders"].includes(cameFrom) ? cameFrom : "/orders";
  const backLabel = backTo === "/tickets" ? "Back to tickets" : backTo === "/inventory" ? "Back to inventory" : "Back to orders";
  // 1.9.6: marko clarified what he meant by wanting Tickets/Inventory to
  // behave like Event/Order/Sale's own click-through ("more info about that
  // object, not thrown elsewhere") - landing here still FEELS like being
  // thrown to a different section ("Order") even though the data shown
  // (this order's tickets) genuinely is what a Tickets/Inventory row's own
  // detail view would show. There's no separate underlying data to show -
  // the tickets ARE the order's tickets - so instead of a duplicate page,
  // this eyebrow label reframes the same page contextually: arriving from
  // Tickets/Inventory reads as "Ticket detail"/"Inventory detail", arriving
  // from Orders (or a direct link/refresh) reads as "Order detail". The
  // order code stays as the heading either way - it's still the one
  // genuinely unique identifier for what's on this page.
  const detailLabel = backTo === "/tickets" ? "Ticket detail" : backTo === "/inventory" ? "Inventory detail" : "Order detail";

  const [order, setOrder] = useState<OrderRecord | null>(null);
  const [tickets, setTickets] = useState<Ticket[] | null>(null);
  const [salesSummary, setSalesSummary] = useState<OrderSalesSummary | null>(null);
  const [editOpen, setEditOpen] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [deleting, setDeleting] = useState(false);
  // 2.0.51: "Convert to EUR" next to the Currency field, for an order
  // already created in another currency (manually, via CSV, or via Sheets
  // sync - see api.convertOrderCurrency's own doc comment).
  const [convertOpen, setConvertOpen] = useState(false);
  const [converting, setConverting] = useState(false);
  const [editTicket, setEditTicket] = useState<Ticket | null>(null);
  // 1.8.3: bulk ticket actions.
  const [selected, setSelected] = useState<Set<number>>(new Set());
  // 2.0.24: "Received pulls" section - see that section's own comment below.
  const [pullsReceived, setPullsReceived] = useState<PullReceived[] | null>(null);
  const [addPullOpen, setAddPullOpen] = useState(false);
  const [editPull, setEditPull] = useState<PullReceived | null>(null);
  const [deletePullTarget, setDeletePullTarget] = useState<PullReceived | null>(null);
  const [deletingPull, setDeletingPull] = useState(false);

  const loadPullsReceived = useCallback(() => {
    api.listPullsReceivedForOrder(orderId).then(setPullsReceived).catch((e) => toast.error(errMsg(e)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [orderId]);

  const load = useCallback(() => {
    // Every reload (mount, edit, delete, or a bulk edit just applied) starts
    // from a clean selection - same reasoning as Sale Detail's own load().
    setSelected(new Set());
    api.getOrder(orderId).then(setOrder).catch((e) => toast.error(errMsg(e)));
    api
      .listTickets({ orderId, sortBy: "code", sortDir: "asc" })
      .then(setTickets)
      .catch((e) => toast.error(errMsg(e)));
    api.getOrderSalesSummary(orderId).then(setSalesSummary).catch((e) => toast.error(errMsg(e)));
    loadPullsReceived();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [orderId]);

  useEffect(() => {
    load();
  }, [load]);

  if (!order) return <LoadingBlock />;

  // 1.8.3: bulk ticket actions. Selection itself stays unrestricted - any
  // ticket, sold or not, can be checked - but 1.9.3 narrowed what applying
  // the selection actually does: it's now a status-only action
  // (TicketStatusBar below) that rejects the whole batch if it contains a
  // sold ticket, rather than the old Section/Row/Seat/Listing-price editor
  // that had no such restriction. See bulk_update_ticket_status_impl's doc
  // comment (tickets.rs) for why sold tickets are excluded.
  const allSelected = tickets !== null && tickets.length > 0 && tickets.every((t) => selected.has(t.id));
  const toggleSelectAll = () => {
    if (!tickets) return;
    setSelected(allSelected ? new Set() : new Set(tickets.map((t) => t.id)));
  };
  const toggleOne = (id: number) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  // 1.9.0 (section 5, "Order Detail"): Paid/Outstanding, derived honestly
  // from the existing order.paymentStatus label. Unlike a Sale, an Order has
  // no numeric "amount paid so far" field anywhere - payment state here is a
  // pure status, not a per-line amount - so 'unpaid'/'paid' map to a hard
  // 0/total (unambiguous), but 'partial' genuinely has no backing number to
  // show. Rather than guess or fabricate a split, that case is shown as
  // "Partial" text with no invented cents value - see the 1.9.0 report.
  const orderPaidCents =
    order.paymentStatus === "paid" ? order.totalCostCents : order.paymentStatus === "unpaid" ? 0 : null;
  const orderOutstandingCents =
    order.paymentStatus === "paid" ? 0 : order.paymentStatus === "unpaid" ? order.totalCostCents : null;

  return (
    <div>
      <Link to={backTo} className="mb-3 inline-flex items-center gap-1 text-sm text-slate-500 dark:text-slate-400 hover:text-slate-800 dark:hover:text-slate-200">
        <IconArrowLeft className="h-4 w-4" /> {backLabel}
      </Link>

      <div className="mb-5 flex flex-wrap items-start justify-between gap-3">
        <div>
          <p className="text-xs font-medium uppercase text-slate-400 dark:text-slate-500">{detailLabel}</p>
          <div className="flex items-center gap-2">
            <h1 className="text-xl font-semibold text-slate-900 dark:text-slate-100">{order.code}</h1>
            <Badge tone={order.paymentStatus}>{order.paymentStatus}</Badge>
          </div>
          {/* 1.9.1: the event name used to be a <Link> to Event Detail -
              removed per marko's request to stop every "this reference jumps
              me to a different section" link in Orders/Tickets/Sales. */}
          <p className="mt-1 text-sm text-slate-500 dark:text-slate-400">
            {order.eventName}
            {" "}&middot; purchased {formatDate(order.purchaseDate)}
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

      <div className="mb-8 grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-6">
        <Card className="p-4">
          <p className="text-xs font-medium uppercase text-slate-400 dark:text-slate-500">Quantity</p>
          <p className="mt-1 text-lg font-semibold">{order.quantity}</p>
        </Card>
        <Card className="p-4">
          <p className="text-xs font-medium uppercase text-slate-400 dark:text-slate-500">Unit price</p>
          <p className="mt-1 text-lg font-semibold">{formatMoney(order.unitPriceCents, order.currency)}</p>
        </Card>
        <Card className="p-4">
          <p className="text-xs font-medium uppercase text-slate-400 dark:text-slate-500">Fees + other</p>
          <p className="mt-1 text-lg font-semibold">
            {formatMoney(order.feesCents + order.otherCostsCents, order.currency)}
          </p>
        </Card>
        <Card className="p-4">
          <p className="text-xs font-medium uppercase text-slate-400 dark:text-slate-500">Total cost</p>
          <p className="mt-1 text-lg font-semibold">{formatMoney(order.totalCostCents, order.currency)}</p>
        </Card>
        {/* 1.9.0 (section 5): Paid/Outstanding - see the orderPaidCents/
            orderOutstandingCents derivation above. 'partial' shows as text,
            never a fabricated number - the order model has no field to back
            one (see the 1.9.0 report's audit). */}
        <Card className="p-4">
          <p className="text-xs font-medium uppercase text-slate-400 dark:text-slate-500">Paid</p>
          <p className={`mt-1 text-lg font-semibold ${orderPaidCents === null ? "text-amber-600 dark:text-amber-400" : ""}`}>
            {orderPaidCents !== null ? formatMoney(orderPaidCents, order.currency) : "Partial"}
          </p>
          {orderPaidCents === null && (
            <p className="mt-1 text-xs text-slate-400 dark:text-slate-500">Exact amount not tracked</p>
          )}
        </Card>
        <Card className="p-4">
          <p className="text-xs font-medium uppercase text-slate-400 dark:text-slate-500">Outstanding</p>
          <p
            className={`mt-1 text-lg font-semibold ${orderOutstandingCents === null ? "text-amber-600 dark:text-amber-400" : ""}`}
          >
            {orderOutstandingCents !== null ? formatMoney(orderOutstandingCents, order.currency) : "Partial"}
          </p>
          {orderOutstandingCents === null && (
            <p className="mt-1 text-xs text-slate-400 dark:text-slate-500">Exact amount not tracked</p>
          )}
        </Card>
      </div>

      {/* 1.9.10: marko wanted Platform/Notes/Currency combined into one row
          in that order - was Platform+Currency on one row with Notes below
          as its own full-width, conditionally-shown block. Notes now
          always renders (with a "-" fallback) to match how Platform/
          Currency already behave, rather than sometimes leaving this row
          with only 2 of 3 cells filled. Long notes still wrap
          (whitespace-pre-wrap) instead of truncating - just inside a
          narrower cell than before. */}
      <Card className="mb-8 grid grid-cols-2 gap-4 p-4 sm:grid-cols-3">
        <div>
          <p className="text-xs font-medium uppercase text-slate-400 dark:text-slate-500">Platform</p>
          <p className="mt-1 text-sm text-slate-700 dark:text-slate-300">{order.platformName ?? "-"}</p>
        </div>
        <div>
          <p className="text-xs font-medium uppercase text-slate-400 dark:text-slate-500">Notes</p>
          <p className="mt-1 whitespace-pre-wrap text-sm text-slate-700 dark:text-slate-300">{order.notes || "-"}</p>
        </div>
        <div>
          <p className="text-xs font-medium uppercase text-slate-400 dark:text-slate-500">Currency</p>
          <div className="mt-1 flex items-center gap-2">
            <p className="text-sm text-slate-700 dark:text-slate-300">{order.currency}</p>
            {/* 2.0.51: marko's own follow-up to 2.0.50 - that version only
                let a NEW order's currency be converted at creation time; this
                lets an already-created order (manual, CSV, or Sheets sync)
                be converted too, for any currency the order might be in, not
                just GBP. Only shown once there's actually something to
                convert. */}
            {order.currency !== "EUR" && (
              <button
                type="button"
                className="text-xs font-medium text-brand-600 dark:text-brand-400 hover:underline"
                onClick={() => setConvertOpen(true)}
              >
                Convert to EUR
              </button>
            )}
          </div>
        </div>
      </Card>

      {/* 2.0.24: marko's own request - fill in who pulled this order (and
          for how much) right here, instead of only via the connected Google
          Sheet (orders_sheet_sync::maybe_link_pull_received) or by leaving
          this page to search for the order on the Pulls screen. A list, not
          a single optional slot - nothing stops more than one linked row per
          order (see api.listPullsReceivedForOrder's own doc comment), and
          most orders have none at all, which is why this card always shows
          but the "no pulls" state is a plain one-line sentence, not an
          EmptyState block. */}
      <Card className="mb-8 p-4">
        <div className="mb-3 flex items-center justify-between gap-3">
          <h2 className="text-sm font-semibold text-slate-800 dark:text-slate-200">
            Received pulls{pullsReceived && pullsReceived.length > 0 ? ` (${pullsReceived.length})` : ""}
          </h2>
          <Button variant="secondary" onClick={() => setAddPullOpen(true)}>
            <IconPlus className="h-4 w-4" /> Add pull info
          </Button>
        </div>
        {pullsReceived === null ? (
          <LoadingBlock />
        ) : pullsReceived.length === 0 ? (
          <p className="text-sm text-slate-400 dark:text-slate-500">
            Nobody pulled this order for you yet - or it just hasn&apos;t been recorded.
          </p>
        ) : (
          <div className="flex flex-col gap-2">
            {pullsReceived.map((p) => (
              <button
                key={p.id}
                type="button"
                className="flex items-center justify-between gap-3 rounded-lg border border-slate-200 px-3 py-2 text-left hover:bg-slate-50 dark:border-slate-800 dark:hover:bg-slate-800/60"
                onClick={() => setEditPull(p)}
              >
                <span className="flex min-w-0 items-center gap-2">
                  <IconLink className="h-4 w-4 shrink-0 text-slate-400 dark:text-slate-500" />
                  <span className="truncate text-sm font-medium text-slate-800 dark:text-slate-200">{p.pullerName}</span>
                  {p.source === "sheet_sync" && <Badge tone="synced">Synced</Badge>}
                </span>
                <span className="flex shrink-0 items-center gap-3 text-sm text-slate-500 dark:text-slate-400">
                  <span className="tabular-nums">{p.quantity}&times;</span>
                  <span className="tabular-nums">{formatMoney(p.amountCents, p.currency)}</span>
                  <IconPencil className="h-3.5 w-3.5 text-slate-400 dark:text-slate-500" />
                </span>
              </button>
            ))}
          </div>
        )}
      </Card>

      <h2 className="mb-3 text-sm font-semibold text-slate-800 dark:text-slate-200">Order summary</h2>
      <div className="mb-8 grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-6">
        <Card className="p-4">
          <p className="text-xs font-medium uppercase text-slate-400 dark:text-slate-500">Total tickets</p>
          <p className="mt-1 text-lg font-semibold">{order.quantity}</p>
        </Card>
        <Card className="p-4">
          <p className="text-xs font-medium uppercase text-slate-400 dark:text-slate-500">Available</p>
          <p className="mt-1 text-lg font-semibold">{order.availableCount + order.listedCount}</p>
        </Card>
        <Card className="p-4">
          <p className="text-xs font-medium uppercase text-slate-400 dark:text-slate-500">Sold</p>
          <p className="mt-1 text-lg font-semibold">{order.soldCount}</p>
        </Card>
        <Card className="p-4">
          <p className="text-xs font-medium uppercase text-slate-400 dark:text-slate-500">Purchase cost</p>
          <p className="mt-1 text-lg font-semibold">{formatMoney(order.totalCostCents, order.currency)}</p>
        </Card>
        <Card className="p-4">
          <p className="text-xs font-medium uppercase text-slate-400 dark:text-slate-500">Sales revenue</p>
          <p className="mt-1 text-lg font-semibold">
            {salesSummary ? formatMoney(salesSummary.revenueCents, order.currency) : "..."}
          </p>
        </Card>
        <Card className="p-4">
          <p className="text-xs font-medium uppercase text-slate-400 dark:text-slate-500">Realized profit</p>
          <p
            className={`mt-1 text-lg font-semibold ${
              salesSummary && salesSummary.profitCents > 0
                ? "text-emerald-600 dark:text-emerald-400"
                : salesSummary && salesSummary.profitCents < 0
                  ? "text-red-600 dark:text-red-400"
                  : ""
            }`}
          >
            {salesSummary ? formatMoney(salesSummary.profitCents, order.currency) : "..."}
          </p>
        </Card>
      </div>
      <p className="-mt-5 mb-8 text-xs text-slate-400 dark:text-slate-500">
        Revenue and realized profit only count tickets that are actually sold and not refunded - not-yet-sold
        tickets are never included.
      </p>

      <h2 className="mb-3 text-sm font-semibold text-slate-800 dark:text-slate-200">Tickets in this order ({tickets?.length ?? 0})</h2>
      {tickets && tickets.length >= 5000 && (
        // 1.6.0 audit H4: this list is capped the same way Orders/Tickets/
        // Sales already are (see LIST_CAP in tickets.rs) - but a single
        // order's quantity can go up to 50,000, well past that cap, and
        // unlike those other three pages this one had no banner at all, so
        // tickets past #5,000 were silently invisible with no indication.
        <div className="mb-4 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-800 dark:border-amber-500/30 dark:bg-amber-500/10 dark:text-amber-400">
          Showing the first 5,000 tickets in this order. This order has more than that - the rest exist and are
          counted correctly everywhere else, they just aren&apos;t listed individually below.
        </div>
      )}
      <TicketStatusBar
        selectedIds={Array.from(selected)}
        onClear={() => setSelected(new Set())}
        onApplied={() => load()}
      />

      {tickets === null ? (
        <LoadingBlock />
      ) : tickets.length === 0 ? (
        <EmptyState title="No tickets found for this order" />
      ) : (
        // 2.0.35: same proportional-percentage model Sales.tsx now uses -
        // see that file's colgroup comment for the full history and the
        // honest narrow-window tradeoff. Two changes bundled in with the
        // conversion since these exact columns were already being touched:
        // Ticket went 84px -> 120px (basis for its new percentage) - same
        // truncating-10-char-code bug as Sale's own 2.0.33 fix
        // ("TIX-000001" didn't fit in 84px either), flagged since the
        // 2.0.33 report's FOUND BUT NOT TOUCHED section. Seat's share grew
        // from "whatever's left" to an explicit 65% (910px at the 1400px
        // reference, generous since this table has fewer competing fixed
        // columns than Sale Detail's own version of this) - marko's own
        // answer (chat) to widening it: same reasoning as Sale Detail's
        // identical change, see that file's comment. Section/Row/Seat
        // merge into one Seat column via formatSeatLocation (lib/
        // format.ts, same treatment Sale Detail got in 1.8.2) - the 3
        // underlying fields are untouched, only how they display here
        // changed.
        // 2.0.37: same shift as Sales.tsx made - min-w-[1400px] plus a
        // single percentage set couldn't stop a horizontal scrollbar below
        // 1400px wide, only stop columns shrinking below their floor. Now
        // two full percentage sets switched by the same shared
        // useNarrowTables() breakpoint as every other table: Listing price
        // hides below 1690px (still on Cost/Status, and the ticket's own
        // page - never Ticket/Seat/Cost), everything else grows a little
        // and switches to the smaller .th-c-narrow/.td-c-narrow. See
        // Sales.tsx's own colgroup comment and PROTECTED-AREAS-NOTES.md
        // (2.0.37 section) for the full reasoning and verification.
        // 2.0.38: Ticket's own code column was STILL under-measured (same
        // root cause as every other table this version - see PROTECTED-
        // AREAS-NOTES.md's 2.0.38 section). Recomputed every column against
        // real rendered content, which also caught the same gap as Sale
        // Detail's own file: the trailing actions column (just an Edit
        // button here) never had a real measured width - its old
        // 2.707%/4.787% were unmeasured guesses. Shared breakpoint moved to
        // 1649px (was 1690px) - see useNarrowTables.ts.
        <div className="overflow-x-auto rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-sm">
          <table className="w-full table-fixed border-collapse">
            {isNarrow ? (
              <colgroup>
                <col className="w-8" />
                <col className="w-[9.756%]" />
                <col className="w-[67.195%]" />
                <col className="w-[10%]" />
                <col className="w-[10.122%]" />
                <col className="w-[2.927%]" />
              </colgroup>
            ) : (
              <colgroup>
                <col className="w-8" />
                <col className="w-[7.638%]" />
                <col className="w-[68.175%]" />
                <col className="w-[7.779%]" />
                <col className="w-[8.274%]" />
                <col className="w-[6.436%]" />
                <col className="w-[1.697%]" />
              </colgroup>
            )}
            <thead className="border-b border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-800/60">
              <tr>
                <th className={isNarrow ? "th-c-narrow" : "th-c"}>
                  <input
                    type="checkbox"
                    className={CHECKBOX_CLASS}
                    checked={allSelected}
                    onChange={toggleSelectAll}
                    aria-label="Select all tickets in this order"
                  />
                </th>
                <th className={isNarrow ? "th-c-narrow" : "th-c"}>Ticket</th>
                <th className={isNarrow ? "th-c-narrow" : "th-c"}>Seat</th>
                <th className={`${isNarrow ? "th-c-narrow" : "th-c"} text-right`}>Cost</th>
                {!isNarrow && <th className="th-c text-right">Listing price</th>}
                <th className={isNarrow ? "th-c-narrow" : "th-c"}>Status</th>
                <th className={isNarrow ? "th-c-narrow" : "th-c"} />
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
              {tickets.map((t) => {
                const seatLabel = formatSeatLocation(t.section, t.rowLabel, t.seat);
                return (
                  <tr
                    key={t.id}
                    className={`hover:bg-slate-50 dark:hover:bg-slate-800/60 ${selected.has(t.id) ? "bg-brand-50/60 dark:bg-brand-500/5" : ""}`}
                  >
                    <td className={isNarrow ? "td-c-narrow" : "td-c"}>
                      <input
                        type="checkbox"
                        className={CHECKBOX_CLASS}
                        checked={selected.has(t.id)}
                        onChange={() => toggleOne(t.id)}
                        aria-label={`Select ticket ${t.code}`}
                      />
                    </td>
                    <td className={`${isNarrow ? "td-c-narrow" : "td-c"} truncate font-medium text-slate-900 dark:text-slate-100`} title={t.code}>
                      {t.code}
                    </td>
                    <td className={`${isNarrow ? "td-c-narrow" : "td-c"} truncate text-slate-500 dark:text-slate-400`} title={seatLabel}>
                      {seatLabel}
                    </td>
                    <td className={`${isNarrow ? "td-c-narrow" : "td-c"} text-right tabular-nums whitespace-nowrap`}>{formatMoney(t.totalCostCents, t.currency)}</td>
                    {!isNarrow && (
                      <td className="td-c text-right tabular-nums whitespace-nowrap">
                        {t.listingPriceCents != null ? formatMoney(t.listingPriceCents, t.currency) : "-"}
                      </td>
                    )}
                    <td className={isNarrow ? "td-c-narrow" : "td-c"}>
                      <Badge tone={t.status}>{t.status}</Badge>
                    </td>
                    <td className={isNarrow ? "td-c-narrow" : "td-c"}>
                      <div className="flex flex-wrap items-center justify-end gap-x-2 gap-y-0.5">
                        {/* 1.9.1: this used to have a "View sale" link into
                            /sales (added in 1.6.0 for a sold ticket, since
                            there was previously no way to get from here to
                            the sale that sold it) - removed per marko's
                            explicit request to stop every "this reference
                            jumps me to a different section" link in
                            Orders/Tickets/Sales. Search the Sales screen by
                            this ticket's code directly if you need that sale. */}
                        <button
                          className="text-xs font-medium text-brand-600 dark:text-brand-400 hover:underline"
                          onClick={() => setEditTicket(t)}
                        >
                          Edit
                        </button>
                      </div>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}

      <OrderEditModal
        open={editOpen}
        order={order}
        onClose={() => setEditOpen(false)}
        onSaved={() => {
          setEditOpen(false);
          load();
        }}
      />

      <TicketEditModal
        open={!!editTicket}
        ticket={editTicket}
        onClose={() => setEditTicket(null)}
        onSaved={() => {
          setEditTicket(null);
          load();
        }}
      />

      <ConfirmDialog
        open={confirmDelete}
        title="Delete this order?"
        message="This deletes the order and every ticket it generated. Only possible while none of its tickets have ever been sold (including refunded sales - delete those individually from Sale Detail first if you need to clear them). This cannot be undone."
        confirmLabel="Delete order"
        danger
        busy={deleting}
        onCancel={() => setConfirmDelete(false)}
        onConfirm={async () => {
          setDeleting(true);
          try {
            await api.deleteOrder(orderId);
            toast.success("Order deleted");
            navigate("/orders");
          } catch (e) {
            toast.error(errMsg(e));
            setConfirmDelete(false);
          } finally {
            setDeleting(false);
          }
        }}
      />

      {/* 2.0.51: see the Currency card's "Convert to EUR" button above. The
          exact converted amounts aren't known until the live rate is
          actually fetched (on confirm), so this dialog explains what WILL
          happen rather than previewing numbers - same reasoning as why
          2.0.50's New Order version only shows the rate after the click. */}
      <ConfirmDialog
        open={convertOpen}
        title="Convert this order to EUR?"
        message={`Fetches today's live ${order.currency} → EUR rate and converts this order's amounts, every one of its tickets, and every sale on those tickets (including refunded/historical ones) to EUR, so the numbers stay consistent everywhere. This cannot be undone.`}
        confirmLabel="Convert to EUR"
        danger
        busy={converting}
        onCancel={() => setConvertOpen(false)}
        onConfirm={async () => {
          setConverting(true);
          try {
            const result = await api.convertOrderCurrency(orderId);
            setConvertOpen(false);
            const { rate, rateDate, ticketsConverted, salesConverted, linkedToSheet, sheetPushError } = result.conversion;
            const salesPart = salesConverted > 0 ? ` and ${salesConverted} sale${salesConverted === 1 ? "" : "s"}` : "";
            let message = `Converted to EUR at ${rate.toFixed(4)} (${formatDateNumeric(rateDate)}) - ${ticketsConverted} ticket${ticketsConverted === 1 ? "" : "s"}${salesPart} updated.`;
            // 2.0.53: linkedToSheet is false for most orders (never synced) -
            // nothing to add then, same message as before this version.
            if (linkedToSheet) {
              message += sheetPushError
                ? ` Your Google Sheet couldn't be updated to match: ${sheetPushError}.`
                : " Your Google Sheet was updated to match.";
            }
            toast.success(message);
            load();
          } catch (e) {
            toast.error(errMsg(e));
          } finally {
            setConverting(false);
          }
        }}
      />

      <AddOrderPullModal
        open={addPullOpen}
        orderId={orderId}
        currency={order.currency}
        onClose={() => setAddPullOpen(false)}
        onSaved={() => {
          setAddPullOpen(false);
          loadPullsReceived();
        }}
      />

      {/* Reuses Pulls.tsx's own full edit form (event/date/quantity/
          currency/more info/re-link) - see that component's own 2.0.24 doc
          comment for why editing an existing linked pull gets the full form
          while creating one here deliberately doesn't. */}
      <PullReceivedFormModal
        open={editPull !== null}
        pull={editPull}
        onClose={() => setEditPull(null)}
        onSaved={() => {
          setEditPull(null);
          loadPullsReceived();
        }}
        onRequestDelete={(p) => setDeletePullTarget(p)}
      />

      <ConfirmDialog
        open={deletePullTarget !== null}
        title="Delete this received pull?"
        message={`This removes ${deletePullTarget?.code} (${deletePullTarget?.pullerName}) permanently. This can't be undone (the order itself is not affected).`}
        confirmLabel="Delete"
        danger
        busy={deletingPull}
        onCancel={() => setDeletePullTarget(null)}
        onConfirm={async () => {
          if (!deletePullTarget) return;
          setDeletingPull(true);
          try {
            await api.deletePullReceived(deletePullTarget.id);
            toast.success(`Pull ${deletePullTarget.code} deleted`);
            setDeletePullTarget(null);
            setEditPull(null);
            loadPullsReceived();
          } catch (e) {
            toast.error(errMsg(e));
          } finally {
            setDeletingPull(false);
          }
        }}
      />
    </div>
  );
}

/** 1.9.3: Order Detail's bulk action, narrowed to status only - replaces the
 * general BulkTicketEditBar that used to live here (marko: "jedine chcem
 * tam mat na vyber zmenenie statusu" - the only thing he wants here is a
 * status choice). Mirrors Sale Detail's SalePaymentStatusBar pattern
 * exactly, just a 3-way status instead of a 2-way payment status.
 * Deliberately never offers "Sold" as a destination - see
 * `bulk_update_ticket_status_impl`'s doc comment (tickets.rs) for why that
 * transition only ever happens via the Sales screen (create a sale). If the
 * selection includes an already-sold ticket, the backend rejects the whole
 * batch rather than silently skipping it - the error surfaces as a toast,
 * same as every other bulk action in this app. */
function TicketStatusBar({
  selectedIds,
  onClear,
  onApplied,
}: {
  selectedIds: number[];
  onClear: () => void;
  onApplied: () => void;
}) {
  const toast = useToast();
  const [saving, setSaving] = useState<"available" | "listed" | "cancelled" | null>(null);

  if (selectedIds.length === 0) return null;

  const apply = async (status: "available" | "listed" | "cancelled") => {
    setSaving(status);
    try {
      const updated = await api.bulkUpdateTicketStatus({ ticketIds: selectedIds, status });
      toast.success(`${updated.length} ticket${updated.length === 1 ? "" : "s"} marked as ${status}`);
      onApplied();
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setSaving(null);
    }
  };

  return (
    <div className="mb-4 flex flex-wrap items-center gap-3 rounded-lg bg-brand-50 dark:bg-brand-500/10 px-4 py-2.5 text-sm ring-1 ring-inset ring-brand-200 dark:ring-brand-500/30">
      <span className="font-medium text-brand-800 dark:text-brand-300">Selected: {selectedIds.length}</span>
      <Button variant="secondary" onClick={() => apply("available")} disabled={saving !== null}>
        {saving === "available" ? "Marking as Available..." : "Mark as Available"}
      </Button>
      <Button variant="secondary" onClick={() => apply("listed")} disabled={saving !== null}>
        {saving === "listed" ? "Marking as Listed..." : "Mark as Listed"}
      </Button>
      <Button variant="secondary" onClick={() => apply("cancelled")} disabled={saving !== null}>
        {saving === "cancelled" ? "Marking as Cancelled..." : "Mark as Cancelled"}
      </Button>
      <button
        type="button"
        className="ml-auto text-xs font-medium text-brand-700 dark:text-brand-400 hover:underline disabled:opacity-50"
        onClick={onClear}
        disabled={saving !== null}
      >
        Clear selection
      </button>
    </div>
  );
}

function OrderEditModal({
  open,
  order,
  onClose,
  onSaved,
}: {
  open: boolean;
  order: OrderRecord;
  onClose: () => void;
  onSaved: () => void;
}) {
  const toast = useToast();
  const [platforms, setPlatforms] = useState<Platform[]>([]);
  // 1.9.4: marko wants Supplier out of Order Detail's edit form entirely
  // (New Order dropped it back in 1.7.4; this was the last manual-entry
  // FORM that still offered it - CSV import can still set supplier_id via
  // its own "supplier" column, see csv_import.rs). supplierId itself is
  // kept and still round-trips through submit() below UNCHANGED from
  // order.supplierId - only the picker UI is gone, so an order that
  // already had a supplier keeps it; there's just no way to set or change
  // it from this form anymore. Deliberately not touching supplier_id in
  // the DB, CSV import/export, or the data model - see the report.
  const [supplierId, setSupplierId] = useState<number | null>(null);
  const [platformId, setPlatformId] = useState<number | null>(null);
  const [purchaseDate, setPurchaseDate] = useState("");
  const [currency, setCurrency] = useState("EUR");
  const [paymentStatus, setPaymentStatus] = useState<OrderPaymentStatus>("unpaid");
  const [notes, setNotes] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!open) return;
    api.listPlatforms().then(setPlatforms).catch(() => {});
    setSupplierId(order.supplierId);
    setPlatformId(order.platformId);
    setPurchaseDate(order.purchaseDate);
    setCurrency(order.currency);
    setPaymentStatus(order.paymentStatus);
    setNotes(order.notes ?? "");
    setError(null);
  }, [open, order]);

  const submit = async () => {
    if (!purchaseDate) return setError("Purchase date is required");
    const input: OrderEditInput = {
      supplierId,
      platformId,
      purchaseDate,
      currency,
      paymentStatus,
      notes: notes || null,
    };
    setSaving(true);
    setError(null);
    try {
      await api.updateOrder(order.id, input);
      toast.success("Order updated");
      onSaved();
    } catch (e) {
      setError(errMsg(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal open={open} onClose={onClose} title={`Edit ${order.code}`}>
      <p className="mb-4 text-xs text-slate-400 dark:text-slate-500">
        Quantity and pricing are locked after creation because they&apos;ve already been allocated to
        individual tickets. Edit ticket cost/listing price directly if you need to fix a mistake.
      </p>
      <div className="grid grid-cols-2 gap-4">
        {/* 1.9.4: Supplier used to pair with Platform here - marko wants it
            gone from this form entirely (see supplierId's own comment above).
            Platform lost its pairing partner, so it now spans the full width
            alone instead of leaving an empty cell next to it; Purchase
            date + Currency (already adjacent) become the paired row below. */}
        <div className="col-span-2">
          <LookupSelect
            label="Platform"
            // 1.9.3: purchase/both only - see the matching comment in
            // Orders.tsx's New Order form for the full reasoning.
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
        <Field label="Purchase date" required>
          <input
            type="date"
            className="input"
            value={purchaseDate}
            onChange={(e) => setPurchaseDate(e.target.value)}
          />
        </Field>
        <Field label="Currency">
          <input
            className="input"
            value={currency}
            onChange={(e) => setCurrency(e.target.value.toUpperCase())}
          />
        </Field>
        <div className="col-span-2">
          <Field label="Payment status">
            <Select value={paymentStatus} onChange={(e) => setPaymentStatus(e.target.value as OrderPaymentStatus)}>
              <option value="unpaid">Unpaid</option>
              <option value="partial">Partial</option>
              <option value="paid">Paid</option>
            </Select>
          </Field>
        </div>
        <div className="col-span-2">
          <Field label="Notes">
            <Textarea rows={3} value={notes} onChange={(e) => setNotes(e.target.value)} />
          </Field>
        </div>
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

/** 2.0.24: Order Detail's lean "Add pull info" action - see this file's
 * "Received pulls" card and commands::pulls_received's module doc comment
 * (Rust) for the full rationale. Deliberately only 2 fields, unlike Pulls
 * screen's full PullReceivedFormModal (reused here only for EDITING an
 * already-linked pull, further down): event name/date, quantity and
 * currency are all already visible elsewhere on this exact page and get
 * copied from the order automatically by `link_pull_received_to_order` -
 * asking marko to retype any of them here would just be a second, possibly
 * drifting copy of numbers the order itself already owns. */
function AddOrderPullModal({
  open,
  orderId,
  currency,
  onClose,
  onSaved,
}: {
  open: boolean;
  orderId: number;
  currency: string;
  onClose: () => void;
  onSaved: (pull: PullReceived) => void;
}) {
  const toast = useToast();
  const [pullerName, setPullerName] = useState("");
  const [amount, setAmount] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!open) return;
    setPullerName("");
    setAmount("");
    setError(null);
  }, [open]);

  const submit = async () => {
    setError(null);
    if (!pullerName.trim()) return setError("Who pulled is required");
    // Same "blank is fine, defaults to 0" rule as the sheet-sync path's own
    // "how much pull" cell - this is informational only, never required.
    const amountCents = amount.trim() ? decimalStringToCents(amount) : 0;
    if (amountCents === null) return setError("Amount is not a valid number");

    setSaving(true);
    try {
      const created = await api.linkPullReceivedToOrder(orderId, pullerName.trim(), amountCents);
      toast.success(`Pull ${created.code} linked`);
      onSaved(created);
    } catch (e) {
      setError(errMsg(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal open={open} onClose={onClose} title="Add pull info">
      <p className="mb-4 text-xs text-slate-400 dark:text-slate-500">
        Event, quantity and currency are copied from this order automatically - just fill in who pulled it and what
        you paid them.
      </p>
      <div className="flex flex-col gap-4">
        <Field label="Who pulled" required hint="Who pulled these tickets for you">
          <Input autoFocus value={pullerName} onChange={(e) => setPullerName(e.target.value)} />
        </Field>
        <Field label={`How much (${currency})`} hint="What you paid the puller - optional, defaults to 0">
          <Input inputMode="decimal" placeholder="0.00" value={amount} onChange={(e) => setAmount(e.target.value)} />
        </Field>
      </div>
      {error && <p className="mt-3 text-sm text-red-600 dark:text-red-400">{error}</p>}
      <ModalFooter>
        <Button variant="secondary" onClick={onClose} disabled={saving}>
          Cancel
        </Button>
        <Button variant="primary" onClick={submit} disabled={saving}>
          {saving ? "Saving..." : "Save"}
        </Button>
      </ModalFooter>
    </Modal>
  );
}

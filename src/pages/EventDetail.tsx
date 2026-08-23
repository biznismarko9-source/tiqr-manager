import { useCallback, useEffect, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { api, errMsg } from "../lib/api";
import type { EventWithStats, OrderRecord, Ticket } from "../lib/types";
import { formatDate, formatMoney, formatMoneyOrMixed, formatPercentOrMixed } from "../lib/format";
import {
  Badge,
  Button,
  Card,
  ConfirmDialog,
  EmptyState,
  LoadingBlock,
  StatCard,
} from "../components/ui";
import { IconArrowLeft, IconPencil, IconPlus, IconTrash } from "../components/icons";
import { useToast } from "../lib/toast";
import { EventFormModal } from "./Events";

export default function EventDetail() {
  const { id } = useParams();
  const eventId = Number(id);
  const navigate = useNavigate();
  const toast = useToast();

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

  const s = event.stats;

  // 1.8.3 (section 14): "Potential profit" for this event's still-unsold
  // stock, computed client-side from the tickets already loaded above -
  // mirrors the Dashboard's InventoryPotential block field-for-field
  // (inventory_cost_cents/listing_value_cents/potential_profit_cents), just
  // scoped to one event instead of the whole database, so no new backend
  // command is needed. `tickets` can still be null on first paint (it loads
  // independently of `event`) - falls back to an empty list, same as the
  // "Tickets (0)" heading below already tolerates.
  const unsoldTickets = (tickets ?? []).filter((t) => t.status === "available" || t.status === "listed");
  const potentialInventoryCostCents = unsoldTickets.reduce((sum, t) => sum + t.totalCostCents, 0);
  const potentialListingValueCents = unsoldTickets.reduce((sum, t) => sum + (t.listingPriceCents ?? 0), 0);
  const potentialProfitCents = potentialListingValueCents - potentialInventoryCostCents;
  // Checked against just this unsold subset (not the event-wide s.currency
  // flag) - cheap to do precisely here since it's a plain filter over an
  // already-loaded array, unlike the Dashboard's version which reuses its
  // global mixed-currency flag to avoid an extra SQL query.
  const unsoldCurrencies = Array.from(new Set(unsoldTickets.map((t) => t.currency)));
  const potentialCurrency = unsoldCurrencies.length <= 1 ? (unsoldCurrencies[0] ?? s.currency) : null;
  const missingListingPriceCount = unsoldTickets.filter((t) => t.listingPriceCents == null).length;

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

      <div className="mb-6 grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-5">
        <StatCard label="Purchased" value={String(s.purchasedTickets)} />
        {/* 1.8.3 (section 7): "Remaining" = available + listed together - the
            true "still need to sell this" count. Previously this card led
            with just availableTickets and buried listed in the sub-line,
            which understated how much unsold stock was actually left
            whenever any of it was already listed. Both individual counts
            are still shown, just in the sub-line, not lost. */}
        <StatCard
          label="Remaining"
          value={String(s.availableTickets + s.listedTickets)}
          sub={`${s.availableTickets} available, ${s.listedTickets} listed`}
        />
        <StatCard label="Sold" value={String(s.soldTickets)} sub={`${s.cancelledTickets} cancelled`} />
        <StatCard label="Cost" value={formatMoneyOrMixed(s.totalCostCents, s.currency)} />
        <StatCard label="Revenue" value={formatMoneyOrMixed(s.revenueCents, s.currency)} />
      </div>
      {s.currency === null && (
        <p className="-mt-5 mb-6 text-xs text-amber-700 dark:text-amber-400">
          This event has tickets in more than one currency, so cost/revenue/profit/margin/ROI can&apos;t be combined into one number here. Check individual orders and sales instead.
        </p>
      )}
      <div className="mb-8 grid grid-cols-2 gap-3 sm:grid-cols-3">
        <StatCard
          label="Profit"
          value={formatMoneyOrMixed(s.profitCents, s.currency)}
          tone={s.profitCents > 0 ? "positive" : s.profitCents < 0 ? "negative" : "default"}
        />
        <StatCard label="Margin" value={formatPercentOrMixed(s.margin, s.currency)} />
        <StatCard label="ROI" value={formatPercentOrMixed(s.roi, s.currency)} />
      </div>

      {/* 1.9.10: the "Potential Profit" tinted zone used to sit right here,
          between the realized Profit/Margin/ROI stats and Orders - marko
          wanted it moved all the way down, below both Orders and Tickets.
          See the bottom of this component for where it landed. */}

      {event.notes && (
        <Card className="mb-8 p-4">
          <p className="mb-1 text-xs font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">Notes</p>
          <p className="whitespace-pre-wrap text-sm text-slate-700 dark:text-slate-300">{event.notes}</p>
        </Card>
      )}

      <div className="mb-3 flex items-center justify-between">
        <h2 className="text-sm font-semibold text-slate-800 dark:text-slate-200">Orders ({orders?.length ?? 0})</h2>
        <Button
          variant="secondary"
          onClick={() => navigate("/orders", { state: { presetEventId: event.id } })}
        >
          <IconPlus className="h-4 w-4" /> New order for this event
        </Button>
      </div>
      {orders === null ? (
        <LoadingBlock />
      ) : orders.length === 0 ? (
        <EmptyState title="No orders for this event yet" />
      ) : (
        <div className="mb-8 max-w-[1400px] overflow-x-auto rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-sm">
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
        // 2.0.32: max-w-[1400px] added - see Sales.tsx's own comment on the
        // identical change for the full rationale (this table uses plain
        // auto table-layout rather than table-fixed, but a w-full table
        // still stretches every column proportionally on a wide window,
        // same fix applies).
        <div className="max-w-[1400px] overflow-x-auto rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-sm">
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

      {/* 1.8.3 (section 14): same tinted-zone treatment as the Dashboard's
          "Inventory & Potential Profit" block, just scoped to this one
          event - deliberately never called "Profit" alone, so it can't be
          mistaken for the realized Profit stat up near the top. 1.9.10:
          relocated to the bottom of the page (below Orders and Tickets) per
          marko - same content and calculation, purely a position change. */}
      <div className="mb-8 rounded-xl border border-slate-200 dark:border-slate-800 bg-slate-50/60 dark:bg-slate-800/30 p-4">
        <p className="mb-1 text-xs font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">
          Potential Profit
        </p>
        <p className="mb-3 text-xs text-slate-400 dark:text-slate-500">
          This event&apos;s unsold stock (available + listed), not yet sold. This is an estimate, not realized profit.
        </p>
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
          <StatCard
            label="Inventory cost"
            value={formatMoneyOrMixed(potentialInventoryCostCents, potentialCurrency)}
            sub="What unsold tickets cost you"
          />
          <StatCard
            label="Listing value"
            value={formatMoneyOrMixed(potentialListingValueCents, potentialCurrency)}
            sub="Unsold tickets that have a listing price"
          />
          <StatCard
            label="Potential profit"
            value={formatMoneyOrMixed(potentialProfitCents, potentialCurrency)}
            sub="Listing value minus inventory cost"
          />
        </div>
        {missingListingPriceCount > 0 && (
          <p className="mt-3 text-xs text-slate-400 dark:text-slate-500">
            {missingListingPriceCount} unsold ticket{missingListingPriceCount === 1 ? "" : "s"} still{" "}
            {missingListingPriceCount === 1 ? "has" : "have"} no listing price, so potential profit understates what
            full inventory could be worth once priced.
          </p>
        )}
      </div>

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

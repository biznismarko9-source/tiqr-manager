import { useCallback, useEffect, useState } from "react";
import { Link, useLocation, useNavigate, useParams } from "react-router-dom";
import { api, errMsg } from "../lib/api";
import type { OrderEditInput, OrderPaymentStatus, OrderRecord, OrderSalesSummary, Platform, Ticket } from "../lib/types";
import { formatDate, formatMoney, formatSeatLocation } from "../lib/format";
import {
  Badge,
  Button,
  CHECKBOX_CLASS,
  Card,
  ConfirmDialog,
  EmptyState,
  Field,
  LoadingBlock,
  Modal,
  ModalFooter,
  Select,
  Textarea,
} from "../components/ui";
import { LookupSelect } from "../components/LookupSelect";
import { IconArrowLeft, IconPencil, IconTrash } from "../components/icons";
import { useToast } from "../lib/toast";
import { TicketEditModal } from "./Tickets";

export default function OrderDetail() {
  const { id } = useParams();
  const orderId = Number(id);
  const navigate = useNavigate();
  const location = useLocation();
  const toast = useToast();

  // 1.8.3 (section 8): if the user arrived from Orders - which passes
  // state={{ from: location.pathname }} on its link into Order Detail (see
  // Orders.tsx) - Back returns to that exact page (which itself remembers
  // its last search, see lastOrdersSearch) instead of always landing on the
  // plain Orders list. Allowlisted rather than trusting state.from blindly,
  // and falls back to the pre-1.8.3 default when absent (e.g. a direct link
  // or a page refresh). 1.9.1: Tickets and Inventory used to link into Order
  // Detail too (and so are still accepted here for backward-compatible
  // fallback labeling), but marko had that navigation removed entirely - see
  // Tickets.tsx - so in practice Orders is now the only page this ever
  // arrives from.
  const cameFrom = (location.state as { from?: string } | null)?.from;
  const backTo = cameFrom && ["/tickets", "/inventory", "/orders"].includes(cameFrom) ? cameFrom : "/orders";
  const backLabel = backTo === "/tickets" ? "Back to tickets" : backTo === "/inventory" ? "Back to inventory" : "Back to orders";

  const [order, setOrder] = useState<OrderRecord | null>(null);
  const [tickets, setTickets] = useState<Ticket[] | null>(null);
  const [salesSummary, setSalesSummary] = useState<OrderSalesSummary | null>(null);
  const [editOpen, setEditOpen] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [editTicket, setEditTicket] = useState<Ticket | null>(null);
  // 1.8.3: bulk ticket actions.
  const [selected, setSelected] = useState<Set<number>>(new Set());

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

      <Card className="mb-8 grid grid-cols-2 gap-4 p-4 sm:grid-cols-3">
        <div>
          <p className="text-xs font-medium uppercase text-slate-400 dark:text-slate-500">Platform</p>
          <p className="mt-1 text-sm text-slate-700 dark:text-slate-300">{order.platformName ?? "-"}</p>
        </div>
        <div>
          <p className="text-xs font-medium uppercase text-slate-400 dark:text-slate-500">Currency</p>
          <p className="mt-1 text-sm text-slate-700 dark:text-slate-300">{order.currency}</p>
        </div>
        {order.notes && (
          <div className="col-span-2 sm:col-span-3">
            <p className="text-xs font-medium uppercase text-slate-400 dark:text-slate-500">Notes</p>
            <p className="mt-1 whitespace-pre-wrap text-sm text-slate-700 dark:text-slate-300">{order.notes}</p>
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
        // 1.8.3 table-UX audit: brought onto the same table-layout:fixed +
        // <colgroup> technique as Sales/Sale Detail (see Sales.tsx for the
        // full rationale) instead of the old min-w-[900px]+overflow-x-auto
        // pattern, which could scroll horizontally on this app's smallest
        // supported window. Section/Row/Seat are merged into one Seat column
        // via formatSeatLocation (lib/format.ts, same treatment Sale Detail
        // got in 1.8.2) - the 3 underlying fields are untouched, only how
        // they display here changed. Also added the leading checkbox column
        // (bulk actions, see TicketStatusBar above).
        <div className="overflow-x-auto rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-sm">
          <table className="w-full table-fixed border-collapse">
            <colgroup>
              <col className="w-8" />
              <col className="w-[84px]" />
              <col />
              <col className="w-[72px]" />
              <col className="w-[92px]" />
              <col className="w-24" />
              <col className="w-[110px]" />
            </colgroup>
            <thead className="border-b border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-800/60">
              <tr>
                <th className="th-c">
                  <input
                    type="checkbox"
                    className={CHECKBOX_CLASS}
                    checked={allSelected}
                    onChange={toggleSelectAll}
                    aria-label="Select all tickets in this order"
                  />
                </th>
                <th className="th-c">Ticket</th>
                <th className="th-c">Seat</th>
                <th className="th-c text-right">Cost</th>
                <th className="th-c text-right">Listing price</th>
                <th className="th-c">Status</th>
                <th className="th-c" />
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
                    <td className="td-c">
                      <input
                        type="checkbox"
                        className={CHECKBOX_CLASS}
                        checked={selected.has(t.id)}
                        onChange={() => toggleOne(t.id)}
                        aria-label={`Select ticket ${t.code}`}
                      />
                    </td>
                    <td className="td-c truncate font-medium text-slate-900 dark:text-slate-100" title={t.code}>
                      {t.code}
                    </td>
                    <td className="td-c truncate text-slate-500 dark:text-slate-400" title={seatLabel}>
                      {seatLabel}
                    </td>
                    <td className="td-c text-right tabular-nums">{formatMoney(t.totalCostCents, t.currency)}</td>
                    <td className="td-c text-right tabular-nums">
                      {t.listingPriceCents != null ? formatMoney(t.listingPriceCents, t.currency) : "-"}
                    </td>
                    <td className="td-c">
                      <Badge tone={t.status}>{t.status}</Badge>
                    </td>
                    <td className="td-c">
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

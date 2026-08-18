import { useCallback, useEffect, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { api, errMsg } from "../lib/api";
import type { OrderEditInput, OrderPaymentStatus, OrderRecord, OrderSalesSummary, Platform, Supplier, Ticket } from "../lib/types";
import { formatDate, formatMoney } from "../lib/format";
import {
  Badge,
  Button,
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
  const toast = useToast();

  const [order, setOrder] = useState<OrderRecord | null>(null);
  const [tickets, setTickets] = useState<Ticket[] | null>(null);
  const [salesSummary, setSalesSummary] = useState<OrderSalesSummary | null>(null);
  const [editOpen, setEditOpen] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [editTicket, setEditTicket] = useState<Ticket | null>(null);

  const load = useCallback(() => {
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

  return (
    <div>
      <Link to="/orders" className="mb-3 inline-flex items-center gap-1 text-sm text-slate-500 dark:text-slate-400 hover:text-slate-800 dark:hover:text-slate-200">
        <IconArrowLeft className="h-4 w-4" /> Back to orders
      </Link>

      <div className="mb-5 flex flex-wrap items-start justify-between gap-3">
        <div>
          <div className="flex items-center gap-2">
            <h1 className="text-xl font-semibold text-slate-900 dark:text-slate-100">{order.code}</h1>
            <Badge tone={order.paymentStatus}>{order.paymentStatus}</Badge>
          </div>
          <p className="mt-1 text-sm text-slate-500 dark:text-slate-400">
            <Link to={`/events/${order.eventId}`} className="hover:text-brand-700 dark:hover:text-brand-400">
              {order.eventName}
            </Link>
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

      <div className="mb-8 grid grid-cols-2 gap-3 sm:grid-cols-4">
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
      </div>

      <Card className="mb-8 grid grid-cols-2 gap-4 p-4 sm:grid-cols-3">
        <div>
          <p className="text-xs font-medium uppercase text-slate-400 dark:text-slate-500">Supplier</p>
          <p className="mt-1 text-sm text-slate-700 dark:text-slate-300">{order.supplierName ?? "-"}</p>
        </div>
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
      {tickets === null ? (
        <LoadingBlock />
      ) : tickets.length === 0 ? (
        <EmptyState title="No tickets found for this order" />
      ) : (
        <div className="overflow-x-auto rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-sm">
          <table className="w-full min-w-[900px] border-collapse">
            <thead className="border-b border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-800/60">
              <tr>
                <th className="th">Ticket</th>
                <th className="th">Section</th>
                <th className="th">Row</th>
                <th className="th">Seat</th>
                <th className="th text-right">Cost</th>
                <th className="th text-right">Listing price</th>
                <th className="th">Status</th>
                <th className="th" />
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
              {tickets.map((t) => (
                <tr key={t.id} className="hover:bg-slate-50 dark:hover:bg-slate-800/60">
                  <td className="td font-medium text-slate-900 dark:text-slate-100">{t.code}</td>
                  <td className="td text-slate-500 dark:text-slate-400">{t.section ?? "-"}</td>
                  <td className="td text-slate-500 dark:text-slate-400">{t.rowLabel ?? "-"}</td>
                  <td className="td text-slate-500 dark:text-slate-400">{t.seat ?? "-"}</td>
                  <td className="td text-right tabular-nums">{formatMoney(t.totalCostCents, t.currency)}</td>
                  <td className="td text-right tabular-nums">
                    {t.listingPriceCents != null ? formatMoney(t.listingPriceCents, t.currency) : "-"}
                  </td>
                  <td className="td">
                    <Badge tone={t.status}>{t.status}</Badge>
                  </td>
                  <td className="td text-right">
                    <button
                      className="text-xs font-medium text-brand-600 dark:text-brand-400 hover:underline"
                      onClick={() => setEditTicket(t)}
                    >
                      Edit
                    </button>
                  </td>
                </tr>
              ))}
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
        message="This deletes the order and every ticket it generated. Only possible while none of its tickets have ever been sold (including refunded sales, which stay on record). This cannot be undone."
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
  const [suppliers, setSuppliers] = useState<Supplier[]>([]);
  const [platforms, setPlatforms] = useState<Platform[]>([]);
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
    api.listSuppliers().then(setSuppliers).catch(() => {});
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
        <LookupSelect
          label="Supplier"
          options={suppliers}
          value={supplierId}
          onChange={setSupplierId}
          onCreate={async (name) => {
            const s = await api.createSupplier(name);
            setSuppliers((prev) => [...prev, s]);
            return s;
          }}
        />
        <LookupSelect
          label="Platform"
          options={platforms}
          value={platformId}
          onChange={setPlatformId}
          onCreate={async (name) => {
            const p = await api.createPlatform(name, "purchase");
            setPlatforms((prev) => [...prev, p]);
            return p;
          }}
        />
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

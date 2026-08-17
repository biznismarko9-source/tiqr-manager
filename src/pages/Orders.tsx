import { useEffect, useMemo, useState } from "react";
import { Link, useLocation, useNavigate } from "react-router-dom";
import { api, errMsg } from "../lib/api";
import type { EventWithStats, OrderInput, OrderPaymentStatus, Platform, Supplier } from "../lib/types";
import { decimalStringToCents, formatDate, formatMoney, todayIso } from "../lib/format";
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
  Textarea,
} from "../components/ui";
import { LookupSelect } from "../components/LookupSelect";
import { IconPackage, IconPlus, IconSearch } from "../components/icons";
import { useToast } from "../lib/toast";
import type { OrderRecord } from "../lib/types";

const CURRENCIES = ["EUR", "USD", "GBP", "CHF", "CZK", "PLN", "HUF", "SEK", "NOK", "DKK"];

export default function Orders() {
  const toast = useToast();
  const location = useLocation();
  const navigate = useNavigate();
  const [orders, setOrders] = useState<OrderRecord[] | null>(null);
  const [search, setSearch] = useState("");
  const [modalOpen, setModalOpen] = useState(false);
  const [presetEventId, setPresetEventId] = useState<number | undefined>(undefined);

  const load = (q?: string) => {
    api
      .listOrders(q || undefined)
      .then(setOrders)
      .catch((e) => toast.error(errMsg(e)));
  };

  useEffect(() => {
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const t = setTimeout(() => load(search), 250);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [search]);

  useEffect(() => {
    const state = location.state as { presetEventId?: number } | null;
    if (state?.presetEventId) {
      setPresetEventId(state.presetEventId);
      setModalOpen(true);
      navigate(location.pathname, { replace: true, state: null });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [location.state]);

  return (
    <div>
      <PageHeader
        title="Orders"
        subtitle="Ticket purchases. Each order automatically generates one ticket per unit."
        actions={
          <Button
            variant="primary"
            onClick={() => {
              setPresetEventId(undefined);
              setModalOpen(true);
            }}
          >
            <IconPlus className="h-4 w-4" /> New Order
          </Button>
        }
      />

      <div className="mb-4 max-w-xs">
        <div className="relative">
          <IconSearch className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-400" />
          <Input
            placeholder="Search orders..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="pl-9"
          />
        </div>
      </div>

      {orders === null ? (
        <LoadingBlock />
      ) : orders.length === 0 ? (
        <EmptyState
          icon={<IconPackage className="h-8 w-8" />}
          title="No orders yet"
          description="Record a ticket purchase to automatically generate its individual tickets."
          action={
            <Button variant="primary" onClick={() => setModalOpen(true)}>
              <IconPlus className="h-4 w-4" /> New Order
            </Button>
          }
        />
      ) : (
        <div className="overflow-x-auto rounded-xl border border-slate-200 bg-white shadow-sm">
          <table className="w-full min-w-[950px] border-collapse">
            <thead className="border-b border-slate-200 bg-slate-50">
              <tr>
                <th className="th">Order</th>
                <th className="th">Event</th>
                <th className="th">Date</th>
                <th className="th">Supplier</th>
                <th className="th">Platform</th>
                <th className="th text-right">Qty</th>
                <th className="th text-right">Sold</th>
                <th className="th text-right">Total cost</th>
                <th className="th">Payment</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100">
              {orders.map((o) => (
                <tr key={o.id} className="hover:bg-slate-50">
                  <td className="td">
                    <Link to={`/orders/${o.id}`} className="font-medium text-slate-900 hover:text-brand-700">
                      {o.code}
                    </Link>
                    {o.isDemo && (
                      <span className="ml-1.5">
                        <Badge tone="demo">demo</Badge>
                      </span>
                    )}
                  </td>
                  <td className="td">
                    <Link to={`/events/${o.eventId}`} className="hover:text-brand-700">
                      {o.eventName}
                    </Link>
                  </td>
                  <td className="td whitespace-nowrap">{formatDate(o.purchaseDate)}</td>
                  <td className="td">{o.supplierName ?? "-"}</td>
                  <td className="td">{o.platformName ?? "-"}</td>
                  <td className="td text-right tabular-nums">{o.quantity}</td>
                  <td className="td text-right tabular-nums">
                    {o.soldCount}/{o.quantity}
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

      <OrderFormModal
        open={modalOpen}
        presetEventId={presetEventId}
        onClose={() => setModalOpen(false)}
        onCreated={(order) => {
          setModalOpen(false);
          load(search);
          navigate(`/orders/${order.id}`);
        }}
      />
    </div>
  );
}

function OrderFormModal({
  open,
  presetEventId,
  onClose,
  onCreated,
}: {
  open: boolean;
  presetEventId?: number;
  onClose: () => void;
  onCreated: (order: OrderRecord) => void;
}) {
  const toast = useToast();
  const [events, setEvents] = useState<EventWithStats[]>([]);
  const [suppliers, setSuppliers] = useState<Supplier[]>([]);
  const [platforms, setPlatforms] = useState<Platform[]>([]);

  const [eventId, setEventId] = useState<number | "">("");
  const [supplierId, setSupplierId] = useState<number | null>(null);
  const [platformId, setPlatformId] = useState<number | null>(null);
  const [purchaseDate, setPurchaseDate] = useState(todayIso());
  const [quantity, setQuantity] = useState("1");
  const [unitPrice, setUnitPrice] = useState("");
  const [fees, setFees] = useState("0");
  const [otherCosts, setOtherCosts] = useState("0");
  const [currency, setCurrency] = useState("EUR");
  const [paymentStatus, setPaymentStatus] = useState<OrderPaymentStatus>("unpaid");
  const [ticketType, setTicketType] = useState("");
  const [section, setSection] = useState("");
  const [notes, setNotes] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!open) return;
    api.listEvents().then(setEvents).catch(() => {});
    api.listSuppliers().then(setSuppliers).catch(() => {});
    api.listPlatforms().then(setPlatforms).catch(() => {});
    setEventId(presetEventId ?? "");
    setSupplierId(null);
    setPlatformId(null);
    setPurchaseDate(todayIso());
    setQuantity("1");
    setUnitPrice("");
    setFees("0");
    setOtherCosts("0");
    setCurrency("EUR");
    setPaymentStatus("unpaid");
    setTicketType("");
    setSection("");
    setNotes("");
    setError(null);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, presetEventId]);

  const totalPreviewCents = useMemo(() => {
    const q = parseInt(quantity, 10) || 0;
    const up = decimalStringToCents(unitPrice) ?? 0;
    const f = decimalStringToCents(fees) ?? 0;
    const oc = decimalStringToCents(otherCosts) ?? 0;
    return q * up + f + oc;
  }, [quantity, unitPrice, fees, otherCosts]);

  const submit = async () => {
    setError(null);
    const q = parseInt(quantity, 10);
    const upCents = decimalStringToCents(unitPrice);
    const feesCents = decimalStringToCents(fees);
    const otherCents = decimalStringToCents(otherCosts);

    if (!eventId) return setError("Please select an event");
    if (!Number.isFinite(q) || q < 1) return setError("Quantity must be at least 1");
    if (upCents === null) return setError("Unit price is not a valid amount");
    if (feesCents === null) return setError("Fees is not a valid amount");
    if (otherCents === null) return setError("Other costs is not a valid amount");
    if (!purchaseDate) return setError("Purchase date is required");

    const input: OrderInput = {
      eventId: Number(eventId),
      supplierId,
      platformId,
      purchaseDate,
      quantity: q,
      unitPriceCents: upCents,
      feesCents: feesCents,
      otherCostsCents: otherCents,
      currency,
      paymentStatus,
      notes: notes || null,
      ticketType: ticketType || null,
      section: section || null,
    };

    setSaving(true);
    try {
      const created = await api.createOrder(input);
      toast.success(`Order ${created.code} created with ${created.quantity} tickets`);
      onCreated(created);
    } catch (e) {
      setError(errMsg(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal open={open} onClose={onClose} title="New order" width="max-w-2xl">
      <div className="grid grid-cols-2 gap-4">
        <div className="col-span-2">
          <Field label="Event" required>
            <Select value={eventId} onChange={(e) => setEventId(e.target.value ? Number(e.target.value) : "")}>
              <option value="">Select an event...</option>
              {events.map((ev) => (
                <option key={ev.id} value={ev.id}>
                  {ev.name} {ev.eventDate ? `(${ev.eventDate})` : ""}
                </option>
              ))}
            </Select>
          </Field>
        </div>

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
          <Input type="date" value={purchaseDate} onChange={(e) => setPurchaseDate(e.target.value)} />
        </Field>
        <Field label="Quantity" required hint="One ticket record is generated per unit">
          <Input
            type="number"
            min={1}
            step={1}
            value={quantity}
            onChange={(e) => setQuantity(e.target.value)}
          />
        </Field>

        <Field label={`Unit purchase price (${currency})`} required>
          <Input inputMode="decimal" placeholder="0.00" value={unitPrice} onChange={(e) => setUnitPrice(e.target.value)} />
        </Field>
        <Field label="Currency">
          <>
            <Input list="currency-list" value={currency} onChange={(e) => setCurrency(e.target.value.toUpperCase())} />
            <datalist id="currency-list">
              {CURRENCIES.map((c) => (
                <option key={c} value={c} />
              ))}
            </datalist>
          </>
        </Field>

        <Field label="Purchase fees (total)" hint="Split evenly across all tickets">
          <Input inputMode="decimal" value={fees} onChange={(e) => setFees(e.target.value)} />
        </Field>
        <Field label="Other costs (total)" hint="Split evenly across all tickets">
          <Input inputMode="decimal" value={otherCosts} onChange={(e) => setOtherCosts(e.target.value)} />
        </Field>

        <Field label="Ticket type">
          <Input placeholder="e.g. Category 1" value={ticketType} onChange={(e) => setTicketType(e.target.value)} />
        </Field>
        <Field label="Section">
          <Input value={section} onChange={(e) => setSection(e.target.value)} />
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
            <Textarea rows={2} value={notes} onChange={(e) => setNotes(e.target.value)} />
          </Field>
        </div>
      </div>

      <div className="mt-4 flex items-center justify-between rounded-lg bg-slate-50 px-4 py-3 text-sm">
        <span className="text-slate-500">Total cost (preview)</span>
        <span className="font-semibold tabular-nums text-slate-900">
          {formatMoney(totalPreviewCents, currency)}
        </span>
      </div>

      {error && <p className="mt-3 text-sm text-red-600">{error}</p>}
      <ModalFooter>
        <Button variant="secondary" onClick={onClose} disabled={saving}>
          Cancel
        </Button>
        <Button variant="primary" onClick={submit} disabled={saving}>
          {saving ? "Creating..." : "Create order"}
        </Button>
      </ModalFooter>
    </Modal>
  );
}

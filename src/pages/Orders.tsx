import { useEffect, useMemo, useState } from "react";
import { Link, useLocation, useNavigate } from "react-router-dom";
import { api, errMsg } from "../lib/api";
import type { EventWithStats, OrderInput, OrderPaymentStatus, Platform } from "../lib/types";
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

const CURRENCIES = ["EUR", "USD", "GBP", "CHF", "CZK", "PLN", "HUF", "SEK", "NOK", "DKK", "RON", "TRY", "BGN"];

// 1.8.3 (section 8): remembers the last-used search for this app session
// only, same convention as Sales.tsx's `lastFilters` / Tickets.tsx's
// `lastTicketsFilters`, so returning here (in particular via Order Detail's
// context-aware Back link) finds the same search instead of starting blank.
let lastOrdersSearch: string | null = null;

/** Turns the free-form "Seats" input into one label per ticket.
 * Accepts a numeric range ("12-15" -> 12,13,14,15, either direction) or a
 * comma-separated list ("12, 14, 16A"). Blank input -> no seats assigned. */
function parseSeats(raw: string): string[] {
  const trimmed = raw.trim();
  if (!trimmed) return [];
  const rangeMatch = trimmed.match(/^(\d+)\s*-\s*(\d+)$/);
  if (rangeMatch) {
    const start = parseInt(rangeMatch[1], 10);
    const end = parseInt(rangeMatch[2], 10);
    const step = start <= end ? 1 : -1;
    const out: string[] = [];
    for (let n = start; step > 0 ? n <= end : n >= end; n += step) out.push(String(n));
    return out;
  }
  return trimmed
    .split(",")
    .map((s) => s.trim())
    .filter((s) => s.length > 0);
}

export default function Orders() {
  const toast = useToast();
  const location = useLocation();
  const navigate = useNavigate();
  const [orders, setOrders] = useState<OrderRecord[] | null>(null);
  const [search, setSearch] = useState(lastOrdersSearch ?? "");
  const [modalOpen, setModalOpen] = useState(false);
  const [presetEventId, setPresetEventId] = useState<number | undefined>(undefined);

  useEffect(() => {
    lastOrdersSearch = search;
  }, [search]);

  const load = (q?: string) => {
    api
      .listOrders({ search: q || undefined })
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
    // 1.8.3 (section 11): `openCreate` (no event preset) additionally lets
    // the Dashboard's "New Order" Quick Action open this same modal without
    // pinning it to one event - purely additive, presetEventId's own
    // behavior below is unchanged.
    const state = location.state as { presetEventId?: number; openCreate?: boolean } | null;
    if (state?.presetEventId || state?.openCreate) {
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
          <IconSearch className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-400 dark:text-slate-500" />
          <Input
            placeholder="Search orders..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="pl-9"
          />
        </div>
      </div>

      {orders && orders.length >= 5000 && (
        <div className="mb-4 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-800 dark:border-amber-500/30 dark:bg-amber-500/10 dark:text-amber-400">
          Showing the most recent 5,000 orders that match your filters. Narrow the search or event filter to see the
          rest.
        </div>
      )}

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
        // 1.8.3 table-UX audit: table-layout:fixed + <colgroup> (see
        // Sales.tsx for the full rationale) instead of the old
        // min-w-[950px]+overflow-x-auto pattern. Also added whole-row
        // click-to-navigate to Order Detail, mirroring Events.tsx's own BUG
        // #7 fix - a click that lands on the Event link still goes to the
        // event instead (closest("a") defers to whichever link was actually
        // clicked). `state={{ from: location.pathname }}` lets Order
        // Detail's Back link return to this exact page (section 8).
        <div className="overflow-x-auto rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-sm">
          <table className="w-full table-fixed border-collapse">
            <colgroup>
              <col className="w-[92px]" />
              <col />
              <col className="w-[84px]" />
              <col className="w-[92px]" />
              <col className="w-[92px]" />
              <col className="w-12" />
              <col className="w-[64px]" />
              <col className="w-[88px]" />
              <col className="w-[88px]" />
            </colgroup>
            <thead className="border-b border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-800/60">
              <tr>
                <th className="th-c">Order</th>
                <th className="th-c">Event</th>
                <th className="th-c">Date</th>
                <th className="th-c">Supplier</th>
                <th className="th-c">Platform</th>
                <th className="th-c text-right">Qty</th>
                <th className="th-c text-right">Sold</th>
                <th className="th-c text-right">Total cost</th>
                <th className="th-c">Payment</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
              {orders.map((o) => (
                <tr
                  key={o.id}
                  className="cursor-pointer hover:bg-slate-50 dark:hover:bg-slate-800/60"
                  onClick={(e) => {
                    if ((e.target as HTMLElement).closest("a")) return;
                    navigate(`/orders/${o.id}`, { state: { from: location.pathname } });
                  }}
                >
                  <td className="td-c truncate" title={o.code}>
                    <Link
                      to={`/orders/${o.id}`}
                      state={{ from: location.pathname }}
                      className="font-medium text-slate-900 dark:text-slate-100 hover:text-brand-700 dark:hover:text-brand-400"
                    >
                      {o.code}
                    </Link>
                  </td>
                  <td className="td-c truncate" title={o.eventName}>
                    <Link to={`/events/${o.eventId}`} className="hover:text-brand-700 dark:hover:text-brand-400">
                      {o.eventName}
                    </Link>
                  </td>
                  <td className="td-c whitespace-nowrap">{formatDate(o.purchaseDate)}</td>
                  <td className="td-c truncate" title={o.supplierName ?? undefined}>{o.supplierName ?? "-"}</td>
                  <td className="td-c truncate" title={o.platformName ?? undefined}>{o.platformName ?? "-"}</td>
                  <td className="td-c text-right tabular-nums">{o.quantity}</td>
                  <td className="td-c text-right tabular-nums">
                    {o.soldCount}/{o.quantity}
                  </td>
                  <td className="td-c text-right tabular-nums">{formatMoney(o.totalCostCents, o.currency)}</td>
                  <td className="td-c">
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
          navigate(`/orders/${order.id}`, { state: { from: location.pathname } });
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
  const [platforms, setPlatforms] = useState<Platform[]>([]);

  const [eventId, setEventId] = useState<number | "">("");
  const [platformId, setPlatformId] = useState<number | null>(null);
  const [purchaseDate, setPurchaseDate] = useState(todayIso());
  const [quantity, setQuantity] = useState("1");
  const [unitPrice, setUnitPrice] = useState("");
  const [unitFees, setUnitFees] = useState("0");
  const [otherCosts, setOtherCosts] = useState("0");
  const [currency, setCurrency] = useState("EUR");
  const [customCurrency, setCustomCurrency] = useState(false);
  const [paymentStatus, setPaymentStatus] = useState<OrderPaymentStatus>("unpaid");
  const [section, setSection] = useState("");
  const [rowLabel, setRowLabel] = useState("");
  const [seatsRaw, setSeatsRaw] = useState("");
  const [notes, setNotes] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!open) return;
    api.listEvents().then(setEvents).catch(() => {});
    api.listPlatforms().then(setPlatforms).catch(() => {});
    setEventId(presetEventId ?? "");
    setPlatformId(null);
    setPurchaseDate(todayIso());
    setQuantity("1");
    setUnitPrice("");
    setUnitFees("0");
    setOtherCosts("0");
    setCurrency("EUR");
    setCustomCurrency(false);
    setPaymentStatus("unpaid");
    setSection("");
    setRowLabel("");
    setSeatsRaw("");
    setNotes("");
    setError(null);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, presetEventId]);

  const qNum = parseInt(quantity, 10) || 0;

  const totalPreviewCents = useMemo(() => {
    const up = decimalStringToCents(unitPrice) ?? 0;
    const f = decimalStringToCents(unitFees) ?? 0;
    const oc = decimalStringToCents(otherCosts) ?? 0;
    // Fees are entered per ticket now (same as unit price) - multiply by
    // quantity here too. Other costs stays a plain order-wide total.
    return qNum * up + qNum * f + oc;
  }, [qNum, unitPrice, unitFees, otherCosts]);

  const submit = async () => {
    setError(null);
    const q = parseInt(quantity, 10);
    const upCents = decimalStringToCents(unitPrice);
    const unitFeesCents = decimalStringToCents(unitFees);
    const otherCents = decimalStringToCents(otherCosts);
    const seats = parseSeats(seatsRaw);

    if (!eventId) return setError("Please select an event");
    if (!Number.isFinite(q) || q < 1) return setError("Quantity must be at least 1");
    if (upCents === null) return setError("Unit price is not a valid amount");
    if (unitFeesCents === null) return setError("Fees is not a valid amount");
    if (otherCents === null) return setError("Other costs is not a valid amount");
    if (!purchaseDate) return setError("Purchase date is required");
    if (seats.length > 0 && seats.length !== q) {
      return setError(`You entered ${seats.length} seat(s) but quantity is ${q} - provide one seat per ticket, or clear the Seats field`);
    }

    const input: OrderInput = {
      eventId: Number(eventId),
      // 1.7.4: Supplier is no longer collected on this form (see LookupSelect
      // removal below) - marko flagged it as clutter he never uses when
      // quickly creating an order. Always sending null here, rather than
      // omitting the key, keeps the intent explicit for the next reader.
      // Existing/CSV-imported orders that already have a supplier keep it -
      // this form just never sets one going forward.
      supplierId: null,
      platformId,
      purchaseDate,
      quantity: q,
      unitPriceCents: upCents,
      // Fees are entered per ticket in this form (same as unit price), but
      // the backend's OrderInput.feesCents has always meant - and still
      // means - the order-wide total (it splits that total evenly across
      // tickets via the existing, tested allocate_cents). Multiplying here
      // keeps that backend contract 100% unchanged: the total sent is an
      // exact multiple of quantity, so allocate_cents hands every ticket
      // back exactly this per-unit amount with zero remainder to distribute.
      feesCents: unitFeesCents * q,
      otherCostsCents: otherCents,
      currency,
      paymentStatus,
      notes: notes || null,
      ticketType: null,
      section: section || null,
      rowLabel: rowLabel || null,
      seats: seats.length > 0 ? seats : null,
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

        {/* 1.7.4: Supplier used to sit here next to Platform - removed as
            clutter marko never uses on this form (still settable per-order
            via Edit on Order Detail, and CSV import still recognizes a
            "supplier" column, so nothing already using it breaks). Platform
            now takes the full width on its own row instead of leaving an
            empty half-row gap. */}
        <div className="col-span-2">
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
        </div>

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
        <div>
          {/* 1.7.3: this used to be an <input list="currency-list"> +
              <datalist> - CURRENCIES already had 13 options (EUR/GBP/USD
              included) but a plain text input pre-filled with "EUR" gives no
              visible sign that anything else is pickable, so marko never
              found them. A real <Select> makes every option visible on
              click; "Other..." (same toggle pattern as LookupSelect's
              "+ New") still allows typing any currency code freely. */}
          <div className="flex items-center justify-between">
            <span className="label mb-1">Currency</span>
            <button
              type="button"
              className="mb-1 text-xs font-medium text-brand-600 dark:text-brand-400 hover:underline"
              onClick={() => setCustomCurrency((c) => !c)}
            >
              {customCurrency ? "Choose from list" : "Other..."}
            </button>
          </div>
          {customCurrency ? (
            <Input
              autoFocus
              placeholder="e.g. AED"
              value={currency}
              onChange={(e) => setCurrency(e.target.value.toUpperCase())}
            />
          ) : (
            <Select value={currency} onChange={(e) => setCurrency(e.target.value)}>
              {(CURRENCIES.includes(currency) ? CURRENCIES : [currency, ...CURRENCIES]).map((c) => (
                <option key={c} value={c}>
                  {c}
                </option>
              ))}
            </Select>
          )}
        </div>

        <Field label={`Unit purchase fees (${currency})`}>
          <Input inputMode="decimal" value={unitFees} onChange={(e) => setUnitFees(e.target.value)} />
        </Field>
        <Field label="Other costs (total)" hint="Split evenly across all tickets">
          <Input inputMode="decimal" value={otherCosts} onChange={(e) => setOtherCosts(e.target.value)} />
        </Field>

        <Field label="Section">
          <Input value={section} onChange={(e) => setSection(e.target.value)} />
        </Field>
        <Field label="Row">
          <Input value={rowLabel} onChange={(e) => setRowLabel(e.target.value)} />
        </Field>

        <div className="col-span-2">
          <Field
            label="Seats"
            hint={qNum > 1 ? `One per ticket, e.g. "12-${11 + qNum}" or "12,13,14" - optional` : 'Optional, e.g. "12"'}
          >
            <Input
              placeholder={qNum > 1 ? "12-15" : "12"}
              value={seatsRaw}
              onChange={(e) => setSeatsRaw(e.target.value)}
            />
          </Field>
        </div>

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

      <div className="mt-4 flex items-center justify-between rounded-lg bg-slate-50 dark:bg-slate-800/60 px-4 py-3 text-sm">
        <span className="text-slate-500 dark:text-slate-400">Total cost (preview)</span>
        <span className="font-semibold tabular-nums text-slate-900 dark:text-slate-100">
          {formatMoney(totalPreviewCents, currency)}
        </span>
      </div>

      {error && <p className="mt-3 text-sm text-red-600 dark:text-red-400">{error}</p>}
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

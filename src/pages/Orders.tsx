import { useEffect, useMemo, useState, type ReactNode } from "react";
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

// 1.9.1: preset ticket-type options for the New Order form's "Ticket type"
// field (see submit() below) - the most common delivery formats in ticket
// resale. Free-form via "Other..." (same toggle pattern as Currency, right
// below) covers anything not listed here. This replaces "ticket type" as a
// bulk-edit field (removed from BulkTicketEditBar per marko's request) -
// it's now a one-time choice made here at order creation, copied onto every
// generated ticket (see OrderInput.ticketType / insert_order_with_tickets),
// same as Section/Row already are.
//
// 2.0.19: this is now only the FALLBACK shown until `api.listTicketTypes()`
// resolves (and the safety net if that call ever fails) - the real,
// growable list comes from the backend (`known_ticket_type_names` in
// commands/tickets.rs), seeded with exactly these same 5 values, so a value
// typed via "Other..." here (or found in a synced sheet cell) shows up as a
// real option next time, everywhere this list is used - both here and in
// the Orders & Sales sheet's own Ticket Type dropdown.
const TICKET_TYPES = ["E-ticket", "PDF", "Mobile transfer", "Physical", "Will call"];

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
        // #7 fix - a click that lands on the Order code link still goes to
        // that link instead (closest("a") defers to whichever link was
        // actually clicked). `state={{ from: location.pathname }}` lets
        // Order Detail's Back link return to this exact page (section 8).
        // 1.9.1: the Event column used to be a <Link> to Event Detail - marko
        // asked to remove every "this reference jumps me to a different
        // section" link across Orders/Tickets/Sales (he never wants an
        // incidental click to auto-navigate him away), so it's now plain
        // text. The Order code link/row-click above is unaffected - opening
        // this exact order's own detail page isn't "being thrown elsewhere".
        // 1.9.2 (section 2): removed the Supplier column entirely from this
        // list (Supplier stays fully intact in the data model, CSV import/
        // export, and Edit Order on Order Detail - this is a list-view-only
        // simplification, no DB/migration change). Fixed columns summed to
        // 556px right after that (was 648px before removing Supplier's own
        // 92px).
        // 1.9.4: marko pointed out Platform names were getting truncated
        // ("Fnac Spect...") with room to spare elsewhere, so Platform grew
        // 92px -> 160px. Fixed columns summed to 624px then, Event's floor
        // 184px.
        // 1.9.10: marko added a Notes column (between Date and Platform,
        // truncate + title tooltip like every other text column here - he
        // didn't ask for full-text visibility the way he did for Pulls'
        // Seats/More info, so this follows the normal pattern instead of
        // that one). Fixed columns now sum to 754px (92 Order + 84 Date +
        // 130 Notes + 160 Platform + 48 Qty + 64 Sold + 88 Total cost + 88
        // Payment), leaving Event (still the one unspecified <col>) only a
        // 54px floor at this app's absolute 808px worst-case window width -
        // genuinely too tight to show much of an event name. Rather than
        // shrink Notes or Platform back down to force-fit the old 808px
        // floor, this table now accepts needing horizontal scroll (already
        // handled by the existing overflow-x-auto below) somewhat below
        // where it used to - the same tradeoff already made for Pulls this
        // round for the same underlying reason: more columns/more width
        // asked for than the original floor was sized for. In normal usage
        // (wider than the absolute minimum) this isn't noticeable; it only
        // matters at the smallest supported window.
        <div className="overflow-x-auto rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-sm">
          <table className="w-full table-fixed border-collapse">
            <colgroup>
              <col className="w-[92px]" />
              <col />
              <col className="w-[84px]" />
              <col className="w-[130px]" />
              <col className="w-[160px]" />
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
                <th className="th-c">Notes</th>
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
                    {o.eventName}
                  </td>
                  <td className="td-c whitespace-nowrap">{formatDate(o.purchaseDate)}</td>
                  <td className="td-c truncate text-slate-500 dark:text-slate-400" title={o.notes ?? undefined}>
                    {o.notes || "-"}
                  </td>
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

/** 1.9.2 (section 6): purely visual grouping for New Order's fields - wraps a
 * labelled cluster in its own mini 2-column grid, separated from the group
 * above it by a thin top border (the first group in a form has none, via
 * `first:border-t-0`). Local to this file, not promoted to ui.tsx - New
 * Order is the only form in the app asking for this kind of sectioning so
 * far; nothing else needs it yet. `title` is optional so the same component
 * can wrap Payment status/Notes at the bottom (deliberately left out of
 * marko's 4 named groups - EVENT/TICKETS/PURCHASE/SUMMARY - but still
 * visually separated from the group above them). */
function FormGroup({ title, children }: { title?: string; children: ReactNode }) {
  return (
    <div className="border-t border-slate-200 pt-4 first:border-t-0 first:pt-0 dark:border-slate-800">
      {title && (
        <p className="mb-3 text-xs font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">{title}</p>
      )}
      <div className="grid grid-cols-2 gap-4">{children}</div>
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
  // 2.0.19: real, growable list - see TICKET_TYPES's own comment above.
  const [ticketTypeOptions, setTicketTypeOptions] = useState<string[]>(TICKET_TYPES);

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
  const [ticketType, setTicketType] = useState("");
  const [customTicketType, setCustomTicketType] = useState(false);
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
    api.listTicketTypes().then(setTicketTypeOptions).catch(() => {});
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
    setTicketType("");
    setCustomTicketType(false);
    setSection("");
    setRowLabel("");
    setSeatsRaw("");
    setNotes("");
    setError(null);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, presetEventId]);

  const qNum = parseInt(quantity, 10) || 0;

  // 1.9.2 (section 7): was a single `totalPreviewCents` number - broken into
  // named subtotals here only so the new SUMMARY bar below can show a
  // breakdown ("4 tickets / Purchase: X / Fees: X / Total: X"). The formula
  // itself (fees entered per ticket, same as unit price, so both are
  // multiplied by quantity; other costs stays a plain order-wide total) is
  // untouched - purchaseCents + feesCents + otherCents is mathematically
  // identical to the old totalPreviewCents value.
  const summary = useMemo(() => {
    const up = decimalStringToCents(unitPrice) ?? 0;
    const f = decimalStringToCents(unitFees) ?? 0;
    const oc = decimalStringToCents(otherCosts) ?? 0;
    const purchaseCents = qNum * up;
    const feesCents = qNum * f;
    const otherCents = oc;
    return { purchaseCents, feesCents, otherCents, totalCents: purchaseCents + feesCents + otherCents };
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
      ticketType: ticketType.trim() || null,
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
      <div className="flex flex-col gap-4">
        <FormGroup title="Event">
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
          <Field label="Purchase date" required>
            <Input type="date" value={purchaseDate} onChange={(e) => setPurchaseDate(e.target.value)} />
          </Field>
        </FormGroup>

        <FormGroup title="Tickets">
          <Field label="Quantity" required hint="One ticket record is generated per unit">
            <Input
              type="number"
              min={1}
              step={1}
              value={quantity}
              onChange={(e) => setQuantity(e.target.value)}
            />
          </Field>
          {/* 1.9.1: replaces "ticket type" as a bulk-edit field on Sale/Order
              Detail (removed per marko's request) - set once here, copied onto
              every ticket this order generates. Same Select + "Other..."
              freeform toggle pattern as Currency below. 1.9.2 (section 6):
              now paired half-width with Quantity (same layout Currency
              already uses next to Unit price) instead of its own full-width
              row - part of this round's form simplification. */}
          <div>
            <div className="flex items-center justify-between">
              <span className="label mb-1">Ticket type</span>
              <button
                type="button"
                className="mb-1 text-xs font-medium text-brand-600 dark:text-brand-400 hover:underline"
                onClick={() => setCustomTicketType((c) => !c)}
              >
                {customTicketType ? "Choose from list" : "Other..."}
              </button>
            </div>
            {customTicketType ? (
              <Input
                autoFocus
                placeholder="e.g. Will call"
                value={ticketType}
                onChange={(e) => setTicketType(e.target.value)}
              />
            ) : (
              <Select value={ticketType} onChange={(e) => setTicketType(e.target.value)}>
                <option value="">Not specified</option>
                {/* Same "keep a custom value visible" fallback as Currency below -
                    without this, toggling back from "Other..." after typing a
                    value not in the list would show a blank-looking select
                    even though `ticketType` state is still correct. */}
                {(ticketType && !ticketTypeOptions.includes(ticketType)
                  ? [ticketType, ...ticketTypeOptions]
                  : ticketTypeOptions
                ).map((t) => (
                  <option key={t} value={t}>
                    {t}
                  </option>
                ))}
              </Select>
            )}
          </div>

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
        </FormGroup>

        <FormGroup title="Purchase">
          {/* 1.7.4: Supplier used to sit here next to Platform - removed as
              clutter marko never uses on this form (still settable per-order
              via Edit on Order Detail, and CSV import still recognizes a
              "supplier" column, so nothing already using it breaks - 1.9.2
              section 6 reconfirms Supplier stays out of this form). Platform
              takes the full width on its own row instead of leaving an empty
              half-row gap. */}
          <div className="col-span-2">
            <LookupSelect
              label="Platform"
              // 1.9.3: Orders only offers "purchase"/"both" platforms now -
              // marko split the shared platform pool into where-you-bought
              // vs where-you-sold lists (see Settings -> Lookups). onCreate
              // right below was already tagging new platforms "purchase"
              // from here even before 1.9.3; only this display filter is new.
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
        </FormGroup>

        {/* Payment status/Notes deliberately aren't one of marko's 4 named
            groups (EVENT/TICKETS/PURCHASE/SUMMARY) - FormGroup without a
            title still gives them the same divider separation as a named
            group, just no heading text. */}
        <FormGroup>
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
        </FormGroup>
      </div>

      {/* 1.9.2 (section 7): was a single "Total cost (preview)" line - kept
          the exact same underlying calculation (see the `summary` useMemo
          above; purchaseCents + feesCents + otherCents is mathematically
          identical to the old totalPreviewCents value) and only improved the
          presentation, breaking it into the ticket count and each cost
          component the way marko's brief spelled out, e.g. "4 tickets ·
          Purchase: EUR200.00 · Fees: EUR8.00 · Total: EUR208.00". */}
      <div className="mt-4">
        <p className="mb-2 text-xs font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">Summary</p>
        <div className="flex flex-wrap items-center gap-x-2 gap-y-1 rounded-lg bg-slate-50 dark:bg-slate-800/60 px-4 py-3 text-sm text-slate-500 dark:text-slate-400">
          <span>
            {qNum} ticket{qNum === 1 ? "" : "s"}
          </span>
          <span className="text-slate-300 dark:text-slate-600">&middot;</span>
          <span>Purchase: {formatMoney(summary.purchaseCents, currency)}</span>
          <span className="text-slate-300 dark:text-slate-600">&middot;</span>
          <span>Fees: {formatMoney(summary.feesCents, currency)}</span>
          {summary.otherCents !== 0 && (
            <>
              <span className="text-slate-300 dark:text-slate-600">&middot;</span>
              <span>Other costs: {formatMoney(summary.otherCents, currency)}</span>
            </>
          )}
          <span className="text-slate-300 dark:text-slate-600">&middot;</span>
          <span className="font-semibold text-slate-900 dark:text-slate-100">
            Total: {formatMoney(summary.totalCents, currency)}
          </span>
        </div>
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

import { useEffect, useMemo, useState } from "react";
import { Link, useLocation, useNavigate } from "react-router-dom";
import { api, errMsg } from "../lib/api";
import type { EventWithStats, Platform, SaleBatchInput, SaleGroup, SalePaymentStatus, Ticket } from "../lib/types";
import { formatDate, formatMoney, formatMoneyOrMixed, formatPercentOrMixed, todayIso } from "../lib/format";
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
import { IconPlus, IconReceipt, IconSearch, IconX } from "../components/icons";
import { useToast } from "../lib/toast";

export default function Sales() {
  const toast = useToast();
  const location = useLocation();
  const navigate = useNavigate();
  const [groups, setGroups] = useState<SaleGroup[] | null>(null);
  const [events, setEvents] = useState<EventWithStats[]>([]);
  const [search, setSearch] = useState("");
  const [eventId, setEventId] = useState<number | "">("");
  const [paymentStatus, setPaymentStatus] = useState("");
  const [refundStatus, setRefundStatus] = useState("");
  const [dateFrom, setDateFrom] = useState("");
  const [dateTo, setDateTo] = useState("");
  const [modalOpen, setModalOpen] = useState(false);

  useEffect(() => {
    api.listEvents().then(setEvents).catch(() => {});
  }, []);

  // Lets another page (e.g. a "View sale" link next to a sold ticket in
  // OrderDetail.tsx) jump here pre-filtered to one ticket's sale, using the
  // exact same navigate(path, { state }) + consume-and-clear convention
  // Orders.tsx already uses for presetEventId - not a new pattern, no new
  // backend query, just reusing the existing ticket-code search (BUG #5).
  useEffect(() => {
    const state = location.state as { presetSearch?: string } | null;
    if (state?.presetSearch) {
      setSearch(state.presetSearch);
      navigate(location.pathname, { replace: true, state: null });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [location.state]);

  const load = () => {
    api
      .listSaleGroups({
        search: search || undefined,
        eventId: eventId || undefined,
        paymentStatus: paymentStatus || undefined,
        refundStatus: refundStatus || undefined,
        dateFrom: dateFrom || undefined,
        dateTo: dateTo || undefined,
      })
      .then(setGroups)
      .catch((e) => toast.error(errMsg(e)));
  };

  useEffect(() => {
    const t = setTimeout(load, 200);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [search, eventId, paymentStatus, refundStatus, dateFrom, dateTo]);

  const totals = useMemo(() => {
    if (!groups) return null;
    // Every group's own revenue/profit already excludes its refunded lines,
    // so this can sum them directly. Amounts in different currencies can
    // never be added together, so this only sums when every group shares one.
    const currency =
      groups.length > 0 && groups.every((g) => g.currency === groups[0].currency) ? groups[0].currency : null;
    const sums = groups.reduce(
      (acc, g) => ({
        revenue: acc.revenue + g.revenueCents,
        profit: acc.profit + g.profitCents,
        tickets: acc.tickets + g.ticketCount,
        refunded: acc.refunded + g.refundedCount,
      }),
      { revenue: 0, profit: 0, tickets: 0, refunded: 0 },
    );
    return { ...sums, currency };
  }, [groups]);

  return (
    <div>
      <PageHeader
        title="Sales"
        subtitle="Every sale you've recorded, with profit calculated automatically."
        actions={
          <Button variant="primary" onClick={() => setModalOpen(true)}>
            <IconPlus className="h-4 w-4" /> New Sale
          </Button>
        }
      />

      <div className="mb-4 flex flex-wrap items-end gap-3">
        <div className="w-56">
          <span className="label">Search</span>
          <div className="relative">
            <IconSearch className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-400 dark:text-slate-500" />
            <Input
              placeholder="Sale code, ticket, buyer..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="pl-9"
            />
          </div>
        </div>
        <div className="w-52">
          <span className="label">Event</span>
          <Select value={eventId} onChange={(e) => setEventId(e.target.value ? Number(e.target.value) : "")}>
            <option value="">All events</option>
            {events.map((ev) => (
              <option key={ev.id} value={ev.id}>
                {ev.name}
              </option>
            ))}
          </Select>
        </div>
        <div className="w-40">
          <span className="label">Payment</span>
          <Select value={paymentStatus} onChange={(e) => setPaymentStatus(e.target.value)}>
            <option value="">All</option>
            <option value="pending">Pending</option>
            <option value="paid">Paid</option>
            <option value="refunded">Refunded</option>
          </Select>
        </div>
        <div className="w-40">
          <span className="label">Refunds</span>
          <Select value={refundStatus} onChange={(e) => setRefundStatus(e.target.value)}>
            <option value="">All</option>
            <option value="has_refund">Has a refund</option>
            <option value="no_refund">No refunds</option>
          </Select>
        </div>
        <div className="w-36">
          <span className="label">From</span>
          <Input type="date" value={dateFrom} onChange={(e) => setDateFrom(e.target.value)} />
        </div>
        <div className="w-36">
          <span className="label">To</span>
          <Input type="date" value={dateTo} onChange={(e) => setDateTo(e.target.value)} />
        </div>
        {totals && groups && (
          <p className="ml-auto text-xs text-slate-400 dark:text-slate-500">
            {groups.length} sales &middot; {totals.tickets} tickets &middot; revenue{" "}
            {formatMoneyOrMixed(totals.revenue, totals.currency)} &middot; profit{" "}
            {formatMoneyOrMixed(totals.profit, totals.currency)}
            {totals.refunded > 0 ? ` (${totals.refunded} ticket${totals.refunded === 1 ? "" : "s"} refunded, excluded)` : ""}
          </p>
        )}
      </div>

      {groups && groups.length >= 5000 && (
        <div className="mb-4 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-800 dark:border-amber-500/30 dark:bg-amber-500/10 dark:text-amber-400">
          Showing the most recent 5,000 sales that match your filters. Narrow the date range, event, or payment
          filter to see the rest.
        </div>
      )}

      {groups === null ? (
        <LoadingBlock />
      ) : groups.length === 0 ? (
        <EmptyState
          icon={<IconReceipt className="h-8 w-8" />}
          title="No sales match these filters"
          action={
            <Button variant="primary" onClick={() => setModalOpen(true)}>
              <IconPlus className="h-4 w-4" /> New Sale
            </Button>
          }
        />
      ) : (
        // One row per sale action (single ticket or multi-ticket batch) -
        // same table style as the Tickets screen's order-grouped list. A
        // batch of e.g. 8 tickets sold as 4+2+2 shows as 3 rows here, never
        // as 8 separate rows; clicking a row's Sale code opens Sale Detail,
        // which lists every ticket inside that one sale. The data was
        // already grouped this way (SaleGroup/batch_id, see sales.rs) - only
        // the layout changed here, no field or number is new.
        <div className="overflow-x-auto rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-sm">
          <table className="w-full min-w-[1100px] border-collapse">
            <thead className="border-b border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-800/60">
              <tr>
                <th className="th">Sale</th>
                <th className="th">Event</th>
                <th className="th">Platform</th>
                <th className="th">Sale date</th>
                <th className="th text-right">Tickets</th>
                <th className="th text-right">Revenue</th>
                <th className="th text-right">Fees</th>
                <th className="th text-right">Profit</th>
                <th className="th text-right">Margin / ROI</th>
                <th className="th">Status</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
              {groups.map((g) => (
                <tr key={g.id} className="hover:bg-slate-50 dark:hover:bg-slate-800/60">
                  <td className="td">
                    <Link
                      to={`/sales/${g.id}`}
                      className="font-medium text-slate-900 dark:text-slate-100 hover:text-brand-700 dark:hover:text-brand-400"
                    >
                      {g.code}
                    </Link>
                  </td>
                  <td className="td">
                    {g.eventId && g.eventName ? (
                      <Link to={`/events/${g.eventId}`} className="hover:text-brand-700 dark:hover:text-brand-400">
                        {g.eventName}
                      </Link>
                    ) : (
                      <span className="italic text-slate-400 dark:text-slate-500">Mixed events</span>
                    )}
                  </td>
                  <td className="td text-slate-500 dark:text-slate-400">{g.platformName ?? "-"}</td>
                  <td className="td whitespace-nowrap">{formatDate(g.saleDate)}</td>
                  <td className="td text-right tabular-nums">{g.ticketCount}</td>
                  <td className="td text-right tabular-nums">{formatMoneyOrMixed(g.revenueCents, g.currency)}</td>
                  <td className="td text-right tabular-nums">{formatMoneyOrMixed(g.sellingFeesCents, g.currency)}</td>
                  <td
                    className={`td text-right tabular-nums font-medium ${
                      g.profitCents > 0
                        ? "text-emerald-600 dark:text-emerald-400"
                        : g.profitCents < 0
                          ? "text-red-600 dark:text-red-400"
                          : ""
                    }`}
                  >
                    {formatMoneyOrMixed(g.profitCents, g.currency)}
                  </td>
                  <td className="td text-right tabular-nums">
                    {formatPercentOrMixed(g.margin, g.currency)} / {formatPercentOrMixed(g.roi, g.currency)}
                  </td>
                  <td className="td">
                    {g.paymentStatus ? <Badge tone={g.paymentStatus}>{g.paymentStatus}</Badge> : <Badge tone="mixed">Mixed</Badge>}
                    {g.refundedCount > 0 && (
                      <p className="mt-0.5 text-xs font-medium text-amber-700 dark:text-amber-400">
                        {g.refundedCount} of {g.ticketCount} refunded
                      </p>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      <SaleFormModal
        open={modalOpen}
        onClose={() => setModalOpen(false)}
        onCreated={() => {
          setModalOpen(false);
          load();
        }}
      />
    </div>
  );
}

interface SaleLineDraft {
  price: string;
  fees: string;
}

function SaleFormModal({
  open,
  onClose,
  onCreated,
}: {
  open: boolean;
  onClose: () => void;
  onCreated: () => void;
}) {
  const toast = useToast();
  const [step, setStep] = useState<"pick" | "details">("pick");
  const [selected, setSelected] = useState<Ticket[]>([]);
  const [lines, setLines] = useState<Record<number, SaleLineDraft>>({});
  const [ticketQuery, setTicketQuery] = useState("");
  const [ticketOptions, setTicketOptions] = useState<Ticket[]>([]);
  const [platforms, setPlatforms] = useState<Platform[]>([]);
  const [platformId, setPlatformId] = useState<number | null>(null);
  const [saleDate, setSaleDate] = useState(todayIso());
  const [bulkPrice, setBulkPrice] = useState("");
  const [bulkFees, setBulkFees] = useState("");
  const [paymentStatus, setPaymentStatus] = useState<SalePaymentStatus>("pending");
  const [buyerReference, setBuyerReference] = useState("");
  const [notes, setNotes] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!open) return;
    setStep("pick");
    setSelected([]);
    setLines({});
    setTicketQuery("");
    setTicketOptions([]);
    setPlatformId(null);
    setSaleDate(todayIso());
    setBulkPrice("");
    setBulkFees("");
    setPaymentStatus("pending");
    setBuyerReference("");
    setNotes("");
    setError(null);
    api.listPlatforms().then(setPlatforms).catch(() => {});
  }, [open]);

  useEffect(() => {
    if (!open || step !== "pick") return;
    const t = setTimeout(() => {
      api
        .listTickets({ search: ticketQuery || undefined, status: "available,listed", sortBy: "created", sortDir: "desc" })
        .then((res) => setTicketOptions(res.slice(0, 25)))
        .catch(() => {});
    }, 200);
    return () => clearTimeout(t);
  }, [open, step, ticketQuery]);

  // If every ticket gets removed while on the details step, drop back to
  // picking rather than showing an empty pricing form.
  useEffect(() => {
    if (step === "details" && selected.length === 0) setStep("pick");
  }, [step, selected.length]);

  const addTicket = (t: Ticket) => {
    setSelected((prev) => (prev.some((s) => s.id === t.id) ? prev : [...prev, t]));
    setLines((prev) => (prev[t.id] ? prev : { ...prev, [t.id]: { price: "", fees: "0" } }));
  };

  const removeTicket = (id: number) => {
    setSelected((prev) => prev.filter((t) => t.id !== id));
    setLines((prev) => {
      const next = { ...prev };
      delete next[id];
      return next;
    });
  };

  const updateLine = (id: number, field: keyof SaleLineDraft, value: string) => {
    setLines((prev) => ({ ...prev, [id]: { ...prev[id], [field]: value } }));
  };

  const applyBulkPrice = () => {
    if (!bulkPrice.trim()) return;
    setLines((prev) => {
      const next = { ...prev };
      for (const t of selected) next[t.id] = { ...next[t.id], price: bulkPrice };
      return next;
    });
  };

  const applyBulkFees = () => {
    if (!bulkFees.trim()) return;
    setLines((prev) => {
      const next = { ...prev };
      for (const t of selected) next[t.id] = { ...next[t.id], fees: bulkFees };
      return next;
    });
  };

  const visibleOptions = ticketOptions.filter((t) => !selected.some((s) => s.id === t.id));
  const singleCurrency =
    selected.length > 0 && selected.every((t) => t.currency === selected[0].currency) ? selected[0].currency : null;

  const totals = useMemo(() => {
    let revenue = 0;
    let fees = 0;
    let cost = 0;
    for (const t of selected) {
      const line = lines[t.id];
      const p = parseFloat((line?.price ?? "").trim().replace(",", "."));
      const f = parseFloat((line?.fees ?? "0").trim().replace(",", ".")) || 0;
      if (Number.isFinite(p)) revenue += Math.round(p * 100);
      fees += Math.round(f * 100);
      cost += t.totalCostCents;
    }
    return { revenue, cost, fees, profit: revenue - cost - fees };
  }, [selected, lines]);

  // Only used when the selection is mixed-currency (1.6.0 audit UX finding:
  // previously that case showed no profit preview at all, just a one-line
  // notice). Same lenient, live-preview parsing as `totals` above, just
  // grouped by currency instead of assuming the whole batch is one.
  const perCurrencyTotals = useMemo(() => {
    const byCurrency = new Map<string, { revenue: number; cost: number; fees: number; count: number }>();
    for (const t of selected) {
      const line = lines[t.id];
      const p = parseFloat((line?.price ?? "").trim().replace(",", "."));
      const f = parseFloat((line?.fees ?? "0").trim().replace(",", ".")) || 0;
      const entry = byCurrency.get(t.currency) ?? { revenue: 0, cost: 0, fees: 0, count: 0 };
      if (Number.isFinite(p)) entry.revenue += Math.round(p * 100);
      entry.fees += Math.round(f * 100);
      entry.cost += t.totalCostCents;
      entry.count += 1;
      byCurrency.set(t.currency, entry);
    }
    return Array.from(byCurrency.entries()).map(([currency, v]) => ({ currency, ...v, profit: v.revenue - v.cost - v.fees }));
  }, [selected, lines]);

  const submit = async () => {
    setError(null);
    if (selected.length === 0) return setError("Select at least one ticket to sell first");
    if (!saleDate) return setError("Sale date is required");

    const batchLines: SaleBatchInput["lines"] = [];
    for (const t of selected) {
      const line = lines[t.id];
      const priceStr = (line?.price ?? "").trim().replace(",", ".");
      if (!/^\d+(\.\d{1,2})?$/.test(priceStr)) {
        return setError(`Sale price for ${t.code} is not a valid amount`);
      }
      const feesStr = (line?.fees ?? "").trim().replace(",", ".") || "0";
      if (!/^\d+(\.\d{1,2})?$/.test(feesStr)) {
        return setError(`Selling fees for ${t.code} is not a valid amount`);
      }
      batchLines.push({
        ticketId: t.id,
        salePriceCents: Math.round(parseFloat(priceStr) * 100),
        sellingFeesCents: Math.round(parseFloat(feesStr) * 100),
      });
    }

    const input: SaleBatchInput = {
      lines: batchLines,
      platformId,
      saleDate,
      paymentStatus,
      buyerReference: buyerReference || null,
      notes: notes || null,
    };
    setSaving(true);
    try {
      const sales = await api.createSalesBatch(input);
      if (sales.length === 1) {
        toast.success(`${sales[0].code} recorded - ${sales[0].ticketCode} marked as sold`);
      } else {
        toast.success(
          `${sales.length} sales recorded (${sales[0].code}–${sales[sales.length - 1].code}) - ${sales.length} tickets marked as sold`,
        );
      }
      onCreated();
    } catch (e) {
      setError(errMsg(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal open={open} onClose={onClose} title="New sale" width="max-w-2xl">
      {step === "pick" ? (
        <div>
          <Field
            label="Find tickets to sell"
            required
            hint="Only Available and Listed tickets can be sold. Add as many as you like to this one sale."
          >
            <Input
              autoFocus
              placeholder="Search by code, event, seat..."
              value={ticketQuery}
              onChange={(e) => setTicketQuery(e.target.value)}
            />
          </Field>
          <div className="mt-3 max-h-64 divide-y divide-slate-100 dark:divide-slate-800 overflow-y-auto rounded-lg border border-slate-200 dark:border-slate-800">
            {visibleOptions.length === 0 ? (
              <p className="p-4 text-center text-sm text-slate-400 dark:text-slate-500">
                {ticketQuery ? "No matching available/listed tickets" : "Start typing to search your inventory"}
              </p>
            ) : (
              visibleOptions.map((t) => (
                <button
                  key={t.id}
                  className="flex w-full items-center justify-between gap-3 px-3 py-2 text-left hover:bg-slate-50 dark:hover:bg-slate-800/60"
                  onClick={() => addTicket(t)}
                >
                  <span className="min-w-0">
                    <span className="block truncate text-sm font-medium text-slate-800 dark:text-slate-200">{t.code} &middot; {t.eventName}</span>
                    <span className="block truncate text-xs text-slate-400 dark:text-slate-500">
                      {[t.section, t.rowLabel, t.seat].filter(Boolean).join(" / ") || "No seat info"}
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

          {selected.length > 0 && (
            <div className="mt-4">
              <p className="label mb-1.5">Selected ({selected.length})</p>
              <div className="flex flex-wrap gap-1.5">
                {selected.map((t) => (
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
        </div>
      ) : (
        <>
          <div className="mb-3 flex items-center justify-between">
            <p className="label mb-0">Selected tickets ({selected.length})</p>
            <button type="button" className="text-xs font-medium text-brand-600 dark:text-brand-400 hover:underline" onClick={() => setStep("pick")}>
              + Add more tickets
            </button>
          </div>

          <div className="mb-3 flex flex-wrap items-end gap-2 rounded-lg bg-slate-50 dark:bg-slate-800/60 p-3">
            <div className="w-28">
              <span className="label">Quick-fill price</span>
              <Input inputMode="decimal" placeholder="0.00" value={bulkPrice} onChange={(e) => setBulkPrice(e.target.value)} />
            </div>
            <Button type="button" variant="secondary" disabled={!bulkPrice.trim()} onClick={applyBulkPrice}>
              Apply to all
            </Button>
            <div className="ml-4 w-24">
              <span className="label">Quick-fill fees</span>
              <Input inputMode="decimal" placeholder="0.00" value={bulkFees} onChange={(e) => setBulkFees(e.target.value)} />
            </div>
            <Button type="button" variant="secondary" disabled={!bulkFees.trim()} onClick={applyBulkFees}>
              Apply to all
            </Button>
            <p className="w-full text-xs text-slate-400 dark:text-slate-500">
              Applying overwrites any price/fees already entered below for every selected ticket.
            </p>
          </div>

          <div className="max-h-52 divide-y divide-slate-100 dark:divide-slate-800 overflow-y-auto rounded-lg border border-slate-200 dark:border-slate-800">
            {selected.map((t) => {
              const line = lines[t.id] ?? { price: "", fees: "0" };
              return (
                <div key={t.id} className="flex items-center gap-2 px-3 py-2">
                  <div className="min-w-0 flex-1">
                    <p className="truncate text-sm font-medium text-slate-800 dark:text-slate-200">{t.code}</p>
                    <p className="truncate text-xs text-slate-400 dark:text-slate-500">
                      Cost {formatMoney(t.totalCostCents, t.currency)}
                      {[t.section, t.rowLabel, t.seat].some(Boolean)
                        ? ` · ${[t.section, t.rowLabel, t.seat].filter(Boolean).join(" / ")}`
                        : ""}
                    </p>
                  </div>
                  {/* Persistent currency label (1.6.0 audit UX finding: a
                      placeholder alone disappears the moment a value is
                      typed, which is exactly when it's most useful to still
                      see what currency these two fields are in). One label
                      covers both - price and fees on one ticket are always
                      the same currency (copied from the ticket itself). */}
                  <span className="w-9 shrink-0 text-center text-xs font-medium text-slate-400 dark:text-slate-500">
                    {t.currency}
                  </span>
                  <div className="w-24 shrink-0">
                    <Input
                      inputMode="decimal"
                      placeholder="Price"
                      value={line.price}
                      onChange={(e) => updateLine(t.id, "price", e.target.value)}
                    />
                  </div>
                  <div className="w-20 shrink-0">
                    <Input
                      inputMode="decimal"
                      placeholder="Fees"
                      value={line.fees}
                      onChange={(e) => updateLine(t.id, "fees", e.target.value)}
                    />
                  </div>
                  <button
                    type="button"
                    className="shrink-0 text-slate-400 dark:text-slate-500 hover:text-red-600 dark:hover:text-red-400"
                    title="Remove from this sale"
                    onClick={() => removeTicket(t.id)}
                  >
                    <IconX className="h-4 w-4" />
                  </button>
                </div>
              );
            })}
          </div>

          {singleCurrency ? (
            <div className="mt-4 grid grid-cols-2 gap-3 rounded-lg bg-slate-50 dark:bg-slate-800/60 px-4 py-3 text-sm">
              <div>
                <p className="text-xs text-slate-400 dark:text-slate-500">Total revenue ({selected.length} ticket{selected.length === 1 ? "" : "s"})</p>
                <p className="font-semibold text-slate-900 dark:text-slate-100">{formatMoney(totals.revenue, singleCurrency)}</p>
              </div>
              <div>
                <p className="text-xs text-slate-400 dark:text-slate-500">Estimated profit</p>
                <p className={`font-semibold ${totals.profit >= 0 ? "text-emerald-600 dark:text-emerald-400" : "text-red-600 dark:text-red-400"}`}>
                  {formatMoney(totals.profit, singleCurrency)}
                </p>
              </div>
            </div>
          ) : (
            // 1.6.0 audit UX finding: previously this case showed no profit
            // preview at all, just a one-line notice. A mixed-currency batch
            // still can't be blended into ONE total (that's a real, correct
            // rule elsewhere in this app - see finance.rs), but each
            // individual currency within it is still summable on its own.
            <div className="mt-4 rounded-lg bg-slate-50 dark:bg-slate-800/60 px-4 py-3 text-sm">
              <p className="mb-2 text-xs text-slate-400 dark:text-slate-500">
                Selected tickets use different currencies - shown separately, never blended into one total:
              </p>
              <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
                {perCurrencyTotals.map((c) => (
                  <div key={c.currency}>
                    <p className="text-xs text-slate-400 dark:text-slate-500">
                      {c.currency} ({c.count} ticket{c.count === 1 ? "" : "s"})
                    </p>
                    <p className="font-semibold text-slate-900 dark:text-slate-100">{formatMoney(c.revenue, c.currency)}</p>
                    <p className={`text-xs font-medium ${c.profit >= 0 ? "text-emerald-600 dark:text-emerald-400" : "text-red-600 dark:text-red-400"}`}>
                      {formatMoney(c.profit, c.currency)} profit
                    </p>
                  </div>
                ))}
              </div>
            </div>
          )}

          <div className="mt-4 grid grid-cols-2 gap-4">
            <LookupSelect
              label="Platform"
              options={platforms}
              value={platformId}
              onChange={setPlatformId}
              onCreate={async (name) => {
                const p = await api.createPlatform(name, "sale");
                setPlatforms((prev) => [...prev, p]);
                return p;
              }}
            />
            <Field label="Sale date" required>
              <Input type="date" value={saleDate} onChange={(e) => setSaleDate(e.target.value)} />
            </Field>
            <Field label="Payment status" hint="A sale can't be created as already refunded">
              <Select value={paymentStatus} onChange={(e) => setPaymentStatus(e.target.value as SalePaymentStatus)}>
                <option value="pending">Pending</option>
                <option value="paid">Paid</option>
              </Select>
            </Field>
            <Field label="Buyer / reference">
              <Input value={buyerReference} onChange={(e) => setBuyerReference(e.target.value)} />
            </Field>
            <div className="col-span-2">
              <Field label="Notes">
                <Textarea rows={2} value={notes} onChange={(e) => setNotes(e.target.value)} />
              </Field>
            </div>
          </div>
        </>
      )}

      {error && <p className="mt-3 text-sm text-red-600 dark:text-red-400">{error}</p>}
      <ModalFooter>
        <Button variant="secondary" onClick={onClose} disabled={saving}>
          Cancel
        </Button>
        {step === "pick" ? (
          <Button variant="primary" onClick={() => setStep("details")} disabled={selected.length === 0}>
            Continue ({selected.length})
          </Button>
        ) : (
          <Button variant="primary" onClick={submit} disabled={saving}>
            {saving ? "Recording..." : `Record ${selected.length} sale${selected.length === 1 ? "" : "s"}`}
          </Button>
        )}
      </ModalFooter>
    </Modal>
  );
}


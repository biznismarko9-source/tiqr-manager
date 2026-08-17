import { useEffect, useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { api, errMsg } from "../lib/api";
import type { EventWithStats, Platform, Sale, SaleBatchInput, SaleEditInput, SalePaymentStatus, Ticket } from "../lib/types";
import { formatDate, formatMoney, formatMoneyOrMixed, formatPercent, todayIso } from "../lib/format";
import {
  Badge,
  Button,
  ConfirmDialog,
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
import { IconPlus, IconReceipt, IconSearch, IconTrash, IconX } from "../components/icons";
import { useToast } from "../lib/toast";

export default function Sales() {
  const toast = useToast();
  const [sales, setSales] = useState<Sale[] | null>(null);
  const [events, setEvents] = useState<EventWithStats[]>([]);
  const [search, setSearch] = useState("");
  const [eventId, setEventId] = useState<number | "">("");
  const [paymentStatus, setPaymentStatus] = useState("");
  const [dateFrom, setDateFrom] = useState("");
  const [dateTo, setDateTo] = useState("");
  const [modalOpen, setModalOpen] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<Sale | null>(null);
  const [deleting, setDeleting] = useState(false);
  const [editTarget, setEditTarget] = useState<Sale | null>(null);
  const [refundTarget, setRefundTarget] = useState<Sale | null>(null);

  useEffect(() => {
    api.listEvents().then(setEvents).catch(() => {});
  }, []);

  const load = () => {
    api
      .listSales({
        search: search || undefined,
        eventId: eventId || undefined,
        paymentStatus: paymentStatus || undefined,
        dateFrom: dateFrom || undefined,
        dateTo: dateTo || undefined,
      })
      .then(setSales)
      .catch((e) => toast.error(errMsg(e)));
  };

  useEffect(() => {
    const t = setTimeout(load, 200);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [search, eventId, paymentStatus, dateFrom, dateTo]);

  const totals = useMemo(() => {
    if (!sales) return null;
    // Refunded sales are history, not revenue - they must not inflate this
    // total. And amounts in different currencies can never be added
    // together, so this only sums when every counted sale shares one.
    const counted = sales.filter((s) => s.paymentStatus !== "refunded");
    const currency =
      counted.length > 0 && counted.every((s) => s.currency === counted[0].currency) ? counted[0].currency : null;
    const sums = counted.reduce(
      (acc, s) => ({
        revenue: acc.revenue + s.salePriceCents,
        profit: acc.profit + s.profitCents,
      }),
      { revenue: 0, profit: 0 },
    );
    return { ...sums, currency, refundedCount: sales.length - counted.length };
  }, [sales]);

  return (
    <div>
      <PageHeader
        title="Sales"
        subtitle="Every ticket you've sold, with profit calculated automatically."
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
        <div className="w-36">
          <span className="label">From</span>
          <Input type="date" value={dateFrom} onChange={(e) => setDateFrom(e.target.value)} />
        </div>
        <div className="w-36">
          <span className="label">To</span>
          <Input type="date" value={dateTo} onChange={(e) => setDateTo(e.target.value)} />
        </div>
        {totals && sales && (
          <p className="ml-auto text-xs text-slate-400 dark:text-slate-500">
            {sales.length} sales &middot; revenue {formatMoneyOrMixed(totals.revenue, totals.currency)} &middot; profit{" "}
            {formatMoneyOrMixed(totals.profit, totals.currency)}
            {totals.refundedCount > 0 ? ` (${totals.refundedCount} refunded, excluded)` : ""}
          </p>
        )}
      </div>

      {sales && sales.length >= 5000 && (
        <div className="mb-4 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-800 dark:border-amber-500/30 dark:bg-amber-500/10 dark:text-amber-400">
          Showing the most recent 5,000 sales that match your filters. Narrow the date range, event, or payment
          filter to see the rest.
        </div>
      )}

      {sales === null ? (
        <LoadingBlock />
      ) : sales.length === 0 ? (
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
        <div className="overflow-x-auto rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-sm">
          <table className="w-full min-w-[1100px] border-collapse">
            <thead className="border-b border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-800/60">
              <tr>
                <th className="th">Sale</th>
                <th className="th">Event / Ticket</th>
                <th className="th">Date</th>
                <th className="th">Platform</th>
                <th className="th text-right">Price</th>
                <th className="th text-right">Fees</th>
                <th className="th text-right">Profit</th>
                <th className="th text-right">Margin</th>
                <th className="th text-right">ROI</th>
                <th className="th">Payment</th>
                <th className="th" />
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
              {sales.map((s) => (
                <tr key={s.id} className="hover:bg-slate-50 dark:hover:bg-slate-800/60">
                  <td className="td font-medium text-slate-900 dark:text-slate-100">
                    {s.code}
                  </td>
                  <td className="td">
                    <Link to={`/events/${s.eventId}`} className="hover:text-brand-700 dark:hover:text-brand-400">
                      {s.eventName}
                    </Link>
                    <p className="text-xs text-slate-400 dark:text-slate-500">{s.ticketCode}</p>
                  </td>
                  <td className="td whitespace-nowrap">{formatDate(s.saleDate)}</td>
                  <td className="td">{s.platformName ?? "-"}</td>
                  <td className="td text-right tabular-nums">{formatMoney(s.salePriceCents, s.currency)}</td>
                  <td className="td text-right tabular-nums">{formatMoney(s.sellingFeesCents, s.currency)}</td>
                  <td
                    className={`td text-right tabular-nums font-medium ${s.profitCents > 0 ? "text-emerald-600 dark:text-emerald-400" : s.profitCents < 0 ? "text-red-600 dark:text-red-400" : ""}`}
                  >
                    {formatMoney(s.profitCents, s.currency)}
                  </td>
                  <td className="td text-right tabular-nums">{formatPercent(s.margin)}</td>
                  <td className="td text-right tabular-nums">{formatPercent(s.roi)}</td>
                  <td className="td">
                    <Badge tone={s.paymentStatus}>{s.paymentStatus}</Badge>
                    {s.paymentStatus === "refunded" && s.refundedAt && (
                      <p className="mt-0.5 text-xs text-slate-400 dark:text-slate-500">
                        {formatDate(s.refundedAt)}
                        {s.refundReason ? ` · ${s.refundReason}` : ""}
                      </p>
                    )}
                  </td>
                  <td className="td">
                    {s.paymentStatus === "refunded" ? (
                      <p className="text-right text-xs text-slate-400 dark:text-slate-500">Locked - refunded</p>
                    ) : (
                      <div className="flex items-center justify-end gap-3">
                        <button
                          className="text-xs font-medium text-brand-600 dark:text-brand-400 hover:underline"
                          onClick={() => setEditTarget(s)}
                        >
                          Edit
                        </button>
                        <button
                          className="text-xs font-medium text-amber-600 dark:text-amber-400 hover:underline"
                          onClick={() => setRefundTarget(s)}
                        >
                          Refund
                        </button>
                        <button
                          className="text-slate-400 dark:text-slate-500 hover:text-red-600 dark:hover:text-red-400"
                          title="Delete sale (returns ticket to available)"
                          onClick={() => setDeleteTarget(s)}
                        >
                          <IconTrash className="h-4 w-4" />
                        </button>
                      </div>
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

      <SaleEditModal
        open={!!editTarget}
        sale={editTarget}
        onClose={() => setEditTarget(null)}
        onSaved={() => {
          setEditTarget(null);
          load();
        }}
      />

      <ConfirmDialog
        open={!!deleteTarget}
        title="Delete this sale?"
        message={
          <>
            Use this only to undo a mistake (e.g. the wrong ticket was picked) - it permanently removes sale{" "}
            <b>{deleteTarget?.code}</b> with no record left behind, and sets ticket{" "}
            <b>{deleteTarget?.ticketCode}</b> back to Available. This cannot be undone.
            <br />
            If a real buyer is returning a ticket, cancel this and use <b>Refund</b> instead - it keeps a record.
          </>
        }
        confirmLabel="Delete sale"
        danger
        busy={deleting}
        onCancel={() => setDeleteTarget(null)}
        onConfirm={async () => {
          if (!deleteTarget) return;
          setDeleting(true);
          try {
            await api.deleteSale(deleteTarget.id);
            toast.success("Sale deleted, ticket is available again");
            setDeleteTarget(null);
            load();
          } catch (e) {
            toast.error(errMsg(e));
          } finally {
            setDeleting(false);
          }
        }}
      />

      <RefundDialog
        sale={refundTarget}
        onClose={() => setRefundTarget(null)}
        onRefunded={() => {
          setRefundTarget(null);
          load();
        }}
      />
    </div>
  );
}

function RefundDialog({
  sale,
  onClose,
  onRefunded,
}: {
  sale: Sale | null;
  onClose: () => void;
  onRefunded: () => void;
}) {
  const toast = useToast();
  const [reason, setReason] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    setReason("");
  }, [sale]);

  if (!sale) return null;

  const confirm = async () => {
    setBusy(true);
    try {
      await api.refundSale(sale.id, reason.trim() || undefined);
      toast.success(`${sale.code} refunded - ${sale.ticketCode} is available again`);
      onRefunded();
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Modal open={!!sale} onClose={onClose} title={`Refund ${sale.code}`} width="max-w-sm">
      <p className="text-sm text-slate-500 dark:text-slate-400">
        Ticket <b>{sale.ticketCode}</b> will return to Available so it can be sold again. The sale itself stays on
        record marked as refunded and is excluded from revenue/profit - it can no longer be edited or deleted
        afterwards. This cannot be undone.
      </p>
      <div className="mt-3">
        <Field label="Reason (optional)">
          <Textarea rows={2} value={reason} onChange={(e) => setReason(e.target.value)} placeholder="e.g. buyer couldn't attend" />
        </Field>
      </div>
      <ModalFooter>
        <Button variant="secondary" onClick={onClose} disabled={busy}>
          Cancel
        </Button>
        <Button variant="danger" onClick={confirm} disabled={busy}>
          {busy ? "Refunding..." : "Refund sale"}
        </Button>
      </ModalFooter>
    </Modal>
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
            <p className="mt-3 text-xs text-slate-400 dark:text-slate-500">Selected tickets use different currencies - totals shown per ticket only.</p>
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

function SaleEditModal({
  open,
  sale,
  onClose,
  onSaved,
}: {
  open: boolean;
  sale: Sale | null;
  onClose: () => void;
  onSaved: () => void;
}) {
  const toast = useToast();
  const [platforms, setPlatforms] = useState<Platform[]>([]);
  const [platformId, setPlatformId] = useState<number | null>(null);
  const [saleDate, setSaleDate] = useState("");
  const [salePrice, setSalePrice] = useState("");
  const [sellingFees, setSellingFees] = useState("");
  const [paymentStatus, setPaymentStatus] = useState<SalePaymentStatus>("pending");
  const [buyerReference, setBuyerReference] = useState("");
  const [notes, setNotes] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!sale) return;
    api.listPlatforms().then(setPlatforms).catch(() => {});
    setPlatformId(sale.platformId);
    setSaleDate(sale.saleDate);
    setSalePrice((sale.salePriceCents / 100).toFixed(2));
    setSellingFees((sale.sellingFeesCents / 100).toFixed(2));
    setPaymentStatus(sale.paymentStatus);
    setBuyerReference(sale.buyerReference ?? "");
    setNotes(sale.notes ?? "");
    setError(null);
  }, [sale]);

  if (!sale) return null;

  const submit = async () => {
    const s = salePrice.trim().replace(",", ".");
    if (!/^\d+(\.\d{1,2})?$/.test(s)) return setError("Sale price is not a valid amount");
    const feesStr = sellingFees.trim().replace(",", ".") || "0";
    if (!/^\d+(\.\d{1,2})?$/.test(feesStr)) return setError("Selling fees is not a valid amount");

    const input: SaleEditInput = {
      platformId,
      saleDate,
      salePriceCents: Math.round(parseFloat(s) * 100),
      sellingFeesCents: Math.round(parseFloat(feesStr) * 100),
      paymentStatus,
      buyerReference: buyerReference || null,
      notes: notes || null,
    };
    setSaving(true);
    setError(null);
    try {
      await api.updateSale(sale.id, input);
      toast.success("Sale updated");
      onSaved();
    } catch (e) {
      setError(errMsg(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal open={open} onClose={onClose} title={`Edit ${sale.code}`}>
      <div className="grid grid-cols-2 gap-4">
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
        <Field label={`Sale price (${sale.currency})`} required>
          <Input inputMode="decimal" value={salePrice} onChange={(e) => setSalePrice(e.target.value)} />
        </Field>
        <Field label="Selling fees">
          <Input inputMode="decimal" value={sellingFees} onChange={(e) => setSellingFees(e.target.value)} />
        </Field>
        <Field label="Payment status" hint="Use the Refund action on the sales list to refund this sale">
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

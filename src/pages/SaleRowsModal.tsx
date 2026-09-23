import { useEffect, useMemo, useState } from "react";
import { api, errMsg } from "../lib/api";
import type { OrderRecord, Platform, SaleBatchInput, SalePaymentStatus, Ticket } from "../lib/types";
import { centsToDecimalString, decimalStringToCents, formatDateNumeric, formatMoney, formatSeatLocation, todayIso } from "../lib/format";
import {
  cellError,
  Input,
  Modal,
  RowFormFooter,
  RowFormTable,
  RowNumber,
  RowRemove,
  Select,
  visibleProblems,
  type RowProblem,
} from "../components/ui";
import AiImportPanel from "../components/AiImportPanel";
import { isIsoDate, matchByName } from "../lib/aiImport";
import { useToast } from "../lib/toast";

/**
 * New Sale, as ROWS inside a modal over the Sales list.
 *
 * 2.45.0: the same row form as New Order and New Pull - marko asked for one
 * way of entering things everywhere. What is genuinely different here, and
 * has to be: **a sale cannot be typed.** An order or a pull is free text, but
 * a sale points at REAL tickets you already own (`SaleBatchLineInput.ticketId`),
 * so the rows are not blank lines you fill in - they are the tickets
 * themselves, pulled in from an order and priced.
 *
 * So "add another row" means "bring in another order's tickets". Everything
 * else reads the same: a row per thing, prices typed down the column, one
 * footer that says what is about to happen.
 *
 * Tickets from SEVERAL orders can sit in one sale - that is what
 * `create_sales_batch` has always accepted, and it is the case the old
 * one-order-at-a-time form made awkward.
 *
 * The sale date is today's, stamped when the form opens, exactly as New Order
 * stamps its purchase date ("datum... by mal byt automaticky"). It stays
 * editable on the sale's own page.
 */

type Row = {
  ticket: Ticket;
  price: string;
  fee: string;
};

const PAYMENT_STATUSES: SalePaymentStatus[] = ["paid", "pending"];

export default function SaleRowsModal({
  open,
  onClose,
  onCreated,
}: {
  open: boolean;
  onClose: () => void;
  onCreated: () => void;
}) {
  const toast = useToast();

  const [platforms, setPlatforms] = useState<Platform[]>([]);
  const [orderQuery, setOrderQuery] = useState("");
  const [orderOptions, setOrderOptions] = useState<OrderRecord[]>([]);
  const [pickOrderId, setPickOrderId] = useState<number | "">("");

  const [rows, setRows] = useState<Row[]>([]);
  const [platformId, setPlatformId] = useState<number | null>(null);
  const [currency, setCurrency] = useState("EUR");
  const [buyer, setBuyer] = useState("");
  const [paymentStatus, setPaymentStatus] = useState<SalePaymentStatus>("paid");
  const [notes, setNotes] = useState("");
  const [saleDate, setSaleDate] = useState(() => todayIso());
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [submitted, setSubmitted] = useState(false);
  const [loadingTickets, setLoadingTickets] = useState(false);

  useEffect(() => {
    if (!open) return;
    api.listPlatforms().then(setPlatforms).catch(() => {});
  }, [open]);

  useEffect(() => {
    if (!open) return;
    setRows([]);
    setSubmitted(false);
    setPickOrderId("");
    setOrderQuery("");
    setPlatformId(null);
    setCurrency("EUR");
    setBuyer("");
    setPaymentStatus("paid");
    setNotes("");
    setSaleDate(todayIso());
    setError(null);
  }, [open]);

  // Only orders that still have something sellable - the same scope the old
  // form used.
  useEffect(() => {
    if (!open) return;
    const t = setTimeout(() => {
      api
        .listOrders({ search: orderQuery || undefined, status: "available,listed" })
        .then((res) => setOrderOptions(res.slice(0, 25)))
        .catch(() => {});
    }, 200);
    return () => clearTimeout(t);
  }, [open, orderQuery]);

  const sellSide = useMemo(
    () => platforms.filter((p) => p.kind === "sale" || p.kind === "both"),
    [platforms],
  );

  async function addFromOrder() {
    if (!pickOrderId) return;
    setLoadingTickets(true);
    setError(null);
    try {
      const tickets = await api.listTickets({
        orderId: Number(pickOrderId),
        status: "available,listed",
        sortBy: "created",
        sortDir: "desc",
      });
      setRows((rs) => {
        const have = new Set(rs.map((r) => r.ticket.id));
        const add = tickets
          .filter((tk) => !have.has(tk.id))
          .map((tk) => ({
            ticket: tk,
            // The listing price is what he already decided to ask for - the
            // best possible default. Falls back to empty rather than to the
            // cost, which would quietly suggest selling at break-even.
            price: tk.listingPriceCents !== null ? centsToDecimalString(tk.listingPriceCents) : "",
            fee: "0",
          }));
        if (add.length === 0) toast.error("Every free ticket from that order is already in the list.");
        return [...rs, ...add];
      });
      setPickOrderId("");
    } catch (e) {
      setError(errMsg(e));
    } finally {
      setLoadingTickets(false);
    }
  }

  function patch(i: number, change: Partial<Row>) {
    setRows((rs) => rs.map((r, k) => (k === i ? { ...r, ...change } : r)));
  }

  const totals = useMemo(() => {
    let revenue = 0, fees = 0, cost = 0;
    for (const r of rows) {
      revenue += decimalStringToCents(r.price) ?? 0;
      fees += decimalStringToCents(r.fee) ?? 0;
      cost += r.ticket.totalCostCents;
    }
    return { revenue, fees, cost, profit: revenue - fees - cost };
  }, [rows]);

  // Every problem with every row, not just the first. A Sales row IS a real
  // ticket, so the only things that can be wrong here are the two amounts
  // typed into it.
  const problems = useMemo<RowProblem[]>(() => {
    const out: RowProblem[] = [];
    rows.forEach((r, i) => {
      if (!r.price.trim()) out.push({ row: i, field: "price", message: "sale price is missing", kind: "missing" });
      else if (decimalStringToCents(r.price) === null)
        out.push({ row: i, field: "price", message: `price “${r.price}” is not a valid amount`, kind: "invalid" });
      if (r.fee.trim() && decimalStringToCents(r.fee) === null)
        out.push({ row: i, field: "fee", message: `fees “${r.fee}” is not a valid amount`, kind: "invalid" });
    });
    return out;
  }, [rows]);
  const shown = visibleProblems(problems, submitted);

  async function submit() {
    setError(null);
    setSubmitted(true);
    if (rows.length === 0) return setError("Add at least one ticket from an order");
    if (!currency.trim()) return setError("Currency is required");
    // Every bad cell is already outlined and the footer counts them.
    if (problems.length > 0) return;

    const input: SaleBatchInput = {
      lines: rows.map((r) => ({
        ticketId: r.ticket.id,
        salePriceCents: decimalStringToCents(r.price) ?? 0,
        sellingFeesCents: decimalStringToCents(r.fee) ?? 0,
      })),
      platformId,
      saleDate,
      paymentStatus,
      buyerReference: buyer.trim() || null,
      notes: notes.trim() || null,
      currency: currency.trim().toUpperCase(),
    };

    setSaving(true);
    try {
      const sales = await api.createSalesBatch(input);
      toast.success(
        sales.length === 1
          ? `${sales[0].code} recorded`
          : `${sales.length} sales recorded (${sales[0].code}–${sales[sales.length - 1].code})`,
      );
      onCreated();
    } catch (e) {
      setError(errMsg(e));
    } finally {
      setSaving(false);
    }
  }

  return (
    <Modal open={open} onClose={onClose} title="New sale" width="max-w-[min(1560px,94vw)]">
      {/* 2.47.0: marko asked for the photo import on ALL four forms, not just
          New Order. A sale's rows ARE real tickets, so a screenshot cannot
          conjure them - what it CAN fill is everything that describes the
          sale itself (date, marketplace, buyer, payment state) plus the price
          per ticket, which is otherwise typed once per row. `multiGroup` is
          deliberately not passed: there are no rows here for groups to
          become. */}
      <AiImportPanel
        kind="sale"
        className="mb-4"
        onApply={({ fields }) => {
          if (isIsoDate(fields.saleDate)) setSaleDate(fields.saleDate);
          if (fields.currency) setCurrency(fields.currency.trim().toUpperCase());
          if (fields.buyerReference) setBuyer(fields.buyerReference);
          const platform = matchByName(platforms, fields.marketplace ?? fields.platform);
          if (platform) setPlatformId(platform.id);
          // Only one of the app's own two states is ever taken, never inferred.
          if (PAYMENT_STATUSES.includes((fields.paymentStatus ?? "") as SalePaymentStatus)) {
            setPaymentStatus(fields.paymentStatus as SalePaymentStatus);
          }
          // A price read off the screenshot fills every row that has none yet;
          // a price marko already typed is never overwritten.
          if (fields.salePrice) {
            setRows((rs) => rs.map((r) => (r.price.trim() ? r : { ...r, price: fields.salePrice })));
          }
          if (fields.sellingFees) {
            setRows((rs) => rs.map((r) => (r.fee.trim() ? r : { ...r, fee: fields.sellingFees })));
          }
          toast.info("Sale details filled in from the image - check them, then record.");
        }}
      />
      <div className="mb-4 flex flex-wrap items-end gap-3">
        <div className="min-w-[210px] flex-1">
          <span className="label">Find an order</span>
          <Input
            value={orderQuery}
            onChange={(e) => setOrderQuery(e.target.value)}
            placeholder="Order code, event, ticket code…"
          />
        </div>
        <div className="min-w-[230px] flex-1">
          <span className="label">Order</span>
          <Select value={pickOrderId} onChange={(e) => setPickOrderId(e.target.value ? Number(e.target.value) : "")}>
            <option value="">Select an order…</option>
            {orderOptions.map((o) => (
              <option key={o.id} value={o.id}>
                {o.code} · {o.eventName} · {o.availableCount + o.listedCount} voľných
              </option>
            ))}
          </Select>
        </div>
        <button
          type="button"
          onClick={addFromOrder}
          disabled={!pickOrderId || loadingTickets}
          className="mb-0.5 rounded-lg bg-brand-600 px-3 py-2.5 text-sm font-semibold text-white transition disabled:cursor-not-allowed disabled:opacity-50"
        >
          {loadingTickets ? "Loading…" : "Add its tickets"}
        </button>
        <span className="pb-2 text-xs text-slate-500 dark:text-slate-400">
          Dátum predaja: {formatDateNumeric(saleDate)}
        </span>
      </div>

      <div className="mb-4 flex flex-wrap items-end gap-3">
        <div className="w-44">
          <span className="label">Platform</span>
          <Select
            value={platformId ?? ""}
            onChange={(e) => setPlatformId(e.target.value ? Number(e.target.value) : null)}
          >
            <option value="">—</option>
            {sellSide.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name}
              </option>
            ))}
          </Select>
        </div>
        <div className="w-28">
          <span className="label">Currency</span>
          <Input value={currency} onChange={(e) => setCurrency(e.target.value.toUpperCase())} />
        </div>
        <div className="w-40">
          <span className="label">Payment status</span>
          <Select value={paymentStatus} onChange={(e) => setPaymentStatus(e.target.value as SalePaymentStatus)}>
            {PAYMENT_STATUSES.map((s) => (
              <option key={s} value={s}>
                {s === "paid" ? "Paid" : "Awaiting payment"}
              </option>
            ))}
          </Select>
        </div>
        <div className="min-w-[150px] flex-1">
          <span className="label">Buyer</span>
          <Input value={buyer} onChange={(e) => setBuyer(e.target.value)} placeholder="optional" />
        </div>
        <div className="min-w-[150px] flex-1">
          <span className="label">Notes</span>
          <Input value={notes} onChange={(e) => setNotes(e.target.value)} placeholder="optional" />
        </div>
      </div>

      {rows.length === 0 ? (
        <div className="rounded-xl border border-dashed border-line px-4 py-8 text-center text-sm text-slate-500 dark:text-slate-400">
          No tickets yet. Pick an order above and add its free tickets — you can pull them from several orders
          into one sale.
        </div>
      ) : (
        <RowFormTable
          head={["Ticket", "Event", "Seat", "Cost", "Sale/ea", "Fees", "Profit"]}
        rightAlign={[3, 4, 5, 6]}
          onAdd={addFromOrder}
          addLabel="Add tickets from another order"
        >
          {rows.map((r, i) => {
            const price = decimalStringToCents(r.price) ?? 0;
            const fee = decimalStringToCents(r.fee) ?? 0;
            const profit = price - fee - r.ticket.totalCostCents;
            return (
              <tr key={r.ticket.id}>
                <RowNumber n={i + 1} />
                <td className="td-c w-[170px] font-medium text-slate-900 dark:text-slate-100">{r.ticket.code}</td>
                <td className="td-c w-[360px] truncate" title={r.ticket.eventName}>
                  {r.ticket.eventName}
                </td>
                <td className="td-c w-[220px]">
                  {formatSeatLocation(r.ticket.section, r.ticket.rowLabel, r.ticket.seat)}
                </td>
                <td className="td-c w-[150px] text-right tabular-nums">
                  {formatMoney(r.ticket.totalCostCents, r.ticket.currency)}
                </td>
                <td className="td-c w-[160px]">
                  <Input
                    value={r.price}
                    onChange={(e) => patch(i, { price: e.target.value })}
                    placeholder="245,00"
                    className={`text-right ${cellError(shown, i, "price")}`}
                    aria-label={`Price, ${r.ticket.code}`}
                  />
                </td>
                <td className="td-c w-[150px]">
                  <Input
                    value={r.fee}
                    onChange={(e) => patch(i, { fee: e.target.value })}
                    className={`text-right ${cellError(shown, i, "fee")}`}
                    aria-label={`Fees, ${r.ticket.code}`}
                  />
                </td>
                <td
                  className={`td-c w-[160px] text-right tabular-nums ${
                    profit > 0 ? "text-emerald-600 dark:text-emerald-400" : profit < 0 ? "text-red-600 dark:text-red-400" : ""
                  }`}
                >
                  {formatMoney(profit, currency)}
                </td>
                <RowRemove
                  show
                  onRemove={() => setRows((rs) => rs.filter((_, k) => k !== i))}
                  label={`Remove ${r.ticket.code}`}
                />
              </tr>
            );
          })}
        </RowFormTable>
      )}

      <RowFormFooter
        error={error}
        problems={shown}
        saving={saving}
        onCancel={onClose}
        onSubmit={submit}
        submitLabel={rows.length > 1 ? `Record ${rows.length} sales` : "Record sale"}
        summary={
          rows.length === 0 ? (
            "No tickets"
          ) : (
            <>
              {rows.length} {rows.length === 1 ? "ticket" : "tickets"} ·{" "}
              {formatMoney(totals.revenue, currency)} revenue ·{" "}
              <span className={totals.profit >= 0 ? "text-emerald-600 dark:text-emerald-400" : "text-red-600 dark:text-red-400"}>
                {formatMoney(totals.profit, currency)} profit
              </span>
            </>
          )
        }
      />
    </Modal>
  );
}

import { useEffect, useMemo, useState } from "react";
import { api, errMsg } from "../lib/api";
import type { OrderRecord, Platform, SaleBatchInput, SalePaymentStatus, Ticket } from "../lib/types";
import { centsToDecimalString, decimalStringToCents, formatDateNumeric, formatMoney, formatSeatLocation, todayIso } from "../lib/format";
import { Input, Modal, RowFormFooter, RowFormTable, RowRemove, Select } from "../components/ui";
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
  const [loadingTickets, setLoadingTickets] = useState(false);

  useEffect(() => {
    if (!open) return;
    api.listPlatforms().then(setPlatforms).catch(() => {});
  }, [open]);

  useEffect(() => {
    if (!open) return;
    setRows([]);
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
        if (add.length === 0) toast.error("Z tejto objednávky už máš všetky voľné lístky v zozname.");
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

  async function submit() {
    setError(null);
    if (rows.length === 0) return setError("Pridaj aspoň jeden lístok z objednávky");
    if (!currency.trim()) return setError("Mena je povinná");
    for (const r of rows) {
      if (decimalStringToCents(r.price) === null) return setError(`${r.ticket.code}: cena "${r.price}" nie je platná suma`);
      if (decimalStringToCents(r.fee) === null) return setError(`${r.ticket.code}: poplatok "${r.fee}" nie je platná suma`);
    }

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
          ? `${sales[0].code} zapísaný`
          : `${sales.length} predajov zapísaných (${sales[0].code}–${sales[sales.length - 1].code})`,
      );
      onCreated();
    } catch (e) {
      setError(errMsg(e));
    } finally {
      setSaving(false);
    }
  }

  return (
    <Modal open={open} onClose={onClose} title="Nový predaj" width="max-w-6xl">
      <div className="mb-4 flex flex-wrap items-end gap-3">
        <div className="min-w-[210px] flex-1">
          <span className="label">Hľadať objednávku</span>
          <Input
            value={orderQuery}
            onChange={(e) => setOrderQuery(e.target.value)}
            placeholder="Kód objednávky, event, kód lístka…"
          />
        </div>
        <div className="min-w-[230px] flex-1">
          <span className="label">Objednávka</span>
          <Select value={pickOrderId} onChange={(e) => setPickOrderId(e.target.value ? Number(e.target.value) : "")}>
            <option value="">Vyber objednávku...</option>
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
          {loadingTickets ? "Načítavam…" : "Pridať jej lístky"}
        </button>
        <span className="pb-2 text-xs text-slate-500 dark:text-slate-400">
          Dátum predaja: {formatDateNumeric(saleDate)}
        </span>
      </div>

      <div className="mb-4 flex flex-wrap items-end gap-3">
        <div className="w-44">
          <span className="label">Platforma</span>
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
          <span className="label">Mena</span>
          <Input value={currency} onChange={(e) => setCurrency(e.target.value.toUpperCase())} />
        </div>
        <div className="w-40">
          <span className="label">Stav platby</span>
          <Select value={paymentStatus} onChange={(e) => setPaymentStatus(e.target.value as SalePaymentStatus)}>
            {PAYMENT_STATUSES.map((s) => (
              <option key={s} value={s}>
                {s === "paid" ? "Zaplatené" : "Čaká na platbu"}
              </option>
            ))}
          </Select>
        </div>
        <div className="min-w-[150px] flex-1">
          <span className="label">Kupec</span>
          <Input value={buyer} onChange={(e) => setBuyer(e.target.value)} placeholder="nepovinné" />
        </div>
        <div className="min-w-[150px] flex-1">
          <span className="label">Poznámka</span>
          <Input value={notes} onChange={(e) => setNotes(e.target.value)} placeholder="nepovinné" />
        </div>
      </div>

      {rows.length === 0 ? (
        <div className="rounded-xl border border-dashed border-line px-4 py-8 text-center text-sm text-slate-500 dark:text-slate-400">
          Zatiaľ žiadne lístky. Vyber objednávku hore a pridaj jej voľné lístky — môžeš ich pridať aj z viacerých
          objednávok do jedného predaja.
        </div>
      ) : (
        <RowFormTable
          head={["Lístok", "Event", "Sedadlo", "Nákup", "Predaj/ks", "Poplatok", "Zisk"]}
          onAdd={addFromOrder}
          addLabel="Pridať lístky z ďalšej objednávky"
        >
          {rows.map((r, i) => {
            const price = decimalStringToCents(r.price) ?? 0;
            const fee = decimalStringToCents(r.fee) ?? 0;
            const profit = price - fee - r.ticket.totalCostCents;
            return (
              <tr key={r.ticket.id}>
                <td className="td w-[150px] font-medium text-slate-900 dark:text-slate-100">{r.ticket.code}</td>
                <td className="td w-[170px] truncate" title={r.ticket.eventName}>
                  {r.ticket.eventName}
                </td>
                <td className="td w-[140px]">
                  {formatSeatLocation(r.ticket.section, r.ticket.rowLabel, r.ticket.seat)}
                </td>
                <td className="td w-[110px] tabular-nums">{formatMoney(r.ticket.totalCostCents, r.ticket.currency)}</td>
                <td className="td w-[120px]">
                  <Input
                    value={r.price}
                    onChange={(e) => patch(i, { price: e.target.value })}
                    placeholder="245,00"
                    aria-label={`Cena, ${r.ticket.code}`}
                  />
                </td>
                <td className="td w-[110px]">
                  <Input
                    value={r.fee}
                    onChange={(e) => patch(i, { fee: e.target.value })}
                    aria-label={`Poplatok, ${r.ticket.code}`}
                  />
                </td>
                <td
                  className={`td w-[110px] tabular-nums ${
                    profit > 0 ? "text-emerald-600 dark:text-emerald-400" : profit < 0 ? "text-red-600 dark:text-red-400" : ""
                  }`}
                >
                  {formatMoney(profit, currency)}
                </td>
                <RowRemove
                  show
                  onRemove={() => setRows((rs) => rs.filter((_, k) => k !== i))}
                  label={`Odobrať ${r.ticket.code}`}
                />
              </tr>
            );
          })}
        </RowFormTable>
      )}

      <RowFormFooter
        error={error}
        saving={saving}
        onCancel={onClose}
        onSubmit={submit}
        submitLabel={rows.length > 1 ? `Zapísať ${rows.length} predajov` : "Zapísať predaj"}
        summary={
          rows.length === 0 ? (
            "Žiadne lístky"
          ) : (
            <>
              {rows.length} {rows.length === 1 ? "lístok" : rows.length < 5 ? "lístky" : "lístkov"} ·{" "}
              {formatMoney(totals.revenue, currency)} tržba ·{" "}
              <span className={totals.profit >= 0 ? "text-emerald-600 dark:text-emerald-400" : "text-red-600 dark:text-red-400"}>
                {formatMoney(totals.profit, currency)} zisk
              </span>
            </>
          )
        }
      />
    </Modal>
  );
}

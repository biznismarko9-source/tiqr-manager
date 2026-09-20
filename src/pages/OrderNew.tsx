import { useEffect, useMemo, useState } from "react";
import { Link, useNavigate, useSearchParams } from "react-router-dom";
import { api, errMsg } from "../lib/api";
import { matchByName } from "../lib/aiImport";
import type { EventWithStats, OrderInput, Platform } from "../lib/types";
import { decimalStringToCents, formatDateNumeric, formatMoney, todayIso } from "../lib/format";
import { Button, Card, Input, PageHeader, Select, Spinner } from "../components/ui";
import { IconArrowLeft, IconPlus, IconX } from "../components/icons";
import AiImportPanel from "../components/AiImportPanel";
import { parseSeats, TICKET_TYPES } from "./Orders";
import { useToast } from "../lib/toast";

/**
 * 2.42.0 - New Order, rebuilt as ROWS on a full page.
 *
 * marko's brief, in his words: "teraz chcem aby sme mali riadky a nie taketo
 * okna kde si vies vybrat event a ptm riadky podla toho kolko listkov mas,
 * ked budu mat ine sektor, row, seats tak sa vytvoria samostatne zlozky".
 * He then picked the exact shape out of a preview of ten: a full page (not a
 * modal), table rows, rows added with a button, the event chosen once at the
 * top, and the rarely-touched fields visible rather than hidden in a drawer.
 *
 * ELEVEN FIELDS, and deliberately no others - his list: event, quantity,
 * type, section, row, seats, platform, price per ticket, currency, pull,
 * notes. What that list leaves out, and what happens to it:
 *
 * - **Purchase date** is required by the database and is not in the list.
 *   "datum nakupu by mal byt automaticky" - so it is today's date, stamped
 *   without asking. Editing it afterwards is still possible on Order Detail.
 * - **Fees / other costs** are not collected here at all (0). They were
 *   per-order totals in the old modal; the price marko types is now the real
 *   per-ticket cost, full stop.
 * - **Payment status** keeps the old form's default of "paid" - 2.0.70's
 *   reasoning has not changed: most of his orders are already paid when he
 *   types them in.
 * - **Pull fee** is not asked for. The pull switch records WHO pulled it;
 *   the fee is editable on Order Detail, the same screen that has always
 *   owned it.
 *
 * THE SPLIT RULE. One row is one place. Two rows become ONE order only when
 * everything about them matches - section, row, price, currency, type,
 * platform, pull - AND their seats run straight into each other (14-15 and
 * 16 are one order of 14-16; 14-15 and 19 are two). Anything else is its own
 * order, which is exactly what marko asked for: "co nebude sediet a nebude
 * vedla seba tak rozdielne orders".
 *
 * Nothing about how an order is STORED changed. Each group is one ordinary
 * `api.createOrder` call with the same `OrderInput` the modal always sent, so
 * the backend's cost allocation, code generation and ticket creation are
 * untouched - see `insert_order_with_tickets`.
 */

type Row = {
  qty: string;
  ticketType: string;
  section: string;
  rowLabel: string;
  seats: string;
  platformId: number | null;
  price: string;
  currency: string;
  pulled: boolean;
  puller: string;
  notes: string;
};

function blankRow(prev?: Row): Row {
  return {
    qty: "1",
    ticketType: prev?.ticketType ?? "",
    section: "",
    rowLabel: "",
    seats: "",
    // A second row is almost always the same purchase, so the things that
    // describe the PURCHASE rather than the seat carry over.
    platformId: prev?.platformId ?? null,
    price: prev?.price ?? "",
    currency: prev?.currency ?? "EUR",
    pulled: false,
    puller: "",
    notes: "",
  };
}

/** How many tickets a row is worth: the seat list wins when there is one,
 *  because the backend requires one seat per ticket. */
function rowQty(r: Row): number {
  const seats = parseSeats(r.seats);
  if (seats.length > 0) return seats.length;
  const n = parseInt(r.qty, 10);
  return Number.isFinite(n) && n > 0 ? n : 0;
}

/** Everything except the seats themselves. Two rows that differ here can
 *  never be one order - an order carries ONE unit price, one currency, one
 *  platform. */
function sameShape(a: Row, b: Row): boolean {
  const norm = (s: string) => s.trim().toLowerCase();
  return (
    norm(a.section) === norm(b.section) &&
    norm(a.rowLabel) === norm(b.rowLabel) &&
    // Compared as MONEY, not as text: "135,00" and "135.00" are the same
    // price typed two ways, and this app reads both.
    decimalStringToCents(a.price) === decimalStringToCents(b.price) &&
    norm(a.currency) === norm(b.currency) &&
    norm(a.ticketType) === norm(b.ticketType) &&
    a.platformId === b.platformId &&
    a.pulled === b.pulled &&
    norm(a.puller) === norm(b.puller)
  );
}

type Group = { head: Row; rows: Row[]; seats: string[]; qty: number };

export function groupRows(rows: Row[]): Group[] {
  const out: Group[] = [];
  for (const r of rows) {
    const seats = parseSeats(r.seats);
    const numeric = seats.length > 0 && seats.every((s) => /^\d+$/.test(s));
    let merged = false;

    if (numeric) {
      for (const g of out) {
        if (!sameShape(g.head, r)) continue;
        if (g.seats.length === 0 || !g.seats.every((s) => /^\d+$/.test(s))) continue;
        const all = [...g.seats, ...seats].map(Number).sort((x, y) => x - y);
        // Duplicated seat, or a gap - not one block, so not one order.
        const runs = all.every((n, i) => i === 0 || n === all[i - 1] + 1);
        if (!runs) continue;
        g.rows.push(r);
        g.seats = all.map(String);
        g.qty = all.length;
        merged = true;
        break;
      }
    }

    if (!merged) out.push({ head: r, rows: [r], seats, qty: rowQty(r) });
  }
  return out;
}

export default function OrderNew() {
  const navigate = useNavigate();
  const [params] = useSearchParams();
  const toast = useToast();

  const [events, setEvents] = useState<EventWithStats[]>([]);
  const [platforms, setPlatforms] = useState<Platform[]>([]);
  const [ticketTypeOptions, setTicketTypeOptions] = useState<string[]>(TICKET_TYPES);

  const presetEvent = params.get("event");
  const [eventId, setEventId] = useState<number | "">(presetEvent ? Number(presetEvent) : "");
  const [rows, setRows] = useState<Row[]>([blankRow()]);
  const [notes, setNotes] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  // Stamped once, when the page opens. marko: "datum nakupu by mal byt
  // automaticky" - so it is never a field, and never moves under him while
  // he is still typing.
  const [purchaseDate] = useState(() => todayIso());

  useEffect(() => {
    api.listEvents().then(setEvents).catch(() => {});
    api.listPlatforms().then(setPlatforms).catch(() => {});
    api.listTicketTypes().then(setTicketTypeOptions).catch(() => {});
  }, []);

  const purchaseSide = useMemo(
    () => platforms.filter((p) => p.kind === "purchase" || p.kind === "both"),
    [platforms],
  );

  const groups = useMemo(() => groupRows(rows), [rows]);
  const totalTickets = groups.reduce((s, g) => s + g.qty, 0);
  const totalCost = groups.reduce((s, g) => s + g.qty * (decimalStringToCents(g.head.price) ?? 0), 0);
  const oneCurrency = groups.length > 0 && groups.every((g) => g.head.currency === groups[0].head.currency);

  function patch(i: number, change: Partial<Row>) {
    setRows((rs) => rs.map((r, k) => (k === i ? { ...r, ...change } : r)));
  }

  async function submit() {
    setError(null);
    if (!eventId) return setError("Vyber event");
    if (rows.length === 0) return setError("Pridaj aspoň jedno miesto");

    for (const g of groups) {
      const cents = decimalStringToCents(g.head.price);
      if (cents === null) return setError(`Cena "${g.head.price}" nie je platná suma`);
      if (g.qty < 1) return setError("Každý riadok potrebuje počet kusov alebo sedadlá");
      if (g.seats.length > 0 && g.seats.length !== g.qty) {
        return setError(`Sektor ${g.head.section || "?"}: ${g.seats.length} sedadiel, ale ${g.qty} kusov`);
      }
      if (g.head.pulled && !g.head.puller.trim()) return setError("Pri zapnutom pulle treba meno");
      if (!g.head.currency.trim()) return setError("Mena je povinná");
    }

    setSaving(true);
    const made: string[] = [];
    try {
      for (const g of groups) {
        const input: OrderInput = {
          eventId: Number(eventId),
          supplierId: null,
          platformId: g.head.platformId,
          purchaseDate,
          quantity: g.qty,
          unitPriceCents: decimalStringToCents(g.head.price) ?? 0,
          // Not collected on this form any more - the price marko types is
          // the whole per-ticket cost. See this file's doc comment.
          feesCents: 0,
          otherCostsCents: 0,
          currency: g.head.currency.trim().toUpperCase(),
          paymentStatus: "paid",
          notes: [notes.trim(), g.head.notes.trim()].filter(Boolean).join("\n") || null,
          ticketType: g.head.ticketType.trim() || null,
          section: g.head.section.trim() || null,
          rowLabel: g.head.rowLabel.trim() || null,
          tier: null,
          seats: g.seats.length > 0 ? g.seats : null,
        };
        const created = await api.createOrder(input);
        made.push(created.code);

        // Same linking action Order Detail's "Add pull info" has always
        // called. A failure here never undoes the order - it is already
        // real - so it surfaces on its own and the pull can be added by hand.
        if (g.head.pulled && g.head.puller.trim()) {
          try {
            await api.linkPullReceivedToOrder(created.id, g.head.puller.trim(), 0);
          } catch (e) {
            toast.error(`${created.code}: objednávka je vytvorená, pull sa nepodarilo prepojiť - ${errMsg(e)}`);
          }
        }
      }

      toast.success(
        made.length === 1
          ? `Objednávka ${made[0]} vytvorená`
          : `Vytvorené: ${made.join(", ")}`,
      );
      navigate("/orders");
    } catch (e) {
      // Anything already created stays created - say so rather than leaving
      // him guessing which half landed.
      setError(
        made.length > 0
          ? `${errMsg(e)} — už vytvorené a ponechané: ${made.join(", ")}`
          : errMsg(e),
      );
    } finally {
      setSaving(false);
    }
  }

  const headCls = "th whitespace-nowrap";

  return (
    <div>
      <PageHeader
        title="Nová objednávka"
        subtitle={`Jeden riadok = jedno miesto. Dátum nákupu: ${formatDateNumeric(purchaseDate)}.`}
        actions={
          <Link
            to="/orders"
            className="inline-flex items-center gap-1 text-sm text-slate-500 hover:text-slate-800 dark:text-slate-400 dark:hover:text-slate-200"
          >
            <IconArrowLeft className="h-4 w-4" /> Späť na Inventory
          </Link>
        }
      />

      <AiImportPanel
        kind="order"
        className="mb-4"
        onApply={({ fields, group }) => {
          const ev = matchByName(events, fields.eventName);
          if (ev) setEventId(ev.id);
          if (fields.orderReference) {
            setNotes((prev) =>
              prev.includes(fields.orderReference!)
                ? prev
                : [prev.trim(), `Order ref: ${fields.orderReference}`].filter(Boolean).join("\n"),
            );
          }
          // The AI reads one seat block at a time, so it fills the FIRST row
          // rather than guessing how many rows marko meant.
          const platform = matchByName(platforms, fields.platform);
          setRows((rs) => {
            const first = { ...rs[0] };
            if (platform) first.platformId = platform.id;
            if (fields.currency) first.currency = fields.currency.trim().toUpperCase();
            if (group?.quantity) first.qty = group.quantity;
            if (group?.unitPrice) first.price = group.unitPrice;
            if (group?.section) first.section = group.section;
            if (group?.row) first.rowLabel = group.row;
            if (group?.ticketType) first.ticketType = group.ticketType;
            // `group.seats` arrives already expanded to one label per ticket
            // ("21","22",...); this field is the text the user edits, so it
            // is joined back into the same comma form parseSeats reads.
            if (group?.seats?.length) first.seats = group.seats.join(", ");
            return [first, ...rs.slice(1)];
          });
        }}
      />

      <Card className="mb-4 p-4">
        <div className="flex flex-wrap items-end gap-3">
          <div className="min-w-[260px] flex-1">
            <span className="label">Event</span>
            <Select value={eventId} onChange={(e) => setEventId(e.target.value ? Number(e.target.value) : "")}>
              <option value="">Vyber event...</option>
              {events.map((ev) => (
                <option key={ev.id} value={ev.id}>
                  {ev.name}
                  {ev.eventDate ? ` · ${formatDateNumeric(ev.eventDate)}` : ""}
                </option>
              ))}
            </Select>
          </div>
          <div className="min-w-[220px] flex-1">
            <span className="label">Poznámka k celému nákupu</span>
            <Input value={notes} onChange={(e) => setNotes(e.target.value)} placeholder="nepovinné" />
          </div>
        </div>
      </Card>

      <div className="table-shell mb-3">
        <table className="w-full border-collapse">
          <thead>
            <tr>
              <th className={headCls}>Ks</th>
              <th className={headCls}>Typ</th>
              <th className={headCls}>Sektor</th>
              <th className={headCls}>Rad</th>
              <th className={headCls}>Sedadlá</th>
              <th className={headCls}>Platforma</th>
              <th className={headCls}>Cena/ks</th>
              <th className={headCls}>Mena</th>
              <th className={headCls}>Pull</th>
              <th className={headCls}>Poznámka</th>
              <th className={headCls} />
            </tr>
          </thead>
          <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
            {rows.map((r, i) => (
              <tr key={i}>
                <td className="td w-[70px]">
                  <Input
                    type="number"
                    min={1}
                    value={r.qty}
                    onChange={(e) => patch(i, { qty: e.target.value })}
                    aria-label={`Počet kusov, riadok ${i + 1}`}
                  />
                </td>
                <td className="td w-[130px]">
                  <Select value={r.ticketType} onChange={(e) => patch(i, { ticketType: e.target.value })} aria-label="Typ">
                    <option value="">—</option>
                    {ticketTypeOptions.map((t) => (
                      <option key={t} value={t}>
                        {t}
                      </option>
                    ))}
                  </Select>
                </td>
                <td className="td w-[100px]">
                  <Input value={r.section} onChange={(e) => patch(i, { section: e.target.value })} aria-label="Sektor" />
                </td>
                <td className="td w-[80px]">
                  <Input value={r.rowLabel} onChange={(e) => patch(i, { rowLabel: e.target.value })} aria-label="Rad" />
                </td>
                <td className="td w-[120px]">
                  <Input
                    value={r.seats}
                    onChange={(e) => patch(i, { seats: e.target.value })}
                    placeholder="23-24"
                    aria-label="Sedadlá"
                  />
                </td>
                <td className="td w-[150px]">
                  <Select
                    value={r.platformId ?? ""}
                    onChange={(e) => patch(i, { platformId: e.target.value ? Number(e.target.value) : null })}
                    aria-label="Platforma"
                  >
                    <option value="">—</option>
                    {purchaseSide.map((p) => (
                      <option key={p.id} value={p.id}>
                        {p.name}
                      </option>
                    ))}
                  </Select>
                </td>
                <td className="td w-[110px]">
                  <Input
                    value={r.price}
                    onChange={(e) => patch(i, { price: e.target.value })}
                    placeholder="135,00"
                    aria-label="Cena za kus"
                  />
                </td>
                <td className="td w-[90px]">
                  <Input
                    value={r.currency}
                    onChange={(e) => patch(i, { currency: e.target.value.toUpperCase() })}
                    aria-label="Mena"
                  />
                </td>
                <td className="td w-[170px]">
                  {/* 2.42.0: a switch first, a name only when the answer is
                      yes - marko: "daj na pull nejak ze viem kliknut ci ano
                      alebo nie... a ked ano tak si vies napisat meno". The
                      typed name is kept when it is switched off, so flipping
                      back does not lose it; only `pulled` decides. */}
                  <div className="flex items-center gap-2">
                    <button
                      type="button"
                      onClick={() => patch(i, { pulled: !r.pulled })}
                      aria-pressed={r.pulled}
                      aria-label={`Pullnuté cez niekoho, riadok ${i + 1}`}
                      title="Pullnuté cez niekoho?"
                      className={`h-5 w-5 shrink-0 rounded-full transition ${
                        r.pulled
                          ? "bg-brand-600 shadow-[inset_0_0_0_1px_theme(colors.brand.400)]"
                          : "bg-surface-sunken shadow-[inset_0_0_0_1px_theme(colors.slate.700)]"
                      }`}
                    />
                    {r.pulled ? (
                      <Input
                        value={r.puller}
                        onChange={(e) => patch(i, { puller: e.target.value })}
                        placeholder="kto ťahal"
                        aria-label="Kto pullol"
                      />
                    ) : (
                      <span className="text-xs text-slate-500 dark:text-slate-400">nie</span>
                    )}
                  </div>
                </td>
                <td className="td w-[170px]">
                  <Input value={r.notes} onChange={(e) => patch(i, { notes: e.target.value })} aria-label="Poznámka" />
                </td>
                <td className="td w-[46px]">
                  {rows.length > 1 && (
                    <button
                      type="button"
                      onClick={() => setRows((rs) => rs.filter((_, k) => k !== i))}
                      aria-label={`Zmazať riadok ${i + 1}`}
                      className="rounded-md p-1.5 text-slate-500 transition hover:bg-slate-100 hover:text-red-600 dark:text-slate-400 dark:hover:bg-slate-800 dark:hover:text-red-400"
                    >
                      <IconX className="h-4 w-4" />
                    </button>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <Button variant="secondary" onClick={() => setRows((rs) => [...rs, blankRow(rs[rs.length - 1])])}>
        <IconPlus className="h-4 w-4" /> Ďalšie miesto
      </Button>

      {error && (
        <p className="mt-4 rounded-lg bg-red-50 px-3 py-2 text-sm text-red-700 dark:bg-red-500/10 dark:text-red-300">
          {error}
        </p>
      )}

      <div className="mt-5 flex flex-wrap items-center gap-3 border-t border-line pt-4">
        <span className="text-sm text-slate-500 dark:text-slate-400">
          {totalTickets} {totalTickets === 1 ? "lístok" : totalTickets < 5 ? "lístky" : "lístkov"}
          {oneCurrency && totalCost > 0 ? ` · ${formatMoney(totalCost, groups[0].head.currency)}` : ""}
        </span>
        <span className="ml-auto flex gap-2">
          <Button variant="secondary" onClick={() => navigate("/orders")} disabled={saving}>
            Zrušiť
          </Button>
          <Button variant="primary" onClick={submit} disabled={saving}>
            {saving ? <Spinner className="h-4 w-4" /> : null}
            {groups.length > 1
              ? `Vytvoriť ${groups.length} ${groups.length < 5 ? "objednávky" : "objednávok"}`
              : "Vytvoriť objednávku"}
          </Button>
        </span>
      </div>
    </div>
  );
}

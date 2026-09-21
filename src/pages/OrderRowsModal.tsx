import { useEffect, useMemo, useState } from "react";
import { api, errMsg } from "../lib/api";
import { matchByName } from "../lib/aiImport";
import type { EventWithStats, OrderInput, OrderRecord, Platform } from "../lib/types";
import { decimalStringToCents, formatDateNumeric, formatMoney, todayIso } from "../lib/format";
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
import { parseSeats, TICKET_TYPES } from "./Orders";
import { useToast } from "../lib/toast";

/**
 * New Order, as ROWS inside a modal over the Inventory list.
 *
 * 2.42.0 built this as rows on a full page, because that is the shape marko
 * picked out of a preview of ten. 2.45.0 moves it back into a modal at his
 * own request - "nechcem aby to ked kliknes na new order... bolo ako svoje
 * okno ale tak aby to bolo nad orders... a podtym rozmazane vidis ostatne".
 * The ROWS are the part he wanted; the full page was not. The list stays
 * visible and blurred underneath, which is what `Modal`'s backdrop already
 * does for every other form in the app.
 *
 * ELEVEN FIELDS, and deliberately no others - marko's list: event, quantity,
 * type, section, row, seats, platform, price per ticket, currency, pull,
 * notes. What that list leaves out, and where it went:
 *
 * - **Purchase date** is required by the database and is not in the list.
 *   "datum nakupu by mal byt automaticky" - so it is today's date, stamped
 *   when the form opens. Editable afterwards on Order Detail.
 * - **Fees / other costs** are not collected (0). The price marko types is
 *   the whole per-ticket cost.
 * - **Payment status** keeps 2.0.70's "paid" default.
 * - **Pull fee** is not asked for; the switch records WHO pulled it, and the
 *   fee stays on Order Detail, which has always owned it.
 *
 * THE SPLIT RULE. One row is one place. Two rows become ONE order only when
 * everything about them matches - section, row, price, currency, type,
 * platform, pull - AND their seats run straight into each other (14-15 and
 * 16 are one order of 14-16; 14-15 and 19 are two). Anything else is its own
 * order: "co nebude sediet a nebude vedla seba tak rozdielne orders".
 *
 * Nothing about how an order is STORED changed. Each group is one ordinary
 * `api.createOrder` call with the same `OrderInput` the old modal sent.
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

/** Every problem with every row, not just the first. Deliberately per ROW:
 *  a group is something this form derives, so an error that named one would
 *  point at nothing you could click. */
function validate(rows: Row[]): RowProblem[] {
  const out: RowProblem[] = [];
  rows.forEach((r, i) => {
    const seats = parseSeats(r.seats);
    // Seats win over the typed count - that is `rowQty`'s own rule, and the
    // Ks cell shows the seat count read-only when they are present, so the
    // two can no longer disagree and there is nothing here to check.
    if (seats.length === 0) {
      const qty = parseInt(r.qty, 10);
      if (!r.qty.trim()) out.push({ row: i, field: "qty", message: "chýba počet kusov alebo sedadlá", kind: "missing" });
      else if (!Number.isFinite(qty) || qty < 1)
        out.push({ row: i, field: "qty", message: "počet kusov musí byť aspoň 1", kind: "invalid" });
    }
    if (!r.price.trim()) out.push({ row: i, field: "price", message: "chýba cena za kus", kind: "missing" });
    else if (decimalStringToCents(r.price) === null)
      out.push({ row: i, field: "price", message: `cena „${r.price}“ nie je platná suma`, kind: "invalid" });
    if (!r.currency.trim()) out.push({ row: i, field: "currency", message: "chýba mena", kind: "missing" });
    if (r.pulled && !r.puller.trim())
      out.push({ row: i, field: "puller", message: "pri zapnutom pulle treba meno", kind: "missing" });
  });
  return out;
}

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

export default function OrderRowsModal({
  open,
  presetEventId,
  onClose,
  onCreated,
}: {
  open: boolean;
  presetEventId?: number;
  onClose: () => void;
  onCreated: (created: OrderRecord[]) => void;
}) {
  const toast = useToast();

  const [events, setEvents] = useState<EventWithStats[]>([]);
  const [platforms, setPlatforms] = useState<Platform[]>([]);
  const [ticketTypeOptions, setTicketTypeOptions] = useState<string[]>(TICKET_TYPES);

  const [eventId, setEventId] = useState<number | "">("");
  const [rows, setRows] = useState<Row[]>([blankRow()]);
  const [notes, setNotes] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [submitted, setSubmitted] = useState(false);
  // Stamped when the form opens. marko: "datum nakupu by mal byt
  // automaticky" - so it is never a field, and never moves under him while
  // he is still typing.
  const [purchaseDate, setPurchaseDate] = useState(() => todayIso());

  useEffect(() => {
    if (!open) return;
    api.listEvents().then(setEvents).catch(() => {});
    api.listPlatforms().then(setPlatforms).catch(() => {});
    api.listTicketTypes().then(setTicketTypeOptions).catch(() => {});
  }, [open]);

  // Every open starts clean - this component is mounted permanently by the
  // Inventory page and only its inner Modal comes and goes, so without this
  // the previous order's rows would still be sitting there.
  useEffect(() => {
    if (!open) return;
    setEventId(presetEventId ?? "");
    setRows([blankRow()]);
    setNotes("");
    setError(null);
    setPurchaseDate(todayIso());
  }, [open, presetEventId]);

  const purchaseSide = useMemo(
    () => platforms.filter((p) => p.kind === "purchase" || p.kind === "both"),
    [platforms],
  );

  const groups = useMemo(() => groupRows(rows), [rows]);
  const problems = useMemo(() => validate(rows), [rows]);
  const shown = visibleProblems(problems, submitted);
  const totalTickets = groups.reduce((s, g) => s + g.qty, 0);
  const totalCost = groups.reduce((s, g) => s + g.qty * (decimalStringToCents(g.head.price) ?? 0), 0);
  const oneCurrency = groups.length > 0 && groups.every((g) => g.head.currency === groups[0].head.currency);

  function patch(i: number, change: Partial<Row>) {
    setRows((rs) => rs.map((r, k) => (k === i ? { ...r, ...change } : r)));
  }

  async function submit() {
    setError(null);
    setSubmitted(true);
    if (!eventId) return setError("Vyber event");
    if (rows.length === 0) return setError("Pridaj aspoň jedno miesto");
    // Every bad cell is already outlined and the footer counts them.
    if (problems.length > 0) return;

    setSaving(true);
    const made: OrderRecord[] = [];
    try {
      for (const g of groups) {
        const input: OrderInput = {
          eventId: Number(eventId),
          supplierId: null,
          platformId: g.head.platformId,
          purchaseDate,
          quantity: g.qty,
          unitPriceCents: decimalStringToCents(g.head.price) ?? 0,
          // Not collected on this form - the price marko types is the whole
          // per-ticket cost. See this file's doc comment.
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
        made.push(created);

        // Same linking action Order Detail's "Add pull info" has always
        // called. A failure here never undoes the order - it is already
        // real - so it surfaces on its own.
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
          ? `Objednávka ${made[0].code} vytvorená`
          : `Vytvorené: ${made.map((m) => m.code).join(", ")}`,
      );
      onCreated(made);
    } catch (e) {
      // Anything already created stays created - say so rather than leaving
      // him guessing which half landed.
      setError(
        made.length > 0
          ? `${errMsg(e)} — už vytvorené a ponechané: ${made.map((m) => m.code).join(", ")}`
          : errMsg(e),
      );
    } finally {
      setSaving(false);
    }
  }

  return (
    <Modal open={open} onClose={onClose} title="Nová objednávka" width="max-w-6xl">
      <AiImportPanel
        kind="order"
        className="mb-4"
        multiGroup
        onApply={({ fields, groups }) => {
          const ev = matchByName(events, fields.eventName);
          if (ev) setEventId(ev.id);
          if (fields.orderReference) {
            setNotes((prev) =>
              prev.includes(fields.orderReference!)
                ? prev
                : [prev.trim(), `Order ref: ${fields.orderReference}`].filter(Boolean).join("\n"),
            );
          }
          const platform = matchByName(platforms, fields.platform);
          const currency = fields.currency ? fields.currency.trim().toUpperCase() : null;

          setRows((rs) => {
            // 2.47.0: ONE ROW PER GROUP. marko: "ked vo fotke bude viac
            // sektorov rows atd tam to bude vediet pekne priradit ku inej
            // objednavke aby sa to nemiesalo". Nothing merges them here - the
            // rows are laid down as the image read them, and `groupRows`
            // below decides what is one order and what is two, by exactly the
            // same rule it applies to rows marko typed himself.
            const made: Row[] = groups.map((g) => {
              const r = blankRow();
              if (platform) r.platformId = platform.id;
              if (currency) r.currency = currency;
              if (g.quantity) r.qty = g.quantity;
              if (g.unitPrice) r.price = g.unitPrice;
              if (g.section) r.section = g.section;
              if (g.row) r.rowLabel = g.row;
              if (g.ticketType) r.ticketType = g.ticketType;
              // `g.seats` arrives already expanded to one label per ticket
              // ("21","22",...); this field is the text marko edits, so it is
              // joined back into the same comma form parseSeats reads.
              if (g.seats?.length) r.seats = g.seats.join(", ");
              return r;
            });

            // No ticket detail in the image: fill only what the fields gave,
            // into the row already on screen, rather than wiping it.
            if (made.length === 0) {
              const first = { ...rs[0] };
              if (platform) first.platformId = platform.id;
              if (currency) first.currency = currency;
              return [first, ...rs.slice(1)];
            }

            // A form marko has already typed into keeps what he typed; the
            // read rows are appended. An untouched form is replaced outright,
            // so the image does not leave an empty row sitting on top.
            const untouched =
              rs.length === 1 && !rs[0].section && !rs[0].rowLabel && !rs[0].seats && !rs[0].price;
            return untouched ? made : [...rs, ...made];
          });
        }}
      />

      <div className="mb-4 flex flex-wrap items-end gap-3">
        <div className="min-w-[250px] flex-1">
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
        <div className="min-w-[200px] flex-1">
          <span className="label">Poznámka k celému nákupu</span>
          <Input value={notes} onChange={(e) => setNotes(e.target.value)} placeholder="nepovinné" />
        </div>
        <span className="pb-2 text-xs text-slate-500 dark:text-slate-400">
          Dátum nákupu: {formatDateNumeric(purchaseDate)}
        </span>
      </div>

      <RowFormTable
        head={["Ks", "Typ", "Sektor", "Rad", "Sedadlá", "Platforma", "Cena/ks", "Mena", "Pull", "Poznámka"]}
        rightAlign={[0, 6]}
        onAdd={() => setRows((rs) => [...rs, blankRow(rs[rs.length - 1])])}
        addLabel="Ďalšie miesto"
      >
        {rows.map((r, i) => (
          <tr key={i}>
            <RowNumber n={i + 1} />
            <td className="td-c w-[56px]">
              {/* 2.46.1: once seats are typed, the count IS the number of
                  seats (see rowQty). marko: "odstran tam to puzdro a urob to
                  tak ze tam vidno ten pocet" - so it is the number itself, not
                  a box holding a number you cannot edit. A greyed-out input is
                  a control that refuses you; a figure is just the answer. */}
              {parseSeats(r.seats).length > 0 ? (
                <span
                  className="block px-1 text-right text-sm font-medium tabular-nums text-slate-900 dark:text-slate-100"
                  title="Počet vychádza zo sedadiel"
                >
                  {parseSeats(r.seats).length}
                </span>
              ) : (
                <Input
                  type="number"
                  min={1}
                  value={r.qty}
                  onChange={(e) => patch(i, { qty: e.target.value })}
                  className={`text-right ${cellError(shown, i, "qty")}`}
                  aria-label={`Počet kusov, riadok ${i + 1}`}
                />
              )}
            </td>
            <td className="td-c w-[110px]">
              <Select value={r.ticketType} onChange={(e) => patch(i, { ticketType: e.target.value })} aria-label="Typ">
                <option value="">—</option>
                {ticketTypeOptions.map((t) => (
                  <option key={t} value={t}>
                    {t}
                  </option>
                ))}
              </Select>
            </td>
            <td className="td-c w-[92px]">
              <Input value={r.section} onChange={(e) => patch(i, { section: e.target.value })} aria-label="Sektor" />
            </td>
            <td className="td-c w-[64px]">
              <Input value={r.rowLabel} onChange={(e) => patch(i, { rowLabel: e.target.value })} aria-label="Rad" />
            </td>
            <td className="td-c w-[108px]">
              <Input
                value={r.seats}
                onChange={(e) => patch(i, { seats: e.target.value })}
                placeholder="23-24"
                className={cellError(shown, i, "seats")}
                aria-label="Sedadlá"
              />
            </td>
            <td className="td-c w-[130px]">
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
            <td className="td-c w-[96px]">
              <Input
                value={r.price}
                onChange={(e) => patch(i, { price: e.target.value })}
                placeholder="135,00"
                className={`text-right ${cellError(shown, i, "price")}`}
                aria-label="Cena za kus"
              />
            </td>
            <td className="td-c w-[70px]">
              <Input
                value={r.currency}
                onChange={(e) => patch(i, { currency: e.target.value.toUpperCase() })}
                className={cellError(shown, i, "currency")}
                aria-label="Mena"
              />
            </td>
            <td className="td-c w-[152px]">
              {/* A switch first, a name only when the answer is yes - marko:
                  "daj na pull nejak ze viem kliknut ci ano alebo nie... a ked
                  ano tak si vies napisat meno". The typed name is kept when
                  it is switched off, so flipping back does not lose it. */}
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
                    className={cellError(shown, i, "puller")}
                  aria-label="Kto pullol"
                  />
                ) : (
                  <span className="text-xs text-slate-500 dark:text-slate-400">nie</span>
                )}
              </div>
            </td>
            <td className="td-c w-[142px]">
              <Input value={r.notes} onChange={(e) => patch(i, { notes: e.target.value })} aria-label="Poznámka" />
            </td>
            <RowRemove
              show={rows.length > 1}
              onRemove={() => setRows((rs) => rs.filter((_, k) => k !== i))}
              label={`Zmazať riadok ${i + 1}`}
              // Seats are cleared on purpose: a duplicated row is the same
              // block at a different seat, and two rows with identical seats
              // would be the same tickets twice.
              onDuplicate={() => setRows((rs) => [...rs.slice(0, i + 1), { ...rs[i], seats: "" }, ...rs.slice(i + 1)])}
              duplicateLabel={`Duplikovať riadok ${i + 1}`}
            />
          </tr>
        ))}
      </RowFormTable>

      <RowFormFooter
        error={error}
        problems={shown}
        saving={saving}
        onCancel={onClose}
        onSubmit={submit}
        submitLabel={
          groups.length > 1
            ? `Vytvoriť ${groups.length} ${groups.length < 5 ? "objednávky" : "objednávok"}`
            : "Vytvoriť objednávku"
        }
        summary={
          <>
            {totalTickets} {totalTickets === 1 ? "lístok" : totalTickets < 5 ? "lístky" : "lístkov"}
            {oneCurrency && totalCost > 0 ? ` · ${formatMoney(totalCost, groups[0].head.currency)}` : ""}
          </>
        }
      />
    </Modal>
  );
}

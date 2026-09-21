import { useEffect, useMemo, useState } from "react";
import { api, errMsg } from "../lib/api";
import { isIsoDate, matchByName } from "../lib/aiImport";
import type { Platform, Pull, PullInput } from "../lib/types";
import { decimalStringToCents, formatMoney } from "../lib/format";
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
import { useToast } from "../lib/toast";

/**
 * New Pull, as ROWS inside a modal over the Pulls list.
 *
 * 2.45.0: marko - "vsade events, inventory, sales a pulls budu podobne tie
 * vyplnovace udajov". Same shell, same rhythm as New Order: a row per thing,
 * an "add another" underneath, one footer. One row is one pull, so three
 * people asking for the same evening is three rows typed once.
 *
 * A pull is free text BY DESIGN and that does not change here: it is not one
 * of your orders, and the event it names need not exist in your Events list
 * at all - you are buying on someone else's card. So unlike New Order there
 * is no event picker; the event is typed, and a new row inherits it so one
 * evening is not retyped four times.
 *
 * NOT asked for, deliberately: the **transfer deadline**, dropped from this
 * form back in 1.9.8 and set on the pull afterwards; and the ticket price,
 * which was never marko's money - the fee is his own reward (see
 * migrations/005_pulls.sql).
 *
 * Each row is one ordinary `api.createPull`, the same call the old form made.
 */

type Row = {
  buyerName: string;
  eventName: string;
  eventDate: string;
  quantity: string;
  platformId: number | null;
  section: string;
  rowLabel: string;
  seat: string;
  price: string;
  currency: string;
  moreInfo: string;
};

function blankRow(prev?: Row): Row {
  return {
    buyerName: "",
    // The evening, the platform, the fee and the currency describe the JOB,
    // not the person - a second row inherits them and only the buyer changes.
    eventName: prev?.eventName ?? "",
    eventDate: prev?.eventDate ?? "",
    quantity: "1",
    platformId: prev?.platformId ?? null,
    section: "",
    rowLabel: "",
    seat: "",
    price: prev?.price ?? "",
    currency: prev?.currency ?? "EUR",
    moreInfo: "",
  };
}

/** Every problem with every row, not just the first. */
function validate(rows: Row[]): RowProblem[] {
  const out: RowProblem[] = [];
  rows.forEach((r, i) => {
    if (!r.buyerName.trim()) out.push({ row: i, field: "buyerName", message: "chýba, pre koho ťaháš", kind: "missing" });
    if (!r.eventName.trim()) out.push({ row: i, field: "eventName", message: "chýba event", kind: "missing" });
    const q = parseInt(r.quantity, 10);
    if (!r.quantity.trim()) out.push({ row: i, field: "quantity", message: "chýba počet kusov", kind: "missing" });
    else if (!Number.isFinite(q) || q < 1)
      out.push({ row: i, field: "quantity", message: "počet kusov musí byť aspoň 1", kind: "invalid" });
    if (!r.price.trim()) out.push({ row: i, field: "price", message: "chýba tvoja odmena", kind: "missing" });
    else if (decimalStringToCents(r.price) === null)
      out.push({ row: i, field: "price", message: `odmena „${r.price}“ nie je platná suma`, kind: "invalid" });
    if (!r.currency.trim()) out.push({ row: i, field: "currency", message: "chýba mena", kind: "missing" });
    if (r.eventDate && !isIsoDate(r.eventDate))
      out.push({ row: i, field: "eventDate", message: "dátum eventu nie je platný", kind: "invalid" });
  });
  return out;
}

export default function PullRowsModal({
  open,
  onClose,
  onCreated,
}: {
  open: boolean;
  onClose: () => void;
  onCreated: (created: Pull[]) => void;
}) {
  const toast = useToast();
  const [platforms, setPlatforms] = useState<Platform[]>([]);
  const [rows, setRows] = useState<Row[]>([blankRow()]);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [submitted, setSubmitted] = useState(false);

  useEffect(() => {
    if (!open) return;
    api.listPlatforms().then(setPlatforms).catch(() => {});
  }, [open]);

  // Every open starts clean - the list keeps this component mounted and only
  // the inner Modal comes and goes.
  useEffect(() => {
    if (!open) return;
    setRows([blankRow()]);
    setError(null);
    setSubmitted(false);
  }, [open]);

  // A pull is bought on a marketplace - the same purchase side an order uses.
  const purchaseSide = useMemo(
    () => platforms.filter((p) => p.kind === "purchase" || p.kind === "both"),
    [platforms],
  );

  const problems = useMemo(() => validate(rows), [rows]);
  const shown = visibleProblems(problems, submitted);

  const totalTickets = rows.reduce((s, r) => s + (parseInt(r.quantity, 10) || 0), 0);
  const oneCurrency = rows.length > 0 && rows.every((r) => r.currency === rows[0].currency);
  const totalFee = rows.reduce((s, r) => s + (decimalStringToCents(r.price) ?? 0), 0);

  function patch(i: number, change: Partial<Row>) {
    setRows((rs) => rs.map((r, k) => (k === i ? { ...r, ...change } : r)));
  }

  async function submit() {
    setError(null);
    setSubmitted(true);
    // Every bad cell is already outlined and the footer counts them.
    if (problems.length > 0) return;

    setSaving(true);
    const made: Pull[] = [];
    try {
      for (const r of rows) {
        const input: PullInput = {
          buyerName: r.buyerName.trim(),
          eventName: r.eventName.trim(),
          eventDate: r.eventDate || null,
          quantity: parseInt(r.quantity, 10),
          platformId: r.platformId,
          section: r.section.trim() || null,
          rowLabel: r.rowLabel.trim() || null,
          seat: r.seat.trim() || null,
          moreInfo: r.moreInfo.trim() || null,
          priceCents: decimalStringToCents(r.price) ?? 0,
          currency: r.currency.trim().toUpperCase(),
        };
        made.push(await api.createPull(input));
      }
      toast.success(
        made.length === 1 ? `Pull ${made[0].code} vytvorený` : `Vytvorené: ${made.map((m) => m.code).join(", ")}`,
      );
      onCreated(made);
    } catch (e) {
      // Whatever already went in stays in - name it rather than implying a
      // rollback the backend does not offer.
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
    <Modal open={open} onClose={onClose} title="Nový pull" width="max-w-[min(1560px,94vw)]">
      <AiImportPanel
        kind="pull"
        className="mb-4"
        multiGroup
        onApply={({ fields, groups }) => {
          const platform = matchByName(platforms, fields.platform);
          setRows((rs) => {
            // 2.47.0: one row per group, same rule as New Order. Three seat
            // blocks on one screenshot are three pulls, not one pull with the
            // first block's seats and the rest quietly dropped.
            const base = (): Row => {
              const r = blankRow();
              if (fields.eventName) r.eventName = fields.eventName;
              if (isIsoDate(fields.eventDate)) r.eventDate = fields.eventDate;
              if (fields.currency) r.currency = fields.currency.trim().toUpperCase();
              if (platform) r.platformId = platform.id;
              return r;
            };
            const made: Row[] = groups.map((g) => {
              const r = base();
              if (g.quantity) r.quantity = g.quantity;
              if (g.section) r.section = g.section;
              if (g.row) r.rowLabel = g.row;
              // `pulls.seat` is one free-text column, not a row per ticket -
              // so the expanded labels are joined rather than parsed.
              if (g.seats?.length) r.seat = g.seats.join(", ");
              return r;
            });

            if (made.length === 0) {
              const first = { ...rs[0] };
              if (fields.eventName) first.eventName = fields.eventName;
              if (isIsoDate(fields.eventDate)) first.eventDate = fields.eventDate;
              if (fields.currency) first.currency = fields.currency.trim().toUpperCase();
              if (platform) first.platformId = platform.id;
              return [first, ...rs.slice(1)];
            }

            const untouched =
              rs.length === 1 && !rs[0].buyerName && !rs[0].section && !rs[0].seat && !rs[0].price;
            return untouched ? made : [...rs, ...made];
          });
        }}
      />

      <RowFormTable
        head={["Pre koho", "Event", "Dátum eventu", "Ks", "Sektor", "Rad", "Sedadlá", "Platforma", "Tvoja odmena", "Mena", "Poznámka"]}
        rightAlign={[3, 8]}
        onAdd={() => setRows((rs) => [...rs, blankRow(rs[rs.length - 1])])}
        addLabel="Ďalší pull"
      >
        {rows.map((r, i) => (
          <tr key={i}>
            <RowNumber n={i + 1} />
            <td className="td-c w-[160px]">
              <Input
                value={r.buyerName}
                onChange={(e) => patch(i, { buyerName: e.target.value })}
                className={cellError(shown, i, "buyerName")}
                aria-label={`Pre koho, riadok ${i + 1}`}
              />
            </td>
            <td className="td-c w-[190px]">
              <Input
                value={r.eventName}
                onChange={(e) => patch(i, { eventName: e.target.value })}
                className={cellError(shown, i, "eventName")}
                aria-label="Event"
              />
            </td>
            <td className="td-c w-[150px]">
              <Input
                type="date"
                value={r.eventDate}
                onChange={(e) => patch(i, { eventDate: e.target.value })}
                className={cellError(shown, i, "eventDate")}
                aria-label="Dátum eventu"
              />
            </td>
            <td className="td-c w-[78px]">
              <Input
                /* 2.47.1: no stepper - see OrderRowsModal's Ks cell. */
                inputMode="numeric"
                value={r.quantity}
                onChange={(e) => patch(i, { quantity: e.target.value.replace(/[^\d]/g, "") })}
                className={`text-right ${cellError(shown, i, "quantity")}`}
                aria-label="Ks"
              />
            </td>
            <td className="td-c w-[110px]">
              <Input value={r.section} onChange={(e) => patch(i, { section: e.target.value })} aria-label="Sektor" />
            </td>
            <td className="td-c w-[80px]">
              <Input value={r.rowLabel} onChange={(e) => patch(i, { rowLabel: e.target.value })} aria-label="Rad" />
            </td>
            <td className="td-c w-[130px]">
              <Input
                value={r.seat}
                onChange={(e) => patch(i, { seat: e.target.value })}
                placeholder="23-24"
                aria-label="Sedadlá"
              />
            </td>
            <td className="td-c w-[170px]">
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
            <td className="td-c w-[120px]">
              <Input
                value={r.price}
                onChange={(e) => patch(i, { price: e.target.value })}
                placeholder="15,00"
                className={`text-right ${cellError(shown, i, "price")}`}
                aria-label="Tvoja odmena"
              />
            </td>
            <td className="td-c w-[90px]">
              <Input
                value={r.currency}
                onChange={(e) => patch(i, { currency: e.target.value.toUpperCase() })}
                className={cellError(shown, i, "currency")}
                aria-label="Mena"
              />
            </td>
            <td className="td-c w-[150px]">
              <Input value={r.moreInfo} onChange={(e) => patch(i, { moreInfo: e.target.value })} aria-label="Poznámka" />
            </td>
            <RowRemove
              show={rows.length > 1}
              onRemove={() => setRows((rs) => rs.filter((_, k) => k !== i))}
              label={`Zmazať riadok ${i + 1}`}
              onDuplicate={() =>
                setRows((rs) => [...rs.slice(0, i + 1), { ...rs[i], buyerName: "" }, ...rs.slice(i + 1)])
              }
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
          rows.length > 1 ? `Vytvoriť ${rows.length} ${rows.length < 5 ? "pully" : "pullov"}` : "Vytvoriť pull"
        }
        summary={
          <>
            {totalTickets} {totalTickets === 1 ? "lístok" : totalTickets < 5 ? "lístky" : "lístkov"}
            {oneCurrency && totalFee > 0 ? ` · odmena spolu ${formatMoney(totalFee, rows[0].currency)}` : ""}
          </>
        }
      />
    </Modal>
  );
}

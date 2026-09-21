import { useEffect, useMemo, useState } from "react";
import { api, errMsg } from "../lib/api";
import { isIsoDate, matchByName } from "../lib/aiImport";
import type { EventCategory, EventInput, EventRecord, EventStatus } from "../lib/types";
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
 * New Event, as ROWS inside a modal over the Events list.
 *
 * 2.45.0: the fourth of the four forms marko asked to make the same -
 * "vsade events, inventory, sales a pulls budu podobne tie vyplnovace
 * udajov". One row is one event, which is exactly how events actually
 * arrive: a tour is one artist and six cities, typed in one pass instead of
 * six trips through the same modal.
 *
 * A new row inherits the category and the country from the one above it -
 * what a tour's nights have in common; the name, venue, city and date are
 * what differ, so they start empty.
 *
 * Same columns as the old form had, minus Notes: notes are per-event prose
 * rather than a column, and they stay on the event's own edit form, which is
 * unchanged and is still `EventFormModal`. Category is a plain Select here
 * (as platform is in the other three row forms) - creating a new category
 * stays where it was, on that edit form's LookupSelect.
 *
 * Each row is one ordinary `api.createEvent` call.
 */

const STATUSES: EventStatus[] = ["upcoming", "completed", "cancelled"];
const STATUS_LABELS: Record<EventStatus, string> = {
  upcoming: "Upcoming",
  completed: "Completed",
  cancelled: "Cancelled",
};

type Row = {
  name: string;
  eventDate: string;
  venue: string;
  city: string;
  country: string;
  categoryId: number | "";
  status: EventStatus;
};

function blankRow(prev?: Row): Row {
  return {
    name: "",
    eventDate: "",
    venue: "",
    city: "",
    // A tour keeps its category and usually its country; the city, the venue
    // and the night are what change from row to row.
    country: prev?.country ?? "",
    categoryId: prev?.categoryId ?? "",
    status: "upcoming",
  };
}

/** Every problem with every row, not just the first. `missing` stays quiet
 *  until Create is pressed; `invalid` shows the moment it is true. */
function validate(rows: Row[]): RowProblem[] {
  const out: RowProblem[] = [];
  rows.forEach((r, i) => {
    if (!r.name.trim()) out.push({ row: i, field: "name", message: "chýba názov eventu", kind: "missing" });
    // An event whose date is not settled yet is a real state here - empty is
    // allowed, a half-typed date is not.
    if (r.eventDate && !isIsoDate(r.eventDate))
      out.push({ row: i, field: "eventDate", message: "dátum nie je platný", kind: "invalid" });
  });
  return out;
}

export default function EventRowsModal({
  open,
  onClose,
  onCreated,
}: {
  open: boolean;
  onClose: () => void;
  onCreated: (created: EventRecord[]) => void;
}) {
  const toast = useToast();
  const [categories, setCategories] = useState<EventCategory[]>([]);
  const [rows, setRows] = useState<Row[]>([blankRow()]);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [submitted, setSubmitted] = useState(false);

  useEffect(() => {
    if (!open) return;
    api.listEventCategories().then(setCategories).catch(() => {});
  }, [open]);

  // Every open starts clean - the list keeps this component mounted and only
  // the inner Modal comes and goes.
  useEffect(() => {
    if (!open) return;
    setRows([blankRow()]);
    setError(null);
    setSubmitted(false);
  }, [open]);

  const dated = useMemo(() => rows.filter((r) => r.eventDate).length, [rows]);
  const problems = useMemo(() => validate(rows), [rows]);
  const shown = visibleProblems(problems, submitted);

  function patch(i: number, change: Partial<Row>) {
    setRows((rs) => rs.map((r, k) => (k === i ? { ...r, ...change } : r)));
  }

  async function submit() {
    setError(null);
    setSubmitted(true);
    // Every bad cell is already outlined and the footer counts them - nothing
    // to say here that the form is not already saying.
    if (problems.length > 0) return;

    setSaving(true);
    const made: EventRecord[] = [];
    try {
      for (const r of rows) {
        const input: EventInput = {
          name: r.name.trim(),
          artistTeam: null,
          venue: r.venue.trim() || null,
          city: r.city.trim() || null,
          country: r.country.trim() || null,
          eventDate: r.eventDate || null,
          categoryId: r.categoryId === "" ? null : Number(r.categoryId),
          status: r.status,
          notes: null,
        };
        made.push(await api.createEvent(input));
      }
      toast.success(
        made.length === 1 ? `Event ${made[0].name} vytvorený` : `Vytvorených ${made.length} eventov`,
      );
      onCreated(made);
    } catch (e) {
      // Whatever already went in stays in - name it rather than implying a
      // rollback the backend does not offer.
      setError(
        made.length > 0
          ? `${errMsg(e)} — už vytvorené a ponechané: ${made.map((m) => m.name).join(", ")}`
          : errMsg(e),
      );
    } finally {
      setSaving(false);
    }
  }

  return (
    <Modal open={open} onClose={onClose} title="Nový event" width="max-w-6xl">
      <AiImportPanel
        kind="event"
        className="mb-4"
        onApply={({ fields }) => {
          const cat = matchByName(categories, fields.category);
          setRows((rs) => {
            const first = { ...rs[0] };
            if (fields.name) first.name = fields.name;
            // A date only lands if it really is YYYY-MM-DD, and a status only
            // if it is one of the app's own three - the same two guards the
            // edit form applies.
            if (isIsoDate(fields.eventDate)) first.eventDate = fields.eventDate;
            if (fields.venue) first.venue = fields.venue;
            if (fields.city) first.city = fields.city;
            if (fields.country) first.country = fields.country;
            if (cat) first.categoryId = cat.id;
            if (STATUSES.includes((fields.status ?? "") as EventStatus)) {
              first.status = fields.status as EventStatus;
            }
            return [first, ...rs.slice(1)];
          });
          toast.info("Prvý riadok vyplnený z obrázka - skontroluj ho a potom vytvor.");
        }}
      />

      <RowFormTable
        head={["Názov", "Dátum", "Miesto", "Mesto", "Krajina", "Kategória", "Stav"]}
        onAdd={() => setRows((rs) => [...rs, blankRow(rs[rs.length - 1])])}
        addLabel="Ďalší event"
      >
        {rows.map((r, i) => (
          <tr key={i}>
            <RowNumber n={i + 1} />
            <td className="td-c w-[232px]">
              <Input
                value={r.name}
                onChange={(e) => patch(i, { name: e.target.value })}
                className={cellError(shown, i, "name")}
                aria-label={`Názov, riadok ${i + 1}`}
              />
            </td>
            <td className="td-c w-[126px]">
              <Input
                type="date"
                value={r.eventDate}
                onChange={(e) => patch(i, { eventDate: e.target.value })}
                className={cellError(shown, i, "eventDate")}
                aria-label="Dátum"
              />
            </td>
            <td className="td-c w-[166px]">
              <Input value={r.venue} onChange={(e) => patch(i, { venue: e.target.value })} aria-label="Miesto" />
            </td>
            <td className="td-c w-[136px]">
              <Input value={r.city} onChange={(e) => patch(i, { city: e.target.value })} aria-label="Mesto" />
            </td>
            <td className="td-c w-[106px]">
              <Input value={r.country} onChange={(e) => patch(i, { country: e.target.value })} aria-label="Krajina" />
            </td>
            <td className="td-c w-[140px]">
              <Select
                value={r.categoryId}
                onChange={(e) => patch(i, { categoryId: e.target.value ? Number(e.target.value) : "" })}
                aria-label="Kategória"
              >
                <option value="">—</option>
                {categories.map((c) => (
                  <option key={c.id} value={c.id}>
                    {c.name}
                  </option>
                ))}
              </Select>
            </td>
            <td className="td-c w-[114px]">
              <Select
                value={r.status}
                onChange={(e) => patch(i, { status: e.target.value as EventStatus })}
                aria-label="Stav"
              >
                {STATUSES.map((st) => (
                  <option key={st} value={st}>
                    {STATUS_LABELS[st]}
                  </option>
                ))}
              </Select>
            </td>
            <RowRemove
              show={rows.length > 1}
              onRemove={() => setRows((rs) => rs.filter((_, k) => k !== i))}
              label={`Zmazať riadok ${i + 1}`}
              onDuplicate={() => setRows((rs) => [...rs.slice(0, i + 1), { ...rs[i], name: "" }, ...rs.slice(i + 1)])}
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
          rows.length > 1 ? `Vytvoriť ${rows.length} ${rows.length < 5 ? "eventy" : "eventov"}` : "Vytvoriť event"
        }
        summary={
          <>
            {rows.length} {rows.length === 1 ? "event" : rows.length < 5 ? "eventy" : "eventov"}
            {dated < rows.length ? ` · ${rows.length - dated} bez dátumu` : ""}
          </>
        }
      />
    </Modal>
  );
}

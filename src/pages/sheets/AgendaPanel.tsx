import { useMemo } from "react";
import type { NoteRow, NoteSheet, SheetAlert } from "../../lib/types";
import { Button, EmptyState, Modal, ModalFooter } from "../../components/ui";
import { IconCalendarDays } from "../../components/icons";
import { colLetter } from "./Grid";

/**
 * "What and when" — marko: *"add calendar alebo time a da ti to policko a ptm
 * budes vediet co a kedy sa deje"*.
 *
 * Inserting a date gives you a cell. This is the other half: it reads the
 * sheet back and tells you what is coming, in order.
 *
 * ## It finds dates rather than being told where they are
 *
 * There is no "date column" — the whole point of this grid is that a column
 * does not say what belongs in it. So this scans every cell for something that
 * looks like a date, and shows the rest of that row as the "what". A sheet
 * nobody has put dates in simply shows nothing, which is the honest answer.
 *
 * Reminders (migration 033) are folded into the same list, because "what is
 * coming" is one question and it would be silly to answer it in two places.
 */

/** `25/09/2026`, `2026-09-25`, `25.9.2026`, optionally followed by `14:30`.
 *  Anchored so a ticket code like `2026ABC` is not read as a year. */
const DATE_RE =
  /^(\d{4})-(\d{2})-(\d{2})(?:[ T](\d{1,2}):(\d{2}))?$|^(\d{1,2})[./](\d{1,2})[./](\d{4})(?:[ ,]+(\d{1,2}):(\d{2}))?$/;

/** A cell to a sortable `YYYY-MM-DDTHH:MM`, or null if it is not a date. */
export function parseCellDate(raw: string): string | null {
  const m = DATE_RE.exec(raw.trim());
  if (!m) return null;
  const p = (n: string) => n.padStart(2, "0");
  if (m[1]) {
    const time = m[4] ? `${p(m[4])}:${m[5]}` : "00:00";
    return `${m[1]}-${m[2]}-${m[3]}T${time}`;
  }
  const time = m[9] ? `${p(m[9])}:${m[10]}` : "00:00";
  return `${m[8]}-${p(m[7]!)}-${p(m[6]!)}T${time}`;
}

function human(iso: string): string {
  const [date, time] = iso.split("T");
  const [y, mo, d] = date.split("-");
  return `${Number(d)}. ${Number(mo)}. ${y}${time === "00:00" ? "" : `, ${time}`}`;
}

function todayIsoLocal(): string {
  const d = new Date();
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}T${p(d.getHours())}:${p(d.getMinutes())}`;
}

type Entry = { when: string; what: string; where: string; kind: "cell" | "alert" };

export default function AgendaPanel({
  open,
  onClose,
  sheet,
  rows,
  alerts,
  onGoTo,
}: {
  open: boolean;
  onClose: () => void;
  sheet: NoteSheet | null;
  rows: NoteRow[];
  alerts: SheetAlert[];
  onGoTo: (r: number, c: number) => void;
}) {
  const entries = useMemo<(Entry & { r?: number; c?: number })[]>(() => {
    const out: (Entry & { r?: number; c?: number })[] = [];
    rows.forEach((row, r) => {
      row.cells.forEach((cell, c) => {
        const when = parseCellDate(cell);
        if (!when) return;
        // The "what" is the rest of the row - whatever he actually wrote next
        // to the date, which is the only thing that can describe it.
        const what = row.cells.filter((_, i) => i !== c).find((x) => x.trim()) ?? "(empty row)";
        out.push({ when, what, where: `${colLetter(c)}${r + 1}`, kind: "cell", r, c });
      });
    });
    alerts
      .filter((a) => !a.done && (!sheet || a.sheetId === sheet.id))
      .forEach((a) => out.push({ when: a.remindAt, what: a.title, where: "reminder", kind: "alert" }));
    out.sort((a, b) => a.when.localeCompare(b.when));
    return out;
  }, [rows, alerts, sheet]);

  const now = todayIsoLocal();
  const today = now.slice(0, 10);
  const past = entries.filter((e) => e.when < now);
  const upcoming = entries.filter((e) => e.when >= now);

  function Row({ e }: { e: Entry & { r?: number; c?: number } }) {
    const isToday = e.when.slice(0, 10) === today;
    return (
      <button
        type="button"
        onClick={() => {
          if (e.r !== undefined && e.c !== undefined) {
            onGoTo(e.r, e.c);
            onClose();
          }
        }}
        className="flex w-full items-baseline gap-3 rounded-md px-2 py-1.5 text-left transition hover:bg-surface-sunken"
      >
        <span
          className={`w-[132px] shrink-0 text-xs tabular-nums ${
            isToday ? "font-semibold text-brand-600 dark:text-brand-400" : "text-slate-500 dark:text-slate-400"
          }`}
        >
          {human(e.when)}
          {isToday && " · dnes"}
        </span>
        <span className="min-w-0 flex-1 truncate text-sm text-slate-800 dark:text-slate-100">{e.what}</span>
        <span className="shrink-0 text-[11px] uppercase tracking-wide text-slate-400 dark:text-slate-500">
          {e.where}
        </span>
      </button>
    );
  }

  return (
    <Modal open={open} onClose={onClose} title={sheet ? `What's coming — ${sheet.name}` : "What's coming"} width="max-w-2xl">
      <div className="flex flex-col gap-4">
        {entries.length === 0 ? (
          <EmptyState
            icon={<IconCalendarDays className="h-8 w-8" />}
            title="Nothing dated yet"
            description="Put a date in a cell (Insert → Date…) and it shows up here with whatever is written next to it."
          />
        ) : (
          <>
            {upcoming.length > 0 && (
              <div>
                <div className="label">Coming up ({upcoming.length})</div>
                <div className="flex flex-col">
                  {upcoming.map((e, i) => (
                    <Row key={`u${i}`} e={e} />
                  ))}
                </div>
              </div>
            )}
            {past.length > 0 && (
              <div>
                <div className="label">Already past ({past.length})</div>
                <div className="flex flex-col opacity-60">
                  {past.slice(-25).map((e, i) => (
                    <Row key={`p${i}`} e={e} />
                  ))}
                </div>
              </div>
            )}
          </>
        )}
        <p className="text-xs text-slate-500 dark:text-slate-400">
          Dates are found by looking at every cell — <code>25/09/2026</code>, <code>2026-09-25</code> and{" "}
          <code>25.9.2026</code> all count, with an optional time after them. Click a line to jump to that cell.
        </p>
      </div>
      <ModalFooter>
        <Button variant="secondary" onClick={onClose}>
          Close
        </Button>
      </ModalFooter>
    </Modal>
  );
}

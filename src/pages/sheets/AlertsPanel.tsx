import { useEffect, useState } from "react";
import { api, errMsg } from "../../lib/api";
import type { SheetAlert } from "../../lib/types";
import { Button, Input, Modal, ModalFooter, Textarea } from "../../components/ui";
import { IconTrash } from "../../components/icons";
import { useToast } from "../../lib/toast";
import { cellRef, colLetter } from "./Grid";

/**
 * Reminders on a sheet — marko: "nejake alert by som si tam chcel nastavit
 * casovo a tak".
 *
 * One list and one small form, nothing more. An alert has a title, a time, an
 * optional note, and optionally points at the cell that was selected when it
 * was created, which is what puts the little amber corner on that cell.
 *
 * ## It says plainly what it cannot do
 *
 * Nothing fires while the app is closed. That is written on the panel rather
 * than left for marko to find out by missing something — see `check_sheet_alerts`
 * in commands/alerts.rs.
 */

/** Local wall-clock, rounded to the next five minutes, for a new reminder. */
function defaultWhen(): string {
  const d = new Date();
  d.setMinutes(d.getMinutes() + 30, 0, 0);
  d.setMinutes(Math.ceil(d.getMinutes() / 5) * 5);
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}T${p(d.getHours())}:${p(d.getMinutes())}`;
}

function nowLocal(): string {
  const d = new Date();
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}T${p(d.getHours())}:${p(d.getMinutes())}`;
}

/** `2026-09-28T09:00` → `28. 9. 2026, 9:00`, without reformatting through a
 *  Date (which would drag a timezone into a value that deliberately has none). */
export function formatWhen(v: string): string {
  if (v.length < 16) return v;
  const [date, time] = v.split("T");
  const [y, m, d] = date.split("-");
  return `${Number(d)}. ${Number(m)}. ${y}, ${time}`;
}

export default function AlertsPanel({
  open,
  onClose,
  sheetId,
  sheetName,
  anchor,
  alerts,
  onChanged,
}: {
  open: boolean;
  onClose: () => void;
  sheetId: number;
  sheetName: string;
  /** The cell that was selected when the panel opened, if a new reminder
   *  should point at it. */
  anchor: { rowId: number | null; colIndex: number; r: number } | null;
  alerts: SheetAlert[];
  onChanged: () => void;
}) {
  const toast = useToast();
  const [editingId, setEditingId] = useState<number | null>(null);
  const [title, setTitle] = useState("");
  const [note, setNote] = useState("");
  const [when, setWhen] = useState(defaultWhen());
  const [attach, setAttach] = useState(true);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!open) return;
    setEditingId(null);
    setTitle("");
    setNote("");
    setWhen(defaultWhen());
    setAttach(anchor !== null);
  }, [open, anchor]);

  function editExisting(a: SheetAlert) {
    setEditingId(a.id);
    setTitle(a.title);
    setNote(a.note);
    setWhen(a.remindAt);
    setAttach(a.rowId !== null);
  }

  async function save() {
    setSaving(true);
    try {
      const existing = alerts.find((a) => a.id === editingId);
      await api.saveSheetAlert({
        id: editingId ?? undefined,
        sheetId,
        // Editing keeps whatever the alert already pointed at; only a NEW one
        // takes the current cell, and only when "attach" is ticked.
        rowId: editingId ? (existing?.rowId ?? null) : attach ? (anchor?.rowId ?? null) : null,
        colIndex: editingId ? (existing?.colIndex ?? null) : attach ? (anchor?.colIndex ?? null) : null,
        title,
        note,
        remindAt: when,
      });
      toast.success(editingId ? "Reminder saved" : "Reminder set");
      setEditingId(null);
      setTitle("");
      setNote("");
      setWhen(defaultWhen());
      onChanged();
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setSaving(false);
    }
  }

  const now = nowLocal();
  const mine = alerts.filter((a) => a.sheetId === sheetId);
  const others = alerts.filter((a) => a.sheetId !== sheetId);

  function Row({ a }: { a: SheetAlert }) {
    const overdue = !a.done && a.remindAt <= now;
    return (
      <div
        className={`flex items-start gap-2 rounded-md border px-2.5 py-2 ${
          overdue
            ? "border-amber-300 bg-amber-50 dark:border-amber-800 dark:bg-amber-950/40"
            : "border-slate-200 dark:border-slate-700"
        }`}
      >
        <input
          type="checkbox"
          checked={a.done}
          onChange={async () => {
            try {
              await api.setSheetAlertDone(a.id, !a.done);
              onChanged();
            } catch (e) {
              toast.error(errMsg(e));
            }
          }}
          className="mt-1"
          aria-label={a.done ? "Mark as not done" : "Mark as done"}
        />
        <button type="button" onClick={() => editExisting(a)} className="min-w-0 flex-1 text-left">
          <div className={`truncate text-sm ${a.done ? "text-slate-400 line-through dark:text-slate-500" : "font-medium text-slate-900 dark:text-slate-50"}`}>
            {a.title}
          </div>
          <div className="mt-0.5 flex flex-wrap gap-x-2 text-[11px] text-slate-500 dark:text-slate-400">
            <span className={overdue ? "font-semibold text-amber-700 dark:text-amber-400" : ""}>
              {formatWhen(a.remindAt)}
              {overdue ? " · teraz" : ""}
            </span>
            {a.sheetId !== sheetId && <span>{a.sheetName}</span>}
            {a.colIndex !== null && a.rowId !== null && <span>column {colLetter(a.colIndex)}</span>}
            {a.note.trim() && <span className="truncate">{a.note}</span>}
          </div>
        </button>
        <button
          type="button"
          onClick={async () => {
            try {
              await api.deleteSheetAlert(a.id);
              onChanged();
            } catch (e) {
              toast.error(errMsg(e));
            }
          }}
          aria-label="Delete reminder"
          title="Delete reminder"
          className="rounded p-1 text-slate-400 transition hover:text-red-600 dark:hover:text-red-400"
        >
          <IconTrash className="h-3.5 w-3.5" />
        </button>
      </div>
    );
  }

  return (
    <Modal open={open} onClose={onClose} title={sheetName ? `Reminders — ${sheetName}` : "Reminders"} width="max-w-xl">
      <div className="flex flex-col gap-4">
        <div className="rounded-lg border border-slate-200 p-3 dark:border-slate-700">
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-[1fr_200px]">
            <label>
              <span className="label">What</span>
              <Input
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                placeholder="Poslať Johnovi zvyšné 2 kódy"
                aria-label="Reminder title"
              />
            </label>
            <label>
              <span className="label">When</span>
              <Input
                type="datetime-local"
                value={when}
                onChange={(e) => setWhen(e.target.value)}
                aria-label="Reminder time"
              />
            </label>
          </div>
          <label className="mt-3 block">
            <span className="label">Note (optional)</span>
            <Textarea rows={2} value={note} onChange={(e) => setNote(e.target.value)} placeholder="ABC123, DEF456" />
          </label>
          {anchor && !editingId && (
            <label className="mt-2 flex items-center gap-2 text-xs text-slate-600 dark:text-slate-300">
              <input type="checkbox" checked={attach} onChange={(e) => setAttach(e.target.checked)} />
              Attach to cell {cellRef(anchor.r, anchor.colIndex)}
              {anchor.rowId === null && <span className="text-slate-400"> — type in that row first</span>}
            </label>
          )}
          <div className="mt-3 flex items-center gap-2">
            <Button variant="primary" onClick={save} disabled={saving}>
              {editingId ? "Save reminder" : "Set reminder"}
            </Button>
            {editingId && (
              <Button
                variant="secondary"
                onClick={() => {
                  setEditingId(null);
                  setTitle("");
                  setNote("");
                  setWhen(defaultWhen());
                }}
              >
                New instead
              </Button>
            )}
          </div>
        </div>

        {mine.length > 0 && (
          <div>
            <div className="label">This sheet</div>
            <div className="flex flex-col gap-1.5">
              {mine.map((a) => (
                <Row key={a.id} a={a} />
              ))}
            </div>
          </div>
        )}
        {others.length > 0 && (
          <div>
            <div className="label">Other sheets</div>
            <div className="flex flex-col gap-1.5">
              {others.map((a) => (
                <Row key={a.id} a={a} />
              ))}
            </div>
          </div>
        )}
        {mine.length === 0 && others.length === 0 && (
          <p className="text-sm text-slate-500 dark:text-slate-400">No reminders yet.</p>
        )}

        <p className="text-xs text-slate-500 dark:text-slate-400">
          A reminder shows up here and as a desktop notification <strong>while TIQR is open</strong>. With the app
          closed nothing fires — anything that came due meanwhile is delivered the next time you open it.
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

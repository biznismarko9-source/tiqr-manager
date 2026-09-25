import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { KeyboardEvent as ReactKeyboardEvent } from "react";
import { api, errMsg } from "../../lib/api";
import type { NoteRow, NoteSheet } from "../../lib/types";
import { Button, ConfirmDialog, Input, TableSkeleton } from "../../components/ui";
import {
  IconChevronDown,
  IconChevronLeft,
  IconChevronRight,
  IconChevronUp,
  IconPencil,
  IconPlus,
  IconSearch,
  IconTrash,
  IconX,
} from "../../components/icons";
import { useToast } from "../../lib/toast";

/**
 * The grid - marko: "urobil to ako realne google sheets uplne jednoduche".
 *
 * The 2.51.0-2.53.0 table put an `<input>` in every cell. It worked, it saved
 * itself, and it did not feel like a spreadsheet - it felt like a form with a
 * hundred boxes. This is the same data through a real grid:
 *
 *   - a cell is **plain text** until it is being edited; the box appears only
 *     in the one cell you are in
 *   - the **keyboard** drives it: arrows move, Enter edits then goes down, Tab
 *     goes right, Esc cancels, typing straight over a cell replaces it,
 *     Delete clears it
 *   - **row numbers** down the left, like every spreadsheet
 *   - **one blank row is always waiting at the bottom** - typing in it creates
 *     the row, so there is no "Add row" step
 *
 * Storage is untouched: `note_sheets` / `note_rows` from migration 031, one
 * write per committed cell, positional cells (see `PROTECTED_AREAS.md`).
 */

type Sel = { r: number; c: number };
type Sort = { index: number; dir: "asc" | "desc" } | null;

export default function Grid({ sheet, onSheetChanged }: { sheet: NoteSheet; onSheetChanged: () => void }) {
  const toast = useToast();
  const [rows, setRows] = useState<NoteRow[] | null>(null);
  const [filter, setFilter] = useState("");
  const [sort, setSort] = useState<Sort>(null);
  const [sel, setSel] = useState<Sel>({ r: 0, c: 0 });
  const [editing, setEditing] = useState<{ r: number; c: number; value: string } | null>(null);
  const [confirmSheet, setConfirmSheet] = useState(false);
  const [confirmColumn, setConfirmColumn] = useState<number | null>(null);
  const boxRef = useRef<HTMLDivElement>(null);

  const columns = sheet.columns;

  const loadRows = useCallback(async () => {
    try {
      setRows(await api.listNoteRows(sheet.id));
    } catch (e) {
      toast.error(errMsg(e));
      setRows([]);
    }
  }, [sheet.id, toast]);

  useEffect(() => {
    setRows(null);
    setSel({ r: 0, c: 0 });
    setEditing(null);
    setFilter("");
    setSort(null);
    void loadRows();
  }, [loadRows]);

  /** Filter, then sort. Both are display-only: `rows` keeps the stored order,
   *  so nothing here can write a value into the wrong row. */
  const display = useMemo(() => {
    let list = rows ?? [];
    const q = filter.trim().toLowerCase();
    if (q) list = list.filter((r) => r.cells.some((c) => c.toLowerCase().includes(q)));
    if (sort) {
      const { index, dir } = sort;
      list = [...list].sort((a, b) => {
        const x = a.cells[index] ?? "";
        const y = b.cells[index] ?? "";
        // An empty cell is a missing value, not a small one - it sinks either way.
        if (!x && !y) return a.position - b.position;
        if (!x) return 1;
        if (!y) return -1;
        const cmp = x.localeCompare(y, undefined, { numeric: true, sensitivity: "base" });
        return dir === "asc" ? cmp : -cmp;
      });
    }
    return list;
  }, [rows, filter, sort]);

  /** The blank row waiting at the bottom. It is hidden while filtering,
   *  because a row that does not match the filter would vanish the moment it
   *  was typed into. */
  const hasBlank = !filter.trim();
  const rowCount = display.length + (hasBlank ? 1 : 0);

  function cellAt(r: number, c: number): string {
    const row = display[r];
    return row ? (row.cells[c] ?? "") : "";
  }

  /** Writes one cell. The blank bottom row becomes a real row here, and the
   *  return value says so - the caller needs it to know that the grid is one
   *  row taller than the render it is still standing in. */
  async function commit(r: number, c: number, value: string): Promise<boolean> {
    const row = display[r];
    if (row) {
      if ((row.cells[c] ?? "") === value) return false;
      const next = [...row.cells];
      while (next.length < columns.length) next.push("");
      next[c] = value;
      setRows((rs) => rs?.map((x) => (x.id === row.id ? { ...x, cells: next } : x)) ?? rs);
      try {
        const saved = await api.updateNoteRow(row.id, next);
        setRows((rs) => rs?.map((x) => (x.id === saved.id ? saved : x)) ?? rs);
        onSheetChanged();
      } catch (e) {
        toast.error(errMsg(e));
        setRows((rs) => rs?.map((x) => (x.id === row.id ? row : x)) ?? rs);
      }
      return false;
    }
    // The blank row: nothing is stored until something is actually typed.
    if (!value) return false;
    const cells = columns.map((_, i) => (i === c ? value : ""));
    try {
      const created = await api.createNoteRow(sheet.id, cells);
      setRows((rs) => [...(rs ?? []), created]);
      onSheetChanged();
      return true;
    } catch (e) {
      toast.error(errMsg(e));
      return false;
    }
  }

  function focusBox() {
    boxRef.current?.focus();
  }

  /** `grown` is 1 when the caller has just created a row that this render does
   *  not know about yet - without it, Enter on the blank bottom row would
   *  clamp straight back onto the row it had only just created instead of
   *  landing on the new blank one below. */
  function moveTo(r: number, c: number, grown = 0) {
    setSel({
      r: Math.max(0, Math.min(r, Math.max(0, rowCount - 1 + grown))),
      c: Math.max(0, Math.min(c, columns.length - 1)),
    });
  }

  function startEdit(r: number, c: number, initial?: string) {
    setSel({ r, c });
    setEditing({ r, c, value: initial ?? cellAt(r, c) });
  }

  async function finishEdit(move: "down" | "right" | "left" | "none") {
    if (!editing) return;
    const { r, c, value } = editing;
    setEditing(null);
    const grown = (await commit(r, c, value)) ? 1 : 0;
    if (move === "down") moveTo(r + 1, c, grown);
    else if (move === "right") moveTo(r, c + 1, grown);
    else if (move === "left") moveTo(r, c - 1, grown);
    focusBox();
  }

  function onGridKey(e: ReactKeyboardEvent<HTMLDivElement>) {
    if (editing) return; // the input owns the keyboard while a cell is open
    const { r, c } = sel;
    const k = e.key;

    if (k === "ArrowDown") { e.preventDefault(); moveTo(r + 1, c); return; }
    if (k === "ArrowUp") { e.preventDefault(); moveTo(r - 1, c); return; }
    if (k === "ArrowRight") { e.preventDefault(); moveTo(r, c + 1); return; }
    if (k === "ArrowLeft") { e.preventDefault(); moveTo(r, c - 1); return; }
    if (k === "Tab") { e.preventDefault(); moveTo(r, e.shiftKey ? c - 1 : c + 1); return; }
    if (k === "Enter" || k === "F2") { e.preventDefault(); startEdit(r, c); return; }
    if (k === "Delete" || k === "Backspace") { e.preventDefault(); void commit(r, c, ""); return; }
    if (k === "Home") { e.preventDefault(); moveTo(r, 0); return; }
    if (k === "End") { e.preventDefault(); moveTo(r, columns.length - 1); return; }

    // Typing straight over a selected cell replaces it, as in any spreadsheet.
    if (k.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey) {
      e.preventDefault();
      startEdit(r, c, k);
    }
  }

  /* ----------------------------- columns ----------------------------- */

  async function afterColumnChange() {
    await loadRows();
    setSort(null);
    setEditing(null);
    onSheetChanged();
  }

  async function addColumn() {
    const name = window.prompt("Column name");
    if (!name?.trim()) return;
    try {
      await api.addNoteColumn(sheet.id, name.trim());
      await afterColumnChange();
    } catch (e) {
      toast.error(errMsg(e));
    }
  }

  async function renameColumn(i: number) {
    const name = window.prompt("Column name", columns[i]);
    if (!name?.trim() || name.trim() === columns[i]) return;
    try {
      await api.renameNoteColumn(sheet.id, i, name.trim());
      await afterColumnChange();
    } catch (e) {
      toast.error(errMsg(e));
    }
  }

  async function removeColumn(i: number) {
    try {
      await api.deleteNoteColumn(sheet.id, i);
      await afterColumnChange();
      moveTo(sel.r, Math.min(sel.c, columns.length - 2));
    } catch (e) {
      toast.error(errMsg(e));
    }
    setConfirmColumn(null);
  }

  async function moveColumn(i: number, delta: -1 | 1) {
    const to = i + delta;
    if (to < 0 || to >= columns.length) return;
    try {
      await api.reorderNoteColumn(sheet.id, i, to);
      await afterColumnChange();
      moveTo(sel.r, to);
    } catch (e) {
      toast.error(errMsg(e));
    }
  }

  async function removeRow(r: number) {
    const row = display[r];
    if (!row) return;
    try {
      await api.deleteNoteRow(row.id);
      setRows((rs) => rs?.filter((x) => x.id !== row.id) ?? rs);
      onSheetChanged();
    } catch (e) {
      toast.error(errMsg(e));
    }
  }

  function toggleSort(i: number) {
    setEditing(null);
    setSort((s) => {
      if (!s || s.index !== i) return { index: i, dir: "asc" };
      if (s.dir === "asc") return { index: i, dir: "desc" };
      return null; // a third click puts the sheet back in its own order
    });
  }

  /* ------------------------------ render ------------------------------ */

  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-wrap items-center gap-2">
        <h2 className="text-sm font-semibold text-slate-900 dark:text-slate-50">{sheet.name}</h2>
        <div className="relative w-full max-w-xs">
          <IconSearch className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-500 dark:text-slate-400" />
          <Input
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            placeholder="Filter rows…"
            aria-label="Filter rows in this sheet"
            className="pl-9"
          />
        </div>
        {(filter.trim() || sort) && (
          <span className="text-xs text-slate-500 dark:text-slate-400">
            {display.length} of {rows?.length ?? 0}
            {sort ? ` · ${columns[sort.index]} ${sort.dir === "asc" ? "A→Z" : "Z→A"}` : ""}{" "}
            <button
              type="button"
              className="underline underline-offset-2 hover:text-slate-700 dark:hover:text-slate-200"
              onClick={() => {
                setFilter("");
                setSort(null);
              }}
            >
              clear
            </button>
          </span>
        )}
        <div className="ml-auto flex flex-wrap gap-2">
          <Button
            variant="secondary"
            onClick={async () => {
              const name = window.prompt("Sheet name", sheet.name);
              if (!name?.trim() || name.trim() === sheet.name) return;
              try {
                await api.renameNoteSheet(sheet.id, name.trim());
                onSheetChanged();
              } catch (e) {
                toast.error(errMsg(e));
              }
            }}
          >
            <IconPencil className="h-4 w-4" /> Rename
          </Button>
          <Button variant="secondary" onClick={addColumn}>
            <IconPlus className="h-4 w-4" /> Column
          </Button>
          <Button variant="secondary" onClick={() => setConfirmSheet(true)}>
            <IconTrash className="h-4 w-4" /> Delete sheet
          </Button>
        </div>
      </div>

      {rows === null ? (
        <TableSkeleton />
      ) : (
        <>
          <div
            ref={boxRef}
            tabIndex={0}
            onKeyDown={onGridKey}
            className="table-shell outline-none focus-visible:ring-1 focus-visible:ring-brand-500"
          >
            <table className="w-full border-collapse text-sm">
              <thead className="sticky top-0 z-10 bg-surface">
                <tr>
                  <th className="w-[44px] border-b border-r border-slate-200 bg-surface-sunken px-1 py-1.5 text-[11px] font-normal text-slate-400 dark:border-slate-700 dark:text-slate-500" />
                  {columns.map((c, i) => {
                    const sortDir = sort && sort.index === i ? sort.dir : null;
                    return (
                    <th
                      key={i}
                      className="min-w-[120px] border-b border-r border-slate-200 bg-surface-sunken px-2 py-1.5 text-left text-xs font-semibold text-slate-600 dark:border-slate-700 dark:text-slate-300"
                    >
                      <span className="flex items-center gap-1">
                        <button
                          type="button"
                          onClick={() => toggleSort(i)}
                          title={`Sort by ${c}`}
                          className="inline-flex min-w-0 flex-1 items-center gap-1 truncate hover:text-slate-900 dark:hover:text-slate-100"
                        >
                          <span className="truncate">{c}</span>
                          {sortDir === "asc" && <IconChevronUp className="h-3 w-3 shrink-0" />}
                          {sortDir === "desc" && <IconChevronDown className="h-3 w-3 shrink-0" />}
                        </button>
                        <span className="flex shrink-0 items-center text-slate-300 dark:text-slate-600">
                          {i > 0 && (
                            <button
                              type="button"
                              onClick={() => void moveColumn(i, -1)}
                              aria-label={`Move column ${c} left`}
                              title="Move left"
                              className="hover:text-slate-700 dark:hover:text-slate-200"
                            >
                              <IconChevronLeft className="h-3 w-3" />
                            </button>
                          )}
                          {i < columns.length - 1 && (
                            <button
                              type="button"
                              onClick={() => void moveColumn(i, 1)}
                              aria-label={`Move column ${c} right`}
                              title="Move right"
                              className="hover:text-slate-700 dark:hover:text-slate-200"
                            >
                              <IconChevronRight className="h-3 w-3" />
                            </button>
                          )}
                          <button
                            type="button"
                            onClick={() => void renameColumn(i)}
                            aria-label={`Rename column ${c}`}
                            title="Rename"
                            className="ml-0.5 hover:text-slate-700 dark:hover:text-slate-200"
                          >
                            <IconPencil className="h-3 w-3" />
                          </button>
                          {columns.length > 1 && (
                            <button
                              type="button"
                              onClick={() => setConfirmColumn(i)}
                              aria-label={`Delete column ${c}`}
                              title="Delete column"
                              className="hover:text-red-600 dark:hover:text-red-400"
                            >
                              <IconX className="h-3 w-3" />
                            </button>
                          )}
                        </span>
                      </span>
                    </th>
                    );
                  })}
                  <th className="w-[36px] border-b border-slate-200 bg-surface-sunken dark:border-slate-700" />
                </tr>
              </thead>
              <tbody>
                {Array.from({ length: rowCount }, (_, r) => {
                  const isBlank = r >= display.length;
                  return (
                    <tr key={display[r]?.id ?? "blank"} className="group">
                      <td className="border-b border-r border-slate-200 bg-surface-sunken px-1 py-1 text-center text-[11px] tabular-nums text-slate-400 dark:border-slate-700 dark:text-slate-500">
                        {r + 1}
                      </td>
                      {columns.map((_, c) => {
                        const isSel = sel.r === r && sel.c === c;
                        const cellEdit = editing && editing.r === r && editing.c === c ? editing : null;
                        return (
                          <td
                            key={c}
                            onMouseDown={() => {
                              if (cellEdit) return;
                              if (editing) void finishEdit("none");
                              setSel({ r, c });
                              focusBox();
                            }}
                            onDoubleClick={() => startEdit(r, c)}
                            className={`border-b border-r border-slate-200 p-0 align-middle dark:border-slate-700 ${
                              isSel && !cellEdit ? "ring-2 ring-inset ring-brand-500" : ""
                            }`}
                          >
                            {cellEdit ? (
                              <input
                                autoFocus
                                value={cellEdit.value}
                                onChange={(e) => setEditing({ r, c, value: e.target.value })}
                                onBlur={() => void finishEdit("none")}
                                onKeyDown={(e) => {
                                  if (e.key === "Enter") {
                                    e.preventDefault();
                                    void finishEdit("down");
                                  } else if (e.key === "Tab") {
                                    e.preventDefault();
                                    void finishEdit(e.shiftKey ? "left" : "right");
                                  } else if (e.key === "Escape") {
                                    e.preventDefault();
                                    setEditing(null);
                                    focusBox();
                                  }
                                }}
                                aria-label={`${columns[c]}, row ${r + 1}`}
                                className="w-full bg-transparent px-2 py-1.5 text-sm text-slate-900 outline-none ring-2 ring-inset ring-brand-500 dark:text-slate-50"
                              />
                            ) : (
                              <div
                                className={`truncate px-2 py-1.5 ${
                                  isBlank ? "text-slate-300 dark:text-slate-600" : "text-slate-700 dark:text-slate-200"
                                }`}
                                title={cellAt(r, c) || undefined}
                              >
                                {cellAt(r, c) || " "}
                              </div>
                            )}
                          </td>
                        );
                      })}
                      <td className="border-b border-slate-200 px-1 text-center dark:border-slate-700">
                        {!isBlank && (
                          <button
                            type="button"
                            onClick={() => void removeRow(r)}
                            aria-label={`Delete row ${r + 1}`}
                            title="Delete row"
                            className="rounded p-1 text-transparent transition group-hover:text-slate-400 hover:!text-red-600 dark:hover:!text-red-400"
                          >
                            <IconX className="h-3.5 w-3.5" />
                          </button>
                        )}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>

          <p className="text-xs text-slate-500 dark:text-slate-400">
            Click a cell and type. <strong>Enter</strong> edits and moves down, <strong>Tab</strong> moves right,{" "}
            <strong>Esc</strong> cancels, <strong>Delete</strong> clears. The last row is always empty — typing in it
            adds a row. Everything saves itself.
            {filter.trim() && " (The empty row is hidden while a filter is on.)"}
          </p>
        </>
      )}

      <ConfirmDialog
        open={confirmSheet}
        title={`Delete "${sheet.name}"?`}
        message="Every row in this sheet goes with it, on this computer and the other one. This cannot be undone."
        confirmLabel="Delete sheet"
        danger
        onCancel={() => setConfirmSheet(false)}
        onConfirm={async () => {
          try {
            await api.deleteNoteSheet(sheet.id);
            setConfirmSheet(false);
            onSheetChanged();
          } catch (e) {
            toast.error(errMsg(e));
          }
        }}
      />

      <ConfirmDialog
        open={confirmColumn !== null}
        title={confirmColumn !== null ? `Delete column "${columns[confirmColumn]}"?` : ""}
        message="What is written in that column is deleted from every row in this sheet. The other columns are untouched."
        confirmLabel="Delete column"
        danger
        onCancel={() => setConfirmColumn(null)}
        onConfirm={() => {
          if (confirmColumn !== null) void removeColumn(confirmColumn);
        }}
      />
    </div>
  );
}

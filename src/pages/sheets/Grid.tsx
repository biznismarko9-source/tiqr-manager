import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { KeyboardEvent as ReactKeyboardEvent } from "react";
import { api, errMsg } from "../../lib/api";
import type { NoteRow, NoteSheet, SheetAlert } from "../../lib/types";
import { useToast } from "../../lib/toast";

/**
 * The grid — marko picked the "Classic" design, asked for it darker, and asked
 * for the header to stop dictating the contents:
 *
 *   "nechcem aby podla toho horneho stlpca si mohol zapisovat len co je v nom,
 *    chcem aby to bolo volne a vedel s tym pracovat"
 *
 * So the header is **letters** — A, B, C — exactly like a spreadsheet, and a
 * column's name is now an optional label underneath that may be empty. A
 * column no longer describes what belongs in it; it is only a position. (The
 * Rust validation that rejected an empty column name is gone — 2.55.0.)
 *
 * Everything else is the 2.54 grid, which worked: plain-text cells with one
 * `<input>` only in the cell being edited, keyboard-first movement, row
 * numbers, and blank rows always waiting at the bottom.
 *
 * ## Rows that are drawn but not stored
 *
 * The grid always draws at least `MIN_ROWS`. Typing into one of those calls
 * `ensure_note_rows`, which creates every row up to it in ONE transaction —
 * otherwise row 30 would be stored as row 4 and jump up the screen.
 *
 * ## Colour
 *
 * Built from the app's own surface tokens, so it follows the light/dark switch
 * like every other page. Dark is the one that was tuned: chrome on
 * `surface-sunken` (near-black), cells on `surface` (a step up), so it reads
 * as lit cells on dark chrome rather than one flat black sheet.
 */

const MIN_ROWS = 60;
const BASE_FONT = 12.5;
const DEFAULT_COL_W = 118;
/** 2.56.0: marko asked for the numbering to be miniature. The gutter and the
 *  letter strip are sized independently of the cells so they stay small even
 *  when the rows are made tall. */
const GUTTER_W = 30;
const LETTER_H = 15;
const MICRO_FONT = 9;

/** Text colours. 2.57.0: marko asked for these to be stronger ("farby nech su
 *  sytsie"), so light mode went 600 -> 700 saturated hues and dark mode 400 ->
 *  300, which is a real step up on both grounds rather than one tuned for one
 *  theme and merely legible on the other. */
const COLOUR_CLASS: Record<string, string> = {
  r: "text-red-700 dark:text-red-300",
  o: "text-orange-700 dark:text-orange-300",
  g: "text-emerald-700 dark:text-emerald-300",
  b: "text-blue-700 dark:text-blue-300",
  p: "text-fuchsia-700 dark:text-fuchsia-300",
  m: "text-slate-500 dark:text-slate-400",
};

/** Fills. Deliberately far lighter than the text colours - a fill sits UNDER
 *  text that has to stay readable in both themes, so these are tints, not the
 *  same hue at the same strength. */
const FILL_CLASS: Record<string, string> = {
  "1": "bg-red-100 dark:bg-red-950",
  "2": "bg-orange-100 dark:bg-orange-950",
  "3": "bg-emerald-100 dark:bg-emerald-950",
  "4": "bg-blue-100 dark:bg-blue-950",
  "5": "bg-fuchsia-100 dark:bg-fuchsia-950",
  "6": "bg-slate-200 dark:bg-slate-800",
  "7": "bg-yellow-100 dark:bg-yellow-950",
};

const ALIGN_CLASS: Record<string, string> = { L: "text-left", M: "text-center", R: "text-right" };

export const COLOURS: { flag: string; label: string; dot: string }[] = [
  { flag: "", label: "Default text", dot: "bg-slate-400" },
  { flag: "r", label: "Red", dot: "bg-red-600" },
  { flag: "o", label: "Orange", dot: "bg-orange-600" },
  { flag: "g", label: "Green", dot: "bg-emerald-600" },
  { flag: "b", label: "Blue", dot: "bg-blue-600" },
  { flag: "p", label: "Pink", dot: "bg-fuchsia-600" },
  { flag: "m", label: "Grey", dot: "bg-slate-500" },
];

export const FILLS: { flag: string; label: string; dot: string }[] = [
  { flag: "", label: "No fill", dot: "bg-transparent border border-slate-300 dark:border-slate-600" },
  { flag: "1", label: "Red", dot: "bg-red-300" },
  { flag: "2", label: "Orange", dot: "bg-orange-300" },
  { flag: "3", label: "Green", dot: "bg-emerald-300" },
  { flag: "4", label: "Blue", dot: "bg-blue-300" },
  { flag: "5", label: "Pink", dot: "bg-fuchsia-300" },
  { flag: "6", label: "Grey", dot: "bg-slate-400" },
  { flag: "7", label: "Yellow", dot: "bg-yellow-300" },
];

/** A stored flag string to the classes that draw it. An unknown letter draws
 *  nothing rather than breaking the cell - same forgiving rule as the backend. */
export function formatClass(f: string): string {
  let out = "";
  for (const ch of f) {
    if (COLOUR_CLASS[ch]) out += ` ${COLOUR_CLASS[ch]}`;
    else if (FILL_CLASS[ch]) out += ` ${FILL_CLASS[ch]}`;
    else if (ALIGN_CLASS[ch]) out += ` ${ALIGN_CLASS[ch]}`;
    else if (ch === "B") out += " font-semibold";
    else if (ch === "I") out += " italic";
    else if (ch === "U") out += " underline underline-offset-2";
    else if (ch === "S") out += " line-through";
  }
  return out;
}

/** Toggling one flag on a cell: a colour REPLACES the old colour (there is
 *  only ever one), a style toggles on and off. */
/** Which alphabet a flag belongs to decides how it toggles. A member of an
 *  exclusive set REPLACES whatever was there; a style toggles on and off. The
 *  four sets are the same ones `clean_format` uses in Rust, and the two must
 *  keep agreeing or a colour will appear to flip back on the next load. */
const EXCLUSIVE: RegExp[] = [/[rogbpm]/g, /[1234567]/g, /[LMR]/g];

/** The same normal form `clean_format` produces in Rust: colour, fill, sorted
 *  styles, alignment. Without this the UI appends styles in click order and the
 *  backend sorts them, so "BUS" here becomes "BSU" there - identical to look
 *  at, but the two strings diverge, which is exactly what PROTECTED_AREAS says
 *  must not happen. */
function normaliseFlags(f: string): string {
  let colour = "";
  let fill = "";
  let align = "";
  const styles: string[] = [];
  for (const ch of f) {
    if ("rogbpm".includes(ch)) colour = ch;
    else if ("1234567".includes(ch)) fill = ch;
    else if ("LMR".includes(ch)) align = ch;
    else if ("BIUS".includes(ch) && !styles.includes(ch)) styles.push(ch);
  }
  styles.sort();
  return colour + fill + styles.join("") + align;
}

export function toggleFlag(current: string, flag: string): string {
  // "" clears the text colour, "fill:" clears the fill, "align:" the alignment.
  if (flag === "") return normaliseFlags(current.replace(EXCLUSIVE[0], ""));
  if (flag === "fill:") return normaliseFlags(current.replace(EXCLUSIVE[1], ""));
  if (flag === "align:") return normaliseFlags(current.replace(EXCLUSIVE[2], ""));
  if ("BIUS".includes(flag)) {
    return normaliseFlags(current.includes(flag) ? current.replace(flag, "") : current + flag);
  }
  const set = EXCLUSIVE.find((re) => {
    re.lastIndex = 0;
    return re.test(flag);
  });
  if (!set) return normaliseFlags(current);
  set.lastIndex = 0;
  const without = current.replace(set, "");
  return normaliseFlags(current.includes(flag) ? without : flag + without);
}

export type Sel = { r: number; c: number };
type Sort = { index: number; dir: "asc" | "desc" } | null;

/** 0 → A, 25 → Z, 26 → AA. */
export function colLetter(index: number): string {
  let s = "";
  let i = index + 1;
  while (i > 0) {
    const m = (i - 1) % 26;
    s = String.fromCharCode(65 + m) + s;
    i = Math.floor((i - 1) / 26);
  }
  return s;
}

export function cellRef(r: number, c: number): string {
  return `${colLetter(c)}${r + 1}`;
}

/** What the toolbar and the menus are allowed to do to the grid, so neither of
 *  them needs to know how a row or a column is stored. */
export type GridHandle = {
  addRows: (n: number) => void;
  addColumn: () => void;
  renameColumn: () => void;
  deleteColumn: () => void;
  moveColumn: (delta: -1 | 1) => void;
  deleteRow: () => void;
  insertToday: () => void;
  /** Writes a literal into the selected cell — the date/time picker uses it. */
  insertText: (text: string) => void;
  clearCell: () => void;
  sortBySelected: (dir: "asc" | "desc" | null) => void;
  /** 2.56.0 */
  applyFormat: (flag: string) => void;
  currentFormat: () => string;
  nudgeRowHeight: (delta: number) => void;
  resetRowHeight: () => void;
  nudgeColumnWidth: (delta: number) => void;
  setSheetRowHeight: (h: number) => void;
  /** 2.57.0 */
  mergeSelected: (span: number) => void;
  unmergeSelected: () => void;
  insertRow: (where: "above" | "below") => void;
  duplicateRow: () => void;
  clearFormatting: () => void;
  setFrozenRows: (n: number) => void;
  columnStats: () => { count: number; sum: number | null };
};

export default function Grid({
  sheet,
  rows,
  setRows,
  zoom,
  filter,
  sel,
  setSel,
  alerts,
  onSheetChanged,
  onCellValue,
  bind,
}: {
  sheet: NoteSheet;
  rows: NoteRow[];
  setRows: (fn: (rs: NoteRow[]) => NoteRow[]) => void;
  zoom: number;
  filter: string;
  sel: Sel;
  setSel: (s: Sel) => void;
  alerts: SheetAlert[];
  onSheetChanged: () => void;
  /** Feeds the value bar above the grid, and tells the page which STORED row
   *  the cursor is on - `sel.r` is a display index, so with a filter or a sort
   *  on it does not line up with `rows` and anything resolving it upstairs
   *  would point at the wrong row. */
  onCellValue: (value: string, rowId: number | null) => void;
  bind: (handle: GridHandle) => void;
}) {
  const toast = useToast();
  const [sort, setSort] = useState<Sort>(null);
  const [editing, setEditing] = useState<{ r: number; c: number; value: string } | null>(null);
  const boxRef = useRef<HTMLDivElement>(null);
  /** Live drag of a column edge or a row edge. `px` is what is on screen right
   *  now; the write happens once, on mouse-up, so a drag is one row in the
   *  database and not one per pixel. */
  const [drag, setDrag] = useState<
    { kind: "col"; index: number; px: number } | { kind: "row"; rowId: number; index: number; px: number } | null
  >(null);
  const dragRef = useRef<typeof drag>(null);
  dragRef.current = drag;
  /** Where the edge being dragged started, in page coordinates, so the live
   *  size is just `pointer - origin` instead of an accumulated delta that
   *  drifts once the pointer leaves the window and comes back. */
  const dragOrigin = useRef(0);

  const columns = sheet.columns;
  const baseRowH = sheet.rowHeight > 0 ? sheet.rowHeight : 24;
  const font = (BASE_FONT * zoom).toFixed(1);
  /** A row's own height if it has one, otherwise the sheet's - and whatever
   *  the pointer is doing right now, if this is the edge being dragged. */
  const heightOf = (row: NoteRow | undefined) => {
    if (drag?.kind === "row" && row && drag.rowId === row.id) return drag.px;
    return Math.round((row && row.height > 0 ? row.height : baseRowH) * zoom);
  };
  const widthOf = (i: number) => {
    if (drag?.kind === "col" && drag.index === i) return drag.px;
    return Math.round(((sheet.widths[i] ?? 0) > 0 ? sheet.widths[i] : DEFAULT_COL_W) * zoom);
  };
  const rowH = Math.round(baseRowH * zoom);

  useEffect(() => {
    setSort(null);
    setEditing(null);
  }, [sheet.id]);

  /** Filter, then sort. Both are display-only: `rows` keeps the stored order,
   *  so nothing here can write a value into the wrong row. */
  const display = useMemo(() => {
    let list = rows;
    const q = filter.trim().toLowerCase();
    if (q) list = list.filter((r) => r.cells.some((c) => c.toLowerCase().includes(q)));
    if (sort) {
      const { index, dir } = sort;
      list = [...list].sort((a, b) => {
        const x = a.cells[index] ?? "";
        const y = b.cells[index] ?? "";
        // An empty cell is a missing value, not a small one — it sinks either way.
        if (!x && !y) return a.position - b.position;
        if (!x) return 1;
        if (!y) return -1;
        const cmp = x.localeCompare(y, undefined, { numeric: true, sensitivity: "base" });
        return dir === "asc" ? cmp : -cmp;
      });
    }
    return list;
  }, [rows, filter, sort]);

  const hasBlank = !filter.trim();
  const rowCount = hasBlank ? Math.max(display.length + 1, MIN_ROWS) : display.length;

  const cellAt = useCallback(
    (r: number, c: number) => {
      const row = display[r];
      return row ? (row.cells[c] ?? "") : "";
    },
    [display],
  );

  useEffect(() => {
    onCellValue(editing ? editing.value : cellAt(sel.r, sel.c), display[sel.r]?.id ?? null);
  }, [editing, sel, cellAt, display, onCellValue]);

  /** Which cells carry a live reminder, so the corner marker can be drawn. */
  const alertCells = useMemo(() => {
    const m = new Set<string>();
    alerts.forEach((a) => {
      if (a.rowId !== null && a.colIndex !== null && !a.done) m.add(`${a.rowId}:${a.colIndex}`);
    });
    return m;
  }, [alerts]);

  /* ------------------------------- writing ------------------------------ */

  /** Writes one cell. Rows the grid was only drawing become real here, and the
   *  return value says whether the grid grew — the caller needs it, because it
   *  is still standing in the render that did not know. */
  const commit = useCallback(
    async (r: number, c: number, value: string): Promise<boolean> => {
      const row = display[r];
      if (row) {
        if ((row.cells[c] ?? "") === value) return false;
        const next = [...row.cells];
        while (next.length < columns.length) next.push("");
        next[c] = value;
        setRows((rs) => rs.map((x) => (x.id === row.id ? { ...x, cells: next } : x)));
        try {
          const saved = await api.updateNoteRow(row.id, next);
          setRows((rs) => rs.map((x) => (x.id === saved.id ? saved : x)));
        } catch (e) {
          toast.error(errMsg(e));
          setRows((rs) => rs.map((x) => (x.id === row.id ? row : x)));
        }
        return false;
      }
      // A drawn-but-not-stored row. Nothing is created until something is typed.
      if (!value) return false;
      try {
        const all = await api.ensureNoteRows(sheet.id, r + 1);
        const target = all[r];
        if (!target) return false;
        const next = [...target.cells];
        while (next.length < columns.length) next.push("");
        next[c] = value;
        const saved = await api.updateNoteRow(target.id, next);
        setRows(() => all.map((x) => (x.id === saved.id ? saved : x)));
        onSheetChanged();
        return true;
      } catch (e) {
        toast.error(errMsg(e));
        return false;
      }
    },
    [display, columns.length, setRows, sheet.id, toast, onSheetChanged],
  );

  function focusBox() {
    boxRef.current?.focus();
  }

  /** A drag is tracked on the DOCUMENT, not on the handle: the pointer moves
   *  far faster than a 5px strip and would otherwise slip off it mid-drag. */
  useEffect(() => {
    if (!drag) return;
    const move = (e: MouseEvent) => {
      setDrag((d) => {
        if (!d) return d;
        const px = d.kind === "col" ? Math.max(40, Math.min(900, e.clientX - dragOrigin.current))
                                    : Math.max(16, Math.min(400, e.clientY - dragOrigin.current));
        return { ...d, px };
      });
    };
    const up = () => {
      const d = dragRef.current;
      setDrag(null);
      if (!d) return;
      void (async () => {
        try {
          if (d.kind === "col") {
            await api.setNoteColumnWidth(sheet.id, d.index, Math.round(d.px / zoom));
            onSheetChanged();
          } else {
            const h = Math.round(d.px / zoom);
            await api.setNoteRowHeight(d.rowId, h);
            setRows((rs) => rs.map((x) => (x.id === d.rowId ? { ...x, height: h } : x)));
          }
        } catch (e) {
          toast.error(errMsg(e));
        }
      })();
    };
    document.addEventListener("mousemove", move);
    document.addEventListener("mouseup", up);
    return () => {
      document.removeEventListener("mousemove", move);
      document.removeEventListener("mouseup", up);
    };
  }, [drag, sheet.id, zoom, setRows, toast, onSheetChanged]);

  /** `grown` is 1 when the caller has just created rows this render does not
   *  know about, so Enter on the blank bottom row lands on the new blank row
   *  below instead of clamping back onto the one it just created. */
  const moveTo = useCallback(
    (r: number, c: number, grown = 0) => {
      setSel({
        r: Math.max(0, Math.min(r, Math.max(0, rowCount - 1 + grown))),
        c: Math.max(0, Math.min(c, columns.length - 1)),
      });
    },
    [rowCount, columns.length, setSel],
  );

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
    if (editing) return; // the open cell's own input owns the keyboard
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
    if (k === "PageDown") { e.preventDefault(); moveTo(r + 15, c); return; }
    if (k === "PageUp") { e.preventDefault(); moveTo(r - 15, c); return; }
    // Typing straight over a selected cell replaces it, as in any spreadsheet.
    if (k.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey) {
      e.preventDefault();
      startEdit(r, c, k);
    }
  }

  /* ------------------------ structure (menu actions) -------------------- */

  const reload = useCallback(async () => {
    try {
      const fresh = await api.listNoteRows(sheet.id);
      setRows(() => fresh);
    } catch (e) {
      toast.error(errMsg(e));
    }
    setSort(null);
    setEditing(null);
    onSheetChanged();
  }, [sheet.id, setRows, toast, onSheetChanged]);

  useEffect(() => {
    bind({
      addRows: (n) => {
        void (async () => {
          try {
            const all = await api.ensureNoteRows(sheet.id, rows.length + n);
            setRows(() => all);
            onSheetChanged();
          } catch (e) {
            toast.error(errMsg(e));
          }
        })();
      },
      addColumn: () => {
        void (async () => {
          try {
            // Unlabelled on purpose: a new column is a position, not a field.
            await api.addNoteColumn(sheet.id, "");
            await reload();
          } catch (e) {
            toast.error(errMsg(e));
          }
        })();
      },
      renameColumn: () => {
        const cur = columns[sel.c] ?? "";
        const name = window.prompt(`Label for column ${colLetter(sel.c)} — leave empty for none`, cur);
        if (name === null || name.trim() === cur) return;
        void (async () => {
          try {
            await api.renameNoteColumn(sheet.id, sel.c, name.trim());
            await reload();
          } catch (e) {
            toast.error(errMsg(e));
          }
        })();
      },
      deleteColumn: () => {
        if (columns.length <= 1) {
          toast.error("A sheet needs at least one column.");
          return;
        }
        void (async () => {
          try {
            await api.deleteNoteColumn(sheet.id, sel.c);
            await reload();
            moveTo(sel.r, Math.min(sel.c, columns.length - 2));
          } catch (e) {
            toast.error(errMsg(e));
          }
        })();
      },
      moveColumn: (delta) => {
        const to = sel.c + delta;
        if (to < 0 || to >= columns.length) return;
        void (async () => {
          try {
            await api.reorderNoteColumn(sheet.id, sel.c, to);
            await reload();
            setSel({ r: sel.r, c: to });
          } catch (e) {
            toast.error(errMsg(e));
          }
        })();
      },
      deleteRow: () => {
        const row = display[sel.r];
        if (!row) return;
        void (async () => {
          try {
            await api.deleteNoteRow(row.id);
            setRows((rs) => rs.filter((x) => x.id !== row.id));
            onSheetChanged();
          } catch (e) {
            toast.error(errMsg(e));
          }
        })();
      },
      insertToday: () => {
        const d = new Date();
        const iso = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
        void commit(sel.r, sel.c, iso);
      },
      insertText: (text) => void commit(sel.r, sel.c, text),
      clearCell: () => void commit(sel.r, sel.c, ""),
      sortBySelected: (dir) => setSort(dir ? { index: sel.c, dir } : null),

      currentFormat: () => display[sel.r]?.formats[sel.c] ?? "",
      applyFormat: (flag) => {
        const row = display[sel.r];
        // Colour belongs to a stored cell. A row the grid is only drawing has
        // nothing to colour yet, and saying so beats doing nothing silently.
        if (!row) {
          toast.error("Type something in that row first, then colour it.");
          return;
        }
        const next = toggleFlag(row.formats[sel.c] ?? "", flag);
        void (async () => {
          try {
            const saved = await api.setNoteCellFormat(row.id, sel.c, next);
            setRows((rs) => rs.map((x) => (x.id === saved.id ? saved : x)));
          } catch (e) {
            toast.error(errMsg(e));
          }
        })();
      },
      nudgeRowHeight: (delta) => {
        const row = display[sel.r];
        if (!row) {
          toast.error("Type something in that row first, then resize it.");
          return;
        }
        const next = Math.max(16, Math.min(400, (row.height > 0 ? row.height : baseRowH) + delta));
        void (async () => {
          try {
            await api.setNoteRowHeight(row.id, next);
            setRows((rs) => rs.map((x) => (x.id === row.id ? { ...x, height: next } : x)));
          } catch (e) {
            toast.error(errMsg(e));
          }
        })();
      },
      resetRowHeight: () => {
        const row = display[sel.r];
        if (!row) return;
        void (async () => {
          try {
            await api.setNoteRowHeight(row.id, 0);
            setRows((rs) => rs.map((x) => (x.id === row.id ? { ...x, height: 0 } : x)));
          } catch (e) {
            toast.error(errMsg(e));
          }
        })();
      },
      setSheetRowHeight: (h) => {
        void (async () => {
          try {
            await api.setNoteSheetRowHeight(sheet.id, h);
            onSheetChanged();
          } catch (e) {
            toast.error(errMsg(e));
          }
        })();
      },
      mergeSelected: (span) => {
        const row = display[sel.r];
        if (!row) {
          toast.error("Type something in that row first, then merge it.");
          return;
        }
        void (async () => {
          try {
            const saved = await api.setNoteCellMerge(row.id, sel.c, span);
            setRows((rs) => rs.map((x) => (x.id === saved.id ? saved : x)));
          } catch (e) {
            toast.error(errMsg(e));
          }
        })();
      },
      unmergeSelected: () => {
        const row = display[sel.r];
        if (!row) return;
        void (async () => {
          try {
            const saved = await api.setNoteCellMerge(row.id, sel.c, 1);
            setRows((rs) => rs.map((x) => (x.id === saved.id ? saved : x)));
          } catch (e) {
            toast.error(errMsg(e));
          }
        })();
      },
      insertRow: (where) => {
        const row = display[sel.r];
        const at = row ? row.position + (where === "below" ? 1 : 0) : rows.length;
        void (async () => {
          try {
            const all = await api.insertNoteRowAt(sheet.id, at);
            setRows(() => all);
            onSheetChanged();
          } catch (e) {
            toast.error(errMsg(e));
          }
        })();
      },
      duplicateRow: () => {
        const row = display[sel.r];
        if (!row) {
          toast.error("There is nothing in that row to duplicate.");
          return;
        }
        void (async () => {
          try {
            const all = await api.duplicateNoteRow(row.id);
            setRows(() => all);
            onSheetChanged();
          } catch (e) {
            toast.error(errMsg(e));
          }
        })();
      },
      clearFormatting: () => {
        const row = display[sel.r];
        if (!row) return;
        void (async () => {
          try {
            const saved = await api.setNoteCellFormat(row.id, sel.c, "");
            setRows((rs) => rs.map((x) => (x.id === saved.id ? saved : x)));
          } catch (e) {
            toast.error(errMsg(e));
          }
        })();
      },
      setFrozenRows: (n) => {
        void (async () => {
          try {
            await api.setNoteFrozenRows(sheet.id, n);
            onSheetChanged();
          } catch (e) {
            toast.error(errMsg(e));
          }
        })();
      },
      columnStats: () => {
        let count = 0;
        let sum = 0;
        let anyNumber = false;
        for (const row of display) {
          const raw = (row.cells[sel.c] ?? "").trim();
          if (!raw) continue;
          count += 1;
          // European decimals and thousands separators, because that is how
          // prices get typed here: "1 234,50" and "180,00" both read as numbers.
          const n = Number(raw.replace(/\s/g, "").replace(/\.(?=\d{3}\b)/g, "").replace(",", "."));
          if (Number.isFinite(n)) {
            sum += n;
            anyNumber = true;
          }
        }
        return { count, sum: anyNumber ? sum : null };
      },
      nudgeColumnWidth: (delta) => {
        const cur = (sheet.widths[sel.c] ?? 0) > 0 ? sheet.widths[sel.c] : DEFAULT_COL_W;
        void (async () => {
          try {
            await api.setNoteColumnWidth(sheet.id, sel.c, Math.max(40, Math.min(900, cur + delta)));
            onSheetChanged();
          } catch (e) {
            toast.error(errMsg(e));
          }
        })();
      },
    });
  }, [bind, sheet, columns, sel, display, rows.length, baseRowH, reload, commit, moveTo, setRows, setSel, toast, onSheetChanged]);

  /* ------------------------------- render ------------------------------- */

  const chrome = "bg-surface-sunken text-slate-500 dark:text-slate-400";
  const line = "border-slate-200 dark:border-slate-800";

  return (
    <div
      ref={boxRef}
      tabIndex={0}
      onKeyDown={onGridKey}
      className="table-shell h-full outline-none focus-visible:ring-1 focus-visible:ring-inset focus-visible:ring-brand-500"
      style={{ fontSize: `${font}px` }}
    >
      <table className="w-full border-collapse" style={{ tableLayout: "fixed" }}>
        <thead className="sticky top-0 z-20">
          <tr>
            <th className={`border-b border-r ${line} ${chrome}`} style={{ width: GUTTER_W, height: LETTER_H }} />
            {columns.map((_, i) => (
              <th
                key={i}
                className={`border-b border-r ${line} text-center font-normal leading-none tracking-wider ${
                  sel.c === i ? "bg-brand-600 text-white" : chrome
                }`}
                style={{ width: widthOf(i), height: LETTER_H, fontSize: `${MICRO_FONT}px`, position: "relative" }}
                title={`Column ${colLetter(i)} — drag the edge to resize`}
              >
                {colLetter(i)}
                <span
                  role="separator"
                  aria-label={`Resize column ${colLetter(i)}`}
                  onMouseDown={(e) => {
                    e.preventDefault();
                    e.stopPropagation();
                    dragOrigin.current = e.clientX - widthOf(i);
                    setDrag({ kind: "col", index: i, px: widthOf(i) });
                  }}
                  className="absolute -right-[3px] top-0 z-30 h-full w-[6px] cursor-col-resize hover:bg-brand-500/60"
                />
              </th>
            ))}
          </tr>
          <tr>
            <th className={`border-b border-r ${line} ${chrome}`} style={{ width: GUTTER_W, height: rowH }} />
            {columns.map((label, i) => (
              <th
                key={i}
                className={`truncate border-b border-r ${line} bg-surface px-1.5 text-left font-semibold text-slate-700 dark:text-slate-200`}
                style={{ width: widthOf(i), height: rowH }}
                title={label || `No label — column ${colLetter(i)} takes anything`}
              >
                {label || <span className="font-normal text-slate-300 dark:text-slate-700">·</span>}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {Array.from({ length: rowCount }, (_, r) => {
            const row = display[r];
            const h = heightOf(row);
            return (
              <tr
                key={row?.id ?? `blank-${r}`}
                className={r < sheet.frozenRows ? "sticky z-10" : undefined}
                style={r < sheet.frozenRows ? { top: LETTER_H + rowH + r * rowH } : undefined}
              >
                <th
                  className={`border-b border-r ${line} text-center align-middle font-normal leading-none ${
                    sel.r === r ? "bg-brand-600 text-white" : chrome
                  }`}
                  style={{ width: GUTTER_W, height: h, fontSize: `${MICRO_FONT}px`, position: "relative" }}
                  title={row ? "Drag the bottom edge to resize" : undefined}
                >
                  {r + 1}
                  {row && (
                    <span
                      role="separator"
                      aria-label={`Resize row ${r + 1}`}
                      onMouseDown={(e) => {
                        e.preventDefault();
                        e.stopPropagation();
                        dragOrigin.current = e.clientY - h;
                        setDrag({ kind: "row", rowId: row.id, index: r, px: h });
                      }}
                      className="absolute -bottom-[3px] left-0 z-30 h-[6px] w-full cursor-row-resize hover:bg-brand-500/60"
                    />
                  )}
                </th>
                {columns.map((_, c) => {
                  // "0" means a merge to the left already covers this cell, so
                  // it must not be drawn at all - the colSpan does its job.
                  if ((row?.merges[c] ?? "") === "0") return null;
                  const span = Math.max(1, Math.min(Number(row?.merges[c] ?? 1) || 1, columns.length - c));
                  const isSel = sel.r === r && sel.c === c;
                  const cellEdit = editing && editing.r === r && editing.c === c ? editing : null;
                  const hasAlert = row ? alertCells.has(`${row.id}:${c}`) : false;
                  const text = cellAt(r, c);
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
                      className={`relative border-b border-r ${line} bg-surface p-0 align-middle ${
                        isSel && !cellEdit ? "ring-2 ring-inset ring-brand-500" : ""
                      }`}
                      colSpan={span > 1 ? span : undefined}
                      style={{
                        width: span > 1 ? undefined : widthOf(c),
                        height: h,
                      }}
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
                          aria-label={cellRef(r, c)}
                          className="w-full bg-surface px-1.5 text-slate-900 outline-none ring-2 ring-inset ring-brand-500 dark:text-slate-50"
                          style={{ height: h, fontSize: `${font}px` }}
                        />
                      ) : (
                        <div
                          className={`truncate px-1.5 text-slate-800 dark:text-slate-200${formatClass(row?.formats[c] ?? "")}`}
                          title={text || undefined}
                        >
                          {text || " "}
                        </div>
                      )}
                      {hasAlert && !cellEdit && (
                        <span
                          aria-label="Has a reminder"
                          title="Has a reminder"
                          className="pointer-events-none absolute right-0 top-0 h-0 w-0 border-l-[6px] border-t-[6px] border-l-transparent border-t-amber-500"
                        />
                      )}
                    </td>
                  );
                })}
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}

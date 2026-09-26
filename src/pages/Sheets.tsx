import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { api, errMsg } from "../lib/api";
import type { NoteRow, NoteSheet, SheetAlert, WorkspaceHit, WorkspaceItem } from "../lib/types";
import { Button, EmptyState, Input, Modal, ModalFooter, TableSkeleton } from "../components/ui";
import { IconBell, IconLayoutGrid, IconPlus, IconSearch } from "../components/icons";
import { useToast } from "../lib/toast";
import Grid, { cellRef, colLetter } from "./sheets/Grid";
import type { GridHandle, Sel } from "./sheets/Grid";
import MenuBar from "./sheets/MenuBar";
import type { Menu } from "./sheets/MenuBar";
import AlertsPanel from "./sheets/AlertsPanel";

/**
 * Sheets — the dark "Classic" spreadsheet marko chose out of ten.
 *
 * His brief for this pass, in order:
 *   · "ten prvy by som dal ale urobil ho tmavsi ale tak vyvazeny aby sa v nom
 *      dalo pracovat"
 *   · "nechcem aby podla toho horneho stlpca si mohol zapisovat len co je v
 *      nom, chcem aby to bolo volne"      → the header is letters; see Grid.tsx
 *   · "nejake alert by som si tam chcel nastavit casovo a tak"  → AlertsPanel
 *   · "toto co je hore ze file edit data to je good"            → MenuBar
 *   · "aj to dole prepinanie"                                   → the tab strip
 *
 * The sheet list moved from a left sidebar to a **bottom tab strip**, which is
 * where a spreadsheet keeps it and where he asked for it. The left rail is the
 * app's own navigation and is untouched.
 *
 * ## The layout is a fixed-height column
 *
 * Header, menu, toolbar, value bar, grid, tabs. The grid is the only part that
 * scrolls, so the tabs stay put at the bottom however long the sheet is —
 * which needs `min-h-0` on the flex child, or the table's own height pushes
 * the tab strip off the screen.
 */

const ZOOMS = [0.75, 0.9, 1, 1.15, 1.35, 1.6];

export default function Sheets() {
  const toast = useToast();
  const [sheets, setSheets] = useState<NoteSheet[] | null>(null);
  const [activeId, setActiveId] = useState<number | null>(null);
  const [rows, setRowsState] = useState<NoteRow[]>([]);
  const [loadingRows, setLoadingRows] = useState(false);
  const [sel, setSel] = useState<Sel>({ r: 0, c: 0 });
  const [cellValue, setCellValue] = useState("");
  const [selRowId, setSelRowId] = useState<number | null>(null);
  const [zoom, setZoom] = useState(1);
  const [filter, setFilter] = useState("");
  const [query, setQuery] = useState("");
  const [hits, setHits] = useState<WorkspaceHit[] | null>(null);
  const [alerts, setAlerts] = useState<SheetAlert[]>([]);
  const [alertsOpen, setAlertsOpen] = useState(false);
  const [newOpen, setNewOpen] = useState(false);
  const [oldNotes, setOldNotes] = useState<WorkspaceItem[]>([]);
  const [importing, setImporting] = useState(false);
  const gridRef = useRef<GridHandle | null>(null);
  const bind = useCallback((h: GridHandle) => {
    gridRef.current = h;
  }, []);

  const active = useMemo(() => sheets?.find((s) => s.id === activeId) ?? null, [sheets, activeId]);

  const loadSheets = useCallback(async () => {
    try {
      const list = await api.listNoteSheets();
      setSheets(list);
      setActiveId((cur) => (cur && list.some((s) => s.id === cur) ? cur : (list[0]?.id ?? null)));
    } catch (e) {
      toast.error(errMsg(e));
      setSheets([]);
    }
  }, [toast]);

  const loadAlerts = useCallback(async () => {
    try {
      setAlerts(await api.listSheetAlerts(true));
    } catch (e) {
      toast.error(errMsg(e));
    }
  }, [toast]);

  useEffect(() => {
    void loadSheets();
    void loadAlerts();
  }, [loadSheets, loadAlerts]);

  // Anything written into the 2.52/2.53 notes. Almost always nothing, in which
  // case nothing on this page mentions it.
  useEffect(() => {
    api
      .listWorkspaceItems(true)
      .then((items) => setOldNotes(items.filter((i) => i.title.trim() || i.content.trim())))
      .catch(() => setOldNotes([]));
  }, []);

  // Rows belong to the open sheet, and the selection goes home with them.
  useEffect(() => {
    if (activeId === null) {
      setRowsState([]);
      return;
    }
    let alive = true;
    setLoadingRows(true);
    setSel({ r: 0, c: 0 });
    setFilter("");
    api
      .listNoteRows(activeId)
      .then((r) => {
        if (alive) setRowsState(r);
      })
      .catch((e) => {
        if (alive) toast.error(errMsg(e));
      })
      .finally(() => {
        if (alive) setLoadingRows(false);
      });
    return () => {
      alive = false;
    };
  }, [activeId, toast]);

  // One search across every sheet — names and cells.
  useEffect(() => {
    const q = query.trim();
    if (!q) {
      setHits(null);
      return;
    }
    const t = setTimeout(() => {
      api
        .searchWorkspace(q)
        .then((all) => setHits(all.filter((h) => h.kind === "table")))
        .catch((e) => toast.error(errMsg(e)));
    }, 180);
    return () => clearTimeout(t);
  }, [query, toast]);

  const setRows = useCallback((fn: (rs: NoteRow[]) => NoteRow[]) => setRowsState((rs) => fn(rs)), []);

  const dueCount = useMemo(() => {
    const d = new Date();
    const p = (n: number) => String(n).padStart(2, "0");
    const now = `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}T${p(d.getHours())}:${p(d.getMinutes())}`;
    return alerts.filter((a) => !a.done && a.remindAt <= now).length;
  }, [alerts]);

  async function renameSheet() {
    if (!active) return;
    const name = window.prompt("Sheet name", active.name);
    if (!name?.trim() || name.trim() === active.name) return;
    try {
      await api.renameNoteSheet(active.id, name.trim());
      await loadSheets();
    } catch (e) {
      toast.error(errMsg(e));
    }
  }

  async function deleteSheet() {
    if (!active) return;
    if (!window.confirm(`Delete "${active.name}"? Every row goes with it, on both computers.`)) return;
    try {
      await api.deleteNoteSheet(active.id);
      setActiveId(null);
      await loadSheets();
      await loadAlerts();
    } catch (e) {
      toast.error(errMsg(e));
    }
  }

  async function importOldNotes() {
    setImporting(true);
    try {
      const sheet = await api.createNoteSheet("Notes from the old version", ["Title", "Text", "Date", "Tags"]);
      for (const n of oldNotes) {
        await api.createNoteRow(sheet.id, [n.title, n.content, n.dueDate ?? "", n.tags.map((t) => `#${t}`).join(" ")]);
      }
      setOldNotes([]);
      await loadSheets();
      setActiveId(sheet.id);
      toast.success(`${oldNotes.length} brought across`);
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setImporting(false);
    }
  }

  const g = () => gridRef.current;
  const menus: Menu[] = [
    {
      label: "File",
      items: [
        { kind: "item", label: "New sheet…", onClick: () => setNewOpen(true) },
        { kind: "item", label: "Rename sheet…", onClick: renameSheet },
        { kind: "sep" },
        { kind: "item", label: "Delete sheet", danger: true, onClick: deleteSheet },
      ],
    },
    {
      label: "Edit",
      items: [
        { kind: "item", label: "Clear cell", hint: "Del", onClick: () => g()?.clearCell() },
        { kind: "sep" },
        { kind: "item", label: "Delete row", danger: true, onClick: () => g()?.deleteRow() },
        { kind: "item", label: "Delete column", danger: true, onClick: () => g()?.deleteColumn() },
      ],
    },
    {
      label: "View",
      items: ZOOMS.map((z) => ({
        kind: "check" as const,
        label: `${Math.round(z * 100)} %`,
        on: zoom === z,
        onClick: () => setZoom(z),
      })),
    },
    {
      label: "Insert",
      items: [
        { kind: "item", label: "10 rows below", onClick: () => g()?.addRows(10) },
        { kind: "item", label: "50 rows below", onClick: () => g()?.addRows(50) },
        { kind: "item", label: "Column at the end", onClick: () => g()?.addColumn() },
        { kind: "sep" },
        { kind: "item", label: "Today's date", hint: "into the cell", onClick: () => g()?.insertToday() },
        { kind: "item", label: "Reminder…", onClick: () => setAlertsOpen(true) },
      ],
    },
    {
      label: "Format",
      items: [
        { kind: "item", label: `Label for column ${colLetter(sel.c)}…`, onClick: () => g()?.renameColumn() },
        { kind: "sep" },
        { kind: "item", label: "Move column left", onClick: () => g()?.moveColumn(-1) },
        { kind: "item", label: "Move column right", onClick: () => g()?.moveColumn(1) },
      ],
    },
    {
      label: "Data",
      items: [
        { kind: "item", label: `Sort by ${colLetter(sel.c)} · A→Z`, onClick: () => g()?.sortBySelected("asc") },
        { kind: "item", label: `Sort by ${colLetter(sel.c)} · Z→A`, onClick: () => g()?.sortBySelected("desc") },
        { kind: "item", label: "Clear sort", onClick: () => g()?.sortBySelected(null) },
      ],
    },
  ];

  /* ------------------------------- render ------------------------------- */

  if (sheets === null) {
    return (
      <div className="p-1">
        <TableSkeleton />
      </div>
    );
  }

  return (
    <div className="flex min-h-[440px] flex-col" style={{ height: "calc(100vh - 150px)" }}>
      {oldNotes.length > 0 && (
        <div className="mb-3 flex flex-wrap items-center gap-3 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 dark:border-amber-900 dark:bg-amber-950/30">
          <p className="flex-1 text-sm text-amber-900 dark:text-amber-200">
            You have {oldNotes.length} {oldNotes.length === 1 ? "note" : "notes"} from the previous version. Bring them
            across as a sheet?
          </p>
          <Button variant="secondary" onClick={importOldNotes} disabled={importing}>
            {importing ? "Bringing them across…" : "Import as a sheet"}
          </Button>
        </div>
      )}

      {/* ── title ─────────────────────────────────────────────────────── */}
      <div className="flex flex-wrap items-center gap-2">
        <button
          type="button"
          onClick={renameSheet}
          className="text-[17px] font-semibold text-slate-900 hover:underline dark:text-slate-50"
          title="Rename"
        >
          {active?.name ?? "Sheets"}
        </button>
        <span className="text-xs text-slate-500 dark:text-slate-400">
          {active ? `${rows.length} rows · ${active.columns.length} columns` : ""}
        </span>
        <div className="relative ml-auto w-full max-w-[240px]">
          <IconSearch className="pointer-events-none absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-slate-500 dark:text-slate-400" />
          <Input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search every sheet…"
            aria-label="Search every sheet"
            className="!py-1 pl-8 text-[12.5px]"
          />
        </div>
        <button
          type="button"
          onClick={() => setAlertsOpen(true)}
          title="Reminders"
          className={`relative rounded-md border px-2 py-1.5 transition ${
            dueCount > 0
              ? "border-amber-400 bg-amber-50 text-amber-700 dark:border-amber-700 dark:bg-amber-950/40 dark:text-amber-300"
              : "border-slate-200 text-slate-500 hover:bg-surface-sunken dark:border-slate-700 dark:text-slate-400"
          }`}
        >
          <IconBell className="h-4 w-4" />
          {dueCount > 0 && (
            <span className="absolute -right-1.5 -top-1.5 min-w-[16px] rounded-full bg-amber-500 px-1 text-[10px] font-bold leading-4 text-white">
              {dueCount}
            </span>
          )}
        </button>
      </div>

      {hits !== null ? (
        <div className="mt-4 flex flex-col gap-2">
          {hits.length === 0 ? (
            <EmptyState
              icon={<IconSearch className="h-8 w-8" />}
              title="Nothing found"
              description={`No sheet contains "${query.trim()}".`}
            />
          ) : (
            hits.map((h) => (
              <button
                key={`${h.id}-${h.whereFound}`}
                type="button"
                onClick={() => {
                  setQuery("");
                  setActiveId(h.id);
                }}
                className="rounded-lg border border-slate-200 px-3 py-2 text-left transition hover:bg-surface-sunken dark:border-slate-700"
              >
                <div className="flex flex-wrap items-baseline gap-x-2">
                  <span className="text-sm font-semibold text-slate-900 dark:text-slate-50">{h.title}</span>
                  <span className="text-[11px] uppercase tracking-wide text-slate-400 dark:text-slate-500">
                    {h.whereFound}
                  </span>
                </div>
                {h.preview && <p className="mt-0.5 truncate text-xs text-slate-500 dark:text-slate-400">{h.preview}</p>}
              </button>
            ))
          )}
        </div>
      ) : sheets.length === 0 ? (
        <div className="mt-6">
          <EmptyState
            icon={<IconLayoutGrid className="h-8 w-8" />}
            title="No sheets yet"
            description="A sheet is an empty grid — columns are just A, B, C and you write whatever you want, wherever you want."
            action={
              <Button variant="primary" onClick={() => setNewOpen(true)}>
                <IconPlus className="h-4 w-4" /> New sheet
              </Button>
            }
          />
        </div>
      ) : (
        <>
          {/* ── menu + toolbar + value bar ─────────────────────────────── */}
          <div className="mt-1.5 flex flex-wrap items-center gap-2 border-b border-slate-200 pb-1.5 dark:border-slate-800">
            <MenuBar menus={menus} />
            <span className="mx-1 hidden h-4 w-px bg-slate-200 dark:bg-slate-700 sm:block" />
            <button
              type="button"
              onClick={() => g()?.insertToday()}
              className="rounded border border-slate-200 px-2 py-0.5 text-[12px] text-slate-600 transition hover:bg-surface-sunken dark:border-slate-700 dark:text-slate-300"
            >
              Today
            </button>
            <button
              type="button"
              onClick={() => g()?.addColumn()}
              className="rounded border border-slate-200 px-2 py-0.5 text-[12px] text-slate-600 transition hover:bg-surface-sunken dark:border-slate-700 dark:text-slate-300"
            >
              + Column
            </button>
            <div className="relative w-[170px]">
              <Input
                value={filter}
                onChange={(e) => setFilter(e.target.value)}
                placeholder="Filter rows…"
                aria-label="Filter rows in this sheet"
                className="!py-0.5 text-[12px]"
              />
            </div>
            <div className="ml-auto flex items-center gap-1 text-[11px] text-slate-500 dark:text-slate-400">
              <button
                type="button"
                onClick={() => setZoom((z) => ZOOMS[Math.max(0, ZOOMS.indexOf(z) - 1)] ?? z)}
                className="rounded px-1.5 py-0.5 hover:bg-surface-sunken"
                aria-label="Zoom out"
              >
                −
              </button>
              <span className="w-10 text-center tabular-nums">{Math.round(zoom * 100)} %</span>
              <button
                type="button"
                onClick={() => setZoom((z) => ZOOMS[Math.min(ZOOMS.length - 1, ZOOMS.indexOf(z) + 1)] ?? z)}
                className="rounded px-1.5 py-0.5 hover:bg-surface-sunken"
                aria-label="Zoom in"
              >
                +
              </button>
            </div>
          </div>

          <div className="flex items-stretch border-b border-slate-200 text-[12.5px] dark:border-slate-800">
            <div className="w-[62px] border-r border-slate-200 px-2 py-1 font-semibold text-slate-700 dark:border-slate-800 dark:text-slate-200">
              {cellRef(sel.r, sel.c)}
            </div>
            <div className="border-r border-slate-200 px-2 py-1 italic text-slate-400 dark:border-slate-800 dark:text-slate-500">
              fx
            </div>
            <div className="min-w-0 flex-1 truncate px-2 py-1 text-slate-700 dark:text-slate-200">{cellValue}</div>
          </div>

          {/* ── grid ───────────────────────────────────────────────────── */}
          <div className="min-h-0 flex-1">
            {loadingRows || !active ? (
              <TableSkeleton />
            ) : (
              <Grid
                key={active.id}
                sheet={active}
                rows={rows}
                setRows={setRows}
                zoom={zoom}
                filter={filter}
                sel={sel}
                setSel={setSel}
                alerts={alerts}
                onSheetChanged={() => void loadSheets()}
                onCellValue={(v, rowId) => {
                  setCellValue(v);
                  setSelRowId(rowId);
                }}
                bind={bind}
              />
            )}
          </div>

          {/* ── bottom tab strip ───────────────────────────────────────── */}
          <div className="flex items-center gap-0.5 overflow-x-auto border-t border-slate-200 bg-surface-sunken px-1 pt-1 dark:border-slate-800">
            {sheets.map((s) => (
              <button
                key={s.id}
                type="button"
                onClick={() => setActiveId(s.id)}
                onDoubleClick={renameSheet}
                className={`shrink-0 rounded-t-md border border-b-0 px-3 py-1.5 text-[12.5px] transition ${
                  s.id === activeId
                    ? "border-slate-200 border-t-2 border-t-brand-600 bg-surface font-semibold text-brand-700 dark:border-slate-700 dark:border-t-brand-500 dark:text-brand-300"
                    : "border-transparent text-slate-500 hover:text-slate-800 dark:text-slate-400 dark:hover:text-slate-100"
                }`}
                title={`${s.rowCount} rows`}
              >
                {s.name}
              </button>
            ))}
            <button
              type="button"
              onClick={() => setNewOpen(true)}
              className="shrink-0 rounded px-2.5 py-1 text-base leading-none text-slate-500 transition hover:bg-surface hover:text-brand-600 dark:text-slate-400"
              title="New sheet"
              aria-label="New sheet"
            >
              +
            </button>
          </div>
        </>
      )}

      <NewSheetModal
        open={newOpen}
        onClose={() => setNewOpen(false)}
        onCreated={async (sheet) => {
          setNewOpen(false);
          await loadSheets();
          setActiveId(sheet.id);
        }}
      />

      <AlertsPanel
        open={alertsOpen}
        onClose={() => setAlertsOpen(false)}
        sheetId={active?.id ?? 0}
        sheetName={active?.name ?? ""}
        anchor={active ? { rowId: selRowId, colIndex: sel.c, r: sel.r } : null}
        alerts={alerts}
        onChanged={() => void loadAlerts()}
      />
    </div>
  );
}

/* ------------------------------------------------------------------ *
 * New sheet
 * ------------------------------------------------------------------ */

function NewSheetModal({
  open,
  onClose,
  onCreated,
}: {
  open: boolean;
  onClose: () => void;
  onCreated: (sheet: NoteSheet) => void;
}) {
  const toast = useToast();
  const [name, setName] = useState("");
  const [cols, setCols] = useState(14);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!open) return;
    setName("");
    setCols(14);
  }, [open]);

  async function create() {
    setSaving(true);
    try {
      // Unlabelled columns: a new sheet is an empty grid, not a form to fill in.
      onCreated(await api.createNoteSheet(name.trim() || "Sheet", Array.from({ length: cols }, () => "")));
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setSaving(false);
    }
  }

  return (
    <Modal open={open} onClose={onClose} title="New sheet" width="max-w-md">
      <div className="flex flex-col gap-3">
        <label>
          <span className="label">Name</span>
          <Input
            value={name}
            onChange={(e) => setName(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !saving) void create();
            }}
            placeholder="Oasis codes"
            aria-label="Sheet name"
          />
        </label>
        <label>
          <span className="label">Columns</span>
          <Input
            type="number"
            min={1}
            max={60}
            value={cols}
            onChange={(e) => setCols(Math.max(1, Math.min(60, Number(e.target.value) || 1)))}
            aria-label="Number of columns"
          />
        </label>
        <p className="text-xs text-slate-500 dark:text-slate-400">
          Columns start as plain A, B, C — no names, no rules about what goes in them. You can give any of them a label
          later, add more, or move them around.
        </p>
      </div>
      <ModalFooter>
        <Button variant="secondary" onClick={onClose} disabled={saving}>
          Cancel
        </Button>
        <Button variant="primary" onClick={create} disabled={saving}>
          Create
        </Button>
      </ModalFooter>
    </Modal>
  );
}

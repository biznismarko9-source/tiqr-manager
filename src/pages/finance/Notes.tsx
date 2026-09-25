import { useCallback, useEffect, useMemo, useState } from "react";
import { api, errMsg } from "../../lib/api";
import type { NoteHit, NoteRow, NoteSheet } from "../../lib/types";
import {
  Button,
  ConfirmDialog,
  EmptyState,
  Input,
  Modal,
  ModalFooter,
  Select,
  TableSkeleton,
} from "../../components/ui";
import { IconPlus, IconSearch, IconTrash, IconX } from "../../components/icons";
import { useToast } from "../../lib/toast";
import { formatDateNumeric } from "../../lib/format";

/**
 * Notes - the place marko keeps everything the rest of the app has no column
 * for.
 *
 * 2.51.0. His own brief: "miesto na ktore sa mozem spolahnut, nieco kde si
 * viem zapisat napr aky kod a co som komu predal, aky je jeho nick, ake info
 * treba mat ulozene ... mozno nejake plany, ulozit si ucty, prehlad, najst
 * vsetky jednoducho". Offered three shapes, he picked sheets with columns he
 * names himself, standalone rather than attached to an event or order.
 *
 * Three things make it a place you can rely on rather than a second inbox:
 *
 *   1. **One search across every sheet.** A nick, a ticket code, half a name -
 *      it finds the row and says which sheet and which column it was in.
 *   2. **Templates.** A new sheet starts as Buyers / Accounts / Plans /
 *      Blank, so it is useful the moment it exists instead of an empty grid
 *      that has to be configured before it can be typed into.
 *   3. **It saves itself.** A cell writes on blur, not on a Save button, so
 *      there is no state in which something typed is not yet kept.
 *
 * It syncs between his two machines like everything else: the tables carry
 * `uid` and tombstones and are listed in `MERGE_TABLES` (see migration 031).
 */

/** Marko's own examples, turned into starting points. Titles are the columns;
 *  nothing here is enforced afterwards - a sheet is his to reshape. */
const TEMPLATES: { key: string; label: string; description: string; columns: string[] }[] = [
  {
    key: "buyers",
    label: "Buyers",
    description: "Who bought what, their nick, and how to reach them.",
    columns: ["Nick", "Ticket code", "Event", "Paid", "Contact", "Note"],
  },
  {
    key: "accounts",
    label: "Accounts",
    description: "Platform logins and what each one is for.",
    columns: ["Platform", "Account", "Email", "Note"],
  },
  {
    key: "plans",
    label: "Plans",
    description: "What is coming up and by when.",
    columns: ["What", "By when", "Status", "Note"],
  },
  { key: "blank", label: "Blank", description: "One column, name it yourself.", columns: ["Note"] },
];

export default function Notes() {
  const toast = useToast();
  const [sheets, setSheets] = useState<NoteSheet[] | null>(null);
  const [activeId, setActiveId] = useState<number | null>(null);
  const [rows, setRows] = useState<NoteRow[] | null>(null);
  const [query, setQuery] = useState("");
  const [hits, setHits] = useState<NoteHit[] | null>(null);
  const [newOpen, setNewOpen] = useState(false);
  const [confirmSheet, setConfirmSheet] = useState<NoteSheet | null>(null);
  const [confirmColumn, setConfirmColumn] = useState<number | null>(null);

  const active = useMemo(() => sheets?.find((s) => s.id === activeId) ?? null, [sheets, activeId]);

  const loadSheets = useCallback(async () => {
    try {
      const list = await api.listNoteSheets();
      setSheets(list);
      setActiveId((current) => (current && list.some((s) => s.id === current) ? current : (list[0]?.id ?? null)));
    } catch (e) {
      toast.error(errMsg(e));
      setSheets([]);
    }
  }, [toast]);

  useEffect(() => {
    void loadSheets();
  }, [loadSheets]);

  useEffect(() => {
    if (activeId === null) {
      setRows([]);
      return;
    }
    let alive = true;
    setRows(null);
    api
      .listNoteRows(activeId)
      .then((r) => {
        if (alive) setRows(r);
      })
      .catch((e) => {
        if (alive) {
          toast.error(errMsg(e));
          setRows([]);
        }
      });
    return () => {
      alive = false;
    };
  }, [activeId, toast]);

  // Search runs a short moment after typing stops, not on every keystroke -
  // it reads every row in every sheet.
  useEffect(() => {
    const q = query.trim();
    if (!q) {
      setHits(null);
      return;
    }
    const t = setTimeout(() => {
      api
        .searchNotes(q)
        .then(setHits)
        .catch((e) => toast.error(errMsg(e)));
    }, 200);
    return () => clearTimeout(t);
  }, [query, toast]);

  async function saveCell(row: NoteRow, index: number, value: string) {
    if (row.cells[index] === value) return;
    const next = [...row.cells];
    next[index] = value;
    // Written straight away, and the local copy is already showing it - a
    // failure puts the stored value back rather than leaving a lie on screen.
    setRows((rs) => rs?.map((r) => (r.id === row.id ? { ...r, cells: next } : r)) ?? rs);
    try {
      const saved = await api.updateNoteRow(row.id, next);
      setRows((rs) => rs?.map((r) => (r.id === saved.id ? saved : r)) ?? rs);
    } catch (e) {
      toast.error(errMsg(e));
      setRows((rs) => rs?.map((r) => (r.id === row.id ? row : r)) ?? rs);
    }
  }

  async function addRow() {
    if (!active) return;
    try {
      const created = await api.createNoteRow(active.id, active.columns.map(() => ""));
      setRows((rs) => [...(rs ?? []), created]);
      void loadSheets();
    } catch (e) {
      toast.error(errMsg(e));
    }
  }

  async function removeRow(id: number) {
    try {
      await api.deleteNoteRow(id);
      setRows((rs) => rs?.filter((r) => r.id !== id) ?? rs);
      void loadSheets();
    } catch (e) {
      toast.error(errMsg(e));
    }
  }

  async function addColumn() {
    if (!active) return;
    const name = window.prompt("Column name");
    if (!name?.trim()) return;
    try {
      await api.addNoteColumn(active.id, name.trim());
      await loadSheets();
      setRows(await api.listNoteRows(active.id));
    } catch (e) {
      toast.error(errMsg(e));
    }
  }

  async function renameColumn(index: number) {
    if (!active) return;
    const name = window.prompt("Column name", active.columns[index]);
    if (!name?.trim() || name.trim() === active.columns[index]) return;
    try {
      await api.renameNoteColumn(active.id, index, name.trim());
      await loadSheets();
    } catch (e) {
      toast.error(errMsg(e));
    }
  }

  async function removeColumn(index: number) {
    if (!active) return;
    try {
      await api.deleteNoteColumn(active.id, index);
      await loadSheets();
      setRows(await api.listNoteRows(active.id));
    } catch (e) {
      toast.error(errMsg(e));
    }
    setConfirmColumn(null);
  }

  return (
    <div>
      {/* 2.51.1: Notes moved from its own sidebar entry to a tab inside
          Finance - marko: "to urob ako vlastnu zlozku pod finance a tam to
          bude cele". Finance owns the page header, so this keeps only its own
          toolbar and the sidebar stays flat (PROTECTED_AREAS, 2.43.0). */}
      <div className="mb-4 flex flex-wrap items-end justify-between gap-3">
        <div className="w-full max-w-md">
          <span className="label">Search every sheet</span>
          <div className="relative">
            <IconSearch className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-500 dark:text-slate-400" />
            <Input
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="A nick, a ticket code, a name…"
              className="pl-9"
            />
          </div>
        </div>
        <Button variant="primary" onClick={() => setNewOpen(true)}>
          <IconPlus className="h-4 w-4" /> New sheet
        </Button>
      </div>

      {hits !== null ? (
        <SearchResults
          hits={hits}
          query={query}
          onOpen={(sheetId) => {
            setActiveId(sheetId);
            setQuery("");
          }}
        />
      ) : sheets === null ? (
        <TableSkeleton />
      ) : sheets.length === 0 ? (
        <EmptyState
          icon={<IconPlus className="h-8 w-8" />}
          title="No sheets yet"
          description="A sheet is a table with columns you name yourself. Start from one of the templates, or a blank one."
          action={
            <Button variant="primary" onClick={() => setNewOpen(true)}>
              <IconPlus className="h-4 w-4" /> New sheet
            </Button>
          }
        />
      ) : (
        <div className="grid gap-4 lg:grid-cols-[240px_1fr]">
          <aside className="flex flex-col gap-1">
            {sheets.map((s) => (
              <button
                key={s.id}
                type="button"
                onClick={() => setActiveId(s.id)}
                className={`rounded-lg px-3 py-2 text-left text-sm transition ${
                  s.id === activeId
                    ? "bg-brand-600 text-white"
                    : "text-slate-600 hover:bg-surface-sunken dark:text-slate-400"
                }`}
              >
                <span className="block truncate font-medium">{s.name}</span>
                <span className={`text-[11px] ${s.id === activeId ? "text-white/70" : "text-slate-500 dark:text-slate-500"}`}>
                  {s.rowCount} {s.rowCount === 1 ? "row" : "rows"} · {formatDateNumeric(s.updatedAt.slice(0, 10))}
                </span>
              </button>
            ))}
          </aside>

          {active && (
            <section>
              <div className="mb-2 flex flex-wrap items-center gap-2">
                <h2 className="text-sm font-semibold text-slate-900 dark:text-slate-50">{active.name}</h2>
                <Button
                  variant="secondary"
                  onClick={async () => {
                    const name = window.prompt("Sheet name", active.name);
                    if (!name?.trim() || name.trim() === active.name) return;
                    try {
                      await api.renameNoteSheet(active.id, name.trim());
                      await loadSheets();
                    } catch (e) {
                      toast.error(errMsg(e));
                    }
                  }}
                >
                  Rename
                </Button>
                <Button variant="secondary" onClick={addColumn}>
                  <IconPlus className="h-4 w-4" /> Column
                </Button>
                <Button variant="secondary" className="ml-auto" onClick={() => setConfirmSheet(active)}>
                  <IconTrash className="h-4 w-4" /> Delete sheet
                </Button>
              </div>

              {rows === null ? (
                <TableSkeleton />
              ) : (
                <>
                  <div className="table-shell table-shell-compact mb-3">
                    <table className="w-full border-collapse">
                      <thead className="sticky top-0 z-10 bg-surface">
                        <tr>
                          {active.columns.map((c, i) => (
                            <th key={i} className="th-c whitespace-nowrap">
                              <span className="inline-flex items-center gap-1">
                                <button
                                  type="button"
                                  onClick={() => void renameColumn(i)}
                                  title="Rename this column"
                                  className="hover:text-slate-900 dark:hover:text-slate-100"
                                >
                                  {c}
                                </button>
                                {active.columns.length > 1 && (
                                  <button
                                    type="button"
                                    onClick={() => setConfirmColumn(i)}
                                    aria-label={`Delete column ${c}`}
                                    title={`Delete column ${c}`}
                                    className="text-slate-400 hover:text-red-600 dark:text-slate-500 dark:hover:text-red-400"
                                  >
                                    <IconX className="h-3 w-3" />
                                  </button>
                                )}
                              </span>
                            </th>
                          ))}
                          <th className="th-c w-[44px]" />
                        </tr>
                      </thead>
                      <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
                        {rows.map((r) => (
                          <tr key={r.id}>
                            {active.columns.map((_, i) => (
                              <td key={i} className="td-c">
                                <Input
                                  defaultValue={r.cells[i] ?? ""}
                                  onBlur={(e) => void saveCell(r, i, e.target.value)}
                                  aria-label={`${active.columns[i]}, row ${r.position + 1}`}
                                />
                              </td>
                            ))}
                            <td className="td-c w-[44px]">
                              <button
                                type="button"
                                onClick={() => void removeRow(r.id)}
                                aria-label="Delete row"
                                title="Delete row"
                                className="rounded-md p-1 text-slate-400 transition hover:bg-slate-100 hover:text-red-600 dark:text-slate-500 dark:hover:bg-slate-800 dark:hover:text-red-400"
                              >
                                <IconX className="h-4 w-4" />
                              </button>
                            </td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                  <Button variant="secondary" onClick={addRow}>
                    <IconPlus className="h-4 w-4" /> Add row
                  </Button>
                  {rows.length === 0 && (
                    <p className="mt-3 text-xs text-slate-500 dark:text-slate-400">
                      Nothing in this sheet yet. Every cell saves itself as soon as you click out of it.
                    </p>
                  )}
                </>
              )}
            </section>
          )}
        </div>
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

      <ConfirmDialog
        open={confirmSheet !== null}
        title={`Delete "${confirmSheet?.name}"?`}
        message="Every row in this sheet goes with it, on this computer and the other one. This cannot be undone."
        confirmLabel="Delete sheet"
        danger
        onCancel={() => setConfirmSheet(null)}
        onConfirm={async () => {
          if (!confirmSheet) return;
          try {
            await api.deleteNoteSheet(confirmSheet.id);
            setConfirmSheet(null);
            setActiveId(null);
            await loadSheets();
          } catch (e) {
            toast.error(errMsg(e));
          }
        }}
      />

      <ConfirmDialog
        open={confirmColumn !== null}
        title={confirmColumn !== null ? `Delete column "${active?.columns[confirmColumn]}"?` : ""}
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

function SearchResults({
  hits,
  query,
  onOpen,
}: {
  hits: NoteHit[];
  query: string;
  onOpen: (sheetId: number) => void;
}) {
  if (hits.length === 0) {
    return (
      <EmptyState
        icon={<IconSearch className="h-8 w-8" />}
        title="Nothing found"
        description={`No row in any sheet contains "${query.trim()}".`}
      />
    );
  }
  return (
    <div className="table-shell table-shell-compact">
      <table className="w-full border-collapse">
        <thead>
          <tr>
            <th className="th-c">Sheet</th>
            <th className="th-c">Found in</th>
            <th className="th-c">Row</th>
          </tr>
        </thead>
        <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
          {hits.map((h) => (
            <tr key={`${h.sheetId}-${h.rowId}`} className="cursor-pointer" onClick={() => onOpen(h.sheetId)}>
              <td className="td-c font-medium text-slate-900 dark:text-slate-100">{h.sheetName}</td>
              <td className="td-c text-slate-500 dark:text-slate-400">{h.columnName}</td>
              <td className="td-c">{h.cells.filter(Boolean).join(" · ")}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

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
  const [template, setTemplate] = useState(TEMPLATES[0].key);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!open) return;
    setName("");
    setTemplate(TEMPLATES[0].key);
  }, [open]);

  const chosen = TEMPLATES.find((t) => t.key === template) ?? TEMPLATES[0];

  return (
    <Modal open={open} onClose={onClose} title="New sheet" width="max-w-lg">
      <div className="flex flex-col gap-3">
        <label>
          <span className="label">Name</span>
          <Input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder={chosen.label}
            aria-label="Sheet name"
          />
        </label>
        <label>
          <span className="label">Start from</span>
          <Select value={template} onChange={(e) => setTemplate(e.target.value)}>
            {TEMPLATES.map((t) => (
              <option key={t.key} value={t.key}>
                {t.label}
              </option>
            ))}
          </Select>
        </label>
        <p className="text-xs text-slate-500 dark:text-slate-400">
          {chosen.description} Columns: {chosen.columns.join(", ")}. You can add, rename and remove them afterwards.
        </p>
      </div>
      <ModalFooter>
        <Button variant="secondary" onClick={onClose} disabled={saving}>
          Cancel
        </Button>
        <Button
          variant="primary"
          disabled={saving}
          onClick={async () => {
            setSaving(true);
            try {
              const sheet = await api.createNoteSheet(name.trim() || chosen.label, chosen.columns);
              toast.success(`Sheet ${sheet.name} created`);
              onCreated(sheet);
            } catch (e) {
              toast.error(errMsg(e));
            } finally {
              setSaving(false);
            }
          }}
        >
          Create sheet
        </Button>
      </ModalFooter>
    </Modal>
  );
}

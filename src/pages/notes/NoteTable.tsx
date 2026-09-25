import { useCallback, useEffect, useMemo, useState } from "react";
import { api, errMsg } from "../../lib/api";
import type { NoteRow, NoteSheet } from "../../lib/types";
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
import {
  IconArrowLeft,
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
 * One table, open for editing - the lightweight spreadsheet half of Notes.
 *
 * The engine is unchanged since 2.51.0 (migration 031): `note_sheets` /
 * `note_columns` / `note_rows`, every cell saving itself on blur, no Save
 * button, so there is never a state in which something typed is not kept.
 *
 * 2.53.0 adds what marko asked for on top of it - sort, filter and column
 * reorder - and drops the cross-sheet search that used to live here, because
 * the Notes home now searches notes and tables together in one field.
 *
 * Sort and filter are entirely on screen: they change which rows are drawn and
 * in what order, and never touch stored positions. Reorder is the opposite -
 * it is a real change to the sheet, so it goes through the backend, where one
 * transaction moves the column and the matching cell in every row.
 */

type Sort = { index: number; dir: "asc" | "desc" } | null;

export default function NoteTable({
  sheetId,
  onBack,
  onChanged,
}: {
  sheetId: number;
  onBack: () => void;
  onChanged: () => void;
}) {
  const toast = useToast();
  const [sheet, setSheet] = useState<NoteSheet | null | undefined>(undefined);
  const [rows, setRows] = useState<NoteRow[] | null>(null);
  const [filter, setFilter] = useState("");
  const [sort, setSort] = useState<Sort>(null);
  const [confirmSheet, setConfirmSheet] = useState(false);
  const [confirmColumn, setConfirmColumn] = useState<number | null>(null);
  /** Bumped whenever the COLUMNS change shape. The cell inputs are
   *  uncontrolled (`defaultValue`, so typing is never fought by a re-render),
   *  and an uncontrolled input ignores a new defaultValue. Without this, after
   *  deleting or moving a column React reuses the old `<td>` elements and the
   *  screen keeps showing the cells in their old order while the database has
   *  them in the new one. Putting the version in the row key remounts them. */
  const [structure, setStructure] = useState(0);

  const loadSheet = useCallback(async () => {
    try {
      const list = await api.listNoteSheets();
      // `null` means it is gone - deleted here, or deleted on the other
      // machine and removed by a merge. Saying so beats a skeleton forever.
      setSheet(list.find((s) => s.id === sheetId) ?? null);
    } catch (e) {
      toast.error(errMsg(e));
    }
  }, [sheetId, toast]);

  const loadRows = useCallback(async () => {
    try {
      setRows(await api.listNoteRows(sheetId));
    } catch (e) {
      toast.error(errMsg(e));
      setRows([]);
    }
  }, [sheetId, toast]);

  useEffect(() => {
    void loadSheet();
    void loadRows();
  }, [loadSheet, loadRows]);

  const columns = sheet?.columns ?? [];

  /** Filter first, then sort. Both are display-only - `rows` keeps the stored
   *  order, so nothing here can write a row into the wrong place. */
  const visible = useMemo(() => {
    let list = rows ?? [];
    const q = filter.trim().toLowerCase();
    if (q) list = list.filter((r) => r.cells.some((c) => c.toLowerCase().includes(q)));
    if (sort) {
      const { index, dir } = sort;
      list = [...list].sort((a, b) => {
        const x = a.cells[index] ?? "";
        const y = b.cells[index] ?? "";
        // Empty cells sink to the bottom either way - an empty cell is not a
        // small value, it is a missing one.
        if (!x && !y) return a.position - b.position;
        if (!x) return 1;
        if (!y) return -1;
        const cmp = x.localeCompare(y, undefined, { numeric: true, sensitivity: "base" });
        return dir === "asc" ? cmp : -cmp;
      });
    }
    return list;
  }, [rows, filter, sort]);

  async function saveCell(row: NoteRow, index: number, value: string) {
    if ((row.cells[index] ?? "") === value) return;
    const next = [...row.cells];
    next[index] = value;
    // Written straight away, and the local copy already shows it - a failure
    // puts the stored value back rather than leaving a lie on screen.
    setRows((rs) => rs?.map((r) => (r.id === row.id ? { ...r, cells: next } : r)) ?? rs);
    try {
      const saved = await api.updateNoteRow(row.id, next);
      setRows((rs) => rs?.map((r) => (r.id === saved.id ? saved : r)) ?? rs);
    } catch (e) {
      toast.error(errMsg(e));
      setRows((rs) => rs?.map((r) => (r.id === row.id ? row : r)) ?? rs);
      setStructure((v) => v + 1); // put the shown value back too
    }
  }

  async function addRow() {
    try {
      const created = await api.createNoteRow(sheetId, columns.map(() => ""));
      setRows((rs) => [...(rs ?? []), created]);
      void loadSheet();
      onChanged();
    } catch (e) {
      toast.error(errMsg(e));
    }
  }

  async function removeRow(id: number) {
    try {
      await api.deleteNoteRow(id);
      setRows((rs) => rs?.filter((r) => r.id !== id) ?? rs);
      void loadSheet();
      onChanged();
    } catch (e) {
      toast.error(errMsg(e));
    }
  }

  /** Every column operation reloads both halves and remounts the cells, so
   *  what is on screen is always what came back from the database. */
  async function afterColumnChange() {
    await loadSheet();
    await loadRows();
    setStructure((v) => v + 1);
    setSort(null);
    onChanged();
  }

  async function addColumn() {
    const name = window.prompt("Column name");
    if (!name?.trim()) return;
    try {
      await api.addNoteColumn(sheetId, name.trim());
      await afterColumnChange();
    } catch (e) {
      toast.error(errMsg(e));
    }
  }

  async function renameColumn(index: number) {
    const name = window.prompt("Column name", columns[index]);
    if (!name?.trim() || name.trim() === columns[index]) return;
    try {
      await api.renameNoteColumn(sheetId, index, name.trim());
      await afterColumnChange();
    } catch (e) {
      toast.error(errMsg(e));
    }
  }

  async function removeColumn(index: number) {
    try {
      await api.deleteNoteColumn(sheetId, index);
      await afterColumnChange();
    } catch (e) {
      toast.error(errMsg(e));
    }
    setConfirmColumn(null);
  }

  async function moveColumn(index: number, delta: -1 | 1) {
    const to = index + delta;
    if (to < 0 || to >= columns.length) return;
    try {
      await api.reorderNoteColumn(sheetId, index, to);
      await afterColumnChange();
    } catch (e) {
      toast.error(errMsg(e));
    }
  }

  function toggleSort(index: number) {
    setSort((s) => {
      if (!s || s.index !== index) return { index, dir: "asc" };
      if (s.dir === "asc") return { index, dir: "desc" };
      return null; // third click returns the sheet to its own order
    });
  }

  if (sheet === undefined) {
    return (
      <div>
        <Button variant="secondary" onClick={onBack}>
          <IconArrowLeft className="h-4 w-4" /> Notes
        </Button>
        <div className="mt-4">
          <TableSkeleton />
        </div>
      </div>
    );
  }

  if (sheet === null) {
    return (
      <EmptyState
        icon={<IconSearch className="h-8 w-8" />}
        title="This table is gone"
        description="It was deleted here or on the other computer."
        action={
          <Button variant="secondary" onClick={onBack}>
            <IconArrowLeft className="h-4 w-4" /> Back to Notes
          </Button>
        }
      />
    );
  }

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-center gap-2">
        <Button variant="secondary" onClick={onBack}>
          <IconArrowLeft className="h-4 w-4" /> Notes
        </Button>
        <h2 className="text-sm font-semibold text-slate-900 dark:text-slate-50">{sheet.name}</h2>
        <Button
          variant="secondary"
          onClick={async () => {
            const name = window.prompt("Table name", sheet.name);
            if (!name?.trim() || name.trim() === sheet.name) return;
            try {
              await api.renameNoteSheet(sheetId, name.trim());
              await loadSheet();
              onChanged();
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
        <Button variant="secondary" className="ml-auto" onClick={() => setConfirmSheet(true)}>
          <IconTrash className="h-4 w-4" /> Delete table
        </Button>
      </div>

      <div className="flex flex-wrap items-center gap-3">
        <div className="relative w-full max-w-sm">
          <IconSearch className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-500 dark:text-slate-400" />
          <Input
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            placeholder="Filter rows in this table…"
            aria-label="Filter rows"
            className="pl-9"
          />
        </div>
        {(filter.trim() || sort) && (
          <span className="text-xs text-slate-500 dark:text-slate-400">
            {visible.length} of {rows?.length ?? 0} rows
            {sort ? ` · sorted by ${columns[sort.index]} ${sort.dir === "asc" ? "A→Z" : "Z→A"}` : ""}
            {" · "}
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
      </div>

      {rows === null ? (
        <TableSkeleton />
      ) : (
        <>
          <div className="table-shell table-shell-compact">
            <table className="w-full border-collapse">
              <thead className="sticky top-0 z-10 bg-surface">
                <tr>
                  {columns.map((c, i) => (
                    <th key={i} className="th-c whitespace-nowrap">
                      <span className="inline-flex items-center gap-1">
                        <button
                          type="button"
                          onClick={() => toggleSort(i)}
                          title={`Sort by ${c}`}
                          className="inline-flex items-center gap-1 hover:text-slate-900 dark:hover:text-slate-100"
                        >
                          {c}
                          {sort?.index === i &&
                            (sort.dir === "asc" ? (
                              <IconChevronUp className="h-3 w-3" />
                            ) : (
                              <IconChevronDown className="h-3 w-3" />
                            ))}
                        </button>
                        <span className="inline-flex items-center text-slate-300 dark:text-slate-600">
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
                  ))}
                  <th className="th-c w-[44px]" />
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
                {visible.map((r) => (
                  <tr key={`${r.id}:${structure}`}>
                    {columns.map((_, i) => (
                      <td key={i} className="td-c">
                        <Input
                          defaultValue={r.cells[i] ?? ""}
                          onBlur={(e) => void saveCell(r, i, e.target.value)}
                          aria-label={`${columns[i]}, row ${r.position + 1}`}
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

          <div>
            <Button variant="secondary" onClick={addRow}>
              <IconPlus className="h-4 w-4" /> Add row
            </Button>
            {rows.length === 0 && (
              <p className="mt-3 text-xs text-slate-500 dark:text-slate-400">
                Nothing in this table yet. Every cell saves itself as soon as you click out of it.
              </p>
            )}
            {rows.length > 0 && visible.length === 0 && (
              <p className="mt-3 text-xs text-slate-500 dark:text-slate-400">
                No row in this table contains "{filter.trim()}".
              </p>
            )}
          </div>
        </>
      )}

      <ConfirmDialog
        open={confirmSheet}
        title={`Delete "${sheet.name}"?`}
        message="Every row in this table goes with it, on this computer and the other one. This cannot be undone."
        confirmLabel="Delete table"
        danger
        onCancel={() => setConfirmSheet(false)}
        onConfirm={async () => {
          try {
            await api.deleteNoteSheet(sheetId);
            setConfirmSheet(false);
            onChanged();
            onBack();
          } catch (e) {
            toast.error(errMsg(e));
          }
        }}
      />

      <ConfirmDialog
        open={confirmColumn !== null}
        title={confirmColumn !== null ? `Delete column "${columns[confirmColumn]}"?` : ""}
        message="What is written in that column is deleted from every row in this table. The other columns are untouched."
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

/* ------------------------------------------------------------------ *
 * New table
 * ------------------------------------------------------------------ */

/** marko's own examples, turned into starting points. A table is his to
 *  reshape afterwards - nothing here is enforced. */
const TEMPLATES: { key: string; label: string; columns: string[] }[] = [
  { key: "blank", label: "Blank", columns: ["Column 1"] },
  { key: "codes", label: "Codes", columns: ["Date", "Buyer", "Nick", "Code", "Status", "Notes"] },
  { key: "accounts", label: "Accounts", columns: ["Platform", "Account", "Email", "Status", "Notes"] },
  { key: "plans", label: "Plans", columns: ["What", "By when", "Status", "Notes"] },
];

export function NewTableModal({
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

  async function create() {
    setSaving(true);
    try {
      const sheet = await api.createNoteSheet(name.trim() || chosen.label, chosen.columns);
      onCreated(sheet);
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setSaving(false);
    }
  }

  return (
    <Modal open={open} onClose={onClose} title="New table" width="max-w-lg">
      <div className="flex flex-col gap-3">
        <label>
          <span className="label">Name</span>
          <Input
            value={name}
            onChange={(e) => setName(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !saving) void create();
            }}
            placeholder="Oasis Codes"
            aria-label="Table name"
          />
        </label>
        <label>
          <span className="label">Columns</span>
          <Select value={template} onChange={(e) => setTemplate(e.target.value)}>
            {TEMPLATES.map((t) => (
              <option key={t.key} value={t.key}>
                {t.label}
              </option>
            ))}
          </Select>
        </label>
        <p className="text-xs text-slate-500 dark:text-slate-400">
          {chosen.columns.join(" · ")} — add, rename, move or remove them afterwards.
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

export function NoTables({ onNew }: { onNew: () => void }) {
  return (
    <EmptyState
      icon={<IconPlus className="h-8 w-8" />}
      title="No tables yet"
      description="A table has columns you name yourself - dates, buyers, codes, whatever you need in rows."
      action={
        <Button variant="primary" onClick={onNew}>
          <IconPlus className="h-4 w-4" /> New table
        </Button>
      }
    />
  );
}

import { useCallback, useEffect, useMemo, useState } from "react";
import { api, errMsg } from "../lib/api";
import type { NoteSheet, WorkspaceHit, WorkspaceItem } from "../lib/types";
import {
  Button,
  EmptyState,
  Input,
  Modal,
  ModalFooter,
  PageHeader,
  Select,
  TableSkeleton,
} from "../components/ui";
import { IconLayoutGrid, IconPlus, IconSearch } from "../components/icons";
import { useToast } from "../lib/toast";
import { formatDateNumeric } from "../lib/format";
import Grid from "./sheets/Grid";

/**
 * Sheets - marko: "naozaj by som radsej urobil to ako realne google sheets
 * uplne jednoduche".
 *
 * So that is all this is. A list of sheets on the left, the sheet you are in
 * on the right, one search across all of them, and nothing else. The free-text
 * notes that 2.52.0 and 2.53.0 put beside the tables are gone - he chose
 * "len tabulky" when asked, and a section that does one thing is the whole
 * point of "uplne jednoduche".
 *
 * The data is untouched again: `note_sheets` / `note_columns` / `note_rows`
 * from migration 031, the same rows since 2.51.0. `workspace_items` (032) is
 * still in the database and still syncs; nothing reads it any more except the
 * one-click import below, which exists so anything typed into the 2.52/2.53
 * notes can be brought across instead of being stranded.
 */

export default function Sheets() {
  const toast = useToast();
  const [sheets, setSheets] = useState<NoteSheet[] | null>(null);
  const [activeId, setActiveId] = useState<number | null>(null);
  const [query, setQuery] = useState("");
  const [hits, setHits] = useState<WorkspaceHit[] | null>(null);
  const [newOpen, setNewOpen] = useState(false);
  const [oldNotes, setOldNotes] = useState<WorkspaceItem[]>([]);
  const [importing, setImporting] = useState(false);

  const load = useCallback(async () => {
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
    void load();
  }, [load]);

  // Anything written into the 2.52/2.53 notes. Almost always nothing, in which
  // case nothing about this page mentions it.
  useEffect(() => {
    api
      .listWorkspaceItems(true)
      .then((items) => setOldNotes(items.filter((i) => i.title.trim() || i.content.trim())))
      .catch(() => setOldNotes([]));
  }, []);

  // One search across every sheet - name and cells. It runs a moment after
  // typing stops, because it reads every row.
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

  const active = useMemo(() => sheets?.find((s) => s.id === activeId) ?? null, [sheets, activeId]);

  /** Brings the old notes across as one sheet, then leaves them alone. Nothing
   *  is deleted - if the import is wrong he still has the originals. */
  async function importOldNotes() {
    setImporting(true);
    try {
      const sheet = await api.createNoteSheet("Notes from the old version", ["Title", "Text", "Date", "Tags"]);
      for (const n of oldNotes) {
        await api.createNoteRow(sheet.id, [
          n.title,
          n.content,
          n.dueDate ?? "",
          n.tags.map((t) => `#${t}`).join(" "),
        ]);
      }
      setOldNotes([]);
      await load();
      setActiveId(sheet.id);
      toast.success(`${oldNotes.length} brought across`);
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setImporting(false);
    }
  }

  return (
    <div>
      <PageHeader
        title="Sheets"
        subtitle="Your own tables — codes, buyers, accounts, whatever you need in rows."
        actions={
          <div className="flex flex-wrap items-center gap-2">
            <div className="relative w-full min-w-[220px] sm:w-72">
              <IconSearch className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-500 dark:text-slate-400" />
              <Input
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                placeholder="Search every sheet…"
                aria-label="Search every sheet"
                className="pl-9"
              />
            </div>
            <Button variant="primary" onClick={() => setNewOpen(true)}>
              <IconPlus className="h-4 w-4" /> New sheet
            </Button>
          </div>
        }
      />

      {oldNotes.length > 0 && (
        <div className="mb-4 flex flex-wrap items-center gap-3 rounded-xl border border-amber-200 bg-amber-50 px-4 py-3 dark:border-amber-900 dark:bg-amber-950/30">
          <p className="flex-1 text-sm text-amber-900 dark:text-amber-200">
            You have {oldNotes.length} {oldNotes.length === 1 ? "note" : "notes"} from the previous version. Sheets no
            longer shows notes — bring them across as a sheet?
          </p>
          <Button variant="secondary" onClick={importOldNotes} disabled={importing}>
            {importing ? "Bringing them across…" : "Import as a sheet"}
          </Button>
        </div>
      )}

      {hits !== null ? (
        <SearchResults
          hits={hits}
          query={query}
          onOpen={(id) => {
            setQuery("");
            setActiveId(id);
          }}
        />
      ) : sheets === null ? (
        <TableSkeleton />
      ) : sheets.length === 0 ? (
        <EmptyState
          icon={<IconLayoutGrid className="h-8 w-8" />}
          title="No sheets yet"
          description="A sheet is a table with columns you name yourself. Click a cell and type — it saves itself."
          action={
            <Button variant="primary" onClick={() => setNewOpen(true)}>
              <IconPlus className="h-4 w-4" /> New sheet
            </Button>
          }
        />
      ) : (
        <div className="grid gap-4 lg:grid-cols-[220px_1fr]">
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
                <span
                  className={`text-[11px] ${
                    s.id === activeId ? "text-white/70" : "text-slate-500 dark:text-slate-500"
                  }`}
                >
                  {s.rowCount} {s.rowCount === 1 ? "row" : "rows"} · {formatDateNumeric(s.updatedAt.slice(0, 10))}
                </span>
              </button>
            ))}
          </aside>

          {active ? (
            // Remounted per sheet, so the selected cell, the filter and the
            // sort never travel from one sheet into another.
            <Grid key={active.id} sheet={active} onSheetChanged={() => void load()} />
          ) : (
            <p className="text-sm text-slate-500 dark:text-slate-400">Pick a sheet on the left.</p>
          )}
        </div>
      )}

      <NewSheetModal
        open={newOpen}
        onClose={() => setNewOpen(false)}
        onCreated={async (sheet) => {
          setNewOpen(false);
          await load();
          setActiveId(sheet.id);
        }}
      />
    </div>
  );
}

/* ------------------------------------------------------------------ *
 * Search
 * ------------------------------------------------------------------ */

function SearchResults({
  hits,
  query,
  onOpen,
}: {
  hits: WorkspaceHit[];
  query: string;
  onOpen: (sheetId: number) => void;
}) {
  if (hits.length === 0) {
    return (
      <EmptyState
        icon={<IconSearch className="h-8 w-8" />}
        title="Nothing found"
        description={`No sheet contains "${query.trim()}".`}
      />
    );
  }
  return (
    <div className="flex flex-col gap-2">
      {hits.map((h) => (
        <button
          key={`${h.id}-${h.whereFound}`}
          type="button"
          onClick={() => onOpen(h.id)}
          className="rounded-xl border border-slate-200 px-4 py-3 text-left transition hover:bg-surface-sunken dark:border-slate-700"
        >
          <div className="flex flex-wrap items-baseline gap-x-2">
            <span className="text-sm font-semibold text-slate-900 dark:text-slate-50">{h.title}</span>
            <span className="text-[11px] uppercase tracking-wide text-slate-400 dark:text-slate-500">
              {h.whereFound}
            </span>
          </div>
          {h.preview && <p className="mt-1 truncate text-xs text-slate-500 dark:text-slate-400">{h.preview}</p>}
        </button>
      ))}
    </div>
  );
}

/* ------------------------------------------------------------------ *
 * New sheet
 * ------------------------------------------------------------------ */

/** marko's own examples, turned into starting points. A sheet is his to
 *  reshape afterwards - nothing here is enforced. */
const TEMPLATES: { key: string; label: string; columns: string[] }[] = [
  { key: "blank", label: "Blank", columns: ["Column 1", "Column 2", "Column 3"] },
  { key: "codes", label: "Codes", columns: ["Date", "Buyer", "Nick", "Code", "Status", "Notes"] },
  { key: "accounts", label: "Accounts", columns: ["Platform", "Account", "Email", "Status", "Notes"] },
  { key: "plans", label: "Plans", columns: ["What", "By when", "Status", "Notes"] },
];

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

  async function create() {
    setSaving(true);
    try {
      onCreated(await api.createNoteSheet(name.trim() || chosen.label, chosen.columns));
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setSaving(false);
    }
  }

  return (
    <Modal open={open} onClose={onClose} title="New sheet" width="max-w-lg">
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
            aria-label="Sheet name"
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

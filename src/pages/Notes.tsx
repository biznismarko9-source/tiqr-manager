import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { api, errMsg } from "../lib/api";
import type { NoteSheet, WorkspaceHit, WorkspaceItem } from "../lib/types";
import { Button, Card, EmptyState, Input, PageHeader, Select, TableSkeleton } from "../components/ui";
import { IconClipboard, IconLayoutGrid, IconPlus, IconSearch } from "../components/icons";
import { useToast } from "../lib/toast";
import { formatDateNumeric } from "../lib/format";
import NoteEditor from "./notes/NoteEditor";
import NoteTable, { NewTableModal, NoTables } from "./notes/NoteTable";

/**
 * Notes - one clean place to write down and organize anything worth keeping.
 *
 * 2.53.0, rewritten to marko's second, simpler brief. Two things live here and
 * only two: a **note** (free writing) and a **table** (a small spreadsheet).
 * His words: "Keep it extremely simple", "Do NOT overengineer", and the test
 * he set for it - opening this page should make you think *"I can just write
 * something here."*
 *
 * So: no records with custom fields, no tasks with statuses, no categories, no
 * statistics, no dashboards. The 2.52.0 Workspace had all of that and it was
 * too much. What is stored has not changed (`workspace_items`, migration 032)
 * - a note is simply the only kind this screen writes, and anything a 2.52.0
 * row still carries is handed straight back on save rather than dropped.
 *
 * `Write -> Save -> Organize -> Search -> Find`.
 */

type Sort = "updated" | "newest" | "oldest" | "alpha";

const SORTS: { key: Sort; label: string }[] = [
  { key: "updated", label: "Recently updated" },
  { key: "newest", label: "Newest" },
  { key: "oldest", label: "Oldest" },
  { key: "alpha", label: "Alphabetical" },
];

const RECENT_COUNT = 5;

function noteTitle(n: WorkspaceItem): string {
  if (n.title.trim()) return n.title.trim();
  const firstLine = n.content.split("\n").find((l) => l.trim());
  return firstLine?.trim().slice(0, 80) || "Untitled";
}

/** The couple of lines shown under a title on the home screen. Markers are
 *  stripped so a note full of `#` and `[ ]` still reads as a sentence here. */
function snippet(n: WorkspaceItem): string {
  const body = n.title.trim() ? n.content : n.content.split("\n").slice(1).join(" ");
  return body
    .replace(/[#*]+/g, "")
    .replace(/\[( |x|X)\]|☐|☑/g, "")
    .replace(/\s+/g, " ")
    .trim()
    .slice(0, 150);
}

export default function Notes() {
  const toast = useToast();
  const [notes, setNotes] = useState<WorkspaceItem[] | null>(null);
  const [tables, setTables] = useState<NoteSheet[]>([]);
  const [query, setQuery] = useState("");
  const [hits, setHits] = useState<WorkspaceHit[] | null>(null);
  const [sort, setSort] = useState<Sort>("updated");
  const [tag, setTag] = useState("");
  const [showArchived, setShowArchived] = useState(false);
  const [newOpen, setNewOpen] = useState(false);
  const [newTableOpen, setNewTableOpen] = useState(false);
  const [openNote, setOpenNote] = useState<WorkspaceItem | null>(null);
  const [openTable, setOpenTable] = useState<number | null>(null);
  const newRef = useRef<HTMLDivElement>(null);

  const load = useCallback(async () => {
    try {
      const [items, sheets] = await Promise.all([api.listWorkspaceItems(true), api.listNoteSheets()]);
      setNotes(items);
      setTables(sheets);
    } catch (e) {
      toast.error(errMsg(e));
      setNotes([]);
    }
  }, [toast]);

  useEffect(() => {
    void load();
  }, [load]);

  // One search across note titles, note content, table names and table cells.
  // It runs a moment after typing stops, because it reads every row.
  useEffect(() => {
    const q = query.trim();
    if (!q) {
      setHits(null);
      return;
    }
    const t = setTimeout(() => {
      api
        .searchWorkspace(q)
        .then(setHits)
        .catch((e) => toast.error(errMsg(e)));
    }, 180);
    return () => clearTimeout(t);
  }, [query, toast]);

  // The "+ New" menu closes when you click anywhere else.
  useEffect(() => {
    if (!newOpen) return;
    const close = (e: MouseEvent) => {
      if (!newRef.current?.contains(e.target as Node)) setNewOpen(false);
    };
    document.addEventListener("mousedown", close);
    return () => document.removeEventListener("mousedown", close);
  }, [newOpen]);

  const live = useMemo(() => (notes ?? []).filter((n) => !n.archived), [notes]);
  const archived = useMemo(() => (notes ?? []).filter((n) => n.archived), [notes]);

  const allTags = useMemo(() => {
    const set = new Set<string>();
    live.forEach((n) => n.tags.forEach((t) => set.add(t)));
    return [...set].sort();
  }, [live]);

  const sorted = useMemo(() => {
    const list = live.filter((n) => !tag || n.tags.includes(tag));
    const by = [...list];
    if (sort === "updated") by.sort((a, b) => b.updatedAt.localeCompare(a.updatedAt));
    else if (sort === "newest") by.sort((a, b) => b.createdAt.localeCompare(a.createdAt));
    else if (sort === "oldest") by.sort((a, b) => a.createdAt.localeCompare(b.createdAt));
    else by.sort((a, b) => noteTitle(a).localeCompare(noteTitle(b), undefined, { sensitivity: "base" }));
    return by;
  }, [live, sort, tag]);

  const pinnedNotes = useMemo(() => sorted.filter((n) => n.pinned), [sorted]);
  const unpinnedNotes = useMemo(() => sorted.filter((n) => !n.pinned), [sorted]);
  const pinnedTables = useMemo(() => tables.filter((t) => t.pinned && !t.archived), [tables]);
  const liveTables = useMemo(() => tables.filter((t) => !t.archived), [tables]);

  async function createNote() {
    setNewOpen(false);
    try {
      const created = await api.saveWorkspaceItem({
        kind: "note",
        title: "",
        content: "",
        category: null,
        tags: [],
        fields: [],
        checklist: [],
        status: null,
        dueDate: null,
        pinned: false,
        archived: false,
      });
      setNotes((ns) => [created, ...(ns ?? [])]);
      setOpenNote(created);
    } catch (e) {
      toast.error(errMsg(e));
    }
  }

  async function togglePinNote(n: WorkspaceItem) {
    try {
      const saved = await api.setWorkspaceItemFlags(n.id, { pinned: !n.pinned });
      setNotes((ns) => ns?.map((x) => (x.id === saved.id ? saved : x)) ?? ns);
    } catch (e) {
      toast.error(errMsg(e));
    }
  }

  async function togglePinTable(t: NoteSheet) {
    try {
      await api.setNoteSheetFlags(t.id, { pinned: !t.pinned });
      setTables(await api.listNoteSheets());
    } catch (e) {
      toast.error(errMsg(e));
    }
  }

  async function setArchived(n: WorkspaceItem, archivedNext: boolean) {
    try {
      const saved = await api.setWorkspaceItemFlags(n.id, { archived: archivedNext });
      setNotes((ns) => ns?.map((x) => (x.id === saved.id ? saved : x)) ?? ns);
    } catch (e) {
      toast.error(errMsg(e));
    }
  }

  /* ---------------- one note, or one table, full screen ---------------- */

  if (openNote) {
    return (
      <div>
        <PageHeader title="Notes" />
        {/* `onChanged` updates the LIST only, never `openNote`. The editor
            flushes an unsaved change on its way out, so it fires once more
            after the back button has already closed the editor - writing the
            saved note back into `openNote` there would re-open it behind him.
            It is also why leaving does not reload: the list already holds
            what was saved. */}
        <NoteEditor
          note={openNote}
          onBack={() => setOpenNote(null)}
          onChanged={(saved) => setNotes((ns) => ns?.map((x) => (x.id === saved.id ? saved : x)) ?? ns)}
          onDeleted={(id) => {
            setNotes((ns) => ns?.filter((x) => x.id !== id) ?? ns);
            setOpenNote(null);
          }}
        />
      </div>
    );
  }

  if (openTable !== null) {
    return (
      <div>
        <PageHeader title="Notes" />
        <NoteTable
          sheetId={openTable}
          onBack={() => {
            setOpenTable(null);
            void load();
          }}
          onChanged={() => void load()}
        />
      </div>
    );
  }

  /* ----------------------------- the home ----------------------------- */

  return (
    <div>
      <PageHeader
        title="Notes"
        subtitle="Everything worth remembering, in one place."
        actions={
          <div className="flex flex-wrap items-center gap-2">
            <div className="relative w-full min-w-[220px] sm:w-72">
              <IconSearch className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-500 dark:text-slate-400" />
              <Input
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                placeholder="Search notes and tables…"
                aria-label="Search notes and tables"
                className="pl-9"
              />
            </div>
            <div className="relative" ref={newRef}>
              <Button variant="primary" onClick={() => setNewOpen((o) => !o)}>
                <IconPlus className="h-4 w-4" /> New
              </Button>
              {newOpen && (
                <div className="absolute right-0 top-full z-30 mt-1 w-40 overflow-hidden rounded-lg border border-slate-200 bg-surface shadow-lg dark:border-slate-700">
                  <button
                    type="button"
                    onClick={createNote}
                    className="flex w-full items-center gap-2 px-3 py-2 text-left text-sm text-slate-700 transition hover:bg-surface-sunken dark:text-slate-200"
                  >
                    <IconClipboard className="h-4 w-4" /> Note
                  </button>
                  <button
                    type="button"
                    onClick={() => {
                      setNewOpen(false);
                      setNewTableOpen(true);
                    }}
                    className="flex w-full items-center gap-2 px-3 py-2 text-left text-sm text-slate-700 transition hover:bg-surface-sunken dark:text-slate-200"
                  >
                    <IconLayoutGrid className="h-4 w-4" /> Table
                  </button>
                </div>
              )}
            </div>
          </div>
        }
      />

      {hits !== null ? (
        <SearchResults
          hits={hits}
          query={query}
          onOpenNote={(id) => {
            const n = (notes ?? []).find((x) => x.id === id);
            if (n) {
              setQuery("");
              setOpenNote(n);
            }
          }}
          onOpenTable={(id) => {
            setQuery("");
            setOpenTable(id);
          }}
        />
      ) : notes === null ? (
        <TableSkeleton />
      ) : live.length === 0 && liveTables.length === 0 ? (
        <EmptyState
          icon={<IconClipboard className="h-8 w-8" />}
          title="Nothing written yet"
          description="A note is for writing anything down. A table is for rows and columns you name yourself. Start with whichever fits."
          action={
            <div className="flex gap-2">
              <Button variant="primary" onClick={createNote}>
                <IconPlus className="h-4 w-4" /> New note
              </Button>
              <Button variant="secondary" onClick={() => setNewTableOpen(true)}>
                <IconPlus className="h-4 w-4" /> New table
              </Button>
            </div>
          }
        />
      ) : (
        <div className="flex flex-col gap-6">
          {allTags.length > 0 && (
            <div className="flex flex-wrap items-center gap-2">
              <button
                type="button"
                onClick={() => setTag("")}
                className={`rounded-full px-2.5 py-1 text-xs transition ${
                  tag === ""
                    ? "bg-brand-600 text-white"
                    : "bg-surface-sunken text-slate-600 hover:text-slate-900 dark:text-slate-400 dark:hover:text-slate-100"
                }`}
              >
                All
              </button>
              {allTags.map((t) => (
                <button
                  key={t}
                  type="button"
                  onClick={() => setTag((current) => (current === t ? "" : t))}
                  className={`rounded-full px-2.5 py-1 text-xs transition ${
                    tag === t
                      ? "bg-brand-600 text-white"
                      : "bg-surface-sunken text-slate-600 hover:text-slate-900 dark:text-slate-400 dark:hover:text-slate-100"
                  }`}
                >
                  #{t}
                </button>
              ))}
            </div>
          )}

          {(pinnedNotes.length > 0 || pinnedTables.length > 0) && (
            <Section title="Pinned">
              <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
                {pinnedTables.map((t) => (
                  <TableCard key={`t${t.id}`} table={t} onOpen={() => setOpenTable(t.id)} onPin={() => void togglePinTable(t)} />
                ))}
                {pinnedNotes.map((n) => (
                  <NoteCard
                    key={n.id}
                    note={n}
                    onOpen={() => setOpenNote(n)}
                    onPin={() => void togglePinNote(n)}
                    onArchive={() => void setArchived(n, true)}
                  />
                ))}
              </div>
            </Section>
          )}

          {/* Only worth its own heading when it is not simply the whole list
              again - with five notes or fewer, "Recent" IS "All notes". */}
          {unpinnedNotes.length > RECENT_COUNT && sort === "updated" && (
            <Section title="Recent">
              <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
                {unpinnedNotes.slice(0, RECENT_COUNT).map((n) => (
                  <NoteCard
                    key={n.id}
                    note={n}
                    onOpen={() => setOpenNote(n)}
                    onPin={() => void togglePinNote(n)}
                    onArchive={() => void setArchived(n, true)}
                  />
                ))}
              </div>
            </Section>
          )}

          <Section
            title="Tables"
            action={
              <Button variant="secondary" onClick={() => setNewTableOpen(true)}>
                <IconPlus className="h-4 w-4" /> New table
              </Button>
            }
          >
            {liveTables.length === 0 ? (
              <NoTables onNew={() => setNewTableOpen(true)} />
            ) : (
              <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
                {liveTables.map((t) => (
                  <TableCard key={t.id} table={t} onOpen={() => setOpenTable(t.id)} onPin={() => void togglePinTable(t)} />
                ))}
              </div>
            )}
          </Section>

          <Section
            title="All notes"
            action={
              <label className="flex items-center gap-2">
                <span className="text-xs text-slate-500 dark:text-slate-400">Sort</span>
                <span className="w-44">
                  <Select value={sort} onChange={(e) => setSort(e.target.value as Sort)}>
                    {SORTS.map((s) => (
                      <option key={s.key} value={s.key}>
                        {s.label}
                      </option>
                    ))}
                  </Select>
                </span>
              </label>
            }
          >
            {unpinnedNotes.length === 0 ? (
              <p className="text-sm text-slate-500 dark:text-slate-400">
                {tag ? `No note tagged #${tag}.` : "No notes yet — everything you have is pinned above."}
              </p>
            ) : (
              <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
                {unpinnedNotes.map((n) => (
                  <NoteCard
                    key={n.id}
                    note={n}
                    onOpen={() => setOpenNote(n)}
                    onPin={() => void togglePinNote(n)}
                    onArchive={() => void setArchived(n, true)}
                  />
                ))}
              </div>
            )}
          </Section>

          {/* Archiving is not in this brief, but 2.52.0 could archive a note.
              This keeps those reachable instead of invisible. */}
          {archived.length > 0 && (
            <div>
              <button
                type="button"
                onClick={() => setShowArchived((s) => !s)}
                className="text-xs text-slate-500 underline underline-offset-2 hover:text-slate-700 dark:text-slate-400 dark:hover:text-slate-200"
              >
                {showArchived ? "Hide" : "Show"} archived ({archived.length})
              </button>
              {showArchived && (
                <div className="mt-3 grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
                  {archived.map((n) => (
                    <NoteCard
                      key={n.id}
                      note={n}
                      onOpen={() => setOpenNote(n)}
                      onPin={() => void togglePinNote(n)}
                      onArchive={() => void setArchived(n, false)}
                      archived
                    />
                  ))}
                </div>
              )}
            </div>
          )}
        </div>
      )}

      <NewTableModal
        open={newTableOpen}
        onClose={() => setNewTableOpen(false)}
        onCreated={async (sheet) => {
          setNewTableOpen(false);
          setTables(await api.listNoteSheets());
          setOpenTable(sheet.id);
        }}
      />
    </div>
  );
}

/* ------------------------------------------------------------------ *
 * Pieces
 * ------------------------------------------------------------------ */

function Section({ title, action, children }: { title: string; action?: ReactNode; children: ReactNode }) {
  return (
    <section>
      <div className="mb-2 flex flex-wrap items-center justify-between gap-2">
        <h2 className="text-xs font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">{title}</h2>
        {action}
      </div>
      {children}
    </section>
  );
}

function PinButton({ pinned, onClick }: { pinned: boolean; onClick: () => void }) {
  return (
    <button
      type="button"
      onClick={(e) => {
        e.stopPropagation();
        onClick();
      }}
      title={pinned ? "Unpin" : "Pin to the top"}
      aria-label={pinned ? "Unpin" : "Pin to the top"}
      className={`shrink-0 text-sm leading-none transition ${
        pinned ? "text-amber-500 hover:text-amber-600" : "text-slate-300 hover:text-amber-500 dark:text-slate-600"
      }`}
    >
      {pinned ? "★" : "☆"}
    </button>
  );
}

function NoteCard({
  note,
  onOpen,
  onPin,
  onArchive,
  archived = false,
}: {
  note: WorkspaceItem;
  onOpen: () => void;
  onPin: () => void;
  onArchive: () => void;
  archived?: boolean;
}) {
  const text = snippet(note);
  return (
    <Card interactive onClick={onOpen} className="flex cursor-pointer flex-col gap-2">
      <div className="flex items-start gap-2">
        <h3 className="flex-1 truncate text-sm font-semibold text-slate-900 dark:text-slate-50">{noteTitle(note)}</h3>
        <PinButton pinned={note.pinned} onClick={onPin} />
      </div>
      {text && <p className="line-clamp-3 text-xs leading-relaxed text-slate-500 dark:text-slate-400">{text}</p>}
      <div className="mt-auto flex flex-wrap items-center gap-x-2 gap-y-1 text-[11px] text-slate-400 dark:text-slate-500">
        {note.dueDate && <span className="text-slate-500 dark:text-slate-400">{formatDateNumeric(note.dueDate)}</span>}
        {note.tags.map((t) => (
          <span key={t} className="rounded-full bg-surface-sunken px-1.5 py-0.5">
            #{t}
          </span>
        ))}
        <span className="ml-auto">{formatDateNumeric(note.updatedAt.slice(0, 10))}</span>
        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            onArchive();
          }}
          className="underline underline-offset-2 hover:text-slate-600 dark:hover:text-slate-300"
        >
          {archived ? "Restore" : "Archive"}
        </button>
      </div>
    </Card>
  );
}

function TableCard({ table, onOpen, onPin }: { table: NoteSheet; onOpen: () => void; onPin: () => void }) {
  return (
    <Card interactive onClick={onOpen} className="flex cursor-pointer flex-col gap-2">
      <div className="flex items-start gap-2">
        <IconLayoutGrid className="mt-0.5 h-4 w-4 shrink-0 text-slate-400 dark:text-slate-500" />
        <h3 className="flex-1 truncate text-sm font-semibold text-slate-900 dark:text-slate-50">{table.name}</h3>
        <PinButton pinned={table.pinned} onClick={onPin} />
      </div>
      <p className="text-xs text-slate-500 dark:text-slate-400">{table.columns.join(" · ")}</p>
      <div className="mt-auto flex items-center justify-between text-[11px] text-slate-400 dark:text-slate-500">
        <span>
          {table.rowCount} {table.rowCount === 1 ? "row" : "rows"}
        </span>
        <span>{formatDateNumeric(table.updatedAt.slice(0, 10))}</span>
      </div>
    </Card>
  );
}

function SearchResults({
  hits,
  query,
  onOpenNote,
  onOpenTable,
}: {
  hits: WorkspaceHit[];
  query: string;
  onOpenNote: (id: number) => void;
  onOpenTable: (id: number) => void;
}) {
  if (hits.length === 0) {
    return (
      <EmptyState
        icon={<IconSearch className="h-8 w-8" />}
        title="Nothing found"
        description={`No note or table contains "${query.trim()}".`}
      />
    );
  }
  const noteHits = hits.filter((h) => h.kind !== "table");
  const tableHits = hits.filter((h) => h.kind === "table");
  return (
    <div className="flex flex-col gap-6">
      {noteHits.length > 0 && (
        <Section title={`Notes (${noteHits.length})`}>
          <div className="flex flex-col gap-2">
            {noteHits.map((h) => (
              <HitRow key={`n${h.id}`} hit={h} onOpen={() => onOpenNote(h.id)} />
            ))}
          </div>
        </Section>
      )}
      {tableHits.length > 0 && (
        <Section title={`Tables (${tableHits.length})`}>
          <div className="flex flex-col gap-2">
            {tableHits.map((h) => (
              <HitRow key={`t${h.id}`} hit={h} onOpen={() => onOpenTable(h.id)} />
            ))}
          </div>
        </Section>
      )}
    </div>
  );
}

function HitRow({ hit, onOpen }: { hit: WorkspaceHit; onOpen: () => void }) {
  return (
    <Card interactive onClick={onOpen} className="cursor-pointer">
      <div className="flex flex-wrap items-baseline gap-x-2 gap-y-1">
        <span className="text-sm font-semibold text-slate-900 dark:text-slate-50">{hit.title || "Untitled"}</span>
        <span className="text-[11px] uppercase tracking-wide text-slate-400 dark:text-slate-500">{hit.whereFound}</span>
      </div>
      {hit.preview && <p className="mt-1 text-xs text-slate-500 dark:text-slate-400">{hit.preview}</p>}
    </Card>
  );
}

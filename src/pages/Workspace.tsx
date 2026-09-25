import { useCallback, useEffect, useMemo, useState } from "react";
import { api, errMsg } from "../lib/api";
import type { NoteSheet, WorkspaceHit, WorkspaceItem, WorkspaceKind } from "../lib/types";
import {
  Badge,
  Button,
  Card,
  ConfirmDialog,
  EmptyState,
  Input,
  PageHeader,
  Select,
  StatCard,
  TableSkeleton,
} from "../components/ui";
import { IconPlus, IconSearch, IconClipboard } from "../components/icons";
import { useToast } from "../lib/toast";
import { formatDateNumeric, todayIso } from "../lib/format";
import ItemEditor, { CATEGORIES } from "./workspace/ItemEditor";
import NoteTables from "./workspace/NoteTables";

/**
 * Workspace - the operational memory.
 *
 * 2.52.0, built from marko's written brief. Four content kinds live here:
 * notes, records and tasks (one row each in `workspace_items`) and tables
 * (the sheet engine from migration 031). The home screen answers the six
 * questions the brief asks it to answer at a glance - what exists, what
 * changed, what is pinned, what needs attention, where to create, how to find.
 *
 * `Capture -> Organize -> Find -> Use`. Deliberately NOT a Notion clone: no
 * nesting, no relations, no recurrence, no rich text.
 */

type View = "all" | "notes" | "records" | "tasks" | "tables" | "archived";

const VIEWS: { key: View; label: string }[] = [
  { key: "all", label: "All" },
  { key: "notes", label: "Notes" },
  { key: "records", label: "Records" },
  { key: "tasks", label: "Tasks" },
  { key: "tables", label: "Tables" },
  { key: "archived", label: "Archived" },
];

const KIND_LABEL: Record<string, string> = { note: "Note", record: "Record", task: "Task", table: "Table" };

function dueLabel(due: string): string {
  const today = todayIso();
  if (due === today) return "Today";
  const t = new Date(`${today}T00:00:00`);
  t.setDate(t.getDate() + 1);
  const tomorrow = `${t.getFullYear()}-${String(t.getMonth() + 1).padStart(2, "0")}-${String(t.getDate()).padStart(2, "0")}`;
  if (due === tomorrow) return "Tomorrow";
  return formatDateNumeric(due);
}

export default function Workspace() {
  const toast = useToast();
  const [items, setItems] = useState<WorkspaceItem[] | null>(null);
  const [sheets, setSheets] = useState<NoteSheet[]>([]);
  const [view, setView] = useState<View>("all");
  const [category, setCategory] = useState("");
  const [tag, setTag] = useState("");
  const [query, setQuery] = useState("");
  const [hits, setHits] = useState<WorkspaceHit[] | null>(null);
  const [editing, setEditing] = useState<WorkspaceItem | null>(null);
  const [editorOpen, setEditorOpen] = useState(false);
  const [editorKind, setEditorKind] = useState<WorkspaceKind>("note");
  const [quick, setQuick] = useState("");
  const [confirmDelete, setConfirmDelete] = useState<WorkspaceItem | null>(null);
  const [openTableId, setOpenTableId] = useState<number | null>(null);

  const load = useCallback(async () => {
    try {
      const [i, s] = await Promise.all([api.listWorkspaceItems(true), api.listNoteSheets()]);
      setItems(i);
      setSheets(s);
    } catch (e) {
      toast.error(errMsg(e));
      setItems([]);
    }
  }, [toast]);

  useEffect(() => {
    void load();
  }, [load]);

  // Search runs a moment after typing stops - it reads every item and cell.
  useEffect(() => {
    const q = query.trim();
    if (!q) {
      setHits(null);
      return;
    }
    const t = setTimeout(() => {
      api.searchWorkspace(q).then(setHits).catch((e) => toast.error(errMsg(e)));
    }, 180);
    return () => clearTimeout(t);
  }, [query, toast]);

  const live = useMemo(() => (items ?? []).filter((i) => !i.archived), [items]);

  const allTags = useMemo(() => {
    const set = new Set<string>();
    live.forEach((i) => i.tags.forEach((t) => set.add(t)));
    return [...set].sort();
  }, [live]);

  const upcoming = useMemo(
    () =>
      live
        .filter((i) => i.dueDate && i.status !== "done")
        .sort((a, b) => (a.dueDate ?? "").localeCompare(b.dueDate ?? ""))
        .slice(0, 6),
    [live],
  );

  const visible = useMemo(() => {
    let list = view === "archived" ? (items ?? []).filter((i) => i.archived) : live;
    if (view === "notes") list = list.filter((i) => i.kind === "note");
    if (view === "records") list = list.filter((i) => i.kind === "record");
    if (view === "tasks") list = list.filter((i) => i.kind === "task");
    if (view === "tables") list = [];
    if (category) list = list.filter((i) => i.category === category);
    if (tag) list = list.filter((i) => i.tags.includes(tag));
    return list;
  }, [items, live, view, category, tag]);

  const visibleSheets = useMemo(() => {
    if (view === "archived") return sheets.filter((s) => s.archived);
    if (view === "all" || view === "tables") return sheets.filter((s) => !s.archived);
    return [];
  }, [sheets, view]);

  async function quickSave() {
    const text = quick.trim();
    if (!text) return;
    try {
      // The brief's own example: one line, no category, no tags, saved
      // immediately. The first line becomes the title only when the note has
      // more than one - otherwise the whole thing is simply the content.
      const lines = text.split("\n");
      await api.saveWorkspaceItem({
        kind: "note",
        title: lines.length > 1 ? lines[0].slice(0, 80) : "",
        content: lines.length > 1 ? lines.slice(1).join("\n") : text,
        category: null,
        tags: [],
        fields: [],
        checklist: [],
        status: null,
        dueDate: null,
        pinned: false,
        archived: false,
      });
      setQuick("");
      toast.success("Saved");
      await load();
    } catch (e) {
      toast.error(errMsg(e));
    }
  }

  async function flags(item: WorkspaceItem, f: { pinned?: boolean; archived?: boolean; status?: string }) {
    try {
      await api.setWorkspaceItemFlags(item.id, f);
      await load();
    } catch (e) {
      toast.error(errMsg(e));
    }
  }

  if (openTableId !== null) {
    return (
      <NoteTables
        openSheetId={openTableId}
        onBack={() => {
          setOpenTableId(null);
          void load();
        }}
      />
    );
  }

  return (
    <div>
      <PageHeader
        title="Workspace"
        subtitle="Your operational notes, records and plans in one place."
        actions={
          <div className="flex flex-wrap items-center gap-2">
            <Button variant="secondary" onClick={() => { setEditing(null); setEditorKind("record"); setEditorOpen(true); }}>
              Record
            </Button>
            <Button variant="secondary" onClick={() => { setEditing(null); setEditorKind("task"); setEditorOpen(true); }}>
              Task
            </Button>
            <Button variant="secondary" onClick={() => setOpenTableId(0)}>
              Table
            </Button>
            <Button variant="primary" onClick={() => { setEditing(null); setEditorKind("note"); setEditorOpen(true); }}>
              <IconPlus className="h-4 w-4" /> New note
            </Button>
          </div>
        }
      />

      {/* Quick note: the brief's speed requirement. Type, press Enter, done -
          no category, no tags, no dialog. Everything can be edited later. */}
      <div className="mb-4 flex flex-wrap items-center gap-2">
        <Input
          value={quick}
          onChange={(e) => setQuick(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.shiftKey) {
              e.preventDefault();
              void quickSave();
            }
          }}
          placeholder="Quick note — type and press Enter…"
          className="min-w-[260px] flex-1"
          aria-label="Quick note"
        />
        <Button variant="secondary" onClick={quickSave} disabled={!quick.trim()}>
          Save
        </Button>
      </div>

      <div className="mb-4 w-full max-w-md">
        <div className="relative">
          <IconSearch className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-500 dark:text-slate-400" />
          <Input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search Workspace…"
            className="pl-9"
            aria-label="Search Workspace"
          />
        </div>
      </div>

      {hits !== null ? (
        <SearchResults
          hits={hits}
          query={query}
          onOpen={(hit) => {
            if (hit.kind === "table") {
              setQuery("");
              setOpenTableId(hit.id);
              return;
            }
            const found = (items ?? []).find((i) => i.id === hit.id);
            if (found) {
              setQuery("");
              setEditing(found);
              setEditorKind(found.kind);
              setEditorOpen(true);
            }
          }}
        />
      ) : items === null ? (
        <TableSkeleton />
      ) : (
        <>
          <div className="summary-bar mb-4">
            <StatCard label="Items" value={String(live.length + sheets.filter((s) => !s.archived).length)} />
            <StatCard label="Notes" value={String(live.filter((i) => i.kind === "note").length)} />
            <StatCard label="Tables" value={String(sheets.filter((s) => !s.archived).length)} />
            <StatCard
              label="Open tasks"
              value={String(live.filter((i) => i.kind === "task" && i.status !== "done").length)}
            />
            <StatCard label="Upcoming" value={String(upcoming.length)} />
          </div>

          {upcoming.length > 0 && (
            <Card className="mb-4 p-4">
              <p className="section-title mb-2">Upcoming</p>
              <ul className="flex flex-col gap-1.5">
                {upcoming.map((i) => (
                  <li key={i.id} className="flex flex-wrap items-baseline gap-2 text-sm">
                    <span className="w-20 shrink-0 text-xs font-medium text-slate-500 dark:text-slate-400">
                      {dueLabel(i.dueDate ?? "")}
                    </span>
                    <button
                      type="button"
                      onClick={() => { setEditing(i); setEditorKind(i.kind); setEditorOpen(true); }}
                      className="truncate text-left hover:underline"
                    >
                      {i.title || i.content.split("\n")[0] || "Untitled"}
                    </button>
                  </li>
                ))}
              </ul>
            </Card>
          )}

          <div className="mb-3 flex flex-wrap items-end gap-3">
            <div className="flex flex-wrap gap-1">
              {VIEWS.map((v) => (
                <button
                  key={v.key}
                  type="button"
                  onClick={() => setView(v.key)}
                  aria-pressed={view === v.key}
                  className={`rounded-lg px-3 py-1.5 text-xs font-medium transition ${
                    view === v.key
                      ? "bg-brand-600 text-white"
                      : "bg-surface-sunken text-slate-600 hover:text-slate-900 dark:text-slate-400 dark:hover:text-slate-100"
                  }`}
                >
                  {v.label}
                </button>
              ))}
            </div>
            <div className="w-40">
              <span className="label">Category</span>
              <Select value={category} onChange={(e) => setCategory(e.target.value)}>
                <option value="">All</option>
                {CATEGORIES.map((c) => (
                  <option key={c} value={c}>
                    {c}
                  </option>
                ))}
              </Select>
            </div>
            {allTags.length > 0 && (
              <div className="w-40">
                <span className="label">Tag</span>
                <Select value={tag} onChange={(e) => setTag(e.target.value)}>
                  <option value="">All</option>
                  {allTags.map((t) => (
                    <option key={t} value={t}>
                      #{t}
                    </option>
                  ))}
                </Select>
              </div>
            )}
          </div>

          {visible.length === 0 && visibleSheets.length === 0 ? (
            <EmptyState
              icon={<IconClipboard className="h-8 w-8" />}
              title={view === "archived" ? "Nothing archived" : "Your workspace is empty"}
              description={
                view === "archived"
                  ? "Items you put away land here, and can be restored from here."
                  : "Keep important notes, records, tables and plans here. The quick note box above is the fastest way in."
              }
              action={
                view === "archived" ? undefined : (
                  <Button variant="primary" onClick={() => { setEditing(null); setEditorKind("note"); setEditorOpen(true); }}>
                    <IconPlus className="h-4 w-4" /> Create your first item
                  </Button>
                )
              }
            />
          ) : (
            <div className="flex flex-col gap-2">
              {visibleSheets.map((s) => (
                <Card key={`sheet-${s.id}`} className="flex flex-wrap items-center gap-3 p-3">
                  <Badge tone="available">Table</Badge>
                  <button
                    type="button"
                    onClick={() => setOpenTableId(s.id)}
                    className="min-w-0 flex-1 text-left"
                  >
                    <span className="block truncate text-sm font-medium text-slate-900 dark:text-slate-50">
                      {s.pinned ? "★ " : ""}
                      {s.name}
                    </span>
                    <span className="text-xs text-slate-500 dark:text-slate-400">
                      {s.rowCount} {s.rowCount === 1 ? "row" : "rows"} · {formatDateNumeric(s.updatedAt.slice(0, 10))}
                      {s.description ? ` · ${s.description}` : ""}
                    </span>
                  </button>
                  <Button
                    variant="secondary"
                    onClick={async () => {
                      try {
                        await api.setNoteSheetFlags(s.id, { pinned: !s.pinned });
                        await load();
                      } catch (e) {
                        toast.error(errMsg(e));
                      }
                    }}
                  >
                    {s.pinned ? "Unpin" : "Pin"}
                  </Button>
                  <Button
                    variant="secondary"
                    onClick={async () => {
                      try {
                        await api.setNoteSheetFlags(s.id, { archived: !s.archived });
                        await load();
                      } catch (e) {
                        toast.error(errMsg(e));
                      }
                    }}
                  >
                    {s.archived ? "Restore" : "Archive"}
                  </Button>
                </Card>
              ))}

              {visible.map((i) => (
                <ItemCard
                  key={i.id}
                  item={i}
                  onOpen={() => { setEditing(i); setEditorKind(i.kind); setEditorOpen(true); }}
                  onFlags={(f) => void flags(i, f)}
                  onDuplicate={async () => {
                    try {
                      await api.duplicateWorkspaceItem(i.id);
                      toast.success("Duplicated");
                      await load();
                    } catch (e) {
                      toast.error(errMsg(e));
                    }
                  }}
                  onDelete={() => setConfirmDelete(i)}
                  onTag={(t) => setTag(t)}
                />
              ))}
            </div>
          )}
        </>
      )}

      <ItemEditor
        open={editorOpen}
        initial={editing}
        defaultKind={editorKind}
        onClose={() => setEditorOpen(false)}
        onSaved={async () => {
          setEditorOpen(false);
          await load();
        }}
      />

      <ConfirmDialog
        open={confirmDelete !== null}
        title="Delete this permanently?"
        message="Archiving keeps it and hides it. Deleting cannot be undone, here or on your other computer."
        confirmLabel="Delete"
        danger
        onCancel={() => setConfirmDelete(null)}
        onConfirm={async () => {
          if (!confirmDelete) return;
          try {
            await api.deleteWorkspaceItem(confirmDelete.id);
            setConfirmDelete(null);
            await load();
          } catch (e) {
            toast.error(errMsg(e));
          }
        }}
      />
    </div>
  );
}

function ItemCard({
  item,
  onOpen,
  onFlags,
  onDuplicate,
  onDelete,
  onTag,
}: {
  item: WorkspaceItem;
  onOpen: () => void;
  onFlags: (f: { pinned?: boolean; archived?: boolean; status?: string }) => void;
  onDuplicate: () => void;
  onDelete: () => void;
  onTag: (tag: string) => void;
}) {
  const done = item.kind === "task" && item.status === "done";
  const firstLine = item.title || item.content.split("\n")[0] || "Untitled";
  const rest = item.title ? item.content.split("\n")[0] : item.content.split("\n")[1] ?? "";
  const steps = item.checklist.length;
  const stepsDone = item.checklist.filter((c) => c.done).length;

  return (
    <Card className="p-3">
      <div className="flex flex-wrap items-start gap-3">
        {item.kind === "task" && (
          <input
            type="checkbox"
            checked={done}
            onChange={(e) => onFlags({ status: e.target.checked ? "done" : "open" })}
            aria-label={`Mark ${firstLine} done`}
            className="mt-1"
          />
        )}
        <button type="button" onClick={onOpen} className="min-w-0 flex-1 text-left">
          <span
            className={`block truncate text-sm font-medium ${
              done ? "text-slate-400 line-through dark:text-slate-500" : "text-slate-900 dark:text-slate-50"
            }`}
          >
            {item.pinned ? "★ " : ""}
            {firstLine}
          </span>
          {rest && <span className="block truncate text-xs text-slate-500 dark:text-slate-400">{rest}</span>}
          <span className="mt-1 flex flex-wrap items-center gap-2 text-[11px] text-slate-500 dark:text-slate-400">
            <span>{KIND_LABEL[item.kind]}</span>
            {item.category && <span>· {item.category}</span>}
            {item.dueDate && <span>· due {formatDateNumeric(item.dueDate)}</span>}
            {steps > 0 && (
              <span>
                · {stepsDone}/{steps} done
              </span>
            )}
            <span>· {formatDateNumeric(item.updatedAt.slice(0, 10))}</span>
          </span>
        </button>
        <div className="flex flex-wrap items-center gap-1">
          {item.tags.map((t) => (
            <button
              key={t}
              type="button"
              onClick={() => onTag(t)}
              className="rounded-md bg-surface-sunken px-1.5 py-0.5 text-[11px] text-slate-600 transition hover:text-slate-900 dark:text-slate-400 dark:hover:text-slate-100"
            >
              #{t}
            </button>
          ))}
        </div>
        <div className="flex flex-wrap gap-1">
          <Button variant="secondary" onClick={() => onFlags({ pinned: !item.pinned })}>
            {item.pinned ? "Unpin" : "Pin"}
          </Button>
          <Button variant="secondary" onClick={onDuplicate}>
            Duplicate
          </Button>
          <Button variant="secondary" onClick={() => onFlags({ archived: !item.archived })}>
            {item.archived ? "Restore" : "Archive"}
          </Button>
          <Button variant="secondary" onClick={onDelete}>
            Delete
          </Button>
        </div>
      </div>
    </Card>
  );
}

function SearchResults({
  hits,
  query,
  onOpen,
}: {
  hits: WorkspaceHit[];
  query: string;
  onOpen: (hit: WorkspaceHit) => void;
}) {
  // Grouped by kind, because the brief asks for exactly that - NOTES, TABLES,
  // RECORDS - rather than one undifferentiated list.
  const groups = useMemo(() => {
    const order = ["note", "record", "task", "table"];
    return order
      .map((k) => ({ kind: k, rows: hits.filter((h) => h.kind === k) }))
      .filter((g) => g.rows.length > 0);
  }, [hits]);

  if (hits.length === 0) {
    return (
      <EmptyState
        icon={<IconSearch className="h-8 w-8" />}
        title="Nothing found"
        description={`Nothing in your workspace contains "${query.trim()}". Archived items are not searched.`}
      />
    );
  }
  return (
    <div className="flex flex-col gap-4">
      {groups.map((g) => (
        <div key={g.kind}>
          <p className="section-title mb-1.5">{KIND_LABEL[g.kind] ?? g.kind}s</p>
          <div className="flex flex-col gap-1">
            {g.rows.map((h) => (
              <button
                key={`${h.kind}-${h.id}-${h.whereFound}`}
                type="button"
                onClick={() => onOpen(h)}
                className="card flex flex-wrap items-baseline gap-2 p-2.5 text-left transition hover:bg-surface-sunken"
              >
                <span className="text-sm font-medium text-slate-900 dark:text-slate-50">{h.title}</span>
                <span className="text-[11px] text-slate-500 dark:text-slate-400">{h.whereFound}</span>
                <span className="min-w-0 flex-1 truncate text-xs text-slate-600 dark:text-slate-300">{h.preview}</span>
              </button>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}

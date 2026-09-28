import { useCallback, useEffect, useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { api, errMsg } from "../lib/api";
import type { Note, NotepadHit } from "../lib/types";
import { Button, EmptyState, Input, TableSkeleton } from "../components/ui";
import { IconPlus, IconSearch } from "../components/icons";
import { useToast } from "../lib/toast";
import { formatDateNumeric } from "../lib/format";
import Editor from "./notes/Editor";
import { LINK_KINDS } from "./notes/LinkPicker";

/**
 * Notes — design 02 of the ten, which marko picked.
 *
 * Two panes: the list of notes on the left, the one you are in on the right.
 * The app's own rail is the third column and belongs to `Layout`, so this page
 * only draws two.
 *
 * Everything that makes a note a note lives in `notes/Editor.tsx`: blocks with
 * their own size and colour, checkboxes, images, sub-tabs along the bottom,
 * and what the note is attached to.
 *
 * ## Height
 *
 * `Layout` gives its pages no height, so a viewport-based one is what keeps
 * the two panes the same length and the sub-tab strip pinned to the bottom of
 * the editor rather than floating under the page.
 */

export default function Notes() {
  const toast = useToast();
  const [notes, setNotes] = useState<Note[] | null>(null);
  const [activeId, setActiveId] = useState<number | null>(null);
  const [query, setQuery] = useState("");
  const [hits, setHits] = useState<NotepadHit[] | null>(null);
  const [showArchived, setShowArchived] = useState(false);
  const [sheetCount, setSheetCount] = useState(0);

  const load = useCallback(async () => {
    try {
      const list = await api.listNotes(true);
      setNotes(list);
      setActiveId((cur) => {
        if (cur && list.some((n) => n.id === cur)) return cur;
        return list.find((n) => !n.archived)?.id ?? null;
      });
    } catch (e) {
      toast.error(errMsg(e));
      setNotes([]);
    }
  }, [toast]);

  useEffect(() => {
    void load();
  }, [load]);

  // Anything still in the old spreadsheet. Nothing was deleted when Notes
  // replaced it in the sidebar, so this is how he gets back to it.
  useEffect(() => {
    api
      .listNoteSheets()
      .then((s) => setSheetCount(s.length))
      .catch(() => setSheetCount(0));
  }, []);

  useEffect(() => {
    const q = query.trim();
    if (!q) {
      setHits(null);
      return;
    }
    const t = setTimeout(() => {
      api
        .searchNotepad(q)
        .then(setHits)
        .catch((e) => toast.error(errMsg(e)));
    }, 180);
    return () => clearTimeout(t);
  }, [query, toast]);

  const live = useMemo(() => (notes ?? []).filter((n) => !n.archived), [notes]);
  const archived = useMemo(() => (notes ?? []).filter((n) => n.archived), [notes]);
  const shown = showArchived ? archived : live;
  const active = useMemo(() => (notes ?? []).find((n) => n.id === activeId) ?? null, [notes, activeId]);

  async function newNote() {
    try {
      const created = await api.createNote("");
      setNotes((ns) => [created, ...(ns ?? [])]);
      setActiveId(created.id);
      setShowArchived(false);
    } catch (e) {
      toast.error(errMsg(e));
    }
  }

  async function togglePin(n: Note) {
    try {
      const saved = await api.setNoteFlags(n.id, { pinned: !n.pinned });
      setNotes((ns) => ns?.map((x) => (x.id === saved.id ? saved : x)) ?? ns);
    } catch (e) {
      toast.error(errMsg(e));
    }
  }

  return (
    <div
      className="flex min-h-[460px] gap-0 overflow-hidden rounded-xl border border-slate-200 dark:border-slate-800"
      style={{ height: "calc(100vh - 150px)" }}
    >
      {/* ── list ──────────────────────────────────────────────────────── */}
      <aside className="flex w-[300px] shrink-0 flex-col border-r border-slate-200 bg-surface-sunken dark:border-slate-800">
        <div className="flex items-center gap-2 px-4 pb-2 pt-3.5">
          <h1 className="text-[16px] font-semibold text-slate-900 dark:text-slate-50">Poznámky</h1>
          <button
            type="button"
            onClick={newNote}
            title="Nová poznámka"
            aria-label="Nová poznámka"
            className="ml-auto grid h-6 w-6 place-items-center rounded-md bg-brand-600 text-[15px] leading-none text-white transition hover:bg-brand-500"
          >
            +
          </button>
        </div>
        <div className="relative px-3 pb-2">
          <IconSearch className="pointer-events-none absolute left-5.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-slate-400 dark:text-slate-500" />
          <Input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Hľadať v poznámkach…"
            aria-label="Hľadať v poznámkach"
            className="!py-1.5 pl-8 text-[12.5px]"
          />
        </div>

        <div className="min-h-0 flex-1 overflow-y-auto px-2 pb-2">
          {notes === null ? (
            <div className="p-2">
              <TableSkeleton />
            </div>
          ) : hits !== null ? (
            hits.length === 0 ? (
              <p className="px-2 py-4 text-[12.5px] text-slate-500 dark:text-slate-400">Nič sa nenašlo.</p>
            ) : (
              hits.map((h) => (
                <button
                  key={`${h.pageId}`}
                  type="button"
                  onClick={() => {
                    setQuery("");
                    setActiveId(h.noteId);
                  }}
                  className="mb-0.5 block w-full rounded-lg px-3 py-2 text-left transition hover:bg-surface"
                >
                  <span className="block truncate text-[13.5px] font-semibold text-slate-900 dark:text-slate-50">
                    {h.noteTitle || "Bez názvu"}
                  </span>
                  <span className="block truncate text-[11.5px] text-slate-500 dark:text-slate-400">{h.excerpt}</span>
                  {h.pageName && (
                    <span className="mt-1 block text-[10.5px] uppercase tracking-wide text-slate-400 dark:text-slate-500">
                      {h.pageName}
                    </span>
                  )}
                </button>
              ))
            )
          ) : shown.length === 0 ? (
            <p className="px-2 py-4 text-[12.5px] text-slate-500 dark:text-slate-400">
              {showArchived ? "Archív je prázdny." : "Zatiaľ žiadne poznámky."}
            </p>
          ) : (
            shown.map((n) => (
              <div
                key={n.id}
                role="button"
                tabIndex={0}
                onClick={() => setActiveId(n.id)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") setActiveId(n.id);
                }}
                className={`relative mb-0.5 cursor-pointer rounded-lg px-3 py-2.5 transition ${
                  n.id === activeId
                    ? "bg-surface ring-1 ring-slate-200 dark:ring-slate-700"
                    : "hover:bg-surface/60"
                }`}
              >
                <span className="block truncate pr-5 text-[13.5px] font-semibold text-slate-900 dark:text-slate-50">
                  {n.title || "Bez názvu"}
                </span>
                {n.preview && (
                  <span className="mt-0.5 block truncate text-[12px] text-slate-500 dark:text-slate-400">
                    {n.preview}
                  </span>
                )}
                <span className="mt-1.5 flex flex-wrap items-center gap-x-2 gap-y-1 text-[10.5px] text-slate-400 dark:text-slate-500">
                  <span>{formatDateNumeric(n.updatedAt.slice(0, 10))}</span>
                  {n.pageCount > 1 && <span>{n.pageCount} podkariet</span>}
                  {n.imageCount > 0 && <span>▣ {n.imageCount}</span>}
                  {n.links.map((l) => (
                    <span key={l.id} className="text-amber-600 dark:text-amber-400">
                      {LINK_KINDS.find((k) => k.kind === l.kind)?.label ?? l.kind}
                    </span>
                  ))}
                </span>
                <button
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation();
                    void togglePin(n);
                  }}
                  title={n.pinned ? "Odopnúť" : "Pripnúť"}
                  aria-label={n.pinned ? "Odopnúť" : "Pripnúť"}
                  className={`absolute right-2.5 top-2.5 text-[11px] transition ${
                    n.pinned ? "text-amber-500" : "text-slate-300 hover:text-amber-500 dark:text-slate-700"
                  }`}
                >
                  ★
                </button>
              </div>
            ))
          )}
        </div>

        <div className="flex items-center gap-2 border-t border-slate-200 px-3 py-1.5 text-[11px] dark:border-slate-800">
          {archived.length > 0 && (
            <button
              type="button"
              onClick={() => setShowArchived((s) => !s)}
              className="text-slate-500 underline underline-offset-2 transition hover:text-slate-800 dark:text-slate-400 dark:hover:text-slate-100"
            >
              {showArchived ? "Späť na poznámky" : `Archív (${archived.length})`}
            </button>
          )}
          {sheetCount > 0 && (
            <Link
              to="/sheets"
              className="ml-auto text-slate-400 underline underline-offset-2 transition hover:text-slate-700 dark:hover:text-slate-200"
            >
              Staré hárky ({sheetCount})
            </Link>
          )}
        </div>
      </aside>

      {/* ── editor ───────────────────────────────────────────────────── */}
      <section className="min-w-0 flex-1 bg-surface">
        {notes === null ? (
          <div className="p-5">
            <TableSkeleton />
          </div>
        ) : active ? (
          <Editor
            key={active.id}
            note={active}
            onNoteChanged={() => void load()}
            onDeleted={async () => {
              if (!window.confirm(`Zmazať poznámku „${active.title || "Bez názvu"}"? Aj s podkartami a obrázkami.`))
                return;
              try {
                await api.deleteNote(active.id);
                setActiveId(null);
                await load();
              } catch (e) {
                toast.error(errMsg(e));
              }
            }}
          />
        ) : (
          <div className="grid h-full place-items-center p-6">
            <EmptyState
              icon={<IconPlus className="h-8 w-8" />}
              title="Žiadna poznámka"
              description="Poznámka je prázdny papier — píš čo chceš, kde chceš. Veľkosť, farbu a obrázok si vyberáš pre každý riadok zvlášť, a dole si vieš pridať podkarty."
              action={
                <Button variant="primary" onClick={newNote}>
                  <IconPlus className="h-4 w-4" /> Nová poznámka
                </Button>
              }
            />
          </div>
        )}
      </section>
    </div>
  );
}

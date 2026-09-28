import { useCallback, useEffect, useMemo, useState } from "react";
import { api, errMsg } from "../lib/api";
import type { Note, NotepadHit } from "../lib/types";
import { Button, ConfirmDialog, EmptyState, Input, Modal, ModalFooter, TableSkeleton } from "../components/ui";
import { IconPlus, IconSearch } from "../components/icons";
import { useToast } from "../lib/toast";
import { formatDateNumeric } from "../lib/format";
import Editor from "./notes/Editor";
import { LINK_KINDS } from "./notes/LinkPicker";
import Menu, { menuAt } from "./notes/Menu";
import type { MenuAt } from "./notes/Menu";
import DuplicateDialog from "./notes/DuplicateDialog";

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
 * ## 2.62.0
 *
 * The list is where a note is MANAGED: it is dragged into place, duplicated,
 * pinned, archived and deleted from the "⋯" menu here, so the editor toolbar
 * is only about what is inside the note. Deleting used to sit in that toolbar,
 * one click away from the formatting buttons; it is now behind the menu and
 * behind the same confirm as before.
 *
 * ## Height
 *
 * `Layout` gives its pages no height, so a viewport-based one is what keeps
 * the two panes the same length and the sub-tab strip pinned to the bottom of
 * the editor rather than floating under the page.
 */

/** A template lays out the SUB-TABS of a new note and nothing else. It writes
 *  no text, invents no values and fills in no fields: an empty tab named
 *  "Platba" is a place to write, not a claim that anything was paid. */
const TEMPLATES: { key: string; label: string; pages: string[] }[] = [
  { key: "blank", label: "Prázdna", pages: [] },
  { key: "event", label: "Event", pages: ["Event", "Lístky", "Nákup", "Prevod", "Predaj", "Poznámky"] },
  {
    key: "order",
    label: "Objednávka",
    pages: ["Objednávka", "Platforma", "Číslo objednávky", "Lístky", "Platba", "Poznámky"],
  },
  { key: "pull", label: "Pull", pages: ["Pull", "Event", "Lístky", "Platba", "Prevod", "Poznámky"] },
  { key: "sale", label: "Predaj", pages: ["Predaj", "Kupec", "Cena", "Prevod", "Poznámky"] },
];

export default function Notes() {
  const toast = useToast();
  const [notes, setNotes] = useState<Note[] | null>(null);
  const [activeId, setActiveId] = useState<number | null>(null);
  const [query, setQuery] = useState("");
  const [hits, setHits] = useState<NotepadHit[] | null>(null);
  const [showArchived, setShowArchived] = useState(false);
  const [menu, setMenu] = useState<null | { at: MenuAt; id: number }>(null);
  const [dup, setDup] = useState<Note | null>(null);
  const [remove, setRemove] = useState<Note | null>(null);
  const [tplOpen, setTplOpen] = useState(false);
  const [tpl, setTpl] = useState("blank");

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
  const menuNote = useMemo(() => (notes ?? []).find((n) => n.id === menu?.id) ?? null, [notes, menu]);

  /** The chosen template only creates the sub-tabs, one call each. There is no
   *  "create a note with these tabs" command and 2.62.0 adds no migration, so
   *  this is the whole of it: create, name the tab that always exists, add the
   *  rest. */
  async function newNote(pages: string[]) {
    try {
      const created = await api.createNote("");
      if (pages.length > 0) {
        const first = await api.listNotePages(created.id);
        if (first[0]) await api.renameNotePage(first[0].id, pages[0]!);
        for (const name of pages.slice(1)) await api.createNotePage(created.id, name);
      }
      setShowArchived(false);
      setActiveId(created.id);
      await load();
    } catch (e) {
      toast.error(errMsg(e));
    }
  }

  async function duplicateNote(n: Note, withLinks: boolean) {
    setDup(null);
    try {
      const copy = await api.duplicateNote(n.id, withLinks);
      setShowArchived(false);
      setActiveId(copy.id);
      await load();
      toast.success(withLinks ? "Poznámka aj s priradeniami" : "Poznámka bez priradení");
    } catch (e) {
      toast.error(errMsg(e));
    }
  }

  async function deleteNote(n: Note) {
    setRemove(null);
    try {
      await api.deleteNote(n.id);
      if (activeId === n.id) setActiveId(null);
      await load();
    } catch (e) {
      toast.error(errMsg(e));
    }
  }

  /** Writes the whole order back - see `renumber` in commands/notepad.rs for
   *  why the whole set is renumbered rather than two rows. */
  async function reorder(next: Note[]) {
    setNotes((ns) => {
      if (!ns) return ns;
      const moved = new Map(next.map((x, k) => [x.id, k]));
      return [...ns].sort((a, b) => (moved.get(a.id) ?? 0) - (moved.get(b.id) ?? 0));
    });
    try {
      await api.reorderNotes(next.map((x) => x.id));
      await load();
    } catch (e) {
      toast.error(errMsg(e));
      await load();
    }
  }

  /** marko: "mala by byt moznost hybat s poznamkamy". */
  async function moveNote(n: Note, delta: -1 | 1) {
    const list = shown;
    const i = list.findIndex((x) => x.id === n.id);
    const to = i + delta;
    if (i < 0 || to < 0 || to >= list.length) return;
    const next = [...list];
    const [m] = next.splice(i, 1);
    next.splice(to, 0, m!);
    await reorder(next);
  }

  /** Dropped ON a note: the dragged one takes that place. */
  async function dropNote(fromId: number, onto: Note) {
    const list = shown;
    const from = list.findIndex((x) => x.id === fromId);
    const to = list.findIndex((x) => x.id === onto.id);
    if (from < 0 || to < 0 || from === to) return;
    const next = [...list];
    const [m] = next.splice(from, 1);
    next.splice(to, 0, m!);
    await reorder(next);
  }

  async function setFlags(n: Note, flags: { pinned?: boolean; archived?: boolean }) {
    try {
      const saved = await api.setNoteFlags(n.id, flags);
      setNotes((ns) => ns?.map((x) => (x.id === saved.id ? saved : x)) ?? ns);
      if (saved.archived && saved.id === activeId) setActiveId(null);
      await load();
    } catch (e) {
      toast.error(errMsg(e));
    }
  }

  const act =
    "rounded px-1 text-[11px] leading-none text-transparent transition group-hover:text-slate-300 hover:!text-slate-600 dark:group-hover:text-slate-600 dark:hover:!text-slate-200";

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
            onClick={() => {
              setTpl("blank");
              setTplOpen(true);
            }}
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
                draggable
                onDragStart={(e) => {
                  e.dataTransfer.setData("text/plain", `note:${n.id}`);
                  e.dataTransfer.effectAllowed = "move";
                }}
                onDragOver={(e) => e.preventDefault()}
                onDrop={(e) => {
                  e.preventDefault();
                  const raw = e.dataTransfer.getData("text/plain");
                  if (!raw.startsWith("note:")) return;
                  void dropNote(Number(raw.slice(5)), n);
                }}
                onClick={() => setActiveId(n.id)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") setActiveId(n.id);
                }}
                className={`group relative mb-0.5 cursor-pointer rounded-lg px-3 py-2.5 transition ${
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
                <span className="absolute right-2 top-2 flex items-center gap-0.5">
                  <button
                    type="button"
                    onClick={(e) => {
                      e.stopPropagation();
                      void moveNote(n, -1);
                    }}
                    title="Posunúť hore"
                    aria-label="Posunúť hore"
                    className={act}
                  >
                    ↑
                  </button>
                  <button
                    type="button"
                    onClick={(e) => {
                      e.stopPropagation();
                      void moveNote(n, 1);
                    }}
                    title="Posunúť dole"
                    aria-label="Posunúť dole"
                    className={act}
                  >
                    ↓
                  </button>
                  <button
                    type="button"
                    onClick={(e) => {
                      e.stopPropagation();
                      void setFlags(n, { pinned: !n.pinned });
                    }}
                    title={n.pinned ? "Odopnúť" : "Pripnúť"}
                    aria-label={n.pinned ? "Odopnúť" : "Pripnúť"}
                    className={`text-[11px] transition ${
                      n.pinned ? "text-amber-500" : "text-slate-300 hover:text-amber-500 dark:text-slate-700"
                    }`}
                  >
                    ★
                  </button>
                  <button
                    type="button"
                    onClick={(e) => {
                      e.stopPropagation();
                      setMenu({ at: menuAt(e), id: n.id });
                    }}
                    title="Viac"
                    aria-label="Viac"
                    className={act}
                  >
                    ⋯
                  </button>
                </span>
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
        </div>
      </aside>

      {/* ── editor ───────────────────────────────────────────────────── */}
      <section className="min-w-0 flex-1 bg-surface">
        {notes === null ? (
          <div className="p-5">
            <TableSkeleton />
          </div>
        ) : active ? (
          <Editor key={active.id} note={active} onNoteChanged={() => void load()} />
        ) : (
          <div className="grid h-full place-items-center p-6">
            <EmptyState
              icon={<IconPlus className="h-8 w-8" />}
              title="Žiadna poznámka"
              description="Poznámka je prázdny papier — píš čo chceš, kde chceš. Veľkosť, farbu a obrázok si vyberáš pre každý riadok zvlášť, a dole si vieš pridať podkarty."
              action={
                <Button
                  variant="primary"
                  onClick={() => {
                    setTpl("blank");
                    setTplOpen(true);
                  }}
                >
                  <IconPlus className="h-4 w-4" /> Nová poznámka
                </Button>
              }
            />
          </div>
        )}
      </section>

      <Menu
        at={menu?.at ?? null}
        onClose={() => setMenu(null)}
        items={
          menuNote
            ? [
                { label: "Duplikovať…", onClick: () => setDup(menuNote) },
                "-",
                { label: menuNote.pinned ? "Odopnúť" : "Pripnúť", onClick: () => void setFlags(menuNote, { pinned: !menuNote.pinned }) },
                {
                  label: menuNote.archived ? "Vrátiť z archívu" : "Archivovať",
                  onClick: () => void setFlags(menuNote, { archived: !menuNote.archived }),
                },
                "-",
                { label: "Zmazať poznámku", danger: true, onClick: () => setRemove(menuNote) },
              ]
            : []
        }
      />

      <DuplicateDialog
        open={dup !== null}
        title="Duplikovať poznámku"
        intro="Skopíruje sa celá poznámka — názov, štítok, dátum, všetky podkarty s ich obsahom aj poradím. Priradenia len ak si ich vypýtaš."
        contentLabel="Kópia nebude priradená k ničomu."
        linksLabel="Každá podkarta kópie sa priradí k tomu istému, čo originál."
        onCancel={() => setDup(null)}
        onConfirm={(withLinks) => dup && void duplicateNote(dup, withLinks)}
      />

      <Modal open={tplOpen} onClose={() => setTplOpen(false)} title="Nová poznámka" width="max-w-md">
        <p className="mb-3 text-[12.5px] leading-relaxed text-slate-500 dark:text-slate-400">
          Šablóna iba pripraví podkarty — nič do nich nenapíše a nič si nevymyslí.
        </p>
        <div className="flex flex-col gap-2">
          {TEMPLATES.map((t) => (
            <button
              key={t.key}
              type="button"
              onClick={() => setTpl(t.key)}
              className={`flex w-full items-start gap-2.5 rounded-lg px-3 py-2.5 text-left ring-1 transition ${
                tpl === t.key
                  ? "bg-brand-50 ring-brand-400 dark:bg-brand-500/10 dark:ring-brand-500"
                  : "ring-slate-200 hover:bg-surface-sunken dark:ring-slate-700"
              }`}
            >
              <span
                className={`mt-1 h-3.5 w-3.5 shrink-0 rounded-full ${
                  tpl === t.key ? "border-[4px] border-brand-600" : "border-[1.5px] border-slate-400 dark:border-slate-500"
                }`}
              />
              <span className="min-w-0">
                <b className="block text-[13px] font-semibold text-slate-900 dark:text-slate-50">{t.label}</b>
                <span className="block text-[11.5px] text-slate-500 dark:text-slate-400">
                  {t.pages.length === 0 ? "Jedna prázdna podkarta." : t.pages.join(" · ")}
                </span>
              </span>
            </button>
          ))}
        </div>
        <ModalFooter>
          <Button variant="secondary" onClick={() => setTplOpen(false)}>
            Zrušiť
          </Button>
          <Button
            variant="primary"
            onClick={() => {
              setTplOpen(false);
              void newNote(TEMPLATES.find((t) => t.key === tpl)?.pages ?? []);
            }}
          >
            Vytvoriť
          </Button>
        </ModalFooter>
      </Modal>

      <ConfirmDialog
        open={remove !== null}
        title={`Zmazať poznámku „${remove?.title || "Bez názvu"}"?`}
        message="Zmaže sa aj so všetkými podkartami, obrázkami a priradeniami, na tomto aj na druhom počítači. Toto sa nedá vrátiť."
        confirmLabel="Áno, zmazať"
        danger
        onCancel={() => setRemove(null)}
        onConfirm={() => remove && void deleteNote(remove)}
      />
    </div>
  );
}

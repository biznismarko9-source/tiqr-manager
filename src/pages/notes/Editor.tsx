import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { api, errMsg } from "../../lib/api";
import type { Note, NoteBlock, NoteColour, NoteImage, NoteLink, NotePage, NoteTextSize } from "../../lib/types";
import { Button, ConfirmDialog, Input, Modal, ModalFooter } from "../../components/ui";
import { IconPlus, IconX } from "../../components/icons";
import { useToast } from "../../lib/toast";
import LinkPicker, { LINK_KINDS } from "./LinkPicker";
import Menu, { menuAt } from "./Menu";
import type { MenuAt } from "./Menu";
import DuplicateDialog from "./DuplicateDialog";

/**
 * One note: toolbar, body, and its sub-tabs along the bottom.
 *
 * 2.61.0 was marko's list of what was wrong with 2.58.0 — links per sub-tab,
 * an in-app name box, blocks and tabs that move, a removable tag and date,
 * a confirm before deleting, undo, and Enter that takes the cursor with it.
 *
 * 2.62.0 is the working-with-it pass:
 *
 * · a chip is a link — clicking it opens what the tab is attached to
 * · a sub-tab, or a whole note, can be duplicated (with or without its links)
 * · **drag** — blocks, sub-tabs and notes all move by hand, not only by ↑↓
 * · Ctrl+F finds inside the note and puts the cursor on the hit
 * · an image opens full size and carries a caption of its own
 * · bullets and numbering
 * · a checklist says how much of it is done
 * · half-width blocks stand side by side, and the writing column has a width
 *   slider — marko: *"2 riadky vedla seba"*, *"velkost toho kde sa pise"*
 *
 * ## Undo is per tab and lives only as long as the tab is open
 *
 * A stack of previous `blocks` snapshots. That is deliberately not a full
 * history: it covers the case he named — something got broken or written
 * wrong just now — without pretending the app has version control. Switching
 * tab or note clears it, because an undo that reaches back into a different
 * tab would be worse than none.
 *
 * ## The numbering is not stored
 *
 * A `num` block knows it is numbered, not which number it is. The digit comes
 * from the run of `num` blocks above it at render time, so inserting, moving
 * or deleting one renumbers the rest by itself and no two lines can ever
 * disagree about who is third.
 */

const SAVE_DEBOUNCE_MS = 700;
const UNDO_DEPTH = 60;
const MAX_IMAGE_EDGE = 1400;
const JPEG_QUALITY = 0.72;

/** The writing column's width. A view preference of THIS machine, so it lives
 *  in localStorage and not in the note: it is not something that should sync
 *  across to the other computer, and it needs no migration. */
const DOC_WIDTH_KEY = "tiqr.notes.docWidth";
const DOC_WIDTH_MIN = 520;
const DOC_WIDTH_MAX = 1180;
const DOC_WIDTH_DEFAULT = 760;

function readDocWidth(): number {
  try {
    const raw = window.localStorage.getItem(DOC_WIDTH_KEY);
    const n = raw === null ? NaN : Number(raw);
    if (!Number.isFinite(n)) return DOC_WIDTH_DEFAULT;
    return Math.min(DOC_WIDTH_MAX, Math.max(DOC_WIDTH_MIN, Math.round(n)));
  } catch {
    return DOC_WIDTH_DEFAULT; // private mode, or storage turned off
  }
}

function writeDocWidth(n: number) {
  try {
    window.localStorage.setItem(DOC_WIDTH_KEY, String(n));
  } catch {
    /* nothing to do: the slider still works for this session */
  }
}

const SIZE_CLASS: Record<NoteTextSize, string> = {
  h1: "text-[22px] font-bold leading-tight",
  h2: "text-[17px] font-semibold leading-snug",
  p: "text-[15px] leading-relaxed",
  small: "text-[13px] leading-relaxed",
};

const COLOUR_CLASS: Record<NoteColour, string> = {
  r: "text-red-600 dark:text-red-400",
  o: "text-orange-600 dark:text-orange-400",
  g: "text-emerald-600 dark:text-emerald-400",
  b: "text-blue-600 dark:text-blue-400",
  p: "text-fuchsia-600 dark:text-fuchsia-400",
  m: "text-slate-500 dark:text-slate-400",
};

const COLOURS: { flag: NoteColour | ""; dot: string; label: string }[] = [
  { flag: "", dot: "bg-slate-400", label: "Základná" },
  { flag: "r", dot: "bg-red-500", label: "Červená" },
  { flag: "o", dot: "bg-orange-500", label: "Oranžová" },
  { flag: "g", dot: "bg-emerald-500", label: "Zelená" },
  { flag: "b", dot: "bg-blue-500", label: "Modrá" },
  { flag: "p", dot: "bg-fuchsia-500", label: "Ružová" },
  { flag: "m", dot: "bg-slate-500", label: "Sivá" },
];

/** Where each kind of chip lands, said in words. Three of the six have no
 *  detail page in TIQR, so they open the nearest thing that exists rather
 *  than a page invented for them. */
const LINK_DEST: Record<string, string> = {
  order: "otvorí objednávku",
  event: "otvorí event",
  sale: "otvorí predaj",
  ticket: "otvorí objednávku, v ktorej lístok je",
  pull: "otvorí zoznam pullov",
  finance: "otvorí Financie",
};

/** True for anything that is a written line: text, bullet, numbered, check. */
function isWritten(b: NoteBlock | undefined): boolean {
  return !!b && b.k !== "image" && b.k !== "rule";
}

/** Shrinks and re-encodes on a canvas, so nothing new is installed for it. */
function shrinkImage(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onerror = () => reject(new Error("Súbor sa nepodarilo prečítať."));
    reader.onload = () => {
      const img = new Image();
      img.onerror = () => reject(new Error("Toto nie je obrázok, ktorý viem otvoriť."));
      img.onload = () => {
        const scale = Math.min(1, MAX_IMAGE_EDGE / Math.max(img.width, img.height));
        const w = Math.max(1, Math.round(img.width * scale));
        const h = Math.max(1, Math.round(img.height * scale));
        const canvas = document.createElement("canvas");
        canvas.width = w;
        canvas.height = h;
        const ctx = canvas.getContext("2d");
        if (!ctx) {
          reject(new Error("Obrázok sa nepodarilo spracovať."));
          return;
        }
        // A white ground: a transparent PNG turns black on JPEG otherwise.
        ctx.fillStyle = "#ffffff";
        ctx.fillRect(0, 0, w, h);
        ctx.drawImage(img, 0, 0, w, h);
        resolve(canvas.toDataURL("image/jpeg", JPEG_QUALITY));
      };
      img.src = String(reader.result);
    };
    reader.readAsDataURL(file);
  });
}

/** One match of the find bar. `key` names the field it is in: `title`, `bN`
 *  for block N's text, `cN` for the caption under block N's image. */
type FindHit = { key: string; start: number; end: number };

export default function Editor({ note, onNoteChanged }: { note: Note; onNoteChanged: () => void }) {
  const toast = useToast();
  const navigate = useNavigate();
  const [pages, setPages] = useState<NotePage[] | null>(null);
  const [pageId, setPageId] = useState<number | null>(null);
  const [blocks, setBlocks] = useState<NoteBlock[]>([]);
  const [undoStack, setUndoStack] = useState<NoteBlock[][]>([]);
  const [sel, setSel] = useState(0);
  const [images, setImages] = useState<NoteImage[]>([]);
  const [links, setLinks] = useState<NoteLink[]>([]);
  const [title, setTitle] = useState(note.title);
  const [tag, setTag] = useState(note.tag);
  const [date, setDate] = useState(note.noteDate ?? "");
  const [dirty, setDirty] = useState(false);
  const [linkOpen, setLinkOpen] = useState(false);
  const [namePrompt, setNamePrompt] = useState<null | { mode: "new" | "rename"; id?: number; value: string }>(null);
  const [confirm, setConfirm] = useState<null | { what: "tab" | "image"; id: number; label: string }>(null);
  const [docWidth, setDocWidth] = useState(readDocWidth);
  const [find, setFind] = useState<string | null>(null);
  const [findAt, setFindAt] = useState(0);
  const [lightbox, setLightbox] = useState<number | null>(null);
  const [tabMenu, setTabMenu] = useState<null | { at: MenuAt; id: number }>(null);
  const [dupTab, setDupTab] = useState<NotePage | null>(null);
  const fileRef = useRef<HTMLInputElement>(null);
  const findRef = useRef<HTMLInputElement>(null);
  /** Every editable field on the page, so the find bar can put the cursor on
   *  a hit. Textareas and inputs both answer `setSelectionRange`. */
  const fieldRefs = useRef<Record<string, HTMLTextAreaElement | HTMLInputElement | null>>({});

  useEffect(() => {
    setTitle(note.title);
    setTag(note.tag);
    setDate(note.noteDate ?? "");
  }, [note.id, note.title, note.tag, note.noteDate]);

  const loadPage = useCallback(
    async (id: number, all: NotePage[]) => {
      const p = all.find((x) => x.id === id) ?? null;
      setPageId(id);
      setBlocks(p?.blocks ?? []);
      setSel(0);
      setDirty(false);
      setUndoStack([]); // a tab's history does not reach into another tab
      try {
        setLinks(await api.listNotePageLinks(id));
      } catch {
        setLinks([]);
      }
    },
    [],
  );

  useEffect(() => {
    let alive = true;
    setPages(null);
    Promise.all([api.listNotePages(note.id), api.listNoteImages(note.id)])
      .then(async ([p, im]) => {
        if (!alive) return;
        setPages(p);
        setImages(im);
        if (p[0]) await loadPage(p[0].id, p);
      })
      .catch((e) => {
        if (alive) toast.error(errMsg(e));
      });
    return () => {
      alive = false;
    };
  }, [note.id, toast, loadPage]);

  /* -------------------------------- saving ------------------------------ */

  const draft = useRef({ pageId, blocks });
  draft.current = { pageId, blocks };

  const savePage = useCallback(async () => {
    const d = draft.current;
    if (d.pageId === null) return;
    try {
      const saved = await api.saveNotePage(d.pageId, d.blocks);
      setPages((ps) => ps?.map((p) => (p.id === saved.id ? saved : p)) ?? ps);
      setDirty(false);
      onNoteChanged();
    } catch (e) {
      toast.error(errMsg(e));
    }
  }, [onNoteChanged, toast]);

  const saveRef = useRef(savePage);
  saveRef.current = savePage;
  const dirtyRef = useRef(dirty);
  dirtyRef.current = dirty;

  useEffect(() => {
    if (!dirty) return;
    const t = setTimeout(() => void saveRef.current(), SAVE_DEBOUNCE_MS);
    return () => clearTimeout(t);
  }, [dirty]);

  useEffect(
    () => () => {
      if (dirtyRef.current) void saveRef.current();
    },
    [],
  );

  async function saveHeader(nextTag = tag, nextDate = date) {
    try {
      await api.updateNote(note.id, title, nextTag, nextDate.trim() || null);
      onNoteChanged();
    } catch (e) {
      toast.error(errMsg(e));
    }
  }

  /* -------------------------------- blocks ------------------------------ */

  /** Every change goes through here, so every change is undoable. */
  function edit(fn: (bs: NoteBlock[]) => NoteBlock[]) {
    setBlocks((bs) => {
      setUndoStack((st) => [...st.slice(-(UNDO_DEPTH - 1)), bs]);
      return fn(bs);
    });
    setDirty(true);
  }

  function undo() {
    setUndoStack((st) => {
      if (st.length === 0) return st;
      const prev = st[st.length - 1]!;
      setBlocks(prev);
      setDirty(true);
      setSel((i) => Math.min(i, Math.max(0, prev.length - 1)));
      return st.slice(0, -1);
    });
  }

  // Ctrl/Cmd+Z and Ctrl/Cmd+F anywhere in the editor. Both only ever call
  // state setters, so the empty dependency list is safe.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const mod = e.ctrlKey || e.metaKey;
      if (mod && e.key.toLowerCase() === "z" && !e.shiftKey) {
        e.preventDefault();
        undo();
      } else if (mod && e.key.toLowerCase() === "f") {
        e.preventDefault();
        setFind((f) => f ?? "");
        setFindAt(0);
        requestAnimationFrame(() => findRef.current?.focus());
      }
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  function patchSelected(patch: Record<string, unknown>) {
    edit((bs) =>
      bs.map((b, i) => {
        if (i !== sel || !isWritten(b)) return b;
        const next = { ...b, ...patch } as Record<string, unknown>;
        // undefined means "back to default" — carried through it would
        // serialise as null and match no colour or size.
        Object.keys(patch).forEach((k) => {
          if (patch[k] === undefined) delete next[k];
        });
        return next as unknown as NoteBlock;
      }),
    );
  }

  /** Check / bullet / numbered are the same line with a different marker, so
   *  one toggle covers all three, and pressing the one it already is turns it
   *  back into plain text. */
  function toggleKind(kind: "check" | "bullet" | "num") {
    edit((bs) =>
      bs.map((b, i) => {
        if (i !== sel || !isWritten(b)) return b;
        const next = { ...b, k: b.k === kind ? "text" : kind } as Record<string, unknown>;
        if (next.k !== "check") delete next.d;
        return next as unknown as NoteBlock;
      }),
    );
  }

  /** Half width — and on EVERY kind, images and dividers included, because
   *  marko asked for it "na vsetky zlozky". */
  function toggleHalf(at = sel) {
    edit((bs) =>
      bs.map((b, i) => {
        if (i !== at) return b;
        const next = { ...b } as Record<string, unknown>;
        if (next.w === "half") delete next.w;
        else next.w = "half";
        return next as unknown as NoteBlock;
      }),
    );
    setSel(at);
  }

  function insertAfter(block: NoteBlock, at = sel) {
    edit((bs) => [...bs.slice(0, at + 1), block, ...bs.slice(at + 1)]);
    setSel(at + 1);
  }

  /** marko: "fungovanie s novym riadkom" — the new line has to take the
   *  cursor with it, otherwise Enter means "now reach for the mouse". */
  function focusBlock(i: number) {
    requestAnimationFrame(() => {
      const el = fieldRefs.current[`b${i}`];
      if (!el) return;
      el.focus();
      el.setSelectionRange(el.value.length, el.value.length);
    });
  }

  function moveBlock(i: number, delta: -1 | 1) {
    const to = i + delta;
    if (to < 0 || to >= blocks.length) return;
    edit((bs) => {
      const next = [...bs];
      const [m] = next.splice(i, 1);
      next.splice(to, 0, m!);
      return next;
    });
    setSel(to);
  }

  /** Dropped ON block `to`: the dragged one ends up at exactly that index,
   *  dragging up or down alike, and the block that was there moves aside.
   *  Same arithmetic as the sub-tabs and the note list, so all three feel
   *  the same. */
  function dropBlock(from: number, to: number) {
    if (from === to || from < 0 || from >= blocks.length) return;
    edit((bs) => {
      const next = [...bs];
      const [m] = next.splice(from, 1);
      next.splice(to, 0, m!);
      return next;
    });
    setSel(to);
  }

  async function pickImage(file: File) {
    try {
      const dataUri = await shrinkImage(file);
      const saved = await api.addNoteImage(note.id, dataUri, "");
      setImages((im) => [...im, saved]);
      insertAfter({ k: "image", id: saved.id });
      onNoteChanged();
    } catch (e) {
      toast.error(errMsg(e));
    }
  }

  /** Typed into the caption box: kept locally while typing, written on blur.
   *  The caption lives in `note_images.caption`, the column 2.58.0 already
   *  created and never used — no new field, no migration. */
  function typeCaption(id: number, caption: string) {
    setImages((im) => im.map((x) => (x.id === id ? { ...x, caption } : x)));
  }

  async function saveCaption(id: number) {
    const im = images.find((x) => x.id === id);
    if (!im) return;
    try {
      await api.setNoteImageCaption(id, im.caption);
    } catch (e) {
      toast.error(errMsg(e));
    }
  }

  /* --------------------------------- find ------------------------------- */

  const findHits = useMemo<FindHit[]>(() => {
    const q = (find ?? "").trim().toLowerCase();
    if (q === "") return [];
    const out: FindHit[] = [];
    const scan = (key: string, text: string) => {
      const low = text.toLowerCase();
      let at = low.indexOf(q);
      while (at !== -1) {
        out.push({ key, start: at, end: at + q.length });
        at = low.indexOf(q, at + q.length);
      }
    };
    scan("title", title);
    blocks.forEach((b, i) => {
      if (b.k === "rule") return;
      if (b.k === "image") {
        const im = images.find((x) => x.id === b.id);
        if (im) scan(`c${i}`, im.caption);
        return;
      }
      scan(`b${i}`, b.t);
    });
    return out;
  }, [find, title, blocks, images]);

  /** Puts the cursor ON the match rather than painting over it. The body is
   *  built from real textareas, so a highlight would mean shadowing every
   *  one of them with a second, read-only copy of its own text. */
  function goToHit(i: number) {
    if (findHits.length === 0) return;
    const at = ((i % findHits.length) + findHits.length) % findHits.length;
    setFindAt(at);
    const hit = findHits[at]!;
    if (hit.key.startsWith("b")) setSel(Number(hit.key.slice(1)));
    requestAnimationFrame(() => {
      const el = fieldRefs.current[hit.key];
      if (!el) return;
      el.focus();
      el.setSelectionRange(hit.start, hit.end);
      el.scrollIntoView({ block: "center" });
    });
  }

  function closeFind() {
    setFind(null);
    setFindAt(0);
  }

  /* ------------------------------- sub-tabs ----------------------------- */

  async function switchPage(id: number) {
    if (id === pageId || !pages) return;
    if (dirtyRef.current) await saveRef.current();
    await loadPage(id, pages);
  }

  async function submitName(value: string) {
    const p = namePrompt;
    setNamePrompt(null);
    if (!p) return;
    try {
      if (p.mode === "new") {
        if (dirtyRef.current) await saveRef.current();
        const created = await api.createNotePage(note.id, value.trim());
        const all = [...(pages ?? []), created];
        setPages(all);
        await loadPage(created.id, all);
      } else if (p.id !== undefined) {
        const saved = await api.renameNotePage(p.id, value.trim());
        setPages((ps) => ps?.map((x) => (x.id === saved.id ? saved : x)) ?? ps);
      }
      onNoteChanged();
    } catch (e) {
      toast.error(errMsg(e));
    }
  }

  async function reorderTabs(next: NotePage[]) {
    const before = pages;
    setPages(next); // shown immediately; the write confirms it
    try {
      setPages(await api.reorderNotePages(note.id, next.map((p) => p.id)));
      onNoteChanged();
    } catch (e) {
      toast.error(errMsg(e));
      setPages(before);
    }
  }

  async function moveTab(id: number, delta: -1 | 1) {
    if (!pages) return;
    const i = pages.findIndex((p) => p.id === id);
    const to = i + delta;
    if (i < 0 || to < 0 || to >= pages.length) return;
    const next = [...pages];
    const [m] = next.splice(i, 1);
    next.splice(to, 0, m!);
    await reorderTabs(next);
  }

  async function dropTab(fromId: number, toId: number) {
    if (!pages || fromId === toId) return;
    const from = pages.findIndex((p) => p.id === fromId);
    const to = pages.findIndex((p) => p.id === toId);
    if (from < 0 || to < 0) return;
    const next = [...pages];
    const [m] = next.splice(from, 1);
    next.splice(to, 0, m!);
    await reorderTabs(next);
  }

  async function duplicateTab(page: NotePage, withLinks: boolean) {
    setDupTab(null);
    try {
      if (dirtyRef.current) await saveRef.current();
      const known = new Set((pages ?? []).map((p) => p.id));
      const all = await api.duplicateNotePage(page.id, withLinks);
      setPages(all);
      const fresh = all.find((p) => !known.has(p.id));
      if (fresh) await loadPage(fresh.id, all);
      onNoteChanged();
      toast.success(withLinks ? "Podkarta aj s priradeniami" : "Podkarta bez priradení");
    } catch (e) {
      toast.error(errMsg(e));
    }
  }

  async function doConfirm() {
    const c = confirm;
    setConfirm(null);
    if (!c) return;
    try {
      if (c.what === "tab") {
        const left = await api.deleteNotePage(c.id);
        setPages(left);
        if (left[0]) await loadPage(left[0].id, left);
        onNoteChanged();
      } else {
        await api.deleteNoteImage(c.id);
        setImages((im) => im.filter((x) => x.id !== c.id));
        edit((bs) => bs.filter((b) => !(b.k === "image" && b.id === c.id)));
        setLightbox(null);
        onNoteChanged();
      }
    } catch (e) {
      toast.error(errMsg(e));
    }
  }

  const selected = blocks[sel];
  const canFormat = isWritten(selected);
  const activePage = pages?.find((p) => p.id === pageId) ?? null;
  const menuPage = pages?.find((p) => p.id === tabMenu?.id) ?? null;
  const lightboxImage = images.find((x) => x.id === lightbox) ?? null;
  const checks = blocks.filter((b) => b.k === "check");
  const checksDone = checks.filter((b) => b.k === "check" && b.d).length;

  const tbBtn =
    "rounded border border-slate-200 px-2 py-1 text-[12px] text-slate-600 transition hover:bg-surface-sunken disabled:opacity-40 dark:border-slate-700 dark:text-slate-300";
  const tbOn = "rounded border border-brand-500 bg-brand-600 px-2 py-1 text-[12px] font-semibold text-white";

  return (
    <div className="flex h-full min-h-0 flex-col">
      {/* ── toolbar ─────────────────────────────────────────────────── */}
      <div className="flex flex-wrap items-center gap-1.5 border-b border-slate-200 px-4 py-2 dark:border-slate-800">
        {(["h1", "h2", "p", "small"] as NoteTextSize[]).map((s) => (
          <button
            key={s}
            type="button"
            disabled={!canFormat}
            onClick={() => patchSelected({ s })}
            title={s === "h1" ? "Veľký nadpis" : s === "h2" ? "Nadpis" : s === "p" ? "Text" : "Malý text"}
            className={canFormat && (selected as { s?: NoteTextSize }).s === s ? tbOn : tbBtn}
            style={{ fontSize: s === "h1" ? 15 : s === "h2" ? 13 : s === "p" ? 11.5 : 10 }}
          >
            A
          </button>
        ))}
        <span className="mx-1 h-4 w-px bg-slate-200 dark:bg-slate-700" />
        <button type="button" disabled={!canFormat} onClick={() => patchSelected({ b: (selected as { b?: 1 })?.b ? undefined : 1 })}
          className={`${canFormat && (selected as { b?: 1 }).b ? tbOn : tbBtn} font-bold`}>B</button>
        <button type="button" disabled={!canFormat} onClick={() => patchSelected({ i: (selected as { i?: 1 })?.i ? undefined : 1 })}
          className={`${canFormat && (selected as { i?: 1 }).i ? tbOn : tbBtn} italic`}>I</button>
        <button type="button" disabled={!canFormat} onClick={() => toggleKind("check")} title="Zaškrtávací riadok"
          className={selected?.k === "check" ? tbOn : tbBtn}>☑ Zoznam</button>
        <button type="button" disabled={!canFormat} onClick={() => toggleKind("bullet")} title="Odrážka"
          className={selected?.k === "bullet" ? tbOn : tbBtn}>• Odrážka</button>
        <button type="button" disabled={!canFormat} onClick={() => toggleKind("num")} title="Číslovaný zoznam"
          className={selected?.k === "num" ? tbOn : tbBtn}>1. Číslovanie</button>
        <span className="mx-1 h-4 w-px bg-slate-200 dark:bg-slate-700" />
        {COLOURS.map((c) => (
          <button
            key={c.flag || "none"}
            type="button"
            disabled={!canFormat}
            onClick={() => patchSelected({ c: c.flag === "" ? undefined : c.flag })}
            title={c.label}
            aria-label={`Farba ${c.label}`}
            className="flex h-6 w-6 items-center justify-center rounded border border-slate-200 transition hover:border-slate-400 disabled:opacity-40 dark:border-slate-700"
          >
            <span className={`h-3 w-3 rounded-full ${c.dot}${c.flag ? "" : " opacity-40"}`} />
          </button>
        ))}
        <span className="mx-1 h-4 w-px bg-slate-200 dark:bg-slate-700" />
        <button type="button" className={tbBtn} onClick={() => fileRef.current?.click()}>▣ Obrázok</button>
        <button type="button" className={tbBtn} onClick={() => insertAfter({ k: "rule" })}>— Čiara</button>
        <button type="button" className={tbBtn} onClick={() => setLinkOpen(true)} disabled={pageId === null}>
          ⧉ Priradiť
        </button>
        <button
          type="button"
          disabled={!selected}
          onClick={() => toggleHalf()}
          title="Na polovicu — dva takéto bloky stoja vedľa seba"
          className={selected?.w === "half" ? tbOn : tbBtn}
        >
          ⬓ Na polovicu
        </button>
        <span className="mx-1 h-4 w-px bg-slate-200 dark:bg-slate-700" />
        <button type="button" className={find !== null ? tbOn : tbBtn} title="Nájsť v poznámke (Ctrl+F)"
          onClick={() => {
            if (find !== null) closeFind();
            else {
              setFind("");
              setFindAt(0);
              requestAnimationFrame(() => findRef.current?.focus());
            }
          }}>
          ⌕ Nájsť
        </button>
        <button type="button" className={tbBtn} onClick={undo} disabled={undoStack.length === 0}
          title="Vrátiť poslednú zmenu (Ctrl+Z)">
          ↶ Späť{undoStack.length > 0 ? ` (${undoStack.length})` : ""}
        </button>
        <input
          ref={fileRef}
          type="file"
          accept="image/*"
          className="hidden"
          onChange={(e) => {
            const f = e.target.files?.[0];
            e.target.value = "";
            if (f) void pickImage(f);
          }}
        />
        <label className="ml-auto flex items-center gap-2 text-[11.5px] text-slate-500 dark:text-slate-400">
          Šírka
          <input
            type="range"
            min={DOC_WIDTH_MIN}
            max={DOC_WIDTH_MAX}
            step={20}
            value={docWidth}
            title="Šírka plochy, kde sa píše"
            aria-label="Šírka plochy, kde sa píše"
            onChange={(e) => {
              const n = Number(e.target.value);
              setDocWidth(n);
              writeDocWidth(n);
            }}
            className="h-1.5 w-24 accent-brand-600"
          />
          <span className="w-[46px] text-right tabular-nums">{docWidth} px</span>
        </label>
      </div>

      {/* ── body ────────────────────────────────────────────────────── */}
      <div className="min-h-0 flex-1 overflow-y-auto px-8 py-5">
        <div className="mx-auto transition-[max-width] duration-100" style={{ maxWidth: docWidth }}>
          {find !== null && (
            <div className="mb-3 flex items-center gap-1.5 rounded-lg bg-surface-sunken px-2 py-1.5 ring-1 ring-slate-200 dark:ring-slate-700">
              <input
                ref={findRef}
                value={find}
                onChange={(e) => {
                  setFind(e.target.value);
                  setFindAt(0);
                }}
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    e.preventDefault();
                    goToHit(findAt + (e.shiftKey ? -1 : 1));
                  } else if (e.key === "Escape") {
                    e.preventDefault();
                    closeFind();
                  }
                }}
                placeholder="Nájsť v tejto poznámke…"
                aria-label="Nájsť v tejto poznámke"
                className="w-[200px] rounded border border-slate-200 bg-surface px-2 py-1 text-[12.5px] outline-none dark:border-slate-700"
              />
              <span className="w-[54px] text-center text-[11.5px] tabular-nums text-slate-500 dark:text-slate-400">
                {findHits.length === 0 ? "0 / 0" : `${findAt + 1} / ${findHits.length}`}
              </span>
              <button type="button" className={tbBtn} title="Predchádzajúce" aria-label="Predchádzajúce"
                disabled={findHits.length === 0} onClick={() => goToHit(findAt - 1)}>‹</button>
              <button type="button" className={tbBtn} title="Ďalšie" aria-label="Ďalšie"
                disabled={findHits.length === 0} onClick={() => goToHit(findAt + 1)}>›</button>
              <button type="button" className={`${tbBtn} ml-auto`} title="Zavrieť (Esc)" aria-label="Zavrieť hľadanie"
                onClick={closeFind}>×</button>
            </div>
          )}

          <input
            ref={(el) => {
              fieldRefs.current.title = el;
            }}
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            onBlur={() => void saveHeader()}
            placeholder="Názov poznámky"
            aria-label="Názov poznámky"
            className="w-full bg-transparent text-[24px] font-bold text-slate-900 outline-none placeholder:text-slate-300 dark:text-slate-50 dark:placeholder:text-slate-700"
          />
          <div className="mb-4 mt-2 flex flex-wrap items-center gap-2 text-[11.5px] text-slate-500 dark:text-slate-400">
            {/* The date and the tag can be taken off again - marko: "stitok a
                datum moznost odstranit ked ho tam nechces". */}
            <span className="inline-flex items-center gap-1">
              <Input type="date" value={date} onChange={(e) => setDate(e.target.value)} onBlur={() => void saveHeader()}
                className="!w-[150px] !py-0.5 !text-[11.5px]" aria-label="Dátum" />
              {date && (
                <button type="button" aria-label="Odstrániť dátum" title="Odstrániť dátum"
                  onClick={() => { setDate(""); void saveHeader(tag, ""); }}
                  className="text-slate-400 transition hover:text-red-500">
                  <IconX className="h-3 w-3" />
                </button>
              )}
            </span>
            <span className="inline-flex items-center gap-1">
              <Input value={tag} onChange={(e) => setTag(e.target.value)} onBlur={() => void saveHeader()}
                placeholder="štítok" className="!w-[130px] !py-0.5 !text-[11.5px]" aria-label="Štítok" />
              {tag && (
                <button type="button" aria-label="Odstrániť štítok" title="Odstrániť štítok"
                  onClick={() => { setTag(""); void saveHeader("", date); }}
                  className="text-slate-400 transition hover:text-red-500">
                  <IconX className="h-3 w-3" />
                </button>
              )}
            </span>
            {/* A chip is a link (2.62.0). `href` comes from Rust, built from
                the routes that really exist - see `link_href`. */}
            {links.map((l) => (
              <span key={l.id}
                className="inline-flex items-center gap-1.5 rounded-full border border-amber-300 bg-amber-50 py-0.5 pl-2.5 pr-1.5 text-amber-800 dark:border-amber-800 dark:bg-amber-950/40 dark:text-amber-300">
                <button type="button" onClick={() => l.href && navigate(l.href)} disabled={!l.href}
                  title={LINK_DEST[l.kind] ? `Otvoriť — ${LINK_DEST[l.kind]}` : "Otvoriť"}
                  className="inline-flex items-center gap-1.5 transition hover:underline disabled:no-underline">
                  <b className="font-semibold">{LINK_KINDS.find((k) => k.kind === l.kind)?.label ?? l.kind}</b>
                  {l.label}
                </button>
                <button type="button" aria-label="Odpojiť" title="Odpojiť"
                  onClick={async () => {
                    try {
                      await api.deleteNoteLink(l.id);
                      if (pageId !== null) setLinks(await api.listNotePageLinks(pageId));
                      onNoteChanged();
                    } catch (e) {
                      toast.error(errMsg(e));
                    }
                  }}
                  className="opacity-60 transition hover:opacity-100">
                  <IconX className="h-3 w-3" />
                </button>
              </span>
            ))}
            {checks.length > 0 && (
              <span className="inline-flex items-center gap-2" title="Hotové položky tejto podkarty">
                <span className="tabular-nums">
                  {checksDone} / {checks.length} hotových
                </span>
                <span className="h-[3px] w-14 overflow-hidden rounded-full bg-slate-200 dark:bg-slate-700">
                  <span className="block h-full rounded-full bg-brand-500 transition-[width]"
                    style={{ width: `${Math.round((checksDone / checks.length) * 100)}%` }} />
                </span>
              </span>
            )}
          </div>

          {pages === null ? (
            <p className="text-sm text-slate-500 dark:text-slate-400">Načítavam…</p>
          ) : (
            <div className="flex flex-wrap items-start gap-x-3 gap-y-1">
              {blocks.map((b, i) => (
                <BlockRow
                  key={i}
                  block={b}
                  index={i}
                  number={numberOf(blocks, i)}
                  selected={i === sel}
                  first={i === 0}
                  last={i === blocks.length - 1}
                  image={b.k === "image" ? images.find((x) => x.id === b.id) : undefined}
                  onSelect={() => setSel(i)}
                  onText={(t) =>
                    edit((bs) =>
                      bs.map((x, k) => (k === i && x.k !== "image" && x.k !== "rule" ? ({ ...x, t } as NoteBlock) : x)),
                    )
                  }
                  onCaption={(t) => b.k === "image" && typeCaption(b.id, t)}
                  onCaptionBlur={() => b.k === "image" && void saveCaption(b.id)}
                  onOpenImage={() => b.k === "image" && setLightbox(b.id)}
                  onToggleDone={() =>
                    edit((bs) =>
                      bs.map((x, k) => {
                        if (k !== i || x.k !== "check") return x;
                        if (x.d) {
                          const { d: _d, ...rest } = x;
                          return rest as NoteBlock;
                        }
                        return { ...x, d: 1 } as NoteBlock;
                      }),
                    )
                  }
                  onEnter={() => {
                    // Enter continues the kind you are in. Written as four
                    // literals rather than one `{ k: kind }`: a union-typed
                    // `k` is not a NoteBlock, only a literal one is.
                    const next: NoteBlock =
                      b.k === "check"
                        ? { k: "check", t: "" }
                        : b.k === "bullet"
                          ? { k: "bullet", t: "" }
                          : b.k === "num"
                            ? { k: "num", t: "" }
                            : { k: "text", t: "" };
                    insertAfter(next, i);
                    focusBlock(i + 1);
                  }}
                  onRemove={() => {
                    if (b.k === "image") {
                      setConfirm({ what: "image", id: b.id, label: "obrázok" });
                      return;
                    }
                    edit((bs) => bs.filter((_, k) => k !== i));
                    setSel(Math.max(0, i - 1));
                    focusBlock(Math.max(0, i - 1));
                  }}
                  onMove={(d) => moveBlock(i, d)}
                  onHalf={() => toggleHalf(i)}
                  onDropFrom={(from) => dropBlock(from, i)}
                  registerRef={(el) => {
                    fieldRefs.current[`b${i}`] = el;
                  }}
                  registerCaptionRef={(el) => {
                    fieldRefs.current[`c${i}`] = el;
                  }}
                />
              ))}
              <button
                type="button"
                onClick={() => {
                  const at = blocks.length - 1;
                  insertAfter({ k: "text", t: "" }, at);
                  focusBlock(at + 1);
                }}
                className="mt-2 flex w-full items-center gap-1.5 rounded px-1 py-1 text-[12.5px] text-slate-400 transition hover:text-slate-700 dark:hover:text-slate-200"
              >
                <IconPlus className="h-3.5 w-3.5" /> Ďalší riadok
              </button>
            </div>
          )}
        </div>
      </div>

      {/* ── sub-tabs ────────────────────────────────────────────────── */}
      <div className="flex items-center gap-0.5 overflow-x-auto border-t border-slate-200 bg-surface-sunken px-2 pt-1 dark:border-slate-800">
        {(pages ?? []).map((p) => {
          const on = p.id === pageId;
          return (
            <span
              key={p.id}
              draggable
              onDragStart={(e) => {
                e.dataTransfer.setData("text/plain", `page:${p.id}`);
                e.dataTransfer.effectAllowed = "move";
              }}
              onDragOver={(e) => e.preventDefault()}
              onDrop={(e) => {
                e.preventDefault();
                const raw = e.dataTransfer.getData("text/plain");
                if (!raw.startsWith("page:")) return;
                void dropTab(Number(raw.slice(5)), p.id);
              }}
              className={`group flex shrink-0 cursor-grab items-center gap-1 rounded-t-md border border-b-0 px-2.5 py-1.5 text-[12.5px] transition ${
                on
                  ? "border-slate-200 border-t-2 border-t-brand-600 bg-surface font-semibold text-brand-700 dark:border-slate-700 dark:border-t-brand-500 dark:text-brand-300"
                  : "border-transparent text-slate-500 hover:text-slate-800 dark:text-slate-400 dark:hover:text-slate-100"
              }`}
            >
              <button type="button" onClick={() => void switchPage(p.id)}
                onDoubleClick={() => setNamePrompt({ mode: "rename", id: p.id, value: p.name })}>
                {p.name || "bez názvu"}
              </button>
              {on && (
                <>
                  <button type="button" aria-label="Posunúť doľava" title="Posunúť doľava"
                    onClick={() => void moveTab(p.id, -1)}
                    className="opacity-40 transition hover:opacity-100">‹</button>
                  <button type="button" aria-label="Posunúť doprava" title="Posunúť doprava"
                    onClick={() => void moveTab(p.id, 1)}
                    className="opacity-40 transition hover:opacity-100">›</button>
                  <button type="button" aria-label="Viac" title="Viac"
                    onClick={(e) => setTabMenu({ at: menuAt(e, true), id: p.id })}
                    className="opacity-40 transition hover:opacity-100">⋯</button>
                </>
              )}
            </span>
          );
        })}
        <button type="button" onClick={() => setNamePrompt({ mode: "new", value: "" })}
          title="Nová podkarta" aria-label="Nová podkarta"
          className="shrink-0 rounded px-2.5 py-1 text-base leading-none text-slate-500 transition hover:bg-surface hover:text-brand-600 dark:text-slate-400">
          +
        </button>
        <span className="ml-auto pr-2 text-[11px] text-slate-400 dark:text-slate-500">
          {dirty ? "Ukladám…" : "Uložené"}
        </span>
      </div>

      <Menu
        at={tabMenu?.at ?? null}
        onClose={() => setTabMenu(null)}
        items={
          menuPage
            ? [
                { label: "Premenovať", onClick: () => setNamePrompt({ mode: "rename", id: menuPage.id, value: menuPage.name }) },
                { label: "Duplikovať…", onClick: () => setDupTab(menuPage) },
                "-",
                { label: "Posunúť doľava", onClick: () => void moveTab(menuPage.id, -1) },
                { label: "Posunúť doprava", onClick: () => void moveTab(menuPage.id, 1) },
                "-",
                {
                  label: "Zmazať podkartu",
                  danger: true,
                  disabled: (pages?.length ?? 0) < 2,
                  onClick: () => setConfirm({ what: "tab", id: menuPage.id, label: menuPage.name || "bez názvu" }),
                },
              ]
            : []
        }
      />

      <DuplicateDialog
        open={dupTab !== null}
        title="Duplikovať podkartu"
        intro="Obsah sa skopíruje vždy — text, nadpisy, zaškrtávacie riadky, obrázky, čiary aj poradie. Priradenia len ak si ich vypýtaš."
        contentLabel="Kópia nebude priradená k ničomu."
        linksLabel="Kópia sa priradí k tomu istému, čo táto podkarta."
        onCancel={() => setDupTab(null)}
        onConfirm={(withLinks) => dupTab && void duplicateTab(dupTab, withLinks)}
      />

      <LinkPicker
        open={linkOpen}
        pageId={pageId}
        pageName={activePage?.name ?? ""}
        onClose={() => setLinkOpen(false)}
        onLinked={async () => {
          if (pageId !== null) setLinks(await api.listNotePageLinks(pageId).catch(() => []));
          onNoteChanged();
        }}
      />

      <NamePrompt
        state={namePrompt}
        onCancel={() => setNamePrompt(null)}
        onSubmit={(v) => void submitName(v)}
      />

      {lightboxImage && (
        <div
          className="fixed inset-0 z-[80] flex flex-col items-center justify-center gap-3 bg-slate-950/80 p-6 backdrop-blur-[2px] animate-[fadein_.15s_ease-out]"
          onMouseDown={(e) => {
            if (e.target === e.currentTarget) setLightbox(null);
          }}
        >
          <img src={lightboxImage.dataUri} alt={lightboxImage.caption}
            className="max-h-[70vh] max-w-[min(92vw,900px)] rounded-lg" />
          <p className="max-w-[min(92vw,900px)] text-center text-[12.5px] text-slate-300">
            {lightboxImage.caption || "Bez popisu"}
          </p>
          <div className="flex gap-2">
            <Button variant="secondary" onClick={() => setLightbox(null)}>
              Zavrieť
            </Button>
            <Button variant="danger" onClick={() => setConfirm({ what: "image", id: lightboxImage.id, label: "obrázok" })}>
              Zmazať obrázok
            </Button>
          </div>
        </div>
      )}

      <ConfirmDialog
        open={confirm !== null}
        title={confirm?.what === "tab" ? `Zmazať podkartu „${confirm.label}"?` : "Zmazať obrázok?"}
        message={
          confirm?.what === "tab"
            ? "Zmaže sa všetko, čo je v nej napísané, aj jej priradenia. Toto sa nedá vrátiť."
            : "Obrázok sa zmaže z poznámky aj z databázy. Toto sa nedá vrátiť."
        }
        confirmLabel="Áno, zmazať"
        danger
        onCancel={() => setConfirm(null)}
        onConfirm={() => void doConfirm()}
      />
    </div>
  );
}

/** What number a `num` block shows: how many numbered lines run without a
 *  break right above it, plus one. Nothing about it is stored. */
function numberOf(blocks: NoteBlock[], i: number): number {
  if (blocks[i]?.k !== "num") return 0;
  let n = 1;
  for (let k = i - 1; k >= 0 && blocks[k]?.k === "num"; k--) n++;
  return n;
}

/* ------------------------------------------------------------------ *
 * The in-app name box — marko: "nie systemove ale apkove"
 * ------------------------------------------------------------------ */

function NamePrompt({
  state,
  onCancel,
  onSubmit,
}: {
  state: null | { mode: "new" | "rename"; id?: number; value: string };
  onCancel: () => void;
  onSubmit: (value: string) => void;
}) {
  const [value, setValue] = useState("");
  useEffect(() => {
    if (state) setValue(state.value);
  }, [state]);
  return (
    <Modal
      open={state !== null}
      onClose={onCancel}
      title={state?.mode === "rename" ? "Premenovať podkartu" : "Nová podkarta"}
      width="max-w-sm"
    >
      <label>
        <span className="label">Názov</span>
        <Input
          autoFocus
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") onSubmit(value);
          }}
          placeholder="napr. ABC123"
          aria-label="Názov podkarty"
        />
      </label>
      <ModalFooter>
        <Button variant="secondary" onClick={onCancel}>
          Zrušiť
        </Button>
        <Button variant="primary" onClick={() => onSubmit(value)}>
          {state?.mode === "rename" ? "Premenovať" : "Vytvoriť"}
        </Button>
      </ModalFooter>
    </Modal>
  );
}

/* ------------------------------------------------------------------ *
 * One block
 * ------------------------------------------------------------------ */

function BlockRow({
  block,
  index,
  number,
  selected,
  first,
  last,
  image,
  onSelect,
  onText,
  onCaption,
  onCaptionBlur,
  onOpenImage,
  onToggleDone,
  onEnter,
  onRemove,
  onMove,
  onHalf,
  onDropFrom,
  registerRef,
  registerCaptionRef,
}: {
  block: NoteBlock;
  index: number;
  number: number;
  selected: boolean;
  first: boolean;
  last: boolean;
  image?: NoteImage;
  onSelect: () => void;
  onText: (t: string) => void;
  onCaption: (t: string) => void;
  onCaptionBlur: () => void;
  onOpenImage: () => void;
  onToggleDone: () => void;
  onEnter: () => void;
  onRemove: () => void;
  onMove: (delta: -1 | 1) => void;
  onHalf: () => void;
  onDropFrom: (from: number) => void;
  registerRef: (el: HTMLTextAreaElement | null) => void;
  registerCaptionRef: (el: HTMLInputElement | null) => void;
}) {
  const half = block.w === "half";
  const shell = `group flex items-start gap-2 rounded ${half ? "basis-[calc(50%_-_0.375rem)]" : "w-full"} min-w-0 ${
    selected ? "ring-1 ring-brand-500/40" : ""
  }`;

  /* The GRIP is what is draggable, not the row: a draggable row would take
     the mouse away from selecting text inside its own textarea. */
  const grip = (
    <span
      draggable
      onDragStart={(e) => {
        e.dataTransfer.setData("text/plain", `blk:${index}`);
        e.dataTransfer.effectAllowed = "move";
      }}
      title="Potiahni pre presun"
      aria-hidden="true"
      className="w-3.5 shrink-0 cursor-grab select-none self-start pt-1 text-center text-[11px] leading-none text-transparent transition group-hover:text-slate-300 dark:group-hover:text-slate-600"
    >
      ⠿
    </span>
  );

  const handles = (
    <span className="ml-auto flex shrink-0 items-center gap-0.5 self-start pt-1 text-transparent transition group-hover:text-slate-300 dark:group-hover:text-slate-600">
      <button type="button" onClick={() => onMove(-1)} disabled={first} aria-label="Posunúť hore" title="Posunúť hore"
        className="rounded px-1 leading-none hover:!text-slate-700 disabled:opacity-0 dark:hover:!text-slate-200">↑</button>
      <button type="button" onClick={() => onMove(1)} disabled={last} aria-label="Posunúť dole" title="Posunúť dole"
        className="rounded px-1 leading-none hover:!text-slate-700 disabled:opacity-0 dark:hover:!text-slate-200">↓</button>
      <button type="button" onClick={onHalf} aria-label={half ? "Na celú šírku" : "Na polovicu"}
        title={half ? "Na celú šírku" : "Na polovicu — vedľa seba s ďalším"}
        className="rounded px-1 leading-none hover:!text-brand-600 dark:hover:!text-brand-400">{half ? "▭" : "⬓"}</button>
      <button type="button" onClick={onRemove} aria-label="Zmazať riadok" title="Zmazať riadok"
        className="rounded p-0.5 hover:!text-red-500">
        <IconX className="h-3.5 w-3.5" />
      </button>
    </span>
  );

  const dnd = {
    onDragOver: (e: React.DragEvent) => e.preventDefault(),
    onDrop: (e: React.DragEvent) => {
      e.preventDefault();
      const raw = e.dataTransfer.getData("text/plain");
      if (!raw.startsWith("blk:")) return;
      onDropFrom(Number(raw.slice(4)));
    },
  };

  if (block.k === "rule") {
    return (
      <div onMouseDown={onSelect} {...dnd} className={`${shell} py-2`}>
        {grip}
        <hr className="mt-2 flex-1 border-slate-200 dark:border-slate-700" />
        {handles}
      </div>
    );
  }

  if (block.k === "image") {
    return (
      <div onMouseDown={onSelect} {...dnd} className={`${shell} p-1`}>
        {grip}
        <figure className={`${half ? "max-w-full" : "max-w-[420px]"} min-w-0 overflow-hidden rounded-lg border border-slate-200 dark:border-slate-700`}>
          {image ? (
            <img src={image.dataUri} alt={image.caption} onClick={onOpenImage}
              title="Otvoriť na celú veľkosť" className="block w-full cursor-zoom-in" />
          ) : (
            <div className="grid h-24 place-items-center text-xs text-slate-400">Obrázok sa nenašiel</div>
          )}
          <figcaption>
            <input
              ref={registerCaptionRef}
              value={image?.caption ?? ""}
              disabled={!image}
              onChange={(e) => onCaption(e.target.value)}
              onBlur={onCaptionBlur}
              placeholder="Pridať popis…"
              aria-label="Popis obrázka"
              className="w-full bg-surface-sunken px-2 py-1 text-[11.5px] text-slate-600 outline-none placeholder:text-slate-400 dark:text-slate-300 dark:placeholder:text-slate-600"
            />
          </figcaption>
        </figure>
        {handles}
      </div>
    );
  }

  const size = SIZE_CLASS[block.s ?? "p"];
  const colour = block.c ? COLOUR_CLASS[block.c] : "text-slate-800 dark:text-slate-200";
  const marks = `${block.b ? " font-semibold" : ""}${block.i ? " italic" : ""}`;
  const done = block.k === "check" && block.d;

  return (
    <div onMouseDown={onSelect} {...dnd} className={`${shell} px-1 py-0.5`}>
      {grip}
      {block.k === "check" && (
        <button
          type="button"
          onClick={onToggleDone}
          aria-label={done ? "Odškrtnúť" : "Zaškrtnúť"}
          className={`mt-[7px] h-4 w-4 shrink-0 rounded border transition ${
            done ? "border-brand-600 bg-brand-600" : "border-slate-300 dark:border-slate-600"
          }`}
        >
          {done && <span className="block text-[10px] leading-4 text-white">✓</span>}
        </button>
      )}
      {block.k === "bullet" && (
        <span aria-hidden="true" className="mt-[3px] shrink-0 text-slate-400 dark:text-slate-500">
          •
        </span>
      )}
      {block.k === "num" && (
        <span aria-hidden="true" className="mt-[3px] min-w-[16px] shrink-0 text-right text-[13px] tabular-nums text-slate-400 dark:text-slate-500">
          {number}.
        </span>
      )}
      <textarea
        ref={registerRef}
        value={block.t}
        rows={1}
        onChange={(e) => {
          onText(e.target.value);
          e.target.style.height = "auto";
          e.target.style.height = `${e.target.scrollHeight}px`;
        }}
        onFocus={(e) => {
          e.target.style.height = "auto";
          e.target.style.height = `${e.target.scrollHeight}px`;
        }}
        onKeyDown={(e) => {
          if (e.key === "Enter" && !e.shiftKey) {
            e.preventDefault();
            onEnter();
          } else if (e.key === "Backspace" && block.t === "") {
            e.preventDefault();
            onRemove();
          }
        }}
        placeholder="Píš…"
        className={`w-full resize-none overflow-hidden bg-transparent outline-none placeholder:text-slate-300 dark:placeholder:text-slate-700 ${size} ${colour}${marks}${
          done ? " line-through opacity-50" : ""
        }`}
      />
      {handles}
    </div>
  );
}

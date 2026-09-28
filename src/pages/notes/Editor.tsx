import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { api, errMsg } from "../../lib/api";
import type { Note, NoteBlock, NoteColour, NoteImage, NotePage, NoteTextSize } from "../../lib/types";
import { Input } from "../../components/ui";
import { IconPlus, IconTrash, IconX } from "../../components/icons";
import { useToast } from "../../lib/toast";
import LinkPicker, { LINK_KINDS } from "./LinkPicker";

/**
 * One note: its toolbar, its body, and its sub-tabs along the bottom.
 *
 * marko, picking design 02: *"presne aj to ze si vies zaskrtnut, ze si vies
 * pridat take veci ktore vies pouzit, aj to ze si vies vybrat kde chces mat
 * fotku, kde chces pisat, kde ten text ma byt vacsi, aka farba"* — so the body
 * is a list of BLOCKS and every one of those decisions is made per block, not
 * per note. Then: *"v jednej karte si vies pridat podkarty niekde dole ako to
 * je v google sheets"* — hence the tab strip at the bottom.
 *
 * ## The selected block is the subject of the toolbar
 *
 * Exactly one block is selected at a time and the toolbar acts on it. That is
 * what makes "kde ten text ma byt vacsi" a thing you can actually do rather
 * than a setting for the whole note.
 *
 * ## It saves itself
 *
 * One debounce per page, plus a flush when you leave the page or the note.
 * Never per keystroke: a page is stored whole.
 */

const SAVE_DEBOUNCE_MS = 700;
/** The longest edge an attached image is allowed to keep. Anything bigger is
 *  re-encoded before it is ever sent: the whole database is uploaded on every
 *  sync, and a 6MB phone screenshot would be paid for again and again. */
const MAX_IMAGE_EDGE = 1400;
const JPEG_QUALITY = 0.72;

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

/** Reads a picked file, shrinks it and re-encodes it as JPEG. Uses a canvas,
 *  so nothing new is installed for it. */
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
        // A white ground, because a PNG with transparency turns black on JPEG.
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

export default function Editor({
  note,
  onNoteChanged,
  onDeleted,
}: {
  note: Note;
  onNoteChanged: () => void;
  onDeleted: () => void;
}) {
  const toast = useToast();
  const [pages, setPages] = useState<NotePage[] | null>(null);
  const [pageId, setPageId] = useState<number | null>(null);
  const [blocks, setBlocks] = useState<NoteBlock[]>([]);
  const [sel, setSel] = useState(0);
  const [images, setImages] = useState<NoteImage[]>([]);
  const [title, setTitle] = useState(note.title);
  const [tag, setTag] = useState(note.tag);
  const [date, setDate] = useState(note.noteDate ?? "");
  const [dirty, setDirty] = useState(false);
  const [linkOpen, setLinkOpen] = useState(false);
  const fileRef = useRef<HTMLInputElement>(null);
  const areaRefs = useRef<Record<number, HTMLTextAreaElement | null>>({});

  /* ------------------------------- loading ------------------------------ */

  useEffect(() => {
    setTitle(note.title);
    setTag(note.tag);
    setDate(note.noteDate ?? "");
  }, [note.id, note.title, note.tag, note.noteDate]);

  useEffect(() => {
    let alive = true;
    setPages(null);
    Promise.all([api.listNotePages(note.id), api.listNoteImages(note.id)])
      .then(([p, im]) => {
        if (!alive) return;
        setPages(p);
        setImages(im);
        const first = p[0] ?? null;
        setPageId(first?.id ?? null);
        setBlocks(first?.blocks ?? []);
        setSel(0);
        setDirty(false);
      })
      .catch((e) => {
        if (alive) toast.error(errMsg(e));
      });
    return () => {
      alive = false;
    };
  }, [note.id, toast]);

  const page = useMemo(() => pages?.find((p) => p.id === pageId) ?? null, [pages, pageId]);

  /* -------------------------------- saving ------------------------------ */

  const draft = useRef({ pageId, blocks, title, tag, date });
  draft.current = { pageId, blocks, title, tag, date };

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

  // Leaving the note, or switching tabs, flushes whatever is still unsaved.
  useEffect(
    () => () => {
      if (dirtyRef.current) void saveRef.current();
    },
    [],
  );

  async function saveHeader() {
    try {
      await api.updateNote(note.id, title, tag, date.trim() || null);
      onNoteChanged();
    } catch (e) {
      toast.error(errMsg(e));
    }
  }

  /* -------------------------------- blocks ------------------------------ */

  function edit(fn: (bs: NoteBlock[]) => NoteBlock[]) {
    setBlocks((bs) => fn(bs));
    setDirty(true);
  }

  function patchSelected(patch: Partial<Extract<NoteBlock, { k: "text" }>>) {
    edit((bs) =>
      bs.map((b, i) => {
        if (i !== sel || (b.k !== "text" && b.k !== "check")) return b;
        const next = { ...b, ...patch } as NoteBlock;
        // An empty value means "back to default" - carrying `c: undefined`
        // around would serialise as null and then not match any colour.
        Object.keys(patch).forEach((key) => {
          const k = key as keyof typeof patch;
          if (patch[k] === undefined) delete (next as Record<string, unknown>)[key];
        });
        return next;
      }),
    );
  }

  function setSize(s: NoteTextSize) {
    patchSelected({ s });
  }

  function setColour(c: NoteColour | "") {
    patchSelected(c === "" ? { c: undefined } : { c });
  }

  function toggleMark(mark: "b" | "i") {
    const b = blocks[sel];
    if (!b || (b.k !== "text" && b.k !== "check")) return;
    patchSelected(b[mark] ? { [mark]: undefined } : ({ [mark]: 1 } as Partial<Extract<NoteBlock, { k: "text" }>>));
  }

  function toggleCheck() {
    edit((bs) =>
      bs.map((b, i) => {
        if (i !== sel) return b;
        if (b.k === "text") return { ...b, k: "check" } as NoteBlock;
        if (b.k === "check") {
          const { d: _d, ...rest } = b;
          return { ...rest, k: "text" } as NoteBlock;
        }
        return b;
      }),
    );
  }

  function insertAfter(block: NoteBlock) {
    edit((bs) => [...bs.slice(0, sel + 1), block, ...bs.slice(sel + 1)]);
    setSel((i) => i + 1);
  }

  function removeBlock(i: number) {
    edit((bs) => bs.filter((_, k) => k !== i));
    setSel((s) => Math.max(0, Math.min(s, blocks.length - 2)));
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

  /* ------------------------------- sub-tabs ----------------------------- */

  async function switchPage(id: number) {
    if (id === pageId) return;
    if (dirtyRef.current) await saveRef.current();
    const p = pages?.find((x) => x.id === id);
    setPageId(id);
    setBlocks(p?.blocks ?? []);
    setSel(0);
    setDirty(false);
  }

  async function addPage() {
    const name = window.prompt("Názov podkarty");
    if (name === null) return;
    try {
      if (dirtyRef.current) await saveRef.current();
      const created = await api.createNotePage(note.id, name.trim());
      setPages((ps) => [...(ps ?? []), created]);
      setPageId(created.id);
      setBlocks([]);
      setSel(0);
      setDirty(false);
      onNoteChanged();
    } catch (e) {
      toast.error(errMsg(e));
    }
  }

  const selected = blocks[sel];
  const canFormat = !!selected && (selected.k === "text" || selected.k === "check");

  const tbBtn =
    "rounded border border-slate-200 px-2 py-1 text-[12px] text-slate-600 transition hover:bg-surface-sunken dark:border-slate-700 dark:text-slate-300";
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
            onClick={() => setSize(s)}
            title={s === "h1" ? "Veľký nadpis" : s === "h2" ? "Nadpis" : s === "p" ? "Text" : "Malý text"}
            className={`${canFormat && (selected as { s?: NoteTextSize }).s === s ? tbOn : tbBtn} disabled:opacity-40`}
            style={{ fontSize: s === "h1" ? 15 : s === "h2" ? 13 : s === "p" ? 11.5 : 10 }}
          >
            A
          </button>
        ))}
        <span className="mx-1 h-4 w-px bg-slate-200 dark:bg-slate-700" />
        <button type="button" disabled={!canFormat} onClick={() => toggleMark("b")}
          className={`${canFormat && (selected as { b?: 1 }).b ? tbOn : tbBtn} font-bold disabled:opacity-40`}>B</button>
        <button type="button" disabled={!canFormat} onClick={() => toggleMark("i")}
          className={`${canFormat && (selected as { i?: 1 }).i ? tbOn : tbBtn} italic disabled:opacity-40`}>I</button>
        <button type="button" disabled={!canFormat} onClick={toggleCheck}
          className={`${selected?.k === "check" ? tbOn : tbBtn} disabled:opacity-40`}>☑ Zoznam</button>
        <span className="mx-1 h-4 w-px bg-slate-200 dark:bg-slate-700" />
        {COLOURS.map((c) => (
          <button
            key={c.flag || "none"}
            type="button"
            disabled={!canFormat}
            onClick={() => setColour(c.flag)}
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
        <button type="button" className={tbBtn} onClick={() => setLinkOpen(true)}>⧉ Priradiť</button>
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
        <button type="button" className={`${tbBtn} ml-auto`} onClick={onDeleted}>
          <IconTrash className="inline h-3.5 w-3.5" /> Zmazať
        </button>
      </div>

      {/* ── body ────────────────────────────────────────────────────── */}
      <div className="min-h-0 flex-1 overflow-y-auto px-8 py-5">
        <input
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          onBlur={saveHeader}
          placeholder="Názov poznámky"
          aria-label="Názov poznámky"
          className="w-full bg-transparent text-[24px] font-bold text-slate-900 outline-none placeholder:text-slate-300 dark:text-slate-50 dark:placeholder:text-slate-700"
        />
        <div className="mb-4 mt-2 flex flex-wrap items-center gap-2 text-[11.5px] text-slate-500 dark:text-slate-400">
          <Input type="date" value={date} onChange={(e) => setDate(e.target.value)} onBlur={saveHeader}
            className="!w-[150px] !py-0.5 !text-[11.5px]" aria-label="Dátum" />
          <Input value={tag} onChange={(e) => setTag(e.target.value)} onBlur={saveHeader} placeholder="štítok"
            className="!w-[130px] !py-0.5 !text-[11.5px]" aria-label="Štítok" />
          {note.links.map((l) => (
            <span key={l.id}
              className="inline-flex items-center gap-1.5 rounded-full border border-amber-300 bg-amber-50 px-2.5 py-0.5 text-amber-800 dark:border-amber-800 dark:bg-amber-950/40 dark:text-amber-300">
              <b className="font-semibold">{LINK_KINDS.find((k) => k.kind === l.kind)?.label ?? l.kind}</b>
              {l.label}
              <button type="button" aria-label="Odpojiť" title="Odpojiť"
                onClick={async () => {
                  try {
                    await api.deleteNoteLink(l.id);
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
        </div>

        {pages === null ? (
          <p className="text-sm text-slate-500 dark:text-slate-400">Načítavam…</p>
        ) : (
          <div className="flex flex-col gap-1">
            {blocks.map((b, i) => (
              <BlockRow
                key={i}
                block={b}
                selected={i === sel}
                image={b.k === "image" ? images.find((x) => x.id === b.id) : undefined}
                onSelect={() => setSel(i)}
                onText={(t) => edit((bs) => bs.map((x, k) => (k === i ? ({ ...x, t } as NoteBlock) : x)))}
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
                  setSel(i);
                  edit((bs) => [
                    ...bs.slice(0, i + 1),
                    // A new line inherits the look of the one it came from, so a
                    // checklist keeps making checkboxes.
                    (b.k === "check" ? { k: "check", t: "" } : { k: "text", t: "" }) as NoteBlock,
                    ...bs.slice(i + 1),
                  ]);
                  setSel(i + 1);
                }}
                onRemove={() => removeBlock(i)}
                registerRef={(el) => {
                  areaRefs.current[i] = el;
                }}
              />
            ))}
            <button
              type="button"
              onClick={() => {
                edit((bs) => [...bs, { k: "text", t: "" }]);
                setSel(blocks.length);
              }}
              className="mt-2 flex items-center gap-1.5 self-start rounded px-1 py-1 text-[12.5px] text-slate-400 transition hover:text-slate-700 dark:hover:text-slate-200"
            >
              <IconPlus className="h-3.5 w-3.5" /> Ďalší riadok
            </button>
          </div>
        )}
      </div>

      {/* ── sub-tabs, along the bottom like a spreadsheet ───────────── */}
      <div className="flex items-center gap-0.5 overflow-x-auto border-t border-slate-200 bg-surface-sunken px-2 pt-1 dark:border-slate-800">
        {(pages ?? []).map((p) => (
          <button
            key={p.id}
            type="button"
            onClick={() => void switchPage(p.id)}
            onDoubleClick={async () => {
              const name = window.prompt("Názov podkarty", p.name);
              if (name === null) return;
              try {
                const saved = await api.renameNotePage(p.id, name.trim());
                setPages((ps) => ps?.map((x) => (x.id === saved.id ? saved : x)) ?? ps);
                onNoteChanged();
              } catch (e) {
                toast.error(errMsg(e));
              }
            }}
            className={`group shrink-0 rounded-t-md border border-b-0 px-3 py-1.5 text-[12.5px] transition ${
              p.id === pageId
                ? "border-slate-200 border-t-2 border-t-brand-600 bg-surface font-semibold text-brand-700 dark:border-slate-700 dark:border-t-brand-500 dark:text-brand-300"
                : "border-transparent text-slate-500 hover:text-slate-800 dark:text-slate-400 dark:hover:text-slate-100"
            }`}
          >
            {p.name || "bez názvu"}
            {p.id === pageId && (pages?.length ?? 0) > 1 && (
              <span
                role="button"
                aria-label="Zmazať podkartu"
                title="Zmazať podkartu"
                onClick={async (e) => {
                  e.stopPropagation();
                  if (!window.confirm(`Zmazať podkartu „${p.name || "bez názvu"}"?`)) return;
                  try {
                    const left = await api.deleteNotePage(p.id);
                    setPages(left);
                    const first = left[0] ?? null;
                    setPageId(first?.id ?? null);
                    setBlocks(first?.blocks ?? []);
                    setDirty(false);
                    onNoteChanged();
                  } catch (err) {
                    toast.error(errMsg(err));
                  }
                }}
                className="ml-1.5 opacity-40 transition hover:opacity-100"
              >
                ×
              </span>
            )}
          </button>
        ))}
        <button type="button" onClick={addPage} title="Nová podkarta" aria-label="Nová podkarta"
          className="shrink-0 rounded px-2.5 py-1 text-base leading-none text-slate-500 transition hover:bg-surface hover:text-brand-600 dark:text-slate-400">
          +
        </button>
        <span className="ml-auto pr-2 text-[11px] text-slate-400 dark:text-slate-500">
          {dirty ? "Ukladám…" : "Uložené"}
        </span>
      </div>

      <LinkPicker open={linkOpen} noteId={note.id} onClose={() => setLinkOpen(false)} onLinked={onNoteChanged} />
    </div>
  );
}

/* ------------------------------------------------------------------ *
 * One block
 * ------------------------------------------------------------------ */

function BlockRow({
  block,
  selected,
  image,
  onSelect,
  onText,
  onToggleDone,
  onEnter,
  onRemove,
  registerRef,
}: {
  block: NoteBlock;
  selected: boolean;
  image?: NoteImage;
  onSelect: () => void;
  onText: (t: string) => void;
  onToggleDone: () => void;
  onEnter: () => void;
  onRemove: () => void;
  registerRef: (el: HTMLTextAreaElement | null) => void;
}) {
  const ring = selected ? "ring-1 ring-brand-500/40" : "";

  if (block.k === "rule") {
    return (
      <div onMouseDown={onSelect} className={`group flex items-center gap-2 rounded px-1 py-2 ${ring}`}>
        <hr className="flex-1 border-slate-200 dark:border-slate-700" />
        <RemoveDot onRemove={onRemove} />
      </div>
    );
  }

  if (block.k === "image") {
    return (
      <div onMouseDown={onSelect} className={`group flex items-start gap-2 rounded p-1 ${ring}`}>
        <figure className="max-w-[420px] overflow-hidden rounded-lg border border-slate-200 dark:border-slate-700">
          {image ? (
            <img src={image.dataUri} alt={block.t ?? ""} className="block w-full" />
          ) : (
            <div className="grid h-24 place-items-center text-xs text-slate-400">Obrázok sa nenašiel</div>
          )}
        </figure>
        <RemoveDot onRemove={onRemove} />
      </div>
    );
  }

  const size = SIZE_CLASS[block.s ?? "p"];
  const colour = block.c ? COLOUR_CLASS[block.c] : "text-slate-800 dark:text-slate-200";
  const marks = `${block.b ? " font-semibold" : ""}${block.i ? " italic" : ""}`;
  const done = block.k === "check" && block.d;

  return (
    <div onMouseDown={onSelect} className={`group flex items-start gap-2 rounded px-1 py-0.5 ${ring}`}>
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
      <textarea
        ref={registerRef}
        value={block.t}
        rows={1}
        onChange={(e) => {
          onText(e.target.value);
          // Grows with what is in it, so a long paragraph is never a 1-line
          // slot you have to scroll inside.
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
      <RemoveDot onRemove={onRemove} />
    </div>
  );
}

function RemoveDot({ onRemove }: { onRemove: () => void }) {
  return (
    <button
      type="button"
      onClick={onRemove}
      aria-label="Zmazať riadok"
      title="Zmazať riadok"
      className="mt-1 shrink-0 rounded p-0.5 text-transparent transition group-hover:text-slate-300 hover:!text-red-500 dark:group-hover:text-slate-600"
    >
      <IconX className="h-3.5 w-3.5" />
    </button>
  );
}

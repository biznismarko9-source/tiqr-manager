import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api, errMsg } from "../../lib/api";
import type { WorkspaceItem } from "../../lib/types";
import { Button, Input } from "../../components/ui";
import { IconArrowLeft, IconTrash } from "../../components/icons";
import { useToast } from "../../lib/toast";
import { formatDateTime } from "../../lib/format";

/**
 * One note, open for writing.
 *
 * 2.53.0, from marko's brief: "The content area should be large and
 * comfortable for writing", "The main goal is fast writing", "Do not build a
 * complicated text editor."
 *
 * So the writing surface is a plain textarea and stays one - no rich-text
 * model, no contentEditable, nothing that can lose a keystroke. The toolbar
 * inserts plain markers into that text (`**bold**`, `# heading`, `- item`,
 * `1. item`, `[ ] step`, `[label](url)`) and Preview renders them read-only.
 * What is stored is exactly what he typed, so a note is still readable as
 * plain text if this screen ever goes away.
 *
 * It saves itself, the same promise the tables make: a debounce after typing
 * stops, a flush on blur, and a flush on the way out. There is never a state
 * where something typed is not kept.
 *
 * The textarea is a bare `<textarea class="input">` rather than the shared
 * `Textarea`, only because that component is not a `forwardRef` and the
 * toolbar needs the element to place the caret. Same class, same look.
 */

const SAVE_DEBOUNCE_MS = 700;

/* ------------------------------------------------------------------ *
 * The small marker language
 * ------------------------------------------------------------------ */

type Block =
  | { kind: "h"; level: 1 | 2 | 3; text: string }
  | { kind: "bullet"; text: string }
  | { kind: "number"; index: number; text: string }
  | { kind: "check"; done: boolean; text: string; line: number }
  | { kind: "p"; text: string }
  | { kind: "blank" };

/** A checkbox is written as `[ ]` / `[x]`, and `☐` / `☑` are accepted too,
 *  because that is what marko's own brief wrote. A leading `- ` is optional. */
const CHECK_RE = /^\s*(?:[-*]\s*)?(?:\[( |x|X)\]|(☐|☑))\s?(.*)$/;

function isDone(marker: string | undefined, symbol: string | undefined): boolean {
  return marker === "x" || marker === "X" || symbol === "☑";
}

function parseBlocks(text: string): Block[] {
  const out: Block[] = [];
  let numbering = 0;
  text.split("\n").forEach((raw, line) => {
    const check = CHECK_RE.exec(raw);
    if (check) {
      numbering = 0;
      out.push({ kind: "check", done: isDone(check[1], check[2]), text: check[3] ?? "", line });
      return;
    }
    const heading = /^(#{1,3})\s+(.*)$/.exec(raw);
    if (heading) {
      numbering = 0;
      out.push({ kind: "h", level: heading[1].length as 1 | 2 | 3, text: heading[2] });
      return;
    }
    const bullet = /^\s*[-*]\s+(.*)$/.exec(raw);
    if (bullet) {
      numbering = 0;
      out.push({ kind: "bullet", text: bullet[1] });
      return;
    }
    const numbered = /^\s*\d+[.)]\s+(.*)$/.exec(raw);
    if (numbered) {
      numbering += 1;
      out.push({ kind: "number", index: numbering, text: numbered[1] });
      return;
    }
    numbering = 0;
    if (!raw.trim()) out.push({ kind: "blank" });
    else out.push({ kind: "p", text: raw });
  });
  return out;
}

/** Inline `**bold**`, `*italic*`, `[label](url)` and bare `https://` links.
 *  Anything that does not match stays exactly as it was typed. */
function Inline({ text }: { text: string }) {
  const parts: ReactNode[] = [];
  const re = /\*\*([^*]+)\*\*|\*([^*]+)\*|\[([^\]]+)\]\((https?:\/\/[^\s)]+)\)|(https?:\/\/[^\s]+)/g;
  let last = 0;
  let m: RegExpExecArray | null;
  let key = 0;
  while ((m = re.exec(text)) !== null) {
    if (m.index > last) parts.push(text.slice(last, m.index));
    if (m[1] !== undefined) {
      parts.push(<strong key={key++}>{m[1]}</strong>);
    } else if (m[2] !== undefined) {
      parts.push(<em key={key++}>{m[2]}</em>);
    } else {
      const href = m[4] ?? m[5] ?? "";
      const label = m[3] ?? m[5] ?? "";
      parts.push(
        // Opened in the system browser, never inside the app window.
        <button
          key={key++}
          type="button"
          onClick={() => void openUrl(href).catch(() => undefined)}
          className="text-brand-600 underline underline-offset-2 hover:text-brand-700 dark:text-brand-400 dark:hover:text-brand-300"
          title={href}
        >
          {label}
        </button>,
      );
    }
    last = m.index + m[0].length;
  }
  if (last < text.length) parts.push(text.slice(last));
  return <>{parts}</>;
}

function Preview({ text, onToggleCheck }: { text: string; onToggleCheck: (line: number) => void }) {
  const blocks = useMemo(() => parseBlocks(text), [text]);
  if (!text.trim()) {
    return <p className="text-sm text-slate-400 dark:text-slate-500">Nothing written yet.</p>;
  }
  return (
    <div className="flex flex-col gap-1 text-sm leading-relaxed text-slate-700 dark:text-slate-300">
      {blocks.map((b, i) => {
        if (b.kind === "blank") return <div key={i} className="h-2" />;
        if (b.kind === "h") {
          const size = b.level === 1 ? "text-lg" : b.level === 2 ? "text-base" : "text-sm";
          return (
            <h3 key={i} className={`${size} mt-2 font-semibold text-slate-900 dark:text-slate-50`}>
              <Inline text={b.text} />
            </h3>
          );
        }
        if (b.kind === "bullet") {
          return (
            <div key={i} className="flex gap-2 pl-1">
              <span className="text-slate-400 dark:text-slate-500">•</span>
              <span>
                <Inline text={b.text} />
              </span>
            </div>
          );
        }
        if (b.kind === "number") {
          return (
            <div key={i} className="flex gap-2 pl-1">
              <span className="tabular-nums text-slate-400 dark:text-slate-500">{b.index}.</span>
              <span>
                <Inline text={b.text} />
              </span>
            </div>
          );
        }
        if (b.kind === "check") {
          return (
            <label key={i} className="flex cursor-pointer items-start gap-2 pl-1">
              <input type="checkbox" checked={b.done} onChange={() => onToggleCheck(b.line)} className="mt-[3px]" />
              <span className={b.done ? "text-slate-400 line-through dark:text-slate-500" : ""}>
                <Inline text={b.text} />
              </span>
            </label>
          );
        }
        return (
          <p key={i}>
            <Inline text={b.text} />
          </p>
        );
      })}
    </div>
  );
}

/* ------------------------------------------------------------------ *
 * The editor
 * ------------------------------------------------------------------ */

type SaveState = "clean" | "dirty" | "saving";

export default function NoteEditor({
  note,
  onBack,
  onChanged,
  onDeleted,
}: {
  note: WorkspaceItem;
  onBack: () => void;
  onChanged: (item: WorkspaceItem) => void;
  onDeleted: (id: number) => void;
}) {
  const toast = useToast();
  const areaRef = useRef<HTMLTextAreaElement>(null);
  const [title, setTitle] = useState(note.title);
  const [content, setContent] = useState(note.content);
  const [tagText, setTagText] = useState(note.tags.join(", "));
  const [date, setDate] = useState(note.dueDate ?? "");
  const [pinned, setPinned] = useState(note.pinned);
  const [preview, setPreview] = useState(false);
  const [state, setState] = useState<SaveState>("clean");
  const [updatedAt, setUpdatedAt] = useState(note.updatedAt);
  const [confirmDelete, setConfirmDelete] = useState(false);

  // Whatever is on screen right now, readable from a timer or the unmount
  // handler without making `save` depend on every field.
  const draft = useRef({ title, content, tagText, date, pinned });
  draft.current = { title, content, tagText, date, pinned };

  const save = useCallback(async () => {
    const d = draft.current;
    setState("saving");
    try {
      const saved = await api.saveWorkspaceItem({
        id: note.id,
        kind: "note",
        title: d.title,
        content: d.content,
        category: note.category,
        tags: d.tagText
          .split(/[,\s]+/)
          .map((t) => t.trim())
          .filter(Boolean),
        // Records and tasks are gone from the UI in 2.53.0, but a row written
        // by 2.52.0 can still carry these. Passing them straight back means
        // opening a note never drops what is stored on it.
        fields: note.fields,
        checklist: note.checklist,
        status: null,
        dueDate: d.date.trim() || null,
        pinned: d.pinned,
        archived: note.archived,
      });
      setUpdatedAt(saved.updatedAt);
      setState("clean");
      onChanged(saved);
    } catch (e) {
      setState("dirty");
      toast.error(errMsg(e));
    }
  }, [note, onChanged, toast]);

  const saveRef = useRef(save);
  saveRef.current = save;
  const stateRef = useRef<SaveState>(state);
  stateRef.current = state;

  // Saves itself a moment after typing stops. Depends on `state` alone, so a
  // re-render from the parent cannot keep pushing the timer back.
  useEffect(() => {
    if (state !== "dirty") return;
    const t = setTimeout(() => void saveRef.current(), SAVE_DEBOUNCE_MS);
    return () => clearTimeout(t);
  }, [state]);

  // And on the way out, so leaving fast cannot lose the last few keystrokes.
  useEffect(
    () => () => {
      if (stateRef.current !== "clean") void saveRef.current();
    },
    [],
  );

  function touch() {
    setState("dirty");
  }

  /** Wraps or prefixes the selection, then puts the caret back where it
   *  belongs so typing continues without reaching for the mouse. */
  function apply(kind: "bold" | "italic" | "h" | "bullet" | "number" | "check" | "link") {
    const area = areaRef.current;
    if (!area) return;
    const start = area.selectionStart;
    const end = area.selectionEnd;
    const selected = content.slice(start, end);

    if (kind === "bold" || kind === "italic" || kind === "link") {
      const before = kind === "bold" ? "**" : kind === "italic" ? "*" : "[";
      const after = kind === "bold" ? "**" : kind === "italic" ? "*" : "](https://)";
      const body = selected || (kind === "link" ? "link" : kind);
      const next = `${content.slice(0, start)}${before}${body}${after}${content.slice(end)}`;
      setContent(next);
      touch();
      const caret = start + before.length;
      requestAnimationFrame(() => {
        area.focus();
        area.setSelectionRange(caret, caret + body.length);
      });
      return;
    }

    // Line markers apply to every line the selection touches.
    const prefix = kind === "h" ? "# " : kind === "bullet" ? "- " : "[ ] ";
    const lineStart = content.lastIndexOf("\n", start - 1) + 1;
    const lineEndRaw = content.indexOf("\n", end);
    const lineEnd = lineEndRaw === -1 ? content.length : lineEndRaw;
    const rewritten = content
      .slice(lineStart, lineEnd)
      .split("\n")
      .map((l, i) => (kind === "number" ? `${i + 1}. ${l}` : `${prefix}${l}`))
      .join("\n");
    const next = `${content.slice(0, lineStart)}${rewritten}${content.slice(lineEnd)}`;
    setContent(next);
    touch();
    const caret = lineStart + rewritten.length;
    requestAnimationFrame(() => {
      area.focus();
      area.setSelectionRange(caret, caret);
    });
  }

  /** Ticking a box in Preview writes back into the text, because the text is
   *  the only copy of it. */
  function toggleCheck(line: number) {
    const lines = content.split("\n");
    const raw = lines[line];
    if (raw === undefined) return;
    const m = CHECK_RE.exec(raw);
    if (!m) return;
    lines[line] = raw.replace(/\[( |x|X)\]|☐|☑/, isDone(m[1], m[2]) ? "[ ]" : "[x]");
    setContent(lines.join("\n"));
    touch();
  }

  const saveLabel = state === "saving" ? "Saving…" : state === "dirty" ? "Unsaved" : "Saved";

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-center gap-2">
        <Button variant="secondary" onClick={onBack}>
          <IconArrowLeft className="h-4 w-4" /> Notes
        </Button>
        <button
          type="button"
          onClick={() => {
            setPinned((p) => !p);
            touch();
          }}
          title={pinned ? "Unpin" : "Pin to the top"}
          aria-label={pinned ? "Unpin" : "Pin to the top"}
          className={`rounded-lg px-2 py-1.5 text-base leading-none transition ${
            pinned ? "text-amber-500 hover:text-amber-600" : "text-slate-300 hover:text-amber-500 dark:text-slate-600"
          }`}
        >
          {pinned ? "★" : "☆"}
        </button>
        <span className="ml-auto text-xs text-slate-500 dark:text-slate-400">{saveLabel}</span>
        <Button variant="secondary" onClick={() => setPreview((p) => !p)}>
          {preview ? "Edit" : "Preview"}
        </Button>
        <Button variant="secondary" onClick={() => setConfirmDelete(true)}>
          <IconTrash className="h-4 w-4" /> Delete
        </Button>
      </div>

      <Input
        value={title}
        onChange={(e) => {
          setTitle(e.target.value);
          touch();
        }}
        placeholder="Title"
        aria-label="Title"
        className="text-lg font-semibold"
      />

      <div className="grid grid-cols-1 gap-3 sm:grid-cols-[180px_1fr]">
        <label>
          <span className="label">Date</span>
          <Input
            type="date"
            value={date}
            onChange={(e) => {
              setDate(e.target.value);
              touch();
            }}
          />
        </label>
        <label>
          <span className="label">Tags</span>
          <Input
            value={tagText}
            onChange={(e) => {
              setTagText(e.target.value);
              touch();
            }}
            placeholder="oasis, accounts, important"
          />
        </label>
      </div>

      {!preview && (
        <div className="flex flex-wrap gap-1">
          {(
            [
              ["h", "H"],
              ["bold", "B"],
              ["italic", "I"],
              ["bullet", "•"],
              ["number", "1."],
              ["check", "☐"],
              ["link", "Link"],
            ] as const
          ).map(([kind, label]) => (
            <button
              key={kind}
              type="button"
              onClick={() => apply(kind)}
              title={kind}
              className={`min-w-[32px] rounded-md border border-slate-200 px-2 py-1 text-xs text-slate-600 transition hover:bg-surface-sunken dark:border-slate-700 dark:text-slate-300 ${
                kind === "bold" ? "font-bold" : kind === "italic" ? "italic" : ""
              }`}
            >
              {label}
            </button>
          ))}
        </div>
      )}

      {preview ? (
        <div className="min-h-[420px] rounded-xl border border-slate-200 p-4 dark:border-slate-700">
          <Preview text={content} onToggleCheck={toggleCheck} />
        </div>
      ) : (
        <textarea
          ref={areaRef}
          value={content}
          onChange={(e) => {
            setContent(e.target.value);
            touch();
          }}
          onBlur={() => {
            if (stateRef.current === "dirty") void save();
          }}
          rows={20}
          placeholder={"25/09/2026\n\nOasis codes\n\nJohn123 — 4 codes\nABC123\n\n[ ] Follow up 28/09"}
          className="input min-h-[420px] leading-relaxed"
          aria-label="Content"
        />
      )}

      <p className="text-xs text-slate-500 dark:text-slate-400">
        Created {formatDateTime(note.createdAt)} · Last updated {formatDateTime(updatedAt)}
      </p>

      {confirmDelete && (
        <div className="rounded-xl border border-red-200 bg-red-50 p-4 dark:border-red-900 dark:bg-red-950/40">
          <p className="mb-3 text-sm text-red-800 dark:text-red-300">
            Delete this note? It goes from this computer and the other one, and cannot be undone.
          </p>
          <div className="flex gap-2">
            <Button variant="secondary" onClick={() => setConfirmDelete(false)}>
              Cancel
            </Button>
            <Button
              variant="danger"
              onClick={async () => {
                try {
                  // Nothing may be written after the row is gone.
                  setState("clean");
                  stateRef.current = "clean";
                  await api.deleteWorkspaceItem(note.id);
                  onDeleted(note.id);
                } catch (e) {
                  toast.error(errMsg(e));
                }
              }}
            >
              <IconTrash className="h-4 w-4" /> Delete note
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}

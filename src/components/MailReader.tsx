// 2.113.0 - the mail reader: a message the way it was designed, safely.
//
// marko: "kusok pomenit ako vyzera ten obsah mailu nech len nevypisuje mail
// ale aj ho pekne zobrazi", "linky ktore su v maily nech su klikatelne a
// ukazuju sa tak ako v maily", "aby som to vedel aj jednym klikom
// skopirovat", "OTP kody nech sa skopiruju na klik", "pri kazdom maile kde je
// nejaky attachment tak ten tiez dat do tiqr aby sme si ho vedeli kuknut
// normalne aj ho tam otvorit bez problemov", "built in translator", and
// "aby som vedel dane maily presunut tak kde chcem".
//
// THE RULE OVER ALL OF IT: "e-mailovy obsah a prilohy treba spracuvat ako
// nedoveryhodny vstup". Two walls stand between a message and this app:
//
//   1. Rust cleans the HTML before the screen ever sees it (no script, no
//      handler, no form, no frame, links http/https/mailto only) and copies
//      its pictures into TIQR (commands/mail_view.rs).
//   2. The cleaned HTML is shown ONLY inside an iframe that may run no script
//      (`sandbox` without allow-scripts) and whose page may load nothing
//      (lib/mailText.ts::readerDocument). It is never put into this page.
//
// A click on a link is caught here and opened in the browser - after asking,
// when the text of the link names a different site than the one it opens,
// or when the message is in Spam or Trash.
//
// 2.113.1: a link that is a file ("Download receipt", "Download invoice") is
// downloaded INTO TIQR on the click and kept with the message like an
// attachment - "realne ked na to kliknem tak sa mi stiahne". And the key
// details of the message (where the tickets go, to whom, the price, by when)
// sit at the top, a person's details hidden until clicked.
import { Fragment, memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api, errMsg } from "../lib/api";
import { copyText } from "../lib/clipboard";
import {
  fileSize,
  hostOf,
  isDownloadLink,
  languageName,
  linkify,
  previewType,
  readerDocument,
  shownElsewhere,
  textBlocks,
} from "../lib/mailText";
import { useToast } from "../lib/toast";
import type {
  MailAiResult,
  MailAttachmentInfo,
  MailConnectionStatus,
  MailFact,
  MailMessageSummary,
  MailProposal,
  MailView,
} from "../lib/types";
import { SendToDiscordButton } from "./Discord";
import {
  IconAlertTriangle,
  IconChevronDown,
  IconCopy,
  IconDownload,
  IconExternalLink,
  IconEye,
  IconEyeOff,
  IconFolder,
  IconLanguages,
  IconPaperclip,
  IconRefresh,
  IconSparkles,
  IconTrash,
  IconX,
} from "./icons";
import { MailProposalCard } from "./MailProposal";
import { Badge, Button, ConfirmDialog, Modal, Spinner } from "./ui";

/** What changed in the reader: a field of this message (patched in place,
 *  so the list does not jump), or `null` for "something bigger - reload". */
export type ReaderChange = Partial<MailMessageSummary> | null;

/** Somewhere a message can be moved to. */
export interface MailPlace {
  key: string;
  label: string;
  group: "TIQR" | "Your labels" | "Your folders";
}

const CATEGORY_LABELS: Record<string, string> = {
  sale_confirmation: "Sale",
  purchase_confirmation: "Purchase",
  payout: "Payout",
  refund: "Refund",
  cancellation: "Cancellation",
  event_update: "Event update",
  order_status: "Order status",
  account_notice: "Account notice",
  other: "Other",
};

export const categoryLabel = (c: string) => CATEGORY_LABELS[c] ?? c;
const has = (m: MailMessageSummary, l: string) => m.labels.split(",").includes(l);
export const isJunk = (m: MailMessageSummary) => has(m, "spam") || has(m, "trash");

/** Only a web address or an e-mail address is ever opened. */
const openable = (href: string) => /^(https?:\/\/|mailto:)/i.test(href.trim());

/* ---------------------------------------------------------------------------
   Move to...
   --------------------------------------------------------------------------- */

/** A small menu of every place a message can go. Used by the reader and by
 *  the list's selection bar. `current` is where the message is now (so it is
 *  not offered); `moved` adds "back to where Gmail has it". */
export function MailMoveMenu({
  places,
  current,
  moved,
  align = "left",
  onPick,
}: {
  places: MailPlace[];
  current?: string | null;
  moved?: boolean;
  align?: "left" | "right";
  onPick: (target: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!open) return;
    const close = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", close);
    document.addEventListener("keydown", key);
    return () => {
      document.removeEventListener("mousedown", close);
      document.removeEventListener("keydown", key);
    };
  }, [open]);

  const groups = (["TIQR", "Your labels", "Your folders"] as const)
    .map((g) => ({ g, items: places.filter((p) => p.group === g && p.key !== current) }))
    .filter((x) => x.items.length > 0);
  const pick = (t: string) => {
    setOpen(false);
    onPick(t);
  };

  return (
    <div ref={ref} className="relative">
      <Button variant="secondary" size="sm" onClick={() => setOpen((v) => !v)} aria-expanded={open}>
        <IconFolder className="h-3.5 w-3.5" />
        Move to
        <IconChevronDown className="h-3 w-3" />
      </Button>
      {open && (
        <div
          role="menu"
          className={`absolute top-full z-30 mt-1 max-h-[22rem] w-60 overflow-y-auto rounded-lg bg-surface-raised p-1 shadow-overlay animate-[pop-in_.14s_ease-out] ${
            align === "right" ? "right-0" : "left-0"
          }`}
        >
          {moved && (
            <button
              type="button"
              role="menuitem"
              className="mb-1 w-full rounded-md px-2.5 py-1.5 text-left text-xs font-medium text-brand-700 hover:bg-surface-muted dark:text-brand-300"
              onClick={() => pick("")}
            >
              Back to where Gmail has it
            </button>
          )}
          {groups.map(({ g, items }) => (
            <div key={g} className="py-0.5">
              <p className="px-2.5 pb-0.5 pt-1.5 text-[10px] font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">
                {g}
              </p>
              {items.map((p) => (
                <button
                  key={p.key}
                  type="button"
                  role="menuitem"
                  className="w-full truncate rounded-md px-2.5 py-1.5 text-left text-xs text-slate-700 hover:bg-surface-muted dark:text-slate-200"
                  onClick={() => pick(p.key)}
                >
                  {p.label}
                </button>
              ))}
            </div>
          ))}
          <p className="border-t border-slate-200 px-2.5 pb-1 pt-1.5 text-[10.5px] leading-snug text-slate-400 dark:border-slate-800 dark:text-slate-500">
            Moved in TIQR only - Gmail is never changed.
          </p>
        </div>
      )}
    </div>
  );
}

/* ---------------------------------------------------------------------------
   Links: the hover chip with one-click copy.
   --------------------------------------------------------------------------- */

interface ChipState {
  href: string;
  text: string;
  /** The link's box, in this window's coordinates. */
  left: number;
  top: number;
  bottom: number;
}

function LinkChip({
  chip,
  canDownload,
  onOpen,
  onDownload,
  onCopy,
  onEnter,
  onLeave,
}: {
  chip: ChipState;
  /** False for mail in Spam or Trash, whose files TIQR never downloads. */
  canDownload: boolean;
  onOpen: () => void;
  onDownload: () => void;
  onCopy: () => void;
  onEnter: () => void;
  onLeave: () => void;
}) {
  const mail = chip.href.toLowerCase().startsWith("mailto:");
  const where = mail ? chip.href.slice(7) : (hostOf(chip.href) ?? chip.href);
  const shown = shownElsewhere(chip.text, chip.href);
  const file = canDownload && !mail && isDownloadLink(chip.text, chip.href);
  const width = file ? 400 : 360;
  const left = Math.max(8, Math.min(chip.left, window.innerWidth - width - 8));
  const below = chip.bottom + 40 < window.innerHeight;
  return (
    <div
      onMouseEnter={onEnter}
      onMouseLeave={onLeave}
      style={{ position: "fixed", left, top: below ? chip.bottom + 4 : chip.top - 36, maxWidth: width }}
      className="z-40 flex items-center gap-1 rounded-lg bg-slate-900 py-1 pl-2.5 pr-1 text-[11px] text-white shadow-overlay animate-[fadein_.12s_ease-out] dark:bg-slate-700"
    >
      {shown && <IconAlertTriangle className="h-3.5 w-3.5 shrink-0 text-amber-300" />}
      <span className="min-w-0 truncate font-mono" title={chip.href}>
        {shown ? `Opens ${where}, not ${shown}` : where}
      </span>
      <button
        type="button"
        className="ml-1 inline-flex shrink-0 items-center gap-1 rounded-md px-1.5 py-0.5 font-medium hover:bg-white/15"
        onClick={onCopy}
      >
        <IconCopy className="h-3 w-3" />
        {mail ? "Copy address" : "Copy link"}
      </button>
      {file && (
        <button
          type="button"
          title="Download the file into TIQR"
          className="inline-flex shrink-0 items-center gap-1 rounded-md px-1.5 py-0.5 font-medium hover:bg-white/15"
          onClick={onDownload}
        >
          <IconDownload className="h-3 w-3" />
          Download
        </button>
      )}
      <button
        type="button"
        title={file ? "Open it in the browser instead" : undefined}
        className="inline-flex shrink-0 items-center gap-1 rounded-md px-1.5 py-0.5 font-medium hover:bg-white/15"
        onClick={onOpen}
      >
        <IconExternalLink className="h-3 w-3" />
        Open
      </button>
    </div>
  );
}

/* ---------------------------------------------------------------------------
   The designed message, inside its sandbox.
   --------------------------------------------------------------------------- */

/** Wraps each whole-word copy of the code in a click-to-copy mark - inside
 *  the message's own text, so the code is copyable where marko sees it. */
function markCode(doc: Document, code: string) {
  if (!doc.body || code.length < 4) return;
  const walker = doc.createTreeWalker(doc.body, 4 /* NodeFilter.SHOW_TEXT */);
  const hits: Text[] = [];
  while (walker.nextNode() && hits.length < 8) {
    const n = walker.currentNode as Text;
    if (n.data.includes(code) && !n.parentElement?.closest("style")) hits.push(n);
  }
  for (const n of hits) {
    const i = n.data.indexOf(code);
    const before = n.data[i - 1];
    const after = n.data[i + code.length];
    if ((before && /[\w-]/.test(before)) || (after && /[\w-]/.test(after))) continue;
    const mid = n.splitText(i);
    mid.splitText(code.length);
    const mark = doc.createElement("span");
    mark.className = "tiqr-otp";
    mark.title = "Click to copy";
    mark.textContent = code;
    mid.parentNode?.replaceChild(mark, mid);
  }
}

const MailHtmlFrame = memo(function MailHtmlFrame({
  html,
  code,
  onLink,
  onHover,
  onHoverEnd,
  onCode,
}: {
  html: string;
  code: string | null;
  onLink: (href: string, text: string) => void;
  onHover: (chip: ChipState) => void;
  onHoverEnd: () => void;
  onCode: (code: string) => void;
}) {
  const ref = useRef<HTMLIFrameElement>(null);
  const [height, setHeight] = useState(240);
  const cleanup = useRef<(() => void) | null>(null);
  const doc = useMemo(() => readerDocument(html), [html]);
  // The handlers change identity with the reader; the frame keeps one set.
  const handlers = useRef({ onLink, onHover, onHoverEnd, onCode });
  handlers.current = { onLink, onHover, onHoverEnd, onCode };

  useEffect(() => () => cleanup.current?.(), []);

  const setup = () => {
    cleanup.current?.();
    const frame = ref.current;
    const d = frame?.contentDocument;
    if (!frame || !d || !d.body) return;

    /* As tall as the message, so the reader scrolls as one page. The body's
       own box, and the page's overflow only when there is some - so a mail
       whose body is "height: 100%" cannot make the frame grow forever. */
    const measure = () => {
      const root = d.documentElement;
      const bodyH = Math.ceil(d.body.getBoundingClientRect().height);
      const overflow = root.scrollHeight > root.clientHeight + 1 ? root.scrollHeight : 0;
      const h = Math.min(Math.max(bodyH, overflow, 60), 60_000);
      setHeight((prev) => (Math.abs(prev - h) > 1 ? h : prev));
    };
    measure();
    const ro = new ResizeObserver(measure);
    ro.observe(d.body);
    const timers = [80, 400, 1500].map((ms) => window.setTimeout(measure, ms));

    if (code) markCode(d, code);

    const anchorOf = (t: EventTarget | null) =>
      (t as Element | null)?.closest?.("a[href]") as HTMLAnchorElement | null | undefined;
    const onClick = (e: MouseEvent) => {
      const mark = (e.target as Element | null)?.closest?.(".tiqr-otp");
      if (mark && code) {
        e.preventDefault();
        mark.classList.add("tiqr-copied");
        handlers.current.onCode(code);
        return;
      }
      const a = anchorOf(e.target);
      if (!a) return;
      e.preventDefault();
      e.stopPropagation();
      handlers.current.onLink(a.getAttribute("href") ?? "", a.textContent ?? "");
    };
    const block = (e: Event) => {
      if (anchorOf(e.target)) e.preventDefault();
    };
    const onOver = (e: MouseEvent) => {
      const a = anchorOf(e.target);
      if (!a) return;
      const r = a.getBoundingClientRect();
      const f = frame.getBoundingClientRect();
      handlers.current.onHover({
        href: a.getAttribute("href") ?? "",
        text: a.textContent ?? "",
        left: f.left + r.left,
        top: f.top + r.top,
        bottom: f.top + r.bottom,
      });
    };
    const onOut = (e: MouseEvent) => {
      const a = anchorOf(e.target);
      if (a && !(e.relatedTarget && a.contains(e.relatedTarget as Node))) handlers.current.onHoverEnd();
    };
    d.addEventListener("click", onClick, true);
    d.addEventListener("auxclick", block, true);
    d.addEventListener("dragstart", block, true);
    d.addEventListener("mouseover", onOver);
    d.addEventListener("mouseout", onOut);
    cleanup.current = () => {
      ro.disconnect();
      timers.forEach((t) => window.clearTimeout(t));
      d.removeEventListener("click", onClick, true);
      d.removeEventListener("auxclick", block, true);
      d.removeEventListener("dragstart", block, true);
      d.removeEventListener("mouseover", onOver);
      d.removeEventListener("mouseout", onOut);
      cleanup.current = null;
    };
  };

  return (
    <iframe
      ref={ref}
      title="Message"
      // No allow-scripts: nothing in the message can ever run. allow-same-
      // origin only lets THIS page measure it and catch its link clicks.
      sandbox="allow-same-origin"
      referrerPolicy="no-referrer"
      srcDoc={doc}
      onLoad={setup}
      style={{ height }}
      className="block w-full border-0 bg-white"
    />
  );
});

/* ---------------------------------------------------------------------------
   A text-only message, set as a mail program sets it.
   --------------------------------------------------------------------------- */

function TextLine({
  line,
  code,
  onLink,
  onHover,
  onHoverEnd,
  onCode,
}: {
  line: string;
  code: string | null;
  onLink: (href: string, text: string) => void;
  onHover: (chip: ChipState) => void;
  onHoverEnd: () => void;
  onCode: (code: string) => void;
}) {
  return (
    <>
      {linkify(line, code).map((p, i) => {
        if (p.kind === "text") return <span key={i}>{p.text}</span>;
        if (p.kind === "code") {
          return (
            <button
              key={i}
              type="button"
              title="Click to copy"
              onClick={() => onCode(p.text)}
              className="rounded bg-brand-500/15 px-1 font-semibold text-brand-700 ring-1 ring-brand-500/40 hover:bg-brand-500/25 dark:text-brand-300"
            >
              {p.text}
            </button>
          );
        }
        return (
          <a
            key={i}
            href={p.href}
            rel="noreferrer"
            className="break-all text-brand-600 underline decoration-brand-500/40 underline-offset-2 hover:decoration-brand-500 dark:text-brand-400"
            onClick={(e) => {
              e.preventDefault();
              onLink(p.href, p.text);
            }}
            onAuxClick={(e) => e.preventDefault()}
            onMouseEnter={(e) => {
              const r = e.currentTarget.getBoundingClientRect();
              onHover({ href: p.href, text: p.text, left: r.left, top: r.top, bottom: r.bottom });
            }}
            onMouseLeave={onHoverEnd}
          >
            {p.text}
          </a>
        );
      })}
    </>
  );
}

export function MailText({
  text,
  code = null,
  onLink,
  onHover,
  onHoverEnd,
  onCode,
}: {
  text: string;
  code?: string | null;
  onLink: (href: string, text: string) => void;
  onHover: (chip: ChipState) => void;
  onHoverEnd: () => void;
  onCode: (code: string) => void;
}) {
  const blocks = useMemo(() => textBlocks(text), [text]);
  const lineProps = { code, onLink, onHover, onHoverEnd, onCode };
  return (
    <div className="space-y-3 text-sm leading-relaxed text-slate-700 dark:text-slate-300">
      {blocks.map((b, bi) => {
        const lines = b.lines.map((l, li) => (
          <span key={li}>
            {li > 0 && "\n"}
            <TextLine line={l} {...lineProps} />
          </span>
        ));
        return b.kind === "quote" ? (
          <blockquote
            key={bi}
            className="whitespace-pre-wrap break-words border-l-2 border-slate-300 pl-3 text-slate-500 dark:border-slate-600 dark:text-slate-400"
          >
            {lines}
          </blockquote>
        ) : (
          <p key={bi} className="whitespace-pre-wrap break-words">
            {lines}
          </p>
        );
      })}
    </div>
  );
}

/* ---------------------------------------------------------------------------
   Attachments.
   --------------------------------------------------------------------------- */

type Preview =
  | { kind: "image"; url: string }
  | { kind: "pdf"; url: string }
  | { kind: "text"; text: string }
  | { kind: "error"; message: string };

function AttachmentPreview({ messageId, att, onClose }: { messageId: number; att: MailAttachmentInfo; onClose: () => void }) {
  const [preview, setPreview] = useState<Preview | null>(null);
  useEffect(() => {
    let url: string | null = null;
    let alive = true;
    api
      .readMailAttachment(messageId, att.partId)
      .then((buf) => {
        const bytes = new Uint8Array(buf);
        const t = previewType(bytes);
        if (!alive) return;
        if (t.kind === "text") {
          // As TEXT, always - an .html or .svg attachment is shown as its
          // source, never drawn.
          setPreview({ kind: "text", text: new TextDecoder("utf-8").decode(bytes.subarray(0, 400_000)) });
          return;
        }
        const blob = URL.createObjectURL(new Blob([bytes], { type: t.mime }));
        url = blob;
        setPreview(t.kind === "pdf" ? { kind: "pdf", url: blob } : { kind: "image", url: blob });
      })
      .catch((e) => alive && setPreview({ kind: "error", message: errMsg(e) }));
    return () => {
      alive = false;
      if (url) URL.revokeObjectURL(url);
    };
  }, [messageId, att.partId]);

  return (
    <Modal open onClose={onClose} title={att.filename} width="max-w-4xl">
      {!preview ? (
        <div className="flex items-center gap-2 text-sm text-slate-500 dark:text-slate-400">
          <Spinner className="h-4 w-4" /> Loading...
        </div>
      ) : preview.kind === "error" ? (
        <p className="text-sm text-red-600 dark:text-red-400">{preview.message}</p>
      ) : preview.kind === "image" ? (
        <div className="flex justify-center rounded-lg bg-surface-muted p-2">
          <img src={preview.url} alt={att.filename} className="max-h-[70vh] max-w-full object-contain" />
        </div>
      ) : preview.kind === "pdf" ? (
        <iframe src={preview.url} title={att.filename} className="h-[70vh] w-full rounded-lg bg-white" />
      ) : (
        <pre className="max-h-[70vh] overflow-auto whitespace-pre-wrap break-words rounded-lg bg-surface-muted p-3 font-mono text-xs text-slate-700 dark:text-slate-200">
          {preview.text}
        </pre>
      )}
    </Modal>
  );
}

function AttachmentList({
  messageId,
  items,
  onView,
  onUpdated,
}: {
  messageId: number;
  items: MailAttachmentInfo[];
  onView: (att: MailAttachmentInfo) => void;
  onUpdated: () => void;
}) {
  const toast = useToast();
  const [busy, setBusy] = useState<string | null>(null);

  const run = async (att: MailAttachmentInfo, what: "open" | "save") => {
    setBusy(att.partId);
    try {
      if (what === "open") await api.openMailAttachment(messageId, att.partId);
      else {
        const name = await api.saveMailAttachment(messageId, att.partId);
        if (name) toast.success(`Saved ${name}`);
      }
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setBusy(null);
      onUpdated();
    }
  };

  return (
    <div className="mt-4">
      <p className="section-title mb-1.5 flex items-center gap-1.5">
        <IconPaperclip className="h-3.5 w-3.5" />
        {items.length} attachment{items.length === 1 ? "" : "s"}
      </p>
      <ul className="grid gap-2 sm:grid-cols-2">
        {items.map((a) => (
          <li key={a.partId} className="rounded-lg border border-slate-200 bg-surface px-3 py-2 dark:border-slate-800">
            <div className="flex items-center gap-2">
              <span className="min-w-0 flex-1">
                <span className="block truncate text-xs font-medium text-slate-800 dark:text-slate-100" title={a.filename}>
                  {a.filename}
                </span>
                <span className="text-[11px] text-slate-500 dark:text-slate-400">
                  {[
                    fileSize(a.size),
                    a.kind && a.kind !== "other" ? a.kind.toUpperCase() : null,
                    // 2.113.1: downloaded from a link in the message.
                    a.partId.startsWith("L") ? "from a link" : null,
                  ]
                    .filter(Boolean)
                    .join(" · ")}
                </span>
              </span>
              {busy === a.partId && <Spinner className="h-3.5 w-3.5" />}
            </div>
            {a.warning && (
              <p className="mt-1 flex gap-1 text-[11px] leading-snug text-amber-700 dark:text-amber-400">
                <IconAlertTriangle className="mt-px h-3 w-3 shrink-0" />
                {a.warning}
              </p>
            )}
            <div className="mt-1.5 flex flex-wrap gap-1">
              {a.canPreview && (
                <Button size="sm" variant="ghost" disabled={!!busy} onClick={() => onView(a)}>
                  <IconEye className="h-3.5 w-3.5" /> View
                </Button>
              )}
              {a.canOpen && (
                <Button size="sm" variant="ghost" disabled={!!busy} onClick={() => void run(a, "open")}>
                  <IconExternalLink className="h-3.5 w-3.5" /> Open
                </Button>
              )}
              <Button size="sm" variant="ghost" disabled={!!busy} onClick={() => void run(a, "save")}>
                <IconDownload className="h-3.5 w-3.5" /> Save
              </Button>
            </div>
          </li>
        ))}
      </ul>
    </div>
  );
}

/* ---------------------------------------------------------------------------
   Key details (2.113.1).
   --------------------------------------------------------------------------- */

/** The few lines that matter, from the message's own text
 *  (commands/mail_facts.rs). marko: "poslat tam, meno a mail a to daj ako
 *  hidden a ked na to kliknes tak sa to zobrazi a aj cena". A person's
 *  details show as dots until clicked; any line copies with one click,
 *  hidden or not. */
function KeyDetails({ facts, onCopy }: { facts: MailFact[]; onCopy: (text: string, what: string) => void }) {
  const [shown, setShown] = useState<Set<number>>(() => new Set());
  const hidable = facts.flatMap((f, i) => (f.private ? [i] : []));
  const allShown = hidable.length > 0 && hidable.every((i) => shown.has(i));
  const toggle = (i: number) =>
    setShown((prev) => {
      const next = new Set(prev);
      if (next.has(i)) next.delete(i);
      else next.add(i);
      return next;
    });

  return (
    <div className="mt-3 rounded-xl bg-surface-muted px-4 py-3">
      <div className="flex items-center gap-2">
        <p className="section-title">Key details</p>
        {hidable.length > 0 && (
          <button
            type="button"
            className="ml-auto inline-flex items-center gap-1 text-[11px] font-medium text-brand-600 hover:underline dark:text-brand-400"
            onClick={() => setShown(allShown ? new Set() : new Set(hidable))}
          >
            {allShown ? <IconEyeOff className="h-3 w-3" /> : <IconEye className="h-3 w-3" />}
            {allShown ? "Hide personal details" : "Show personal details"}
          </button>
        )}
      </div>
      <dl className="mt-1.5 grid grid-cols-[minmax(5.5rem,max-content)_minmax(0,1fr)] items-center gap-x-4 gap-y-1 text-xs">
        {facts.map((f, i) => {
          const hidden = f.private && !shown.has(i);
          return (
            <Fragment key={`${f.label}-${i}`}>
              <dt className="truncate text-slate-500 dark:text-slate-400" title={f.label}>
                {f.label}
              </dt>
              <dd className="group flex min-w-0 items-center gap-1.5">
                {hidden ? (
                  <button
                    type="button"
                    onClick={() => toggle(i)}
                    title="Click to show"
                    className="-ml-2 inline-flex items-center gap-1.5 rounded-md bg-slate-200/80 px-2 py-0.5 text-slate-500 transition hover:bg-slate-300/80 dark:bg-slate-700/70 dark:text-slate-400 dark:hover:bg-slate-600/70"
                  >
                    <span aria-hidden="true" className="tracking-[0.18em]">
                      ••••••••
                    </span>
                    <span className="text-[10.5px] font-medium">Show</span>
                  </button>
                ) : f.private ? (
                  <button
                    type="button"
                    onClick={() => toggle(i)}
                    title="Click to hide"
                    className="-ml-2 min-w-0 break-words rounded-md px-2 py-0.5 text-left text-slate-800 transition hover:bg-slate-200/60 dark:text-slate-100 dark:hover:bg-slate-700/50"
                  >
                    {f.value}
                  </button>
                ) : (
                  <span
                    className={`min-w-0 break-words py-0.5 text-slate-800 dark:text-slate-100 ${f.kind === "money" ? "num font-semibold" : ""}`}
                  >
                    {f.value}
                  </span>
                )}
                <button
                  type="button"
                  aria-label={`Copy ${f.label}`}
                  title="Copy"
                  className="shrink-0 rounded p-0.5 text-slate-400 opacity-0 transition hover:text-slate-700 focus-visible:opacity-100 group-hover:opacity-100 dark:hover:text-slate-200"
                  onClick={() => onCopy(f.value, f.label)}
                >
                  <IconCopy className="h-3 w-3" />
                </button>
              </dd>
            </Fragment>
          );
        })}
      </dl>
      {hidable.length > 0 && (
        <p className="mt-2 text-[10.5px] text-slate-400 dark:text-slate-500">
          A person&apos;s details stay hidden until you click them - in Discord too, as a spoiler.
        </p>
      )}
    </div>
  );
}

/* ---------------------------------------------------------------------------
   The reader.
   --------------------------------------------------------------------------- */

export function MailReader({
  message,
  status,
  places,
  placeName,
  onMove,
  onTrash,
  onChanged,
}: {
  message: MailMessageSummary;
  status: MailConnectionStatus;
  places: MailPlace[];
  /** The name of the place a message was moved to, for its badge. */
  placeName: (key: string) => string | null;
  onMove: (ids: number[], target: string) => void;
  onTrash: (ids: number[], restore: boolean) => void;
  onChanged: (change: ReaderChange) => void;
}) {
  const toast = useToast();
  const [view, setView] = useState<MailView | null>(null);
  const [loading, setLoading] = useState(true);
  const [failed, setFailed] = useState<string | null>(null);
  const [showHeaders, setShowHeaders] = useState(false);
  const [proposal, setProposal] = useState<MailProposal | null>(null);
  const [ai, setAi] = useState<MailAiResult | null>(null);
  const [aiBusy, setAiBusy] = useState<"translate" | "summary" | null>(null);
  const [chip, setChip] = useState<ChipState | null>(null);
  const [confirmLink, setConfirmLink] = useState<{ href: string; shown: string | null; download: boolean } | null>(null);
  const [busy, setBusy] = useState(false);
  const [previewing, setPreviewing] = useState<MailAttachmentInfo | null>(null);
  /** The link whose file is being downloaded into TIQR. */
  const [downloading, setDownloading] = useState<string | null>(null);
  const hideTimer = useRef<number | null>(null);
  const currentId = useRef(message.id);
  currentId.current = message.id;

  const junk = isJunk(message);
  const code = message.otpCode ?? null;

  const load = useCallback(
    (refresh = false) => {
      setLoading(true);
      setFailed(null);
      api
        .getMailView(message.id, refresh)
        .then((v) => {
          setView(v);
          // Opening marks it read, in TIQR only - and is when TIQR first
          // learns whether it carries files.
          const change: Partial<MailMessageSummary> = {};
          if (message.isUnread) change.isUnread = false;
          if (!message.hasAttachments && v.attachments.some((a) => !a.inline)) change.hasAttachments = true;
          if (Object.keys(change).length > 0) onChanged(change);
        })
        .catch((e) => setFailed(errMsg(e)))
        .finally(() => setLoading(false));
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [message.id],
  );

  useEffect(() => {
    setView(null);
    setShowHeaders(false);
    setProposal(null);
    setAi(null);
    setChip(null);
    setPreviewing(null);
    load();
    api
      .getMailProposal(message.id)
      .then(setProposal)
      .catch(() => setProposal(null));
  }, [message.id, load]);

  const refreshAttachments = useCallback(() => {
    api
      .getMailView(message.id)
      .then((v) => setView((prev) => (prev ? { ...prev, attachments: v.attachments } : v)))
      .catch(() => undefined);
  }, [message.id]);

  /* ---- links and codes ------------------------------------------------ */

  const openNow = async (href: string) => {
    try {
      await openUrl(href.trim());
    } catch (e) {
      toast.error(errMsg(e));
    }
  };
  /** A link that is a file, downloaded into TIQR and kept with the message.
   *  A web page - or a file TIQR could not get (a sign-in in front of it) -
   *  opens in the browser instead, so a click always gets marko somewhere. */
  const downloadLink = async (href: string) => {
    const id = message.id;
    if (downloading === href) return;
    setDownloading(href);
    try {
      const got = await api.downloadMailLink(id, href);
      if (currentId.current !== id) return;
      if (got.outcome !== "file" || !got.partId) {
        await openNow(href);
        return;
      }
      const v = await api.getMailView(id);
      if (currentId.current !== id) return;
      setView((prev) => (prev ? { ...prev, attachments: v.attachments } : v));
      if (!message.hasAttachments) onChanged({ hasAttachments: true });
      const att = v.attachments.find((a) => a.partId === got.partId);
      if (att?.canPreview) setPreviewing(att);
      else toast.success(`${got.filename ?? "The file"} is in TIQR, with the attachments.`);
    } catch (e) {
      toast.error(`${errMsg(e)} Opening it in the browser.`);
      await openNow(href);
    } finally {
      setDownloading((d) => (d === href ? null : d));
    }
  };
  const downloadRef = useRef(downloadLink);
  downloadRef.current = downloadLink;

  /** `auto`: a file is downloaded, anything else opens in the browser. The
   *  link chip asks for one of the two outright. */
  const onLink = useCallback(
    (href: string, text: string, mode: "auto" | "browser" | "download" = "auto") => {
      if (!openable(href)) return;
      setChip(null);
      // A click inside the message leaves the keyboard in its frame, where
      // Escape would never reach the preview or the question that follows.
      if (document.activeElement instanceof HTMLIFrameElement) document.activeElement.blur();
      // Mail in Spam or Trash never has a file downloaded into TIQR.
      const web = /^https?:\/\//i.test(href.trim());
      const download = !junk && web && (mode === "download" || (mode === "auto" && isDownloadLink(text, href)));
      const shown = shownElsewhere(text, href);
      if (shown || junk) setConfirmLink({ href, shown, download });
      else if (download) void downloadRef.current(href);
      else void openNow(href);
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [junk],
  );
  const copy = useCallback(
    async (text: string, what: string) => {
      if (await copyText(text)) toast.success(`${what} copied`);
      else toast.error("Couldn't copy - select it and press Ctrl+C.");
    },
    [toast],
  );
  const onCode = useCallback((c: string) => void copy(c, `Code ${c}`), [copy]);
  const onHover = useCallback((c: ChipState) => {
    if (hideTimer.current) window.clearTimeout(hideTimer.current);
    if (openable(c.href)) setChip(c);
  }, []);
  const onHoverEnd = useCallback(() => {
    if (hideTimer.current) window.clearTimeout(hideTimer.current);
    hideTimer.current = window.setTimeout(() => setChip(null), 350);
  }, []);
  useEffect(() => () => void (hideTimer.current && window.clearTimeout(hideTimer.current)), []);

  /* ---- the AI helper ---------------------------------------------------- */

  const askAi = async (task: "translate" | "summary") => {
    if (ai?.task === task) {
      setAi(null);
      return;
    }
    setAiBusy(task);
    try {
      setAi(await api.mailAi(message.id, task));
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setAiBusy(null);
    }
  };

  const target = status.translateTo || "sk";
  const sameLanguage = !!view?.language && view.language === target;
  const attachments = view ? view.attachments.filter((a) => !a.inline || !view.html) : [];
  const movedName = message.movedTo ? placeName(message.movedTo) : null;
  const linkProps = { onLink, onHover, onHoverEnd, onCode };

  return (
    <div className="flex h-full min-h-0 flex-col">
      {/* ---- what can be done with it ------------------------------------ */}
      <div className="flex flex-wrap items-center gap-1.5 border-b border-slate-200 px-4 py-2 dark:border-slate-800">
        <MailMoveMenu places={places} current={message.movedTo} moved={!!message.movedTo} onPick={(t) => onMove([message.id], t)} />
        {message.trashedAt ? (
          <Button variant="secondary" size="sm" onClick={() => onTrash([message.id], true)}>
            Restore
          </Button>
        ) : (
          <Button variant="ghost" size="sm" onClick={() => onTrash([message.id], false)} title="Move to TIQR's trash">
            <IconTrash className="h-3.5 w-3.5" /> Trash
          </Button>
        )}
        <Button
          variant="ghost"
          size="sm"
          onClick={async () => {
            try {
              await api.setMailRead(message.id, message.isUnread);
              onChanged({ isUnread: !message.isUnread });
            } catch (e) {
              toast.error(errMsg(e));
            }
          }}
        >
          {message.isUnread ? "Mark read" : "Mark unread"}
        </Button>
        <span className="mx-0.5 h-5 w-px bg-slate-200 dark:bg-slate-700" aria-hidden="true" />
        {!sameLanguage && (
          <Button
            variant={ai?.task === "translate" ? "primary" : "ghost"}
            size="sm"
            disabled={!view || aiBusy !== null}
            title={`Translate into ${languageName(target)} with the AI helper`}
            onClick={() => void askAi("translate")}
          >
            {aiBusy === "translate" ? <Spinner className="h-3.5 w-3.5" /> : <IconLanguages className="h-3.5 w-3.5" />}
            Translate
          </Button>
        )}
        <Button
          variant={ai?.task === "summary" ? "primary" : "ghost"}
          size="sm"
          disabled={!view || aiBusy !== null}
          title="A short summary by the AI helper"
          onClick={() => void askAi("summary")}
        >
          {aiBusy === "summary" ? <Spinner className="h-3.5 w-3.5" /> : <IconSparkles className="h-3.5 w-3.5" />}
          Summary
        </Button>
        <SendToDiscordButton kind="mail" id={message.id} size="sm" />
        {downloading && (
          <span className="ml-auto inline-flex items-center gap-1.5 text-[11px] text-slate-500 dark:text-slate-400">
            <Spinner className="h-3.5 w-3.5" /> Downloading the file...
          </span>
        )}
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto" onScroll={() => chip && setChip(null)}>
        {/* ---- who, when, what -------------------------------------------- */}
        <div className="px-5 pb-3 pt-4">
          <h2 className="text-[17px] font-semibold leading-snug text-slate-900 dark:text-slate-100">
            {message.subject || "(no subject)"}
          </h2>

          {/* "odkial ide, z akeho mailu" - the real address first, the
              display name second, because a display name is whatever the
              sender typed and the address is not. */}
          <div className="mt-2 flex items-start gap-3">
            <span
              aria-hidden="true"
              className="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-brand-500/15 text-sm font-semibold text-brand-700 dark:text-brand-300"
            >
              {(message.fromName || message.fromAddress).trim().charAt(0).toUpperCase()}
            </span>
            <div className="min-w-0 flex-1">
              <div className="flex flex-wrap items-baseline gap-x-2 gap-y-0.5">
                {message.fromName && (
                  <span className="text-sm font-medium text-slate-800 dark:text-slate-100">{message.fromName}</span>
                )}
                <span className="break-all font-mono text-xs text-slate-500 dark:text-slate-400">
                  &lt;{message.fromAddress}&gt;
                </span>
                <button
                  type="button"
                  onClick={() => setShowHeaders((v) => !v)}
                  className="text-[11px] text-brand-600 hover:underline dark:text-brand-400"
                >
                  {showHeaders ? "Hide details" : "Details"}
                </button>
              </div>
              <div className="mt-0.5 flex flex-wrap items-center gap-2">
                <span className="text-[11px] text-slate-400 dark:text-slate-500">
                  {new Date(message.receivedAt).toLocaleString()}
                </span>
                {message.isStarred && <span className="text-[11px] text-amber-500">&#9733; Starred</span>}
                {message.category !== "other" && !junk && <Badge tone="sold">{categoryLabel(message.category)}</Badge>}
                {junk && <Badge tone="cancelled">{has(message, "spam") ? "spam" : "trash"}</Badge>}
                {movedName && (
                  <span className="inline-flex items-center gap-1 rounded-md bg-surface-muted px-1.5 py-0.5 text-[11px] text-slate-600 dark:text-slate-300">
                    <IconFolder className="h-3 w-3" /> {movedName}
                  </span>
                )}
              </div>
            </div>
          </div>

          {showHeaders && (
            <dl className="mt-3 space-y-1 rounded-lg bg-surface-muted px-3 py-2 text-[11px]">
              {[
                ["From", message.fromRaw ?? message.fromAddress],
                ["To", message.toAddress],
                ["Reply-To", message.replyTo],
                ["Gmail labels", message.labels || null],
              ].map(([k, v]) =>
                v ? (
                  <div key={k} className="flex gap-2">
                    <dt className="w-24 shrink-0 text-slate-500 dark:text-slate-400">{k}</dt>
                    <dd className="min-w-0 break-all font-mono text-slate-600 dark:text-slate-300">{v}</dd>
                  </div>
                ) : null,
              )}
              {/* A reply address that disagrees with the sender is the single
                  most useful thing on this panel, so it is pointed at. */}
              {message.replyTo && !message.replyTo.includes(message.fromAddress) && (
                <p className="pt-1 text-amber-600 dark:text-amber-400">A reply would go somewhere other than the sender.</p>
              )}
            </dl>
          )}

          {/* The code, one click to copy. A one-time code is useless slowly. */}
          {code && (
            <button
              type="button"
              onClick={() => onCode(code)}
              title="Click to copy"
              className="group mt-3 flex w-full items-center justify-between gap-3 rounded-xl bg-surface-muted px-4 py-3 text-left transition hover:bg-brand-500/10"
            >
              <span>
                <span className="section-title block">One-time code</span>
                <span className="num mt-1 block text-2xl font-bold tracking-[0.3em] text-brand-600 dark:text-brand-400">{code}</span>
              </span>
              <span className="inline-flex items-center gap-1 text-xs font-medium text-slate-500 group-hover:text-brand-600 dark:text-slate-400 dark:group-hover:text-brand-400">
                <IconCopy className="h-4 w-4" /> Copy
              </span>
            </button>
          )}

          {/* 2.113.1: the lines that matter, from the whole message. */}
          {!junk && (view?.facts ?? []).length > 0 && (
            <KeyDetails key={message.id} facts={view?.facts ?? []} onCopy={(t, what) => void copy(t, what)} />
          )}

          {/* WHAT THIS COULD BECOME - every figure with its source, how sure,
              and whether something like it already exists
              (components/MailProposal.tsx). */}
          {proposal && !junk && (
            <MailProposalCard
              messageId={message.id}
              proposal={proposal}
              onChanged={(p) => {
                setProposal(p);
                onChanged(null);
              }}
            />
          )}

          {/* With nothing to propose, what the message says is still shown -
              read, not acted on. */}
          {!proposal &&
            !junk &&
            (message.extractedAmountCents != null || message.extractedQuantity != null || message.extractedReference) && (
              <div className="mt-3 rounded-xl bg-surface-muted px-4 py-3">
                <p className="section-title">What TIQR read</p>
                <dl className="mt-1.5 flex flex-wrap gap-x-6 gap-y-1 text-xs">
                  {message.extractedAmountCents != null && (
                    <div>
                      <dt className="text-slate-500 dark:text-slate-400">
                        Total{message.extractedBasis ? ` (${message.extractedBasis})` : ""}
                      </dt>
                      <dd className="num text-sm text-slate-800 dark:text-slate-200">
                        {(message.extractedAmountCents / 100).toFixed(2)} {message.extractedCurrency}
                      </dd>
                    </div>
                  )}
                  {message.extractedQuantity != null && (
                    <div>
                      <dt className="text-slate-500 dark:text-slate-400">Tickets</dt>
                      <dd className="num text-sm text-slate-800 dark:text-slate-200">{message.extractedQuantity}</dd>
                    </div>
                  )}
                  {message.extractedReference && (
                    <div>
                      <dt className="text-slate-500 dark:text-slate-400">Reference</dt>
                      <dd className="font-mono text-sm text-slate-800 dark:text-slate-200">{message.extractedReference}</dd>
                    </div>
                  )}
                </dl>
              </div>
            )}

          {junk && (
            <p className="mt-3 rounded-lg bg-surface-muted px-3 py-2 text-[11px] text-amber-600 dark:text-amber-400">
              This is in {has(message, "spam") ? "Spam" : "Trash"}. TIQR never classifies these and will never act on one,
              however convincing it looks - and asks before opening any of its links.
            </p>
          )}

          {/* ---- the AI helper's answer ------------------------------------ */}
          {ai && (
            <div className="mt-3 rounded-xl border border-brand-500/25 bg-brand-500/[0.06] px-4 py-3">
              <div className="flex items-center gap-2">
                {ai.task === "translate" ? (
                  <IconLanguages className="h-4 w-4 text-brand-600 dark:text-brand-400" />
                ) : (
                  <IconSparkles className="h-4 w-4 text-brand-600 dark:text-brand-400" />
                )}
                <p className="text-xs font-semibold text-slate-800 dark:text-slate-100">
                  {ai.task === "translate" ? `In ${languageName(ai.language)}` : "In short"}
                </p>
                {ai.partial && (
                  <span className="text-[11px] text-amber-700 dark:text-amber-400">the first part of a long email</span>
                )}
                <button
                  type="button"
                  aria-label="Close"
                  className="ml-auto rounded p-0.5 text-slate-400 hover:text-slate-700 dark:hover:text-slate-200"
                  onClick={() => setAi(null)}
                >
                  <IconX className="h-3.5 w-3.5" />
                </button>
              </div>
              <div className="mt-2">
                <MailText text={ai.text} {...linkProps} />
              </div>
              <p className="mt-2 text-[10.5px] text-slate-400 dark:text-slate-500">
                By the AI helper, on your click - the cheapest model, kept with this message
                {ai.cached ? " (this one cost nothing: it was kept from before)" : ""}. Check names and numbers in the
                original below.
              </p>
            </div>
          )}

          {attachments.length > 0 && (
            <AttachmentList messageId={message.id} items={attachments} onView={setPreviewing} onUpdated={refreshAttachments} />
          )}
        </div>

        {/* ---- the message itself ----------------------------------------- */}
        <div className="px-5 pb-5">
          {loading && !view ? (
            <div className="flex items-center gap-2 py-6 text-xs text-slate-500 dark:text-slate-400">
              <Spinner className="h-4 w-4" /> Opening the message...
            </div>
          ) : failed && !view ? (
            <div className="rounded-lg bg-surface-muted px-4 py-3">
              <p className="text-xs text-red-600 dark:text-red-400">{failed}</p>
              <Button variant="secondary" size="sm" className="mt-2" onClick={() => load()}>
                Try again
              </Button>
            </div>
          ) : view?.html ? (
            <div className="overflow-hidden rounded-xl border border-slate-200 bg-white dark:border-slate-700">
              <MailHtmlFrame html={view.html} code={code} {...linkProps} />
            </div>
          ) : view?.text.trim() ? (
            <div className="rounded-xl border border-slate-200 px-5 py-4 dark:border-slate-800">
              <MailText text={view.text} code={code} {...linkProps} />
            </div>
          ) : (
            <p className="text-xs text-slate-500 dark:text-slate-400">Nothing to show - this message has no readable text.</p>
          )}

          {view && (
            <div className="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1 text-[11px] text-slate-400 dark:text-slate-500">
              <span>{view.fromCache ? "From TIQR's copy" : "Opened from Gmail and kept in TIQR"}</span>
              {view.imagesCopied > 0 && (
                <span>
                  {view.imagesCopied} picture{view.imagesCopied === 1 ? "" : "s"} copied into TIQR
                </span>
              )}
              {view.imagesLeftOut > 0 && (
                <span title="Tracking pixels, pictures too big, or files that were not pictures">
                  {view.imagesLeftOut} left out
                </span>
              )}
              {!status.remoteImages && view.html && <span>Pictures from the web are off (Mail settings)</span>}
              <button
                type="button"
                disabled={loading}
                className="inline-flex items-center gap-1 text-brand-600 hover:underline disabled:opacity-50 dark:text-brand-400"
                onClick={() => load(true)}
              >
                <IconRefresh className="h-3 w-3" /> Download again
              </button>
            </div>
          )}
        </div>
      </div>

      <div className="flex flex-wrap items-center gap-2 border-t border-slate-200 px-5 py-2.5 dark:border-slate-800">
        {message.status === "review_required" && (
          <Button
            variant="secondary"
            size="sm"
            disabled={busy}
            onClick={async () => {
              setBusy(true);
              try {
                await api.setMailReviewOutcome(message.id, "reviewed");
                onChanged(null);
              } catch (e) {
                toast.error(errMsg(e));
              } finally {
                setBusy(false);
              }
            }}
          >
            Mark reviewed
          </Button>
        )}
        <span className="text-[11px] text-slate-400 dark:text-slate-500">
          Read state and folders are TIQR&apos;s own - Gmail is never changed.
        </span>
      </div>

      {chip && (
        <LinkChip
          chip={chip}
          canDownload={!junk}
          onEnter={() => hideTimer.current && window.clearTimeout(hideTimer.current)}
          onLeave={onHoverEnd}
          onOpen={() => onLink(chip.href, chip.text, "browser")}
          onDownload={() => onLink(chip.href, chip.text, "download")}
          onCopy={() => {
            const mail = chip.href.toLowerCase().startsWith("mailto:");
            void copy(mail ? chip.href.slice(7) : chip.href, mail ? "Address" : "Link");
            setChip(null);
          }}
        />
      )}

      <ConfirmDialog
        open={!!confirmLink}
        title={confirmLink?.shown ? "This link is not what it says" : "Open a link from Spam or Trash?"}
        message={
          confirmLink?.shown ? (
            <>
              The text says <b className="font-semibold">{confirmLink.shown}</b>, but it opens{" "}
              <b className="break-all font-semibold">{hostOf(confirmLink.href) ?? confirmLink.href}</b>. That is how phishing
              mail looks - open it only if you expected it.
            </>
          ) : (
            <>
              It opens <b className="break-all font-semibold">{hostOf(confirmLink?.href ?? "") ?? confirmLink?.href}</b>. Mail in
              Spam or Trash is not trusted.
            </>
          )
        }
        confirmLabel={confirmLink?.download ? "Download it" : "Open it"}
        danger={!!confirmLink?.shown}
        onCancel={() => setConfirmLink(null)}
        onConfirm={() => {
          const c = confirmLink;
          setConfirmLink(null);
          if (!c) return;
          if (c.download) void downloadRef.current(c.href);
          else void openNow(c.href);
        }}
      />

      {previewing && (
        <AttachmentPreview
          messageId={message.id}
          att={previewing}
          onClose={() => {
            setPreviewing(null);
            refreshAttachments();
          }}
        />
      )}
    </div>
  );
}

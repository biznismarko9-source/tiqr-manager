// 2.113.0: Discord on the screen - the "Send to Discord" button every record
// carries, and a preview that draws a message exactly as TIQR would post it.
//
// marko: "pri kazdom evente, sale, finance atd vies kliknut na vytvorit
// webhook aby sa ti poslal" - and in Settings, "vies si to otvorit a pozriet
// co to obsahuje". The message itself is built by the backend
// (commands/discord.rs); nothing here decides what a message says.
import { useEffect, useState, type MouseEvent, type ReactNode } from "react";
import logo from "../assets/logo.png";
import { api, errMsg } from "../lib/api";
import { useToast } from "../lib/toast";
import type { DiscordEmbed, DiscordSubjectKind, NotificationStatus } from "../lib/types";
import { IconSend } from "./icons";
import { Button, Spinner } from "./ui";

// ---------------------------------------------------------------------------
// Is Discord set up at all? Read once and shared by every button on screen;
// Settings calls `rememberDiscordStatus` after a save so they follow at once.
// ---------------------------------------------------------------------------

let known: boolean | null = null;
let asking: Promise<boolean> | null = null;
const listeners = new Set<(ready: boolean) => void>();

function readyFrom(s: NotificationStatus): boolean {
  return s.discordEnabled && (s.discordSalesSet || s.discordFinanceSet || s.discordInventorySet || s.discordMailSet);
}

export function rememberDiscordStatus(s: NotificationStatus) {
  known = readyFrom(s);
  listeners.forEach((l) => l(known as boolean));
}

function askOnce(): Promise<boolean> {
  if (!asking) {
    asking = api
      .getNotificationStatus()
      .then((s) => {
        rememberDiscordStatus(s);
        return readyFrom(s);
      })
      .catch(() => false);
  }
  return asking;
}

/** True once Discord is on and at least one channel has a webhook. */
export function useDiscordReady(): boolean {
  const [ready, setReady] = useState<boolean>(known ?? false);
  useEffect(() => {
    listeners.add(setReady);
    if (known === null) askOnce().then(setReady);
    return () => {
      listeners.delete(setReady);
    };
  }, []);
  return ready;
}

/** Posts one record (or email) to its channel now and says whether it arrived.
 *  Hidden while Discord is not set up, so it never offers what cannot work. */
export function SendToDiscordButton({
  kind,
  id,
  size = "md",
  look = "button",
}: {
  kind: DiscordSubjectKind;
  id: number;
  size?: "sm" | "md";
  /** "row": the small bare icon a table row's own Edit/Delete use. */
  look?: "button" | "row";
}) {
  const ready = useDiscordReady();
  const toast = useToast();
  const [busy, setBusy] = useState(false);
  if (!ready) return null;
  const send = async () => {
    setBusy(true);
    try {
      const r = await api.sendToDiscord(kind, id);
      if (r.status === "delivered") toast.success(r.message);
      // Not a failure: Discord may already have it. Said plainly instead.
      else if (r.status === "unknown") toast.info(r.message);
      else toast.error(r.message);
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setBusy(false);
    }
  };
  const click = (e: MouseEvent) => {
    e.stopPropagation();
    send();
  };
  if (look === "row") {
    return (
      <button
        type="button"
        className="rounded p-1 text-slate-300 hover:text-brand-600 disabled:opacity-50 dark:text-slate-600 dark:hover:text-brand-400"
        title="Send to Discord"
        aria-label="Send to Discord"
        disabled={busy}
        onClick={click}
      >
        {busy ? <Spinner className="h-4 w-4" /> : <IconSend className="h-4 w-4" />}
      </button>
    );
  }
  return (
    <Button variant="secondary" size={size} onClick={click} disabled={busy} title="Send to Discord">
      {busy ? <Spinner className="h-4 w-4" /> : <IconSend className="h-4 w-4" />}
      Send to Discord
    </Button>
  );
}

// ---------------------------------------------------------------------------
// The preview. Discord's own Markdown, as much of it as TIQR's messages use:
// **bold**, \-escapes, "> " quotes and <t:…> times.
// ---------------------------------------------------------------------------

const RELATIVE = new Intl.RelativeTimeFormat(undefined, { numeric: "auto" });

function relativeTime(seconds: number): string {
  const diff = seconds - Date.now() / 1000;
  const abs = Math.abs(diff);
  const units: [Intl.RelativeTimeFormatUnit, number][] = [
    ["year", 31536000],
    ["month", 2592000],
    ["week", 604800],
    ["day", 86400],
    ["hour", 3600],
    ["minute", 60],
  ];
  for (const [unit, size] of units) {
    if (abs >= size) return RELATIVE.format(Math.round(diff / size), unit);
  }
  return RELATIVE.format(Math.round(diff), "second");
}

function discordTime(seconds: number, style: string): string {
  const d = new Date(seconds * 1000);
  if (style === "R") return relativeTime(seconds);
  if (style === "d") return d.toLocaleDateString();
  return d.toLocaleString(undefined, { dateStyle: "long", timeStyle: "short" });
}

/** One line of Discord text as React nodes. Exported for its test. */
export function discordInline(text: string): { text: string; bold: boolean }[] {
  const out: { text: string; bold: boolean }[] = [];
  let bold = false;
  let buf = "";
  const flush = () => {
    if (buf) out.push({ text: buf, bold });
    buf = "";
  };
  const re = /\\([^A-Za-z0-9\s])|\*\*|<t:(-?\d+)(?::([a-zA-Z]))?>/g;
  let last = 0;
  for (let m = re.exec(text); m; m = re.exec(text)) {
    buf += text.slice(last, m.index);
    last = re.lastIndex;
    if (m[1] !== undefined) buf += m[1];
    else if (m[0] === "**") {
      flush();
      bold = !bold;
    } else buf += discordTime(Number(m[2]), m[3] ?? "f");
  }
  buf += text.slice(last);
  flush();
  return out;
}

function DiscordText({ text }: { text: string }) {
  const lines = text.split("\n");
  return (
    <>
      {lines.map((line, i) => {
        const quote = line.startsWith("> ");
        const parts = discordInline(quote ? line.slice(2) : line).map((p, j) =>
          p.bold ? (
            <strong key={j} className="font-semibold text-slate-900 dark:text-white">
              {p.text}
            </strong>
          ) : (
            <span key={j}>{p.text}</span>
          ),
        );
        return quote ? (
          <div key={i} className="my-0.5 border-l-4 border-slate-300 pl-2 dark:border-slate-600">
            {parts}
          </div>
        ) : (
          <div key={i}>{parts.length ? parts : " "}</div>
        );
      })}
    </>
  );
}

function hex(color?: number): string {
  return `#${(color ?? 0x8b5cf6).toString(16).padStart(6, "0")}`;
}

/** A message drawn the way Discord shows it: the bot's line, then the embed. */
export function DiscordEmbedPreview({ embed }: { embed: DiscordEmbed }) {
  const fields = embed.fields ?? [];
  // Inline fields sit three to a row, as in Discord; a full-width one breaks
  // the row.
  const rows: { inline: boolean; items: typeof fields }[] = [];
  for (const f of fields) {
    const lastRow = rows[rows.length - 1];
    if (f.inline && lastRow?.inline && lastRow.items.length < 3) lastRow.items.push(f);
    else rows.push({ inline: !!f.inline, items: [f] });
  }
  let when = "";
  try {
    when = embed.timestamp ? new Date(embed.timestamp).toLocaleString(undefined, { timeStyle: "short" }) : "";
  } catch {
    when = "";
  }
  return (
    <div className="flex gap-3 rounded-lg bg-[#f2f3f5] p-3 text-[13px] leading-snug text-slate-700 dark:bg-[#2b2d31] dark:text-[#dbdee1]">
      <img src={logo} alt="" className="h-9 w-9 shrink-0 rounded-full" />
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-1.5">
          <span className="font-semibold text-slate-900 dark:text-white">TIQR Manager</span>
          <span className="rounded bg-[#5865f2] px-1 text-[10px] font-semibold leading-4 text-white">APP</span>
          {when && <span className="text-[11px] text-slate-500 dark:text-slate-400">{when}</span>}
        </div>
        <div
          className="mt-1 max-w-[520px] rounded border-l-4 bg-white px-3 py-2.5 shadow-sm dark:bg-[#232428]"
          style={{ borderLeftColor: hex(embed.color) }}
        >
          {embed.author?.name && (
            <div className="mb-1 flex items-center gap-1.5 text-xs font-semibold text-slate-800 dark:text-white">
              <img src={logo} alt="" className="h-5 w-5 rounded-full" />
              <span className="truncate">{embed.author.name}</span>
            </div>
          )}
          {embed.title && (
            <div className={`font-semibold ${embed.url ? "text-[#006ce7] dark:text-[#00a8fc]" : "text-slate-900 dark:text-white"}`}>
              {embed.title}
            </div>
          )}
          {embed.description && (
            <div className="mt-1 break-words">
              <DiscordText text={embed.description} />
            </div>
          )}
          {rows.length > 0 && (
            <div className="mt-2 space-y-2">
              {rows.map((row, i) => (
                <div key={i} className={row.inline ? "grid grid-cols-3 gap-x-3" : ""}>
                  {row.items.map((f, j) => (
                    <div key={j} className="min-w-0">
                      <div className="text-xs font-semibold text-slate-900 dark:text-white">{f.name}</div>
                      <div className="break-words text-xs">
                        <DiscordText text={f.value} />
                      </div>
                    </div>
                  ))}
                </div>
              ))}
            </div>
          )}
          {embed.footer?.text && (
            <div className="mt-2 flex items-center gap-1.5 text-[11px] text-slate-500 dark:text-slate-400">
              <img src={logo} alt="" className="h-4 w-4 rounded-full" />
              <span className="truncate">
                {embed.footer.text}
                {when ? ` • ${when}` : ""}
              </span>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

/** A small titled block inside a channel row. */
export function DiscordRowSection({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div>
      <p className="mb-1.5 text-[11px] font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">{title}</p>
      {children}
    </div>
  );
}

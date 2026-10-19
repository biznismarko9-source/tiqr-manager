// 2.111.0 - what the mail reader is doing, in words, for the Mail screen and
// its settings.
//
// The reader works in lanes (commands/mail_sync.rs): new mail first, then
// every starred message, then every message in his labels, and only then the
// history. The screen says which lane it is on and how much each one still
// has to download - and it never invents a total: a count is what is queued,
// and a lane that may still grow says so ("finding more").
//
// Pure functions with type-only imports, so `npm test` runs them in Node
// with no build step (mailSyncStatus.test.ts).
import type { MailLaneProgress, MailSyncProgress } from "./types";

export type LaneTone = "active" | "done" | "waiting" | "warn";

export interface LaneView {
  key: "new" | "starred" | "labels" | "backlog" | "retry";
  label: string;
  text: string;
  tone: LaneTone;
  /** A sentence for the tooltip. */
  hint: string;
}

type Fmt = (n: number) => string;

/** One lane in a few words. `after` names the lane it waits for. */
export function laneText(lane: MailLaneProgress, fmt: Fmt, after: string): { text: string; tone: LaneTone } {
  const left = lane.waiting;
  switch (lane.discovery) {
    case "watching":
      return left > 0 ? { text: `${fmt(left)} to download`, tone: "active" } : { text: "watching", tone: "done" };
    case "pending":
      return left > 0
        ? { text: `${fmt(left)} to download, finding more after ${after}`, tone: "waiting" }
        : { text: `after ${after}`, tone: "waiting" };
    case "discovering":
      return left > 0 ? { text: `${fmt(left)}+ to download`, tone: "active" } : { text: "finding…", tone: "active" };
    default:
      return left > 0 ? { text: `${fmt(left)} to download`, tone: "active" } : { text: "all here", tone: "done" };
  }
}

/** The lanes, in the order the reader works them. */
export function laneViews(p: MailSyncProgress, fmt: Fmt): LaneView[] {
  const views: LaneView[] = [
    {
      key: "new",
      label: "New mail",
      ...(p.newMail.discovery === "pending"
        ? { text: "from the first sync", tone: "waiting" as const }
        : laneText(p.newMail, fmt, "")),
      hint: "Mail that just arrived always goes first, even in the middle of a long first read.",
    },
    {
      key: "starred",
      label: "Starred",
      ...laneText(p.starred, fmt, "new mail"),
      hint: "Every starred message, however old - before anything else in your history.",
    },
    {
      key: "labels",
      label: "Labels",
      ...laneText(p.labels, fmt, "Starred"),
      hint: "Every message in your own labels, however old - after Starred, before the rest.",
    },
    {
      key: "backlog",
      label: "History",
      ...laneText(p.backlog, fmt, "labels"),
      hint: "Everything else, newest first. It carries on in the background while you use the inbox.",
    },
  ];
  if (p.retryLater > 0) {
    views.push({
      key: "retry",
      label: "Retrying later",
      text: fmt(p.retryLater),
      tone: "warn",
      hint: "Messages Gmail would not give this time. They stay queued and are tried again by themselves.",
    });
  }
  return views;
}

export interface HeadlineInput {
  progress: MailSyncProgress;
  paused: boolean;
  /** The last run failed - what went wrong is shown below it. */
  failed: boolean;
  rateLimited: boolean;
  intervalSeconds: number;
}

/** One line: what the reader is doing right now. */
export function syncHeadline({ progress, paused, failed, rateLimited, intervalSeconds }: HeadlineInput): string {
  if (paused) return "Paused";
  if (failed) return "Not reading right now - see below";
  if (rateLimited) return "Google asked TIQR to slow down - carrying on by itself";
  const where = progress.reading;
  switch (progress.phase) {
    case "new":
      return "Downloading new mail first";
    case "starred":
      return "Starred first - the rest of your history follows";
    case "labels":
      return `Your labels next${where && where !== "Labels" ? ` (${where})` : ""} - history follows`;
    case "backlog":
      return `History in the background${where ? ` · ${where}` : ""}`;
    default:
      if (intervalSeconds <= 0) return "Up to date";
      return `Up to date · checks every ${
        intervalSeconds < 60 ? `${intervalSeconds}s` : `${Math.round(intervalSeconds / 60)} min`
      }`;
  }
}

/** "Last sync 14:05", "Last sync 17 Oct", or that there has been none. */
export function lastSyncText(iso: string | null | undefined, now: Date): string {
  if (!iso) return "Not synced yet";
  const at = new Date(iso);
  if (Number.isNaN(at.getTime())) return "Not synced yet";
  const sameDay =
    at.getFullYear() === now.getFullYear() && at.getMonth() === now.getMonth() && at.getDate() === now.getDate();
  const hh = String(at.getHours()).padStart(2, "0");
  const mm = String(at.getMinutes()).padStart(2, "0");
  if (sameDay) return `Last sync ${hh}:${mm}`;
  const month = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"][at.getMonth()];
  return `Last sync ${at.getDate()} ${month}`;
}

/** Whether the reader is working on something - for the spinner. */
export function isReading(progress: MailSyncProgress, paused: boolean, failed: boolean): boolean {
  return !paused && !failed && progress.phase !== "idle";
}

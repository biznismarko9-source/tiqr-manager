// 2.115.0 - two computers, one Gmail reader: the pure part.
//
// marko: "najlepsie ak to vedia robit oba podla toho ktory prave runnuje, ak
// naraz tak len jeden a ten to bude posielat tomu druhemu aby to bolo
// obmedzene". While both computers run, the one holding the lease reads
// Gmail and sends the other what it stored; alone, a computer reads Gmail
// itself. Everything sits under the account's own users/{uid}/mail
// (firestore.rules):
//
//   lease~{mailbox}              who reads Gmail for that mailbox, renewed
//                                every TICK_MS, free once nobody renewed it
//                                for LEASE_STALE_MS
//   cursor~{follower}~{reader}   how far the follower has got with the reader -
//                                renewed while it follows, so it says it is there
//   drop~{follower}~{seq}        one batch for the follower, deleted as soon as
//                                it is stored there
//
// Pure functions with type-only imports, so `npm test` runs them in Node
// (mailShareCore.test.ts); the session that talks to Firestore is
// lib/mailShareSession.ts.
import type { ShareCursor, SharedMail } from "./types";

/** How often the lease is renewed and the follower says it is there - once
 *  a minute, which is as often as a minimised window's timers may run. */
export const TICK_MS = 60_000;
/** A lease or a follower not renewed for this long is gone. */
export const LEASE_STALE_MS = 180_000;
/** A lease this old at first sight is gone without waiting - no computer's
 *  clock is that far off. */
export const LEASE_ABANDONED_MS = 15 * 60_000;

export type ShareRole = "off" | "solo" | "reader" | "follower";

/** The one lease per mailbox - FNV-1a 64 of the address, so no address is in
 *  a document id. In two 32-bit halves rather than a BigInt, which the Mac
 *  build's oldest Safari does not have: the prime is 2^40 + 0x1b3, so a
 *  multiply is a shift by 40 plus a small product, each exact in a double. */
export function leaseId(account: string): string {
  let hi = 0xcbf29ce4;
  let lo = 0x84222325;
  for (const b of new TextEncoder().encode(account.trim().toLowerCase())) {
    lo = (lo ^ b) >>> 0;
    const low = lo * 0x1b3;
    const carry = Math.floor(low / 0x1_0000_0000);
    hi = (hi * 0x1b3 + carry + ((lo << 8) >>> 0)) >>> 0;
    lo = low >>> 0;
  }
  return `lease~${hi.toString(16).padStart(8, "0")}${lo.toString(16).padStart(8, "0")}`;
}

export const cursorId = (follower: string, reader: string) => `cursor~${follower}~${reader}`;
export const dropPrefix = (follower: string) => `drop~${follower}~`;

/** Drop ids sort in the order they were written. */
export const dropId = (follower: string, nowMs: number, n: number) =>
  `${dropPrefix(follower)}${Math.floor(nowMs).toString(36).padStart(9, "0").slice(-9)}${(n % 46_656).toString(36).padStart(3, "0")}`;

const DEVICE = /^[0-9a-f]{32}$/;
export const isDevice = (s: unknown): s is string => typeof s === "string" && DEVICE.test(s);

export interface Lease {
  dev: string;
  atMs: number;
  account: string;
  platform: string;
}

/** The lease as this computer last saw it, and since when - by its own
 *  clock - it has looked like that. */
export interface LeaseSeen {
  key: string;
  sinceMs: number;
}

const leaseKey = (l: Lease) => `${l.dev}@${l.atMs}`;

/** One more look at the lease: the same as before keeps its time, a renewed
 *  one starts again. */
export function seeLease(lease: Lease | null, seen: LeaseSeen | null, nowMs: number): LeaseSeen | null {
  if (!lease) return null;
  const key = leaseKey(lease);
  return seen?.key === key ? seen : { key, sinceMs: nowMs };
}

/** This computer reads Gmail: the lease is free or already its own, or the
 *  computer holding it stopped renewing it. Stopped is judged by this
 *  computer's own clock - the lease looked the same for `staleMs` - so two
 *  clocks that do not agree never have both computers taking it in turns. */
export function takesLease(
  lease: Lease | null,
  me: string,
  seen: LeaseSeen | null,
  nowMs: number,
  staleMs = LEASE_STALE_MS,
  abandonedMs = LEASE_ABANDONED_MS,
): boolean {
  if (!lease || lease.dev === me) return true;
  if (seen && seen.key === leaseKey(lease) && nowMs - seen.sinceMs > staleMs) return true;
  return nowMs - lease.atMs > abandonedMs;
}

/** `a` is further on than `b`. */
export function cursorAfter(a: ShareCursor, b: ShareCursor): boolean {
  return a.at > b.at || (a.at === b.at && a.id > b.id);
}

function json(p: unknown): Record<string, unknown> | null {
  if (typeof p !== "string") return null;
  try {
    const v: unknown = JSON.parse(p);
    return v && typeof v === "object" && !Array.isArray(v) ? (v as Record<string, unknown>) : null;
  } catch {
    return null;
  }
}

function aCursor(v: unknown): ShareCursor | null {
  if (!v || typeof v !== "object") return null;
  const c = v as Record<string, unknown>;
  return typeof c.at === "string" && typeof c.id === "number" && Number.isInteger(c.id) ? { at: c.at, id: c.id } : null;
}

type At = { toMillis(): number } | null | undefined;
const millis = (at: unknown): number => {
  const t = at as At;
  return t && typeof t.toMillis === "function" ? t.toMillis() : 0;
};

/** What Firestore holds is read like anything from outside: a document with
 *  the wrong shape is not a lease, a cursor or a batch. */
export function parseLease(data: Record<string, unknown> | undefined): Lease | null {
  if (!data || !isDevice(data.dev)) return null;
  const p = json(data.p);
  if (!p || typeof p.account !== "string") return null;
  return { dev: data.dev, atMs: millis(data.at), account: p.account, platform: typeof p.platform === "string" ? p.platform : "" };
}

export interface CursorDoc {
  follower: string;
  reader: string;
  upTo: ShareCursor;
  received: number;
  platform: string;
  atMs: number;
  /** When Sync was last pressed on the follower - the reader looks at Gmail
   *  then, rather than at its next turn. */
  nudge: number;
}

/** Until when, by the reader's clock, a follower counts as there: for
 *  `staleMs` after its cursor was seen to change - and in the first look at
 *  Firestore, only for what is left of that by the time it says. */
export function presentUntil(cur: CursorDoc, nowMs: number, firstLook: boolean, staleMs = LEASE_STALE_MS): number {
  return nowMs + staleMs - (firstLook ? Math.max(0, nowMs - cur.atMs) : 0);
}

export function parseCursorDoc(id: string, data: Record<string, unknown> | undefined): CursorDoc | null {
  const m = /^cursor~([0-9a-f]{32})~([0-9a-f]{32})$/.exec(id);
  if (!m || !data || data.dev !== m[1]) return null;
  const p = json(data.p);
  const upTo = aCursor(p?.upTo);
  if (!p || !upTo) return null;
  return {
    follower: m[1],
    reader: m[2],
    upTo,
    received: typeof p.received === "number" ? p.received : 0,
    platform: typeof p.platform === "string" ? p.platform : "",
    atMs: millis(data.at),
    nudge: typeof p.nudge === "number" && Number.isFinite(p.nudge) ? p.nudge : 0,
  };
}

export interface Drop {
  sender: string;
  account: string;
  rows: SharedMail[];
  upTo: ShareCursor;
}

export function parseDrop(data: Record<string, unknown> | undefined): Drop | null {
  if (!data || !isDevice(data.dev)) return null;
  const p = json(data.p);
  const upTo = aCursor(p?.upTo);
  if (!p || !upTo || typeof p.account !== "string" || !Array.isArray(p.rows)) return null;
  // Each message is checked again where it is stored (commands/mail_share.rs).
  const rows = p.rows.filter((r): r is SharedMail => !!r && typeof r === "object" && typeof (r as SharedMail).id === "string");
  return { sender: data.dev, account: p.account, rows, upTo };
}

/** "Mac", "Windows" - what to call a computer by its `navigator.platform`. */
export function platformName(platform: string): string {
  const p = platform.toLowerCase();
  if (p.startsWith("mac")) return "Mac";
  if (p.startsWith("win")) return "Windows computer";
  if (p.includes("linux")) return "Linux computer";
  return "other computer";
}

export interface MailShareView {
  role: ShareRole;
  /** The other computer, by name - the reader when this one follows, the
   *  follower when this one reads and someone follows. */
  peer: string | null;
  sent: number;
  received: number;
  /** Why sharing is not running, or what went wrong. */
  detail: string | null;
  /** `detail` is something going wrong, not a choice. */
  warn: boolean;
}

export const EMPTY_SHARE_VIEW: MailShareView = { role: "off", peer: null, sent: 0, received: 0, detail: null, warn: false };

/** The sentence Mail -> Settings and the Mail screen show. */
export function shareLine(v: MailShareView, fmt: (n: number) => string): string {
  switch (v.role) {
    case "follower":
      return `Your ${v.peer ?? "other computer"} reads Gmail and sends the mail here${v.received > 0 ? ` - ${fmt(v.received)} received` : ""}.`;
    case "reader":
      return v.peer
        ? `This computer reads Gmail and sends the mail to your ${v.peer}${v.sent > 0 ? ` - ${fmt(v.sent)} sent` : ""}.`
        : "This computer reads Gmail. When your other computer is on too, it gets the mail from here.";
    case "solo":
      return v.detail ?? "This computer reads Gmail itself.";
    default:
      return v.detail ?? "Each computer reads Gmail itself.";
  }
}

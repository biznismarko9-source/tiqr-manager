// 2.112.0 - the rules of live sync that never touch Firestore, kept apart so
// `npm test` can hold them without a network. liveSync.ts does the talking;
// commands/live_sync.rs keeps the records.

/** One record as it travels - commands/live_sync.rs's `SyncDoc`. */
export type SyncDoc = {
  id: string;
  t: string;
  u: string;
  h: number;
  del: boolean;
  d: Record<string, unknown>;
  dev: string;
};

/** What decides which of two versions of a record is kept. */
export type Version = { h: number; del: boolean; dev: string };

/**
 * The rule firestore.rules enforces, and the one the fallback transaction
 * applies itself: a deletion replaces any record and is replaced by nothing
 * but a newer deletion; otherwise only a newer version replaces one - newer
 * by the hybrid clock, and for the same clock by the higher device id, so
 * both computers always agree which one that is.
 */
export function replaces(next: Version, current: Version | null): boolean {
  if (!current) return true;
  const newer = next.h > current.h || (next.h === current.h && next.dev > current.dev);
  if (!current.del && next.del) return true;
  if (current.del && !next.del) return false;
  return newer;
}

/** Firestore counts a string as its UTF-8 bytes. */
export function utf8Length(s: string): number {
  let n = 0;
  for (let i = 0; i < s.length; i++) {
    const c = s.charCodeAt(i);
    if (c < 0x80) n += 1;
    else if (c < 0x800) n += 2;
    else if (c >= 0xd800 && c <= 0xdbff && i + 1 < s.length) {
      n += 4;
      i++;
    } else n += 3;
  }
  return n;
}

/**
 * A body that fits in the record's own document. Firestore's limit is 1 MiB
 * for the whole document, field names and all; this leaves room for them.
 */
export const INLINE_BYTES = 700_000;

/** At most 3 bytes per UTF-16 unit (4 per surrogate pair), so 250k units is
 *  never over 750 KB - one part per document. */
export const PART_UNITS = 250_000;

/** A body too big for one document, in parts. Never splits a surrogate pair:
 *  each half alone is not a character, and would arrive as two "?". */
export function splitBody(body: string, units = PART_UNITS): string[] {
  const parts: string[] = [];
  let i = 0;
  while (i < body.length) {
    let end = Math.min(i + units, body.length);
    const last = body.charCodeAt(end - 1);
    // Back off one, or - when that would leave nothing - take the pair whole.
    if (end < body.length && last >= 0xd800 && last <= 0xdbff) end = end - 1 > i ? end - 1 : end + 1;
    parts.push(body.slice(i, end));
    i = end;
  }
  return parts.length ? parts : [""];
}

/** The id of one part of one version - a newer version never reads an
 *  older one's parts by mistake. */
export function partId(docId: string, h: number, index: number): string {
  return `${docId}~${h}~${index}`;
}

/** What is stored in `users/{uid}/rows/{id}`. `body` is the record as JSON,
 *  or absent when it is in `parts`. */
export type StoredRow = Version & {
  t: string;
  u: string;
  body?: string;
  parts?: number;
};

export function toStored(doc: SyncDoc): { row: StoredRow; parts: string[] } {
  const body = JSON.stringify(doc.d);
  const base = { t: doc.t, u: doc.u, h: doc.h, del: doc.del, dev: doc.dev };
  if (doc.del) return { row: { ...base, body: "{}" }, parts: [] };
  if (utf8Length(body) <= INLINE_BYTES) return { row: { ...base, body }, parts: [] };
  const parts = splitBody(body);
  return { row: { ...base, parts: parts.length }, parts };
}

/** The other way round. `parts` are the stored parts in order, when the
 *  record has them; `null` when one is missing (not written yet). */
export function fromStored(id: string, row: StoredRow, parts?: (string | undefined)[]): SyncDoc | null {
  let body = row.body;
  if (body === undefined) {
    if (!row.parts || !parts || parts.length !== row.parts || parts.some((p) => p === undefined)) return null;
    body = parts.join("");
  }
  let d: Record<string, unknown>;
  try {
    d = JSON.parse(body) as Record<string, unknown>;
  } catch {
    return null;
  }
  return { id, t: row.t, u: row.u, h: row.h, del: !!row.del, d, dev: row.dev };
}

/** Writes in one batch: Firestore allows 500 operations and about 10 MB per
 *  request; this stays well inside both. */
export function batches<T>(items: T[], size: (item: T) => number, maxOps = 400, maxBytes = 8_000_000): T[][] {
  const out: T[][] = [];
  let cur: T[] = [];
  let bytes = 0;
  for (const item of items) {
    const s = size(item);
    if (cur.length && (cur.length >= maxOps || bytes + s > maxBytes)) {
      out.push(cur);
      cur = [];
      bytes = 0;
    }
    cur.push(item);
    bytes += s;
  }
  if (cur.length) out.push(cur);
  return out;
}

/** How a failure reads to marko, and how long to wait before trying again. */
export type Trouble = "rules" | "quota" | "offline" | "signed-out" | "error";

export function classify(code: string | undefined): Trouble {
  switch (code) {
    case "permission-denied":
      return "rules";
    case "resource-exhausted":
      return "quota";
    case "unavailable":
    case "deadline-exceeded":
    case "cancelled":
      return "offline";
    case "unauthenticated":
      return "signed-out";
    default:
      return "error";
  }
}

/** 2 s, doubling to 5 min; Firebase's daily free limit is waited out in
 *  quarters of an hour. Jitter, so two computers do not retry in step. */
export function retryDelay(trouble: Trouble, attempt: number, random = Math.random): number {
  if (trouble === "quota") return 15 * 60_000;
  if (trouble === "rules" || trouble === "signed-out") return 60_000;
  const base = Math.min(2_000 * 2 ** Math.max(0, attempt - 1), 5 * 60_000);
  return Math.round(base * (0.8 + random() * 0.4));
}

/** Firestore's commit time, as `{seconds, nanoseconds}`. */
export type Cursor = { seconds: number; nanoseconds: number };

export function laterCursor(a: Cursor | null, b: Cursor | null): Cursor | null {
  if (!a) return b;
  if (!b) return a;
  if (a.seconds !== b.seconds) return a.seconds > b.seconds ? a : b;
  return a.nanoseconds >= b.nanoseconds ? a : b;
}

/** What the sidebar line and Settings say. */
export type LivePhase = "off" | "starting" | "catching-up" | "live" | "sending" | Trouble;

export function phaseLabel(phase: LivePhase, outbox: number): string {
  switch (phase) {
    case "off":
      return "Live sync off";
    case "starting":
      return "Live sync starting";
    case "catching-up":
      return "Getting changes";
    case "sending":
      return outbox > 0 ? `Sending ${outbox}` : "Sending";
    case "live":
      return outbox > 0 ? `Live · ${outbox} to send` : "Live";
    // The sidebar has room for about eighteen characters.
    case "rules":
      return "Sync needs setup";
    case "quota":
      return "Daily limit reached";
    case "offline":
      return outbox > 0 ? `Offline · ${outbox} queued` : "Offline";
    case "signed-out":
      return "Sign in to sync";
    default:
      return "Live sync paused";
  }
}

export function phaseTone(phase: LivePhase): "ok" | "busy" | "warn" | "off" {
  if (phase === "live") return "ok";
  if (phase === "starting" || phase === "catching-up" || phase === "sending") return "busy";
  if (phase === "off") return "off";
  return "warn";
}

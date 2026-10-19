// 2.111.0 - keeping the Mail list steady while new mail pours in.
//
// During a long first read the newest page of a folder is fetched again every
// few seconds. Replacing the whole list each time made React redraw every one
// of its rows (each row is memoised on its message object, and every object
// was new) and dropped what he had scrolled down to. This keeps what was
// already on screen: a row whose message did not change keeps the very same
// object, so only new or changed rows are drawn again, and rows older than
// the fresh page stay where they were.
//
// Pure, with a type-only import, so `npm test` runs it in Node
// (mailRows.test.ts).
import type { MailMessageSummary } from "./types";

type Row = Pick<MailMessageSummary, "id" | "receivedAt">;

/** Same message, same values - field by field, one level deep. */
export function sameRow<T extends object>(a: T, b: T): boolean {
  if (a === b) return true;
  const ka = Object.keys(a) as (keyof T)[];
  if (ka.length !== Object.keys(b).length) return false;
  return ka.every((k) => {
    const x = a[k];
    const y = b[k];
    if (Array.isArray(x) && Array.isArray(y)) return x.length === y.length && x.every((v, i) => v === y[i]);
    return x === y;
  });
}

/** Newest first, the way every folder is ordered. */
function olderThan(m: Row, edge: Row): boolean {
  return m.receivedAt < edge.receivedAt || (m.receivedAt === edge.receivedAt && m.id < edge.id);
}

/**
 * The folder's fresh first page, merged into what is on screen.
 *
 * `prev` is the list as shown, `page` the newest `pageSize` messages as they
 * are now. Returns the rows to show and whether more may be loaded below.
 */
export function mergeTopPage<T extends Row>(
  prev: T[],
  page: T[],
  pageSize: number,
): { rows: T[]; hasMore: boolean | null } {
  const before = new Map(prev.map((m) => [m.id, m]));
  const keep = (m: T): T => {
    const old = before.get(m.id);
    return old && sameRow(old, m) ? old : m;
  };
  const fresh = page.map(keep);
  if (prev.length <= pageSize || page.length < pageSize) {
    // Nothing beyond the first page was on screen (or the folder is that
    // small): the fresh page is the whole list.
    return { rows: fresh, hasMore: page.length === pageSize };
  }
  // He had scrolled past the first page: the fresh page in front, and
  // everything older than it that he already had, kept as it was.
  const edge = page[page.length - 1];
  const seen = new Set(fresh.map((m) => m.id));
  const older = prev.filter((m) => olderThan(m, edge) && !seen.has(m.id));
  return { rows: [...fresh, ...older], hasMore: null };
}

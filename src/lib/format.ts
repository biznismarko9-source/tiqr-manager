// Mirrors src-tauri/src/money.rs. All money is handled as integer cents
// end-to-end; these helpers only exist at the UI boundary for display and
// for turning user keystrokes into cents before they ever reach the backend.

import type { SeatEntry } from "./types";

export function centsToDecimalString(cents: number): string {
  const sign = cents < 0 ? "-" : "";
  const abs = Math.round(Math.abs(cents));
  return `${sign}${Math.floor(abs / 100)}.${String(abs % 100).padStart(2, "0")}`;
}

/** Returns null if the input isn't a valid amount (at most 2 decimal places). */
export function decimalStringToCents(input: string): number | null {
  const s = input.trim().replace(",", ".");
  if (s === "") return 0;
  if (!/^-?\d+(\.\d{1,2})?$/.test(s)) return null;
  const neg = s.startsWith("-");
  const unsigned = neg ? s.slice(1) : s;
  const [wholeRaw, fracRaw = ""] = unsigned.split(".");
  const whole = parseInt(wholeRaw || "0", 10);
  const frac = parseInt(fracRaw.padEnd(2, "0").slice(0, 2), 10);
  const total = whole * 100 + frac;
  return neg ? -total : total;
}

export function formatMoney(cents: number | null | undefined, currency: string): string {
  if (cents === null || cents === undefined) return "-";
  try {
    return new Intl.NumberFormat(undefined, {
      style: "currency",
      currency: currency || "EUR",
      currencyDisplay: "narrowSymbol",
    }).format(cents / 100);
  } catch {
    return `${currency} ${centsToDecimalString(cents)}`;
  }
}

/** Same as formatMoney, but for a FinanceSummary-style amount whose currency
 * may be "mixed" (null - the underlying tickets/sales aren't all one
 * currency). Never blends currencies into one number - shows "Mixed" instead
 * so nobody mistakes it for a real total. */
export function formatMoneyOrMixed(cents: number | null | undefined, currency: string | null): string {
  if (currency === null) return "Mixed";
  return formatMoney(cents, currency);
}

export function formatPercent(ratio: number | null | undefined): string {
  if (ratio === null || ratio === undefined || Number.isNaN(ratio)) return "N/A";
  return `${(ratio * 100).toFixed(1)}%`;
}

/** Same as formatPercent, but for a margin/ROI value that came from a group
 * (SaleGroup, Sale Detail header, ...) whose currency may be "mixed" (null -
 * the underlying lines aren't all one currency). A blended ratio across
 * different currencies (e.g. EUR + USD) is mathematically well-formed but
 * economically meaningless, so this shows "Mixed" instead - same convention
 * as formatMoneyOrMixed - rather than the ordinary "N/A" formatPercent shows
 * for a ratio that's null for another reason (e.g. zero revenue). */
export function formatPercentOrMixed(ratio: number | null | undefined, currency: string | null): string {
  if (currency === null) return "Mixed";
  return formatPercent(ratio);
}

/** "vs previous period" delta shown on a Dashboard KPI card (2.0.47,
 * DIR-001 - see REDESIGN-2.0.47-REPORT.md). `direction` drives the
 * arrow icon; `label` is already fully formatted, never a raw number the
 * caller has to format again. */
export interface TrendInfo {
  direction: "up" | "down" | "flat";
  label: string;
}

/** "vs previous period" delta for a plain magnitude (money cents, ticket
 * counts) - a generic relative percent change, not metric-specific:
 * `(current - previous) / |previous| * 100`. `previous` null/undefined
 * means there's no comparison to make (e.g. period = "All time" - see
 * `previousPeriod` on DashboardData) - returns null so the caller renders
 * no trend line rather than a misleading one. `previous === 0` is its own
 * case (a plain percent change from zero is undefined) - shown as "New",
 * with direction still following `current`'s own sign (e.g. Profit going
 * from a break-even 0 to a loss is a "New" DOWN, not an "up" - this must
 * not default to "up" just because a real percentage can't be computed),
 * otherwise "No change".
 *
 * For a value that's already a ratio (Margin/ROI), use `computeTrendPoints`
 * instead - a relative percent-of-a-percent change reads as confusingly
 * large for those (see that function's own doc comment). */
export function computeTrend(current: number, previous: number | null | undefined): TrendInfo | null {
  if (previous === null || previous === undefined) return null;
  if (previous === 0) {
    if (current === 0) return { direction: "flat", label: "No change" };
    return { direction: current > 0 ? "up" : "down", label: "New" };
  }
  const pct = ((current - previous) / Math.abs(previous)) * 100;
  if (Math.abs(pct) < 0.05) return { direction: "flat", label: "No change" };
  return { direction: pct > 0 ? "up" : "down", label: `${Math.abs(pct).toFixed(1)}%` };
}

/** Same "vs previous period" concept as `computeTrend`, but for a value
 * that's ALREADY a ratio (Margin/ROI, both 0-1 fractions here) - shows a
 * percentage-POINT delta (`current - previous`, e.g. "+3.2pp") instead of a
 * relative percent-of-a-percent change. A margin that moved from 20% to 30%
 * is "+10.0pp"; `computeTrend` would instead call that "+50.0%", which reads
 * next to a "30%" value as if margin jumped to ~45-73%. Same null-handling
 * as `computeTrend`: either side missing (no previous period, or a ratio
 * that's null because its own period had zero revenue - see safe_ratio in
 * finance.rs) means no comparison, not a misleading one. */
export function computeTrendPoints(current: number | null, previous: number | null | undefined): TrendInfo | null {
  if (current === null || previous === null || previous === undefined) return null;
  const deltaPoints = (current - previous) * 100;
  if (Math.abs(deltaPoints) < 0.05) return { direction: "flat", label: "No change" };
  return { direction: deltaPoints > 0 ? "up" : "down", label: `${Math.abs(deltaPoints).toFixed(1)}pp` };
}

export function formatDate(iso: string | null | undefined): string {
  if (!iso) return "-";
  const d = new Date(iso.length <= 10 ? `${iso}T00:00:00` : iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" });
}

export function formatDateTime(iso: string | null | undefined): string {
  if (!iso) return "-";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function todayIso(): string {
  const d = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

export function titleCase(s: string | null | undefined): string {
  if (!s) return "";
  return s.charAt(0).toUpperCase() + s.slice(1).replace(/_/g, " ");
}

/** 2.0.38: marko's explicitly requested date shape for every list-table Date
 * column - full 4-digit year, zero-padded, dot-separated, e.g. "11.09.2026"
 * (day.month.year - standard Slovak/European convention, matching exactly
 * what he typed when asking for this). Deliberately NOT locale-dependent
 * (unlike formatDate's toLocaleDateString call) - he gave an exact literal
 * format, so this always produces that one shape regardless of the OS
 * locale, rather than only happening to match it. Used everywhere a table
 * shows its own dedicated Date column - see PROTECTED-AREAS-NOTES.md's
 * 2.0.38 section for the full list. Replaces the old formatDateCompact
 * helper (deleted this version, was down to zero remaining call sites once
 * every one of those columns switched here) - its abbreviated "11 sep 26"
 * shape is exactly what marko asked to stop seeing, so there was no reason
 * to keep it around for a future column to accidentally pick back up. */
export function formatDateNumeric(iso: string | null | undefined): string {
  if (!iso) return "-";
  const d = new Date(iso.length <= 10 ? `${iso}T00:00:00` : iso);
  if (Number.isNaN(d.getTime())) return iso;
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${pad(d.getDate())}.${pad(d.getMonth() + 1)}.${d.getFullYear()}`;
}

/** 1.8.2: combines a ticket's section/row/seat into one compact string for
 * Sale Detail's merged "Seat" column (was 3 separate columns). Omits
 * whichever parts are null (general-admission tickets often have none) and
 * falls back to "General admission" when all three are missing, rather than
 * a bare "-" that could be mistaken for missing data.
 *
 * 2.2.10: dropped the "Sec"/"Row"/"Seat" prefixes marko's own 2.2.9 request
 * had just added - a real section value is sometimes already a full label
 * on its own (e.g. "Category D, Standing", or literally "Sec 408"), and
 * gluing another "Sec " in front of that read as broken ("Sec Sec 408",
 * "Sec Category D, Stan..."). Now just the raw values, "·"-joined - e.g.
 * "402 · 56 · 27" - so whatever is actually stored is exactly what's shown,
 * never relabeled. */
export function formatSeatLocation(
  section: string | null | undefined,
  rowLabel: string | null | undefined,
  seat: string | null | undefined,
): string {
  const parts = [section, rowLabel, seat].filter((p): p is string => !!p);
  return parts.length > 0 ? parts.join(" · ") : "General admission";
}

/** Collapses a group's seat labels into one compact string: consecutive pure
 * numbers become a range ("128-131"), everything else is naturally sorted
 * and comma-joined ("A1, A2, A10" - not the plain-string-sort "A1, A10, A2").
 * Empty input -> "". Helper for `formatSeatsSummary` below, not exported -
 * nothing else needs "just the seat numbers, compacted" on its own. */
function compactSeatList(seatLabels: string[]): string {
  if (seatLabels.length === 0) return "";
  const numeric: number[] = [];
  const other: string[] = [];
  for (const s of seatLabels) {
    if (/^\d+$/.test(s)) numeric.push(parseInt(s, 10));
    else other.push(s);
  }
  numeric.sort((a, b) => a - b);
  other.sort((a, b) => a.localeCompare(b, undefined, { numeric: true }));

  const ranges: string[] = [];
  let i = 0;
  while (i < numeric.length) {
    let j = i;
    while (j + 1 < numeric.length && numeric[j + 1] === numeric[j] + 1) j++;
    ranges.push(i === j ? `${numeric[i]}` : `${numeric[i]}-${numeric[j]}`);
    i = j + 1;
  }
  return [...ranges, ...other].join(", ");
}

/** 2.0.38: turns a full `SeatEntry[]` (OrderRecord.seats/SaleGroup.seats)
 * into one compact display string for the "Seats" column on Orders/Tickets/
 * Sales/Inventory/Pulls. Groups by section+row first (tickets bought/sold
 * together are almost always the same section/row - see compactSeatList
 * above for how the seat numbers within a group get shortened), falls back
 * to "General admission" for a group with no section/row/seat at all, and
 * comma-joins multiple truly distinct groups with "; " so a rare mixed-
 * section order doesn't read as one run-on list. Always returns the FULL
 * string, uncut - same convention as every other formatter here; truncation/
 * tooltip is a display concern handled where this is rendered (the usual
 * `truncate` class + `title={...}` pattern).
 *
 * 2.2.9: each group used to read "204/AA 128-131" (marko's own original
 * example) - marko's request was to remove that "/" and clearly separate
 * section/row/seat instead, so this reused formatSeatLocation's dot-
 * separated convention (already established above for the single-ticket
 * detail views) instead of a bare slash-joined pair.
 *
 * 2.2.10: formatSeatLocation itself dropped its "Sec"/"Row"/"Seat" labels
 * (see that function's own doc comment - real section text sometimes
 * already reads as a full label on its own, and the prefix duplicated it).
 * This function reuses that same convention unchanged, so a group now reads
 * as e.g. "402 · 56 · 27" rather than "Sec 402 · Row 56 · Seat 27" - only
 * the labeling changed here too; the grouping/compaction logic below is
 * unchanged. */
export function formatSeatsSummary(seats: SeatEntry[]): string {
  if (seats.length === 0) return "-";

  const groups = new Map<string, { section: string | null; rowLabel: string | null; seatNums: string[] }>();
  for (const s of seats) {
    const key = `${s.section ?? ""}\0${s.rowLabel ?? ""}`;
    let g = groups.get(key);
    if (!g) {
      g = { section: s.section, rowLabel: s.rowLabel, seatNums: [] };
      groups.set(key, g);
    }
    if (s.seat) g.seatNums.push(s.seat);
  }

  const parts = Array.from(groups.values()).map((g) => {
    const seatPart = compactSeatList(g.seatNums);
    const raw = [g.section, g.rowLabel, seatPart].filter((p): p is string => !!p);
    return raw.length > 0 ? raw.join(" · ") : "General admission";
  });

  return parts.join("; ");
}

/** 2.0.28: turns a `BulkDeleteResult.skipped` array into one short line for
 * a toast, e.g. "2x This order has sold tickets and cannot be deleted. 1x
 * This order has sales history (including refunds) and cannot be deleted."
 * Groups by the exact reason text rather than listing every id individually
 * - a bulk delete that skips several orders for the SAME reason (the common
 * case) would otherwise repeat that sentence once per id, which reads as
 * noise rather than information. Shared by every list page with the new
 * "Delete" selection mode (Pulls both tabs/Orders/Events/Sales) so they all
 * summarize skips identically. */
export function summarizeBulkDeleteSkips(skipped: { id: number; reason: string }[]): string {
  const counts = new Map<string, number>();
  for (const s of skipped) {
    counts.set(s.reason, (counts.get(s.reason) ?? 0) + 1);
  }
  return Array.from(counts.entries())
    .map(([reason, n]) => `${n}x ${reason}`)
    .join(" ");
}

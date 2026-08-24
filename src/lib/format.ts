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
 * a bare "-" that could be mistaken for missing data. */
export function formatSeatLocation(
  section: string | null | undefined,
  rowLabel: string | null | undefined,
  seat: string | null | undefined,
): string {
  const parts = [
    section ? `Sec ${section}` : null,
    rowLabel ? `Row ${rowLabel}` : null,
    seat ? `Seat ${seat}` : null,
  ].filter((p): p is string => p !== null);
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
 * into one compact display string for the new "Seats" column on
 * Orders/Tickets/Sales - e.g. "204/AA 128-131", marko's own example when
 * asked how a multi-ticket row (one order/sale can cover several different
 * seats at once) should summarize them. Groups by section+row first (tickets
 * bought/sold together are almost always the same section/row - see
 * compactSeatList above for how the seat numbers within a group get
 * shortened), falls back to "General admission" for a group with no
 * section/row/seat at all, and comma-joins multiple truly distinct groups
 * with "; " so a rare mixed-section order doesn't read as one run-on list.
 * Always returns the FULL string, uncut - same convention as every other
 * formatter here; truncation/tooltip is a display concern handled where this
 * is rendered (the usual `truncate` class + `title={...}` pattern). */
export function formatSeatsSummary(seats: SeatEntry[]): string {
  if (seats.length === 0) return "-";

  const groups = new Map<string, { section: string | null; rowLabel: string | null; seatNums: string[] }>();
  for (const s of seats) {
    const key = `${s.section ?? ""} ${s.rowLabel ?? ""}`;
    let g = groups.get(key);
    if (!g) {
      g = { section: s.section, rowLabel: s.rowLabel, seatNums: [] };
      groups.set(key, g);
    }
    if (s.seat) g.seatNums.push(s.seat);
  }

  const parts = Array.from(groups.values()).map((g) => {
    const seatPart = compactSeatList(g.seatNums);
    if (g.section && g.rowLabel) return seatPart ? `${g.section}/${g.rowLabel} ${seatPart}` : `${g.section}/${g.rowLabel}`;
    if (g.section) return seatPart ? `Sec ${g.section} ${seatPart}` : `Sec ${g.section}`;
    if (g.rowLabel) return seatPart ? `Row ${g.rowLabel} ${seatPart}` : `Row ${g.rowLabel}`;
    return seatPart ? `Seat ${seatPart}` : "General admission";
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

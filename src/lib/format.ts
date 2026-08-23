// Mirrors src-tauri/src/money.rs. All money is handled as integer cents
// end-to-end; these helpers only exist at the UI boundary for display and
// for turning user keystrokes into cents before they ever reach the backend.

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

/** 1.8.2: a shorter date for narrow table columns (Sales list) - e.g. "15 Aug
 * 26". Month is a short NAME (never a bare number) so it never reads as
 * ambiguous DD/MM vs MM/DD, and the 2-digit year is kept (not dropped) since
 * this app tracks sales across multiple years and two same-day-different-year
 * sales must still look different in a list. Prefer `formatDate` everywhere
 * space isn't tight. 2.0.30: also used by Pulls' Date column (both tabs) for
 * the same reason - no longer Sales-only, but still not the default. */
export function formatDateCompact(iso: string | null | undefined): string {
  if (!iso) return "-";
  const d = new Date(iso.length <= 10 ? `${iso}T00:00:00` : iso);
  if (Number.isNaN(d.getTime())) return iso;
  const day = d.getDate();
  const month = d.toLocaleDateString(undefined, { month: "short" });
  const year = String(d.getFullYear()).slice(-2);
  return `${day} ${month} ${year}`;
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

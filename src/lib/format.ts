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

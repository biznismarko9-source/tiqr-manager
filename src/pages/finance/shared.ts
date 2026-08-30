// 2.1.0: shared by Finance.tsx and every finance/*.tsx tab - the same
// time/scope filter concepts the original single-page Finance.tsx used
// (unchanged), now shared across the tabs that both offer them (marko's own
// point 2: "Zachovaj existujúce time filters"; point 11 lists "Date"/
// "Personal, Business" among Transactions' own filters too), plus the one
// data bundle every tab is handed so the container and its tabs never
// import from each other directly.
import type { Account, FinanceCategory, FinanceEntry, RecurringExpense, Transfer } from "../../lib/types";
import { todayIso } from "../../lib/format";

/** Everything Finance.tsx loads once and hands down to whichever tab is
 * active - one shared `reload()` so any mutation (a new entry, a new
 * transfer, pausing a recurring template...) refreshes every tab at once,
 * not just the one that triggered it. */
export interface FinanceData {
  entries: FinanceEntry[];
  categories: FinanceCategory[];
  accounts: Account[];
  transfers: Transfer[];
  recurringExpenses: RecurringExpense[];
  loading: boolean;
  reload: () => void;
}

// marko's own original ask was specifically "daily/monthly/yearly"
// (denný/mesačný/ročný) - calendar periods, not Dashboard's trailing
// "last N" windows (its "1 Mo"/"3 Mo" pills).
export type PeriodKey = "today" | "month" | "year" | "all" | "custom";

export const PERIODS: { key: PeriodKey; label: string }[] = [
  { key: "today", label: "Today" },
  { key: "month", label: "This month" },
  { key: "year", label: "This year" },
  { key: "all", label: "All time" },
  { key: "custom", label: "Custom" },
];

export function periodBounds(period: PeriodKey, customFrom: string, customTo: string): { from: string | null; to: string | null } {
  const today = todayIso();
  if (period === "today") return { from: today, to: today };
  if (period === "month") return { from: `${today.slice(0, 7)}-01`, to: today };
  if (period === "year") return { from: `${today.slice(0, 4)}-01-01`, to: today };
  if (period === "custom") return { from: customFrom || null, to: customTo || null };
  return { from: null, to: null };
}

export type ScopeFilter = "all" | "personal" | "business";

export const SCOPES: { key: ScopeFilter; label: string }[] = [
  { key: "all", label: "All" },
  { key: "personal", label: "Personal" },
  { key: "business", label: "Business" },
];

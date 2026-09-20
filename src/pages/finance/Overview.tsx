import { useEffect, useMemo, useState } from "react";
import { api, errMsg } from "../../lib/api";
import type { CashflowForecast, FinanceEntry, FinanceEntryInput, RevenueTimeSeriesPoint } from "../../lib/types";
import { formatMoney, formatMoneyOrMixed } from "../../lib/format";
import {
  Button,
  Card,
  ConfirmDialog,
  EmptyState,
  Input,
  PanelSkeleton,
  SEGMENTED_TRACK,
  segmentedItemClass,
  StatCard,
  StatsSkeleton,
} from "../../components/ui";
import { MetricChart, type MetricKey } from "../../components/MetricChart";
import { FinanceCategorySwatch } from "../../components/FinanceCategoryBadge";
import { IconBarChart, IconPlus, IconTrendingUp } from "../../components/icons";
import { useToast } from "../../lib/toast";
import { PERIODS, SCOPES, periodBounds, type FinanceData, type PeriodKey, type ScopeFilter } from "./shared";
import { EntryFormModal } from "./Transactions";
import { AccountFormModal } from "./Accounts";

// 2.1.0: this is the ORIGINAL Finance.tsx's own Overview logic (period/scope
// filtering, stat cards, both charts, the mixed-currency banner/convert
// flow), moved here unchanged, plus three additions marko's own FINANCE 2.1
// spec asked for: a "Current Balance" card (real running total across
// active EUR accounts - distinct from the period-scoped Income/Expenses/Net
// Cash Flow cards right next to it), a "Pending/Outstanding" card (reusing
// the EXISTING Dashboard alert - never recomputed here), and the Cashflow
// Forecast card.
//
// 2.2.9: "New entry"/"New account" quick-action buttons - marko's own
// request, so starting either doesn't require first switching to the
// Transactions/Accounts tab. Reuses those tabs' own EntryFormModal/
// AccountFormModal exactly (now exported from their own files) rather than
// building a second copy of either form - `categories`/`accounts` are
// already part of `FinanceData`, and `reload` already refreshes every tab's
// data at once, so no new data-loading was needed either.

// ---------------------------------------------------------------------------
// Bucketing for the "Income vs Expenses" chart.
//
// 2.46.0: the bucket is a DAY on a short period and a MONTH on a long one -
// the same rule the Dashboard's own series follows. marko: "nech uz tam vidno
// aj nejaky ten graf nieze on je prazdny nech tam je od nejakej po nejaku
// dobu", and he was right: the default period is "This month", which under
// monthly bucketing was exactly ONE bucket, so the chart drew a single dot.
//
// What this does NOT do is widen the window past the selected period to make
// the line longer. `entries` arrives already period-filtered, so a month
// outside the period would draw as zero while real money sat in it - a chart
// that lies. The window IS the period; only the bucket size changes.
//
// Empty buckets inside the window are real zeros, not invented data: nothing
// was booked that day. They are what gives the line an axis to run along.
// ---------------------------------------------------------------------------

interface SeriesBucket {
  /** Always a real calendar date - the day itself, or the 1st of its month.
   *  This is what MetricChart reads as `bucketStart`. */
  bucketStart: string;
  incomeCents: number;
  expenseCents: number;
}

type Granularity = "day" | "month";

function monthKey(iso: string): string {
  return iso.slice(0, 7);
}

function addMonths(key: string, n: number): string {
  const [y, m] = key.split("-").map(Number);
  const total = y * 12 + (m - 1) + n;
  const ny = Math.floor(total / 12);
  const nm = (total % 12) + 1;
  return `${ny}-${String(nm).padStart(2, "0")}`;
}

function addDays(iso: string, n: number): string {
  const d = new Date(`${iso}T00:00:00`);
  d.setDate(d.getDate() + n);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}

function daysApart(a: string, b: string): number {
  const ms = new Date(`${b}T00:00:00`).getTime() - new Date(`${a}T00:00:00`).getTime();
  return Math.round(ms / 86400000);
}

// Past a quarter, one dot per day is a smear rather than a line - that is
// where months take over. Both caps keep an "All time" ledger readable.
const DAY_GRANULARITY_MAX_DAYS = 92;
const MAX_CHART_DAYS = 92;
const MAX_CHART_MONTHS = 24;

function buildSeries(
  entries: FinanceEntry[],
  from: string | null,
  to: string | null,
): { buckets: SeriesBucket[]; granularity: Granularity } {
  const dates = entries.map((e) => e.entryDate).sort();
  // An unbounded period ("All time") takes its window from the ledger itself.
  // With no entries at all there is genuinely nothing to draw, and saying so
  // beats drawing a flat line through a period marko never had.
  const startDay = from ?? dates[0] ?? null;
  const endDay = to ?? dates[dates.length - 1] ?? null;
  if (!startDay || !endDay || endDay < startDay) return { buckets: [], granularity: "month" };

  const granularity: Granularity = daysApart(startDay, endDay) <= DAY_GRANULARITY_MAX_DAYS ? "day" : "month";
  const keyOf = (iso: string) => (granularity === "day" ? iso : monthKey(iso));

  const sums = new Map<string, { incomeCents: number; expenseCents: number }>();
  for (const e of entries) {
    const key = keyOf(e.entryDate);
    const cur = sums.get(key) ?? { incomeCents: 0, expenseCents: 0 };
    if (e.entryType === "income") cur.incomeCents += e.amountCents;
    else cur.expenseCents += e.amountCents;
    sums.set(key, cur);
  }

  const keys: string[] = [];
  if (granularity === "day") {
    for (let c = startDay; c <= endDay; c = addDays(c, 1)) {
      keys.push(c);
      if (keys.length > 1000) break; // sanity guard, never realistically hit
    }
  } else {
    for (let c = monthKey(startDay); c <= monthKey(endDay); c = addMonths(c, 1)) {
      keys.push(c);
      if (keys.length > 1000) break;
    }
  }
  const cap = granularity === "day" ? MAX_CHART_DAYS : MAX_CHART_MONTHS;
  const shown = keys.length > cap ? keys.slice(-cap) : keys;

  return {
    granularity,
    buckets: shown.map((key) => ({
      bucketStart: granularity === "day" ? key : `${key}-01`,
      incomeCents: sums.get(key)?.incomeCents ?? 0,
      expenseCents: sums.get(key)?.expenseCents ?? 0,
    })),
  };
}


interface CategoryBreakdownRow {
  key: string;
  name: string;
  colorSlot: number | null;
  totalCents: number;
}

interface PendingSummary {
  count: number;
  amountCents: number;
  currency: string | null;
}

export default function Overview({ entries, categories, accounts, loading, reload }: FinanceData) {
  const toast = useToast();
  const [period, setPeriod] = useState<PeriodKey>("month");
  const [customFrom, setCustomFrom] = useState("");
  const [customTo, setCustomTo] = useState("");
  const [scopeFilter, setScopeFilter] = useState<ScopeFilter>("all");

  const [convertConfirm, setConvertConfirm] = useState<{ currency: string | null; label: string } | null>(null);
  const [converting, setConverting] = useState(false);

  // 2.2.9: quick-action modals - see this file's own top-of-file doc comment.
  const [entryFormOpen, setEntryFormOpen] = useState(false);
  const [accountFormOpen, setAccountFormOpen] = useState(false);

  const [forecast, setForecast] = useState<CashflowForecast | null>(null);
  const [forecastLoading, setForecastLoading] = useState(true);
  const [pending, setPending] = useState<PendingSummary | null>(null);

  useEffect(() => {
    setForecastLoading(true);
    api
      .getCashflowForecast()
      .then(setForecast)
      .catch((e) => toast.error(errMsg(e)))
      .finally(() => setForecastLoading(false));
    // "ak už existujú relevantné dáta" (point 2) - reuses the EXISTING
    // Dashboard pending-sales alert as-is, never recomputed here. Silently
    // skipped on failure - it's a bonus card, not core to this tab.
    api
      .getDashboard({ period: "all" })
      .then((d) => setPending({ count: d.alerts.pendingSalesCount, amountCents: d.alerts.pendingSalesAmountCents, currency: d.alerts.pendingSalesCurrency }))
      .catch(() => undefined);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const { from, to } = periodBounds(period, customFrom, customTo);
  const customDatesMissing = period === "custom" && !customFrom && !customTo;

  const periodScoped = useMemo(() => {
    if (customDatesMissing) return [];
    return entries.filter((e) => {
      if (from && e.entryDate < from) return false;
      if (to && e.entryDate > to) return false;
      if (scopeFilter !== "all" && e.scope !== scopeFilter) return false;
      return true;
    });
  }, [entries, from, to, scopeFilter, customDatesMissing]);

  const eurScoped = useMemo(() => periodScoped.filter((e) => e.currency === "EUR"), [periodScoped]);
  const excludedNonEurCount = periodScoped.length - eurScoped.length;

  const incomeCents = useMemo(
    () => eurScoped.filter((e) => e.entryType === "income").reduce((s, e) => s + e.amountCents, 0),
    [eurScoped],
  );
  const expenseCents = useMemo(
    () => eurScoped.filter((e) => e.entryType === "expense").reduce((s, e) => s + e.amountCents, 0),
    [eurScoped],
  );
  const netCashFlowCents = incomeCents - expenseCents;

  // Real running total across every active EUR account - distinct from
  // netCashFlowCents above (which is period-scoped income-expenses); this
  // is "what I actually have right now", same figure the Accounts tab and
  // the Forecast card's own "Current balance" line both show.
  const currentBalanceCents = useMemo(
    () => accounts.filter((a) => a.currency === "EUR" && a.isActive).reduce((s, a) => s + a.currentBalanceCents, 0),
    [accounts],
  );
  const hasNonEurAccount = useMemo(() => accounts.some((a) => a.currency !== "EUR" && a.isActive), [accounts]);

  /* 2.29.5 - the gap that made Finance look like it miscounted.
   *
   * An entry's account is OPTIONAL, and the three screens then quietly use
   * three different populations of the same rows: this page's Income/Expenses
   * count every EUR entry, the account balances only move for entries that
   * have an account, and Reports skips account-less entries outright. Each is
   * correct for the question it answers, and nothing on screen said so - so
   * "Expenses" and the balances below simply refused to reconcile.
   *
   * Not fixed by changing which rows count - that would silently alter a
   * figure marko has been reading. Fixed by naming the difference, the same
   * way `excludedNonEurCount` right below already handles non-EUR entries. */
  const unassigned = useMemo(() => {
    const rows = eurScoped.filter((e) => e.accountId === null);
    return {
      count: rows.length,
      cents: rows.reduce((sum, e) => sum + (e.entryType === "expense" ? -e.amountCents : e.amountCents), 0),
    };
  }, [eurScoped]);

  // Non-EUR currencies present ANYWHERE in the ledger, not just the current
  // period/scope filter - same "always show the real global picture" scope
  // Dashboard's own MixedCurrencyBanner uses for orders.
  const nonEurCurrencies = useMemo(() => {
    const map = new Map<string, number>();
    for (const e of entries) {
      if (e.currency !== "EUR") map.set(e.currency, (map.get(e.currency) ?? 0) + 1);
    }
    return Array.from(map.entries()).map(([currency, count]) => ({ currency, count }));
  }, [entries]);

  const categoryBreakdown = useMemo<CategoryBreakdownRow[]>(() => {
    const map = new Map<string, CategoryBreakdownRow>();
    for (const e of eurScoped) {
      if (e.entryType !== "expense") continue;
      const key = e.categoryId ? String(e.categoryId) : "none";
      const cur = map.get(key) ?? {
        key,
        name: e.categoryName ?? "No category",
        colorSlot: e.categoryColorSlot,
        totalCents: 0,
      };
      cur.totalCents += e.amountCents;
      map.set(key, cur);
    }
    return Array.from(map.values()).sort((a, b) => b.totalCents - a.totalCents);
  }, [eurScoped]);

  const series = useMemo(() => buildSeries(eurScoped, from, to), [eurScoped, from, to]);

  // Reuses the existing generic `convertCurrency` command as-is (same one
  // the Dashboard's own mixed-currency banner and the New Order form already
  // call) - fetches one live rate per currency, batched across every entry
  // in that currency, then persists each converted amount via
  // `updateFinanceEntry`. No dedicated backend command for this.
  const runConversion = async () => {
    if (!convertConfirm) return;
    const targets = convertConfirm.currency ? [convertConfirm.currency] : nonEurCurrencies.map((c) => c.currency);
    setConverting(true);
    let convertedCount = 0;
    const failures: string[] = [];
    for (const cur of targets) {
      const matching = entries.filter((e) => e.currency === cur);
      if (matching.length === 0) continue;
      try {
        const result = await api.convertCurrency(cur, "EUR", matching.map((e) => e.amountCents));
        for (let i = 0; i < matching.length; i++) {
          const e = matching[i];
          const input: FinanceEntryInput = {
            entryType: e.entryType,
            entryDate: e.entryDate,
            amountCents: result.convertedCents[i],
            currency: "EUR",
            scope: e.scope,
            categoryId: e.categoryId,
            accountId: e.accountId,
            // 2.2.1: carry the order link through unchanged - this is a
            // currency re-save of an existing entry, not a new one, so a
            // link to an order (if any) must survive a conversion exactly
            // like every other field here that isn't the amount/currency.
            orderId: e.orderId,
            place: e.place,
            note: e.note,
          };
          await api.updateFinanceEntry(e.id, input);
          convertedCount++;
        }
      } catch (err) {
        failures.push(`${cur}: ${errMsg(err)}`);
      }
    }
    setConverting(false);
    setConvertConfirm(null);
    reload();
    if (convertedCount > 0) toast.success(`${convertedCount} entr${convertedCount === 1 ? "y" : "ies"} converted to EUR.`);
    if (failures.length > 0) toast.error(`Some currencies could not be converted: ${failures.join("; ")}`);
  };

  return (
    <div>
      {(nonEurCurrencies.length > 0 || hasNonEurAccount) && (
        <div className="mb-4 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-800 dark:border-amber-500/30 dark:bg-amber-500/10 dark:text-amber-400">
          <div className="flex flex-wrap items-center gap-2">
            <span className="font-medium">
              You have entries in another currency - totals below only include EUR. Convert to EUR:
            </span>
            {nonEurCurrencies.map((c) => (
              <button
                key={c.currency}
                type="button"
                className="rounded border border-amber-300 bg-surface px-2 py-0.5 font-medium text-amber-800 hover:bg-amber-100 dark:border-amber-500/40 dark:bg-slate-900 dark:text-amber-400 dark:hover:bg-amber-500/10"
                onClick={() => setConvertConfirm({ currency: c.currency, label: c.currency })}
              >
                {c.currency} ({c.count})
              </button>
            ))}
            {nonEurCurrencies.length > 1 && (
              <button
                type="button"
                className="rounded border border-amber-300 bg-surface px-2 py-0.5 font-medium text-amber-800 hover:bg-amber-100 dark:border-amber-500/40 dark:bg-slate-900 dark:text-amber-400 dark:hover:bg-amber-500/10"
                onClick={() => setConvertConfirm({ currency: null, label: nonEurCurrencies.map((c) => c.currency).join(", ") })}
              >
                All
              </button>
            )}
            {hasNonEurAccount && <span>Non-EUR accounts are shown on the Accounts tab, not included in Current Balance here.</span>}
          </div>
        </div>
      )}

      <Card className="mb-4 flex flex-wrap items-center gap-3 p-3">
        <div className="flex flex-wrap items-center gap-1 rounded-lg border border-slate-200 dark:border-slate-800 bg-surface dark:bg-slate-900 p-1">
          {PERIODS.map((p) => (
            <button
              key={p.key}
              onClick={() => setPeriod(p.key)}
              className={`rounded-md px-2.5 py-1.5 text-xs font-medium transition-colors ${
                period === p.key ? "bg-brand-600 text-white" : "text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800"
              }`}
            >
              {p.label}
            </button>
          ))}
        </div>
        <div className="flex flex-wrap items-center gap-1 rounded-lg border border-slate-200 dark:border-slate-800 bg-surface dark:bg-slate-900 p-1">
          {SCOPES.map((s) => (
            <button
              key={s.key}
              onClick={() => setScopeFilter(s.key)}
              className={`rounded-md px-2.5 py-1.5 text-xs font-medium transition-colors ${
                scopeFilter === s.key ? "bg-brand-600 text-white" : "text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800"
              }`}
            >
              {s.label}
            </button>
          ))}
        </div>
        {period === "custom" && (
          <div className="flex flex-wrap items-end gap-3">
            <label className="text-xs font-medium text-slate-600 dark:text-slate-400">
              From
              <Input type="date" value={customFrom} onChange={(e) => setCustomFrom(e.target.value)} className="mt-1" />
            </label>
            <label className="text-xs font-medium text-slate-600 dark:text-slate-400">
              To
              <Input type="date" value={customTo} onChange={(e) => setCustomTo(e.target.value)} className="mt-1" />
            </label>
          </div>
        )}
        <div className="ml-auto flex items-center gap-2">
          {/* Same variant="primary" the real "New entry"/"New account"
              buttons already use on Transactions/Accounts - these open the
              exact same modals, just from a second, more convenient spot. */}
          <Button variant="primary" onClick={() => setEntryFormOpen(true)}>
            <IconPlus className="h-4 w-4" /> New entry
          </Button>
          <Button variant="primary" onClick={() => setAccountFormOpen(true)}>
            <IconPlus className="h-4 w-4" /> New account
          </Button>
        </div>
      </Card>

      {loading ? (
        <>
          {/* 2.40.0: see Dashboard - skeleton in the shape of what loads. */}
          <StatsSkeleton count={5} />
          <PanelSkeleton lines={5} />
        </>
      ) : customDatesMissing ? (
        <div className="mb-6 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-800 dark:border-amber-500/30 dark:bg-amber-500/10 dark:text-amber-400">
          Please select at least one date.
        </div>
      ) : (
        <>
          {/* 2.13.0 (FIN-01): two bands, because these are two different kinds
              of number and the old single row hid that. Balance and what you
              are owed are STOCKS - what exists right now - and the period
              filter does not touch them; income, expenses and net are FLOWS
              measured over whatever period is selected. Sitting in one row at
              one size, a EUR 68k balance and a EUR 900 monthly expense read as
              peers, and changing the period appeared to do nothing to half the
              row for no visible reason. Same five figures, same order within
              each band - only the grouping and the two labels are new. */}
          <p className="section-title mb-2">What you have · not affected by the period filter</p>
          <div className="summary-bar">
            <StatCard label="Current Balance" value={formatMoney(currentBalanceCents, "EUR")} sub="Across active EUR accounts" />
            {pending && pending.count > 0 && (
              <StatCard
                label="Owed to you"
                value={formatMoneyOrMixed(pending.amountCents, pending.currency)}
                sub={`${pending.count} unpaid sale${pending.count === 1 ? "" : "s"}`}
              />
            )}
          </div>
          <p className="section-title mb-2">What moved · in the selected period</p>
          <div className="summary-bar">
            <StatCard label="Income" value={formatMoney(incomeCents, "EUR")} />
            <StatCard label="Expenses" value={formatMoney(expenseCents, "EUR")} />
            <StatCard
              label="Net Cash Flow"
              value={formatMoney(netCashFlowCents, "EUR")}
              tone={netCashFlowCents > 0 ? "positive" : netCashFlowCents < 0 ? "negative" : "default"}
            />
          </div>
          {unassigned.count > 0 && (
            <p className="-mt-4 mb-2 text-xs text-slate-500 dark:text-slate-400">
              {unassigned.count} of these entr{unassigned.count === 1 ? "y has" : "ies have"} no account, so{" "}
              {unassigned.count === 1 ? "it is" : "they are"} counted here but move no balance - a net{" "}
              {formatMoney(unassigned.cents, "EUR")} difference between this block and the accounts above.
            </p>
          )}
          {excludedNonEurCount > 0 && (
            <p className="-mt-4 mb-6 text-xs text-slate-500 dark:text-slate-400">
              {excludedNonEurCount} entr{excludedNonEurCount === 1 ? "y" : "ies"} in this period{" "}
              {excludedNonEurCount === 1 ? "isn't" : "aren't"} in EUR yet, so{" "}
              {excludedNonEurCount === 1 ? "it isn't" : "they aren't"} included above - convert with the banner up top.
            </p>
          )}

          <div className="mb-6 grid grid-cols-1 gap-5 lg:grid-cols-2">
            <CategoryBreakdownCard rows={categoryBreakdown} />
            <Card className="p-4">
              <IncomeExpenseChart
                buckets={series.buckets}
                granularity={series.granularity}
                incomeCents={incomeCents}
                expenseCents={expenseCents}
              />
            </Card>
          </div>

          <ForecastCard forecast={forecast} loading={forecastLoading} />
        </>
      )}

      <ConfirmDialog
        open={convertConfirm !== null}
        title="Convert to EUR?"
        message={`Fetches today's live conversion rate(s) to EUR and converts every entry currently in ${convertConfirm?.label}. This cannot be undone.`}
        confirmLabel="Convert to EUR"
        danger
        busy={converting}
        onCancel={() => setConvertConfirm(null)}
        onConfirm={runConversion}
      />

      <EntryFormModal
        open={entryFormOpen}
        onClose={() => setEntryFormOpen(false)}
        onSaved={reload}
        categories={categories}
        accounts={accounts}
        initial={null}
      />
      <AccountFormModal open={accountFormOpen} onClose={() => setAccountFormOpen(false)} onSaved={reload} initial={null} />
    </div>
  );
}

// ---------------------------------------------------------------------------
// Charts - small, hand-rolled, no charting library.
// ---------------------------------------------------------------------------

function CategoryBreakdownCard({ rows }: { rows: CategoryBreakdownRow[] }) {
  const maxVal = Math.max(1, ...rows.map((r) => r.totalCents));
  return (
    <Card>
      <div className="flex items-center gap-2 border-b border-slate-100 dark:border-slate-800 px-4 py-3">
        <IconBarChart className="h-4 w-4 text-slate-500 dark:text-slate-400" />
        <h3 className="text-sm font-semibold text-slate-800 dark:text-slate-200">Expenses by category</h3>
      </div>
      {rows.length === 0 ? (
        <div className="p-4">
          <EmptyState title="No expenses in this period yet" />
        </div>
      ) : (
        <ul className="divide-y divide-slate-100 dark:divide-slate-800">
          {rows.map((r) => (
            <li key={r.key} className="px-4 py-2.5">
              <div className="mb-1.5 flex items-center justify-between gap-2">
                <span className="flex min-w-0 items-center gap-1.5 truncate text-sm font-medium text-slate-800 dark:text-slate-200">
                  {r.colorSlot !== null && <FinanceCategorySwatch colorSlot={r.colorSlot} />}
                  {r.name}
                </span>
                <span className="shrink-0 text-sm tabular-nums text-slate-600 dark:text-slate-400">{formatMoney(r.totalCents, "EUR")}</span>
              </div>
              <div className="h-1.5 rounded-full bg-slate-100 dark:bg-slate-800">
                <div
                  className="h-1.5 rounded-full bg-brand-500"
                  style={{ width: `${Math.max(4, (r.totalCents / maxVal) * 100)}%` }}
                />
              </div>
            </li>
          ))}
        </ul>
      )}
    </Card>
  );
}

// 2.45.0: marko - "vo finance ako je income vs expense widget tak urobit
// taky isty graf ako ktory je na dashboarde v overview". So this is now
// literally the Dashboard's chart: the same MetricChart component, the same
// segmented pill row, the same big number above it. What changed is only the
// wording - a ledger calls these three series Income / Expenses / Net, and
// MetricChart takes those as `labels` (see its 2.45.0 prop comment).
//
// The numbers are the SAME ones the paired bar chart drew: each bucket maps
// onto one RevenueTimeSeriesPoint, which is the shape MetricChart already
// reads. `profitCents` is income - expenses, i.e. exactly the "Net Cash Flow"
// card above, per bucket. `sellingFeesCents` and `soldTickets` have no meaning
// in a ledger and are never plotted here (no "Sales" pill), so they are zero
// rather than invented.
const FINANCE_METRICS: { key: MetricKey; label: string }[] = [
  { key: "revenue", label: "Income" },
  { key: "cost", label: "Expenses" },
  { key: "profit", label: "Net" },
];

const FINANCE_METRIC_LABELS: Partial<Record<MetricKey, string>> = {
  revenue: "Income",
  cost: "Expenses",
  profit: "Net",
};

function IncomeExpenseChart({
  buckets,
  granularity,
  incomeCents,
  expenseCents,
}: {
  buckets: SeriesBucket[];
  granularity: Granularity;
  incomeCents: number;
  expenseCents: number;
}) {
  const [metric, setMetric] = useState<MetricKey>("revenue");
  const points = useMemo<RevenueTimeSeriesPoint[]>(
    () =>
      buckets.map((b) => ({
        bucketStart: b.bucketStart,
        revenueCents: b.incomeCents,
        sellingFeesCents: 0,
        cogsCents: b.expenseCents,
        soldTickets: 0,
        profitCents: b.incomeCents - b.expenseCents,
      })),
    [buckets],
  );
  // The headline number is the period's own total, handed down from the
  // cards above - NOT a sum of the plotted buckets. `buildSeries` caps an
  // "All time" ledger at the most recent 24 months (or 92 days), so summing
  // the plotted points would quietly disagree with the Income/Expenses/Net
  // Cash Flow cards on the same screen.
  const total = metric === "cost" ? expenseCents : metric === "profit" ? incomeCents - expenseCents : incomeCents;

  return (
    <div>
      <div className="mb-3 flex flex-wrap items-start justify-between gap-3">
        <div>
          <p className="text-xs font-medium uppercase tracking-wide text-slate-500 dark:text-slate-400">
            {FINANCE_METRIC_LABELS[metric]} {granularity === "day" ? "by day" : "by month"}
          </p>
          <p
            className={`mt-1.5 text-[22px] font-semibold leading-none tabular-nums ${
              metric !== "profit"
                ? ""
                : total > 0
                  ? "text-emerald-600 dark:text-emerald-400"
                  : total < 0
                    ? "text-red-600 dark:text-red-400"
                    : ""
            }`}
          >
            {formatMoney(total, "EUR")}
          </p>
        </div>
        <div className={SEGMENTED_TRACK}>
          {FINANCE_METRICS.map((m) => (
            <button
              key={m.key}
              onClick={() => setMetric(m.key)}
              aria-pressed={metric === m.key}
              className={segmentedItemClass(metric === m.key)}
            >
              {m.label}
            </button>
          ))}
        </div>
      </div>
      <MetricChart
        points={points}
        granularity={granularity}
        currency="EUR"
        metric={metric}
        labels={FINANCE_METRIC_LABELS}
        emptyLabel="No entries in this period yet."
      />
    </div>
  );
}

// ---------------------------------------------------------------------------
// Cashflow Forecast (2.1.0) - a simple, non-AI projection built entirely
// server-side (commands::finance_forecast) from data already in the app.
// Clearly labeled FORECAST throughout (marko's own point 9: never let this
// be mistaken for an actual/current balance).
// ---------------------------------------------------------------------------

function ForecastCard({ forecast, loading }: { forecast: CashflowForecast | null; loading: boolean }) {
  return (
    <Card className="mt-6 p-4">
      <div className="mb-3 flex items-center gap-2">
        <IconTrendingUp className="h-4 w-4 text-slate-500 dark:text-slate-400" />
        <h3 className="text-sm font-semibold text-slate-800 dark:text-slate-200">Cashflow Forecast</h3>
        <span className="rounded-full bg-brand-50 px-2 py-0.5 text-[10px] font-bold uppercase tracking-wide text-brand-700 dark:bg-brand-500/10 dark:text-brand-400">
          Forecast
        </span>
      </div>
      {loading ? (
        <PanelSkeleton lines={3} className="shadow-none" />
      ) : !forecast || !forecast.available ? (
        <EmptyState title="Forecast unavailable" description="Add an active EUR account on the Accounts tab to see a forecast." />
      ) : (
        <div>
          <dl className="grid grid-cols-2 gap-x-6 gap-y-2 text-sm sm:grid-cols-4">
            <div>
              <dt className="text-xs text-slate-500 dark:text-slate-400">Current balance</dt>
              <dd className="tabular-nums text-slate-800 dark:text-slate-200">{formatMoney(forecast.currentBalanceCents, "EUR")}</dd>
            </div>
            <div>
              <dt className="text-xs text-slate-500 dark:text-slate-400">Expected income</dt>
              <dd className="tabular-nums text-emerald-600 dark:text-emerald-400">+{formatMoney(forecast.expectedIncomeCents, "EUR")}</dd>
            </div>
            <div>
              <dt className="text-xs text-slate-500 dark:text-slate-400">Recurring expenses</dt>
              <dd className="tabular-nums text-rose-600 dark:text-rose-400">-{formatMoney(forecast.recurringExpensesCents, "EUR")}</dd>
            </div>
            <div>
              <dt className="text-xs text-slate-500 dark:text-slate-400">Upcoming expenses</dt>
              <dd className="tabular-nums text-rose-600 dark:text-rose-400">-{formatMoney(forecast.upcomingExpensesCents, "EUR")}</dd>
            </div>
          </dl>
          <div className="mt-3 flex items-center justify-between border-t border-slate-100 dark:border-slate-800 pt-3">
            <span className="text-sm font-semibold text-slate-800 dark:text-slate-200">Forecast balance (next {forecast.windowDays} days)</span>
            <span
              className={`text-lg font-bold tabular-nums ${
                forecast.forecastBalanceCents >= forecast.currentBalanceCents ? "text-emerald-600 dark:text-emerald-400" : "text-rose-600 dark:text-rose-400"
              }`}
            >
              {formatMoney(forecast.forecastBalanceCents, "EUR")}
            </span>
          </div>
          {forecast.excludesNonEurData && (
            <p className="mt-2 text-xs text-slate-500 dark:text-slate-400">
              Some non-EUR balances, sales or entries exist and aren't included above (no exchange rate is guessed).
            </p>
          )}
        </div>
      )}
    </Card>
  );
}

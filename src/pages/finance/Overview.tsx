import { useEffect, useMemo, useState } from "react";
import { api, errMsg } from "../../lib/api";
import type { CashflowForecast, FinanceEntry, FinanceEntryInput } from "../../lib/types";
import { formatMoney, formatMoneyOrMixed } from "../../lib/format";
import { Button, Card, ConfirmDialog, EmptyState, Input, LoadingBlock, StatCard } from "../../components/ui";
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
// Monthly bucketing for the "Income vs Expenses" chart.
// ---------------------------------------------------------------------------

interface MonthBucket {
  key: string; // YYYY-MM
  label: string;
  incomeCents: number;
  expenseCents: number;
}

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

function monthLabel(key: string): string {
  const [y, m] = key.split("-").map(Number);
  return new Date(y, m - 1, 1).toLocaleDateString(undefined, { year: "numeric", month: "short" });
}

// A genuinely "All time" ledger spanning years would otherwise render an
// unreadable wall of bars - capped to the most recent 24 months.
const MAX_CHART_MONTHS = 24;

function buildMonthlySeries(entries: FinanceEntry[], from: string | null, to: string | null): MonthBucket[] {
  const sums = new Map<string, { incomeCents: number; expenseCents: number }>();
  for (const e of entries) {
    const key = monthKey(e.entryDate);
    const cur = sums.get(key) ?? { incomeCents: 0, expenseCents: 0 };
    if (e.entryType === "income") cur.incomeCents += e.amountCents;
    else cur.expenseCents += e.amountCents;
    sums.set(key, cur);
  }
  if (sums.size === 0) return [];
  const keys = Array.from(sums.keys()).sort();
  const startKey = from ? monthKey(from) : keys[0];
  const endKey = to ? monthKey(to) : keys[keys.length - 1];
  const allMonths: string[] = [];
  for (let cursor = startKey; cursor <= endKey; cursor = addMonths(cursor, 1)) {
    allMonths.push(cursor);
    if (allMonths.length > 1000) break; // sanity guard, never realistically hit
  }
  const shown = allMonths.length > MAX_CHART_MONTHS ? allMonths.slice(-MAX_CHART_MONTHS) : allMonths;
  return shown.map((key) => ({
    key,
    label: monthLabel(key),
    incomeCents: sums.get(key)?.incomeCents ?? 0,
    expenseCents: sums.get(key)?.expenseCents ?? 0,
  }));
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

  const monthlySeries = useMemo(() => buildMonthlySeries(eurScoped, from, to), [eurScoped, from, to]);

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
                className="rounded border border-amber-300 bg-white px-2 py-0.5 font-medium text-amber-800 hover:bg-amber-100 dark:border-amber-500/40 dark:bg-slate-900 dark:text-amber-400 dark:hover:bg-amber-500/10"
                onClick={() => setConvertConfirm({ currency: c.currency, label: c.currency })}
              >
                {c.currency} ({c.count})
              </button>
            ))}
            {nonEurCurrencies.length > 1 && (
              <button
                type="button"
                className="rounded border border-amber-300 bg-white px-2 py-0.5 font-medium text-amber-800 hover:bg-amber-100 dark:border-amber-500/40 dark:bg-slate-900 dark:text-amber-400 dark:hover:bg-amber-500/10"
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
        <div className="flex flex-wrap items-center gap-1 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-1">
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
        <div className="flex flex-wrap items-center gap-1 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-1">
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
        <LoadingBlock label="Loading Finance..." />
      ) : customDatesMissing ? (
        <div className="mb-6 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-800 dark:border-amber-500/30 dark:bg-amber-500/10 dark:text-amber-400">
          Please select at least one date.
        </div>
      ) : (
        <>
          <div className="mb-6 grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-5">
            <StatCard label="Income" value={formatMoney(incomeCents, "EUR")} />
            <StatCard label="Expenses" value={formatMoney(expenseCents, "EUR")} />
            <StatCard
              label="Net Cash Flow"
              value={formatMoney(netCashFlowCents, "EUR")}
              tone={netCashFlowCents > 0 ? "positive" : netCashFlowCents < 0 ? "negative" : "default"}
            />
            <StatCard label="Current Balance" value={formatMoney(currentBalanceCents, "EUR")} sub="Across active EUR accounts" />
            {pending && pending.count > 0 && (
              <StatCard
                label="Pending / Outstanding"
                value={formatMoneyOrMixed(pending.amountCents, pending.currency)}
                sub={`${pending.count} unpaid sale${pending.count === 1 ? "" : "s"}`}
              />
            )}
          </div>
          {excludedNonEurCount > 0 && (
            <p className="-mt-4 mb-6 text-xs text-slate-400 dark:text-slate-500">
              {excludedNonEurCount} entr{excludedNonEurCount === 1 ? "y" : "ies"} in this period{" "}
              {excludedNonEurCount === 1 ? "isn't" : "aren't"} in EUR yet, so{" "}
              {excludedNonEurCount === 1 ? "it isn't" : "they aren't"} included above - convert with the banner up top.
            </p>
          )}

          <div className="mb-6 grid grid-cols-1 gap-5 lg:grid-cols-2">
            <CategoryBreakdownCard rows={categoryBreakdown} />
            <Card className="p-4">
              <p className="mb-3 text-xs font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">
                Income vs Expenses by month
              </p>
              <IncomeExpenseChart buckets={monthlySeries} />
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
        <IconBarChart className="h-4 w-4 text-slate-400 dark:text-slate-500" />
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

const CHART_ROW_HEIGHT = 160;

function IncomeExpenseChart({ buckets }: { buckets: MonthBucket[] }) {
  if (buckets.length === 0) {
    return (
      <div className="flex items-center justify-center text-sm text-slate-400 dark:text-slate-500" style={{ height: CHART_ROW_HEIGHT + 28 }}>
        No entries in this period yet.
      </div>
    );
  }
  const maxVal = Math.max(1, ...buckets.flatMap((b) => [b.incomeCents, b.expenseCents]));
  return (
    <div>
      <div className="mb-3 flex items-center gap-4 text-xs">
        <span className="inline-flex items-center gap-1.5 text-slate-600 dark:text-slate-400">
          <span className="h-2 w-2 rounded-full bg-emerald-500" /> Income
        </span>
        <span className="inline-flex items-center gap-1.5 text-slate-600 dark:text-slate-400">
          <span className="h-2 w-2 rounded-full bg-rose-500" /> Expenses
        </span>
      </div>
      <div className="flex items-end gap-3 overflow-x-auto px-1 pb-1" style={{ height: CHART_ROW_HEIGHT }}>
        {buckets.map((b) => (
          <div key={b.key} className="flex h-full shrink-0 flex-col items-center justify-end gap-1" style={{ width: 56 }}>
            <div className="flex items-end gap-0.5" style={{ height: CHART_ROW_HEIGHT - 20 }}>
              <div
                className="w-3.5 rounded-t bg-emerald-500"
                style={{ height: `${Math.max(2, (b.incomeCents / maxVal) * (CHART_ROW_HEIGHT - 20))}px` }}
                title={`${b.label} - Income ${formatMoney(b.incomeCents, "EUR")}`}
              />
              <div
                className="w-3.5 rounded-t bg-rose-500"
                style={{ height: `${Math.max(2, (b.expenseCents / maxVal) * (CHART_ROW_HEIGHT - 20))}px` }}
                title={`${b.label} - Expenses ${formatMoney(b.expenseCents, "EUR")}`}
              />
            </div>
            <span className="whitespace-nowrap text-[10px] text-slate-400 dark:text-slate-500">{b.label}</span>
          </div>
        ))}
      </div>
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
        <IconTrendingUp className="h-4 w-4 text-slate-400 dark:text-slate-500" />
        <h3 className="text-sm font-semibold text-slate-800 dark:text-slate-200">Cashflow Forecast</h3>
        <span className="rounded-full bg-brand-50 px-2 py-0.5 text-[10px] font-bold uppercase tracking-wide text-brand-700 dark:bg-brand-500/10 dark:text-brand-400">
          Forecast
        </span>
      </div>
      {loading ? (
        <LoadingBlock label="Calculating forecast..." />
      ) : !forecast || !forecast.available ? (
        <EmptyState title="Forecast unavailable" description="Add an active EUR account on the Accounts tab to see a forecast." />
      ) : (
        <div>
          <dl className="grid grid-cols-2 gap-x-6 gap-y-2 text-sm sm:grid-cols-4">
            <div>
              <dt className="text-xs text-slate-400 dark:text-slate-500">Current balance</dt>
              <dd className="tabular-nums text-slate-800 dark:text-slate-200">{formatMoney(forecast.currentBalanceCents, "EUR")}</dd>
            </div>
            <div>
              <dt className="text-xs text-slate-400 dark:text-slate-500">Expected income</dt>
              <dd className="tabular-nums text-emerald-600 dark:text-emerald-400">+{formatMoney(forecast.expectedIncomeCents, "EUR")}</dd>
            </div>
            <div>
              <dt className="text-xs text-slate-400 dark:text-slate-500">Recurring expenses</dt>
              <dd className="tabular-nums text-rose-600 dark:text-rose-400">-{formatMoney(forecast.recurringExpensesCents, "EUR")}</dd>
            </div>
            <div>
              <dt className="text-xs text-slate-400 dark:text-slate-500">Upcoming expenses</dt>
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
            <p className="mt-2 text-xs text-slate-400 dark:text-slate-500">
              Some non-EUR balances, sales or entries exist and aren't included above (no exchange rate is guessed).
            </p>
          )}
        </div>
      )}
    </Card>
  );
}

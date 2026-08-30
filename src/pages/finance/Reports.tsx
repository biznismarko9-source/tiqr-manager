import { useMemo, useState, type ReactNode } from "react";
import type { Account, FinanceEntry, Transfer } from "../../lib/types";
import { formatMoney, formatPercent, todayIso } from "../../lib/format";
import { Card, EmptyState, Input, LoadingBlock } from "../../components/ui";
import { FinanceCategorySwatch } from "../../components/FinanceCategoryBadge";
import { IconBarChart, IconTrendingUp, IconUsers, IconWallet } from "../../components/icons";
import { PERIODS, periodBounds, type FinanceData, type PeriodKey } from "./shared";

// 2.1.0: marko's own "FINANCE 2.1" request - a set of standard read-only
// statements (Profit & Loss / Cash Flow / Expenses by Category / Expenses
// by Account / Business vs Personal), all computed client-side from the
// SAME entries/transfers/accounts already loaded by Finance.tsx - no new
// backend command for this tab, same "flat data + derive on the frontend"
// approach Overview.tsx's own stat cards already use. Read-only: no
// mutations happen anywhere in this file.
//
// One deliberate, load-bearing design choice: Profit & Loss counts EVERY
// EUR entry in the period, but Cash Flow only counts entries/transfers
// linked to an ACTIVE EUR account (see `eurBalanceAsOf` below). Those are
// genuinely different questions - "what did I earn/spend" vs "how did my
// wallets' balances move" - and conflating them would make Opening +
// Inflows - Outflows + Transfers silently stop equaling Closing for anyone
// with entries that predate 2.1.0 (their account_id is NULL - the 016
// migration never backfills one - so pre-2.1.0 history never touched any
// wallet balance in the first place). Expenses by Account makes this
// explicit rather than hiding it: unlinked expenses get their own "No
// account" row instead of being dropped.
//
// Not implemented: a "Ticket-business vs Other-business" expense split
// (marko's own point 13, explicitly conditional - "ak je mozne bez velkeho
// zasahu"/"if possible without a big change"). There is no existing field
// anywhere - not on FinanceEntry, not on FinanceCategory - that marks a
// business expense as ticket-related; the ticket business's own P&L lives
// entirely in the separate, DO-NOT-TOUCH ticket/order/sales tables. Adding
// one would mean either a schema change or guessing from category name
// text, and marko's own qualifier says to skip it rather than do either.
// See FINANCE-2.1.0-REPORT.md for the full note.

/** Sum of every active EUR account's balance strictly as of `cutoffIso`
 * (only entries/transfers dated on-or-before that date count) - a
 * client-side mirror of finance_forecast.rs's own `eur_balance_as_of`,
 * built from the same entries/transfers/accounts arrays every tab already
 * has (no new backend command). `cutoffIso === null` means "before any
 * recorded activity at all" - just the accounts' own opening balances. */
function eurBalanceAsOf(accounts: Account[], entries: FinanceEntry[], transfers: Transfer[], cutoffIso: string | null): number {
  const eurActiveIds = new Set(accounts.filter((a) => a.currency === "EUR" && a.isActive).map((a) => a.id));
  let total = 0;
  for (const a of accounts) {
    if (eurActiveIds.has(a.id)) total += a.openingBalanceCents;
  }
  if (cutoffIso === null) return total;
  for (const e of entries) {
    if (e.accountId === null || !eurActiveIds.has(e.accountId) || e.entryDate > cutoffIso) continue;
    total += e.entryType === "income" ? e.amountCents : -e.amountCents;
  }
  for (const t of transfers) {
    if (t.transferDate > cutoffIso) continue;
    if (eurActiveIds.has(t.toAccountId)) total += t.amountCents;
    if (eurActiveIds.has(t.fromAccountId)) total -= t.amountCents;
  }
  return total;
}

/** Calendar-safe "the day before this ISO date" - done in UTC so it can
 * never shift a day depending on the browser's own local timezone. */
function isoDayBefore(iso: string): string {
  const d = new Date(`${iso}T00:00:00Z`);
  d.setUTCDate(d.getUTCDate() - 1);
  return d.toISOString().slice(0, 10);
}

interface BreakdownRow {
  key: string;
  name: string;
  colorSlot?: number | null;
  totalCents: number;
}

export default function Reports({ entries, accounts, transfers, loading }: FinanceData) {
  const [period, setPeriod] = useState<PeriodKey>("month");
  const [customFrom, setCustomFrom] = useState("");
  const [customTo, setCustomTo] = useState("");

  const { from, to } = periodBounds(period, customFrom, customTo);
  const customDatesMissing = period === "custom" && !customFrom && !customTo;

  const periodScoped = useMemo(() => {
    if (customDatesMissing) return [];
    return entries.filter((e) => {
      if (from && e.entryDate < from) return false;
      if (to && e.entryDate > to) return false;
      return true;
    });
  }, [entries, from, to, customDatesMissing]);

  const eurScoped = useMemo(() => periodScoped.filter((e) => e.currency === "EUR"), [periodScoped]);
  const excludedNonEurCount = periodScoped.length - eurScoped.length;

  const eurActiveAccountIds = useMemo(() => new Set(accounts.filter((a) => a.currency === "EUR" && a.isActive).map((a) => a.id)), [accounts]);

  // --- Profit & Loss: every EUR entry in the period, regardless of
  // whether it's linked to any account. -----------------------------------
  const incomeCents = useMemo(() => eurScoped.filter((e) => e.entryType === "income").reduce((s, e) => s + e.amountCents, 0), [eurScoped]);
  const expenseCents = useMemo(() => eurScoped.filter((e) => e.entryType === "expense").reduce((s, e) => s + e.amountCents, 0), [eurScoped]);
  const netCents = incomeCents - expenseCents;

  // --- Cash Flow: wallet-scoped - see this file's own header comment for
  // why this deliberately differs from Profit & Loss above. ---------------
  const openingCents = useMemo(() => eurBalanceAsOf(accounts, entries, transfers, from ? isoDayBefore(from) : null), [accounts, entries, transfers, from]);
  const closingCents = useMemo(() => eurBalanceAsOf(accounts, entries, transfers, to ?? todayIso()), [accounts, entries, transfers, to]);
  const walletScoped = useMemo(
    () => eurScoped.filter((e) => e.accountId !== null && eurActiveAccountIds.has(e.accountId)),
    [eurScoped, eurActiveAccountIds],
  );
  const inflowsCents = useMemo(() => walletScoped.filter((e) => e.entryType === "income").reduce((s, e) => s + e.amountCents, 0), [walletScoped]);
  const outflowsCents = useMemo(() => walletScoped.filter((e) => e.entryType === "expense").reduce((s, e) => s + e.amountCents, 0), [walletScoped]);
  // Always nets to 0 in practice: a transfer only ever moves money between
  // two accounts of the SAME currency (create_transfer_impl rejects
  // cross-currency outright), so within "every active EUR account" the
  // inflow on one side and the outflow on the other always cancel. Shown
  // anyway (marko's own spec lists it as its own Cash Flow line) as an
  // honest confirmation that transfers never change the total, not a bug.
  const transfersNetCents = useMemo(() => {
    if (customDatesMissing) return 0;
    let net = 0;
    for (const t of transfers) {
      if (t.currency !== "EUR") continue;
      if (from && t.transferDate < from) continue;
      if (to && t.transferDate > to) continue;
      if (eurActiveAccountIds.has(t.toAccountId)) net += t.amountCents;
      if (eurActiveAccountIds.has(t.fromAccountId)) net -= t.amountCents;
    }
    return net;
  }, [transfers, from, to, eurActiveAccountIds, customDatesMissing]);

  // --- Expenses by Category: same population as Profit & Loss (every EUR
  // expense in the period), not wallet-scoped. -----------------------------
  const byCategory = useMemo<BreakdownRow[]>(() => {
    const map = new Map<string, BreakdownRow>();
    for (const e of eurScoped) {
      if (e.entryType !== "expense") continue;
      const key = e.categoryId ? String(e.categoryId) : "none";
      const cur = map.get(key) ?? { key, name: e.categoryName ?? "No category", colorSlot: e.categoryColorSlot, totalCents: 0 };
      cur.totalCents += e.amountCents;
      map.set(key, cur);
    }
    return Array.from(map.values()).sort((a, b) => b.totalCents - a.totalCents);
  }, [eurScoped]);

  // --- Expenses by Account: same population as Profit & Loss too - "No
  // account" is a real, meaningful bucket here (not excluded, unlike Cash
  // Flow above), since the whole point is showing how much spending isn't
  // assigned to a wallet yet. --------------------------------------------
  const byAccount = useMemo<BreakdownRow[]>(() => {
    const map = new Map<string, BreakdownRow>();
    for (const e of eurScoped) {
      if (e.entryType !== "expense") continue;
      const key = e.accountId ? String(e.accountId) : "none";
      const cur = map.get(key) ?? { key, name: e.accountName ?? "No account", totalCents: 0 };
      cur.totalCents += e.amountCents;
      map.set(key, cur);
    }
    return Array.from(map.values()).sort((a, b) => b.totalCents - a.totalCents);
  }, [eurScoped]);

  // --- Business vs Personal ------------------------------------------------
  const bizVsPersonal = useMemo(() => {
    const calc = (scope: "personal" | "business") => {
      const scoped = eurScoped.filter((e) => e.scope === scope);
      const income = scoped.filter((e) => e.entryType === "income").reduce((s, e) => s + e.amountCents, 0);
      const expense = scoped.filter((e) => e.entryType === "expense").reduce((s, e) => s + e.amountCents, 0);
      return { income, expense, net: income - expense };
    };
    return { personal: calc("personal"), business: calc("business") };
  }, [eurScoped]);

  return (
    <div>
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
      </Card>

      {loading ? (
        <LoadingBlock label="Loading reports..." />
      ) : customDatesMissing ? (
        <div className="rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-800 dark:border-amber-500/30 dark:bg-amber-500/10 dark:text-amber-400">
          Please select at least one date.
        </div>
      ) : (
        <>
          {excludedNonEurCount > 0 && (
            <p className="mb-4 text-xs text-slate-400 dark:text-slate-500">
              {excludedNonEurCount} entr{excludedNonEurCount === 1 ? "y" : "ies"} in this period{" "}
              {excludedNonEurCount === 1 ? "isn't" : "aren't"} in EUR yet, so {excludedNonEurCount === 1 ? "it isn't" : "they aren't"} included
              below - convert {excludedNonEurCount === 1 ? "it" : "them"} on the Overview tab.
            </p>
          )}

          <ReportCard title="Profit &amp; Loss" icon={<IconTrendingUp className="h-4 w-4 text-slate-400 dark:text-slate-500" />}>
            <dl className="grid grid-cols-3 gap-4 text-sm">
              <div>
                <dt className="text-xs text-slate-400 dark:text-slate-500">Income</dt>
                <dd className="mt-1 text-lg font-semibold tabular-nums text-emerald-600 dark:text-emerald-400">{formatMoney(incomeCents, "EUR")}</dd>
              </div>
              <div>
                <dt className="text-xs text-slate-400 dark:text-slate-500">Expenses</dt>
                <dd className="mt-1 text-lg font-semibold tabular-nums text-rose-600 dark:text-rose-400">{formatMoney(expenseCents, "EUR")}</dd>
              </div>
              <div>
                <dt className="text-xs text-slate-400 dark:text-slate-500">Net</dt>
                <dd className={`mt-1 text-lg font-semibold tabular-nums ${netCents >= 0 ? "text-emerald-600 dark:text-emerald-400" : "text-rose-600 dark:text-rose-400"}`}>
                  {formatMoney(netCents, "EUR")}
                </dd>
              </div>
            </dl>
          </ReportCard>

          <ReportCard
            title="Cash Flow"
            icon={<IconWallet className="h-4 w-4 text-slate-400 dark:text-slate-500" />}
            note="Only entries and transfers linked to an active EUR account are counted, so Opening + Inflows - Outflows + Transfers always equals Closing. Entries with no account are in Profit & Loss above but not here - see Expenses by Account below."
          >
            <dl className="grid grid-cols-2 gap-x-6 gap-y-3 text-sm sm:grid-cols-5">
              <div>
                <dt className="text-xs text-slate-400 dark:text-slate-500">Opening</dt>
                <dd className="mt-1 tabular-nums text-slate-800 dark:text-slate-200">{formatMoney(openingCents, "EUR")}</dd>
              </div>
              <div>
                <dt className="text-xs text-slate-400 dark:text-slate-500">Inflows</dt>
                <dd className="mt-1 tabular-nums text-emerald-600 dark:text-emerald-400">+{formatMoney(inflowsCents, "EUR")}</dd>
              </div>
              <div>
                <dt className="text-xs text-slate-400 dark:text-slate-500">Outflows</dt>
                <dd className="mt-1 tabular-nums text-rose-600 dark:text-rose-400">-{formatMoney(outflowsCents, "EUR")}</dd>
              </div>
              <div>
                <dt className="text-xs text-slate-400 dark:text-slate-500">Transfers</dt>
                <dd className="mt-1 tabular-nums text-slate-500 dark:text-slate-400">{formatMoney(transfersNetCents, "EUR")}</dd>
              </div>
              <div>
                <dt className="text-xs text-slate-400 dark:text-slate-500">Closing</dt>
                <dd className="mt-1 font-semibold tabular-nums text-slate-900 dark:text-slate-100">{formatMoney(closingCents, "EUR")}</dd>
              </div>
            </dl>
          </ReportCard>

          <div className="grid grid-cols-1 gap-5 lg:grid-cols-2">
            <ReportCard title="Expenses by Category" icon={<IconBarChart className="h-4 w-4 text-slate-400 dark:text-slate-500" />}>
              <BreakdownList rows={byCategory} showSwatch />
            </ReportCard>
            <ReportCard title="Expenses by Account" icon={<IconWallet className="h-4 w-4 text-slate-400 dark:text-slate-500" />}>
              <BreakdownList rows={byAccount} />
            </ReportCard>
          </div>

          <ReportCard title="Business vs Personal" icon={<IconUsers className="h-4 w-4 text-slate-400 dark:text-slate-500" />}>
            <div className="overflow-x-auto">
              <table className="w-full border-collapse text-sm">
                <thead>
                  <tr className="border-b border-slate-100 dark:border-slate-800">
                    <th className="py-1.5 pr-3 text-left font-medium text-slate-400 dark:text-slate-500" />
                    <th className="px-3 py-1.5 text-right font-medium text-slate-400 dark:text-slate-500">Personal</th>
                    <th className="px-3 py-1.5 text-right font-medium text-slate-400 dark:text-slate-500">Business</th>
                    <th className="py-1.5 pl-3 text-right font-medium text-slate-400 dark:text-slate-500">Total</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
                  <tr>
                    <td className="py-2 pr-3 text-slate-500 dark:text-slate-400">Income</td>
                    <td className="px-3 py-2 text-right tabular-nums text-slate-700 dark:text-slate-300">{formatMoney(bizVsPersonal.personal.income, "EUR")}</td>
                    <td className="px-3 py-2 text-right tabular-nums text-slate-700 dark:text-slate-300">{formatMoney(bizVsPersonal.business.income, "EUR")}</td>
                    <td className="py-2 pl-3 text-right tabular-nums font-medium text-emerald-600 dark:text-emerald-400">{formatMoney(incomeCents, "EUR")}</td>
                  </tr>
                  <tr>
                    <td className="py-2 pr-3 text-slate-500 dark:text-slate-400">Expenses</td>
                    <td className="px-3 py-2 text-right tabular-nums text-slate-700 dark:text-slate-300">{formatMoney(bizVsPersonal.personal.expense, "EUR")}</td>
                    <td className="px-3 py-2 text-right tabular-nums text-slate-700 dark:text-slate-300">{formatMoney(bizVsPersonal.business.expense, "EUR")}</td>
                    <td className="py-2 pl-3 text-right tabular-nums font-medium text-rose-600 dark:text-rose-400">{formatMoney(expenseCents, "EUR")}</td>
                  </tr>
                  <tr>
                    <td className="py-2 pr-3 font-medium text-slate-700 dark:text-slate-300">Net</td>
                    <td className="px-3 py-2 text-right tabular-nums font-medium text-slate-800 dark:text-slate-200">{formatMoney(bizVsPersonal.personal.net, "EUR")}</td>
                    <td className="px-3 py-2 text-right tabular-nums font-medium text-slate-800 dark:text-slate-200">{formatMoney(bizVsPersonal.business.net, "EUR")}</td>
                    <td className={`py-2 pl-3 text-right tabular-nums font-semibold ${netCents >= 0 ? "text-emerald-600 dark:text-emerald-400" : "text-rose-600 dark:text-rose-400"}`}>
                      {formatMoney(netCents, "EUR")}
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </ReportCard>
        </>
      )}
    </div>
  );
}

function ReportCard({ title, icon, children, note }: { title: string; icon?: ReactNode; children: ReactNode; note?: string }) {
  return (
    <Card className="mb-5">
      <div className="flex items-center gap-2 border-b border-slate-100 dark:border-slate-800 px-4 py-3">
        {icon}
        <h3 className="text-sm font-semibold text-slate-800 dark:text-slate-200">{title}</h3>
      </div>
      <div className="p-4">{children}</div>
      {note && <p className="border-t border-slate-100 px-4 py-2.5 text-xs text-slate-400 dark:border-slate-800 dark:text-slate-500">{note}</p>}
    </Card>
  );
}

function BreakdownList({ rows, showSwatch }: { rows: BreakdownRow[]; showSwatch?: boolean }) {
  if (rows.length === 0) return <EmptyState title="No expenses in this period yet" />;
  const totalCents = rows.reduce((s, r) => s + r.totalCents, 0);
  const maxVal = Math.max(1, ...rows.map((r) => r.totalCents));
  return (
    <div className="space-y-2.5">
      {rows.map((r) => (
        <div key={r.key}>
          <div className="mb-1 flex items-center justify-between gap-2 text-sm">
            <span className="flex min-w-0 items-center gap-1.5 truncate font-medium text-slate-800 dark:text-slate-200">
              {showSwatch && r.colorSlot !== null && r.colorSlot !== undefined && <FinanceCategorySwatch colorSlot={r.colorSlot} />}
              {r.name}
            </span>
            <span className="shrink-0 tabular-nums text-slate-600 dark:text-slate-400">
              {formatMoney(r.totalCents, "EUR")}
              <span className="ml-1.5 text-xs text-slate-400 dark:text-slate-500">{totalCents > 0 ? formatPercent(r.totalCents / totalCents) : ""}</span>
            </span>
          </div>
          <div className="h-1.5 rounded-full bg-slate-100 dark:bg-slate-800">
            <div className="h-1.5 rounded-full bg-brand-500" style={{ width: `${Math.max(4, (r.totalCents / maxVal) * 100)}%` }} />
          </div>
        </div>
      ))}
    </div>
  );
}

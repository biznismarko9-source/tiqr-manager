import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { Link } from "react-router-dom";
import { api, errMsg } from "../lib/api";
import type { AttentionCenterItem, DashboardData, DashboardTab, UpcomingEventAlert } from "../lib/types";
import {
  computeTrend,
  computeTrendPoints,
  formatDate,
  formatMoney,
  formatMoneyOrMixed,
  formatPercent,
  summarizeBulkDeleteSkips,
  todayIso,
} from "../lib/format";
import { Badge, Button, Card, ConfirmDialog, EmptyState, LoadingBlock, PageHeader, StatCard } from "../components/ui";
import { MetricChart, METRICS, type MetricKey } from "../components/MetricChart";
import {
  IconAlertTriangle,
  IconBarChart,
  IconBell,
  IconCalendarDays,
  IconPackage,
  IconReceipt,
} from "../components/icons";
import { useToast } from "../lib/toast";

// 2.0.47 (DIR-001 signature idea #01): the exact same "warning starting N
// days before, escalating daily, gone once resolved" mechanism Pulls.tsx
// already established for its own transfer-deadline warning - just applied
// to the Dashboard's existing "Upcoming events (next 14 days)" alert list
// instead of a pull's transfer deadline. Deliberately duplicated here
// rather than extracted into a shared lib/format.ts helper: this round's
// confirmed scope is Dashboard-only (see REDESIGN-2.0.47-REPORT.md), and
// Pulls.tsx is working, shipped code this round doesn't otherwise touch -
// same names/behavior as Pulls.tsx's own daysUntil/warningLabel, so a
// future round can still unify them into one shared helper with zero
// behavior change.
const UPCOMING_WARNING_WINDOW_DAYS = 3;

/** Whole days between today and `dateIso` (positive = in the future,
 * negative = already passed). Plain calendar-day difference, not
 * time-of-day-sensitive - matches how `eventDate` is always a plain
 * "YYYY-MM-DD" string in this app. Byte-for-byte the same as Pulls.tsx's
 * own daysUntil - see the comment above. */
function daysUntil(dateIso: string): number {
  const start = new Date(`${todayIso()}T00:00:00`);
  const end = new Date(dateIso.length <= 10 ? `${dateIso}T00:00:00` : dateIso);
  return Math.round((end.getTime() - start.getTime()) / 86_400_000);
}

function warningLabel(daysLeft: number): string {
  if (daysLeft > 0) return `${daysLeft}d left`;
  if (daysLeft === 0) return "Today!";
  return `${Math.abs(daysLeft)}d overdue`;
}

// Same fixed window the backend uses (dashboard.rs::UPCOMING_EVENT_WINDOW_DAYS)
// - shown in the section label only, never re-derived/recomputed here.
const UPCOMING_EVENT_WINDOW_DAYS = 14;

// 2.0.70: how many rows each Activity-tab Recent card shows before its "Show
// more" button appears - the backend now sends up to 15 (dashboard.rs), this
// is purely how many of those are rendered up front.
const RECENT_LIST_PREVIEW_COUNT = 5;

// dashboard.rs's period_bounds() encodes "no lower/upper bound" (the "All
// time" period, or a Custom range with the From date left blank) as these
// two sentinel dates so the underlying SQL BETWEEN query always has a real
// pair of bind params. They must never be shown to the user as if they were
// real dates (1.6.0 audit H3 - this used to render as e.g.
// "Activity 0001-01-01 -> 9999-12-31").
const PERIOD_MIN_SENTINEL = "0001-01-01";
const PERIOD_MAX_SENTINEL = "9999-12-31";
function periodBoundLabel(iso: string, fallback: string): string {
  return iso === PERIOD_MIN_SENTINEL || iso === PERIOD_MAX_SENTINEL ? fallback : formatDate(iso);
}

// 1.7.5: replaced with a standard Today/1W/1M/3M/YTD/1Y/5Y/All range-picker
// set (marko's reference screenshot), still driving both the StatCards below
// and the chart from this one shared selection - see period_bounds() in
// dashboard.rs for what each key resolves to. "Custom" is kept at the end -
// the reference screenshot doesn't show it, but removing the existing
// custom-date-range feature wasn't part of the ask (see 1.7.5 report).
const PERIODS: { key: string; label: string }[] = [
  { key: "today", label: "Today" },
  { key: "1w", label: "1 Wk" },
  { key: "1m", label: "1 Mo" },
  { key: "3m", label: "3 Mo" },
  { key: "ytd", label: "YTD" },
  { key: "1y", label: "1 Yr" },
  { key: "5y", label: "5 Yr" },
  { key: "all", label: "All" },
  { key: "custom", label: "Custom" },
];

/** The big number in the chart card header - always read straight from
 * `data.period` (the exact same FinanceSummary the StatCards above already
 * render), never re-derived from the chart's own bucket array, so this
 * number can never drift from the StatCard for the same metric. */
function periodMetricValue(data: DashboardData, metric: MetricKey): number {
  if (metric === "profit") return data.period.profitCents;
  if (metric === "sales") return data.period.soldTickets;
  return data.period.revenueCents;
}

/** Same emerald/red/slate convention the "Profit" StatCard already uses
 * (see StatCard's tone prop in ui.tsx) - reused here rather than inventing a
 * second color rule for the same profit/loss distinction. Revenue and Sales
 * are never negative, so they're always the neutral/default tone. */
function periodMetricTone(data: DashboardData, metric: MetricKey): string {
  if (metric !== "profit") return "text-slate-900 dark:text-slate-100";
  if (data.period.profitCents > 0) return "text-emerald-600 dark:text-emerald-400";
  if (data.period.profitCents < 0) return "text-red-600 dark:text-red-400";
  return "text-slate-900 dark:text-slate-100";
}

// 1.9.3: which Dashboard tab is active, stored under this one app_settings
// key (same generic key/value mechanism lib/theme.ts's useTheme already
// established for the dark-mode preference - see useDashboardTab below,
// which mirrors its load/persist shape) as a plain string. No new backend
// command, no migration.
//
// Replaces 1.9.2's "Customize" panel entirely (DashboardWidgets, its 10
// show/hide toggles, the Customize button+modal) - marko tried it and
// decided against it: every section is important enough that hiding one
// felt wrong, but scrolling past all ten in one long page was the actual
// complaint. The fix is navigation, not visibility - nothing on this page
// can be hidden any more, you just jump straight to the group you want.
const DASHBOARD_TAB_KEY = "dashboardTab";

const TABS: { key: DashboardTab; label: string }[] = [
  { key: "overview", label: "Overview" },
  { key: "financials", label: "Financials" },
  { key: "activity", label: "Activity" },
];

function isDashboardTab(value: string): value is DashboardTab {
  return TABS.some((t) => t.key === value);
}

/** Loads/persists the active tab. Mirrors useTheme's (lib/theme.ts)
 * load-on-mount + persist-immediately-on-change shape, just a single string
 * instead of an object of booleans. An unrecognized saved value (an older
 * build's leftover, or a corrupted setting) falls back to "overview" rather
 * than crashing this page. */
function useDashboardTab(): [DashboardTab, (tab: DashboardTab) => void] {
  const [tab, setTabState] = useState<DashboardTab>("overview");

  useEffect(() => {
    let cancelled = false;
    api
      .getAppSetting(DASHBOARD_TAB_KEY)
      .then((raw) => {
        if (cancelled || !raw || !isDashboardTab(raw)) return;
        setTabState(raw);
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, []);

  const setTab = useCallback((next: DashboardTab) => {
    setTabState(next);
    api.setAppSetting(DASHBOARD_TAB_KEY, next).catch(() => {});
  }, []);

  return [tab, setTab];
}

export default function Dashboard() {
  const toast = useToast();
  // 1.7.5: default changed from "30d" to "1y" to match the reference
  // screenshot's default selection (its "1 Yr" pill is the one shown
  // active) - purely an initial UI state, one click away from anything
  // else.
  const [period, setPeriod] = useState("1y");
  const [metric, setMetric] = useState<MetricKey>("revenue");
  const [from, setFrom] = useState("");
  const [to, setTo] = useState("");
  const [data, setData] = useState<DashboardData | null>(null);
  const [loading, setLoading] = useState(true);
  // 1.9.3: which Dashboard tab is active - see useDashboardTab above.
  const [tab, setTab] = useDashboardTab();
  // 2.0.70: each Recent card on the Activity tab shows RECENT_LIST_PREVIEW_COUNT
  // rows by default and reveals the rest (up to however many the backend sent -
  // see dashboard.rs's own 2.0.70 comment) on click. Three independent flags,
  // not one - expanding Recent orders shouldn't also expand Recent sales.
  // Plain local UI state, not persisted (unlike the tab itself) - always
  // starts collapsed on a fresh page load, same as before this existed.
  const [eventsExpanded, setEventsExpanded] = useState(false);
  const [ordersExpanded, setOrdersExpanded] = useState(false);
  const [salesExpanded, setSalesExpanded] = useState(false);
  // 2.2.8: Dashboard's global "Attention Center" (Activity tab) - its own
  // independent fetch, deliberately NOT tied to `period`/`from`/`to` (unlike
  // `data` below) since none of its 5 categories are period-filtered - see
  // commands/attention_center.rs's own doc comment. Fetched once on mount;
  // `null` while loading, so the block renders nothing until real data
  // exists rather than briefly flashing "nothing needs attention".
  const [attentionCenter, setAttentionCenter] = useState<AttentionCenterItem[] | null>(null);
  // BUG (Custom date filter): Custom with both From/To empty must not
  // silently behave like "All time" (see period_bounds() fallback in
  // dashboard.rs). This is recomputed from current state on every render,
  // so it can never get "stuck" - the moment either date is filled in, or
  // the user switches to a different period button, it clears itself.
  const customDatesMissing = period === "custom" && !from && !to;

  const load = useCallback(() => {
    setLoading(true);
    api
      .getDashboard({ period, from: from || undefined, to: to || undefined })
      .then(setData)
      .catch((e) => toast.error(errMsg(e)))
      .finally(() => setLoading(false));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [period, from, to]);

  useEffect(() => {
    load();
  }, [load]);

  useEffect(() => {
    let cancelled = false;
    api
      .getAttentionCenter()
      .then((items) => {
        if (!cancelled) setAttentionCenter(items);
      })
      .catch((e) => toast.error(errMsg(e)));
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div>
      <PageHeader
        title="Dashboard"
        subtitle="A real-time snapshot of your ticket reselling business."
        actions={
          // 1.9.3: replaces the old period-pills-plus-Customize-button
          // actions row. This is now the dashboard's primary navigation -
          // see the tab-gated sections below - so it lives here, in the
          // same prominent spot Customize used to occupy. The period picker
          // itself moved into the Overview tab's own content (below) since
          // it's the only tab it actually affects - see that comment for why.
          <>
            <div className="flex flex-wrap items-center gap-1 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-1">
              {TABS.map((t) => (
                <button
                  key={t.key}
                  onClick={() => setTab(t.key)}
                  className={`rounded-md px-2.5 py-1.5 text-xs font-medium transition-colors ${
                    tab === t.key ? "bg-brand-600 text-white" : "text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800"
                  }`}
                >
                  {t.label}
                </button>
              ))}
            </div>
            {/* 2.0.75: marko's own request - "v dashboarde hore vpravo tie
                najdolezitejsie sa ukazu ako nejaka notification alebo
                warning". Rendered only once real data has loaded (never a
                bell with stale/fabricated counts). */}
            {data && <AlertBell data={data} onShowUpcoming={() => setTab("activity")} />}
          </>
        }
      />

      {tab === "overview" && (
        <>
          {/* 1.9.6 moved the Quick Actions row (New Event/Order/Sale/
              Import/Export) down here, to the bottom of this tab, after
              the chart. 2.0.79 removed that row entirely at marko's
              request - Events/Orders/Sales each already have their own
              "New X" button, and Import/Export CSV already live in
              Settings -> Data, so the shortcut was redundant. */}

          {/* 1.9.3: moved here from PageHeader's actions. Financials
              (Cashflow/Inventory/Potential profit) and Activity (Attention/
              Recent) are all explicitly all-time or right-now sections (see
              each one's own comment below) - the period filter never
              affected them even before this round, it just used to sit
              next to sections it had no effect on. Only the StatCards/chart
              below actually use it, so it now lives only where it matters.
              1.9.10: unlike the tab switcher up in PageHeader's actions
              (which sits inside a flex header row that naturally sizes it),
              this box is a standalone block-level element in the page's own
              flow, so it was stretching to the full content width by
              default - its bordered/background "rectangle" extended well
              past the last button (Custom) into empty space. marko wanted
              it to end right at Custom instead; w-fit makes the box hug its
              buttons rather than fill the row. */}
          <div className="mb-4 flex w-fit flex-wrap items-center gap-1 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-1">
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
            <Card className="mb-4 flex flex-wrap items-end gap-3 p-3">
              <label className="text-xs font-medium text-slate-600 dark:text-slate-400">
                From
                <input
                  type="date"
                  value={from}
                  onChange={(e) => setFrom(e.target.value)}
                  className="input mt-1"
                />
              </label>
              <label className="text-xs font-medium text-slate-600 dark:text-slate-400">
                To
                <input type="date" value={to} onChange={(e) => setTo(e.target.value)} className="input mt-1" />
              </label>
            </Card>
          )}
        </>
      )}

      {loading || !data ? (
        <LoadingBlock label="Loading dashboard..." />
      ) : (
        <>
          {tab === "overview" && (
            <>
              <MixedCurrencyBanner data={data} onConverted={load} />
              {customDatesMissing ? (
                // Custom filter selected but both From/To are empty: previously
                // this silently fell back to an (effectively) All-time range on
                // the backend (period_bounds()) with no indication to the user.
                // Show a clear inline message instead of any period numbers,
                // reusing the existing amber warning-banner style used
                // elsewhere on this page (mixed-currency notice above).
                <div className="mb-6 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-800 dark:border-amber-500/30 dark:bg-amber-500/10 dark:text-amber-400">
                  Please select at least one date.
                </div>
              ) : (
                <>
                  <p className="mb-2 text-xs font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">
                    Activity{" "}
                    {data.periodFrom === PERIOD_MIN_SENTINEL && data.periodTo === PERIOD_MAX_SENTINEL
                      ? "All time"
                      : `${periodBoundLabel(data.periodFrom, "the beginning")} → ${periodBoundLabel(data.periodTo, "today")}`}
                  </p>
                  {/* 2.2.11: mb-6 -> mb-5 (Part B, Dashboard cleanup) - a small,
                      deliberate trim of vertical rhythm on this tab, not a
                      redesign. See the chart Card's own 2.2.11 comment below
                      for the full reasoning. */}
                  <div className="mb-5 grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-6">
                    <StatCard
                      label="Revenue"
                      value={formatMoney(data.period.revenueCents, data.primaryCurrency)}
                      trend={computeTrend(data.period.revenueCents, data.previousPeriod?.revenueCents)}
                    />
                    <StatCard
                      label="Purchase cost"
                      value={formatMoney(data.period.totalCostCents, data.primaryCurrency)}
                      trend={computeTrend(data.period.totalCostCents, data.previousPeriod?.totalCostCents)}
                      trendColored={false}
                    />
                    <StatCard
                      label="Profit"
                      value={formatMoney(data.period.profitCents, data.primaryCurrency)}
                      tone={data.period.profitCents > 0 ? "positive" : data.period.profitCents < 0 ? "negative" : "default"}
                      trend={computeTrend(data.period.profitCents, data.previousPeriod?.profitCents)}
                    />
                    <StatCard
                      label="Margin"
                      value={formatPercent(data.period.margin)}
                      trend={computeTrendPoints(data.period.margin, data.previousPeriod?.margin)}
                    />
                    <StatCard
                      label="ROI"
                      value={formatPercent(data.period.roi)}
                      trend={computeTrendPoints(data.period.roi, data.previousPeriod?.roi)}
                    />
                    <StatCard
                      label="Tickets sold"
                      value={String(data.period.soldTickets)}
                      sub={`${data.period.purchasedTickets} purchased in period`}
                      trend={computeTrend(data.period.soldTickets, data.previousPeriod?.soldTickets)}
                    />
                  </div>
                  {/* Big number + line always come straight from data.period /
                      data.revenueTimeSeries - the exact same source the
                      StatCards above already render, never re-derived locally -
                      so this card can never disagree with them (see
                      revenue_time_series in dashboard.rs, which shares
                      period_summary's exact scope). 1.7.5: switches between 3
                      metrics (marko's reference screenshot) instead of always
                      showing Revenue - see MetricChart.tsx. Tab row reuses the
                      exact same pill pattern as the period selector above,
                      rather than a new visual pattern. 1.9.3: no longer
                      independently toggleable - both this and the StatCards
                      above are simply "the Overview tab" now. */}
                  {/* 2.2.11 (Part B, Dashboard cleanup - marko's own request:
                      "odstráň zbytočný celostránkový vertical scroll, ak sa
                      celý obsah zmestí do viewportu"): mb-8 -> mb-6. A modest,
                      one-step trim of an existing Tailwind spacing value, not
                      a redesign - every component/layout here is unchanged,
                      this only tightens the vertical gap before "Sales by
                      platform" below. Paired with that card's own internal
                      scroll fix (see SalesByPlatformCard's 2.2.11 comment) -
                      together these remove the two concrete, well-justified
                      contributors to this tab occasionally exceeding the
                      viewport height (an unbounded list, plus a bit of extra
                      whitespace), without touching anything shared
                      (PageHeader/Layout's own spacing is untouched - it's
                      used by every other page too). */}
                  <Card className="mb-6 p-4">
                  <div className="mb-3 flex flex-wrap items-start justify-between gap-3">
                    <div>
                      <p className="text-xs font-medium uppercase tracking-wide text-slate-400 dark:text-slate-500">
                        {METRICS.find((m) => m.key === metric)?.label} over time
                      </p>
                      <p className={`mt-1 text-2xl font-semibold tabular-nums ${periodMetricTone(data, metric)}`}>
                        {metric === "sales"
                          ? String(periodMetricValue(data, metric))
                          : formatMoney(periodMetricValue(data, metric), data.primaryCurrency)}
                      </p>
                    </div>
                    <div className="flex flex-wrap items-center gap-1 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-1">
                      {METRICS.map((m) => (
                        <button
                          key={m.key}
                          onClick={() => setMetric(m.key)}
                          className={`rounded-md px-2.5 py-1.5 text-xs font-medium transition-colors ${
                            metric === m.key
                              ? "bg-brand-600 text-white"
                              : "text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800"
                          }`}
                        >
                          {m.label}
                        </button>
                      ))}
                    </div>
                  </div>
                  <MetricChart
                    points={data.revenueTimeSeries}
                    granularity={data.timeSeriesGranularity}
                    currency={data.primaryCurrency}
                    metric={metric}
                  />
                </Card>
                {/* "Sales by platform" (2.0.47, DIR-001 signature idea #02) -
                    same period/currency scope as the StatCards/chart above
                    (data.salesByPlatform shares period_summary's exact
                    scope - see dashboard.rs), so switching the period pills
                    above updates this too, automatically. Orders/Sales
                    already store platform_id today; this is the first place
                    it gets grouped and shown as "which platform actually
                    earns the most" (Eventbrite's "Sales by Source" - see
                    REDESIGN-2.0.47-REPORT.md). */}
                <SalesByPlatformCard data={data} />
                </>
              )}
            </>
          )}

          {tab === "financials" && (
            <>
              <p className="mb-2 text-xs font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">
                Current inventory (all time)
              </p>
              <div className="mb-8 grid grid-cols-2 gap-3 sm:grid-cols-4">
                <StatCard label="Available" value={String(data.inventory.availableTickets)} />
                <StatCard label="Listed" value={String(data.inventory.listedTickets)} />
                <StatCard label="Sold (total)" value={String(data.inventory.soldTickets)} />
                <StatCard label="Purchased (total)" value={String(data.inventory.purchasedTickets)} />
              </div>

              {/* Cashflow (1.9.0): what's been sold vs. what's actually been
                  collected from buyers vs. what they still owe - all-time
                  realized figures (not period-filtered), same "right now"
                  convention as Attention on the Activity tab. Revenue/Profit
                  here are the same numbers as data.inventory (already
                  computed, just not previously shown anywhere on this page)
                  - Paid/Outstanding are new. Refunded sales are excluded
                  from all four, same as everywhere else in this app.
                  Deliberately just 4 cards, plain (not a tinted zone like
                  Inventory & Potential Profit below) - this is a realized,
                  not a future/estimated block. */}
              <p className="mb-2 text-xs font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">
                Cashflow (all time)
              </p>
              <div className="mb-8 grid grid-cols-2 gap-3 sm:grid-cols-4">
                <StatCard
                  label="Revenue"
                  value={formatMoneyOrMixed(data.cashflow.revenueCents, data.cashflow.currency)}
                  sub="Total sold, realized"
                />
                <StatCard
                  label="Profit"
                  value={formatMoneyOrMixed(data.cashflow.profitCents, data.cashflow.currency)}
                  tone={
                    data.cashflow.currency !== null
                      ? data.cashflow.profitCents > 0
                        ? "positive"
                        : data.cashflow.profitCents < 0
                          ? "negative"
                          : "default"
                      : "default"
                  }
                />
                <StatCard
                  label="Paid"
                  value={formatMoneyOrMixed(data.cashflow.paidCents, data.cashflow.currency)}
                  sub="Collected from buyers"
                />
                <StatCard
                  label="Outstanding"
                  value={formatMoneyOrMixed(data.cashflow.outstandingCents, data.cashflow.currency)}
                  sub="Sold but not yet paid"
                />
              </div>

              {/* Inventory & Potential Profit: deliberately its own tinted
                  zone (not plain StatCards like the sections above) and its
                  own "Potential profit" label - never "Profit" alone - so
                  it can never be mistaken for the realized Profit above.
                  Not affected by the period filter (unsold stock is a
                  right-now state, same reasoning as "Current inventory"). */}
              <div className="mb-8 rounded-xl border border-slate-200 dark:border-slate-800 bg-slate-50/60 dark:bg-slate-800/30 p-4">
                <p className="mb-1 text-xs font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">
                  Inventory &amp; Potential Profit
                </p>
                <p className="mb-3 text-xs text-slate-400 dark:text-slate-500">
                  Current unsold stock (available + listed), not affected by the period filter above. This is an
                  estimate, not realized profit.
                </p>
                <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
                  <StatCard
                    label="Inventory cost"
                    value={formatMoneyOrMixed(data.inventoryPotential.inventoryCostCents, data.inventoryPotential.currency)}
                    sub="What unsold tickets cost you"
                  />
                  <StatCard
                    label="Listing value"
                    value={formatMoneyOrMixed(data.inventoryPotential.listingValueCents, data.inventoryPotential.currency)}
                    sub="Unsold tickets that have a listing price"
                  />
                  <StatCard
                    label="Potential profit"
                    value={formatMoneyOrMixed(data.inventoryPotential.potentialProfitCents, data.inventoryPotential.currency)}
                    sub="Listing value minus inventory cost"
                  />
                </div>
                {data.alerts.missingListingPriceCount > 0 && (
                  <p className="mt-3 text-xs text-slate-400 dark:text-slate-500">
                    {data.alerts.missingListingPriceCount} unsold ticket{data.alerts.missingListingPriceCount === 1 ? "" : "s"} still{" "}
                    {data.alerts.missingListingPriceCount === 1 ? "has" : "have"} no listing price, so potential profit
                    understates what full inventory could be worth once priced - see Attention on the Activity tab.
                  </p>
                )}
              </div>
            </>
          )}

          {tab === "activity" && (
            <>
              {attentionCenter && <AttentionCenterBlock items={attentionCenter} />}
              <AttentionSection data={data} />

              <div className="grid grid-cols-1 gap-5 lg:grid-cols-3">
                <RecentCard title="Recent events" icon={<IconCalendarDays className="h-4 w-4" />}>
                  {data.recentEvents.length === 0 ? (
                    <EmptyRow text="No events yet" />
                  ) : (
                    <>
                      <ul className="divide-y divide-slate-100 dark:divide-slate-800">
                        {(eventsExpanded ? data.recentEvents : data.recentEvents.slice(0, RECENT_LIST_PREVIEW_COUNT)).map((ev) => (
                          <li key={ev.id}>
                            <Link
                              to={`/events/${ev.id}`}
                              className="flex items-center justify-between gap-2 px-4 py-2.5 hover:bg-slate-50 dark:hover:bg-slate-800/60"
                            >
                              <div className="min-w-0">
                                <p className="truncate text-sm font-medium text-slate-800 dark:text-slate-200">{ev.name}</p>
                                <p className="text-xs text-slate-400 dark:text-slate-500">{formatDate(ev.eventDate)}</p>
                              </div>
                              <Badge tone={ev.status}>{ev.status}</Badge>
                            </Link>
                          </li>
                        ))}
                      </ul>
                      <ShowMoreToggle
                        expanded={eventsExpanded}
                        onToggle={() => setEventsExpanded((v) => !v)}
                        hiddenCount={data.recentEvents.length - RECENT_LIST_PREVIEW_COUNT}
                      />
                    </>
                  )}
                </RecentCard>

                <RecentCard title="Recent orders" icon={<IconPackage className="h-4 w-4" />}>
                  {data.recentOrders.length === 0 ? (
                    <EmptyRow text="No orders yet" />
                  ) : (
                    <>
                      <ul className="divide-y divide-slate-100 dark:divide-slate-800">
                        {(ordersExpanded ? data.recentOrders : data.recentOrders.slice(0, RECENT_LIST_PREVIEW_COUNT)).map((o) => (
                          <li key={o.id}>
                            <Link
                              to={`/orders/${o.id}`}
                              className="flex items-center justify-between gap-2 px-4 py-2.5 hover:bg-slate-50 dark:hover:bg-slate-800/60"
                            >
                              <div className="min-w-0">
                                <p className="truncate text-sm font-medium text-slate-800 dark:text-slate-200">{o.code}</p>
                                <p className="truncate text-xs text-slate-400 dark:text-slate-500">{o.eventName}</p>
                              </div>
                              <p className="shrink-0 text-sm tabular-nums text-slate-600 dark:text-slate-400">
                                {formatMoney(o.totalCostCents, o.currency)}
                              </p>
                            </Link>
                          </li>
                        ))}
                      </ul>
                      <ShowMoreToggle
                        expanded={ordersExpanded}
                        onToggle={() => setOrdersExpanded((v) => !v)}
                        hiddenCount={data.recentOrders.length - RECENT_LIST_PREVIEW_COUNT}
                      />
                    </>
                  )}
                </RecentCard>

                <RecentCard title="Recent sales" icon={<IconReceipt className="h-4 w-4" />}>
                  {data.recentSales.length === 0 ? (
                    <EmptyRow text="No sales yet" />
                  ) : (
                    <>
                      <ul className="divide-y divide-slate-100 dark:divide-slate-800">
                        {(salesExpanded ? data.recentSales : data.recentSales.slice(0, RECENT_LIST_PREVIEW_COUNT)).map((s) => (
                        <li key={s.id} className="flex items-center justify-between gap-2 px-4 py-2.5">
                          <div className="min-w-0">
                            <p className="truncate text-sm font-medium text-slate-800 dark:text-slate-200">
                              {s.eventName ?? <span className="italic text-slate-400 dark:text-slate-500">Mixed events</span>}
                            </p>
                            <p className="text-xs text-slate-400 dark:text-slate-500">{formatDate(s.saleDate)}</p>
                          </div>
                          {/* 2.0.54: one row per sale ACTION now (a single
                              ticket, or a whole multi-ticket batch) - a
                              4-ticket batch used to show as 4 identical
                              entries here, one per ticket. paymentStatus is
                              only "refunded" when EVERY line in the group
                              is (same convention as the main Sales list) -
                              a partial refund instead shows the batch's
                              real remaining revenue plus the same "X/Y
                              refunded" note Sales.tsx already uses. */}
                          {s.paymentStatus === "refunded" ? (
                            <div className="flex shrink-0 flex-col items-end gap-0.5">
                              <Badge tone="refunded">Refunded</Badge>
                              <p className="text-xs tabular-nums text-slate-400 line-through dark:text-slate-500">
                                {formatMoneyOrMixed(s.revenueCents, s.currency)}
                              </p>
                            </div>
                          ) : (
                            <div className="flex shrink-0 flex-col items-end gap-0.5">
                              <p className="text-sm tabular-nums text-emerald-600 dark:text-emerald-400">
                                {formatMoneyOrMixed(s.revenueCents, s.currency)}
                              </p>
                              {s.refundedCount > 0 && (
                                <p
                                  className="text-[11px] font-medium text-amber-700 dark:text-amber-400"
                                  title={`${s.refundedCount} of ${s.ticketCount} refunded`}
                                >
                                  {s.refundedCount}/{s.ticketCount} refunded
                                </p>
                              )}
                            </div>
                          )}
                        </li>
                      ))}
                      </ul>
                      <ShowMoreToggle
                        expanded={salesExpanded}
                        onToggle={() => setSalesExpanded((v) => !v)}
                        hiddenCount={data.recentSales.length - RECENT_LIST_PREVIEW_COUNT}
                      />
                    </>
                  )}
                </RecentCard>
              </div>
            </>
          )}

        </>
      )}
    </div>
  );
}

/** The Overview tab's mixed-currency warning - unchanged wording from
 * before 2.0.51, now followed by an optional "Convert to EUR" action row
 * when there's actually something order-level to convert (data.
 * nonEurOrderCurrencies - see that field's own doc comment for why it's a
 * separate, order-scoped list from the ticket-scoped check that decides
 * data.mixedCurrencies itself). marko's own request: a button per currency
 * present, plus one "All" button once there's more than one to choose from
 * ("bude na vyber tie, ktore su v inej menej alebo vsetky"). The exact
 * converted totals aren't known until the live rate is actually fetched (on
 * confirm), so the dialog explains what WILL happen rather than previewing
 * numbers - same reasoning as Order Detail's own version of this dialog. */
function MixedCurrencyBanner({ data, onConverted }: { data: DashboardData; onConverted: () => void }) {
  const toast = useToast();
  const [pending, setPending] = useState<{ currencies: string[] | null; label: string } | null>(null);
  const [converting, setConverting] = useState(false);

  const nonEur = data.nonEurOrderCurrencies;
  // 2.0.51 fix (second pair of eyes caught this): mixedCurrencies (ticket-
  // scoped, needs 2+ distinct currencies to ever be true) and nonEur (order-
  // scoped, needs just 1+ non-EUR order) are INDEPENDENT signals. A book of
  // business that's entirely ONE non-EUR currency - e.g. a whole Google
  // Sheets connection configured in GBP, arguably marko's most common real
  // case - has mixedCurrencies=false (nothing is "mixed") but nonEur
  // non-empty (there genuinely are non-EUR orders to convert). Gating the
  // whole banner on mixedCurrencies alone made the bulk convert action
  // unreachable for exactly that case. Show this section whenever EITHER
  // signal is true; the warning text and the convert row below are each
  // independently gated on their own actual condition, not on each other.
  if (!data.mixedCurrencies && nonEur.length === 0) return null;

  const runConversion = async () => {
    if (!pending) return;
    setConverting(true);
    try {
      const result = await api.convertCurrenciesToEur(pending.currencies ?? undefined);
      setPending(null);
      if (result.converted.length > 0) {
        // 2.0.53: most orders were never linked to a sheet at all - only
        // mention Sheets when at least one converted order actually was.
        const linked = result.converted.filter((c) => c.linkedToSheet);
        const pushFailures = linked.filter((c) => c.sheetPushError);
        let message = `${result.converted.length} order${result.converted.length === 1 ? "" : "s"} converted to EUR`;
        if (linked.length > 0) {
          message +=
            pushFailures.length > 0
              ? ` - ${linked.length - pushFailures.length}/${linked.length} linked Sheet row(s) updated, ${pushFailures.length} couldn't be reached`
              : ` (${linked.length} linked Sheet row${linked.length === 1 ? "" : "s"} updated too)`;
        }
        toast.success(message);
        onConverted();
      }
      if (result.skipped.length > 0) {
        toast.error(`${result.skipped.length} skipped: ${summarizeBulkDeleteSkips(result.skipped)}`);
      }
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setConverting(false);
    }
  };

  return (
    <>
      <div className="mb-4 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-800 dark:border-amber-500/30 dark:bg-amber-500/10 dark:text-amber-400">
        {data.mixedCurrencies && (
          <p>
            You have data in more than one currency. To avoid adding different currencies together, the totals
            below only include <b>{data.primaryCurrency}</b>. Filter by event/platform to see the others.
          </p>
        )}
        {nonEur.length > 0 && (
          <div className={`flex flex-wrap items-center gap-2 ${data.mixedCurrencies ? "mt-2" : ""}`}>
            <span className="font-medium">Convert to EUR:</span>
            {nonEur.map((c) => (
              <button
                key={c.currency}
                type="button"
                className="rounded border border-amber-300 bg-white px-2 py-0.5 font-medium text-amber-800 hover:bg-amber-100 dark:border-amber-500/40 dark:bg-slate-900 dark:text-amber-400 dark:hover:bg-amber-500/10"
                onClick={() => setPending({ currencies: [c.currency], label: c.currency })}
              >
                {c.currency} ({c.orderCount})
              </button>
            ))}
            {nonEur.length > 1 && (
              <button
                type="button"
                className="rounded border border-amber-300 bg-white px-2 py-0.5 font-medium text-amber-800 hover:bg-amber-100 dark:border-amber-500/40 dark:bg-slate-900 dark:text-amber-400 dark:hover:bg-amber-500/10"
                onClick={() => setPending({ currencies: null, label: nonEur.map((c) => c.currency).join(", ") })}
              >
                All
              </button>
            )}
          </div>
        )}
      </div>

      <ConfirmDialog
        open={pending !== null}
        title="Convert to EUR?"
        message={`Fetches today's live conversion rate(s) to EUR and converts every order currently in ${pending?.label} - plus its tickets and every sale on them, including refunded ones. Orders that can't be safely converted are skipped and reported, never guessed at. This cannot be undone.`}
        confirmLabel="Convert to EUR"
        danger
        busy={converting}
        onCancel={() => setPending(null)}
        onConfirm={runConversion}
      />
    </>
  );
}

function RecentCard({
  title,
  icon,
  children,
  className = "",
}: {
  title: string;
  icon: ReactNode;
  children: ReactNode;
  /** 2.0.47: optional, additive - every pre-2.0.47 call site (Activity tab's
   * three Recent cards) relies on the grid wrapper around them for spacing
   * and never passed this, so they render byte-identical to before. Added
   * so the new standalone "Sales by platform" card (Overview tab) can get
   * its own margin without a wrapper div just for that. */
  className?: string;
}) {
  return (
    <Card className={className}>
      <div className="flex items-center gap-2 border-b border-slate-100 dark:border-slate-800 px-4 py-3">
        <span className="text-slate-400 dark:text-slate-500">{icon}</span>
        <h3 className="text-sm font-semibold text-slate-800 dark:text-slate-200">{title}</h3>
      </div>
      {children}
    </Card>
  );
}

/** 2.0.70: footer button for the three Activity-tab Recent cards - marko's
 * own request ("daj tlacitko ku kazdemu see more a ukaze sa toho viac").
 * Renders nothing once there's nothing to expand into (hiddenCount <= 0,
 * i.e. the backend's own recent list - up to 15, see dashboard.rs - already
 * fits inside RECENT_LIST_PREVIEW_COUNT), so a business with only a couple
 * of orders/sales/events never sees a dead button. */
function ShowMoreToggle({
  expanded,
  onToggle,
  hiddenCount,
}: {
  expanded: boolean;
  onToggle: () => void;
  hiddenCount: number;
}) {
  if (!expanded && hiddenCount <= 0) return null;
  return (
    <button
      type="button"
      onClick={onToggle}
      className="w-full border-t border-slate-100 px-4 py-2 text-center text-xs font-medium text-brand-600 hover:bg-slate-50 dark:border-slate-800 dark:text-brand-400 dark:hover:bg-slate-800/60"
    >
      {expanded ? "Show less" : `Show ${hiddenCount} more`}
    </button>
  );
}

/** "Sales by platform" (2.0.47, DIR-001 signature idea #02) - see the usage
 * site's comment in the Overview tab. Bars are relative to this list's own
 * biggest platform (not some fixed scale) - the point is comparing
 * platforms against each other, not reading an absolute value off the bar
 * itself (the revenue figure to its right already gives the exact number).
 *
 * 2.2.11 (Part B, Dashboard cleanup): this was the one list on the Overview
 * tab with no size limit at all - every OTHER list here is either a fixed
 * few StatCards or a single chart, but this one grows by one row per
 * distinct platform a business has ever sold through (Orders/Sales already
 * let marko pick or free-type any platform name), so a business with a
 * dozen platforms would previously push the whole Overview tab, and the
 * page's own scrollbar, further down for every additional one - exactly the
 * "unnecessary full-page scroll" marko asked to remove. Capped at
 * `max-h-72` (~5 rows) with its own `overflow-y-auto` instead: a typical
 * handful of platforms (marko's own screenshot shows 4) still shows in full
 * with no scrollbar at all, and only a genuinely long list gets an internal
 * scrollbar of its own - never the page's. */
function SalesByPlatformCard({ data }: { data: DashboardData }) {
  const rows = data.salesByPlatform;
  const maxRevenue = Math.max(1, ...rows.map((r) => r.revenueCents));
  return (
    <RecentCard title="Sales by platform" icon={<IconBarChart className="h-4 w-4" />} className="mb-8">
      {rows.length === 0 ? (
        <EmptyRow text="No sales in this period yet" />
      ) : (
        <ul className="max-h-72 divide-y divide-slate-100 overflow-y-auto dark:divide-slate-800">
          {rows.map((r) => (
            <li key={r.platformId ?? "none"} className="px-4 py-2.5">
              <div className="mb-1.5 flex items-center justify-between gap-2">
                <span className="truncate text-sm font-medium text-slate-800 dark:text-slate-200">
                  {r.platformName ?? "No platform"}
                </span>
                <span className="shrink-0 text-sm tabular-nums text-slate-600 dark:text-slate-400">
                  {formatMoney(r.revenueCents, data.primaryCurrency)}
                </span>
              </div>
              <div className="flex items-center gap-2">
                <div className="h-1.5 flex-1 rounded-full bg-slate-100 dark:bg-slate-800">
                  <div
                    className="h-1.5 rounded-full bg-brand-500"
                    style={{ width: `${Math.max(4, (r.revenueCents / maxRevenue) * 100)}%` }}
                  />
                </div>
                <span className="shrink-0 text-xs tabular-nums text-slate-400 dark:text-slate-500">{r.soldTickets} sold</span>
              </div>
            </li>
          ))}
        </ul>
      )}
    </RecentCard>
  );
}

function EmptyRow({ text }: { text: string }) {
  return (
    <div className="p-4">
      <EmptyState title={text} />
    </div>
  );
}

/** 2.0.75: Dashboard's top-right alert bell. Deliberately reuses the exact
 * same 4 fields AttentionSection's own `allClear` check below uses - this
 * bell must never be able to disagree with the Attention tiles about
 * whether something needs attention. No new detection logic, no new alert
 * engine - purely a compact, always-visible summary of what
 * AttentionSection already renders further down the page.
 *
 * Badge counts how many of the 4 categories are non-zero (0-4), amber by
 * default. It only escalates to red once the soonest upcoming event is due
 * today or overdue - same `daysUntil`/threshold `UpcomingEventRow` already
 * uses for its own amber-vs-red split, so the two never disagree either. */
function AlertBell({ data, onShowUpcoming }: { data: DashboardData; onShowUpcoming: () => void }) {
  const { alerts } = data;
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  // Same click-outside-to-close pattern as Layout.tsx's profile dropdown -
  // this is a small anchored menu, not a full-screen Modal.
  useEffect(() => {
    if (!open) return;
    const onClick = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", onClick);
    return () => document.removeEventListener("mousedown", onClick);
  }, [open]);

  // 2.0.79: "pulls" replaces "unpaid" (unpaid orders) here - marko's own
  // request. See DashboardAlerts.pullsNeedingTransferCount's doc comment
  // (types.ts) for why unpaidOrdersCount itself is untouched elsewhere.
  const rows = [
    { key: "pulls", label: "Pulls near deadline", count: alerts.pullsNeedingTransferCount, linkTo: "/pulls" },
    { key: "pending", label: "Pending sales", count: alerts.pendingSalesCount, linkTo: "/sales" },
    { key: "missing", label: "Missing listing price", count: alerts.missingListingPriceOrdersCount, linkTo: "/inventory" },
  ] as const;

  const soonestEvent = alerts.upcomingEvents[0];
  const upcomingCritical = soonestEvent !== undefined && daysUntil(soonestEvent.eventDate) <= 0;
  const activeCount =
    (alerts.pullsNeedingTransferCount > 0 ? 1 : 0) +
    (alerts.missingListingPriceCount > 0 ? 1 : 0) +
    (alerts.upcomingEventsCount > 0 ? 1 : 0) +
    (alerts.pendingSalesCount > 0 ? 1 : 0);

  return (
    <div ref={ref} className="relative">
      <button
        type="button"
        onClick={() => setOpen((o) => !o)}
        title="Attention summary"
        className="relative flex h-9 w-9 items-center justify-center rounded-lg border border-slate-200 bg-white text-slate-500 hover:bg-slate-50 dark:border-slate-800 dark:bg-slate-900 dark:text-slate-400 dark:hover:bg-slate-800"
      >
        <IconBell className="h-4 w-4" />
        {activeCount > 0 && (
          <span
            className={`absolute -right-1 -top-1 flex h-4 min-w-[1rem] items-center justify-center rounded-full px-1 text-[10px] font-semibold text-white ${
              upcomingCritical ? "bg-red-500" : "bg-amber-500"
            }`}
          >
            {activeCount}
          </span>
        )}
      </button>
      {open && (
        <div className="absolute right-0 top-full z-10 mt-1 w-72 origin-top-right animate-[pop-in_.16s_ease-out] overflow-hidden rounded-lg border border-slate-200 bg-white shadow-lg dark:border-slate-800 dark:bg-slate-900">
          {activeCount === 0 ? (
            <p className="px-4 py-3 text-xs text-slate-500 dark:text-slate-400">Nothing needs your attention right now.</p>
          ) : (
            <ul className="divide-y divide-slate-100 dark:divide-slate-800">
              {rows
                .filter((r) => r.count > 0)
                .map((r) => (
                  <li key={r.key}>
                    <Link
                      to={r.linkTo}
                      onClick={() => setOpen(false)}
                      className="flex items-center justify-between gap-2 px-4 py-2.5 text-xs hover:bg-slate-50 dark:hover:bg-slate-800/60"
                    >
                      <span className="font-medium text-slate-700 dark:text-slate-300">{r.label}</span>
                      <span className="tabular-nums text-slate-400 dark:text-slate-500">{r.count}</span>
                    </Link>
                  </li>
                ))}
              {alerts.upcomingEventsCount > 0 && (
                <li>
                  <button
                    type="button"
                    onClick={() => {
                      setOpen(false);
                      onShowUpcoming();
                    }}
                    className="flex w-full items-center justify-between gap-2 px-4 py-2.5 text-left text-xs hover:bg-slate-50 dark:hover:bg-slate-800/60"
                  >
                    <span className="flex items-center gap-1.5 font-medium text-slate-700 dark:text-slate-300">
                      {upcomingCritical && <IconAlertTriangle className="h-3.5 w-3.5 shrink-0 text-red-500" />}
                      Upcoming events
                    </span>
                    <span className="tabular-nums text-slate-400 dark:text-slate-500">{alerts.upcomingEventsCount}</span>
                  </button>
                </li>
              )}
            </ul>
          )}
        </div>
      )}
    </div>
  );
}

// 2.2.11: which category each Attention Center box represents, and in what
// order - marko's own exact spec (5 named, always-visible boxes, this exact
// order and wording), replacing the old priority-grouped
// ATTENTION_CENTER_GROUPS/AttentionCenterGroup entirely. This only changes
// how the frontend SLICES/DISPLAYS the same `items` the backend already
// returns (still the exact same categories/priorities/sort order computed
// server-side, see attention_center.rs) - no new field, no new command, no
// new business-logic rule. `subtext` is a short, static description of what
// the category means, not a dynamically computed figure - kept simple on
// purpose ("žiadne zbytočné karty/animácie, žiadny veľký redesign").
const ATTENTION_CENTER_CATEGORIES: {
  key: AttentionCenterItem["category"];
  title: string;
  subtext: string;
}[] = [
  { key: "missing_listing_price", title: "NO LISTING PRICE YET", subtext: "Unsold tickets with no listing price set" },
  { key: "no_active_listing", title: "NO ACTIVE LISTING", subtext: "Unsold tickets not listed on any marketplace" },
  { key: "sold_undelivered", title: "NOT DELIVERED YET", subtext: "Sold tickets still waiting on delivery" },
  { key: "event_soon", title: "EVENT COMING SOON", subtext: "Events approaching soon with unsold inventory" },
  {
    key: "outside_market_price",
    title: "MARKET ATTENTION",
    // 2.2.11: worded to make clear this only ever reflects real Price
    // Checker data (attention_center.rs's `outside_market_price` arm only
    // fires when `attention_item.available` is true, i.e. Price Checker
    // data actually exists for that event) - never an automatic/suggested
    // price, and section/row/tier are never read as a pricing factor
    // anywhere in that module. This box can legitimately stay at 0 forever
    // for a business that never opened Price Checker on any event.
    subtext: "Listings priced well outside real Price Checker market data",
  },
];

/** Worst (most urgent) priority currently present among a category's own
 * rows - critical > attention > info > none. A display convenience only,
 * computed from the exact per-row `priority` the backend already sets (see
 * attention_center.rs) - never a new severity rule of its own. `null` when
 * the category currently has zero rows (rendered as a plain, undotted box). */
function worstPriority(items: AttentionCenterItem[]): AttentionCenterItem["priority"] | null {
  if (items.some((i) => i.priority === "critical")) return "critical";
  if (items.some((i) => i.priority === "attention")) return "attention";
  return items.length > 0 ? "info" : null;
}

const PRIORITY_DOT_CLASS: Record<AttentionCenterItem["priority"], string> = {
  critical: "bg-red-500",
  attention: "bg-amber-500",
  info: "bg-slate-400",
};

/** One of the 5 Attention Center boxes - same label/value/sub visual
 * language as ui.tsx's StatCard (a plain, already-established look, not a
 * new one) wrapped in a real `<button>` so it's keyboard-accessible too.
 * Toggles this category's selection on click; disabled once it has zero
 * rows - there is nothing to drill into, so no detail view to open. */
function AttentionCategoryCard({
  title,
  subtext,
  items,
  selected,
  onSelect,
}: {
  title: string;
  subtext: string;
  items: AttentionCenterItem[];
  selected: boolean;
  onSelect: () => void;
}) {
  const priority = worstPriority(items);
  const count = items.length;
  return (
    <button
      type="button"
      onClick={onSelect}
      disabled={count === 0}
      className={`rounded-xl border p-4 text-left transition-colors ${
        selected
          ? "border-brand-500 bg-brand-50/60 dark:border-brand-500 dark:bg-brand-500/10"
          : "border-slate-200 bg-white hover:bg-slate-50 dark:border-slate-800 dark:bg-slate-900 dark:hover:bg-slate-800/60"
      } ${count === 0 ? "cursor-default opacity-60" : "cursor-pointer"}`}
    >
      <p className="flex items-center gap-1.5 text-xs font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">
        {priority && <span className={`h-1.5 w-1.5 shrink-0 rounded-full ${PRIORITY_DOT_CLASS[priority]}`} />}
        <span className="truncate">{title}</span>
      </p>
      <p className="mt-1.5 text-2xl font-semibold tabular-nums text-slate-900 dark:text-slate-100">{count}</p>
      <p className="mt-1 text-xs text-slate-400 dark:text-slate-500">{subtext}</p>
    </button>
  );
}

/** One Attention Center row - reuses UpcomingEventRow's exact urgency-badge
 * convention (daysUntil/warningLabel/UPCOMING_WARNING_WINDOW_DAYS, all
 * defined at the top of this file) rather than a second "how soon" style.
 *
 * 2.2.9: every order-grouped category (all except event_soon) now links
 * straight to that order's own page (`/orders/:id`, OrderDetail.tsx) instead
 * of a single ticket's `?code=` deep link - marko's own feedback on the
 * 2.2.8 shape ("nedáva zmysel" - one row per ticket flooded the list for a
 * bulk order) asked for orders you click into to reveal their tickets, and
 * OrderDetail.tsx already lists every one of `item.ticketIds` with its own
 * status/listing price/delivery indicators, so this reuses that existing
 * page rather than building a second ticket-list widget here - marko's own
 * "ak už existuje vhodný route/navigation systém, použi ho". `event_soon`
 * has no single order (see the Rust module's doc comment) and keeps going
 * to the event's own Event Workspace, unchanged from 2.2.8. */
function AttentionCenterRow({ item }: { item: AttentionCenterItem }) {
  const href = item.orderId != null ? `/orders/${item.orderId}` : `/events/${item.eventId}`;
  const ticketCount = item.ticketIds.length;
  const daysLeft = item.eventDate ? daysUntil(item.eventDate) : null;
  const urgent = daysLeft !== null && daysLeft <= UPCOMING_WARNING_WINDOW_DAYS;
  const critical = daysLeft !== null && daysLeft <= 0;
  return (
    <li>
      <Link to={href} className="flex items-center justify-between gap-2 px-4 py-2.5 hover:bg-slate-50 dark:hover:bg-slate-800/60">
        <div className="min-w-0">
          <p className="truncate text-sm font-medium text-slate-800 dark:text-slate-200">
            {item.eventName}
            {item.orderCode && (
              <span className="font-normal text-slate-400 dark:text-slate-500">
                {" "}
                · Order {item.orderCode}
                {ticketCount > 1 && ` · ${ticketCount} tickets`}
              </span>
            )}
          </p>
          <p className="truncate text-xs text-slate-400 dark:text-slate-500">{item.reason}</p>
        </div>
        <div className="flex shrink-0 flex-col items-end gap-0.5">
          {item.amountCents != null && (
            <span className="text-xs tabular-nums text-slate-600 dark:text-slate-400">
              {formatMoney(item.amountCents, item.currency ?? "EUR")}
            </span>
          )}
          {item.eventDate &&
            (urgent ? (
              <span
                className={`inline-flex items-center gap-1 whitespace-nowrap text-xs font-medium ${
                  critical ? "text-red-600 dark:text-red-400" : "text-amber-600 dark:text-amber-400"
                }`}
              >
                <IconAlertTriangle className="h-3.5 w-3.5 shrink-0" />
                {warningLabel(daysLeft as number)}
              </span>
            ) : (
              <span className="text-xs tabular-nums text-slate-400 dark:text-slate-500">{formatDate(item.eventDate)}</span>
            ))}
        </div>
      </Link>
    </li>
  );
}

/** 2.2.8: Dashboard's GLOBAL (every event) "Attention Center" block - marko's
 * own request. A distinct, ADDITIONAL block from AttentionSection further
 * down (unchanged) - see commands/attention_center.rs's module doc comment
 * for exactly how the two differ and why both exist.
 *
 * 2.2.11 rework (marko's own follow-up - see REDESIGN-2.2.11-REPORT.md):
 * replaces the single priority-grouped mixed feed with 5 distinct,
 * ALWAYS-VISIBLE boxes (one per `category`, marko's own exact naming/order -
 * ATTENTION_CENTER_CATEGORIES above), each just a count + short subtext. The
 * mixed list is gone as default/main content - clicking a box is now the
 * only way to see its individual rows (AttentionCenterRow, unchanged, reused
 * verbatim below), one category at a time, never several stacked together.
 * `items` is still the exact same, already-sorted array the backend sends -
 * this only changes how the frontend slices/displays it, never the
 * underlying rules or data (zero backend changes this release - confirmed
 * by reading attention_center.rs's own module doc comment, which already
 * guarantees the MARKET ATTENTION constraints from marko's 2.2.11 request:
 * Price-Checker-gated, no automatic pricing, section/row/tier never a
 * pricing factor). */
function AttentionCenterBlock({ items }: { items: AttentionCenterItem[] }) {
  const [selected, setSelected] = useState<AttentionCenterItem["category"] | null>(null);
  const [expanded, setExpanded] = useState(false);

  // If the selected category's last row disappears from underneath it (the
  // underlying issue got resolved and a fresh fetch dropped it to 0), close
  // the detail view instead of leaving it open and empty beneath a now-
  // disabled box.
  useEffect(() => {
    if (selected && items.every((i) => i.category !== selected)) {
      setSelected(null);
      setExpanded(false);
    }
  }, [items, selected]);

  const activeCategory = ATTENTION_CENTER_CATEGORIES.find((c) => c.key === selected) ?? null;
  const selectedItems = selected ? items.filter((i) => i.category === selected) : [];

  return (
    <div className="mb-8">
      <p className="mb-2 flex items-center gap-1.5 text-xs font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">
        <IconAlertTriangle className="h-3.5 w-3.5" /> Attention Center
      </p>
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-5">
        {ATTENTION_CENTER_CATEGORIES.map((c) => (
          <AttentionCategoryCard
            key={c.key}
            title={c.title}
            subtext={c.subtext}
            items={items.filter((i) => i.category === c.key)}
            selected={selected === c.key}
            onSelect={() => {
              setSelected((cur) => (cur === c.key ? null : c.key));
              setExpanded(false);
            }}
          />
        ))}
      </div>

      {activeCategory && (
        <Card className="mt-3">
          <div className="flex items-center justify-between gap-2 border-b border-slate-100 px-4 py-3 dark:border-slate-800">
            <div className="min-w-0">
              <h3 className="text-sm font-semibold text-slate-800 dark:text-slate-200">{activeCategory.title}</h3>
              <p className="truncate text-xs text-slate-400 dark:text-slate-500">{activeCategory.subtext}</p>
            </div>
            <button
              type="button"
              onClick={() => {
                setSelected(null);
                setExpanded(false);
              }}
              className="shrink-0 text-xs font-medium text-slate-400 hover:text-slate-600 dark:text-slate-500 dark:hover:text-slate-300"
            >
              Close
            </button>
          </div>
          {selectedItems.length === 0 ? (
            <EmptyRow text="Nothing in this category right now" />
          ) : (
            <>
              <ul className="divide-y divide-slate-100 dark:divide-slate-800">
                {(expanded ? selectedItems : selectedItems.slice(0, RECENT_LIST_PREVIEW_COUNT)).map((item) => (
                  <AttentionCenterRow key={item.key} item={item} />
                ))}
              </ul>
              <ShowMoreToggle
                expanded={expanded}
                onToggle={() => setExpanded((v) => !v)}
                hiddenCount={selectedItems.length - RECENT_LIST_PREVIEW_COUNT}
              />
            </>
          )}
        </Card>
      )}
    </div>
  );
}

/** Dashboard "Attention" section - simple, transparent counts sourced
 * directly from `data.alerts` (see dashboard.rs). No client-side scoring or
 * filtering logic here - this component only renders what the backend
 * already decided is attention-worthy. */
function AttentionSection({ data }: { data: DashboardData }) {
  const { alerts } = data;
  // 2.0.79: pullsNeedingTransferCount replaces unpaidOrdersCount here -
  // marko's own request. See DashboardAlerts.pullsNeedingTransferCount's
  // doc comment (types.ts) for why unpaidOrdersCount itself is unchanged
  // elsewhere (still used by the outbound-notifications feature).
  const allClear =
    alerts.pullsNeedingTransferCount === 0 &&
    alerts.missingListingPriceCount === 0 &&
    alerts.upcomingEventsCount === 0 &&
    alerts.pendingSalesCount === 0;

  return (
    <div className="mb-8">
      <p className="mb-2 flex items-center gap-1.5 text-xs font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">
        <IconAlertTriangle className="h-3.5 w-3.5" /> Attention
      </p>
      {allClear ? (
        <Card className="p-4 text-sm text-slate-500 dark:text-slate-400">Nothing needs your attention right now.</Card>
      ) : (
        <div className="grid grid-cols-1 gap-5 sm:grid-cols-2 lg:grid-cols-4">
          <AlertCard
            label="Pulls near deadline"
            count={alerts.pullsNeedingTransferCount}
            description="Pulls not yet transferred, with the event coming up soon or already past"
            linkTo="/pulls"
            linkLabel="View pulls"
          />
          <AlertCard
            label="Pending sales"
            count={alerts.pendingSalesCount}
            description={
              alerts.pendingSalesCount > 0
                ? `${formatMoneyOrMixed(alerts.pendingSalesAmountCents, alerts.pendingSalesCurrency)} not yet collected from buyers`
                : "Sales awaiting payment from the buyer"
            }
            linkTo="/sales"
            linkLabel="View sales"
          />
          <AlertCard
            label="Missing listing price"
            count={alerts.missingListingPriceOrdersCount}
            description="Orders with a ticket that has no listing price set"
            linkTo="/inventory"
            linkLabel="View inventory"
          />
          <RecentCard title={`Upcoming events (next ${UPCOMING_EVENT_WINDOW_DAYS} days)`} icon={<IconCalendarDays className="h-4 w-4" />}>
            {alerts.upcomingEvents.length === 0 ? (
              <EmptyRow text="No upcoming events with unsold inventory" />
            ) : (
              <>
                <ul className="divide-y divide-slate-100 dark:divide-slate-800">
                  {alerts.upcomingEvents.map((ev) => (
                    <UpcomingEventRow key={ev.id} event={ev} />
                  ))}
                </ul>
                {alerts.upcomingEventsCount > alerts.upcomingEvents.length && (
                  <p className="px-4 py-2 text-xs text-slate-400 dark:text-slate-500">
                    Showing the soonest {alerts.upcomingEvents.length} of {alerts.upcomingEventsCount}.
                  </p>
                )}
              </>
            )}
          </RecentCard>
        </div>
      )}
    </div>
  );
}

/** 2.0.47 (DIR-001 signature idea #01): adds an escalating amber/red warning
 * badge - daysUntil/warningLabel/UPCOMING_WARNING_WINDOW_DAYS at the top of
 * this file, ported from Pulls.tsx's own transfer-deadline warning - once
 * an event is within UPCOMING_WARNING_WINDOW_DAYS. Every row in this list
 * already passed the backend's own 14-day window (see
 * UPCOMING_EVENT_WINDOW_DAYS in dashboard.rs), so the badge here is purely
 * a further urgency escalation for the closest few, not a second filter -
 * rows further than 3 days out keep showing just the plain day count they
 * always did. The existing "{relevantInventory} left" text is unchanged,
 * just joined by the new badge above it. */
function UpcomingEventRow({ event }: { event: UpcomingEventAlert }) {
  const daysLeft = daysUntil(event.eventDate);
  const urgent = daysLeft <= UPCOMING_WARNING_WINDOW_DAYS;
  const critical = daysLeft <= 0;
  return (
    <li>
      <Link
        to={`/events/${event.id}`}
        className="flex items-center justify-between gap-2 px-4 py-2.5 hover:bg-slate-50 dark:hover:bg-slate-800/60"
      >
        <div className="min-w-0">
          <p className="truncate text-sm font-medium text-slate-800 dark:text-slate-200">{event.name}</p>
          <p className="text-xs text-slate-400 dark:text-slate-500">{formatDate(event.eventDate)}</p>
        </div>
        <div className="flex shrink-0 flex-col items-end gap-0.5">
          {urgent && (
            <span
              className={`inline-flex items-center gap-1 whitespace-nowrap text-xs font-medium ${
                critical ? "text-red-600 dark:text-red-400" : "text-amber-600 dark:text-amber-400"
              }`}
            >
              <IconAlertTriangle className="h-3.5 w-3.5 shrink-0" />
              {warningLabel(daysLeft)}
            </span>
          )}
          <span className="text-xs tabular-nums text-slate-500 dark:text-slate-400">{event.relevantInventory} left</span>
        </div>
      </Link>
    </li>
  );
}

/** A single "N of something needs attention" card - simple count + one link
 * to the existing page where the user can act on it. No new filtering is
 * added to that target page; this only links to routes that already exist. */
function AlertCard({
  label,
  count,
  description,
  linkTo,
  linkLabel,
}: {
  label: string;
  count: number;
  description: string;
  linkTo: string;
  linkLabel: string;
}) {
  return (
    <Card className="p-4">
      <p className="text-sm font-medium text-slate-800 dark:text-slate-200">{label}</p>
      <p
        className={`mt-1.5 text-2xl font-semibold tabular-nums ${count > 0 ? "text-amber-600 dark:text-amber-400" : "text-slate-900 dark:text-slate-100"}`}
      >
        {count}
      </p>
      <p className="mt-1 text-xs text-slate-400 dark:text-slate-500">{description}</p>
      <Link to={linkTo} className="mt-3 inline-block text-xs font-medium text-brand-700 hover:underline dark:text-brand-400">
        {linkLabel} &rarr;
      </Link>
    </Card>
  );
}

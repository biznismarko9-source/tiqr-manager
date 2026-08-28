import { useCallback, useEffect, useState, type ReactNode } from "react";
import { Link, useNavigate } from "react-router-dom";
import { api, errMsg } from "../lib/api";
import type { DashboardData, DashboardTab, UpcomingEventAlert } from "../lib/types";
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
  IconCalendarDays,
  IconDownload,
  IconPackage,
  IconPlus,
  IconReceipt,
  IconUpload,
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
  const navigate = useNavigate();
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
        }
      />

      {tab === "overview" && (
        <>
          {/* 1.9.6: the Quick Actions row (New Event/Order/Sale/Import/
              Export) used to open right here, above the period switcher -
              marko wanted it lower on the page so it's not the first thing
              you see. It now renders at the very bottom of this tab,
              after the chart - see the matching comment down there for why
              that's inside the data-loaded branch instead of up here. */}

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
                  <div className="mb-6 grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-6">
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
                  <Card className="mb-8 p-4">
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
              {/* 1.9.6: relocated here from the top of this tab (see the
                  comment up there) - marko wanted it lower on the page, not
                  the first thing you see. Same buttons, same behavior
                  (reuses existing routes/modals via navigate(path, {state}),
                  no new backend command or page), just moved. One real
                  side effect of the move: it's now inside the
                  `loading || !data` branch, so it briefly doesn't render
                  during the initial load spinner instead of always being
                  there - a deliberate small tradeoff for "lower on the
                  page" actually meaning lower, not just visually secondary
                  in the same spot. Local SQLite loads are fast enough that
                  this is a moment, not a real gap. */}
              <div className="mt-6 flex flex-wrap items-center gap-2">
                <Button variant="secondary" onClick={() => navigate("/events", { state: { openCreate: true } })}>
                  <IconPlus className="h-4 w-4" /> New Event
                </Button>
                <Button variant="secondary" onClick={() => navigate("/orders", { state: { openCreate: true } })}>
                  <IconPlus className="h-4 w-4" /> New Order
                </Button>
                <Button variant="secondary" onClick={() => navigate("/sales", { state: { openCreate: true } })}>
                  <IconPlus className="h-4 w-4" /> New Sale
                </Button>
                <Button variant="secondary" onClick={() => navigate("/settings/data")}>
                  <IconUpload className="h-4 w-4" /> Import CSV
                </Button>
                <Button variant="secondary" onClick={() => navigate("/settings/data")}>
                  <IconDownload className="h-4 w-4" /> Export CSV
                </Button>
              </div>
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
 * itself (the revenue figure to its right already gives the exact number). */
function SalesByPlatformCard({ data }: { data: DashboardData }) {
  const rows = data.salesByPlatform;
  const maxRevenue = Math.max(1, ...rows.map((r) => r.revenueCents));
  return (
    <RecentCard title="Sales by platform" icon={<IconBarChart className="h-4 w-4" />} className="mb-8">
      {rows.length === 0 ? (
        <EmptyRow text="No sales in this period yet" />
      ) : (
        <ul className="divide-y divide-slate-100 dark:divide-slate-800">
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

/** Dashboard "Attention" section - simple, transparent counts sourced
 * directly from `data.alerts` (see dashboard.rs). No client-side scoring or
 * filtering logic here - this component only renders what the backend
 * already decided is attention-worthy. */
function AttentionSection({ data }: { data: DashboardData }) {
  const { alerts } = data;
  const allClear =
    alerts.unpaidOrdersCount === 0 &&
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
            label="Unpaid payments"
            count={alerts.unpaidOrdersCount}
            description="Orders that are unpaid or only partially paid"
            linkTo="/orders"
            linkLabel="View orders"
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

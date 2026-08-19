import { useCallback, useEffect, useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import { api, errMsg } from "../lib/api";
import type { DashboardData, UpcomingEventAlert } from "../lib/types";
import { formatDate, formatMoney, formatMoneyOrMixed, formatPercent } from "../lib/format";
import { Badge, Button, Card, EmptyState, LoadingBlock, PageHeader, StatCard } from "../components/ui";
import { MetricChart, METRICS, type MetricKey } from "../components/MetricChart";
import {
  IconAlertTriangle,
  IconCalendarDays,
  IconDownload,
  IconPackage,
  IconPlus,
  IconReceipt,
  IconUpload,
} from "../components/icons";
import { useToast } from "../lib/toast";

// Same fixed window the backend uses (dashboard.rs::UPCOMING_EVENT_WINDOW_DAYS)
// - shown in the section label only, never re-derived/recomputed here.
const UPCOMING_EVENT_WINDOW_DAYS = 14;

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
        }
      />

      {/* 1.8.3 (section 11): small, secondary launcher row - every button
          just reuses an existing route/modal via the same navigate(path,
          {state}) convention Orders.tsx/Sales.tsx/Events.tsx already use
          elsewhere (see their `openCreate` handling); no new backend
          command and no new page. Kept visually secondary (small, plain
          buttons, no card/heading) so it doesn't compete with Activity
          below. */}
      <div className="mb-5 flex flex-wrap items-center gap-2">
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

      {loading || !data ? (
        <LoadingBlock label="Loading dashboard..." />
      ) : (
        <>
          {data.mixedCurrencies && (
            <div className="mb-4 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-800 dark:border-amber-500/30 dark:bg-amber-500/10 dark:text-amber-400">
              You have data in more than one currency. To avoid adding different currencies together, the totals
              below only include <b>{data.primaryCurrency}</b>. Filter by event/platform to see the others.
            </div>
          )}
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
                <StatCard label="Revenue" value={formatMoney(data.period.revenueCents, data.primaryCurrency)} />
                <StatCard label="Purchase cost" value={formatMoney(data.period.totalCostCents, data.primaryCurrency)} />
                <StatCard
                  label="Profit"
                  value={formatMoney(data.period.profitCents, data.primaryCurrency)}
                  tone={data.period.profitCents > 0 ? "positive" : data.period.profitCents < 0 ? "negative" : "default"}
                />
                <StatCard label="Margin" value={formatPercent(data.period.margin)} />
                <StatCard label="ROI" value={formatPercent(data.period.roi)} />
                <StatCard
                  label="Tickets sold"
                  value={String(data.period.soldTickets)}
                  sub={`${data.period.purchasedTickets} purchased in period`}
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
                  rather than a new visual pattern. */}
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
            </>
          )}

          <p className="mb-2 text-xs font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">
            Current inventory (all time)
          </p>
          <div className="mb-8 grid grid-cols-2 gap-3 sm:grid-cols-4">
            <StatCard label="Available" value={String(data.inventory.availableTickets)} />
            <StatCard label="Listed" value={String(data.inventory.listedTickets)} />
            <StatCard label="Sold (total)" value={String(data.inventory.soldTickets)} />
            <StatCard label="Purchased (total)" value={String(data.inventory.purchasedTickets)} />
          </div>

          {/* Inventory & Potential Profit: deliberately its own tinted zone
              (not plain StatCards like the sections above) and its own
              "Potential profit" label - never "Profit" alone - so it can
              never be mistaken for the realized Profit shown in Activity
              above. Not affected by the period filter (unsold stock is a
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
                understates what full inventory could be worth once priced - see Attention below.
              </p>
            )}
          </div>

          <AttentionSection data={data} />

          <div className="grid grid-cols-1 gap-5 lg:grid-cols-3">
            <RecentCard title="Recent events" icon={<IconCalendarDays className="h-4 w-4" />}>
              {data.recentEvents.length === 0 ? (
                <EmptyRow text="No events yet" />
              ) : (
                <ul className="divide-y divide-slate-100 dark:divide-slate-800">
                  {data.recentEvents.map((ev) => (
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
              )}
            </RecentCard>

            <RecentCard title="Recent orders" icon={<IconPackage className="h-4 w-4" />}>
              {data.recentOrders.length === 0 ? (
                <EmptyRow text="No orders yet" />
              ) : (
                <ul className="divide-y divide-slate-100 dark:divide-slate-800">
                  {data.recentOrders.map((o) => (
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
              )}
            </RecentCard>

            <RecentCard title="Recent sales" icon={<IconReceipt className="h-4 w-4" />}>
              {data.recentSales.length === 0 ? (
                <EmptyRow text="No sales yet" />
              ) : (
                <ul className="divide-y divide-slate-100 dark:divide-slate-800">
                  {data.recentSales.map((s) => (
                    <li key={s.id} className="flex items-center justify-between gap-2 px-4 py-2.5">
                      <div className="min-w-0">
                        <p className="truncate text-sm font-medium text-slate-800 dark:text-slate-200">{s.eventName}</p>
                        <p className="text-xs text-slate-400 dark:text-slate-500">{formatDate(s.saleDate)}</p>
                      </div>
                      {s.paymentStatus === "refunded" ? (
                        // BUG #3: a refunded sale must never look like a normal
                        // completed one. Recent activity intentionally still
                        // includes it (same "history is never hidden" rule as
                        // Sales/Sale Detail), just clearly marked - same Badge
                        // tone="refunded" already used on the Sales screen.
                        <div className="flex shrink-0 flex-col items-end gap-0.5">
                          <Badge tone="refunded">Refunded</Badge>
                          <p className="text-xs tabular-nums text-slate-400 line-through dark:text-slate-500">
                            {formatMoney(s.salePriceCents, s.currency)}
                          </p>
                        </div>
                      ) : (
                        <p className="shrink-0 text-sm tabular-nums text-emerald-600 dark:text-emerald-400">
                          {formatMoney(s.salePriceCents, s.currency)}
                        </p>
                      )}
                    </li>
                  ))}
                </ul>
              )}
            </RecentCard>
          </div>
        </>
      )}
    </div>
  );
}

function RecentCard({
  title,
  icon,
  children,
}: {
  title: string;
  icon: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <Card>
      <div className="flex items-center gap-2 border-b border-slate-100 dark:border-slate-800 px-4 py-3">
        <span className="text-slate-400 dark:text-slate-500">{icon}</span>
        <h3 className="text-sm font-semibold text-slate-800 dark:text-slate-200">{title}</h3>
      </div>
      {children}
    </Card>
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
            count={alerts.missingListingPriceCount}
            description="Available/listed tickets with no listing price set"
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

function UpcomingEventRow({ event }: { event: UpcomingEventAlert }) {
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
        <span className="shrink-0 text-xs tabular-nums text-slate-500 dark:text-slate-400">{event.relevantInventory} left</span>
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

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Link } from "react-router-dom";
import { save } from "@tauri-apps/plugin-dialog";
import { api, errMsg } from "../lib/api";
import type { DashboardData, EventWithStats, FinanceSummary } from "../lib/types";
import {
  computeTrend,
  computeTrendPoints,
  formatDate,
  formatMoney,
  formatMoneyOrMixed,
  formatPercent,
  type TrendInfo,
} from "../lib/format";
import { Button, EmptyState, Spinner } from "./ui";
import { MetricChart, METRICS, type MetricKey } from "./MetricChart";
import { IconAlertTriangle, IconCheck, IconDownload, IconX } from "./icons";
import { useToast } from "../lib/toast";

/* ==========================================================================
   TIQR Recap (2.24.0)

   A PRESENTATION LAYER. Not a reporting backend, not a second finance engine,
   and not one new business calculation - marko was explicit about all three,
   and the honest finding while building it was that none were needed.

   Everything on screen comes from ONE existing, already-verified call:
   `get_dashboard(from, to)`. That one command already returns the period's
   FinanceSummary, the equal-length previous period, the cashflow split, the
   inventory potential, the per-platform breakdown and the time series. So the
   Recap makes one round trip, not the "dozens of separate DB queries" the
   brief warned against - and every number it shows is the same number the
   Dashboard shows, from the same aggregation, under the same definitions.

   The one thing it fetches separately is the event list, and only for the
   all-time "best event" line - see `bestEventAllTime`, which is labelled as
   all-time on screen precisely because that is the scope that exists.

   THREE SCOPES, NEVER MIXED - the rule the whole layout is built around:

     REALIZED   money that has actually moved: sales made, cost paid.
     PENDING    money that is owed, either way, and has not moved yet.
     POTENTIAL  what unsold stock MIGHT be worth. Never called profit.

   Each has its own band, its own heading and its own one-line definition, so
   a figure can never be read as belonging to a stronger claim than it makes.
   ========================================================================== */

export type RecapKind = "ticket" | "finance";

export const RECAP_PERIODS = [
  { key: "thisMonth", label: "This month" },
  { key: "lastMonth", label: "Last month" },
  { key: "3m", label: "3 months" },
  { key: "6m", label: "6 months" },
  { key: "thisYear", label: "This year" },
  { key: "custom", label: "Custom" },
] as const;

export type RecapPeriodKey = (typeof RECAP_PERIODS)[number]["key"];

const iso = (d: Date) =>
  `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;

/**
 * The date range each period means, as two concrete days.
 *
 * Always resolved to explicit `from`/`to` rather than passing a period NAME to
 * the backend: the Dashboard's own period vocabulary ("1 Mo", "YTD") is a
 * different set of ideas from the calendar months marko asked for here, and
 * teaching the backend a second vocabulary would be exactly the new business
 * logic this feature is not allowed to add. `get_dashboard` already takes a
 * concrete range under its existing `period: "custom"`, so it learns nothing
 * new - the recap just hands it two dates.
 *
 * "This month" and "Last month" are CALENDAR months. "3 months" and "6 months"
 * are rolling windows ending today, which is what those words mean when nobody
 * says "calendar" - and the label under the picker says which is which, so it
 * is never a guess.
 *
 * Exported for the range check in the release notes: this is pure date maths
 * with no React and no network, so it can be run and read on its own.
 */
export function recapRange(
  key: RecapPeriodKey,
  today: Date,
  customFrom?: string,
  customTo?: string,
): { from: string; to: string; note: string } {
  const y = today.getFullYear();
  const m = today.getMonth();
  switch (key) {
    case "thisMonth":
      return { from: iso(new Date(y, m, 1)), to: iso(today), note: "calendar month so far" };
    case "lastMonth":
      // Day 0 of this month IS the last day of the previous one - no
      // month-length table, and February and leap years fall out correctly.
      return { from: iso(new Date(y, m - 1, 1)), to: iso(new Date(y, m, 0)), note: "whole calendar month" };
    case "3m":
      return { from: iso(new Date(y, m - 3, today.getDate())), to: iso(today), note: "rolling, ending today" };
    case "6m":
      return { from: iso(new Date(y, m - 6, today.getDate())), to: iso(today), note: "rolling, ending today" };
    case "thisYear":
      return { from: iso(new Date(y, 0, 1)), to: iso(today), note: "1 January to today" };
    case "custom":
    default:
      return {
        from: customFrom || iso(new Date(y, m, 1)),
        to: customTo || iso(today),
        note: "your own dates",
      };
  }
}

/** Per-ticket figures the recap shows. Division by a zero count is `null`, not
 *  zero - the same rule `finance::safe_ratio` follows everywhere else in this
 *  app, because "no answer" and "zero" are different claims. */
function perTicket(cents: number, tickets: number): number | null {
  return tickets > 0 ? Math.round(cents / tickets) : null;
}

/* ========================== small presentational pieces ================== */

function Band({
  tone,
  title,
  meaning,
  children,
}: {
  tone: "realized" | "pending" | "potential";
  title: string;
  meaning: string;
  children: React.ReactNode;
}) {
  const ring = {
    realized: "ring-emerald-500/25 bg-emerald-500/[0.04]",
    pending: "ring-amber-500/25 bg-amber-500/[0.04]",
    potential: "ring-slate-400/20 bg-slate-500/[0.04]",
  }[tone];
  const dot = { realized: "bg-emerald-500", pending: "bg-amber-500", potential: "bg-slate-400" }[tone];
  return (
    <section className={`rounded-2xl p-5 ring-1 ring-inset ${ring}`}>
      <div className="mb-4 flex flex-wrap items-baseline gap-x-3 gap-y-1">
        <span className="flex items-center gap-2">
          <span className={`h-1.5 w-1.5 rounded-full ${dot}`} />
          <h3 className="text-[13px] font-semibold uppercase tracking-[0.09em] text-slate-700 dark:text-slate-200">
            {title}
          </h3>
        </span>
        {/* The definition travels with the band. Without it "pending" and
            "potential" read as the same kind of number, which is the exact
            confusion this whole layout exists to prevent. */}
        <span className="text-xs text-slate-500 dark:text-slate-400">{meaning}</span>
      </div>
      {children}
    </section>
  );
}

function Figure({
  label,
  value,
  sub,
  tone = "default",
  big = false,
}: {
  label: string;
  value: string;
  sub?: string;
  tone?: "default" | "positive" | "negative";
  big?: boolean;
}) {
  const color =
    tone === "positive"
      ? "text-emerald-600 dark:text-emerald-400"
      : tone === "negative"
        ? "text-red-600 dark:text-red-400"
        : "text-slate-900 dark:text-slate-50";
  return (
    <div className="min-w-0">
      <p className="truncate text-[11px] font-medium uppercase tracking-[0.07em] text-slate-500 dark:text-slate-400">
        {label}
      </p>
      <p className={`mt-1.5 truncate font-semibold tabular-nums ${big ? "text-[34px]" : "text-[22px]"} leading-none ${color}`}>
        {value}
      </p>
      {sub && <p className="mt-1.5 truncate text-[11px] text-slate-400 dark:text-slate-500">{sub}</p>}
    </div>
  );
}

function Grid({ children }: { children: React.ReactNode }) {
  return <div className="grid gap-5" style={{ gridTemplateColumns: "repeat(auto-fit, minmax(10rem, 1fr))" }}>{children}</div>;
}

/** A trend, rendered the same way the Dashboard's own cards render one - same
 *  helpers, same up/down colouring, same "vs. previous" wording, so a reader
 *  never has to learn a second visual language for the same idea. */
function Delta({ trend, colored = true }: { trend: TrendInfo | null; colored?: boolean }) {
  if (!trend) return <span className="text-slate-400 dark:text-slate-500">-</span>;
  const cls =
    !colored || trend.direction === "flat"
      ? "text-slate-400 dark:text-slate-500"
      : trend.direction === "up"
        ? "text-emerald-600 dark:text-emerald-400"
        : "text-red-600 dark:text-red-400";
  const arrow = trend.direction === "up" ? "▲" : trend.direction === "down" ? "▼" : "";
  return (
    <span className={`inline-flex items-center gap-1 font-medium tabular-nums ${cls}`}>
      {arrow && <span className="text-[9px] leading-none">{arrow}</span>}
      {trend.label}
    </span>
  );
}

/** Purchased / Sold / Remaining as one bar.
 *
 *  Deliberately NOT a time series: nothing in the existing aggregation reports
 *  how many tickets were BOUGHT in each bucket, only how many sold, so a
 *  purchased-over-time line would have to be invented. This is the same three
 *  numbers as a composition, which is what they actually are. */
function StockBar({ bought, sold, remaining }: { bought: number; sold: number; remaining: number }) {
  const total = Math.max(1, sold + remaining);
  const parts = [
    { key: "sold", n: sold, label: "Sold", cls: "bg-emerald-500" },
    { key: "remaining", n: remaining, label: "Remaining", cls: "bg-brand-500" },
  ].filter((p) => p.n > 0);
  return (
    <div>
      <div className="flex h-3 gap-0.5 overflow-hidden rounded-full">
        {parts.length === 0 ? (
          <div className="h-3 w-full rounded-full bg-slate-200 dark:bg-slate-800" />
        ) : (
          parts.map((p) => (
            <div
              key={p.key}
              className={`${p.cls} first:rounded-l-full last:rounded-r-full`}
              style={{ width: `${(p.n / total) * 100}%` }}
              title={`${p.label}: ${p.n}`}
            />
          ))
        )}
      </div>
      <div className="mt-3 flex flex-wrap gap-x-6 gap-y-1.5 text-xs">
        <span className="text-slate-500 dark:text-slate-400">
          <b className="font-semibold tabular-nums text-slate-800 dark:text-slate-100">{bought}</b> bought in this period
        </span>
        <span className="flex items-center gap-1.5 text-slate-500 dark:text-slate-400">
          <span className="h-2 w-2 rounded-sm bg-emerald-500" />
          <b className="font-semibold tabular-nums text-slate-800 dark:text-slate-100">{sold}</b> sold
        </span>
        <span className="flex items-center gap-1.5 text-slate-500 dark:text-slate-400">
          <span className="h-2 w-2 rounded-sm bg-brand-500" />
          <b className="font-semibold tabular-nums text-slate-800 dark:text-slate-100">{remaining}</b> still held (all time)
        </span>
      </div>
    </div>
  );
}

/* ============================ the summary table ========================== */

type SummaryRow = {
  label: string;
  now: string;
  prev: string;
  trend: TrendInfo | null;
  colored?: boolean;
};

function SummaryTable({ rows, note }: { rows: SummaryRow[]; note: string }) {
  return (
    <div>
      <div className="overflow-x-auto">
        <table className="w-full">
          <thead>
            <tr>
              <th className="th">Metric</th>
              <th className="th text-right">This period</th>
              <th className="th text-right">Previous period</th>
              <th className="th text-right">Change</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
            {rows.map((r) => (
              <tr key={r.label}>
                <td className="td">{r.label}</td>
                <td className="td text-right tabular-nums">{r.now}</td>
                <td className="td text-right tabular-nums text-slate-400 dark:text-slate-500">{r.prev}</td>
                <td className="td text-right">
                  <Delta trend={r.trend} colored={r.colored !== false} />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <p className="mt-3 text-xs leading-relaxed text-slate-400 dark:text-slate-500">{note}</p>
    </div>
  );
}

/* ============================== share card =============================== */

/** Builds the shareable card as SVG, laid out by hand.
 *
 *  Not a screenshot of the app, which is what marko asked it not to look like -
 *  and not a screenshot library either, since this project carries no UI
 *  dependencies. Hand-laid SVG also means the card is sized and composed for
 *  sharing (1200x675) rather than being whatever happened to be on screen.
 *
 *  System font names only: an SVG rasterised through an <img> cannot fetch an
 *  external font, so anything else would silently fall back and ruin the
 *  spacing.
 */
function buildShareSvg(title: string, rangeLabel: string, figures: { label: string; value: string }[], footer: string): string {
  const esc = (t: string) => t.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
  const cols = Math.min(3, Math.max(1, Math.ceil(figures.length / 2)));
  const cellW = 1040 / cols;
  const cells = figures
    .map((f, i) => {
      const x = 80 + (i % cols) * cellW;
      const yTop = 300 + Math.floor(i / cols) * 150;
      return `
    <text x="${x}" y="${yTop}" fill="#94a3b8" font-family="system-ui, -apple-system, Segoe UI, Roboto, sans-serif" font-size="18" font-weight="600" letter-spacing="1.6">${esc(
        f.label.toUpperCase(),
      )}</text>
    <text x="${x}" y="${yTop + 52}" fill="#f8fafc" font-family="system-ui, -apple-system, Segoe UI, Roboto, sans-serif" font-size="44" font-weight="650">${esc(
        f.value,
      )}</text>`;
    })
    .join("");
  return `<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="675" viewBox="0 0 1200 675">
  <defs>
    <linearGradient id="bg" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0%" stop-color="#0e1116"/>
      <stop offset="55%" stop-color="#12151c"/>
      <stop offset="100%" stop-color="#171a27"/>
    </linearGradient>
    <linearGradient id="rule" x1="0" y1="0" x2="1" y2="0">
      <stop offset="0%" stop-color="#6483f9"/>
      <stop offset="100%" stop-color="#6483f9" stop-opacity="0"/>
    </linearGradient>
  </defs>
  <rect width="1200" height="675" fill="url(#bg)"/>
  <rect x="80" y="86" width="360" height="3" fill="url(#rule)"/>
  <text x="80" y="70" fill="#6483f9" font-family="system-ui, -apple-system, Segoe UI, Roboto, sans-serif" font-size="20" font-weight="700" letter-spacing="3">TIQR RECAP</text>
  <text x="80" y="160" fill="#f8fafc" font-family="system-ui, -apple-system, Segoe UI, Roboto, sans-serif" font-size="52" font-weight="650">${esc(title)}</text>
  <text x="80" y="205" fill="#94a3b8" font-family="system-ui, -apple-system, Segoe UI, Roboto, sans-serif" font-size="24">${esc(rangeLabel)}</text>
  ${cells}
  <text x="80" y="625" fill="#64748b" font-family="system-ui, -apple-system, Segoe UI, Roboto, sans-serif" font-size="17">${esc(footer)}</text>
</svg>`;
}

/** SVG -> canvas -> PNG data URL, with no library.
 *
 *  The SVG goes in as a blob URL rather than a `data:` URL because a data URL
 *  breaks on any non-ASCII character in the text unless it is base64'd first,
 *  and money strings carry € as a matter of course. */
function svgToPngDataUrl(svg: string, scale = 2): Promise<string> {
  return new Promise((resolve, reject) => {
    const blob = new Blob([svg], { type: "image/svg+xml;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const img = new Image();
    img.onload = () => {
      const canvas = document.createElement("canvas");
      canvas.width = 1200 * scale;
      canvas.height = 675 * scale;
      const ctx = canvas.getContext("2d");
      if (!ctx) {
        URL.revokeObjectURL(url);
        reject(new Error("no canvas"));
        return;
      }
      ctx.drawImage(img, 0, 0, canvas.width, canvas.height);
      URL.revokeObjectURL(url);
      resolve(canvas.toDataURL("image/png"));
    };
    img.onerror = () => {
      URL.revokeObjectURL(url);
      reject(new Error("could not render the card"));
    };
    img.src = url;
  });
}

/* ================================= recap ================================= */

export function Recap({ kind, onClose }: { kind: RecapKind; onClose: () => void }) {
  const toast = useToast();
  const [periodKey, setPeriodKey] = useState<RecapPeriodKey>("thisMonth");
  const [customFrom, setCustomFrom] = useState("");
  const [customTo, setCustomTo] = useState("");
  const [data, setData] = useState<DashboardData | null>(null);
  const [events, setEvents] = useState<EventWithStats[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [metric, setMetric] = useState<MetricKey>(kind === "finance" ? "profit" : "sales");
  const [sharing, setSharing] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);

  const range = useMemo(
    () => recapRange(periodKey, new Date(), customFrom, customTo),
    [periodKey, customFrom, customTo],
  );

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    // ONE call. Everything below reads off this single existing aggregation -
    // see this file's header for why that is the whole design.
    api
      // `period: "custom"` is REQUIRED alongside from/to. `period_bounds` in
      // dashboard.rs matches on the period name first and only reads from/to
      // under "custom" - passing the dates alone falls into the `None` arm,
      // which returns today->today and would have made every recap show a
      // single day's figures under a month's heading. Checked against that
      // function rather than assumed.
      .getDashboard({ period: "custom", from: range.from, to: range.to })
      .then((d) => {
        if (!cancelled) setData(d);
      })
      .catch((e) => {
        if (!cancelled) setError(errMsg(e));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [range.from, range.to]);

  useEffect(() => {
    // Separate, and only for the all-time highlight - see `bestEventAllTime`.
    api
      .listEvents({})
      .then(setEvents)
      .catch(() => setEvents([]));
  }, []);

  // Esc closes, like every other overlay in the app.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [onClose]);

  const title = kind === "ticket" ? "Ticket Recap" : "Finance Recap";

  /** The best event by REALIZED profit, all time.
   *
   *  All-time on purpose and said so on screen: `EventWithStats.stats` is an
   *  all-time FinanceSummary, which is the scope that exists. Re-scoping it to
   *  the recap's period would mean a new per-event, per-period aggregation -
   *  a new business calculation, which this feature is not allowed to add. */
  const bestEventAllTime = useMemo(() => {
    const withProfit = events.filter((e) => e.stats.soldTickets > 0);
    if (withProfit.length === 0) return null;
    return withProfit.reduce((a, b) => (b.stats.profitCents > a.stats.profitCents ? b : a));
  }, [events]);

  const share = useCallback(async () => {
    if (!data) return;
    setSharing(true);
    try {
      const p = data.period;
      const cur = data.primaryCurrency;
      const figures =
        kind === "ticket"
          ? [
              { label: "Tickets sold", value: String(p.soldTickets) },
              { label: "Revenue", value: formatMoney(p.revenueCents, cur) },
              { label: "Realized profit", value: formatMoney(p.profitCents, cur) },
              { label: "Tickets bought", value: String(p.purchasedTickets) },
              { label: "Invested", value: formatMoney(p.totalCostCents, cur) },
              { label: "ROI", value: formatPercent(p.roi) },
            ]
          : [
              { label: "Money in", value: formatMoney(p.revenueCents, cur) },
              { label: "Money out", value: formatMoney(p.totalCostCents + p.sellingFeesCents, cur) },
              { label: "Realized profit", value: formatMoney(p.profitCents, cur) },
              { label: "Collected", value: formatMoneyOrMixed(data.cashflow.paidCents, data.cashflow.currency) },
              { label: "Still owed to you", value: formatMoneyOrMixed(data.cashflow.outstandingCents, data.cashflow.currency) },
              { label: "Capital tied up", value: formatMoneyOrMixed(data.inventoryPotential.inventoryCostCents, data.inventoryPotential.currency) },
            ];
      const svg = buildShareSvg(
        title,
        `${formatDate(range.from)} - ${formatDate(range.to)}`,
        figures,
        "Realized figures only. Unsold stock is not counted as profit.",
      );
      const dataUrl = await svgToPngDataUrl(svg);
      const stamp = range.to.replace(/-/g, "");
      const path = await save({
        defaultPath: `tiqr-${kind}-recap-${stamp}.png`,
        filters: [{ name: "PNG image", extensions: ["png"] }],
      });
      if (!path) return;
      await api.savePngFile(path, dataUrl.replace(/^data:image\/png;base64,/, ""));
      toast.success(`Saved to ${path}`);
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setSharing(false);
    }
  }, [data, kind, range, title, toast]);

  const body = () => {
    if (loading) {
      return (
        <div className="flex items-center justify-center py-28 text-sm text-slate-500 dark:text-slate-400">
          <Spinner className="mr-3 h-5 w-5" /> Reading your numbers...
        </div>
      );
    }
    if (error) {
      return (
        <div className="py-20">
          <EmptyState title={error} />
        </div>
      );
    }
    if (!data) return null;

    const p: FinanceSummary = data.period;
    const prev = data.previousPeriod;
    const inv = data.inventory;
    const cur = data.primaryCurrency;
    const remaining = inv.availableTickets + inv.listedTickets;
    const nothingHappened = p.soldTickets === 0 && p.purchasedTickets === 0;

    const money = (c: number) => formatMoney(c, cur);
    const nowPer = perTicket(p.profitCents, p.soldTickets);
    const prevPer = prev ? perTicket(prev.profitCents, prev.soldTickets) : null;

    const summaryRows: SummaryRow[] =
      kind === "ticket"
        ? [
            { label: "Tickets bought", now: String(p.purchasedTickets), prev: prev ? String(prev.purchasedTickets) : "-", trend: prev ? computeTrend(p.purchasedTickets, prev.purchasedTickets) : null, colored: false },
            { label: "Tickets sold", now: String(p.soldTickets), prev: prev ? String(prev.soldTickets) : "-", trend: prev ? computeTrend(p.soldTickets, prev.soldTickets) : null },
            { label: "Revenue", now: money(p.revenueCents), prev: prev ? money(prev.revenueCents) : "-", trend: prev ? computeTrend(p.revenueCents, prev.revenueCents) : null },
            { label: "Realized profit", now: money(p.profitCents), prev: prev ? money(prev.profitCents) : "-", trend: prev ? computeTrend(p.profitCents, prev.profitCents) : null },
            { label: "ROI", now: formatPercent(p.roi), prev: prev ? formatPercent(prev.roi) : "-", trend: prev ? computeTrendPoints(p.roi, prev.roi) : null },
            { label: "Profit per ticket", now: nowPer === null ? "-" : money(nowPer), prev: prevPer === null ? "-" : money(prevPer), trend: nowPer === null ? null : computeTrend(nowPer, prevPer) },
          ]
        : [
            { label: "Money in", now: money(p.revenueCents), prev: prev ? money(prev.revenueCents) : "-", trend: prev ? computeTrend(p.revenueCents, prev.revenueCents) : null },
            { label: "Money out", now: money(p.totalCostCents + p.sellingFeesCents), prev: prev ? money(prev.totalCostCents + prev.sellingFeesCents) : "-", trend: prev ? computeTrend(p.totalCostCents + p.sellingFeesCents, prev.totalCostCents + prev.sellingFeesCents) : null, colored: false },
            { label: "Platform fees", now: money(p.sellingFeesCents), prev: prev ? money(prev.sellingFeesCents) : "-", trend: prev ? computeTrend(p.sellingFeesCents, prev.sellingFeesCents) : null, colored: false },
            { label: "Realized profit", now: money(p.profitCents), prev: prev ? money(prev.profitCents) : "-", trend: prev ? computeTrend(p.profitCents, prev.profitCents) : null },
            { label: "Margin", now: formatPercent(p.margin), prev: prev ? formatPercent(prev.margin) : "-", trend: prev ? computeTrendPoints(p.margin, prev.margin) : null },
            { label: "ROI", now: formatPercent(p.roi), prev: prev ? formatPercent(prev.roi) : "-", trend: prev ? computeTrendPoints(p.roi, prev.roi) : null },
          ];

    return (
      <>
        {/* ---------------- hero ---------------- */}
        <div className="relative overflow-hidden rounded-2xl border border-slate-200 bg-gradient-to-br from-white via-white to-brand-50/50 px-7 py-8 dark:border-slate-800 dark:from-slate-900 dark:via-slate-900 dark:to-brand-500/[0.07]">
          <p className="text-[11px] font-semibold uppercase tracking-[0.14em] text-brand-600 dark:text-brand-400">
            {title}
          </p>
          <p className="mt-1 text-sm text-slate-500 dark:text-slate-400">
            {formatDate(range.from)} — {formatDate(range.to)} · {range.note}
          </p>
          {nothingHappened ? (
            <p className="mt-6 text-[30px] font-semibold leading-none text-slate-400 dark:text-slate-500">
              Nothing bought or sold in this period
            </p>
          ) : (
            <div className="mt-6 flex flex-wrap items-end gap-x-12 gap-y-5">
              <div>
                <p className="text-[11px] font-medium uppercase tracking-[0.07em] text-slate-500 dark:text-slate-400">
                  {kind === "ticket" ? "Tickets sold" : "Realized profit"}
                </p>
                <p
                  className={`mt-2 text-[56px] font-semibold leading-none tracking-tight tabular-nums ${
                    kind === "finance"
                      ? p.profitCents > 0
                        ? "text-emerald-600 dark:text-emerald-400"
                        : p.profitCents < 0
                          ? "text-red-600 dark:text-red-400"
                          : "text-slate-900 dark:text-slate-50"
                      : "text-slate-900 dark:text-slate-50"
                  }`}
                >
                  {kind === "ticket" ? p.soldTickets : money(p.profitCents)}
                </p>
              </div>
              <div className="flex flex-wrap items-end gap-x-10 gap-y-4">
                <Figure
                  label={kind === "ticket" ? "Realized profit" : "Money in"}
                  value={kind === "ticket" ? money(p.profitCents) : money(p.revenueCents)}
                  tone={kind === "ticket" && p.profitCents > 0 ? "positive" : "default"}
                />
                <Figure label="ROI" value={formatPercent(p.roi)} />
                {prev && (
                  <div className="min-w-0">
                    <p className="text-[11px] font-medium uppercase tracking-[0.07em] text-slate-500 dark:text-slate-400">
                      vs. previous
                    </p>
                    <p className="mt-1.5 text-[22px] leading-none">
                      <Delta trend={computeTrend(p.profitCents, prev.profitCents)} />
                    </p>
                  </div>
                )}
              </div>
            </div>
          )}
        </div>

        {data.mixedCurrencies && (
          <p className="mt-3 flex items-start gap-2 text-xs text-amber-700 dark:text-amber-400">
            <IconAlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0" />
            You have records in more than one currency. Every total here is in {cur} only - the rest are left out rather
            than converted at a rate this app did not have at the time.
          </p>
        )}

        {/* ---------------- realized ---------------- */}
        <div className="mt-6 space-y-5">
          <Band tone="realized" title="Realized" meaning="money that has actually moved, inside this period">
            {kind === "ticket" ? (
              <Grid>
                <Figure label="Tickets bought" value={String(p.purchasedTickets)} />
                <Figure label="Tickets sold" value={String(p.soldTickets)} />
                <Figure label="Total invested" value={money(p.totalCostCents)} />
                <Figure label="Sales revenue" value={money(p.revenueCents)} />
                <Figure label="Realized profit" value={money(p.profitCents)} tone={p.profitCents > 0 ? "positive" : p.profitCents < 0 ? "negative" : "default"} />
                <Figure label="ROI" value={formatPercent(p.roi)} sub={`${formatPercent(p.margin)} margin`} />
              </Grid>
            ) : (
              <Grid>
                <Figure label="Money in" value={money(p.revenueCents)} sub="sales made in this period" />
                <Figure label="Money out" value={money(p.totalCostCents + p.sellingFeesCents)} sub="tickets bought + platform fees" />
                <Figure label="Realized profit" value={money(p.profitCents)} tone={p.profitCents > 0 ? "positive" : p.profitCents < 0 ? "negative" : "default"} />
                <Figure
                  label="Received payouts"
                  value={formatMoneyOrMixed(data.cashflow.paidCents, data.cashflow.currency)}
                  sub="collected from buyers, all time"
                />
              </Grid>
            )}
          </Band>

          {/* ---------------- pending ---------------- */}
          <Band tone="pending" title="Pending" meaning="owed, one way or the other, and not moved yet">
            <Grid>
              <Figure
                label="Pending payouts"
                value={formatMoneyOrMixed(data.cashflow.outstandingCents, data.cashflow.currency)}
                sub={`${data.alerts.pendingSalesCount} sale${data.alerts.pendingSalesCount === 1 ? "" : "s"} not collected`}
              />
              {/* Count only, and said so: the app tracks WHICH orders are
                  unpaid but never totals what is owed on them, and adding that
                  sum would be a new business calculation. */}
              <Figure
                label="Unpaid orders"
                value={String(data.alerts.unpaidOrdersCount)}
                sub="amount owed is not tracked"
              />
              <Figure
                label="Pulls to transfer"
                value={String(data.alerts.pullsNeedingTransferCount)}
                sub="past or near their deadline"
              />
            </Grid>
          </Band>

          {/* ---------------- potential ---------------- */}
          <Band tone="potential" title="Potential" meaning="what unsold stock might be worth - never counted as profit">
            <Grid>
              <Figure label="Tickets still held" value={String(remaining)} sub="all time, right now" />
              <Figure
                label="Capital tied up"
                value={formatMoneyOrMixed(data.inventoryPotential.inventoryCostCents, data.inventoryPotential.currency)}
                sub="what those cost you"
              />
              <Figure
                label="Listing value"
                value={formatMoneyOrMixed(data.inventoryPotential.listingValueCents, data.inventoryPotential.currency)}
                sub="only the ones that have a price"
              />
              <Figure
                label="Potential profit"
                value={formatMoneyOrMixed(data.inventoryPotential.potentialProfitCents, data.inventoryPotential.currency)}
                sub="if they all sold at those prices"
              />
            </Grid>
            {data.alerts.missingListingPriceCount > 0 && (
              <p className="mt-4 flex items-start gap-2 text-xs text-amber-700 dark:text-amber-400">
                <IconAlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0" />
                {data.alerts.missingListingPriceCount} of those have no listing price, so they count against the capital
                above but add nothing to the value.
              </p>
            )}
          </Band>
        </div>

        {/* ---------------- stock + chart ---------------- */}
        <div className="mt-6 grid gap-5 xl:grid-cols-2">
          <section className="rounded-2xl border border-slate-200 p-5 dark:border-slate-800">
            <h3 className="mb-4 text-[13px] font-semibold uppercase tracking-[0.09em] text-slate-700 dark:text-slate-200">
              Bought, sold, still held
            </h3>
            <StockBar bought={p.purchasedTickets} sold={p.soldTickets} remaining={remaining} />
          </section>

          <section className="rounded-2xl border border-slate-200 p-5 dark:border-slate-800">
            <div className="mb-4 flex flex-wrap items-center justify-between gap-3">
              <h3 className="text-[13px] font-semibold uppercase tracking-[0.09em] text-slate-700 dark:text-slate-200">
                Over the period
              </h3>
              <div className="flex gap-1 rounded-lg border border-slate-200 p-1 dark:border-slate-800">
                {METRICS.map((m) => (
                  <button
                    key={m.key}
                    type="button"
                    onClick={() => setMetric(m.key)}
                    aria-pressed={metric === m.key}
                    className={`rounded-md px-2.5 py-1 text-xs transition-colors ${
                      metric === m.key
                        ? "bg-slate-100 font-medium text-slate-900 dark:bg-slate-800 dark:text-slate-100"
                        : "text-slate-500 hover:text-slate-700 dark:text-slate-400 dark:hover:text-slate-200"
                    }`}
                  >
                    {m.label}
                  </button>
                ))}
              </div>
            </div>
            {/* The app's own chart, unchanged - not a second charting
                approach for the same data. */}
            <MetricChart
              points={data.revenueTimeSeries}
              granularity={data.timeSeriesGranularity}
              currency={cur}
              metric={metric}
            />
          </section>
        </div>

        {/* ---------------- highlights ---------------- */}
        <section className="mt-6 rounded-2xl border border-slate-200 p-5 dark:border-slate-800">
          <h3 className="mb-4 text-[13px] font-semibold uppercase tracking-[0.09em] text-slate-700 dark:text-slate-200">
            Highlights
          </h3>
          <div className="grid gap-5 md:grid-cols-2">
            <div>
              <p className="text-[11px] font-medium uppercase tracking-[0.07em] text-slate-500 dark:text-slate-400">
                Best channel this period
              </p>
              {data.salesByPlatform.length === 0 ? (
                <p className="mt-2 text-sm text-slate-400 dark:text-slate-500">No sales in this period.</p>
              ) : (
                (() => {
                  const best = data.salesByPlatform.reduce((a, b) => (b.profitCents > a.profitCents ? b : a));
                  return (
                    <p className="mt-2">
                      <span className="text-[22px] font-semibold text-slate-900 dark:text-slate-50">
                        {best.platformName ?? "No platform"}
                      </span>
                      <span className="ml-3 text-sm tabular-nums text-slate-500 dark:text-slate-400">
                        {money(best.profitCents)} profit · {best.soldTickets} sold
                      </span>
                    </p>
                  );
                })()
              )}
            </div>
            <div>
              <p className="text-[11px] font-medium uppercase tracking-[0.07em] text-slate-500 dark:text-slate-400">
                Best event · all time
              </p>
              {bestEventAllTime === null ? (
                <p className="mt-2 text-sm text-slate-400 dark:text-slate-500">Nothing sold yet.</p>
              ) : (
                <p className="mt-2">
                  <Link
                    to={`/events/${bestEventAllTime.id}`}
                    onClick={onClose}
                    className="text-[22px] font-semibold text-slate-900 hover:text-brand-700 dark:text-slate-50 dark:hover:text-brand-400"
                  >
                    {bestEventAllTime.name}
                  </Link>
                  <span className="ml-3 text-sm tabular-nums text-slate-500 dark:text-slate-400">
                    {formatMoneyOrMixed(bestEventAllTime.stats.profitCents, bestEventAllTime.stats.currency)} profit
                  </span>
                </p>
              )}
            </div>
          </div>
          <p className="mt-4 text-xs leading-relaxed text-slate-400 dark:text-slate-500">
            The event figure is all-time, because that is the scope the app already keeps per event. Biggest single
            sale, fastest-selling event and best tier are deliberately absent: none of them exist as an aggregation
            today, and inventing one would make this a reporting engine rather than a recap.
          </p>
        </section>

        {/* ---------------- summary table ---------------- */}
        <section className="mt-6 rounded-2xl border border-slate-200 p-5 dark:border-slate-800">
          <h3 className="mb-4 text-[13px] font-semibold uppercase tracking-[0.09em] text-slate-700 dark:text-slate-200">
            This period vs. previous
          </h3>
          {prev === null ? (
            <p className="text-sm text-slate-400 dark:text-slate-500">
              This range has nothing before it to compare against.
            </p>
          ) : (
            <SummaryTable
              rows={summaryRows}
              note={
                '"Previous period" is the equal-length window immediately before this one - the app\'s own definition, unchanged. ' +
                "Money and counts change by percent; ROI and margin are already percentages, so they change by percentage POINTS (pp)."
              }
            />
          )}
        </section>

        <div className="h-10" />
      </>
    );
  };

  return (
    <div className="fixed inset-0 z-[80] flex flex-col bg-slate-50 dark:bg-slate-950">
      {/* Header stays put while the report scrolls under it - the period
          picker is the one control someone reaches for repeatedly. */}
      <header className="shrink-0 border-b border-slate-200 bg-white/90 backdrop-blur dark:border-slate-800 dark:bg-slate-900/90">
        <div className="mx-auto flex max-w-6xl flex-wrap items-center gap-3 px-6 py-3">
          <h2 className="text-sm font-semibold text-slate-900 dark:text-slate-100">{title}</h2>
          <div className="flex flex-wrap gap-1 rounded-lg border border-slate-200 p-1 dark:border-slate-800">
            {RECAP_PERIODS.map((p) => (
              <button
                key={p.key}
                type="button"
                onClick={() => setPeriodKey(p.key)}
                aria-pressed={periodKey === p.key}
                className={`rounded-md px-2.5 py-1 text-xs transition-colors ${
                  periodKey === p.key
                    ? "bg-slate-100 font-medium text-slate-900 dark:bg-slate-800 dark:text-slate-100"
                    : "text-slate-500 hover:text-slate-700 dark:text-slate-400 dark:hover:text-slate-200"
                }`}
              >
                {p.label}
              </button>
            ))}
          </div>
          {periodKey === "custom" && (
            <span className="flex items-center gap-1.5">
              <input
                type="date"
                value={customFrom}
                onChange={(e) => setCustomFrom(e.target.value)}
                className="rounded-lg border border-slate-200 bg-white px-2 py-1 text-xs text-slate-700 dark:border-slate-800 dark:bg-slate-900 dark:text-slate-200"
              />
              <span className="text-xs text-slate-400">-</span>
              <input
                type="date"
                value={customTo}
                onChange={(e) => setCustomTo(e.target.value)}
                className="rounded-lg border border-slate-200 bg-white px-2 py-1 text-xs text-slate-700 dark:border-slate-800 dark:bg-slate-900 dark:text-slate-200"
              />
            </span>
          )}
          <div className="ml-auto flex items-center gap-2">
            <Button variant="secondary" disabled={sharing || !data} onClick={share}>
              {sharing ? <Spinner className="h-4 w-4" /> : <IconDownload className="h-4 w-4" />}
              Share as image
            </Button>
            <Button variant="ghost" onClick={onClose} aria-label="Close recap">
              <IconX className="h-4 w-4" />
              Close
            </Button>
          </div>
        </div>
      </header>

      <div ref={scrollRef} className="min-h-0 flex-1 overflow-y-auto">
        <div className="mx-auto max-w-6xl px-6 py-6">{body()}</div>
      </div>
    </div>
  );
}

/** The two entry cards, rendered inside Settings -> Insights.
 *
 *  Deliberately not a sidebar item: marko asked for it to be somewhere you go
 *  when you want to look at how things are going, not something in the way of
 *  everyday work. */
export function InsightsCards() {
  const [open, setOpen] = useState<RecapKind | null>(null);
  const cards: { kind: RecapKind; emoji: string; title: string; blurb: string }[] = [
    {
      kind: "ticket",
      emoji: "🎟️",
      title: "Ticket Recap",
      blurb: "Bought, sold, still held - with what it returned and how that compares to the period before.",
    },
    {
      kind: "finance",
      emoji: "💰",
      title: "Finance Recap",
      blurb: "Money in, money out, what is still owed to you, and how much capital is sitting in unsold stock.",
    },
  ];
  return (
    <>
      <div className="grid gap-4 md:grid-cols-2 lg:max-w-4xl">
        {cards.map((c) => (
          <button
            key={c.kind}
            type="button"
            onClick={() => setOpen(c.kind)}
            className="group relative overflow-hidden rounded-2xl border border-slate-200 bg-gradient-to-br from-white to-slate-50 p-6 text-left transition hover:-translate-y-px hover:border-brand-300 hover:shadow-raised dark:border-slate-800 dark:from-slate-900 dark:to-slate-900/60 dark:hover:border-brand-500/40"
          >
            <span className="text-3xl">{c.emoji}</span>
            <h3 className="mt-3 text-base font-semibold text-slate-900 dark:text-slate-50">{c.title}</h3>
            <p className="mt-1.5 text-xs leading-relaxed text-slate-500 dark:text-slate-400">{c.blurb}</p>
            <span className="mt-4 inline-flex items-center gap-1.5 text-xs font-medium text-brand-600 dark:text-brand-400">
              Open <IconCheck className="h-3 w-3" />
            </span>
          </button>
        ))}
      </div>
      <p className="mt-4 max-w-3xl text-xs leading-relaxed text-slate-400 dark:text-slate-500">
        Both read the same figures the Dashboard uses - nothing here is calculated a second way. Realized, pending and
        potential money are kept apart on purpose: unsold stock is never counted as profit.
      </p>
      {open && <Recap kind={open} onClose={() => setOpen(null)} />}
    </>
  );
}

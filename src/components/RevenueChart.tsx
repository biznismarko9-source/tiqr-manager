import { formatMoney } from "../lib/format";
import type { RevenueTimeSeriesPoint } from "../lib/types";

// Small, dependency-free SVG bar chart. Deliberately hand-rolled instead of
// pulling in a charting library (recharts/chart.js/etc.) - this app has no
// UI dependencies beyond React itself, and adding one just to show two bars
// per bucket would grow the Windows bundle/build surface for no real benefit.
// Data comes from dashboard.rs's revenue_time_series, which is already
// scoped to the dashboard's primaryCurrency (same rule as every other total
// on this page) - so this component only ever renders one concrete
// currency, never "Mixed" itself.

const CHART_HEIGHT = 160;
const CHART_WIDTH = 640; // viewBox units only - actual size follows the container via width="100%"
const PADDING_LEFT = 46;
const PADDING_TOP = 10;
const PADDING_BOTTOM = 22;
const PADDING_RIGHT = 8;

function bucketLabel(iso: string, granularity: string): string {
  const d = new Date(`${iso}T00:00:00`);
  if (Number.isNaN(d.getTime())) return iso;
  if (granularity === "month") {
    return d.toLocaleDateString(undefined, { year: "numeric", month: "short" });
  }
  return d.toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

export function RevenueChart({
  points,
  granularity,
  currency,
}: {
  points: RevenueTimeSeriesPoint[];
  granularity: string;
  currency: string;
}) {
  if (points.length === 0) {
    return (
      <div className="flex h-40 items-center justify-center text-sm text-slate-400 dark:text-slate-500">
        No sales in this period yet.
      </div>
    );
  }

  const plotWidth = CHART_WIDTH - PADDING_LEFT - PADDING_RIGHT;
  const plotHeight = CHART_HEIGHT - PADDING_TOP - PADDING_BOTTOM;

  // Revenue is never negative, but profit can be (a period sold at a net
  // loss is a real, valid state elsewhere in this app - e.g. finance.rs's
  // own tests cover a negative profit_cents - so the scale must accommodate
  // bars that go below the zero line, not just above it).
  const maxRevenue = Math.max(0, ...points.map((p) => p.revenueCents));
  const minProfit = Math.min(0, ...points.map((p) => p.profitCents));
  const maxProfit = Math.max(0, ...points.map((p) => p.profitCents));
  const yMax = Math.max(maxRevenue, maxProfit, 1); // the `1` avoids a zero-height scale when every bucket is 0
  const yMin = Math.min(0, minProfit);
  const yRange = yMax - yMin || 1;

  const yToPixel = (cents: number) => PADDING_TOP + (plotHeight * (yMax - cents)) / yRange;
  const zeroY = yToPixel(0);

  const groupWidth = plotWidth / points.length;
  const barGap = Math.min(6, groupWidth * 0.08);
  const barWidth = Math.max(1, (groupWidth - barGap * 3) / 2);

  const gridValues = [yMax, yMin + yRange / 2, yMin].filter((v, i, arr) => arr.indexOf(v) === i);

  // With many buckets, labeling every single one would overlap - thin the
  // x-axis labels down to roughly a dozen, evenly spaced, same idea as any
  // standard chart axis (the hover tooltip on every bar still has the exact
  // date, so no information is lost, just decluttered).
  const labelEvery = points.length <= 12 ? 1 : Math.ceil(points.length / 12);

  return (
    <div>
      <div className="mb-2 flex items-center gap-4 text-xs text-slate-500 dark:text-slate-400">
        <span className="inline-flex items-center gap-1.5">
          <span className="h-2.5 w-2.5 rounded-sm bg-brand-500" /> Revenue
        </span>
        <span className="inline-flex items-center gap-1.5">
          <span className="h-2.5 w-2.5 rounded-sm bg-emerald-500" /> Profit
        </span>
      </div>
      <svg viewBox={`0 0 ${CHART_WIDTH} ${CHART_HEIGHT}`} className="h-40 w-full" preserveAspectRatio="none">
        {gridValues.map((v) => (
          <g key={v}>
            <line
              x1={PADDING_LEFT}
              x2={CHART_WIDTH}
              y1={yToPixel(v)}
              y2={yToPixel(v)}
              className="stroke-slate-100 dark:stroke-slate-800"
              strokeWidth={1}
            />
            <text
              x={PADDING_LEFT - 6}
              y={yToPixel(v)}
              textAnchor="end"
              dominantBaseline="middle"
              className="fill-slate-400 text-[9px] dark:fill-slate-500"
            >
              {formatMoney(Math.round(v), currency)}
            </text>
          </g>
        ))}
        {points.map((p, i) => {
          const groupX = PADDING_LEFT + i * groupWidth;
          const revenueX = groupX + barGap;
          const profitX = revenueX + barWidth + barGap;
          const revenueY = Math.min(yToPixel(p.revenueCents), zeroY);
          const revenueH = Math.max(0.5, Math.abs(zeroY - yToPixel(p.revenueCents)));
          const profitY = Math.min(yToPixel(p.profitCents), zeroY);
          const profitH = Math.max(0.5, Math.abs(zeroY - yToPixel(p.profitCents)));
          return (
            <g key={p.bucketStart}>
              <title>
                {bucketLabel(p.bucketStart, granularity)}: revenue {formatMoney(p.revenueCents, currency)}, profit{" "}
                {formatMoney(p.profitCents, currency)}
              </title>
              <rect x={revenueX} y={revenueY} width={barWidth} height={revenueH} className="fill-brand-500" rx={1} />
              <rect
                x={profitX}
                y={profitY}
                width={barWidth}
                height={profitH}
                className={p.profitCents < 0 ? "fill-red-500" : "fill-emerald-500"}
                rx={1}
              />
              {i % labelEvery === 0 && (
                <text
                  x={groupX + groupWidth / 2}
                  y={CHART_HEIGHT - 6}
                  textAnchor="middle"
                  className="fill-slate-400 text-[9px] dark:fill-slate-500"
                >
                  {bucketLabel(p.bucketStart, granularity)}
                </text>
              )}
            </g>
          );
        })}
        <line
          x1={PADDING_LEFT}
          x2={CHART_WIDTH}
          y1={zeroY}
          y2={zeroY}
          className="stroke-slate-300 dark:stroke-slate-700"
          strokeWidth={1}
        />
      </svg>
    </div>
  );
}

import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { formatMoney } from "../lib/format";
import type { RevenueTimeSeriesPoint } from "../lib/types";

// Small, dependency-free SVG chart. Deliberately hand-rolled instead of
// pulling in a charting library (recharts/chart.js/etc.) - this app has no
// UI dependencies beyond React itself, and adding one just for a revenue
// line would grow the Windows bundle/build surface for no real benefit.
// Data comes from dashboard.rs's revenue_time_series, which is already
// scoped to the dashboard's primaryCurrency (same rule as every other total
// on this page) - so this component only ever renders one concrete
// currency, never "Mixed" itself.
//
// The SVG's viewBox width is kept in sync with the actual measured render
// width (via ResizeObserver) instead of using a fixed viewBox stretched to
// fit. That keeps the coordinate system 1:1 with real pixels on both axes,
// so the circular hover markers stay circular instead of rendering as
// ellipses, and mouse coordinates can be read directly off the container
// without any extra scale-factor math.

const CHART_HEIGHT = 208;
// Generous enough for most real revenue figures (e.g. "€2,900.00") without
// the y-axis label crowding the plot area. overflow-visible on the <svg>
// below is the safety net for anything wider still (a big month, a
// currency with a longer symbol) - it lets the label bleed a few px into
// the surrounding Card's own padding instead of ever hard-clipping it.
const PADDING_LEFT = 68;
const PADDING_TOP = 20;
const PADDING_BOTTOM = 26;
const PADDING_RIGHT = 12;
const FALLBACK_WIDTH = 640; // only used before layout/ResizeObserver report a real width

function bucketLabel(iso: string, granularity: string): string {
  const d = new Date(`${iso}T00:00:00`);
  if (Number.isNaN(d.getTime())) return iso;
  if (granularity === "month") {
    return d.toLocaleDateString(undefined, { year: "numeric", month: "short" });
  }
  return d.toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

interface Pt {
  x: number;
  y: number;
}

interface Segment {
  x0: number;
  y0: number;
  c1x: number;
  c1y: number;
  c2x: number;
  c2y: number;
  x1: number;
  y1: number;
}

// Catmull-Rom -> cubic Bezier conversion, so the line/area reads as a smooth
// curve instead of sharp straight-line joins between buckets. Purely a
// rendering technique - every real data point still lands exactly on the
// curve, nothing is interpolated or approximated away.
function smoothSegments(pts: Pt[]): Segment[] {
  const segments: Segment[] = [];
  for (let i = 0; i < pts.length - 1; i++) {
    const p0 = pts[i - 1] ?? pts[i];
    const p1 = pts[i];
    const p2 = pts[i + 1];
    const p3 = pts[i + 2] ?? p2;
    segments.push({
      x0: p1.x,
      y0: p1.y,
      c1x: p1.x + (p2.x - p0.x) / 6,
      c1y: p1.y + (p2.y - p0.y) / 6,
      c2x: p2.x - (p3.x - p1.x) / 6,
      c2y: p2.y - (p3.y - p1.y) / 6,
      x1: p2.x,
      y1: p2.y,
    });
  }
  return segments;
}

function segmentPath(s: Segment): string {
  return `M ${s.x0} ${s.y0} C ${s.c1x} ${s.c1y} ${s.c2x} ${s.c2y} ${s.x1} ${s.y1}`;
}

function segmentsPath(segments: Segment[]): string {
  if (segments.length === 0) return "";
  let d = `M ${segments[0].x0} ${segments[0].y0}`;
  for (const s of segments) d += ` C ${s.c1x} ${s.c1y} ${s.c2x} ${s.c2y} ${s.x1} ${s.y1}`;
  return d;
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
  const containerRef = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(FALLBACK_WIDTH);
  const [hovered, setHovered] = useState<number | null>(null);
  const [mounted, setMounted] = useState(false);

  // Measured synchronously before paint (useLayoutEffect) so the very first
  // frame already uses the real container width instead of the fallback -
  // ResizeObserver still keeps it in sync after that (window resizes, the
  // sidebar collapsing, etc).
  useLayoutEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    setWidth(el.getBoundingClientRect().width || FALLBACK_WIDTH);
    const ro = new ResizeObserver((entries) => {
      const w = entries[0]?.contentRect.width;
      if (w && w > 0) setWidth(w);
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  // Small entrance fade/rise on first mount - purely cosmetic.
  useEffect(() => {
    const frame = requestAnimationFrame(() => setMounted(true));
    return () => cancelAnimationFrame(frame);
  }, []);

  if (points.length === 0) {
    return (
      <div
        className="flex items-center justify-center text-sm text-slate-400 dark:text-slate-500"
        style={{ height: CHART_HEIGHT }}
      >
        No sales in this period yet.
      </div>
    );
  }

  const plotWidth = Math.max(1, width - PADDING_LEFT - PADDING_RIGHT);
  const plotHeight = CHART_HEIGHT - PADDING_TOP - PADDING_BOTTOM;

  // Revenue is never negative, but profit can be (a period sold at a net
  // loss is a real, valid state elsewhere in this app - e.g. finance.rs's
  // own tests cover a negative profit_cents - so the scale must accommodate
  // the line dipping below the zero baseline, not just above it).
  const maxRevenue = Math.max(0, ...points.map((p) => p.revenueCents));
  const minProfit = Math.min(0, ...points.map((p) => p.profitCents));
  const maxProfit = Math.max(0, ...points.map((p) => p.profitCents));
  const yMax = Math.max(maxRevenue, maxProfit, 1); // the `1` avoids a zero-height scale when every bucket is 0
  const yMin = Math.min(0, minProfit);
  const yRange = yMax - yMin || 1;

  const yToPixel = (cents: number) => PADDING_TOP + (plotHeight * (yMax - cents)) / yRange;
  const zeroY = yToPixel(0);

  const n = points.length;
  const stepX = n > 1 ? plotWidth / (n - 1) : 0;
  const xAt = (i: number) => (n > 1 ? PADDING_LEFT + i * stepX : PADDING_LEFT + plotWidth / 2);

  const revenuePts = points.map((p, i) => ({ x: xAt(i), y: yToPixel(p.revenueCents) }));
  const profitPts = points.map((p, i) => ({ x: xAt(i), y: yToPixel(p.profitCents) }));

  const revenueSegments = smoothSegments(revenuePts);
  const revenueLinePath = segmentsPath(revenueSegments);
  const revenueAreaPath =
    revenueLinePath && `${revenueLinePath} L ${revenuePts[n - 1].x} ${zeroY} L ${revenuePts[0].x} ${zeroY} Z`;
  const profitSegments = smoothSegments(profitPts);

  const gridValues = [yMax, yMin + yRange / 2, yMin].filter((v, i, arr) => arr.indexOf(v) === i);

  // With many buckets, labeling every single one would overlap - thin the
  // x-axis labels down to a handful, evenly spaced. The hover tooltip still
  // shows the exact bucket, so nothing is lost, just decluttered.
  const labelEvery = n <= 8 ? 1 : Math.ceil(n / 8);
  const hoveredPoint = hovered !== null ? points[hovered] : null;

  return (
    <div>
      {/* This header doubles as the legend (nothing hovered) and the live
          readout for the hovered bucket - a fixed-position panel instead of
          a tooltip that follows the cursor, so it never has to dodge its
          own data (e.g. the highest point on the chart) to avoid covering
          it. */}
      <div className="mb-3 flex min-h-[18px] flex-wrap items-center gap-x-5 gap-y-1 text-xs">
        {hoveredPoint ? (
          <>
            <span className="font-semibold text-slate-700 dark:text-slate-200">
              {bucketLabel(hoveredPoint.bucketStart, granularity)}
            </span>
            <span className="inline-flex items-center gap-1.5 font-medium text-slate-600 dark:text-slate-300">
              <span className="h-2 w-2 rounded-full bg-brand-500" /> Revenue {formatMoney(hoveredPoint.revenueCents, currency)}
            </span>
            <span
              className={`inline-flex items-center gap-1.5 font-medium ${
                hoveredPoint.profitCents < 0 ? "text-red-600 dark:text-red-400" : "text-emerald-600 dark:text-emerald-400"
              }`}
            >
              <span className={`h-2 w-2 rounded-full ${hoveredPoint.profitCents < 0 ? "bg-red-500" : "bg-emerald-500"}`} />
              Profit {formatMoney(hoveredPoint.profitCents, currency)}
            </span>
          </>
        ) : (
          <>
            <span className="inline-flex items-center gap-1.5 font-medium text-slate-500 dark:text-slate-400">
              <span className="h-2 w-2 rounded-full bg-brand-500" /> Revenue
            </span>
            <span className="inline-flex items-center gap-1.5 font-medium text-slate-500 dark:text-slate-400">
              <span className="h-2 w-2 rounded-full bg-emerald-500" /> Profit
            </span>
          </>
        )}
      </div>
      <div
        ref={containerRef}
        className="relative"
        onMouseMove={(e) => {
          if (n <= 1) return;
          const rect = containerRef.current?.getBoundingClientRect();
          if (!rect) return;
          const x = e.clientX - rect.left;
          const idx = Math.round((x - PADDING_LEFT) / stepX);
          setHovered(Math.min(n - 1, Math.max(0, idx)));
        }}
        onMouseLeave={() => setHovered(null)}
      >
        <svg
          width="100%"
          height={CHART_HEIGHT}
          viewBox={`0 0 ${width} ${CHART_HEIGHT}`}
          className="overflow-visible"
        >
          <defs>
            <linearGradient id="revenueAreaFill" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" className="text-brand-500" stopColor="currentColor" stopOpacity={0.25} />
              <stop offset="100%" className="text-brand-500" stopColor="currentColor" stopOpacity={0} />
            </linearGradient>
          </defs>

          {gridValues.map((v) => (
            <g key={v}>
              <line
                x1={PADDING_LEFT}
                x2={width - PADDING_RIGHT}
                y1={yToPixel(v)}
                y2={yToPixel(v)}
                className="stroke-slate-100 dark:stroke-slate-800"
                strokeWidth={1}
                strokeDasharray="3 4"
              />
              <text
                x={PADDING_LEFT - 8}
                y={yToPixel(v)}
                textAnchor="end"
                dominantBaseline="middle"
                className="fill-slate-400 text-[10px] dark:fill-slate-500"
              >
                {formatMoney(Math.round(v), currency)}
              </text>
            </g>
          ))}
          <line
            x1={PADDING_LEFT}
            x2={width - PADDING_RIGHT}
            y1={zeroY}
            y2={zeroY}
            className="stroke-slate-300 dark:stroke-slate-700"
            strokeWidth={1}
          />

          <g
            style={{
              opacity: mounted ? 1 : 0,
              transform: mounted ? "translateY(0)" : "translateY(6px)",
              transition: "opacity 600ms ease-out, transform 600ms ease-out",
            }}
          >
            {revenueAreaPath && <path d={revenueAreaPath} fill="url(#revenueAreaFill)" />}
            {revenueLinePath && (
              <path
                d={revenueLinePath}
                fill="none"
                className="stroke-brand-500"
                strokeWidth={2.25}
                strokeLinecap="round"
                strokeLinejoin="round"
              />
            )}
            {profitSegments.map((s, i) => (
              <path
                key={i}
                d={segmentPath(s)}
                fill="none"
                className={
                  points[i].profitCents < 0 || points[i + 1].profitCents < 0 ? "stroke-red-500" : "stroke-emerald-500"
                }
                strokeWidth={2.25}
                strokeLinecap="round"
                strokeLinejoin="round"
              />
            ))}
            {n === 1 && (
              <>
                <circle cx={revenuePts[0].x} cy={revenuePts[0].y} r={4} className="fill-brand-500">
                  <title>Revenue {formatMoney(points[0].revenueCents, currency)}</title>
                </circle>
                <circle
                  cx={profitPts[0].x}
                  cy={profitPts[0].y}
                  r={4}
                  className={points[0].profitCents < 0 ? "fill-red-500" : "fill-emerald-500"}
                >
                  <title>Profit {formatMoney(points[0].profitCents, currency)}</title>
                </circle>
              </>
            )}
          </g>

          {points.map(
            (p, i) =>
              i % labelEvery === 0 && (
                <text
                  key={p.bucketStart}
                  x={xAt(i)}
                  y={CHART_HEIGHT - 8}
                  textAnchor="middle"
                  className="fill-slate-400 text-[10px] dark:fill-slate-500"
                >
                  {bucketLabel(p.bucketStart, granularity)}
                </text>
              ),
          )}

          {hovered !== null && (
            <g pointerEvents="none">
              <line
                x1={xAt(hovered)}
                x2={xAt(hovered)}
                y1={PADDING_TOP}
                y2={CHART_HEIGHT - PADDING_BOTTOM}
                className="stroke-slate-300 dark:stroke-slate-600"
                strokeWidth={1}
                strokeDasharray="3 3"
              />
              <circle
                cx={xAt(hovered)}
                cy={revenuePts[hovered].y}
                r={4}
                strokeWidth={2}
                className="fill-white stroke-brand-500 dark:fill-slate-900"
              />
              <circle
                cx={xAt(hovered)}
                cy={profitPts[hovered].y}
                r={4}
                strokeWidth={2}
                className={`fill-white dark:fill-slate-900 ${
                  points[hovered].profitCents < 0 ? "stroke-red-500" : "stroke-emerald-500"
                }`}
              />
            </g>
          )}
        </svg>
      </div>
    </div>
  );
}

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
// 1.7.4: simplified to a single Revenue line (was Revenue area + a second
// Profit line that changed color at zero) - marko found the two-series
// version busy and asked for something that just shows the line going up or
// down. Profit is still a first-class number elsewhere on the Dashboard
// (its own StatCard, unaffected by this file); it's just not broken out
// bucket-by-bucket here anymore. `RevenueTimeSeriesPoint` still carries
// `profitCents`/`cogsCents` from the backend - deliberately left alone
// rather than trimming the type/query, since that's already-correct,
// already-tested code this change has no real reason to touch - this
// component simply no longer reads those two fields.
//
// The SVG's viewBox width is kept in sync with the actual measured render
// width (via ResizeObserver) instead of using a fixed viewBox stretched to
// fit. That keeps the coordinate system 1:1 with real pixels on both axes,
// so the circular hover marker stays circular instead of rendering as an
// ellipse, and mouse coordinates can be read directly off the container
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

// Catmull-Rom -> cubic Bezier conversion, so the line reads as a smooth
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

  // Revenue is never negative, so the scale always starts at 0 - no need to
  // accommodate a line dipping below a zero baseline the way profit could.
  const yMax = Math.max(1, ...points.map((p) => p.revenueCents)); // the `1` avoids a zero-height scale when every bucket is 0
  const yToPixel = (cents: number) => PADDING_TOP + (plotHeight * (yMax - cents)) / yMax;

  const n = points.length;
  const stepX = n > 1 ? plotWidth / (n - 1) : 0;
  const xAt = (i: number) => (n > 1 ? PADDING_LEFT + i * stepX : PADDING_LEFT + plotWidth / 2);

  const revenuePts = points.map((p, i) => ({ x: xAt(i), y: yToPixel(p.revenueCents) }));
  const revenueLinePath = segmentsPath(smoothSegments(revenuePts));

  const gridValues = [yMax, yMax / 2, 0].filter((v, i, arr) => arr.indexOf(v) === i);

  // With many buckets, labeling every single one would overlap - thin the
  // x-axis labels down to a handful, evenly spaced. The hover tooltip still
  // shows the exact bucket, so nothing is lost, just decluttered.
  const labelEvery = n <= 8 ? 1 : Math.ceil(n / 8);
  const hoveredPoint = hovered !== null ? points[hovered] : null;

  return (
    <div>
      {/* A single series needs no legend box - there's only one color, and
          the card title above this chart ("Revenue over time") already
          says what's plotted. This header is just the hover readout, empty
          until something is hovered. */}
      <div className="mb-3 flex min-h-[18px] flex-wrap items-center gap-x-5 gap-y-1 text-xs">
        {hoveredPoint && (
          <>
            <span className="font-semibold text-slate-700 dark:text-slate-200">
              {bucketLabel(hoveredPoint.bucketStart, granularity)}
            </span>
            <span className="inline-flex items-center gap-1.5 font-medium text-slate-600 dark:text-slate-300">
              <span className="h-2 w-2 rounded-full bg-brand-500" /> Revenue {formatMoney(hoveredPoint.revenueCents, currency)}
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
          {gridValues.map((v) => (
            <g key={v}>
              <line
                x1={PADDING_LEFT}
                x2={width - PADDING_RIGHT}
                y1={yToPixel(v)}
                y2={yToPixel(v)}
                className="stroke-slate-100 dark:stroke-slate-800"
                strokeWidth={1}
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

          <g
            style={{
              opacity: mounted ? 1 : 0,
              transform: mounted ? "translateY(0)" : "translateY(6px)",
              transition: "opacity 600ms ease-out, transform 600ms ease-out",
            }}
          >
            {revenueLinePath && (
              <path
                d={revenueLinePath}
                fill="none"
                className="stroke-brand-500"
                strokeWidth={2}
                strokeLinecap="round"
                strokeLinejoin="round"
              />
            )}
            {n === 1 && (
              <circle cx={revenuePts[0].x} cy={revenuePts[0].y} r={4} className="fill-brand-500">
                <title>Revenue {formatMoney(points[0].revenueCents, currency)}</title>
              </circle>
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
            </g>
          )}
        </svg>
      </div>
    </div>
  );
}

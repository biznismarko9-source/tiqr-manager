import { useState } from "react";

/**
 * 2.74.0 - the two Overview charts from marko's redesign reference.
 *
 * Hand-written SVG, like `MetricChart` beside it, because this project has no
 * charting library and adding one for two charts is not a trade worth making
 * (its own module comment makes the same argument). Both take numbers the
 * caller has already derived from real data and neither computes a business
 * figure of its own - that rule is what keeps a chart from quietly
 * disagreeing with the KPI card above it.
 */

// ---------------------------------------------------------------------------
// Shared path maths
// ---------------------------------------------------------------------------

/** A smooth curve through every point that CANNOT overshoot between them.
 *
 *  Fritsch-Carlson monotone cubic interpolation, not Catmull-Rom. The first
 *  draft of this used Catmull-Rom and it was wrong in a way only looking at
 *  it revealed: on marko's own four months (Apr 700, Aug 2 494, Sep 3 771,
 *  Oct 509) the crest of the income line landed BETWEEN August and September
 *  rather than on September, because the steep fall into October drags the
 *  preceding control point. A reader would have taken the highest month off
 *  that chart and got the wrong answer.
 *
 *  Monotone interpolation fixes it by construction: where the data rises the
 *  curve rises, where it falls it falls, and it never leaves the interval
 *  between two neighbouring values. The cost is that the curve is slightly
 *  less round through a peak - which is the correct trade for money.
 *
 *  The three-step shape (secants, then tangents, then the limiter) is the
 *  published algorithm; the limiter is the part that does the actual work. */
function smoothPath(pts: { x: number; y: number }[]): string {
  const n = pts.length;
  if (n === 0) return "";
  if (n === 1) return `M${pts[0].x} ${pts[0].y}`;
  if (n === 2) return `M${pts[0].x.toFixed(2)} ${pts[0].y.toFixed(2)} L${pts[1].x.toFixed(2)} ${pts[1].y.toFixed(2)}`;

  // Secant slope of each segment.
  const dx: number[] = [];
  const slope: number[] = [];
  for (let i = 0; i < n - 1; i++) {
    const h = pts[i + 1].x - pts[i].x;
    dx.push(h);
    slope.push(h === 0 ? 0 : (pts[i + 1].y - pts[i].y) / h);
  }

  // Tangent at each point: the average of its two secants, and the secant
  // itself at the ends.
  const m: number[] = new Array(n);
  m[0] = slope[0];
  m[n - 1] = slope[n - 2];
  for (let i = 1; i < n - 1; i++) {
    // A local extremum (the secants disagree in sign) gets a FLAT tangent.
    // This single line is what stops a peak being drawn past its own value.
    m[i] = slope[i - 1] * slope[i] <= 0 ? 0 : (slope[i - 1] + slope[i]) / 2;
  }

  // Fritsch-Carlson limiter: keep every tangent inside the circle of radius 3
  // around the neighbouring secants, which is the condition for monotonicity.
  for (let i = 0; i < n - 1; i++) {
    if (slope[i] === 0) {
      m[i] = 0;
      m[i + 1] = 0;
      continue;
    }
    const a = m[i] / slope[i];
    const b = m[i + 1] / slope[i];
    const sq = a * a + b * b;
    if (sq > 9) {
      const t = 3 / Math.sqrt(sq);
      m[i] = t * a * slope[i];
      m[i + 1] = t * b * slope[i];
    }
  }

  let d = `M${pts[0].x.toFixed(2)} ${pts[0].y.toFixed(2)}`;
  for (let i = 0; i < n - 1; i++) {
    const h = dx[i];
    const c1x = pts[i].x + h / 3;
    const c1y = pts[i].y + (m[i] * h) / 3;
    const c2x = pts[i + 1].x - h / 3;
    const c2y = pts[i + 1].y - (m[i + 1] * h) / 3;
    d += ` C${c1x.toFixed(2)} ${c1y.toFixed(2)}, ${c2x.toFixed(2)} ${c2y.toFixed(2)}, ${pts[i + 1].x.toFixed(2)} ${pts[i + 1].y.toFixed(2)}`;
  }
  return d;
}

// ---------------------------------------------------------------------------
// Balance Cashflow
// ---------------------------------------------------------------------------

export type CashflowPoint = {
  /** Already formatted by the caller - this component does no date work, so
   *  it cannot disagree with how the rest of the app writes a date. */
  label: string;
  /** Minor units (cents), same as everywhere else in this app. */
  income: number;
  expenses: number;
};

/**
 * Income against expenses over the selected period.
 *
 * Both series are REAL: the Dashboard derives income from each bucket's
 * `revenueCents` and expenses from `cogsCents + sellingFeesCents`, which is
 * the same data the Revenue card above it renders. Nothing here is modelled,
 * smoothed into a trend, or extended past the last real bucket.
 */
export function CashflowChart({
  points,
  formatValue,
}: {
  points: CashflowPoint[];
  /** Formats cents for the axis and the hovered-bucket line. The CALLER owns
   *  currency, so this chart cannot format money a different way from the KPI
   *  card sitting above it. */
  formatValue: (cents: number) => string;
}) {
  // Which bucket the floating label describes. Defaults to the LAST one - the
  // most recent real number is the one worth showing before anyone hovers,
  // and it means the chart is never in a state with no value on it.
  const [active, setActive] = useState<number | null>(null);

  const W = 760;
  const H = 230;
  const PAD = { top: 18, right: 14, bottom: 26, left: 48 };
  // 2.113.0: the axis says "12.3k", not "€12,345.67" - a full amount does
  // not fit the 48px gutter and ran out of the card's left edge (Finance).
  // The hover line below still shows the exact figure.
  const axisLabel = (cents: number) => {
    const v = Math.abs(cents) / 100;
    const sign = cents < 0 ? "-" : "";
    if (v >= 1_000_000) return `${sign}${(v / 1_000_000).toFixed(v >= 10_000_000 ? 0 : 1)}M`;
    if (v >= 1_000) return `${sign}${(v / 1_000).toFixed(v >= 10_000 ? 0 : 1)}k`;
    return `${sign}${Math.round(v)}`;
  };
  const plotW = W - PAD.left - PAD.right;
  const plotH = H - PAD.top - PAD.bottom;

  if (points.length < 2) {
    // Not an invented flat line: two points are the minimum a line can
    // honestly be drawn from, and saying so beats drawing nothing.
    return (
      <p className="py-10 text-center text-xs text-slate-500 dark:text-slate-400">
        Not enough activity in this period to draw a line yet.
      </p>
    );
  }

  const hi = Math.max(...points.flatMap((p) => [p.income, p.expenses]), 0);
  // A zero-height scale would divide by zero; a period with no money in it
  // draws both lines along the baseline instead.
  const scaleY = (v: number) => (hi === 0 ? PAD.top + plotH : PAD.top + plotH - (v / hi) * plotH);
  const scaleX = (i: number) => PAD.left + (i / (points.length - 1)) * plotW;

  const incomePts = points.map((p, i) => ({ x: scaleX(i), y: scaleY(p.income) }));
  const expensePts = points.map((p, i) => ({ x: scaleX(i), y: scaleY(p.expenses) }));
  const incomeLine = smoothPath(incomePts);
  const expenseLine = smoothPath(expensePts);
  const base = PAD.top + plotH;
  const areaOf = (line: string, pts: { x: number }[]) =>
    `${line} L${pts[pts.length - 1].x.toFixed(2)} ${base} L${pts[0].x.toFixed(2)} ${base} Z`;

  const marked = active ?? points.length - 1;
  const mp = points[marked];
  const markX = scaleX(marked);
  const markY = scaleY(mp.income);

  // Four horizontal guides, labelled with the values they actually sit on.
  const guides = [0, 0.5, 1].map((f) => ({ f, v: hi * f, y: PAD.top + plotH - f * plotH }));

  // Labels thin out rather than overlap: a year of daily buckets cannot print
  // 365 of them, and a rotated, unreadable axis is worse than fewer labels.
  const labelStep = Math.max(1, Math.ceil(points.length / 12));

  return (
    <div>
      {/* The card above owns the heading and the headline figure; this is only
          the legend, which belongs to the lines rather than to the card. */}
      <div className="mb-1 flex items-center justify-end gap-3">
        <span className="flex items-center gap-1.5 text-[11px] text-slate-500 dark:text-slate-400">
          <span className="h-2 w-2 rounded-[3px] bg-blue-500" aria-hidden="true" /> Income
        </span>
        <span className="flex items-center gap-1.5 text-[11px] text-slate-500 dark:text-slate-400">
          <span className="h-2 w-2 rounded-[3px] bg-orange-500" aria-hidden="true" /> Expenses
        </span>
      </div>

      <svg
        viewBox={`0 0 ${W} ${H}`}
        className="w-full overflow-visible"
        role="img"
        aria-label={`Income and expenses over ${points.length} periods`}
        onMouseLeave={() => setActive(null)}
      >
        <defs>
          <linearGradient id="tiqr-cash-income" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" className="text-blue-500" stopColor="currentColor" stopOpacity="0.22" />
            <stop offset="100%" className="text-blue-500" stopColor="currentColor" stopOpacity="0" />
          </linearGradient>
          <linearGradient id="tiqr-cash-expense" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" className="text-orange-500" stopColor="currentColor" stopOpacity="0.18" />
            <stop offset="100%" className="text-orange-500" stopColor="currentColor" stopOpacity="0" />
          </linearGradient>
        </defs>

        {guides.map((g) => (
          <g key={g.f}>
            <line
              x1={PAD.left}
              x2={W - PAD.right}
              y1={g.y}
              y2={g.y}
              className="stroke-slate-200 dark:stroke-slate-800"
              strokeWidth={1}
            />
            <text
              x={PAD.left - 8}
              y={g.y + 3}
              textAnchor="end"
              className="fill-slate-400 text-[11px] tabular-nums dark:fill-slate-500"
            >
              {axisLabel(g.v)}
            </text>
          </g>
        ))}

        <path d={areaOf(expenseLine, expensePts)} fill="url(#tiqr-cash-expense)" stroke="none" />
        <path d={areaOf(incomeLine, incomePts)} fill="url(#tiqr-cash-income)" stroke="none" />
        <path d={expenseLine} fill="none" className="stroke-orange-500" strokeWidth={2} strokeLinecap="round" />
        <path d={incomeLine} fill="none" className="stroke-blue-500" strokeWidth={2} strokeLinecap="round" />

        {points.map((p, i) => (
          <g key={`${p.label}-${i}`}>
            {i % labelStep === 0 && (
              <text
                x={scaleX(i)}
                y={H - 6}
                textAnchor="middle"
                className="fill-slate-400 text-[11px] dark:fill-slate-500"
              >
                {p.label}
              </text>
            )}
            {/* One invisible full-height band per bucket - a hit target that
                does not require landing on a 2px line. */}
            <rect
              x={scaleX(i) - plotW / points.length / 2}
              y={PAD.top}
              width={plotW / points.length}
              height={plotH}
              fill="transparent"
              onMouseEnter={() => setActive(i)}
            />
          </g>
        ))}

        <line
          x1={markX}
          x2={markX}
          y1={PAD.top}
          y2={base}
          className="stroke-slate-300 dark:stroke-slate-600"
          strokeWidth={1}
          strokeDasharray="3 3"
        />
        <circle cx={markX} cy={markY} r={4} strokeWidth={2} className="fill-white stroke-blue-500 dark:fill-slate-900" />
      </svg>

      {/* The floating value label, as HTML rather than SVG text: it carries
          the app's own type and surface tokens, so it cannot drift from every
          other label in the product. */}
      <div className="mt-1 flex flex-wrap items-center gap-x-4 gap-y-1 text-[11px]">
        <span className="font-medium text-slate-700 dark:text-slate-200">{mp.label}</span>
        <span className="tabular-nums text-blue-500">Income {formatValue(mp.income)}</span>
        <span className="tabular-nums text-orange-500">Expenses {formatValue(mp.expenses)}</span>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Sales by Platform
// ---------------------------------------------------------------------------

export type PlatformSlice = { name: string; value: number };

/** 2.75.0: five accents, then everything else is grouped.
 *
 *  A sixth generated hue would be a colour nobody chose. `MAX_SLICES` is
 *  exported because the LIST that acts as this ring's legend has to group at
 *  exactly the same index, or row six would wear a colour the ring never
 *  drew. */
export const MAX_PLATFORM_SLICES = 5;

/** Ring strokes and the matching dot/bar fills, in one place and the same
 *  order, so a legend row and its arc can never drift apart. Index 5 is the
 *  grey every grouped platform shares. */
export const PLATFORM_RING_STROKE = [
  "stroke-blue-500",
  "stroke-orange-500",
  "stroke-brand-500",
  "stroke-teal-500",
  "stroke-emerald-500",
  "stroke-slate-400",
];
export const PLATFORM_SLICE_FILL = [
  "bg-blue-500",
  "bg-orange-500",
  "bg-brand-500",
  "bg-teal-500",
  "bg-emerald-500",
  "bg-slate-400",
];

/** The colour a row at `index` wears, in the ring and in the legend alike. */
export function platformFill(index: number): string {
  return PLATFORM_SLICE_FILL[Math.min(index, PLATFORM_SLICE_FILL.length - 1)];
}

/**
 * 2.75.0 - the ring ONLY.
 *
 * The first draft of this was a whole card with its own heading and legend.
 * That was the wrong shape: `SalesByPlatformCard` already exists, already has
 * a heading, and already lists every platform with its revenue, its share and
 * a show-more toggle. Dropping a second widget beside it would have meant two
 * platform breakdowns on one screen, or deleting a working one to make room -
 * and marko's first rule for this redesign is that nothing working gets
 * removed to look like a screenshot.
 *
 * So this is the ring, and that card is its legend.
 *
 * Caller passes slices ALREADY ordered the way its own legend lists them. The
 * backend orders `sales_by_platform` by revenue descending (there is a test
 * named for it), so index 0 is the biggest platform in both places.
 */
export function PlatformRing({
  slices,
  centerValue,
  centerLabel,
}: {
  slices: PlatformSlice[];
  centerValue: string;
  centerLabel: string;
}) {
  const positive = slices.filter((s) => s.value > 0);
  const total = positive.reduce((sum, s) => sum + s.value, 0);
  if (total === 0) return null;

  const head = positive.slice(0, MAX_PLATFORM_SLICES);
  const tailValue = positive.slice(MAX_PLATFORM_SLICES).reduce((sum, s) => sum + s.value, 0);
  const shown = tailValue > 0 ? [...head, { name: "Other", value: tailValue }] : head;

  // One circle per slice, drawn with stroke-dasharray around a shared radius.
  // No arc maths, and no hairline gap where two arcs are meant to meet.
  const R = 54;
  const C = 2 * Math.PI * R;
  let offset = 0;

  return (
    <svg viewBox="0 0 140 140" className="h-[132px] w-[132px]" role="img" aria-label="Revenue share by platform">
      {/* -90deg so the first slice starts at twelve o'clock rather than at
          three, which is where a bare circle's dash offset begins. */}
      <g transform="rotate(-90 70 70)">
        {shown.map((s, i) => {
          const len = (s.value / total) * C;
          const arc = (
            <circle
              key={s.name}
              cx={70}
              cy={70}
              r={R}
              fill="none"
              strokeWidth={13}
              strokeDasharray={`${len} ${C - len}`}
              strokeDashoffset={-offset}
              className={PLATFORM_RING_STROKE[Math.min(i, PLATFORM_RING_STROKE.length - 1)]}
            />
          );
          offset += len;
          return arc;
        })}
      </g>
      <text
        x={70}
        y={67}
        textAnchor="middle"
        className="fill-slate-900 text-[15px] font-semibold tabular-nums dark:fill-slate-50"
      >
        {centerValue}
      </text>
      <text x={70} y={81} textAnchor="middle" className="fill-slate-400 text-[8px] dark:fill-slate-500">
        {centerLabel}
      </text>
    </svg>
  );
}

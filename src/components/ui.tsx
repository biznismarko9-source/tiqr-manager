import { useEffect, useState, type ButtonHTMLAttributes, type HTMLAttributes, type InputHTMLAttributes, type ReactNode, type SelectHTMLAttributes, type TextareaHTMLAttributes } from "react";
import { IconAlertTriangle, IconChevronDown, IconTrendingDown, IconTrendingUp, IconX } from "./icons";
import type { TrendInfo } from "../lib/format";

// ---------------------------------------------------------------------------
// Buttons
// ---------------------------------------------------------------------------
type ButtonVariant = "primary" | "secondary" | "danger" | "ghost";
type ButtonSize = "sm" | "md";

export function Button({
  variant = "secondary",
  size = "md",
  className = "",
  children,
  ...rest
}: ButtonHTMLAttributes<HTMLButtonElement> & { variant?: ButtonVariant; size?: ButtonSize }) {
  // 2.0.74: `transition-colors` -> `transition` (Tailwind's curated
  // property list - color/background/border/opacity/box-shadow/transform/
  // filter, not literally every property) plus `active:scale-[0.97]` - a
  // small, brief press-down on click/tap, the same tactile feedback most
  // native buttons already give you for free, that "hover:bg-..." alone
  // doesn't. Purely cosmetic - disabled:pointer-events-none above already
  // stops :active from ever triggering on a disabled button.
  //
  // 2.6.0 (visual redesign): the press is softer (0.98 rather than 0.97, at
  // the shared 150ms budget), focus moved to `focus-visible` so a mouse
  // click no longer leaves a ring behind, and `size` was added with "md" as
  // the default - every pre-2.6.0 call site keeps exactly the size it had.
  const base =
    "inline-flex shrink-0 items-center justify-center gap-1.5 rounded-lg font-medium transition active:scale-[0.98] disabled:opacity-45 disabled:pointer-events-none focus:outline-none focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-slate-950";
  const sizes: Record<ButtonSize, string> = {
    sm: "px-2.5 py-1.5 text-xs",
    md: "px-3.5 py-2 text-sm",
  };
  const variants: Record<ButtonVariant, string> = {
    primary:
      "bg-brand-600 text-white shadow-card hover:bg-brand-700 active:bg-brand-700 focus-visible:ring-brand-500",
    secondary:
      "border border-slate-300 bg-white text-slate-700 shadow-card hover:border-slate-400 hover:bg-slate-50 focus-visible:ring-slate-400 dark:border-slate-700 dark:bg-slate-900 dark:text-slate-200 dark:hover:border-slate-600 dark:hover:bg-slate-800 dark:focus-visible:ring-slate-500",
    danger: "bg-red-600 text-white shadow-card hover:bg-red-700 focus-visible:ring-red-500",
    ghost:
      "text-slate-600 hover:bg-slate-100 hover:text-slate-900 focus-visible:ring-slate-400 dark:text-slate-400 dark:hover:bg-slate-800 dark:hover:text-slate-100",
  };
  return (
    <button className={`${base} ${sizes[size]} ${variants[variant]} ${className}`} {...rest}>
      {children}
    </button>
  );
}

// ---------------------------------------------------------------------------
// Form inputs
// ---------------------------------------------------------------------------
export function Input(props: InputHTMLAttributes<HTMLInputElement>) {
  const { className = "", ...rest } = props;
  return <input className={`input ${className}`} {...rest} />;
}

export function Select(props: SelectHTMLAttributes<HTMLSelectElement>) {
  const { className = "", children, ...rest } = props;
  return (
    <div className="relative w-full">
      {/* appearance-none: some WebKit builds render a <select>'s closed box
          with native (light) chrome regardless of background-color once a
          non-first option is chosen, which breaks dark mode. Drawing our
          own chevron keeps a dropdown affordance either way. */}
      <select className={`input appearance-none pr-9 ${className}`} {...rest}>
        {children}
      </select>
      <IconChevronDown className="pointer-events-none absolute right-3 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-400 transition-colors dark:text-slate-500" />
    </div>
  );
}

export function Textarea(props: TextareaHTMLAttributes<HTMLTextAreaElement>) {
  const { className = "", ...rest } = props;
  return <textarea className={`input ${className}`} {...rest} />;
}

// checkbox styling shared by every bulk-selection UI (Sales, Sale Detail,
// Order Detail - header "select all" and per-row boxes alike). index.css has
// no `.checkbox` component class (only .input/.th/.td/.label/.card), so this
// is spelled out directly rather than assuming one exists. 1.8.3: hoisted
// here from Sales.tsx (its original, 1.8.0 home) so it has one definition
// instead of being copy-pasted into every page that grew a selection UI.
// 2.6.0: same size and semantics, restyled to match the new focus treatment
// (focus-visible ring, brand fill) - `radio:` callers are unaffected because
// there are none; every use of this constant is a real checkbox.
export const CHECKBOX_CLASS =
  "h-4 w-4 shrink-0 cursor-pointer rounded border-slate-300 text-brand-600 transition focus:outline-none focus-visible:ring-2 focus-visible:ring-brand-500 focus-visible:ring-offset-1 dark:border-slate-600 dark:bg-slate-800 dark:focus-visible:ring-offset-slate-900";

/** 2.0.28: the "Delete" bulk-selection toolbar shared by the Pulls (both
 * tabs)/Orders/Events/Sales list pages - marko's own request. Unlike the
 * always-visible checkbox column above (Sale Detail/Order Detail's older
 * per-ticket bulk-action pattern), these 4 lists stay completely clean by
 * default: a single "Delete" toggle button on the page itself puts it into
 * selection mode, which is the only time a checkbox column and this bar
 * exist at all. Confirming (or cancelling) always leaves selection mode
 * again, so the checkboxes disappear until "Delete" is clicked once more.
 * Deliberately dumb/presentational, visually modeled on `SalePaymentStatusBar`
 * (SaleDetail.tsx) - each page owns its own `selectionMode`/`selected` state
 * and just passes the current count in here. */
export function BulkDeleteBar({
  count,
  itemLabel,
  onConfirm,
  onCancel,
  busy = false,
}: {
  count: number;
  /** Singular noun for one item, e.g. "order" - pluralized here as needed. */
  itemLabel: string;
  onConfirm: () => void;
  onCancel: () => void;
  busy?: boolean;
}) {
  return (
    <div className="mb-4 flex items-center gap-3 rounded-xl bg-red-50 px-4 py-2.5 text-sm ring-1 ring-inset ring-red-200 dark:bg-red-500/10 dark:ring-red-500/25">
      <span className="font-medium text-red-800 dark:text-red-300">
        {count === 0 ? `Select ${itemLabel}s to delete` : `Selected: ${count} ${itemLabel}${count === 1 ? "" : "s"}`}
      </span>
      <Button variant="danger" size="sm" onClick={onConfirm} disabled={busy || count === 0}>
        Delete selected
      </Button>
      <button
        type="button"
        className="ml-auto rounded text-xs font-medium text-red-700 hover:underline disabled:opacity-50 dark:text-red-400"
        onClick={onCancel}
        disabled={busy}
      >
        Cancel
      </button>
    </div>
  );
}

export function Field({
  label,
  required,
  error,
  children,
  hint,
}: {
  label: string;
  required?: boolean;
  error?: string | null;
  hint?: string;
  children: ReactNode;
}) {
  // 2.6.0: an errored field now also turns its own control red, not just the
  // message under it. `.field-invalid` is a wrapper class (index.css) that
  // restyles whatever `.input` sits inside it, so all 127 existing <Field>
  // call sites get the error state without any of them passing a new prop or
  // knowing which control they wrapped.
  return (
    <label className="block">
      <span className="label">
        {label}
        {required && <span className="text-red-500"> *</span>}
      </span>
      <div className={error ? "field-invalid" : ""}>{children}</div>
      {hint && !error && <span className="mt-1.5 block text-xs text-slate-400 dark:text-slate-500">{hint}</span>}
      {error && <span className="mt-1.5 block text-xs font-medium text-red-600 dark:text-red-400">{error}</span>}
    </label>
  );
}

// ---------------------------------------------------------------------------
// Layout bits
// ---------------------------------------------------------------------------
export function Card({
  children,
  className = "",
  interactive = false,
  ...rest
}: { children: ReactNode; className?: string; interactive?: boolean } & Omit<HTMLAttributes<HTMLDivElement>, "className" | "children">) {
  // `interactive` opts a card into the shared hover lift - only for cards
  // that actually do something when clicked. A static container that lifts
  // under the cursor reads as a bug, so this is never on by default.
  return (
    <div className={`card ${interactive ? "card-interactive" : ""} ${className}`} {...rest}>
      {children}
    </div>
  );
}

export function PageHeader({
  title,
  subtitle,
  actions,
}: {
  title: string;
  subtitle?: string;
  actions?: ReactNode;
}) {
  // 2.6.0: one page-header shape for the whole app - title, optional
  // subtitle/meta line, right-aligned actions, and a hairline rule that
  // separates the header band from the page content. Same props as before,
  // so every page that already renders one picks this up unchanged.
  return (
    <div className="mb-5 border-b border-slate-200 pb-4 dark:border-slate-800">
      <div className="flex flex-wrap items-center justify-between gap-x-4 gap-y-2">
        <div className="min-w-0">
          <h1 className="truncate text-[19px] font-semibold leading-tight text-slate-900 dark:text-slate-50">
            {title}
          </h1>
          {subtitle && <p className="mt-1 text-sm text-slate-500 dark:text-slate-400">{subtitle}</p>}
        </div>
        {actions && <div className="flex flex-wrap items-center gap-2">{actions}</div>}
      </div>
    </div>
  );
}

export function Spinner({ className = "" }: { className?: string }) {
  return (
    <svg className={`animate-spin ${className}`} viewBox="0 0 24 24" fill="none">
      <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
      <path
        className="opacity-75"
        fill="currentColor"
        d="M4 12a8 8 0 0 1 8-8V0C5.373 0 0 5.373 0 12h4Z"
      />
    </svg>
  );
}

/** 2.6.0: a single grey placeholder block. Sized entirely by the caller
 * (`className`) - see `.skeleton` in index.css for the sweep, which stops
 * on its own under `prefers-reduced-motion`. */
export function Skeleton({ className = "" }: { className?: string }) {
  return <div className={`skeleton ${className}`} />;
}

/** 2.6.0: the loading state for a list table. Draws the shell, a header
 * band and `rows` placeholder rows so the page keeps its real height and
 * layout while data loads, instead of collapsing to a centred spinner and
 * then jumping. Deliberately structural-only - it never guesses at column
 * widths, because every table in this app sets its own `colgroup`. */
export function TableSkeleton({ rows = 8, className = "" }: { rows?: number; className?: string }) {
  return (
    <div className={`overflow-hidden rounded-xl border border-slate-200 bg-white shadow-card dark:border-slate-800 dark:bg-slate-900 ${className}`}>
      <div className="flex items-center gap-4 border-b border-slate-200 bg-slate-50 px-3 py-3 dark:border-slate-800 dark:bg-slate-800/40">
        <Skeleton className="h-2.5 w-24" />
        <Skeleton className="h-2.5 w-16" />
        <Skeleton className="ml-auto h-2.5 w-20" />
      </div>
      <div>
        {Array.from({ length: rows }).map((_, i) => (
          <div
            key={i}
            className="flex items-center gap-4 px-3 py-3.5"
            style={{ opacity: Math.max(0.25, 1 - i * 0.1) }}
          >
            <Skeleton className="h-3 w-32" />
            <Skeleton className="h-3 w-20" />
            <Skeleton className="h-3 w-16" />
            <Skeleton className="ml-auto h-3 w-24" />
          </div>
        ))}
      </div>
    </div>
  );
}

export function LoadingBlock({ label = "Loading..." }: { label?: string }) {
  return (
    <div className="flex items-center justify-center gap-2.5 py-16 text-sm text-slate-400 animate-[fadein_.15s_ease-out] dark:text-slate-500">
      <Spinner className="h-4 w-4" />
      {label}
    </div>
  );
}

export function EmptyState({
  icon,
  title,
  description,
  action,
}: {
  icon?: ReactNode;
  title: string;
  description?: string;
  action?: ReactNode;
}) {
  // 2.6.0: the icon now sits in a soft, bordered medallion instead of
  // floating as a large grey glyph, and the whole block sits on a real
  // (dashed) surface. Same props, same call sites.
  return (
    <div className="flex flex-col items-center justify-center gap-1 rounded-xl border border-dashed border-slate-300 bg-slate-50/70 px-6 py-14 text-center dark:border-slate-700 dark:bg-slate-900/50">
      {icon && (
        <div className="mb-3 flex h-11 w-11 items-center justify-center rounded-full border border-slate-200 bg-white text-slate-400 shadow-card dark:border-slate-700 dark:bg-slate-800 dark:text-slate-500">
          {icon}
        </div>
      )}
      <p className="text-sm font-semibold text-slate-800 dark:text-slate-200">{title}</p>
      {description && (
        <p className="max-w-sm text-sm leading-relaxed text-slate-500 dark:text-slate-400">{description}</p>
      )}
      {action && <div className="mt-4">{action}</div>}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Badge
// ---------------------------------------------------------------------------
// 2.6.0: every tone below follows ONE recipe - a tinted surface, a matching
// text color, and a hairline inset ring, in both modes. The keys, and which
// key each business value maps to, are completely unchanged from 2.5.2: this
// is a restyle of the same status vocabulary, not a change to what any status
// means or to which statuses exist.
const STATUS_TONES: Record<string, string> = {
  available: "bg-slate-100 text-slate-700 ring-slate-200 dark:bg-slate-100/5 dark:text-slate-300 dark:ring-slate-100/10",
  listed: "bg-blue-50 text-blue-700 ring-blue-200/70 dark:bg-blue-500/10 dark:text-blue-300 dark:ring-blue-400/20",
  sold: "bg-emerald-50 text-emerald-700 ring-emerald-200/70 dark:bg-emerald-500/10 dark:text-emerald-300 dark:ring-emerald-400/20",
  cancelled: "bg-red-50 text-red-700 ring-red-200/70 dark:bg-red-500/10 dark:text-red-300 dark:ring-red-400/20",
  upcoming: "bg-blue-50 text-blue-700 ring-blue-200/70 dark:bg-blue-500/10 dark:text-blue-300 dark:ring-blue-400/20",
  completed: "bg-emerald-50 text-emerald-700 ring-emerald-200/70 dark:bg-emerald-500/10 dark:text-emerald-300 dark:ring-emerald-400/20",
  unpaid: "bg-amber-50 text-amber-800 ring-amber-200/70 dark:bg-amber-500/10 dark:text-amber-300 dark:ring-amber-400/20",
  partial: "bg-blue-50 text-blue-700 ring-blue-200/70 dark:bg-blue-500/10 dark:text-blue-300 dark:ring-blue-400/20",
  paid: "bg-emerald-50 text-emerald-700 ring-emerald-200/70 dark:bg-emerald-500/10 dark:text-emerald-300 dark:ring-emerald-400/20",
  pending: "bg-amber-50 text-amber-800 ring-amber-200/70 dark:bg-amber-500/10 dark:text-amber-300 dark:ring-amber-400/20",
  refunded: "bg-red-50 text-red-700 ring-red-200/70 dark:bg-red-500/10 dark:text-red-300 dark:ring-red-400/20",
  demo: "bg-violet-50 text-violet-700 ring-violet-200/70 dark:bg-violet-500/10 dark:text-violet-300 dark:ring-violet-400/20",
  // 2.13.0 (TKT-01): Ticket Center's "Needs" column. Named for what they
  // mean rather than reusing unpaid/partial/demo, whose names would read as
  // wrong next to "Needs listing". Colors match the three filter tiles.
  needslisting: "bg-amber-50 text-amber-800 ring-amber-200/70 dark:bg-amber-500/10 dark:text-amber-300 dark:ring-amber-400/20",
  needspayment: "bg-cyan-50 text-cyan-700 ring-cyan-200/70 dark:bg-cyan-500/10 dark:text-cyan-300 dark:ring-cyan-400/20",
  needsdelivery: "bg-violet-50 text-violet-700 ring-violet-200/70 dark:bg-violet-500/10 dark:text-violet-300 dark:ring-violet-400/20",
  // Order-inventory status (derived client-side from ticket counts, not a DB column).
  active: "bg-emerald-50 text-emerald-700 ring-emerald-200/70 dark:bg-emerald-500/10 dark:text-emerald-300 dark:ring-emerald-400/20",
  soldout: "bg-slate-100 text-slate-700 ring-slate-200 dark:bg-slate-100/5 dark:text-slate-300 dark:ring-slate-100/10",
  // 2.0.68: marko's own manual resaleStatus/deliveryStatus (Ticket.resaleStatus/
  // deliveryStatus - free text, not a DB enum) shown as their own badges for
  // the first time (Sale Detail, Order Detail) - see REDESIGN-2.0.68-REPORT.md.
  // Callers pass `value.toLowerCase()` as the tone (the canonical values are
  // capitalized - "Listed"/"Not delivered" - to match the <Select> options in
  // Tickets.tsx's TicketEditModal), so keys here are lowercase. "listed"/
  // "sold" deliberately reuse the SAME keys ticket.status already defines
  // above - the two fields are conceptually related even though they're
  // independent, so sharing a color reads as consistent rather than
  // confusing. Any other free-text value (or the sheet-sync import path)
  // falls back to Badge's own default slate below, same as any unrecognized
  // tone already does.
  unlisted: "bg-slate-100 text-slate-700 ring-slate-200 dark:bg-slate-100/5 dark:text-slate-300 dark:ring-slate-100/10",
  delivered: "bg-emerald-50 text-emerald-700 ring-emerald-200/70 dark:bg-emerald-500/10 dark:text-emerald-300 dark:ring-emerald-400/20",
  "not delivered": "bg-amber-50 text-amber-800 ring-amber-200/70 dark:bg-amber-500/10 dark:text-amber-300 dark:ring-amber-400/20",
  // Shown when a grouped sale's lines don't all share one value (e.g. one
  // ticket in a batch was refunded while the rest weren't).
  mixed: "bg-amber-50 text-amber-800 ring-amber-200/70 dark:bg-amber-500/10 dark:text-amber-300 dark:ring-amber-400/20",
};

const DEFAULT_TONE =
  "bg-slate-100 text-slate-700 ring-slate-200 dark:bg-slate-100/5 dark:text-slate-300 dark:ring-slate-100/10";

export function Badge({ tone, title, children }: { tone: string; title?: string; children: ReactNode }) {
  const cls = STATUS_TONES[tone] ?? DEFAULT_TONE;
  // The leading dot inherits `currentColor`, so it is automatically the
  // right color for every tone above (and for the fallback) without a
  // second per-tone table to keep in sync.
  return (
    <span
      title={title}
      className={`inline-flex max-w-full items-center gap-1.5 rounded-md px-2 py-0.5 text-xs font-medium capitalize ring-1 ring-inset ${cls}`}
    >
      <span className="h-1.5 w-1.5 shrink-0 rounded-full bg-current opacity-70" aria-hidden="true" />
      <span className="truncate">{children}</span>
    </span>
  );
}

/** 2.0.69 (marko's report): a `Badge` that IS the editing control - clicking
 * it opens a native dropdown of `options` and commits the change immediately
 * on selection, no separate Edit button/modal/Save step. Used for the
 * Status/Delivery status/Payout status columns on Sale Detail and Order
 * Detail, replacing the old "click Edit, change a field in the modal, click
 * Save" round trip for exactly these 3 fields (every OTHER field - Section/
 * Row/Seat/Notes/etc - still only edits through the full ticket/sale editor,
 * unchanged).
 *
 * Deliberately a real `<select>`, not a custom floating listbox: same
 * `appearance-none` + hand-drawn chevron trick the shared `Select` component
 * above already uses (some WebKit builds otherwise force native light chrome
 * on a `<select>`'s closed box once a non-first option is chosen, breaking
 * dark mode) - proven to already work correctly in this exact app, and it
 * comes with working keyboard/screen-reader support for free that a custom
 * popup would have to rebuild from scratch. The trade-off: the OPEN
 * dropdown's own list styling is whatever the OS/webview renders natively,
 * not fully themeable - an accepted, minor cosmetic limitation of this
 * approach, not a bug.
 *
 * `onChange` is awaited and this shows a brief disabled/"saving" state on
 * the control itself while in flight - the caller is responsible for the
 * actual API call, a toast, and reloading the page's data afterward (this
 * component has no idea which field or endpoint it's driving). Selecting the
 * value already shown, or the empty placeholder, is a no-op.
 *
 * 2.6.0: matched to the restyled `Badge` above - same radius, same ring, and
 * the same leading status dot, so an editable status and a read-only one
 * read as the same object. Behaviour is untouched. */
export function InlineStatusSelect({
  value,
  options,
  onChange,
  title,
  emptyLabel = "-",
}: {
  value: string | null;
  options: readonly string[];
  onChange: (next: string) => void | Promise<void>;
  title?: string;
  emptyLabel?: string;
}) {
  const [saving, setSaving] = useState(false);
  const cls = STATUS_TONES[(value ?? "").toLowerCase()] ?? DEFAULT_TONE;

  return (
    <div className={`relative inline-flex items-center rounded-md ring-1 ring-inset ${cls}`}>
      <span
        className="pointer-events-none absolute left-2 h-1.5 w-1.5 shrink-0 rounded-full bg-current opacity-70"
        aria-hidden="true"
      />
      <select
        title={title}
        aria-label={title}
        disabled={saving}
        value={value ?? ""}
        onChange={async (e) => {
          const next = e.target.value;
          if (!next || next === value) return;
          setSaving(true);
          try {
            await onChange(next);
          } finally {
            setSaving(false);
          }
        }}
        className="cursor-pointer appearance-none rounded-md bg-transparent py-0.5 pl-5 pr-5 text-xs font-medium capitalize text-current transition focus:outline-none focus-visible:ring-2 focus-visible:ring-brand-500 disabled:cursor-wait disabled:opacity-60"
      >
        {!value && (
          <option value="" disabled>
            {emptyLabel}
          </option>
        )}
        {options.map((o) => (
          <option key={o} value={o}>
            {o}
          </option>
        ))}
      </select>
      <IconChevronDown className="pointer-events-none absolute right-1 top-1/2 h-3 w-3 -translate-y-1/2 opacity-60" />
    </div>
  );
}

// ---------------------------------------------------------------------------
// Stat card (used on Dashboard + Event detail)
// ---------------------------------------------------------------------------

export function StatCard({
  label,
  value,
  sub,
  tone = "default",
  trend,
  trendColored = true,
}: {
  label: string;
  value: string;
  sub?: string;
  tone?: "default" | "positive" | "negative";
  /** Optional "vs previous period" delta (2.0.47, DIR-001) - see
   * computeTrend/computeTrendPoints in lib/format.ts. Omitted/null renders
   * nothing, so every pre-2.0.47 caller (Event Detail's StatCard usages
   * never pass this) stays visually unchanged. */
  trend?: TrendInfo | null;
  /** false = the trend arrow/text always render in neutral slate regardless
   * of direction - for a metric where "up" isn't unambiguously good (e.g.
   * Purchase cost - spending more isn't necessarily bad). Default true
   * colors it emerald(up)/red(down)/slate(flat), the same up=good
   * convention this card's own `tone` prop already uses for realized
   * profit/loss. Ignored when `trend` is absent. */
  trendColored?: boolean;
}) {
  // 2.13.1: renders SummaryStat below - see its own comment.
  // 2.13.0: the same journey 2.6.0 started, finished. That round made the
  // KPI card smaller; this one stops it being a card at all. A summary row
  // sits above a table on ten screens, and a bordered box per figure gave
  // the summary the same visual weight as the content underneath it. Now
  // each figure is one pill - number loud, label quiet beside it, trend and
  // sub kept inline rather than dropped, so no information is lost.
  //
  // Callers wrap these in `flex flex-wrap gap-2`, NOT a grid: a chip inside
  // a grid cell stretches to the column and stops reading as a chip. If you
  // add a new summary row, use flex.
  const valueTone =
    tone === "positive"
      ? "text-emerald-600 dark:text-emerald-400"
      : tone === "negative"
        ? "text-red-600 dark:text-red-400"
        : "text-slate-900 dark:text-slate-50";
  const trendTone =
    !trend || !trendColored || trend.direction === "flat"
      ? "text-slate-400 dark:text-slate-500"
      : trend.direction === "up"
        ? "text-emerald-600 dark:text-emerald-400"
        : "text-red-600 dark:text-red-400";
  return (
    <SummaryStat
      label={label}
      value={value}
      tone={tone}
      extra={
        <>
          {trend && (
            <span className={`ml-1.5 inline-flex items-center gap-0.5 text-xs font-medium ${trendTone}`}>
              {trend.direction === "up" && <IconTrendingUp className="h-3 w-3 shrink-0" />}
              {trend.direction === "down" && <IconTrendingDown className="h-3 w-3 shrink-0" />}
              {trend.label}
            </span>
          )}
          {sub && <span className="ml-1.5 text-xs text-slate-400 dark:text-slate-500">· {sub}</span>}
        </>
      }
    />
  );
}

/** 2.13.1: the one summary figure in the app - "Label: value", grey label,
 * value beside it, nothing around it. Lived in Sales.tsx since 1.9.0; marko
 * pointed at that bar and asked for it on every screen, so it moved here and
 * `StatCard` above is now a thin wrapper over it. Callers put a row of these
 * inside a single `.summary-bar` (index.css) - one border around the row, not
 * one per figure. */
export function SummaryStat({
  label,
  value,
  tone = "default",
  extra,
}: {
  label: string;
  value: string;
  tone?: "default" | "positive" | "negative";
  extra?: ReactNode;
}) {
  const toneCls =
    tone === "positive"
      ? "text-emerald-600 dark:text-emerald-400"
      : tone === "negative"
        ? "text-red-600 dark:text-red-400"
        : "text-slate-900 dark:text-slate-100";
  return (
    <span className="whitespace-nowrap">
      <span className="text-slate-400 dark:text-slate-500">{label}: </span>
      <span className={`font-medium tabular-nums ${toneCls}`}>{value}</span>
      {extra}
    </span>
  );
}


// ---------------------------------------------------------------------------
// Modal
// ---------------------------------------------------------------------------
export function Modal({
  open,
  onClose,
  title,
  children,
  width = "max-w-lg",
}: {
  open: boolean;
  onClose: () => void;
  title: string;
  children: ReactNode;
  width?: string;
}) {
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  if (!open) return null;
  return (
    // 2.0.74: backdrop fades in (`fadein`), panel fades+scales in
    // (`pop-in`) - both defined once in index.css, shared with
    // ConfirmDialog and the profile dropdown so every "a box just appeared"
    // moment in the app moves the same way. Entrance-only (closing is still
    // an instant unmount, same as before) - keeps this a small, contained
    // change rather than needing every caller to also manage an "is this
    // still animating out" state just to unmount a beat later.
    // 2.6.0: the panel is now a real bordered surface with the shared
    // `shadow-overlay`, its own header band, and a footer-safe scroll area -
    // the header no longer scrolls away with the content.
    <div className="fixed inset-0 z-50 flex items-start justify-center overflow-y-auto bg-slate-900/40 p-4 pt-[8vh] backdrop-blur-[2px] animate-[fadein_.15s_ease-out] dark:bg-slate-950/70">
      <div
        className={`w-full ${width} overflow-hidden rounded-xl border border-slate-200 bg-white shadow-overlay animate-[pop-in_.18s_ease-out] dark:border-slate-800 dark:bg-slate-900`}
      >
        <div className="flex items-center justify-between gap-4 border-b border-slate-200 bg-slate-50/70 px-5 py-3.5 dark:border-slate-800 dark:bg-slate-800/30">
          <h2 className="truncate text-sm font-semibold text-slate-900 dark:text-slate-50">{title}</h2>
          <button
            onClick={onClose}
            className="-mr-1 shrink-0 rounded-md p-1.5 text-slate-400 transition hover:bg-slate-200/70 hover:text-slate-700 focus:outline-none focus-visible:ring-2 focus-visible:ring-brand-500 dark:text-slate-500 dark:hover:bg-slate-700/60 dark:hover:text-slate-200"
            aria-label="Close"
          >
            <IconX className="h-4 w-4" />
          </button>
        </div>
        <div className="max-h-[75vh] overflow-y-auto px-5 py-4">{children}</div>
      </div>
    </div>
  );
}

export function ModalFooter({ children }: { children: ReactNode }) {
  return (
    <div className="-mx-5 -mb-4 mt-5 flex justify-end gap-2 border-t border-slate-200 bg-slate-50/70 px-5 py-3.5 dark:border-slate-800 dark:bg-slate-800/30">
      {children}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Confirm dialog
// ---------------------------------------------------------------------------
export function ConfirmDialog({
  open,
  title,
  message,
  confirmLabel = "Confirm",
  danger = false,
  busy = false,
  onConfirm,
  onCancel,
}: {
  open: boolean;
  title: string;
  message: ReactNode;
  confirmLabel?: string;
  danger?: boolean;
  busy?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  if (!open) return null;
  return (
    // 2.0.74: same entrance treatment as Modal above - see its comment.
    <div className="fixed inset-0 z-[60] flex items-center justify-center bg-slate-900/40 p-4 backdrop-blur-[2px] animate-[fadein_.15s_ease-out] dark:bg-slate-950/70">
      <div className="w-full max-w-sm rounded-xl border border-slate-200 bg-white p-5 shadow-overlay animate-[pop-in_.18s_ease-out] dark:border-slate-800 dark:bg-slate-900">
        <div className="flex gap-3.5">
          <div
            className={`flex h-9 w-9 shrink-0 items-center justify-center rounded-full ring-1 ring-inset ${danger ? "bg-red-50 text-red-600 ring-red-200/70 dark:bg-red-500/10 dark:text-red-400 dark:ring-red-400/20" : "bg-amber-50 text-amber-600 ring-amber-200/70 dark:bg-amber-500/10 dark:text-amber-400 dark:ring-amber-400/20"}`}
          >
            <IconAlertTriangle className="h-5 w-5" />
          </div>
          <div className="min-w-0">
            <h3 className="text-sm font-semibold text-slate-900 dark:text-slate-50">{title}</h3>
            <div className="mt-1 text-sm leading-relaxed text-slate-500 dark:text-slate-400">{message}</div>
          </div>
        </div>
        <div className="mt-5 flex justify-end gap-2">
          <Button variant="secondary" onClick={onCancel} disabled={busy}>
            Cancel
          </Button>
          <Button variant={danger ? "danger" : "primary"} onClick={onConfirm} disabled={busy}>
            {busy ? <Spinner className="h-4 w-4" /> : confirmLabel}
          </Button>
        </div>
      </div>
    </div>
  );
}

/** 2.6.0: the app's ONE segmented-control look, exported as raw class
 * strings rather than only as a component. `TabSwitcher` below is the
 * two/three-tab component form, but Dashboard.tsx has three of these rows
 * whose state/keys don't fit that component's shape (a period picker, a
 * metric picker, and its own top-level tab row), and before 2.6.0 all three
 * hand-rolled the same classes independently - which is exactly how the two
 * looks drifted apart in the first place. Sharing the strings means there is
 * genuinely one definition, with no call site forced to restructure its
 * state to use it.
 *
 * The look itself: a recessed track with a raised, near-white thumb on the
 * active item, replacing the solid brand-blue fill. The blue fill made an
 * ordinary list filter read as the loudest control on the page. */
export const SEGMENTED_TRACK =
  "inline-flex w-fit max-w-full flex-wrap items-center gap-1 rounded-lg border border-slate-200 bg-slate-100/70 p-1 dark:border-slate-800 dark:bg-slate-900/70";

export function segmentedItemClass(active: boolean): string {
  return `rounded-md px-3 py-1.5 text-xs font-medium transition focus:outline-none focus-visible:ring-2 focus-visible:ring-brand-500 ${
    active
      ? "bg-white text-slate-900 shadow-card dark:bg-slate-700/70 dark:text-slate-50"
      : "text-slate-500 hover:text-slate-800 dark:text-slate-400 dark:hover:text-slate-200"
  }`;
}

/** 2.0.59: shared "Active vs Completed" pill switcher for Events/Orders/
 * Tickets/Sales - same visual pattern (and exact same classNames) as
 * Dashboard.tsx's own Overview/Financials/Activity tab row, extracted here
 * so four pages don't each hand-roll their own copy. Dashboard's own tab row
 * is untouched (it has 3 tabs, not 2, and already shipped/works) - this is
 * for the new pages only, paired with lib/useListTab.ts for the
 * load/persist half of the pattern.
 *
 * 2.6.0: restyled onto the shared strings above, which Dashboard's three
 * rows now use too - so the pattern this component was extracted to protect
 * finally has a single definition rather than two that merely looked alike. */
export function TabSwitcher<T extends string>({
  tabs,
  active,
  onChange,
  className = "",
}: {
  tabs: { key: T; label: string }[];
  active: T;
  onChange: (key: T) => void;
  className?: string;
}) {
  return (
    <div className={`${SEGMENTED_TRACK} mb-4 ${className}`}>
      {tabs.map((t) => (
        <button
          key={t.key}
          type="button"
          onClick={() => onChange(t.key)}
          aria-pressed={active === t.key}
          className={segmentedItemClass(active === t.key)}
        >
          {t.label}
        </button>
      ))}
    </div>
  );
}

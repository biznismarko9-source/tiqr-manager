import { useEffect, useMemo, useRef, useState, type ButtonHTMLAttributes, type ChangeEvent, type HTMLAttributes, type InputHTMLAttributes, type ReactNode, type SelectHTMLAttributes, type TextareaHTMLAttributes } from "react";
import { IconAlertTriangle, IconCalendarDays, IconChevronDown, IconChevronLeft, IconChevronRight, IconCopy, IconPlus, IconTrendingDown, IconTrendingUp, IconX } from "./icons";
import { formatDateNumeric } from "../lib/format";
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
      "bg-brand-600 text-white hover:bg-brand-500 focus-visible:ring-brand-500",
    secondary:
      "bg-surface text-slate-700 shadow-card hover:bg-surface-muted focus-visible:ring-slate-400 dark:text-slate-200 dark:focus-visible:ring-slate-500",
    danger: "bg-red-600 text-white hover:bg-red-500 focus-visible:ring-red-500",
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
// ---------------------------------------------------------------------------
// Date field
// ---------------------------------------------------------------------------

/** 2.38.0: every `<input type="date">` in the app dropped the BROWSER's own
 * calendar - a white panel with its own type, its own week layout and no idea
 * the app is in dark mode (marko sent a screenshot of exactly that). It is
 * browser chrome, so no amount of CSS reaches it; the only way to make that
 * panel ours is to draw it ourselves.
 *
 * `Input` routes `type="date"` here automatically, so not one of the ~30 call
 * sites changed: they still pass `value` as "YYYY-MM-DD" and still read
 * `e.target.value` inside onChange. The object handed to onChange is a minimal
 * stand-in carrying `target.value`/`target.name` - checked against every date
 * call site in the app, that is the only thing any of them reads off it.
 *
 * Deliberately NOT a dependency: TIQR ships offline-first and adds no packages.
 */
const WEEKDAY_LABELS = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];
const MONTH_LABELS = [
  "January",
  "February",
  "March",
  "April",
  "May",
  "June",
  "July",
  "August",
  "September",
  "October",
  "November",
  "December",
];

/** Roughly what the open panel needs, used only to decide which way it opens. */
const PANEL_H = 330;
const PANEL_W = 280;

/** "YYYY-MM-DD" -> parts, or null for anything else (including ""). Parsed by
 * hand rather than through `new Date(s)`: that reads a bare ISO date as UTC
 * midnight and then renders it in local time, which west of Greenwich shows
 * the day BEFORE the one that is stored. */
function parseIsoDate(s: string): { y: number; m: number; d: number } | null {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(s);
  if (!m) return null;
  const y = Number(m[1]);
  const mo = Number(m[2]);
  const d = Number(m[3]);
  if (mo < 1 || mo > 12 || d < 1 || d > 31) return null;
  return { y, m: mo, d };
}

function isoOf(y: number, m: number, d: number): string {
  return `${String(y).padStart(4, "0")}-${String(m).padStart(2, "0")}-${String(d).padStart(2, "0")}`;
}

/** Day 0 of the NEXT month is the last day of this one. `m` is 1-based. */
function daysInMonth(y: number, m: number): number {
  return new Date(y, m, 0).getDate();
}

/** Monday-first index (0 = Monday) of the 1st of the given month. */
function mondayFirstOffset(y: number, m: number): number {
  return (new Date(y, m - 1, 1).getDay() + 6) % 7;
}

function DateField(props: InputHTMLAttributes<HTMLInputElement>) {
  const { className = "", value, onChange, disabled, placeholder, id, name } = props;
  const ariaLabel = props["aria-label"];
  const text = typeof value === "string" ? value : "";
  const selected = parseIsoDate(text);

  const [open, setOpen] = useState(false);
  // 2.47.1: the panel is POSITION-FIXED and carries its own viewport
  // coordinates. It used to be `absolute`, which an ancestor with
  // `overflow:auto` clips - and that is exactly what `.table-shell` is. Inside
  // the row forms (New Order / New Pull / New Event) the calendar therefore
  // opened into a clipped box and read as "the date doesn't work".
  const [rect, setRect] = useState<{ top: number; left: number } | null>(null);
  const [view, setView] = useState(() => {
    const p = parseIsoDate(typeof value === "string" ? value : "");
    const now = new Date();
    return p ? { y: p.y, m: p.m } : { y: now.getFullYear(), m: now.getMonth() + 1 };
  });
  const wrapRef = useRef<HTMLDivElement | null>(null);

  // A value set from OUTSIDE (Clear all, loading a record into a form) moves
  // the visible month with it, so re-opening never starts somewhere else.
  useEffect(() => {
    const p = parseIsoDate(text);
    if (p) setView({ y: p.y, m: p.m });
  }, [text]);

  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (wrapRef.current && !wrapRef.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", onDown);
    return () => document.removeEventListener("mousedown", onDown);
  }, [open]);

  const emit = (next: string) => {
    if (!onChange) return;
    onChange({ target: { value: next, name: name ?? "" } } as unknown as ChangeEvent<HTMLInputElement>);
  };

  const openPanel = () => {
    const r = wrapRef.current?.getBoundingClientRect();
    if (r) {
      // Flip up when there is no room below, and pull left when the panel
      // would run off the right edge - the same two decisions as before, now
      // expressed as real viewport coordinates instead of CSS sides.
      const above = r.bottom + PANEL_H > window.innerHeight && r.top > PANEL_H;
      const left = Math.max(8, Math.min(r.left, window.innerWidth - PANEL_W - 8));
      setRect({ top: above ? r.top - PANEL_H - 6 : r.bottom + 6, left });
    }
    setOpen(true);
  };

  // A fixed panel does not travel with its field, so any scroll or resize
  // closes it rather than leaving a calendar floating over the wrong row.
  useEffect(() => {
    if (!open) return;
    const close = () => setOpen(false);
    window.addEventListener("scroll", close, true);
    window.addEventListener("resize", close);
    return () => {
      window.removeEventListener("scroll", close, true);
      window.removeEventListener("resize", close);
    };
  }, [open]);

  const shiftMonth = (by: number) => {
    setView((v) => {
      const raw = v.m - 1 + by;
      return { y: v.y + Math.floor(raw / 12), m: (((raw % 12) + 12) % 12) + 1 };
    });
  };

  // Always six rows, so the panel never changes height as you page through
  // months - the days either side are real dates and pick like any other.
  const cells = useMemo(() => {
    const lead = mondayFirstOffset(view.y, view.m);
    const prevM = view.m === 1 ? 12 : view.m - 1;
    const prevY = view.m === 1 ? view.y - 1 : view.y;
    const prevDim = daysInMonth(prevY, prevM);
    const nextM = view.m === 12 ? 1 : view.m + 1;
    const nextY = view.m === 12 ? view.y + 1 : view.y;
    const out: { y: number; m: number; d: number; outside: boolean }[] = [];
    for (let i = lead - 1; i >= 0; i--) out.push({ y: prevY, m: prevM, d: prevDim - i, outside: true });
    const dim = daysInMonth(view.y, view.m);
    for (let d = 1; d <= dim; d++) out.push({ y: view.y, m: view.m, d, outside: false });
    for (let d = 1; out.length < 42; d++) out.push({ y: nextY, m: nextM, d, outside: true });
    return out;
  }, [view]);

  const now = new Date();
  const todayKey = isoOf(now.getFullYear(), now.getMonth() + 1, now.getDate());
  const navClass =
    "rounded-md p-1.5 text-slate-500 transition hover:bg-surface-sunken hover:text-slate-900 dark:text-slate-400 dark:hover:text-slate-100";
  const footClass =
    "rounded-md px-2 py-1 text-[11.5px] font-medium text-slate-500 transition hover:bg-surface-sunken hover:text-slate-900 dark:text-slate-400 dark:hover:text-slate-100";

  return (
    <div
      ref={wrapRef}
      className="relative"
      onKeyDown={(e) => {
        // Stopped here on purpose: Modal listens for Escape on `window`, and
        // without this, closing the calendar would close the form under it.
        if (e.key === "Escape" && open) {
          e.stopPropagation();
          setOpen(false);
        }
      }}
    >
      <button
        type="button"
        id={id}
        disabled={disabled}
        aria-label={ariaLabel}
        aria-haspopup="dialog"
        aria-expanded={open}
        onClick={() => (open ? setOpen(false) : openPanel())}
        className={`input flex items-center justify-between gap-2 text-left ${className}`}
      >
        <span className={selected ? "truncate" : "truncate text-slate-500 dark:text-slate-400"}>
          {selected ? formatDateNumeric(text) : placeholder || "dd.mm.yyyy"}
        </span>
        <IconCalendarDays className="h-4 w-4 shrink-0 text-slate-500 dark:text-slate-400" />
      </button>

      {open && (
        <div
          role="dialog"
          className="fixed z-[60] w-[17.5rem] rounded-xl bg-surface-raised p-3 shadow-overlay"
          style={{ top: rect?.top ?? 0, left: rect?.left ?? 0 }}
        >
          <div className="mb-2 flex items-center justify-between gap-1">
            <button type="button" aria-label="Previous month" onClick={() => shiftMonth(-1)} className={navClass}>
              <IconChevronLeft className="h-4 w-4" />
            </button>
            <span className="text-[13px] font-semibold text-slate-800 dark:text-slate-100">
              {MONTH_LABELS[view.m - 1]} {view.y}
            </span>
            <button type="button" aria-label="Next month" onClick={() => shiftMonth(1)} className={navClass}>
              <IconChevronRight className="h-4 w-4" />
            </button>
          </div>

          <div className="grid grid-cols-7 gap-0.5">
            {WEEKDAY_LABELS.map((w) => (
              <span
                key={w}
                className="py-1 text-center text-[10.5px] font-medium text-slate-500 dark:text-slate-400"
              >
                {w}
              </span>
            ))}
            {cells.map((c) => {
              const iso = isoOf(c.y, c.m, c.d);
              const isSelected = iso === text;
              const isToday = iso === todayKey;
              return (
                <button
                  key={iso}
                  type="button"
                  aria-current={isSelected ? "date" : undefined}
                  onClick={() => {
                    emit(iso);
                    setView({ y: c.y, m: c.m });
                    setOpen(false);
                  }}
                  className={`h-8 rounded-md text-[12.5px] tabular-nums transition ${
                    isSelected
                      ? "bg-brand-600 font-semibold text-white"
                      : isToday
                        ? "font-semibold text-brand-600 hover:bg-surface-sunken dark:text-brand-400"
                        : c.outside
                          ? "text-slate-400 hover:bg-surface-sunken dark:text-slate-600"
                          : "text-slate-700 hover:bg-surface-sunken dark:text-slate-200"
                  }`}
                >
                  {c.d}
                </button>
              );
            })}
          </div>

          <div className="mt-2 flex items-center justify-between border-t border-line pt-2">
            <button
              type="button"
              className={footClass}
              onClick={() => {
                emit(todayKey);
                setOpen(false);
              }}
            >
              Today
            </button>
            <button
              type="button"
              className={footClass}
              onClick={() => {
                emit("");
                setOpen(false);
              }}
            >
              Clear
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

export function Input(props: InputHTMLAttributes<HTMLInputElement>) {
  const { className = "", ...rest } = props;
  // See DateField above: the native date popup cannot be themed, so we draw it.
  if (props.type === "date") return <DateField {...props} />;
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
      {hint && !error && <span className="mt-1.5 block text-xs text-slate-500 dark:text-slate-400">{hint}</span>}
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
    // 2.25.0: `data-tour` only - two anchors for the guided tour
    // (components/Tour.tsx), which is how one edit here gives every page in
    // the app a place for the tour to point at instead of a dozen page edits.
    // No behaviour, no styling, no prop.
    <div data-tour="page-header" className="mb-5 pb-4">
      <div className="flex flex-wrap items-center justify-between gap-x-4 gap-y-2">
        <div className="min-w-0">
          <h1 className="truncate text-[19px] font-semibold leading-tight text-slate-900 dark:text-slate-50">
            {title}
          </h1>
          {subtitle && <p className="mt-1 text-sm text-slate-500 dark:text-slate-400">{subtitle}</p>}
        </div>
        {actions && <div data-tour="page-actions" className="flex flex-wrap items-center gap-2">{actions}</div>}
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
    <div className={`overflow-hidden rounded-xl bg-surface shadow-card ${className}`}>
      <div className="flex items-center gap-4 bg-surface-muted px-3 py-3">
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

/** 2.40.0: the loading state for a row of StatCards - same `summary-bar`
 * grid the real row uses, so the page does not resize when the numbers
 * arrive. Counterpart to TableSkeleton above, for the pages whose first
 * screenful is figures rather than a table. */
export function StatsSkeleton({ count = 5 }: { count?: number }) {
  return (
    <div className="summary-bar">
      {Array.from({ length: count }).map((_, i) => (
        <div key={i} className="rounded-xl bg-surface p-3 shadow-card">
          <Skeleton className="h-2.5 w-16" />
          <Skeleton className="mt-2.5 h-4 w-24" />
        </div>
      ))}
    </div>
  );
}

/** 2.40.0: a card-shaped placeholder - a title band and `lines` rows. Used
 * where the thing loading is a panel of text or a chart rather than a table
 * or a figure row. */
export function PanelSkeleton({ lines = 4, className = "" }: { lines?: number; className?: string }) {
  return (
    <div className={`rounded-xl bg-surface p-4 shadow-card ${className}`}>
      <Skeleton className="h-2.5 w-28" />
      <div className="mt-4 flex flex-col gap-3">
        {Array.from({ length: lines }).map((_, i) => (
          <Skeleton key={i} className={i === lines - 1 ? "h-3 w-2/3" : "h-3"} />
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
    <div className="flex flex-col items-center justify-center gap-1 rounded-xl bg-surface-muted px-6 py-14 text-center shadow-card">
      {icon && (
        <div className="mb-3 flex h-11 w-11 items-center justify-center rounded-full bg-surface text-slate-400 shadow-card dark:text-slate-500">
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

/** 2.40.0: strips the fill and the ring out of a tone, leaving only its text
 * colours. There is deliberately no second colour table - one source of
 * truth (STATUS_TONES above) still decides every tone, and this just drops
 * the two parts of it a quiet badge does not wear. `InlineStatusSelect`
 * below keeps the FULL tone on purpose: it is a control, and a control has
 * to look like one. */
function quietTone(cls: string): string {
  return cls
    .split(" ")
    .filter((c) => {
      const base = c.startsWith("dark:") ? c.slice(5) : c;
      return !base.startsWith("bg-") && !base.startsWith("ring-");
    })
    .join(" ");
}

export function Badge({ tone, title, children }: { tone: string; title?: string; children: ReactNode }) {
  const cls = STATUS_TONES[tone] ?? DEFAULT_TONE;
  // The leading dot inherits `currentColor`, so it is automatically the
  // right color for every tone above (and for the fallback) without a
  // second per-tone table to keep in sync.
  //
  // 2.40.0 (marko picked this out of thirty): no fill, no ring - a coloured
  // DOT beside a plain label. In a table where every row carries a status,
  // the filled pills were the loudest thing on screen and drowned out the
  // numbers beside them. The dot goes full-strength and one step larger now
  // that it carries the identity on its own. Nothing about which tone means
  // what has changed, and no call site changed.
  return (
    <span
      title={title}
      className={`inline-flex max-w-full items-center gap-1.5 py-0.5 text-xs font-medium capitalize ${quietTone(cls)}`}
    >
      <span className="h-2 w-2 shrink-0 rounded-full bg-current" aria-hidden="true" />
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
    // 2.34.2: `max-w-full` + `min-w-0` are what stop this from spilling into
    // the next column. A <select> with `appearance-none` still sizes itself to
    // its WIDEST <option>, not to its current value, and an inline-flex box
    // will not shrink below that on its own - so on a narrow cell "Not
    // delivered" pushed the whole pill past the cell edge, and because every
    // `td` in these tables is `whitespace-nowrap` there was nothing to stop
    // it landing on top of the neighbouring one. That is the overlap marko
    // photographed on his Mac.
    <div className={`relative inline-flex min-w-0 max-w-full items-center rounded-md ring-1 ring-inset ${cls}`}>
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
        // `w-full min-w-0` lets it shrink inside the box above; `truncate`
        // then ends a label that still does not fit with an ellipsis instead
        // of letting it escape. The full value stays readable via `title`.
        className="w-full min-w-0 cursor-pointer truncate appearance-none rounded-md bg-transparent py-0.5 pl-5 pr-5 text-xs font-medium capitalize text-current transition focus:outline-none focus-visible:ring-2 focus-visible:ring-brand-500 disabled:cursor-wait disabled:opacity-60"
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

/** 2.34.2: one row of dots, one per completion check - the shape marko asked
 *  to have "everywhere the same". Pulls had it hand-rolled; this is that,
 *  extracted, so Sales and Pulls cannot drift apart.
 *
 *  It takes the exact `CompletionCheck[]` every list page already builds for
 *  `completionStatus()`, so the dots and the badge can never disagree: same
 *  array, same order, same truth.
 *
 *  Reading it without hovering: the column header names the dots in order
 *  ("Paid · Done", "Sold · Delivered · Paid"), green means done, grey means
 *  not yet. Hovering spells every one out in words, because a dot on its own
 *  is a colour, not a sentence.
 *
 *  A check with an `onToggle` renders as a real button with a proper hit area
 *  (Pulls, where these are things you tick); one without renders as plain
 *  text (Sales, where they are derived and clicking would be a lie). Same
 *  component either way - that is the point of it. */
export function StatusDots({
  checks,
  title,
}: {
  checks: readonly { label: string; done: boolean; onToggle?: () => void }[];
  title?: string;
}) {
  const spelled = title ?? checks.map((c) => `${c.label}: ${c.done ? "done" : "pending"}`).join(" · ");
  return (
    <span className="inline-flex items-center gap-0.5" title={spelled} aria-label={spelled}>
      {checks.map((c) =>
        c.onToggle ? (
          <button
            key={c.label}
            type="button"
            onClick={c.onToggle}
            aria-label={`${c.label}: ${c.done ? "done" : "pending"}`}
            aria-pressed={c.done}
            className="rounded-full p-1.5 transition-colors hover:bg-slate-100 dark:hover:bg-slate-800"
          >
            <span className={`block h-2.5 w-2.5 rounded-full ${c.done ? "bg-emerald-500" : "bg-slate-300 dark:bg-slate-600"}`} />
          </button>
        ) : (
          <span key={c.label} className="p-1.5">
            <span className={`block h-2.5 w-2.5 rounded-full ${c.done ? "bg-emerald-500" : "bg-slate-300 dark:bg-slate-600"}`} />
          </span>
        ),
      )}
    </span>
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
  emphasis = false,
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
  /** 2.13.3: the one figure on a row that is the answer, not the context -
   * Dashboard's Profit. A slightly larger number and a ring, which is enough
   * to find it without giving it a band of the page to itself (DSH-01 did
   * that; marko wanted everything on one line instead).
   * At most one card per row should set this, or none of them stands out.
   *
   * 2.47.4: the ring is NEUTRAL, and it is a ring rather than a border.
   * Two separate bugs lived in the old `border-brand-300 bg-brand-50/40`:
   *   - `.card` sets `border: 0`, so the border colour drew NOTHING. The only
   *     thing that ever rendered was the fill.
   *   - `brand-50` (#e4ddfd) is a saturated lavender, so in light mode that
   *     fill turned the whole card purple next to four white ones - it read
   *     as "selected", and it fought the emerald profit figure sitting on it.
   *     marko: "na light mode je tam chybna farba pri tom profit widgete".
   * A ring needs no border width to exist and cannot be beaten by `.card`'s
   * own `bg-surface`, which is what made the old fill unpredictable too. */
  emphasis?: boolean;
  /** false = the trend arrow/text always render in neutral slate regardless
   * of direction - for a metric where "up" isn't unambiguously good (e.g.
   * Purchase cost - spending more isn't necessarily bad). Default true
   * colors it emerald(up)/red(down)/slate(flat), the same up=good
   * convention this card's own `tone` prop already uses for realized
   * profit/loss. Ignored when `trend` is absent. */
  trendColored?: boolean;
}) {
  // 2.13.2: a card again, one per figure - marko compared 2.13.1's single
  // bar against the Attention Center row and wanted each figure in its own
  // box. Same metrics as `AttentionCategoryCard` in Dashboard.tsx (p-3.5,
  // 22px value) so a KPI tile and an Attention tile stay the same object.
  // `flex-1` with a min width is what lets a row of 3 and a row of 6 both
  // fill the width without a grid that has to be re-tuned per screen.
  // 2.13.0/2.13.1 tried a chip and then a shared bar; both are in the
  // changelog, and `SummaryStat` below survives from that round because
  // Sales' own results strip genuinely is one line of text. That round made the
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
      ? "text-slate-500 dark:text-slate-400"
      : trend.direction === "up"
        ? "text-emerald-600 dark:text-emerald-400"
        : "text-red-600 dark:text-red-400";
  return (
    // 2.13.3: narrower and a touch tighter than 2.13.2, so six of these fit
    // one row on a normal window instead of wrapping - marko's own ask. They
    // still grow to fill the width when there are only three.
    // `min-w-0` matters now that `.summary-bar` is a grid: a grid item
    // defaults to min-content width, so without it a long figure widens its
    // own column and pushes the row out of the page instead of truncating.
    // `flex-1` is kept for the handful of callers that still use their own
    // flex row (see SummaryStat's note below) - it is inert inside the grid.
    <Card
      className={`min-w-0 flex-1 p-3 ${
        emphasis ? "ring-1 ring-slate-300 dark:ring-slate-600" : ""
      }`}
    >
      <p className="section-title truncate">{label}</p>
      <p
        className={`mt-1.5 truncate font-semibold leading-none tabular-nums ${emphasis ? "text-[24px]" : "text-[19px]"} ${valueTone}`}
      >
        {value}
      </p>
      {trend && (
        <p className={`mt-2 flex items-center gap-1 text-[11px] font-medium ${trendTone}`}>
          {trend.direction === "up" && <IconTrendingUp className="h-3 w-3 shrink-0" />}
          {trend.direction === "down" && <IconTrendingDown className="h-3 w-3 shrink-0" />}
          {trend.label} <span className="font-normal text-slate-500 dark:text-slate-400">vs. previous</span>
        </p>
      )}
      {sub && <p className="mt-1.5 truncate text-[11px] text-slate-500 dark:text-slate-400">{sub}</p>}
    </Card>
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
      <span className="text-slate-500 dark:text-slate-400">{label}: </span>
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
        className={`w-full ${width} overflow-hidden rounded-xl bg-surface shadow-overlay animate-[pop-in_.18s_ease-out]`}
      >
        <div className="flex items-center justify-between gap-4 bg-surface-muted px-5 py-3.5">
          <h2 className="truncate text-sm font-semibold text-slate-900 dark:text-slate-50">{title}</h2>
          <button
            onClick={onClose}
            className="-mr-1 shrink-0 rounded-md p-1.5 text-slate-400 transition hover:bg-slate-200/70 hover:text-slate-700 focus:outline-none focus-visible:ring-2 focus-visible:ring-brand-500 dark:text-slate-500 dark:hover:bg-slate-700/60 dark:hover:text-slate-200"
            aria-label="Close"
          >
            <IconX className="h-4 w-4" />
          </button>
        </div>
        {/* 2.41.0: one column. The split-preview branch and its `preview`
            prop are gone with PreviewPanel - see the Orders/Events forms. */}
        <div className="max-h-[75vh] overflow-y-auto px-5 py-4">{children}</div>
      </div>
    </div>
  );
}

/* ---------------------------------------------------------------------------
   Row forms

   2.45.0: marko - "vsade events, inventory, sales a pulls budu podobne tie
   vyplnovace udajov". Creating anything in this app is now the same gesture:
   a table of rows inside a modal, one row per thing, an "add another" button
   under it and one footer that says what is about to happen.

   These three pieces are what make the four forms LOOK the same rather than
   four tables that merely resemble each other. Each caller still writes its
   own cells - the fields genuinely differ - but the shell, the header band,
   the remove column and the footer come from here, so a change to the look
   is one edit, not four.
   --------------------------------------------------------------------------- */

/** The table shell plus the "add another" button underneath it. `head` is the
 *  visible column labels; the trailing remove column is added here so no
 *  caller has to remember it. */
export function RowFormTable({
  head,
  children,
  onAdd,
  addLabel,
  rightAlign = [],
}: {
  head: string[];
  children: ReactNode;
  onAdd: () => void;
  addLabel: string;
  /** Column indices whose values are numbers. A right-aligned figure under a
   *  left-aligned label is the classic tell of a table nobody set properly. */
  rightAlign?: number[];
}) {
  const bodyRef = useRef<HTMLTableSectionElement>(null);

  // 2.46.0: adding a row moves the cursor into it. Typing the next ticket
  // should never cost a mouse trip back to the first column - and without
  // this, a long form scrolls the new row into view but leaves focus behind
  // on the button.
  function handleAdd() {
    onAdd();
    requestAnimationFrame(() => {
      const rows = bodyRef.current?.querySelectorAll("tr");
      const last = rows?.[rows.length - 1];
      last?.querySelector<HTMLElement>("input, select, textarea, button")?.focus();
      last?.scrollIntoView({ block: "nearest" });
    });
  }

  return (
    <>
      <div className="table-shell table-shell-compact mb-3">
        <table className="w-full border-collapse">
          {/* The shell already scrolls (see .table-shell in index.css), so a
              sticky header costs nothing and a twenty-row tour keeps its
              column names. */}
          <thead className="sticky top-0 z-10 bg-surface">
            <tr>
              <th className="th-c w-[28px]" aria-label="Row" />
              {head.map((h, i) => (
                <th
                  key={`${h}-${i}`}
                  className={`th-c whitespace-nowrap ${rightAlign.includes(i) ? "text-right" : ""}`}
                >
                  {h}
                </th>
              ))}
              <th className="th-c w-[64px]" />
            </tr>
          </thead>
          <tbody ref={bodyRef} className="divide-y divide-slate-100 dark:divide-slate-800">{children}</tbody>
        </table>
      </div>
      <Button variant="secondary" onClick={handleAdd}>
        <IconPlus className="h-4 w-4" /> {addLabel}
      </Button>
    </>
  );
}

/** The row number, first cell of every row. It exists so a problem can name
 *  a row ("riadok 3") and be found without counting. */
export function RowNumber({ n }: { n: number }) {
  return (
    <td className="td-c w-[28px] text-right text-[11px] tabular-nums text-slate-400 dark:text-slate-500">{n}</td>
  );
}

/** One problem with one cell. `missing` is an empty required field - shown
 *  only once Create has been pressed, because a blank form is not yet wrong.
 *  `invalid` is something actually typed that cannot be what it claims, and
 *  that is shown the moment it is true. */
export interface RowProblem {
  row: number;
  field: string;
  message: string;
  kind: "missing" | "invalid";
}

/** `.input-error` for a cell with a problem - the same red halo <Field error>
 *  already draws everywhere else in the app, so a bad cell looks like a bad
 *  field rather than inventing a second error style. */
export function cellError(problems: RowProblem[], row: number, field: string): string {
  return problems.some((p) => p.row === row && p.field === field) ? "input-error" : "";
}

/** What the footer shows and what submit is blocked on: everything once
 *  Create has been pressed, only the genuinely malformed before that. */
export function visibleProblems(problems: RowProblem[], submitted: boolean): RowProblem[] {
  return submitted ? problems : problems.filter((p) => p.kind === "invalid");
}

/** The last cell of every row. Hidden - not disabled - on a one-row form,
 *  because a form you cannot empty needs no explanation. */
export function RowRemove({
  show,
  onRemove,
  label,
  onDuplicate,
  duplicateLabel,
}: {
  show: boolean;
  onRemove: () => void;
  label: string;
  /** 2.46.0: six nights of one tour, or four tickets that differ by a seat
   *  number, are faster to duplicate and edit than to retype. Omitted by a
   *  form where a row is a real record that cannot be copied (Sales rows ARE
   *  tickets). */
  onDuplicate?: () => void;
  duplicateLabel?: string;
}) {
  return (
    <td className="td-c w-[64px]">
      <div className="flex items-center gap-0.5">
        {onDuplicate && (
          <button
            type="button"
            onClick={onDuplicate}
            aria-label={duplicateLabel ?? "Duplicate row"}
            title={duplicateLabel ?? "Duplicate row"}
            className="rounded-md p-1 text-slate-400 transition hover:bg-slate-100 hover:text-slate-800 dark:text-slate-500 dark:hover:bg-slate-800 dark:hover:text-slate-100"
          >
            <IconCopy className="h-4 w-4" />
          </button>
        )}
        {show && (
          <button
            type="button"
            onClick={onRemove}
            aria-label={label}
            title={label}
            className="rounded-md p-1 text-slate-400 transition hover:bg-slate-100 hover:text-red-600 dark:text-slate-500 dark:hover:bg-slate-800 dark:hover:text-red-400"
          >
            <IconX className="h-4 w-4" />
          </button>
        )}
      </div>
    </td>
  );
}

/** One footer for all four: what is about to be created on the left, the two
 *  buttons on the right. Sits flush with the modal edge, same as
 *  `ModalFooter` below - a row form IS the modal's content.
 *
 *  2.46.0: it also reports what is wrong and where. `problems` replaces the
 *  old "first bad row, as one sentence" behaviour - Create no longer stops at
 *  the first problem and hides the rest, and every bad cell is already
 *  outlined in the table above. `error` stays for what the BACKEND said,
 *  which is a different thing from a field being wrong.
 */
export function RowFormFooter({
  summary,
  error,
  saving,
  submitLabel,
  onCancel,
  onSubmit,
  problems = [],
}: {
  summary: ReactNode;
  error?: string | null;
  saving?: boolean;
  submitLabel: string;
  onCancel: () => void;
  onSubmit: () => void;
  problems?: RowProblem[];
}) {
  // Cmd/Ctrl+Enter creates, from any cell. A plain Enter is left alone on
  // purpose: inside a date field or a select it already means something.
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if ((e.metaKey || e.ctrlKey) && e.key === "Enter" && !saving) {
        e.preventDefault();
        onSubmit();
      }
    }
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [onSubmit, saving]);

  const first = problems[0];

  return (
    <>
      {problems.length > 0 && (
        <p className="mt-4 flex flex-wrap items-baseline gap-x-2 rounded-lg bg-amber-50 px-3 py-2 text-sm text-amber-800 dark:bg-amber-500/10 dark:text-amber-200">
          <span className="font-medium">
            {problems.length === 1 ? "1 thing to fix" : `${problems.length} things to fix`}
          </span>
          {first && (
            <span className="text-amber-700 dark:text-amber-300">
              row {first.row + 1}: {first.message}
            </span>
          )}
        </p>
      )}
      {error && (
        <p className="mt-4 rounded-lg bg-red-50 px-3 py-2 text-sm text-red-700 dark:bg-red-500/10 dark:text-red-300">
          {error}
        </p>
      )}
      <div className="-mx-5 -mb-4 mt-5 flex flex-wrap items-center gap-3 bg-surface-muted px-5 py-3.5">
        <span className="text-sm text-slate-500 dark:text-slate-400">{summary}</span>
        <span className="ml-auto flex items-center gap-2">
          <kbd className="hidden rounded border border-line px-1.5 py-0.5 text-[11px] text-slate-400 sm:inline dark:text-slate-500">
            ⌘↵
          </kbd>
          <Button variant="secondary" onClick={onCancel} disabled={saving}>
            Cancel
          </Button>
          <Button variant="primary" onClick={onSubmit} disabled={saving}>
            {saving ? <Spinner className="h-4 w-4" /> : null}
            {submitLabel}
          </Button>
        </span>
      </div>
    </>
  );
}

export function ModalFooter({ children }: { children: ReactNode }) {
  return (
    <div className="-mx-5 -mb-4 mt-5 flex justify-end gap-2 bg-surface-muted px-5 py-3.5">
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
      <div className="w-full max-w-sm rounded-xl bg-surface p-5 shadow-overlay animate-[pop-in_.18s_ease-out]">
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
  "inline-flex w-fit max-w-full flex-wrap items-center gap-1 rounded-lg bg-surface-sunken p-1 shadow-card";

export function segmentedItemClass(active: boolean): string {
  return `rounded-md px-3 py-1.5 text-xs font-medium transition focus:outline-none focus-visible:ring-2 focus-visible:ring-brand-500 ${
    active
      ? "bg-brand-600 text-white"
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
    <div className={`${SEGMENTED_TRACK} ${className}`}>
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

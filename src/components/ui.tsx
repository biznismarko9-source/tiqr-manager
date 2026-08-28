import { useEffect, type ButtonHTMLAttributes, type InputHTMLAttributes, type ReactNode, type SelectHTMLAttributes, type TextareaHTMLAttributes } from "react";
import { IconAlertTriangle, IconChevronDown, IconTrendingDown, IconTrendingUp, IconX } from "./icons";
import type { TrendInfo } from "../lib/format";

// ---------------------------------------------------------------------------
// Buttons
// ---------------------------------------------------------------------------
type ButtonVariant = "primary" | "secondary" | "danger" | "ghost";

export function Button({
  variant = "secondary",
  className = "",
  children,
  ...rest
}: ButtonHTMLAttributes<HTMLButtonElement> & { variant?: ButtonVariant }) {
  const base =
    "inline-flex items-center justify-center gap-1.5 rounded-lg px-3.5 py-2 text-sm font-medium transition-colors disabled:opacity-50 disabled:pointer-events-none focus:outline-none focus:ring-2 focus:ring-offset-1";
  const variants: Record<ButtonVariant, string> = {
    primary: "bg-brand-600 text-white hover:bg-brand-700 shadow-sm focus:ring-brand-300",
    secondary:
      "bg-white text-slate-700 border border-slate-300 hover:bg-slate-50 shadow-sm focus:ring-slate-200 dark:bg-slate-900 dark:text-slate-300 dark:border-slate-700 dark:hover:bg-slate-800 dark:focus:ring-slate-700",
    danger: "bg-red-600 text-white hover:bg-red-700 shadow-sm focus:ring-red-300 dark:focus:ring-red-900",
    ghost: "text-slate-600 hover:bg-slate-100 focus:ring-slate-200 dark:text-slate-400 dark:hover:bg-slate-800 dark:focus:ring-slate-700",
  };
  return (
    <button className={`${base} ${variants[variant]} ${className}`} {...rest}>
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
      <IconChevronDown className="pointer-events-none absolute right-3 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-400 dark:text-slate-500" />
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
export const CHECKBOX_CLASS =
  "h-4 w-4 rounded border-slate-300 text-brand-600 focus:ring-2 focus:ring-brand-200 dark:border-slate-600 dark:bg-slate-800 dark:focus:ring-brand-900";

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
    <div className="mb-4 flex items-center gap-3 rounded-lg bg-red-50 dark:bg-red-500/10 px-4 py-2.5 text-sm ring-1 ring-inset ring-red-200 dark:ring-red-500/30">
      <span className="font-medium text-red-800 dark:text-red-300">
        {count === 0 ? `Select ${itemLabel}s to delete` : `Selected: ${count} ${itemLabel}${count === 1 ? "" : "s"}`}
      </span>
      <Button variant="danger" onClick={onConfirm} disabled={busy || count === 0}>
        Delete selected
      </Button>
      <button
        type="button"
        className="ml-auto text-xs font-medium text-red-700 dark:text-red-400 hover:underline disabled:opacity-50"
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
  return (
    <label className="block">
      <span className="label">
        {label}
        {required && <span className="text-red-500"> *</span>}
      </span>
      {children}
      {hint && !error && <span className="mt-1 block text-xs text-slate-400 dark:text-slate-500">{hint}</span>}
      {error && <span className="mt-1 block text-xs text-red-600 dark:text-red-400">{error}</span>}
    </label>
  );
}

// ---------------------------------------------------------------------------
// Layout bits
// ---------------------------------------------------------------------------
export function Card({ children, className = "" }: { children: ReactNode; className?: string }) {
  return <div className={`card ${className}`}>{children}</div>;
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
  return (
    <div className="mb-5 flex flex-wrap items-start justify-between gap-3">
      <div>
        <h1 className="text-xl font-semibold text-slate-900 dark:text-slate-100">{title}</h1>
        {subtitle && <p className="mt-0.5 text-sm text-slate-500 dark:text-slate-400">{subtitle}</p>}
      </div>
      {actions && <div className="flex flex-wrap items-center gap-2">{actions}</div>}
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

export function LoadingBlock({ label = "Loading..." }: { label?: string }) {
  return (
    <div className="flex items-center justify-center gap-2 py-16 text-sm text-slate-400 dark:text-slate-500">
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
  return (
    <div className="flex flex-col items-center justify-center gap-2 rounded-xl border border-dashed border-slate-300 bg-slate-50/60 py-16 px-6 text-center dark:border-slate-700 dark:bg-slate-900/40">
      {icon && <div className="mb-1 text-slate-300 dark:text-slate-600">{icon}</div>}
      <p className="text-sm font-medium text-slate-700 dark:text-slate-300">{title}</p>
      {description && <p className="max-w-sm text-sm text-slate-400 dark:text-slate-500">{description}</p>}
      {action && <div className="mt-2">{action}</div>}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Badge
// ---------------------------------------------------------------------------
const STATUS_TONES: Record<string, string> = {
  available: "bg-slate-100 text-slate-700 ring-slate-300 dark:bg-slate-800 dark:text-slate-300 dark:ring-slate-600",
  listed: "bg-blue-50 text-blue-700 ring-blue-200 dark:bg-blue-500/10 dark:text-blue-400 dark:ring-blue-500/30",
  sold: "bg-emerald-50 text-emerald-700 ring-emerald-200 dark:bg-emerald-500/10 dark:text-emerald-400 dark:ring-emerald-500/30",
  cancelled: "bg-red-50 text-red-700 ring-red-200 dark:bg-red-500/10 dark:text-red-400 dark:ring-red-500/30",
  upcoming: "bg-blue-50 text-blue-700 ring-blue-200 dark:bg-blue-500/10 dark:text-blue-400 dark:ring-blue-500/30",
  completed: "bg-emerald-50 text-emerald-700 ring-emerald-200 dark:bg-emerald-500/10 dark:text-emerald-400 dark:ring-emerald-500/30",
  unpaid: "bg-amber-50 text-amber-700 ring-amber-200 dark:bg-amber-500/10 dark:text-amber-400 dark:ring-amber-500/30",
  partial: "bg-blue-50 text-blue-700 ring-blue-200 dark:bg-blue-500/10 dark:text-blue-400 dark:ring-blue-500/30",
  paid: "bg-emerald-50 text-emerald-700 ring-emerald-200 dark:bg-emerald-500/10 dark:text-emerald-400 dark:ring-emerald-500/30",
  pending: "bg-amber-50 text-amber-700 ring-amber-200 dark:bg-amber-500/10 dark:text-amber-400 dark:ring-amber-500/30",
  refunded: "bg-red-50 text-red-700 ring-red-200 dark:bg-red-500/10 dark:text-red-400 dark:ring-red-500/30",
  demo: "bg-violet-50 text-violet-700 ring-violet-200 dark:bg-violet-500/10 dark:text-violet-400 dark:ring-violet-500/30",
  // Order-inventory status (derived client-side from ticket counts, not a DB column).
  active: "bg-emerald-50 text-emerald-700 ring-emerald-200 dark:bg-emerald-500/10 dark:text-emerald-400 dark:ring-emerald-500/30",
  soldout: "bg-slate-100 text-slate-700 ring-slate-300 dark:bg-slate-800 dark:text-slate-300 dark:ring-slate-600",
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
  unlisted: "bg-slate-100 text-slate-700 ring-slate-300 dark:bg-slate-800 dark:text-slate-300 dark:ring-slate-600",
  delivered: "bg-emerald-50 text-emerald-700 ring-emerald-200 dark:bg-emerald-500/10 dark:text-emerald-400 dark:ring-emerald-500/30",
  "not delivered": "bg-amber-50 text-amber-700 ring-amber-200 dark:bg-amber-500/10 dark:text-amber-400 dark:ring-amber-500/30",
  // Shown when a grouped sale's lines don't all share one value (e.g. one
  // ticket in a batch was refunded while the rest weren't).
  mixed: "bg-amber-50 text-amber-700 ring-amber-200 dark:bg-amber-500/10 dark:text-amber-400 dark:ring-amber-500/30",
};

export function Badge({ tone, title, children }: { tone: string; title?: string; children: ReactNode }) {
  const cls = STATUS_TONES[tone] ?? "bg-slate-100 text-slate-700 ring-slate-300 dark:bg-slate-800 dark:text-slate-300 dark:ring-slate-600";
  return (
    <span
      title={title}
      className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium capitalize ring-1 ring-inset ${cls}`}
    >
      {children}
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
  const valueTone =
    tone === "positive"
      ? "text-emerald-600 dark:text-emerald-400"
      : tone === "negative"
        ? "text-red-600 dark:text-red-400"
        : "text-slate-900 dark:text-slate-100";
  const trendTone =
    !trend || !trendColored || trend.direction === "flat"
      ? "text-slate-400 dark:text-slate-500"
      : trend.direction === "up"
        ? "text-emerald-600 dark:text-emerald-400"
        : "text-red-600 dark:text-red-400";
  return (
    <Card className="p-4">
      <p className="text-xs font-medium uppercase tracking-wide text-slate-400 dark:text-slate-500">{label}</p>
      <p className={`mt-1.5 text-2xl font-semibold tabular-nums ${valueTone}`}>{value}</p>
      {trend && (
        <p className={`mt-1 flex items-center gap-1 text-xs font-medium ${trendTone}`}>
          {trend.direction === "up" && <IconTrendingUp className="h-3 w-3 shrink-0" />}
          {trend.direction === "down" && <IconTrendingDown className="h-3 w-3 shrink-0" />}
          {trend.label} <span className="font-normal text-slate-400 dark:text-slate-500">vs. previous period</span>
        </p>
      )}
      {sub && <p className="mt-1 text-xs text-slate-400 dark:text-slate-500">{sub}</p>}
    </Card>
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
    <div className="fixed inset-0 z-50 flex items-start justify-center overflow-y-auto bg-slate-900/40 p-4 pt-[8vh] backdrop-blur-[1px] dark:bg-black/60">
      <div className={`w-full ${width} rounded-xl bg-white shadow-xl dark:bg-slate-900`}>
        <div className="flex items-center justify-between border-b border-slate-200 px-5 py-3.5 dark:border-slate-800">
          <h2 className="text-base font-semibold text-slate-900 dark:text-slate-100">{title}</h2>
          <button
            onClick={onClose}
            className="rounded-md p-1 text-slate-400 hover:bg-slate-100 hover:text-slate-600 dark:text-slate-500 dark:hover:bg-slate-800 dark:hover:text-slate-300"
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
  return <div className="mt-5 flex justify-end gap-2 border-t border-slate-100 pt-4 dark:border-slate-800">{children}</div>;
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
    <div className="fixed inset-0 z-[60] flex items-center justify-center bg-slate-900/40 p-4 backdrop-blur-[1px] dark:bg-black/60">
      <div className="w-full max-w-sm rounded-xl bg-white p-5 shadow-xl dark:bg-slate-900">
        <div className="flex gap-3">
          <div
            className={`flex h-9 w-9 shrink-0 items-center justify-center rounded-full ${danger ? "bg-red-100 text-red-600 dark:bg-red-500/10 dark:text-red-400" : "bg-amber-100 text-amber-600 dark:bg-amber-500/10 dark:text-amber-400"}`}
          >
            <IconAlertTriangle className="h-5 w-5" />
          </div>
          <div>
            <h3 className="text-sm font-semibold text-slate-900 dark:text-slate-100">{title}</h3>
            <div className="mt-1 text-sm text-slate-500 dark:text-slate-400">{message}</div>
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

/** 2.0.59: shared "Active vs Completed" pill switcher for Events/Orders/
 * Tickets/Sales - same visual pattern (and exact same classNames) as
 * Dashboard.tsx's own Overview/Financials/Activity tab row, extracted here
 * so four pages don't each hand-roll their own copy. Dashboard's own tab row
 * is untouched (it has 3 tabs, not 2, and already shipped/works) - this is
 * for the new pages only, paired with lib/useListTab.ts for the
 * load/persist half of the pattern. */
export function TabSwitcher<T extends string>({
  tabs,
  active,
  onChange,
}: {
  tabs: { key: T; label: string }[];
  active: T;
  onChange: (key: T) => void;
}) {
  return (
    <div className="mb-4 flex w-fit flex-wrap items-center gap-1 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-1">
      {tabs.map((t) => (
        <button
          key={t.key}
          type="button"
          onClick={() => onChange(t.key)}
          className={`rounded-md px-2.5 py-1.5 text-xs font-medium transition-colors ${
            active === t.key
              ? "bg-brand-600 text-white"
              : "text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800"
          }`}
        >
          {t.label}
        </button>
      ))}
    </div>
  );
}

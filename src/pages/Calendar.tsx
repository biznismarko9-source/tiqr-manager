import { useEffect, useMemo, useState, type SVGProps } from "react";
import { useNavigate } from "react-router-dom";
import { api } from "../lib/api";
import type { CalendarEntry, CalendarEntryKind, CalendarSeverity } from "../lib/types";
import { formatDate, formatDateNumeric, formatMoney, todayIso } from "../lib/format";
import { Card, LoadingBlock, Modal, PageHeader, TabSwitcher } from "../components/ui";
import {
  IconAlertTriangle,
  IconCalendarDays,
  IconChevronLeft,
  IconChevronRight,
  IconPackage,
  IconReceipt,
  IconUsers,
} from "../components/icons";

// 2.5.0: "TIQR Operations Calendar" - marko's own request for one Month/Week
// view over every part of the app that has a real date, instead of five
// separate places to go check what's happening when. See
// commands/calendar.rs's own module doc comment for the full research behind
// which 5 categories are real (event/order/sale/pull/attention) and which 3
// of marko's original 8 candidates are NOT (payouts/payments/fulfillment -
// none of those has a real, reliably-existing date anywhere in this app, so
// none of them is invented here either).
//
// Every entry already comes from `get_calendar` fully formed (title,
// subtitle, severity, navigation target) - this page is a grid/list
// renderer, it never computes a business fact of its own. Navigation reuses
// the exact same routes/pages Attention Center, Ticket Control Center and
// Fulfillment Center already send the user to for the same underlying
// records - no new detail view exists (or is needed) for any of this.

const KIND_META: Record<CalendarEntryKind, { label: string; icon: (p: SVGProps<SVGSVGElement>) => JSX.Element }> = {
  event: { label: "Events", icon: IconCalendarDays },
  order: { label: "Orders", icon: IconPackage },
  sale: { label: "Sales", icon: IconReceipt },
  pull: { label: "Pulls", icon: IconUsers },
  attention: { label: "Attention", icon: IconAlertTriangle },
};
const ALL_KINDS: CalendarEntryKind[] = ["event", "order", "sale", "pull", "attention"];

const SEVERITY_DOT: Record<CalendarSeverity, string> = {
  critical: "bg-red-500",
  attention: "bg-amber-500",
  info: "bg-blue-500",
  neutral: "bg-slate-300 dark:bg-slate-600",
};

const WEEKDAY_LABELS = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
const MONTH_CELL_CAP = 3;
const WEEK_CELL_CAP = 6;

// ---------------------------------------------------------------------------
// Plain calendar-day math - Monday-first weeks, no time-of-day component
// anywhere (matches this app's own `event_date`/`purchase_date`/`sale_date`
// columns, which are all plain "YYYY-MM-DD" with no time either).
// ---------------------------------------------------------------------------
function pad2(n: number): string {
  return String(n).padStart(2, "0");
}
function isoOf(d: Date): string {
  return `${d.getFullYear()}-${pad2(d.getMonth() + 1)}-${pad2(d.getDate())}`;
}
function addDays(d: Date, n: number): Date {
  const r = new Date(d);
  r.setDate(r.getDate() + n);
  return r;
}
function startOfDay(d: Date): Date {
  const r = new Date(d);
  r.setHours(0, 0, 0, 0);
  return r;
}
function startOfWeek(d: Date): Date {
  const day = d.getDay(); // 0 = Sunday .. 6 = Saturday
  const diff = day === 0 ? -6 : 1 - day; // shift so Monday is the start
  return addDays(startOfDay(d), diff);
}
function monthGridRange(anchor: Date): { start: Date; end: Date } {
  const start = startOfWeek(new Date(anchor.getFullYear(), anchor.getMonth(), 1));
  const lastDayOfMonth = new Date(anchor.getFullYear(), anchor.getMonth() + 1, 0);
  const end = addDays(startOfWeek(lastDayOfMonth), 6);
  return { start, end };
}
function weekGridRange(anchor: Date): { start: Date; end: Date } {
  const start = startOfWeek(anchor);
  return { start, end: addDays(start, 6) };
}
function daysBetweenInclusive(start: Date, end: Date): Date[] {
  const days: Date[] = [];
  for (let d = start; d.getTime() <= end.getTime(); d = addDays(d, 1)) days.push(d);
  return days;
}

function navigateToEntry(navigate: ReturnType<typeof useNavigate>, entry: CalendarEntry) {
  switch (entry.linkKind) {
    case "event":
      navigate(`/events/${entry.linkId}`);
      break;
    case "order":
      navigate(`/orders/${entry.linkId}`);
      break;
    case "sale":
      navigate(`/sales/${entry.linkId}`);
      break;
    case "pulls":
      navigate("/pulls");
      break;
  }
}

export default function Calendar() {
  const navigate = useNavigate();
  const [viewMode, setViewMode] = useState<"month" | "week">("month");
  const [anchor, setAnchor] = useState<Date>(() => startOfDay(new Date()));
  const [entries, setEntries] = useState<CalendarEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [activeKinds, setActiveKinds] = useState<Set<CalendarEntryKind>>(() => new Set(ALL_KINDS));
  const [dayDetail, setDayDetail] = useState<string | null>(null);

  const { start: gridStart, end: gridEnd } = viewMode === "month" ? monthGridRange(anchor) : weekGridRange(anchor);
  const dateFrom = isoOf(gridStart);
  const dateTo = isoOf(gridEnd);
  const gridDays = useMemo(() => daysBetweenInclusive(gridStart, gridEnd), [dateFrom, dateTo]);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    api
      .getCalendar({ dateFrom, dateTo })
      .then((data) => {
        if (!cancelled) setEntries(data);
      })
      .catch(() => {
        if (!cancelled) setError("Couldn't load the calendar - try again.");
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [dateFrom, dateTo]);

  const visibleEntries = useMemo(() => entries.filter((e) => activeKinds.has(e.kind)), [entries, activeKinds]);
  const entriesByDate = useMemo(() => {
    const map = new Map<string, CalendarEntry[]>();
    for (const e of visibleEntries) {
      const list = map.get(e.date);
      if (list) list.push(e);
      else map.set(e.date, [e]);
    }
    return map;
  }, [visibleEntries]);

  const today = isoOf(new Date());
  const cellCap = viewMode === "month" ? MONTH_CELL_CAP : WEEK_CELL_CAP;

  const rangeLabel =
    viewMode === "month"
      ? anchor.toLocaleDateString(undefined, { month: "long", year: "numeric" })
      : `${formatDateNumeric(isoOf(gridStart))} - ${formatDateNumeric(isoOf(gridEnd))}`;

  const goToday = () => setAnchor(startOfDay(new Date()));
  const goPrev = () => setAnchor((a) => (viewMode === "week" ? addDays(a, -7) : new Date(a.getFullYear(), a.getMonth() - 1, 1)));
  const goNext = () => setAnchor((a) => (viewMode === "week" ? addDays(a, 7) : new Date(a.getFullYear(), a.getMonth() + 1, 1)));

  const toggleKind = (kind: CalendarEntryKind) => {
    setActiveKinds((prev) => {
      const next = new Set(prev);
      if (next.has(kind)) next.delete(kind);
      else next.add(kind);
      return next;
    });
  };

  return (
    <div>
      <PageHeader
        title="Calendar"
        subtitle="Every event, order, sale, pull, and attention item that has a real date, in one place."
        actions={
          <div className="flex flex-wrap items-center gap-2">
            <TabSwitcher tabs={[{ key: "month", label: "Month" }, { key: "week", label: "Week" }]} active={viewMode} onChange={setViewMode} />
            <div className="flex items-center gap-1 rounded-lg border border-slate-200 bg-white p-1 dark:border-slate-800 dark:bg-slate-900">
              <button
                type="button"
                onClick={goPrev}
                aria-label="Previous"
                className="rounded-md p-1.5 text-slate-500 hover:bg-slate-100 hover:text-slate-900 dark:text-slate-400 dark:hover:bg-slate-800 dark:hover:text-slate-100"
              >
                <IconChevronLeft className="h-4 w-4" />
              </button>
              <button
                type="button"
                onClick={goToday}
                className="rounded-md px-2.5 py-1 text-xs font-medium text-slate-600 hover:bg-slate-100 hover:text-slate-900 dark:text-slate-400 dark:hover:bg-slate-800 dark:hover:text-slate-100"
              >
                Today
              </button>
              <button
                type="button"
                onClick={goNext}
                aria-label="Next"
                className="rounded-md p-1.5 text-slate-500 hover:bg-slate-100 hover:text-slate-900 dark:text-slate-400 dark:hover:bg-slate-800 dark:hover:text-slate-100"
              >
                <IconChevronRight className="h-4 w-4" />
              </button>
            </div>
          </div>
        }
      />

      <UpcomingSummary />

      <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
        <p className="text-sm font-semibold text-slate-900 dark:text-slate-100">{rangeLabel}</p>
        <div className="flex flex-wrap items-center gap-1.5">
          {ALL_KINDS.map((kind) => {
            const meta = KIND_META[kind];
            const active = activeKinds.has(kind);
            return (
              <button
                key={kind}
                type="button"
                onClick={() => toggleKind(kind)}
                className={`inline-flex items-center gap-1.5 rounded-full px-2.5 py-1 text-xs font-medium ring-1 ring-inset transition-colors ${
                  active
                    ? "bg-brand-50 text-brand-700 ring-brand-200 dark:bg-brand-500/10 dark:text-brand-400 dark:ring-brand-500/30"
                    : "bg-white text-slate-400 ring-slate-200 dark:bg-slate-900 dark:text-slate-500 dark:ring-slate-800"
                }`}
              >
                <meta.icon className="h-3.5 w-3.5" />
                {meta.label}
              </button>
            );
          })}
          {activeKinds.size < ALL_KINDS.length && (
            <button
              type="button"
              onClick={() => setActiveKinds(new Set(ALL_KINDS))}
              className="text-xs font-medium text-slate-400 hover:text-slate-600 hover:underline dark:text-slate-500 dark:hover:text-slate-300"
            >
              Reset
            </button>
          )}
        </div>
      </div>

      {error && <p className="mb-3 text-sm text-red-600 dark:text-red-400">{error}</p>}

      <Card className="overflow-hidden p-0">
        <div className="grid grid-cols-7 border-b border-slate-200 dark:border-slate-800">
          {WEEKDAY_LABELS.map((label) => (
            <div key={label} className="px-2 py-2 text-center text-[11px] font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">
              {label}
            </div>
          ))}
        </div>
        {loading ? (
          <LoadingBlock label="Loading calendar..." />
        ) : (
          <div className="grid grid-cols-7">
            {gridDays.map((day) => {
              const iso = isoOf(day);
              const dayEntries = entriesByDate.get(iso) ?? [];
              const inCurrentMonth = viewMode === "week" || day.getMonth() === anchor.getMonth();
              const isToday = iso === today;
              return (
                <div
                  key={iso}
                  className={`flex flex-col gap-1 border-b border-r border-slate-100 p-1.5 last:border-r-0 dark:border-slate-800/60 ${
                    viewMode === "week" ? "min-h-[220px]" : "min-h-[96px]"
                  } ${inCurrentMonth ? "" : "bg-slate-50/60 dark:bg-slate-900/40"}`}
                >
                  <button
                    type="button"
                    onClick={() => dayEntries.length > 0 && setDayDetail(iso)}
                    disabled={dayEntries.length === 0}
                    className={`flex h-5 w-5 shrink-0 items-center justify-center self-start rounded-full text-xs font-medium ${
                      isToday
                        ? "bg-brand-600 text-white"
                        : inCurrentMonth
                          ? "text-slate-600 dark:text-slate-300"
                          : "text-slate-300 dark:text-slate-600"
                    } ${dayEntries.length > 0 ? "cursor-pointer hover:opacity-80" : "cursor-default"}`}
                  >
                    {day.getDate()}
                  </button>
                  <div className="flex flex-1 flex-col gap-0.5 overflow-hidden">
                    {dayEntries.slice(0, cellCap).map((entry) => (
                      <button
                        key={entry.key}
                        type="button"
                        onClick={() => navigateToEntry(navigate, entry)}
                        title={`${entry.title}${entry.subtitle ? ` - ${entry.subtitle}` : ""}`}
                        className="flex items-center gap-1 truncate rounded px-1 py-0.5 text-left text-[11px] text-slate-600 hover:bg-slate-100 dark:text-slate-300 dark:hover:bg-slate-800"
                      >
                        <span className={`h-1.5 w-1.5 shrink-0 rounded-full ${SEVERITY_DOT[entry.severity]}`} />
                        <span className="truncate">{entry.title}</span>
                      </button>
                    ))}
                    {dayEntries.length > cellCap && (
                      <button
                        type="button"
                        onClick={() => setDayDetail(iso)}
                        className="px-1 text-left text-[11px] font-medium text-brand-600 hover:underline dark:text-brand-400"
                      >
                        +{dayEntries.length - cellCap} more
                      </button>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </Card>

      <DayDetailModal iso={dayDetail} entries={dayDetail ? (entriesByDate.get(dayDetail) ?? []) : []} onClose={() => setDayDetail(null)} onNavigate={(e) => navigateToEntry(navigate, e)} />
    </div>
  );
}

function DayDetailModal({
  iso,
  entries,
  onClose,
  onNavigate,
}: {
  iso: string | null;
  entries: CalendarEntry[];
  onClose: () => void;
  onNavigate: (entry: CalendarEntry) => void;
}) {
  return (
    <Modal open={iso !== null} onClose={onClose} title={iso ? formatDate(iso) : ""}>
      {entries.length === 0 ? (
        <p className="py-6 text-center text-sm text-slate-400 dark:text-slate-500">Nothing here.</p>
      ) : (
        <ul className="divide-y divide-slate-100 dark:divide-slate-800">
          {entries.map((entry) => {
            const meta = KIND_META[entry.kind];
            return (
              <li key={entry.key}>
                <button
                  type="button"
                  onClick={() => onNavigate(entry)}
                  className="flex w-full items-start gap-2.5 py-2.5 text-left hover:bg-slate-50 dark:hover:bg-slate-800/60"
                >
                  <span className={`mt-1.5 h-2 w-2 shrink-0 rounded-full ${SEVERITY_DOT[entry.severity]}`} />
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-1.5">
                      <meta.icon className="h-3.5 w-3.5 shrink-0 text-slate-400 dark:text-slate-500" />
                      <p className="truncate text-sm font-medium text-slate-900 dark:text-slate-100">{entry.title}</p>
                    </div>
                    {entry.subtitle && <p className="mt-0.5 truncate text-xs text-slate-500 dark:text-slate-400">{entry.subtitle}</p>}
                  </div>
                  {entry.amountCents !== null && entry.currency && (
                    <span className="shrink-0 text-xs font-medium tabular-nums text-slate-600 dark:text-slate-300">
                      {formatMoney(entry.amountCents, entry.currency)}
                    </span>
                  )}
                </button>
              </li>
            );
          })}
        </ul>
      )}
    </Modal>
  );
}

/** Today + next 7 days, fetched independently of whatever Month/Week range
 * is currently on screen - same `get_calendar` command, just a different
 * range, so this is never a second, duplicate business computation (marko's
 * own explicit requirement). */
function UpcomingSummary() {
  const navigate = useNavigate();
  const [entries, setEntries] = useState<CalendarEntry[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const from = todayIso();
    const to = isoOf(addDays(new Date(), 6));
    api
      .getCalendar({ dateFrom: from, dateTo: to })
      .then(setEntries)
      .catch(() => setEntries([]))
      .finally(() => setLoading(false));
  }, []);

  if (loading) return null;

  const counts = ALL_KINDS.map((kind) => ({ kind, count: entries.filter((e) => e.kind === kind).length })).filter((c) => c.count > 0);
  const upNext = [...entries]
    .sort((a, b) => {
      const rank: Record<CalendarSeverity, number> = { critical: 0, attention: 1, info: 2, neutral: 3 };
      return rank[a.severity] - rank[b.severity] || a.date.localeCompare(b.date);
    })
    .slice(0, 4);

  return (
    <Card className="mb-4 p-4">
      <p className="mb-2 text-xs font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">Today &amp; next 7 days</p>
      {entries.length === 0 ? (
        <p className="text-sm text-slate-400 dark:text-slate-500">Nothing coming up in the next 7 days.</p>
      ) : (
        <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:gap-6">
          <div className="flex flex-wrap gap-2">
            {counts.map(({ kind, count }) => {
              const meta = KIND_META[kind];
              return (
                <span
                  key={kind}
                  className="inline-flex items-center gap-1.5 rounded-full bg-slate-100 px-2.5 py-1 text-xs font-medium text-slate-600 dark:bg-slate-800 dark:text-slate-300"
                >
                  <meta.icon className="h-3.5 w-3.5" />
                  {count} {meta.label.toLowerCase()}
                </span>
              );
            })}
          </div>
          <div className="min-w-0 flex-1 border-t border-slate-100 pt-2 sm:border-l sm:border-t-0 sm:pl-6 sm:pt-0 dark:border-slate-800">
            <ul className="flex flex-col gap-1.5">
              {upNext.map((entry) => (
                <li key={entry.key}>
                  <button
                    type="button"
                    onClick={() => navigateToEntry(navigate, entry)}
                    className="flex w-full items-center gap-2 truncate text-left text-xs text-slate-600 hover:text-brand-700 dark:text-slate-300 dark:hover:text-brand-400"
                  >
                    <span className={`h-1.5 w-1.5 shrink-0 rounded-full ${SEVERITY_DOT[entry.severity]}`} />
                    <span className="shrink-0 tabular-nums text-slate-400 dark:text-slate-500">{formatDateNumeric(entry.date)}</span>
                    <span className="truncate">{entry.title}</span>
                  </button>
                </li>
              ))}
            </ul>
          </div>
        </div>
      )}
    </Card>
  );
}

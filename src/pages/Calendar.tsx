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
//
// 2.5.1: marko's own follow-up - "viac moderny, viac prehladny" (more
// modern, more legible). Purely a presentation pass: every hook, API call,
// and navigation target above is untouched. The one real legibility change
// is giving each entry KIND (event/order/sale/pull/attention) its own
// consistent color - same "assign a color per category" idea
// EventCategoryBadge/FinanceCategoryBadge already use elsewhere in this app,
// just applied to the calendar's own 5 fixed categories instead of a
// user-defined list. Severity (critical/attention/info/neutral) used to be
// the ONLY visual signal (a plain dot); it's now a second, independent
// channel layered on top (a ring on critical/attention chips, colored text
// in the list views) so a busy day reads as "5 sales, 1 critical order" at a
// glance instead of 6 identical gray dots. The kind-toggle row doubles as
// the color legend, so no separate legend UI was needed.

const KIND_META: Record<CalendarEntryKind, { label: string; icon: (p: SVGProps<SVGSVGElement>) => JSX.Element }> = {
  event: { label: "Events", icon: IconCalendarDays },
  order: { label: "Orders", icon: IconPackage },
  sale: { label: "Sales", icon: IconReceipt },
  pull: { label: "Pulls", icon: IconUsers },
  attention: { label: "Attention", icon: IconAlertTriangle },
};
const ALL_KINDS: CalendarEntryKind[] = ["event", "order", "sale", "pull", "attention"];

// 2.5.1: one consistent accent color per KIND - the calendar's own "category
// palette", same spirit as EventCategoryBadge's colorSlot but a fixed set of
// 5 rather than user-configurable. Chosen to stay clear of the severity
// palette below (red/amber/blue) so the two channels never look like the
// same signal: indigo/sky/emerald/violet for event/order/sale/pull, and
// attention keeps its own amber since that category IS a severity signal by
// definition.
const KIND_ACCENT: Record<CalendarEntryKind, { dot: string; chip: string; text: string; legend: string }> = {
  event: {
    dot: "bg-indigo-500",
    chip: "bg-indigo-50 text-indigo-700 dark:bg-indigo-500/10 dark:text-indigo-300",
    text: "text-indigo-700 dark:text-indigo-300",
    legend: "bg-indigo-50 text-indigo-700 ring-indigo-200 dark:bg-indigo-500/10 dark:text-indigo-300 dark:ring-indigo-500/30",
  },
  order: {
    dot: "bg-sky-500",
    chip: "bg-sky-50 text-sky-700 dark:bg-sky-500/10 dark:text-sky-300",
    text: "text-sky-700 dark:text-sky-300",
    legend: "bg-sky-50 text-sky-700 ring-sky-200 dark:bg-sky-500/10 dark:text-sky-300 dark:ring-sky-500/30",
  },
  sale: {
    dot: "bg-emerald-500",
    chip: "bg-emerald-50 text-emerald-700 dark:bg-emerald-500/10 dark:text-emerald-300",
    text: "text-emerald-700 dark:text-emerald-300",
    legend: "bg-emerald-50 text-emerald-700 ring-emerald-200 dark:bg-emerald-500/10 dark:text-emerald-300 dark:ring-emerald-500/30",
  },
  pull: {
    dot: "bg-violet-500",
    chip: "bg-violet-50 text-violet-700 dark:bg-violet-500/10 dark:text-violet-300",
    text: "text-violet-700 dark:text-violet-300",
    legend: "bg-violet-50 text-violet-700 ring-violet-200 dark:bg-violet-500/10 dark:text-violet-300 dark:ring-violet-500/30",
  },
  attention: {
    dot: "bg-amber-500",
    chip: "bg-amber-50 text-amber-700 dark:bg-amber-500/10 dark:text-amber-300",
    text: "text-amber-700 dark:text-amber-300",
    legend: "bg-amber-50 text-amber-700 ring-amber-200 dark:bg-amber-500/10 dark:text-amber-300 dark:ring-amber-500/30",
  },
};
// 2.5.1: severity's own channel, now just an emphasis ring layered on top of
// a kind-colored chip (month/week grid) or a text-color override (list
// views, where there's no chip background to ring). `info`/`neutral` add no
// extra emphasis - the kind color alone is enough signal for a routine item.
const SEVERITY_RING: Record<CalendarSeverity, string> = {
  critical: "ring-2 ring-inset ring-red-400 dark:ring-red-500/70",
  attention: "ring-1 ring-inset ring-amber-400 dark:ring-amber-500/60",
  info: "",
  neutral: "",
};
const SEVERITY_TEXT: Record<CalendarSeverity, string> = {
  critical: "text-red-600 dark:text-red-400 font-semibold",
  attention: "text-amber-700 dark:text-amber-400 font-medium",
  info: "",
  neutral: "",
};
const LEGEND_INACTIVE = "bg-white text-slate-400 ring-slate-200 dark:bg-slate-900 dark:text-slate-500 dark:ring-slate-800";

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
// 2.5.1: Sat/Sun get a faint background wash in both the weekday header and
// the grid body - a common, quick "which of these are weekend dates" cue for
// a ticket-event calendar, purely visual (native getDay(), independent of
// the Monday-first LAYOUT above).
function isWeekend(d: Date): boolean {
  const day = d.getDay();
  return day === 0 || day === 6;
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
        <p className="text-base font-semibold text-slate-900 dark:text-slate-100">{rangeLabel}</p>
        {/* 2.5.1: this row is both the kind filter AND the calendar's color
            legend - each pill's dot is the exact color its entries use below,
            so there's no separate "what does this color mean" key to add. */}
        <div className="flex flex-wrap items-center gap-1.5">
          {ALL_KINDS.map((kind) => {
            const meta = KIND_META[kind];
            const accent = KIND_ACCENT[kind];
            const active = activeKinds.has(kind);
            return (
              <button
                key={kind}
                type="button"
                onClick={() => toggleKind(kind)}
                aria-pressed={active}
                className={`inline-flex items-center gap-1.5 rounded-full px-2.5 py-1 text-xs font-medium ring-1 ring-inset transition-colors ${
                  active ? accent.legend : LEGEND_INACTIVE
                }`}
              >
                <span className={`h-2 w-2 shrink-0 rounded-full ${active ? accent.dot : "bg-slate-300 dark:bg-slate-600"}`} />
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
          {WEEKDAY_LABELS.map((label, i) => (
            <div
              key={label}
              className={`px-2 py-2.5 text-center text-[11px] font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500 ${
                i >= 5 ? "bg-slate-50/70 dark:bg-slate-900/40" : ""
              }`}
            >
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
              const weekend = isWeekend(day);
              return (
                <div
                  key={iso}
                  className={`flex flex-col gap-1 border-b border-r border-slate-100 p-2 transition-colors last:border-r-0 dark:border-slate-800/60 ${
                    viewMode === "week" ? "min-h-[220px]" : "min-h-[96px]"
                  } ${
                    isToday
                      ? "bg-brand-50/50 dark:bg-brand-500/[0.06]"
                      : !inCurrentMonth
                        ? "bg-slate-50/60 dark:bg-slate-900/40"
                        : weekend
                          ? "bg-slate-50/40 dark:bg-slate-900/20"
                          : ""
                  }`}
                >
                  <div className="flex items-center justify-between">
                    <button
                      type="button"
                      onClick={() => dayEntries.length > 0 && setDayDetail(iso)}
                      disabled={dayEntries.length === 0}
                      className={`flex h-6 w-6 shrink-0 items-center justify-center rounded-full text-xs font-semibold transition-colors ${
                        isToday
                          ? "bg-brand-600 text-white shadow-sm"
                          : inCurrentMonth
                            ? "text-slate-600 dark:text-slate-300"
                            : "text-slate-300 dark:text-slate-600"
                      } ${dayEntries.length > 0 && !isToday ? "cursor-pointer hover:bg-slate-200/70 dark:hover:bg-slate-700/60" : dayEntries.length > 0 ? "cursor-pointer" : "cursor-default"}`}
                    >
                      {day.getDate()}
                    </button>
                    {dayEntries.length > 0 && (
                      <span className="text-[10px] font-medium tabular-nums text-slate-300 dark:text-slate-600">{dayEntries.length}</span>
                    )}
                  </div>
                  <div className="flex flex-1 flex-col gap-1 overflow-hidden">
                    {dayEntries.slice(0, cellCap).map((entry) => {
                      const accent = KIND_ACCENT[entry.kind];
                      return (
                        <button
                          key={entry.key}
                          type="button"
                          onClick={() => navigateToEntry(navigate, entry)}
                          title={`${entry.title}${entry.subtitle ? ` - ${entry.subtitle}` : ""}`}
                          className={`flex items-center gap-1 truncate rounded-md px-1.5 py-0.5 text-left text-[11px] font-medium transition-colors hover:brightness-95 dark:hover:brightness-125 ${accent.chip} ${SEVERITY_RING[entry.severity]}`}
                        >
                          <span className="truncate">{entry.title}</span>
                        </button>
                      );
                    })}
                    {dayEntries.length > cellCap && (
                      <button
                        type="button"
                        onClick={() => setDayDetail(iso)}
                        className="rounded-md px-1.5 py-0.5 text-left text-[11px] font-semibold text-slate-500 hover:bg-slate-100 hover:text-slate-700 dark:text-slate-400 dark:hover:bg-slate-800 dark:hover:text-slate-200"
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
  // 2.5.1: "Monday, Aug 17, 2026" instead of just "Aug 17, 2026" - a small
  // legibility win for a modal whose whole job is "what's on this day",
  // where the day-of-week is often the first thing marko actually needs.
  // formatDate itself is untouched (still used everywhere else as-is).
  const weekday = iso ? new Date(`${iso}T00:00:00`).toLocaleDateString(undefined, { weekday: "long" }) : "";
  return (
    <Modal open={iso !== null} onClose={onClose} title={iso ? `${weekday}, ${formatDate(iso)}` : ""}>
      {entries.length === 0 ? (
        <p className="py-6 text-center text-sm text-slate-400 dark:text-slate-500">Nothing here.</p>
      ) : (
        <ul className="divide-y divide-slate-100 dark:divide-slate-800">
          {entries.map((entry) => {
            const meta = KIND_META[entry.kind];
            const accent = KIND_ACCENT[entry.kind];
            return (
              <li key={entry.key}>
                <button
                  type="button"
                  onClick={() => onNavigate(entry)}
                  className="flex w-full items-start gap-2.5 py-2.5 text-left hover:bg-slate-50 dark:hover:bg-slate-800/60"
                >
                  <span className={`mt-1.5 h-2 w-2 shrink-0 rounded-full ${accent.dot}`} />
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-1.5">
                      <meta.icon className={`h-3.5 w-3.5 shrink-0 ${accent.text}`} />
                      <p className={`truncate text-sm text-slate-900 dark:text-slate-100 ${SEVERITY_TEXT[entry.severity] || "font-medium"}`}>{entry.title}</p>
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
    <Card className="mb-4 overflow-hidden p-0">
      <p className="border-b border-slate-100 px-4 py-2.5 text-xs font-semibold uppercase tracking-wide text-slate-400 dark:border-slate-800 dark:text-slate-500">
        Today &amp; next 7 days
      </p>
      {entries.length === 0 ? (
        <p className="px-4 py-4 text-sm text-slate-400 dark:text-slate-500">Nothing coming up in the next 7 days.</p>
      ) : (
        <div className="flex flex-col gap-3 p-4 sm:flex-row sm:items-start sm:gap-6">
          <div className="flex flex-wrap gap-1.5">
            {counts.map(({ kind, count }) => {
              const meta = KIND_META[kind];
              const accent = KIND_ACCENT[kind];
              return (
                <span
                  key={kind}
                  className={`inline-flex items-center gap-1.5 rounded-full px-2.5 py-1 text-xs font-medium ${accent.chip}`}
                >
                  <meta.icon className="h-3.5 w-3.5" />
                  {count} {meta.label.toLowerCase()}
                </span>
              );
            })}
          </div>
          <div className="min-w-0 flex-1 border-t border-slate-100 pt-3 sm:border-l sm:border-t-0 sm:pl-6 sm:pt-0 dark:border-slate-800">
            <ul className="flex flex-col gap-2">
              {upNext.map((entry) => {
                const accent = KIND_ACCENT[entry.kind];
                return (
                  <li key={entry.key}>
                    <button
                      type="button"
                      onClick={() => navigateToEntry(navigate, entry)}
                      className="flex w-full items-center gap-2 truncate text-left text-xs text-slate-600 hover:text-brand-700 dark:text-slate-300 dark:hover:text-brand-400"
                    >
                      <span className={`h-1.5 w-1.5 shrink-0 rounded-full ${accent.dot}`} />
                      <span className="shrink-0 tabular-nums text-slate-400 dark:text-slate-500">{formatDateNumeric(entry.date)}</span>
                      <span className={`truncate ${SEVERITY_TEXT[entry.severity]}`}>{entry.title}</span>
                    </button>
                  </li>
                );
              })}
            </ul>
          </div>
        </div>
      )}
    </Card>
  );
}

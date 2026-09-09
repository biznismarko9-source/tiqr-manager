import { useEffect, useMemo, useState, type SVGProps } from "react";
import { useNavigate } from "react-router-dom";
import { api } from "../lib/api";
import type { CalendarEntry, CalendarEntryKind, CalendarSeverity } from "../lib/types";
import { formatDate, formatDateNumeric, formatMoney } from "../lib/format";
import {
  Card,
  EmptyState,
  Input,
  LoadingBlock,
  Modal,
  PageHeader,
  SEGMENTED_TRACK,
  segmentedItemClass,
} from "../components/ui";
import {
  IconAlertTriangle,
  IconCalendarDays,
  IconChevronLeft,
  IconChevronRight,
  IconPackage,
  IconReceipt,
  IconRefresh,
  IconSearch,
  IconUsers,
  IconWallet,
} from "../components/icons";

// 2.5.0: "TIQR Operations Calendar" - marko's own request for one Month/Week
// view over every part of the app that has a real date, instead of five
// separate places to go check what's happening when. See
// commands/calendar.rs's own module doc comment for the full research behind
// which categories are real and which of marko's original candidates are NOT.
//
// Every entry already comes from `get_calendar` fully formed (title,
// subtitle, severity, navigation target) - this page is a grid/list
// renderer, it never computes a business fact of its own. Navigation reuses
// the exact same routes/pages Attention Center, Ticket Control Center and
// Fulfillment Center already send the user to for the same underlying
// records - no new detail view exists (or is needed) for any of this.
//
// 2.5.1: presentation pass - one consistent color per entry KIND, with
// severity as a second, independent channel layered on top (a ring on grid
// chips, colored text in list views). The kind-toggle row doubles as the
// color legend, so no separate legend UI was needed.
//
// 2.8.0: marko asked to make this one of TIQR's main work screens. What
// actually changed, and what deliberately did not:
//
//   - **Four views instead of two** - Month, Week, Day, Agenda. All four
//     share one data path (`get_calendar` over a date range) and one filter/
//     search state; only the range and the layout differ.
//   - **Two genuinely new date sources**, `finance` and `recurring` - see
//     `commands/calendar.rs`'s `finance_in_range` doc comment. marko asked
//     again for payouts/payments/fulfillment; all three are still absent
//     because no such date exists in this schema, and inventing one is the
//     single thing this whole feature must not do.
//   - **No time-of-day axis anywhere, on purpose.** Every date this app
//     stores (`events.event_date`, `orders.purchase_date`, `sales.sale_date`,
//     `pulls.event_date`, `finance_entries.entry_date`,
//     `recurring_expenses.next_date`) is a plain date-only "YYYY-MM-DD" with
//     no time component. marko's own spec said not to fake precise time
//     positions when the data has no time - so Week is seven day COLUMNS,
//     not a 24-hour timetable, and Day groups by kind rather than by hour.
//   - **Every date stays a string.** Dates are compared, bucketed and
//     rendered as ISO text and are never parsed into a `Date` for placement,
//     so nothing can shift a day across a timezone boundary. The only
//     `new Date(...)` calls here are for grid geometry (which days are in
//     this month) and for human labels, never for deciding which cell an
//     entry belongs in.

const KIND_META: Record<CalendarEntryKind, { label: string; icon: (p: SVGProps<SVGSVGElement>) => JSX.Element }> = {
  event: { label: "Events", icon: IconCalendarDays },
  order: { label: "Orders", icon: IconPackage },
  sale: { label: "Sales", icon: IconReceipt },
  finance: { label: "Finance", icon: IconWallet },
  recurring: { label: "Recurring", icon: IconRefresh },
  pull: { label: "Pulls", icon: IconUsers },
  attention: { label: "Attention", icon: IconAlertTriangle },
};

/** Display order for the filter row and for Day Detail's grouping. Not the
 * same as the backend's sort (which is date-first) - this is purely "which
 * category do I want to read first". */
const ALL_KINDS: CalendarEntryKind[] = ["event", "order", "sale", "finance", "recurring", "pull", "attention"];

// 2.5.1: one consistent accent color per KIND - the calendar's own "category
// palette", chosen to stay clear of the severity palette below (red/amber)
// so the two channels never look like the same signal.
// 2.8.0 added `finance` (teal) and `recurring` (fuchsia). Teal rather than
// another green: emerald is already "sale", and money-in-the-ledger must not
// look like a ticket sale at a glance. Fuchsia is the only remaining hue in
// this set that is neither a severity color nor within a shade of another
// kind.
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
  finance: {
    dot: "bg-teal-500",
    chip: "bg-teal-50 text-teal-700 dark:bg-teal-500/10 dark:text-teal-300",
    text: "text-teal-700 dark:text-teal-300",
    legend: "bg-teal-50 text-teal-700 ring-teal-200 dark:bg-teal-500/10 dark:text-teal-300 dark:ring-teal-500/30",
  },
  recurring: {
    dot: "bg-fuchsia-500",
    chip: "bg-fuchsia-50 text-fuchsia-700 dark:bg-fuchsia-500/10 dark:text-fuchsia-300",
    text: "text-fuchsia-700 dark:text-fuchsia-300",
    legend: "bg-fuchsia-50 text-fuchsia-700 ring-fuchsia-200 dark:bg-fuchsia-500/10 dark:text-fuchsia-300 dark:ring-fuchsia-500/30",
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
const WEEK_CELL_CAP = 8;

type ViewMode = "month" | "week" | "day" | "agenda";

/** How far ahead Agenda looks. A fixed forward window rather than an
 * infinite/paged list: marko asked for "chronologický zoznam budúcich
 * udalostí", and a bounded range is also what keeps this on the same
 * one-request-per-range footing as every other view. */
const AGENDA_DAYS_AHEAD = 45;

/** How far BACK the summary strip looks, only so an overdue recurring item
 * can be counted. `recurring_expenses.next_date` advances one occurrence at
 * a time, so anything genuinely overdue is at most a few cycles back; 90
 * days covers weekly and monthly templates comfortably without turning this
 * into a full-history read. */
const OVERDUE_LOOKBACK_DAYS = 90;

// ---------------------------------------------------------------------------
// Plain calendar-day math - Monday-first weeks, no time-of-day component
// anywhere (matches this app's own date columns, which are all plain
// "YYYY-MM-DD" with no time either).
// ---------------------------------------------------------------------------
function pad2(n: number): string {
  return n < 10 ? `0${n}` : String(n);
}

function isoOf(d: Date): string {
  return `${d.getFullYear()}-${pad2(d.getMonth() + 1)}-${pad2(d.getDate())}`;
}

function addDays(d: Date, n: number): Date {
  const copy = new Date(d);
  copy.setDate(copy.getDate() + n);
  return copy;
}

function startOfDay(d: Date): Date {
  const copy = new Date(d);
  copy.setHours(0, 0, 0, 0);
  return copy;
}

function startOfWeek(d: Date): Date {
  const copy = startOfDay(d);
  // JS weeks start on Sunday (0); this app shows Monday-first weeks.
  const shift = (copy.getDay() + 6) % 7;
  return addDays(copy, -shift);
}

function monthGridRange(anchor: Date): { start: Date; end: Date } {
  const first = new Date(anchor.getFullYear(), anchor.getMonth(), 1);
  const last = new Date(anchor.getFullYear(), anchor.getMonth() + 1, 0);
  return { start: startOfWeek(first), end: addDays(startOfWeek(last), 6) };
}

function weekGridRange(anchor: Date): { start: Date; end: Date } {
  const start = startOfWeek(anchor);
  return { start, end: addDays(start, 6) };
}

function daysBetweenInclusive(start: Date, end: Date): Date[] {
  const out: Date[] = [];
  let cursor = startOfDay(start);
  const limit = startOfDay(end);
  while (cursor <= limit) {
    out.push(cursor);
    cursor = addDays(cursor, 1);
  }
  return out;
}

function isWeekend(d: Date): boolean {
  const day = d.getDay();
  return day === 0 || day === 6;
}

/** Whole days between two ISO date strings, computed at UTC noon so a DST
 * transition inside the span can never round the result to the wrong day.
 * Used only for human-facing labels ("In 3 days"), never for placement. */
function daysBetweenIso(fromIso: string, toIso: string): number {
  const a = Date.parse(`${fromIso}T12:00:00Z`);
  const b = Date.parse(`${toIso}T12:00:00Z`);
  return Math.round((b - a) / 86_400_000);
}

/** marko's section 11 - a countdown, but only for events, and only ever
 * derived from the event's own date. Nothing else on this calendar has a
 * "how long until" that means anything: an order or a finance entry is a
 * thing that already happened. Past dates return null rather than a
 * "3 days ago", which would read like a deadline that was missed. */
function eventCountdown(entryDate: string, today: string): string | null {
  const diff = daysBetweenIso(today, entryDate);
  if (diff < 0) return null;
  if (diff === 0) return "Today";
  if (diff === 1) return "Tomorrow";
  return `In ${diff} days`;
}

/** marko's section 12 - "jemný vizuálny signál", explicitly not a color
 * scale. Three levels, rendered as a small 3-segment bar in one muted color;
 * how full the bar is carries the signal, not which hue it is. */
function workloadLevel(count: number): 0 | 1 | 2 | 3 {
  if (count === 0) return 0;
  if (count <= 2) return 1;
  if (count <= 5) return 2;
  return 3;
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
    // 2.8.0: Finance is a single route with client-side tabs and has no
    // per-entry detail page - same shape as Pulls, so the entry carries no
    // linkId and this opens the Finance page itself.
    case "finance":
      navigate("/finance");
      break;
  }
}

function matchesSearch(entry: CalendarEntry, needle: string): boolean {
  if (!needle) return true;
  const haystack = `${entry.title} ${entry.subtitle ?? ""}`.toLowerCase();
  return haystack.includes(needle);
}

// ---------------------------------------------------------------------------

export default function Calendar() {
  const navigate = useNavigate();
  const [viewMode, setViewMode] = useState<ViewMode>("month");
  const [anchor, setAnchor] = useState<Date>(() => startOfDay(new Date()));
  const [entries, setEntries] = useState<CalendarEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [activeKinds, setActiveKinds] = useState<Set<CalendarEntryKind>>(() => new Set(ALL_KINDS));
  const [dayDetail, setDayDetail] = useState<string | null>(null);
  const [search, setSearch] = useState("");

  const today = isoOf(new Date());

  // One range per view. Every view goes through the SAME `get_calendar`
  // command with a different window - there is no view here that loads more
  // than it draws, and switching period refetches only the new window
  // (marko's section 17).
  const { rangeStart, rangeEnd } = useMemo(() => {
    if (viewMode === "month") {
      const { start, end } = monthGridRange(anchor);
      return { rangeStart: start, rangeEnd: end };
    }
    if (viewMode === "week") {
      const { start, end } = weekGridRange(anchor);
      return { rangeStart: start, rangeEnd: end };
    }
    if (viewMode === "day") {
      const d = startOfDay(anchor);
      return { rangeStart: d, rangeEnd: d };
    }
    const start = startOfDay(new Date());
    return { rangeStart: start, rangeEnd: addDays(start, AGENDA_DAYS_AHEAD) };
  }, [viewMode, anchor]);

  const dateFrom = isoOf(rangeStart);
  const dateTo = isoOf(rangeEnd);
  const gridDays = useMemo(
    () => (viewMode === "agenda" ? [] : daysBetweenInclusive(rangeStart, rangeEnd)),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [viewMode, dateFrom, dateTo],
  );

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
  }, [dateFrom, dateTo]);

  const needle = search.trim().toLowerCase();

  // Filters apply in every view (marko's section 5). Search is deliberately
  // NOT folded in here: the grid still needs to draw a non-matching entry so
  // you can see where the matches sit relative to everything else - it just
  // dims it. The list views (Day/Agenda) filter properly, below.
  const visibleEntries = useMemo(() => entries.filter((e) => activeKinds.has(e.kind)), [entries, activeKinds]);
  const matchCount = useMemo(
    () => (needle ? visibleEntries.filter((e) => matchesSearch(e, needle)).length : 0),
    [visibleEntries, needle],
  );
  const firstMatchDate = useMemo(() => {
    if (!needle) return null;
    const hits = visibleEntries.filter((e) => matchesSearch(e, needle)).map((e) => e.date).sort();
    return hits[0] ?? null;
  }, [visibleEntries, needle]);

  const entriesByDate = useMemo(() => {
    const map = new Map<string, CalendarEntry[]>();
    for (const e of visibleEntries) {
      const list = map.get(e.date);
      if (list) list.push(e);
      else map.set(e.date, [e]);
    }
    return map;
  }, [visibleEntries]);

  /** Only kinds that actually occur in the loaded range get a chip - marko's
   * "Ak niektorý typ reálne neexistuje: nepridávaj prázdny filter", applied
   * to what is really on screen rather than to a hardcoded list. A kind that
   * is currently switched OFF still keeps its chip, or there would be no way
   * to switch it back on. */
  const availableKinds = useMemo(() => {
    const present = new Set(entries.map((e) => e.kind));
    return ALL_KINDS.filter((k) => present.has(k) || !activeKinds.has(k));
  }, [entries, activeKinds]);

  const cellCap = viewMode === "month" ? MONTH_CELL_CAP : WEEK_CELL_CAP;

  const rangeLabel =
    viewMode === "month"
      ? anchor.toLocaleDateString(undefined, { month: "long", year: "numeric" })
      : viewMode === "day"
        ? `${new Date(`${isoOf(anchor)}T00:00:00`).toLocaleDateString(undefined, { weekday: "long" })}, ${formatDate(isoOf(anchor))}`
        : viewMode === "week"
          ? `${formatDateNumeric(dateFrom)} - ${formatDateNumeric(dateTo)}`
          : `Next ${AGENDA_DAYS_AHEAD} days`;

  const goToday = () => setAnchor(startOfDay(new Date()));
  const step = (dir: 1 | -1) =>
    setAnchor((a) => {
      if (viewMode === "week") return addDays(a, 7 * dir);
      if (viewMode === "day") return addDays(a, dir);
      return new Date(a.getFullYear(), a.getMonth() + dir, 1);
    });

  const toggleKind = (kind: CalendarEntryKind) => {
    setActiveKinds((prev) => {
      const next = new Set(prev);
      if (next.has(kind)) next.delete(kind);
      else next.add(kind);
      return next;
    });
  };

  /** Jumping to a match switches to Day view on that date - the one place
   * search changes the view, and only because "show me where this is" has no
   * useful answer in a month grid that may not contain it. */
  const jumpToFirstMatch = () => {
    if (!firstMatchDate) return;
    setAnchor(startOfDay(new Date(`${firstMatchDate}T00:00:00`)));
    setViewMode("day");
  };

  return (
    <div>
      <PageHeader
        title="Calendar"
        subtitle="Every event, order, sale, finance entry, recurring due date, pull and attention item that has a real date."
        actions={
          <div className="flex flex-wrap items-center gap-2">
            <div className={SEGMENTED_TRACK}>
              {(["month", "week", "day", "agenda"] as ViewMode[]).map((mode) => (
                <button
                  key={mode}
                  type="button"
                  onClick={() => setViewMode(mode)}
                  aria-pressed={viewMode === mode}
                  className={`${segmentedItemClass(viewMode === mode)} capitalize`}
                >
                  {mode}
                </button>
              ))}
            </div>
            {/* Agenda is always anchored to today, so stepping it would mean
                nothing - the control is hidden rather than shown disabled. */}
            {viewMode !== "agenda" && (
              <div className={SEGMENTED_TRACK}>
                <button
                  type="button"
                  onClick={() => step(-1)}
                  aria-label="Previous"
                  className={`${segmentedItemClass(false)} px-1.5`}
                >
                  <IconChevronLeft className="h-4 w-4" />
                </button>
                <button type="button" onClick={goToday} className={segmentedItemClass(false)}>
                  Today
                </button>
                <button
                  type="button"
                  onClick={() => step(1)}
                  aria-label="Next"
                  className={`${segmentedItemClass(false)} px-1.5`}
                >
                  <IconChevronRight className="h-4 w-4" />
                </button>
              </div>
            )}
          </div>
        }
      />

      <SummaryStrip today={today} onOpenDay={setDayDetail} />

      <div className="mb-3 flex flex-wrap items-center justify-between gap-x-3 gap-y-2">
        <div className="flex items-center gap-3">
          <p className="text-[15px] font-semibold tracking-tight text-slate-900 dark:text-slate-50">{rangeLabel}</p>
          {/* Calendar-local search only - it filters and highlights what is
              already loaded for the current range, and never queries anything
              of its own. Deliberately not a global search (marko's own
              section 15). */}
          <div className="relative w-48">
            <IconSearch className="pointer-events-none absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-slate-400 dark:text-slate-500" />
            <Input
              className="h-8 py-0 pl-8 text-xs"
              placeholder="Search this range..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
            />
          </div>
          {needle && (
            <span className="flex items-center gap-2 text-xs text-slate-500 dark:text-slate-400">
              {matchCount} match{matchCount === 1 ? "" : "es"}
              {firstMatchDate && viewMode !== "day" && (
                <button
                  type="button"
                  onClick={jumpToFirstMatch}
                  className="font-medium text-brand-600 hover:underline dark:text-brand-400"
                >
                  Jump to {formatDateNumeric(firstMatchDate)}
                </button>
              )}
            </span>
          )}
        </div>
        {/* This row is both the kind filter AND the calendar's color legend -
            each pill's dot is the exact color its entries use below, so
            there's no separate "what does this color mean" key to add. */}
        <div className="flex flex-wrap items-center gap-1.5">
          {availableKinds.map((kind) => {
            const meta = KIND_META[kind];
            const accent = KIND_ACCENT[kind];
            const active = activeKinds.has(kind);
            return (
              <button
                key={kind}
                type="button"
                onClick={() => toggleKind(kind)}
                aria-pressed={active}
                className={`inline-flex items-center gap-1.5 rounded-full px-2.5 py-1 text-xs font-medium ring-1 ring-inset transition ${
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

      {/* 2.13.0: the month keeps every day on screen AND keeps the day you
          clicked open beside it. The detail used to be a modal, which meant
          stepping through days was open-read-close-open-read-close and the
          grid disappeared each time. As a column it stays put, so the grid
          and the day are readable at once. Below xl there is not room for
          two columns, so the panel drops under the grid rather than
          squeezing it - and the modal is still what Week/Day/Agenda use,
          since those have no grid to sit beside. */}
      {viewMode === "month" || viewMode === "week" ? (
        <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_320px]">
        <Card className="overflow-hidden p-0">
          <div className="grid grid-cols-7 border-b border-slate-200 dark:border-slate-800">
            {WEEKDAY_LABELS.map((label, i) => (
              <div
                key={label}
                className={`section-title px-2 py-2.5 text-center ${i >= 5 ? "bg-slate-50/70 dark:bg-slate-900/40" : ""}`}
              >
                {label}
              </div>
            ))}
          </div>
          {loading ? (
            <LoadingBlock label="Loading calendar..." />
          ) : (
            // The grid itself is the scroll area, not the page (marko's
            // section 16) - a tall month never pushes the header, filters or
            // summary strip off screen, and there is only ever one scrollbar.
            <div
              className="grid grid-cols-7 overflow-y-auto"
              style={{ maxHeight: "calc(100vh - 22rem)" }}
            >
              {gridDays.map((day) => {
                const iso = isoOf(day);
                const dayEntries = entriesByDate.get(iso) ?? [];
                const inCurrentMonth = viewMode === "week" || day.getMonth() === anchor.getMonth();
                const isToday = iso === today;
                const weekend = isWeekend(day);
                const load = workloadLevel(dayEntries.length);
                return (
                  <div
                    key={iso}
                    className={`relative flex flex-col gap-1 border-b border-r border-slate-100 p-2 transition-colors last:border-r-0 dark:border-slate-800/60 ${
                      viewMode === "week" ? "min-h-[240px]" : "min-h-[104px]"
                    } ${
                      (dayDetail ?? today) === iso
                        ? "bg-brand-50 ring-2 ring-inset ring-brand-500/60 dark:bg-brand-500/[0.14] dark:ring-brand-400/50"
                        : isToday
                        ? "bg-brand-50/60 ring-1 ring-inset ring-brand-500/30 dark:bg-brand-500/[0.08] dark:ring-brand-400/25"
                        : !inCurrentMonth
                          ? "bg-slate-50/70 dark:bg-slate-950/40"
                          : weekend
                            ? "bg-slate-50/50 dark:bg-slate-950/20"
                            : ""
                    }`}
                  >
                    <div className="flex items-center justify-between">
                      <button
                        type="button"
                        onClick={() => setDayDetail(iso)}
                        className={`flex h-6 w-6 shrink-0 items-center justify-center rounded-full text-xs font-semibold tabular-nums transition ${
                          isToday
                            ? "bg-brand-600 text-white shadow-card"
                            : inCurrentMonth
                              ? "text-slate-600 dark:text-slate-300"
                              : "text-slate-300 dark:text-slate-600"
                        } ${dayEntries.length > 0 && !isToday ? "cursor-pointer hover:bg-slate-200/80 dark:hover:bg-slate-700/70" : dayEntries.length > 0 ? "cursor-pointer" : "cursor-default"}`}
                      >
                        {day.getDate()}
                      </button>
                      {load > 0 && <WorkloadBar level={load} count={dayEntries.length} />}
                    </div>
                    <div className="flex flex-1 flex-col gap-1 overflow-hidden">
                      {dayEntries.slice(0, cellCap).map((entry) => {
                        const accent = KIND_ACCENT[entry.kind];
                        const dim = needle.length > 0 && !matchesSearch(entry, needle);
                        return (
                          <button
                            key={entry.key}
                            type="button"
                            onClick={() => navigateToEntry(navigate, entry)}
                            title={`${entry.title}${entry.subtitle ? ` - ${entry.subtitle}` : ""}`}
                            className={`flex items-center gap-1 truncate rounded-md px-1.5 py-[3px] text-left text-[11px] font-medium leading-tight transition hover:ring-2 hover:ring-inset hover:ring-slate-900/10 dark:hover:ring-white/15 ${accent.chip} ${SEVERITY_RING[entry.severity]} ${dim ? "opacity-25" : ""}`}
                          >
                            <span className="truncate">{entry.title}</span>
                          </button>
                        );
                      })}
                      {dayEntries.length > cellCap && (
                        <button
                          type="button"
                          onClick={() => setDayDetail(iso)}
                          className="rounded-md px-1.5 py-0.5 text-left text-[11px] font-semibold text-slate-500 transition hover:bg-slate-100 hover:text-slate-700 dark:text-slate-400 dark:hover:bg-slate-800 dark:hover:text-slate-200"
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
        <DayDetailPanel
          iso={dayDetail ?? today}
          today={today}
          entries={entriesByDate.get(dayDetail ?? today) ?? []}
          onNavigate={(e) => navigateToEntry(navigate, e)}
        />
        </div>
      ) : (
        <ListView
          mode={viewMode}
          loading={loading}
          today={today}
          needle={needle}
          entries={visibleEntries}
          onNavigate={(e) => navigateToEntry(navigate, e)}
        />
      )}

      <DayDetailModal
        iso={viewMode === "month" || viewMode === "week" ? null : dayDetail}
        today={today}
        entries={dayDetail ? (entriesByDate.get(dayDetail) ?? []) : []}
        onClose={() => setDayDetail(null)}
        onNavigate={(e) => navigateToEntry(navigate, e)}
      />
    </div>
  );
}

/** marko's section 12. One muted color, three segments - how full it is
 * carries the signal. Deliberately not red/amber/green, which would collide
 * with the severity channel and produce exactly the "farebný chaos" he asked
 * to avoid. */
function WorkloadBar({ level, count }: { level: number; count: number }) {
  return (
    <span
      className="flex items-center gap-[2px]"
      title={`${count} item${count === 1 ? "" : "s"}`}
      aria-label={`${count} items`}
    >
      {[1, 2, 3].map((i) => (
        <span
          key={i}
          className={`h-2.5 w-[3px] rounded-full ${
            i <= level ? "bg-slate-400 dark:bg-slate-500" : "bg-slate-200 dark:bg-slate-700"
          }`}
        />
      ))}
    </span>
  );
}

/** One entry as a row - shared by Day, Agenda and Day Detail so the three
 * never drift apart. Shows exactly what marko's section 8 asked for and
 * nothing more: title, kind, its own date context, one supporting line, and
 * the amount when the entry really has one. */
function EntryRow({
  entry,
  today,
  onNavigate,
}: {
  entry: CalendarEntry;
  today: string;
  onNavigate: (entry: CalendarEntry) => void;
}) {
  const meta = KIND_META[entry.kind];
  const accent = KIND_ACCENT[entry.kind];
  const countdown = entry.kind === "event" ? eventCountdown(entry.date, today) : null;
  return (
    <button
      type="button"
      onClick={() => onNavigate(entry)}
      className="flex w-full items-start gap-2.5 px-3 py-2.5 text-left transition hover:bg-slate-50 dark:hover:bg-slate-800/60"
    >
      <span className={`mt-1.5 h-2 w-2 shrink-0 rounded-full ${accent.dot}`} />
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-1.5">
          <meta.icon className={`h-3.5 w-3.5 shrink-0 ${accent.text}`} />
          <p className={`truncate text-sm text-slate-900 dark:text-slate-100 ${SEVERITY_TEXT[entry.severity] || "font-medium"}`}>
            {entry.title}
          </p>
        </div>
        <p className="mt-0.5 flex flex-wrap items-center gap-x-1.5 truncate text-xs text-slate-500 dark:text-slate-400">
          <span className={accent.text}>{meta.label.replace(/s$/, "")}</span>
          {entry.subtitle && <span>· {entry.subtitle}</span>}
          {countdown && <span className="font-medium text-slate-600 dark:text-slate-300">· {countdown}</span>}
        </p>
      </div>
      {entry.amountCents !== null && entry.currency && (
        <span className="shrink-0 text-xs font-medium tabular-nums text-slate-600 dark:text-slate-300">
          {formatMoney(entry.amountCents, entry.currency)}
        </span>
      )}
    </button>
  );
}

/** Day and Agenda. Both are chronological lists over the range the page
 * already loaded - Day groups by kind (one day, so the date adds nothing),
 * Agenda groups by date (many days, so the date is the whole point). */
function ListView({
  mode,
  loading,
  today,
  needle,
  entries,
  onNavigate,
}: {
  mode: "day" | "agenda";
  loading: boolean;
  today: string;
  needle: string;
  entries: CalendarEntry[];
  onNavigate: (entry: CalendarEntry) => void;
}) {
  // Unlike the grid, a list has no "where is it relative to everything else"
  // to preserve - so here search really filters.
  const shown = useMemo(() => entries.filter((e) => matchesSearch(e, needle)), [entries, needle]);

  const groups = useMemo(() => {
    if (mode === "day") {
      return ALL_KINDS.map((kind) => ({
        key: kind,
        label: KIND_META[kind].label,
        items: shown.filter((e) => e.kind === kind),
      })).filter((g) => g.items.length > 0);
    }
    const byDate = new Map<string, CalendarEntry[]>();
    for (const e of shown) {
      const list = byDate.get(e.date);
      if (list) list.push(e);
      else byDate.set(e.date, [e]);
    }
    return [...byDate.entries()]
      .sort((a, b) => a[0].localeCompare(b[0]))
      .map(([date, items]) => ({
        key: date,
        label: `${new Date(`${date}T00:00:00`).toLocaleDateString(undefined, { weekday: "long" })}, ${formatDate(date)}`,
        items,
      }));
  }, [shown, mode]);

  if (loading) {
    return (
      <Card className="p-0">
        <LoadingBlock label="Loading calendar..." />
      </Card>
    );
  }

  if (groups.length === 0) {
    return (
      <EmptyState
        icon={<IconCalendarDays className="h-5 w-5" />}
        title={needle ? "Nothing matches that search" : mode === "day" ? "Nothing on this day" : "Nothing coming up"}
        description={
          needle
            ? "Try a shorter search, or clear it to see everything in this range."
            : mode === "day"
              ? "No event, order, sale, finance entry, pull or attention item is dated to this day."
              : `Nothing is dated within the next ${AGENDA_DAYS_AHEAD} days.`
        }
      />
    );
  }

  return (
    <Card className="overflow-hidden p-0">
      <div className="overflow-y-auto" style={{ maxHeight: "calc(100vh - 22rem)" }}>
        {groups.map((group) => (
          <div key={group.key}>
            <p className="section-title sticky top-0 z-10 border-b border-slate-100 bg-slate-50 px-3 py-2 dark:border-slate-800 dark:bg-slate-800/60">
              {group.label}
              <span className="ml-1.5 font-normal normal-case tracking-normal text-slate-400 dark:text-slate-500">
                {group.items.length}
              </span>
            </p>
            <ul className="divide-y divide-slate-100 dark:divide-slate-800">
              {group.items.map((entry) => (
                <li key={entry.key}>
                  <EntryRow entry={entry} today={today} onNavigate={onNavigate} />
                </li>
              ))}
            </ul>
          </div>
        ))}
      </div>
    </Card>
  );
}

/** marko's section 7 - the day's items on the left, a count-by-kind summary
 * on the right. The summary is derived from the same `entries` array the list
 * renders, never a second fetch or a second rule. */
/** 2.13.0: the same day, as a column rather than a modal - month/week use
 * this, everything else still uses DayDetailModal below. Deliberately shares
 * `EntryRow` and the same per-kind counts, so the two can never drift into
 * showing a day differently. */
function DayDetailPanel({
  iso,
  today,
  entries,
  onNavigate,
}: {
  iso: string;
  today: string;
  entries: CalendarEntry[];
  onNavigate: (entry: CalendarEntry) => void;
}) {
  const weekday = new Date(`${iso}T00:00:00`).toLocaleDateString(undefined, { weekday: "long" });
  const counts = ALL_KINDS.map((kind) => ({ kind, count: entries.filter((e) => e.kind === kind).length })).filter(
    (c) => c.count > 0,
  );
  return (
    <Card className="overflow-hidden p-0 xl:sticky xl:top-0 xl:self-start">
      <div className="border-b border-slate-200 px-3.5 py-3 dark:border-slate-800">
        <p className="text-sm font-semibold text-slate-900 dark:text-slate-100">
          {weekday}, {formatDate(iso)}
        </p>
        <p className="mt-0.5 text-xs text-slate-400 dark:text-slate-500">
          {entries.length === 0 ? "Nothing on this day" : `${entries.length} item${entries.length === 1 ? "" : "s"}`}
          {iso === today && " · today"}
        </p>
      </div>
      {entries.length === 0 ? (
        <p className="px-3.5 py-8 text-center text-sm text-slate-400 dark:text-slate-500">
          Pick a day in the grid.
        </p>
      ) : (
        <>
          <ul className="max-h-[420px] divide-y divide-slate-100 overflow-y-auto dark:divide-slate-800">
            {entries.map((entry) => (
              <li key={entry.key}>
                <EntryRow entry={entry} today={today} onNavigate={onNavigate} />
              </li>
            ))}
          </ul>
          <div className="border-t border-slate-200 px-3.5 py-3 dark:border-slate-800">
            <ul className="space-y-1.5">
              {counts.map(({ kind, count }) => (
                <li key={kind} className="flex items-center gap-2 text-xs">
                  <span className={`h-2 w-2 shrink-0 rounded-full ${KIND_ACCENT[kind].dot}`} />
                  <span className="flex-1 text-slate-600 dark:text-slate-400">{KIND_META[kind].label}</span>
                  <span className="font-semibold tabular-nums text-slate-900 dark:text-slate-100">{count}</span>
                </li>
              ))}
            </ul>
          </div>
        </>
      )}
    </Card>
  );
}

function DayDetailModal({
  iso,
  today,
  entries,
  onClose,
  onNavigate,
}: {
  iso: string | null;
  today: string;
  entries: CalendarEntry[];
  onClose: () => void;
  onNavigate: (entry: CalendarEntry) => void;
}) {
  const weekday = iso ? new Date(`${iso}T00:00:00`).toLocaleDateString(undefined, { weekday: "long" }) : "";
  const counts = ALL_KINDS.map((kind) => ({ kind, count: entries.filter((e) => e.kind === kind).length })).filter(
    (c) => c.count > 0,
  );
  return (
    <Modal open={iso !== null} onClose={onClose} title={iso ? `${weekday}, ${formatDate(iso)}` : ""} width="max-w-3xl">
      {entries.length === 0 ? (
        <p className="py-6 text-center text-sm text-slate-400 dark:text-slate-500">Nothing here.</p>
      ) : (
        <div className="flex flex-col gap-4 sm:flex-row">
          <div className="min-w-0 flex-1 overflow-hidden rounded-xl border border-slate-200 dark:border-slate-800">
            <ul className="divide-y divide-slate-100 dark:divide-slate-800">
              {entries.map((entry) => (
                <li key={entry.key}>
                  <EntryRow entry={entry} today={today} onNavigate={onNavigate} />
                </li>
              ))}
            </ul>
          </div>
          <div className="shrink-0 sm:w-44">
            <p className="section-title mb-2">Day summary</p>
            <ul className="space-y-1.5">
              {counts.map(({ kind, count }) => (
                <li key={kind} className="flex items-center gap-2 text-xs">
                  <span className={`h-2 w-2 shrink-0 rounded-full ${KIND_ACCENT[kind].dot}`} />
                  <span className="flex-1 text-slate-600 dark:text-slate-400">{KIND_META[kind].label}</span>
                  <span className="font-semibold tabular-nums text-slate-900 dark:text-slate-100">{count}</span>
                </li>
              ))}
              <li className="flex items-center gap-2 border-t border-slate-200 pt-1.5 text-xs dark:border-slate-800">
                <span className="flex-1 font-medium text-slate-700 dark:text-slate-300">Total</span>
                <span className="font-semibold tabular-nums text-slate-900 dark:text-slate-100">{entries.length}</span>
              </li>
            </ul>
          </div>
        </div>
      )}
    </Modal>
  );
}

/** marko's sections 9 + 10. Today / Tomorrow / Next 7 days / Overdue, over
 * ONE extra `get_calendar` call with its own fixed range - the same command
 * and therefore the same business rules as the grid, never a duplicate
 * computation.
 *
 * "Overdue" is real here only because exactly one thing in this app has a
 * real due date: an active `recurring_expenses.next_date` in the past. That
 * is also precisely how Finance's own Accounts tab defines overdue, and the
 * backend already marks those entries `critical` - so this counts them
 * rather than re-deriving the rule a third time. Nothing else is ever
 * counted as overdue, because nothing else has a deadline to miss. */
function SummaryStrip({ today, onOpenDay }: { today: string; onOpenDay: (iso: string) => void }) {
  const [entries, setEntries] = useState<CalendarEntry[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const from = isoOf(addDays(new Date(), -OVERDUE_LOOKBACK_DAYS));
    const to = isoOf(addDays(new Date(), 7));
    api
      .getCalendar({ dateFrom: from, dateTo: to })
      .then(setEntries)
      .catch(() => setEntries([]))
      .finally(() => setLoading(false));
  }, []);

  if (loading) return null;

  const tomorrow = isoOf(addDays(new Date(), 1));
  const in7 = isoOf(addDays(new Date(), 7));
  const todayItems = entries.filter((e) => e.date === today);
  const tomorrowItems = entries.filter((e) => e.date === tomorrow);
  const next7 = entries.filter((e) => e.date > today && e.date <= in7);
  const overdue = entries.filter((e) => e.kind === "recurring" && e.severity === "critical" && e.date < today);

  const tiles: { label: string; count: number; onClick?: () => void; tone?: "danger" }[] = [
    { label: "Today", count: todayItems.length, onClick: todayItems.length ? () => onOpenDay(today) : undefined },
    { label: "Tomorrow", count: tomorrowItems.length, onClick: tomorrowItems.length ? () => onOpenDay(tomorrow) : undefined },
    { label: "Next 7 days", count: next7.length },
  ];
  // Only ever shown when there is something real to show - an "Overdue 0"
  // tile on a calendar whose data mostly has no deadlines would imply this
  // app tracks more deadlines than it does.
  if (overdue.length > 0) {
    tiles.push({
      label: "Overdue",
      count: overdue.length,
      tone: "danger",
      onClick: () => onOpenDay(overdue.map((e) => e.date).sort()[0]!),
    });
  }

  // 2.13.2: separate cards again, matching every other tile in the app -
  // marko's call after seeing 2.13.1's single bar next to the Attention
  // Center row. Tiles with nothing to open stay non-interactive rather than
  // becoming dead buttons; that part of 2.13.1 was worth keeping.
  return (
    <div className="summary-bar mb-4">
      {tiles.map((tile) => (
        <Card
          key={tile.label}
          interactive={Boolean(tile.onClick)}
          className="min-w-[9.5rem] flex-1 p-3.5"
          onClick={tile.onClick}
          role={tile.onClick ? "button" : undefined}
        >
          <p className="section-title truncate">{tile.label}</p>
          <p
            className={`mt-2 text-[22px] font-semibold leading-none tabular-nums ${
              tile.tone === "danger" ? "text-red-600 dark:text-red-400" : "text-slate-900 dark:text-slate-50"
            }`}
          >
            {tile.count}
          </p>
          <p className="mt-2 truncate text-xs text-slate-400 dark:text-slate-500">
            {tile.count === 1 ? "item" : "items"}
          </p>
        </Card>
      ))}
    </div>
  );
}

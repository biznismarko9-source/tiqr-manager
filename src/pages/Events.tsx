import { useEffect, useMemo, useState } from "react";
import { Link, useLocation, useNavigate } from "react-router-dom";
import { api, errMsg } from "../lib/api";
import type { EventCategory, EventInput, EventStatus, EventWithStats } from "../lib/types";
import { formatDateNumeric, formatMoneyOrMixed, formatPercentOrMixed, summarizeBulkDeleteSkips, todayIso } from "../lib/format";
import {
  Badge,
  Button,
  BulkDeleteBar,
  CHECKBOX_CLASS,
  ConfirmDialog,
  EmptyState,
  Field,
  Input,
  LoadingBlock,
  Modal,
  ModalFooter,
  PageHeader,
  Select,
  TabSwitcher,
  Textarea,
} from "../components/ui";
import { EventCategoryBadge } from "../components/EventCategoryBadge";
import { LookupSelect } from "../components/LookupSelect";
import { IconCalendarDays, IconPlus, IconSearch, IconTag, IconTrash } from "../components/icons";
import { useToast } from "../lib/toast";
import { useListTab } from "../lib/useListTab";
import { useNarrowTables } from "../lib/useNarrowTables";

// 2.0.59: "Upcoming" vs "Completed" tabs (marko's request - see
// REDESIGN-2.0.59-REPORT.md), same shape as the Dashboard's own tabs. A
// cancelled event is grouped into "Completed" - it isn't upcoming either,
// and it's just as much "out of the way, no longer needs attention" as a
// genuinely completed one.
const EVENT_TABS: { key: "upcoming" | "completed"; label: string }[] = [
  { key: "upcoming", label: "Upcoming" },
  { key: "completed", label: "Completed" },
];

const EMPTY_INPUT: EventInput = {
  name: "",
  artistTeam: null,
  venue: "",
  city: "",
  country: "",
  eventDate: "",
  categoryId: null,
  status: "upcoming",
  notes: "",
};

// 2.0.34: "" reproduces exactly what list_events already always returned
// (event_date DESC, id DESC, nulls last - see events.rs) unchanged, so the
// default view is provably identical to today's. "soonest" is the new,
// opt-in direction - named for what it actually does (event_date ascending)
// rather than reusing "Newest/Oldest" from Sales/Orders, which doesn't map
// cleanly onto a scheduled date that's usually in the future, not the past.
//
// 2.0.65: this page's own "Soonest/Furthest" naming is now the app-wide
// standard (Orders/Tickets/Sales/Pulls all adopted it too - see
// REDESIGN-2.0.65-REPORT.md), and marko confirmed Soonest should be the
// default everywhere, so the two keys below swap roles: "" is now the
// ascending (soonest) branch, and "furthest" is the new explicit
// descending/pass-through value.
const EVENT_SORT_LABELS: Record<string, string> = {
  "": "Soonest first",
  furthest: "Furthest first",
};

// ---------------------------------------------------------------------------
// Event Lifecycle (2.3.0) - a single, consistent "what stage is this event
// at" phase, derived entirely from data the app already has (event.status,
// event.eventDate, event.stats.*) - no new column, no manually-set status,
// no new backend command. marko's own message proposed 7 stages (UPCOMING ->
// BUYING/INVENTORY -> LISTED -> SELLING -> EVENT DAY -> POST EVENT ->
// COMPLETED); this ships 6 - see computeEventLifecyclePhase below for
// exactly why POST EVENT was folded into COMPLETED, and
// REDESIGN-2.3.0-REPORT.md for the full writeup. Lives here (not in a new
// lib/ file) and is imported by EventDetail.tsx - the same "first-named
// page owns it, others import it" convention FulfillmentCenter.tsx already
// established by importing isSaleGroupDone from Sales.tsx.
export type EventLifecyclePhase = "upcoming" | "inventory" | "listed" | "selling" | "event_day" | "completed";

export const EVENT_LIFECYCLE_PHASES: { key: EventLifecyclePhase; label: string }[] = [
  { key: "upcoming", label: "Upcoming" },
  { key: "inventory", label: "Inventory" },
  { key: "listed", label: "Listed" },
  { key: "selling", label: "Selling" },
  { key: "event_day", label: "Event Day" },
  { key: "completed", label: "Completed" },
];

// 2.3.0: "is this event over" - word-for-word the same rule Orders.tsx's own
// isEventDone (and attention_center.rs's independent Rust twin,
// event_is_done) already use: status is completed/cancelled, OR its date has
// strictly passed (today itself is NOT over yet - "event_day" below needs
// that exact distinction). Kept as its own copy here rather than importing
// Orders.tsx's version - the same reasoning attention_center.rs documents
// for keeping its own Rust copy instead of depending on frontend code: this
// is a one-line, extremely stable rule, and Events.tsx has no other reason
// to depend on Orders.tsx. If this rule ever changes, it has to change in
// Orders.tsx, attention_center.rs, AND here, identically.
function isEventLifecycleDone(ev: { status: EventStatus; eventDate: string | null }): boolean {
  return ev.status === "completed" || ev.status === "cancelled" || (ev.eventDate !== null && ev.eventDate < todayIso());
}

// 2.3.0: the one function this whole feature is built on. Deliberately a
// pure function of EventWithStats alone (event + its existing finance/ticket
// stats) - already returned by both list_events (Events overview) and
// get_event (Event Workspace), so computing a phase for every event in the
// list costs zero extra network/IPC calls. Order matters - first match wins:
//
// 1. COMPLETED - isEventLifecycleDone(ev). This is marko's own literal
//    COMPLETED rule (event date passed OR status completed/cancelled), and
//    it also absorbs his proposed POST EVENT stage: that rule is purely
//    date/status based, so the day after an event it is already COMPLETED -
//    there is no real gap left to place a distinct "just happened, cleanup
//    still pending" phase without inventing a new, arbitrary grace-period
//    threshold nothing else in this app is grounded in. Per his own "can't
//    reliably distinguish it -> merge with the nearest usable phase, say
//    why" instruction, POST EVENT is merged into COMPLETED here. This does
//    NOT lose the "pending fulfillment" signal he wants surfaced for it -
//    EventDetail.tsx's EventLifecycleBlock shows a pending-fulfillment count
//    and a "sold, not delivered" Next Action for ANY phase, COMPLETED
//    included - the exact same "never gated by event_is_done" choice
//    attention_center.rs's own sold_undelivered category already makes, for
//    the same reason (an undelivered sale is just as real a problem once
//    the event is over).
// 2. EVENT_DAY - event.eventDate === today, checked before any ticket-count
//    phase. Whole-day only - no event time exists anywhere in this app, so
//    no hour-level math is attempted, per marko's own explicit instruction.
// 3. SELLING - at least one sold ticket.
// 4. LISTED - zero sold, at least one currently listed (ticket status =
//    'listed' - the same field this table's own "Available" column already
//    breaks out as a sub-line).
// 5. INVENTORY - zero sold, zero listed, at least one ticket purchased.
//    marko's message proposed "BUYING / INVENTORY" for this stage - it's
//    named INVENTORY here, matching the ticket-status vocabulary this exact
//    table already uses (Available/Listed/Sold), since "buying" reads like
//    an in-progress action this app has no way to observe, while
//    "inventory" describes the actual, observable state (tickets bought,
//    sitting unsold and unlisted).
// 6. UPCOMING - nothing purchased yet.
//
// CANCELLED is not a distinct 8th phase - it already has its own dedicated,
// always-visible Status badge right next to this one (both on this table
// and on the Event Workspace header), so folding it into COMPLETED here
// doesn't hide anything; it just avoids this new badge re-stating what the
// existing one already says.
export function computeEventLifecyclePhase(ev: EventWithStats): EventLifecyclePhase {
  if (isEventLifecycleDone(ev)) return "completed";
  if (ev.eventDate === todayIso()) return "event_day";
  if (ev.stats.soldTickets > 0) return "selling";
  if (ev.stats.listedTickets > 0) return "listed";
  if (ev.stats.purchasedTickets > 0) return "inventory";
  return "upcoming";
}

// 2.3.0: same small "colored dot + label" idiom Dashboard.tsx's Attention
// Center already uses for priority (PRIORITY_DOT_CLASS) - not a new visual
// language, just a new color map keyed by phase instead of priority.
// Exported so EventDetail.tsx's own, larger current-phase display can reuse
// the exact same colors rather than defining a second map.
export const LIFECYCLE_PHASE_DOT: Record<EventLifecyclePhase, string> = {
  upcoming: "bg-slate-400 dark:bg-slate-500",
  inventory: "bg-sky-500",
  listed: "bg-indigo-500",
  selling: "bg-amber-500",
  event_day: "bg-red-500",
  completed: "bg-emerald-500",
};

/** Small "phase" pill - dot + label, deliberately not built on ui.tsx's
 * Badge/STATUS_TONES (those are keyed by literal status/payment/delivery
 * strings this new concept doesn't share). Used by this page's own table
 * below and by EventDetail.tsx's Overview tab (imported from here). */
export function EventLifecyclePhaseBadge({ phase, className }: { phase: EventLifecyclePhase; className?: string }) {
  const label = EVENT_LIFECYCLE_PHASES.find((p) => p.key === phase)?.label ?? phase;
  return (
    <span className={`inline-flex items-center gap-1.5 whitespace-nowrap text-xs font-medium text-slate-600 dark:text-slate-300 ${className ?? ""}`}>
      <span className={`h-1.5 w-1.5 shrink-0 rounded-full ${LIFECYCLE_PHASE_DOT[phase]}`} />
      {label}
    </span>
  );
}

export default function Events() {
  const toast = useToast();
  const navigate = useNavigate();
  const location = useLocation();
  const isNarrow = useNarrowTables();
  const [events, setEvents] = useState<EventWithStats[] | null>(null);
  const [categories, setCategories] = useState<EventCategory[]>([]);
  const [search, setSearch] = useState("");
  // 2.0.27: event category filter (marko's request - filter Events/Orders/
  // Sales by category, same as every other list-page dropdown filter here).
  const [categoryId, setCategoryId] = useState<number | "">("");
  // 2.0.65: date-range filter, matching Orders/Tickets/Pulls - no session
  // memory here either, same reasoning as sortBy just below (this file
  // doesn't persist search/categoryId across visits, so these shouldn't be
  // the first filters here that suddenly do).
  const [dateFrom, setDateFrom] = useState("");
  const [dateTo, setDateTo] = useState("");
  // 2.0.34: marko asked for a way to sort Events/Orders/Sales by date "so
  // nothing gets lost". No session-memory var for this (unlike Orders.tsx's
  // lastOrdersSortBy) - this file doesn't remember search/categoryId across
  // visits either, so a new sort control shouldn't be the one filter here
  // that suddenly persists.
  const [sortBy, setSortBy] = useState("");
  // 2.3.0: lifecycle phase filter (marko's request - "FILTER: umožni
  // filtrovanie eventov podľa lifecycle phase"). Client-side only, same
  // reasoning as sortBy above - no session memory for this file's filters.
  // Applied ON TOP of the Upcoming/Completed tab below (AND, not instead
  // of) - that tab is about the manually-set `status` field; this filter is
  // about the new derived phase, and the two can legitimately disagree
  // (e.g. a still-"upcoming"-status event whose date has quietly passed
  // shows as COMPLETED phase while still under the Upcoming tab) - that's
  // useful signal, not a bug, so nothing here forces them to agree.
  const [lifecyclePhase, setLifecyclePhase] = useState<EventLifecyclePhase | "">("");
  // 2.0.59: see EVENT_TABS above.
  const [tab, setTab] = useListTab("eventsTab", ["upcoming", "completed"] as const);
  const [modalOpen, setModalOpen] = useState(false);
  const [editing, setEditing] = useState<EventWithStats | null>(null);
  // 2.0.28: bulk-delete selection mode - marko's own request. No checkbox
  // column sitting there all the time; the "Delete" toggle button below
  // reveals it, and it disappears again the moment you confirm or cancel.
  const [selectionMode, setSelectionMode] = useState(false);
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [confirmBulkDelete, setConfirmBulkDelete] = useState(false);
  const [bulkDeleting, setBulkDeleting] = useState(false);
  // 2.0.63: "Detect categories" - retroactively runs the same free-rules-
  // then-AI detection a sheet sync already runs automatically on brand-new
  // events (see ai_categorize.rs), but against every event already sitting
  // here with no category. Only ever touches events with no category yet,
  // so re-running this is always safe - same "safe to click repeatedly"
  // property Settings.tsx's "Fix sync" (2.0.60) already holds itself to.
  const [confirmDetectCategories, setConfirmDetectCategories] = useState(false);
  const [detectingCategories, setDetectingCategories] = useState(false);

  useEffect(() => {
    api.listEventCategories().then(setCategories).catch(() => {});
  }, []);

  const load = () => {
    api
      .listEvents({
        search: search || undefined,
        categoryId: categoryId || undefined,
        dateFrom: dateFrom || undefined,
        dateTo: dateTo || undefined,
      })
      .then(setEvents)
      .catch((e) => toast.error(errMsg(e)));
  };

  const toggleOne = (id: number) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const allSelected = events !== null && events.length > 0 && events.every((ev) => selected.has(ev.id));
  const toggleSelectAll = () => {
    setSelected(allSelected ? new Set() : new Set((events ?? []).map((ev) => ev.id)));
  };

  // 2.0.34: events without an event_date (status can be "upcoming" with no
  // date set yet, e.g. TBD) can't be placed on a "soonest/furthest" axis at
  // all - list_events already always puts them last (the `(e.event_date IS
  // NULL)` clause), so this keeps that same rule for the new sort direction
  // too rather than letting them jump to the top when reversed. 2.0.65:
  // "furthest" (was "") is now the one left as a straight pass-through of
  // `events` - it's still exactly event_date DESC, id DESC from the
  // backend - and the default ("", was "soonest") is now the ascending
  // branch, per marko's app-wide "soonest is the default everywhere" choice.
  const sortedEvents = useMemo(() => {
    if (!events) return [];
    if (sortBy === "furthest") return events;
    const withDate = events.filter((ev) => ev.eventDate !== null);
    const withoutDate = events.filter((ev) => ev.eventDate === null);
    withDate.sort((a, b) => (a.eventDate as string).localeCompare(b.eventDate as string) || a.id - b.id);
    return [...withDate, ...withoutDate];
  }, [events, sortBy]);

  // 2.0.59: tab split happens client-side, after sorting, on data the page
  // already fetched - no new backend query, same "filter what's already in
  // memory" approach the sort above already uses.
  const visibleEvents = useMemo(
    () =>
      sortedEvents
        .filter((ev) => (tab === "upcoming" ? ev.status === "upcoming" : ev.status !== "upcoming"))
        .filter((ev) => !lifecyclePhase || computeEventLifecyclePhase(ev) === lifecyclePhase),
    [sortedEvents, tab, lifecyclePhase],
  );

  const exitSelectionMode = () => {
    setSelectionMode(false);
    setSelected(new Set());
  };

  const confirmDeleteSelected = async () => {
    setBulkDeleting(true);
    try {
      const result = await api.bulkDeleteEvents(Array.from(selected));
      if (result.deletedIds.length > 0) {
        toast.success(`${result.deletedIds.length} event${result.deletedIds.length === 1 ? "" : "s"} deleted`);
      }
      if (result.skipped.length > 0) {
        toast.error(`${result.skipped.length} skipped: ${summarizeBulkDeleteSkips(result.skipped)}`);
      }
      setConfirmBulkDelete(false);
      exitSelectionMode();
      load();
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setBulkDeleting(false);
    }
  };

  const runDetectCategories = async () => {
    setDetectingCategories(true);
    try {
      const result = await api.detectEventCategories();
      const resolved = result.categorizedByRule + result.categorizedByAi;
      if (result.checked === 0) {
        toast.success("Every event already has a category - nothing to do.");
      } else if (resolved === 0) {
        toast.error(
          result.aiConfigured
            ? `Checked ${result.checked}, but none could be confidently identified.`
            : `Checked ${result.checked}, but none matched a free keyword rule (AI lookup isn't set up in this build).`,
        );
      } else {
        const aiNote = result.categorizedByAi > 0 ? ` (${result.categorizedByAi} via AI)` : "";
        toast.success(
          `Categorized ${resolved} of ${result.checked} event${result.checked === 1 ? "" : "s"}${aiNote}.` +
            (result.leftUncategorized > 0 ? ` ${result.leftUncategorized} still need a category by hand.` : ""),
        );
      }
      setConfirmDetectCategories(false);
      load();
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setDetectingCategories(false);
    }
  };

  useEffect(() => {
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // 1.8.3 (section 11): lets the Dashboard's "New Event" Quick Action jump
  // here with the create modal already open - same navigate(path, {state})
  // + consume-and-clear convention already used by Orders.tsx's
  // presetEventId and Sales.tsx's own openCreate handling, no new pattern.
  useEffect(() => {
    const state = location.state as { openCreate?: boolean } | null;
    if (state?.openCreate) {
      setEditing(null);
      setModalOpen(true);
      navigate(location.pathname, { replace: true, state: null });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [location.state]);

  useEffect(() => {
    const t = setTimeout(load, 250);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [search, categoryId, dateFrom, dateTo]);

  return (
    <div>
      <PageHeader
        title="Events"
        subtitle="Every event you buy or sell tickets for."
        actions={
          <div className="flex items-center gap-2">
            {!selectionMode && events && events.length > 0 && (
              <Button variant="secondary" onClick={() => setConfirmDetectCategories(true)}>
                <IconTag className="h-4 w-4" /> Detect categories
              </Button>
            )}
            {!selectionMode && events && events.length > 0 && (
              <Button variant="secondary" onClick={() => setSelectionMode(true)}>
                <IconTrash className="h-4 w-4" /> Delete
              </Button>
            )}
            <Button
              variant="primary"
              onClick={() => {
                setEditing(null);
                setModalOpen(true);
              }}
            >
              <IconPlus className="h-4 w-4" /> New Event
            </Button>
          </div>
        }
      />

      <TabSwitcher tabs={EVENT_TABS} active={tab} onChange={setTab} />

      <div className="mb-2 flex flex-wrap items-end gap-3">
        <div className="w-64">
          <span className="label">Search</span>
          <div className="relative">
            <IconSearch className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-400 dark:text-slate-500" />
            <Input
              placeholder="Search events..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="pl-9"
            />
          </div>
        </div>
        <div className="w-52">
          <span className="label">Category</span>
          <Select value={categoryId} onChange={(e) => setCategoryId(e.target.value ? Number(e.target.value) : "")}>
            <option value="">All categories</option>
            {categories.map((c) => (
              <option key={c.id} value={c.id}>
                {c.name}
              </option>
            ))}
          </Select>
        </div>
        <div className="w-36">
          <span className="label">From</span>
          <Input type="date" value={dateFrom} onChange={(e) => setDateFrom(e.target.value)} />
        </div>
        <div className="w-36">
          <span className="label">To</span>
          <Input type="date" value={dateTo} onChange={(e) => setDateTo(e.target.value)} />
        </div>
        <div className="w-44">
          <span className="label">Lifecycle phase</span>
          <Select
            value={lifecyclePhase}
            onChange={(e) => setLifecyclePhase(e.target.value as EventLifecyclePhase | "")}
            aria-label="Filter by lifecycle phase"
          >
            <option value="">All phases</option>
            {EVENT_LIFECYCLE_PHASES.map((p) => (
              <option key={p.key} value={p.key}>
                {p.label}
              </option>
            ))}
          </Select>
        </div>
        <div className="w-44">
          <span className="label">Sort</span>
          <Select value={sortBy} onChange={(e) => setSortBy(e.target.value)} aria-label="Sort events">
            {Object.entries(EVENT_SORT_LABELS).map(([value, label]) => (
              <option key={value || "soonest"} value={value}>
                {label}
              </option>
            ))}
          </Select>
        </div>
      </div>

      {selectionMode && (
        <BulkDeleteBar
          count={selected.size}
          itemLabel="event"
          busy={bulkDeleting}
          onConfirm={() => setConfirmBulkDelete(true)}
          onCancel={exitSelectionMode}
        />
      )}

      {events === null ? (
        <LoadingBlock />
      ) : events.length === 0 ? (
        <EmptyState
          icon={<IconCalendarDays className="h-8 w-8" />}
          title="No events found"
          description="Create your first event to start tracking orders, tickets and sales for it."
          action={
            <Button variant="primary" onClick={() => setModalOpen(true)}>
              <IconPlus className="h-4 w-4" /> New Event
            </Button>
          }
        />
      ) : visibleEvents.length === 0 ? (
        // 2.0.59: events exist, just none in the active tab - a lighter
        // message than the "create your first event" empty state above,
        // which would make no sense to repeat when events already exist.
        <EmptyState
          icon={<IconCalendarDays className="h-8 w-8" />}
          title={tab === "upcoming" ? "No upcoming events" : "No completed events yet"}
          description={
            tab === "upcoming"
              ? "Every event is completed or cancelled. Switch to the Completed tab to see them."
              : "Events move here once their status is Completed or Cancelled."
          }
        />
      ) : (
        // 1.8.3 table-UX audit: table-layout:fixed + <colgroup> (see
        // Sales.tsx for the full rationale) instead of the old
        // min-w-[900px]+overflow-x-auto pattern, which could actually
        // overflow on this app's smallest supported window (900px needed vs.
        // an 808px floor). Row click-to-navigate (BUG #7) is untouched.
        // 2.0.32: max-w-[1400px] added - see Sales.tsx's own comment on the
        // identical change for the full rationale.
        //
        // 2.0.37: switched from a fixed-px colgroup (one absorbing Event
        // column, constant widths for everything else regardless of window
        // size) to the same pure-percentage, two-mode model Sales.tsx uses -
        // max-w-[1400px] on the wrapper is gone for the same reason it left
        // Sales.tsx in 2.0.35: a pure-percentage table has no single column
        // that runs away on a wide window (every column grows together), so
        // there's nothing left to cap. Below the shared useNarrowTables()
        // breakpoint (1690px window, same for every table in the app so
        // they all resize together), Margin and ROI hide (Profit - the
        // bottom-line number - stays) and everything else grows a little
        // and switches to the smaller .th-c-narrow/.td-c-narrow. Verified
        // (Playwright, real Intl.NumberFormat data across en-US/sk-SK/
        // de-DE, not just header text) to fit without scrolling or wrapping
        // all the way down to 1080px, this app's enforced minimum window
        // width - see PROTECTED-AREAS-NOTES.md, 2.0.37 section.
        // 2.0.38: percentages recomputed alongside every other table's
        // (Events has no code/ID column, so it wasn't hit by the
        // under-measurement bug the others had - see PROTECTED-AREAS-
        // NOTES.md's 2.0.38 section - but its money columns were re-verified
        // with the same rigor anyway, and Date switched to formatDateNumeric
        // ("11.09.2026", full 4-digit year) in place of the old locale-
        // dependent formatDate call, per marko's explicit format request.
        // Shared breakpoint moved to 1649px (was 1690px) - see
        // useNarrowTables.ts.
        <div className="overflow-x-auto rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-sm">
          <table className="w-full table-fixed border-collapse">
            {isNarrow ? (
              <colgroup>
                {selectionMode && <col className="w-8" />}
                <col className="w-[30%]" />
                <col className="w-[8.659%]" />
                <col className="w-[10.122%]" />
                <col className="w-[7.561%]" />
                <col className="w-[9.512%]" />
                <col className="w-[11.22%]" />
                <col className="w-[11.22%]" />
                <col className="w-[11.707%]" />
              </colgroup>
            ) : (
              <colgroup>
                {selectionMode && <col className="w-8" />}
                <col className="w-[37.341%]" />
                <col className="w-[6.86%]" />
                <col className="w-[6.436%]" />
                <col className="w-[5.233%]" />
                <col className="w-[6.506%]" />
                <col className="w-[8.699%]" />
                <col className="w-[8.699%]" />
                <col className="w-[9.052%]" />
                <col className="w-[5.587%]" />
                <col className="w-[5.587%]" />
              </colgroup>
            )}
            <thead className="border-b border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-800/60">
              <tr>
                {selectionMode && (
                  <th className={isNarrow ? "th-c-narrow" : "th-c"}>
                    <input
                      type="checkbox"
                      className={CHECKBOX_CLASS}
                      checked={allSelected}
                      onChange={toggleSelectAll}
                      aria-label="Select all events"
                    />
                  </th>
                )}
                <th className={isNarrow ? "th-c-narrow" : "th-c"}>Event</th>
                <th className={isNarrow ? "th-c-narrow" : "th-c"}>Date</th>
                <th className={isNarrow ? "th-c-narrow" : "th-c"}>Status</th>
                <th className={`${isNarrow ? "th-c-narrow" : "th-c"} text-right`}>Tickets</th>
                <th className={`${isNarrow ? "th-c-narrow" : "th-c"} text-right`}>Available</th>
                <th className={`${isNarrow ? "th-c-narrow" : "th-c"} text-right`}>Cost</th>
                <th className={`${isNarrow ? "th-c-narrow" : "th-c"} text-right`}>Revenue</th>
                <th className={`${isNarrow ? "th-c-narrow" : "th-c"} text-right`}>Profit</th>
                {!isNarrow && <th className="th-c text-right">Margin</th>}
                {!isNarrow && <th className="th-c text-right">ROI</th>}
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
              {visibleEvents.map((ev) => (
                <tr
                  key={ev.id}
                  className="cursor-pointer hover:bg-slate-50 dark:hover:bg-slate-800/60"
                  onClick={(e) => {
                    // BUG #7 fix: the event name cell already has its own
                    // <Link> below, which performs a single, correct router
                    // navigation (and keeps keyboard/middle-click/right-click
                    // working). This handler only covers "click anywhere
                    // ELSE in the row" as a convenience, so a click that
                    // lands on the Link is never navigated a second time -
                    // it defers entirely to the Link instead of also firing
                    // its own navigation for the same click. 2.0.28: also
                    // excludes the new checkbox (its own onChange handles
                    // it), and while selectionMode is on, a row click
                    // toggles selection instead of navigating away.
                    if ((e.target as HTMLElement).closest("a, input")) return;
                    if (selectionMode) {
                      toggleOne(ev.id);
                      return;
                    }
                    navigate(`/events/${ev.id}`);
                  }}
                >
                  {selectionMode && (
                    <td className={isNarrow ? "td-c-narrow" : "td-c"}>
                      <input
                        type="checkbox"
                        className={CHECKBOX_CLASS}
                        checked={selected.has(ev.id)}
                        onChange={() => toggleOne(ev.id)}
                        aria-label={`Select ${ev.name}`}
                      />
                    </td>
                  )}
                  <td className={isNarrow ? "td-c-narrow" : "td-c"}>
                    <div className="flex items-center gap-1.5">
                      <Link
                        to={`/events/${ev.id}`}
                        title={ev.name}
                        className="truncate font-medium text-slate-900 dark:text-slate-100 hover:text-brand-700 dark:hover:text-brand-400"
                      >
                        {ev.name}
                      </Link>
                      {/* 2.0.27: category color-coding, staying "in theme" -
                          same pill idiom as every status Badge on this page,
                          just keyed by a fixed per-category palette instead
                          of a status string. Only shown once category is
                          actually set - most events won't have one at first. */}
                      {ev.category && ev.categoryColorSlot !== null && (
                        <span className="shrink-0">
                          <EventCategoryBadge name={ev.category} colorSlot={ev.categoryColorSlot} />
                        </span>
                      )}
                    </div>
                    <p className="truncate text-xs text-slate-400 dark:text-slate-500">
                      {[ev.venue, ev.city].filter(Boolean).join(", ")}
                    </p>
                  </td>
                  <td className={`${isNarrow ? "td-c-narrow" : "td-c"} whitespace-nowrap`}>{formatDateNumeric(ev.eventDate)}</td>
                  <td className={isNarrow ? "td-c-narrow" : "td-c"}>
                    <Badge tone={ev.status}>{ev.status}</Badge>
                    {/* 2.3.0: lifecycle phase - stacked under the existing
                        Status badge, same "second line under the primary
                        one" idiom the Event cell already uses (name +
                        venue/city). Keeps this column's colgroup width
                        completely unchanged - no new column, no redesign. */}
                    <div className="mt-1">
                      <EventLifecyclePhaseBadge phase={computeEventLifecyclePhase(ev)} />
                    </div>
                  </td>
                  <td className={`${isNarrow ? "td-c-narrow" : "td-c"} text-right tabular-nums whitespace-nowrap`}>{ev.stats.purchasedTickets}</td>
                  <td className={`${isNarrow ? "td-c-narrow" : "td-c"} text-right tabular-nums whitespace-nowrap`}>{ev.stats.availableTickets}</td>
                  <td className={`${isNarrow ? "td-c-narrow" : "td-c"} text-right tabular-nums whitespace-nowrap`}>{formatMoneyOrMixed(ev.stats.totalCostCents, ev.stats.currency)}</td>
                  <td className={`${isNarrow ? "td-c-narrow" : "td-c"} text-right tabular-nums whitespace-nowrap`}>{formatMoneyOrMixed(ev.stats.revenueCents, ev.stats.currency)}</td>
                  <td
                    className={`${isNarrow ? "td-c-narrow" : "td-c"} text-right tabular-nums whitespace-nowrap font-medium ${ev.stats.profitCents > 0 ? "text-emerald-600 dark:text-emerald-400" : ev.stats.profitCents < 0 ? "text-red-600 dark:text-red-400" : ""}`}
                  >
                    {formatMoneyOrMixed(ev.stats.profitCents, ev.stats.currency)}
                  </td>
                  {!isNarrow && (
                    <td className="td-c text-right tabular-nums whitespace-nowrap">{formatPercentOrMixed(ev.stats.margin, ev.stats.currency)}</td>
                  )}
                  {!isNarrow && (
                    <td className="td-c text-right tabular-nums whitespace-nowrap">{formatPercentOrMixed(ev.stats.roi, ev.stats.currency)}</td>
                  )}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      <EventFormModal
        open={modalOpen}
        initial={editing}
        onClose={() => setModalOpen(false)}
        onSaved={() => {
          setModalOpen(false);
          load();
        }}
      />

      <ConfirmDialog
        open={confirmBulkDelete}
        title={`Delete ${selected.size} selected event${selected.size === 1 ? "" : "s"}?`}
        message="Events with any orders or tickets linked to them will be skipped automatically. This cannot be undone."
        confirmLabel="Delete selected"
        danger
        busy={bulkDeleting}
        onCancel={() => setConfirmBulkDelete(false)}
        onConfirm={confirmDeleteSelected}
      />

      <ConfirmDialog
        open={confirmDetectCategories}
        title="Detect categories for uncategorized events?"
        message="Tries to figure out a category for every event that currently has none - first using a few safe keywords (Grand Prix/Festival/Musical/Comedy), then, only for what those cannot recognize, asking an AI model to actually identify the team or artist by name (needs an Anthropic API key embedded in this build - if none is set up, that second step is simply skipped). An event that already has a category, however it got one, is never touched or changed - so this is always safe to run again later."
        confirmLabel="Detect categories"
        busy={detectingCategories}
        onCancel={() => setConfirmDetectCategories(false)}
        onConfirm={runDetectCategories}
      />
    </div>
  );
}

export function EventFormModal({
  open,
  initial,
  onClose,
  onSaved,
}: {
  open: boolean;
  initial: EventWithStats | null;
  onClose: () => void;
  onSaved: (id: number) => void;
}) {
  const toast = useToast();
  const [form, setForm] = useState<EventInput>(EMPTY_INPUT);
  // 2.0.27: fetched fresh every time the modal opens, same "own independent
  // fetch, not shared with the list page's filter dropdown" convention as
  // Orders/Sales' own LookupSelect-backed pickers (e.g. OrderFormModal's own
  // api.listPlatforms() call, separate from Orders.tsx's page-level one).
  const [categories, setCategories] = useState<EventCategory[]>([]);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (open) {
      api.listEventCategories().then(setCategories).catch(() => {});
      setForm(
        initial
          ? {
              name: initial.name,
              artistTeam: null,
              venue: initial.venue ?? "",
              city: initial.city ?? "",
              country: initial.country ?? "",
              eventDate: initial.eventDate ?? "",
              categoryId: initial.categoryId,
              status: initial.status,
              notes: initial.notes ?? "",
            }
          : EMPTY_INPUT,
      );
      setError(null);
    }
  }, [open, initial]);

  const submit = async () => {
    if (!form.name.trim()) {
      setError("Event name is required");
      return;
    }
    setSaving(true);
    setError(null);
    try {
      if (initial) {
        await api.updateEvent(initial.id, form);
        toast.success("Event updated");
        onSaved(initial.id);
      } else {
        const created = await api.createEvent(form);
        toast.success("Event created");
        onSaved(created.id);
      }
    } catch (e) {
      setError(errMsg(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal open={open} onClose={onClose} title={initial ? "Edit event" : "New event"}>
      {/* 1.9.10: reordered per marko's spec - was Event name / [Category,
          Venue] / [Event date, City] / Country (full) / Status (full) /
          Notes (full), 6 rows. Now Event name / [Category, Country] /
          [Event date, City] / [Status, Venue] / Notes, 5 rows - Country
          moved up to pair with Category, Status and Venue now pair with
          each other instead of each sitting alone on a full-width row.
          Event date/City and Notes are unchanged. No fields added or
          removed, purely a layout reorder. */}
      <div className="grid grid-cols-2 gap-4">
        <div className="col-span-2">
          <Field label="Event name" required>
            <Input value={form.name} onChange={(e) => setForm({ ...form, name: e.target.value })} autoFocus />
          </Field>
        </div>
        <LookupSelect
          label="Category"
          options={categories}
          value={form.categoryId ?? null}
          onChange={(id) => setForm({ ...form, categoryId: id })}
          onCreate={async (name) => {
            const c = await api.createEventCategory(name);
            setCategories((prev) => [...prev, c]);
            return c;
          }}
          placeholder="No category"
        />
        <Field label="Country">
          <Input value={form.country ?? ""} onChange={(e) => setForm({ ...form, country: e.target.value })} />
        </Field>
        <Field label="Event date">
          <Input
            type="date"
            value={form.eventDate ?? ""}
            onChange={(e) => setForm({ ...form, eventDate: e.target.value })}
          />
        </Field>
        <Field label="City">
          <Input value={form.city ?? ""} onChange={(e) => setForm({ ...form, city: e.target.value })} />
        </Field>
        <Field label="Status">
          <Select
            value={form.status ?? "upcoming"}
            onChange={(e) => setForm({ ...form, status: e.target.value as EventStatus })}
          >
            <option value="upcoming">Upcoming</option>
            <option value="completed">Completed</option>
            <option value="cancelled">Cancelled</option>
          </Select>
        </Field>
        <Field label="Venue">
          <Input value={form.venue ?? ""} onChange={(e) => setForm({ ...form, venue: e.target.value })} />
        </Field>
        <div className="col-span-2">
          <Field label="Notes">
            <Textarea
              rows={3}
              value={form.notes ?? ""}
              onChange={(e) => setForm({ ...form, notes: e.target.value })}
            />
          </Field>
        </div>
      </div>
      {error && <p className="mt-3 text-sm text-red-600 dark:text-red-400">{error}</p>}
      <ModalFooter>
        <Button variant="secondary" onClick={onClose} disabled={saving}>
          Cancel
        </Button>
        <Button variant="primary" onClick={submit} disabled={saving}>
          {saving ? "Saving..." : initial ? "Save changes" : "Create event"}
        </Button>
      </ModalFooter>
    </Modal>
  );
}

import { useEffect, useState, type ReactNode } from "react";
import { Link } from "react-router-dom";
import { api, errMsg } from "../lib/api";
import type {
  OrderRecord,
  Platform,
  Pull,
  PullEditInput,
  PullInput,
  PullReceived,
  PullReceivedEditInput,
  PullReceivedInput,
} from "../lib/types";
import {
  centsToDecimalString,
  decimalStringToCents,
  formatDate,
  formatDateNumeric,
  formatMoney,
  formatSeatsSummary,
  summarizeBulkDeleteSkips,
  todayIso,
} from "../lib/format";
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
  Textarea,
} from "../components/ui";
import { LookupSelect } from "../components/LookupSelect";
import { IconAlertTriangle, IconLink, IconPlus, IconSearch, IconTrash, IconUsers } from "../components/icons";
import { useToast } from "../lib/toast";
import { useNarrowTables } from "../lib/useNarrowTables";
import { completionStatus } from "../lib/completion";

const CURRENCIES = ["EUR", "USD", "GBP", "CHF", "CZK", "PLN", "HUF", "SEK", "NOK", "DKK", "RON", "TRY", "BGN"];

// 2.0.66: the new "Completed" indicator (see REDESIGN-2.0.66-REPORT.md),
// unified in style with Orders/Sales - but Pulls (Given) only has ONE
// underlying condition (transferDone), unlike those pages' 3. Pulls
// (Received) has no completion-like field at all yet, so it doesn't get this
// column - see the report's own note asking marko what that should mean.
function pullCompletionChecks(p: Pull) {
  return [{ label: "Transferred", done: p.transferDone }];
}

// Same safety-cap convention as Orders.tsx/Tickets.tsx/Sales.tsx - mirrors
// the backend's own LIST_CAP (commands/pulls.rs, commands/pulls_received.rs)
// so the banner below only ever shows once the backend has actually
// truncated the results.
const LIST_CAP = 5000;

// Session-only "remember the last search" convention, same as Orders.tsx's
// lastOrdersSearch / Sales.tsx's lastFilters - resets on app restart. Each
// category keeps its own remembered search, since they're two unrelated lists.
let lastPullsSearch: string | null = null;
let lastPullsReceivedSearch: string | null = null;
// 2.0.37: same session-only convention, now for the new Sort control on
// each tab (marko asked for the same Sort control Sales/Orders/Events
// already have, added everywhere it was still missing - Tickets/Inventory,
// and both of these tabs). Sorted client-side, same reasoning as Orders.tsx's
// own sortedOrders: listPulls/listPullsReceived already return the full
// matching result set in one response (up to LIST_CAP), so sorting what's
// already in memory is exactly as complete as a backend sort would be. Two
// separate variables, matching lastPullsSearch/lastPullsReceivedSearch above -
// Given and Received are unrelated lists with their own sort preference.
//
// 2.0.65: two real fixes, not just a relabel. (1) "Newest/Oldest" renamed to
// the app-wide "Soonest/Furthest first" standard, default flipped to
// ascending - same change as Orders/Tickets/Sales/Events. (2) a genuine bug
// fix: both tabs used to sort by `createdAt` (when the row was typed into
// the app) while the table's own displayed "Date" column shows `eventDate` -
// two unrelated fields, so "Newest first" produced an order that didn't
// match what was on screen at all. Both tabs now sort by `eventDate`
// itself, client-side, null-safe, mirroring Events.tsx's own sortedEvents
// exactly (events/pulls with no date at all always sort last, in either
// direction) - see each tab's own sortedPulls for the actual comparator.
let lastPullsSortBy: string = "";
let lastPullsReceivedSortBy: string = "";
const PULL_SORT_LABELS: Record<string, string> = {
  "": "Soonest first",
  furthest: "Furthest first",
};

// 1.9.8: how many days before the event the "transfer this!" warning starts
// showing (and keeps showing every day, escalating once the event date
// itself has passed) - replaces the old manual "Transfer deadline" field
// marko had to fill in by hand. Local to this file only. Given-pulls only -
// see this file's module doc comment for why received pulls have no
// equivalent "transfer" concept.
const WARNING_WINDOW_DAYS = 3;

type TransferFilter = "all" | "pending" | "done";
type PullCategory = "given" | "received";

/** Whole days between today and `dateIso` (positive = in the future,
 * negative = already passed). Plain calendar-day difference, not
 * time-of-day-sensitive - matches how `eventDate`/`todayIso()` are always
 * plain "YYYY-MM-DD" strings in this app. */
function daysUntil(dateIso: string): number {
  const start = new Date(`${todayIso()}T00:00:00`);
  const end = new Date(dateIso.length <= 10 ? `${dateIso}T00:00:00` : dateIso);
  return Math.round((end.getTime() - start.getTime()) / 86_400_000);
}

function warningLabel(daysLeft: number): string {
  if (daysLeft > 0) return `${daysLeft}d left`;
  if (daysLeft === 0) return "Today!";
  return `${Math.abs(daysLeft)}d overdue`;
}

/** Pulls (1.9.7, category toggle added 2.0.17): a toggle between pulls marko
 * did FOR other people ("Given" - the original feature, unchanged) and pulls
 * marko TOOK FROM other people ("Received" - 2.0.17). The two directions
 * share almost no fields and no lifecycle (Given pulls never become marko's
 * own inventory and track a "transfer done" deadline; Received pulls DO
 * become his own inventory - see PullReceived's shape in types.ts - can
 * optionally link to the resulting Order, and can be auto-created by Orders
 * & Sales sheet sync whenever a synced row's "pull" column says "yes"), so
 * this is two entirely separate lists/forms/tables under one shared toggle,
 * rather than one table with a filter - marko's own 2.0.17 request. */
export default function Pulls() {
  const [category, setCategory] = useState<PullCategory>("given");

  return (
    <div>
      <PageHeader
        title="Pulls"
        subtitle={
          category === "given"
            ? "Tickets bought on someone else's behalf for a fee - queue, pay, transfer, get paid."
            : "Tickets someone else pulled for you - who pulled them, what you paid, and which order they became."
        }
        actions={
          <div className="inline-flex rounded-lg border border-slate-200 dark:border-slate-800 p-0.5">
            {(["given", "received"] as const).map((c) => (
              <button
                key={c}
                type="button"
                onClick={() => setCategory(c)}
                className={`rounded-md px-3 py-1.5 text-sm font-medium transition-colors ${
                  category === c
                    ? "bg-brand-600 text-white"
                    : "text-slate-500 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-100"
                }`}
              >
                {c === "given" ? "Given" : "Received"}
              </button>
            ))}
          </div>
        }
      />
      {category === "given" ? <GivenPulls /> : <ReceivedPulls />}
    </div>
  );
}

/** Same lightweight section-grouping helper as Orders.tsx's own local
 * FormGroup - kept local here too rather than promoted to ui.tsx, same
 * reasoning as that file's comment (still only a handful of forms in the app
 * want this kind of sectioning). Shared by both PullFormModal and
 * PullReceivedFormModal below. */
function FormGroup({ title, children }: { title?: string; children: ReactNode }) {
  return (
    <div className="border-t border-slate-200 pt-4 first:border-t-0 first:pt-0 dark:border-slate-800">
      {title && (
        <p className="mb-3 text-xs font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">{title}</p>
      )}
      <div className="grid grid-cols-2 gap-4">{children}</div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Given pulls (1.9.7) - the original feature, unchanged by 2.0.17 beyond
// being moved into its own component and gaining a "New Pull" button next to
// its search/filter row instead of in a page-level header (now shared with
// Received pulls above). A pull has no child entities (no tickets are
// generated) so there's no separate Detail page - this owns both create and
// edit via one shared PullFormModal below.
// ---------------------------------------------------------------------------

function GivenPulls() {
  const toast = useToast();
  const isNarrow = useNarrowTables();
  const [pulls, setPulls] = useState<Pull[] | null>(null);
  const [platforms, setPlatforms] = useState<Platform[]>([]);
  const [search, setSearch] = useState(lastPullsSearch ?? "");
  const [statusFilter, setStatusFilter] = useState<TransferFilter>("all");
  // 2.0.65: Platform + date-range filters, matching Orders/Tickets - new
  // list_pulls_impl params added specifically for this (see pulls.rs).
  const [platformId, setPlatformId] = useState<number | "">("");
  const [dateFrom, setDateFrom] = useState("");
  const [dateTo, setDateTo] = useState("");
  const [sortBy, setSortBy] = useState(lastPullsSortBy);
  // undefined = modal closed, null = create mode, a Pull = edit mode.
  const [modalPull, setModalPull] = useState<Pull | null | undefined>(undefined);
  const [deleteTarget, setDeleteTarget] = useState<Pull | null>(null);
  const [deleting, setDeleting] = useState(false);
  // 2.0.28: bulk-delete selection mode - marko's own request. No checkbox
  // column sitting there all the time; the "Delete" toggle button below
  // reveals it, and it disappears again the moment you confirm or cancel.
  const [selectionMode, setSelectionMode] = useState(false);
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [confirmBulkDelete, setConfirmBulkDelete] = useState(false);
  const [bulkDeleting, setBulkDeleting] = useState(false);

  useEffect(() => {
    lastPullsSearch = search;
  }, [search]);

  useEffect(() => {
    lastPullsSortBy = sortBy;
  }, [sortBy]);

  useEffect(() => {
    api.listPlatforms().then(setPlatforms).catch(() => {});
  }, []);

  const load = (q?: string, filter?: TransferFilter) => {
    const f = filter ?? statusFilter;
    api
      .listPulls({
        search: q || undefined,
        transferDone: f === "all" ? undefined : f === "done",
        platformId: platformId || undefined,
        dateFrom: dateFrom || undefined,
        dateTo: dateTo || undefined,
      })
      .then(setPulls)
      .catch((e) => toast.error(errMsg(e)));
  };

  // 2.0.65: sorts by `eventDate` (what the table's own "Date" column shows),
  // not `createdAt` - see PULL_SORT_LABELS' own comment above for why this
  // is a genuine bug fix, not just a relabel. Both directions are computed
  // here, client-side - unlike Events.tsx, the backend's own order
  // (created_at DESC) isn't a usable stand-in for "furthest by event date",
  // so there's no pass-through branch to reuse here. No-date pulls always
  // sort last regardless of direction, same rule as Events.tsx.
  const sortedPulls: Pull[] = (() => {
    if (pulls === null) return [];
    const withDate = pulls.filter((p) => p.eventDate !== null);
    const withoutDate = pulls.filter((p) => p.eventDate === null);
    const dir = sortBy === "furthest" ? -1 : 1;
    withDate.sort((a, b) => dir * ((a.eventDate as string).localeCompare(b.eventDate as string) || a.id - b.id));
    return [...withDate, ...withoutDate];
  })();

  const toggleOne = (id: number) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const allSelected = pulls !== null && pulls.length > 0 && pulls.every((p) => selected.has(p.id));
  const toggleSelectAll = () => {
    setSelected(allSelected ? new Set() : new Set((pulls ?? []).map((p) => p.id)));
  };

  const exitSelectionMode = () => {
    setSelectionMode(false);
    setSelected(new Set());
  };

  const confirmDeleteSelected = async () => {
    setBulkDeleting(true);
    try {
      const result = await api.bulkDeletePulls(Array.from(selected));
      if (result.deletedIds.length > 0) {
        toast.success(`${result.deletedIds.length} pull${result.deletedIds.length === 1 ? "" : "s"} deleted`);
      }
      if (result.skipped.length > 0) {
        toast.error(`${result.skipped.length} skipped: ${summarizeBulkDeleteSkips(result.skipped)}`);
      }
      setConfirmBulkDelete(false);
      exitSelectionMode();
      load(search);
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setBulkDeleting(false);
    }
  };

  useEffect(() => {
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const t = setTimeout(() => load(search), 250);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [search, statusFilter, platformId, dateFrom, dateTo]);

  const confirmDelete = async () => {
    if (!deleteTarget) return;
    setDeleting(true);
    try {
      await api.deletePull(deleteTarget.id);
      toast.success(`Pull ${deleteTarget.code} deleted`);
      setDeleteTarget(null);
      setModalPull(undefined);
      load(search);
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setDeleting(false);
    }
  };

  const toggleTransferDone = async (p: Pull) => {
    try {
      const updated = await api.setPullTransferDone(p.id, !p.transferDone);
      setPulls((prev) => (prev ? prev.map((x) => (x.id === updated.id ? updated : x)) : prev));
    } catch (e) {
      toast.error(errMsg(e));
    }
  };

  return (
    <>
      {/* 2.0.65: buttons moved to their own row, filters below now follow
          the same items-end + labeled w-NN convention as Orders/Tickets/
          Sales/Events - marko asked for filters/sort to look "roughly the
          same everywhere"; this tab (and Received below) were the one
          layout left over from before that convention existed. */}
      <div className="mb-3 flex justify-end gap-2">
        {!selectionMode && pulls && pulls.length > 0 && (
          <Button variant="secondary" onClick={() => setSelectionMode(true)}>
            <IconTrash className="h-4 w-4" /> Delete
          </Button>
        )}
        <Button variant="primary" onClick={() => setModalPull(null)}>
          <IconPlus className="h-4 w-4" /> New Pull
        </Button>
      </div>

      <div className="mb-4 flex flex-wrap items-end gap-3">
        <div className="w-52">
          <span className="label">Search</span>
          <div className="relative">
            <IconSearch className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-400 dark:text-slate-500" />
            <Input
              placeholder="Search pulls..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="pl-9"
            />
          </div>
        </div>
        <div className="w-44">
          <span className="label">Status</span>
          <Select value={statusFilter} onChange={(e) => setStatusFilter(e.target.value as TransferFilter)}>
            <option value="all">All pulls</option>
            <option value="pending">Not transferred yet</option>
            <option value="done">Transferred</option>
          </Select>
        </div>
        <div className="w-40">
          <span className="label">Platform</span>
          <Select value={platformId} onChange={(e) => setPlatformId(e.target.value ? Number(e.target.value) : "")}>
            <option value="">All platforms</option>
            {/* A pull is buying tickets on someone else's behalf, i.e.
                marko's own purchase side - same purchase/both scoping as
                every other purchase-side Platform picker in the app. */}
            {platforms
              .filter((p) => p.kind === "purchase" || p.kind === "both")
              .map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
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
          <span className="label">Sort</span>
          <Select value={sortBy} onChange={(e) => setSortBy(e.target.value)} aria-label="Sort pulls">
            {Object.entries(PULL_SORT_LABELS).map(([value, label]) => (
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
          itemLabel="pull"
          busy={bulkDeleting}
          onConfirm={() => setConfirmBulkDelete(true)}
          onCancel={exitSelectionMode}
        />
      )}

      {pulls && pulls.length >= LIST_CAP && (
        <div className="mb-4 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-800 dark:border-amber-500/30 dark:bg-amber-500/10 dark:text-amber-400">
          Showing the most recent {LIST_CAP.toLocaleString()} pulls that match your filters. Narrow the search to see
          the rest.
        </div>
      )}

      {pulls === null ? (
        <LoadingBlock />
      ) : pulls.length === 0 ? (
        <EmptyState
          icon={<IconUsers className="h-8 w-8" />}
          title="No pulls yet"
          description="Record a pull when you queue up to buy tickets on someone else's behalf."
          action={
            <Button variant="primary" onClick={() => setModalPull(null)}>
              <IconPlus className="h-4 w-4" /> New Pull
            </Button>
          }
        />
      ) : (
        // Same table-fixed + colgroup convention as Orders.tsx/Sales.tsx.
        // Seats shows via formatSeatsSummary - a Pull is always exactly one
        // section/row/seat triple (never a list, see Pull's own DB columns),
        // so it's wrapped as a one-element array and formatted with the same
        // compact "204/AA 128" shorthand as the new Orders/Tickets/Sales
        // Seats column (falls back to "General admission" when Section/Row/
        // Seat are all blank) - 2.0.38: replaces the older, more verbose
        // formatSeatLocation ("Sec 204 · Row AA · Seat 128") this cell used
        // to call, both for consistency with the new column elsewhere and
        // because the shorter form is a big part of what makes this table's
        // narrow mode fit at all (see PROTECTED-AREAS-NOTES.md's 2.0.38
        // section). The "Deadline" column is a warning that appears
        // automatically starting WARNING_WINDOW_DAYS before the event date
        // and disappears the moment transfer is marked done. Row click
        // opens the edit modal (no separate Detail page exists for Pull,
        // unlike Order/Sale) - guarded so a click on the checkbox doesn't
        // also open it. 2.0.38: Date uses formatDateNumeric ("11.09.2026")
        // instead of the old abbreviated formatDateCompact ("13 Aug 26") -
        // marko explicitly asked for the full 4-digit year everywhere - and
        // drops the old title tooltip (it existed only to spell out what the
        // abbreviated form hid; formatDateNumeric is already the full,
        // unambiguous date, so there's nothing left for a hover to add).
        //
        // 2.0.37: this used to be the one table in the app that accepted
        // needing horizontal scroll below ~1100px (marko's own explicit
        // tradeoff at the time, since Seats/More info needed to show in
        // full) - both the wrapper's max-w-[1400px] and the table's own
        // min-w-[1220px] are gone now, replaced with the same pure-
        // percentage, two-mode model every other table in the app uses:
        // below the shared useNarrowTables() breakpoint (1649px window as
        // of 2.0.38, was 1690px), Warning and Platform hide (Seats/More
        // info/the money and pull-code columns that matter most stay) and
        // everything else grows a little and switches to the smaller
        // .th-c-narrow/.td-c-narrow. Verified (Playwright, real cell DOM
        // measured against actual rendered content - real code samples from
        // src-tauri/src/codes.rs, real Intl.NumberFormat/date data across
        // en-US/sk-SK/de-DE, not just header text) to fit without scrolling
        // or wrapping all the way down to 1080px, this app's enforced
        // minimum window width - see Sales.tsx's own colgroup comment and
        // PROTECTED-AREAS-NOTES.md (2.0.38 section) for the full reasoning.
        <div className="overflow-x-auto rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-sm">
          <table className="w-full table-fixed border-collapse">
            {isNarrow ? (
              <colgroup>
                {selectionMode && <col className="w-8" />}
                <col className="w-[10.732%]" />
                <col className="w-[9.146%]" />
                <col className="w-[15.634%]" />
                <col className="w-[8.659%]" />
                <col className="w-[9.634%]" />
                <col className="w-[17.317%]" />
                <col className="w-[4.39%]" />
                <col className="w-[10%]" />
                <col className="w-[5.488%]" />
                {/* 2.0.66: new "Completed" column - width is my own estimate
                    (not measured against real content like the rest of this
                    colgroup), taken entirely from Event's share above. Flag
                    to marko if this looks visually off. */}
                <col className="w-[9%]" />
              </colgroup>
            ) : (
              <colgroup>
                {selectionMode && <col className="w-8" />}
                <col className="w-[8.375%]" />
                <col className="w-[10.22%]" />
                <col className="w-[11.527%]" />
                <col className="w-[6.884%]" />
                <col className="w-[6.175%]" />
                <col className="w-[14.123%]" />
                <col className="w-[3.691%]" />
                <col className="w-[11.781%]" />
                <col className="w-[7.807%]" />
                <col className="w-[8.943%]" />
                <col className="w-[3.974%]" />
                {/* 2.0.66: see the narrow colgroup's identical comment above. */}
                <col className="w-[6.5%]" />
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
                      aria-label="Select all pulls"
                    />
                  </th>
                )}
                <th className={isNarrow ? "th-c-narrow" : "th-c"}>Pull</th>
                <th className={isNarrow ? "th-c-narrow" : "th-c"}>For</th>
                <th className={isNarrow ? "th-c-narrow" : "th-c"}>Event</th>
                <th className={isNarrow ? "th-c-narrow" : "th-c"}>Date</th>
                <th className={isNarrow ? "th-c-narrow" : "th-c"}>Seats</th>
                <th className={isNarrow ? "th-c-narrow" : "th-c"}>More info</th>
                <th className={`${isNarrow ? "th-c-narrow" : "th-c"} text-right`}>Ks</th>
                {!isNarrow && <th className="th-c">Platform</th>}
                <th className={`${isNarrow ? "th-c-narrow" : "th-c"} text-right`}>Fee</th>
                {!isNarrow && <th className="th-c">Warning</th>}
                <th className={`${isNarrow ? "th-c-narrow" : "th-c"} text-center`}>Done</th>
                <th className={isNarrow ? "th-c-narrow" : "th-c"}>Completed</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
              {sortedPulls.map((p) => {
                const daysLeft = p.eventDate ? daysUntil(p.eventDate) : null;
                const seatsSummary = formatSeatsSummary([{ section: p.section, rowLabel: p.rowLabel, seat: p.seat }]);
                const showWarning = !p.transferDone && daysLeft !== null && daysLeft <= WARNING_WINDOW_DAYS;
                const warningText = daysLeft !== null ? warningLabel(daysLeft) : "";
                const warningTone = daysLeft !== null && daysLeft <= 0 ? "red" : "amber";
                return (
                  <tr
                    key={p.id}
                    className="cursor-pointer hover:bg-slate-50 dark:hover:bg-slate-800/60"
                    onClick={(e) => {
                      if ((e.target as HTMLElement).closest("input, button")) return;
                      if (selectionMode) {
                        toggleOne(p.id);
                        return;
                      }
                      setModalPull(p);
                    }}
                  >
                    {selectionMode && (
                      <td className={isNarrow ? "td-c-narrow" : "td-c"}>
                        <input
                          type="checkbox"
                          className={CHECKBOX_CLASS}
                          checked={selected.has(p.id)}
                          onChange={() => toggleOne(p.id)}
                          aria-label={`Select pull ${p.code}`}
                        />
                      </td>
                    )}
                    <td
                      className={`${isNarrow ? "td-c-narrow" : "td-c"} truncate font-medium text-slate-900 dark:text-slate-100`}
                      title={`Added ${formatDate(p.createdAt)}`}
                    >
                      {p.code}
                    </td>
                    <td className={`${isNarrow ? "td-c-narrow" : "td-c"} truncate`} title={p.buyerName}>
                      {p.buyerName}
                    </td>
                    <td className={`${isNarrow ? "td-c-narrow" : "td-c"} truncate`} title={p.eventName}>
                      {p.eventName}
                    </td>
                    <td className={`${isNarrow ? "td-c-narrow" : "td-c"} whitespace-nowrap`}>
                      {formatDateNumeric(p.eventDate)}
                    </td>
                    <td
                      className={`${isNarrow ? "td-c-narrow" : "td-c"} truncate text-xs text-slate-500 dark:text-slate-400`}
                      title={seatsSummary}
                    >
                      {seatsSummary}
                    </td>
                    <td
                      className={`${isNarrow ? "td-c-narrow" : "td-c"} truncate text-xs text-slate-500 dark:text-slate-400`}
                      title={p.moreInfo ?? undefined}
                    >
                      {p.moreInfo || "-"}
                    </td>
                    <td className={`${isNarrow ? "td-c-narrow" : "td-c"} text-right tabular-nums whitespace-nowrap`}>{p.quantity}</td>
                    {!isNarrow && (
                      <td className="td-c truncate" title={p.platformName ?? undefined}>
                        {p.platformName ?? "-"}
                      </td>
                    )}
                    <td className={`${isNarrow ? "td-c-narrow" : "td-c"} text-right tabular-nums whitespace-nowrap`}>{formatMoney(p.priceCents, p.currency)}</td>
                    {!isNarrow && (
                      <td className="td-c">
                        {showWarning && (
                          <span
                            className={`inline-flex items-center gap-1 whitespace-nowrap text-xs font-medium ${
                              warningTone === "red" ? "text-red-600 dark:text-red-400" : "text-amber-600 dark:text-amber-400"
                            }`}
                          >
                            <IconAlertTriangle className="h-3.5 w-3.5 shrink-0" />
                            {warningText}
                          </span>
                        )}
                      </td>
                    )}
                    <td className={`${isNarrow ? "td-c-narrow" : "td-c"} text-center`}>
                      <input
                        type="checkbox"
                        className={CHECKBOX_CLASS}
                        checked={p.transferDone}
                        onChange={() => toggleTransferDone(p)}
                        aria-label={`Mark pull ${p.code} as ${p.transferDone ? "not transferred" : "transferred"}`}
                      />
                    </td>
                    <td className={isNarrow ? "td-c-narrow" : "td-c"}>
                      {(() => {
                        const c = completionStatus(pullCompletionChecks(p));
                        return (
                          <Badge tone={c.tone} title={c.title}>
                            {c.label}
                          </Badge>
                        );
                      })()}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}

      <PullFormModal
        open={modalPull !== undefined}
        pull={modalPull ?? null}
        onClose={() => setModalPull(undefined)}
        onSaved={() => {
          setModalPull(undefined);
          load(search);
        }}
        onRequestDelete={(p) => setDeleteTarget(p)}
      />

      <ConfirmDialog
        open={!!deleteTarget}
        title="Delete this pull?"
        message={
          <>
            This removes <strong>{deleteTarget?.code}</strong> ({deleteTarget?.buyerName} - {deleteTarget?.eventName})
            permanently. This can't be undone.
          </>
        }
        confirmLabel="Delete"
        danger
        busy={deleting}
        onConfirm={confirmDelete}
        onCancel={() => setDeleteTarget(null)}
      />

      <ConfirmDialog
        open={confirmBulkDelete}
        title={`Delete ${selected.size} selected pull${selected.size === 1 ? "" : "s"}?`}
        message="This removes the selected pulls permanently. This can't be undone."
        confirmLabel="Delete selected"
        danger
        busy={bulkDeleting}
        onCancel={() => setConfirmBulkDelete(false)}
        onConfirm={confirmDeleteSelected}
      />
    </>
  );
}

function PullFormModal({
  open,
  pull,
  onClose,
  onSaved,
  onRequestDelete,
}: {
  open: boolean;
  pull: Pull | null;
  onClose: () => void;
  onSaved: (pull: Pull) => void;
  onRequestDelete: (pull: Pull) => void;
}) {
  const toast = useToast();
  const editing = pull !== null;
  const [platforms, setPlatforms] = useState<Platform[]>([]);

  const [buyerName, setBuyerName] = useState("");
  const [eventName, setEventName] = useState("");
  const [eventDate, setEventDate] = useState("");
  const [quantity, setQuantity] = useState("1");
  const [platformId, setPlatformId] = useState<number | null>(null);
  const [section, setSection] = useState("");
  const [rowLabel, setRowLabel] = useState("");
  const [seat, setSeat] = useState("");
  const [moreInfo, setMoreInfo] = useState("");
  const [price, setPrice] = useState("");
  const [currency, setCurrency] = useState("EUR");
  const [customCurrency, setCustomCurrency] = useState(false);
  const [transferDone, setTransferDone] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!open) return;
    api.listPlatforms().then(setPlatforms).catch(() => {});
    if (pull) {
      setBuyerName(pull.buyerName);
      setEventName(pull.eventName);
      setEventDate(pull.eventDate ?? "");
      setQuantity(String(pull.quantity));
      setPlatformId(pull.platformId);
      setSection(pull.section ?? "");
      setRowLabel(pull.rowLabel ?? "");
      setSeat(pull.seat ?? "");
      setMoreInfo(pull.moreInfo ?? "");
      setPrice(centsToDecimalString(pull.priceCents));
      setCurrency(pull.currency);
      setCustomCurrency(!CURRENCIES.includes(pull.currency));
      setTransferDone(pull.transferDone);
    } else {
      setBuyerName("");
      setEventName("");
      setEventDate("");
      setQuantity("1");
      setPlatformId(null);
      setSection("");
      setRowLabel("");
      setSeat("");
      setMoreInfo("");
      setPrice("");
      setCurrency("EUR");
      setCustomCurrency(false);
      setTransferDone(false);
    }
    setError(null);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, pull]);

  const submit = async () => {
    setError(null);
    const q = parseInt(quantity, 10);
    const priceCents = decimalStringToCents(price);

    if (!buyerName.trim()) return setError("Buyer name is required");
    if (!eventName.trim()) return setError("Event name is required");
    if (!Number.isFinite(q) || q < 1) return setError("Quantity must be at least 1");
    if (priceCents === null) return setError("Fee is not a valid amount");
    if (!currency.trim()) return setError("Currency is required");

    setSaving(true);
    try {
      if (editing && pull) {
        const input: PullEditInput = {
          buyerName: buyerName.trim(),
          eventName: eventName.trim(),
          eventDate: eventDate || null,
          quantity: q,
          platformId,
          section: section.trim() || null,
          rowLabel: rowLabel.trim() || null,
          seat: seat.trim() || null,
          moreInfo: moreInfo.trim() || null,
          priceCents,
          currency,
          transferDone,
        };
        const updated = await api.updatePull(pull.id, input);
        toast.success(`Pull ${updated.code} updated`);
        onSaved(updated);
      } else {
        const input: PullInput = {
          buyerName: buyerName.trim(),
          eventName: eventName.trim(),
          eventDate: eventDate || null,
          quantity: q,
          platformId,
          section: section.trim() || null,
          rowLabel: rowLabel.trim() || null,
          seat: seat.trim() || null,
          moreInfo: moreInfo.trim() || null,
          priceCents,
          currency,
        };
        const created = await api.createPull(input);
        toast.success(`Pull ${created.code} created`);
        onSaved(created);
      }
    } catch (e) {
      setError(errMsg(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal open={open} onClose={onClose} title={editing ? `Edit ${pull?.code}` : "New pull"} width="max-w-2xl">
      <div className="flex flex-col gap-4">
        <FormGroup title="Pull">
          <Field label="For (buyer)" required hint="Who you're pulling these tickets for">
            <Input value={buyerName} onChange={(e) => setBuyerName(e.target.value)} />
          </Field>
          <Field label="Quantity" required>
            <Input type="number" min={1} step={1} value={quantity} onChange={(e) => setQuantity(e.target.value)} />
          </Field>
          <Field label="Event name" required hint="Free text - not linked to your Events list">
            <Input value={eventName} onChange={(e) => setEventName(e.target.value)} />
          </Field>
          <Field label="Event date">
            <Input type="date" value={eventDate} onChange={(e) => setEventDate(e.target.value)} />
          </Field>
          <div className="col-span-2">
            <LookupSelect
              label="Platform"
              options={platforms.filter((p) => p.kind === "purchase" || p.kind === "both")}
              value={platformId}
              onChange={setPlatformId}
              onCreate={async (name) => {
                const p = await api.createPlatform(name, "purchase");
                setPlatforms((prev) => [...prev, p]);
                return p;
              }}
            />
          </div>
        </FormGroup>

        <FormGroup title="Seats & details">
          <Field label="Section" hint="Optional - leave blank for general admission">
            <Input value={section} onChange={(e) => setSection(e.target.value)} />
          </Field>
          <Field label="Row" hint="Optional">
            <Input value={rowLabel} onChange={(e) => setRowLabel(e.target.value)} />
          </Field>
          <Field label="Seat" hint="Optional">
            <Input value={seat} onChange={(e) => setSeat(e.target.value)} />
          </Field>
          <Field label="More info" hint="Optional - e.g. the email the tickets will arrive on">
            <Textarea rows={2} value={moreInfo} onChange={(e) => setMoreInfo(e.target.value)} />
          </Field>
        </FormGroup>

        <FormGroup title="Fee & tracking">
          <Field label={`Your fee (${currency})`} required hint="What you earn for the pull - not the ticket price">
            <Input inputMode="decimal" placeholder="0.00" value={price} onChange={(e) => setPrice(e.target.value)} />
          </Field>
          <div>
            <div className="flex items-center justify-between">
              <span className="label mb-1">Currency</span>
              <button
                type="button"
                className="mb-1 text-xs font-medium text-brand-600 dark:text-brand-400 hover:underline"
                onClick={() => setCustomCurrency((c) => !c)}
              >
                {customCurrency ? "Choose from list" : "Other..."}
              </button>
            </div>
            {customCurrency ? (
              <Input
                autoFocus
                placeholder="e.g. AED"
                value={currency}
                onChange={(e) => setCurrency(e.target.value.toUpperCase())}
              />
            ) : (
              <Select value={currency} onChange={(e) => setCurrency(e.target.value)}>
                {(CURRENCIES.includes(currency) ? CURRENCIES : [currency, ...CURRENCIES]).map((c) => (
                  <option key={c} value={c}>
                    {c}
                  </option>
                ))}
              </Select>
            )}
          </div>
          <div className="col-span-2">
            {editing ? (
              <label className="flex items-center gap-2">
                <input
                  type="checkbox"
                  className={CHECKBOX_CLASS}
                  checked={transferDone}
                  onChange={(e) => setTransferDone(e.target.checked)}
                />
                <span className="text-sm text-slate-700 dark:text-slate-300">Transfer done</span>
              </label>
            ) : (
              <p className="text-xs text-slate-400 dark:text-slate-500">
                A warning appears automatically starting {WARNING_WINDOW_DAYS} days before the event date, and every
                day after that, until this pull is marked as transferred.
              </p>
            )}
          </div>
        </FormGroup>
      </div>

      {error && <p className="mt-3 text-sm text-red-600 dark:text-red-400">{error}</p>}
      <ModalFooter>
        {editing && pull && (
          <Button variant="danger" className="mr-auto" onClick={() => onRequestDelete(pull)} disabled={saving}>
            Delete
          </Button>
        )}
        <Button variant="secondary" onClick={onClose} disabled={saving}>
          Cancel
        </Button>
        <Button variant="primary" onClick={submit} disabled={saving}>
          {saving ? "Saving..." : editing ? "Save changes" : "Create pull"}
        </Button>
      </ModalFooter>
    </Modal>
  );
}

// ---------------------------------------------------------------------------
// Received pulls (2.0.17) - the mirror direction: someone ELSE pulls tickets
// FOR marko, marko pays them a fee, and (unlike Given pulls) the tickets DO
// become marko's own inventory - so a row here can optionally link to the
// resulting Order. Also auto-created by Orders & Sales sheet sync whenever a
// synced row's "pull" column says "yes" (source shows as "Synced" below) -
// see src-tauri/commands/orders_sheet_sync.rs::maybe_link_pull_received.
// Structurally mirrors GivenPulls/PullFormModal above (list + one shared
// create/edit modal, no separate Detail page), minus the concepts that don't
// carry over: no seats/platform (not tracked for this direction), no
// "transfer done" warning (nothing is being transferred BY marko here).
// ---------------------------------------------------------------------------

function ReceivedPulls() {
  const toast = useToast();
  const isNarrow = useNarrowTables();
  const [pulls, setPulls] = useState<PullReceived[] | null>(null);
  const [search, setSearch] = useState(lastPullsReceivedSearch ?? "");
  // 2.0.65: date-range filter, matching Given Pulls/Orders/Tickets. No
  // Platform filter here - pulls_received has no platform column of its own
  // (see pulls_received.rs's list_pulls_received_impl doc comment).
  const [dateFrom, setDateFrom] = useState("");
  const [dateTo, setDateTo] = useState("");
  const [sortBy, setSortBy] = useState(lastPullsReceivedSortBy);
  // undefined = modal closed, null = create mode, a PullReceived = edit mode.
  const [modalPull, setModalPull] = useState<PullReceived | null | undefined>(undefined);
  const [deleteTarget, setDeleteTarget] = useState<PullReceived | null>(null);
  const [deleting, setDeleting] = useState(false);
  // 2.0.28: bulk-delete selection mode - marko's own request. No checkbox
  // column sitting there all the time; the "Delete" toggle button below
  // reveals it, and it disappears again the moment you confirm or cancel.
  const [selectionMode, setSelectionMode] = useState(false);
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [confirmBulkDelete, setConfirmBulkDelete] = useState(false);
  const [bulkDeleting, setBulkDeleting] = useState(false);

  useEffect(() => {
    lastPullsReceivedSearch = search;
  }, [search]);

  useEffect(() => {
    lastPullsReceivedSortBy = sortBy;
  }, [sortBy]);

  const load = (q?: string) => {
    api
      .listPullsReceived({ search: q || undefined, dateFrom: dateFrom || undefined, dateTo: dateTo || undefined })
      .then(setPulls)
      .catch((e) => toast.error(errMsg(e)));
  };

  // 2.0.65: sorts by `eventDate` (what the table's own "Date" column
  // shows), not `createdAt` - same genuine bug fix as GivenPulls' own
  // sortedPulls above (see PULL_SORT_LABELS' comment for why), computed the
  // same null-safe way for both directions.
  const sortedPulls: PullReceived[] = (() => {
    if (pulls === null) return [];
    const withDate = pulls.filter((p) => p.eventDate !== null);
    const withoutDate = pulls.filter((p) => p.eventDate === null);
    const dir = sortBy === "furthest" ? -1 : 1;
    withDate.sort((a, b) => dir * ((a.eventDate as string).localeCompare(b.eventDate as string) || a.id - b.id));
    return [...withDate, ...withoutDate];
  })();

  const toggleOne = (id: number) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const allSelected = pulls !== null && pulls.length > 0 && pulls.every((p) => selected.has(p.id));
  const toggleSelectAll = () => {
    setSelected(allSelected ? new Set() : new Set((pulls ?? []).map((p) => p.id)));
  };

  const exitSelectionMode = () => {
    setSelectionMode(false);
    setSelected(new Set());
  };

  const confirmDeleteSelected = async () => {
    setBulkDeleting(true);
    try {
      const result = await api.bulkDeletePullsReceived(Array.from(selected));
      if (result.deletedIds.length > 0) {
        toast.success(`${result.deletedIds.length} pull${result.deletedIds.length === 1 ? "" : "s"} deleted`);
      }
      if (result.skipped.length > 0) {
        toast.error(`${result.skipped.length} skipped: ${summarizeBulkDeleteSkips(result.skipped)}`);
      }
      setConfirmBulkDelete(false);
      exitSelectionMode();
      load(search);
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setBulkDeleting(false);
    }
  };

  useEffect(() => {
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const t = setTimeout(() => load(search), 250);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [search, dateFrom, dateTo]);

  const confirmDelete = async () => {
    if (!deleteTarget) return;
    setDeleting(true);
    try {
      await api.deletePullReceived(deleteTarget.id);
      toast.success(`Pull ${deleteTarget.code} deleted`);
      setDeleteTarget(null);
      setModalPull(undefined);
      load(search);
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setDeleting(false);
    }
  };

  return (
    <>
      {/* 2.0.65: same buttons-row-then-labeled-filter-row restructuring as
          GivenPulls above - see that component's own comment. */}
      <div className="mb-3 flex justify-end gap-2">
        {!selectionMode && pulls && pulls.length > 0 && (
          <Button variant="secondary" onClick={() => setSelectionMode(true)}>
            <IconTrash className="h-4 w-4" /> Delete
          </Button>
        )}
        <Button variant="primary" onClick={() => setModalPull(null)}>
          <IconPlus className="h-4 w-4" /> New received pull
        </Button>
      </div>

      <div className="mb-4 flex flex-wrap items-end gap-3">
        <div className="w-52">
          <span className="label">Search</span>
          <div className="relative">
            <IconSearch className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-400 dark:text-slate-500" />
            <Input
              placeholder="Search received pulls..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="pl-9"
            />
          </div>
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
          <span className="label">Sort</span>
          <Select value={sortBy} onChange={(e) => setSortBy(e.target.value)} aria-label="Sort received pulls">
            {Object.entries(PULL_SORT_LABELS).map(([value, label]) => (
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
          itemLabel="received pull"
          busy={bulkDeleting}
          onConfirm={() => setConfirmBulkDelete(true)}
          onCancel={exitSelectionMode}
        />
      )}

      {pulls && pulls.length >= LIST_CAP && (
        <div className="mb-4 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-800 dark:border-amber-500/30 dark:bg-amber-500/10 dark:text-amber-400">
          Showing the most recent {LIST_CAP.toLocaleString()} received pulls that match your search. Narrow the
          search to see the rest.
        </div>
      )}

      {pulls === null ? (
        <LoadingBlock />
      ) : pulls.length === 0 ? (
        <EmptyState
          icon={<IconLink className="h-8 w-8" />}
          title="No received pulls yet"
          description="Record it when someone else pulls tickets for you - manually, or automatically once your Orders & Sales sheet syncs a row with Pull = yes."
          action={
            <Button variant="primary" onClick={() => setModalPull(null)}>
              <IconPlus className="h-4 w-4" /> New received pull
            </Button>
          }
        />
      ) : (
        // Same table-fixed + colgroup + guarded-row-click convention as
        // GivenPulls' own table above. The Order cell contains a real link
        // (react-router <Link>, not just a <button>/<input>) so the row-click
        // guard here also checks for "a", unlike GivenPulls' table which has
        // no links inside its rows. 2.0.38: Date uses formatDateNumeric
        // ("11.09.2026"), same as GivenPulls' own Date column - see that
        // table's comment above for why the old title tooltip is gone too.
        //
        // 2.0.37: same shift as GivenPulls' own table made just above - both
        // the wrapper's max-w-[1400px] and the table's own min-w-[1050px]
        // are gone, replaced with the same pure-percentage, two-mode model
        // every table in the app now uses. Below the shared
        // useNarrowTables() breakpoint (1649px window as of 2.0.38, was
        // 1690px), Order hides (still visible from the Orders page itself,
        // and from GivenPulls' own table when the same pull was made there -
        // never Pull/From/Event/Fee) and everything else grows a little and
        // switches to the smaller .th-c-narrow/.td-c-narrow. See Sales.tsx's
        // own colgroup comment and PROTECTED-AREAS-NOTES.md (2.0.38 section)
        // for the full reasoning and verification.
        <div className="overflow-x-auto rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-sm">
          <table className="w-full table-fixed border-collapse">
            {isNarrow ? (
              <colgroup>
                {selectionMode && <col className="w-8" />}
                <col className="w-[11.707%]" />
                <col className="w-[9.146%]" />
                <col className="w-[38.78%]" />
                <col className="w-[8.659%]" />
                <col className="w-[4.39%]" />
                <col className="w-[10%]" />
                <col className="w-[17.317%]" />
              </colgroup>
            ) : (
              <colgroup>
                {selectionMode && <col className="w-8" />}
                <col className="w-[9.084%]" />
                <col className="w-[10.22%]" />
                <col className="w-[40.028%]" />
                <col className="w-[6.884%]" />
                <col className="w-[3.691%]" />
                <col className="w-[7.807%]" />
                <col className="w-[8.162%]" />
                <col className="w-[14.123%]" />
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
                      aria-label="Select all received pulls"
                    />
                  </th>
                )}
                <th className={isNarrow ? "th-c-narrow" : "th-c"}>Pull</th>
                <th className={isNarrow ? "th-c-narrow" : "th-c"}>From</th>
                <th className={isNarrow ? "th-c-narrow" : "th-c"}>Event</th>
                <th className={isNarrow ? "th-c-narrow" : "th-c"}>Date</th>
                <th className={`${isNarrow ? "th-c-narrow" : "th-c"} text-right`}>Ks</th>
                <th className={`${isNarrow ? "th-c-narrow" : "th-c"} text-right`}>Fee</th>
                {!isNarrow && <th className="th-c">Order</th>}
                <th className={isNarrow ? "th-c-narrow" : "th-c"}>More info</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
              {sortedPulls.map((p) => (
                <tr
                  key={p.id}
                  className="cursor-pointer hover:bg-slate-50 dark:hover:bg-slate-800/60"
                  onClick={(e) => {
                    if ((e.target as HTMLElement).closest("input, button, a")) return;
                    if (selectionMode) {
                      toggleOne(p.id);
                      return;
                    }
                    setModalPull(p);
                  }}
                >
                  {selectionMode && (
                    <td className={isNarrow ? "td-c-narrow" : "td-c"}>
                      <input
                        type="checkbox"
                        className={CHECKBOX_CLASS}
                        checked={selected.has(p.id)}
                        onChange={() => toggleOne(p.id)}
                        aria-label={`Select pull ${p.code}`}
                      />
                    </td>
                  )}
                  <td
                    className={`${isNarrow ? "td-c-narrow" : "td-c"} truncate font-medium text-slate-900 dark:text-slate-100`}
                    title={`Added ${formatDate(p.createdAt)}`}
                  >
                    {p.code}
                  </td>
                  <td className={`${isNarrow ? "td-c-narrow" : "td-c"} truncate`} title={p.pullerName}>
                    {p.pullerName}
                  </td>
                  <td className={`${isNarrow ? "td-c-narrow" : "td-c"} truncate`} title={p.eventName}>
                    {p.eventName}
                  </td>
                  <td className={`${isNarrow ? "td-c-narrow" : "td-c"} whitespace-nowrap`}>
                    {formatDateNumeric(p.eventDate)}
                  </td>
                  <td className={`${isNarrow ? "td-c-narrow" : "td-c"} text-right tabular-nums whitespace-nowrap`}>{p.quantity}</td>
                  <td className={`${isNarrow ? "td-c-narrow" : "td-c"} text-right tabular-nums whitespace-nowrap`}>{formatMoney(p.amountCents, p.currency)}</td>
                  {!isNarrow && (
                    <td className="td-c">
                      {p.orderId && p.orderCode ? (
                        <div className="flex items-center gap-1.5">
                          <Link
                            to={`/orders/${p.orderId}`}
                            className="truncate font-medium text-brand-600 dark:text-brand-400 hover:underline"
                            title={`Open order ${p.orderCode}`}
                          >
                            {p.orderCode}
                          </Link>
                          {p.source === "sheet_sync" && <Badge tone="synced">Synced</Badge>}
                        </div>
                      ) : (
                        <span className="text-slate-400 dark:text-slate-500">Standalone</span>
                      )}
                    </td>
                  )}
                  <td
                    className={`${isNarrow ? "td-c-narrow" : "td-c"} truncate text-xs text-slate-500 dark:text-slate-400`}
                    title={p.moreInfo ?? undefined}
                  >
                    {p.moreInfo || "-"}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      <PullReceivedFormModal
        open={modalPull !== undefined}
        pull={modalPull ?? null}
        onClose={() => setModalPull(undefined)}
        onSaved={() => {
          setModalPull(undefined);
          load(search);
        }}
        onRequestDelete={(p) => setDeleteTarget(p)}
      />

      <ConfirmDialog
        open={!!deleteTarget}
        title="Delete this received pull?"
        message={
          <>
            This removes <strong>{deleteTarget?.code}</strong> ({deleteTarget?.pullerName} - {deleteTarget?.eventName}
            ) permanently. This can't be undone{deleteTarget?.orderId ? " (the linked order itself is not affected)" : ""}.
          </>
        }
        confirmLabel="Delete"
        danger
        busy={deleting}
        onConfirm={confirmDelete}
        onCancel={() => setDeleteTarget(null)}
      />

      <ConfirmDialog
        open={confirmBulkDelete}
        title={`Delete ${selected.size} selected received pull${selected.size === 1 ? "" : "s"}?`}
        message="This removes the selected received pulls permanently (any linked orders themselves are not affected). This can't be undone."
        confirmLabel="Delete selected"
        danger
        busy={bulkDeleting}
        onCancel={() => setConfirmBulkDelete(false)}
        onConfirm={confirmDeleteSelected}
      />
    </>
  );
}

/** Search-as-you-type picker for optionally linking a received pull to an
 * existing Order. Deliberately not LookupSelect (components/LookupSelect.tsx)
 * - that component is a plain <select> of a short, fully-loaded list with an
 * inline "+ New" affordance, neither of which fits here: Orders can run into
 * the thousands (a <select> with every one of them would be unusable), and
 * "+ New" makes no sense for linking to an order that must already exist. */
function OrderLinkPicker({
  orderId,
  orderCode,
  onChange,
}: {
  orderId: number | null;
  orderCode: string | null;
  onChange: (order: { id: number; code: string } | null) => void;
}) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<OrderRecord[]>([]);
  const [open, setOpen] = useState(false);
  const [searching, setSearching] = useState(false);

  useEffect(() => {
    if (!open || !query.trim()) {
      setResults([]);
      return;
    }
    setSearching(true);
    const t = setTimeout(() => {
      api
        .listOrders({ search: query.trim() })
        .then((rows) => setResults(rows.slice(0, 20)))
        .catch(() => setResults([]))
        .finally(() => setSearching(false));
    }, 250);
    return () => clearTimeout(t);
  }, [query, open]);

  if (orderId && orderCode && !open) {
    return (
      <div>
        <span className="label mb-1 block">Linked order</span>
        <div className="flex items-center gap-2 rounded-lg border border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-800/60 px-3 py-2">
          <IconLink className="h-4 w-4 shrink-0 text-slate-400 dark:text-slate-500" />
          <Link
            to={`/orders/${orderId}`}
            className="flex-1 truncate text-sm font-medium text-brand-600 dark:text-brand-400 hover:underline"
          >
            {orderCode}
          </Link>
          <button
            type="button"
            className="text-xs font-medium text-slate-400 hover:text-red-600 dark:text-slate-500 dark:hover:text-red-400"
            onClick={() => onChange(null)}
          >
            Unlink
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="relative">
      <span className="label mb-1 block">Linked order</span>
      <Input
        placeholder="Search by order code or event..."
        value={query}
        onChange={(e) => {
          setQuery(e.target.value);
          setOpen(true);
        }}
        onFocus={() => setOpen(true)}
        onBlur={() => setTimeout(() => setOpen(false), 150)}
      />
      {open && query.trim() && (
        <div className="absolute z-10 mt-1 max-h-56 w-full overflow-y-auto rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-lg">
          {searching ? (
            <p className="px-3 py-2 text-xs text-slate-400 dark:text-slate-500">Searching...</p>
          ) : results.length === 0 ? (
            <p className="px-3 py-2 text-xs text-slate-400 dark:text-slate-500">No matching orders</p>
          ) : (
            results.map((o) => (
              <button
                key={o.id}
                type="button"
                className="block w-full truncate px-3 py-2 text-left text-sm hover:bg-slate-50 dark:hover:bg-slate-800/60"
                onMouseDown={(e) => e.preventDefault()}
                onClick={() => {
                  onChange({ id: o.id, code: o.code });
                  setQuery("");
                  setOpen(false);
                }}
              >
                <span className="font-medium text-slate-800 dark:text-slate-200">{o.code}</span>
                <span className="ml-2 text-slate-400 dark:text-slate-500">
                  {o.eventName} · {formatDate(o.purchaseDate)}
                </span>
              </button>
            ))
          )}
        </div>
      )}
      <p className="mt-1 text-xs text-slate-400 dark:text-slate-500">
        Optional - link this to the order these tickets became, or leave it standalone.
      </p>
    </div>
  );
}

/** Exported since 2.0.24 - OrderDetail.tsx reuses this exact modal (in
 * edit-only mode, `pull` always non-null there) so editing a received pull
 * linked to the order you're already looking at gets the full form (event/
 * date/quantity/currency/more info/re-link), without a second, duplicate
 * edit UI. Creating one FROM Order Detail deliberately does NOT reuse this -
 * see OrderDetail.tsx's own `AddOrderPullModal` for why a full 8-field form
 * (most of it already visible on the order itself) would be redundant there. */
export function PullReceivedFormModal({
  open,
  pull,
  onClose,
  onSaved,
  onRequestDelete,
}: {
  open: boolean;
  pull: PullReceived | null;
  onClose: () => void;
  onSaved: (pull: PullReceived) => void;
  onRequestDelete: (pull: PullReceived) => void;
}) {
  const toast = useToast();
  const editing = pull !== null;

  const [pullerName, setPullerName] = useState("");
  const [eventName, setEventName] = useState("");
  const [eventDate, setEventDate] = useState("");
  const [quantity, setQuantity] = useState("1");
  const [amount, setAmount] = useState("");
  const [currency, setCurrency] = useState("EUR");
  const [customCurrency, setCustomCurrency] = useState(false);
  const [moreInfo, setMoreInfo] = useState("");
  const [linkedOrder, setLinkedOrder] = useState<{ id: number; code: string } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!open) return;
    if (pull) {
      setPullerName(pull.pullerName);
      setEventName(pull.eventName);
      setEventDate(pull.eventDate ?? "");
      setQuantity(String(pull.quantity));
      setAmount(centsToDecimalString(pull.amountCents));
      setCurrency(pull.currency);
      setCustomCurrency(!CURRENCIES.includes(pull.currency));
      setMoreInfo(pull.moreInfo ?? "");
      setLinkedOrder(pull.orderId && pull.orderCode ? { id: pull.orderId, code: pull.orderCode } : null);
    } else {
      setPullerName("");
      setEventName("");
      setEventDate("");
      setQuantity("1");
      setAmount("");
      setCurrency("EUR");
      setCustomCurrency(false);
      setMoreInfo("");
      setLinkedOrder(null);
    }
    setError(null);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, pull]);

  const submit = async () => {
    setError(null);
    const q = parseInt(quantity, 10);
    const amountCents = decimalStringToCents(amount);

    if (!pullerName.trim()) return setError("Puller name is required");
    if (!eventName.trim()) return setError("Event name is required");
    if (!Number.isFinite(q) || q < 1) return setError("Quantity must be at least 1");
    if (amountCents === null) return setError("Fee is not a valid amount");
    if (!currency.trim()) return setError("Currency is required");

    setSaving(true);
    try {
      if (editing && pull) {
        const input: PullReceivedEditInput = {
          pullerName: pullerName.trim(),
          eventName: eventName.trim(),
          eventDate: eventDate || null,
          quantity: q,
          amountCents,
          currency,
          moreInfo: moreInfo.trim() || null,
          orderId: linkedOrder?.id ?? null,
        };
        const updated = await api.updatePullReceived(pull.id, input);
        toast.success(`Pull ${updated.code} updated`);
        onSaved(updated);
      } else {
        const input: PullReceivedInput = {
          pullerName: pullerName.trim(),
          eventName: eventName.trim(),
          eventDate: eventDate || null,
          quantity: q,
          amountCents,
          currency,
          moreInfo: moreInfo.trim() || null,
          orderId: linkedOrder?.id ?? null,
        };
        const created = await api.createPullReceived(input);
        toast.success(`Pull ${created.code} created`);
        onSaved(created);
      }
    } catch (e) {
      setError(errMsg(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal open={open} onClose={onClose} title={editing ? `Edit ${pull?.code}` : "New received pull"} width="max-w-2xl">
      <div className="flex flex-col gap-4">
        {editing && pull?.source === "sheet_sync" && (
          <div className="flex items-center gap-2 rounded-lg border border-slate-200 bg-slate-50 px-3 py-2 text-xs text-slate-500 dark:border-slate-800 dark:bg-slate-800/60 dark:text-slate-400">
            <IconLink className="h-3.5 w-3.5 shrink-0" />
            Auto-linked from your Orders & Sales sheet sync. You can still edit anything below by hand.
          </div>
        )}

        <FormGroup title="Pull">
          <Field label="From (who pulled for you)" required hint="Who pulled these tickets for you">
            <Input value={pullerName} onChange={(e) => setPullerName(e.target.value)} />
          </Field>
          <Field label="Quantity" required>
            <Input type="number" min={1} step={1} value={quantity} onChange={(e) => setQuantity(e.target.value)} />
          </Field>
          <Field label="Event name" required hint="Free text - not linked to your Events list">
            <Input value={eventName} onChange={(e) => setEventName(e.target.value)} />
          </Field>
          <Field label="Event date">
            <Input type="date" value={eventDate} onChange={(e) => setEventDate(e.target.value)} />
          </Field>
        </FormGroup>

        <FormGroup title="Fee & link">
          <Field label={`Fee you paid (${currency})`} required hint="What you paid the puller - not the ticket price">
            <Input inputMode="decimal" placeholder="0.00" value={amount} onChange={(e) => setAmount(e.target.value)} />
          </Field>
          <div>
            <div className="flex items-center justify-between">
              <span className="label mb-1">Currency</span>
              <button
                type="button"
                className="mb-1 text-xs font-medium text-brand-600 dark:text-brand-400 hover:underline"
                onClick={() => setCustomCurrency((c) => !c)}
              >
                {customCurrency ? "Choose from list" : "Other..."}
              </button>
            </div>
            {customCurrency ? (
              <Input
                autoFocus
                placeholder="e.g. AED"
                value={currency}
                onChange={(e) => setCurrency(e.target.value.toUpperCase())}
              />
            ) : (
              <Select value={currency} onChange={(e) => setCurrency(e.target.value)}>
                {(CURRENCIES.includes(currency) ? CURRENCIES : [currency, ...CURRENCIES]).map((c) => (
                  <option key={c} value={c}>
                    {c}
                  </option>
                ))}
              </Select>
            )}
          </div>
          <div className="col-span-2">
            <Field label="More info" hint="Optional">
              <Textarea rows={2} value={moreInfo} onChange={(e) => setMoreInfo(e.target.value)} />
            </Field>
          </div>
          <div className="col-span-2">
            <OrderLinkPicker orderId={linkedOrder?.id ?? null} orderCode={linkedOrder?.code ?? null} onChange={setLinkedOrder} />
          </div>
        </FormGroup>
      </div>

      {error && <p className="mt-3 text-sm text-red-600 dark:text-red-400">{error}</p>}
      <ModalFooter>
        {editing && pull && (
          <Button variant="danger" className="mr-auto" onClick={() => onRequestDelete(pull)} disabled={saving}>
            Delete
          </Button>
        )}
        <Button variant="secondary" onClick={onClose} disabled={saving}>
          Cancel
        </Button>
        <Button variant="primary" onClick={submit} disabled={saving}>
          {saving ? "Saving..." : editing ? "Save changes" : "Create received pull"}
        </Button>
      </ModalFooter>
    </Modal>
  );
}

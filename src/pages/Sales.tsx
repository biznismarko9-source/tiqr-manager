import { useEffect, useMemo, useRef, useState } from "react";
import { Link, useLocation, useNavigate } from "react-router-dom";
import { api, errMsg } from "../lib/api";
import type { EventCategory, EventWithStats, OrderRecord, Platform, SaleBatchInput, SaleGroup, SalePaymentStatus, Ticket } from "../lib/types";
import {
  centsToDecimalString,
  decimalStringToCents,
  formatDate,
  formatDateNumeric,
  formatMoney,
  formatMoneyOrMixed,
  formatPercentOrMixed,
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
  TabSwitcher,
  Textarea,
} from "../components/ui";
import { BulkCompletionBar } from "../components/BulkCompletionBar";
import { EventCategoryBadge } from "../components/EventCategoryBadge";
import { LookupSelect } from "../components/LookupSelect";
import { IconArrowLeft, IconChevronDown, IconPlus, IconReceipt, IconSearch, IconTrash, IconX } from "../components/icons";
import { useToast } from "../lib/toast";
import { useListTab } from "../lib/useListTab";
import { useNarrowTables } from "../lib/useNarrowTables";
import { completionStatus } from "../lib/completion";

// 2.0.66: the new "Completed" indicator (see REDESIGN-2.0.66-REPORT.md) -
// deliberately separate from SALES_TABS above, which keeps its existing,
// payment-only meaning (2.0.59) unchanged. "Sold" is almost always true here
// (a SaleGroup only exists for sold tickets) - it only fails for a line that
// was refunded, which reverts that one ticket back to "available" (see
// refund_sale_impl); "Paid"/"Delivered" use the group's own per-line counts
// rather than the collapsed paymentStatus field, so a partially-resolved
// batch reads as "2 pending" instead of just "Mixed".
function saleGroupCompletionChecks(g: SaleGroup) {
  return [
    { label: "Sold", done: g.soldCount === g.ticketCount },
    { label: "Delivered", done: g.deliveredCount === g.ticketCount },
    { label: "Paid", done: g.paidCount === g.ticketCount },
  ];
}

// 2.0.59: "Pending" vs "Completed" tabs (marko's request - see
// REDESIGN-2.0.59-REPORT.md). A group's paymentStatus is only Some(...) when
// EVERY line in it shares one status (see GROUP_BASE_SELECT in sales.rs) -
// null ("Mixed", e.g. one ticket refunded later than another) stays in
// Pending rather than Completed, since a group that isn't cleanly resolved
// one way still deserves a look, same "don't quietly resolve ambiguity"
// spirit as showing "Mixed" instead of guessing elsewhere in this app.
const SALES_TABS: { key: "pending" | "completed"; label: string }[] = [
  { key: "pending", label: "Pending" },
  { key: "completed", label: "Completed" },
];

// 1.8.0: preferred/well-known currency codes for the Sales screen's Currency
// filter (section 4 of the brief) - always offered regardless of whether the
// database currently has data in them. Any OTHER currency actually present
// in `sales` (list_sale_currencies) is appended after these, so a "custom"
// currency the user already sold in still shows up without needing a
// separate free-text input.
const PREFERRED_CURRENCIES = ["EUR", "USD", "GBP", "CHF", "CZK", "PLN", "HUF", "SEK", "NOK", "DKK", "RON", "TRY", "BGN"];

const REFUND_STATUS_LABELS: Record<string, string> = {
  no_refund: "No refunds",
  partial_refund: "Partially refunded",
  full_refund: "Fully refunded",
};

// 2.0.65: relabeled from "Newest/Oldest first" to "Soonest/Furthest first",
// and "soonest" (ascending) is now the default sent on load, in place of the
// old empty-string-means-descending convention - see this page's own
// sortBy state default below, and sales.rs's list_sale_groups_impl doc
// comment for why the backend keeps the old "oldest"/unset-default pair
// working unchanged alongside these two new named values.
const SORT_LABELS: Record<string, string> = {
  soonest: "Soonest first",
  furthest: "Furthest first",
  revenue_desc: "Highest revenue",
  revenue_asc: "Lowest revenue",
  profit_desc: "Highest profit",
  profit_asc: "Lowest profit",
  tickets_desc: "Most tickets",
};

// 1.8.0: remembers the last-used Sales filters for this app session only
// (module-level, so it survives navigating away to Sale Detail and back,
// but resets on app restart - never written to disk). Deliberately NOT tied
// to the URL or to Sale Detail's own "Back to sales" link/history, so it
// works no matter how the user returns to this page, and touches nothing in
// Sale Detail's own (protected) anchor/navigation logic. See section 10 of
// the 1.8.0 brief and the 1.8.0 report for the reasoning.
interface SalesFilterState {
  search: string;
  eventId: number | "";
  /** 2.0.27 */
  categoryId: number | "";
  platformId: number | "";
  currency: string;
  refundStatus: string;
  dateFrom: string;
  dateTo: string;
  sortBy: string;
}
let lastFilters: SalesFilterState | null = null;

function FilterChip({ label, onRemove }: { label: string; onRemove: () => void }) {
  return (
    <span className="inline-flex items-center gap-1 rounded-full bg-slate-100 dark:bg-slate-800 py-1 pl-2.5 pr-1.5 text-xs font-medium text-slate-700 dark:text-slate-300 ring-1 ring-inset ring-slate-200 dark:ring-slate-700">
      {label}
      <button
        type="button"
        onClick={onRemove}
        className="rounded-full p-0.5 hover:bg-slate-200 dark:hover:bg-slate-700"
        aria-label={`Remove filter: ${label}`}
      >
        <IconX className="h-3 w-3" />
      </button>
    </span>
  );
}

function SummaryStat({ label, value, tone }: { label: string; value: string; tone?: "positive" | "negative" }) {
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
    </span>
  );
}

export default function Sales() {
  const toast = useToast();
  const location = useLocation();
  const navigate = useNavigate();
  const isNarrow = useNarrowTables();
  const [groups, setGroups] = useState<SaleGroup[] | null>(null);
  const [events, setEvents] = useState<EventWithStats[]>([]);
  const [platforms, setPlatforms] = useState<Platform[]>([]);
  const [currencies, setCurrencies] = useState<string[]>([]);
  const [categories, setCategories] = useState<EventCategory[]>([]);

  const [search, setSearch] = useState(lastFilters?.search ?? "");
  const [eventId, setEventId] = useState<number | "">(lastFilters?.eventId ?? "");
  // 2.0.27: category filter (marko's request - filter Events/Orders/Sales by
  // category), sitting next to the existing Event filter.
  const [categoryId, setCategoryId] = useState<number | "">(lastFilters?.categoryId ?? "");
  const [platformId, setPlatformId] = useState<number | "">(lastFilters?.platformId ?? "");
  const [currency, setCurrency] = useState(lastFilters?.currency ?? "");
  const [refundStatus, setRefundStatus] = useState(lastFilters?.refundStatus ?? "");
  const [dateFrom, setDateFrom] = useState(lastFilters?.dateFrom ?? "");
  const [dateTo, setDateTo] = useState(lastFilters?.dateTo ?? "");
  const [sortBy, setSortBy] = useState(lastFilters?.sortBy ?? "soonest");
  const [showMoreFilters, setShowMoreFilters] = useState(!!lastFilters?.refundStatus);
  // 2.0.59: see SALES_TABS above.
  const [tab, setTab] = useListTab("salesTab", ["pending", "completed"] as const);

  const [modalOpen, setModalOpen] = useState(false);
  // 2.0.28: bulk-delete selection mode - marko's own request. No checkbox
  // column sitting there all the time; the "Delete" toggle button below
  // reveals it, and it disappears again the moment you confirm or cancel.
  const [selectionMode, setSelectionMode] = useState(false);
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [confirmBulkDelete, setConfirmBulkDelete] = useState(false);
  const [bulkDeleting, setBulkDeleting] = useState(false);

  useEffect(() => {
    api.listEvents().then(setEvents).catch(() => {});
    api.listPlatforms().then(setPlatforms).catch(() => {});
    api.listSaleCurrencies().then(setCurrencies).catch(() => {});
    api.listEventCategories().then(setCategories).catch(() => {});
  }, []);

  // 1.8.3 (section 11): lets the Dashboard's "New Sale" Quick Action open
  // this page's New Sale modal directly, via a navigate(path, { state }) +
  // consume-and-clear convention (same one Orders.tsx uses for
  // presetEventId).
  //
  // 1.9.1: this effect used to also handle a `presetSearch` flag, letting
  // OrderDetail.tsx's "View sale" link jump here pre-filtered to one
  // ticket's sale. That link was removed this round (marko: no more
  // automatic cross-section navigation out of Orders/Tickets/Sales), so
  // `presetSearch` lost its only caller and was removed here too.
  useEffect(() => {
    const state = location.state as { openCreate?: boolean } | null;
    if (state?.openCreate) {
      setModalOpen(true);
      navigate(location.pathname, { replace: true, state: null });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [location.state]);

  // 1.8.0: remember these filters (module-level `lastFilters`, see top of
  // file) so returning from Sale Detail finds the Sales screen exactly as it
  // was left - without touching Sale Detail's own navigation at all.
  useEffect(() => {
    lastFilters = { search, eventId, categoryId, platformId, currency, refundStatus, dateFrom, dateTo, sortBy };
  }, [search, eventId, categoryId, platformId, currency, refundStatus, dateFrom, dateTo, sortBy]);

  const load = () => {
    api
      .listSaleGroups({
        search: search || undefined,
        eventId: eventId || undefined,
        categoryId: categoryId || undefined,
        platformId: platformId || undefined,
        // 2.0.59: no longer sent - the Payment dropdown this used to come
        // from is gone, replaced by the Pending/Completed tabs below, which
        // filter client-side instead (see visibleGroups) so every group's
        // own paymentStatus - including "Mixed" (null) - can be bucketed by
        // the same rule the tabs themselves document.
        currency: currency || undefined,
        refundStatus: refundStatus || undefined,
        dateFrom: dateFrom || undefined,
        dateTo: dateTo || undefined,
        sortBy: sortBy || undefined,
      })
      .then(setGroups)
      .catch((e) => toast.error(errMsg(e)));
  };

  // 2.0.59: tab split happens client-side on data the page already fetched -
  // see SALES_TABS above for the exact bucketing rule (Mixed stays Pending).
  const visibleGroups = useMemo(() => {
    if (!groups) return [];
    return groups.filter((g) => {
      const isCompleted = g.paymentStatus === "paid" || g.paymentStatus === "refunded";
      return tab === "completed" ? isCompleted : !isCompleted;
    });
  }, [groups, tab]);

  const toggleOne = (id: number) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  // 2.0.59: scoped to visibleGroups (the current tab), not every group ever
  // fetched - "select all" should only ever select what you can actually
  // see, so it can never bulk-delete something the other tab is hiding.
  const allSelected = visibleGroups.length > 0 && visibleGroups.every((g) => selected.has(g.id));
  const toggleSelectAll = () => {
    setSelected(allSelected ? new Set() : new Set(visibleGroups.map((g) => g.id)));
  };

  const exitSelectionMode = () => {
    setSelectionMode(false);
    setSelected(new Set());
  };

  const confirmDeleteSelected = async () => {
    setBulkDeleting(true);
    try {
      const result = await api.bulkDeleteSaleGroups(Array.from(selected));
      if (result.deletedIds.length > 0) {
        toast.success(`${result.deletedIds.length} sale${result.deletedIds.length === 1 ? "" : "s"} deleted`);
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

  useEffect(() => {
    const t = setTimeout(load, 200);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [search, eventId, categoryId, platformId, currency, refundStatus, dateFrom, dateTo, sortBy]);

  // 2.0.59: scoped to visibleGroups (the current tab) so this caption always
  // describes what's actually on screen, same choice as Events/Orders/
  // Tickets' own new tabs. cashTotals below is deliberately different - see
  // its own comment.
  const totals = useMemo(() => {
    if (!groups) return null;
    // Every group's own revenue/profit already excludes its refunded lines,
    // so this can sum them directly. Amounts in different currencies can
    // never be added together, so this only sums when every group shares one.
    const summaryCurrency =
      visibleGroups.length > 0 && visibleGroups.every((g) => g.currency === visibleGroups[0].currency)
        ? visibleGroups[0].currency
        : null;
    const sums = visibleGroups.reduce(
      (acc, g) => ({
        revenue: acc.revenue + g.revenueCents,
        profit: acc.profit + g.profitCents,
        tickets: acc.tickets + g.ticketCount,
        refunded: acc.refunded + g.refundedCount,
      }),
      { revenue: 0, profit: 0, tickets: 0, refunded: 0 },
    );
    return { ...sums, currency: summaryCurrency };
  }, [groups, visibleGroups]);

  // 2.0.59: deliberately stays on `groups` (every matching sale), not
  // visibleGroups - Paid/Outstanding is a cash-position summary ("how much
  // money is owed vs already collected, overall"), not a caption for what
  // the table below happens to be showing right now. Scoping it to the
  // Pending tab would make "Outstanding" swing to ~0 the moment you switch
  // to Completed, which would misreport real outstanding cash rather than
  // just changing what's on screen.
  //
  // 1.9.0 (section 3, "Sales - payment summary"): Paid/Outstanding totals,
  // built only from what's already on each SaleGroup row - no new backend
  // query. A group's `paymentStatus` is only Some(...) when EVERY line in it
  // shares one status (see GROUP_BASE_SELECT/map_sale_group in sales.rs) -
  // for those groups, `revenueCents` (already refund-excluded) IS the full
  // paid/pending amount, so it can be attributed directly. A "Mixed" group
  // (some lines paid, some still pending) can't be safely split into
  // Paid/Outstanding from what this screen has - that would need a new
  // per-status aggregate on GROUP_BASE_SELECT itself, which is intentionally
  // left untouched (see the 1.9.0 report) - so those groups are excluded
  // here and called out explicitly via `excludedCount` rather than guessed.
  const cashTotals = useMemo(() => {
    // No results at all (fresh install, or a filter combination matching
    // nothing) is "nothing to show", not "a currency conflict" - returning
    // null here (rather than falling through to the empty-array checks
    // below, which would otherwise resolve to a false "Mixed" for an
    // honest zero) hides the stats entirely via the `{cashTotals && ...}`
    // guard in the JSX, the same way `totals.refunded > 0 &&` already hides
    // an irrelevant zero elsewhere in this same summary bar.
    if (!groups || groups.length === 0) return null;
    const definite = groups.filter((g) => g.paymentStatus !== null);
    const currency =
      definite.length > 0
        ? definite.every((g) => g.currency === definite[0].currency)
          ? definite[0].currency
          : null
        : groups.every((g) => g.currency === groups[0].currency)
          ? groups[0].currency
          : null;
    const paid = definite.filter((g) => g.paymentStatus === "paid").reduce((sum, g) => sum + g.revenueCents, 0);
    const outstanding = definite
      .filter((g) => g.paymentStatus === "pending")
      .reduce((sum, g) => sum + g.revenueCents, 0);
    return { paid, outstanding, currency, excludedCount: groups.length - definite.length };
  }, [groups]);

  const currencyOptions = useMemo(() => {
    const extra = currencies.filter((c) => !PREFERRED_CURRENCIES.includes(c)).sort();
    return [...PREFERRED_CURRENCIES, ...extra];
  }, [currencies]);

  const activeFilters = useMemo(() => {
    const chips: { key: string; label: string; onRemove: () => void }[] = [];
    if (eventId) {
      const ev = events.find((e) => e.id === eventId);
      chips.push({ key: "event", label: `Event: ${ev?.name ?? eventId}`, onRemove: () => setEventId("") });
    }
    if (categoryId) {
      const c = categories.find((cat) => cat.id === categoryId);
      chips.push({ key: "category", label: `Category: ${c?.name ?? categoryId}`, onRemove: () => setCategoryId("") });
    }
    if (platformId) {
      const p = platforms.find((pl) => pl.id === platformId);
      chips.push({ key: "platform", label: `Platform: ${p?.name ?? platformId}`, onRemove: () => setPlatformId("") });
    }
    if (currency) {
      chips.push({ key: "currency", label: `Currency: ${currency}`, onRemove: () => setCurrency("") });
    }
    if (refundStatus) {
      chips.push({
        key: "refund",
        label: `Refunds: ${REFUND_STATUS_LABELS[refundStatus] ?? refundStatus}`,
        onRemove: () => setRefundStatus(""),
      });
    }
    if (dateFrom) chips.push({ key: "from", label: `From: ${dateFrom}`, onRemove: () => setDateFrom("") });
    if (dateTo) chips.push({ key: "to", label: `To: ${dateTo}`, onRemove: () => setDateTo("") });
    return chips;
  }, [eventId, categoryId, platformId, currency, refundStatus, dateFrom, dateTo, events, platforms, categories]);

  const hasActiveFilters = activeFilters.length > 0 || !!search;

  const clearAllFilters = () => {
    setSearch("");
    setEventId("");
    setCategoryId("");
    setPlatformId("");
    setCurrency("");
    setRefundStatus("");
    setDateFrom("");
    setDateTo("");
  };

  return (
    <div>
      <PageHeader
        title="Sales"
        subtitle="Every sale you've recorded, with profit calculated automatically."
        actions={
          <div className="flex items-center gap-2">
            {!selectionMode && groups && groups.length > 0 && (
              <Button variant="secondary" onClick={() => setSelectionMode(true)}>
                <IconTrash className="h-4 w-4" /> Delete
              </Button>
            )}
            <Button variant="primary" onClick={() => setModalOpen(true)}>
              <IconPlus className="h-4 w-4" /> New Sale
            </Button>
          </div>
        }
      />

      <TabSwitcher tabs={SALES_TABS} active={tab} onChange={setTab} />

      <div className="mb-2 flex flex-wrap items-end gap-3">
        <div className="w-56">
          <span className="label">Search</span>
          <div className="relative">
            <IconSearch className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-400 dark:text-slate-500" />
            <Input
              placeholder="Sale, ticket, order, buyer..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="pl-9"
            />
          </div>
        </div>
        <div className="w-52">
          <span className="label">Event</span>
          <Select value={eventId} onChange={(e) => setEventId(e.target.value ? Number(e.target.value) : "")}>
            <option value="">All events</option>
            {events.map((ev) => (
              <option key={ev.id} value={ev.id}>
                {ev.name}
              </option>
            ))}
          </Select>
        </div>
        <div className="w-44">
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
        <div className="w-40">
          <span className="label">Platform</span>
          {/* 1.9.5: this used to be the one deliberately-unscoped Platform
              picker in the app (filtering the Sales LIST by whatever
              platform a sale record actually has, not a per-record choice).
              marko looked at it again and wants it scoped too - now matches
              every other Platform picker: sale+both here, purchase+both on
              the Orders side (New/Edit Order, Tickets/Inventory filter). */}
          <Select value={platformId} onChange={(e) => setPlatformId(e.target.value ? Number(e.target.value) : "")}>
            <option value="">All platforms</option>
            {platforms
              .filter((p) => p.kind === "sale" || p.kind === "both")
              .map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
          </Select>
        </div>
        <div className="w-32">
          <span className="label">Currency</span>
          <Select value={currency} onChange={(e) => setCurrency(e.target.value)}>
            <option value="">All</option>
            {currencyOptions.map((c) => (
              <option key={c} value={c}>
                {c}
              </option>
            ))}
          </Select>
        </div>
        {/* 1.9.4: marko wanted From/To kept next to each other instead of
            wrapping apart - they used to be two independent items in this
            flex-wrap row, so a narrower window could wrap To onto its own
            line while From stayed on the first. Wrapping them as one flex
            item means the pair now moves as a unit: if there's no room,
            BOTH wrap down together, never split. */}
        <div className="flex items-end gap-2">
          <div className="w-36">
            <span className="label">From</span>
            <Input type="date" value={dateFrom} onChange={(e) => setDateFrom(e.target.value)} />
          </div>
          <div className="w-36">
            <span className="label">To</span>
            <Input type="date" value={dateTo} onChange={(e) => setDateTo(e.target.value)} />
          </div>
        </div>
        <button
          type="button"
          className="mb-2 inline-flex items-center gap-1 text-xs font-medium text-brand-600 dark:text-brand-400 hover:underline"
          onClick={() => setShowMoreFilters((v) => !v)}
        >
          More filters
          <IconChevronDown className={`h-3.5 w-3.5 transition-transform ${showMoreFilters ? "rotate-180" : ""}`} />
        </button>
      </div>

      {showMoreFilters && (
        <div className="mb-2 flex flex-wrap items-end gap-3">
          <div className="w-44">
            <span className="label">Refund status</span>
            <Select value={refundStatus} onChange={(e) => setRefundStatus(e.target.value)}>
              <option value="">All</option>
              <option value="no_refund">No refunds</option>
              <option value="partial_refund">Partially refunded</option>
              <option value="full_refund">Fully refunded</option>
            </Select>
          </div>
        </div>
      )}

      {activeFilters.length > 0 && (
        <div className="mb-4 flex flex-wrap items-center gap-1.5">
          {activeFilters.map((f) => (
            <FilterChip key={f.key} label={f.label} onRemove={f.onRemove} />
          ))}
          <button
            type="button"
            className="ml-1 text-xs font-medium text-slate-400 dark:text-slate-500 hover:text-slate-700 dark:hover:text-slate-300 hover:underline"
            onClick={clearAllFilters}
          >
            Clear all
          </button>
        </div>
      )}

      {/* 2.0.32 capped this row (and the table below) at max-w-[1400px] so
          the summary/sort row stayed aligned with the table it describes,
          rather than the page's full width - see that report for why.
          2.0.35 removes the cap from both: the table itself now grows
          proportionally with the window instead of stopping at 1400px
          (see the colgroup comment below), so keeping this row hard-capped
          while the table grows past it would misalign them again, just in
          the opposite direction. No `max-w` here anymore for the same
          reason there's none on the table wrapper - see that comment. */}
      <div className="mb-3 flex flex-wrap items-center justify-between gap-3">
        <div className="flex flex-wrap items-center gap-x-6 gap-y-1 rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 px-4 py-2.5 text-sm">
          {totals && groups ? (
            <>
              {/* 2.0.59: visibleGroups, not groups - this sits directly beside
                  Tickets/Revenue/Profit (also from `totals`, also tab-scoped)
                  and directly above a table listing only visibleGroups, so
                  it must count the same set they do. groups.length (every
                  sale matching the filters, both tabs combined) would read
                  as a mismatch against the very rows underneath it. */}
              <SummaryStat
                label="Results"
                value={`${visibleGroups.length} sale${visibleGroups.length === 1 ? "" : "s"}`}
              />
              <SummaryStat label="Tickets" value={String(totals.tickets)} />
              <SummaryStat label="Revenue" value={formatMoneyOrMixed(totals.revenue, totals.currency)} />
              <SummaryStat
                label="Profit"
                value={formatMoneyOrMixed(totals.profit, totals.currency)}
                tone={totals.currency !== null ? (totals.profit > 0 ? "positive" : totals.profit < 0 ? "negative" : undefined) : undefined}
              />
              {/* 1.9.0 (section 3): Paid/Outstanding totals across every
                  visible, unambiguous-status sale - see cashTotals above. */}
              {cashTotals && (
                <>
                  <SummaryStat label="Paid" value={formatMoneyOrMixed(cashTotals.paid, cashTotals.currency)} />
                  <SummaryStat label="Outstanding" value={formatMoneyOrMixed(cashTotals.outstanding, cashTotals.currency)} />
                </>
              )}
              {totals.refunded > 0 && (
                <SummaryStat label="Refunded" value={`${totals.refunded} ticket${totals.refunded === 1 ? "" : "s"}`} />
              )}
            </>
          ) : (
            <span className="text-slate-400 dark:text-slate-500">Loading…</span>
          )}
        </div>
        <div className="w-48">
          <Select value={sortBy} onChange={(e) => setSortBy(e.target.value)} aria-label="Sort sales">
            {Object.entries(SORT_LABELS).map(([value, label]) => (
              <option key={value} value={value}>
                {label}
              </option>
            ))}
          </Select>
        </div>
      </div>

      {cashTotals && cashTotals.excludedCount > 0 && (
        <p className="-mt-2 mb-3 text-xs text-slate-400 dark:text-slate-500">
          {cashTotals.excludedCount} sale{cashTotals.excludedCount === 1 ? "" : "s"} with a mixed payment status
          (some tickets paid, some still pending) {cashTotals.excludedCount === 1 ? "isn't" : "aren't"} counted in
          Paid/Outstanding above - open the sale to see its exact breakdown.
        </p>
      )}

      {groups && groups.length >= 5000 && (
        <div className="mb-4 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-800 dark:border-amber-500/30 dark:bg-amber-500/10 dark:text-amber-400">
          {/* 2.0.59: no longer mentions a "payment filter" - the tabs above
              split what's already fetched, they don't narrow this fetch
              itself (see visibleGroups), so switching tabs never helps this
              banner go away. */}
          Showing the most recent 5,000 sales that match your filters. Narrow the date range or event filter to see
          the rest.
        </div>
      )}

      {selectionMode && (
        <>
          {/* 2.0.67: marko's own request - mark Delivered/Paid on many sales
           * at once, right from this list, next to the existing bulk-delete
           * bar below (same selection, same checkboxes). See
           * BulkCompletionBar's own doc comment for why applying one action
           * here never clears the selection - marko can mark a batch
           * Delivered and then, still selected, also mark it Paid. */}
          <BulkCompletionBar
            count={selected.size}
            itemLabel="sale"
            onSetDelivered={async (delivered) => {
              const updated = await api.bulkSetSaleGroupsDeliveryStatus({
                groupIds: Array.from(selected),
                deliveryStatus: delivered ? "Delivered" : "Not delivered",
              });
              load();
              return updated;
            }}
            onSetPaid={async (paid) => {
              const updated = await api.bulkSetSaleGroupsPaymentStatus({
                groupIds: Array.from(selected),
                paymentStatus: paid ? "paid" : "pending",
              });
              load();
              return updated;
            }}
            onClear={() => setSelected(new Set())}
          />
          <BulkDeleteBar
            count={selected.size}
            itemLabel="sale"
            busy={bulkDeleting}
            onConfirm={() => setConfirmBulkDelete(true)}
            onCancel={exitSelectionMode}
          />
        </>
      )}

      {groups === null ? (
        <LoadingBlock />
      ) : groups.length === 0 ? (
        hasActiveFilters ? (
          <EmptyState
            icon={<IconReceipt className="h-8 w-8" />}
            title="No sales match these filters"
            action={
              <Button variant="secondary" onClick={clearAllFilters}>
                Clear filters
              </Button>
            }
          />
        ) : (
          <EmptyState
            icon={<IconReceipt className="h-8 w-8" />}
            title="No sales yet"
            description="Record your first sale to start tracking revenue and profit."
            action={
              <Button variant="primary" onClick={() => setModalOpen(true)}>
                <IconPlus className="h-4 w-4" /> New Sale
              </Button>
            }
          />
        )
      ) : visibleGroups.length === 0 ? (
        // 2.0.59: sales exist and match every filter above, just none in
        // the active tab.
        <EmptyState
          icon={<IconReceipt className="h-8 w-8" />}
          title={tab === "pending" ? "Nothing pending" : "Nothing completed yet"}
          description={
            tab === "pending"
              ? "Every matching sale is paid or refunded. Switch to the Completed tab to see them."
              : "Sales move here once their Payment status is Paid or Refunded."
          }
        />
      ) : (
        // One row per sale action (single ticket or multi-ticket batch) -
        // same table style as the Tickets screen's order-grouped list. A
        // batch of e.g. 8 tickets sold as 4+2+2 shows as 3 rows here, never
        // as 8 separate rows; clicking a row's Sale code opens Sale Detail,
        // which lists every ticket inside that one sale. The data was
        // already grouped this way (SaleGroup/batch_id, see sales.rs) - only
        // the layout changed here, no field or number is new.
        // 2.0.35: table-layout:fixed with every column pinned to a
        // PERCENTAGE of the table's own width (not a fixed px, and not one
        // single column absorbing 100% of any leftover space either - see
        // the history below for both of those older approaches). marko
        // wanted the table itself to keep growing with the window/
        // fullscreen (like the header/filters already do since 2.0.31),
        // but without recreating 2.0.32's original complaint - one column
        // (Event) stretching hundreds of px wider than its content needs
        // while looking like empty space. Splitting the growth across
        // every column proportionally solves both at once: every
        // percentage below is exactly that column's old fixed px value /
        // 1400 (the old cap, and the exact configuration already proven to
        // look right over a dozen versions - see
        // REDESIGN-2.0.32-REPORT.md) - so at 1400px+ this renders
        // pixel-identical to 2.0.34, and above that it keeps scaling
        // instead of stopping dead at a hard cap. `max-w-[1400px]` is gone
        // from this wrapper and the summary/sort row above for the same
        // reason - both now fill the real window width, no cap left
        // anywhere in this family of pages.
        //
        // Honest tradeoff, not hidden: below 1400px (down to this app's
        // smallest supported window, 1080px - see REDESIGN-1.8.2-REPORT.md
        // section 2 for the exact original math), the columns that used to
        // have a genuine guaranteed floor (everything except Event, which
        // was always "whatever's left over", never a real floor) are now
        // proportionally SMALLER than that old floor - specifically Sale
        // and Date, the two columns 2.0.33/2.0.34 widened to stop them
        // truncating. Event itself is NOT worse off here - at 808px it now
        // gets ~361px (44.714% of 808), a real improvement over its old
        // 34px leftover-scraps floor. `truncate` + a title tooltip on Sale
        // and Date (and `overflow-x-auto` on this wrapper) are the same
        // defensive fallbacks this codebase already relies on elsewhere
        // for exactly this kind of narrow-window edge case (see
        // Orders.tsx's own documented version of the same tradeoff) -
        // marko chose this approach (see chat, 2.0.35) knowing that
        // tradeoff, over reverting 2.0.32 entirely or leaving the hard cap
        // in place.
        //
        // `.th-c`/`.td-c` (index.css) scoped to this table and Sale
        // Detail's only - `.th`/`.td` (Tickets/Orders/Events/Inventory)
        // untouched. Checkbox column (1.9.1 removed it by default,
        // selectionMode brings it back for bulk delete) stays a small
        // fixed w-8, outside the percentage budget, same as it always was.
        //
        // 2.0.37: marko reported that even after 2.0.36 widened the tight
        // columns, the table could still force a horizontal scrollbar on a
        // window narrower than the 1400px design reference (percentages
        // only guarantee *ratios*, not a floor - the min-w-[1400px] 2.0.36
        // added stopped columns shrinking, but a scrollbar below 1400px
        // wide is still a scrollbar). marko's explicit answer: shrink the
        // whole table to fit instead - smaller font, and if that's still
        // not enough, hide the least important columns rather than ever
        // scroll or wrap text (see PROTECTED-AREAS-NOTES.md, 2.0.37
        // section, for the full reasoning and the exact measurements this
        // was verified against).
        //
        // So min-w-[1400px] and the single percentage set are both gone,
        // replaced with TWO complete, independently-100%-summing percentage
        // sets, switched by useNarrowTables() (one shared breakpoint, same
        // for every table in the app, so they all change size together):
        // - wide (window >= 1690px): every column as before, normal
        //   .th-c/.td-c font+padding.
        // - narrow (window < 1690px): Fees and Margin/ROI hide (their sale-
        //   level numbers are still on Sale Detail, one click away - never
        //   Sale/Event/the money columns that matter most), remaining
        //   columns get a bit more room, and every cell switches to the
        //   smaller .th-c-narrow/.td-c-narrow. Verified (Playwright, real
        //   Intl.NumberFormat data across en-US/sk-SK/de-DE, not just
        //   header text) to fit without scrolling or wrapping all the way
        //   down to 1080px, this app's enforced minimum window width.
        <div className="overflow-x-auto rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-sm">
          <table className="w-full table-fixed border-collapse">
            {isNarrow ? (
              <colgroup>
                {selectionMode && <col className="w-8" />}
                <col className="w-[10%]" />
                <col className="w-[12.098%]" />
                <col className="w-[15.366%]" />
                <col className="w-[8.659%]" />
                <col className="w-[4.39%]" />
                <col className="w-[10%]" />
                <col className="w-[10%]" />
                <col className="w-[10.488%]" />
                <col className="w-[10%]" />
                {/* 2.0.66: new "Completed" column - width is my own estimate
                    (not measured against real content like the rest of this
                    colgroup), taken entirely from Event's share above. Flag
                    to marko if this looks visually off. */}
                <col className="w-[9%]" />
              </colgroup>
            ) : (
              <colgroup>
                {selectionMode && <col className="w-8" />}
                <col className="w-[7.779%]" />
                <col className="w-[9.129%]" />
                <col className="w-[11.74%]" />
                <col className="w-[6.86%]" />
                <col className="w-[9.194%]" />
                <col className="w-[3.678%]" />
                <col className="w-[7.779%]" />
                <col className="w-[7.779%]" />
                <col className="w-[7.779%]" />
                <col className="w-[8.133%]" />
                <col className="w-[7.284%]" />
                <col className="w-[6.365%]" />
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
                      aria-label="Select all sales"
                    />
                  </th>
                )}
                <th className={isNarrow ? "th-c-narrow" : "th-c"}>Sale</th>
                <th className={isNarrow ? "th-c-narrow" : "th-c"}>Event</th>
                <th className={isNarrow ? "th-c-narrow" : "th-c"}>Platform</th>
                <th className={isNarrow ? "th-c-narrow" : "th-c"}>Date</th>
                {!isNarrow && <th className="th-c">Seats</th>}
                <th className={`${isNarrow ? "th-c-narrow" : "th-c"} text-right`} title="Tickets">Tix</th>
                <th className={`${isNarrow ? "th-c-narrow" : "th-c"} text-right`}>Revenue</th>
                {!isNarrow && <th className="th-c text-right">Fees</th>}
                <th className={`${isNarrow ? "th-c-narrow" : "th-c"} text-right`}>Cost</th>
                <th className={`${isNarrow ? "th-c-narrow" : "th-c"} text-right`}>Profit</th>
                {!isNarrow && (
                  <th className="th-c text-right leading-tight">
                    <div>Margin</div>
                    <div>ROI</div>
                  </th>
                )}
                <th className={isNarrow ? "th-c-narrow" : "th-c"}>Status</th>
                <th className={isNarrow ? "th-c-narrow" : "th-c"}>Completed</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
              {visibleGroups.map((g) => (
                <tr
                  key={g.id}
                  className={`hover:bg-slate-50 dark:hover:bg-slate-800/60 ${selectionMode ? "cursor-pointer" : ""}`}
                  onClick={(e) => {
                    // 2.0.28: unlike Events/Orders, this row never navigated
                    // on click before (only the Sale code cell's own <Link>
                    // below did) - so there's no existing behavior to guard
                    // against here, just toggle selection while in
                    // selectionMode, deferring to the Link/checkbox
                    // otherwise.
                    if (!selectionMode) return;
                    if ((e.target as HTMLElement).closest("a, input")) return;
                    toggleOne(g.id);
                  }}
                >
                  {selectionMode && (
                    <td className={isNarrow ? "td-c-narrow" : "td-c"}>
                      <input
                        type="checkbox"
                        className={CHECKBOX_CLASS}
                        checked={selected.has(g.id)}
                        onChange={() => toggleOne(g.id)}
                        aria-label={`Select ${g.code}`}
                      />
                    </td>
                  )}
                  <td className={isNarrow ? "td-c-narrow" : "td-c"}>
                    <Link
                      to={`/sales/${g.id}`}
                      title={g.code}
                      className="block truncate font-medium text-slate-900 dark:text-slate-100 hover:text-brand-700 dark:hover:text-brand-400"
                    >
                      {g.code}
                    </Link>
                  </td>
                  {/* 1.9.1: the Event name used to be a <Link> to Event
                      Detail - removed per marko's request to stop every
                      "this reference jumps me to a different section" link
                      in Orders/Tickets/Sales. The Sale code link above stays
                      (opening this exact sale's own detail page isn't a
                      foreign jump). */}
                  <td className={isNarrow ? "td-c-narrow" : "td-c"} title={g.eventId && g.eventName ? g.eventName : undefined}>
                    {g.eventId && g.eventName ? (
                      <div className="flex items-center gap-1.5">
                        <span className="truncate">{g.eventName}</span>
                        {g.categoryName && g.categoryColorSlot !== null && (
                          <span className="shrink-0">
                            <EventCategoryBadge name={g.categoryName} colorSlot={g.categoryColorSlot} />
                          </span>
                        )}
                      </div>
                    ) : (
                      <span className="italic text-slate-400 dark:text-slate-500">Mixed events</span>
                    )}
                  </td>
                  <td className={`${isNarrow ? "td-c-narrow" : "td-c"} truncate`} title={g.platformName ?? undefined}>
                    {g.platformName ?? "-"}
                  </td>
                  <td className={`${isNarrow ? "td-c-narrow" : "td-c"} whitespace-nowrap`}>
                    {formatDateNumeric(g.saleDate)}
                  </td>
                  {!isNarrow && (
                    <td className="td-c truncate" title={formatSeatsSummary(g.seats)}>
                      {formatSeatsSummary(g.seats)}
                    </td>
                  )}
                  <td className={`${isNarrow ? "td-c-narrow" : "td-c"} text-right tabular-nums whitespace-nowrap`}>{g.ticketCount}</td>
                  <td className={`${isNarrow ? "td-c-narrow" : "td-c"} text-right tabular-nums whitespace-nowrap`}>{formatMoneyOrMixed(g.revenueCents, g.currency)}</td>
                  {!isNarrow && (
                    <td className="td-c text-right tabular-nums whitespace-nowrap">{formatMoneyOrMixed(g.sellingFeesCents, g.currency)}</td>
                  )}
                  <td className={`${isNarrow ? "td-c-narrow" : "td-c"} text-right tabular-nums whitespace-nowrap`}>{formatMoneyOrMixed(g.costCents, g.currency)}</td>
                  <td
                    className={`${isNarrow ? "td-c-narrow" : "td-c"} text-right tabular-nums whitespace-nowrap font-medium ${
                      g.profitCents > 0
                        ? "text-emerald-600 dark:text-emerald-400"
                        : g.profitCents < 0
                          ? "text-red-600 dark:text-red-400"
                          : ""
                    }`}
                  >
                    {formatMoneyOrMixed(g.profitCents, g.currency)}
                  </td>
                  {!isNarrow && (
                    <td className="td-c text-right tabular-nums text-xs leading-tight whitespace-nowrap">
                      <div>{formatPercentOrMixed(g.margin, g.currency)}</div>
                      <div className="text-slate-400 dark:text-slate-500">{formatPercentOrMixed(g.roi, g.currency)}</div>
                    </td>
                  )}
                  <td className={isNarrow ? "td-c-narrow" : "td-c"}>
                    {g.paymentStatus ? <Badge tone={g.paymentStatus}>{g.paymentStatus}</Badge> : <Badge tone="mixed">Mixed</Badge>}
                    {g.refundedCount > 0 && (
                      <p
                        className="mt-0.5 truncate text-[11px] font-medium text-amber-700 dark:text-amber-400"
                        title={`${g.refundedCount} of ${g.ticketCount} refunded`}
                      >
                        {g.refundedCount}/{g.ticketCount} refunded
                      </p>
                    )}
                  </td>
                  <td className={isNarrow ? "td-c-narrow" : "td-c"}>
                    {(() => {
                      const c = completionStatus(saleGroupCompletionChecks(g));
                      return (
                        <Badge tone={c.tone} title={c.title}>
                          {c.label}
                        </Badge>
                      );
                    })()}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      <SaleFormModal
        open={modalOpen}
        onClose={() => setModalOpen(false)}
        onCreated={() => {
          setModalOpen(false);
          load();
        }}
      />

      <ConfirmDialog
        open={confirmBulkDelete}
        title={`Delete ${selected.size} selected sale${selected.size === 1 ? "" : "s"}?`}
        message="Tickets that are actively sold return to Available; any refunded lines are removed as history with no trace left. This cannot be undone."
        confirmLabel="Delete selected"
        danger
        busy={bulkDeleting}
        onCancel={() => setConfirmBulkDelete(false)}
        onConfirm={confirmDeleteSelected}
      />
    </div>
  );
}

interface SaleLineDraft {
  price: string;
  fees: string;
}

function SaleFormModal({
  open,
  onClose,
  onCreated,
}: {
  open: boolean;
  onClose: () => void;
  onCreated: () => void;
}) {
  const toast = useToast();
  const [step, setStep] = useState<"pick" | "details">("pick");
  const [selected, setSelected] = useState<Ticket[]>([]);
  const [lines, setLines] = useState<Record<number, SaleLineDraft>>({});
  // 1.7.3: ticket picking is order-grouped now (same pattern as
  // Tickets/Inventory) instead of one flat searchable ticket list - browse
  // orders that still have sellable tickets, open one, pick tickets from
  // just that order. `activeOrder` is null while browsing orders themselves.
  const [orderQuery, setOrderQuery] = useState("");
  const [orderOptions, setOrderOptions] = useState<OrderRecord[]>([]);
  const [activeOrder, setActiveOrder] = useState<OrderRecord | null>(null);
  const [orderTicketOptions, setOrderTicketOptions] = useState<Ticket[]>([]);
  const [platforms, setPlatforms] = useState<Platform[]>([]);
  const [platformId, setPlatformId] = useState<number | null>(null);
  const [saleDate, setSaleDate] = useState(todayIso());
  const [bulkPrice, setBulkPrice] = useState("");
  const [bulkFees, setBulkFees] = useState("");
  // 2.0.57: the currency this WHOLE sale is recorded in - independent of
  // whatever currency the ticket(s) being sold were themselves bought in
  // (e.g. paid out by a marketplace in EUR even though the ticket cost
  // USD). One value for the whole batch, same reasoning as Quick-fill
  // price/fees already applying to every selected ticket at once (one sale
  // action, one buyer, one payment). Defaulted in `goToDetails` below,
  // not here - at the moment the modal opens no ticket is selected yet, so
  // there's nothing to derive a sensible default from.
  const [saleCurrency, setSaleCurrency] = useState("EUR");
  const [customSaleCurrency, setCustomSaleCurrency] = useState(false);
  // "Convert to EUR" button next to it - own loading flag (not `saving`,
  // which is for the final Record submit) so the button can show
  // "Converting..." without touching the rest of the form. Mirrors Orders.tsx
  // OrderFormModal's `convertingCurrency`/`conversionToken` pair exactly,
  // including the same stale-request guard (this modal doesn't unmount on
  // close either).
  const [convertingCurrency, setConvertingCurrency] = useState(false);
  const conversionToken = useRef(0);
  // True once marko has deliberately set a sale currency himself (by hand,
  // or via "Convert to EUR") - once true, goToDetails() must never silently
  // overwrite that choice again just because he went back to "+ Add more
  // tickets" and returned (a real papercut this app's own past audits have
  // repeatedly flagged elsewhere: never discard a deliberate choice under a
  // recompute). Reset on every fresh modal open.
  const currencyTouched = useRef(false);
  const [paymentStatus, setPaymentStatus] = useState<SalePaymentStatus>("pending");
  const [buyerReference, setBuyerReference] = useState("");
  const [notes, setNotes] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!open) return;
    conversionToken.current += 1;
    currencyTouched.current = false;
    setStep("pick");
    setSelected([]);
    setLines({});
    setOrderQuery("");
    setOrderOptions([]);
    setActiveOrder(null);
    setOrderTicketOptions([]);
    setPlatformId(null);
    setSaleDate(todayIso());
    setBulkPrice("");
    setBulkFees("");
    setSaleCurrency("EUR");
    setCustomSaleCurrency(false);
    setConvertingCurrency(false);
    setPaymentStatus("pending");
    setBuyerReference("");
    setNotes("");
    setError(null);
    api.listPlatforms().then(setPlatforms).catch(() => {});
  }, [open]);

  // 2.0.57: called when leaving the "pick" step - defaults the sale currency
  // to what every selected ticket was itself bought in (reproducing this
  // app's original behaviour exactly, so an ordinary same-currency sale
  // needs zero extra clicks), or EUR when the selection already spans more
  // than one purchase currency (no single obvious default). Always still
  // freely editable afterward via the picker below.
  const goToDetails = () => {
    if (!currencyTouched.current) {
      const first = selected[0]?.currency;
      const uniform = first && selected.every((t) => t.currency === first) ? first : "EUR";
      setSaleCurrency(uniform);
      setCustomSaleCurrency(!PREFERRED_CURRENCIES.includes(uniform));
    }
    setStep("details");
  };

  /** 2.0.57: same "one shared live rate, then switch the fields AND the
   * currency to EUR together" pattern as Orders.tsx's own `convertToEur` -
   * see that function's doc comment for the full reasoning. Converts every
   * selected ticket's price AND fees in one round trip. */
  const convertToEur = async () => {
    if (saleCurrency === "EUR" || convertingCurrency) return;
    setError(null);

    const fields: Array<[string, (v: string) => void, string]> = [];
    for (const t of selected) {
      const line = lines[t.id] ?? { price: "", fees: "0" };
      fields.push([line.price, (v) => updateLine(t.id, "price", v), `${t.code} sale price`]);
      fields.push([line.fees || "0", (v) => updateLine(t.id, "fees", v), `${t.code} selling fees`]);
    }
    const parsedAmounts = fields.map(([value]) => decimalStringToCents(value));
    const invalidIndex = parsedAmounts.findIndex((cents) => cents === null);
    if (invalidIndex !== -1) {
      setError(`${fields[invalidIndex][2]} is not a valid amount - fix it before converting to EUR`);
      return;
    }

    const myToken = conversionToken.current;
    setConvertingCurrency(true);
    try {
      const amountsCents = parsedAmounts as number[];
      const result = await api.convertCurrency(saleCurrency, "EUR", amountsCents);
      if (conversionToken.current !== myToken) return;
      if (result.convertedCents.length !== fields.length) {
        throw new Error("Currency conversion returned an unexpected number of amounts");
      }
      fields.forEach(([, setValue], i) => setValue(centsToDecimalString(result.convertedCents[i])));
      const fromCurrency = saleCurrency;
      currencyTouched.current = true;
      setSaleCurrency("EUR");
      setCustomSaleCurrency(false);
      toast.success(
        `Converted at today's rate: 1 ${fromCurrency} = ${result.rate.toFixed(4)} EUR (rate as of ${formatDateNumeric(result.rateDate)})`,
      );
    } catch (e) {
      if (conversionToken.current !== myToken) return;
      toast.error(errMsg(e));
    } finally {
      if (conversionToken.current === myToken) setConvertingCurrency(false);
    }
  };

  // Browse orders that still have at least one sellable ticket. `status`
  // does the "has a sellable ticket" filtering server-side (same filter
  // list_orders already supports for the Orders/Tickets pages), and its
  // `search` already matches order code, event, supplier, platform AND
  // ticket code (BUG #5) - so typing an exact ticket code here still finds
  // the right order to open, even though you no longer add it directly.
  useEffect(() => {
    if (!open || step !== "pick" || activeOrder) return;
    const t = setTimeout(() => {
      api
        .listOrders({ search: orderQuery || undefined, status: "available,listed" })
        .then((res) => setOrderOptions(res.slice(0, 25)))
        .catch(() => {});
    }, 200);
    return () => clearTimeout(t);
  }, [open, step, orderQuery, activeOrder]);

  // Once an order is opened, load just its sellable tickets - this is the
  // "small window" of tickets to pick from, scoped to one order at a time.
  useEffect(() => {
    if (!open || step !== "pick" || !activeOrder) return;
    setOrderTicketOptions([]);
    api
      .listTickets({ orderId: activeOrder.id, status: "available,listed", sortBy: "created", sortDir: "desc" })
      .then(setOrderTicketOptions)
      .catch(() => {});
  }, [open, step, activeOrder]);

  // If every ticket gets removed while on the details step, drop back to
  // picking rather than showing an empty pricing form.
  useEffect(() => {
    if (step === "details" && selected.length === 0) setStep("pick");
  }, [step, selected.length]);

  const addTicket = (t: Ticket) => {
    setSelected((prev) => (prev.some((s) => s.id === t.id) ? prev : [...prev, t]));
    setLines((prev) => (prev[t.id] ? prev : { ...prev, [t.id]: { price: "", fees: "0" } }));
  };

  const removeTicket = (id: number) => {
    setSelected((prev) => prev.filter((t) => t.id !== id));
    setLines((prev) => {
      const next = { ...prev };
      delete next[id];
      return next;
    });
  };

  const updateLine = (id: number, field: keyof SaleLineDraft, value: string) => {
    setLines((prev) => ({ ...prev, [id]: { ...prev[id], [field]: value } }));
  };

  const applyBulkPrice = () => {
    if (!bulkPrice.trim()) return;
    setLines((prev) => {
      const next = { ...prev };
      for (const t of selected) next[t.id] = { ...next[t.id], price: bulkPrice };
      return next;
    });
  };

  const applyBulkFees = () => {
    if (!bulkFees.trim()) return;
    setLines((prev) => {
      const next = { ...prev };
      for (const t of selected) next[t.id] = { ...next[t.id], fees: bulkFees };
      return next;
    });
  };

  const visibleOptions = orderTicketOptions.filter((t) => !selected.some((s) => s.id === t.id));
  // 2.0.57: this is now purely about each ticket's own PURCHASE currency
  // (cost side) - it no longer doubles as "the currency to show revenue
  // in", since revenue is always `saleCurrency` now (one value for the
  // whole batch, picked in the form below).
  const costCurrencyUniform =
    selected.length > 0 && selected.every((t) => t.currency === selected[0].currency) ? selected[0].currency : null;
  // Profit only means anything as one number when cost and revenue share a
  // currency - same "never blend, show Mixed" rule finance.rs already
  // enforces everywhere else in this app (BUG #6). Before 2.0.57 this was
  // implied for free (a sale's currency always equalled its own ticket's),
  // so this check simply didn't exist yet.
  const profitComputable = costCurrencyUniform !== null && costCurrencyUniform === saleCurrency;

  const totals = useMemo(() => {
    let revenue = 0;
    let fees = 0;
    let cost = 0;
    for (const t of selected) {
      const line = lines[t.id];
      const p = parseFloat((line?.price ?? "").trim().replace(",", "."));
      const f = parseFloat((line?.fees ?? "0").trim().replace(",", ".")) || 0;
      if (Number.isFinite(p)) revenue += Math.round(p * 100);
      fees += Math.round(f * 100);
      cost += t.totalCostCents;
    }
    return { revenue, cost, fees, profit: revenue - cost - fees };
  }, [selected, lines]);

  // Only rendered when `profitComputable` is false - each selected ticket's
  // own cost, grouped by ITS OWN purchase currency (never blended, same
  // convention as every other mixed-currency view in this app), so the
  // "why can't you just show me profit" notice still shows real numbers
  // instead of nothing.
  const costByCurrency = useMemo(() => {
    const byCurrency = new Map<string, { cost: number; count: number }>();
    for (const t of selected) {
      const entry = byCurrency.get(t.currency) ?? { cost: 0, count: 0 };
      entry.cost += t.totalCostCents;
      entry.count += 1;
      byCurrency.set(t.currency, entry);
    }
    return Array.from(byCurrency.entries()).map(([currency, v]) => ({ currency, ...v }));
  }, [selected]);

  const submit = async () => {
    setError(null);
    if (selected.length === 0) return setError("Select at least one ticket to sell first");
    if (!saleDate) return setError("Sale date is required");
    if (!saleCurrency.trim()) return setError("Currency is required");

    const batchLines: SaleBatchInput["lines"] = [];
    for (const t of selected) {
      const line = lines[t.id];
      const priceStr = (line?.price ?? "").trim().replace(",", ".");
      if (!/^\d+(\.\d{1,2})?$/.test(priceStr)) {
        return setError(`Sale price for ${t.code} is not a valid amount`);
      }
      const feesStr = (line?.fees ?? "").trim().replace(",", ".") || "0";
      if (!/^\d+(\.\d{1,2})?$/.test(feesStr)) {
        return setError(`Selling fees for ${t.code} is not a valid amount`);
      }
      batchLines.push({
        ticketId: t.id,
        salePriceCents: Math.round(parseFloat(priceStr) * 100),
        sellingFeesCents: Math.round(parseFloat(feesStr) * 100),
      });
    }

    const input: SaleBatchInput = {
      lines: batchLines,
      platformId,
      saleDate,
      paymentStatus,
      buyerReference: buyerReference || null,
      notes: notes || null,
      currency: saleCurrency.trim().toUpperCase(),
    };
    setSaving(true);
    try {
      const sales = await api.createSalesBatch(input);
      if (sales.length === 1) {
        toast.success(`${sales[0].code} recorded - ${sales[0].ticketCode} marked as sold`);
      } else {
        toast.success(
          `${sales.length} sales recorded (${sales[0].code}–${sales[sales.length - 1].code}) - ${sales.length} tickets marked as sold`,
        );
      }
      onCreated();
    } catch (e) {
      setError(errMsg(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal open={open} onClose={onClose} title="New sale" width="max-w-2xl">
      {step === "pick" ? (
        <div>
          {!activeOrder ? (
            <>
              <Field
                label="Find an order to sell from"
                required
                hint="Only orders with an Available or Listed ticket are shown. Open one to pick which of its tickets to add."
              >
                <Input
                  autoFocus
                  placeholder="Search by order code, event, platform, ticket code..."
                  value={orderQuery}
                  onChange={(e) => setOrderQuery(e.target.value)}
                />
              </Field>
              <div className="mt-3 max-h-64 divide-y divide-slate-100 dark:divide-slate-800 overflow-y-auto rounded-lg border border-slate-200 dark:border-slate-800">
                {orderOptions.length === 0 ? (
                  <p className="p-4 text-center text-sm text-slate-400 dark:text-slate-500">
                    {orderQuery ? "No matching orders with sellable tickets" : "Start typing to search your orders"}
                  </p>
                ) : (
                  orderOptions.map((o) => {
                    const sellable = o.availableCount + o.listedCount;
                    return (
                      <button
                        key={o.id}
                        className="flex w-full items-center justify-between gap-3 px-3 py-2 text-left hover:bg-slate-50 dark:hover:bg-slate-800/60"
                        onClick={() => setActiveOrder(o)}
                      >
                        <span className="min-w-0">
                          <span className="block truncate text-sm font-medium text-slate-800 dark:text-slate-200">
                            {o.code} &middot; {o.eventName}
                          </span>
                          <span className="block truncate text-xs text-slate-400 dark:text-slate-500">
                            {o.platformName ?? "No platform"} · {formatDate(o.purchaseDate)}
                          </span>
                        </span>
                        <span className="flex shrink-0 items-center gap-2">
                          <span className="whitespace-nowrap rounded-full bg-emerald-50 dark:bg-emerald-500/10 px-2 py-0.5 text-xs font-medium text-emerald-700 dark:text-emerald-400">
                            {sellable} available
                          </span>
                          <IconChevronDown className="h-4 w-4 -rotate-90 text-slate-400 dark:text-slate-500" />
                        </span>
                      </button>
                    );
                  })
                )}
              </div>
            </>
          ) : (
            <>
              <div className="mb-2 flex items-center justify-between gap-2">
                <button
                  type="button"
                  className="inline-flex shrink-0 items-center gap-1 text-xs font-medium text-brand-600 dark:text-brand-400 hover:underline"
                  onClick={() => setActiveOrder(null)}
                >
                  <IconArrowLeft className="h-3.5 w-3.5" /> Back to orders
                </button>
                <span className="min-w-0 truncate text-xs text-slate-400 dark:text-slate-500">
                  {activeOrder.code} &middot; {activeOrder.eventName}
                </span>
              </div>
              <div className="max-h-64 divide-y divide-slate-100 dark:divide-slate-800 overflow-y-auto rounded-lg border border-slate-200 dark:border-slate-800">
                {visibleOptions.length === 0 ? (
                  <p className="p-4 text-center text-sm text-slate-400 dark:text-slate-500">
                    {orderTicketOptions.length === 0
                      ? "Loading tickets..."
                      : "Every sellable ticket from this order is already selected"}
                  </p>
                ) : (
                  visibleOptions.map((t) => (
                    <button
                      key={t.id}
                      className="flex w-full items-center justify-between gap-3 px-3 py-2 text-left hover:bg-slate-50 dark:hover:bg-slate-800/60"
                      onClick={() => addTicket(t)}
                    >
                      <span className="min-w-0">
                        <span className="block truncate text-sm font-medium text-slate-800 dark:text-slate-200">{t.code}</span>
                        <span className="block truncate text-xs text-slate-400 dark:text-slate-500">
                          {[t.section, t.rowLabel, t.seat].filter(Boolean).join(" / ") || "No seat info"}
                        </span>
                      </span>
                      <span className="flex shrink-0 items-center gap-2">
                        <Badge tone={t.status}>{t.status}</Badge>
                        <IconPlus className="h-4 w-4 text-brand-600 dark:text-brand-400" />
                      </span>
                    </button>
                  ))
                )}
              </div>
            </>
          )}

          {selected.length > 0 && (
            <div className="mt-4">
              <p className="label mb-1.5">Selected ({selected.length})</p>
              <div className="flex flex-wrap gap-1.5">
                {selected.map((t) => (
                  <span
                    key={t.id}
                    className="inline-flex items-center gap-1 rounded-full bg-brand-50 dark:bg-brand-500/10 py-1 pl-2.5 pr-1.5 text-xs font-medium text-brand-700 dark:text-brand-400 ring-1 ring-inset ring-brand-200 dark:ring-brand-500/30"
                  >
                    {t.code}
                    <button
                      type="button"
                      onClick={() => removeTicket(t.id)}
                      className="rounded-full p-0.5 hover:bg-brand-100 dark:hover:bg-brand-500/20"
                      aria-label={`Remove ${t.code}`}
                    >
                      <IconX className="h-3 w-3" />
                    </button>
                  </span>
                ))}
              </div>
            </div>
          )}
        </div>
      ) : (
        <>
          <div className="mb-3 flex items-center justify-between">
            <p className="label mb-0">Selected tickets ({selected.length})</p>
            <button type="button" className="text-xs font-medium text-brand-600 dark:text-brand-400 hover:underline" onClick={() => setStep("pick")}>
              + Add more tickets
            </button>
          </div>

          <div className="mb-3 flex flex-wrap items-end gap-2 rounded-lg bg-slate-50 dark:bg-slate-800/60 p-3">
            {/* 2.0.57: the currency this whole sale is recorded in - one
                value for every selected ticket, same "applies to all at
                once" spirit as Quick-fill price/fees right below. Defaulted
                in goToDetails() when this step was entered; freely editable
                here afterward. */}
            <div className="w-24">
              <span className="label">Sale currency</span>
              {customSaleCurrency ? (
                <Input
                  autoFocus
                  placeholder="e.g. AED"
                  value={saleCurrency}
                  onChange={(e) => {
                    currencyTouched.current = true;
                    setSaleCurrency(e.target.value.toUpperCase());
                  }}
                  disabled={convertingCurrency}
                />
              ) : (
                <Select
                  value={saleCurrency}
                  onChange={(e) => {
                    currencyTouched.current = true;
                    setSaleCurrency(e.target.value);
                  }}
                  disabled={convertingCurrency}
                >
                  {(PREFERRED_CURRENCIES.includes(saleCurrency) ? PREFERRED_CURRENCIES : [saleCurrency, ...PREFERRED_CURRENCIES]).map(
                    (c) => (
                      <option key={c} value={c}>
                        {c}
                      </option>
                    ),
                  )}
                </Select>
              )}
            </div>
            <button
              type="button"
              className="text-xs font-medium text-brand-600 dark:text-brand-400 hover:underline"
              onClick={() => {
                currencyTouched.current = true;
                setCustomSaleCurrency((v) => !v);
              }}
              disabled={convertingCurrency}
            >
              {customSaleCurrency ? "Choose from list" : "Other..."}
            </button>
            {saleCurrency !== "EUR" && (
              <Button type="button" variant="secondary" disabled={convertingCurrency} onClick={convertToEur}>
                {convertingCurrency ? "Converting..." : "Convert to EUR"}
              </Button>
            )}
            <div className="ml-4 w-28">
              <span className="label">Quick-fill price</span>
              <Input inputMode="decimal" placeholder="0.00" value={bulkPrice} onChange={(e) => setBulkPrice(e.target.value)} />
            </div>
            <Button type="button" variant="secondary" disabled={!bulkPrice.trim()} onClick={applyBulkPrice}>
              Apply to all
            </Button>
            <div className="ml-4 w-24">
              <span className="label">Quick-fill fees</span>
              <Input inputMode="decimal" placeholder="0.00" value={bulkFees} onChange={(e) => setBulkFees(e.target.value)} />
            </div>
            <Button type="button" variant="secondary" disabled={!bulkFees.trim()} onClick={applyBulkFees}>
              Apply to all
            </Button>
            <p className="w-full text-xs text-slate-400 dark:text-slate-500">
              Applying overwrites any price/fees already entered below for every selected ticket.
            </p>
          </div>

          <div className="max-h-52 divide-y divide-slate-100 dark:divide-slate-800 overflow-y-auto rounded-lg border border-slate-200 dark:border-slate-800">
            {selected.map((t) => {
              const line = lines[t.id] ?? { price: "", fees: "0" };
              return (
                <div key={t.id} className="flex items-center gap-2 px-3 py-2">
                  <div className="min-w-0 flex-1">
                    <p className="truncate text-sm font-medium text-slate-800 dark:text-slate-200">{t.code}</p>
                    <p className="truncate text-xs text-slate-400 dark:text-slate-500">
                      Cost {formatMoney(t.totalCostCents, t.currency)}
                      {[t.section, t.rowLabel, t.seat].some(Boolean)
                        ? ` · ${[t.section, t.rowLabel, t.seat].filter(Boolean).join(" / ")}`
                        : ""}
                    </p>
                  </div>
                  {/* Persistent currency label (1.6.0 audit UX finding: a
                      placeholder alone disappears the moment a value is
                      typed, which is exactly when it's most useful to still
                      see what currency these two fields are in). One label
                      covers both - price and fees on one ticket are always
                      the same currency. 2.0.57: this is now `saleCurrency`
                      (the picker above), not `t.currency` - price/fees are
                      entered in the SALE's chosen currency, which is no
                      longer necessarily the same as this ticket's own
                      purchase currency (shown instead in the "Cost" line
                      above, for comparison). */}
                  <span className="w-9 shrink-0 text-center text-xs font-medium text-slate-400 dark:text-slate-500">
                    {saleCurrency}
                  </span>
                  <div className="w-24 shrink-0">
                    <Input
                      inputMode="decimal"
                      placeholder="Price"
                      value={line.price}
                      onChange={(e) => updateLine(t.id, "price", e.target.value)}
                    />
                  </div>
                  <div className="w-20 shrink-0">
                    <Input
                      inputMode="decimal"
                      placeholder="Fees"
                      value={line.fees}
                      onChange={(e) => updateLine(t.id, "fees", e.target.value)}
                    />
                  </div>
                  <button
                    type="button"
                    className="shrink-0 text-slate-400 dark:text-slate-500 hover:text-red-600 dark:hover:text-red-400"
                    title="Remove from this sale"
                    onClick={() => removeTicket(t.id)}
                  >
                    <IconX className="h-4 w-4" />
                  </button>
                </div>
              );
            })}
          </div>

          {/* 2.0.57: revenue is always ONE number now (every line is
              entered in `saleCurrency`) - only profit can still be "Mixed",
              when a ticket's own purchase currency differs from that sale
              currency (never blended into a meaningless cross-currency
              subtraction - same rule finance.rs enforces everywhere else). */}
          <div className="mt-4 rounded-lg bg-slate-50 dark:bg-slate-800/60 px-4 py-3 text-sm">
            <div className="grid grid-cols-2 gap-3">
              <div>
                <p className="text-xs text-slate-400 dark:text-slate-500">
                  Total revenue ({selected.length} ticket{selected.length === 1 ? "" : "s"})
                </p>
                <p className="font-semibold text-slate-900 dark:text-slate-100">{formatMoney(totals.revenue, saleCurrency)}</p>
              </div>
              <div>
                <p className="text-xs text-slate-400 dark:text-slate-500">Estimated profit</p>
                {profitComputable ? (
                  <p
                    className={`font-semibold ${totals.profit >= 0 ? "text-emerald-600 dark:text-emerald-400" : "text-red-600 dark:text-red-400"}`}
                  >
                    {formatMoney(totals.profit, saleCurrency)}
                  </p>
                ) : (
                  <p className="font-semibold text-slate-400 dark:text-slate-500">Mixed</p>
                )}
              </div>
            </div>
            {!profitComputable && (
              <div className="mt-2 border-t border-slate-200 dark:border-slate-700 pt-2">
                <p className="mb-1.5 text-xs text-slate-400 dark:text-slate-500">
                  {costCurrencyUniform === null
                    ? "Selected tickets were bought in different currencies"
                    : `Selected tickets were bought in ${costCurrencyUniform}`}
                  , not {saleCurrency} - profit can&apos;t be shown as one number. Cost by purchase currency:
                </p>
                <div className="flex flex-wrap gap-x-4 gap-y-1">
                  {costByCurrency.map((c) => (
                    <p key={c.currency} className="text-xs text-slate-500 dark:text-slate-400">
                      {c.currency} ({c.count} ticket{c.count === 1 ? "" : "s"}): {formatMoney(c.cost, c.currency)}
                    </p>
                  ))}
                </div>
              </div>
            )}
          </div>

          <div className="mt-4 grid grid-cols-2 gap-4">
            <LookupSelect
              label="Platform"
              // 1.9.3: sale/both only - see the matching comment in
              // SaleDetail.tsx's Edit Sale form for the full reasoning. The
              // list-page filter dropdown above (`All platforms`) is
              // deliberately left showing every platform regardless of kind
              // - it filters existing recorded sales, which may reference a
              // platform since re-tagged out of "sale".
              options={platforms.filter((p) => p.kind === "sale" || p.kind === "both")}
              value={platformId}
              onChange={setPlatformId}
              onCreate={async (name) => {
                const p = await api.createPlatform(name, "sale");
                setPlatforms((prev) => [...prev, p]);
                return p;
              }}
            />
            <Field label="Sale date" required>
              <Input type="date" value={saleDate} onChange={(e) => setSaleDate(e.target.value)} />
            </Field>
            <Field label="Payment status" hint="A sale can't be created as already refunded">
              <Select value={paymentStatus} onChange={(e) => setPaymentStatus(e.target.value as SalePaymentStatus)}>
                <option value="pending">Pending</option>
                <option value="paid">Paid</option>
              </Select>
            </Field>
            <Field label="Buyer / reference">
              <Input value={buyerReference} onChange={(e) => setBuyerReference(e.target.value)} />
            </Field>
            <div className="col-span-2">
              <Field label="Notes">
                <Textarea rows={2} value={notes} onChange={(e) => setNotes(e.target.value)} />
              </Field>
            </div>
          </div>
        </>
      )}

      {error && <p className="mt-3 text-sm text-red-600 dark:text-red-400">{error}</p>}
      <ModalFooter>
        <Button variant="secondary" onClick={onClose} disabled={saving}>
          Cancel
        </Button>
        {step === "pick" ? (
          <Button variant="primary" onClick={goToDetails} disabled={selected.length === 0}>
            Continue ({selected.length})
          </Button>
        ) : (
          <Button variant="primary" onClick={submit} disabled={saving}>
            {saving ? "Recording..." : `Record ${selected.length} sale${selected.length === 1 ? "" : "s"}`}
          </Button>
        )}
      </ModalFooter>
    </Modal>
  );
}


// Price Checker (2.0.81) - marko's own new section: pick an event, save each
// marketplace's listings-page link, then record what the market is asking
// there (lowest/median/average/highest price, listing count) and compare it
// against his own unsold inventory. Originally manual-entry-only (see
// src-tauri/src/commands/price_checker.rs's module doc comment for why:
// researched, not assumed - none of the original 3 marketplaces offered an
// accessible public API to an individual seller, and StubHub actively blocks
// casual scraping - marko's own instruction was to fall back to manual entry
// rather than bypass any site's protection).
// 2.1.1-2.1.8 added a series of HIDDEN-WebView automated read attempts on top
// of that (retry loops, guessed CSS selectors, an AI-vision fallback). Marko
// rejected the whole approach as unreliable ("tymto sposobom ktorym
// pokracujeme nedava zmysel a nikde sa neposuvame" - the way we're
// continuing doesn't make sense, we're getting nowhere).
// 2.1.9 replaces ALL of that with the "Visible Scanner": a real, VISIBLE
// browser window marko scrolls/navigates himself, read on demand via "Scan
// Visible Prices" - see commands/price_checker_scanner.rs's module doc
// comment (Rust) for the full design. Manual paste/entry stays exactly as it
// was, still the fallback (and still the ONLY way anything ever reaches
// saved history - a scan result always goes through the same review-then-
// save step as a manual entry, never saved directly).
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useLocation } from "react-router-dom";
import { listen } from "@tauri-apps/api/event";
import { save } from "@tauri-apps/plugin-dialog";
import { api, errMsg } from "../lib/api";
import type {
  ComparableLevel,
  ComparableReferenceInput,
  CurrencyMarketAnalysis,
  DataQuality,
  EventWithStats,
  MarketAnalysisResult,
  MarketplacePriceView,
  NormalizedListing,
  PriceCheck,
  PriceCheckerEventOverview,
  PriceCheckerSummary,
  RankedComparable,
  ScannerClosedPayload,
  ScannerErrorPayload,
  ScannerOpenedPayload,
  ScannerStatus,
  ScanResultPayload,
  ScanRunFinishedPayload,
  TierBreakdownInput,
  YourTicketGroup,
} from "../lib/types";
import {
  decimalStringToCents,
  formatDateNumeric,
  formatDateTime,
  formatMoney,
  formatMoneyOrMixed,
  formatPercent,
  todayIso,
} from "../lib/format";
import {
  Button,
  Card,
  EmptyState,
  Input,
  LoadingBlock,
  ModalFooter,
  PageHeader,
  SEGMENTED_TRACK,
  segmentedItemClass,
  Select,
  Spinner,
  StatCard,
  Textarea,
} from "../components/ui";
import {
  IconAlertTriangle,
  IconDownload,
  IconArrowLeft,
  IconLink,
  IconSearch,
  IconTag,
  IconTrendingDown,
  IconTrendingUp,
  IconX,
} from "../components/icons";
import { useToast } from "../lib/toast";
import { CURRENCIES } from "./Orders";
import { extractPricesFromText } from "../lib/priceParse";

// ---------------------------------------------------------------------------
// Visible Scanner (2.1.9) - see commands/price_checker_scanner.rs's module
// doc comment (Rust) for the backend half of this. Event names below must
// match that module's own EVENT_SCANNER_OPENED/EVENT_SCANNER_ERROR/
// EVENT_SCAN_RESULT/EVENT_SCANNER_CLOSED constants exactly - there's no
// shared source of truth across the Rust/TS boundary for a plain event-name
// string, so if any of those is ever renamed there, it must be renamed here
// too.
// ---------------------------------------------------------------------------

const SCANNER_OPENED_EVENT = "price-scanner-opened";
const SCANNER_ERROR_EVENT = "price-scanner-error";
const SCAN_RESULT_EVENT = "price-scanner-scan-result";
const SCANNER_CLOSED_EVENT = "price-scanner-closed";
/** 2.27.0 - one automatic run ended. Fires once per run, not per pass. */
const RUN_FINISHED_EVENT = "price-scanner-run-finished";

const SCANNER_STATUS_META: Record<ScannerStatus, { label: string; className: string }> = {
  ready: { label: "Ready", className: "bg-slate-100 text-slate-600 dark:bg-slate-800 dark:text-slate-300" },
  scanning: { label: "Scanning...", className: "bg-brand-50 text-brand-700 dark:bg-brand-900/40 dark:text-brand-300" },
  success: { label: "Success", className: "bg-emerald-50 text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-400" },
  partial: { label: "Partial", className: "bg-amber-50 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400" },
  unable_to_read: { label: "Unable to read", className: "bg-slate-100 text-slate-500 dark:bg-slate-800 dark:text-slate-400" },
  blocked: { label: "Blocked", className: "bg-amber-50 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400" },
  error: { label: "Error", className: "bg-red-50 text-red-700 dark:bg-red-900/30 dark:text-red-400" },
};

// ---------------------------------------------------------------------------
// Market Analysis (2.2.0) - built entirely on top of the Visible Scanner
// above, one analysis per scanner session (never blended across marketplace
// cards - see commands/price_checker_analysis.rs's module doc comment,
// Rust). Own small pill maps rather than reusing the shared `Badge`
// component's STATUS_TONES - same reasoning as SCANNER_STATUS_META just
// above: these are a genuinely different domain (comparable-market
// classification, not a ticket/order/sale status), and "tier_comparable"
// happens to be a real value in BOTH ComparableLevel and DataQuality, so a
// dedicated local map avoids any risk of an unrelated shared style changing
// out from under this feature later.
// ---------------------------------------------------------------------------

const COMPARABLE_LEVEL_META: Record<ComparableLevel, { label: string; className: string }> = {
  exact_comparable: { label: "Exact comparable", className: "bg-emerald-50 text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-400" },
  close_comparable: { label: "Close comparable", className: "bg-sky-50 text-sky-700 dark:bg-sky-900/30 dark:text-sky-400" },
  tier_comparable: { label: "Tier comparable", className: "bg-slate-100 text-slate-600 dark:bg-slate-800 dark:text-slate-300" },
  general_market: { label: "General market", className: "bg-slate-100 text-slate-500 dark:bg-slate-800 dark:text-slate-400" },
};

const DATA_QUALITY_META: Record<DataQuality, { label: string; className: string }> = {
  strong_comparable: { label: "Strong data", className: "bg-emerald-50 text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-400" },
  section_comparable: { label: "Section data", className: "bg-sky-50 text-sky-700 dark:bg-sky-900/30 dark:text-sky-400" },
  tier_comparable: { label: "Tier data", className: "bg-slate-100 text-slate-600 dark:bg-slate-800 dark:text-slate-300" },
  partial: { label: "Partial data", className: "bg-amber-50 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400" },
};

function LevelPill({ level }: { level: ComparableLevel }) {
  const meta = COMPARABLE_LEVEL_META[level];
  return <span className={`inline-flex items-center rounded-full px-2 py-0.5 text-[11px] font-medium ${meta.className}`}>{meta.label}</span>;
}

function DataQualityPill({ quality }: { quality: DataQuality }) {
  const meta = DATA_QUALITY_META[quality];
  return <span className={`inline-flex items-center rounded-full px-2 py-0.5 text-[11px] font-medium ${meta.className}`}>{meta.label}</span>;
}

/** One marketplace card's live scanner state, kept on PriceChecker (not
 * MarketplaceCard itself) for the same reason `autoCheck` used to live one
 * level up: a session outlives any single card render (it's tied to a real
 * OS window) and PriceChecker is what owns the Tauri event subscriptions
 * that feed it. Keyed in `PriceChecker`'s own `scannerSessions` map by
 * `sessionKey(eventId, marketplaceId)` - NOT marketplaceId alone, since
 * `marketplaces` is a shared lookup table and marko can have a session open
 * for "Vivid Seats on Event A" while looking at "Vivid Seats on Event B",
 * which the backend itself treats as two entirely independent sessions (see
 * commands::price_checker_scanner::insert_new_session's own doc comment,
 * Rust) - keying by marketplaceId alone would make the second one silently
 * clobber the frontend's only handle on the first. */
interface ScannerCardState {
  /** Whatever PriceChecker minted for this session (its own `requestIdRef`)
   * - echoed back on every event, and passed to every later
   * scanVisiblePrices/cancelPriceScan/closePriceScanner call. */
  requestId: number;
  /** True between calling `openPriceScanner` and the window actually
   * finishing opening (`price-scanner-opened`) or failing to
   * (`price-scanner-error`, which removes the session entirely instead). */
  opening: boolean;
  /** True between calling `scanVisiblePrices` and its `ScanResultPayload`
   * arriving - purely a frontend overlay flag, never written into `status`
   * itself, so `status` always holds the last SETTLED outcome and a
   * cancelled/interrupted scan has something sensible to fall back to. */
  scanning: boolean;
  /** Last settled outcome - "ready" until the first scan completes, then
   * whatever the backend's `derive_session_status` decided. Never
   * "scanning" - see `scanning` above. */
  status: ScannerStatus;
  listings: NormalizedListing[];
  lowestPriceCents: number | null;
  medianPriceCents: number | null;
  averagePriceCents: number | null;
  highestPriceCents: number | null;
  currency: string | null;
  scanCount: number;
  /** 2.27.0 - why the last automatic run ended, in marko's own reading
   *  language ("the page stopped showing anything new"). `undefined` until a
   *  run has finished on this session. */
  runReason?: string;
  lastScanAt: string | null;
  message: string | null;
  /** 2.9.0 - the LAST scan's own accounting (marko's Part D). `listings`
   * above stays the accumulated session total, so these four intentionally
   * do not have to add up to it: they describe one read of the page. */
  lastScanFound: number;
  lastScanAccepted: number;
  lastScanSkipped: number;
  lastScanDuplicates: number;
  lastScanSkipReasons: Record<string, number>;
  /** How many accumulated listings the headline stats came from, and how
   * many were left out for being in another currency (or having none). */
  statsListingCount: number;
  statsExcludedCount: number;
}

function sessionKey(eventId: number, marketplaceId: number): string {
  return `${eventId}:${marketplaceId}`;
}

/** Finds which session (if any) a `requestId` echoed back on an event
 * belongs to - a small linear scan over at most a handful of concurrent
 * sessions, never a bottleneck. Returns the composite key so the caller can
 * update `scannerSessions` directly. */
function keyForRequestId(sessions: Record<string, ScannerCardState>, requestId: number): string | null {
  for (const [key, session] of Object.entries(sessions)) {
    if (session.requestId === requestId) return key;
  }
  return null;
}


// ---------------------------------------------------------------------------
// Small local helpers
// ---------------------------------------------------------------------------

/** `myAvgPurchaseCostCents`/`myAvgListingPriceCents` + `myCurrency` follow
 * the same "always return the blended figure, let the currency flag decide
 * Mixed vs. real" convention `formatMoneyOrMixed` already uses everywhere
 * else in this app - EXCEPT `unsoldTicketCount === 0` is a DIFFERENT reason
 * for `myCurrency` to be null than an actual mixed-currency blend (there's
 * simply nothing to average), so that case shows "-" instead of a
 * misleading "Mixed". See PriceCheckerSummary's own doc comment (types.ts). */
function formatMyMoney(cents: number | null, currency: string | null, unsoldCount: number): string {
  if (unsoldCount === 0) return "-";
  return formatMoneyOrMixed(cents, currency);
}

type Trend = { direction: "up" | "down" | "flat"; deltaCents: number };

/** Compares the two most recent checks' lowest price - `history` is always
 * newest-first (see MarketplacePriceView's own doc comment), so this is
 * simply history[0] vs history[1]. Null when there's nothing to compare yet
 * (0 or 1 checks so far). */
function trendFromHistory(history: PriceCheck[]): Trend | null {
  if (history.length < 2) return null;
  const delta = history[0].lowestPriceCents - history[1].lowestPriceCents;
  return { direction: delta === 0 ? "flat" : delta > 0 ? "up" : "down", deltaCents: delta };
}

/** Up = the market price rose since the last check - good news for a
 * seller, so this is colored the same "up is good" way StatCard's revenue-
 * like trends are (unlike e.g. a cost trend, where up would be bad). */
function TrendNote({ trend, currency }: { trend: Trend; currency: string }) {
  if (trend.direction === "flat") {
    return <span className="text-xs font-medium text-slate-400 dark:text-slate-500">No change since last check</span>;
  }
  const up = trend.direction === "up";
  const Icon = up ? IconTrendingUp : IconTrendingDown;
  return (
    <span
      className={`inline-flex items-center gap-1 text-xs font-medium ${up ? "text-emerald-600 dark:text-emerald-400" : "text-red-600 dark:text-red-400"}`}
    >
      <Icon className="h-3 w-3 shrink-0" />
      {formatMoney(Math.abs(trend.deltaCents), currency)} {up ? "higher" : "lower"} than the previous check
    </span>
  );
}

/** Small colored status pill shared by every scanner card - "Opening..." is
 * its own transient sub-state (see ScannerCardState.opening's own doc
 * comment) rendered separately from the settled/scanning states in
 * SCANNER_STATUS_META. */
/** 2.10.0 - the Price Checker event overview: every upcoming event as a
 * dense card, replacing the single "Select an event..." dropdown that used to
 * be the only way in.
 *
 * Everything shown here comes from ONE `list_price_checker_overview` call.
 * The list never triggers a scan, never touches a marketplace, and never
 * calls `get_price_checker_summary` per row - opening a page must not cost
 * network requests marko did not ask for (his own Part 13).
 *
 * There is deliberately no "Scan failed" filter or badge. A failed scan is
 * never stored anywhere in this app - see `PriceCheckerMarketplaceStatus`'s
 * own doc comment - so the four filters below are exactly the states the
 * database can actually answer for. */
type OverviewFilter = "all" | "needs_link" | "not_scanned" | "scanned";

const OVERVIEW_FILTERS: { key: OverviewFilter; label: string }[] = [
  { key: "all", label: "All" },
  { key: "needs_link", label: "Needs link" },
  { key: "not_scanned", label: "Not scanned" },
  { key: "scanned", label: "Scanned" },
];

/** "2h ago" / "3d ago" / "just now" from an ISO timestamp. Relative time is
 * the thing marko actually asked to see on a card ("Last scan 2h ago"); the
 * exact timestamp stays available as the element's title. */
function relativeTime(iso: string): string {
  const then = Date.parse(iso);
  if (Number.isNaN(then)) return "";
  const mins = Math.floor((Date.now() - then) / 60000);
  if (mins < 1) return "just now";
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  if (days < 30) return `${days}d ago`;
  return `${Math.floor(days / 30)}mo ago`;
}

function EventOverviewList({
  rows,
  loading,
  onOpen,
}: {
  rows: PriceCheckerEventOverview[];
  loading: boolean;
  onOpen: (eventId: number) => void;
}) {
  const [search, setSearch] = useState("");
  const [filter, setFilter] = useState<OverviewFilter>("all");

  const needle = search.trim().toLowerCase();
  const visible = useMemo(
    () =>
      rows.filter((r) => {
        if (needle) {
          const haystack = `${r.eventName} ${r.venue ?? ""} ${r.city ?? ""}`.toLowerCase();
          if (!haystack.includes(needle)) return false;
        }
        if (filter === "needs_link") return r.linkedCount === 0;
        if (filter === "not_scanned") return r.checkedCount === 0;
        if (filter === "scanned") return r.checkedCount > 0;
        return true;
      }),
    [rows, needle, filter],
  );

  // Selecting is scoped to what is currently visible - "Select all" on a
  // filtered list must not quietly also select events you cannot see.
  if (loading) return <LoadingBlock label="Loading events..." />;

  if (rows.length === 0) {
    return (
      <EmptyState
        icon={<IconTag className="h-5 w-5" />}
        title="No upcoming events"
        description="Price Checker only lists events that are still upcoming - once an event is completed or cancelled, checking live prices for it no longer means anything."
      />
    );
  }

  return (
    <>
      <div className="mb-3 flex flex-wrap items-center gap-2">
        <div className="relative w-64">
          <IconSearch className="pointer-events-none absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-slate-400 dark:text-slate-500" />
          <Input
            className="h-8 py-0 pl-8 text-xs"
            placeholder="Search event, venue or city..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </div>
        <div className={SEGMENTED_TRACK}>
          {OVERVIEW_FILTERS.map((f) => (
            <button
              key={f.key}
              type="button"
              onClick={() => setFilter(f.key)}
              aria-pressed={filter === f.key}
              className={segmentedItemClass(filter === f.key)}
            >
              {f.label}
            </button>
          ))}
        </div>
        <span className="ml-auto text-xs text-slate-500 dark:text-slate-400">
          <span className="tabular-nums">{visible.length}</span> of{" "}
          <span className="tabular-nums">{rows.length}</span> events
        </span>
      </div>

      {/* 2.29.4: selection removed at marko's request. The scanner opens one
          visible window per marketplace and he drives it himself, so a
          multi-select never led anywhere a single click did not - "Check
          selected" could only ever open the FIRST one. The row's own Check
          button is the whole interaction now. */}
      {visible.length === 0 ? (
        <EmptyState
          icon={<IconSearch className="h-5 w-5" />}
          title="No event matches"
          description="Try a shorter search, or switch the filter back to All."
        />
      ) : (
        /* 2.13.0: one row per event instead of a card each. The card
           carried all three marketplaces inline, which meant nine numbers
           per event and no way to compare the same marketplace across two
           events - they were never in the same column. This keeps only what
           you scan a list for (is it linked, how many listings, how stale)
           and moves the per-marketplace breakdown behind the click that
           already opened the event anyway. */
        <Card className="overflow-hidden p-0">
          <table className="w-full border-collapse">
            <thead>
              <tr>
                <th className="th-c">Event</th>
                <th className="th-c">Date</th>
                <th className="th-c">Platforms</th>
                <th className="th-c text-right">Listings</th>
                <th className="th-c text-right">Last scan</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
              {visible.map((row) => (
                <EventOverviewRow key={row.eventId} row={row} onOpen={() => onOpen(row.eventId)} />
              ))}
            </tbody>
          </table>
        </Card>
      )}
    </>
  );
}

/** One event as a dense card - marko asked for compact rows, not cards that
 * take half the screen. Everything on it is a real stored value; an event
 * with no data simply shows "No link" / "Not scanned" rather than a zero. */
function EventOverviewRow({
  row,
  onOpen,
}: {
  row: PriceCheckerEventOverview;
  onOpen: () => void;
}) {
  const place = [row.city, row.venue].filter(Boolean).join(" · ");
  const total = row.marketplaces.length;
  return (
    <tr
      onClick={onOpen}
      className="cursor-pointer transition hover:bg-slate-50 dark:hover:bg-slate-800/50"
    >
      <td className="td-c">
        <p className="truncate text-sm font-medium text-slate-900 dark:text-slate-100">{row.eventName}</p>
        {place && <p className="mt-0.5 truncate text-xs text-slate-400 dark:text-slate-500">{place}</p>}
      </td>
      <td className="td-c whitespace-nowrap tabular-nums text-slate-500 dark:text-slate-400">
        {row.eventDate ? formatDateNumeric(row.eventDate) : "No date"}
      </td>
      <td className="td-c whitespace-nowrap">
        {row.linkedCount === 0 ? (
          <span className="text-slate-400 dark:text-slate-500">No link</span>
        ) : (
          <span className="font-medium text-emerald-600 dark:text-emerald-400">
            {row.linkedCount === total ? `All ${total}` : `${row.linkedCount} of ${total}`}
          </span>
        )}
      </td>
      <td className="td-c text-right tabular-nums">
        {row.lastListingCount !== null ? (
          <span className="font-semibold text-slate-800 dark:text-slate-200">{row.lastListingCount}</span>
        ) : (
          <span className="text-slate-400 dark:text-slate-500">-</span>
        )}
      </td>
      <td className="td-c whitespace-nowrap text-right">
        {row.lastCheckedAt ? (
          <span className="tabular-nums text-slate-500 dark:text-slate-400" title={row.lastCheckedAt}>
            {relativeTime(row.lastCheckedAt)}
          </span>
        ) : (
          <span className="text-amber-600 dark:text-amber-400">Never</span>
        )}
      </td>
    </tr>
  );
}

/** 2.9.0 - marko's Parts D, K and L in one place: the scan summary, the
 * result filters, the CSV export, and the listings table itself.
 *
 * Split out of the marketplace card's own JSX (where all of this used to be
 * inline) purely so the filter state has somewhere to live - the card body is
 * rendered per marketplace and had no room for four more `useState`s without
 * turning into a second component anyway. No data is computed here that the
 * backend didn't already send: every number below is either a field off
 * `ScanResultPayload` or a count of the already-filtered `listings` array. */
/** Human wording for the skip reasons price_checker_scan.js reports. An
 * unknown reason falls through to its own raw key with underscores replaced,
 * so a reason added on the JS side later still shows up readably instead of
 * disappearing. */
const SKIP_REASON_LABELS: Record<string, string> = {
  crossed_out_price: "crossed-out / was price",
  not_a_listing_price: "fees, total or other non-listing price",
  page_chrome: "header, footer, cart or nav",
  missing_price: "no readable price",
  not_visible: "not on screen",
};


function ScannerStatusPill({ session }: { session: ScannerCardState }) {
  if (session.opening) {
    return (
      <span className="inline-flex items-center gap-1.5 rounded-full bg-slate-100 px-2.5 py-1 text-xs font-medium text-slate-500 dark:bg-slate-800 dark:text-slate-400">
        <Spinner className="h-3 w-3" /> Opening...
      </span>
    );
  }
  const meta = SCANNER_STATUS_META[session.scanning ? "scanning" : session.status];
  return (
    <span className={`inline-flex items-center gap-1.5 rounded-full px-2.5 py-1 text-xs font-medium ${meta.className}`}>
      {session.scanning && <Spinner className="h-3 w-3" />}
      {meta.label}
    </span>
  );
}


/** Marko's own unsold inventory for this event, grouped by section/row/
 * currency (marko's own spec, "## YOUR TICKETS" + "## PRICE RECOMMENDATION")
 * - reuses TIQR's real ticket data, never a duplicate of it. `recommendation`
 * is null for a group whose currency hasn't been scanned yet in THIS
 * session - shown as "No scan in this currency yet" rather than a blank or
 * a fabricated number. */
function YourTicketsTable({ groups }: { groups: YourTicketGroup[] }) {
  if (groups.length === 0) {
    return <p className="text-xs text-slate-400 dark:text-slate-500">No unsold tickets for this event yet.</p>;
  }
  return (
    <div className="table-flush max-h-56 rounded-lg border border-slate-200 dark:border-slate-800">
      <table className="w-full border-collapse">
        <thead>
          <tr>
            <th className="px-2 py-1 text-left text-[11px] font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">Section</th>
            <th className="px-2 py-1 text-left text-[11px] font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">Row</th>
            <th className="px-2 py-1 text-right text-[11px] font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">Qty</th>
            <th className="px-2 py-1 text-right text-[11px] font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">Avg cost</th>
            <th className="px-2 py-1 text-right text-[11px] font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">Your listing</th>
            <th className="px-2 py-1 text-right text-[11px] font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">Recommended</th>
            <th className="px-2 py-1 text-right text-[11px] font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">Profit</th>
            <th className="px-2 py-1 text-right text-[11px] font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">ROI</th>
            <th className="px-2 py-1 text-left text-[11px] font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">Based on</th>
          </tr>
        </thead>
        <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
          {groups.map((g, i) => (
            <tr key={i}>
              <td className="px-2 py-1 text-xs text-slate-700 dark:text-slate-300">{g.section ?? "-"}</td>
              <td className="px-2 py-1 text-xs text-slate-700 dark:text-slate-300">{g.row ?? "-"}</td>
              <td className="px-2 py-1 text-right text-xs tabular-nums text-slate-700 dark:text-slate-300">{g.quantity}</td>
              <td className="px-2 py-1 text-right text-xs tabular-nums text-slate-700 dark:text-slate-300">{formatMoney(g.avgCostCents, g.currency)}</td>
              <td className="px-2 py-1 text-right text-xs tabular-nums text-slate-700 dark:text-slate-300">{formatMoney(g.avgListingPriceCents, g.currency)}</td>
              <td className="px-2 py-1 text-right text-xs tabular-nums text-slate-900 dark:text-slate-100">
                {g.recommendation ? formatMoney(g.recommendation.recommendedPriceCents, g.currency) : "-"}
              </td>
              <td
                className={`px-2 py-1 text-right text-xs tabular-nums ${
                  g.recommendation && g.recommendation.expectedProfitCents < 0
                    ? "text-red-600 dark:text-red-400"
                    : "text-slate-700 dark:text-slate-300"
                }`}
              >
                {g.recommendation ? formatMoney(g.recommendation.expectedProfitCents, g.currency) : "-"}
              </td>
              <td className="px-2 py-1 text-right text-xs tabular-nums text-slate-700 dark:text-slate-300">
                {g.recommendation ? formatPercent(g.recommendation.expectedRoi) : "-"}
              </td>
              <td className="px-2 py-1 text-xs text-slate-500 dark:text-slate-400">
                {g.recommendation ? `${g.recommendation.basedOn} · ${g.recommendation.confidence}` : "No scan in this currency yet"}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}



// ---------------------------------------------------------------------------
// One marketplace's card: link + latest check + full history + Visible
// Scanner controls + "Check Prices" (manual entry, unchanged).
// ---------------------------------------------------------------------------

function MarketplaceCard({
  eventId,
  view,
  onLinkSaved,
  session,
  onOpenScanner,
  onScanVisible,
  onStopScan,
  onCloseScanner,
}: {
  eventId: number;
  view: MarketplacePriceView;
  onLinkSaved: () => void;
  /** This card's live Visible Scanner session, if one is open - undefined
   *  means no session (shows "Open & Scan" instead of Scan/Stop/Close). */
  session: ScannerCardState | undefined;
  onOpenScanner: (view: MarketplacePriceView, url: string) => void;
  onScanVisible: (eventId: number, marketplaceId: number) => void;
  onStopScan: (eventId: number, marketplaceId: number) => void;
  onCloseScanner: (eventId: number, marketplaceId: number) => void;
}) {
  const toast = useToast();
  const [url, setUrl] = useState(view.link?.url ?? "");
  const [savingLink, setSavingLink] = useState(false);

  // 2.2.0: Market Analysis for this card's OWN session - tier/section
  // pricing + Your Tickets recommendations, built entirely on top of
  // `session.listings` (never a separate scan, never touches the scanner
  // itself). Refetches whenever a new scan settles (`session.scanCount`
  // changing IS "a new scan happened", even one that added zero new
  // listings - matches marko's own spec: "each new scan updates overall/
  // tier/section market"). Kept local to this card, not lifted into
  // PriceChecker's own scannerSessions map, since it's pure derived data for
  // exactly one session and nothing else on the page needs it.
  const [analysis, setAnalysis] = useState<MarketAnalysisResult | null>(null);
  const [analysisLoading, setAnalysisLoading] = useState(false);
  const [analysisError, setAnalysisError] = useState<string | null>(null);
  useEffect(() => {
    if (!session || session.listings.length === 0) {
      setAnalysis(null);
      setAnalysisError(null);
      return;
    }
    let cancelled = false;
    setAnalysisLoading(true);
    api
      .computeMarketAnalysis(session.requestId, eventId)
      .then((result) => {
        if (!cancelled) {
          setAnalysis(result);
          setAnalysisError(null);
        }
      })
      .catch((e) => {
        if (!cancelled) setAnalysisError(errMsg(e));
      })
      .finally(() => {
        if (!cancelled) setAnalysisLoading(false);
      });
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [session?.requestId, session?.scanCount, eventId]);

  // 2.26.1 - a finished scan records ITSELF. The modal that used to ask marko
  // to type in numbers the scanner had just read is gone, so this is the only
  // path into `price_checks` now.
  //
  // Guarded on the scan NUMBER, not on a boolean: `analysisLoading` flipping
  // false re-runs this effect, and without the guard the same scan would be
  // written to history twice. `savedScanRef` starts at 0 and `scanCount` is
  // 1-based, so the first scan is never mistaken for "already saved".
  //
  // It waits for the analysis so the per-tier breakdown goes in with it - the
  // analysis PANEL is gone from the card, but the command still runs, purely
  // for that breakdown (it is what the history line's "1 tier" counts).
  //
  // No currency means no save. A price check whose currency had to be guessed
  // is worse than no price check, and the card says so in one line.
  const savedScanRef = useRef(0);
  useEffect(() => {
    if (!session || session.scanning || session.listings.length === 0) return;
    if (!session.currency || analysisLoading) return;
    if (savedScanRef.current >= session.scanCount) return;
    savedScanRef.current = session.scanCount;
    const matching = analysis?.byCurrency.find((c) => c.currency === session.currency);
    api
      .savePriceCheck({
        eventId,
        marketplaceId: view.marketplaceId,
        lowestPriceCents: session.lowestPriceCents ?? 0,
        medianPriceCents: session.medianPriceCents,
        averagePriceCents: session.averagePriceCents ?? 0,
        highestPriceCents: session.highestPriceCents ?? 0,
        listingCount: session.listings.length,
        currency: session.currency,
        tierBreakdown:
          matching?.tiers.map((t) => ({
            tier: t.tier,
            lowestPriceCents: t.stats.lowestPriceCents,
            medianPriceCents: t.stats.medianPriceCents,
            listingCount: t.stats.listingCount,
          })) ?? [],
      })
      // Refreshes this card's own history and the event's "Market vs. mine".
      .then(() => onLinkSaved())
      .catch(() => {
        // Let the scan be re-saved by the next one rather than sticking on a
        // scan number that never actually landed.
        savedScanRef.current = session.scanCount - 1;
      });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [session?.scanCount, session?.scanning, session?.currency, analysisLoading, eventId, view.marketplaceId]);

  // Keeps the field in sync when the parent reloads (e.g. after this exact
  // save, or after switching away and back to this event) without clobbering
  // a save that's still in flight.
  useEffect(() => {
    setUrl(view.link?.url ?? "");
  }, [view.link?.url]);

  const linkDirty = url.trim() !== (view.link?.url ?? "");

  const saveLink = async () => {
    setSavingLink(true);
    try {
      await api.saveEventMarketplaceLink({ eventId, marketplaceId: view.marketplaceId, url });
      toast.success(url.trim() ? "Link saved" : "Link cleared");
      onLinkSaved();
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setSavingLink(false);
    }
  };

  // Tries the saved link first, falling back to whatever is currently typed
  // in the field (not yet saved) - marko shouldn't have to click Save
  // before opening the scanner on it.
  const scannerTarget = (view.link?.url || url).trim();

  const latest = view.history[0] ?? null;
  const trend = trendFromHistory(view.history);
  const older = view.history.slice(1);

  return (
    <Card className="flex flex-col p-4">
      <div className="mb-3 flex items-center justify-between gap-2">
        <div className="flex items-center gap-2">
          <h3 className="text-sm font-semibold text-slate-900 dark:text-slate-100">{view.marketplaceName}</h3>
          {/* 2.1.6: `marketplaceActive` is false only for StubHub, and only
           *  once it has real history against THIS event (see
           *  get_price_checker_summary_impl's own doc comment) - a fresh
           *  event never sees a StubHub card at all, so this badge only
           *  ever shows up alongside real, pre-existing history. */}
          {!view.marketplaceActive && (
            <span
              className="rounded-full bg-slate-100 px-2 py-0.5 text-[11px] font-medium text-slate-500 dark:bg-slate-800 dark:text-slate-400"
              title="Not used for new checks anymore - Viagogo replaced it. The history below stays exactly as it was."
            >
              Retired
            </span>
          )}
        </div>
        {/* 2.26.1: "Check Prices" and its manual-entry modal are gone. That
         *  button opened a form asking marko to type in the numbers the
         *  scanner had just read for him - his own words, "to uplne odstran".
         *  Scanning is now the only way a price check is recorded, and it
         *  records itself (see the auto-save effect below). */}
      </div>

      {view.marketplaceActive ? (
        <div className="mb-4 flex items-center gap-2">
          <IconLink className="h-4 w-4 shrink-0 text-slate-300 dark:text-slate-600" />
          <Input
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            placeholder="Paste this marketplace's listings page URL..."
            className="text-xs"
          />
          {linkDirty && (
            <Button onClick={saveLink} disabled={savingLink} className="shrink-0">
              {savingLink ? <Spinner className="h-4 w-4" /> : "Save"}
            </Button>
          )}
        </div>
      ) : (
        // Read-only URL display -
        // no Input/Save here, saving a new url for a retired marketplace
        // would just be rejected by the backend anyway.
        view.link?.url && (
          <div className="mb-4 flex items-center gap-2">
            <IconLink className="h-4 w-4 shrink-0 text-slate-300 dark:text-slate-600" />
            <p className="select-all break-all rounded-lg bg-slate-50 px-2 py-1.5 font-mono text-xs text-slate-500 dark:bg-slate-800/60 dark:text-slate-400">
              {view.link.url}
            </p>
          </div>
        )
      )}

      {/* Visible Scanner (2.1.9) - a real, visible browser window marko
       *  scrolls/scans himself. See this file's own module doc comment and
       *  commands/price_checker_scanner.rs (Rust) for the full design. */}
      {view.marketplaceActive && (
        <div className="mb-4 rounded-lg border border-slate-100 p-3 dark:border-slate-800">
          <div className="mb-2 flex flex-wrap items-center justify-between gap-2">
            <p className="section-title">Visible Scanner</p>
            {session && <ScannerStatusPill session={session} />}
          </div>

          {!session ? (
            <Button
              variant="secondary"
              onClick={() => onOpenScanner(view, scannerTarget)}
              disabled={!scannerTarget}
              title={
                !scannerTarget
                  ? "Enter this marketplace's listings page URL above first."
                  : "Opens a real, visible browser window on this page. One click then reads the whole page on its own - you can leave this screen and carry on working."
              }
            >
              <IconLink className="h-4 w-4" /> Open & Scan
            </Button>
          ) : (
            <>
              <div className="flex flex-wrap items-center gap-2">
                <Button
                  variant="secondary"
                  onClick={() => onScanVisible(eventId, view.marketplaceId)}
                  disabled={session.opening || session.scanning}
                >
                  <IconTag className="h-4 w-4" /> Read the whole page
                </Button>
                {session.scanning && (
                  <Button variant="secondary" onClick={() => onStopScan(eventId, view.marketplaceId)}>
                    <IconX className="h-4 w-4" /> Stop scanning
                  </Button>
                )}
                <Button variant="secondary" onClick={() => onCloseScanner(eventId, view.marketplaceId)}>
                  <IconX className="h-4 w-4" /> Close
                </Button>
              </div>

              {session.message && (
                <p className="mt-2 flex items-start gap-1.5 text-xs text-amber-700 dark:text-amber-400">
                  <IconAlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0" />
                  {session.message}
                </p>
              )}

              {/* 2.26.1 - marko's screenshots. A marketplace card is a SCAN
                  BUTTON AND ITS HISTORY, nothing else. The listings table with
                  its Min/Max filters and CSV export, the manual "Save to
                  history" button, the Market Map, the Market Analysis panel
                  and the "Compare a specific ticket" tool were all stacked
                  under each of THREE side-by-side cards - which is what made
                  this screen unreadable.

                  Where each went:
                    - the map is now ONE map for the whole event, above the
                      cards, merging every marketplace's listings (marko: "mapa
                      by mala byt pre vsetky platformy rovnaka a tie listingy
                      sa spoja");
                    - saving is automatic - see the effect above;
                    - the comparison he actually reads is the "Market vs. mine"
                      card at the top of the event.

                  What is left is one honest line, and the history below it
                  updates itself. */}
              {session.listings.length > 0 && (
                <p className="mt-2 text-xs text-slate-500 dark:text-slate-400">
                  {session.listings.length} listing{session.listings.length === 1 ? "" : "s"} read
                  {session.scanning
                    ? " so far - still reading, you can leave this screen."
                    : session.currency
                      ? " - saved to the history below."
                      : " - not saved: the page never showed a currency, and a price check without one would be a guess."}
                  {/* Why the run ended, in the backend's own words - never
                      just "done", which would hide a run that hit a cap. */}
                  {!session.scanning && session.runReason ? ` Stopped because ${session.runReason}.` : ""}
                </p>
              )}
            </>
          )}
        </div>
      )}

      {!latest ? (
        <p className="text-xs text-slate-400 dark:text-slate-500">No price checks recorded yet.</p>
      ) : (
        <>
          <div className="mb-2 flex flex-wrap items-center justify-between gap-1">
            <p className="text-xs text-slate-400 dark:text-slate-500">
              Latest check &middot; {formatDateTime(latest.checkedAt)}
              {/* 2.2.0: just a presence indicator - the per-tier numbers
               *  themselves are saved (PriceCheck.tierBreakdown) and ready
               *  for a future charting view, marko's own spec, "## PRICE
               *  HISTORY" - not built out into a full trend chart here. */}
              {latest.tierBreakdown.length > 0 && ` · ${latest.tierBreakdown.length} tier${latest.tierBreakdown.length === 1 ? "" : "s"}`}
            </p>
            {trend && <TrendNote trend={trend} currency={latest.currency} />}
          </div>
          {/* 2.26.1 - THE collision in marko's screenshot. This was
              `grid-cols-5`: five FIXED columns regardless of how wide the card
              actually is. These cards sit three-across (`lg:grid-cols-3`), so
              one fifth of a third of the page is not enough for "EUR1,815.00"
              - it ran straight into the next cell and rendered as
              "EUR1,815.0064". Same class of bug as `.summary-bar` in 2.19.0,
              and the same fix: let the column COUNT follow the width, with a
              minimum a real money value fits in. */}
          <div
            className="mb-3 grid gap-x-3 gap-y-2 text-sm"
            style={{ gridTemplateColumns: "repeat(auto-fit, minmax(6.5rem, 1fr))" }}
          >
            <div>
              <p className="text-[11px] uppercase tracking-wide text-slate-400 dark:text-slate-500">Lowest</p>
              <p className="font-medium tabular-nums text-slate-900 dark:text-slate-100">{formatMoney(latest.lowestPriceCents, latest.currency)}</p>
            </div>
            <div>
              <p className="text-[11px] uppercase tracking-wide text-slate-400 dark:text-slate-500">Median</p>
              <p className="font-medium tabular-nums text-slate-900 dark:text-slate-100">{formatMoney(latest.medianPriceCents, latest.currency)}</p>
            </div>
            <div>
              <p className="text-[11px] uppercase tracking-wide text-slate-400 dark:text-slate-500">Average</p>
              <p className="font-medium tabular-nums text-slate-900 dark:text-slate-100">{formatMoney(latest.averagePriceCents, latest.currency)}</p>
            </div>
            <div>
              <p className="text-[11px] uppercase tracking-wide text-slate-400 dark:text-slate-500">Highest</p>
              <p className="font-medium tabular-nums text-slate-900 dark:text-slate-100">{formatMoney(latest.highestPriceCents, latest.currency)}</p>
            </div>
            <div>
              <p className="text-[11px] uppercase tracking-wide text-slate-400 dark:text-slate-500">Listings</p>
              <p className="font-medium tabular-nums text-slate-900 dark:text-slate-100">{latest.listingCount}</p>
            </div>
          </div>

          {older.length > 0 && (
            <div className="table-flush max-h-36 rounded-lg border border-slate-200 dark:border-slate-800">
              <table className="w-full border-collapse">
                <thead>
                  <tr>
                    <th className="px-2 py-1 text-left text-[11px] font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">Checked</th>
                    <th className="px-2 py-1 text-right text-[11px] font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">Lowest</th>
                    <th className="px-2 py-1 text-right text-[11px] font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">Median</th>
                    <th className="px-2 py-1 text-right text-[11px] font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">Avg</th>
                    <th className="px-2 py-1 text-right text-[11px] font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">Highest</th>
                    <th className="px-2 py-1 text-right text-[11px] font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">Listings</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
                  {older.map((c) => (
                    <tr key={c.id}>
                      <td className="px-2 py-1 text-[11px] text-slate-500 dark:text-slate-400">{formatDateTime(c.checkedAt)}</td>
                      <td className="px-2 py-1 text-right text-xs tabular-nums text-slate-700 dark:text-slate-300">{formatMoney(c.lowestPriceCents, c.currency)}</td>
                      <td className="px-2 py-1 text-right text-xs tabular-nums text-slate-700 dark:text-slate-300">{formatMoney(c.medianPriceCents, c.currency)}</td>
                      <td className="px-2 py-1 text-right text-xs tabular-nums text-slate-700 dark:text-slate-300">{formatMoney(c.averagePriceCents, c.currency)}</td>
                      <td className="px-2 py-1 text-right text-xs tabular-nums text-slate-700 dark:text-slate-300">{formatMoney(c.highestPriceCents, c.currency)}</td>
                      <td className="px-2 py-1 text-right text-xs tabular-nums text-slate-700 dark:text-slate-300">{c.listingCount}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </>
      )}
    </Card>
  );
}

// ---------------------------------------------------------------------------
// "Check Prices" entry form
// ---------------------------------------------------------------------------


// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

export default function PriceChecker() {
  const location = useLocation();
  const toast = useToast();
  // 2.10.0: the overview list replaced the dropdown, so the page loads one
  // aggregated row per upcoming event instead of a bare event list. Kept as
  // its own state (not derived from `summary`) because it describes EVERY
  // event, while `summary` only ever describes the one that is open.
  const [overview, setOverview] = useState<PriceCheckerEventOverview[]>([]);
  const [overviewLoading, setOverviewLoading] = useState(true);
  const [eventId, setEventId] = useState<number | "">("");
  const [summary, setSummary] = useState<PriceCheckerSummary | null>(null);
  const [loading, setLoading] = useState(false);

  // 2.1.9: every open Visible Scanner session on this page, keyed by
  // sessionKey(eventId, marketplaceId) - see ScannerCardState's own doc
  // comment for why the key includes eventId. Independent per session by
  // design (marko's own spec: "Ak jeden marketplace nefunguje, ostatné musia
  // fungovať") - unlike the old auto-check, there is no single shared
  // in-flight slot here.
  const [scannerSessions, setScannerSessions] = useState<Record<string, ScannerCardState>>({});
  // Mirrors scannerSessions for SYNCHRONOUS reads inside the event listeners
  // below and inside scanVisible/stopScan/closeScanner - same "ref mirrors
  // state for a long-lived subscription" idea this codebase already used for
  // the old auto-check's autoCheckRef/summaryRef.
  const scannerSessionsRef = useRef<Record<string, ScannerCardState>>({});
  useEffect(() => {
    scannerSessionsRef.current = scannerSessions;
  }, [scannerSessions]);
  // Mints a fresh id for every openScanner call - one id per session, reused
  // for every later scan/cancel/close call against that same window (this
  // codebase's established "frontend mints a request id, backend echoes it
  // back on every event" convention).
  const requestIdRef = useRef(0);

  useEffect(() => {
    // 2.2.2: only events still ahead of you are worth a market check - once
    // marko marks one "completed" (or it's "cancelled"), checking live
    // prices for it no longer means anything, so it should just quietly
    // stop showing up here, no manual untracking needed. Reuses the exact
    // same `status` field/value Events.tsx's own Upcoming/Completed tabs
    // already use (2.0.59) rather than inventing a date-based rule.
    // One call fills the whole overview. The `status === "upcoming"` rule
    // from 2.2.2 still applies - it just lives in SQL now (see
    // `list_price_checker_overview_impl`) rather than being filtered here.
    // This reads existing rows only: no scan, no marketplace request.
    api
      .listPriceCheckerOverview()
      .then(setOverview)
      .catch((e) => toast.error(errMsg(e)))
      .finally(() => setOverviewLoading(false));
    // Mirrors Orders.tsx's own presetEventId pattern - EventDetail's "Check
    // prices" button navigates here with the event already chosen, so marko
    // never has to find it again in the dropdown.
    const preset = location.state as { presetEventId?: number } | null;
    if (preset?.presetEventId) setEventId(preset.presetEventId);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // 2.1.9: the four Visible Scanner lifecycle events - see
  // commands/price_checker_scanner.rs's module doc comment (Rust) for the
  // full design. One subscription per event name for the whole page,
  // matching the (potentially several) concurrent sessions tracked in
  // scannerSessions above. Every handler resolves `requestId` back to a
  // session key via `keyForRequestId` and no-ops if it can't find one (a
  // stale event for a session already closed/superseded, or simply not
  // ours).
  useEffect(() => {
    let unlistenOpened: (() => void) | undefined;
    let unlistenError: (() => void) | undefined;
    let unlistenResult: (() => void) | undefined;
    let unlistenRunFinished: (() => void) | undefined;
    let unlistenClosed: (() => void) | undefined;
    let disposed = false;

    listen<ScannerOpenedPayload>(SCANNER_OPENED_EVENT, (event) => {
      const key = keyForRequestId(scannerSessionsRef.current, event.payload.requestId);
      if (!key) return;
      setScannerSessions((prev) => {
        const s = prev[key];
        if (!s || s.requestId !== event.payload.requestId) return prev;
        return { ...prev, [key]: { ...s, opening: false } };
      });
    }).then((fn) => {
      if (disposed) fn();
      else unlistenOpened = fn;
    });

    listen<ScannerErrorPayload>(SCANNER_ERROR_EVENT, (event) => {
      const key = keyForRequestId(scannerSessionsRef.current, event.payload.requestId);
      if (!key) return;
      toast.error(event.payload.message);
      setScannerSessions((prev) => {
        const s = prev[key];
        if (!s || s.requestId !== event.payload.requestId) return prev;
        const next = { ...prev };
        delete next[key];
        return next;
      });
    }).then((fn) => {
      if (disposed) fn();
      else unlistenError = fn;
    });

    listen<ScanResultPayload>(SCAN_RESULT_EVENT, (event) => {
      const p = event.payload;
      const key = keyForRequestId(scannerSessionsRef.current, p.requestId);
      if (!key) return;
      setScannerSessions((prev) => {
        const s = prev[key];
        if (!s || s.requestId !== p.requestId) return prev;
        return {
          ...prev,
          [key]: {
            ...s,
            // 2.27.0: a scan RESULT is now one pass of a run, not the end of
            // it. Clearing `scanning` here made the button flicker back to
            // idle between every pass. The run's own finished event is what
            // ends it - see RUN_FINISHED_EVENT below.
            status: p.status,
            listings: p.listings,
            lowestPriceCents: p.lowestPriceCents,
            medianPriceCents: p.medianPriceCents,
            averagePriceCents: p.averagePriceCents,
            highestPriceCents: p.highestPriceCents,
            currency: p.currency,
            scanCount: p.scanCount,
            lastScanAt: p.lastScanAt,
            message: p.message,
            lastScanFound: p.lastScanFound,
            lastScanAccepted: p.lastScanAccepted,
            lastScanSkipped: p.lastScanSkipped,
            lastScanDuplicates: p.lastScanDuplicates,
            lastScanSkipReasons: p.lastScanSkipReasons,
            statsListingCount: p.statsListingCount,
            statsExcludedCount: p.statsExcludedCount,
          },
        };
      });
    }).then((fn) => {
      if (disposed) fn();
      else unlistenResult = fn;
    });

    listen<ScanRunFinishedPayload>(RUN_FINISHED_EVENT, (event) => {
      const p = event.payload;
      const key = keyForRequestId(scannerSessionsRef.current, p.requestId);
      if (!key) return;
      setScannerSessions((prev) => {
        const s = prev[key];
        if (!s || s.requestId !== p.requestId) return prev;
        return { ...prev, [key]: { ...s, scanning: false, runReason: p.reason } };
      });
    }).then((fn) => {
      if (disposed) fn();
      else unlistenRunFinished = fn;
    });

    listen<ScannerClosedPayload>(SCANNER_CLOSED_EVENT, (event) => {
      const key = keyForRequestId(scannerSessionsRef.current, event.payload.requestId);
      if (!key) return;
      setScannerSessions((prev) => {
        const s = prev[key];
        if (!s || s.requestId !== event.payload.requestId) return prev;
        const next = { ...prev };
        delete next[key];
        return next;
      });
    }).then((fn) => {
      if (disposed) fn();
      else unlistenClosed = fn;
    });

    return () => {
      disposed = true;
      unlistenOpened?.();
      unlistenError?.();
      unlistenResult?.();
      unlistenRunFinished?.();
      unlistenClosed?.();
    };
  }, [toast]);

  const load = useCallback(() => {
    if (eventId === "") {
      setSummary(null);
      return;
    }
    setLoading(true);
    api
      .getPriceCheckerSummary(eventId)
      .then(setSummary)
      .catch((e) => toast.error(errMsg(e)))
      .finally(() => setLoading(false));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [eventId]);

  useEffect(() => {
    load();
  }, [load]);

  // "Open & Scan" - opens a real, visible browser window for one marketplace
  // card. Optimistically adds the session (opening: true) right away so the
  // card switches to the Scan/Stop/Close controls immediately; a failure
  // (bad URL, window couldn't open) removes it again via the catch below or
  // the price-scanner-error listener above.
  const openScanner = useCallback(
    (view: MarketplacePriceView, url: string) => {
      if (summary === null) return;
      const trimmedUrl = url.trim();
      if (!trimmedUrl) {
        toast.error("Enter this marketplace's listings page URL above first.");
        return;
      }
      const key = sessionKey(summary.eventId, view.marketplaceId);
      const myRequestId = ++requestIdRef.current;
      const initial: ScannerCardState = {
        requestId: myRequestId,
        opening: true,
        scanning: false,
        status: "ready",
        listings: [],
        lowestPriceCents: null,
        medianPriceCents: null,
        averagePriceCents: null,
        highestPriceCents: null,
        currency: null,
        scanCount: 0,
        lastScanAt: null,
        message: null,
        lastScanFound: 0,
        lastScanAccepted: 0,
        lastScanSkipped: 0,
        lastScanDuplicates: 0,
        lastScanSkipReasons: {},
        statsListingCount: 0,
        statsExcludedCount: 0,
      };
      setScannerSessions((prev) => ({ ...prev, [key]: initial }));
      api.openPriceScanner(myRequestId, summary.eventId, view.marketplaceId, trimmedUrl).catch((e) => {
        toast.error(errMsg(e));
        setScannerSessions((prev) => {
          const s = prev[key];
          if (!s || s.requestId !== myRequestId) return prev;
          const next = { ...prev };
          delete next[key];
          return next;
        });
      });
    },
    [summary, toast],
  );

  // "Scan Visible Prices" - reads whatever's on screen right now, once.
  const scanVisible = useCallback(
    (eventIdForSession: number, marketplaceId: number) => {
      const key = sessionKey(eventIdForSession, marketplaceId);
      const session = scannerSessionsRef.current[key];
      if (!session) return;
      setScannerSessions((prev) => {
        const s = prev[key];
        if (!s) return prev;
        return { ...prev, [key]: { ...s, scanning: true } };
      });
      // 2.27.0 - one press reads the WHOLE page now: scan, scroll, scan, until
      // it stops finding anything new. Marko: "stacu nechat otvoreny link a
      // ona to uz robi a ty si mozes robit ostatne veci." This returns at once
      // and the run continues on a backend thread, so leaving this page - or
      // the app being behind another window - does not interrupt it.
      api.startPriceScanRun(session.requestId).catch((e) => {
        toast.error(errMsg(e));
        setScannerSessions((prev) => {
          const s = prev[key];
          if (!s) return prev;
          return { ...prev, [key]: { ...s, scanning: false } };
        });
      });
    },
    [toast],
  );

  // "Stop scanning" - interrupts the in-flight scan (if the backend catches
  // it in time) and, either way, immediately clears the local `scanning`
  // flag so the button is usable again right away - marko's own "Hlavná
  // aplikácia musí byť stále úplne použiteľná" (the main app must stay
  // fully usable) applies here exactly as it did to the old auto-check's
  // Cancel button.
  const stopScan = useCallback(
    (eventIdForSession: number, marketplaceId: number) => {
      const key = sessionKey(eventIdForSession, marketplaceId);
      const session = scannerSessionsRef.current[key];
      if (!session) return;
      api.cancelPriceScan(session.requestId).catch((e) => toast.error(errMsg(e)));
      setScannerSessions((prev) => {
        const s = prev[key];
        if (!s || s.requestId !== session.requestId) return prev;
        return { ...prev, [key]: { ...s, scanning: false } };
      });
    },
    [toast],
  );

  // "Close" - ends the session and closes the real window. Removes the
  // session from local state immediately rather than waiting for the
  // price-scanner-closed event (which still arrives and safely no-ops by
  // then, since the session is already gone from scannerSessionsRef).
  const closeScanner = useCallback(
    (eventIdForSession: number, marketplaceId: number) => {
      const key = sessionKey(eventIdForSession, marketplaceId);
      const session = scannerSessionsRef.current[key];
      if (!session) return;
      api.closePriceScanner(session.requestId, true).catch((e) => toast.error(errMsg(e)));
      setScannerSessions((prev) => {
        const next = { ...prev };
        delete next[key];
        return next;
      });
    },
    [toast],
  );

  return (
    <div>
      <PageHeader title="Price Checker" subtitle="Compare your unsold inventory against Vivid Seats, Ticombo and Viagogo." />

      {/* 2.10.0: with the dropdown gone, this is the way back to the list -
          the page is now "overview, then one event", so leaving an event has
          to be an explicit control rather than re-picking a blank option. */}
      {eventId !== "" && (
        <button
          type="button"
          onClick={() => setEventId("")}
          className="mb-3 inline-flex items-center gap-1.5 text-xs font-medium text-slate-500 transition hover:text-slate-800 dark:text-slate-400 dark:hover:text-slate-200"
        >
          <IconArrowLeft className="h-3.5 w-3.5" /> All events
        </button>
      )}

      {eventId === "" ? (
        <EventOverviewList rows={overview} loading={overviewLoading} onOpen={setEventId} />
      ) : loading || !summary ? (
        <LoadingBlock />
      ) : (
        <>
          <div className="summary-bar">
            <StatCard label="Unsold tickets" value={String(summary.unsoldTicketCount)} />
            <StatCard
              label="My avg. purchase cost"
              value={formatMyMoney(summary.myAvgPurchaseCostCents, summary.myCurrency, summary.unsoldTicketCount)}
            />
            <StatCard
              label="My avg. listing price"
              value={formatMyMoney(summary.myAvgListingPriceCents, summary.myCurrency, summary.unsoldTicketCount)}
              sub={
                summary.missingListingPriceCount > 0
                  ? `${summary.missingListingPriceCount} unsold ticket${summary.missingListingPriceCount === 1 ? "" : "s"} not listed yet`
                  : undefined
              }
            />
            <StatCard label="Currency" value={summary.myCurrency ?? (summary.unsoldTicketCount === 0 ? "-" : "Mixed")} />
          </div>
          {summary.unsoldTicketCount > 0 && summary.myCurrency === null && (
            <p className="-mt-4 mb-6 text-xs text-amber-700 dark:text-amber-400">
              This event&apos;s unsold tickets are in more than one currency, so the market comparison below can&apos;t pick
              which one to compare against.
            </p>
          )}

          <Card className="mb-6 p-4">
            <p className="mb-3 section-title">Market vs. mine</p>
            <div className="summary-bar">
              <StatCard label="Market lowest" value={formatMoney(summary.marketLowestPriceCents, summary.myCurrency ?? "EUR")} />
              <StatCard label="Market average" value={formatMoney(summary.marketAveragePriceCents, summary.myCurrency ?? "EUR")} />
              <StatCard
                label="Recommended price"
                value={formatMoney(summary.recommendedPriceCents, summary.myCurrency ?? "EUR")}
                sub="5% below the lowest market price"
              />
              <StatCard
                label="Expected profit"
                value={formatMoney(summary.expectedProfitCents, summary.myCurrency ?? "EUR")}
                tone={
                  summary.expectedProfitCents == null
                    ? "default"
                    : summary.expectedProfitCents > 0
                      ? "positive"
                      : summary.expectedProfitCents < 0
                        ? "negative"
                        : "default"
                }
              />
              <StatCard label="Expected ROI" value={formatPercent(summary.expectedRoi)} />
            </div>
            {summary.marketLowestPriceCents === null && (
              <p className="mt-3 text-xs text-slate-400 dark:text-slate-500">
                {summary.myCurrency === null
                  ? "Add unsold tickets in one currency and at least one price check to see a market comparison."
                  : `No price check yet matches your own currency (${summary.myCurrency}) - use "Check Prices" below on a marketplace to add one.`}
              </p>
            )}
          </Card>

          <p className="mb-3 section-title">Marketplaces</p>

          <div className="grid grid-cols-1 gap-4 lg:grid-cols-3">
            {summary.marketplaces.map((view) => (
              <MarketplaceCard
                key={view.marketplaceId}
                eventId={summary.eventId}
                view={view}
                onLinkSaved={load}
                session={scannerSessions[sessionKey(summary.eventId, view.marketplaceId)]}
                onOpenScanner={openScanner}
                onScanVisible={scanVisible}
                onStopScan={stopScan}
                onCloseScanner={closeScanner}
              />
            ))}
          </div>

        </>
      )}
    </div>
  );
}

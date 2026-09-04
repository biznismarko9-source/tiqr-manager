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
import { useCallback, useEffect, useRef, useState } from "react";
import { useLocation } from "react-router-dom";
import { listen } from "@tauri-apps/api/event";
import { api, errMsg } from "../lib/api";
import type {
  ComparableLevel,
  ComparableReferenceInput,
  CurrencyMarketAnalysis,
  DataQuality,
  EventWithStats,
  MarketAlert,
  MarketAnalysisResult,
  MarketMonitorMarketplaceView,
  MarketMonitorSummary,
  MarketplacePriceView,
  MarketSnapshot,
  MarketSourceStatus,
  NormalizedListing,
  PriceCheck,
  PriceCheckerSummary,
  RankedComparable,
  ScannerClosedPayload,
  ScannerErrorPayload,
  ScannerOpenedPayload,
  ScannerStatus,
  ScanResultPayload,
  TierBreakdownInput,
  YourTicketGroup,
} from "../lib/types";
import { centsToDecimalString, decimalStringToCents, formatDateTime, formatMoney, formatMoneyOrMixed, formatPercent } from "../lib/format";
import {
  Button,
  Card,
  CHECKBOX_CLASS,
  EmptyState,
  Field,
  Input,
  LoadingBlock,
  Modal,
  ModalFooter,
  PageHeader,
  Select,
  Spinner,
  StatCard,
  Textarea,
} from "../components/ui";
import { IconAlertTriangle, IconLink, IconRefresh, IconTag, IconTrendingDown, IconTrendingUp, IconX } from "../components/icons";
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
// Live Market Monitor (2.4.1) - marko's replacement for the cancelled "Live
// Event Intelligence" direction: every online/live-market capability lives
// directly inside Price Checker instead of a separate section. Built
// entirely on top of the Visible Scanner above - every scan it completes is
// recorded automatically on the backend (see commands/price_checker_
// monitor.rs's module doc comment, Rust); nothing here opens a window, reads
// a page, or bypasses any CAPTCHA/anti-bot check - marko's own explicit "no
// automation beyond reading whatever's already on screen" boundary applies
// here exactly as it does to the manual scanner above.
// ---------------------------------------------------------------------------

const MARKET_SOURCE_STATUS_META: Record<MarketSourceStatus, { label: string; className: string }> = {
  not_connected: { label: "Not connected", className: "bg-slate-100 text-slate-500 dark:bg-slate-800 dark:text-slate-400" },
  connected: { label: "Connected", className: "bg-slate-100 text-slate-600 dark:bg-slate-800 dark:text-slate-300" },
  success: { label: "Success", className: "bg-emerald-50 text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-400" },
  failed: { label: "Failed", className: "bg-red-50 text-red-700 dark:bg-red-900/30 dark:text-red-400" },
};

/** "market_drop"/"source_failure" read as amber/red (something changed, or
 * something needs fixing); the other 3 are plain market observations, never
 * an instruction to act - same Attention/Info split
 * commands::attention_center's own market_alert priority mapping already
 * uses (Rust), kept visually consistent here rather than a third opinion. */
const MARKET_ALERT_META: Record<MarketAlert["alertType"], { label: string; className: string }> = {
  market_drop: { label: "Market drop", className: "bg-amber-50 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400" },
  market_rise: { label: "Market rise", className: "bg-slate-100 text-slate-600 dark:bg-slate-800 dark:text-slate-300" },
  new_supply: { label: "New supply", className: "bg-slate-100 text-slate-600 dark:bg-slate-800 dark:text-slate-300" },
  supply_drop: { label: "Supply drop", className: "bg-slate-100 text-slate-600 dark:bg-slate-800 dark:text-slate-300" },
  source_failure: { label: "Source failure", className: "bg-red-50 text-red-700 dark:bg-red-900/30 dark:text-red-400" },
};

/** marko's own Section 5: interval-based, never aggressive polling - the
 * shortest option is 15 minutes. Auto Monitor is a plain frontend timer
 * over an already-open, human-opened scanner window (see MarketplaceCard's
 * own doc comment on its auto-monitor effect below) - it reuses the exact
 * same "Scan Visible Prices" call the button makes, on a schedule, nothing
 * more. */
const AUTO_MONITOR_INTERVALS: { label: string; minutes: number }[] = [
  { label: "Every 15 min", minutes: 15 },
  { label: "Every 30 min", minutes: 30 },
  { label: "Every hour", minutes: 60 },
  { label: "Every 3 hours", minutes: 180 },
  { label: "Every 6 hours", minutes: 360 },
];

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
  lastScanAt: string | null;
  message: string | null;
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

/** What `SavePriceCheckModal` gets prefilled with when opened from a live
 * scan's "Save to history" button, instead of a normal blank/latest-check
 * open. Structured cents values straight from the scan session - no text
 * round-trip needed (unlike the old auto-check's paste-pipeline hack),
 * since the scanner already produced exact numbers. */
interface ScanPrefill {
  lowestPriceCents: number;
  medianPriceCents: number | null;
  averagePriceCents: number;
  highestPriceCents: number;
  listingCount: number;
  currency: string | null;
  /** 2.2.0: this session's own Market Analysis tier breakdown for
   * `currency` above, if any was computed by the time "Save to history" was
   * clicked - carried straight through to `savePriceCheck` so
   * `PriceCheck.tierBreakdown` (history/"## PRICE HISTORY") isn't left
   * empty just because a perfectly good breakdown was sitting right there.
   * Empty when no analysis was available yet, or this currency has none. */
  tierBreakdown: TierBreakdownInput[];
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

/** One currency's tier/section breakdown - marko's own spec, "## TIER
 * PRICING" + "## MAP / SECTION ANALYSIS". Deliberately NOT a literal seating
 * chart (marko's spec explicitly doesn't require one) - a plain, scannable
 * list of tiers lowest-price-first, each with its own sections lowest-price-
 * first underneath. */
function CurrencyMarketBlock({ block }: { block: CurrencyMarketAnalysis }) {
  return (
    <div className="mb-4 last:mb-0">
      <p className="mb-2 text-xs font-semibold text-slate-500 dark:text-slate-400">{block.currency} market</p>
      <div className="mb-3 grid grid-cols-2 gap-2 text-sm sm:grid-cols-5">
        <div>
          <p className="text-[11px] uppercase tracking-wide text-slate-400 dark:text-slate-500">Listings</p>
          <p className="font-medium tabular-nums text-slate-900 dark:text-slate-100">{block.overall.listingCount}</p>
        </div>
        <div>
          <p className="text-[11px] uppercase tracking-wide text-slate-400 dark:text-slate-500">Lowest</p>
          <p className="font-medium tabular-nums text-slate-900 dark:text-slate-100">{formatMoney(block.overall.lowestPriceCents, block.currency)}</p>
        </div>
        <div>
          <p className="text-[11px] uppercase tracking-wide text-slate-400 dark:text-slate-500">Median</p>
          <p className="font-medium tabular-nums text-slate-900 dark:text-slate-100">{formatMoney(block.overall.medianPriceCents, block.currency)}</p>
        </div>
        <div>
          <p className="text-[11px] uppercase tracking-wide text-slate-400 dark:text-slate-500">Average</p>
          <p className="font-medium tabular-nums text-slate-900 dark:text-slate-100">{formatMoney(block.overall.averagePriceCents, block.currency)}</p>
        </div>
        <div>
          <p className="text-[11px] uppercase tracking-wide text-slate-400 dark:text-slate-500">Highest</p>
          <p className="font-medium tabular-nums text-slate-900 dark:text-slate-100">{formatMoney(block.overall.highestPriceCents, block.currency)}</p>
        </div>
      </div>

      <div className="space-y-2">
        {block.tiers.map((tier) => (
          <div key={tier.tier} className="rounded-lg border border-slate-100 p-2 dark:border-slate-800">
            <div className="flex flex-wrap items-center justify-between gap-x-3 gap-y-0.5">
              <p className="text-xs font-semibold text-slate-700 dark:text-slate-300">{tier.tier}</p>
              <p className="text-xs tabular-nums text-slate-500 dark:text-slate-400">
                {formatMoney(tier.stats.lowestPriceCents, block.currency)} &ndash; {formatMoney(tier.stats.highestPriceCents, block.currency)}
                {" · "}
                {tier.stats.listingCount} listing{tier.stats.listingCount === 1 ? "" : "s"}
              </p>
            </div>
            {tier.sections.length > 0 && (
              <div className="mt-1.5 flex flex-wrap gap-x-4 gap-y-1">
                {tier.sections.map((s) => (
                  <span key={s.section} className="text-[11px] text-slate-500 dark:text-slate-400">
                    Sec {s.section}: <span className="tabular-nums text-slate-700 dark:text-slate-300">{formatMoney(s.stats.lowestPriceCents, block.currency)}</span>{" "}
                    ({s.stats.listingCount})
                  </span>
                ))}
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
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
    <div className="max-h-56 overflow-auto rounded-lg border border-slate-100 dark:border-slate-800">
      <table className="w-full border-collapse">
        <thead className="sticky top-0 bg-slate-50 dark:bg-slate-800/60">
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

/** The whole Market Analysis block for one scanner session - tier/section
 * pricing per currency, then "Your Tickets" with recommendations. `analysis`
 * is null before the first scan (or while one is loading); this never blocks
 * or replaces the existing raw listings table above it, purely additive. */
function MarketAnalysisPanel({ analysis, loading, error }: { analysis: MarketAnalysisResult | null; loading: boolean; error: string | null }) {
  return (
    <div className="mt-4 border-t border-slate-100 pt-4 dark:border-slate-800">
      <div className="mb-2 flex items-center gap-2">
        <p className="text-xs font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">Market Analysis</p>
        {loading && <Spinner className="h-3.5 w-3.5" />}
      </div>
      {error && (
        <p className="mb-2 flex items-start gap-1.5 text-xs text-amber-700 dark:text-amber-400">
          <IconAlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0" />
          {error}
        </p>
      )}
      {!analysis ? (
        !loading && !error && <p className="text-xs text-slate-400 dark:text-slate-500">Scan to see tier/section pricing and recommendations.</p>
      ) : (
        <>
          {analysis.mixedCurrencies && (
            <p className="mb-3 text-xs text-amber-700 dark:text-amber-400">
              These listings span more than one currency - shown separately below, never blended together.
            </p>
          )}
          {analysis.uncurrenciedListingCount > 0 && (
            <p className="mb-3 text-xs text-slate-400 dark:text-slate-500">
              {analysis.uncurrenciedListingCount} listing{analysis.uncurrenciedListingCount === 1 ? "" : "s"} had a price but no
              detected currency, so {analysis.uncurrenciedListingCount === 1 ? "it isn't" : "they aren't"} included below.
            </p>
          )}
          {analysis.byCurrency.length === 0 ? (
            <p className="text-xs text-slate-400 dark:text-slate-500">None of the listings found so far have a usable currency yet.</p>
          ) : (
            analysis.byCurrency.map((block) => <CurrencyMarketBlock key={block.currency} block={block} />)
          )}

          <p className="mb-2 mt-4 text-xs font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">Your Tickets</p>
          <YourTicketsTable groups={analysis.yourTickets} />
        </>
      )}
    </div>
  );
}

/** "## COMPARABLE MARKET" - ranks this session's listings against ONE
 * specific reference ticket marko types in (his own worked example: Section
 * 112 / Row 8 / Quantity 4). Self-contained (fetches on its own "Compare"
 * click, no shared state with MarketAnalysisPanel) since it's a one-off
 * lookup, not something that needs to refresh automatically on every scan.
 * `currencies` restricts the picker to currencies this session actually has
 * listings in - comparing against an empty currency would only ever come
 * back with zero results. */
function ComparableMarketTool({ requestId, currencies }: { requestId: number; currencies: string[] }) {
  const toast = useToast();
  const [section, setSection] = useState("");
  const [tier, setTier] = useState("");
  const [row, setRow] = useState("");
  const [quantity, setQuantity] = useState("");
  const [currency, setCurrency] = useState("");
  const [results, setResults] = useState<RankedComparable[] | null>(null);
  const [loading, setLoading] = useState(false);

  // Keeps the picker pointed at a currency that actually has data - resets
  // to the first available one whenever the current selection stops being
  // valid (e.g. this is the very first scan, or a currency this session
  // never had disappears from the list - which in practice never happens
  // once a currency has appeared, but stays correct either way).
  useEffect(() => {
    setCurrency((c) => (currencies.includes(c) ? c : (currencies[0] ?? "")));
  }, [currencies]);

  if (currencies.length === 0) return null;

  const compare = async () => {
    if (!currency) return;
    setLoading(true);
    try {
      const qty = quantity.trim() === "" ? NaN : parseInt(quantity, 10);
      const input: ComparableReferenceInput = {
        requestId,
        section: section.trim() || null,
        tier: tier.trim() || null,
        row: row.trim() || null,
        quantity: Number.isFinite(qty) && qty > 0 ? qty : null,
        currency,
      };
      setResults(await api.computeComparableMarket(input));
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="mt-4 rounded-lg border border-slate-100 p-3 dark:border-slate-800">
      <p className="mb-2 text-xs font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">Compare a specific ticket</p>
      <div className="grid grid-cols-2 gap-2 sm:grid-cols-5">
        <Input placeholder="Section" value={section} onChange={(e) => setSection(e.target.value)} className="text-xs" />
        <Input placeholder="Tier / level" value={tier} onChange={(e) => setTier(e.target.value)} className="text-xs" />
        <Input placeholder="Row" value={row} onChange={(e) => setRow(e.target.value)} className="text-xs" />
        <Input type="number" min={1} step={1} placeholder="Quantity" value={quantity} onChange={(e) => setQuantity(e.target.value)} className="text-xs" />
        {currencies.length > 1 ? (
          <Select value={currency} onChange={(e) => setCurrency(e.target.value)}>
            {currencies.map((c) => (
              <option key={c} value={c}>
                {c}
              </option>
            ))}
          </Select>
        ) : (
          <div className="flex items-center px-1 text-xs text-slate-500 dark:text-slate-400">{currency}</div>
        )}
      </div>
      <div className="mt-2">
        <Button variant="secondary" onClick={compare} disabled={loading}>
          {loading ? <Spinner className="h-4 w-4" /> : "Compare"}
        </Button>
      </div>

      {results &&
        (results.length === 0 ? (
          <p className="mt-2 text-xs text-slate-400 dark:text-slate-500">No {currency} listings found yet to compare against.</p>
        ) : (
          <div className="mt-3 max-h-56 overflow-auto rounded-lg border border-slate-100 dark:border-slate-800">
            <table className="w-full border-collapse">
              <thead className="sticky top-0 bg-slate-50 dark:bg-slate-800/60">
                <tr>
                  <th className="px-2 py-1 text-right text-[11px] font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">Price</th>
                  <th className="px-2 py-1 text-left text-[11px] font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">Section</th>
                  <th className="px-2 py-1 text-left text-[11px] font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">Tier</th>
                  <th className="px-2 py-1 text-left text-[11px] font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">Row</th>
                  <th className="px-2 py-1 text-left text-[11px] font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">Match</th>
                  <th className="px-2 py-1 text-left text-[11px] font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">Data</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
                {results.map((r, i) => (
                  <tr key={i}>
                    <td className="px-2 py-1 text-right text-xs tabular-nums text-slate-700 dark:text-slate-300">
                      {formatMoney(r.listing.priceCents, r.listing.currency ?? currency)}
                    </td>
                    <td className="px-2 py-1 text-xs text-slate-700 dark:text-slate-300">{r.listing.section ?? "-"}</td>
                    <td className="px-2 py-1 text-xs text-slate-700 dark:text-slate-300">{r.listing.tier ?? "-"}</td>
                    <td className="px-2 py-1 text-xs text-slate-700 dark:text-slate-300">{r.listing.row ?? "-"}</td>
                    <td className="px-2 py-1">
                      <LevelPill level={r.level} />
                    </td>
                    <td className="px-2 py-1">
                      <DataQualityPill quality={r.dataQuality} />
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ))}
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
  monitor,
  onLinkSaved,
  onCheckPrices,
  session,
  onOpenScanner,
  onScanVisible,
  onStopScan,
  onCloseScanner,
  onSaveScanToHistory,
}: {
  eventId: number;
  view: MarketplacePriceView;
  /** This card's Live Market Monitor data (2.4.0) - undefined only for the
   *  brief moment `PriceChecker`'s own `monitor` summary hasn't loaded yet;
   *  the section below simply doesn't render until it has. */
  monitor: MarketMonitorMarketplaceView | undefined;
  onLinkSaved: () => void;
  onCheckPrices: () => void;
  /** This card's live Visible Scanner session, if one is open - undefined
   *  means no session (shows "Open & Scan" instead of Scan/Stop/Close). */
  session: ScannerCardState | undefined;
  onOpenScanner: (view: MarketplacePriceView, url: string) => void;
  onScanVisible: (eventId: number, marketplaceId: number) => void;
  onStopScan: (eventId: number, marketplaceId: number) => void;
  onCloseScanner: (eventId: number, marketplaceId: number) => void;
  onSaveScanToHistory: (view: MarketplacePriceView, session: ScannerCardState, analysis: MarketAnalysisResult | null) => void;
}) {
  const toast = useToast();
  const [url, setUrl] = useState(view.link?.url ?? "");
  const [savingLink, setSavingLink] = useState(false);

  // ---- Live Market Monitor: Auto Monitor + Market History (2.4.0) --------

  const [autoMonitorEnabled, setAutoMonitorEnabled] = useState(false);
  const [autoMonitorMinutes, setAutoMonitorMinutes] = useState(30);
  // Mirrors `session` for the interval callback below - same "ref mirrors
  // state for a long-lived subscription" idea this file already uses for
  // scannerSessionsRef, needed here because the effect's own dependency
  // array deliberately does NOT include the whole `session` object (see
  // that effect's own comment for why).
  const sessionRef = useRef(session);
  useEffect(() => {
    sessionRef.current = session;
  }, [session]);

  // Auto Monitor never opens a window, navigates, or reads anything the
  // manual "Scan Visible Prices" button couldn't already read - it's the
  // exact same onScanVisible call as that button, fired on a schedule
  // instead of a click (marko's own Section 4/5: "no CAPTCHA bypass, no
  // proxy rotation, no anti-bot workaround" - this never crosses that line
  // because it never touches the page itself, only asks the backend to read
  // whatever's currently rendered). Deliberately keyed on `session?.
  // requestId`, not the whole `session` object, so a scan settling (which
  // replaces `session` with a new object every time) doesn't tear down and
  // rebuild this interval on every single tick - only turning the toggle or
  // the interval on/off, or a genuinely NEW session (new requestId), does.
  useEffect(() => {
    if (!autoMonitorEnabled || !session) return;
    const intervalMs = autoMonitorMinutes * 60_000;
    const id = window.setInterval(() => {
      const current = sessionRef.current;
      // Never overlap a scan already in flight, and never fire into a
      // window that's still opening.
      if (!current || current.opening || current.scanning) return;
      onScanVisible(eventId, view.marketplaceId);
    }, intervalMs);
    return () => window.clearInterval(id);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [autoMonitorEnabled, autoMonitorMinutes, session?.requestId, eventId, view.marketplaceId]);

  // Nothing left to auto-scan once the window closes - leaving the toggle
  // "on" with no session would be misleading, not just inert.
  useEffect(() => {
    if (!session) setAutoMonitorEnabled(false);
  }, [session]);

  const [historyOpen, setHistoryOpen] = useState(false);
  const [historyRows, setHistoryRows] = useState<MarketSnapshot[] | null>(null);
  const [historyLoading, setHistoryLoading] = useState(false);
  const openHistory = () => {
    setHistoryOpen(true);
    setHistoryLoading(true);
    api
      .listMarketSnapshots(eventId, view.marketplaceId, 30)
      .then(setHistoryRows)
      .catch((e) => toast.error(errMsg(e)))
      .finally(() => setHistoryLoading(false));
  };

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
        {/* Check Prices only makes sense for a marketplace still accepting
         *  new checks - the backend refuses a new price check against a
         *  retired one either way (require_marketplace_active in
         *  price_checker.rs), this just keeps marko from ever seeing a
         *  button that would only error. */}
        {view.marketplaceActive && (
          <Button variant="secondary" onClick={onCheckPrices}>
            <IconTag className="h-4 w-4" /> Check Prices
          </Button>
        )}
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
        // Read-only, same styling as SavePriceCheckModal's own URL display -
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
            <p className="text-xs font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">Visible Scanner</p>
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
                  : "Opens a real, visible browser window on this page - scroll it yourself, then click Scan Visible Prices."
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
                  <IconTag className="h-4 w-4" /> Scan Visible Prices
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

              {session.listings.length > 0 && (
                <>
                  <div className="mt-3 grid grid-cols-2 gap-2 text-sm sm:grid-cols-4 lg:grid-cols-7">
                    <div>
                      <p className="text-[11px] uppercase tracking-wide text-slate-400 dark:text-slate-500">Listings</p>
                      <p className="font-medium tabular-nums text-slate-900 dark:text-slate-100">{session.listings.length}</p>
                    </div>
                    <div>
                      <p className="text-[11px] uppercase tracking-wide text-slate-400 dark:text-slate-500">Lowest</p>
                      <p className="font-medium tabular-nums text-slate-900 dark:text-slate-100">
                        {formatMoney(session.lowestPriceCents, session.currency ?? "EUR")}
                      </p>
                    </div>
                    <div>
                      <p className="text-[11px] uppercase tracking-wide text-slate-400 dark:text-slate-500">Median</p>
                      <p className="font-medium tabular-nums text-slate-900 dark:text-slate-100">
                        {formatMoney(session.medianPriceCents, session.currency ?? "EUR")}
                      </p>
                    </div>
                    <div>
                      <p className="text-[11px] uppercase tracking-wide text-slate-400 dark:text-slate-500">Average</p>
                      <p className="font-medium tabular-nums text-slate-900 dark:text-slate-100">
                        {formatMoney(session.averagePriceCents, session.currency ?? "EUR")}
                      </p>
                    </div>
                    <div>
                      <p className="text-[11px] uppercase tracking-wide text-slate-400 dark:text-slate-500">Highest</p>
                      <p className="font-medium tabular-nums text-slate-900 dark:text-slate-100">
                        {formatMoney(session.highestPriceCents, session.currency ?? "EUR")}
                      </p>
                    </div>
                    <div>
                      <p className="text-[11px] uppercase tracking-wide text-slate-400 dark:text-slate-500">Currency</p>
                      <p className="font-medium text-slate-900 dark:text-slate-100">{session.currency ?? "-"}</p>
                    </div>
                    <div>
                      <p className="text-[11px] uppercase tracking-wide text-slate-400 dark:text-slate-500">Last scan</p>
                      <p className="font-medium tabular-nums text-slate-900 dark:text-slate-100">{formatDateTime(session.lastScanAt)}</p>
                    </div>
                  </div>

                  <div className="mt-3 max-h-48 overflow-y-auto rounded-lg border border-slate-100 dark:border-slate-800">
                    <table className="w-full border-collapse">
                      <thead className="sticky top-0 bg-slate-50 dark:bg-slate-800/60">
                        <tr>
                          <th className="px-2 py-1 text-right text-[11px] font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">Price</th>
                          <th className="px-2 py-1 text-left text-[11px] font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">Tier</th>
                          <th className="px-2 py-1 text-left text-[11px] font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">Section</th>
                          <th className="px-2 py-1 text-left text-[11px] font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">Row</th>
                          <th className="px-2 py-1 text-right text-[11px] font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">Qty</th>
                          <th className="px-2 py-1 text-left text-[11px] font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">Marketplace</th>
                        </tr>
                      </thead>
                      <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
                        {session.listings.map((l, i) => (
                          <tr key={i}>
                            <td className="px-2 py-1 text-right text-xs tabular-nums text-slate-700 dark:text-slate-300">
                              {formatMoney(l.priceCents, l.currency ?? session.currency ?? "EUR")}
                            </td>
                            <td className="px-2 py-1 text-xs text-slate-700 dark:text-slate-300">{l.tier ?? "-"}</td>
                            <td className="px-2 py-1 text-xs text-slate-700 dark:text-slate-300">{l.section ?? "-"}</td>
                            <td className="px-2 py-1 text-xs text-slate-700 dark:text-slate-300">{l.row ?? "-"}</td>
                            <td className="px-2 py-1 text-right text-xs tabular-nums text-slate-700 dark:text-slate-300">{l.quantity ?? "-"}</td>
                            <td className="px-2 py-1 text-xs text-slate-700 dark:text-slate-300">{l.marketplace}</td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>

                  <div className="mt-3">
                    <Button variant="primary" onClick={() => onSaveScanToHistory(view, session, analysis)}>
                      Save to history
                    </Button>
                  </div>

                  <MarketAnalysisPanel analysis={analysis} loading={analysisLoading} error={analysisError} />
                  <ComparableMarketTool requestId={session.requestId} currencies={analysis?.byCurrency.map((c) => c.currency) ?? []} />
                </>
              )}
            </>
          )}
        </div>
      )}

      {/* Live Market Monitor (2.4.0) - built entirely on top of the Visible
       *  Scanner above: Auto Monitor just fires the same "Scan Visible
       *  Prices" call on a schedule, and every scan (from either) already
       *  recorded its own snapshot/alerts on the backend (see this file's
       *  own module doc comment, and commands/price_checker_monitor.rs's,
       *  Rust). `monitor` is undefined only for the brief moment the page's
       *  own summary hasn't loaded yet. */}
      {view.marketplaceActive && monitor && (
        <div className="mb-4 rounded-lg border border-slate-100 p-3 dark:border-slate-800">
          <div className="mb-2 flex flex-wrap items-center justify-between gap-2">
            <p className="text-xs font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">Live Market Monitor</p>
            <span
              className={`inline-flex items-center rounded-full px-2 py-0.5 text-[11px] font-medium ${
                session?.scanning ? SCANNER_STATUS_META.scanning.className : MARKET_SOURCE_STATUS_META[monitor.status].className
              }`}
            >
              {session?.scanning ? "Scanning..." : MARKET_SOURCE_STATUS_META[monitor.status].label}
            </span>
          </div>

          <p className="text-[11px] text-slate-400 dark:text-slate-500">
            Last successful scan &middot; {monitor.lastSuccessfulScanAt ? formatDateTime(monitor.lastSuccessfulScanAt) : "Never"}
          </p>
          {/* Cache/offline (marko's own Section 12): a run of failures never
           *  clears the last success above, it only adds this short,
           *  human-readable line alongside it - see MarketSourceStatus's own
           *  doc comment (types.ts). */}
          {monitor.lastErrorMessage && (
            <p className="mt-1 flex items-start gap-1.5 text-xs text-amber-700 dark:text-amber-400">
              <IconAlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0" />
              {monitor.lastErrorMessage}
            </p>
          )}

          {monitor.latestSnapshot && (
            <div className="mt-2 grid grid-cols-2 gap-2 text-sm sm:grid-cols-5">
              <div>
                <p className="text-[11px] uppercase tracking-wide text-slate-400 dark:text-slate-500">Listings</p>
                <p className="font-medium tabular-nums text-slate-900 dark:text-slate-100">{monitor.latestSnapshot.listingCount}</p>
              </div>
              <div>
                <p className="text-[11px] uppercase tracking-wide text-slate-400 dark:text-slate-500">Lowest</p>
                <p className="font-medium tabular-nums text-slate-900 dark:text-slate-100">
                  {formatMoney(monitor.latestSnapshot.lowestPriceCents, monitor.latestSnapshot.currency)}
                </p>
              </div>
              <div>
                <p className="text-[11px] uppercase tracking-wide text-slate-400 dark:text-slate-500">Median</p>
                <p className="font-medium tabular-nums text-slate-900 dark:text-slate-100">
                  {formatMoney(monitor.latestSnapshot.medianPriceCents, monitor.latestSnapshot.currency)}
                </p>
              </div>
              <div>
                <p className="text-[11px] uppercase tracking-wide text-slate-400 dark:text-slate-500">Average</p>
                <p className="font-medium tabular-nums text-slate-900 dark:text-slate-100">
                  {formatMoney(monitor.latestSnapshot.averagePriceCents, monitor.latestSnapshot.currency)}
                </p>
              </div>
              <div>
                <p className="text-[11px] uppercase tracking-wide text-slate-400 dark:text-slate-500">Highest</p>
                <p className="font-medium tabular-nums text-slate-900 dark:text-slate-100">
                  {formatMoney(monitor.latestSnapshot.highestPriceCents, monitor.latestSnapshot.currency)}
                </p>
              </div>
            </div>
          )}

          <div className="mt-3 flex flex-wrap items-center gap-3">
            {/* marko's own Section 5: interval-based, ON/OFF, never
             *  aggressive - disabled entirely without an open scanner
             *  session, since there's nothing for it to read yet (see
             *  the auto-disable-on-close effect above). */}
            <label
              className="flex items-center gap-1.5 text-xs font-medium text-slate-600 dark:text-slate-300"
              title={!session ? "Open & Scan above first - Auto Monitor reschedules the same scan, it can't open a window itself." : undefined}
            >
              <input
                type="checkbox"
                className={CHECKBOX_CLASS}
                checked={autoMonitorEnabled}
                disabled={!session}
                onChange={(e) => setAutoMonitorEnabled(e.target.checked)}
              />
              Auto Monitor
            </label>
            <div className="w-36">
              <Select
                value={String(autoMonitorMinutes)}
                onChange={(e) => setAutoMonitorMinutes(Number(e.target.value))}
                disabled={!session || !autoMonitorEnabled}
              >
                {AUTO_MONITOR_INTERVALS.map((opt) => (
                  <option key={opt.minutes} value={opt.minutes}>
                    {opt.label}
                  </option>
                ))}
              </Select>
            </div>
            <Button variant="secondary" onClick={openHistory}>
              History
            </Button>
          </div>

          {monitor.recentAlerts.length > 0 && (
            <div className="mt-3 space-y-1.5">
              <p className="text-[11px] font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">Market Alerts</p>
              {monitor.recentAlerts.map((a) => (
                <div key={a.id} className="flex items-start gap-1.5 text-xs text-slate-600 dark:text-slate-300">
                  <span
                    className={`mt-0.5 inline-flex shrink-0 items-center rounded-full px-2 py-0.5 text-[11px] font-medium ${MARKET_ALERT_META[a.alertType].className}`}
                  >
                    {MARKET_ALERT_META[a.alertType].label}
                  </span>
                  <span>{a.message}</span>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* Market History (2.4.0) - every successful scan's permanent
       *  snapshot, oldest scans falling off only past `listMarketSnapshots`'s
       *  own fixed limit, never overwritten (marko's own Section 6). */}
      <Modal open={historyOpen} onClose={() => setHistoryOpen(false)} title={`${view.marketplaceName} - Market History`} width="max-w-3xl">
        {historyLoading ? (
          <LoadingBlock />
        ) : !historyRows || historyRows.length === 0 ? (
          <EmptyState title="No snapshots yet" description="A snapshot is saved automatically after every successful scan." />
        ) : (
          <div className="max-h-96 overflow-y-auto rounded-lg border border-slate-100 dark:border-slate-800">
            <table className="w-full border-collapse">
              <thead className="sticky top-0 bg-slate-50 dark:bg-slate-800/60">
                <tr>
                  <th className="px-2 py-1 text-left text-[11px] font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">Checked</th>
                  <th className="px-2 py-1 text-left text-[11px] font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">Result</th>
                  <th className="px-2 py-1 text-right text-[11px] font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">Lowest</th>
                  <th className="px-2 py-1 text-right text-[11px] font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">Median</th>
                  <th className="px-2 py-1 text-right text-[11px] font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">Avg</th>
                  <th className="px-2 py-1 text-right text-[11px] font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">Highest</th>
                  <th className="px-2 py-1 text-right text-[11px] font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">Listings</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
                {historyRows.map((snap) => (
                  <tr key={snap.id}>
                    <td className="px-2 py-1 text-[11px] text-slate-500 dark:text-slate-400">{formatDateTime(snap.checkedAt)}</td>
                    <td className="px-2 py-1 text-[11px] text-slate-500 dark:text-slate-400">{snap.scanStatus}</td>
                    <td className="px-2 py-1 text-right text-xs tabular-nums text-slate-700 dark:text-slate-300">{formatMoney(snap.lowestPriceCents, snap.currency)}</td>
                    <td className="px-2 py-1 text-right text-xs tabular-nums text-slate-700 dark:text-slate-300">{formatMoney(snap.medianPriceCents, snap.currency)}</td>
                    <td className="px-2 py-1 text-right text-xs tabular-nums text-slate-700 dark:text-slate-300">{formatMoney(snap.averagePriceCents, snap.currency)}</td>
                    <td className="px-2 py-1 text-right text-xs tabular-nums text-slate-700 dark:text-slate-300">{formatMoney(snap.highestPriceCents, snap.currency)}</td>
                    <td className="px-2 py-1 text-right text-xs tabular-nums text-slate-700 dark:text-slate-300">{snap.listingCount}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
        <ModalFooter>
          <Button variant="secondary" onClick={() => setHistoryOpen(false)}>
            Close
          </Button>
        </ModalFooter>
      </Modal>

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
          <div className="mb-3 grid grid-cols-5 gap-2 text-sm">
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
            <div className="max-h-36 overflow-y-auto rounded-lg border border-slate-100 dark:border-slate-800">
              <table className="w-full border-collapse">
                <thead className="sticky top-0 bg-slate-50 dark:bg-slate-800/60">
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

function SavePriceCheckModal({
  eventId,
  view,
  defaultCurrency,
  prefill,
  onClose,
  onSaved,
}: {
  eventId: number;
  /** null = closed. */
  view: MarketplacePriceView | null;
  defaultCurrency: string;
  /** 2.1.9: set when this modal was opened via a Visible Scanner session's
   *  "Save to history" button - null for a normal "Check Prices" click. The
   *  4+1 fields below are seeded straight from these exact numbers and stay
   *  fully editable, same as any other open - marko reviews before Save
   *  either way. */
  prefill?: ScanPrefill | null;
  onClose: () => void;
  onSaved: () => void;
}) {
  const toast = useToast();
  const [lowest, setLowest] = useState("");
  const [median, setMedian] = useState("");
  const [average, setAverage] = useState("");
  const [highest, setHighest] = useState("");
  const [listingCount, setListingCount] = useState("");
  const [currency, setCurrency] = useState(defaultCurrency);
  const [customCurrency, setCustomCurrency] = useState(false);
  // 2.2.0: carried straight through to savePriceCheck, never hand-edited
  // here - see ScanPrefill.tierBreakdown's own doc comment. Not shown as
  // editable fields (that would be a lot of new form UI for data marko
  // never asked to hand-tweak per tier), just a small read-only summary
  // below so he can see it's included before saving.
  const [tierBreakdown, setTierBreakdown] = useState<TierBreakdownInput[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  // 2.0.82: "paste from the listings page" - see priceParse.ts. Purely a
  // faster way to fill in the same fields below; marko still has to open
  // the marketplace page and copy the text himself, and every field it
  // fills stays fully editable afterward.
  const [pasteText, setPasteText] = useState("");
  const [pasteInfo, setPasteInfo] = useState<string | null>(null);

  // Prefills either from a just-finished Visible Scanner session
  // (`prefill`) or the latest existing check for this marketplace, if any -
  // most real checks only move a little from last time, so this saves
  // retyping numbers that haven't changed; every field stays fully editable.
  useEffect(() => {
    if (!view) return;
    if (prefill) {
      setLowest(centsToDecimalString(prefill.lowestPriceCents));
      setMedian(prefill.medianPriceCents !== null ? centsToDecimalString(prefill.medianPriceCents) : "");
      setAverage(centsToDecimalString(prefill.averagePriceCents));
      setHighest(centsToDecimalString(prefill.highestPriceCents));
      setListingCount(String(prefill.listingCount));
      const cur = prefill.currency ?? defaultCurrency;
      setCurrency(cur);
      setCustomCurrency(!CURRENCIES.includes(cur));
      setTierBreakdown(prefill.tierBreakdown);
    } else {
      const latest = view.history[0] ?? null;
      setLowest(latest ? centsToDecimalString(latest.lowestPriceCents) : "");
      setMedian(latest && latest.medianPriceCents !== null ? centsToDecimalString(latest.medianPriceCents) : "");
      setAverage(latest ? centsToDecimalString(latest.averagePriceCents) : "");
      setHighest(latest ? centsToDecimalString(latest.highestPriceCents) : "");
      setListingCount(latest ? String(latest.listingCount) : "");
      const cur = latest?.currency ?? defaultCurrency;
      setCurrency(cur);
      setCustomCurrency(!CURRENCIES.includes(cur));
      // Same "start from what's already known" convention as the fields
      // above - re-typing a fresh check for the same marketplace usually
      // means the tiers themselves haven't changed shape even when the
      // prices have, so this saves marko from losing that structure. Fully
      // replaced (not merged) the moment a live scan prefill arrives instead.
      setTierBreakdown(latest?.tierBreakdown.map((t) => ({ tier: t.tier, lowestPriceCents: t.lowestPriceCents, medianPriceCents: t.medianPriceCents, listingCount: t.listingCount })) ?? []);
    }
    setError(null);
    setSaving(false);
    setPasteText("");
    setPasteInfo(null);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [view, defaultCurrency, prefill]);

  // Runs on every keystroke/paste in the textarea - re-extracting from the
  // full current text each time (not just the newly-pasted chunk) so
  // editing or re-pasting on top of earlier text keeps working sensibly.
  // Leaves the fields untouched (rather than clearing them) when nothing is
  // found, so a paste that didn't work never destroys numbers already
  // sitting in the form.
  const handlePasteTextChange = (text: string) => {
    setPasteText(text);
    if (!text.trim()) {
      setPasteInfo(null);
      return;
    }
    const { prices, currency: detected } = extractPricesFromText(text);
    if (prices.length === 0) {
      setPasteInfo("Couldn't find any prices in that text - enter the numbers manually below.");
      return;
    }
    const sorted = [...prices].sort((a, b) => a - b);
    const mid = Math.floor(sorted.length / 2);
    const medianVal = sorted.length % 2 === 1 ? sorted[mid] : (sorted[mid - 1] + sorted[mid]) / 2;
    const lowestVal = sorted[0];
    const highestVal = sorted[sorted.length - 1];
    const avgVal = prices.reduce((a, b) => a + b, 0) / prices.length;
    setLowest(lowestVal.toFixed(2));
    setMedian(medianVal.toFixed(2));
    setAverage(avgVal.toFixed(2));
    setHighest(highestVal.toFixed(2));
    setListingCount(String(prices.length));
    if (detected) {
      setCurrency(detected);
      setCustomCurrency(!CURRENCIES.includes(detected));
    }
    setPasteInfo(
      `Found ${prices.length} price${prices.length === 1 ? "" : "s"}${detected ? ` in ${detected}` : ""} - filled in below, double-check before saving.`,
    );
  };

  if (!view) return null;

  const submit = async () => {
    const lowestCents = decimalStringToCents(lowest);
    const averageCents = decimalStringToCents(average);
    const highestCents = decimalStringToCents(highest);
    const count = parseInt(listingCount, 10);
    if (lowestCents === null || averageCents === null || highestCents === null) {
      setError("Enter valid prices (up to 2 decimal places).");
      return;
    }
    // Blank median means "not provided" (null, never a fabricated 0) -
    // decimalStringToCents("") itself returns 0, which would silently save
    // a real "free" median, so the blank case is checked separately here
    // before ever calling it.
    const medianCents = median.trim() === "" ? null : decimalStringToCents(median);
    if (median.trim() !== "" && medianCents === null) {
      setError("Enter a valid median price (up to 2 decimal places), or leave it blank.");
      return;
    }
    if (!Number.isFinite(count) || count < 0) {
      setError("Enter a valid number of listings (0 or more).");
      return;
    }
    if (lowestCents > averageCents || averageCents > highestCents) {
      setError("Lowest price must be at or below average, and average must be at or below highest.");
      return;
    }
    if (medianCents !== null && (medianCents < lowestCents || medianCents > highestCents)) {
      setError("Median price must be between the lowest and highest price.");
      return;
    }
    if (!currency.trim()) {
      setError("Currency is required.");
      return;
    }
    setError(null);
    setSaving(true);
    try {
      await api.savePriceCheck({
        eventId,
        marketplaceId: view.marketplaceId,
        lowestPriceCents: lowestCents,
        medianPriceCents: medianCents,
        averagePriceCents: averageCents,
        highestPriceCents: highestCents,
        listingCount: count,
        currency: currency.trim().toUpperCase(),
        tierBreakdown,
      });
      toast.success("Price check saved");
      onSaved();
    } catch (e) {
      setError(errMsg(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal open onClose={onClose} title={`Check Prices - ${view.marketplaceName}`}>
      <div className="flex flex-col gap-4">
        {view.link?.url && (
          <p className="select-all break-all rounded-lg bg-slate-50 p-2 font-mono text-xs text-slate-500 dark:bg-slate-800/60 dark:text-slate-400">
            {view.link.url}
          </p>
        )}
        {prefill && (
          <p className="flex items-center gap-1.5 text-xs text-sky-700 dark:text-sky-400">
            Prefilled from your Visible Scanner scan ({prefill.listingCount} listing{prefill.listingCount === 1 ? "" : "s"}) - review before saving.
          </p>
        )}
        {tierBreakdown.length > 0 && (
          <p className="-mt-2 text-xs text-slate-500 dark:text-slate-400">
            Includes a breakdown for {tierBreakdown.length} tier{tierBreakdown.length === 1 ? "" : "s"} ({tierBreakdown.map((t) => t.tier).join(", ")}).
          </p>
        )}
        <Field label="Paste from the listings page" hint="Select the prices on that page, copy, and paste here - the fields below fill in automatically.">
          <Textarea
            rows={3}
            value={pasteText}
            onChange={(e) => handlePasteTextChange(e.target.value)}
            placeholder="e.g. $145  $150  $138  $162 ..."
            className="font-mono text-xs"
          />
        </Field>
        {pasteInfo && <p className="-mt-2 text-xs text-slate-500 dark:text-slate-400">{pasteInfo}</p>}
        <div className="grid grid-cols-2 gap-3">
          <Field label="Lowest price" required>
            <Input inputMode="decimal" value={lowest} onChange={(e) => setLowest(e.target.value)} placeholder="0.00" />
          </Field>
          <Field label="Median price" hint="Optional - leave blank if unknown.">
            <Input inputMode="decimal" value={median} onChange={(e) => setMedian(e.target.value)} placeholder="0.00" />
          </Field>
          <Field label="Average price" required>
            <Input inputMode="decimal" value={average} onChange={(e) => setAverage(e.target.value)} placeholder="0.00" />
          </Field>
          <Field label="Highest price" required>
            <Input inputMode="decimal" value={highest} onChange={(e) => setHighest(e.target.value)} placeholder="0.00" />
          </Field>
          <Field label="Number of listings" required>
            <Input type="number" min={0} step={1} value={listingCount} onChange={(e) => setListingCount(e.target.value)} />
          </Field>
        </div>
        <div>
          <div className="flex items-center justify-between">
            <span className="label mb-1">Currency</span>
            <button
              type="button"
              className="mb-1 text-xs font-medium text-brand-600 hover:underline dark:text-brand-400"
              onClick={() => setCustomCurrency((c) => !c)}
            >
              {customCurrency ? "Choose from list" : "Other..."}
            </button>
          </div>
          {customCurrency ? (
            <Input autoFocus placeholder="e.g. AED" value={currency} onChange={(e) => setCurrency(e.target.value.toUpperCase())} />
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
        {error && <p className="text-sm text-red-600 dark:text-red-400">{error}</p>}
      </div>
      <ModalFooter>
        <Button variant="secondary" onClick={onClose} disabled={saving}>
          Cancel
        </Button>
        <Button variant="primary" onClick={submit} disabled={saving}>
          {saving ? <Spinner className="h-4 w-4" /> : "Save check"}
        </Button>
      </ModalFooter>
    </Modal>
  );
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

export default function PriceChecker() {
  const location = useLocation();
  const toast = useToast();
  const [events, setEvents] = useState<EventWithStats[]>([]);
  const [eventId, setEventId] = useState<number | "">("");
  const [summary, setSummary] = useState<PriceCheckerSummary | null>(null);
  const [loading, setLoading] = useState(false);
  const [checkModalFor, setCheckModalFor] = useState<MarketplacePriceView | null>(null);
  // 2.1.9: set alongside checkModalFor when it was opened from a scan's
  // "Save to history" button - null for a normal "Check Prices" click.
  // Cleared alongside checkModalFor so a stale prefill never leaks into the
  // next open.
  const [checkModalPrefill, setCheckModalPrefill] = useState<ScanPrefill | null>(null);

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

  // Live Market Monitor (2.4.0) - one summary per event covering every
  // marketplace's URL/status/last successful scan/latest snapshot/recent
  // alerts, refreshed on event switch below and again after every scan
  // result (via loadMonitorRef, read from inside the long-lived
  // SCAN_RESULT_EVENT listener further down - same "ref mirrors a callback
  // for a long-lived subscription" idea scannerSessionsRef already uses) -
  // see this file's own module doc comment, and commands/price_checker_
  // monitor.rs's (Rust), for the full design. Entirely separate from
  // `summary`/`load` above - this never touches PriceCheckerSummary or
  // marko's own manually-saved price_checks history, only reads it.
  const [monitor, setMonitor] = useState<MarketMonitorSummary | null>(null);
  const loadMonitor = useCallback(() => {
    if (eventId === "") {
      setMonitor(null);
      return;
    }
    api
      .getMarketMonitorSummary(eventId)
      .then(setMonitor)
      .catch((e) => toast.error(errMsg(e)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [eventId]);
  const loadMonitorRef = useRef(loadMonitor);
  useEffect(() => {
    loadMonitorRef.current = loadMonitor;
  }, [loadMonitor]);
  useEffect(() => {
    loadMonitor();
  }, [loadMonitor]);

  useEffect(() => {
    // 2.2.2: only events still ahead of you are worth a market check - once
    // marko marks one "completed" (or it's "cancelled"), checking live
    // prices for it no longer means anything, so it should just quietly
    // stop showing up here, no manual untracking needed. Reuses the exact
    // same `status` field/value Events.tsx's own Upcoming/Completed tabs
    // already use (2.0.59) rather than inventing a date-based rule.
    api
      .listEvents()
      .then((all) => setEvents(all.filter((ev) => ev.status === "upcoming")))
      .catch((e) => toast.error(errMsg(e)));
    // Mirrors Orders.tsx's own presetEventId pattern - EventDetail's "Check
    // prices" button navigates here with the event already chosen, so marko
    // never has to find it again in the dropdown.
    const preset = location.state as { presetEventId?: number; presetMarketplaceId?: number } | null;
    if (preset?.presetEventId) setEventId(preset.presetEventId);
    // 2.4.0: Attention Center's "LIVE MARKET ALERTS" rows carry a
    // marketplaceId too (marko's own Section 11 - "clicking opens Price
    // Checker at that event/marketplace") - scrolled to and briefly
    // highlighted once this event's cards render, see the effect below.
    if (preset?.presetMarketplaceId) setHighlightMarketplaceId(preset.presetMarketplaceId);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Scrolls the targeted marketplace card into view and highlights it
  // briefly, then clears itself - a plain, self-contained effect, not a new
  // shared pattern (see ticketsAnchorRef's own scrollIntoView in
  // EventDetail.tsx for the closest existing precedent). Waits for `summary`
  // itself (not just `events`) since the card doesn't exist in the DOM until
  // then.
  const [highlightMarketplaceId, setHighlightMarketplaceId] = useState<number | null>(null);
  useEffect(() => {
    if (!summary || highlightMarketplaceId == null) return;
    const el = document.getElementById(`marketplace-card-${highlightMarketplaceId}`);
    el?.scrollIntoView({ behavior: "smooth", block: "center" });
    const t = window.setTimeout(() => setHighlightMarketplaceId(null), 2500);
    return () => window.clearTimeout(t);
  }, [summary, highlightMarketplaceId]);

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
            scanning: false,
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
          },
        };
      });
      // 2.4.0: this exact scan already recorded its own snapshot/alerts on
      // the backend (see commands/price_checker_scanner.rs's record_scan_
      // attempt_impl hook, Rust) by the time this event fires - refresh the
      // Live Market Monitor summary so the panel/History reflect it without
      // marko having to switch events and back.
      loadMonitorRef.current();
    }).then((fn) => {
      if (disposed) fn();
      else unlistenResult = fn;
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
      api.scanVisiblePrices(session.requestId).catch((e) => {
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

  // "Scan All" (2.4.0) - fires the exact same scanVisible call above, once
  // per marketplace on THIS event that currently has an open scanner window,
  // skipping any that's still opening or already mid-scan (same overlap
  // guard Auto Monitor's own interval uses). A marketplace with no window
  // open is skipped, not opened on its own - marko's own Section 4 boundary
  // ("no automation beyond reading whatever's already open") applies here
  // exactly as it does to a single card's own button.
  const scanAll = useCallback(() => {
    if (summary === null) return;
    const prefix = `${summary.eventId}:`;
    const targets = Object.entries(scannerSessionsRef.current)
      .filter(([key, s]) => key.startsWith(prefix) && !s.opening && !s.scanning)
      .map(([key]) => Number(key.slice(prefix.length)));
    if (targets.length === 0) {
      toast.error("Open a scanner window on at least one marketplace first (see Open & Scan below).");
      return;
    }
    targets.forEach((marketplaceId) => scanVisible(summary.eventId, marketplaceId));
  }, [summary, scanVisible, toast]);

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

  // "Save to history" on a live scan - opens the same review-then-save
  // modal any manual "Check Prices" uses, prefilled with this session's
  // current running totals. Never saves directly - marko still reviews and
  // clicks Save himself, same as every other path into price_checks.
  // 2.2.0: also carries through this session's own tier breakdown for
  // `session.currency`, if the card's Market Analysis had finished loading
  // by the time this was clicked - see ScanPrefill.tierBreakdown's own doc
  // comment for why (marko's spec, "## PRICE HISTORY").
  const saveScanToHistory = useCallback((view: MarketplacePriceView, session: ScannerCardState, analysis: MarketAnalysisResult | null) => {
    const matchingCurrency = analysis?.byCurrency.find((c) => c.currency === session.currency);
    setCheckModalPrefill({
      lowestPriceCents: session.lowestPriceCents ?? 0,
      medianPriceCents: session.medianPriceCents,
      averagePriceCents: session.averagePriceCents ?? 0,
      highestPriceCents: session.highestPriceCents ?? 0,
      listingCount: session.listings.length,
      currency: session.currency,
      tierBreakdown:
        matchingCurrency?.tiers.map((t) => ({
          tier: t.tier,
          lowestPriceCents: t.stats.lowestPriceCents,
          medianPriceCents: t.stats.medianPriceCents,
          listingCount: t.stats.listingCount,
        })) ?? [],
    });
    setCheckModalFor(view);
  }, []);

  return (
    <div>
      <PageHeader title="Price Checker" subtitle="Compare your unsold inventory against Vivid Seats, Ticombo and Viagogo." />

      <Card className="mb-6 max-w-md p-4">
        <Field label="Event">
          <Select value={eventId} onChange={(e) => setEventId(e.target.value ? Number(e.target.value) : "")}>
            <option value="">Select an event...</option>
            {events.map((ev) => (
              <option key={ev.id} value={ev.id}>
                {ev.name} {ev.eventDate ? `(${ev.eventDate})` : ""}
              </option>
            ))}
          </Select>
        </Field>
      </Card>

      {eventId === "" ? (
        <EmptyState
          icon={<IconTag className="h-8 w-8" />}
          title="Pick an event to check its prices"
          description="Save each marketplace's listings link, then record what you see there to compare it against your own inventory."
        />
      ) : loading || !summary ? (
        <LoadingBlock />
      ) : (
        <>
          <div className="mb-6 grid grid-cols-2 gap-3 sm:grid-cols-4">
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
            <p className="mb-3 text-xs font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">Market vs. mine</p>
            <div className="grid grid-cols-2 gap-3 sm:grid-cols-5">
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

          <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
            <p className="text-xs font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">Marketplaces</p>
            {/* 2.4.0: scans every marketplace with a window already open on
             *  this event - see scanAll's own doc comment above. */}
            <Button variant="secondary" onClick={scanAll}>
              <IconRefresh className="h-4 w-4" /> Scan All
            </Button>
          </div>

          <div className="grid grid-cols-1 gap-4 lg:grid-cols-3">
            {summary.marketplaces.map((view) => (
              <div
                key={view.marketplaceId}
                id={`marketplace-card-${view.marketplaceId}`}
                className={`rounded-xl transition-shadow duration-300 ${
                  highlightMarketplaceId === view.marketplaceId ? "ring-2 ring-brand-400 dark:ring-brand-500" : ""
                }`}
              >
                <MarketplaceCard
                  eventId={summary.eventId}
                  view={view}
                  monitor={monitor?.marketplaces.find((m) => m.marketplaceId === view.marketplaceId)}
                  onLinkSaved={load}
                  onCheckPrices={() => {
                    setCheckModalPrefill(null);
                    setCheckModalFor(view);
                  }}
                  session={scannerSessions[sessionKey(summary.eventId, view.marketplaceId)]}
                  onOpenScanner={openScanner}
                  onScanVisible={scanVisible}
                  onStopScan={stopScan}
                  onCloseScanner={closeScanner}
                  onSaveScanToHistory={saveScanToHistory}
                />
              </div>
            ))}
          </div>

          <SavePriceCheckModal
            eventId={summary.eventId}
            view={checkModalFor}
            defaultCurrency={summary.myCurrency ?? "EUR"}
            prefill={checkModalPrefill}
            onClose={() => {
              setCheckModalFor(null);
              setCheckModalPrefill(null);
            }}
            onSaved={() => {
              setCheckModalFor(null);
              setCheckModalPrefill(null);
              load();
            }}
          />
        </>
      )}
    </div>
  );
}

// Price Checker (2.0.81) - marko's own new section: pick an event, save each
// marketplace's listings-page link, then manually record what the market is
// asking there (lowest/average/highest price, listing count) and compare it
// against his own unsold inventory. Manual entry only, no live API/scraping -
// see src-tauri/src/commands/price_checker.rs's module doc comment for why
// (researched, not assumed: none of StubHub/Vivid Seats/Ticombo offers an
// accessible public API to an individual seller, and StubHub actively blocks
// casual scraping - marko's own instruction was to fall back to manual entry
// rather than bypass any site's protection).
import { useCallback, useEffect, useRef, useState } from "react";
import { useLocation } from "react-router-dom";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api, errMsg } from "../lib/api";
import type {
  AutoCheckPhase,
  AutoCheckProgressEvent,
  EventWithStats,
  MarketplacePriceView,
  PriceCheck,
  PriceCheckerSummary,
  AutoCheckResult,
} from "../lib/types";
import { centsToDecimalString, decimalStringToCents, formatDateTime, formatMoney, formatMoneyOrMixed, formatPercent } from "../lib/format";
import {
  Button,
  Card,
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
import { IconAlertTriangle, IconLink, IconTag, IconTrendingDown, IconTrendingUp, IconX } from "../components/icons";
import { useToast } from "../lib/toast";
import { CURRENCIES } from "./Orders";
import { extractPricesFromText } from "../lib/priceParse";

// ---------------------------------------------------------------------------
// Auto-check lifecycle (2.1.2 freeze fix) - see
// commands/price_checker_auto.rs's module doc comment (Rust) for the backend
// half of this. `AUTO_CHECK_PROGRESS_EVENT` must match that module's own
// `PROGRESS_EVENT` constant exactly - there's no shared source of truth
// across the Rust/TS boundary for a plain event-name string, so if that
// constant is ever renamed there, this must be renamed here too.
// ---------------------------------------------------------------------------

const AUTO_CHECK_PROGRESS_EVENT = "price-checker-auto-check-progress";

const AUTO_CHECK_PHASE_LABEL: Record<AutoCheckPhase, string> = {
  starting: "Starting...",
  loading: "Loading page...",
  analyzing: "Analyzing...",
  cleaning_up: "Cleaning up...",
};

// 2.1.3 (production hardening): the backend's OWN hard ceiling is
// OVERALL_TIMEOUT + OUTER_GRACE = 17s (see run_with_outer_deadline's doc
// comment, Rust) - this is that plus a 3s margin for ordinary IPC/
// serialization overhead, not a number the backend itself is trying to
// hit. See startAutoCheck's own comment (marko's spec section 7 - "LEVEL 2
// outer watchdog") for what this actually guarantees and what it honestly
// can't (it can't abort the backend call itself - invoke() has no abort).
const FRONTEND_WATCHDOG_MS = 20000;

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

// ---------------------------------------------------------------------------
// One marketplace's card: link + latest check + full history + "Check Prices"
// ---------------------------------------------------------------------------

function MarketplaceCard({
  eventId,
  view,
  onLinkSaved,
  onCheckPrices,
  autoCheckPhase,
  autoCheckRunning,
  canStartAutoCheck,
  onStartAutoCheck,
  onCancelAutoCheck,
}: {
  eventId: number;
  view: MarketplacePriceView;
  onLinkSaved: () => void;
  onCheckPrices: () => void;
  /** 2.1.2: this card's own live phase - non-null exactly while THIS
   *  marketplace is the one being auto-checked right now (see
   *  PriceChecker's own progress-event listener). null the rest of the
   *  time, including while a DIFFERENT card's attempt is running. */
  autoCheckPhase: AutoCheckPhase | null;
  /** True while ANY card (this one or another) has an auto-check in flight -
   *  disables "Check Prices" here so it can never repurpose the one shared
   *  SavePriceCheckModal out from under a still-running attempt that will
   *  itself want that same modal in a moment. */
  autoCheckRunning: boolean;
  /** False while something is running OR the shared modal is already open
   *  for any marketplace - only one auto-check can ever be in flight at a
   *  time (the backend only tracks one), and starting a second one while a
   *  modal is already open would hijack it away from whatever marko is
   *  looking at. See PriceChecker's own comment on `autoCheck` state for the
   *  full reasoning. */
  canStartAutoCheck: boolean;
  onStartAutoCheck: (targetUrl: string) => void;
  onCancelAutoCheck: () => void;
}) {
  const toast = useToast();
  const [url, setUrl] = useState(view.link?.url ?? "");
  const [savingLink, setSavingLink] = useState(false);

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

  // 2.1.1: tries the saved link first, falling back to whatever is
  // currently typed in the field (not yet saved) - marko shouldn't have to
  // click Save before trying this. Uses the CURRENT input, matching what
  // "Open page" would open too.
  const autoCheckTarget = (view.link?.url || url).trim();
  const isThisCardChecking = autoCheckPhase !== null;

  const latest = view.history[0] ?? null;
  const trend = trendFromHistory(view.history);
  const older = view.history.slice(1);

  return (
    <Card className="flex flex-col p-4">
      <div className="mb-3 flex items-center justify-between gap-2">
        <h3 className="text-sm font-semibold text-slate-900 dark:text-slate-100">{view.marketplaceName}</h3>
        <div className="flex items-center gap-2">
          {isThisCardChecking ? (
            <>
              <Button variant="secondary" disabled className="cursor-default disabled:opacity-100">
                <Spinner className="h-4 w-4" /> {AUTO_CHECK_PHASE_LABEL[autoCheckPhase]}
              </Button>
              <Button variant="secondary" onClick={onCancelAutoCheck} title="Stop this auto-check and use manual/paste entry instead.">
                <IconX className="h-4 w-4" /> Cancel
              </Button>
            </>
          ) : (
            <Button
              variant="secondary"
              onClick={() => {
                if (!autoCheckTarget) {
                  toast.error("Enter this marketplace's listings page URL above first.");
                  return;
                }
                onStartAutoCheck(autoCheckTarget);
              }}
              disabled={!canStartAutoCheck || !autoCheckTarget}
              title={
                !canStartAutoCheck
                  ? "Finish or close what's currently open first - only one auto-check can run at a time."
                  : "Try reading this page's prices automatically in the app's own browser view - falls back to pasting/typing if it can't."
              }
            >
              <IconTag className="h-4 w-4" /> Auto-check
            </Button>
          )}
          <Button variant="secondary" onClick={onCheckPrices} disabled={autoCheckRunning} title={autoCheckRunning ? "An auto-check is running right now - wait for it to finish or cancel it first." : undefined}>
            <IconTag className="h-4 w-4" /> Check Prices
          </Button>
        </div>
      </div>

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

      {!latest ? (
        <p className="text-xs text-slate-400 dark:text-slate-500">No price checks recorded yet.</p>
      ) : (
        <>
          <div className="mb-2 flex flex-wrap items-center justify-between gap-1">
            <p className="text-xs text-slate-400 dark:text-slate-500">Latest check &middot; {formatDateTime(latest.checkedAt)}</p>
            {trend && <TrendNote trend={trend} currency={latest.currency} />}
          </div>
          <div className="mb-3 grid grid-cols-4 gap-2 text-sm">
            <div>
              <p className="text-[11px] uppercase tracking-wide text-slate-400 dark:text-slate-500">Lowest</p>
              <p className="font-medium tabular-nums text-slate-900 dark:text-slate-100">{formatMoney(latest.lowestPriceCents, latest.currency)}</p>
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
  autoCheckResult,
  autoCheckTargetUrl,
  onRetryAutoCheck,
  onClose,
  onSaved,
}: {
  eventId: number;
  /** null = closed. */
  view: MarketplacePriceView | null;
  defaultCurrency: string;
  /** 2.1.1: set when this modal was opened right after an auto-check
   *  attempt (see MarketplaceCard's "Auto-check" button) - null for a
   *  normal "Check Prices" click. When its status is "ok", its prices are
   *  run through the exact same handlePasteTextChange pipeline a real
   *  paste already uses below, so the result is fully editable and marko
   *  reviews it before Save exactly like any paste. Any other status shows
   *  its own message plus Retry/Open page (see the banner below); the form
   *  itself is otherwise identical to today. */
  autoCheckResult?: AutoCheckResult | null;
  /** 2.1.2: the exact URL that auto-check attempt targeted - paired with
   *  autoCheckResult (both null together). Powers the banner's "Open page"
   *  link, so marko can look at the listings page himself after a
   *  timeout/blocked/unable_to_read/error/cancelled result. */
  autoCheckTargetUrl?: string | null;
  /** 2.1.2: re-runs the SAME auto-check attempt (closes this modal and
   *  starts a fresh one on the marketplace card, same target URL) - null
   *  whenever there's nothing to retry (a normal "Check Prices" open). See
   *  the banner below. */
  onRetryAutoCheck?: (() => void) | null;
  onClose: () => void;
  onSaved: () => void;
}) {
  const toast = useToast();
  const [lowest, setLowest] = useState("");
  const [average, setAverage] = useState("");
  const [highest, setHighest] = useState("");
  const [listingCount, setListingCount] = useState("");
  const [currency, setCurrency] = useState(defaultCurrency);
  const [customCurrency, setCustomCurrency] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  // 2.0.82: "paste from the listings page" - see priceParse.ts. Purely a
  // faster way to fill in the same 4 fields below; marko still has to open
  // the marketplace page and copy the text himself, and every field it
  // fills stays fully editable afterward.
  const [pasteText, setPasteText] = useState("");
  const [pasteInfo, setPasteInfo] = useState<string | null>(null);

  // Prefills from the latest existing check for this marketplace, if any -
  // most real checks only move a little from last time, so this saves
  // retyping numbers that haven't changed; every field stays fully editable.
  useEffect(() => {
    if (!view) return;
    const latest = view.history[0] ?? null;
    setLowest(latest ? centsToDecimalString(latest.lowestPriceCents) : "");
    setAverage(latest ? centsToDecimalString(latest.averagePriceCents) : "");
    setHighest(latest ? centsToDecimalString(latest.highestPriceCents) : "");
    setListingCount(latest ? String(latest.listingCount) : "");
    const cur = latest?.currency ?? defaultCurrency;
    setCurrency(cur);
    setCustomCurrency(!CURRENCIES.includes(cur));
    setError(null);
    setSaving(false);
    setPasteText("");
    setPasteInfo(null);

    // 2.1.1: an auto-check attempt just ran for this marketplace - seed the
    // paste box with what it found (as plain "$31 $39 ..." text, so it's
    // visible and fully re-editable) and run it through the SAME
    // handlePasteTextChange pipeline a real paste uses, rather than
    // computing lowest/average/highest separately here. A non-"ok" status
    // (unable_to_read/blocked/error/cancelled/timeout, 2.1.2) never fills
    // anything - its own message shows in the dedicated banner below
    // instead (not routed through pasteInfo, which is reserved for actual
    // paste-box feedback) - the form is otherwise identical to a normal
    // "Check Prices" open.
    if (autoCheckResult && autoCheckResult.status === "ok" && autoCheckResult.prices.length > 0) {
      const symbol = autoCheckResult.currency === "EUR" ? "€" : autoCheckResult.currency === "GBP" ? "£" : "$";
      handlePasteTextChange(autoCheckResult.prices.map((p) => `${symbol}${p}`).join(" "));
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [view, defaultCurrency, autoCheckResult]);

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
    const lowestVal = Math.min(...prices);
    const highestVal = Math.max(...prices);
    const avgVal = prices.reduce((a, b) => a + b, 0) / prices.length;
    setLowest(lowestVal.toFixed(2));
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
    if (!Number.isFinite(count) || count < 0) {
      setError("Enter a valid number of listings (0 or more).");
      return;
    }
    if (lowestCents > averageCents || averageCents > highestCents) {
      setError("Lowest price must be at or below average, and average must be at or below highest.");
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
        averagePriceCents: averageCents,
        highestPriceCents: highestCents,
        listingCount: count,
        currency: currency.trim().toUpperCase(),
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
        {autoCheckResult && autoCheckResult.status !== "ok" && (
          <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-amber-700 dark:text-amber-400">
            <span className="inline-flex items-center gap-1.5">
              <IconAlertTriangle className="h-3.5 w-3.5 shrink-0" />
              {autoCheckResult.message ?? "Auto-check couldn't finish - use the paste/manual entry below instead."}
            </span>
            {onRetryAutoCheck && (
              <button type="button" onClick={onRetryAutoCheck} className="font-medium underline hover:no-underline">
                Try Auto-check again
              </button>
            )}
            {autoCheckTargetUrl && (
              <button
                type="button"
                onClick={() => openUrl(autoCheckTargetUrl).catch((e) => toast.error(errMsg(e)))}
                className="font-medium underline hover:no-underline"
              >
                Open the page myself
              </button>
            )}
          </div>
        )}
        <Field label="Paste from the listings page" hint="Select the prices on that page, copy, and paste here - the 4 fields below fill in automatically.">
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
  // 2.1.1/2.1.2: the auto-check attempt that opened checkModalFor (if any) -
  // null for a normal "Check Prices" click. `targetUrl` is kept alongside
  // `result` so the modal's "Try Auto-check again"/"Open the page myself"
  // affordances know exactly what to re-run/open. Cleared alongside
  // checkModalFor so a stale result never leaks into the next open.
  const [autoCheckModalData, setAutoCheckModalData] = useState<{ result: AutoCheckResult; targetUrl: string } | null>(null);
  // 2.1.2: which marketplace (if any) currently has a LIVE auto-check
  // attempt, and its real Starting/Loading/Analyzing phase - lifted up to
  // this page (rather than kept local to MarketplaceCard, as 2.1.1 had it)
  // for two reasons: the backend only ever tracks ONE in-flight attempt at a
  // time (AppState::price_checker_auto_cancel_flag is a single slot - see
  // commands/price_checker_auto.rs), and SavePriceCheckModal above is a
  // SINGLE shared instance - two cards each racing their own local state
  // could send a stale result into a modal marko has since repurposed for a
  // different marketplace (or is still typing a manual entry into). null
  // means nothing is running anywhere on this page.
  // 2.1.3: `requestId` added - whatever startAutoCheck minted for THIS
  // attempt (see requestIdRef below), threaded through so the progress-
  // event listener can tell a stale attempt's late event apart from the
  // one actually being tracked right now.
  const [autoCheck, setAutoCheck] = useState<{ marketplaceId: number; phase: AutoCheckPhase; targetUrl: string; requestId: number } | null>(
    null,
  );
  // 2.1.3: mints a fresh id for every startAutoCheck call - marko's own
  // explicit "Každý request musí mať vlastné request ID. Starý request
  // nesmie meniť UI nového requestu." See startAutoCheck's own comment for
  // exactly which race this closes and why it's needed now specifically
  // (the backend's new single-flight guard can make an OLDER request's
  // result arrive LATER than a newer, immediately-rejected one's).
  const requestIdRef = useRef(0);
  // 2.1.3: mirrors `autoCheck` for SYNCHRONOUS reads inside startAutoCheck
  // and its async .then/.catch/.finally callbacks - React state itself
  // can't be read synchronously outside a render or a functional updater,
  // and these call sites need to check "what's the CURRENT slot holder"
  // (not "what did I last set it to") at moments React state alone can't
  // answer. Kept in sync two ways: this effect (the general case, fires
  // after every commit) AND a direct same-tick assignment inside
  // startAutoCheck itself (see there) - the effect alone would still leave
  // a stale value for the brief window between calling setAutoCheck and
  // this effect actually re-running, which is exactly the window
  // startAutoCheck's own synchronous re-entrancy guard needs to be
  // reliable in.
  const autoCheckRef = useRef<typeof autoCheck>(null);
  useEffect(() => {
    autoCheckRef.current = autoCheck;
  }, [autoCheck]);

  useEffect(() => {
    api.listEvents().then(setEvents).catch((e) => toast.error(errMsg(e)));
    // Mirrors Orders.tsx's own presetEventId pattern - EventDetail's "Check
    // prices" button navigates here with the event already chosen, so marko
    // never has to find it again in the dropdown.
    const preset = (location.state as { presetEventId?: number } | null)?.presetEventId;
    if (preset) setEventId(preset);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // 2.1.2: live Starting/Loading/Analyzing progress pushed from the backend
  // (see commands/price_checker_auto.rs's `emit_phase`) - a real signal from
  // the actual reader window, not a client-side timer standing in for one
  // (marko's own explicit requirement after the freeze/hang report). One
  // subscription for the whole page, matching the single in-flight attempt
  // tracked in `autoCheck` above. The `prev ? ... : prev` guard ignores a
  // stray/late event whenever nothing is currently tracked as running (the
  // attempt already finished, or this page just mounted).
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;
    listen<AutoCheckProgressEvent>(AUTO_CHECK_PROGRESS_EVENT, (event) => {
      // 2.1.3: only apply this event if it's for the attempt CURRENTLY
      // being tracked - ignores a stale attempt's late-arriving phase
      // event once a newer attempt has started (or nothing is tracked as
      // running at all, the pre-existing `prev ?` guard). Needed now
      // specifically because of the backend's single-flight guard - see
      // startAutoCheck's own comment on requestIdRef for the exact race
      // this closes.
      setAutoCheck((prev) => (prev && prev.requestId === event.payload.requestId ? { ...prev, phase: event.payload.phase } : prev));
    }).then((fn) => {
      if (disposed) {
        fn(); // this effect was already cleaned up by the time listen() resolved - unlisten immediately rather than leak it
      } else {
        unlisten = fn;
      }
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

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

  // 2.1.2: starts (or retries) one auto-check attempt for `view`/`targetUrl`
  // - shared by MarketplaceCard's "Auto-check" button and the modal's own
  // "Try Auto-check again". Always settles `autoCheck` back to null in
  // `finally`, whatever the outcome (success, every non-ok status, or a
  // genuine invoke() rejection) - same unconditional-cleanup convention
  // start_google_sign_in's own frontend caller already follows.
  //
  // 2.1.3 (production hardening): rewritten around TWO independent guards,
  // not one - an earlier version of this used only a monotonically-
  // increasing counter ref compared at .then/.catch/.finally time, which
  // turned out to have its own bug: a SECOND, overlapping call (a fast
  // double-click slipping past the button's `disabled` attribute before
  // React re-renders it) still optimistically claimed the `autoCheck` UI
  // slot for itself immediately, and since the backend's single-flight
  // guard makes a rejected ("busy") second attempt resolve almost
  // instantly - often before the FIRST, genuinely-running attempt does -
  // that counter alone could end up treating the fast, spurious second
  // attempt as "current" and the slow, real first attempt as "stale",
  // silently discarding the real attempt's actual result once it finally
  // arrived. Fixed with:
  //   1. A SYNCHRONOUS re-entrancy check right below, against `autoCheckRef`
  //      (not React state, which can't be read synchronously) - refuses to
  //      even START a second attempt while one is already active. Because
  //      JS event handlers run one at a time, never overlapping, this is
  //      airtight against exactly the click-race above: whichever call
  //      runs first sets `autoCheckRef.current` before the browser can even
  //      dispatch the second click's own handler.
  //   2. `isStillCurrent()`, checked inside every async callback below -
  //      still needed even with (1), e.g. so a late result that arrives
  //      after the watchdog already gave up (or after Cancel's own
  //      confirmation already cleared the slot) is correctly ignored rather
  //      than re-applied.
  // Matches marko's own literal "Starý request nesmie meniť UI nového
  // requestu" for real, in every ordering, not just the common case.
  const startAutoCheck = useCallback((view: MarketplacePriceView, targetUrl: string) => {
    if (autoCheckRef.current !== null) {
      // See this function's own comment above - closes the exact race a
      // `disabled` DOM attribute alone can't, since React hasn't
      // necessarily re-rendered it yet at the moment a second click lands.
      toast.error("Another auto-check is already running - wait for it to finish or cancel it first.");
      return;
    }

    const myRequestId = ++requestIdRef.current;
    const optimisticState = { marketplaceId: view.marketplaceId, phase: "starting" as AutoCheckPhase, targetUrl, requestId: myRequestId };
    setAutoCheck(optimisticState);
    autoCheckRef.current = optimisticState; // same-tick, not waiting for the mirroring effect - see autoCheckRef's own comment

    const isStillCurrent = () => autoCheckRef.current?.requestId === myRequestId;

    // Level-2 frontend watchdog (2.1.3, marko's spec section 7) - a pure UI
    // safety net, independent of whatever the backend is actually doing.
    // Honest limitation: invoke() has no abort, so this can't stop the
    // backend call itself if it's ever somehow still running past this
    // point - but it guarantees the BUTTON always comes back to Idle,
    // rather than staying on Starting/Loading/Analyzing/Cleaning-up forever
    // in some unforeseen case the backend's OWN two timeout layers
    // (OVERALL_TIMEOUT + OUTER_GRACE, price_checker_auto.rs) somehow don't
    // cover. Cleared in `finally` below on every normal completion, long
    // before it would ever fire in practice.
    const watchdog = window.setTimeout(() => {
      if (isStillCurrent()) {
        setAutoCheck(null);
        autoCheckRef.current = null;
        toast.error("Auto-check is taking unusually long - resetting. It may still finish in the background.");
      }
    }, FRONTEND_WATCHDOG_MS);

    api
      .autoCheckPrice(targetUrl, myRequestId)
      .then((result) => {
        if (!isStillCurrent()) return; // superseded (watchdog already gave up, or Cancel already confirmed) - this result is stale, ignore it entirely
        if (result.status === "busy") {
          // Should be unreachable via normal UI now (both canStartAutoCheck
          // and startAutoCheck's own synchronous autoCheckRef guard above
          // already prevent starting a second attempt) - the backend's own
          // single-flight guard is the real, authoritative defense; this is
          // just a safe, honest fallback if it's ever hit anyway (e.g. a
          // future caller of autoCheckPrice outside this exact code path).
          // Nothing to review for an attempt that never actually started,
          // so skip the modal.
          toast.error(result.message ?? "Another auto-check is already running.");
          return;
        }
        if (result.status === "ok") {
          toast.success(`Auto-check found ${result.prices.length} price${result.prices.length === 1 ? "" : "s"} - review before saving.`);
        } else if (result.status !== "cancelled") {
          // "cancelled" was marko's own choice a moment ago - an error toast
          // for it would read as scolding him for clicking his own Cancel
          // button. Every other non-ok status still gets one, same as 2.1.1.
          toast.error(result.message ?? "Couldn't read prices automatically - enter them below.");
        }
        setAutoCheckModalData({ result, targetUrl });
        setCheckModalFor(view);
      })
      .catch((e) => {
        if (!isStillCurrent()) return;
        toast.error(errMsg(e));
      })
      .finally(() => {
        window.clearTimeout(watchdog);
        if (isStillCurrent()) {
          setAutoCheck(null);
          autoCheckRef.current = null;
        }
      });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // "Cancel" deliberately does NOT clear `autoCheck` itself - it's still
  // genuinely running until the backend actually confirms that, and
  // startAutoCheck's own `.finally` above clears it moments later once
  // `autoCheckPrice`'s promise resolves with status "cancelled". That round
  // trip is near-instant (the backend checks its cancel flag roughly every
  // 100ms - see cancel_auto_check_price's doc comment, Rust), not a real
  // wait - marko's own "Auto-check button musí byť okamžite znova
  // použiteľné" (immediately usable again) requirement.
  const cancelAutoCheck = useCallback(() => {
    api.cancelAutoCheckPrice().catch((e) => toast.error(errMsg(e)));
  }, [toast]);

  return (
    <div>
      <PageHeader title="Price Checker" subtitle="Compare your unsold inventory against StubHub, Vivid Seats and Ticombo." />

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

          <div className="grid grid-cols-1 gap-4 lg:grid-cols-3">
            {summary.marketplaces.map((view) => (
              <MarketplaceCard
                key={view.marketplaceId}
                eventId={summary.eventId}
                view={view}
                onLinkSaved={load}
                onCheckPrices={() => {
                  setAutoCheckModalData(null);
                  setCheckModalFor(view);
                }}
                autoCheckPhase={autoCheck?.marketplaceId === view.marketplaceId ? autoCheck.phase : null}
                autoCheckRunning={autoCheck !== null}
                canStartAutoCheck={autoCheck === null && checkModalFor === null}
                onStartAutoCheck={(targetUrl) => startAutoCheck(view, targetUrl)}
                onCancelAutoCheck={cancelAutoCheck}
              />
            ))}
          </div>

          <SavePriceCheckModal
            eventId={summary.eventId}
            view={checkModalFor}
            defaultCurrency={summary.myCurrency ?? "EUR"}
            autoCheckResult={autoCheckModalData?.result ?? null}
            autoCheckTargetUrl={autoCheckModalData?.targetUrl ?? null}
            onRetryAutoCheck={
              checkModalFor && autoCheckModalData
                ? () => {
                    const retryView = checkModalFor;
                    const retryUrl = autoCheckModalData.targetUrl;
                    setCheckModalFor(null);
                    setAutoCheckModalData(null);
                    startAutoCheck(retryView, retryUrl);
                  }
                : null
            }
            onClose={() => {
              setCheckModalFor(null);
              setAutoCheckModalData(null);
            }}
            onSaved={() => {
              setCheckModalFor(null);
              setAutoCheckModalData(null);
              load();
            }}
          />
        </>
      )}
    </div>
  );
}

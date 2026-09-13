import { useMemo, useState } from "react";
import { Link } from "react-router-dom";
import type { MarketMap, MarketMapListing, MarketMapSection } from "../lib/types";
import { formatMoney } from "../lib/format";
import { EmptyState, Spinner } from "./ui";
import { IconAlertTriangle } from "./icons";

/* ==========================================================================
   Price Checker - Market Map (2.26.0)

   A SECOND VIEW of one manual scan session. It scans nothing, stores nothing
   and schedules nothing: it draws `compute_market_map`'s read-only fold of
   (a) the listings that session has already accumulated and (b) marko's own
   unsold tickets for the same event.

   WHAT THIS MAP IS NOT

   It is not a geometric replica of a building, and it never pretends to be.
   The scanner reads a listings LIST, not the page's seat-map widget - there
   are no coordinates, no polygons and no real section adjacency anywhere in
   the data (see commands/price_checker_map.rs for the audit). So the blocks
   here are laid out DETERMINISTICALLY - tier band, then sections in numeric-
   then-alphabetical order - and that ordering is a stable way to find a
   section, not a claim about where it physically sits.

   Two consequences that are visible on screen rather than hidden:

     * Market listings have NO seat numbers - the reader has no seat pattern at
       all - so the detail table shows "Seats" as a COUNT. Marko's own tickets
       DO have a real seat, because the app stores it, and that column says so.
     * A market listing has no URL of its own - no href is captured - so a
       listing row cannot link to itself. The event's marketplace link is the
       only URL that exists, and it is offered once, at the top, labelled as
       the page the scan came from.

   PERFORMANCE

   One DOM block per SECTION (tens), never one per listing (hundreds). Listing
   rows are only ever rendered for the one section that is open. Filtering is
   derived state over the same arrays - listings are never copied into a second
   store.
   ========================================================================== */

/** Same rule as `median_of` in price_checker_map.rs: middle value, or the mean
 *  of the middle two in integer cents. Repeated here rather than round-tripped
 *  to the backend because the displayed figures follow the ACTIVE filter, and
 *  a filter change must not cost a command call. The Rust side's tests are the
 *  reference for the unfiltered case. */
function statsFor(listings: MarketMapListing[]): {
  currency: string | null;
  mixed: boolean;
  lowest: number | null;
  median: number | null;
  highest: number | null;
} {
  const currencies = [...new Set(listings.map((l) => l.currency).filter((c): c is string => !!c))];
  const mixed = currencies.length > 1;
  if (listings.length === 0 || mixed) {
    return { currency: currencies.length === 1 ? currencies[0] : null, mixed, lowest: null, median: null, highest: null };
  }
  const sorted = listings.map((l) => l.priceCents).sort((a, b) => a - b);
  const mid = Math.floor(sorted.length / 2);
  const median = sorted.length % 2 === 1 ? sorted[mid] : Math.trunc((sorted[mid - 1] + sorted[mid]) / 2);
  return { currency: currencies[0] ?? null, mixed: false, lowest: sorted[0], median, highest: sorted[sorted.length - 1] };
}

const money = (cents: number | null, currency: string | null) =>
  cents === null ? "—" : formatMoney(cents, currency ?? "EUR");

/** How strongly a block is tinted. Four steps, not a gradient: this is "more
 *  listings here than there", not a measurement, and a continuous ramp would
 *  read as precision the data does not have. */
function densityClass(count: number, max: number): string {
  if (count === 0) return "bg-white dark:bg-slate-900";
  const share = max > 0 ? count / max : 0;
  if (share > 0.66) return "bg-brand-200/80 dark:bg-brand-500/30";
  if (share > 0.33) return "bg-brand-100/80 dark:bg-brand-500/20";
  return "bg-brand-50 dark:bg-brand-500/10";
}

export function MarketMapPanel({
  map,
  loading,
  error,
  sourceUrl,
}: {
  map: MarketMap | null;
  loading: boolean;
  error: string | null;
  /** The event's own marketplace link - the ONLY URL this feature has. */
  sourceUrl?: string | null;
}) {
  const [hidden, setHidden] = useState<Set<string>>(new Set());
  const [showMine, setShowMine] = useState(true);
  const [tierFilter, setTierFilter] = useState("");
  const [selected, setSelected] = useState<string | null>(null);

  /** THE fix for the flashing marko reported ("glitchovanie ked stava tu
   *  mapu"). The map is recomputed after every pass of a scan, and the old
   *  code swapped the entire panel for a loading box each time: the map
   *  vanished, a short box took its place, everything below jumped up, then
   *  the map came back and everything jumped down again - several times a
   *  minute during a run.
   *
   *  Now a refresh is invisible except for one word in the header. The big
   *  loading state is only for the FIRST build, when there is genuinely
   *  nothing to show yet. */
  const refreshing = loading && !!map;

  /** The map with the marketplace/tier filters applied. Derived, not stored -
   *  the listing arrays are the same ones, only shorter. */
  const view = useMemo(() => {
    if (!map) return null;
    const tiers = map.tiers
      .filter((t) => !tierFilter || t.key === tierFilter)
      .map((t) => ({
        ...t,
        sections: t.sections.map((s) => ({
          ...s,
          listings: s.listings.filter((l) => !hidden.has(l.marketplace)),
          myTickets: showMine ? s.myTickets : [],
        })),
      }));
    const maxCount = Math.max(
      0,
      ...tiers.flatMap((t) => t.sections.filter((s) => s.hasSection).map((s) => s.listings.length)),
    );
    return { tiers, maxCount };
  }, [map, hidden, showMine, tierFilter]);

  const selectedSection: MarketMapSection | null = useMemo(() => {
    if (!view || !selected) return null;
    for (const t of view.tiers) {
      const s = t.sections.find((x) => `${t.key}::${x.key}` === selected);
      if (s) return s;
    }
    return null;
  }, [view, selected]);

  if (loading && !map) {
    return (
      <div className="mt-4 flex items-center gap-2 rounded-xl border border-slate-200 p-5 text-sm text-slate-500 dark:border-slate-800 dark:text-slate-400">
        <Spinner className="h-4 w-4" /> Building the map...
      </div>
    );
  }
  // An error while a map is already on screen keeps the map: the last good
  // one is more useful than an error box where the map used to be.
  if (error && !map) {
    return (
      <div className="mt-4 rounded-xl border border-slate-200 p-5 dark:border-slate-800">
        <EmptyState title={error} />
      </div>
    );
  }
  if (!map || !view) return null;

  const toggle = (mp: string) =>
    setHidden((prev) => {
      const next = new Set(prev);
      if (next.has(mp)) next.delete(mp);
      else next.add(mp);
      return next;
    });

  return (
    <section className="mt-4 rounded-xl border border-slate-200 dark:border-slate-800">
      <header className="flex flex-wrap items-center gap-x-4 gap-y-2 border-b border-slate-200 px-5 py-3 dark:border-slate-800">
        <h3 className="text-[13px] font-semibold uppercase tracking-[0.08em] text-slate-700 dark:text-slate-200">
          Market map
        </h3>
        <span className="text-xs tabular-nums text-slate-500 dark:text-slate-400">
          {map.totalListings} listing{map.totalListings === 1 ? "" : "s"} · {map.totalMyTickets} of yours
        </span>
        {/* 2.27.0: the zoom control is GONE. `zoom` is a non-standard CSS
            property that re-lays-out the entire subtree on every change, and
            it was re-applied on every re-render - one of the two things making
            this panel jump while a scan was running. Marko asked for the map
            to be "nehybne" (still), and a map that never scales never has to
            re-lay-out. The blocks are a fixed, readable size and the area
            scrolls. */}
        {refreshing && (
          <span className="ml-auto text-[11px] text-slate-400 dark:text-slate-500">updating…</span>
        )}
      </header>

      {/* ---------------- filters ---------------- */}
      <div className="flex flex-wrap items-center gap-2 border-b border-slate-200 px-5 py-2.5 dark:border-slate-800">
        {map.marketplaces.map((mp) => {
          const on = !hidden.has(mp);
          return (
            <button
              key={mp}
              type="button"
              onClick={() => toggle(mp)}
              aria-pressed={on}
              className={`rounded-lg border px-2.5 py-1 text-xs font-medium capitalize transition-colors ${
                on
                  ? "border-brand-500 bg-brand-50 text-brand-700 dark:bg-brand-500/10 dark:text-brand-300"
                  : "border-slate-200 text-slate-400 dark:border-slate-700 dark:text-slate-500"
              }`}
            >
              {mp}
            </button>
          );
        })}
        <button
          type="button"
          onClick={() => setShowMine((v) => !v)}
          aria-pressed={showMine}
          className={`rounded-lg border px-2.5 py-1 text-xs font-medium transition-colors ${
            showMine
              ? "border-emerald-500 bg-emerald-50 text-emerald-700 dark:bg-emerald-500/10 dark:text-emerald-300"
              : "border-slate-200 text-slate-400 dark:border-slate-700 dark:text-slate-500"
          }`}
        >
          My tickets
        </button>
        {map.tiers.length > 1 && (
          <select
            value={tierFilter}
            onChange={(e) => {
              setTierFilter(e.target.value);
              setSelected(null);
            }}
            className="ml-auto rounded-lg border border-slate-200 bg-white px-2 py-1 text-xs text-slate-700 dark:border-slate-700 dark:bg-slate-900 dark:text-slate-200"
          >
            <option value="">All tiers</option>
            {map.tiers.map((t) => (
              <option key={t.key} value={t.key}>
                {t.label}
              </option>
            ))}
          </select>
        )}
      </div>

      {/* ---------------- the map ---------------- */}
      {!map.hasSectionData ? (
        <div className="px-5 py-6">
          <p className="flex items-start gap-2 text-sm text-amber-700 dark:text-amber-400">
            <IconAlertTriangle className="mt-0.5 h-4 w-4 shrink-0" />
            This scan read prices but no section data, so there is no map to draw. The listings and the market analysis
            above are unaffected - only the section layout is unavailable.
          </p>
        </div>
      ) : (
        // A FIXED min-height. Without it the panel's height changed every
        // time a scan added a section, which moved everything below it and
        // read as the page jumping under the cursor.
        //
        // Line comments on purpose. This position is a ternary BRANCH, not an
        // element's children, so a curly-brace JSX comment is a syntax error
        // here - that is what broke the 2.27.0 build. A block comment would
        // work, but only if its text never contains a comment terminator, and
        // explaining this rule needs to mention one. Line comments cannot be
        // closed early by their own contents, so they cannot repeat either
        // mistake.
        <div
          className="max-h-[28rem] min-h-[11rem] overflow-auto px-5 py-4"
          style={{ overscrollBehavior: "contain" }}
        >
          <div>
            {view.tiers.map((t) => (
              <div key={t.key} className="mb-5 last:mb-0">
                <div className="mb-2 flex items-baseline gap-2">
                  <p className="text-[11px] font-semibold uppercase tracking-[0.12em] text-slate-500 dark:text-slate-400">
                    {t.label}
                  </p>
                  <span className="text-[11px] tabular-nums text-slate-400 dark:text-slate-600">
                    {t.sections.reduce((n, s) => n + s.listings.length, 0)}
                  </span>
                </div>
                <div className="flex flex-wrap gap-2">
                  {t.sections
                    .filter((s) => s.hasSection)
                    .map((s) => {
                      const id = `${t.key}::${s.key}`;
                      const isSel = selected === id;
                      const mineHere = s.myTickets.length > 0;
                      return (
                        <button
                          key={id}
                          type="button"
                          onClick={() => setSelected(isSel ? null : id)}
                          aria-pressed={isSel}
                          title={`${s.label} · ${s.listings.length} listing${s.listings.length === 1 ? "" : "s"}${
                            mineHere ? ` · ${s.myTickets.length} of yours` : ""
                          }`}
                          className={`relative flex h-[3.25rem] w-[5.25rem] flex-col items-center justify-center rounded-lg border text-center ${densityClass(
                            s.listings.length,
                            view.maxCount,
                          )} ${
                            isSel
                              ? "border-brand-600 ring-2 ring-brand-500/40 dark:border-brand-400"
                              : mineHere
                                ? "border-emerald-500/70 dark:border-emerald-400/60"
                                : s.listings.length === 0
                                  ? "border-dashed border-slate-300 dark:border-slate-700"
                                  : "border-slate-200 dark:border-slate-700"
                          }`}
                        >
                          <span className="max-w-full truncate px-1 text-[12px] font-semibold text-slate-800 dark:text-slate-100">
                            {s.label}
                          </span>
                          <span className="text-[10px] tabular-nums text-slate-500 dark:text-slate-400">
                            {s.listings.length === 0 ? "no listings" : s.listings.length}
                          </span>
                          {mineHere && (
                            <span
                              className="absolute right-1 top-1 h-1.5 w-1.5 rounded-full bg-emerald-500"
                              aria-label={`${s.myTickets.length} of your tickets here`}
                            />
                          )}
                        </button>
                      );
                    })}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* The listings a scan read WITHOUT a section. Deliberately outside the
          grid: they belong to no block, and scattering them into one would be
          the exact fabrication this feature is not allowed to make. */}
      {map.listingsWithoutSection > 0 && (
        <p className="border-t border-slate-200 px-5 py-2.5 text-xs text-slate-500 dark:border-slate-800 dark:text-slate-400">
          {map.listingsWithoutSection} listing{map.listingsWithoutSection === 1 ? "" : "s"} in this scan had a price but
          no section, so {map.listingsWithoutSection === 1 ? "it is" : "they are"} not placed on the map. They are still
          counted in the market analysis above.
        </p>
      )}

      {/* ---------------- section detail ---------------- */}
      {selectedSection && (
        <div className="border-t border-slate-200 px-5 py-4 dark:border-slate-800">
          <div className="flex flex-wrap items-baseline gap-x-4 gap-y-1">
            <h4 className="text-sm font-semibold text-slate-900 dark:text-slate-50">Section {selectedSection.label}</h4>
            <span className="text-xs tabular-nums text-slate-500 dark:text-slate-400">
              Market listings: {selectedSection.listings.length} · Your tickets: {selectedSection.myTickets.length}
            </span>
          </div>
          {(() => {
            const st = statsFor(selectedSection.listings);
            if (st.mixed) {
              return (
                <p className="mt-1.5 text-xs text-amber-700 dark:text-amber-400">
                  These listings span more than one currency, so no lowest/median/highest is shown - blending them would
                  be a made-up number.
                </p>
              );
            }
            if (selectedSection.listings.length === 0) {
              return (
                <p className="mt-1.5 text-xs text-slate-400 dark:text-slate-500">
                  No market listings here in this scan.
                </p>
              );
            }
            return (
              <div className="mt-2 flex flex-wrap gap-x-8 gap-y-2">
                {[
                  ["Lowest", st.lowest],
                  ["Median", st.median],
                  ["Highest", st.highest],
                ].map(([label, v]) => (
                  <div key={label as string}>
                    <p className="text-[10px] font-medium uppercase tracking-[0.08em] text-slate-400 dark:text-slate-500">
                      {label}
                    </p>
                    <p className="mt-0.5 text-sm font-semibold tabular-nums text-slate-900 dark:text-slate-100">
                      {money(v as number | null, st.currency)}
                    </p>
                  </div>
                ))}
              </div>
            );
          })()}

          <div className="table-shell mt-3 max-h-72 overflow-auto">
            <table className="w-full text-sm">
              <thead>
                <tr>
                  <th className="th">Row</th>
                  <th className="th">Seats</th>
                  <th className="th">Marketplace</th>
                  <th className="th text-right">Price</th>
                  <th className="th">Mine</th>
                </tr>
              </thead>
              <tbody>
                {/* Marko's own tickets first - they are the reason he opened
                    the section. Each links to the EXISTING order detail; no
                    new ticket screen was built for this. */}
                {selectedSection.myTickets.map((t) => (
                  <tr key={`mine-${t.ticketId}`} className="bg-emerald-50/60 dark:bg-emerald-500/[0.07]">
                    <td className="td">{t.row ?? "—"}</td>
                    <td className="td">{t.seat ?? "—"}</td>
                    <td className="td">
                      <Link
                        to={`/orders/${t.orderId}`}
                        className="font-medium text-brand-700 hover:underline dark:text-brand-400"
                      >
                        {t.code}
                      </Link>
                    </td>
                    <td className="td text-right tabular-nums">
                      {t.listingPriceCents === null ? "not listed" : formatMoney(t.listingPriceCents, t.currency)}
                    </td>
                    <td className="td font-medium text-emerald-700 dark:text-emerald-400">Yes</td>
                  </tr>
                ))}
                {selectedSection.listings.map((l, i) => (
                  <tr key={`l-${l.listingId ?? i}`}>
                    <td className="td">{l.row ?? "—"}</td>
                    {/* A COUNT of seats. The scanner has no seat-number
                        pattern at all, so a seat number here would be
                        invented - see this file's header. */}
                    <td className="td tabular-nums">{l.quantity ?? "—"}</td>
                    <td className="td capitalize">{l.marketplace}</td>
                    <td className="td text-right tabular-nums">
                      {l.currency ? formatMoney(l.priceCents, l.currency) : `${(l.priceCents / 100).toFixed(2)} (no currency)`}
                    </td>
                    <td className="td text-slate-400 dark:text-slate-500">No</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          <p className="mt-2 text-xs leading-relaxed text-slate-400 dark:text-slate-500">
            Seats is the number of seats in a market listing, not a seat number - the scanner never reads seat numbers.
            Your own tickets show their real seat, because the app stores it. Market listings have no link of their own
            either;{" "}
            {sourceUrl ? (
              <a href={sourceUrl} target="_blank" rel="noreferrer" className="text-brand-600 hover:underline dark:text-brand-400">
                the page this scan came from
              </a>
            ) : (
              "the marketplace page this scan came from"
            )}{" "}
            is the only URL there is.
          </p>
        </div>
      )}
    </section>
  );
}

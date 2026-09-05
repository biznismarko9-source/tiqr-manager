# Changelog

Short, append-only entries, newest at the top - one entry per completed
task under the TIQR development protocol (see step 8). This is not a
replacement for the detailed `REDESIGN-X.Y.Z-REPORT.md` / `*-REPORT.md`
files at the repo root (one per release, written for marko, in Slovak) -
those still get written for real releases. This log exists so a future
session can see recent activity at a glance without opening any of them.

(2026-09-01: this file merges two copies that grew independently in
different sessions after 2.0.80 - one bootstrapped at 2.0.80 with entries
back to 2.0.75, the other bootstrapped at 2.1.9 with entries from 2.1.9
onward. Versions 2.0.81-2.1.8 - Finance module, Price Checker auto-check
iterations - fall in the gap between the two bootstraps and are not
backfilled here, consistent with this file's own existing policy below;
read the matching `REDESIGN-X.Y.Z-REPORT.md`/`*-REPORT.md` for any of
those directly.)

## 2.5.2 - "Forgot password?" via a deep link into the app; Discord sign-in deferred

marko's own follow-up request after 2.5.1. No schema changes. Discord
sign-in was asked for too but is NOT built - see `PROTECTED_AREAS.md`'s
"2.5.2" entry for why (needs a Cloud Function + Firebase's paid Blaze plan,
which marko chose against for now).

1. **Added**: a real "Forgot password?" flow from Welcome.tsx's login form,
   using Firebase's own `sendPasswordResetEmail`/`verifyPasswordResetCode`/
   `confirmPasswordReset` - no new backend, no Cloud Functions, no Blaze
   plan.
2. **Added**: the emailed reset link now opens TIQR Manager directly
   (`handleCodeInApp: true` + a new `tiqrmanager://` custom URL scheme via
   `tauri-plugin-deep-link`) instead of a browser tab - marko's first choice
   (a typed short code) would have needed the same paid infrastructure as
   Discord above, so this was worked out with him directly as the
   alternative.
3. **Added**: `docs/reset-redirect.html`, a static hand-off page on the same
   GitHub Pages site `docs/privacy.html` already uses, forwarding Firebase's
   link into the app.
4. **Added**: `src/pages/ResetPassword.tsx` - verifies the incoming reset
   code and sets the new password.
5. **Needs one manual step**: add `biznismarko9-source.github.io` to Firebase
   Console -> Authentication -> Settings -> Authorized domains before this
   works end to end.

## 2.5.1 - Ticket Center rebuilt around orders, sidebar reorder, Calendar visual refresh

marko's own direct follow-up on the 2.5.0 release below, delivered as its
own round. No backend/schema changes. Packaging
(`REDESIGN-2.5.1-REPORT.md` + zip) was deferred past this round - marko
signaled more feedback was coming right after - and built once he
separately asked ("zabal to"; see `PROTECTED_AREAS.md`'s "2.5.1" entry).

1. **Changed**: Ticket Center moved out of Finance (was a subtab there for
   one version, 2.4.4) back to its own top-level page/route
   (`/ticket-center`).
2. **Rebuilt**: Ticket Center now lists ORDERS (via the same `api.listOrders`
   Orders.tsx already calls), not individual tickets/sale-batches - click an
   order to see and edit what's outstanding on each of its tickets on the
   existing Order Detail page. `TicketControlCenter.tsx` (2.4.3),
   `FulfillmentCenter.tsx` (2.2.12), and the `finance/TicketCenter.tsx`
   subtab shell are all deleted; 4 new quick-filter tiles (Needs
   attention/listing/payment/delivery) replace both pages' old filter/
   category systems.
3. **Changed**: sidebar top-level order now matches marko's exact list -
   Dashboard, Tickets, Price Checker, Pulls, Finance, Ticket Center,
   Calendar (Calendar moved from right after Dashboard to last).
4. **Changed**: Order Detail's "arrived from" Back link now also recognizes
   Ticket Center as an origin (`/ticket-center` -> "Back to ticket center").
5. **Visual refresh only**: the Calendar page (2.5.0) - a consistent accent
   color per entry kind across the grid/filters/modal/summary, severity
   shown as a ring/text-color layered on top instead of the only signal,
   weekend/today cell shading, and a weekday name in the Day Detail title.
   No data, hook, or navigation change.

## 2.5.0 - TIQR Operations Calendar

marko's own spec for a new cross-domain Month/Week calendar page, delivered
together with the 2.4.4 round below in this same release.

1. **New**: `/calendar` page - a Month/Week calendar aggregating every part
   of the app with a real date: events, orders, sales (grouped by batch,
   never one row per ticket), pulls, and Attention Center items. Today/
   Previous/Next navigation, a Day Detail view (click any day or its
   "+X more"), a client-side Filters row, and a "Today & next 7 days"
   summary card.
2. **New backend**: `commands/calendar.rs` (`get_calendar`, one command) +
   `CalendarFilters`/`CalendarEntry` models. No new migration, no new
   table - reuses `attention_center::get_attention_center_impl` and
   `sales::GROUP_KEY_EXPR` directly rather than re-deriving either.
3. **Deliberately NOT implemented**: payout, payment, and fulfillment
   calendar entries - none of the 3 has a real, reliably-existing date
   anywhere in this app today (see `PROTECTED_AREAS.md`'s new "2.5.0"
   entry for the full research). Nothing was invented to fill these in.
4. **New sidebar entry**: "Calendar", directly below Dashboard.
5. 14 new Rust tests (`commands/calendar.rs`), full suite green (1052
   tests); `npx tsc -b` and `npm run build` both clean.

## 2.4.4 - Ticket Center consolidation, sidebar regroup, theme toggle

marko's own request, a pure frontend/UX round (no backend, schema, or
migration changes at all) delivered before his separate 2.5.0 Calendar
spec.

1. **Merged**: Ticket Control Center (2.4.3) + Fulfillment Center (2.2.12),
   previously two standalone top-level sidebar pages, now live under
   Finance as one "Ticket Center" tab with two subtabs (Control Center,
   Fulfillment) - new `finance/TicketCenter.tsx`. `/control-center` and
   `/fulfillment` routes removed; both components reused unchanged
   internally aside from Control Center's own fixes below.
2. **Sidebar regrouped**: Events/Orders/Tickets/Sales/Inventory now sit
   under one collapsible "Tickets" entry instead of 5 flat rows - all 5
   routes themselves unchanged.
3. **New**: one-click light/dark toggle above the sidebar's profile widget,
   reusing the existing `useTheme()` hook. Settings -> Appearance (the old
   3-way Light/System/Dark picker) removed - moved, not duplicated.
4. **Fixed**: Ticket Control Center's sticky header had a translucent
   dark-mode background (`dark:bg-slate-800/60`) letting scrolled row text
   bleed through while scrolling - now fully opaque.
5. **Changed** (Ticket Control Center): "Ticket / Seats" column renamed to
   "Seats", now shows only the seat location (ticket code moved to a hover
   tooltip); Order cell now independently opens Order Detail on click.
6. **Changed** (Dashboard): "Sales by platform"'s internal scrollbar
   removed, replaced with the same slice + "Show N more" pattern the
   Activity tab's Recent cards already use.

`npx tsc -b` and `npm run build` both clean; no Rust changed, so
`cargo test --lib` is unaffected. Version bumped **2.4.3 -> 2.4.4**.

## 2.4.3 - Ticket Control Center

marko's own focused-task request: one central work screen to manage and
check tickets across every event at once, built entirely on top of the
existing tickets/listings/sales data - explicitly not a new parallel ticket
system.

1. **New**: `/control-center` page - sticky filters (Event, Date range,
   Tier/Level, Section, Row, Ticket status, Listing status, Sale status,
   Payment status, Delivery status, Marketplace), 8 quick filters (All/
   Unsold/Unlisted/Listed/Sold/Pending payment/Pending delivery/Refunded),
   search across ticket/order/event/section/row/marketplace/listing id-url,
   and a dense table (first in this app to own its own scroll instead of
   growing the whole page) over one new backend query,
   `list_control_center_tickets` (`commands/ticket_control_center.rs`). Row
   click opens the existing Sale Detail or Order Detail, whichever applies.
2. **Bulk actions - all existing mechanisms**: Section/Row/Tier/Seat/Listing
   price via the shared `BulkTicketEditBar` (now with a new Tier option,
   also benefiting Sale Detail/Order Detail); listing status via the
   existing `bulkUpdateTicketListingsStatus`; CSV export of the selection
   via the existing `exportTicketsCsvSelected`. No refund/resell bulk
   actions, per marko's explicit instruction.
3. **New, purely additive read signal**: `isRefunded` (an `EXISTS` check
   against `sales`) - lets the "Refunded" quick filter surface a
   refunded-and-not-yet-resold ticket, which the existing active-sale-only
   join can't otherwise distinguish from a never-sold one. Reads only; no
   refund/resell/money logic touched.
4. **Untouched**: refund/resell logic, `batch_id`, every money/cents column,
   Listings/Sales/Finance/Orders core - per marko's explicit "DÔLEŽITÉ" list.
   No new migration.

Full backend suite green: `cargo test --lib` (1038 passed, +12 over 2.4.2,
0 failed, 3 ignored), `cargo clippy --lib` clean of new warnings, `npx tsc
-b`, `npm run build` all clean. Version bumped **2.4.2 -> 2.4.3**.

## 2.4.2 - Live Market Monitor removed; Price Checker back to a manual tool

marko decided he does not want the 2.4.1 "Live Market Monitor" feature in
the app at all (*"TÚTO FUNKCIU NECHCEM V APLIKÁCII VÔBEC"*) and asked for
it removed entirely - no background/scheduled scanning, no automatic
monitoring, ever - with Price Checker returned to a purely manual tool.
Full reasoning in `PROTECTED_AREAS.md`'s new "2.4.2" entry.

1. **Removed entirely**: backend module `price_checker_monitor.rs` and its
   2 commands (`get_market_monitor_summary`, `list_market_snapshots`); both
   scan-result hooks into it from `price_checker_scanner.rs`; the
   `market_alert` Attention Center category (`attention_center.rs`'s
   `push_item` reverted to its original 2-shape key, Dashboard's 6th box
   and grid column removed); Auto Monitor (ON/OFF + 15m/30m/1h/3h/6h
   interval) and "Scan All" from `PriceChecker.tsx`; the Live Market
   Monitor panel and the Market History view/modal; every related Rust
   struct (`models.rs`) and TypeScript type (`types.ts`/`api.ts`).
2. **Price Checker unchanged**: event selection, marketplace URLs/source
   handling, the manual Visible Scanner, Market Analysis, tier/section
   grouping, price history, Your Tickets comparison - all pre-existing
   functionality, untouched. No redesign.
3. **Database**: migration `026_price_checker_market_monitor.sql` and its 4
   tables (`market_snapshots`, `market_snapshot_tiers`, `market_source_
   status`, `market_alerts`) were **kept in the schema, not deleted** - 2.4.1
   already shipped and marko's own local DB has already run this migration,
   so this codebase's forward-only migration rule means it can never be
   safely deleted or renumbered. Only the application code that read/wrote
   these tables was removed; no user data was touched. The next new
   migration is **027**, not a reused "026".
4. Two small judgment calls, flagged per this codebase's own "smallest
   consistent solution, flag on ambiguity" convention: "Scan All" was
   removed as in-scope (it existed purely to complement Auto Monitor, no
   standalone purpose without it); `price_checker_analysis.rs`'s
   `group_by_tier` was left `pub(crate)` rather than reverted to private
   (bumped in 2.4.1 for the now-deleted module to reuse - harmless residual,
   not worth an extra touch to that protected file for zero functional
   gain).

Full backend suite green: `cargo test --lib` (1026 passed, -32 removed with
the feature, 0 failed, 3 ignored), `npx tsc -b`, `npm run build` all clean.
Version bumped **2.4.1 -> 2.4.2** - removing a shipped feature still bumps
the version forward, never back to a number already used (same precedent as
2.3.0's revert shipping as 2.3.1) - see `PROJECT_STATE/CURRENT_STATE.md`'s
"## Version" section. See `PROTECTED_AREAS.md`'s new "2.4.2" entry before
ever touching migration 026, `group_by_tier`, or migration numbering again.

## 2.4.1 - Price Checker Live Market Monitor

Marko cancelled the "Live Event Intelligence" direction below outright
("Predchádzajúci nápad 'Live Event Intelligence' RUŠÍME ÚPLNE") and asked
for all online/live-market functionality to live directly inside Price
Checker instead: EVENT -> MARKETPLACE SOURCES -> SCAN -> SNAPSHOT -> HISTORY
-> CHANGE DETECTION -> MARKET ALERTS, built entirely on the already-shipped
Visible Scanner (2.1.9) and Market Analysis (2.2.0) - no CAPTCHA bypass, no
proxy rotation, no anti-bot workaround, and no automation beyond reading
whatever a human already has open in a real, visible window. Full reasoning
in `PROTECTED_AREAS.md`'s new "2.4.1 - Price Checker Live Market Monitor"
entry.

1. **New backend module `price_checker_monitor.rs`** - records a permanent,
   never-overwritten snapshot after every successful/partial scan
   (`market_snapshots`/`market_snapshot_tiers`, migration `026_price_
   checker_market_monitor.sql`), tracks each marketplace's connection status
   (`market_source_status`: not_connected/connected/success/failed), and
   diffs each new snapshot against the previous one - overall and per tier
   (never per section/row/seat) - to raise MARKET DROP / MARKET RISE / NEW
   SUPPLY / SUPPLY DROP alerts (`market_alerts`) at transparent, reused
   thresholds (5% price, 20% supply - the same constants Price Checker's own
   recommended-price and Inventory Intelligence's own outside-market logic
   already use elsewhere). A SOURCE FAILURE alert fires only on a genuine
   success-to-failure transition, never on the first-ever failure or on
   repeated consecutive ones - keeps this quiet instead of noisy.
2. **Auto Monitor** - an ON/OFF toggle with a 15m/30m/1h/3h/6h interval,
   scoped to one already-open Visible Scanner window per marketplace card;
   it is the identical "Scan Visible Prices" call the button already makes,
   fired on a schedule, never opening a window or reading a page on its own,
   and it turns itself off the moment that window closes. "Scan All" fires
   the same call once for every marketplace on the current event that
   already has a window open.
3. **Price Checker UI**: each marketplace card gained a Live Market Monitor
   panel - connection status, last successful scan (never cleared by a
   later failure - the app stays useful on cached data even fully offline),
   the latest snapshot's stats, Auto Monitor controls, and its recent Market
   Alerts; plus a "Market History" view of every saved snapshot.
4. **Dashboard Attention Center gained a 6th box, "LIVE MARKET ALERTS"**
   (`market_alert` category, the single most recent alert per event/
   marketplace) - named differently from marko's own literal spec wording
   ("MARKET ATTENTION") because that title was already taken by the
   existing `outside_market_price` box (2.2.11, an unrelated feature: your
   OWN listing prices vs. the market). Clicking a row jumps straight to
   Price Checker at that event and marketplace (scrolled into view and
   briefly highlighted) - no new separate dashboard.

32 new backend unit tests (27 in `price_checker_monitor.rs`, 5 in
`attention_center.rs`). Full suite green: `cargo test --lib` (1058 passed, 0
failed, 3 ignored), `npx tsc -b`, `npm run build`. See `PROTECTED_AREAS.md`'s
new "2.4.1 - Price Checker Live Market Monitor" entry before touching any of
this again, and the entry directly below for why reusing the version number
"2.4.0" was verified safe before this release ultimately moved one step
further to **2.4.1** instead (a plain filename-collision reason, not an
auto-updater one - see `PROJECT_STATE/CURRENT_STATE.md`'s "## Version"
section for the full story).

## 2.4.0 (pre-release direction, never shipped) - Live Event Intelligence Foundation - REVERTED, see entry above

marko's next spec after 2.3.5: an Event can now optionally carry a
CONFIRMED online identity on exactly 3 marketplaces - Viagogo, Vivid Seats,
Ticombo. Foundation work only - no pricing logic, no changes to the
existing Price Checker or its scanner. Full reasoning in
`PROTECTED_AREAS.md`'s new "2.4.0 (pre-release direction)" entry.

**Kept as history only (this file is append-only) - marko reviewed this
build and cancelled the whole direction outright** ("Predchádzajúci nápad
'Live Event Intelligence' RUŠÍME ÚPLNE") in favor of putting all online/
live-market functionality directly inside Price Checker instead - see the
real "2.4.1 - Price Checker Live Market Monitor" entry above. Unlike 2.3.0
(reverted as **2.3.1**, a version bump forward, because that build had
already been offered as a real release), this direction was never released -
only handed over as a review package - so no install anywhere ever recorded
it, and the version number "2.4.0" was safe to reuse for the real feature
that replaced it - though that real feature's own version ultimately moved
one more step forward, to **2.4.1**, for the separate and unrelated
practical reason explained in the entry above and in
`PROJECT_STATE/CURRENT_STATE.md`'s "## Version" section. See that section
and `PROJECT_STATE/PROTECTED_AREAS.md`'s "2.4.0 (pre-release direction)"
entry for the full reasoning, and re-verify that same fact (was anything
with this version/migration number ever actually installed anywhere?)
before assuming a THIRD reverted direction can reuse its number too - it
depends entirely on that, not on precedent alone.

1. **New table `event_online_sources`** (migration
   `026_live_event_intelligence.sql`) - a standalone table, not a new
   column on `events` and not a foreign key onto the general, marko-managed
   `marketplaces` lookup. `UNIQUE(event_id, source)` enforces "at most once
   per marketplace per event"; `verified`/`active` are two independent
   flags (confirmed-by-a-human vs. still-connected).
2. **Discovery, always human-confirmed.** "Find Online Event" opens a real,
   visible browser window (reusing the Visible Scanner's technique, never
   its code/state) on a best-effort search URL; marko searches himself;
   "Capture this page" reads only the current page's title+URL as one
   candidate; "Use this one" is the only action that ever saves a source as
   verified. "Refresh" is the identical flow against an already-saved URL -
   also how a manually-connected source becomes verified. "Connect
   manually" skips the window for when marko already has the URL.
3. **New compact "Live Event Intelligence" block** on EventDetail's
   Overview tab (above Inventory Intelligence) - always exactly 3 rows,
   Find Online Event / Connect manually / Refresh / Open source /
   Disconnect-Reconnect.
4. **No new networking primitive at all** - the only network access this
   feature ever performs is opening a real, visible window a human drives;
   no backend HTTP calls to any of the 3 marketplaces, ever.

19 new backend unit tests + 3 new migration-upgrade tests (existing events
untouched, CHECK constraint enforced on an upgraded database, cascade
delete verified). `cargo test --lib` (1042 passed), `npx tsc -b`, `npm run
build` all green.

## 2.3.5 - Sync/push redesign: self-healing push, real sync diff, no more UI freeze

Marko came back after 2.3.4 with one detailed message re-explaining the
whole intended sync/push design from scratch, using Pulls as the reference -
the narrow bug fixes so far hadn't matched his actual mental model. Three
fixes, all covered in depth in `PROTECTED_AREAS.md`'s new "2.3.5" entry:

1. **UI freeze fixed.** Every sync/push button froze the whole app until its
   network call finished (marko: "ked zapnem alebo kliknem na cokolvvek...
   apka zamrzne"). All 11 sheet-sync commands were plain synchronous `fn`,
   which Tauri runs on its single main/IPC thread - converted to `async fn`
   + `spawn_blocking`, same pattern already proven for Google sign-in
   (2.0.12->2.0.13). Zero changes to the underlying sync/push logic.
2. **Order/Sales sync now updates an already-linked row when the sheet
   changed it**, matching Pulls sync - previously every marked row was
   skipped unconditionally, no comparison at all. Tracks platform/date/
   currency/email/Order ID; deliberately never quantity/price (tickets
   already have exact-cent costs allocated against those - same "ask before
   touching" boundary as the 2.0.53 currency-push feature).
3. **Push Orders is now self-healing** - marko, twice: if he deletes an
   order's row from the sheet by hand and pushes again, it must notice and
   add it back, using the same code. Push Sales needed no changes at all for
   this: it never creates rows, so once Push Orders restores the row, Push
   Sales's existing "fill in blank cells" behavior already re-populates the
   sales columns on the next run - proved with a dedicated test chaining
   both functions. This also resolves the row-426 order that went
   permanently invisible after 2.3.4 (see that entry below).

9 new/updated sync-diff tests, 3 push self-healing tests, 1 cross-function
integration test. Full suite 1020/1020 passed, 0 failed; `tsc -b`/
`npm run build` clean. No frontend changes needed - the sync/push buttons'
busy-state/spinner UI already existed, it was just neutered by the backend
freeze.

## 2.3.4 - Sheets push: row placement fixed properly this time

2.3.3's fix wasn't enough - marko sent a screenshot proving it. Revenue (P)
and Profit (Q) in his real sheet had live formulas filled all the way to
row 425, even though only ~16 rows have real order data. Somewhere in this
sheet's history, `plan_sheet_structure_updates` had written formulas that
far down, and a formula is non-empty content too - so the raw `"A1:AZ"`
row count 2.3.3 anchored on was never actually small in his sheet, it
already agreed with Google's own confused auto-detection. Same bug,
different disguise.

Fixed properly: `next_append_row`/`next_append_range`
(`orders_sheet_sync.rs`) now scan for the LAST row whose **marker cell**
(TIQR ID) is non-empty - the one column only this app ever writes, and
only for a row holding a real order - and target the row right after it,
ignoring any stray formula residue further down. 5 unit tests added,
including the literal shape of marko's real sheet (16 real rows + 408
stray-formula rows still targets row 18) and a deliberate "never reuse a
gap in the middle" case. Full suite 1011/1011 passed, 0 failed; `tsc -b`/
`npm run build` clean.

Also found and documented (not fixed, not asked for): marko deleted the
row-426 order's content directly in the sheet while testing between
versions, which this app's own bookkeeping now can't see - that order
won't automatically come back. See `PROTECTED_AREAS.md`'s "2.3.2-2.3.4"
entry before doing anything about it.

Revenue/Profit formulas being missing on older rows is still believed to
be the same root cause, not independently fixed - marko needs to confirm
on his real sheet after this update. Not marked resolved yet.

## 2.3.3 - Sheets push: row placement fixed at the source

Follow-up to 2.3.2's investigation (see `PROTECTED_AREAS.md`'s "2.3.2/2.3.3"
entry for the full trail). Marko confirmed row 18 and rows 19-425 in his
real sheet are genuinely empty, and that retrying the push already once did
NOT bring back the missing Revenue/Profit formulas - which pointed at one
shared root cause rather than two separate bugs.

Fixed: `push_orders_impl` no longer hands row placement to Google's own
`append_values` table auto-detection (a bare `"A1"` anchor, which was
landing new rows at 426 instead of 18). It now computes the exact target
row itself, via a new pure, unit-tested `next_append_range` function, from
the same `"A1:AZ"` read this function already trusts for its header/
marker-column lookup, and writes with `update_values` instead. 3 new tests
added (`next_append_range_*`), all passing; full suite 1009/1009 passed, 0
failed. `tsc -b`/`npm run build` also clean (frontend untouched this
release).

Not independently touched, believed fixed as a side effect: the missing
Revenue/Profit formulas. `plan_sheet_structure_updates` already recomputes
formulas for the sheet's entire current extent on every push, so once new
rows land in the right place, the very next push should backfill formulas
correctly again. **Marko needs to click Push Orders/Push Sales once more
after this update and confirm** - not marked resolved until he does; see
`PROTECTED_AREAS.md` for exactly what to report back if it isn't.

Known, deliberate limitation: this does not move the order a past, buggy
push already stranded at row 426 in marko's real sheet - that needs a
manual fix in the sheet itself if he wants it back in the contiguous block,
since this app can't safely edit that live row unattended.

## 2.3.2 - Dashboard: all-time Total cost

Marko's request: a place on the Dashboard to see total cost across every
ticket he owns. Added a "Total cost" StatCard to the Financials tab's
existing "Current inventory (all time)" section, next to
Available/Listed/Sold (total)/Purchased (total) - zero backend change,
`data.inventory.totalCostCents`/`.currency` (a `FinanceSummary`) were
already computed and sent to the frontend every load, just never rendered
anywhere. Verified with `tsc -b`/`npm run build`/`cargo test --lib` (1006
passed, 0 failed, unaffected since no `.rs` file changed).

Also investigated (not yet fixed - see `PROTECTED_AREAS.md`'s "2.3.2"
entry) two Google Sheets sync complaints from the same message: Push
Orders/Sales landing a new row at 426 instead of at row 18 (the sheet's
real next empty row), and Revenue/Profit formulas missing on many rows.
Root-caused enough to have a credible fix shape for the first, but stopped
short of writing it - this touches marko's live, real-money Google Sheet,
and the fix's correctness depends on what's actually sitting in rows
19-425 of his real sheet, which cannot be verified from here. Asked marko
directly rather than guess. No Sheets-sync code was changed this release.

## 2.3.1 - Event Lifecycle removed (revert of 2.3.0)

Marko reviewed the 2.3.0 build below (delivered as a zip + Slovak report,
never actually published via `1-CLICK-UPDATE.bat`) and asked to remove it
entirely and go back to the previous version - no specific complaint beyond
not liking it in place ("mi tam nieje sympaticky"). `Events.tsx`/
`EventDetail.tsx` were reverted edit-for-edit to their exact pre-2.3.0
content (verified with `tsc -b`/`npm run build`/`cargo test --lib`, all
clean). The version was NOT rolled back to 2.2.12, even though the code
was - marko himself caught this right after asking for the revert: reusing
an old version number breaks the auto-updater for anyone already offered a
newer one ("ked dam stary tak to nefunguje potom dobre"). So this reverted
build ships as **2.3.1** instead (all 9 locations) - a version bump
forward that happens to contain strictly less than the 2.3.0 it follows.
**Lesson for future sessions:** a revert-to-previous-behavior task still
needs a version bump forward, never a rollback to a number already used
before - the updater compares versions directly. The entry right below is
kept as real history (this file is append-only), not deleted - see
`PROJECT_STATE/CURRENT_STATE.md`'s matching note for what to do if a similar
feature is requested again.

## 2.3.0 - Event Lifecycle / Event Operations (reverted - see entry above)

Marko's next task after 2.2.11/2.2.12: one consistent, derived "what stage
is this event at" lifecycle phase - no new manually-set status, no
migration, no backend change at all (`cargo test --lib` byte-for-byte
unchanged from 2.2.12). See `REDESIGN-2.3.0-REPORT.md` (Slovak) and
`PROJECT_STATE/PROTECTED_AREAS.md`'s "2.3.0" entry.

- **6 phases** - UPCOMING -> INVENTORY -> LISTED -> SELLING -> EVENT DAY ->
  COMPLETED (`computeEventLifecyclePhase`, `Events.tsx`) - a pure function
  of the already-returned `EventWithStats`, zero extra IPC calls. His
  proposed POST EVENT is folded into COMPLETED (his own literal COMPLETED
  rule leaves no gap to place it in); CANCELLED stays inside COMPLETED too
  (it already has its own Status badge). Both judgment calls explained in
  `PROTECTED_AREAS.md`.
- **Events overview**: lifecycle phase shown as a small pill stacked under
  the existing Status badge (no new column/colgroup change), plus a new
  "Lifecycle phase" filter dropdown, ANDed with the existing Upcoming/
  Completed tab.
- **Event Workspace (Overview tab)**: new `EventLifecycleBlock` at the top -
  current phase, a simple progress strip, an operational summary line
  (tickets/listed/sold/pending fulfillment), and a "Next Actions" list -
  sourced entirely from already-existing `list_sale_groups` (per-event
  `isSaleGroupDone`) and `get_attention_center` (global, filtered to this
  event) data, no new business logic.
- **Tests**: 25/25 on a disposable esbuild+Node script exercising marko's
  full scenario list (upcoming with/without inventory, listings, sales,
  event day, date passed, completed/cancelled, phase precedence,
  filter-by-phase, pending fulfillment, Next Actions aggregation) against
  the real exported functions. `cargo test --lib`: 1006 passed, 0 failed, 3
  ignored - unchanged, no `.rs` file touched. `tsc -b`/`npm run build`: 0
  errors.

## 2.2.12 - Fulfillment Center

Marko's ČASŤ C, shipped as its own release right after 2.2.11 (same
message, explicitly split into two releases). See
`REDESIGN-2.2.12-REPORT.md` (Slovak) and
`PROJECT_STATE/PROTECTED_AREAS.md`'s "2.2.12" entry. Zero backend/migration
changes - frontend only.

- **New page: Fulfillment Center** (`src/pages/FulfillmentCenter.tsx`, new
  `/fulfillment` sidebar entry right after Sales) - one place to see every
  sold ticket not yet fully paid, delivered, and completed. Fetches the
  same `SaleGroup[]` Sales.tsx already fetches and reuses its exact
  `isSaleGroupDone` rule (now exported) - no parallel status system, no new
  backend command.
- **4 clickable tiles double as KPIs and category filters**: Pending Sales
  (all), Awaiting Payment, Awaiting Delivery, Ready to Complete - the last
  one a new, pure display derivation (paid + delivered in full; the only
  way such a group is still Pending is a partial refund).
- **Table**: Event / Ticket+Seats / Sale price / Payment status / Delivery
  status (new group-level badge, existing tone colors) / Overall status /
  Action - row click or the Action button both open the existing Sale
  Detail page (`/sales/:id`), no new navigation mechanism.
- Verified with a disposable, esbuild-bundled Node script (built and run
  once, then deleted) asserting all of marko's listed test scenarios
  against the real exported functions - 21/21 passed - since this codebase
  has no frontend test framework.

## 2.2.11 - Attention Center UX rework + Dashboard cleanup

Marko's own next request, split into two explicit parts, both frontend-only
(zero backend/migration changes). See `REDESIGN-2.2.11-REPORT.md` (Slovak)
and `PROJECT_STATE/PROTECTED_AREAS.md`'s "2.2.11" entry for the judgment
calls behind each.

- **Attention Center reworked from one mixed feed into 5 named, always-
  visible boxes** (`Dashboard.tsx`): NO LISTING PRICE YET / NO ACTIVE
  LISTING / NOT DELIVERED YET / EVENT COMING SOON / MARKET ATTENTION -
  grouped by the item's existing `category` field instead of `priority`.
  Clicking a box reveals only that category's own rows (reusing
  `AttentionCenterRow` unchanged); the old mixed feed is gone as default
  content. A box with 0 items is disabled, not hidden. Zero backend
  changes - `attention_center.rs` already satisfied every MARKET ATTENTION
  constraint (Price-Checker-gated, no automatic pricing, section/row/tier
  never a pricing factor), confirmed by reading its own doc comment and
  tests rather than assumed. `AttentionSection`/the alert bell (older,
  separate feature) are untouched.
- **Dashboard Overview: unbounded "Sales by platform" list capped** with
  its own `max-h-72 overflow-y-auto`, so a long platform list scrolls
  internally instead of growing the whole page - paired with a small,
  one-step trim of two existing spacing values on the same tab (not a
  redesign). `Layout.tsx`'s scroll container was checked and needed no
  change.

## 2.2.10 - Eight follow-ups from marko's 2.2.9 review

Marko reviewed 2.2.9 and sent two rapid-fire messages (7 screenshots) with
eight mostly-unrelated requests. See `REDESIGN-2.2.10-REPORT.md` (Slovak)
and `PROJECT_STATE/PROTECTED_AREAS.md`'s "2.2.10" entry for the judgment
calls behind each. No migration this release.

- **Seats format: dropped the "Sec"/"Row"/"Seat" labels 2.2.9 had just
  added**, back to bare " · "-joined values (`formatSeatLocation`/
  `formatSeatsSummary`, `lib/format.ts`) - a real section value ("Sec 408",
  "Category D, Standing") sometimes already read as a full label, so the
  prefix produced "Sec Sec 408"-style duplication. Reaches every "Seats"
  column across the app via the same two shared helpers.
- **Orders tabs reworked: "Active"/"Paid" -> "Active"/"Completed"**, with a
  real bucketing change, not just a relabel - an order is now Completed
  once its event's date has passed (or its status is completed/cancelled)
  OR the order itself is fully sold+delivered+paid, whichever comes first.
  The New Order event picker now excludes those same "done" events too
  (previously unfiltered).
- **Attention Center "mixed" ordering fixed** - the real cause was the
  sort's own tie-break (grouping by category name before order), not the
  2.2.9 grouping-by-order logic itself. Also now excludes done events from
  3 of its 5 categories (missing listing price/no active listing/outside
  market price) - `sold_undelivered` and `event_soon` are deliberately
  exempt.
- **Sales Pending/Completed now requires sold+delivered+paid together**
  (or fully refunded) - a sale missing only its delivery status no longer
  incorrectly showed as Completed.
- **Two confirmed Google Sheets push bugs fixed** (Orders and Pulls push):
  local "already synced" bookkeeping was being written before the actual
  network write was even attempted, so a failed push still silently looked
  successful afterward. Both now record success only after the sheet write
  is confirmed.
- **Google's `invalid_grant` sign-in error now shows a short "sign in
  again" message** instead of a long raw JSON dump - best-effort fix for a
  reported long error after Google sign-in; not independently reproducible
  in this environment.
- **Native right-click context menu disabled everywhere** (no config flag
  exists for this in Tauri/WRY - the standard JS-side `preventDefault` fix).

Verified: `cargo test --lib` (1006 passed, +7 net new tests, 0 failed),
`tsc -b` and `vite build` both clean.

## 2.2.9 - Six follow-ups from marko's 2.2.8 review

Marko reviewed the just-shipped 2.2.8 Attention Center and sent six mostly-
unrelated small requests in one message. See `REDESIGN-2.2.9-REPORT.md`
(Slovak) and `PROJECT_STATE/PROTECTED_AREAS.md`'s "2.2.9" entry for the
judgment calls behind each.

- **Seatriks retired from Price Checker only** (`marketplaces.active = 0`,
  `migrations/025_deactivate_seatriks_price_checker.sql`) - same mechanism
  already used for StubHub. Stays fully available in Listings' "Add
  listing" picker.
- **Settings -> Integrations' Anthropic API key card relabeled** from
  "AI-assisted price reading" to the general "AI features" - same key,
  same storage, just a forward-looking name since it's meant to power more
  than one AI feature over time.
- **No live "balance" number was built** - Anthropic's API has no endpoint
  that returns a remaining credit balance for any key type (confirmed
  against Anthropic's own docs). A "Check usage & balance" link to the
  Anthropic Console was added on the same card instead of fabricating one.
- **Finance -> Overview gained "New entry"/"New account" buttons**, opening
  the exact same modals already used on the Transactions/Accounts tabs.
- **The per-event "Attention" list was removed from Event Workspace**
  (Inventory Intelligence's own 2.2.6 block) - fully superseded by the
  Dashboard's global Attention Center. The backend it was built on is
  untouched, since the Attention Center itself still depends on it.
- **Attention Center (2.2.8) reworked to group by order.** The four
  ticket-level categories now emit one row per order instead of one row
  per ticket - marko's own example was a 49-ticket order shown as 49 rows.
  Clicking a grouped row opens that order's own page, which already lists
  every affected ticket with its own status/price/delivery indicators.
  `event_soon` is unchanged (still one row per event).
- **Seats display reformatted: the "/" is gone.** `formatSeatsSummary`
  (Orders/Tickets/Inventory/Sales/Pulls' "Seats" columns) now shows
  clearly labeled, separated Section/Row/Seat text (e.g.
  "Sec 402 · Row 56 · Seat 27") instead of a bare slash-joined pair, and
  eight duplicate ad-hoc "/" joins across Sales.tsx and EventDetail.tsx
  were consolidated into the same shared formatter.

Verified: `cargo test --lib` (999 passed, +4 net new tests, 0 failed),
`tsc -b` and `vite build` both clean.

## 2.2.8 - Dashboard global "Attention Center"

Focused task on top of 2.2.6/2.2.7: a new compact Dashboard block (Activity
tab) listing individual things across EVERY event that currently need a
look, grouped by priority (Critical/Attention/Info) and sorted by priority
then soonest event. See `REDESIGN-2.2.8-REPORT.md` for the full report
(Slovak) and `PROJECT_STATE/PROTECTED_AREAS.md`'s "2.2.8" entry for the
judgment calls behind it.

- **New backend command**: `get_attention_center`
  (`commands/attention_center.rs`, new file) - no migration, no new
  dependency.
- **Four of five categories reuse 2.2.6's exact per-event Inventory
  Intelligence "Attention" rules** (event within 2 days with unsold
  tickets, unsold ticket with no listing price, unsold ticket with no
  active listing, unsold ticket priced 20%+ off market average - only with
  real Price Checker data), flattened into individual clickable rows.
- **New fifth category**: sold ticket whose `delivery_status` isn't
  literally `"Delivered"` yet - reuses the exact convention 2.0.66's
  "Completed" indicator already established; a refund excludes itself
  automatically (ticket reverts to `available`).
- **Navigation**: reuses `Tickets.tsx`'s existing `?code=` deep link for
  ticket-level rows, and `/events/:id` for the one event-level category
  (`event_soon`) - no new route/navigation mechanism.
- **Display**: reuses the Activity tab's existing `ShowMoreToggle`/
  `RECENT_LIST_PREVIEW_COUNT` pattern per priority group - the backend
  never truncates.
- Deliberately does NOT touch the existing Dashboard alert bell/"Attention"
  cards (pulls/pending sales/missing listing price by order/upcoming
  events) - a separate, additional block, not a replacement.
- No automatic pricing/repricing anywhere; `tier`/`section`/`row` are never
  used as a pricing factor.
- **+10 new Rust unit tests** (event-soon in/out of window, unsold ticket
  without active listing, unsold ticket without listing price, market
  alert only with real Price Checker data, sold-undelivered fires/excludes
  delivered/excludes refunded, sold-undelivered priority window, sold-out
  event still flags undelivered tickets, same ticket under 2 categories
  never twice under 1, priority+date sort order). Full suite: 995 passed /
  0 failed / 3 ignored.

## 2.2.7 - Ticket metadata: Tier / Level

Focused task on top of 2.2.6: every ticket can now optionally carry a
tier/level value (e.g. "VIP", "Lower Bowl", "Level 200"), kept strictly
separate from `ticket_type` (a delivery method, not a price tier - the
same mix-up flagged twice before, now resolved for good). See
`REDESIGN-2.2.7-REPORT.md` for the full report (Slovak) and
`PROJECT_STATE/PROTECTED_AREAS.md`'s "2.2.7" entry for the judgment calls
behind it.

- **New column**: `tickets.tier TEXT`, nullable (`migrations/
  024_ticket_tier.sql`, forward-only). Every existing ticket got NULL - no
  guessed/inferred values, per marko's own explicit instruction.
- **Entry points**: set at order creation (`OrderFormModal`, copied onto
  every generated ticket, same as section/row) and editable per-ticket
  afterward (`TicketEditModal`) - both small, plain text fields, no
  redesign.
- **CSV**: import accepts `tier` (or `level` as a synonym); fully backward
  compatible with CSVs that predate this column. Export (tickets, sales,
  and the downloadable order-import template) all include `tier`, right
  after `row`.
- **Inventory Intelligence** (2.2.6) gained a real "By tier" breakdown -
  blank/null shows as "Unknown"; clicking a tier group filters the Tickets
  table exactly like the section/marketplace breakdowns already do.
- **Deliberately not wired up this round** (prepare-the-data, not
  wire-it-in-yet): Market Analysis / Repricing's `YourTicketGroup.tier`
  still always reports `None`; no bulk-tier-edit action; no tier column
  added to any list/table view; Google Sheets Order sync not wired to
  `tier`. Zero changes to refund/resell, `batch_id`, money/cents logic,
  Orders/Sales/Finance core logic, Listings, or Price Checker scraping.
- Tests: +13 new Rust unit tests (migration upgrade/fresh-db, ticket
  create/update with tier, CSV import old/new format + the `level`
  synonym, CSV export tier presence for tickets/sales/template, Inventory
  Intelligence tier grouping). Full suite: 985 passed / 0 failed / 3
  ignored. `tsc -b` and `npm run build` both clean.

## 2.2.6 - Inventory Intelligence for Event Workspace

Focused task on top of 2.2.5: a compact "Inventory Intelligence" block
added to the Event Workspace's Overview tab, above the existing Orders/
Tickets tables. See `REDESIGN-2.2.6-REPORT.md` for the full report
(Slovak) and `PROJECT_STATE/PROTECTED_AREAS.md`'s "2.2.6" entry for the
judgment calls behind it.

- **KPIs**: Total tickets, Total invested, Current listed value (active
  `ticket_listings` only), Potential profit (legacy `listing_price_cents`
  field, matching Sales' existing card), Sell-through %, Average ticket
  cost - all reusing existing money definitions, no new duplicate
  computations.
- **Aging**: 0-7 / 8-30 / 31-60 / 61+ days since purchase, unsold tickets
  only.
- **Attention**: event within 2 days with unsold stock, unsold ticket with
  no listing price, unsold ticket with no active listing, unsold ticket
  priced 20%+ off the market average (reuses Price Checker's own summary;
  shown as "not available yet" rather than a fake zero when this event has
  no Price Checker data).
- **Breakdown** by section and by marketplace. No "by tier" breakdown -
  `tickets` has no tier/level column anywhere in this schema; the UI says
  so in plain text instead of inventing fallback data, per marko's own
  explicit instruction.
- **Every row is clickable** - filters Overview's own Tickets table to the
  relevant tickets (or switches to the Listings tab, for "Current listed
  value"). No changes to Tickets.tsx, Orders.tsx, or any core Orders/
  Tickets/Sales/refund/resell logic; Finance page untouched.
- New backend: `commands/inventory_intelligence.rs` (1 new command,
  `get_inventory_intelligence`), no migration, no new dependency.
- Tests: +13 new Rust unit tests (KPI scope/formula parity with existing
  screens, aging bucket boundaries, attention-item independence and
  availability, currency-mixed handling, section/marketplace grouping).
  Full suite: 972 passed / 0 failed / 3 ignored. `tsc -b` and
  `npm run build` both clean.

## 2.2.5 - Event Workspace down to 3 tabs; Listings gets filters, search and bulk actions

Fourth pass on the Event Workspace, plus a Price Checker lookup addition.
Final tab order: `Overview | Listings | Sales`.

- **Sales absorbed Finance** - "sales a finance daj dokopy" (unambiguous
  this round). Finance's entries table now renders below Sales' own table
  (and below the Market section 2.2.4 already put there). See
  `PROJECT_STATE/PROTECTED_AREAS.md`'s "2.2.5" entry for the judgment call
  behind Sales (not Finance) surviving as the name.
- **Listings: filters, search, multi-select, bulk actions.** Status filter
  (All/Active/Sold/Removed), marketplace filter, search box, always-visible
  row checkboxes with select-all/deselect-all (scoped to the currently
  filtered/searched rows), and a bulk action bar (shown only while
  something is selected) for Edit status / Edit price / Delete - each
  backed by a new **all-or-nothing** transactional Rust command
  (`bulk_update_ticket_listings_status`, `bulk_update_ticket_listings_price`,
  `bulk_delete_ticket_listings`, all in `ticket_listings.rs`). Bulk price
  edit is refused, on both the frontend and the backend, when the selection
  spans more than one currency.
- **"Add listing" ticket picker rebuilt** as an order-browse flow (search
  this event's own orders, open one, pick tickets from it) mirroring
  Sales.tsx's own New Sale flow, replacing the old flat "every ticket in
  the event in one dropdown" picker. Several tickets can be picked at once,
  creating one listing per ticket on the chosen marketplace (per-ticket
  price, with a quick-fill/apply-to-all helper); Listing ID/URL are offered
  only when exactly one ticket is selected. This create flow is NOT
  all-or-nothing (unlike the 3 bulk actions above) - a partial failure
  keeps whatever succeeded and reports the rest for retry.
- **Marketplaces: added Seatriks** - new pure-data migration
  `023_add_seatriks_marketplace.sql`, no schema change.
- Existing tickets/inventory/sales/refund logic untouched; no automatic
  listing creation, marketplace API, or repricing added.
- Tests: +11 new Rust unit tests for the 3 bulk commands (selection
  scoping, invalid input, mixed currency, dedup, all-or-nothing transaction
  safety), plus 1 existing Price Checker test updated for the new 4th
  active marketplace.

## 2.2.4 - Event Workspace down to 4 tabs; Listings is now a real multi-marketplace system

Third pass on the Event Workspace. Final tab order: `Overview | Listings |
Sales | Finance`.

- **Overview absorbed Inventory** - the Orders/Tickets tables now render
  below Overview's own stat cards instead of having their own tab.
- **Sales absorbed Market** - "Market vs. mine" and "Potential Profit" (the
  former Market tab's content) now render below the Sales table. See
  `PROJECT_STATE/PROTECTED_AREAS.md`'s "2.2.4" entry for the judgment call
  behind Market landing in Sales rather than Finance.
- **Finance is unchanged**, still its own tab.
- **Listings rebuilt into a real system.** New `ticket_listings` table
  (`migrations/022_ticket_listings.sql`) - one ticket can now have several
  listings at once, one per marketplace (reuses the existing `marketplaces`
  lookup table), each with its own price/currency/status/listing id/URL/
  last-updated timestamp. Full add/edit/delete UI in the tab; summary cards
  count active listings only, the table shows every listing regardless of
  status. Manual entry only - no marketplace API, no automatic listing
  creation, no repricing. Never touches `tickets.status`/
  `tickets.listingPriceCents`.
- New backend: `commands/ticket_listings.rs` (4 commands: list-for-event,
  create, update, delete) + `commands::price_checker::
  delete_marketplace_impl`'s existing guard extended to also count
  `ticket_listings` (so deleting a marketplace with real listings against
  it is refused, same as it already was for saved links/price-check
  history).

See `PROJECT_STATE/PROTECTED_AREAS.md`'s "2.2.4" entry before extending any
of these tabs or the new table further.

`cargo test --lib` (948 passed, up from 934 - 14 new tests: 13 for
`ticket_listings`, 1 for the `delete_marketplace_impl` guard extension),
`tsc -b`/`vite build` clean. One new migration (022); no changes to
existing tickets/orders/sales/refund logic. Full detail in
`REDESIGN-2.2.4-REPORT.md`.

## 2.2.3 - Event Workspace: Listings tab, Tasks removed, tables full-width

Second pass on the Event Workspace, all frontend-only. Final tab order:
`Overview | Inventory | Listings | Sales | Market | Finance`.

- **Tasks tab removed entirely** - marko decided against it before it had
  a spec; it was only ever an empty placeholder, so there was nothing to
  migrate.
- **New Listings tab** - a read-only view of this event's tickets already
  filtered to `status === "listed"`: ticket, listing price, currency,
  status, plus an Active listings/Listed value/Lowest/Highest summary.
  Deliberately does NOT show marketplace, listing URL, or last checked -
  none of the three exist anywhere in the `tickets` schema (checked all 21
  migrations) or in Price Checker's own listing data, and marko explicitly
  asked not to invent data that isn't real. The tab says so plainly.
  Reuses the same `tickets` array Inventory already loads - no new API.
- **All 4 Event Workspace tables (Orders, Tickets, Sales, Finance) now
  fill the window width** - removed the `max-w-[1400px]` cap that was
  stopping them short of the right edge, the same fix `Layout.tsx` itself
  got in 2.0.31.

See `PROJECT_STATE/PROTECTED_AREAS.md`'s "2.2.3" entry before extending
any of these tabs further.

Frontend-only - no migration, no backend command changes. `tsc -b`/
`vite build` clean (cargo test suite unaffected - no `.rs` files touched
this release). Full detail in `REDESIGN-2.2.3-REPORT.md`.

## 2.2.2 - Event Workspace, plus 3 small fixes

`EventDetail.tsx` is now a tabbed "Event Workspace"
(`Overview | Inventory | Sales | Market | Finance | Tasks`, via the same
`TabSwitcher` Tickets.tsx/Events.tsx already use):

- **Overview** - exactly marko's own list (tickets, sold, available,
  total cost, revenue, profit, margin, ROI), no backend change.
- **Inventory** - the existing Orders + Tickets tables, unchanged, just
  relocated under their own tab.
- **Sales** - `list_sale_groups({ eventId })` (Sales.tsx's own Event
  filter, reused), compact table, "Open in Sales" for more.
- **Market** - `get_price_checker_summary(eventId)` (PriceChecker.tsx's
  summary command, reused) plus the existing "Potential Profit" block,
  now together in one tab.
- **Finance** - `list_finance_entries_for_order` (2.2.1) called once per
  this event's own orders, merged client-side - no new backend command.
- **Tasks** - honest placeholder (`EmptyState`), no spec given yet.

See `PROJECT_STATE/PROTECTED_AREAS.md`'s "2.2.2" entry before extending
any of these tabs.

Plus three unrelated small fixes: Settings -> Lookups' 3 category lists
no longer cap their scroll area at a fixed 224px (`max-h-[60vh]` now);
Event Detail's last table (Tickets) was missing the `mb-8` its Orders
neighbor had, so the Potential Profit box after it read as crammed
against it; Price Checker's event picker now only lists
`status === "upcoming"` events, so a completed/cancelled event quietly
stops showing up there.

Frontend-only - no migration, no backend command changes. `tsc -b`/
`vite build` clean (cargo test suite unaffected - no `.rs` files touched
this release). Full detail in `REDESIGN-2.2.2-REPORT.md`.

## 2.2.1 - Finance Accounts redesign, Settings Lookups redesign, Price Checker jump links, Finance-Orders linking

Four independent, marko-requested pieces in one release:

- **Finance Accounts** (`src/pages/finance/Accounts.tsx`): the old
  `sm:grid-cols-2 lg:grid-cols-3` grid of large `AccountCard`s replaced
  with one compact divide-y list (`AccountRow`) - same dense-row visual
  language as PlatformList/EventCategoryList. Balance stays the most
  prominent number per row; opening balance moved to a hover tooltip.
- **Settings -> Lookups** (`Settings.tsx`): was one long always-expanded
  Card (Platforms/Event categories/Finance categories); now exactly 3
  clickable summary rows (same row/chevron style as Settings Home's own
  section list), each opening its list(s) in a Modal. The add/delete
  functionality itself is unchanged - only the container is new.
- **Price Checker jump links**: "Check prices" button added to
  `OrderDetail.tsx` and `SaleDetail.tsx` (hidden on a sale group spanning
  mixed events), reusing the exact `navigate("/price-checker", { state: {
  presetEventId } })` pattern `EventDetail.tsx` already used since 2.0.81;
  `PriceChecker.tsx` needed no changes.
- **Finance <-> Orders linking**: `finance_entries.order_id` (new
  `migrations/021_finance_entry_order_link.sql`, `ON DELETE SET NULL`) -
  a deliberate, marko-confirmed (via question) reversal of ONE part of
  `015_finance.sql`'s original "fully independent ledger" design. New
  "Record in Finance" button/modal on `OrderDetail.tsx` pre-fills a new
  expense entry from the order's own amount/currency/date, with
  amount/currency locked read-only so the two numbers can never drift
  apart. New `list_finance_entries_for_order` command. See
  `PROJECT_STATE/PROTECTED_AREAS.md`'s "2.2.1" entry before touching
  `finance_entries.rs` again - in particular the "must round-trip
  `orderId` unchanged on edit" trap, already fixed proactively in
  `Transactions.tsx` and `Overview.tsx`.

6 new Rust tests. 934 passed / 0 failed / 3 ignored (up from 928), clippy
clean, `tsc -b`/`vite build` clean. Full detail, including the
AskUserQuestion decision on the Finance-Orders link design, in
`REDESIGN-2.2.1-REPORT.md`.

## 2.2.0 - Price Checker Market Analysis

New `commands/price_checker_analysis.rs` (2 commands) derives tier/section
price breakdowns, comparable-ticket ranking, and Your Tickets price
recommendations from a Visible Scanner session's already-accumulated
listings - never touches the scanner's own session/lifecycle code.
`migrations/019_price_checker_market_analysis.sql` adds `price_check_tiers`
so saved checks remember a per-tier breakdown going forward. 40 new Rust
unit/integration tests (incl. 2 added during this release's own adversarial
review pass, after finding tier/section grouping was case-sensitive while
comparable-matching already wasn't - see `PROJECT_STATE/PROTECTED_AREAS.md`'s
"2.2.0" entry). Full detail, including every flagged design decision, in
`PRICE-CHECKER-MARKET-ANALYSIS-2.2-REPORT.md`.

## 2.2.0 - StubHub fully removed, including history

`migrations/020_remove_stubhub.sql` deletes the StubHub marketplace row and
every `price_checks`/`price_check_tiers`/`event_marketplace_links` row that
ever referenced it - marko's own explicit, confirmed decision to go further
than 2.1.6's "keep history, stop offering it for new checks." Irreversible
by design.

## 2.1.9 - PROJECT_STATE protocol adopted

Set up `PROJECT_STATE/CURRENT_STATE.md`, `PROJECT_STATE/KNOWN_BUGS.md`,
and `PROJECT_STATE/PROTECTED_AREAS.md` (moved verbatim from the old root
`PROTECTED-AREAS-NOTES.md`, which is now a pointer stub) per marko's
development protocol. No code changes. `KNOWN_BUGS.md` starts empty by
design - see its own header for why.

## 2.1.9 - Price Checker Visible Scanner

Replaced the hidden auto-check WebView with a visible one the user scans
himself. Full detail in `PRICE-CHECKER-VISIBLE-SCANNER-REPORT.md` and
`PROJECT_STATE/PROTECTED_AREAS.md`'s "2.1.9" entry.

## 2.0.80 - Google Sheets Summary block: Paid-gated Revenue/Profit + refund staleness fix

- `plan_orders_summary_updates` (`orders_sheet_sync.rs`): "Total
  Revenue"/"Total Profit" now use the same Paid-gated `SUMPRODUCT` as
  "Total Paid", instead of summing every sold row regardless of payment
  status. Confirmed with marko via question before implementing.
- New `order_fully_refunded` check + clearing branch in
  `apply_sales_push_internal`: once every ticket on an order has been
  refunded, "Push sales"/"Fix sync" now blank that row's 7 Sales-sync
  columns instead of leaving stale pre-refund data forever (previously not
  even "Fix sync" could correct it - `uniform_sale_for_order` returns
  `None` for a refunded order, so nothing detected the drift).
- 9 new Rust tests. 747 passed / 0 failed / 3 ignored.

## 2.0.79 - Dashboard cleanup + CSV export staleness fixes

- Removed the Dashboard Overview tab's Quick Actions button row
  (New Event/Order/Sale, Import/Export CSV) - redundant with each page's
  own button and Settings -> Data.
- Orders/Tickets/Inventory/Sales CSV exports had drifted behind the data
  model over many versions; added the missing columns (event category;
  resale/delivery status; order code, seat location, margin, ROI,
  resale/delivery status, refund details).
- Dashboard Activity's "Unpaid payments" tile replaced with "Pulls near
  deadline" (pulls not yet transferred, event date approaching/past) -
  reuses Pulls.tsx's own existing warning window rather than a new rule.
  `unpaid_orders_count` itself is untouched (still used by notifications).
- 738 passed / 0 failed / 3 ignored.

## 2.0.78 - Pushover -> ntfy

- Swapped the Pushover notification channel for ntfy (no built-in app
  token needed).

## 2.0.77 - Notification simplification

- Removed the email notification channel entirely (SMTP config, `lettre`
  dependency) at marko's request.
- Pushover simplified to user-key-only; app token is now built into the
  binary via a GitHub Actions secret, same pattern as other embedded keys.
- 732 passed / 0 failed / 3 ignored (29 in the notifications module).

## 2.0.76 - Outbound notifications: desktop, email, Pushover

- New `commands/notifications.rs`: desktop (tauri-plugin-notification),
  email (SMTP via `lettre`), Pushover channels. Settings -> Notifications
  with a "Send test" button per channel.
- Background check every 30 min (+ once on launch) against the same 4
  Dashboard "Attention" categories the alert bell (2.0.75) already shows;
  max once per category per calendar day; upcoming-events only pushes
  within a 3-day window (vs. the bell's 14-day display window).
- Secrets stored plain-text in `app_settings`, same existing trust
  boundary as the rest of the app; never echoed back to the UI.
- Only fires while the app process is running - no tray/background
  service (documented limitation, not a bug).
- 729 passed / 0 failed / 3 ignored (26 new).

## 2.0.75 - Dashboard alert bell

- New `AlertBell` on the Dashboard (top-right, next to the tab switcher):
  badge counts how many of the same 4 "Attention" categories are non-zero;
  amber, red only when the soonest upcoming event is due today/overdue.
  Reuses the exact same numbers `DashboardAlerts` already computes - no
  new backend logic. Frontend-only change.
- 703 passed / 0 failed / 3 ignored (unchanged - no Rust touched).

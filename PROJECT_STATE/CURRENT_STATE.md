# TIQR Manager - Current State

Read this file first, every session, before touching anything. Then read
`KNOWN_BUGS.md` and `PROTECTED_AREAS.md` (same folder). Only after that,
open the specific files the current task actually needs - do not re-scan
the repo, and do not open the `REDESIGN-*-REPORT.md` / `*-REPORT.md`
history files at the repo root unless the current bug plausibly
originates there or marko points at one directly.

This file is the source of truth for "what is TIQR Manager right now."
Keep it current: whoever finishes a task updates this file as the last
step, before appending to `CHANGELOG.md`.

## What this is

TIQR Manager - a local-first, offline-by-default desktop app (Tauri 2 +
React/TypeScript frontend, Rust + SQLite backend) for ticket resellers.
No server of its own; the only outbound calls are the user's own machine
talking directly to Google Sheets, Firebase Auth, an FX-rate API, and (for
Price Checker) marketplace pages the user opens himself.

## Version

**2.3.4**, consistent across `package.json`, `src-tauri/tauri.conf.json`,
`src-tauri/Cargo.toml`, `release.ps1`'s `$Version`, and
`1-CLICK-UPDATE.bat` - see the version-bump checklist in
`PROTECTED_AREAS.md` ("2.1.6" entry) before ever bumping it by hand, there
are more places than the obvious 3 files. (2.2.6 was briefly shipped as an
un-bumped, code-labeled-only build first - marko's closing checklist for
that task didn't ask for a version bump - then bumped for real, same
session, once he confirmed he wanted the usual release file too. See
`PROTECTED_AREAS.md`'s "2.2.6" entry. 2.2.7 through 2.2.12's own closing
checklists all asked for the full cadence up front, no ambiguity. A
2.3.0 "Event Lifecycle" release was built and delivered after 2.2.12, then
fully reverted the same session at marko's request before he ever
published it - the CODE went back to exactly the 2.2.12 feature set, but
the version number did NOT go back to 2.2.12 - marko himself pointed out
that reusing an old version number breaks the auto-updater for anyone
already offered a newer one ("ked dam stary tak to nefunguje potom
dobre"), so this reverted build shipped as **2.3.1** instead. **Lesson for
future sessions: a revert-to-previous-behavior task still needs a version
bump FORWARD, never a rollback to a number already used before** - the
Tauri updater compares version numbers directly and won't offer/accept a
downgrade or a repeat. **2.3.2** is the next small, additive release right
after - see "Current focus" below and `CHANGELOG.md`'s own entries for
both.)

## Stack / layout

- **Frontend** (`src/`): React + TypeScript + Tailwind, Vite build.
  Pages under `src/pages/`: Dashboard, Events, EventDetail (2.2.2-2.2.5:
  tabbed "Event Workspace" - Overview/Listings/Sales (Finance folded into
  Sales in 2.2.5), see "Current focus" below and `PROTECTED_AREAS.md`'s
  "2.2.2"/"2.2.3"/"2.2.4"/"2.2.5" entries before adding more event-level
  functionality anywhere else), Orders,
  OrderDetail, Tickets (Inventory), Inventory, Sales, SaleDetail,
  FulfillmentCenter (2.2.12 - new; a narrower work-view over Sales' own
  data, see "Current focus" below), Pulls
  (given/received), Finance (own `finance/` subfolder, 4-tab layout),
  PriceChecker, Settings, Welcome (auth), PendingApproval, DatabaseError.
  Shared: `src/types.ts`, IPC in `src/lib/api.ts`, auth in
  `src/lib/auth.tsx`, money/date parsing helpers in `src/lib/`.
- **Backend** (`src-tauri/src/`): Rust, Tauri 2. One module per domain
  under `commands/`: events, orders (+ `orders_sheet_sync` - 2.2.10: same
  push-bookkeeping-ordering fix as `pulls_sheet_sync` below), tickets,
  ticket_listings (2.2.4 - real per-marketplace listings; 2.2.5 added 3
  all-or-nothing bulk commands - status/price/delete - see "Current focus"
  below), inventory_intelligence (2.2.6 - one read-only command backing
  Overview's "Inventory Intelligence" block, see "Current focus" below),
  attention_center (2.2.8 - one read-only command backing the Dashboard's
  global, cross-event "Attention Center" block; 2.2.9 reworked its 4
  ticket-level categories to group by order instead of one row per ticket;
  2.2.10 fixed its sort tie-break and excluded done events from 3 of its 5
  categories; 2.2.11 changed only how the FRONTEND groups/displays these
  same items - 5 clickable category boxes instead of a priority-grouped
  feed, see "Current focus" below - this module itself is byte-for-byte
  unchanged), sales, event_categories, pulls
  (+ `pulls_received`, `pulls_sheet_sync` - 2.2.10: push direction's
  `sheet_sync_links` bookkeeping now only committed after a confirmed
  network write, see "Current focus" below), finance_accounts/finance_entries (2.2.1: entries can
  optionally link to an Order via `order_id`)/finance_recurring/
  finance_forecast, price_checker (CRUD/marketplaces + saved-check
  history) + price_checker_scanner (the Visible Scanner session/commands)
  + price_checker_scan.js (injected extraction script) +
  price_checker_analysis (2.2.0 - Market Analysis: tier/section stats,
  comparable-ticket ranking, Your Tickets recommendations, all computed
  from a scanner session's already-accumulated listings, never a separate
  read), settings, backup, csv_import/csv_export, notifications,
  dashboard, currency, lookups, database, app_info, google_auth,
  firebase_google_auth. Shared modules at `src-tauri/src/`: `db.rs`
  (connection + migration runner), `models.rs`, `money.rs`, `finance.rs`,
  `fx.rs`, `google_oauth.rs`, `google_sheets.rs`.
- **DB**: SQLite via `rusqlite`, migrations in `src-tauri/migrations/`,
  currently through **025_deactivate_seatriks_price_checker.sql**. Migrations
  run automatically at startup, forward-only.
- **Packaging**: `release.ps1` (invoked via `1-CLICK-UPDATE.bat`) mirrors
  this folder into a fresh clone of the real GitHub repo, cross-checks the
  version in 3 files, commits, tags, and pushes - the tag push triggers
  the signed Windows installer build in GitHub Actions. Exclusion list for
  any manual zip matches `.gitignore` (node_modules, dist*, target, gen,
  logs, etc).

## Current focus / most recent work

**Orders/Sales sheet push - row placement now based on the marker column,
not raw row count (2.3.4, supersedes 2.3.3's first attempt).** Marko's own
report, investigated properly before touching anything, twice (see
`PROTECTED_AREAS.md`'s "2.3.2-2.3.4" entry for the full investigation
trail, both rounds of questions he was asked, and why the first fix wasn't
enough): Push Orders landed a new row at 426 instead of at row 18. 2.3.3's
first fix computed the target row from the raw `"A1:AZ"` read's length -
marko then sent a screenshot proving that wasn't enough: Revenue/Profit in
his real sheet were filled with live formulas all the way to row 425, even
though only ~16 rows have real order data - `plan_sheet_structure_updates`
had, at some point in the past, written formulas that far down, and since a
formula is non-empty content too, every subsequent raw row-count read
(the app's own, and Google's `append_values` auto-detection) agreed the
table was ~425 rows long. 2.3.3's fix trusted that same contaminated
number, so it reproduced the exact bug it was meant to close.

2.3.4 fixes this properly: `next_append_row`/`next_append_range`
(`orders_sheet_sync.rs`) now scan the sheet's own data for the LAST row
whose **marker cell** (TIQR ID) is non-empty - the one column only this
app ever writes, and only for a row holding a real pushed order - and
target the row right after it, completely ignoring any stray formula
residue further down. 5 unit tests cover this directly, including the
literal "16 real rows then 408 stray-formula rows" shape of marko's real
sheet. Two things this does NOT do, on purpose: it does not retroactively
move the order a past push stranded at row 426 (marko already deleted that
row's content himself while testing - see `PROTECTED_AREAS.md` for what
that means for that one order's sync state), and it does not clear the
stray formula residue in rows 18-425 (harmless, cosmetic, and only he
should decide whether to clean up live sheet content) - only he can safely
do either directly in the sheet.

The separate-sounding Revenue/Profit-formula complaint was very likely
never an independent bug - `plan_sheet_structure_updates` already
recomputes formulas across the sheet's entire current extent on every
push, and that extent has apparently included row 18 all along (thanks to
the very same formula contamination that caused the placement bug) - so
once a real order lands at row 18, the same push that places it there
should already give it a working formula too. **Still not marked resolved
until marko confirms on his real sheet.**

**Dashboard: all-time "Total cost" StatCard (2.3.2).** Marko's own request,
folded in alongside investigating two Google Sheets sync complaints (see
`PROTECTED_AREAS.md`'s "2.3.2" entry for those - still open, waiting on his
answer, no code changed for them yet): "aby som si vedel kuknut total cost
za vsetky listky ktore mam" (to be able to see total cost across all
tickets I have). Zero backend change - `DashboardData.inventory` (a
`FinanceSummary`) already carried `totalCostCents`/`currency` for the
Financials tab's existing "Current inventory (all time)" counts
(Available/Listed/Sold/Purchased), just never rendered. Added a 5th
StatCard right there ("Total cost", `formatMoneyOrMixed`), same section,
same all-time scope, grid widened `sm:grid-cols-4` -> `sm:grid-cols-3
lg:grid-cols-5` to fit it. **Do not confuse this with the Overview tab's
"Purchase cost" StatCard (`data.period.totalCostCents`)** - that one is
period-filtered AND scoped to sold-tickets-only cost (`cogs_cents`, per the
2.0.68 fix documented above `period_activity_summary` in `dashboard.rs` -
it exists so Revenue - Purchase cost = Profit reconciles for that period).
`data.inventory.totalCostCents` (this new card) is the opposite on both
axes: never period-filtered, and covers EVERY ticket regardless of status
(available/listed/sold/cancelled) - the true "everything I've ever spent"
figure. Two same-shaped fields, two deliberately different scopes - see
`PROTECTED_AREAS.md`'s "2.3.2" entry before ever "simplifying" these into
one.

**Event Lifecycle (2.3.0) was built, then fully reverted the same session -
read this before touching Events.tsx/EventDetail.tsx again.** Marko's
request right after 2.2.12 ("chcem, aby každý event mal jasne čitateľný
lifecycle / operational status") was designed and shipped in full - a
derived phase (`EventLifecyclePhase`/`computeEventLifecyclePhase`) on
Events/EventDetail, zero backend change. After seeing it, marko asked to
remove it entirely and go back to the previous version ("mi tam nieje
sympaticky... vrátime sa k tej minulej verzii") - no specific reason given
beyond not liking it. `Events.tsx`/`EventDetail.tsx` were both reverted,
edit-for-edit, back to their exact 2.2.12 content (never actually shipped
as 2.3.0 in the wild - marko reviewed the delivered zip/report before ever
running `1-CLICK-UPDATE.bat`). The version itself did NOT go back to
2.2.12 though - marko caught this himself right after: reusing an old
version number "nefunguje potom dobre" (breaks the auto-updater), so the
reverted code shipped as version **2.3.1** instead. See `CHANGELOG.md`'s
matching entry for the full original design (kept there as
history, not deleted, per this file's own "append-only" changelog
convention) - **if a similar "event status/phase" feature is ever
requested again, ask what specifically didn't work about this one before
reusing the same shape**, since no concrete complaint was captured this
time. `PROTECTED_AREAS.md`'s own 2.3.0 entry was removed (it only documented
traps inside code that no longer exists).

**Fulfillment Center - a new page for post-sale work (2.2.12).** Marko's
own ČASŤ C, shipped as its own release right after 2.2.11 (same message,
explicitly split into two releases). New `src/pages/FulfillmentCenter.tsx`
+ sidebar entry (`/fulfillment`, right after Sales) - zero backend changes,
zero migration, no parallel status system:

- **Data**: fetches the exact same `SaleGroup[]` Sales.tsx already fetches
  (`api.listSaleGroups({})`, no new command) and buckets it using
  `isSaleGroupDone` - imported directly from `Sales.tsx` (now exported, see
  its own 2.2.12 comment there) rather than reimplemented, so this page can
  never drift from Sales' own Pending/Completed rule. A group only ever
  appears here while `!isSaleGroupDone(g)` - a fully refunded group is
  "done" under that same existing rule and so never appears, exactly like
  Sales' own Pending tab.
- **4 tiles, doing double duty as both KPIs and category filters** (marko
  listed them twice, once as KPIs and once as categories - they're the same
  4 numbers): Pending Sales (= ALL PENDING, every not-done group), Awaiting
  Payment (= PAYMENT, `paidCount !== ticketCount`), Awaiting Delivery (=
  DELIVERY, `deliveredCount !== ticketCount`), Ready to Complete (= READY TO
  COMPLETE, both counts fully matched). Same clickable-tile visual pattern
  2.2.11 just established for Attention Center, reused rather than
  reinvented. Awaiting Payment/Awaiting Delivery are NOT mutually exclusive
  by design (a group missing both counts under both) - see "Current focus"
  below's 2.2.11 entry for the identical reasoning already applied there.
- **New concept, "Ready to complete"**: a pure display derivation
  (`isReadyToComplete`, exported), never a stored status - paid AND
  delivered in full. The only way such a group can still be Pending at all
  is a PARTIAL refund (`soldCount < ticketCount`, see that field's own doc
  comment) - so in practice this category means "just needs its remaining
  refund/resell bookkeeping looked at," never a group genuinely still
  missing payment or delivery.
- **Table**: Event / Ticket+Seats / Sale price / Payment status / Delivery
  status / Overall status / Action - the minimum marko asked for. Payment
  status reuses Sales.tsx's own Badge pattern verbatim; Delivery status is
  a new group-level badge (`deliveredCount`/`ticketCount`) reusing the
  existing `delivered`/"not delivered"/`mixed` tone keys `ui.tsx` already
  defines (used today by the per-TICKET `InlineStatusSelect` on Sale/Order
  Detail) - no new color. Overall status shows "Ready to complete" (emerald,
  same `completed` tone Sales.tsx's own Completed badge uses) or "Pending"
  (amber) - never "Completed", since a truly completed group can never
  reach this page. Clicking a row OR its "Open" Action button both navigate
  to the existing `/sales/:id` route (`SaleDetail.tsx`) - no new navigation
  mechanism.
- **Verification**: this codebase has no frontend test framework (confirmed
  by grep - no vitest/jest/*.test.* anywhere), so frontend-only logic here
  was verified the same way `isEventDone`/`isOrderDone`/`isSaleGroupDone`
  always have been in every prior release - by `tsc -b`, code-reading, and
  reasoning - PLUS, this time, a disposable esbuild-bundled Node script
  (built and run once during this task, then deleted - never part of the
  repo) that imported the REAL exported `isSaleGroupDone`/
  `isReadyToComplete`/`matchesFulfillmentCategory` and asserted all of
  marko's explicit test scenarios (payment-pending, delivery-pending, both
  pending, ready-to-complete, a fully-done group excluded from Pending, the
  refund rule) - 21/21 passed. See `REDESIGN-2.2.12-REPORT.md` for the exact
  scenarios and `PROTECTED_AREAS.md`'s new "2.2.12" entry for the full
  reasoning.
- `cargo test --lib`: 1006 passed, 0 failed - unchanged from 2.2.11, since
  no `.rs` file was touched for this release either.

**Attention Center UX rework + Dashboard cleanup (2.2.11).** Marko's own
next request after 2.2.10, split into two explicit parts, both frontend-only
- no migration, no new command, no backend change of any kind:

1. **Attention Center: from one mixed feed to 5 named, always-visible
   boxes.** `Dashboard.tsx`'s `AttentionCenterBlock` no longer groups
   `AttentionCenterItem[]` by `priority` (the old Critical/Attention/Info
   feed, `ATTENTION_CENTER_GROUPS`/`AttentionCenterGroup` - both removed).
   It now groups by the item's existing `category` field into exactly the 5
   boxes marko named, in his exact order: **NO LISTING PRICE YET** (
   `missing_listing_price`), **NO ACTIVE LISTING** (`no_active_listing`),
   **NOT DELIVERED YET** (`sold_undelivered`), **EVENT COMING SOON**
   (`event_soon`), **MARKET ATTENTION** (`outside_market_price`). Each box
   (`AttentionCategoryCard`, same label/value/sub visual language as
   `ui.tsx`'s `StatCard`) shows a title, a count, and a short static
   subtext, and is a real `<button>` - clicking one selects it and reveals
   its own rows below (reusing `AttentionCenterRow` byte-for-byte
   unchanged); clicking the same box again, or its detail panel's "Close",
   collapses it. Only ONE category's rows show at a time, and the mixed
   feed is completely gone as default/main content - exactly marko's
   "Žiadny veľký mixed feed ako hlavný obsah... nech sa zobrazí až po
   výbere konkrétnej kategórie." A box with 0 items is disabled (nothing to
   drill into) rather than hidden - all 5 stay visible always, so the 5
   lenses are always visible even at zero. Judgment call: a box's "item
   count" is the number of Attention Center ROWS in that category (an order
   with 40 unpriced tickets is still 1 row, same grouping 2.2.9 already
   established) not a raw ticket count - consistent with the existing
   per-row-already-a-group convention, not a new one. **MARKET ATTENTION
   required zero backend work**: confirmed by reading (not assuming)
   `attention_center.rs`'s own module doc comment and its
   `outside_market_price_only_fires_when_price_checker_data_exists_for_
   that_event` test - that arm already only fires when real Price Checker
   data exists for the event, never determines a price, and the whole
   module's doc comment already guarantees `tier`/`section`/`row` are never
   read as a pricing factor anywhere in it. All of marko's MARKET ATTENTION
   constraints were already true before this task started. The older
   `AttentionSection`/`AlertCard`/alert bell block (2.0.75/2.0.76/2.0.79,
   further down the same Activity tab) is completely untouched - it's a
   different, already-shipped feature (see `attention_center.rs`'s own doc
   comment for why both exist) and wasn't named in this request.
2. **Dashboard Overview: unbounded platform list capped, plus a small
   spacing trim.** `SalesByPlatformCard`'s `<ul>` was the one list on the
   Overview tab with no size limit at all - a business with many distinct
   platforms would previously push the whole tab (and the page's own
   scrollbar) further down for every additional one. It's now
   `max-h-72 overflow-y-auto` - a typical handful of platforms still shows
   in full with no scrollbar anywhere, and only a genuinely long list gets
   an internal scrollbar of its own, never the page's. Paired with a
   modest, one-step trim of two existing Tailwind spacing values on the
   same tab (the StatCard grid's `mb-6`->`mb-5`, the metric chart Card's
   `mb-8`->`mb-6`) - not a redesign, every component/layout is unchanged,
   just a little less vertical whitespace before "Sales by platform".
   `Layout.tsx`'s `<main className="overflow-y-auto">` was checked and is
   already correct (it only ever scrolls when content actually overflows) -
   no change needed or made there. Judgment call, stated plainly in the
   report: this sandbox cannot reproduce a real browser's scrollbar at a
   specific OS/display scaling, so the unbounded list was identified as the
   concrete, well-reasoned root cause (the only vector for indefinite
   growth on this tab) rather than confirmed via a literal reproduction -
   see `REDESIGN-2.2.11-REPORT.md` for the full reasoning and what to check
   if the scrollbar still appears on marko's own machine.

Verified: `cargo test --lib` (1006 passed, 0 failed, 0 new/changed - no
Rust file touched this release, confirming zero regressions), `tsc -b` and
`vite build` both clean. See `REDESIGN-2.2.11-REPORT.md` for the full
report (Slovak) and `PROTECTED_AREAS.md`'s new "2.2.11" entry for every
judgment call above.

**Eight follow-up fixes from marko's review of 2.2.9 (2.2.10).** Marko sent
two rapid-fire messages (7 screenshots combined) after 2.2.9 shipped. No
migration this release - purely query/logic/frontend changes:

1. **Seats format lost its labels again.** `formatSeatLocation`/
   `formatSeatsSummary` (`lib/format.ts`) now join bare values with " · "
   ("402 · 56 · 27") instead of 2.2.9's "Sec 402 · Row 56 · Seat 27" - a
   real section value is sometimes already a full label on its own ("Sec
   408", "Category D, Standing"), and the added prefix produced visible
   duplication ("Sec Sec 408"). Reaches every "Seats" column app-wide via
   the same two shared helpers, no per-page changes needed.
2. **Orders tabs reworked: "Active"/"Paid" -> "Active"/"Completed"**, with a
   real bucketing change. `isOrderDone` (`Orders.tsx`) now marks an order
   Completed once EITHER its event is done (`isEventDone` - status
   completed/cancelled, OR its date has already passed - deliberately an OR
   of both signals, not status alone, since `events.status` has no
   automatic date-based transition anywhere in this codebase) OR the order
   itself is fully sold+delivered+paid (reusing the existing completion-
   badge machinery). `Order` gained `eventDate`/`eventStatus` as a
   read-time join, no migration.
3. **New Order's event picker now excludes those same "done" events too** -
   previously unfiltered, so a purchase could be logged against an event
   that had already happened or was already marked completed.
4. **Attention Center's "mixed" ordering fixed** - root cause was the
   sort's own tie-break (grouping same-priority rows by CATEGORY NAME
   before order), not the 2.2.9 group-by-order logic itself, which was and
   remains correct. Also now excludes done events (same status-or-date
   check as above) from 3 of its 5 categories (missing listing price/no
   active listing/outside market price) - `sold_undelivered` and
   `event_soon` are deliberately exempt, see `PROTECTED_AREAS.md`.
5. **Sales Pending/Completed now requires sold+delivered+paid together (or
   fully refunded)**, not payment status alone - a sale missing only its
   delivery status no longer incorrectly showed as Completed.
6. **Two confirmed Google Sheets push bugs fixed** (`orders_sheet_sync.rs`/
   `pulls_sheet_sync.rs`): the local `sheet_sync_links` "already synced"
   bookkeeping was being written BEFORE the actual network write it
   described had even been attempted - a failed push still silently looked
   successful afterward, permanently. Both push paths now record success
   only once the matching `append_values`/`update_values` call is confirmed
   to have succeeded. `sales_sheet_sync`'s own push was checked and
   confirmed unaffected (it performs no DB writes of its own).
7. **Google's `invalid_grant` sign-in error now shows a short "sign in
   again" message** instead of a long raw JSON dump (`describe_error_response`,
   `google_sheets.rs`, shared with `google_oauth.rs`'s token refresh) -
   best-effort fix for marko's reported long error after Google sign-in;
   NOT independently reproducible in this environment (no live Google OAuth
   access here) - see the report for what to do if it recurs.
8. **Native right-click context menu disabled app-wide** (`main.tsx`) - no
   config flag exists for this in Tauri/WRY, so this is the standard
   JS-side `contextmenu` + `preventDefault` fix, not a workaround.

Verified: `cargo test --lib` (1006 passed, +7 net new tests, 0 failed),
`tsc -b` and `vite build` both clean. See `REDESIGN-2.2.10-REPORT.md` for
the full report (Slovak) and `PROTECTED_AREAS.md`'s new "2.2.10" entry for
every judgment call above, especially item 2's status-or-date `isEventDone`
formula and item 4's exact exemptions.

**Six follow-up fixes from marko's review of 2.2.8 (2.2.9), plus a rework
of the Attention Center itself.** Marko reviewed the just-shipped 2.2.8
result (screenshots + a rapid-fire message) and asked for six mostly-
unrelated changes:

1. **Seatriks retired from Price Checker only.** `marketplaces.active = 0`
   for Seatriks (`migrations/025_deactivate_seatriks_price_checker.sql`),
   the exact same mechanism already used to retire StubHub
   (`017_price_checker_viagogo.sql`) - it stops appearing as a fresh option
   in `get_price_checker_summary_impl`'s marketplace query, but stays fully
   selectable in Listings' "Add listing" picker/filter (`list_marketplaces`
   is unfiltered by `active`). Judgment call: like StubHub, an event that
   already has a saved Seatriks link/check would still show it there - an
   unconditional cut with zero exceptions was NOT built; see
   `PROTECTED_AREAS.md`'s "2.2.9" entry.
2. **Settings -> Integrations' Anthropic API key card renamed.** "AI-
   assisted price reading" -> the general "AI features" (`Settings.tsx`'s
   `AnthropicApiKeyCard`) - marko's own request, since the same stored key
   is meant to power more than one AI feature over time, not just Price
   Checker's reading fallback. No backend/storage change at all.
3. **No live "balance" indicator was built.** Marko asked for a small
   balance/usage indicator near the API key card. Checked first: Anthropic's
   API has no endpoint that returns a remaining credit balance for ANY key
   type - the closest thing (the Usage & Cost API) only returns historical
   token/cost figures, and even that requires an Admin API key or an
   unscoped personal/service key; a workspace key like the one this app
   stores explicitly does not work for it. Rather than fake a number or ask
   marko to also generate a materially more sensitive key, the card just
   links straight to `console.anthropic.com/settings/billing` (opened via
   the already-present `@tauri-apps/plugin-opener`/`tauri-plugin-opener`,
   same dependency `google_oauth.rs` already uses for the sign-in browser
   flow - no new dependency).
4. **Finance -> Overview gained "New entry"/"New account" quick-action
   buttons.** Reuses the exact same `EntryFormModal`/`AccountFormModal`
   already on the Transactions/Accounts tabs (now exported, not
   duplicated) - no new form, no new backend command.
5. **The per-event "Attention" list was deleted from Event Workspace.**
   Only the "Attention" rows inside `InventoryIntelligenceBlock`
   (`EventDetail.tsx`'s Overview tab) - the ATTENTION_COPY-labeled rows
   showing event-soon/missing-price/no-listing/off-market counts - marko's
   own "tuto attention cast celu vymazat s events". The KPIs/Aging/By-tier/
   section/marketplace breakdowns in that same card are UNCHANGED, and
   critically, the BACKEND command/impl behind it
   (`get_inventory_intelligence`) is untouched - the Dashboard's own
   Attention Center (below) calls that same impl function directly and
   still depends on it.
6. **Dashboard Attention Center (2.2.8) reworked to group by order.**
   Marko's screenshot showed one order's 49 tickets, all missing a listing
   price, rendered as 49 separate rows - his own words, "nedáva zmysel"
   (doesn't make sense). The four ticket-level categories
   (`missing_listing_price`/`no_active_listing`/`outside_market_price`/
   `sold_undelivered`) now group their flagged tickets by `order_id` first,
   emitting one row per (event, category, order) with `ticketIds`/
   `ticketCodes` carrying every ticket the row stands for. Clicking a
   grouped row now opens that order's own page (`/orders/:id`,
   `OrderDetail.tsx`) - which already lists every one of those tickets with
   its own status/listing price/delivery indicators - instead of a single
   ticket's `?code=` deep link. `event_soon` is UNCHANGED (still one row
   per event, `orderId: null`) - it has no single order to group under,
   since a soon event's unsold tickets can span more than one order. See
   `PROTECTED_AREAS.md`'s "2.2.9" entry for the full design and the new
   `AttentionCenterItem` shape.
7. **Seats display reformatted everywhere - the "/" is gone.** The shared
   `formatSeatsSummary` (`src/lib/format.ts`), used by Orders/Tickets/
   Inventory/Sales/Pulls' "Seats" columns, used to join section+row with a
   bare "/" (e.g. "402/56 27"). It now reuses `formatSeatLocation`'s own
   labeled, dot-separated convention instead (e.g.
   "Sec 402 · Row 56 · Seat 27"). Six duplicate ad-hoc "/" joins on the
   Event Workspace page (`EventDetail.tsx`) and two more on Sales.tsx's own
   Create Sale modal were consolidated into the same shared
   `formatSeatLocation` call rather than patched individually. Purely a
   frontend display change - the backend already sends section/row/seat as
   separate fields (`SeatEntry`), nothing pre-joins with "/" on the wire.

Verified: `cargo test --lib` (999 passed, +4 net new tests, 0 failed - one
pre-existing hardcoded-migration-count canary and one hardcoded active-
marketplace-list test were updated for the new migration/Seatriks change,
not weakened), `tsc -b` and `vite build` both clean. See
`REDESIGN-2.2.9-REPORT.md` for the full report (Slovak) and
`PROTECTED_AREAS.md`'s new "2.2.9" entry for every judgment call above.

**Dashboard gained a global "Attention Center" (2.2.8).** A new compact
block on the Dashboard's Activity tab, above the existing "Attention"
cards/alert bell (2.0.75/2.0.76/2.0.79 - untouched), backed by one new
read-only command (`get_attention_center`, `commands/attention_center.rs`)
that lists INDIVIDUAL things needing a look across EVERY event, not just
counts:
- **Four of its five categories are the exact per-event Inventory
  Intelligence "Attention" rules (2.2.6) reused as-is**, just flattened
  into per-ticket rows instead of per-event counts: event within 2 days
  with unsold tickets (one row per EVENT, not per ticket - see the
  module's own doc comment for why), unsold ticket with no listing price,
  unsold ticket with no active listing, and unsold ticket priced 20%+ off
  the market average (only ever shown when this event already has real
  Price Checker data - never invented). Nothing here can drift from the
  Event Workspace's own Inventory Intelligence block, since it's the same
  function call, not a second implementation.
- **A new fifth category: sold, delivery not marked complete.** Reuses the
  exact `delivery_status = 'Delivered'` convention the 2.0.66 "Completed"
  indicator already established (`orders.rs`/`sales.rs`'s own
  `delivered_count`) - a refund reverts a ticket's status back to
  `available`, so a refunded ticket drops out automatically, never a guess.
- **Priority grouping**: Critical / Attention / Info (a new concept - see
  `PROTECTED_AREAS.md`'s "2.2.8" entry for the exact category-to-tier
  mapping and why). Sorted by priority, then soonest event.
- **Navigation**: a row with a ticket links to the existing Tickets `?code=`
  deep link (the one cross-page ticket link this app already has); an
  event-level row (event-soon) links to that event's own Event Workspace.
  No new route, no new navigation mechanism.
- **Display**: grouped by priority, each group capped at a preview count
  with the same "Show N more" toggle the Activity tab's Recent cards
  already use - the backend itself never truncates, so "Show all" never
  loses data.
- No new migration, no new dependency, no automatic pricing/repricing -
  every value shown is a value that already exists verbatim on the ticket.
See `PROTECTED_AREAS.md`'s new "2.2.8" entry before touching this again,
in particular the priority-tier mapping and the event-level-vs-ticket-level
granularity judgment calls.

**Event Workspace (2.2.2, revised 2.2.3, 2.2.4 and 2.2.5).**
`EventDetail.tsx` is a tabbed "Event Workspace" (`TabSwitcher`, same
component Tickets.tsx/Events.tsx already use for their own tabs) - current,
final shape: **Overview | Listings | Sales** (down from 4 tabs in 2.2.4 -
Finance folded into Sales this round):
- **Overview** shows marko's own stat list (tickets, sold, available,
  total cost, revenue, profit, margin, ROI - `EventWithStats.stats`, no
  backend change), plus (2.2.4) the Orders + Tickets tables that used to
  be their own "Inventory" tab, appended below - marko's own "spoj do
  jedneho" (merge into one) instruction. Both halves are unchanged from
  their previous tabs, just relocated into one.
- **Listings** (2.2.3: read-only Ticket view; rebuilt in 2.2.4 into a real
  system; **2.2.5 made it genuinely manageable at volume**) is backed by a
  real `ticket_listings` table (`migrations/022_ticket_listings.sql`,
  `commands/ticket_listings.rs`) - one ticket can have several listings at
  once, one per marketplace (StubHub-successor Viagogo/Vivid Seats/
  Ticombo/Seatriks-style, see below), each with its own price/currency/
  status/listing id/listing URL/last-updated timestamp. Reuses the
  EXISTING `marketplaces` lookup table (Price Checker's own) rather than a
  second marketplace concept. Summary cards (Active listings/Listed
  value/Lowest/Highest) count `status === "active"` listings only and are
  never affected by the filters below; the table shows every listing
  matching the current filters regardless of status. Deliberately still
  manual entry only - no marketplace API, no automatic listing creation,
  no repricing. Does NOT touch `tickets.status`/`tickets.listingPriceCents`
  at all - those stay exactly what they were.
  2.2.5 additions, all client-side except the 3 new bulk commands: a
  status filter (All/Active/Sold/Removed), a marketplace filter, a search
  box (ticket/marketplace/listing id/URL), always-visible row checkboxes +
  select-all (scoped to the currently filtered/searched rows, same
  convention as Sales.tsx's own bulk-select), and a bulk action bar (shown
  only while something is selected) covering Edit status / Edit price /
  Delete - all three backed by new **all-or-nothing** transactional
  commands (`bulk_update_ticket_listings_status`/`_price`,
  `bulk_delete_ticket_listings`); bulk price edit is refused (frontend AND
  backend) when the selection spans more than one currency. "Add listing"
  also got a new ticket picker - browse this event's own orders
  (searchable), open one, pick tickets from it, repeat across orders,
  mirroring Sales.tsx's own New Sale flow - replacing the old flat
  "every ticket in the event in one dropdown" picker marko found opaque.
  Several tickets can be selected at once, creating one listing per ticket
  on the chosen marketplace (price editable per ticket via a
  Quick-fill-and-override grid, same UX as New Sale's own price/fees
  grid); Listing ID/URL are offered only when exactly one ticket is
  selected, since each marketplace posting has its own. See
  `PROTECTED_AREAS.md`'s "2.2.4" and "2.2.5" entries before touching this
  table, its bulk commands, or `delete_marketplace_impl`'s guard again.
- **Sales** calls `list_sale_groups({ eventId })` (Sales.tsx's own Event
  filter, reused) for its own table, plus (2.2.4) the former **Market**
  tab's content - "Market vs. mine" (`get_price_checker_summary`) and
  "Potential Profit" - and (2.2.5) the former **Finance** tab's content -
  every Finance entry linked to one of this event's own Orders
  (`list_finance_entries_for_order`, 2.2.1) - both appended below the
  Sales table in that order. Market's and Finance's own tabs/names are
  gone - see `PROTECTED_AREAS.md`'s "2.2.4"/"2.2.5" entries for the
  Sales-survives judgment calls behind both merges (flagged to marko).
  "Open in Sales"/"Open in Price Checker"/"Open in Finance" still link out
  to the real, standalone sections for anything more than a glance.

**Ticket metadata: Tier / Level (2.2.7).** Every ticket can now optionally
carry a `tier`/level value (e.g. "VIP", "Lower Bowl", "Level 200"), a new
nullable `tickets.tier TEXT` column (`migrations/024_ticket_tier.sql`,
forward-only, no backfill - every existing ticket got NULL). Deliberately
a SEPARATE field from `ticket_type` (a DELIVERY method - E-ticket/PDF/
Mobile transfer/Physical/Will call - not a price tier; this exact
confusion was already flagged twice before, see `PROTECTED_AREAS.md`'s
"2.2.0" and "2.2.6" entries, now resolved for good by this task's own
"2.2.7" entry there).
- **Entry points**: since this app has no standalone "Add Ticket" flow
  (tickets are only ever created via an Order), `tier` is set at order
  creation (`OrderFormModal`, copied onto every generated ticket, same as
  `section`/`row_label`) and editable afterward per-ticket
  (`TicketEditModal`). Both are small, plain text fields - no redesign.
- **CSV**: import accepts a `tier` column (or `level` as a synonym);
  entirely absent from an older CSV imports exactly as before (no
  separate "old format" code path needed). Export (tickets, sales, and the
  downloadable order-import template) all include `tier`, positioned
  right after `row` in every header.
- **Inventory Intelligence** (2.2.6, above) gained a real "By tier"
  breakdown - see that section's own updated bullet below. Clicking a
  tier group filters the Tickets table exactly like the section/
  marketplace breakdowns already do.
- **Deliberately NOT done this round** (marko's own "prepare the data,
  don't wire it in yet" instruction): Market Analysis / Repricing
  (`price_checker_analysis.rs`'s `YourTicketGroup.tier`) still always
  reports `None` - the real column exists now, but nothing reads it there
  yet. No bulk-tier-edit action. No tier column added to any list/table
  view (Tickets/OrderDetail/Sales/SaleDetail - section/row aren't shown
  as columns there either, so this is consistent). Google Sheets Order
  sync is not wired to `tier` - no sheet column exists for it.
  Refund/resell, `batch_id`, money/cents logic, Orders/Sales/Finance core
  logic, Listings, and Price Checker scraping are all completely
  untouched.
See `PROTECTED_AREAS.md`'s "2.2.7" entry before touching this column
again, in particular the column-order convention and the CSV-export test
index-shift trap.

**Overview gained an "Inventory Intelligence" block (2.2.6).** A compact
block rendered above the Orders/Tickets
tables on the Overview tab, backed by one new read-only command
(`get_inventory_intelligence`, `commands/inventory_intelligence.rs`) that
reuses existing definitions rather than inventing new money logic:
- **KPIs**: Total tickets / Total invested (same scope as
  `finance::compute_summary`, all tickets including cancelled), Current
  listed value (sum of ACTIVE `ticket_listings.price_cents`, matching
  Listings' own "Listed value"), Potential profit (byte-for-byte Sales'
  own existing legacy-field formula, `tickets.listing_price_cents`),
  Sell-through % (sold / total including cancelled - matches the "Total
  tickets" denominator shown next to it), Average ticket cost.
- **Aging** (unsold tickets only, by days since order purchase date): 0-7 /
  8-30 / 31-60 / 61+ (marko's own spec had an overlapping 8-30/30-60 -
  resolved to 31-60, flagged in the module's doc comment).
- **Attention**: event within 2 days with unsold stock (marko said "48h" -
  translated to whole calendar days since `event_date` has no time
  component anywhere in this schema), unsold ticket with no listing price,
  unsold ticket with no active listing, and unsold ticket priced >=20% off
  the market average - this last one reuses
  `commands::price_checker::get_price_checker_summary_impl` (the same
  function Sales' own "Market vs. mine" card calls) and is explicitly
  `available: false` (not a fake zero) when this event has no Price
  Checker data yet.
- **Breakdown** by tier, by section, and by marketplace (the first two
  scoped to unsold tickets, the last to active listings). Tier grouping
  was added in 2.2.7 (`tickets.tier`, see above) - blank/null groups as
  "Unknown", deliberately different wording from the section breakdown's
  own "No section".
- **Every KPI/aging/attention/breakdown row is clickable** - filters
  Overview's own already-rendered Tickets table down to just those ticket
  ids (backend returns `ticketIds`/`unsoldTicketIds`/`soldTicketIds` lists,
  frontend filters its own already-fetched `Ticket[]` by id membership; no
  new page, no new fetch, no new predicate logic in TypeScript), except
  "Current listed value" which switches to the existing Listings tab
  instead (that number is fundamentally about `ticket_listings` rows, not
  raw tickets). Neither Tickets.tsx nor Orders.tsx gained any new
  filtering - this stays entirely inside `EventDetail.tsx`.
- **Does not touch** refund/resell, `batch_id`, Orders/Tickets/Sales core
  logic, or the Finance page - read-only aggregation of already-existing
  data, no new migration, no new dependency.
See `PROTECTED_AREAS.md`'s new entry before touching this module again,
in particular the two numeric judgment calls (2-day event-soon window,
20% off-market threshold) and the dual listing-value-system nuance.

**Marketplaces: Seatriks added (2.2.5).** `migrations/
023_add_seatriks_marketplace.sql` seeds a 4th row in the shared
`marketplaces` lookup (marko's own request) - pure data, no schema change,
same precedent as 020_remove_stubhub.sql. Available immediately in both
Price Checker and the Listings "Add listing" marketplace picker.

2.2.3 removed the **Tasks** tab entirely (marko decided against it before
it ever got a spec) and removed the `max-w-[1400px]` cap from this page's
tables so they fill the window width (2.0.31's `Layout.tsx` fix, extended
here). 2.2.2 also shipped three unrelated small fixes: Settings -> Lookups'
3 category lists no longer cap their scroll area at a fixed 224px
(`max-h-[60vh]` now); Price Checker's event picker only lists
`status === "upcoming"` events (same field Events.tsx's own Upcoming/
Completed tabs use) - a completed/cancelled event just quietly stops
showing up there, no manual untracking needed.

Read `PROTECTED_AREAS.md`'s "2.2.2"/"2.2.3"/"2.2.4"/"2.2.5" entries before
adding to any of these tabs. 2.2.2/2.2.3 were frontend-only; 2.2.4 added
one new table + 4 commands (`ticket_listings`) and extended
`delete_marketplace_impl`'s existing guard; 2.2.5 added 3 more commands
(the bulk actions) plus one pure-data migration (Seatriks) - no other
backend surface changed either round.

**Finance <-> Orders link, Finance Accounts/Lookups UI simplification,
Price Checker jump links (2.2.1).** Four independent, marko-requested
pieces in one release:
- Finance Accounts (`src/pages/finance/Accounts.tsx`) - the old
  `sm:grid-cols-2 lg:grid-cols-3` grid of large `AccountCard`s is now one
  compact divide-y list (`AccountRow`), same dense-row visual language as
  PlatformList/EventCategoryList and the Recurring expenses table. Balance
  is still the most prominent number per row; opening balance moved to a
  hover tooltip.
- Settings -> Lookups (`Settings.tsx`) - was one long Card with Platforms/
  Event categories/Finance categories always expanded; now exactly 3
  clickable summary rows (same row/chevron style as Settings Home's own
  list), each opening its list(s) in a Modal. The add/delete functionality
  itself (`PlatformList`/`EventCategoryList`/`FinanceCategoryList`) is
  unchanged - only the container is new.
- "Check prices" jump into Price Checker, added to `OrderDetail.tsx` and
  `SaleDetail.tsx` (hidden there when a sale group spans mixed events) -
  same `navigate("/price-checker", { state: { presetEventId } })` pattern
  `EventDetail.tsx` already used since 2.0.81; `PriceChecker.tsx` already
  read `location.state.presetEventId` and needed no changes.
- Finance entries can now optionally link to an Order (`order_id`, new
  `migrations/021_finance_entry_order_link.sql`, `ON DELETE SET NULL` -
  same convention as `category_id`/`account_id`). A deliberate, marko-
  confirmed reversal of one part of `015_finance.sql`'s original "fully
  independent ledger" design - see `PROTECTED_AREAS.md`'s new entry before
  touching `finance_entries.rs` again. `OrderDetail.tsx` has a new "Record
  in Finance" button/modal that pre-fills a new expense entry from the
  order's own `total_cost_cents`/`currency`/`purchase_date` (amount/
  currency are read-only in that modal - the whole point is the two
  numbers can never drift apart) and shows whether the order has already
  been recorded. `list_finance_entries_for_order` is the one new command
  this needed.

**Price Checker Market Analysis, built on top of the Visible Scanner
(2.2.0).** New `commands/price_checker_analysis.rs` module (2 Tauri
commands: `compute_market_analysis`, `compute_comparable_market`) reads a
scanner session's already-accumulated `NormalizedListing`s and derives,
without ever touching the scanner's own session/lifecycle code: tier and
section price breakdowns per currency, a "comparable market" ranking
against one reference ticket (exact/close/tier/general, marko's own
priority order), price recommendations for marko's own unsold inventory
("Your Tickets", reusing the real `tickets` table - never a duplicate of
it), and a market overview. `migrations/019_price_checker_market_
analysis.sql` adds `price_check_tiers` so a saved check can also remember
its per-tier lowest/median/count going forward. Full design, all flagged
design decisions (the required `ComparableReferenceInput.currency`
addition, the two independent `data_quality`/`level` classifications,
`YourTicketGroup.tier` always being `None`, etc.), and the REAL/DERIVED/
UNAVAILABLE data split are in `PRICE-CHECKER-MARKET-ANALYSIS-2.2-REPORT.md`.
Read `PROTECTED_AREAS.md`'s "2.2.0" entry before touching this module
again - in particular the tier/section grouping case-sensitivity trap.

**StubHub fully removed, including all history (2.2.0), on top of the
2.1.6 partial retirement.** `migrations/020_remove_stubhub.sql` deletes
the `marketplaces` row and every `price_checks`/`price_check_tiers`/
`event_marketplace_links` row that ever referenced it - marko's own
explicit, confirmed decision to go further than 2.1.6's "keep history,
just stop offering it for new checks." Irreversible by design; see that
migration file's own doc comment for why it's safe (explicit child-first
delete order + a transaction, even though the existing `ON DELETE
CASCADE`s would have done the same thing on their own).

**Not yet verified**: real StubHub-successor (Viagogo) / Vivid Seats /
Ticombo DOM markup, and the Market Analysis tier/section detection
against it. This sandbox has no network access to those domains,
confirmed fresh as of the 2.1.9 delivery and unchanged since - the
marketplace-specific selectors (including `tierFor` in
`price_checker_scan.js`) are unconfirmed against live pages until marko
runs it on his own machine and reports back what a real scan actually
finds.

## Where the detailed history lives

Every past release has its own `REDESIGN-X.Y.Z-REPORT.md` or
`*-REPORT.md` file at the repo root (Slovak, written for marko) - 114 of
them as of 2.2.8. These are not read by default under this protocol; only
open one when the current bug plausibly traces back to that specific
release, or marko points at it directly.

## Known task-list debt (not yet triaged into KNOWN_BUGS.md)

The internal task tracker used across this whole project's history has a
handful of old `pending`/`in_progress` markers that predate this protocol
and were never explicitly closed out, even though the work they describe
looks superseded by later releases (e.g. early dark-mode/refund-audit
tasks from the very first versions, and the whole 2.1.3 "production
hardening" task block, whose target - the old hidden auto-check - no
longer exists after 2.1.9). None of these were re-audited to write this
file, per the "don't repeat previous audits" rule - they're flagged here
so a real triage pass can happen deliberately, on request, instead of
silently.

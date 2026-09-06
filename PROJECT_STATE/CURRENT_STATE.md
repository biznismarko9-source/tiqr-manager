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

**2.12.0**, consistent across `package.json`, `src-tauri/tauri.conf.json`,
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
downgrade or a repeat. 2.3.2 through 2.3.4 were the Dashboard Total-cost
card and the two-attempt Sheets push row-placement fix. 2.3.5 was the
sync/push behavioral redesign (self-healing push, real diff-and-update
sync, UI-freeze fix). A first "2.4.0" direction, "Live Event Intelligence
Foundation," was designed and built in full, then marko reviewed the
package and cancelled the whole direction outright ("Predchádzajúci nápad
'Live Event Intelligence' RUŠÍME ÚPLNE") before it was ever run through
`release.ps1`/`1-CLICK-UPDATE.bat` - so, unlike the 2.3.0 case above, **no
install anywhere ever recorded a "2.4.0" build**, and reusing the version
number for the real replacement feature was verified safe rather than
assumed (see `PROTECTED_AREAS.md`'s "2.4.0 (pre-release direction)" entry
for the exact reasoning, and re-verify the same fact yourself before ever
reusing a number a third time). Reusing "2.4.0" for the real replacement
was briefly the plan and was verified safe from the auto-updater's own
point of view - but before that build was ever handed over, redelivering
the report and zip under the exact filenames already used once in this
same chat for the cancelled direction (`REDESIGN-2.4.0-REPORT.md`,
`tiqr-manager-2.4.0.zip`) caused a practical download problem for marko,
so the whole thing (app version plus every release file) was bumped one
more step to **2.4.1** before delivery - a plain filename-collision fix,
not an auto-updater concern, and it does not change any of the reasoning
above. **2.4.1** shipped as "Price Checker Live Market Monitor," all
online/live-market functionality folded directly into Price Checker
instead of a separate feature - see "Current focus" below and
`CHANGELOG.md`'s own entries for all of them.)

**2.4.2** removes 2.4.1's "Price Checker Live Market Monitor" entirely, at
marko's explicit request ("TÚTO FUNKCIU NECHCEM V APLIKÁCII VÔBEC" - "I
don't want this feature in the app at all"): Auto Monitor, Scan All, the
Live Market Monitor panel, the Market History view, the `market_alert`
Attention Center category, and the backend `price_checker_monitor` module
and its 2 commands are all gone - Price Checker goes back to being a
purely manual tool (event selection, manual Visible Scanner, Market
Analysis, tier/section grouping, price history, Your Tickets comparison),
exactly as it worked before 2.4.1. Same precedent as the 2.3.0->2.3.1
revert above: removing a shipped feature still bumps the version FORWARD,
never back to a number already used - the resulting code is functionally
equivalent to 2.3.5 (the last version before 2.4.0/2.4.1), but the version
is **2.4.2**, not 2.3.5. Unlike the 2.3.0 case, this revert could NOT
delete migration `026_price_checker_market_monitor.sql` or its 4 tables:
2.4.1 was a real shipped release marko already has installed, so that
migration has already run against his real local DB, and this codebase's
forward-only migration rule means an already-applied migration is never
deleted or renumbered, even when every table it created becomes unused.
The migration file and its 4 tables (`market_snapshots`,
`market_snapshot_tiers`, `market_source_status`, `market_alerts`) stay in
the schema, orphaned but untouched, from 2.4.2 onward - see
`PROTECTED_AREAS.md`'s new "2.4.2" entry before ever touching migration
numbering or that schema again. **The next new migration is 027, not a
reused 026** (still true as of 2.4.3 below - that release added no
migration at all).

**2.4.3** adds the "Ticket Control Center" (`/control-center`) - one dense
work screen over the EXISTING tickets/listings/sales data across every
event, marko's own explicit request. No schema change. See "Current focus"
below and `PROTECTED_AREAS.md`'s new "2.4.3" entry for the full design and
the traps worth knowing before touching it again.

**2.4.4** is a pure frontend/UX round, no backend or schema changes at all:
Ticket Control Center (2.4.3) and Fulfillment Center (2.2.12) merged from
two standalone top-level routes into one "Ticket Center" tab under Finance
(two subtabs); the sidebar regrouped Events/Orders/Tickets/Sales/Inventory
under one collapsible "Tickets" entry; a one-click light/dark toggle moved
out of Settings -> Appearance to sit above the sidebar's profile widget;
and a sticky-header dark-mode transparency bug on Ticket Control Center's
table got fixed. See "Current focus" below and `PROTECTED_AREAS.md`'s new
"2.4.4" entry for the full breakdown.

**2.5.0** adds the "TIQR Operations Calendar" (`/calendar`) - one new
top-level Month/Week calendar page aggregating every part of the app that
has a real date (events, orders, sales, pulls, attention items) into one
read-only, cross-domain view. New backend module `commands/calendar.rs`
(one command, `get_calendar`) and new models `CalendarFilters`/
`CalendarEntry` - no new migration, no new table. Of marko's 8 originally
named candidate categories, 3 (payouts, payments, fulfillment) do NOT exist
as real, reliably-dated data anywhere in this app and are deliberately
absent from the calendar rather than invented - see "Current focus" below
and `PROTECTED_AREAS.md`'s new "2.5.0" entry before ever trying to add one
of those 3 back in.

**2.5.1** is a pure frontend/UX round, no backend or schema changes at all:
Ticket Center (briefly a Finance subtab in 2.4.4) is back out as its own
top-level page/route, marko's own explicit request, and rebuilt from
scratch around ORDERS instead of the old per-ticket Control Center (2.4.3)/
Fulfillment Center (2.2.12) pages (both deleted); and the whole top-level
sidebar order changed to his exact list (Dashboard, Tickets, Price Checker,
Pulls, Finance, Ticket Center, Calendar - Calendar moved from right after
Dashboard to last). The Calendar page (2.5.0) also got a presentation-only
visual refresh ("viac moderny, viac prehladny" - marko's own words): a
consistent color per entry kind plus a severity ring/text-color layered on
top, no change to any hook, API call, or navigation target. See "Current
focus" below and `PROTECTED_AREAS.md`'s new "2.5.1" entry (the one thing
this leaves orphaned: `ticket_control_center.rs`'s backend command).

**2.5.2** adds a real "Forgot password?" flow (marko's own request, alongside
a Discord sign-in ask that's deliberately deferred - see
`PROTECTED_AREAS.md`'s new "2.5.2" entry for why) - built entirely on
Firebase's existing client-side password-reset APIs
(`sendPasswordResetEmail`/`verifyPasswordResetCode`/`confirmPasswordReset`),
no new backend, no Cloud Functions, no Firebase Blaze billing plan. The one
new piece of infrastructure is a `tiqrmanager://` custom URL scheme
(`tauri-plugin-deep-link`) so the emailed reset link opens TIQR Manager
itself instead of a browser tab, landing on a new `src/pages/
ResetPassword.tsx`. See "Current focus" below for the full mechanism and the
one manual Firebase Console step this needs before it actually works.

**2.6.0** is a complete visual redesign of the app and NOTHING else - marko's
own task, framed by him as "VÝLUČNE UI/UX a visual redesign. Žiadne nové
business features. Žiadne nové workflow. Žiadne nové databázové systémy."
`src-tauri/` is byte-for-byte identical to 2.5.2 (verified by diff), there is
no new migration (the next new one is still **027**), and no dependency was
added. The whole change lives in one shared design layer -
`tailwind.config.js`, `src/index.css`, `src/components/ui.tsx`,
`src/components/Layout.tsx` - plus mechanical adoption of that layer across
the pages. See "Current focus" below and `PROTECTED_AREAS.md`'s new "2.6.0"
entry (especially: where the app's look is allowed to be defined, and why
`brand` and the narrow-table metrics were left alone). **One caveat this
release carries that no previous one did: it was implemented on a machine
with no Node.js and no Rust toolchain, so `npx tsc -b`, `npm run build` and
`cargo check --lib` were NOT run** - marko chose to proceed on static review
rather than install a toolchain. Run all three before publishing the tag.

**2.7.0** adds the **AI Import Assistant** - marko's own request. Drop, paste
(Ctrl+V) or upload a screenshot into the New Event, New Order or New Sale form
and Claude reads structured candidate values off it, which then PRE-FILL that
form's existing inputs. **No schema change, no migration (the next new one is
still 027), no new dependency.** The rule the whole thing is built around, in
his words: "AI NIKDY nesmie priamo vytvoriť alebo meniť databázový záznam" -
so the new backend module has no database access at all, and everything still
saves through the same create commands and the same validation as before. New
`commands/ai_import.rs` (one command), new `components/AiImportPanel.tsx` and
`lib/aiImport.ts`. Needs the `ANTHROPIC_API_KEY` GitHub Actions secret, which
until now only gated event-category detection. See "Current focus" below and
`PROTECTED_AREAS.md`'s new "2.7.0" entry. **Same caveat as 2.6.0: no Node.js
and no Rust toolchain on the machine this was built on, so `cargo test --lib`,
`cargo check --lib`, `npx tsc -b` and `npm run build` were NOT run** - and this
time that includes 28 new, never-executed Rust unit tests.

**2.8.0** turns the Calendar into one of the app's main work screens - marko's
own request. **Four views** (Month/Week/Day/Agenda) over the same
range-based `get_calendar` command, plus **two genuinely new date sources**:
`finance` (`finance_entries.entry_date`) and `recurring`
(`recurring_expenses.next_date`). marko asked again for payouts/payments/
fulfillment; all three are still absent because no such date exists in this
schema - re-derived from the live schema and command code this release, and
now guarded by a test. **No schema change, no migration (the next new one is
still 027), no new index, no new dependency**, and `calendar.rs` remains a
read-only aggregator. See "Current focus" below and `PROTECTED_AREAS.md`'s new
"2.8.0" entry. **Same caveat as 2.6.0/2.7.0: no Node.js and no Rust toolchain
on the machine this was built on**, so none of the four build/test commands
were run.

**2.9.0** is a Price Checker accuracy release - marko reported the scanner
reading prices wrong, and this round is diagnosis first, then the smallest fix
per cause. The headline cause was in `price_checker_scan.js`: the generic
text-node walker matched money in one text node and then re-parsed the PARENT
element's whole text, keeping whichever money appeared first - routinely a
crossed-out "was" price or a fees-inclusive total. Fixed, plus three new price
rejection rules, a metadata-scoping fix, a two-way dedup fix, and a
currency-blending fix in `compute_scan_stats`. New: a Found/Accepted/Skipped/
Duplicates scan summary, six result filters, and CSV export. **No schema
change, no migration (next new one is still 027), no new dependency, no new
marketplace, no background monitoring.** See "Current focus" below and
`PROTECTED_AREAS.md`'s new "2.9.0" entry. **Same build caveat as 2.6.0-2.8.0,
plus one more: the injected browser script could not be executed here at all
(no Node.js), and the live marketplaces have never been reachable from this
sandbox - so every DOM-level fix is reasoned from the code and needs
confirming against a real listings page.**

**2.10.0** replaces Price Checker's single "Select an event..." dropdown with
an event overview: every upcoming event as a dense card showing its
per-marketplace link status, that marketplace's last check and listing count,
plus multi-select, local search and four quick filters. One new read-only
command (`list_price_checker_overview`) answers the whole list in four flat
queries; it writes nothing and triggers no scan or marketplace request. **UX
only** - scanner, parser, readers, market analysis and history are untouched.
**No schema change, no migration (next new one is still 027), no new
dependency.** There is deliberately no "Scan failed" state: a failed scan is
never persisted in this app. See "Current focus" below and
`PROTECTED_AREAS.md`'s new "2.10.0" entry. Same build caveat as 2.6.0-2.9.0.

**2.11.0** is a release/distribution infrastructure round - no business logic,
no schema, no migration, no new dependency, no new cloud service. Most of what
marko asked for already existed (Tauri updater plugin with a real public key,
GitHub Releases `latest.json` endpoint, `createUpdaterArtifacts`, per-user NSIS
installer with the WebView2 bootstrapper, `tauri-action` on a `v*` tag,
`release.ps1`/`1-CLICK-UPDATE.bat`, `UpdateOverlay`, the Settings software
section) and was kept. **The real gap was macOS**: the workflow was
Windows-only and no `.dmg` had ever been built. `build-windows.yml` is now
`release.yml` and builds both platforms (universal `.dmg`), the stale-release
delete moved to its own pre-matrix job, the matrix is serialised, and a new
`verify-release` job checks the finished `latest.json`. Plus a Dashboard
update pill, Settings current/latest/last-checked rows, a 6-hour re-check, and
`RELEASE.md`. **Windows and macOS CODE signing remain unconfigured** (paid
external credentials); updater signing is configured. See "Current focus"
below and `PROTECTED_AREAS.md`'s new "2.11.0" entry.

**2.11.1** fixes the macOS build 2.11.0 broke. 2.11.0's Windows release
published fine; its macOS leg built the universal binary successfully (3m36s)
and then failed while codesigning. Cause: 2.11.0 wired the six `APPLE_*`
variables into the release step in advance, and **an unset GitHub secret still
defines the variable as an empty string** - which the Tauri bundler reads as
"sign this". They are removed and left commented, with a warning.
`prepare-release` also no longer paints a red annotation when there is simply
no old release to delete. See `PROTECTED_AREAS.md`'s new "2.11.1" entry -
that empty-vs-unset trap is worth not relearning.

**2.12.0** adds **Cloud Sync** - marko works on a Windows PC and a Mac and
wants what he writes on one to appear on the other. New
`commands/cloud_sync.rs` keeps ONE database snapshot in his own Google Drive
and syncs it **whole-file, one direction at a time** (Sync up / Sync down).
Row-level merging was deliberately not attempted - see `PROTECTED_AREAS.md`'s
"2.12.0" entry for why it would be a months-long rebuild. A lost-update guard
refuses an upload when the other machine has pushed since, offering an
explicit overwrite instead of silently winning; downloads reuse
`backup::restore_database_impl` and inherit its safety backup and rollback.
Storage reuses the Google sign-in already in production for Sheets sync -
no server, no new dependency. **One migration cost: `OAUTH_SCOPE` gains
`drive.file`, so everyone signs in with Google once more.** Nothing syncs
automatically; the app stays local-first and fully usable offline.

## Stack / layout

- **Frontend** (`src/`): React + TypeScript + Tailwind, Vite build.
  Pages under `src/pages/`: Dashboard (2.4.1 briefly added a 6th Attention
  Center box, "LIVE MARKET ALERTS" - removed again in 2.4.2, back to 5
  boxes - see "Current focus" below), Calendar (2.5.0 - new, the "TIQR
  Operations Calendar" - Month/Week grid, Day Detail modal, a Today/next-7-
  days summary card, and a client-side Filters row over 5 real categories
  (event/order/sale/pull/attention) - see "Current focus" below), Events, EventDetail
  (2.2.2-2.2.5: tabbed "Event Workspace" - Overview/Listings/Sales (Finance
  folded into Sales in 2.2.5), see "Current focus" below and
  `PROTECTED_AREAS.md`'s "2.2.2"/"2.2.3"/"2.2.4"/"2.2.5" entries before
  adding more event-level functionality anywhere else), Orders,
  OrderDetail, Tickets (Inventory), Inventory, Sales, SaleDetail, Pulls
  (given/received), Finance (own `finance/` subfolder, back to its original
  4-tab layout - Overview/Transactions/Accounts/Reports - as of 2.5.1; briefly
  grew a 5th "Ticket Center" tab in 2.4.4, moved back out to its own
  top-level page in 2.5.1, see "Current focus" below), TicketCenter (2.5.1 -
  new top-level page/route `/ticket-center`, replacing both
  `finance/TicketCenter.tsx` (deleted) and the two pages it used to host,
  `TicketControlCenter.tsx`/`FulfillmentCenter.tsx` (both deleted) - lists
  ORDERS, not individual tickets; reuses `api.listOrders` and Orders.tsx's
  own `orderCompletionChecks`/`completionStatus`, no new backend at all -
  see "Current focus" below), PriceChecker (2.4.1 briefly gained a Live Market
  Monitor panel per marketplace card; removed entirely in 2.4.2, back to
  the original manual tool - see "Current focus" below and
  `PROTECTED_AREAS.md`'s "2.4.2" entry), Settings (2.4.4: its old
  Appearance section is gone - moved to the sidebar itself, see Layout.tsx
  below, not duplicated), Welcome (auth), PendingApproval, DatabaseError.
  Shared: `src/types.ts`, IPC in `src/lib/api.ts`, auth in
  `src/lib/auth.tsx`, money/date parsing helpers in `src/lib/`.
  **Calendar (2.8.0)**: `pages/Calendar.tsx` renders four views (Month/Week/
  Day/Agenda) over one range-based command. Backed by
  `commands/calendar.rs`, which gained `finance_in_range` and
  `recurring_in_range` this release and remains read-only.
  **AI Import Assistant (2.7.0)**: `components/AiImportPanel.tsx` (one shared
  panel, embedded in the New Event/Order/Sale forms) + `lib/aiImport.ts` (image
  prep, clipboard/drop extraction, the per-session duplicate guard). Backed by
  ONE backend command, `commands/ai_import.rs::analyze_import_image`, which is
  database-free by construction - see "Current focus" below.
  **Design layer (2.6.0)**: the app's entire visual language lives in four
  files and nowhere else - `tailwind.config.js` (the `brand`/`slate` ramps,
  the `shadow-card`/`raised`/`overlay` scale, the radius rhythm, the
  120-180ms `transitionDuration.DEFAULT`), `src/index.css` (base typography,
  the surface/line CSS variables, and the `.input`/`.label`/`.th`/`.td`/
  `.card`/`.section-title`/`.skeleton`/`.table-shell`/`.table-flush`/
  `.row-selected`/`.field-invalid` component classes), `src/components/
  ui.tsx` (every shared control, plus `SEGMENTED_TRACK`/`segmentedItemClass`
  as the app's one tab pattern) and `src/components/Layout.tsx` (the shell).
  A page must not define a visual pattern of its own - if it needs one, it
  belongs in that layer. See `PROTECTED_AREAS.md`'s "2.6.0" entry.
  **Layout** (`src/components/Layout.tsx`, the sidebar): 2.4.4 grouped
  Events/Orders/Tickets/Sales/Inventory under one collapsible "Tickets"
  entry (session-only expand state, defaults open - the 5 routes
  themselves are unchanged), and added a one-click light/dark toggle above
  the profile widget, reusing the same `lib/theme.ts` `useTheme()` hook
  Settings' old Appearance section used to call. 2.5.0 added a new flat
  "Calendar" entry right below Dashboard. **2.5.1** changed the top-level
  order to marko's own exact list - Dashboard, Tickets (group), Price
  Checker, Pulls, Finance, Ticket Center, Calendar - moving Calendar from
  right after Dashboard to last, and adding Ticket Center back as its own
  entry (icon: `IconLayoutGrid`, unused anywhere else in the app - picked
  specifically to avoid looking like `IconAlertTriangle`'s existing
  "Attention" association).
- **Backend** (`src-tauri/src/`): Rust, Tauri 2. One module per domain
  under `commands/`: calendar (2.5.0 - new; one read-only command,
  `get_calendar`, backing the Calendar page - reuses
  `attention_center::get_attention_center_impl` directly (both for its own
  "attention" entries and to decide an "event" entry's severity) and
  `sales::GROUP_KEY_EXPR` for grouping sale batches; writes its own SELECT
  for events/orders/pulls, same "each view aggregator writes its own query"
  convention as `attention_center`/`ticket_control_center` - see "Current
  focus" below), events, orders (+ `orders_sheet_sync` - 2.2.10: same
  push-bookkeeping-ordering fix as `pulls_sheet_sync` below), tickets,
  ticket_listings (2.2.4 - real per-marketplace listings; 2.2.5 added 3
  all-or-nothing bulk commands - status/price/delete - see "Current focus"
  below), ticket_control_center (2.4.3 - one read-only command,
  `list_control_center_tickets`, originally backing the Ticket Control
  Center page. **As of 2.5.1 this command is fully orphaned** - the
  frontend page that called it was deleted (see TicketCenter's own 2.5.1
  entry above), and nothing else in the app calls `get_control_center_*`/
  `listControlCenterTickets`. Left in place deliberately rather than
  removed, same "document, don't necessarily delete" precedent as the
  `payments` table (migration 007) and migration 026's orphaned market-
  monitor tables below - see `PROTECTED_AREAS.md`'s "2.5.1" entry before
  either reviving or removing it), inventory_intelligence (2.2.6 - one
  read-only command backing
  Overview's "Inventory Intelligence" block, see "Current focus" below),
  attention_center (2.2.8 - one read-only command backing the Dashboard's
  global, cross-event "Attention Center" block; 2.2.9 reworked its 4
  ticket-level categories to group by order instead of one row per ticket;
  2.2.10 fixed its sort tie-break and excluded done events from 3 of its 5
  categories; 2.2.11 changed only how the FRONTEND groups/displays these
  same items - 5 clickable category boxes instead of a priority-grouped
  feed; 2.4.1 briefly added a real 6th category, "market_alert", removed
  again in 2.4.2 - back to the same 5 categories as 2.2.11, see "Current
  focus" below), sales, event_categories, pulls
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
  A `price_checker_monitor` module (2.4.1 - Live Market Monitor: permanent
  snapshots after every successful scan, per-tier change detection, MARKET
  DROP/RISE/NEW SUPPLY/SUPPLY DROP/SOURCE FAILURE alerts) existed briefly
  and was deleted again in its entirety in 2.4.2 at marko's explicit
  request - it is NOT in this codebase anymore, do not assume any file/
  command/struct it describes still exists; see "Current focus" below and
  `PROTECTED_AREAS.md`'s "2.4.2" entry. (A separate "Live Event
  Intelligence" module was built even earlier and briefly labeled 2.4.0
  too, before that direction moved on to become 2.4.1's Live Market
  Monitor instead - then fully reverted before ever shipping - see
  `PROTECTED_AREAS.md`'s "2.4.0 (pre-release direction)" entry and the
  "## Version" note above. Neither module exists in this codebase today.)
- **DB**: SQLite via `rusqlite`, migrations in `src-tauri/migrations/`,
  currently through **026_price_checker_market_monitor.sql** (a first
  `026_live_event_intelligence.sql` was deleted before ever shipping when
  that direction was reverted - see `PROTECTED_AREAS.md`'s "2.4.0
  (pre-release direction)" entry for why reusing "026" for the real
  migration was verified safe). Migrations run automatically at startup,
  forward-only. **As of 2.4.2, migration 026's 4 tables (`market_snapshots`,
  `market_snapshot_tiers`, `market_source_status`, `market_alerts`) are
  orphaned** - the migration already shipped in 2.4.1 and stays applied
  forever per the forward-only rule, but no application code reads or
  writes them anymore. Do not reuse "026" for anything and do not delete
  that migration file; the next new migration is **027**. See
  `PROTECTED_AREAS.md`'s "2.4.2" entry.
- **Packaging**: `release.ps1` (invoked via `1-CLICK-UPDATE.bat`) mirrors
  this folder into a fresh clone of the real GitHub repo, cross-checks the
  version in 3 files, commits, tags, and pushes - the tag push triggers
  the signed Windows installer build in GitHub Actions. Exclusion list for
  any manual zip matches `.gitignore` (node_modules, dist*, target, gen,
  logs, etc).

## Current focus / most recent work

**2.12.0 - Cloud Sync: one database, two computers.** marko asked for what he
writes on the Mac to show up on Windows. He first said "aj naraz"
(simultaneously), then clarified he means sequential hand-off - which is the
difference between a months-long rebuild and one contained release.

- **Whole-file, one direction at a time.** `commands/cloud_sync.rs` keeps one
  database snapshot in his own Google Drive. Sync up uploads this machine;
  Sync down downloads and restores the other. It does NOT merge rows.
- **Why not merge**: every PK is a per-machine `INTEGER AUTOINCREMENT` (both
  machines mint id 5), and `insert_order_with_tickets`' exact-cent split and
  `refund_sale_impl`'s one-way transition have no correct automatic merge.
  That was explained to marko before building, and he chose this.
- **The lost-update guard** is the safety story: every upload records the
  Drive `version` it wrote and re-checks it next time. If the other machine
  pushed since, the upload is refused and the UI offers an explicit
  "Overwrite anyway". Downloads go through `restore_database_impl`, so they
  inherit validation, an automatic safety backup and rollback.
- **Almost everything is reused**: Google sign-in and refresh tokens
  (production since Sheets sync), the SQLite Online Backup API, the restore
  path. `backup::snapshot_db_to` was extracted from `create_safety_backup` so
  both callers share one snapshot implementation. No server, no new
  dependency, no hosting cost.
- **The one migration cost**: `OAUTH_SCOPE` gains `drive.file` (the narrowest
  scope that works - only files this app created). An existing refresh token
  was issued against the old scopes, so **everyone signs in with Google once
  more**.
- **Still local-first**: nothing runs on a timer or at startup, sync is off
  until switched on, and every sync is a click.
- **NOT verified by a build, and no Drive call has ever run.** No Node.js or
  Rust toolchain here. The Drive v3 request shapes are written from the API
  docs - first thing to check if sync misbehaves. 5 new Rust unit tests cover
  the guard logic and the off-by-default behaviour; none executed.

**2.11.0 - Release pipeline: macOS builds, verified updater manifest, in-app
update centre.** marko asked for a professional installer + auto-updater +
release pipeline. Phase 0 (mapping what already existed) is most of the story.

- **What already existed and was kept**: the Tauri updater plugin on both
  sides, a real `plugins.updater.pubkey`, the GitHub Releases `latest.json`
  endpoint, `createUpdaterArtifacts: true`, the per-user NSIS installer with
  `webviewInstallMode: downloadBootstrapper`, the two-path GitHub Actions
  workflow (manual unsigned test build; signed release on a `v*` tag),
  `release.ps1` + `1-CLICK-UPDATE.bat`, `lib/updater.ts`, `UpdateOverlay.tsx`
  and Settings' Software section. None of that was rebuilt.
- **The actual gap: macOS.** The workflow was Windows-only. It is renamed
  `release.yml` and now builds a universal `.dmg` (Apple Silicon + Intel, one
  download) alongside the `.exe`. `release.ps1`'s workflow guard was renamed
  in the same change - see `PROTECTED_AREAS.md` for why those must agree.
- **Three structural fixes the matrix exposed**: the stale-release delete now
  runs in its own `prepare-release` job BEFORE the matrix (otherwise the
  second runner deletes the release the first just published); the matrix runs
  `max-parallel: 1` (both legs rewrite `latest.json`); and a new
  `verify-release` job asserts the release really has an `.exe`, a `.dmg` and
  a `latest.json` covering both platform families with a signature and url on
  every entry.
- **In the app**: a small Dashboard update pill that reads the launch-check
  result and LINKS to Settings rather than installing anything itself (one
  updater UI, not two); Settings gained Current/Latest/Last checked; and the
  launch check now repeats every 6 hours - deliberately slow, no polling.
- **Signing, stated honestly**: updater signing is configured and is what
  makes an update safe to apply. **Windows code signing and Apple Developer ID
  / notarization are NOT configured** - both need paid external credentials.
  The workflow passes all six Apple variables through, so adding the secrets
  is the only remaining step, and nothing fakes a certificate. Until then the
  `.exe` shows SmartScreen and the `.dmg` needs right-click → Open once.
- **`RELEASE.md`** documents the release process, every secret, the artifact
  list and both checklists - with no secret values in it.
- **NOT verified by a build, and this round more than most**: no Node.js or
  Rust toolchain here, and **no macOS build has ever run** - the first tagged
  run of this workflow is itself the test. What WAS verified: the workflow
  YAML parses and its job graph is correct (`prepare-release` → `release`
  matrix → `verify-release`), `release.ps1` has zero stale references to the
  old workflow filename, and every changed TS file balances and resolves its
  imports.

**2.10.0 - Price Checker event overview replaces the dropdown.** marko's own
request: see every relevant event at once, then pick one (or several), instead
of hunting through a `<select>`.

- **The list, and what is on a card.** Every UPCOMING event gets a dense card:
  name, date, venue/city, one row per marketplace (Linked / No link, plus that
  marketplace's own last check and listing count), and an event-level line
  with the newest listing count across all marketplaces or "Not scanned yet".
  Checkbox per card, "Select all" (scoped to the currently visible rows),
  "Clear selection", a "Selected: N events" bar, local search over
  name/venue/city, and four quick filters.
- **One read-only command behind it.** `list_price_checker_overview` answers
  the whole list in four flat queries and assembles them in memory - it does
  NOT call `get_price_checker_summary_impl` per event, which would be N round
  trips for data the list discards. It writes nothing and makes no marketplace
  request: opening the page costs one database read.
- **Two rules deliberately mirrored, not reinvented.** Upcoming-only is the
  same rule PriceChecker.tsx applied client-side from 2.2.2, moved into SQL.
  The marketplace set uses the same predicate `get_price_checker_summary_impl`
  already applies (active always; retired only where this event really has a
  link or check).
- **No "Scan failed" state, on purpose.** `price_checks` has no status column
  and a row only lands there via the explicit review-then-save step, so every
  stored check succeeded by construction; the scanner's own error states are
  in-memory only. A "Scan failed" badge would be inventing data - the same
  refusal as payouts/payments on the Calendar. There is a test guarding it.
- **"Check selected" is not a batch runner.** The scanner opens a real visible
  browser window marko drives himself, so it opens the FIRST selected event's
  flow and keeps the selection - the rest are one click each. No queue, no
  parallel sessions, no automation.
- **An "All events" back link** was added, since the dropdown that used to be
  the way back is gone.
- **Judgment calls, none asked about directly**: dropping the "Scan failed"
  filter entirely rather than showing an always-empty one; scoping "Select
  all" to visible rows; showing relative times ("2h ago") on cards with the
  exact timestamp as a tooltip; and a two-column card grid on wide screens.
- **NOT verified by a build**, same as 2.6.0-2.9.0. 8 new Rust unit tests (42
  in `price_checker.rs`), none executed. Statically verified: bracket balance,
  all `#[test]` attributes attached, every TS import resolving, and no
  mixed-type array literal of the kind that broke the 2.9.0 build.

**2.9.0 - Price Checker accuracy + scan report, filters, export.** marko
reported wrong prices coming out of the Visible Scanner. His own instruction
was explicit: diagnose first, do not rewrite.

- **The root cause, and it was one line of wiring.** `readGenericVisibleText`
  found money in a TEXT NODE and then called `candidateFrom(node.parentElement)`,
  which re-parsed that parent's entire `accessibleText()`/`textContent` and
  kept the FIRST money match. So the price stored was never guaranteed to be
  the price found. On "Was $200  Now $120" markup it stored $200; on a wrapper
  whose `aria-label` reads "Total incl. fees $340" it stored $340. The matched
  money and the matched text are now passed in.
- **Three rejection rules that never existed**: struck-through prices
  (`<s>/<del>/<strike>`, a line-through class, or the computed style), money
  labelled total/subtotal/fee/service charge/delivery/tax/was/original/RRP,
  and money inside header/footer/nav/aside/cart/checkout/modal chrome. Every
  rejection is COUNTED and reported, so an over-aggressive rule shows up as a
  number rather than as missing data.
- **Metadata leak fixed.** `findListingContainer` fell back to "3 ancestors
  up" and `nearbyListingContext` regexed that whole subtree - which is how
  section/row/quantity/tier came in from neighbouring listings. The container
  now reports `confident`; when it is false, metadata comes from a tight scope
  and the listing is flagged `incomplete` instead of looking clean.
- **Dedup was wrong in BOTH directions.** A real listing id is now the whole
  cross-scan key (it used to be one of seven fields including the price, so a
  re-read after scrolling counted twice the moment the price read
  differently - which the parser bug above made routine). And `tier` joined
  the fallback key, whose absence merged two listings differing only by tier
  and deleted a real one. Without an id, price stays in the key deliberately.
- **Currency blending fixed.** `compute_scan_stats` averaged across every
  listing regardless of currency and labelled the result with the first
  currency seen - so the scanner headline could disagree with the Market
  Analysis directly underneath it, which has always partitioned correctly. It
  now computes inside the largest single-currency group and says how many
  listings it excluded.
- **New, all additive**: Found/Accepted/Skipped/Duplicates summary with
  per-reason counts; six plain result filters; Export CSV reusing the existing
  `plugin-dialog` `save()` + Rust writer mechanism. `ScannerSession` gained
  `url` (already a parameter of `open_price_scanner`, just never kept).
- **Deliberately unchanged**: the manual Visible Scanner workflow, Tier/Level
  as a grouping only (no section/row/seat pricing, no suggestions, no
  repricing), Your Tickets, history, and every protected business area. No
  background monitor, no scheduled scan, no polling, no CAPTCHA bypass, no new
  marketplace.
- **Verification, honestly**: 15 new Rust unit tests (43 in that module) cover
  the fingerprint rules, the currency-scoped stats, the summary arithmetic and
  the `incomplete` passthrough - **but none were executed** (no Rust
  toolchain). The browser script has **no test at all** and could not be run:
  there is no Node.js here, and the live marketplaces have never been
  reachable from this sandbox. Every DOM-level fix is reasoned from the code.
  A real listings page is the actual acceptance test.

**2.8.0 - Calendar redesign + advanced calendar workflow.** marko's own
request to make the TIQR Operations Calendar a main work screen rather than a
read-only overview.

- **Four views, one data path.** Month, Week, Day and Agenda all call the same
  `get_calendar(dateFrom, dateTo)` with a different window and share one
  filter state and one search box - so switching period fetches exactly one
  new range and nothing more (marko's section 17 was already satisfied by
  2.5.0's range-based command; this release only had to use it correctly).
  Agenda is a fixed forward window (45 days) anchored to today, so its
  prev/next control is hidden rather than shown doing nothing.
- **Two new date sources, and three still-refused ones.** `finance` and
  `recurring` are real columns with live command code behind them. Payouts,
  payments and fulfillment were re-checked against the live schema this
  release and are still not real - see `PROTECTED_AREAS.md`'s "2.8.0" entry
  for the evidence and for the standing test that now guards it. **The thing
  the 2.5.0 pass missed was Finance**, which is where this app's real dated
  money lives; it is added under its own name, never rebranded as a payout.
- **Overdue is real for exactly one reason.** `recurring_expenses.next_date`
  is the only genuine due date in the app. The rule mirrors
  `finance/Accounts.tsx` (active template, `next_date < today`), paused
  templates are excluded, and the strip's Overdue tile is hidden entirely
  when the count is zero rather than implying this app tracks more deadlines
  than it does.
- **No time-of-day axis**, because no date in this app has a time. Week is
  seven day columns; Day groups by kind. marko's own spec asked not to fake
  time positions, and this is that.
- **Everything else the release added** is presentation over data that was
  already there: a Today/Tomorrow/Next 7 days/Overdue strip, a Day Detail
  with a per-kind summary panel, per-day workload bars (three muted segments,
  not a color scale - marko: "NECHCEM farebný chaos"), event countdowns from
  the event's own date only, calendar-local search (dims non-matches in the
  grid, filters the lists, jumps to the first match), and filter chips that
  only render for kinds actually present in the loaded range.
- **Not built: quick-add.** There is no task/note/reminder table anywhere in
  this schema (checked), and marko's own spec said not to stand up a new task
  database for it in this release.
- **Judgment calls, none asked about directly**: two new kinds rather than one
  merged "money" kind (only `recurring` can be overdue, and merging would
  make the Overdue rule ambiguous); teal/fuchsia for the two new kinds (the
  only hues left that are neither a severity color nor a shade of an existing
  kind); a 45-day Agenda window and a 90-day overdue lookback (both bounded
  on purpose - see PROTECTED_AREAS); and search dimming rather than filtering
  in the grid, so you can still see where a match sits relative to
  everything else.
- **NOT verified by a build**, same as 2.6.0/2.7.0. What WAS verified
  statically: `Calendar.tsx` bracket/JSX balance and zero unresolved, unused
  or dead symbols; `calendar.rs` bracket balance with strings and comments
  stripped; all 23 `#[test]` attributes correctly attached to a `fn` (one
  orphaned attribute was found and fixed this way); every column the two new
  queries read verified against the real DDL, including their CHECK
  constraints against the test seeds.

**2.7.0 - AI Import Assistant.** marko's own spec: one shared, compact panel
inside the three create forms that turns a screenshot into pre-filled fields.
His framing of the boundary is the design: image -> Claude -> structured result
-> review -> user confirm -> **the existing TIQR create flow** -> database.

- **Where it is**: inside `EventFormModal` (new events only), `OrderFormModal`,
  and `SaleFormModal`'s DETAILS step. Not a page, not a route, not a tab, not a
  sidebar - marko was explicit ("Nechcem veľký AI dashboard. Nechcem chat.").
- **It cannot write.** `commands/ai_import.rs` exposes one command,
  `analyze_import_image`, which takes no `AppState`, opens no `Connection`, and
  has no insert/update path. The panel's only output is a bag of strings handed
  to the form's own `setState` calls. Every save still goes through
  `create_event`/`create_order`/`create_sale_batch` and their existing
  validation, on values marko has looked at. **If a future change needs this
  module to touch the database, that is a redesign to discuss first.**
- **Structured, not free text.** The model is held to a strict per-kind JSON
  schema (`output_config.format`), whose `field` enum only contains the names
  that kind's form actually has - so a hallucinated field name is rejected at
  the API boundary before `sanitize_result` even has to drop it. Every value is
  a plain string or `null`, with `high`/`medium`/`low` confidence.
- **Nothing is invented.** A value not visible on the image comes back `null`,
  and a null never overwrites a form field. The one deliberate exception to
  "copy verbatim" is date FORMAT (ISO, because the app's inputs are
  `<input type="date">`) - and even there, a missing year is null, never the
  current year.
- **Multiple ticket groups stay separate** (marko: "Nechcem zlievať rôzne
  groups do jednej"). Because `OrderInput` carries one section/row/tier/price
  for a whole order, two groups are two orders - the panel fills one at a time
  and says so, rather than inventing a multi-group order shape.
- **Sale is the deliberate exception**: its field list has no seat/section/row,
  because a sale is always recorded against ticket rows that already exist
  (`SaleInput.ticketId`). The tickets are picked from the database in the
  form's own first step; a screenshot never decides them.
- **Ctrl+V**: a `document`-level paste listener that calls `preventDefault()`
  ONLY when the clipboard actually carried an image. A text paste anywhere in
  the app is completely unaffected.
- **Drag & drop** needed `dragDropEnabled: false` on the main window
  (`tauri.conf.json`) - Tauri's default of `true` means the OS handler swallows
  the drop and no HTML drop event reaches the webview. Verified first that
  nothing in the app used Tauri's own drag-drop event.
- **Cost control** (marko's own "Toto je DÔLEŽITÉ"), enforced on both sides:
  one analysis per explicit user action, a per-session image fingerprint cache
  so the same screenshot is never analyzed twice, editing extracted fields
  never re-calls, retry only on a click, exactly one API call per invocation in
  Rust with at most one transient retry, and no background or timed request
  anywhere.
- **Credentials**: reuses `ai_categorize.rs`'s build-time embedded
  `ANTHROPIC_API_KEY` - the key never crosses the IPC boundary and is never in
  the frontend. Without the secret the panel reports "AI import isn't available
  in this build" and everything else works normally.
- **Judgment calls, none asked about directly**: using `claude-opus-5` rather
  than `ai_categorize`'s Haiku (misreading a seat range is worse than not
  extracting it; the cost lever chosen instead was `effort: "medium"` - both
  are single constants at the top of `ai_import.rs`); putting an extracted
  order reference into the order's `notes` (the order code is backend-
  generated, so there is no other column for it); showing the panel on NEW
  events only; keeping `totalPrice` review-only rather than dividing it into a
  unit price; and matching an extracted category/platform/marketplace only
  against lookups that already exist, never creating one.
- **NOT verified by a build.** No Node.js and no Rust toolchain on the machine
  this was implemented on; `cargo test --lib`, `cargo check --lib`, `npx tsc -b`
  and `npm run build` were never run, and the 28 new Rust unit tests in
  `ai_import.rs` have never been executed. marko was asked and chose to proceed
  on static review. What WAS verified statically: bracket balance (with strings
  and comments stripped) in the new Rust module; JSX closing-tag structure
  unchanged in all three edited pages; every import resolving to a real export;
  no unused imports; the Anthropic request shape asserted by its own tests (no
  `temperature`, which this model rejects; `effort` and `format` as siblings
  inside `output_config`; image block before text block); and the workflow
  already passing `ANTHROPIC_API_KEY` to both build paths.

**2.6.0 - Complete visual redesign, UI/UX only.** marko's own task, and the
first round in this project's history whose explicit scope was "change how
everything looks, change nothing about what it does." His own constraints:
premium SaaS look, excellent readability, compact but not cramped, subtle
depth/borders, consistent spacing and status badges, good hover/focus - and
explicitly NOT: heavy gradients, glassmorphism everywhere, huge rounded
cards, flashy animation, a gaming look, too many colors. Plus a hard list of
things not to add (command palette, global search, notifications, AI,
monitoring, new widgets/modules/workflow).

- **The redesign is a shared LAYER, not a per-page pass.** Four files define
  the app's look now: `tailwind.config.js` (color ramps, a 3-step shadow
  scale, radius rhythm, motion budget), `src/index.css` (base typography +
  the component classes the pages already spell out by name), `ui.tsx` (the
  shared React controls) and `Layout.tsx` (the shell). Pages were only
  touched to ADOPT that layer - to delete a hand-rolled copy of something
  the layer now owns - never to give a page a look of its own.
- **Color**: the `slate` ramp was retuned (quieter blue-grey; dark mode's
  950/900 lifted and de-blued off `#020617`/`#0f172a` so background ->
  surface is a real step rather than near-black on near-black). This is the
  same one-file mechanism 2.0.56/2.0.58 used, so ~23k lines of existing
  `bg-slate-N`/`border-slate-N`/`text-slate-N` picked it up with no page
  edit. **`brand` was deliberately not touched, byte for byte** - `#4a68f7`
  is the accent marko confirmed in 2.0.56 and kept through the 2.0.58
  revert, and a palette change has already been tried and rejected once.
- **Tables** (marko called these out as the most important part): all 22
  tables in the app moved onto one `.table-shell`/`.table-flush` system -
  the identical "overflow-x-auto rounded-xl border ... shadow-sm" wrapper
  that had been copy-pasted around 14 of them is gone. That system owns the
  surface, an opaque sticky header (fixing the same class of dark-mode
  bleed-through 2.4.4 fixed once on a single page), internal vertical
  scrolling instead of page scrolling, one hover treatment and one
  `.row-selected` treatment. Column widths, every `colgroup` percentage set,
  and `useNarrowTables()`'s breakpoint are all untouched, and
  `.th-c-narrow`/`.td-c-narrow` keep their exact measured 2.0.37 metrics.
- **One tab pattern**: `SEGMENTED_TRACK`/`segmentedItemClass` (ui.tsx) is now
  the app's only segmented control. `TabSwitcher` uses it, and so do
  Dashboard's three previously hand-rolled rows (tab/period/metric),
  Welcome's Log in / Sign up switch, and Calendar's prev/Today/next cluster.
  The old solid brand-blue active fill is gone - a list filter shouldn't be
  the loudest control on the page.
- **Loading/empty**: new `Skeleton`/`TableSkeleton` in ui.tsx; six list pages
  (Orders, Sales, Tickets, Events, Pulls, Ticket Center) now hold their
  layout with a skeleton instead of collapsing to a centred spinner.
  `EmptyState` was restyled (medallion icon, real surface); every other
  `LoadingBlock` call site was left alone on purpose - a skeleton only helps
  where the shape of what's coming is known.
- **Motion**: everything lands in marko's 120-180ms band (Tailwind's
  `transitionDuration.DEFAULT`), and `prefers-reduced-motion` now collapses
  every transition and keyframe app-wide - including the toast exit, which
  still fires its `animationend` so `lib/toast.tsx`'s removal sequence keeps
  working rather than hanging.
- **Judgment calls, none asked about directly** (per marko's standing
  "smallest consistent solution, flag it" rule): retuning `slate` at all
  rather than only adding new tokens (it is the only way to reach every page
  without editing every page); capping `.table-shell` at
  `100vh - 13.5rem` with a `--table-inset` escape hatch, and giving Event
  Detail's stacked tables a shorter fixed cap instead; keeping the sidebar at
  `w-48` rather than widening it back; keeping the one-click light/dark
  toggle as a single button rather than a Sun/Moon segmented pair; and
  leaving Dashboard's already-unused `Button` import in place rather than
  taking an unrelated cleanup.
- **NOT verified by a build.** No Node.js and no Rust toolchain exist on the
  machine this was implemented on, so `npx tsc -b`, `npm run build` and
  `cargo check --lib` could not run; marko was asked and chose to proceed on
  static review. What WAS verified statically: `src-tauri/` byte-identical to
  2.5.2; bracket balance unchanged in all 25 edited frontend files; every
  `components/ui` import resolves to a real export; all 22 tables sit inside
  a shell/flush wrapper and none kept its own `<thead>` styling; every
  `shadow-*`/`theme()` reference resolves to a real config key; and the full
  diff contains no `api.` call, no state/handler/effect and no routing
  change. `Cargo.lock`/`package-lock.json` version entries were bumped by
  hand and should be regenerated.

**2.5.2 - "Forgot password?" via a deep link back into the app; Discord
sign-in deferred.** marko asked for two things after 2.5.1: a working
password-reset flow, and Discord as a third sign-in option alongside email/
Google. Both surfaced a real architectural fork, worked through with marko
directly via clarifying questions rather than guessed at - this entry covers
what actually got built.

- **Discord sign-in - deferred, not built.** Unlike Google, Firebase has no
  native Discord provider - keeping Discord accounts in the same
  Firebase-backed identity/approval-gate/per-account-database system as
  Google/email would need a new Cloud Function (Firebase Admin SDK to mint a
  custom token), which requires upgrading this project to Firebase's paid
  "Blaze" plan (a billing card on the project - real usage would stay inside
  the free quota, but the card itself is the cost marko weighed). marko chose
  to skip this for now rather than pay that cost. Nothing was built for this
  - see `PROTECTED_AREAS.md`'s "2.5.2" entry if picking this back up later.
- **Password reset - marko's first choice (a typed 6-digit code) turned out
  to need the exact same Cloud Function + Blaze plan as Discord**, since
  Firebase's client SDK can only send its own link-based reset email, never a
  custom short code, and can't change a password for a signed-out person
  without either that link's own oobCode or a trusted backend. This was
  surfaced back to marko directly rather than silently building the
  expensive version he'd just said no to for Discord, or silently
  downgrading to something else without telling him why - he picked a third
  option: keep Firebase's own link, but make it open TIQR Manager directly
  instead of a browser tab.
- **How the reset link reaches the app**: `sendPasswordResetEmail` now passes
  `handleCodeInApp: true` plus a `url` pointing at a new
  `docs/reset-redirect.html` (see `lib/firebase.ts`'s
  `PASSWORD_RESET_ACTION_CODE_SETTINGS` for the full chain) - published via
  the exact same GitHub Pages site `docs/privacy.html` already uses
  (`https://biznismarko9-source.github.io/tiqr-manager/`), so no new hosting
  concept was introduced. That static page immediately forwards its query
  string to `tiqrmanager://reset-password?...`, a new custom URL scheme
  registered via `tauri-plugin-deep-link` (`Cargo.toml`/`tauri.conf.json`/
  `capabilities/default.json`/`lib.rs` all touched - see `Cargo.toml`'s own
  comment on why `tauri-plugin-single-instance` needed its `deep-link`
  feature turned on too, so a link clicked while the app is already running
  reaches that instance instead of launching a second one). App.tsx's new
  `PasswordResetDeepLinkBridge` catches the incoming URL (both the cold-start
  case via `getCurrent()` and the already-running case via `onOpenUrl`),
  pulls out the `oobCode`, and navigates to a new `/reset-password` route.
- **New `src/pages/ResetPassword.tsx`** (outside `RequireAuth`, same as
  Welcome) verifies the incoming code (`verifyPasswordResetCode`), shows a
  "set a new password for X" form once it's confirmed valid, and finishes
  with `confirmPasswordReset` - all three calls are plain Firebase Auth
  client SDK, no backend involved anywhere in this feature.
- **Welcome.tsx gained a third, non-peer mode**: "Forgot password?" under the
  password field (login mode only) switches the whole card to a "forgot"
  mode (email field + send button + a confirmation message), with "Back to
  log in" as the only way out - it isn't a tab alongside Log in/Sign up the
  way those two are peers of each other.
- **One manual step this needs before it actually works**: add
  `biznismarko9-source.github.io` to Firebase Console -> Authentication ->
  Settings -> Authorized domains. GitHub Pages itself is very likely already
  on (it was required for the Google Sheets OAuth consent screen back in
  2.0.5 - see `REDESIGN-2.0.5-REPORT.md`) - worth confirming
  `docs/reset-redirect.html` actually loads before relying on this.
- **Judgment calls, none asked about directly**: the exact custom scheme name
  (`tiqrmanager`); keeping the redirector page mode-agnostic (forwards
  whatever `mode`/`oobCode` Firebase sends rather than hardcoding
  `resetPassword`, so a future email-verification feature could reuse the
  same page); reusing `docs/`'s existing GitHub Pages site rather than
  introducing Firebase Hosting (this repo deliberately has no Firebase CLI
  project wired up - see `firestore.rules`'s own comment - so a new deploy
  mechanism would have been a bigger, less consistent change than one more
  file on a site that already exists for exactly this kind of thing).

Verified: `npx tsc -b`, `npm run build`, `cargo check --lib` all clean;
`cargo test --lib` still green, full suite 1052 passed, 0 failed, 3 ignored
(unchanged - no new Rust logic worth unit-testing, only plugin wiring).

**2.5.1 - Ticket Center rebuilt around orders, sidebar reorder, Calendar
visual refresh.** marko's own direct feedback on the 2.5.0 release above,
delivered as its own round right after.

- **Ticket Center moved out of Finance.** It was a Finance subtab for
  exactly one version (2.4.4) - marko's follow-up made clear that wasn't
  where he wanted it. It's a standalone top-level page/route again
  (`/ticket-center`), positioned right after Finance in the sidebar.
- **Ticket Center rebuilt around ORDERS, not individual tickets.** Marko's
  own words: it should show orders you open to see what needs doing with
  which tickets, not a flat table of tickets/sale-batches you edit directly
  (which is what both `TicketControlCenter.tsx` (2.4.3) and
  `FulfillmentCenter.tsx` (2.2.12) did). Both old pages and the
  `finance/TicketCenter.tsx` subtab shell are deleted. The new
  `src/pages/TicketCenter.tsx` is a genuinely different page, not a
  restyle: it loads `OrderRecord[]` via the exact same `api.listOrders`
  Orders.tsx already calls (no new backend command, no new query), reuses
  Orders.tsx's own exported `orderCompletionChecks`/`completionStatus` for
  its "Completed" badge (so it can never disagree with Orders.tsx/
  OrderDetail.tsx about what "done" means), and clicking a row navigates
  straight to the existing `/orders/:id` (OrderDetail), which already lists
  every ticket in that order with independently-editable Status/Delivery
  status/Payout status - exactly "what needs to be done with which
  tickets," with no new detail view built or needed. 4 quick-filter tiles
  (Needs attention/listing/payment/delivery) fold both old pages' concerns
  into one set of order-level lenses - see `TicketCenter.tsx`'s own module
  doc comment for exactly how each is computed from `OrderRecord`'s
  existing counts. `OrderDetail.tsx`'s `backTo`/`backLabel` "arrived from"
  logic gained `/ticket-center` as a 4th recognized origin.
  `ticket_control_center.rs`'s backend command is now fully orphaned as a
  result (left in place, documented, not deleted - see `PROTECTED_AREAS.md`
  and the "Backend" bullet above).
- **Sidebar order changed to marko's exact list**: Dashboard, Tickets,
  Price Checker, Pulls, Finance, Ticket Center, Calendar - see `Layout.tsx`
  bullet above. Calendar moved from right after Dashboard (2.5.0) to last.
- **Calendar (2.5.0) visual refresh only** - "viac moderny, viac prehladny"
  (more modern, more legible), marko's own words. Every entry KIND (event/
  order/sale/pull/attention) now has one consistent accent color, used for
  the month/week grid's entry chips, the filter row (which doubles as the
  color legend - no separate legend UI needed), the Day Detail modal, and
  the Today/next-7-days summary. Severity (critical/attention/info/neutral)
  is now a second, independent channel layered on top (a ring on grid
  chips, colored/bold text in list views) instead of being the only signal
  a plain gray/red/amber dot used to carry. Weekend columns get a faint
  background wash, today's cell gets a soft brand tint (not just the date
  badge), and the Day Detail title now includes the weekday name. No hook,
  API call (`get_calendar` params unchanged), or navigation target changed
  - purely `src/pages/Calendar.tsx`'s own JSX/className, verified by
  reading the diff rather than a new test (this app has no frontend test
  runner - see "Testing" below).
- **Judgment calls, none asked about directly (per marko's own standing
  "smallest consistent solution, flag it" rule)**: the exact 4 Ticket
  Center categories and their thresholds (see `TicketCenter.tsx`'s own
  comment for the reasoning); dropping Control Center's tier/section/row/
  marketplace filters entirely (they don't apply once the grain is "one row
  per order"); dropping the old purchase-side `paymentStatus` column from
  the new table (already visible on Orders.tsx/OrderDetail, kept this page
  focused on the sell-side); leaving `ticket_control_center.rs` in place
  rather than deleting it; and building this as its own 2.5.1 round rather
  than folding it into a same-day 2.5.2/2.5.3 once more feedback lands (no
  new zip/release report was produced for 2.5.1 alone - marko signaled more
  changes were coming in the same message, see `CHANGELOG.md`'s own 2.5.1
  entry).

**2.5.0 - TIQR Operations Calendar.** marko's own spec for a new
cross-domain calendar page - delivered directly after the 2.4.4 UI/UX round
above, both shipped together in this same release (see "## Version" note
under 2.4.4/2.5.0 - no separate 2.4.4 zip was ever built).

- **Research first, per marko's own explicit instruction.** Before writing
  any code, all 8 of marko's named candidate categories were checked
  against the real schema AND the real, currently-shipping Rust command
  code (not just column names). Only 5 are backed by real, reliably-
  existing dates: **events** (`events.event_date`), **orders/purchases**
  (`orders.purchase_date`), **sales** (`sales.sale_date`), **pulls**
  (`pulls.event_date` - NOT the deprecated `pulls.transfer_deadline`
  column, unwritten since 1.9.8, see models.rs's own doc comment), and
  **attention** (`AttentionCenterItem.event_date`). The other 3 do NOT
  exist and are deliberately absent from the calendar rather than
  invented: **payouts** (no distinct entity/date anywhere - "Payout" is
  purely Google Sheets column-header text aliasing `Sale.sale_price_cents`/
  `Sale.payment_status`), **payments** (migration 007's `payments` table
  exists in the schema but has ZERO live Rust command code reading or
  writing it - the same schema-present-but-functionally-dead shape as
  migration 026's now-orphaned tables), **fulfillment** (`delivery_status`
  is a plain free-text enum with no associated date column at all). See
  `PROTECTED_AREAS.md`'s new "2.5.0" entry for the full detail.
- **Backend**: new `commands/calendar.rs` - one read-only command
  (`get_calendar`), taking an inclusive `dateFrom`/`dateTo` range (always
  the exact span of days the Month/Week grid has on screen) and returning
  a flat list of `CalendarEntry` (one per event/order/sale-batch/pull/
  attention-item, each with a title/subtitle, a severity, and a navigation
  target). Reuses `attention_center::get_attention_center_impl` directly
  (not re-derived) both for the "attention" entries themselves and to
  decide an "event" entry's own severity (critical exactly when that event
  is also in the SAME call's `event_soon` category - one threshold, one
  place). Sale entries are grouped by `sales::GROUP_KEY_EXPR` (one entry
  per batch, never one per ticket) with a simplified, safe "never blend
  currencies" amount (omitted whenever a batch's lines don't all share one
  `sales.currency` - same rule `SaleGroup` already follows, just without
  its extra profit/margin machinery, which a calendar card has no use
  for). No new migration, no new index - reuses the existing
  `idx_events_date`/`idx_orders_date`/`idx_sales_date` indexes; deliberately
  did NOT add one for `pulls.event_date` (none exists today either, and
  `pulls::list_pulls_impl`'s own already-shipping date filter has never
  needed one - marko's own "don't add an index without a reason").
- **Frontend**: new `src/pages/Calendar.tsx` + new `/calendar` route
  (`App.tsx`) + new flat sidebar entry right below Dashboard
  (`Layout.tsx`). Month view (a padded, Monday-first grid always covering
  full weeks) and Week view, with Today/Previous/Next controls - toggling
  either only refetches the calendar's own data for the new range, never a
  page reload. Each day cell shows up to a few entries (capped, Month: 3,
  Week: 6) plus a "+X more" that opens a Day Detail modal (reuses the
  existing shared `Modal` component) listing everything for that day.
  Filters row toggles the 5 real categories client-side (no extra network
  round trip per toggle - the whole visible range's data is already
  local). A "Today & next 7 days" summary card issues one extra,
  independent `get_calendar` call for that fixed range - same command, same
  data, never a duplicate computation. Every entry's click navigates
  straight to an EXISTING page (`/events/:id`, `/orders/:id`, `/sales/:id`,
  or the `/pulls` list - pulls has no per-record detail route today, same
  as before this task) - no new detail view was built or is needed.
- `cargo test --lib`: 14 new tests in `commands/calendar.rs` (range
  filtering incl. inclusive boundaries, sale-batch grouping/mixed-events/
  mixed-currency, event severity reusing Attention Center's own
  `event_soon`, pull severity mirroring Pulls.tsx's own warning window,
  attention navigation targets, multiple kinds on one day, empty state);
  full suite (1052 tests) green. `npx tsc -b` and `npm run build` both
  clean.

**2.4.4 - Ticket Center consolidation, sidebar regroup, theme toggle.**
marko's own request, a pure frontend/UX round - no backend, schema, or
migration changes at all.

- **Ticket Control Center (2.4.3) + Fulfillment Center (2.2.12) merged**
  into one "Ticket Center" tab under Finance, with two subtabs (Control
  Center, Fulfillment) - new thin shell `src/pages/finance/TicketCenter.tsx`
  mirrors Finance.tsx's own existing tab-shell pattern. Both components are
  reused completely unchanged internally (aside from Control Center's own
  fixes below) - no duplicate business logic, no new data loading. The
  `/control-center` and `/fulfillment` top-level routes and their sidebar
  entries are gone; neither is linked from anywhere else in the app (see
  `PROTECTED_AREAS.md`'s new "2.4.4" entry for the grep that confirmed
  this before the move).
- **Sidebar (`Layout.tsx`) regrouped**: Events/Orders/Tickets/Sales/
  Inventory now sit under one collapsible "Tickets" entry instead of 5 flat
  rows - the 5 routes themselves are unchanged, this is purely a nav
  grouping change. Judgment call: read as a sidebar accordion (keep every
  existing route as-is), not as "consolidate these 5 pages into one hub
  page with tabs" (the much bigger, riskier reading - see
  `PROTECTED_AREAS.md`'s new "2.4.4" entry).
- **One-click light/dark toggle** added above the sidebar's profile widget,
  reusing the existing `lib/theme.ts` `useTheme()` hook. Settings ->
  Appearance (the old 3-way Light/System/Dark picker) is now gone - moved,
  not duplicated. The new toggle is deliberately binary (system mode is
  still what a fresh install starts in, just not reachable from this
  control once you click it).
- **Ticket Control Center fixes** (same file, still nested under Finance
  now): (1) sticky header dark-mode background was `dark:bg-slate-800/60`
  (60% opacity, copied from this app's normal non-sticky `<thead>`
  convention) - fully opaque now, fixing scrolled row text bleeding through
  the sticky header while scrolling (marko's screenshot); (2) "Ticket /
  Seats" column renamed to "Seats", now shows only the seat location (the
  ticket code moved to a hover tooltip, still visible everywhere else -
  Order/Sale Detail); (3) the Order cell now independently opens Order
  Detail on click (`stopPropagation`), regardless of where the row's own
  click goes (Sale Detail for a sold ticket).
- **Dashboard's "Sales by platform" internal scrollbar removed** (marko's
  own request) - converted to the same slice + "Show N more" pattern the
  three Activity-tab Recent cards already use, rather than either an
  unbounded list or a nested scrollbar.
- `cargo test --lib` untouched/unaffected (no Rust changed this round);
  `npx tsc -b` and `npm run build` both clean.

**2.4.3 - Ticket Control Center.** marko's own focused-task spec: one
central work screen ("Ticket Control Center", `/control-center`) to manage
and check tickets across EVERY event at once - explicitly NOT a new
parallel ticket system, one convenient work view over EXISTING tickets
data. Scope was identified from `CURRENT_STATE.md`/`PROTECTED_AREAS.md` plus
the relevant Tickets/Orders/Listings/Sales/Inventory code and existing
filter/search/bulk logic only (no full repo scan, no old reports read),
per marko's own explicit efficiency instruction.

- **New backend module** `commands/ticket_control_center.rs` - ONE new
  read-only query, `list_control_center_tickets`, joining
  tickets->events->orders->(active) sales->(all) ticket_listings->
  marketplaces, following this codebase's own "each view aggregator writes
  its own SELECT" convention (`attention_center`/`inventory_intelligence`/
  `ticket_listings` already do this) rather than reshaping
  `tickets::list_tickets_impl`'s result. No new tables, no migration -
  purely additive reads, including one new signal (`isRefunded`, an
  `EXISTS` subquery) needed for the "Refunded" quick filter that no
  existing query could answer. See `PROTECTED_AREAS.md`'s new "2.4.3" entry
  for the full query design and every judgment call.
- **New frontend page** `src/pages/TicketControlCenter.tsx` - sticky
  filters/quick-filters/search above a dense table that owns its own
  scroll (first page in this app built that way), covering every column,
  filter, quick filter and search target marko's spec listed. Row click
  opens the existing Sale Detail (if sold) or Order Detail (otherwise) -
  no new detail page.
- **Bulk actions - 100% existing mechanisms, one small authorized
  extension**: generic field edit (Section/Row/**Tier**/Seat/Listing price)
  reuses the existing shared `BulkTicketEditBar`/`bulkUpdateTickets`,
  extended with a new `Tier` option marko explicitly asked for this round
  (`BulkTicketField::Tier`, models.rs/tickets.rs - previously left out on
  purpose, see `PROTECTED_AREAS.md`'s "2.2.7" and new "2.4.3" entries);
  "change listing status" reuses the existing, untouched
  `bulkUpdateTicketListingsStatus`; "export selected" reuses the existing,
  untouched `exportTicketsCsvSelected`. Refund/resell bulk actions were
  explicitly NOT added, per marko's own instruction.
- **Untouched, per marko's explicit "DÔLEŽITÉ" list**: refund/resell logic,
  `batch_id`, every money/cents column, Listings/Sales/Finance/Orders core.
  Section/Row/Seat remain metadata-only; Tier/Level remains a separate
  field - neither assumption was relaxed anywhere in this task.
- **Tests**: 12 new backend unit tests (11 for the new query - correct
  loading, the marketplace fan-out, refund/resell regression replaying
  BUG #1's own shape, the new `isRefunded` signal, and each filter/search
  dimension at least once; 1 for the new `Tier` bulk-edit arm). Full suite
  green: `cargo test --lib` (1038 passed, +12 over 2.4.2's 1026, 0 failed,
  3 ignored, same as before), `cargo clippy --lib` clean of new warnings
  (verified by exact line-context inspection of every touched/new file),
  `npx tsc -b` and `npm run build` both clean.
- Version bumped **2.4.2 -> 2.4.3**. No new migration (still 027 next).

See `PROTECTED_AREAS.md`'s new "2.4.3" entry for the full list of judgment
calls (the Payment-status filter's deliberate lack of a "Refunded" option,
the Listing-price coalesce, the combined Event/date cell, which columns
hide in narrow mode and why, the un-measured 68vh table height) and
`CHANGELOG.md`'s matching entry for a shorter summary.

**2.4.2 - Live Market Monitor removed; Price Checker back to manual-only.**
marko decided he does not want this feature in the app at all
("TÚTO FUNKCIU NECHCEM V APLIKÁCII VÔBEC") and asked for a full cleanup
back to a clean Price Checker with no background/scheduled/automatic
monitoring layer. Scope was identified from `CURRENT_STATE.md` and
`PROTECTED_AREAS.md` alone (no full repo scan), confirming "Live Event
Intelligence" needed zero further changes - it was already fully reverted
before 2.4.1 ever shipped, see the 2.4.1 section below - so this task's
entire footprint was 2.4.1's Live Market Monitor.

- **Removed entirely**: backend module `commands/price_checker_monitor.rs`
  (both its `#[tauri::command]`s and both scan-result hook call sites in
  `price_checker_scanner.rs`); the `market_alert` Attention Center category
  (`attention_center.rs` - reverted `push_item` to its original 2-shape key,
  Dashboard's 6th box and its "LIVE MARKET ALERTS" entry); Auto Monitor
  (ON/OFF + interval) and "Scan All" from `PriceChecker.tsx`; the Live
  Market Monitor panel and the Market History view/modal; every related
  Rust struct (`models.rs`) and TypeScript type (`types.ts`/`api.ts`).
- **Price Checker unchanged**: event selection, marketplace URLs/source
  handling, manual Visible Scanner, Market Analysis, tier/section grouping,
  price history (pre-existing, price-checker-session-scoped - never part of
  the deleted monitor), Your Tickets comparison. No redesign, no new
  features.
- **DB**: migration `026_price_checker_market_monitor.sql` and its 4 tables
  were **kept, not deleted** - already shipped in 2.4.1, forward-only
  migration rule, marko's own explicit instruction to never delete existing
  user data or invent a rollback. Only the application code that read/wrote
  them was removed. See `PROTECTED_AREAS.md`'s "2.4.2" entry and the
  "## Version" section above - next new migration is 027.
- **Two flagged judgment calls** (smallest-consistent-solution per
  ambiguity, no explicit spec either way): "Scan All" was removed as
  in-scope (built in 2.4.1 purely to complement Auto Monitor, no standalone
  purpose without it); `price_checker_analysis.rs`'s `group_by_tier` was
  left `pub(crate)` (bumped from private in 2.4.1 for the monitor module to
  reuse) rather than reverted to private, judged harmless residual visibility
  and not worth an extra touch to that protected file.
- **Tests**: full backend suite green at `cargo test --lib` (1026 passed,
  -32 removed with the feature, 0 failed, 3 ignored, same as before minus
  exactly the deleted tests), `npx tsc -b`, `npm run build` all clean.
- Version bumped **2.4.1 -> 2.4.2** (forward-only revert precedent, see
  "## Version" above - the resulting code is functionally the 2.3.5 feature
  set, but the version does not go back to 2.3.5).

See `PROTECTED_AREAS.md`'s new "2.4.2" entry and `CHANGELOG.md`'s matching
entry for the full reasoning. The 2.4.1 write-up below is kept as-is for
history (it explains why migration 026 and its tables exist) - **none of
the module/command/UI/struct/type names it describes exist in this
codebase anymore.**

**2.4.1 - Price Checker Live Market Monitor (REMOVED in 2.4.2 - kept below
for history only; see the 2.4.2 entry above).** marko's next spec after
2.3.5 went through two directions. The first, "Live Event Intelligence" (an
Event's own confirmed online identity on Viagogo/Vivid Seats/Ticombo, as a
brand-new standalone feature/module), was designed and built in full, then
marko reviewed the package and cancelled it outright ("Predchádzajúci nápad
'Live Event Intelligence' RUŠÍME ÚPLNE") before it ever shipped - every
trace of it (table, migration, backend module, TS types, EventDetail UI
block) was removed again. See `PROTECTED_AREAS.md`'s "2.4.0 (pre-release
direction)" entry and `CHANGELOG.md`'s matching entry if that original
design is ever useful as a reference for a differently-shaped future
feature - it is NOT in this codebase anymore, do not assume any file/table/
command it describes still exists.

What actually shipped as **2.4.1** instead (a brief reused-"2.4.0" stopover,
and why it moved one more step before delivery, is explained in "## Version"
above): marko's replacement instruction was to fold ALL online/live-market
functionality directly into Price Checker -
EVENT -> MARKETPLACE SOURCES -> SCAN -> SNAPSHOT -> HISTORY -> CHANGE
DETECTION -> MARKET ALERTS - built entirely on the already-shipped Visible
Scanner (2.1.9) and Market Analysis (2.2.0), never a new scraping surface:
no CAPTCHA bypass, no proxy rotation, no anti-bot workaround, and no
automation beyond reading whatever a human already has open in a real,
visible window.

- **New backend module** `commands/price_checker_monitor.rs` (migration
  `026_price_checker_market_monitor.sql` - 4 tables: `market_snapshots`,
  `market_snapshot_tiers`, `market_source_status`, `market_alerts`) - hooks
  into the Visible Scanner's own success/failure paths
  (`price_checker_scanner.rs`'s `record_scan_attempt_impl` call, both
  outcomes `let _ = ...`-swallowed so this can never affect the scan result
  marko actually sees). Every successful/partial scan writes one permanent,
  never-overwritten snapshot (overall + per tier, reusing
  `price_checker_analysis::group_by_tier` - Section/Row/Seat are metadata
  only, never a pricing factor, same hard constraint as everywhere else in
  Price Checker), then diffs it against the previous snapshot to raise
  MARKET DROP / MARKET RISE / NEW SUPPLY / SUPPLY DROP alerts at reused,
  transparent thresholds (5% price, 20% supply - the same constants Price
  Checker's own recommended-price and Inventory Intelligence's own
  outside-market logic already use). A SOURCE FAILURE alert fires only on a
  genuine success-to-failure transition, never on the first-ever failure or
  on repeated consecutive ones.
- **Auto Monitor + Scan All** (`PriceChecker.tsx`) - a per-marketplace ON/OFF
  toggle with a 15m/30m/1h/3h/6h interval that fires the exact same "Scan
  Visible Prices" call the button already makes, on a schedule, only against
  an already-open window it never opens itself; "Scan All" does the same
  once per marketplace that currently has a window open on the current
  event.
- **UI**: each marketplace card gained a Live Market Monitor panel
  (connection status, last successful scan - never cleared by a later
  failure, so the app stays useful fully offline on cached data - latest
  snapshot stats, Auto Monitor controls, recent Market Alerts) plus a
  "Market History" view of every saved snapshot.
- **Dashboard Attention Center gained a real 6th box, "LIVE MARKET ALERTS"**
  (`market_alert` category - a pure read of this module's own already-
  decided alerts, zero new market-calculation logic in `attention_center.rs`
  itself). Named differently from marko's own literal spec wording ("MARKET
  ATTENTION") because that title was already taken by the existing
  `outside_market_price` box (2.2.11, an unrelated feature) - see
  `PROTECTED_AREAS.md`'s "2.2.11" entry for that box's own "box title <->
  category mapping" note, now extended with this 6th one. Clicking a row
  jumps to Price Checker at that event/marketplace (scrolled into view and
  briefly highlighted) - no new separate dashboard.
- **Tests**: 32 new backend unit tests (27 in `price_checker_monitor.rs`, 5
  in `attention_center.rs`). Full suite green: `cargo test --lib` (1058
  passed, 0 failed, 3 ignored), `npx tsc -b`, `npm run build`.
- Not implemented, deliberately, per marko's own explicit scope: no new
  scraping/API integration with any marketplace beyond reading an already-
  open window; no automatic repricing anywhere; no new marketplace beyond
  the existing Viagogo/Vivid Seats/Ticombo three.

See `PROTECTED_AREAS.md`'s "2.4.1 - Price Checker Live Market Monitor"
entry for the full set of deliberate design decisions this feature was
originally built with (the naming collision, transition-only failure
alerting, the third `push_item` key shape, why reusing the migration
number "026" was verified safe, and why the app version itself ultimately
moved to 2.4.1 instead of staying on the also-verified-safe reused 2.4.0).
**This entire feature was removed again in 2.4.2** - see the 2.4.2 entry
above and `PROTECTED_AREAS.md`'s "2.4.2" entry; nothing described in this
2.4.1 section should be extended or built upon, it no longer exists.

**2.3.5 - Sync/push behavioral redesign: self-healing push, real diff-and-
update sync, and the UI-freeze fix.** After 2.3.4 shipped (row-placement
fixed and verified), marko came back with one detailed message re-explaining
the WHOLE intended sync/push design from first principles, using Pulls as
the reference model, because the narrow bug fixes so far hadn't addressed
his actual mental model of how this should work. Three separate things,
all in `orders_sheet_sync.rs`/`pulls_sheet_sync.rs` unless noted:

1. **Push Orders/Push Sales no longer freeze the whole app.** Root cause
   confirmed via this codebase's own precedent (`google_auth.rs`'s
   `start_google_sign_in`, 2.0.12->2.0.13, and `commands/notifications.rs`'s
   own module doc comment): all 11 sheet-sync `#[tauri::command]` functions
   (`sync_orders`/`push_orders`/`sync_sales`/`push_sales`/`force_push_sales`/
   `create_orders_sheet`/`setup_orders_sheet`, and Pulls' own 4 equivalents)
   were plain synchronous `fn`, and since they call blocking
   `reqwest::blocking` network I/O through Google Sheets, and Tauri runs a
   non-async command directly on its single main/IPC thread, EVERY click of
   any sync/push button froze the entire app until the network round-trip
   finished - marko: "ked zapnem alebo kliknem na cokolvvek ci uz sync alebo
   push tak apka zamrzne". Fixed by converting all 11 to `async fn` +
   `tauri::async_runtime::spawn_blocking`, taking `app: tauri::AppHandle`
   instead of `state: State<AppState>` and re-deriving the same managed
   state inside the closure via `app.state::<AppState>()` (needed because
   `AppState::db` is a bare `Mutex`, not `Arc`-wrapped, so `State` itself
   can't move into a `'static` closure) - zero changes to any `_impl`
   function's actual logic. The frontend (`Settings.tsx`'s shared sync/push
   card component) already had a correct per-button `busy` state/spinner/
   disable from way earlier - it was just neutered by the backend freeze; no
   frontend changes were needed at all.
2. **Order/Sales sync now diffs and updates an already-linked row**,
   matching what Pulls sync already did - marko: "ked je tam nejaka zmena,
   tak tiez to kukne a opravi poprípade updatne". Through 2.3.4, ANY marked
   row was skipped unconditionally with zero comparison (v1's original,
   deliberate creation-only scope-cut). New `OrderRowSnapshot` (mirrors
   Pulls' own `PullRowSnapshot`/`SyncLink` pattern exactly) tracks exactly 5
   fields - `platform`, the sheet's date, `currency` (as a label),
   `Email (used)`, `Order ID` - and on a genuine difference, applies it via
   the SAME `update_order_impl`/`OrderEditInput` the manual "Edit order" form
   already uses. **Deliberately excludes `quantity`/`Price Per Ticket`/
   `Total Purchase Price`** - unlike Pulls, Orders have real Tickets with
   exact-cent purchase cost allocated at creation time
   (`insert_order_with_tickets`), and this project's house rules already say
   not to touch that after the fact without asking first (same boundary the
   2.0.53 currency-push feature drew). A sheet-side change to any of those 3
   is simply invisible to this comparison - not flagged, not applied, exactly
   like before. Same two-sided-edit conflict check as Pulls
   (`order.updated_at > link.last_synced_at`). The legacy `'{}'` placeholder
   every already-linked order's `sheet_sync_links` row carried through 2.3.4
   needs no migration step: `OrderRowSnapshot` derives `Default` with a
   container-level `#[serde(default)]`, so `'{}'` parses as all-blank, looks
   "different" from any real row exactly once, and silently backfills a real
   snapshot via the ordinary update path (never an "unreadable snapshot"
   error). Sales sync (`apply_sales_rows`) was NOT changed - see point 3.
3. **Push Orders is now self-healing by marker** - marko, twice, explicitly:
   "ked nejake ID chyba ktore nieje v tabulke doplni ho, aj ked ho ja
   manualne odstranit a zas dam push, tak musi vediet ze zmizlo a doplnit ho
   tam". `apply_order_push`'s selection query used to be purely
   `sheet_sync_links`-driven (`WHERE NOT EXISTS (...)`) - completely blind to
   whether the sheet still actually had the row. Now every non-demo order is
   checked against the sheet's CURRENTLY-FETCHED marker set: never-linked ->
   append (unchanged case); linked AND marker still present -> untouched
   (unchanged case, still true); **linked but marker now missing anywhere in
   the sheet -> re-appended using the SAME existing code**, no new
   `sheet_sync_links` row (the old one was never wrong), reported via
   `result.corrected` so marko can see the app noticed. This is a deliberate,
   documented DIVERGENCE from `pulls_sheet_sync::apply_pull_push`'s own
   choice for the identical situation (Pulls reports an error instead,
   explicitly to avoid a possible duplicate) - marko's explicit, twice-stated
   words for Orders/Sales were treated as authoritative rather than
   re-asking, and Pulls itself was left untouched since he described it as
   already correct and asked specifically to move on to Orders/Sales. **Push
   Sales needed NO code change at all** for this - it never creates rows to
   begin with (`apply_sales_push_internal` only ever fills blank cells on a
   row that already exists), so once Push Orders restores a deleted order's
   row (its Sales-sync columns necessarily blank again), the very next Push
   Sales already re-fills them through its existing, unchanged "only write a
   fully blank cell group" rule - proven end-to-end by a dedicated test
   (`deleting_a_fully_sold_orders_entire_row_is_healed_by_order_push_then_
   sales_push_alone`) that chains both functions against one shared sheet
   row. This also resolves the dangling row-426 data-integrity gap
   `PROTECTED_AREAS.md`'s "2.3.2-2.3.4" entry flagged as needing a
   deliberate fix - see that entry, now marked resolved.

9 new/updated unit tests for the sync diff logic, 3 for push self-healing
plus the cross-function integration test above; full suite 1020/1020
passed, 0 failed; `tsc -b`/`npm run build` clean. See `PROTECTED_AREAS.md`'s
new "2.3.5" entry for the full reasoning, including the Pulls-divergence
call and what was deliberately left out of scope.

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

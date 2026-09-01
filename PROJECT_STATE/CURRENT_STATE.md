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

**2.2.4**, consistent across `package.json`, `src-tauri/tauri.conf.json`,
`src-tauri/Cargo.toml`, `release.ps1`'s `$Version`, and
`1-CLICK-UPDATE.bat` - see the version-bump checklist in
`PROTECTED_AREAS.md` ("2.1.6" entry) before ever bumping it by hand, there
are more places than the obvious 3 files.

## Stack / layout

- **Frontend** (`src/`): React + TypeScript + Tailwind, Vite build.
  Pages under `src/pages/`: Dashboard, Events, EventDetail (2.2.2-2.2.4:
  tabbed "Event Workspace" - Overview/Listings/Sales/Finance, see "Current
  focus" below and `PROTECTED_AREAS.md`'s "2.2.2"/"2.2.3"/"2.2.4" entries
  before adding more event-level functionality anywhere else), Orders,
  OrderDetail, Tickets (Inventory), Inventory, Sales, SaleDetail, Pulls
  (given/received), Finance (own `finance/` subfolder, 4-tab layout),
  PriceChecker, Settings, Welcome (auth), PendingApproval, DatabaseError.
  Shared: `src/types.ts`, IPC in `src/lib/api.ts`, auth in
  `src/lib/auth.tsx`, money/date parsing helpers in `src/lib/`.
- **Backend** (`src-tauri/src/`): Rust, Tauri 2. One module per domain
  under `commands/`: events, orders (+ `orders_sheet_sync`), tickets,
  ticket_listings (2.2.4 - real per-marketplace listings, see "Current
  focus" below), sales, event_categories, pulls (+ `pulls_received`,
  `pulls_sheet_sync`), finance_accounts/finance_entries (2.2.1: entries can
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
  currently through **022_ticket_listings.sql**. Migrations run
  automatically at startup, forward-only.
- **Packaging**: `release.ps1` (invoked via `1-CLICK-UPDATE.bat`) mirrors
  this folder into a fresh clone of the real GitHub repo, cross-checks the
  version in 3 files, commits, tags, and pushes - the tag push triggers
  the signed Windows installer build in GitHub Actions. Exclusion list for
  any manual zip matches `.gitignore` (node_modules, dist*, target, gen,
  logs, etc).

## Current focus / most recent work

**Event Workspace (2.2.2, revised 2.2.3 and 2.2.4).** `EventDetail.tsx` is
a tabbed "Event Workspace" (`TabSwitcher`, same component Tickets.tsx/
Events.tsx already use for their own tabs) - current, final shape:
**Overview | Listings | Sales | Finance**:
- **Overview** shows marko's own stat list (tickets, sold, available,
  total cost, revenue, profit, margin, ROI - `EventWithStats.stats`, no
  backend change), plus (2.2.4) the Orders + Tickets tables that used to
  be their own "Inventory" tab, appended below - marko's own "spoj do
  jedneho" (merge into one) instruction. Both halves are unchanged from
  their previous tabs, just relocated into one.
- **Listings** (2.2.3: read-only Ticket view; **rebuilt in 2.2.4 into a
  real system**) is now backed by a real `ticket_listings` table
  (`migrations/022_ticket_listings.sql`, `commands/ticket_listings.rs`) -
  one ticket can have several listings at once, one per marketplace
  (StubHub/Vivid/Ticombo-style), each with its own price/currency/status/
  listing id/listing URL/last-updated timestamp. Reuses the EXISTING
  `marketplaces` lookup table (Price Checker's own) rather than a second
  marketplace concept. Full CRUD (add/edit/delete a listing) lives in this
  tab; summary cards (Active listings/Listed value/Lowest/Highest) count
  `status === "active"` listings only, while the table below shows every
  listing regardless of status. Deliberately still manual entry only - no
  marketplace API, no automatic listing creation, no repricing. Does NOT
  touch `tickets.status`/`tickets.listingPriceCents` at all - those stay
  exactly what they were. See `PROTECTED_AREAS.md`'s "2.2.4" entry before
  touching this table or `delete_marketplace_impl`'s guard again.
- **Sales** calls `list_sale_groups({ eventId })` (Sales.tsx's own Event
  filter, reused) for its own table, plus (2.2.4) the former **Market**
  tab's entire content appended below - "Market vs. mine"
  (`get_price_checker_summary`) and "Potential Profit" (this page's own
  unsold-stock estimate, unchanged calculation). Market's name/tab is gone
  - see `PROTECTED_AREAS.md`'s "2.2.4" entry for why its content landed in
  Sales rather than Finance (a judgment call, flagged to marko).
  "Open in Sales"/"Open in Price Checker" still link out for anything more
  than a glance.
- **Finance** calls `list_finance_entries_for_order` (2.2.1) once per this
  event's own orders and merges client-side - completely unchanged since
  2.2.2; marko's own final tab list keeps this as its own tab, not folded
  into anything.

2.2.3 removed the **Tasks** tab entirely (marko decided against it before
it ever got a spec) and removed the `max-w-[1400px]` cap from this page's
tables so they fill the window width (2.0.31's `Layout.tsx` fix, extended
here). 2.2.2 also shipped three unrelated small fixes: Settings -> Lookups'
3 category lists no longer cap their scroll area at a fixed 224px
(`max-h-[60vh]` now); Price Checker's event picker only lists
`status === "upcoming"` events (same field Events.tsx's own Upcoming/
Completed tabs use) - a completed/cancelled event just quietly stops
showing up there, no manual untracking needed.

Read `PROTECTED_AREAS.md`'s "2.2.2"/"2.2.3"/"2.2.4" entries before adding
to any of these tabs. 2.2.2/2.2.3 were frontend-only; 2.2.4 adds one new
table + 4 new commands (`ticket_listings`) and extends
`delete_marketplace_impl`'s existing guard - no other backend surface
changed.

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
`*-REPORT.md` file at the repo root (Slovak, written for marko) - 110 of
them as of 2.2.4. These are not read by default under this protocol; only
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

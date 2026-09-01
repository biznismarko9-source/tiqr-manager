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

**2.2.6**, consistent across `package.json`, `src-tauri/tauri.conf.json`,
`src-tauri/Cargo.toml`, `release.ps1`'s `$Version`, and
`1-CLICK-UPDATE.bat` - see the version-bump checklist in
`PROTECTED_AREAS.md` ("2.1.6" entry) before ever bumping it by hand, there
are more places than the obvious 3 files. (Briefly shipped as an
un-bumped, code-labeled-only "2.2.6" first - marko's closing checklist for
that task didn't ask for a version bump - then bumped for real, same
session, once he confirmed he wanted the usual release file too. See
`PROTECTED_AREAS.md`'s "2.2.6" entry.)

## Stack / layout

- **Frontend** (`src/`): React + TypeScript + Tailwind, Vite build.
  Pages under `src/pages/`: Dashboard, Events, EventDetail (2.2.2-2.2.5:
  tabbed "Event Workspace" - Overview/Listings/Sales (Finance folded into
  Sales in 2.2.5), see "Current focus" below and `PROTECTED_AREAS.md`'s
  "2.2.2"/"2.2.3"/"2.2.4"/"2.2.5" entries before adding more event-level
  functionality anywhere else), Orders,
  OrderDetail, Tickets (Inventory), Inventory, Sales, SaleDetail, Pulls
  (given/received), Finance (own `finance/` subfolder, 4-tab layout),
  PriceChecker, Settings, Welcome (auth), PendingApproval, DatabaseError.
  Shared: `src/types.ts`, IPC in `src/lib/api.ts`, auth in
  `src/lib/auth.tsx`, money/date parsing helpers in `src/lib/`.
- **Backend** (`src-tauri/src/`): Rust, Tauri 2. One module per domain
  under `commands/`: events, orders (+ `orders_sheet_sync`), tickets,
  ticket_listings (2.2.4 - real per-marketplace listings; 2.2.5 added 3
  all-or-nothing bulk commands - status/price/delete - see "Current focus"
  below), inventory_intelligence (2.2.6 - one read-only command backing
  Overview's "Inventory Intelligence" block, see "Current focus" below),
  sales, event_categories, pulls (+ `pulls_received`,
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
  currently through **023_add_seatriks_marketplace.sql**. Migrations run
  automatically at startup, forward-only.
- **Packaging**: `release.ps1` (invoked via `1-CLICK-UPDATE.bat`) mirrors
  this folder into a fresh clone of the real GitHub repo, cross-checks the
  version in 3 files, commits, tags, and pushes - the tag push triggers
  the signed Windows installer build in GitHub Actions. Exclusion list for
  any manual zip matches `.gitignore` (node_modules, dist*, target, gen,
  logs, etc).

## Current focus / most recent work

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
- **Breakdown** by section (unsold tickets) and by marketplace (active
  listings). **No "by tier" breakdown** - `tickets` has no tier/level
  column anywhere in this schema (`ticket_type` is a delivery method, not
  a price tier - the same gap PROTECTED_AREAS.md's 2.2.0 entry already
  flagged); the UI says so in plain text rather than inventing one. See
  `PROTECTED_AREAS.md`'s new entry for the smallest proposed fix.
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
`*-REPORT.md` file at the repo root (Slovak, written for marko) - 112 of
them as of 2.2.6. These are not read by default under this protocol; only
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

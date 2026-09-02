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

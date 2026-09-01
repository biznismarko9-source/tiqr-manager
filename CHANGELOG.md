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

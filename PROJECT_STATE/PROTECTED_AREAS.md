# Protected areas - notes for future sessions

Moved here from the old root-level `PROTECTED-AREAS-NOTES.md` (same
content, same convention, just relocated so all three state files live
together per the TIQR development protocol - see `CURRENT_STATE.md` and
`KNOWN_BUGS.md` in this same folder). A running checklist of things in
this codebase that are easy to break, or easy to forget, when touching
the areas they cover - not a changelog (see `CHANGELOG.md` for that going
forward, or the `REDESIGN-*-REPORT.md` / `*-REPORT.md` files at the repo
root for the detailed per-release history), just traps worth knowing
about before working here again. New entries go at the top, dated by the
version that found them.

2026-09-01 reconciliation note: this file merges two parallel copies that
grew independently in different sessions after 2.0.80 - the dated,
version-tagged entries below (2.1.6 through 2.2.0) came from the session
that built Finance/Price-Checker/Market-Analysis; the undated "Long-standing
invariants" section at the bottom came from a separate session's own
bootstrap of this same file, written from first-hand knowledge of the
older financial/orders/Sheets-sync code that the 2.1.x/2.2.0 work never
touched (so it never needed writing about there). Both halves are real and
current - nothing here is superseded, they just cover different areas.

## 2.2.2 - EventDetail.tsx is now the "Event Workspace" (tabbed)

`EventDetail.tsx` went from one long scrolling page to 6 tabs (Overview,
Inventory, Sales, Market, Finance, Tasks), via the same `TabSwitcher`
component Tickets.tsx/Events.tsx already use for their own tabs. A few
things worth knowing before extending this further:

- **Sales, Market and Finance each fetch their own data independently,
  inside their own tab component's `useEffect`, keyed on the tab
  component actually being mounted.** There is no lifted/shared state and
  no caching - switching away from a tab and back re-fetches it every
  time. Deliberate (kept simple, matches "iba základ"); revisit only if
  marko reports it feeling slow, not preemptively.
- **The Finance tab has NO new backend command.** It calls
  `list_finance_entries_for_order` (2.2.1) once per this event's own
  `orders` and merges the results client-side. Fine at event scale (a
  handful of orders); do not copy this N-calls pattern anywhere that could
  see more than a handful of rows - add a real
  `list_finance_entries_for_event`-style command instead if that ever
  comes up.
- **Inventory intentionally does NOT use the shared `TicketsView`
  component** (`Tickets.tsx`, also used by `Inventory.tsx` the sidebar
  page, via its `lockedStatus` prop). It reuses this page's OWN
  already-existing Orders + Tickets tables instead, unchanged, just moved
  under a tab. This was a deliberate scope call to avoid touching a
  component two other pages depend on - if a future ask wants the full
  Tickets toolset (search/sort/bulk actions) scoped to one event, that
  means adding a `lockedEventId`-style prop to `TicketsView` (mirroring
  `lockedStatus`), not duplicating its logic here.
- **Market's "Market vs. mine" card only renders when
  `get_price_checker_summary` returns a non-null `marketLowestPriceCents`**
  - i.e. it's hidden entirely rather than shown with a "no data yet"
  message, unlike PriceChecker.tsx's own always-visible version of the
  same card. Intentional (an empty card reads as clutter here); keep this
  in mind if the two ever need to look identical.
- **Tasks is a placeholder only** (`EmptyState`, no schema, no commands,
  no types). Building the real feature means starting from nothing - there
  is no partial implementation to extend.

## 2.2.2 - Price Checker's event picker now filters to `status === "upcoming"`

`PriceChecker.tsx`'s own `events` list (used only for its "Event" picker)
now excludes anything not `"upcoming"` - reusing the exact field
Events.tsx's own Upcoming/Completed tabs use (2.0.59), not a new
date-derived rule. This means an event still shows up here until marko
manually marks it "completed"/"cancelled" elsewhere (Edit modal) - it is
NOT automatically inferred from `event_date` having passed. If marko ever
wants that to happen automatically from the date alone (no manual status
flip required), that is a different, bigger change - status is currently
a plain manually-set field everywhere else in this codebase too (Orders/
Sales payment status work the same way), so auto-flipping it here alone
would be a new, inconsistent behavior, not a bug fix.

## 2.2.1 - Finance entries can now link to an Order

`finance_entries.order_id` (`migrations/021_finance_entry_order_link.sql`)
is marko's own explicit, confirmed decision to reverse ONE part of
`015_finance.sql`'s original design - that migration's own doc comment
argued Finance should be "fully independent" of Orders/Tickets/Sales
specifically to avoid double-counting. A few things worth knowing before
extending this further:

- **This stays a soft reference, not a merge of the two ledgers.** Nothing
  anywhere sums a Finance entry's `amount_cents` together with an order's
  `total_cost_cents` into one combined total - Dashboard and Finance
  Reports remain two completely separate views, unchanged by this
  migration. If a future change ever DOES combine them into one number
  (e.g. a "total spent on this event, Finance + Orders" report), it must
  either exclude order-linked Finance entries from that sum or it will
  double-count real money - re-read this note before building that.
- **`OrderDetail.tsx`'s "Record in Finance" modal locks Amount/Currency to
  the order's own `total_cost_cents`/`currency` - deliberately not
  editable there.** This is the actual mechanism that makes "aby to
  sedelo" (so the two reconcile) true by construction rather than by
  convention: the two numbers cannot legitimately drift apart because the
  Finance side is never independently typed in. Every other Finance entry
  creation path (`Transactions.tsx`'s own "New entry", recurring expenses)
  is completely untouched and still free-typed, same as before.
- **No UNIQUE constraint, and no currency-match requirement, on
  `order_id`** - unlike `account_id` (`validate_account`, which DOES
  require a currency match because an account has a running balance an
  entry directly feeds), `validate_order` (`finance_entries.rs`) only
  checks the order exists. More than one Finance entry may point at the
  same order (e.g. a deposit now, the balance later - marko may
  reasonably want this), and a Finance entry could in principle be
  recorded in a different currency than its linked order after a
  conversion. Don't add either constraint without checking this reasoning
  still holds.
- **Editing an order-linked entry from `Transactions.tsx`'s `EntryFormModal`
  must round-trip `orderId` unchanged** - that form has no UI to set or
  clear the link, so its submit handler explicitly carries
  `initial?.orderId ?? null` through rather than defaulting to `null`
  always, which would silently unlink an entry the first time marko
  merely fixed a typo in its note. The currency-conversion re-save in
  `Overview.tsx` has the same requirement and the same fix. If a THIRD
  place ever writes a `FinanceEntryInput` for an existing entry, it needs
  the same care.

## 2.2.0 - Price Checker Market Analysis (built on the Visible Scanner)

`commands/price_checker_analysis.rs` is a pure/derived-data layer over a
scanner session's already-accumulated `NormalizedListing`s - it never
opens a window, runs an eval, or touches `ScannerSession`'s own fields
beyond reading `listings`. Read that module's own doc comment first. A
few things worth knowing before touching this area again:

- **Tier/section grouping must be case-insensitive, even though the
  underlying labels are raw, unnormalized DOM text.** `tierFor`
  (`price_checker_scan.js`) returns whatever text the page itself showed,
  with no case normalization - the exact same real tier can legitimately
  render as "Level 100" in one spot and "LEVEL 100" in another on the same
  page (a legend vs. a listing row is a realistic way this happens).
  `classify_comparable` already compares labels case-insensitively via
  `same_str`, so `group_by_tier`/`group_by_section` must agree - grouping
  on the raw (merely trimmed) string found and fixed during this
  release's own review pass, since it would otherwise silently split one
  real tier/section into two rows in the breakdown. Both functions now key
  on a lowercased form internally while displaying whichever casing was
  seen FIRST (same "first occurrence wins" convention `compute_scan_stats`
  already uses for currency) - if you add another place that groups by a
  page-sourced label, it needs the same treatment, not a plain
  `HashMap<String, _>` keyed on the trimmed-only string.
- **`data_quality` and comparable `level` are two independent
  classifications and must never gate each other.** A listing can be
  `"exact_comparable"` (its section matches the reference) while its own
  `data_quality` is still `"partial"` (nothing else about it is
  confirmed) - marko's own priority order checks section first,
  unconditionally, before tier/row/quantity even come into it. An earlier
  draft of this feature gated one on the other and was corrected before
  anything depended on it (see `RankedComparable`'s own doc comment,
  models.rs) - don't reintroduce that coupling.
- **`ComparableReferenceInput.currency` is required, not optional** - a
  deliberate addition beyond marko's own literal spec wording, applying
  the same "never blend EUR/USD/GBP" rule this whole feature follows
  everywhere else to the comparable-ticket flow specifically. If this
  type is ever reshaped, keep `currency` required; making it optional
  reopens a currency-blending hole in `rank_comparable`.
- **`YourTicketGroup.tier` is always `None` - this is checked, not
  missing.** The real `tickets` table has `section`/`row_label`/`seat`
  but no tier/level column at all; `ticket_type` looks like it could
  stand in for one but is actually a DELIVERY method ("E-ticket"/"PDF"/
  "Mobile transfer"/"Physical"/"Will call" - see `TICKET_TYPES`,
  Orders.tsx). Do not wire `ticket_type` in as a tier source without
  first adding a REAL tier/level column to `tickets` - it would silently
  produce nonsense groupings.
- **`compute_market_analysis` is the first command that needs both
  `AppState::price_scanner_sessions` and `AppState::db` - it locks
  `price_scanner_sessions` just long enough to clone the session's
  listings out, then drops that lock BEFORE acquiring `db`.** Never hold
  both at once; that ordering (plus never taking them in the opposite
  order elsewhere) is the entire deadlock-safety argument, not anything
  more clever.
- **`migrations/020_remove_stubhub.sql` fully deletes the StubHub
  marketplace row and its whole history** (marko's own explicit,
  confirmed-twice decision - 2.1.6 kept history on purpose, 2.2.0 goes
  further on request). It explicitly deletes child rows
  (`price_check_tiers` -> `price_checks` -> `event_marketplace_links`)
  before the parent `marketplaces` row, in a transaction, even though the
  existing `ON DELETE CASCADE`s (from `014_price_checker.sql`) would have
  done the same thing on their own with `PRAGMA foreign_keys = ON` -
  belt-and-suspenders, not a workaround for a real gap. If StubHub (or
  any other marketplace) is ever fully deleted again, follow this same
  explicit child-first pattern rather than trusting cascade alone, and
  grep the migrations for every table with a `marketplace_id` column
  first - a new one added later that ISN'T `ON DELETE CASCADE` would
  make a future full-delete either orphan rows (if FK enforcement were
  ever off) or hard-fail the whole migration (with it on, which is TIQR's
  actual setting) and leave the app unable to start for anyone with
  matching history.

## 2.1.9 - Price Checker: hidden auto-check replaced by the Visible Scanner

The ENTIRE 2.1.1-2.1.8 hidden-WebView auto-check line (`price_checker_auto*`)
is deleted, not just disabled - marko's own explicit call after it kept
being unreliable no matter how the extraction logic was patched. In its
place: `commands/price_checker_scanner.rs` (session state + the 4 Tauri
commands) and `commands/price_checker_scan.js` (the injected 3-layer
extraction script) open a real, visible `WebviewWindow` the user drives
himself. Read that module's own doc comment first for the session model
and status-derivation rules before changing anything here. A few things
that bit during this rewrite, worth knowing before touching this area
again:

- **`WebviewWindow::eval_with_callback`'s result is encoded TWICE, not
  once, when the injected script itself returns `JSON.stringify(x)`.**
  `eval_with_callback`'s own doc comment says the JS completion value "will
  be serialized into a JSON string" for the callback - true, but
  `price_checker_scan.js`'s completion value is ITSELF already a JSON
  string (`return JSON.stringify(payload)`), so the callback's raw `String`
  is a JSON string LITERAL wrapping another JSON string. A single
  `serde_json::from_str::<ScanJsPayload>(&raw)` fails on literally every
  real scan with "invalid type: string ..., expected struct ScanJsPayload"
  - it silently never worked until caught by REAL runtime verification (a
  genuine `WebviewWindow` under Xvfb; no pure-Rust unit test or jsdom-only
  JS test can reach this, since both sides deserialize correctly in
  isolation - only the real Tauri wire format has the extra layer). Fixed
  in `parse_scan_js_payload` (two `serde_json::from_str` calls, unwrap the
  outer JSON string first) - if the injected script is ever changed to
  return a plain object completion value instead of a stringified one,
  this needs to come out again, and re-verify with a real WebviewWindow run
  when you do, not just the unit tests.
- **A session's `cancel_flag` must be REPLACED (a new `Arc`), never reset
  in place, at the start of each scan attempt.** Resetting the same shared
  `AtomicBool` back to `false` when a new scan starts can silently undo an
  in-flight scan's own Stop: scan A is blocked waiting on
  `eval_with_callback`, the user clicks Stop (sets the flag true), then
  clicks Scan again before A's result arrives - if the new attempt just
  flips the SAME flag back to false, A's own cancel check (after its eval
  finally returns) wrongly reads "not cancelled" and merges in a result the
  user explicitly stopped. `scan_visible_prices` now does
  `session.cancel_flag = Arc::new(AtomicBool::new(false))` so each attempt
  owns an independent flag; `cancel_price_scan`/`finish_session` still just
  flip whatever the CURRENT one is.
- **The dedup fingerprint is a literal composite of marketplace + price +
  currency + section + row + quantity + listing id (`fingerprint_for`,
  same file)** - every field feeds the key, missing ones contribute an
  empty slot. This means a listing whose section/row genuinely isn't
  visible yet on one scan, then becomes visible on a later scan of the
  SAME physical listing, is counted as a SECOND listing rather than
  recognized as the same one maturing from "partial" to "success" - an
  accepted, documented trade-off of marko's own literal fingerprint spec,
  not something silently patched over with a heuristic (a heuristic here
  risks the opposite, worse mistake: wrongly merging two genuinely
  different listings that happen to share a price, e.g. a
  general-admission listing and a specific-seat listing). If this ever
  needs real fixing, it needs actual cross-scan listing identity, which
  most real pages don't expose - not a quick tweak.
- **Proving this feature actually works needs a REAL `WebviewWindow`, and
  that needs `Builder::any_thread()` plus `App::run_return` under
  `xvfb-run -a`, run as a temporary/throwaway test.** `cargo test`'s
  harness runs each `#[test]` on a pooled worker thread, not the true OS
  main thread that `tao`'s Linux event loop insists on by default -
  `Builder::any_thread()` is the documented escape hatch, and it's only
  safe/appropriate for a test harness like this, never for the real
  shipped app (which legitimately starts on the real main thread via
  `fn main`). `App::run_return(callback)` (not `.run()`, which never
  returns) pumps the real event loop on the calling thread and hands control
  back once something calls `AppHandle::exit()` - drive the actual
  `#[tauri::command]` functions directly from a spawned thread inside
  `.setup()` (they're plain callable Rust functions), poll `AppState`/
  listen for the real Tauri events, and DELETE the throwaway test module
  again once it's done its job - this is not something to leave enabled in
  the permanent suite (it opens a real window and binds a real local
  socket for a fixture HTTP server; keep it out of ordinary `cargo test`
  runs, including `--ignored`, by not committing it at all).

## 2.1.8 - Price Checker real-DOM reader rewrite

`price_checker_auto_extract.js`/`price_checker_auto_readiness.js` were
rewritten around per-marketplace readers (StubHub/Vivid Seats/Ticombo, each
with its own layered selectors, all falling through to `readGeneric()` as a
last resort) instead of one generic parser, plus a bounded multi-attempt
retry loop in `poll_then_extract`. A few things that bit during this work,
worth knowing before touching this area again:

- **Never concatenate sibling elements' text without a separator when
  scanning for context.** `.textContent` on a parent with adjacent
  `<span>` children (exactly how JSX compiles
  `<span>{a}</span><span>{b}</span>`, with zero whitespace between them)
  runs their text together with NOTHING between them - "Row 12" next to "2
  tickets" reads as "122 tickets" and silently reports the wrong quantity,
  not just messy text. `nearbyListingContext` now uses `textWithGaps` (an
  element-boundary-aware walker) instead of plain `textOf`/`.textContent`
  for exactly this reason - if you add another context-scanning regex, feed
  it `textWithGaps`, not `textOf`.
- **A price parser is not done until it's been checked against a European
  decimal-comma format.** `parseMoney`'s original number parsing did
  `parseFloat(numRaw.replace(/,/g, ""))`, which silently treats a comma as
  a thousands separator always - correct for "$1,234.56", silently 100x-1000x
  wrong for "234,56 €" (marko is in Slovakia; this is the format he
  actually sees). `src/lib/priceParse.ts`'s `normalizeAmountToken` already
  had the correct locale-aware logic (look at the LAST separator and how
  many digits follow it) - `price_checker_auto_extract.js` now has its own
  plain-JS port of the same algorithm (can't `import` a TS module into a
  webview eval string). If either file's money parsing is touched again,
  re-verify BOTH "234,56 €" and "$1,234.56" still come out right, not just
  one format.
- **"ok" now specifically means "a price correlated with real section/row/
  seat context", not just "a price was found".** A bare, uncorrelated price
  is `"partial"` (own status, own amber banner in PriceChecker.tsx, still
  prefills the editable fields) - this rule applies to EVERY path that can
  produce an `AutoCheckResult`, including the AI-assisted fallback
  (`try_ai_extraction_fallback`), which structurally can never populate
  `listings` (its prompt/schema has no section/row/seat slot at all) and so
  can never legitimately be `"ok"` either. If a future change adds a new
  way to produce a result, it needs to honestly decide ok vs. partial by
  this same rule, not default to "ok" because prices exist.
- **An extraction attempt's own eval timeout must NEVER be derived from a
  shrinking remaining-budget clock** - this is the THIRD time this exact
  lesson mattered (2.1.6, 2.1.7, and again while adding the 2.1.8 retry
  loop, where the between-attempts readiness/scroll eval - not the
  extraction eval itself - needed a budget cap for a DIFFERENT reason: its
  result is always discarded, so shrinking IT is safe, but it still needed
  the cap to stop it silently adding up to a whole extra `EVAL_TIMEOUT` of
  overshoot on top of the documented ~63s ceiling). The rule stands: only
  a boolean "is this the last attempt" may depend on remaining budget: no
  eval whose RESULT is actually used may ever get a shrunken timeout.
- **Diagnostics text/attribute scrubbing is a defense-in-depth heuristic,
  not a guarantee** - `scrubSensitiveText`/`stripSuspiciousAttributeValues`
  in the extract script catch labeled patterns (Bearer/JWT/token=/session=)
  plus generic long opaque runs (all-digit 16+, all-letter 24+, mixed
  base64-alphabet 24+), but deliberately EXCLUDE hyphens/dots from the
  generic mixed-run check specifically so ordinary hyphenated section/row
  slugs ("grandstand-outfield-413") survive - don't widen that character
  class back to include hyphens without re-testing against real listing
  markup, or legitimate diagnostic detail silently disappears again.

## 2.1.6 - a version bump is not just the 3 JSON/TOML files

The obvious version-number locations are `package.json`, `src-tauri/tauri.conf.json`
and `src-tauri/Cargo.toml` (all three - `release.ps1` itself cross-checks
that all three agree, see below). It is easy to stop there and still ship a
broken or misleading release. Also check, every time:

- **`release.ps1`'s `$Version` constant.** This drives the actual git tag,
  and `release.ps1` HARD STOPS if `$Version` (with its `v` stripped) doesn't
  match what it finds in the 3 files above after mirroring this folder into
  a fresh clone - so forgetting to bump it is caught, but with a confusing
  "this clone does not actually have vX.Y.Z everywhere / $SourceDir is
  stale" message that points at the wrong cause. Bump `$Version` itself,
  don't rely on that check to remind you.
- **`release.ps1`'s `$CommitMsg`.** This is a fully static string, not
  generated from anything - it describes whatever release it was LAST
  written for. If it isn't rewritten, the git tag for the new version ships
  with a commit message describing the PREVIOUS release's changes instead.
  Easy to miss because nothing fails or warns - the script runs fine either
  way, it just publishes a misleading commit message forever.
- **`1-CLICK-UPDATE.bat`'s title/echo text.** Purely cosmetic (the actual
  release mechanics come entirely from `release.ps1`), but it has its own
  hardcoded `vX.Y.Z` strings that do not follow `release.ps1`'s `$Version`
  automatically - found still saying "v2.1.3" while 3 real releases (2.1.3,
  2.1.4, 2.1.5) had already shipped without it ever being updated.
- **`Cargo.lock` / `package-lock.json`.** Not hand-edited - after bumping
  `Cargo.toml`/`package.json`, run `cargo check` (regenerates the
  `tiqr-manager` entry in `Cargo.lock`) and `npm install --package-lock-only`
  (regenerates the root `""` package entries in `package-lock.json`,
  currently 2 of them) so the lockfiles don't silently drift from the
  manifests. `package-lock.json` also contains unrelated third-party
  packages that happen to share a version number with the app (e.g.
  `@nodelib/fs.scandir` was genuinely at `2.1.5` too) - don't touch those.

None of this is enforced by a test; it was found by grepping the whole repo
for the outgoing version string right before packaging 2.1.6 and reading
`release.ps1` in full rather than assuming its only version reference was
the obvious `$Version` line.

(2.1.6 is the oldest dated entry that existed in this file - this move
carried over the complete original content, nothing was cut.)

## Long-standing invariants (not tied to one dated release)

The areas below predate this file's dated-entry convention and were never
re-visited by the 2.1.x/2.2.0 work above, so they have no "found in vX.Y.Z"
entry of their own - they are still real, current, and especially relevant
to any task touching Finance, Orders, or Sheets sync.

### Financial / data-integrity logic

- **`insert_order_with_tickets`'s exact-cent cost allocation across
  tickets** (`commands/orders.rs`) - splits an order's total cost across N
  tickets to the exact cent. Explicitly called out elsewhere in this
  codebase as "protected financial/data-integrity logic this project's
  house rules say not to touch without asking first." This is also *why*
  Google Sheets sync is deliberately creation-only for orders: editing an
  order's purchase-side numbers after tickets already exist would touch
  this allocation.
- **`finance.rs`** (`profit_cents`, `safe_ratio`) - the single source of
  truth for profit/margin/ROI math. The in-app Sales screen, the CSV
  export, the Price Checker Market Analysis recommendations (2.2.0, see
  above), and (indirectly) the Google Sheets Summary formulas must all stay
  consistent with this; don't reimplement the formula locally anywhere.
- **Sale `payment_status` transitions** (`sales.rs`) -
  pending -> paid is a normal edit; refunding is a one-way, atomic
  transition (`refund_sale_impl`: sale -> `refunded`, ticket back to
  `available`, in one transaction) that can never be undone from the UI.
  A refunded sale is never deleted (history is kept) and never
  re-editable.
- **The new `finance_accounts`/`finance_entries`/`finance_recurring`/
  `finance_forecast` module family (2.1.0+) is a SEPARATE ledger, not
  currently linked to Orders/Tickets in any way.** Any work that connects
  Finance entries to a specific Order (marko has asked for this - see
  `CURRENT_STATE.md`'s current-task notes) needs to decide the link
  direction and cardinality deliberately; there is no existing foreign key
  or convention to copy here yet.

### Google Sheets sync modules

- **`orders_sheet_sync.rs` / `pulls_sheet_sync.rs`** - "creation-only"
  philosophy: ordinary sync/push must never overwrite a cell that already
  has *anything* in it, even if it looks wrong. The only way to correct an
  already-written cell is the explicit, separately-confirmed "Fix sync"
  (force) action, and even that never blanks a field the app has no
  opinion on (see `apply_sales_push_internal`'s 2.0.61 doc comment for the
  regression this rule exists to prevent). Do not make an ordinary
  sync/push more aggressive without asking - this has already caused a
  real regression once.
- **Locale-sensitive Sheets formulas** - any formula this app writes back
  to a sheet must avoid a literal comma as a function-argument separator
  (`SUMIF(a,b,c)`), because Google Sheets parses that separator per the
  spreadsheet's own locale, and this app has no reliable way to know it
  (only the 3-letter currency code, a different setting). Use a single
  array-expression `SUMPRODUCT(...)` argument instead - see
  `plan_orders_summary_updates`'s 2.0.42/2.0.80 doc comments for the real
  incident and the pattern to follow.
- **Reuse existing thresholds/mechanisms, don't invent new scoring or
  rules.** Stated in this project's own original plan file and followed
  since: e.g. the Dashboard alert bell and outbound notifications
  deliberately reuse the exact same 4 "Attention" categories and the same
  3-day upcoming-event window already shown elsewhere, rather than a new
  rule; the "pulls near deadline" Dashboard alert reuses Pulls.tsx's own
  existing warning window rather than a new one; Price Checker Market
  Analysis (2.2.0) reuses `compute_scan_stats` and `finance::safe_ratio`
  rather than reimplementing either.

### Backend fields that outlive one UI consumer

- Before deleting a backend field/column because one screen stopped
  showing it, check whether another feature still reads it. Example:
  `unpaid_orders_count` was removed from the Dashboard's own Activity tab
  in 2.0.79 but deliberately left in the backend `DashboardAlerts`
  struct/computation, because the outbound-notifications feature still
  depends on it and marko only asked to change the Dashboard.

### Explicitly out of scope until marko asks again

- **2FA and email verification at registration** - deliberately deferred
  ("postupom časom"). No design work, no partial implementation.

### Process/packaging invariants

- **Version bump is 9 occurrences across 7 files**, every release (see
  `CURRENT_STATE.md` and the "2.1.6" dated entry above for the full
  checklist beyond the obvious 3 files). Missing one produces a build with
  mismatched version strings.
- **`REDESIGN-X.Y.Z-REPORT.md` (Slovak) is written for every version and
  keeps being written** - confirmed with marko that `CHANGELOG.md` is
  additive, not a replacement.
- **Secrets stay plain text in `app_settings`** - this is an accepted,
  existing trust boundary across the whole app (Google OAuth refresh
  token, Pushover user key, Firebase config, etc.), not something to
  unilaterally "fix" by adding encryption without marko explicitly asking
  for that specific change.

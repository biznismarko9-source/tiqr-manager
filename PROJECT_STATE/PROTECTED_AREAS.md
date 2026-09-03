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

## 2.3.5 - Sync/push behavioral redesign (self-healing push, real sync diff, async commands)

**Read this before touching `orders_sheet_sync.rs`'s sync/push functions, or
either module's `#[tauri::command]` wrappers, again.**

**1. All 11 sheet-sync commands are now `async fn` + `spawn_blocking` -
never add a new one as a plain synchronous `fn`.** `orders_sheet_sync.rs`
(`sync_orders`, `push_orders`, `sync_sales`, `push_sales`, `force_push_sales`,
`create_orders_sheet`, `setup_orders_sheet`) and `pulls_sheet_sync.rs`
(`sync_pulls`, `push_pulls`, `create_pulls_sheet`, `setup_pulls_sheet`) all
follow the exact pattern `google_auth.rs`'s `start_google_sign_in` set in
2.0.13, and that `commands/notifications.rs`'s own module doc comment
already names explicitly:
```rust
#[tauri::command]
pub async fn NAME(app: tauri::AppHandle) -> AppResult<T> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let conn = state.db.lock().unwrap();
        NAME_impl(&conn)
    })
    .await
    .map_err(|e| AppError::External(format!("the ... task did not complete cleanly: {e}")))?
}
```
Takes `AppHandle`, not `State<AppState>`, ON PURPOSE - `AppState::db` is a
bare `Mutex<Connection>`, not `Arc`-wrapped, so `State<'_, AppState>` cannot
be moved into a `'static` `spawn_blocking` closure (a real Rust lifetime
constraint, not a style choice); `app.state::<AppState>()` (the `tauri::
Manager` trait - both files needed a fresh `use tauri::Manager;`, `State`
was removed from both since nothing else in either file still used it)
re-derives the identical managed state from inside the closure instead. Only
the `_impl` functions changed callers, never their own internal logic - the
whole point was zero risk to the sync/push logic itself while fixing the
freeze. Before this, EVERY click of any sync/push button froze the entire
app - Tauri dispatches a non-async command straight onto its single main/
IPC thread, and these commands all do blocking `reqwest::blocking` network
I/O through Google Sheets. Confirmed via the exact same bug class this
codebase already hit and fixed once for Google sign-in (2.0.12->2.0.13).
**A new sheet-sync command (or any command doing real network I/O) must be
written this way from the start - a plain `fn` here is a regression, not a
style nit.**

**2. Order sync (`apply_order_rows`) now diffs and updates an already-linked
row - but ONLY 5 specific fields, and this boundary is load-bearing, not
arbitrary.** `OrderRowSnapshot` (right above `apply_order_rows`) tracks
`platform_name`, `purchase_date` (the sheet's "Date (DD/MM/YYYY)"),
`currency`, `notes` (Email), `external_reference` (Order ID) - and nothing
else. **`quantity`/`unit_price_cents`/`Total Purchase Price` are
deliberately NOT in this snapshot, full stop - never add them without
asking marko first.** `insert_order_with_tickets` allocates each ticket's
exact-cent purchase cost from those 3 numbers at creation time
(`allocate_cents`); silently re-deriving/rewriting that allocation from a
later sheet edit is exactly the "protected financial logic, don't touch
without asking" territory the 2.0.53 currency-push feature already drew a
line around for the PUSH direction - this is the same line, drawn for the
SYNC (pull) direction instead. A sheet-side change to any of those 3 fields
on an already-linked row is simply invisible to `OrderRowSnapshot` - not
flagged, not applied, not even validated - exactly today's (pre-2.3.5)
behavior for them, unchanged. The 5 tracked fields carry no such risk (all
5 are already safely editable after the fact via `update_order_impl`/the
manual "Edit order" form; `external_reference` is the one exception - not
part of that form, but always a plain, cost-free UPDATE, same as this
module's own create path already performs). Legacy `'{}'` snapshots (every
order linked before 2.3.5) need no migration: `OrderRowSnapshot` derives
`Default` with container-level `#[serde(default)]`, so `'{}'` parses as
all-blank fields, looks "different" from any real row on its very next
sync, and silently backfills a real snapshot through the ordinary update
path - never hits the "unreadable snapshot" error branch. Same two-sided
conflict check as Pulls (`order.updated_at > link.last_synced_at`) before
applying anything. **Sales sync (`apply_sales_rows`) was deliberately NOT
given equivalent diff-and-update logic this round** - its own "already
fully sold" branch only ever (idempotently) retries the pull-info link, and
marko's complaint was scoped to Orders/Sales sync/push as a pair described
at a high level, never specifically to Sales' own resale_status/delivery_
status/payout fields on an already-sold row. If he reports that gap
specifically, port the same snapshot-diff pattern there, still never
touching `Sale.sale_price_cents`/an already-booked sale's amount for the
same cost-allocation-adjacent reason as above.

**3. Push Orders is self-healing by marker; Push Sales deliberately is
not (and doesn't need to be) - do not "fix" Sales to match.**
`apply_order_push` now treats an already-linked order as needing a fresh
append when its marker (the order's own `code`) is missing from the sheet's
CURRENTLY-FETCHED data, not just when it was never linked at all - re-using
the existing code, never generating a new one, and never re-inserting the
`sheet_sync_links` row (the old one was never wrong). **This is a
deliberate, permanent divergence from `pulls_sheet_sync::apply_pull_push`**,
which treats the identical situation (linked, marker missing) as an ERROR
instead, explicitly to avoid a possible duplicate - see that function's own
doc comment. marko asked for Orders/Sales self-healing twice, explicitly,
concretely (including the literal repro: delete a row by hand, push again,
it must come back) - his words were treated as the authoritative, considered
design choice for Orders/Sales specifically, not generalized to Pulls, which
he separately described as already working correctly and asked to move past.
**If Pulls ever gets the same complaint for real, this is the entry to
re-read and the design tension to re-resolve (probably by asking him
directly, since the two data sources now deliberately disagree) - don't
assume porting 2.3.5's Orders behavior over is automatically correct for
Pulls too.** `apply_sales_push`/`apply_sales_push_internal` needed, and got,
ZERO code changes for this: neither ever creates a sheet row (`push_sales`
only ever fills Sales-sync columns of a row that already exists) - once
`apply_order_push` restores a deleted order's row (Sales-sync columns
necessarily blank again on the fresh row), the very next Push Sales already
re-fills them through its existing, unchanged "only write when every target
cell is still blank" rule. Proved end-to-end, not just by inference, in
`deleting_a_fully_sold_orders_entire_row_is_healed_by_order_push_then_sales_
push_alone` (chains `apply_order_push` -> `apply_sales_push` against one
shared, marker-linked sheet row built from a combined Orders+Sales header
layout - `combined_order_and_sales_headers()`, a closer stand-in for
marko's real one-sheet layout than `full_headers()`/`sales_headers()` are
individually). **This resolves the row-426 data-integrity gap the
"2.3.2-2.3.4" entry below flagged as needing a deliberate fix** - the next
Push Orders after 2.3.5 ships will notice that order's marker is missing
and re-add it automatically; see that entry, now marked resolved, for the
original incident.

## 2.3.2-2.3.4 - Dashboard Total cost card, and a Google Sheets push bug that took two tries

**Two `totalCostCents` fields on the Dashboard now, don't merge them.**
`data.period.totalCostCents` (Overview tab, "Purchase cost") is
period-filtered and SOLD-ONLY (`cogs_cents`, the 2.0.68 fix - it exists so
Revenue - Purchase cost = Profit reconciles for that period).
`data.inventory.totalCostCents` (new, Financials tab, "Total cost") is
all-time and covers EVERY ticket regardless of status - the true
"everything ever spent" figure marko actually asked for. Same field name
inside two different `FinanceSummary`-shaped objects, deliberately
different populations - see `CURRENT_STATE.md`'s "Current focus" 2.3.2
entry before ever touching either.

**Google Sheets push - two complaints from the same message as the card
above.** 2.3.2 shipped with neither fixed yet - marko was asked directly
first (AskUserQuestion) rather than guessed, since both live in his real,
real-money Google Sheet and this app has the only OAuth access to it (no
separate read path available to a session to check its actual contents).
His answers: row 18 is completely empty and always was; rows 19-425 also
look completely empty, he has no idea why Google jumped past them; and he
had ALREADY retried Push Orders/Sales once since noticing the missing
formulas, and they were still missing. That last answer is what unblocked
2.3.3 below - it rules out the formula gap being just the already-expected
"one push behind" lag (see that entry's own doc comment in
`orders_sheet_sync.rs`), and pointed at the row-placement bug as the more
likely shared cause of both symptoms instead of two unrelated bugs.

- **FIXED in 2.3.4 (2.3.3's first attempt was NOT enough - read this
  before trusting raw row/cell counts against this sheet again) - Push
  Orders landed a new row at 426 instead of ~18** (marko: "no mala to dat
  do 18, tam kde je posledny vynechany riadok"). Root cause:
  `push_orders_impl` (`orders_sheet_sync.rs`) called
  `google_sheets::append_values` with a bare `"A1"` range anchor and
  `insertDataOption=INSERT_ROWS` - this delegated row placement ENTIRELY to
  Google's own table auto-detection, not to anything this app tracks
  itself. `apply_order_push` never had any gap-scanning/row-reuse logic of
  its own to begin with.

  **2.3.3's first fix** computed the target explicitly from
  `data_rows.len()` (the same `"A1:AZ"` read this function already trusts
  for header/marker-column detection) instead of trusting Google's opaque
  auto-detect - sound in isolation, verified wrong in practice: marko sent
  a screenshot showing Revenue (P) and Profit (Q) filled with live formulas
  all the way to row 425, despite confirming rows 18-425 have no real order
  data. **A formula is non-empty content too** - `plan_sheet_structure_
  updates` had, at some point in this sheet's history (before this bug was
  ever investigated - the trigger is not known and not worth chasing
  further), written Revenue/Profit formulas that far down, and once
  written, nothing in this codebase ever clears them. That means `data_rows.
  len()` was NEVER a small, honest "how many real orders" number in marko's
  sheet - it already agreed with Google's own confused auto-detection,
  because both were reading the same formula residue as "the table". 2.3.3
  trusted that same contaminated count, so it reproduced the exact bug it
  was meant to close. **Lesson: raw row/cell count from a plain values.get
  is not a reliable "where does real data end" signal once a sheet has ever
  had formulas written past its real data - which, given the paragraph
  above, this app itself can cause.**

  **2.3.4's fix**: `next_append_row`/`next_append_range` now scan `data_
  rows` for the LAST row whose **marker cell** (TIQR ID column) is
  non-empty, and target the row right after it - completely ignoring
  anything else in the row, including stray formulas.  The marker column is
  the one column in this sheet ONLY this app ever writes, and ONLY for a
  row that holds a real pushed order; `plan_sheet_structure_updates` never
  touches it. 5 unit tests cover this, including
  `next_append_row_ignores_stray_formula_rows_past_the_real_data` (the
  literal "16 real + 408 stray-formula rows" shape of marko's actual sheet)
  and `next_append_row_never_reuses_a_gap_left_by_a_row_cleared_in_the_
  middle` (deliberately scans for the LAST marked row, not the first empty
  one - a row cleared out of order, e.g. an order deleted straight in the
  sheet, must never look like "the next free slot" to a future push - see
  the next bullet for exactly this happening for real). The write itself
  goes through `update_values` (exact-range overwrite, already used
  elsewhere in this file) instead of `append_values`.

  **Still does NOT retroactively clean up marko's actual sheet** - two
  things left exactly as they are, on purpose, because both mean editing
  live data this app cannot see the full context of, and neither blocks
  correct behavior going forward:
  - The stray Revenue/Profit formula residue in rows 18-425 stays (cosmetic
    only - harmless zeros, not wrong data). `ensure_orders_sheet_structure`
    will keep re-writing/re-confirming that same residue on every future
    push too (it still computes `data_row_count` from raw `data_rows.len()`,
    unchanged by this fix) - this is expected, not a regression.
  - The order that was at row 426 - marko deleted its content directly in
    the sheet while testing this (see the very next bullet).

  `pulls_sheet_sync.rs` (`push_pulls_impl`) still uses the same bare-anchor
  `append_values` pattern this fixes here - NOT touched, since marko never
  reported this symptom for Pulls and there is no confirmed evidence its
  sheet has the same formula-residue issue. If a similar "landed in the
  wrong row" report ever comes in for Pulls, this entry (both attempts,
  not just 2.3.4's) is what to port over.

- **RESOLVED in 2.3.5 (see that entry, point 3, above this one) - the order
  that used to be at row 426 was permanently "invisible" to this app.**
  While testing between 2.3.3 and 2.3.4, marko deleted that row's content
  directly in the sheet (not through the app), then ran Order/Sales sync
  again and saw no change - expected, not a bug: `apply_order_rows`
  (the PULL/sync direction, a different code path from push) treats a
  fully blank row as just a gap and silently skips it (see that function's
  own "just a gap" comment) - sync was never going to notice or repair a
  deletion either way, push or pull. The real, lingering problem was that
  order's `sheet_sync_links` row (written back when it was first pushed,
  per the 2.2.10 fix) still existing in this app's own local database, so
  `apply_order_push`'s old `WHERE NOT EXISTS (SELECT 1 FROM
  sheet_sync_links ...)` filter would never offer to push it again - the
  app kept believing it was already synced forever. 2.3.5's self-healing
  push (checks the linked marker against the sheet's CURRENT data, not just
  whether a link row exists) fixes this generically, for this order and any
  future one like it - no manual `sheet_sync_links` surgery needed. The
  next "Push orders" after upgrading to 2.3.5 re-adds it automatically.

- **Believed fixed as a side effect of 2.3.4 above, NOT independently
  verified - Revenue/Profit formulas missing on many rows**, still missing
  even after marko retried the push once before 2.3.4 shipped (ruling out
  the ordinary "next-push" lag as the full explanation - see that lag's own
  doc comment in `orders_sheet_sync.rs`). `plan_sheet_structure_updates`
  already writes one formula per row across the sheet's CURRENT full extent
  on every push (`0..data_row_count`, recomputed from a fresh `"A1:AZ"`
  read each time) - and per the formula-residue discovery above, that
  extent already reaches row 425 regardless of whether anything real is
  there. That means once 2.3.4 correctly places a new order at its real
  next row (e.g. 18), THAT SAME PUSH's own structure-refresh should already
  cover it with a formula too, with no second push needed this time - the
  residue that caused the placement bug happens to make formula coverage
  generous enough to include it immediately. **Ask marko to click Push
  Orders/Push Sales once on 2.3.4 and confirm** - if formulas are still
  missing after that, this needs a fresh report with the EXACT row
  number(s) still affected and whether the push-results panel
  (`SyncResultView`, `Settings.tsx`) showed any red error text (structure-
  refresh failures land there as "Row 0: the sheet's dropdowns/Revenue/
  Profit formulas could not be refreshed this time: ..." -
  `refresh_sheet_structure_soft_fail` - already wired, not silent) - do not
  assume this is resolved until he confirms.

## 2.3.0 - Event Lifecycle - BUILT THEN FULLY REVERTED, same session

Designed and shipped in full (derived `EventLifecyclePhase` on Events.tsx/
EventDetail.tsx, zero backend change), then marko asked to remove it
entirely and go back to the previous version after reviewing the delivered
build - no specific complaint given beyond not liking it in place. Both
files were reverted edit-for-edit to their exact pre-2.3.0 (= 2.2.12)
content; this entry's detailed design notes (phase rule, precedence, Next
Actions/pending-fulfillment sourcing, every DÔLEŽITÉ constraint check) were
removed since they described protections for code that no longer exists in
this codebase - see `CHANGELOG.md`'s "2.3.0" entry for the full original
design if it's ever useful as a reference. **If a similar event-status/
phase feature is requested again, ask what specifically didn't work about
this one first** - no concrete reason was captured this time, so
re-proposing the identical shape blind is a real risk. **Separately: the
version number was NOT rolled back to 2.2.12 even though the code was** -
marko caught this himself, reusing an old version number breaks the
auto-updater for anyone already offered a newer one - so the reverted code
shipped as version 2.3.1. See `CURRENT_STATE.md`'s "## Version" section for
the lesson this leaves for future sessions.

## 2.2.12 - Fulfillment Center

Marko's ČASŤ C from the same message as 2.2.11, shipped as its own release
right after it (explicitly requested as two separate releases). See
`REDESIGN-2.2.12-REPORT.md` (Slovak) for the full report.

- **Every DÔLEŽITÉ constraint from marko's message was checked, not just
  assumed, before this was called done:** no refund/resell logic was
  touched (this page only READS `refundedCount`/`paymentStatus`, the exact
  same fields Sales.tsx's table already reads - `refund_sale_impl` itself
  was never opened); no `batch_id` logic was touched (not referenced
  anywhere in `FulfillmentCenter.tsx`); no money/cents logic was touched
  (every amount is `formatMoneyOrMixed(g.revenueCents, g.currency)`, the
  same call Sales.tsx's table already makes - no new arithmetic on cents
  anywhere in this file); Listings/Price Checker/market pricing/Finance are
  not referenced at all; Tier/Level is not referenced at all, let alone
  used for pricing; and the existing Sales Completed/Pending rule is not
  just "consistent" but the SAME FUNCTION CALL (`isSaleGroupDone`, imported
  from `Sales.tsx`, not reimplemented) - this page is structurally
  incapable of disagreeing with Sales.tsx about what counts as done.
- **"Ready to Complete" - the one real design decision this task required,
  since marko's message didn't define it.** Traced from `SaleGroup`'s own
  field semantics rather than guessed: `soldCount` is "how many of this
  group's `ticketCount` tickets currently have status 'sold' - normally
  equals `ticketCount`, lower only when a line was refunded" (its own doc
  comment, `lib/types.ts`). `isSaleGroupDone` requires `soldCount ===
  ticketCount` (among other things) OR a full refund - so the ONLY way a
  group can be simultaneously fully paid, fully delivered, AND still
  Pending is a PARTIAL refund (some but not all lines refunded), which
  permanently keeps `soldCount < ticketCount` for that group. Therefore
  `isReadyToComplete(g) = paidCount === ticketCount && deliveredCount ===
  ticketCount` is automatically disjoint from "done" (if it also had
  `soldCount === ticketCount` it would already be Completed and excluded
  from this page entirely) - there is no other way to reach this state. In
  plain terms: on this page, "Ready to Complete" always means "this batch's
  remaining refund/resell bookkeeping is the only thing left," never a
  group that's genuinely still missing payment or delivery. **If marko
  disagrees with this definition**, it's a one-function change
  (`isReadyToComplete` in `FulfillmentCenter.tsx`) with zero ripple - it's
  not read anywhere else.
- **KPI row and category filters were unified into ONE set of 4 clickable
  tiles, not built as two separate things.** Marko's message listed "KPI:
  Pending Sales/Awaiting Payment/Awaiting Delivery/Ready to Complete" and,
  separately, "Kategórie... môžu byť: ALL PENDING/PAYMENT/DELIVERY/READY TO
  COMPLETE" (his own softer "môžu byť"/"can be" phrasing, not a rigid
  mandate) - these are the exact same 4 counts under two different naming
  schemes. Showing the same 4 numbers twice (once as static KPI cards, once
  as separate filter pills) would be visual duplication for no benefit, so
  they're one row of 4 tiles that both display the count AND filter the
  table - reusing 2.2.11's own just-established Attention-Center-box
  pattern for visual consistency across both new screens this batch. If
  marko specifically wants a KPI row that stays fixed regardless of which
  filter is active (so you can see "Awaiting Payment: 5" even while looking
  at the Delivery-filtered table), that's a real, easy follow-up - flagged
  here rather than guessed at.
- **Delivery status needed a genuinely new badge** - unlike Payment status
  (copied verbatim from Sales.tsx's own `g.paymentStatus ? <Badge
  tone={g.paymentStatus}>...` pattern), Sales.tsx has no existing
  GROUP-level delivery badge to copy (only the combined Sold+Delivered+Paid
  "Completed" badge). `deliveryStatusBadge` in `FulfillmentCenter.tsx`
  derives one from `deliveredCount`/`ticketCount`, reusing the exact
  `delivered`/`"not delivered"`/`mixed` tone keys `ui.tsx`'s `STATUS_TONES`
  already defines (today used by the per-TICKET `InlineStatusSelect` on
  Sale/Order Detail) - no new color was added anywhere.
- **No responsive narrow-table breakpoint was added** (unlike Sales.tsx/
  Orders.tsx's dual `useNarrowTables()` colgroups) - this table only has 7
  modest columns (vs. their 10-13), so a single fixed `colgroup` was judged
  sufficient without measuring against the app's enforced 1080px minimum
  window width the way those two tables' own narrow mode was originally
  measured. Flag to marko if it ever looks cramped on a small window - it
  would be a small, contained addition, not a redesign.
- **Sidebar placement**: `/fulfillment` sits right after Sales in
  `Layout.tsx`'s `NAV` (not standalone-top-level like Price Checker/Finance)
  - it's a narrower work-view OVER Sales' own data, not an independent
  feature area of its own. Reuses the existing `IconCheck` rather than
  adding a new icon.
- **Testing, given this codebase has no frontend test framework** (grepped
  first, confirmed - no vitest/jest, no `*.test.*` file anywhere, ever, in
  this project's history): `isSaleGroupDone`/`isReadyToComplete`/
  `matchesFulfillmentCategory` were verified with a disposable,
  esbuild-bundled Node script - built OUTSIDE the repo (`/root/verify-
  2.2.12/`, never inside `src/`), importing the REAL exported functions
  (not reimplemented copies) with fabricated `SaleGroup` fixtures covering
  every scenario marko listed (payment pending, delivery pending, both at
  once, ready-to-complete via the partial-refund edge case, a fully-done
  group correctly excluded from the Pending set, the full-refund rule
  correctly excluded too) - 21/21 assertions passed, then the script and
  its output were deleted (not committed, not part of the shipped zip).
  Navigation-to-Sale-Detail and the Attention Center's click/select
  behavior (2.2.11) are UI/routing behavior a pure-function script can't
  exercise - both were instead confirmed by reading the actual route/`<Link
  to=...>` targets against `App.tsx`'s real route table, the same
  code-reading-based verification every prior release's frontend-only
  changes have always relied on in this sandbox (which has no display to
  run the real Tauri app in).

## 2.2.11 - Attention Center UX rework + Dashboard cleanup

Marko's own next request, sent as one large structured message explicitly
split into two releases (this one, plus 2.2.12's Fulfillment Center - see
that entry below/above once it ships). This entry covers 2.2.11 only: Part
A (Attention Center) and Part B (Dashboard cleanup), both entirely
frontend-only. See `REDESIGN-2.2.11-REPORT.md` (Slovak) for the full report.

- **Attention Center: category, not priority, is now the grouping axis -
  and this was a pure frontend change.** `AttentionCenterItem.category`
  (`"event_soon" | "missing_listing_price" | "no_active_listing" |
  "outside_market_price" | "sold_undelivered"`) already existed on every
  item the backend sends (`attention_center.rs`, unchanged since 2.2.9) -
  the OLD frontend (`ATTENTION_CENTER_GROUPS`/`AttentionCenterGroup`) just
  happened to group by the OTHER existing field, `priority`, instead. 2.2.11
  removes both of those and replaces them with `ATTENTION_CENTER_CATEGORIES`
  (5 entries, marko's own exact title/order) + `AttentionCategoryCard` (one
  clickable box per category) + a rewritten `AttentionCenterBlock` that
  tracks which ONE category is `selected` and shows only that category's
  rows below the 5-box grid, via the untouched `AttentionCenterRow`. No new
  field was added anywhere, no new command, no new sort - `items` arrives
  in exactly the same shape and order as before this task.
- **Box title <-> category mapping (memorize this, it's a 1:1 map, not a
  re-derivation):** NO LISTING PRICE YET = `missing_listing_price`, NO
  ACTIVE LISTING = `no_active_listing`, NOT DELIVERED YET =
  `sold_undelivered`, EVENT COMING SOON = `event_soon`, MARKET ATTENTION =
  `outside_market_price`. If a 6th category is ever added to
  `attention_center.rs`, it will silently show 0 boxes for it unless
  `ATTENTION_CENTER_CATEGORIES` is also updated - there is no fallback
  "everything else" bucket by design (marko asked for exactly 5 named
  boxes, not an open-ended list).
- **Judgment call: a box's count is the number of Attention Center ROWS in
  that category, not a raw ticket count.** Every ticket-level category has
  already been grouped by ORDER since 2.2.9 (one row can carry many
  `ticketIds`) - the box count reuses that same existing grouping
  (`items.filter(i => i.category === key).length`) rather than summing
  `ticketIds.length`, matching the precedent the old priority-group headers
  already set (`{label} ({items.length})`). If marko wants "how many
  tickets" instead of "how many rows/orders" shown on the box face, that's
  a one-line change (sum `ticketIds.length`, falling back to 1 for
  `event_soon`'s ticket-less rows) - flagged here rather than guessed at.
- **Judgment call: a 0-count box is disabled, not hidden.** All 5 boxes
  always render (marko's spec: 5 NAMED boxes, not a variable-length list),
  but a box with nothing behind it can't be clicked into - there's no
  detail view to open. If a category empties out while its detail panel is
  open (a fresh `getAttentionCenter()` fetch drops its last row), a
  `useEffect` closes that panel automatically rather than leaving it open
  and empty under a now-disabled box.
- **MARKET ATTENTION's hard constraints were already fully satisfied
  BEFORE this task started - confirmed by reading, not assumed.** Marko's
  message repeated 4 explicit constraints for this box (only when real
  Price Checker data exists; no automatic price determination; section/row
  must not be a pricing factor; tier/level must not determine or change
  price). Read `attention_center.rs`'s own module doc comment line-by-line
  before writing any frontend code: "No new migration, no new dependency,
  no automatic pricing/repricing anywhere in this file, and `tier`/
  `section`/`row` are never read as a pricing factor - every 'value' this
  module ever shows is a value that already exists verbatim on the ticket."
  Its `outside_market_price` arm only ever fires when
  `attention_item.available` is true (Price Checker data exists for that
  event - see `inventory_intelligence.rs`), and its own test
  (`outside_market_price_only_fires_when_price_checker_data_exists_for_
  that_event`) already locks this in. Net result: **zero backend changes
  for Part A**, and none were needed - do not "helpfully" add a pricing
  suggestion or a tier-based adjustment to this module later without
  re-reading marko's own explicit constraints above first.
- **`AttentionSection` (the OTHER Dashboard attention block, further down
  the same Activity tab - alert bell era, 2.0.75/2.0.76/2.0.79) is
  deliberately, completely untouched.** It is a different, already-shipped
  feature backed by `DashboardAlerts`/`data.alerts`, not
  `AttentionCenterItem[]` - see `attention_center.rs`'s own doc comment for
  exactly how the two differ and why both exist. Marko's 2.2.11 message
  named "Attention Center" specifically (capital letters, matching this
  block's own on-page heading) and never mentioned the alert bell/
  `AttentionSection` cards - touching those would also have been a bigger,
  unrequested visual change than anything else in this release.
- **Dashboard cleanup (Part B) root cause: `SalesByPlatformCard`'s `<ul>`
  was the one truly unbounded list on the Overview tab.** Every other
  Overview element is either a fixed handful of StatCards or a single
  chart; this list grows by one row per DISTINCT platform a business has
  ever sold through (free-text/picker field on Orders/Sales), so it has no
  natural ceiling. Fixed with `max-h-72 overflow-y-auto` on just that
  `<ul>` - a handful of platforms (marko's own screenshot showed 4) still
  renders in full with no scrollbar at all; a long list now scrolls inside
  its own card instead of pushing the page down. Paired with a small,
  one-step trim of two existing Tailwind spacing values on the same tab
  (StatCard grid `mb-6`->`mb-5`, metric chart Card `mb-8`->`mb-6`) - picked
  because Layout.tsx's own content wrapper (`<div className="px-6 py-6">`)
  already adds 48px of fixed top+bottom padding around every page, so a
  borderline-overflowing Overview tab is plausible even with few platforms,
  on a smaller/scaled display. Deliberately NOT changed: `PageHeader`'s own
  margin (`ui.tsx`, shared by every page - out of this task's scope, and
  changing it would ripple everywhere) and `Layout.tsx`'s `<main
  className="overflow-y-auto">` itself (already correct - confirmed by
  reading, it only ever scrolls when content genuinely overflows; the bug
  was in how much content there was, not in that wrapper).
- **Caveat marko should know, stated plainly in the report too**: this
  sandbox cannot render a real browser at a specific OS/display scaling, so
  the exact pixel point at which the Overview tab used to overflow (if it
  ever did on marko's own machine at 1920x1080) was reasoned about, not
  literally reproduced. The unbounded list was identified as the one
  concrete, well-justified, always-correct-to-fix root cause regardless of
  whether it was ALSO the exact trigger marko personally saw - if a full-
  page scrollbar still appears on his machine after this release, the next
  thing to check is Windows display scaling (a scaled-down viewport is
  smaller than the 1920x1080 CSS pixels the screenshot suggests).

## 2.2.10 - Eight follow-ups from marko's 2.2.9 review

Marko reviewed 2.2.9 and sent two rapid-fire messages (7 screenshots
combined) covering eight mostly-independent items, shipped together. See
`REDESIGN-2.2.10-REPORT.md` (Slovak) for the full report. No migration this
release - still `025_deactivate_seatriks_price_checker.sql`, everything
below is query/logic/frontend only.

- **Seats format: the "Sec"/"Row"/"Seat" labels 2.2.9 just added are gone
  again, everywhere.** `formatSeatLocation`/`formatSeatsSummary`
  (`src/lib/format.ts`) now join bare values with " · " - "402 · 56 · 27"
  instead of "Sec 402 · Row 56 · Seat 27". Reason: a real `section` value is
  sometimes already a full label on its own (marko's own screenshots -
  "Sec 408" as the stored section text, "Category D, Standing" as another),
  and prepending another "Sec "/gluing the two together read as broken
  ("Sec Sec 408", "Sec Category D, Stan..."). Both functions are called from
  every "Seats" column across Orders/Tickets/Inventory/Sales/Pulls/
  OrderDetail/SaleDetail (all already routed through these two shared
  helpers as of 2.2.9's own consolidation - see that entry below) so this
  one change reaches everywhere with no per-page edits. If a future request
  asks for labels back, put them back on BOTH functions together (they
  share the exact same convention deliberately) - don't add a prefix to one
  and not the other.

- **`isEventDone` (`Orders.tsx`) treats an event as done on EITHER status OR
  date, not status alone - this is a deliberate widening past this
  codebase's own existing precedent, not an oversight.** Two DIFFERENT
  "is this event over" conventions already existed before this release:
  `Events.tsx`'s own Upcoming/Completed tabs and `PriceChecker.tsx`'s event
  picker are purely status-based (`ev.status === "upcoming"`), while
  `inventory_intelligence.rs`'s `event_soon` logic is purely date-based.
  `events.status` (`EventStatus` - upcoming/completed/cancelled) is a
  plain, manually-set field with ZERO automatic date-based transition
  anywhere in the backend (confirmed by grep before writing this) - so a
  status-only check, like PriceChecker's own, silently does nothing for an
  event whose date has quietly passed while marko never flipped its status
  by hand. Marko's own screenshot showed exactly this: a past-dated event
  ("Bad Bunny 2026-08-22", checked against "today" 2026-09-02) still
  selectable in New Order's event picker. `isEventDone` therefore ORs both
  signals: `status === "completed" || status === "cancelled" ||
  (eventDate !== null && eventDate < todayIso())`. This is used by BOTH
  `isOrderDone` (Orders tab bucketing) and the New Order event picker
  filter (see next item) - if only one of the two had been fixed, the tab
  and the picker would disagree about which events are "active", which
  would look like a new bug. Do not narrow this back to status-only without
  first confirming marko is now reliably keeping `events.status` current by
  hand - the whole reason this ORs both is that the codebase gives no
  guarantee he is.

- **Orders tabs reworked: "Active"/"Paid" (2.0.59) -> "Active"/"Completed",
  with a genuinely different bucketing rule, not just a relabel.** Old rule
  was purely `paymentStatus`. New rule (`isOrderDone`, `Orders.tsx`):
  `isEventDone(order) || completionStatus(orderCompletionChecks(order)).tone
  === "completed"` - an order is Completed once EITHER its event is done
  (above) OR the order itself is fully wrapped up (sold + delivered + paid,
  reusing the exact same `orderCompletionChecks`/`completionStatus` pair
  that already powers the order's own "Completed" badge column, 2.0.66/
  2.0.68). This is an OR, not an AND, matching marko's own wording exactly
  ("v completed len vtedy, ak presiel datum ... alebo ak v sales bolo vsetko
  splnene") - an order for a future event that is nonetheless already fully
  sold/delivered/paid belongs in Completed too, it does not have to wait
  for the event date. `useListTab("ordersTab", [...])`'s key changed from
  `"paid"` to `"completed"` - no migration needed, a stale saved `"paid"`
  value simply falls back to the hook's own default (`"active"`, its
  `keys[0]`) the first time a returning user opens this page, by that
  hook's existing built-in design (see `useListTab.ts`). `Order` gained
  `eventDate`/`eventStatus` (`models.rs`, `types.ts`) purely as a read-time
  denormalization via `orders.rs`'s `BASE_SQL` join on `events` - no new
  column, no migration; `map_order` is the only construction site
  (confirmed by grep), so no other call site needed touching.

- **New Order's event picker now excludes done events too (`isEventDone`,
  same helper as above) - previously it listed every event with no
  filter at all.** Marko's screenshot showed a past/already-completed event
  still creatable-against. Confirmed via grep that `OrderFormModal` (the
  component with this picker) is used in exactly one place, always for
  brand-new order creation ("New order" is the modal's own hardcoded
  title) - there is no order-EDIT flow anywhere in this codebase that
  reuses this same picker, so restricting it carries no risk of blocking
  someone from re-opening/re-saving an existing order against its own
  (now "done") event. This filter is deliberately stricter than
  `PriceChecker.tsx`'s own event picker, which still filters on
  `status === "upcoming"` only - that one was not touched this release
  (out of scope, marko did not mention it), so the two pickers now
  disagree slightly (PriceChecker would still offer a past-dated-but-
  status-"upcoming" event; New Order would not). If marko later notices
  that and wants them consistent, decide explicitly which of the two
  conventions should win rather than silently copying one over the other -
  PriceChecker's own picker exists for a different purpose (checking
  market prices while planning a purchase, arguably useful even close to
  showtime) than New Order's (recording a purchase against an event that
  hasn't happened yet).

- **Attention Center (`attention_center.rs`) "je to mixed" root cause: the
  sort's own tie-break key, not the grouping-by-order logic itself.**
  2.2.9's `group_by_order` rework (see that entry below) was and remains
  correct - the bug was purely in `items.sort_by`'s final tie-break, which
  compared `a.key.cmp(&b.key)` where `key` is formatted
  `"{category}:order:{oid}"` - i.e. it sorted by CATEGORY NAME first among
  same-priority/same-event rows, scattering one order's several reason-rows
  apart from each other, interleaved with every other order sharing that
  same category. This exactly reproduced marko's screenshot (all
  `missing_listing_price` rows first, across several different orders, THEN
  all `no_active_listing` rows). Fixed by inserting real tie-breakers
  BEFORE the category-name one: priority -> soonest event date (unchanged)
  -> `event_id` -> `order_id` -> `category` -> `key`. This groups every row
  for the same order adjacently regardless of which categories they came
  from, while still keeping deterministic ordering for everything else.
  Different-reason rows for the same order are still NOT merged into one
  row - marko's own earlier explicit allowance from the 2.2.8 round - this
  fix only changes their ORDER relative to each other, never their count or
  shape.

- **Attention Center now also excludes DONE events (same `event_is_done`
  concept as `isEventDone` above, reimplemented in Rust since this command
  has no access to the frontend helper) from 3 of its 5 categories -
  `missing_listing_price`/`no_active_listing`/`outside_market_price` - but
  deliberately NOT from the other 2.** `event_soon` is exempted because it
  already only fires for events within its own urgency window
  (`EVENT_SOON_DAYS`) - a soon event cannot also be "done" by date, and
  cancelling it should still surface if the app doesn't already know (it
  does - added an explicit `event_status != "cancelled"` guard there too,
  since a cancelled event has nothing to prepare for). `sold_undelivered`
  is exempted ON PURPOSE: a ticket that was sold but never delivered stays
  a real, actionable problem regardless of whether the event already
  happened or was cancelled - if anything, an undelivered ticket for an
  event that's already passed is MORE urgent, not less. Do not extend the
  done-event exclusion to `sold_undelivered` without checking with marko
  first - it would silently hide exactly the kind of "did I ever actually
  send this ticket" gap this category exists to catch. `events_by_id`'s
  value tuple grew a third field (event status) to support this - every
  existing match-arm/destructuring site was updated (`event_is_done`'s own
  new helper, the `sold_undelivered` section's destructuring explicitly
  ignores the new field with `_event_status` since that category must never
  be gated by it).

- **Sales Pending/Completed: `isSaleGroupDone` (`Sales.tsx`) replaces a
  payment-status-only check with the same completion-badge machinery
  Orders now uses, PLUS an explicit refunded carve-out.** Marko's
  screenshot showed a sale with Payment "Paid" but Delivery "Not
  Delivered" still listed under Completed - the old filter apparently
  looked at payment status alone. New rule:
  `paymentStatus === "refunded" || completionStatus(saleGroupCompletionChecks(g)).tone
  === "completed"` - i.e. Completed now requires ALL of sold+delivered+paid
  (any one missing, including just delivery, keeps it in Pending - exactly
  marko's own wording, "aj keby len delivery chyba ostava v pending"),
  UNLESS the sale is fully refunded. The refunded carve-out is NOT new
  behavior - it preserves a 2.0.59 rule that would otherwise silently break,
  since `saleGroupCompletionChecks`'s own "Sold" check fails (correctly, by
  its own definition) for a refunded group, which would have pushed every
  refunded sale into Pending forever with no way out. If a "Mixed" payment-
  status group existed before, it now falls into Pending naturally (it was
  never fully paid, so it can't be fully "completed" either) - there is no
  separate "Mixed" bucket in this rule and none is needed.

- **Two real, confirmed Google Sheets push bugs fixed in
  `orders_sheet_sync.rs`/`pulls_sheet_sync.rs` - both were the SAME root
  cause, present independently in each file.** Marko: "tabulka napisala ze
  bola updated, no ziadna zmena nenastala" (the app reported success, but
  the sheet never actually changed). Root cause, confirmed by reading the
  code (not guessed): `apply_order_push`/`apply_pull_push` (the pure,
  testable "core" half of each push) were writing their `sheet_sync_links`
  bookkeeping row - and counting the item as `created`/`updated` -
  **before** the network call that was supposed to make it true had even
  been attempted, let alone confirmed to succeed. If `push_orders_impl`'s/
  `push_pulls_impl`'s subsequent `append_values`/`update_values` call then
  failed, the local DB already believed the write had happened - silently
  and permanently marking that record "already synced" even though the
  sheet was never touched, with NO way to notice or retry short of
  disconnecting and reconnecting the sheet. Checked and confirmed this
  does NOT affect `sales_sheet_sync`'s own push path - `apply_sales_push`
  performs zero DB writes of its own (confirmed by reading it end to end).
  **The fix changes the calling contract of both core functions: they no
  longer write to `sheet_sync_links` (or count anything as created/updated
  in the pulls case - see below) at all.** They only decide what needs
  writing and hand it back - `apply_order_push` returns a third tuple
  element (`Vec<(order_id, code)>`), and `PullPushWrite`'s two variants
  (`Append`/`Update`) each grew the extra fields (`pull_id`/`code`/
  `snapshot_json`) the shell needs. `push_orders_impl`/`push_pulls_impl`
  now perform the actual `sheet_sync_links` INSERT/UPDATE only in the
  success arm of the matching `append_values`/`update_values` call. Orders'
  append is one all-or-nothing batch call, so `result.created` resets to 0
  entirely on failure; Pulls' per-row `Update` writes are independent
  per-row API calls, so a partial failure there decrements `result.updated`
  for just that row and leaves ITS stored snapshot untouched (letting the
  next push/"Sync from sheet" naturally retry or flag it), while unrelated
  rows in the same run are unaffected. **Any future push-direction sync for
  a new data source must follow this same "decide first, confirm the
  network write, THEN record locally" order** - writing local sync
  bookkeeping before a network call it describes is done is the exact
  anti-pattern this release exists to remove, not a style preference.
  `apply_order_push`/`apply_pull_push`'s own unit tests now explicitly
  assert `sheet_sync_links` has 0 rows immediately after calling them alone
  (previously asserted 1) - a regression back to "linked immediately" would
  fail these tests, which is the point.

- **Google Sheets `invalid_grant` OAuth error now short-circuits to a
  clean "sign in again" message instead of dumping the raw JSON body -
  best-effort, NOT independently confirmed in this sandbox.** Marko: after
  signing in under Settings -> Integrations, pushing anything soon after
  throws "nejaky dlhy error" (some long error). `describe_error_response`
  (`google_sheets.rs`) is shared by BOTH the Sheets API's own error
  responses and `google_oauth.rs`'s token-refresh endpoint
  (`refresh_access_token`) - on any non-2xx response it previously always
  produced `"Google Sheets rejected the request ({status}): {body}"`,
  dumping Google's raw error JSON verbatim regardless of source. The single
  most common real-world OAuth failure of exactly this shape is
  `invalid_grant` (an expired/revoked refresh token - routine for a small/
  personal OAuth client still in Google's "Testing" publishing status,
  where Google auto-expires refresh tokens after 7 days) - a new branch,
  checked FIRST and unlike the two pre-existing hint branches REPLACING the
  message entirely rather than appending to the raw dump (since an OAuth
  token error's raw body has no useful diagnostic value for marko, unlike a
  Sheets-API error's), catches `body.contains("invalid_grant")` and returns
  a short "Google sign-in has expired - go to Settings -> Integrations and
  sign in again" message. **This is a well-reasoned but UNCONFIRMED fix** -
  this sandbox has no live Google OAuth access, so the actual error text
  marko saw was never seen or reproduced here, only inferred from how
  common this specific failure is for this exact kind of OAuth client. If
  the long error still appears after this release, the report explicitly
  asks marko for the literal text next time - don't assume this branch
  already covers it.

- **Native webview right-click context menu (Späť/Obnoviť/Uložiť ako/
  Tlačiť/Ďalšie nástroje) disabled app-wide via
  `document.addEventListener("contextmenu", e => e.preventDefault())` in
  `main.tsx`, run once before the app mounts.** Tauri/WRY has NO
  config-flag to disable its default context menu - this JS-side
  `preventDefault` is the standard, documented fix for this exact webview
  stack, not a workaround. Confirmed via a scoped grep (`src/` +
  `index.html` only - an earlier unscoped attempt across the whole repo
  including `node_modules` timed out) that no context-menu handling existed
  anywhere before this change - a clean slate, no conflicting handler to
  reconcile. If a future feature genuinely needs a custom right-click menu
  somewhere (e.g. a table row), it must build its own explicit menu
  component and stop propagation on that element specifically - it cannot
  rely on the browser's native one coming back, anywhere, ever, since this
  listener is global.

## 2.2.9 - Six follow-ups from marko's 2.2.8 review

Six mostly-independent small changes from one rapid-fire feedback message
(6 screenshots), not a single feature - covered together since they shipped
in one release. Judgment calls and traps, one per item:

- **Seatriks removed from Price Checker via `active = 0`
  (`migrations/025_deactivate_seatriks_price_checker.sql`), NOT an
  unconditional cut.** Exactly the StubHub precedent
  (`017_price_checker_viagogo.sql`): `get_price_checker_summary_impl`'s own
  query still shows an inactive marketplace for an event that ALREADY has a
  saved `event_marketplace_links`/`price_checks` row against it - only
  fresh offering to a new/untouched event stops. Marko said "odstranit
  seatiks uplne" (remove entirely) - if he still sees a Seatriks card on
  some specific event after this ships, that event already has real
  history against it, and the fix is a stronger, unconditional exclusion
  (a real, deliberate follow-up, not a bug in this change). Do NOT
  "fix" this by hardcoding the name "Seatriks" into `price_checker.rs`'s
  query logic - this codebase keeps marketplaces purely data-driven (zero
  hardcoded marketplace names in logic anywhere, confirmed before this
  change), and the migration's own `UPDATE ... WHERE name = 'Seatriks'` is
  the one, precedented place a literal name belongs.
- **`AnthropicApiKeyCard` (`Settings.tsx`) relabeled, storage/commands
  UNCHANGED.** Still `app_secrets`/`get_anthropic_api_key_configured`/
  `set_anthropic_api_key` (`commands/settings.rs`), still only ever a
  presence flag to the frontend, never the key. Only the `h3` label
  ("AI features"), its description paragraph, and the Integrations section
  description changed. If a genuinely new AI feature is wired to this key
  later, this is the card to extend, not a new one to build.
- **No live Anthropic balance is shown anywhere, and no Admin API key was
  added to this app.** Checked directly against Anthropic's own docs
  (`platform.claude.com/docs/en/manage-claude/usage-cost-api`) before
  building anything: there is no endpoint, for any key type, that returns a
  current/remaining credit balance - the Usage & Cost API only returns
  HISTORICAL token/cost data, and that alone requires an Admin API key
  (`sk-ant-admin01-...`) or an unscoped personal/service key; the plain
  workspace key this app's own field asks for explicitly does not work for
  it. Do not "solve" marko's balance request later by adding a second,
  Admin-scoped key field to this same card without asking him first - that
  is a materially bigger, more sensitive ask than "a small balance
  indicator" implied, and still would not produce a live balance figure
  even then (only historical usage/cost). The card instead links to
  `console.anthropic.com/settings/billing` via `openUrl`
  (`@tauri-apps/plugin-opener`, already a dependency - `google_oauth.rs`
  already uses the Rust side of the same plugin for the sign-in browser
  flow, so this added no new dependency, just a new frontend call site).
- **Finance Overview's "New entry"/"New account" buttons open the SAME
  modals as Transactions/Accounts, not copies.** `EntryFormModal`
  (`finance/Transactions.tsx`) and `AccountFormModal`
  (`finance/Accounts.tsx`) are now `export`ed (were module-local) and
  imported directly into `finance/Overview.tsx`. If either modal's form
  changes in the future, there is still only ONE place to change it - do
  not fork a second copy for Overview's convenience.
- **The per-event Attention list deletion was scoped to ONLY the Attention
  rows, not the whole Inventory Intelligence card.** `EventDetail.tsx`'s
  `InventoryIntelligenceBlock` keeps its KPIs/Aging/By tier/section/
  marketplace sections exactly as they were (2.2.6/2.2.7) - only the
  `ATTENTION_COPY` const and the "Attention" `<div>` block (plus the now-
  unused `AttentionItem` type import and `IconAlertTriangle`/`IconCheck`
  icon imports) were removed. **The backend `get_inventory_intelligence`
  command/impl function is completely untouched and must stay that way** -
  `commands/attention_center.rs` calls `get_inventory_intelligence_impl`
  directly (a plain Rust function call, not through the frontend) for 4 of
  its own 5 categories; deleting or changing that backend function would
  silently break the Dashboard's Attention Center, not just this one
  now-removed frontend block.
- **Attention Center (2.2.8) reworked to group by ORDER, not ticket.**
  `AttentionCenterItem` (`models.rs`/`types.ts`) dropped its old singular
  `ticketId`/`ticketCode` fields for `orderId`/`orderCode` (both `null`
  only for `event_soon`) plus `ticketIds`/`ticketCodes` arrays (every
  ticket the row now stands for - length 1 for an order with only one
  affected ticket). `commands/attention_center.rs`'s new `group_by_order`
  helper groups a category's flagged ticket ids by `TicketMini.order_id`
  (a NEW field on that struct - the module's own independent
  `SELECT ... FROM tickets` query now also selects `order_id`) before
  emitting one `AttentionCenterItem` per (event, category, order) instead
  of per ticket; a new `orders_by_id` lookup (`SELECT id, code FROM
  orders`) resolves each group's human-facing order code. This relies on
  every ticket under one order sharing that order's `event_id` - true by
  construction (`orders::create_order_impl` inserts the order and every one
  of its tickets with the same `input.event_id` in one call) - do not group
  by order across a query that could mix tickets from different events
  without re-verifying that invariant still holds. `event_soon` is
  UNCHANGED: still one row per event, `orderId: None`, `ticketIds: []` -
  it has no single order to group under (a soon event's unsold tickets can
  span several orders), and was already aggregated at the coarser event
  level before this task even started. `amountCents`/`currency` (only ever
  populated for `outside_market_price`) are now `None` whenever a group has
  more than one ticket - there is no single "the" listing price for a
  multi-ticket group, so this deliberately does not guess or average one.
  Dashboard's `AttentionCenterRow` (`Dashboard.tsx`) now links a grouped row
  to `/orders/:id` (`OrderDetail.tsx`, which already lists every one of
  `ticketIds` with its own status/listing price/delivery indicators)
  instead of a single ticket's `/tickets?code=` deep link - a genuinely new
  navigation target for this feature, not a reuse of the 2.2.8 pattern, but
  still an EXISTING route/page, consistent with this feature's own original
  "click-to-navigate via existing routes" design. If a future change adds a
  6th category, decide explicitly whether it is ticket-level (group by
  order, like 4 of the current 5) or event-level (like `event_soon`) before
  writing it - don't default to per-ticket rows again, that is exactly the
  "nedáva zmysel" shape marko asked to have fixed this round.
- **Seats formatting: `formatSeatsSummary` (`src/lib/format.ts`) changed
  its OUTPUT FORMAT ONLY, not its grouping/compaction logic.** Each
  section+row group now renders as `formatSeatLocation`'s own
  "Sec X · Row Y · Seat Z" labeled/dot-joined text instead of a bare
  "X/Y Z" slash-join - the grouping-by-section+row and seat-number-range-
  compaction (`compactSeatList`) are byte-for-byte unchanged. Six ad-hoc
  duplicate `[section, rowLabel, seat].filter(Boolean).join(" / ")` call
  sites inside `EventDetail.tsx` (Tickets tab, Listings tab, Create Sale
  modal x4) and two more inside `Sales.tsx`'s own Create Sale modal were
  replaced with calls to the shared `formatSeatLocation` instead of being
  hand-patched - if a NEW ad-hoc `.join(" / ")` for section/row/seat shows
  up anywhere in a future change, that's a regression of this cleanup, not
  a legitimate new one-off. Two sites' empty-state copy changed as a
  side-effect (from "No seat info"/a bare "-" to "General admission") since
  `formatSeatLocation` always returns a real label - this was a deliberate
  consistency choice (matching OrderDetail.tsx/SaleDetail.tsx's own
  existing convention for the same fallback), not an oversight.

## 2.2.8 - Dashboard global "Attention Center"

**Superseded in part by 2.2.9 (above): the 4 ticket-level categories now
group by order, not ticket** - anywhere below that says "per ticket"/"per-
ticket row" for `missing_listing_price`/`no_active_listing`/
`outside_market_price`/`sold_undelivered` describes the shape as it
shipped in 2.2.8, not how it behaves today. Kept here unedited for history,
per this file's own "not a changelog... just traps worth knowing" policy -
see the 2.2.9 entry above for the current shape. Everything else below
(the separateness from `DashboardAlerts`, the reuse of
`get_inventory_intelligence_impl`, the priority mapping, `event_soon`'s
per-event granularity) is still accurate as written.

Focused task adding one new, GLOBAL (every event) Dashboard block listing
individual things needing a look, built almost entirely by reusing
already-shipped logic (`commands/attention_center.rs`, new file; one new
read-only command `get_attention_center`; no migration, no dependency).
Things worth knowing before touching this again:

- **This is a SEPARATE, ADDITIONAL feature from the Dashboard's existing
  `DashboardAlerts` (the alert bell + Activity tab's own "Attention" cards,
  2.0.75/2.0.76/2.0.79) - it does NOT replace or merge with it, and that
  existing feature is completely untouched.** The two overlap on exactly
  one thing - unsold tickets with no listing price - computed the same way
  in both (ticket-scoped, `status IN ('available','listed') AND
  listing_price_cents IS NULL`) but surfaced differently: `DashboardAlerts`
  counts it per ORDER (feeds a glanceable card + the outbound-notifications
  feature), this new block lists it per TICKET (an actionable inbox row).
  Don't "simplify" these into one code path without checking both
  consumers still get what they need - they were kept separate on purpose,
  not by oversight.
- **Four of the five categories are a byte-for-byte reuse of
  `inventory_intelligence::get_inventory_intelligence_impl`'s own
  `attention` list (2.2.6)** - this module calls that function once per
  event that has at least one unsold ticket (a plain performance
  pre-filter via `events_with_unsold`, never a correctness one - an event
  with zero unsold tickets could never produce any of these 4 anyway) and
  flattens the result into individual rows. A future change to
  `EVENT_SOON_DAYS`/`OUTSIDE_MARKET_THRESHOLD_PCT` or either predicate in
  that module automatically applies here too - there is nothing duplicated
  that could quietly drift out of sync. The one visibility change this
  task made to that file: `EVENT_SOON_DAYS` went from private to
  `pub(crate)` so this module could reuse it directly - zero behavior
  change.
- **`event_soon` is deliberately ONE ROW PER EVENT, not one per unsold
  ticket - the other 4 categories are per-ticket.** Marko's own spec listed
  "Ticket/code" as optional ("ak je relevantný"), and an event with e.g. 40
  unsold tickets 1 day out would otherwise flood the list with 40
  near-identical rows, directly against his own "UI musí zostať
  prehľadné". Mirrors how the Dashboard's own existing "Upcoming events"
  list already shows one row per event, not one per ticket. If a future
  ask specifically wants per-ticket event-soon rows, that's a deliberate
  reversal of this call, not a bug fix.
- **The fifth category (`sold_undelivered`) is a NEW alert this task adds,
  and it IS reliably computable - it was not omitted.** It reuses the
  exact `tickets.delivery_status = 'Delivered'` convention the 2.0.66
  "Completed" indicator already established (`orders.rs`/`sales.rs`'s own
  `delivered_count` - see migration 010's own doc comment for why
  `delivery_status` is free text with no CHECK enum: despite that, the
  app's OWN bulk-update commands and the 2.0.66 indicator already treat it
  as an effective two-value field, "Delivered" vs. everything else). Scoped
  directly to `tickets.status = 'sold'` - a refund reverts that column back
  to `'available'` (`refund_sale_impl`), so a refunded ticket drops out on
  its own with no extra join needed. Independent of the
  `events_with_unsold` pre-filter above on purpose - a fully sold-out event
  can still have an undelivered sold ticket (covered by its own regression
  test).
- **Priority mapping (Critical / Attention / Info) is a brand-new judgment
  call this task makes - marko named the 3 tiers but not which category
  goes where:** `event_soon` is always Critical (irreversible if missed -
  can't sell a ticket after the event has passed). `missing_listing_price`
  and `no_active_listing` are Attention (real, actionable gaps in your own
  process). `outside_market_price` is Info, not Attention - deliberately,
  since marko was explicit this task must never recommend or imply a price
  action ("žiadne automatické určovanie ani navrhovanie ceny"); it's a
  pricing OBSERVATION, not a gap to fill in. `sold_undelivered` is Critical
  when the event is within `EVENT_SOON_DAYS` of today OR already in the
  past (reusing that exact constant rather than a new threshold - same
  "no lower bound, never exempt a past event" reasoning `dashboard.rs`'s
  own `PULLS_WARNING_WINDOW_DAYS` check already documents), Attention
  otherwise, and Attention (never a guessed Critical) when the event has no
  `event_date` at all. Revisit this mapping only if marko explicitly says
  a category feels mis-prioritized - it's one small match statement in
  `attention_center.rs`.
- **Click-to-navigate reuses the ONLY cross-page ticket deep link this app
  already has**: `Tickets.tsx`'s own `?code=` query param (prefills the
  search box - see the 2.2.6 entry below for why no richer cross-page
  ticket filtering exists anywhere in this app yet). Event-level rows
  (`event_soon`) link to that event's own Event Workspace (`/events/:id`).
  No new route, no new navigation mechanism was added - if a future ask
  wants Tickets.tsx to actually auto-open one ticket's edit modal instead
  of just pre-filling search, that's a real, separate change to
  `Tickets.tsx` itself, not something to bolt onto this block.
- **"Reasonable limit + Show all" reuses the Activity tab's existing
  `ShowMoreToggle` component and `RECENT_LIST_PREVIEW_COUNT` constant
  verbatim, applied per priority group** - the backend command itself never
  truncates or paginates; `get_attention_center_impl` always returns the
  complete list, exactly like `inventory_intelligence.rs`'s own ticket-id
  lists do. If this list ever grows large enough that returning everything
  becomes a real cost, that's a backend pagination change to make
  deliberately, not something to silently cap without telling marko.
- **No table/list in this app shows tier/section/row as a pricing input,
  and this task doesn't start now** - `outside_market_price`'s "value" is
  always the ticket's own already-entered `listing_price_cents`, never a
  computed/suggested figure, and the market-average comparison itself is
  entirely inside the already-shipped, unmodified
  `get_price_checker_summary_impl`.

## 2.2.7 - Ticket metadata: Tier / Level (Section / Row confirmed unchanged)

Focused task, fully resolving the recurring "tickets have no tier/level
column" gap the "2.2.6" and "2.2.0" entries below both found and flagged as
checked-not-missing. `tickets.tier` (new nullable `TEXT` column,
`migrations/024_ticket_tier.sql`) now exists for real - **the 2.2.0/2.2.6
entries below describing its absence are now historical only**, kept for
context (they explain WHY it didn't exist yet and what the smallest fix
would look like, which is exactly what got built here). Things worth
knowing before touching this column again:

- **`tier` is a separate field from `ticket_type` - this is now the THIRD
  time this exact mix-up has been flagged in this file** (see "2.2.0" and
  "2.2.6" entries below). `ticket_type` is a DELIVERY method (E-ticket/PDF/
  Mobile transfer/Physical/Will call, `TICKET_TYPES` in Orders.tsx); `tier`
  is a seating/pricing category (VIP/Lower Bowl/Level 200/...), free text,
  no CHECK enum (same reasoning as migration 010's resale_status/
  delivery_status - marko's own vocabulary, not a closed set this app
  should validate). Never conflate the two again.
- **No standalone "Add Ticket" flow exists anywhere in this app - tickets
  are only ever created via Order creation.** `tier` is therefore entered
  in exactly the same two places `section`/`row_label` already are:
  `OrderFormModal` (order-level, copied onto every ticket the order
  generates via `insert_order_with_tickets`) and `TicketEditModal`
  (per-ticket edit afterward, `update_ticket_impl`). If a real
  per-ticket-at-creation-time flow is ever built, `tier` needs wiring
  there too - this task only touched the two entry points that already
  existed for section/row.
- **Existing tickets got NULL, never a guessed/inferred value** - marko's
  own explicit instruction ("nevymýšľaj automatické hodnoty tier/section/
  row pre existujúce tikety"). The migration is a plain `ALTER TABLE ADD
  COLUMN tier TEXT`, no backfill of any kind.
- **Column order convention for this task: `tier` sits immediately after
  `row`/`row_label` everywhere it appears** - Rust struct fields, SQL
  SELECTs (see below), CSV headers (both the import template and the
  Settings description text), and both UI forms. Keep new order/ticket
  fields grouped this way if more get added later; don't scatter them
  alphabetically or tack them onto the end just because that's less
  typing.
- **`csv_export.rs`'s existing "append new SQL columns at the end, read
  back BY NAME" convention was followed for `tier` too** - the SELECTs in
  `export_tickets_inner`/`write_sales_csv` got `t.tier` appended at the
  very end, never inserted where "tier" sits in the human-facing header,
  so none of the pre-existing POSITIONAL `row.get(i)` indices in either
  function shifted. Several existing tests in that file DID still need
  their hardcoded column-index assertions bumped by one, though, because
  the human-facing CSV header/`write_record` array order (which those
  tests read back with `csv::Reader`) is intentionally independent from
  the SQL SELECT order. Re-check every `rows[0][N]`-style assertion in a
  test file after adding a column there, not just the SQL/struct code -
  this cost real time this round (5 pre-existing sales-export tests had
  silently-wrong indices after the insertion, all caught by `cargo test`,
  none by `cargo check`).
- **Migration-upgrade tests that seed "pre-existing data" via the CURRENT
  production `insert_order_with_tickets`/`OrderInput` break the moment
  that function's SQL references a column added by a LATER migration than
  the one under test.** `db.rs`'s `migration_004_tests`/`migration_007_tests`
  both did exactly this (seeding via `insert_order_with_tickets` against a
  deliberately-partial `MIGRATIONS[..3]`/`MIGRATIONS[..6]` schema) and
  broke as soon as `tier` (migration 024) was added to that function's
  INSERT - fixed by seeding those two tests with plain SQL matching their
  OWN historical schema instead (same pattern `migration_024_tests`
  already used for its own seeding). If a future column ever gets added
  to `insert_order_with_tickets`'s INSERT, grep `db.rs`'s migration test
  modules for `insert_order_with_tickets`/`OrderInput` first and check
  whether they still apply fewer migrations than exist at the time.
- **`inventory_intelligence.rs`'s breakdown-by-tier uses "Unknown" for
  blank/null - deliberately DIFFERENT wording from the section
  breakdown's own "No section"**, per marko's own explicit instruction for
  this one field specifically. Don't "harmonize" the two labels later
  without checking this was intentional.
- **No bulk-tier-edit was added** - `BulkTicketField`/
  `BulkTicketUpdateInput` (`tickets.rs`/`models.rs`) deliberately still
  only cover Section/RowLabel/Seat/ListingPriceCents. Not an oversight;
  bulk-editing tier wasn't asked for, and adding it means deliberately
  extending that closed enum, not doing so by default.
- **No tier column was added to any list/table display** (Tickets.tsx's
  list, OrderDetail.tsx's ticket table, Sales.tsx, SaleDetail.tsx) -
  checked first: section/row aren't shown as table columns in any of
  those today either, so leaving tier out too is consistent, not a gap.
- **Google Sheets Order sync (`orders_sheet_sync.rs`) is deliberately NOT
  wired to `tier`** - no sheet column exists for it, and adding one is a
  separate, out-of-scope decision (which column, what header text, does
  marko even want it in the sheet at all). The one call site that
  constructs an `OrderInput` from sheet data sets `tier: None` with a
  comment explaining this is deliberate, not a bug.
- **`price_checker_analysis.rs`'s `YourTicketGroup.tier` is STILL always
  `None` - this is now the second time this exact field has been
  deliberately left unwired** (see "2.2.0" entry below for the first). The
  real column now exists, but wiring it into Market Analysis grouping is
  explicitly a NOT-YET-DONE follow-up per marko's own "pripravit data,
  nepouzivat este" instruction this round - don't wire it in without him
  asking for that specific next step, and don't be surprised the data is
  sitting right there unused if you're reading this before that ask
  happens.

## 2.2.6 - Inventory Intelligence block on Overview

Focused, explicitly-scoped task on top of 2.2.5: a compact "Inventory
Intelligence" block above the Orders/Tickets tables on the Overview tab -
KPIs, an aging breakdown, an attention list, and section/marketplace
breakdowns, all reusing existing money logic (see `CURRENT_STATE.md`'s
"Current focus" section for the full field list). One new file
(`commands/inventory_intelligence.rs`), one new command
(`get_inventory_intelligence`), no migration, no new dependency. Things
worth knowing before touching this again:

- **This task's own closing checklist initially named only targeted
  tests, a build check, and updating these two docs - not a version bump,
  a `REDESIGN-*-REPORT.md`, or packaging, unlike every prior task.** Taken
  literally at first (implemented, tested, docs updated, closing summary
  given in chat, no bump/report/zip) and flagged to marko as a judgment
  call in that summary; he confirmed in the very next message that he did
  want the file too ("jasne ze aj file potrebujem"), so the normal cadence
  (version bump, this report, packaging) was completed right after, same
  session. Net effect: a real, normal 2.2.6 release - the brief gap is
  recorded here only so a future session isn't confused by any leftover
  trace of the shorter path (e.g. a stray "2.2.6" code comment written
  before the bump actually happened).
- **Two parallel "listing price" systems both got reused, deliberately not
  unified.** "Current listed value" sums ACTIVE `ticket_listings.price_cents`
  (the 2.2.4+ real per-marketplace system, matching Listings' own "Listed
  value" card); "Potential profit" uses the LEGACY single
  `tickets.listing_price_cents` field (matching Sales' own existing
  "Potential Profit" card and `price_checker.rs`'s market-comparison
  summary). Both are real, existing, independently-shown numbers elsewhere
  in this app - unifying them was not asked for and would make one of them
  disagree with a number marko already trusts on another tab. Don't merge
  these into one "listing price" concept without asking first.
- **HISTORICAL, superseded by 2.2.7 above: "No 'by tier' breakdown" is no
  longer true.** `tickets.tier` exists now (migration 024) and the
  breakdown shows it for real - see 2.2.7's own entry above. The original
  note is kept below for context (it correctly predicted the smallest fix,
  which is exactly what 2.2.7 built). Original text: checked, not missing,
  same gap 2.2.0's own entry above already found. `tickets` has no
  tier/level column anywhere in the schema; `ticket_type` is a delivery
  method (E-ticket/PDF/Mobile transfer/Physical/Will call), not a price
  tier. Per marko's own explicit instruction that round ("ak niektorý údaj
  už databáza spoľahlivo nevie vyrátať, nevymýšľaj fallback dáta"), this
  breakdown was simply omitted, with a plain-text note in the UI saying
  so. Smallest fix if marko wants it for real: a new nullable
  `tickets.tier` column plus UI to set it (Add/Edit ticket forms, CSV
  import mapping) - a real migration + several small UI touch points, not
  a quick add.
- **Two numeric judgment calls, neither given an exact number by marko -
  both easy to tune, both isolated to one `const` each in
  `inventory_intelligence.rs`:** `EVENT_SOON_DAYS = 2` (marko said "48h";
  `event_date` has no time component anywhere in this schema, so this
  means "event date is today, tomorrow, or the day after," not a rolling
  48-hour clock) and `OUTSIDE_MARKET_THRESHOLD_PCT = 0.20` (marko said
  "výrazne mimo market ceny" - significantly outside - with no number).
  Change either in one place if marko wants a different threshold; no
  other code depends on the specific values.
- **Sell-through % denominator is ALL tickets including cancelled ones**,
  matching the "Total tickets" KPI shown right next to it in the same row
  (same scope `finance::compute_summary`/`events.rs`'s own stats already
  use) - an "exclude cancelled" reading of sell-through is equally
  defensible and was NOT what got built; revisit only if marko says the
  number looks wrong against his own mental model.
- **The aging buckets fix an overlap in marko's own spec.** He listed
  "8-30" and "30-60" (day 30 in both); implemented as 0-7 / 8-30 / 31-60 /
  61+ so every unsold ticket lands in exactly one bucket. Flagged here
  rather than silently double-counting day-30 tickets in both buckets.
- **Every clickable KPI/aging/attention/breakdown row filters Overview's
  own Tickets table by an id list the backend already returns** - it does
  NOT navigate to Tickets.tsx or Orders.tsx, because neither page has any
  URL/state-based filtering mechanism to receive such a list today
  (Tickets.tsx only supports `?code=` for one ticket; Orders.tsx's
  `presetEventId` only pre-fills the New Order modal). "Current listed
  value" is the one exception - it switches to this same page's own
  Listings tab (`onSwitchTab`), since that number is about
  `ticket_listings` rows, not raw tickets. If a future ask wants real
  cross-page deep links (e.g. "open Tickets.tsx pre-filtered"), that means
  adding actual filtering support to those pages first, not extending this
  block's local-highlight approach further.

## 2.2.5 - Event Workspace down to 3 tabs; Listings gets filters/search/bulk actions

Fourth pass on the Event Workspace, plus one small Price Checker lookup
addition. Final tab order: **Overview | Listings | Sales** (Finance folded
into Sales - see the judgment-call note below). Things worth knowing before
touching this page or `ticket_listings.rs` again:

- **The Sales-absorbs-Finance judgment call.** 2.2.4's own entry below
  explains why Finance stayed a separate tab that release: marko's message
  said "Sales Market Finance spoj do jedneho" (merge these) but then
  explicitly listed "sales" AND "finance" as two of the 4 surviving tabs,
  so only Market got folded (into Sales). This round marko asked again,
  this time with no such list keeping them apart - "sales a finance daj
  dokopy" (put sales and finance together), unambiguous. Same "first-named
  tab survives, absorbs the rest" convention as every merge on this page so
  far (Overview/Inventory, Sales/Market) - Sales was named first, so Sales
  is the surviving tab; Finance's entries table now renders at the bottom
  of `SalesTab`, below the Market section 2.2.4 already put there. Flagged
  in `EventDetail.tsx`'s own top-of-file doc comment and in
  `REDESIGN-2.2.5-REPORT.md` in case marko meant the name the other way
  round - it's one clearly-bounded block, easy to move back out.
- **Listings bulk actions (`bulk_update_ticket_listings_status`/`_price`,
  `bulk_delete_ticket_listings`, all in `ticket_listings.rs`) are
  ALL-OR-NOTHING transactions - deliberately stricter than
  `bulk_delete_sale_groups`'s own "per-item skip, report what failed"
  contract.** Marko explicitly asked for "ideálne transakčné - buď sa
  zmena podarí pre všetky vybrané listingy, alebo pre žiadny" this round -
  every id is validated to exist in one query BEFORE any row is written,
  same "validate everything, then write everything" shape as
  `tickets::bulk_update_tickets_impl`/`sales::bulk_update_sale_payment_
  status_impl`. Do not weaken this to a skip-and-report model to match
  the sales-list convention without re-confirming with marko first - this
  was an explicit, named requirement, not a default this codebase always
  uses for bulk actions.
- **Bulk price edit refuses a mixed-currency selection outright, on BOTH
  ends.** `bulk_update_ticket_listings_price_impl` reads back every
  selected listing's CURRENT currency in its validation query and rejects
  the whole batch if more than one distinct value is found - a bare amount
  can never be applied blindly across listings priced in different
  currencies. The frontend (`ListingsBulkBar`) also disables the "Edit
  price" button up front when the selection is mixed, so this is defense
  in depth, not the only guard. Currency itself is NEVER written by this
  command - only `price_cents` changes, per marko's own "zachovaj
  currency" instruction.
- **Listings' row checkboxes are always visible - no separate "selection
  mode" toggle** like Sales.tsx/Orders.tsx use for their own (usually much
  longer) lists. Deliberate, reversible UI call for this smaller,
  per-event table - flagged in the report, not in marko's own spec.
  Select-all/deselect-all is scoped to the currently filtered/searched
  rows only, same convention as Sales.tsx's own `allSelected`/
  `toggleSelectAll`.
- **The "Add listing" ticket picker was rebuilt as an order-browse flow**
  (search this event's own orders, open one, pick tickets from it, repeat
  across orders) mirroring Sales.tsx's New Sale flow almost exactly -
  marko's own explicit ask ("urobme to ako pri sale"). Unlike New Sale,
  this needs NO live fetch: `TicketListingFormModal` already receives this
  whole event's own `tickets`/`orders` as props (small, already loaded by
  `ListingsTab`), so the picker is pure client-side filtering. Several
  tickets can be selected at once ("vybrat dany pocet listkov" taken
  literally) - creates one listing per selected ticket on the one chosen
  marketplace. **This batch-create path is deliberately NOT all-or-nothing**
  (unlike the 3 bulk actions above) - marko's "transactional" requirement
  was stated specifically for editing EXISTING listings, not for this
  create flow; a failure partway through keeps whatever succeeded and
  reports exactly which tickets failed and why, leaving just those
  selected to retry. Listing ID/URL are only offered when exactly one
  ticket is selected (each marketplace posting has its own external id/
  URL - a shared value across a batch would be meaningless); for a batch,
  they're added afterward via Edit on each created listing.
- **`marketplaces` gained a 4th seeded row, Seatriks** (marko's request -
  `migrations/023_add_seatriks_marketplace.sql`, pure data, no schema
  change - same precedent as `020_remove_stubhub.sql` for a data-only
  migration against this table). Spelling/capitalization is my own best
  guess at what marko typed ("seatriks") - trivially renameable from Price
  Checker's own marketplace list if wrong, no migration needed for that.

## 2.2.4 - Event Workspace down to 4 tabs; Listings is now a real system

Third pass on the Event Workspace. Final tab order: **Overview | Listings |
Sales | Finance** (Inventory folded into Overview, Market folded into
Sales - see the judgment-call note below). A few things worth knowing
before touching this page or the new backend module again:

- **The Market-vs-Sales judgment call.** Marko's request grouped "Sales
  Market Finance spoj do jedneho" (merge these into one) but then, in the
  same message, explicitly listed the 4 tabs that should remain -
  "overview, listings, sales a finance" - keeping BOTH "sales" and
  "finance" as separate names. Since Market is the one name that
  disappeared from that list, its content (Market vs. mine + Potential
  Profit) was folded into Sales, not Finance - Market's content is about
  pricing/what-could-I-get-for-this-now, which reads as a Sales concern
  more than a Finance ledger one. This is flagged in `EventDetail.tsx`'s
  own top-of-file doc comment and in `REDESIGN-2.2.4-REPORT.md` so marko
  can correct it if he meant the other tab - if he does, it's a small,
  self-contained move (the whole block is one clearly-bounded section at
  the bottom of `SalesTab`).
- **Inventory's old bullet points from the 2.2.2 entry below are now
  historical only** - there is no more "Inventory" tab; its Orders/Tickets
  tables now render at the bottom of `OverviewTab`, completely unchanged
  otherwise. Same for "Market" in that same entry.
- **`ticket_listings` (new table, `migrations/022_ticket_listings.sql`) is
  a real multi-marketplace listing system - one ticket can have several
  rows, one per marketplace.** Reuses the EXISTING `marketplaces` lookup
  table (Price Checker's own, `014_price_checker.sql`) via
  `marketplace_id` rather than inventing a second marketplace concept.
  Both `ticket_id` and `marketplace_id` are `ON DELETE CASCADE` - the
  established rule for every `marketplace_id` column in this schema (see
  the "2.2.0" entry below). **`commands::price_checker::
  delete_marketplace_impl`'s own existing guard query was extended to also
  count `ticket_listings`** - without this, deleting a marketplace that
  still has real listings against it would have silently cascaded them
  away, exactly the bug that guard already exists to prevent for
  `event_marketplace_links`/`price_checks`. Grep for `marketplace_id`
  across migrations again before adding yet another such table.
- **The dedup constraint is `UNIQUE(ticket_id, marketplace_id, listing_id)`,
  with `listing_id` nullable.** SQLite treats every `NULL` as distinct in a
  UNIQUE index, so several hand-entered listings for the same ticket +
  marketplace with no external id yet can coexist - only an EXACT repeat
  (same ticket, same marketplace, same real listing id) is rejected. Don't
  "simplify" this to `NOT NULL` or drop `listing_id` from the key without
  re-reading this reasoning - marko will very often not have an id to
  enter, this being manual-entry-only (no marketplace API this release).
- **`ticket_listings.status` (`active`/`sold`/`removed`) is deliberately a
  SEPARATE vocabulary from `tickets.status` (`available`/`listed`/`sold`/
  `cancelled`), and this feature never reads or writes `tickets.status`/
  `tickets.listingPriceCents` at all.** One ticket can now have listings in
  different states on different marketplaces at once (active on Vivid,
  sold on Ticombo) - folding this into `tickets.status` would make that
  column ambiguous, and marko explicitly asked not to touch existing
  tickets/inventory/sales/refund logic. If a listing being marked "sold"
  should ever also flip the ticket itself to sold, that is a deliberate
  NEW cross-writing behavior to design and confirm with marko, not
  something to wire in quietly.
- **The Listings tab's table shows every listing regardless of status**
  (not just active) - the top summary cards (`Active listings`/`Listed
  value`/`Lowest`/`Highest`) are the ones scoped to `status === "active"`
  only. This is deliberate: marko's own field list asks for a `status`
  column, which is only meaningful if a row can show something other than
  "active".

## 2.2.3 - Event Workspace: Tasks removed, Listings tab added, tables full-width

Second pass on the Event Workspace from 2.2.2, all frontend-only
(`EventDetail.tsx`). Final tab order: **Overview | Inventory | Listings |
Sales | Market | Finance**. A few things worth knowing before touching
this page again:

- **The Tasks tab is gone, completely - not just hidden.** Marko decided
  against it before it ever got a spec (see 2.2.2's own entry below: it
  was always an `EmptyState` placeholder with no schema, no commands, no
  types). The 2.2.2 bullet about Tasks below is now historical only - read
  it as "this is what used to be there," not as a description of current
  code. If Tasks ever comes back, it starts from nothing again; there is
  no branch or dead code to resurrect.
- **The new Listings tab deliberately does NOT show marketplace, listing
  URL, or last-checked/updated - three of the seven fields marko asked
  for - because none of them exist anywhere in this codebase.** Verified
  by reading the full `Ticket` TypeScript interface, grepping every
  `ALTER TABLE tickets` across all 21 migrations (only migration 010 ever
  added columns beyond the original 001 schema: `resale_status`,
  `delivery_status` - neither is a marketplace/URL/timestamp field), and
  checking Price Checker's own `YourTicketGroup` (the closest existing
  "listing" concept), which also has none of the three. The tab shows
  ticket/listing price/currency/status (all real, all already on
  `tickets`) plus an Active listings/Listed value/Lowest/Highest summary,
  and says plainly, in the UI itself, that the other three aren't tracked
  yet. **Do not add fake/placeholder columns for these to "complete" the
  table** - if marko wants them tracked for real, that is a real schema
  migration (new `tickets` columns, likely a new marketplace-link concept)
  and a deliberate follow-up, not a UI-only addition.
- **`ListingsTab` receives `tickets` as a prop from the same fetch
  `InventoryTab` already uses - no new API, no new fetch, no new
  filtering pass beyond `status === "listed"` client-side.** Matches
  marko's explicit "žiadne nové API" / "nevytváraj duplicitný systém"
  constraints for this release. If Listings ever needs data `tickets`
  doesn't carry, that's a sign a real backend change is due, not a reason
  to bolt more client-side derivation onto this component.
- **The `max-w-[1400px]` cap was removed from all 4 of this page's tables**
  (Orders/Tickets in Inventory, Sales, Finance) so they fill the window
  width on wide screens - the same fix `Layout.tsx` itself got in 2.0.31,
  just never carried over to these tables. Deliberately NOT given
  `Sales.tsx`'s own percentage-based `colgroup` treatment (see that file's
  2.0.32-2.0.35 history) - that was judged to be more machinery than this
  ask calls for. Accepted trade-off: a column could look oddly stretched
  on a very wide/ultra-wide monitor. Only add a colgroup here if marko
  actually reports that happening - don't pre-emptively engineer it.

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
  produce nonsense groupings. **Update, 2.2.7: that real column now
  exists (`tickets.tier`, migration 024) - but this field is STILL
  deliberately `None`, unwired on purpose per marko's own "prepare, don't
  use yet" instruction. See 2.2.7's own entry above before changing
  this.**
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
  additive, not a replacement. Rule held even when 2.2.6's own closing
  checklist initially didn't ask for one - see that task's own dated entry
  above: implemented without a bump/report first, then marko confirmed he
  wanted the full cadence too, so it was completed the same session. Don't
  assume a closing checklist that omits "version bump"/"report" means
  marko doesn't want them this time - ask, or flag the interpretation
  plainly and let him correct it, rather than silently skipping the report
  for good.
- **Secrets stay plain text in `app_settings`** - this is an accepted,
  existing trust boundary across the whole app (Google OAuth refresh
  token, Pushover user key, Firebase config, etc.), not something to
  unilaterally "fix" by adding encryption without marko explicitly asking
  for that specific change.

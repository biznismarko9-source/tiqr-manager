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

## 2.13.2 - Separate cards again, editable order prices, two charts

marko's own list after running 2.13.1. **No schema change, no migration (next
new one is still 027), no dependency change.**

1. **Back to one card per figure.** 2.13.1 put every summary figure in a
   single bordered bar; held up against the Attention Center row, marko wanted
   separate boxes. `StatCard` is a card again and `.summary-bar` is now just
   the row that holds them - one component change plus one CSS line, so all 13
   wrappers, Ticket Center's filters and the Calendar tiles followed without
   being touched. `SummaryStat` survives from 2.13.1 for Sales' own results
   strip, which genuinely is one line of text.
2. **An order's purchase price is editable after creation.** Unit price, fees
   and other costs, re-split across the order's tickets with the exact
   `allocate_cents` the create path uses - **including tickets already sold,
   which does retroactively change the profit reported on those sales.**
   marko's explicit choice over blocking the edit; the dialog says so whenever
   the order has sold tickets. 5 new tests.
3. **Fixed before it shipped**: the first version of that form used
   `parseFloat`, and `parseFloat("12,50")` silently returns `12`. It now uses
   `decimalStringToCents`, which the app already had and which handles the
   comma marko actually types. An empty unit price errors instead of zeroing
   the order.
4. **Fixed**: `orders_sheet_sync.rs` also builds an `OrderEditInput`, which the
   new required fields broke. It now reads the three price columns out of the
   database and passes them through, the same way it already did for
   `supplier_id` and `payment_status` - the sheet does not carry a price, and
   filling them with 0 would have zeroed every synced order's price.
5. **Pulls**: the Delete / New Pull buttons sit on the filter row instead of a
   right-aligned row of their own, which had left a band of empty space.
6. **Dashboard: two charts instead of one wide one**, each with its own metric
   switch rather than one being chosen for him. A fourth metric, **Cost**,
   joins Profit / Revenue / Sales - `cogs_cents` has been on every bucket since
   1.6.0 and was the one real series never offered. The headline figure reads
   the same field its line draws (`cogs`, not `total cost`), so the number and
   the chart cannot disagree.

**Not built here:** no toolchain on the machine this was written on. 2.13.2's
first build failed on point 4 above and nothing was ever published under that
number, so it is reused rather than bumped - same reasoning as the cancelled
2.4.0 direction.

## 2.13.1 - One summary style, everywhere

marko ran 2.13.0, pointed at the summary strip `Sales.tsx` has had since
1.9.0 - one border around the whole row, grey label, value beside it - and
asked for exactly that shape on every screen. **No backend change, no schema
change, no migration (next new one is still 027), no dependency change.**

1. **`SummaryStat` moved from `Sales.tsx` into `ui.tsx`** and is now the one
   implementation of a summary figure in the app. `StatCard` is a thin
   wrapper over it (trend and `sub` append inline), so its ~50 call sites
   follow automatically and Sales stops having its own private copy.
2. **`.stat-chip` became `.summary-bar`.** 2.13.0's chips were lighter than
   the old cards but still N bordered objects above a table; this is one
   border around the row. The 13 wrappers converted with it - and they no
   longer carry their own `mb-*`, since `.summary-bar` brings its own.
3. **Ticket Center's four filter cards are segments of that bar.** They were
   the largest thing on a screen whose point is the table underneath. Still
   buttons, still the filter, same counts. Their subtext moved to a `title`
   tooltip - a strip has no second line, and that text explained the count
   rather than naming it.
4. **Calendar's Today / Tomorrow / Next 7 days / Overdue** got the same
   treatment. Tiles with nothing to open now render as plain text instead of
   dead buttons.
5. **Dashboard's profit headline dropped 34px → 26px.** Still the loudest
   number on the page; no longer shouting.
6. **Sync is one button.** It works out the direction itself: if the Drive
   copy has not moved since this machine last synced, it pushes. If it has,
   both sides may hold work and nothing in the app knows whether the local
   side is also dirty - so it stops and asks (*Take theirs* / *Keep mine*)
   rather than guessing. The explicit Sync up / Sync down pair is still there
   under *Choose direction myself*.
7. **The app notices by itself.** `Layout` calls `cloud_sync_status` once on
   open and shows a dismissible bar when the other computer has newer data.
   It **checks and tells - it never syncs**; nothing moves without a click,
   so local-first still holds. This amends 2.12.0's "no startup sync" note in
   `PROTECTED_AREAS.md`, at marko's explicit request.

**Not built here:** no Node or Rust toolchain on the machine this was written
on, so nothing was compiled or tested - the first build is the first check.

## 2.13.0 - Visual redesign across seven screens, plus one balance fix

marko reviewed named alternatives one screen at a time and picked each of
these himself. **No schema change, no migration (next new one is still 027),
no dependency change, no change to refund/resell, `batch_id` or money
handling.** One backend change, unrelated to the redesign, is listed last.

1. **`StatCard` is a chip, not a bordered box** (`ui.tsx`, `index.css`). The
   summary row sat above ten screens at the same visual weight as the table
   underneath it and read as furniture. Changed on the **component**, so all
   ~50 call sites move together and the app keeps ONE summary style - which
   also means `EventDetail`, a screen this round never reviewed, changed with
   it. The callers' `grid` wrappers became `flex flex-wrap gap-2` (12 sites):
   a chip inside a grid cell stretches to the column and stops reading as a
   chip. Trend and `sub` are kept inline rather than dropped.
2. **Sidebar** (`Layout.tsx`): tighter rows, two quiet section headings, and
   the Tickets group given its own surface (the 2.6.0 hairline guide rail and
   the deep indent both go - the card is the grouping cue now). **No counts**,
   deliberately: `Layout` fetches nothing today and marko chose to keep the
   navigation pure frontend rather than add a command to feed numbers.
3. **Ticket Center** (`TicketCenter.tsx`): the `Completed` column mixed a
   stock fact ("Not Sold"), a payment fact ("Not Paid") and a count ("2
   Pending") under one heading, so it had no single meaning and a row never
   said why it was listed. Replaced by a **Needs** column built from the same
   `matchesCategory` predicate the four filter tiles already count with, so
   tiles and rows cannot disagree. Rows now sort **soonest event first**;
   orders with no event date sort last.
4. **Settings** (`Settings.tsx`): section **tabs** instead of six door-cards.
   The `settings/:section` route already existed, so every deep link still
   lands where it did; `/settings` with no section now opens the first tab.
5. **Dashboard** (`Dashboard.tsx`): profit becomes the headline figure with
   margin and ROI as its own sub-line; the other five stay as chips. Six
   identically-sized boxes claimed all six mattered equally.
6. **Finance** (`finance/Overview.tsx`): balance and what you are owed split
   into their own band **above** income/expenses/net. A stock and a flow are
   not the same kind of number, and the period filter only moves one of them.
7. **Calendar** (`Calendar.tsx`): the day detail is a **column beside the
   month grid** instead of a modal, so stepping through days no longer hides
   the grid. Empty days became selectable ("nothing on this day" is an
   answer). Week/Day/Agenda still use the modal - they have no grid to sit
   beside.
8. **Price Checker** (`PriceChecker.tsx`): the event overview is a **light
   table** instead of a card each - link state, listing count, scan age only.
   The per-marketplace breakdown moved behind the click that already opened
   the event (`onOpen={setEventId}` was there before this).
9. **Fixed (backend, separate from the redesign)**: `ACCOUNT_SELECT` had **no
   date filter at all**, so an entry dated in the future was already
   subtracted from a figure labelled *current balance*. It now cuts off at
   `date('now','localtime')`, matching `finance_forecast::eur_balance_as_of`,
   whose own test states the rule this query was breaking: *a future-dated
   entry must not already be folded into 'current' balance*. This is what made
   Finance Overview print two different "current balance" figures a few
   hundred pixels apart. **4 new tests** pin it, all using far-future/far-past
   dates so they never go stale. The cut-off is SQL's own `date()` rather than
   a `today` parameter - see the reasoning on `ACCOUNT_SELECT` before changing
   it.

**Not built in this session:** nothing was compiled or tested here (no Node,
no Rust toolchain on that machine) - the first CI run is the first real check.

## 2.12.1 - Three loose ends from 2.12.0

Cloud Sync is confirmed working end to end on both machines. These are the
follow-ups that were owed, not new features. **No schema change, no migration
(next new one is still 027), no business logic change.**

1. **Fixed**: the `tauri_plugin_deep_link::DeepLinkExt` import now carries the
   same `cfg(all(desktop, debug_assertions))` as the only call that uses it,
   so release builds stop warning. **Deleting it** - the obvious "fix" for an
   unused import - **would have broken `cargo tauri dev`**, where that
   dev-only deep-link self-registration is the whole reason the trait is in
   scope.
2. **Fixed**: the Drive 403 message. That one status covers two very different
   causes and the old text only named the less likely one ("sign in again").
   It now leads with the common case - the Google Drive API not being switched
   on for the OAuth client's Cloud project, which is a one-time, project-wide
   step that applies to every user of the build, not per person.
3. **Added**: Google's own 403 body carries the exact Cloud Console URL for
   that step, so the sync card now extracts it and offers an **Open Google
   settings** button instead of leaving it to be copied out of a toast that
   has already disappeared.
4. **Fixed**: `RELEASE.md`'s macOS instructions said right-click → Open, which
   Apple removed in macOS Sequoia (15). The working path is System Settings →
   Privacy & Security → Open Anyway.

## 2.12.0 - Cloud Sync: one database, two computers

marko works on a Windows PC and a Mac and wants what he writes on one to show
up on the other. **No schema change, no migration (next new one is still 027),
no business logic change, no new dependency, no server.**

1. **Added**: `commands/cloud_sync.rs` - keeps one database snapshot in
   marko's **own Google Drive** and syncs it whole-file, one direction at a
   time. "Sync up" uploads this machine; "Sync down" downloads and restores
   the other.
2. **Deliberately not built**: row-level merging. Every primary key here is a
   per-machine `INTEGER AUTOINCREMENT`, so two machines both mint id 5, and
   invariants like `insert_order_with_tickets`' exact-cent cost split and
   `refund_sale_impl`'s one-way transition have no correct automatic merge. A
   real merge engine is a months-long rebuild, not this release.
3. **Added**: a lost-update guard. Every upload records the Drive file version
   it wrote; the next upload re-checks it. If the other machine pushed since,
   the upload is **refused** and the UI offers an explicit "Overwrite anyway"
   decision instead of silently winning.
4. **Reused, not rebuilt**: downloads go through
   `backup::restore_database_impl`, inheriting its validation, automatic
   safety backup and automatic rollback - the safety backup path is returned
   and shown. Uploads use the SQLite Online Backup API via the new
   `backup::snapshot_db_to`, extracted from `create_safety_backup` so both
   callers share one implementation.
5. **Changed**: `OAUTH_SCOPE` gains `drive.file` - the narrowest scope that
   works, granting access only to files this app created and never to the rest
   of the Drive. **Everyone has to sign in with Google once more**: an
   existing refresh token was issued against the old scope set.
6. **Added**: a "Sync between your computers" card in Settings → Data, above
   Backup/Restore, since it is the same concern made automatic.
7. **Not changed**: nothing runs on a timer or at startup. Sync is off until
   switched on and every sync is a click. With sync off, or offline, the app
   is exactly as local-first as before.

5 new Rust unit tests.

**Build fix applied before this ever shipped**: the first attempt failed to
compile on both platforms with `no method named 'query' found for
reqwest::blocking::RequestBuilder`. `RequestBuilder::query` needs a
`serde_urlencoded` path that this crate's reqwest 0.13 feature set does not
enable, and no other module in this app had ever used it. Every URL is now
built the way `google_sheets.rs` has always built them - `format!` plus
`utf8_percent_encode` for anything dynamic. The download also streams
straight to disk via `std::io::copy` instead of buffering the whole database
in memory.

**Not verified by a build, and no Drive call has ever run.** No Node.js or
Rust toolchain on the machine this was implemented on. The Drive request
shapes are written from the Drive v3 API docs - they are the first thing to
check if sync misbehaves.

## 2.11.1 - Fix the macOS build broken by 2.11.0

2.11.0 published a working **Windows** release, but its macOS leg failed.

1. **Fixed**: the six `APPLE_*` signing variables are removed from the release
   build step. 2.11.0 wired them in "ready for the day credentials exist" -
   but **a missing GitHub secret still defines the environment variable as an
   empty string**, and the Tauri bundler treats `APPLE_CERTIFICATE` being *set*
   as a request to sign. It ran `security import` with an empty certificate and
   aborted with "failed to import keychain certificate". They are now a
   commented block carrying an explicit warning that pasting them back without
   the matching secrets breaks the build again.
2. **Fixed**: `prepare-release` now checks whether a release exists before
   deleting it, instead of letting `gh release delete` exit non-zero on the
   normal first-attempt case and paint a red "failed" annotation on an
   otherwise healthy run.

**What the failed run actually proved**: the Rust `universal-apple-darwin`
build succeeded in 3m36s and the `.app` was already bundling. The universal
target works on CI - signing was the only thing that ever failed.

Nothing else changed: no application code, no business logic, no schema, no
migration (next new one is still 027), no dependency.

## 2.11.0 - macOS builds, verified updater manifest, in-app update centre

marko asked for a professional installer + auto-updater + release pipeline.
Most of it already existed and was kept; the real gap was macOS. **No business
logic change, no schema change, no migration (next new one is still 027), no
new dependency, no new cloud service, no update server.**

1. **Added**: macOS builds. `.github/workflows/build-windows.yml` is renamed
   to `release.yml` and now builds **both** platforms - the Windows NSIS
   `.exe` and a **universal** macOS `.dmg` (Apple Silicon + Intel from one
   download). `release.ps1`'s own "did the workflow survive the mirror" guard
   was renamed in the same change - those two must always agree.
2. **Fixed**: the "delete stale GitHub release" step moved into its own job
   that runs **before** the build matrix. With a matrix, the second runner
   would otherwise delete the release the first one had just published.
3. **Changed**: the release matrix runs one platform at a time
   (`max-parallel: 1`) - both legs publish to the same release and both
   rewrite `latest.json`, so serialising them removes the race.
4. **Added**: a `verify-release` job asserting the finished release really has
   an `.exe`, a `.dmg` and a `latest.json`, that the manifest covers both
   platform families, and that every entry has a non-empty signature and url.
   A silently-wrong updater manifest is the failure nobody notices until users
   stop getting updates.
5. **Added**: a very small update status on the Dashboard. It reads the
   launch-time check result and links to Settings → Software; it does not run
   its own check and does not install anything, so there is still exactly one
   updater UI and one progress state.
6. **Added**: Settings → Software now shows Current version / Latest version /
   Last checked alongside the existing check-and-install flow and release
   notes.
7. **Changed**: the launch-time update check now also repeats every 6 hours
   for sessions that stay open - deliberately slow, no polling. Manual "Check
   for updates" is unaffected.
8. **Added**: `RELEASE.md` - the release process, every repository secret and
   what it does, the artifact list, and both the developer and user
   checklists. No secret values in it.

**Signing, stated honestly:** updater signing is configured and is what makes
in-app updates secure. **Windows code signing and macOS code signing/
notarization are NOT configured** - both need paid external credentials. The
workflow passes all six Apple variables through so adding the secrets is the
only remaining step, and nothing here fakes a certificate. Until then the
`.exe` shows a SmartScreen warning and the `.dmg` needs right-click → Open on
first launch.

**Not verified by a build.** No Node.js and no Rust toolchain on the machine
this was implemented on, and no macOS build has ever run - the first tagged
run of this workflow is itself the test.

## 2.10.0 - Price Checker event overview replaces the event dropdown

marko's own request: see every relevant event at once instead of picking one
from a dropdown. **UX only** - the scanner, parser, readers, market analysis
and history are untouched. **No schema change, no migration (next new one is
still 027), no new dependency.**

1. **Changed**: opening Price Checker now lists every **upcoming** event as a
   dense card - name, date, venue/city, a per-marketplace row (Linked / No
   link, plus that marketplace's last check and listing count), and an
   event-level line with the newest listing count or "Not scanned yet".
2. **Added**: multi-select with checkboxes, "Select all" (scoped to what is
   currently visible, so a filtered list can't silently select what you can't
   see), "Clear selection", and a "Selected: N events" bar.
3. **Added**: local search over event name, venue and city, and four quick
   filters - All / Needs link / Not scanned / Scanned.
4. **Added**: one read-only backend command, `list_price_checker_overview`,
   answering the whole list in four flat queries instead of calling
   `get_price_checker_summary` once per event. It writes nothing, triggers no
   scan and makes no marketplace request.
5. **Deliberately not added**: a "Scan failed" state. A failed scan is never
   persisted anywhere in this app - `price_checks` has no status column, and a
   row only reaches it through the explicit review-then-save step, so by
   construction every stored check succeeded. The scanner's own error/blocked
   states live in memory for the life of one scanner window. A "Scan failed"
   badge would be inventing a state the database does not have.
6. **Check selected**: opens the first selected event's own flow and keeps the
   selection, so the rest are one click each. The scanner opens a real visible
   browser window marko drives himself, so no parallel sessions and no queue
   automation were invented.
7. **Added**: an "All events" back link, since the dropdown that used to be
   the way back is gone.
8. **Not changed**: parser, readers, DOM scanning, market calculations, tier
   grouping, section/row metadata, price history, Your Tickets. Tier/Level
   stays a market grouping; section/row stay metadata. No background
   monitoring, no polling, no scheduled scanning, no repricing.

8 new Rust unit tests (42 in `price_checker.rs` total).

**Not verified by a build.** No Node.js and no Rust toolchain on the machine
this was implemented on, so `cargo test --lib`, `cargo check --lib`,
`npx tsc -b` and `npm run build` could not be run.

## 2.9.0 - Price Checker accuracy fix, scan report, filters, CSV export

marko reported the scanner reading prices wrong. Diagnosis first, then the
smallest fix per cause. **No schema change, no migration (the next new one is
still 027), no new dependency, no new marketplace.**

1. **Fixed (root cause of the wrong prices)**: `price_checker_scan.js`'s
   generic text-node walker found money in one text node and then called
   `candidateFrom(parentElement)`, which re-parsed the parent's *entire* text
   and kept the **first** money match in it. On "Was $200 / Now $120" markup
   that stored the crossed-out old price; on a wrapper whose `aria-label`
   reads "Total incl. fees" it stored the fee-inclusive total. The matched
   money, and the text it was matched in, are now passed into
   `candidateFrom`.
2. **Added**: three price rejection rules that did not exist at all before -
   struck-through prices (`<s>/<del>/<strike>`, line-through class or computed
   style), money labelled total/subtotal/fee/service charge/delivery/tax/was/
   original/RRP, and money inside header/footer/nav/aside/cart/checkout/modal
   page chrome.
3. **Fixed (metadata leak)**: `findListingContainer` fell back to "three
   ancestors up" and `nearbyListingContext` then regexed that whole subtree,
   so section/row/quantity/tier could be read off a *neighbouring* listing or
   off page chrome. The container now reports whether it is confident;
   metadata is read from a tight scope when it is not, and the listing is
   flagged `incomplete` instead of being presented as a clean read.
4. **Fixed (dedup, both directions)**: a real marketplace listing id is now
   the whole cross-scan identity - it used to be one of seven fields
   *including the price*, so the same listing re-read after a scroll counted
   twice as soon as the price read differently. And `tier` joined the fallback
   key, whose absence merged two listings that differed only by tier and
   deleted a real one. Without an id the price stays in the key on purpose.
5. **Fixed (currency blending)**: `compute_scan_stats` averaged across every
   listing regardless of currency and labelled the blended result with
   whichever currency came first. It now computes inside the largest
   single-currency group only and reports how many listings were excluded -
   which also makes it agree with `price_checker_analysis`, which has always
   partitioned by currency correctly.
6. **Added**: a Found / Accepted / Skipped / Duplicates scan summary with
   per-reason skip counts.
7. **Added**: six plain result filters (marketplace, tier, currency, min/max
   price, complete vs incomplete) - not a filter builder.
8. **Added**: Export CSV, reusing the same `plugin-dialog` `save()` + Rust
   writer every other export in this app already uses.
9. **Not changed**: the manual Visible Scanner workflow, Tier/Level as a
   grouping (never a pricing input - no section/row/seat pricing, no price
   suggestions), Your Tickets comparison, history, refund/resell, `batch_id`,
   money/integer cents, Orders/Tickets/Sales/Listings/Finance/Fulfillment/
   Attention/Calendar/Google Sheets. No background monitor, no scheduled scan,
   no polling, no CAPTCHA bypass.

15 new Rust unit tests (43 in `price_checker_scanner.rs` total).

**Not verified by a build, and the DOM fixes are not verified against a live
page.** No Node.js and no Rust toolchain on the machine this was implemented
on, so `cargo test --lib`, `cargo check --lib`, `npx tsc -b` and `npm run
build` could not be run, and the injected browser script cannot be executed
here at all. The live marketplaces have never been reachable from this
sandbox either (the same limitation the 2.1.9 script has always carried), so
every DOM-level fix is reasoned from the code and must be confirmed against a
real listings page.

## 2.8.0 - Calendar redesign + advanced calendar workflow

marko asked to make the TIQR Operations Calendar one of the app's main work
screens. **No schema change, no migration (the next new one is still 027), no
new index, no new dependency.** `commands/calendar.rs` stays what it has
always been: a read-only aggregator with no write path.

1. **Added**: four views instead of two - **Month, Week, Day, Agenda**. All
   four share one data path (`get_calendar` over a date range), one filter
   state and one search box; only the range and the layout differ, so
   switching period still fetches exactly one window.
2. **Added**: two genuinely new date sources, **`finance`**
   (`finance_entries.entry_date`) and **`recurring`**
   (`recurring_expenses.next_date`). Both were re-derived from the live
   schema and command code rather than trusting the 2.5.0 note, exactly as
   `PROTECTED_AREAS.md` instructs.
3. **Still not added**: payouts, payments, fulfillment. Re-checked this
   release - there is no payout entity or date anywhere, migration 007's
   `payments` table still has zero live SQL in any command module, and
   `tickets.delivery_status` still has no date column. What the 2.5.0 pass
   never checked was Finance, which is where this app's real dated money
   lives - so it is added under its own honest name rather than rebranded as
   a "payout". There is a standing test that fails if an invented category
   ever reaches the calendar.
4. **Added**: a real **Overdue** count. `recurring_expenses.next_date` is the
   only genuine due date in the app; the rule is the same one Finance's own
   Accounts tab applies (active template, `next_date` before today), and
   paused templates are excluded because their `next_date` is frozen and not
   actionable.
5. **Added**: a Today / Tomorrow / Next 7 days / Overdue summary strip, a Day
   Detail with a per-kind summary panel, per-day workload bars (three muted
   segments, deliberately not a color scale), event countdowns derived only
   from the event's own date, calendar-local search (dims non-matches in the
   grid, filters the lists, can jump to the first match), and filter chips
   that only appear for kinds actually present in the loaded range.
6. **Deliberately not built**: a time-of-day axis. Every date this app stores
   is date-only, so Week is seven day columns rather than a faked 24-hour
   timetable, and Day groups by kind rather than by hour.
7. **Deliberately not built**: quick-add of tasks/reminders/notes. There is
   no task, note or reminder table anywhere in this schema, and marko was
   explicit that this release must not stand up a new task database.
8. **Changed**: the grid and the lists scroll inside themselves rather than
   growing the page, so the header, filters and summary strip stay put and
   there is only ever one scrollbar.
9. **Not changed**: refund/resell, `batch_id`, money/integer cents,
   Orders/Tickets/Sales/Listings/Finance/Fulfillment/Attention business
   logic, Price Checker, Google Sheets.

9 new Rust unit tests (23 in `calendar.rs` total).

**Not verified by a build.** Implemented on a machine with no Node.js and no
Rust toolchain, so `cargo test --lib`, `cargo check --lib`, `npx tsc -b` and
`npm run build` could NOT be run - marko chose to proceed on static review.
Run all four before publishing the tag, and regenerate
`Cargo.lock`/`package-lock.json`.

## 2.7.0 - AI Import Assistant (screenshot -> pre-filled form, never straight to the database)

marko's own request: drop, paste (Ctrl+V) or upload a screenshot into the New
Event, New Order or New Sale form and have Claude read the structured data off
it. **No schema change, no migration (the next new one is still 027), no new
dependency.**

The rule the whole feature is built around, in his words: "AI NIKDY nesmie
priamo vytvoriť alebo meniť databázový záznam." The flow is image -> Claude ->
structured result -> review -> user confirms -> the EXISTING create form -> the
EXISTING create command -> DB.

1. **Added**: `src-tauri/src/commands/ai_import.rs` - one command,
   `analyze_import_image`. It takes no `AppState`, opens no `Connection` and
   has no write path of any kind, so it cannot create or change a record even
   by mistake. Reuses `ai_categorize.rs`'s build-time embedded
   `ANTHROPIC_API_KEY` (the key never reaches the frontend) and its
   retry-once-on-a-transient-status policy.
2. **Added**: a strict per-kind JSON schema (`output_config.format`). Every
   extracted value is a plain string or `null`, each carries a
   `high`/`medium`/`low` confidence, and a field that isn't on the image comes
   back `null` rather than being filled in. `sanitize_result` then drops any
   field name the form has no slot for, keeps only the first of a duplicate,
   and collapses blank values to `null`.
3. **Added**: multiple ticket groups are returned separately and never merged.
   Because `OrderInput` carries one section/row/tier/price per order, the panel
   fills one group at a time and says so, instead of inventing a multi-group
   order shape the backend has never had.
4. **Added**: `src/components/AiImportPanel.tsx` - one compact shared panel,
   embedded in all three forms. Not a new page, route, tab or sidebar.
5. **Added**: `src/lib/aiImport.ts` - image validation, downscale to 1568px
   (only when needed, so a normal screenshot is sent untouched), an FNV-1a
   fingerprint, clipboard/drop extraction, an ISO-date guard and a lookup
   matcher.
6. **Changed**: `dragDropEnabled: false` on the main window
   (`src-tauri/tauri.conf.json`) so HTML drop events reach the webview -
   Tauri's default of `true` swallows them. Nothing in the app used Tauri's own
   drag-drop event.
7. **Cost control**: one analysis per explicit user action; a per-session image
   fingerprint cache so the same screenshot is never analyzed twice; editing an
   extracted field never re-calls; retry only on a click; no background or
   timed request anywhere.
8. **Not changed**: refund/resell, `batch_id`, money/integer cents,
   Orders/Tickets/Sales/Listings/Finance/Fulfillment logic, Price Checker,
   every existing create command and every existing validation rule. AI
   extraction cannot bypass validation - the same commands still run at save
   time on values the user has seen and confirmed.

9. **Fixed (same version, after the first release run failed)**:
   `release.ps1`'s `$CommitMsg` had literal double quotes in it, which Windows
   PowerShell 5.1 cannot pass to `git.exe` intact - `git commit` exited
   non-zero and the script stopped with "git commit failed". The quotes are
   gone and there is now a guard right before the commit that catches this
   with a real explanation instead of a git error. See `PROTECTED_AREAS.md`'s
   "2.7.0" entry.

**Needs the `ANTHROPIC_API_KEY` GitHub Actions secret** (already wired into
both build paths). Without it the panel reports "AI import isn't available in
this build" and nothing else changes.

**Not verified by a build.** Implemented on a machine with no Node.js and no
Rust toolchain, so `cargo test --lib`, `cargo check --lib`, `npx tsc -b` and
`npm run build` could NOT be run - marko chose to proceed on static review.
The 28 new Rust unit tests are written but have never been executed. Run all
four before publishing the tag, and regenerate `Cargo.lock`/`package-lock.json`.

## 2.6.0 - Complete visual redesign (UI/UX only)

marko's own task: make TIQR Manager look and feel like a modern, premium
desktop app, explicitly with **no new features, no new workflow, no new
database systems**. Nothing in `src-tauri/` changed - the backend is
byte-for-byte identical to 2.5.2. No migration, no new dependency.

1. **Changed**: one shared design layer now defines the whole app's look -
   `tailwind.config.js` (retuned `slate` ramp so light and dark each get a
   real background -> surface -> line hierarchy instead of one being an
   inversion of the other; a 3-step `shadow-card`/`raised`/`overlay` scale;
   a tighter radius rhythm; a 120-180ms motion budget). The `brand` blue
   ramp is deliberately untouched.
2. **Changed**: `src/index.css` - base typography (tabular figures on every
   number in the app), restyled `.input`/`.label`/`.th`/`.td`/`.card`, new
   `.section-title`, `.field-invalid`, `.skeleton`, `.card-interactive`, and
   the new `.table-shell`/`.table-flush`/`.row-selected` table system.
   `.th-c-narrow`/`.td-c-narrow`'s measured 2.0.37 metrics are unchanged.
3. **Changed**: `src/components/ui.tsx` - every shared control restyled.
   `Button` gained an optional `size`; `Field` now turns its own control red
   on error; `Badge`/`InlineStatusSelect` share one status pattern with a
   leading dot; `StatCard` is shorter and quieter; `Modal`/`ConfirmDialog`/
   `EmptyState`/`ModalFooter`/`TabSwitcher` redesigned. The status
   vocabulary (which tones exist, which value maps to which) is unchanged.
4. **Added**: `Skeleton` and `TableSkeleton` in `ui.tsx`, plus
   `SEGMENTED_TRACK`/`segmentedItemClass` - the app's one tab/segmented
   pattern, now genuinely shared instead of hand-rolled per page.
5. **Changed**: `src/components/Layout.tsx` - sidebar redesigned (accent-bar
   active state, hairline section separation, restyled theme toggle and
   profile area). Same items, same order, same routes, same `w-48` width,
   same behaviour.
6. **Changed**: all 22 tables in the app moved onto the shared table shell -
   sticky opaque headers, internal scrolling instead of page scrolling, one
   hover treatment, one selected-row treatment. Column widths, `colgroup`
   percentages and the narrow-window breakpoint are untouched.
7. **Changed**: six list pages (Orders, Sales, Tickets, Events, Pulls,
   Ticket Center) now show a table skeleton while loading instead of a
   centred spinner.
8. **Changed**: `prefers-reduced-motion` now disables every transition and
   animation across the app.
9. **Not changed**: any business logic. The whole diff contains no `api.`
   call, no state/handler/effect, and no routing change - see
   `PROTECTED_AREAS.md`'s "2.6.0" entry.

**Not verified by a build.** This round was implemented on a machine with no
Node.js and no Rust toolchain, so `npx tsc -b`, `npm run build` and
`cargo check --lib` could NOT be run - marko chose to proceed on static
review rather than install a toolchain. Run all three before publishing the
tag, and regenerate `Cargo.lock`/`package-lock.json` (their version entries
were bumped by hand for the same reason).

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

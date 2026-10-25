# TIQR Manager - Known Bugs

Currently open, real, reproducible bugs and gaps - not a wishlist, not a
changelog. If it's fixed, remove it from here (the fix's changelog entry
is the permanent record). If it's a design trade-off that was made on
purpose and documented, it belongs in `PROTECTED_AREAS.md`, not here.

This file starts clean as of 2.1.9 rather than being backfilled from the
project's full history - the detailed per-release history already exists
in the `REDESIGN-*-REPORT.md` / `*-REPORT.md` files in `history/`, and
mining ~105 of them for anything still-open would itself be the kind of
full-repo audit this protocol exists to avoid. See CURRENT_STATE.md's
"Known task-list debt" note for a short list of old, un-triaged task
markers that may or may not still be real - add an entry here if and when
one of them is confirmed to still reproduce.

2026-09-01: the one entry below was carried over from a separate session's
own parallel copy of this file (bootstrapped at 2.0.80, before this file's
"starts clean as of 2.1.9" convention existed elsewhere) - it is a real,
still-reproducible gap in code that hasn't changed since, not a backfill
from old reports.

## Open

- **GitHub Pages is not enabled for the repository** (2.113.1 finding: no
  `pages-build-deployment` workflow, and Discord could not load the logo from
  github.io). `src/lib/firebase.ts` sends the password-reset continue URL to
  `https://biznismarko9-source.github.io/tiqr-manager/reset-redirect.html`,
  and the Google consent screen links `privacy.html` there - both answer 404
  until marko enables Pages (Settings -> Pages -> Deploy from a branch ->
  `main` / `/docs`). The Discord logo no longer depends on it.

- **A sheet sync that creates events asks the AI for their category while
  holding the database** (`orders_sheet_sync::resolve_or_create_event` ->
  `detect_category_for_event_name`), so the window can wait up to 20 s per
  new event during such a sync when an AI key is built in. The "Detect
  categories" button no longer does this (2.116.0); the sheet sync path is
  inside its own transaction and was left as it was.
- **A database that cannot be opened at start still closes the app before
  its window appears.** Since 2.116.0 the reason is in `logs/tiqr.log` next
  to the data, but no screen says it. Starting on a stand-in database
  instead would risk work being typed into it and lost, so it was not done.
- **Until both computers run 2.116.1**, the one still on 2.116.0 keeps
  looking for and showing event pictures, and sends the ones chosen there by
  hand. The updated one sets those aside (`live_sync::RETIRED_TABLES`), so
  nothing is stored and no conflict is counted. It stops with the update,
  whose migration 048 empties that computer's pictures too.

- **Promotions cleanup and the new webhooks (2.114.0): checked with fakes
  only.** The AI sorting was not run against the real API here (no key in
  this environment); its request shape follows the API reference and is
  unit-tested, and an answer that cannot be read leaves a message "not
  sure", never junk. Discord's new compact messages were checked as JSON in
  tests, not in a real Discord client.

- **Pulls (Received), wide table: a synced pull's order code is cut to
  "ORD-0..."** beside its *Synced* badge (seen while checking 2.114.1; the
  Order column's 8.162% is unchanged since 2.0.38 and the narrow table hides
  the column). The link's tooltip still names the order. A fix means
  funding a wider Order column from another one.

- **A star set in TIQR stays on that computer (2.113.1).** The Gmail access
  is read-only, so the star does not reach Gmail and the other computer does
  not see it (read state and moves behave the same). Reading a mail in TIQR
  does not mark it read in Gmail either; the other way round works.

- **Key details and link downloads (2.113.1): checked with fakes only** -
  Rust tests with a local web server and Chromium with a mocked backend, not
  real viagogo/StubHub mail or real receipt links. A label TIQR does not
  know gives no key detail (nothing is guessed); a receipt behind a sign-in
  opens in the browser rather than downloading. Facts are computed when a
  message is opened, so a mail arriving without its body (unknown sender,
  older than 14 days) shows in Discord with the preview only.

- **Mail reader (2.113.0): not run against a real Gmail mailbox, a real
  Discord server or WebView2 on real Windows** - only fakes in the Rust
  tests and screens in Chromium with a mocked backend. The PDF preview
  relies on the webview's own PDF viewer; if it shows nothing, Open still
  works.
- **Mail: the paperclip shows only after a message was opened once** -
  `has_attachments` is learned from the full message, and the list is read
  as metadata only. Reading every message in full to know would cost far
  more Gmail quota and disk.
- **Mail: pictures from the web are fetched at the first open**, so a sender
  can see that first download (tracking pixels are dropped, later opens
  read TIQR's copy). Turning "Copy pictures from the web" off avoids it.

- **Live sync (2.112.0): newest edit wins per RECORD, not per field.** Two
  computers editing different fields of the same order within the same few
  seconds: the later save wins the whole order on both. By decision (marko
  agreed "novšia úprava vyhrá"); a field-level merge would need per-column
  clocks.
- **Live sync: the note open in the editor is not refreshed** when the other
  computer edits the same note - the list is, the open note shows the new
  text after reopening. Reloading under the cursor would eat typing.
- **Live sync: not verified against the real Firebase project** - only the
  emulator with the same rules (scripts/live-sync-emulator-check.ts). The
  first run after marko publishes the rules is the real check.

- **`cloud_sync_local_dirty` can read `false` while unsynced work exists.**
  Proven once, with cost. In `pre-restore-20261001-164651520.sqlite3` the
  flag is `false` and `cloud_sync_last_sync_at` is `2026-09-30T13:23`, while
  orders ESPAA-002/003/004 and sales OASIS-001/002 were created 2026-09-30 at
  14:36 - after that sync, never sent. `decide_auto` turned that into
  `(false, true) -> Pull` and the whole database was replaced by an older
  Drive copy; marko lost 3 orders, 6 tickets and 2 sales (640 EUR). Four
  restore points named "Before Sync Down" say it had happened before.

  Not yet explained: which of the three ways the flag can go stale actually
  fired. Candidates, all in reading distance of each other - `database.rs`
  clears the atomic on the post-login database switch; `flush_local_dirty`
  is best-effort on exit and its failure is ignored; the persisted copy is
  only written on a 5-minute tick, so a write followed quickly by a close
  can fall between them.

  2.68.0 made the consequence non-destructive rather than fixing this: Pull
  and Merge now ask (see its changelog entry). The flag being wrong is still
  wrong - it also means a genuine Push can be skipped, which loses nothing
  but leaves the machines apart.

  **2.73.0 took the cheaper hardening**: `decide_auto`'s `(false, true)` arm
  now returns `Merge` instead of `Pull`. Merge is never more destructive than
  Pull - it applies the same tombstones but keeps rows the other side has
  never seen - and with the flag RIGHT the two produce the same database, so
  nothing was given up. `decide_auto` no longer returns `Pull` at all; the
  variant stays for the hand-pressed "Sync down" button.

  Still open: the flag itself. The robust fix is to stop trusting it and
  compare content instead, but `content_hash` runs over a full snapshot and
  `cloud_sync_auto` is deliberately cheap and read-only, so that is a real
  design change, not a small edit. What 2.68.0 and 2.73.0 together achieved
  is that the flag being wrong is now survivable rather than fatal - it can
  still skip a Push that should have happened, which loses nothing but leaves
  the two machines apart.

- **Sales sync can't independently guard against a duplicate sale from a
  stale, un-pushed refund row.** As of 2.0.80, refunding a sale in the app
  is only reflected in a connected Google Sheet once "Push sales" or "Fix
  sync" is run afterward (they now clear the stale row). If marko runs
  "Sales sync" (the pull direction) again *before* doing that, the still
  non-blank `Payout Per Ticket` cell could in principle be mistaken for a
  brand-new sale and create a duplicate. Not fixed on the pull side -
  flagged in `REDESIGN-2.0.80-REPORT.md` as an operational ordering note
  ("push before you pull again after a refund") rather than code changed,
  since a robust pull-side guard would need real design input, not a
  guess. Revisit if marko reports a duplicate sale after a refund.

- **A hard kill (crash, force-quit, power loss) in the few minutes right
  after an edit can lose that edit's "unsent work" flag** (2.14.0). Automatic
  sync answers "did this machine change anything?" from an in-memory atomic
  that is written to disk on every automatic check (every 5 minutes), on a
  clean exit, and when accounts switch - none of which happen if the process
  is killed outright. Consequence: on the next launch that machine believes it
  is clean, and if the OTHER machine pushed in the meantime, the startup
  auto-pull replaces it. **The data is not gone** - every pull takes a safety
  backup first and the toast names its path - but marko would have to restore
  it by hand. Closing this properly means persisting the flag from the write
  itself, which the SQLite update hook must not do (a hook may not write to
  the database it watches), so it needs a real design pass rather than a
  guess. Revisit if marko reports losing an edit after a crash.

## Documented limitations (not bugs, but worth remembering)

- **Mail is per computer, shared while both run (2.110.0; 2.115.0).** Each
  computer keeps its own mail file. Alone, it reads Gmail itself (since
  2.114.3 at full pace even minimised). While both run with "Two computers,
  one reader" on (the default) and live sync on, one reads Gmail and
  forwards what it stored through `users/{uid}/mail`; each batch is deleted
  once stored, and mail never goes through Drive. Gmail's own total is shown
  beside this computer's count; it includes sent mail, which TIQR does not
  read.

- **Sharing the reading needs the 2.115.0 firestore.rules published** in the
  Firebase Console. Until then each computer reads Gmail itself and Mail ->
  Settings says why. It also stops while live sync is not running, while
  mail is paused on a computer, or when Firebase's daily free limit is used
  up (both computers running all day use about 3,000 writes and 5,000 reads
  of the 20,000/50,000). A minimised window whose timers stop for minutes
  can make the computers swap roles once; nothing is lost either way.

- **The first follow sends everything a computer lacks** (2.115.0) - for a
  mailbox of 100,000 that is about half an hour of batches through
  Firestore, once. A computer whose own reading went through every folder
  gets only the last week before its last read instead.

- **Connecting a DIFFERENT Gmail account keeps the old account's mail in the
  folders** (2.110.0). Folder queries (`commands/mail.rs::folder_expr`) do
  not filter by `account_email`, and the mail file is never emptied - the
  lock forbids that on purpose. Reconnecting the SAME account, the normal
  case (the 7-day Testing token), is unaffected. If marko ever switches
  accounts, the fix is an `account_email = ?` clause in `list_folder_impl`
  and `folder_counts_impl`, not deleting anything.

- Outbound notifications (desktop/Pushover) only fire while the app process
  is actually running - no tray icon, no launch-on-startup, no background
  service. If marko wants notifications even when the app is fully closed,
  that's a separate, larger feature (tray + auto-launch + keep-alive), not
  a bug fix.

## Format for new entries

```
### <short title> (found in vX.Y.Z)
What's broken, how to reproduce it, and the relevant file(s) - one short
paragraph. Link the task/report if one exists.
```

# TIQR Manager - Known Bugs

Currently open, real, reproducible bugs and gaps - not a wishlist, not a
changelog. If it's fixed, remove it from here (the fix's changelog entry
is the permanent record). If it's a design trade-off that was made on
purpose and documented, it belongs in `PROTECTED_AREAS.md`, not here.

This file starts clean as of 2.1.9 rather than being backfilled from the
project's full history - the detailed per-release history already exists
in the `REDESIGN-*-REPORT.md` / `*-REPORT.md` files at the repo root, and
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

  The robust fix is to stop trusting a flag and compare content instead, but
  `content_hash` runs over a full snapshot and `cloud_sync_auto` is
  deliberately cheap and read-only, so it is a real design change, not a
  small edit. A cheaper hardening that is strictly safer and was considered
  but NOT taken in 2.68.0: make `decide_auto`'s `(false, true)` arm return
  `Merge` instead of `Pull`. Merge is never more destructive than Pull -
  it applies the same tombstones but keeps rows the other side has never
  seen - so it is the better default even when the flag is right. It needs
  the unit tests at the bottom of `cloud_sync.rs` updated with it.

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

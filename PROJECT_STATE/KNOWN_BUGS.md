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

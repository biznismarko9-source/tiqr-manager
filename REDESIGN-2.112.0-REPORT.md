# TIQR Manager 2.112.0 — Live sync between computers

marko: *"ako by sme vedeli urobit nejaky sync ktory v realnom case po
zatvoreni apky a otvoreni v inom zariadeni by tam boli insta informacie"*,
*"ze vsetko by ako keby fungovalo spolu"*.

Every change made on one computer (an order, a sale, an edit, a deletion, a
note, a mail rule) appears on the other computer within a few seconds while
both are open. A computer that was closed picks up everything it missed the
moment it opens. If both computers changed the same record, **the newer change
wins**, the same way on both. Google Drive stays as the backup copy.

---

## 1. What was there

| | Before 2.112.0 (Drive file) |
|---|---|
| What travels | the whole database as one file |
| When | only when the app opens or closes |
| A record both computers have, edited on one | **never travelled**: the merge only adds rows and carries deletions |
| Both computers open | nothing travels until one of them is closed |
| Arriving changes | a dialog, then a page reload |

## 2. How it works now

### Capture: in the database itself

`commands/live_sync.rs` and migration **046**.

- A trigger on each of the 25 tables in `cloud_merge::MERGE_TABLES` writes
  the record into `sync_outbox` when it is inserted, changed or deleted. There
  is one entry per record, so ten edits send once.
- **Clock.** Each entry carries a hybrid logical clock (`sync_control.hlc`, in
  milliseconds). It is never behind the wall clock, and never behind anything
  already seen from the other computer.
- **Only real changes.** An update counts only if a column actually changed.
  The triggers are rebuilt at every start, so a column a later migration adds
  is compared too.
- **No echo.** Changes being applied from the other computer are not
  captured. Deletions are the one exception: rows a deletion removes here
  through `ON DELETE CASCADE` are sent, because a deletion is final.

### Sending

`lib/liveSyncSession.ts`: the Rust side hands out pages of the outbox, and
the frontend writes them to Firestore under the signed-in account's own space.

- `users/{uid}/rows/{table~uid}` holds one record per document.
- A reference to another record travels as that record's `uid`, never as a
  local id.
- A record larger than about 700 KB (a pasted picture in a note) goes into
  `users/{uid}/parts` first, then the record itself.
- **Firestore's rules enforce newer-edit-wins on the server.** A document is
  only replaced by a newer version (newer clock, or the same clock and a
  higher device id). A deletion replaces anything and is replaced only by
  another deletion. Two computers sending at the same moment therefore cannot
  undo each other, and nothing brings a deleted record back.
- When Firestore refuses a batch because it holds a newer version of one
  record, that batch is sent again one record at a time, each read first.

### Receiving

`live_sync::apply` runs one transaction per page, with a savepoint per record.

- **Newer edit wins.** For the same clock, the higher device id wins.
- **Deletion wins** over any edit (migration 028's rule, unchanged).
- **Same content, same version.** Two computers that already shared a record
  through Drive do not send it to each other again.
- **Waiting for a parent.** A ticket that arrives before its order waits in
  `sync_pending` and lands when the order arrives.
- **A reference to something deleted here** follows its foreign key:
  - `SET NULL`: the link is cleared.
  - `CASCADE`: the record goes too, and the deletion is sent on.
  - `RESTRICT`: the record is reported and not applied.
- **One lookup created on both computers.** "Ticketmaster" typed on both is
  matched by name and remembered in `sync_uid_alias`. Deleting it on either
  computer deletes it on both.
- **One code on two records.** `ORD-000001` made on both while offline: the
  lower uid keeps it, the other is renumbered and sent again. An event code is
  renumbered within its own event (`OASIS-002`). Counters are lifted so a code
  that arrived is never minted again.
- **`legacy-<id>` rows with different codes** are two different records under
  one migration-027 name. Neither is overwritten; this is reported, as the
  Drive merge's identity clash is.
- **`sheet_alerts.row_id`** has no foreign key on purpose. It travels as the
  row's identity, so a reminder points at the right row on the other computer.

### Settings

The groups `shared_settings.rs` already shares (mail rules, senders, filter,
interval, webhooks) travel live too, newest stamp wins.

**The Gmail credential does not travel through Firebase** (`NOT_LIVE`): a
Firebase password should not be a way into the mailbox. Each computer connects
Gmail once itself. If the other computer connects a *different* mailbox, this
computer drops its own credential and asks for the new one. A computer with no
mailbox never disconnects one that has it.

### Google Drive

Each computer records in `users/{uid}/devices` whether it syncs live.

- **Every computer seen in the last 30 days is live:**
  - At launch, Drive is not combined or pulled. It would only ask a question
    with nothing behind it.
  - On close, the Drive copy is replaced, because this computer's copy is as
    complete as any.
- **Any computer is still on the old way:** Drive works exactly as before.

### Restoring a copy

A backup, a restore point, an older Drive version or a Sync down **pauses live
sync on that computer** (`pause_after_restore`). Left on, it would at once
bring back over the copy everything the other computer changed since, so the
restore would quietly undo itself.

A banner says this and offers "Turn on again". Changes made meanwhile are
still captured and are sent once live sync is back on.

### The screens

- **Refreshing.** Dashboard, Events, Event detail, Orders, Order detail,
  Inventory, Sales, Sale detail, Pulls (both lists), Finance and the Notes
  list refresh by themselves when records arrive (`lib/dataChanged.ts`).
- **No disruption.** A detail page keeps what is ticked. The note open in the
  editor is not reloaded under the cursor.
- **Sidebar.** A line above Light/Dark shows the state: *Live*,
  *Live · 3 to send*, *Sending 12*, *Offline · 4 queued*, *Sync needs setup*,
  *Daily limit reached*.
- **Settings → Data.** A "Live sync" card holds the switch, the last received
  and sent times, records waiting for a parent, anything that could not be
  applied, and the two steps for publishing the rules.
- **First start.** A one-time banner offers live sync. Records leave a
  computer only when marko turns it on there.

## 3. What marko has to do, once

1. **Publish the new rules.** Firebase Console → TIQR Manager project →
   Firestore Database → Rules → replace everything with `firestore.rules` from
   this release → **Publish**. Until then the card says "Sync needs setup" and
   tries again every minute by itself.
2. **Update both computers** to 2.112.0.
3. **On each computer, click "Turn on"** in the banner (or in Settings → Data).

The first turn-on sends every record once.

- **First computer.** For a few thousand records this takes a minute or two.
- **Second computer.** It reads them, keeps whichever version of each record
  is newer, and sends only what it alone has.

## 4. Decisions

- **Newer edit wins, per record, not per field.** If both computers edit the
  same order at the same time, the later save wins the whole order, the same
  on both.
- **Opt-in per computer.** The app has other approved accounts too. Their
  records go to Firebase only if they turn it on.
- **Deletion is final**, as it already was in the Drive merge.
- **Mail itself does not travel.** Each computer reads Gmail itself, as it
  did in 2.110.0.

## 5. Tests

**Rust: 23 new tests** in `commands/live_sync.rs`. They use two in-memory
databases and a fake Firestore with the same write rule as `firestore.rules`.
Pages are applied 7 records at a time, so page boundaries are exercised.

Scenarios:
- new order with tickets, ids differ
- no echo
- an edit travels, and the newer of two concurrent edits wins on both
- deletion with cascade
- deletion beats a later edit, in either order
- one platform typed on both, then deleted
- two orders with one code, and event-code renumbering
- ticket before its order
- computers that already shared everything send each record once
- off/on keeps changes
- nothing captured before the first turn-on, and an untouched save is not a
  change
- triggers on every table, surviving restarts
- a bad document doesn't stop its page
- foreign-key rules for a deleted parent
- reminder row translation
- legacy-uid clash
- settings, with the credential staying home
- the Drive merge is not re-sent
- a 2 MB picture
- the take byte budget
- cursor and device id
- a restore pauses live sync

**Mutation-checked.** Each of these mutations fails its test:
- capturing while applying
- dropping the same-content rule
- reversing newer-wins
- turning "waiting" into "rejected"

**Emulator check: 9 steps** (`scripts/live-sync-emulator-check.ts`, run as
the file's header says). Two computers on the **real `firestore.rules`** in
the Firestore emulator, both signed in as one user:
- records arrive, the picture in parts
- an edit comes back
- an older version is refused and the batch still goes
- deletion wins whichever reaches Firestore first, and nothing brings it back
- settings travel
- a restarted computer reads only what is new
- the device registry
- another account can neither read nor write
- malformed records and deletes are refused

**Commands run:**
- `cargo test --locked`: see the commit message for the final count
- `npm test`: 21 passed (10 new)
- `npm run build`: passes
- The sidebar line, the banners and the card were rendered in Chromium
  against a mocked backend.

**Not verified:** the real Firebase project with the rules published, real
quota behaviour, and two real computers.

## 6. Limits

- **Free tier.** Firestore's free tier allows 20 000 writes and 50 000 reads a
  day. Everyday use is far below that. The first turn-on writes one document
  per record; if the limit is reached, the card says so and sync carries on
  later by itself.
- **The open note.** A note open in the editor while the other computer edits
  the same note shows the new text after reopening it.
- **Restore on both.** A restore on one computer does not restore the other.
  Restoring "both computers to yesterday" means a restore on each, with live
  sync left off until both are done.

## Files

- **New:**
  - `src-tauri/migrations/046_live_sync.sql`
  - `src-tauri/src/commands/live_sync.rs`
  - `src/lib/liveSyncCore.ts` and its `.test.ts`
  - `src/lib/liveSyncSession.ts`
  - `src/lib/liveSync.ts`
  - `src/lib/dataChanged.ts`
  - `src/components/LiveSyncCard.tsx`
  - `scripts/live-sync-emulator-check.ts`
  - `firebase.emulator.json` (an emulator config only: nothing deploys it)
- **Changed:**
  - `firestore.rules`
  - `db.rs`
  - `cloud_merge.rs`
  - `backup.rs`
  - `shared_settings.rs`
  - `mail.rs`
  - `lib.rs`
  - `database.rs` (canary 45 → 46)
  - `Layout.tsx`
  - `Settings.tsx`
  - eleven pages (refresh)
  - `api.ts`
  - `types.ts`
  - `firebase.ts`
  - `GMAIL-SETUP.md`
  - `CHANGELOG.md`
  - `PROJECT_STATE/*`

**Migrations:** 46 numbered ones; the next is **047**.

# TIQR Manager 2.111.0 — Priority-based Gmail sync

New mail first, then **every** starred message, then **every** message in a
Gmail label, and only then the ordinary history. What decides is what a
message *is*, not how old it is.

---

## 1. What the code did before (inspected, not assumed)

Confirmed in `commands/mail_sync.rs` as released in 2.110.0:

| Item | Found |
|---|---|
| Listing | `messages.list`, `maxResults=500`, one page per run, page token kept on disk per folder |
| Download | `messages.get?format=metadata` (From, Subject, Date, To, Reply-To) |
| Queue | `mail.mail_fetch_queue` (`provider_message_id` PK, `priority`, `seq`, `attempts`) |
| Batch / workers | 40 ids per run, 4 threads |
| Incremental | `history.list` with `messageAdded`, `labelAdded` and `labelRemoved`; a 404 response walked every folder again |
| Bodies | fetched only when a message is opened (`get_mail_body`), never stored |
| Driver | `Layout.tsx` chains runs 250 ms apart while there is work |

**Gaps against the spec. These were the real bottlenecks:**

1. **Wrong order.** The lanes were Important → Starred → Labels. Starred and
   labelled mail waited behind all the historical mail from watched senders.
2. **New mail was slow.** History was read only once per `max(interval, 30 s)`,
   so with a 5-minute interval new mail could wait 5 minutes during an import.
   New promotions, spam and trash also went to the back of the queue.
3. **Failing messages were dropped.** After 5 counted failures a message was
   *deleted* from the queue. It came back only on the next walk of its folder,
   and a finished folder is walked again only after Gmail's history expires.
4. **No honest per-lane status.** The screen showed "Reading ⟨first
   unfinished folder⟩ · N waiting". It could not tell starred, label and
   backlog work apart, or say whether discovery was finished.
5. **One run was not atomic.** It was dozens of separate autocommits. Each
   Discord post for an Automatic record was a network call made in the middle
   of the run.
6. **Backoff.** A 400 on the history cursor was not handled. Waits were fixed,
   had no jitter, and ignored `Retry-After` at run level. A refresh replaced
   every row object, so all 200 rows were redrawn.

---

## 2. How it works now

### Lanes (`TIER_*`)

| Lane | Contents | Found by |
|---|---|---|
| 0 new | anything that arrived, in any folder | `history.list`, read every 20 s while there is work |
| 10 starred | every `STARRED` message, any age, archived ones included | walk of `labelIds=STARRED` |
| 20 labels | every message in each user label (`type=user`), any age | one walk per label |
| 30–80 backlog | Important (watched senders), Finances, Inbox, Promotions, Spam, Trash | one walk per folder, newest first |

**System labels.**
- `STARRED` is lane 10.
- `INBOX`, `CATEGORY_*`, `SPAM` and `TRASH` are the backlog folders. The
  Inbox walk includes archived mail.
- Gmail's `IMPORTANT`, `UNREAD` and categories are **kept on the message**
  but do not set its lane. Gmail puts `IMPORTANT` on a large share of all mail.
- `SENT`, `DRAFT` and `CHAT` are not read unless the message is starred or
  labelled. TIQR is an incoming-mail tool; the Inbox folder already left out
  sent-only mail.
- A label walk, like Gmail's own label view, does not list spam or trash.

### Discovery is separate from download

Each run does three things, in this order:

1. **Reads the history.** New ids go to lane 0. A star or label added to a
   message not stored yet moves it **up** to lane 10 or 20. A change to a
   stored message (star, label, inbox, spam or trash, added or removed) is
   applied **in place**, with no download. Read and unread are not taken from
   Gmail; TIQR keeps its own read state, by design since migration 041.
2. **Lists one page** of the first unfinished folder, in lane order. This only
   happens while fewer than 200 ids are due, so discovery stays just ahead of
   downloading.
3. **Downloads one batch** of 40: `ORDER BY priority, seq, provider_message_id`
   over ids that are due now.

### The queue (mail schema v2, `mail_store.rs::SCHEMA_V2`)

**Columns.** `provider_message_id` (PK) and `account_email` stay. New:
- `source`: the folder key or `history`, `history:starred`, `history:label`
- `status`: `queued`, `retrying` or `parked`
- `last_error`
- `updated_at`; `queued_at` is the creation time
- `next_attempt_at`

`priority`, `seq` (listing order) and `attempts` remain. New indexes on `seq`
and `next_attempt_at`.

**Behaviour.**
- **One id, one row.** `enqueue` only ever moves an id *up* to a better lane,
  and records the better source. A message in the Inbox walk that Starred
  finds later becomes a starred one. A starred message in three labels is
  queued once and downloaded once.
- **Stored or deleted ids are never queued.** Walking mail that is already
  here costs listing only.
- **Message date.** The date is not stored in the queue because nothing knows
  it before download: listings and history give only ids. Gmail lists newest
  first and `seq` keeps that order, so within a folder the order is date
  descending.
- **No lease.** Workers never touch the database. One run at a time holds an
  in-memory guard, and a crash leaves every row queued for the next start.

### Failures

- **One message failing** while Google answers for the rest of its batch steps
  aside: retried after 30 s, doubling each time. After 5 attempts it is
  **parked** and retried every 6 h. It is never deleted.
- **No connection, or Google failing everything.** Nothing counts against any
  message. The run says "Could not reach Gmail" and waits 30 s, doubling up to
  2 min.
- **Rate limit** (429, or 403 `rateLimitExceeded`): every worker pauses and the
  batch stays queued. The next run waits 20 s, doubling for each such run in a
  row up to 5 min, with jitter. It never waits less than Google's
  `Retry-After`.
- **Expired or invalid history cursor** (404 or 400): a fresh starting point is
  taken and every folder is walked again. This happens at most once an hour, so
  a request bug cannot loop it. Stored mail and the queue are kept, and a page
  listed in the same run is discarded rather than continuing the old walk.
- **Crash.** A run's writes are **one transaction** (`apply_committed`). Each
  Automatic record has its own savepoint, so a half-made record leaves
  nothing. Discord posts go out after the commit.
- **Gmail batch endpoint: not used.** It charges each call the same 5 units,
  so it cannot get past the quota ceiling (about 50 `messages.get` per second).
  4 workers already reach roughly 40 per second.

### The inbox while syncing

- Folders come straight from the database, so already-stored mail shows at
  once.
- **Sync now** asks the single reader to run now. When reading is paused, the
  button resumes it.
- A refresh merges the fresh first page into what is on screen
  (`lib/mailRows.ts`). Unchanged rows keep the same object, so they are not
  redrawn, and everything already scrolled to stays.
- **The status line** above the list shows, all from `progress_of`:
  - what the reader is doing, e.g. *"Starred first - the rest of your history
    follows"*
  - each lane's exact queued count; `N+` means more are still being found
  - lanes not started yet: *"after Starred"*
  - *Retrying later N*
  - *Last sync*

  No total is ever guessed.

---

## 3. Tests

| # | Scenario | Test |
|---|---|---|
| 1 | new before history | `new_mail_comes_before_ordinary_history`, e2e |
| 2 | year-old star before yesterday | `a_starred_mail_from_last_year_comes_before_yesterdays_ordinary_mail`, e2e |
| 3 | year-old label before history | `a_labelled_mail_from_last_year_comes_before_ordinary_history`, e2e |
| 4 | several labels → stored once | `a_message_in_several_labels_is_queued_once_and_stored_once_with_all_of_them` |
| 5 | starred + labelled, no duplicate fetch | `starred_and_labelled_is_queued_once_in_the_better_lane`, e2e counts every GET |
| 6 | promotion | `a_queued_history_message_moves_up_when_starred_finds_it` |
| 7 | new mail during backfill | `new_mail_is_downloaded_while_a_long_backlog_is_being_read`, e2e |
| 8 | star and label add and remove | `stars_and_labels_follow_gmail_both_ways_without_downloading_again`, `the_end_of_a_walk_takes_off_a_star_gmail_no_longer_has` |
| 9 | restart keeps progress | `a_restart_carries_on_with_the_same_queue_and_the_same_page` (file-backed) |
| 10 | rate limit → backoff and retry | `a_rate_limit_from_gmail_is_waited_out_and_the_request_tried_again` (real HTTP, `Retry-After`), `rate_limited_runs_wait_longer_each_time_and_never_less_than_google_said` |
| 11 | expired cursor recovery | `lost_history_walks_every_folder_again_and_keeps_every_message` (404, then 400 inside and outside the hour) |
| 12 | failed message does not block | `a_failing_message_steps_aside_and_is_never_dropped`, `nothing_but_messages_waiting_to_be_retried_does_not_keep_runs_spinning` |
| 13 | no duplicate messages or records | `syncing_the_same_mail_again_creates_no_second_message_and_no_second_record`, `a_record_automatic_fails_to_create_leaves_nothing_half_written` |
| 14 | UI shows synced mail during sync | `stored_mail_is_listed_while_a_long_read_is_under_way`; `src/lib/mailRows.test.ts` |
| 15 | data survives the migration | `mail_from_2_110_survives_schema_2_and_every_queued_id_keeps_its_lane` |

**End to end.**
`a_whole_mailbox_comes_down_new_then_starred_then_labels_then_history` runs
plan → HTTP → store against a **fake Gmail on 127.0.0.1**, with no account and
no credentials. The fake mailbox:
- 61 starred and 151 labelled messages, all a year old
- 120 ordinary messages from yesterday
- a new message that arrives mid-run

The folders are large enough that each lane is still downloading when the next
is discovered. Only the queue can keep the order.

**Mutation-checked.** Each of these makes the end-to-end test fail:
- moving labels behind the Inbox
- moving starred behind labels
- putting new mail into the backlog

**Commands run, results:**
- `cargo test --locked` (src-tauri): **1461 passed, 0 failed, 4 ignored**
  (the 4 are the existing `#[ignore]` performance tests: 10k, 50k, 100k rows and
  100k mail). 29 test
  names are new; 8 old ones were renamed or replaced where the behaviour
  changed on purpose: dropping after 5 failures, and new spam going to the back.
- `npm test`: **11 passed, 0 failed** (new script; plain Node, no
  dependency, needs Node 22.18+)
- `npm run build` (`tsc -b` + Vite): **passes**. The chunk-size warning was
  already there.
- The Mail screen was rendered in Chromium against a mocked backend in three
  states (Starred phase, Labels phase, up to date) and at 1180 px wide.

**Not verified:** a real Gmail account, Google's real quota behaviour, and
two computers at once.

---

## 4. Known limitations and decisions

- **Important moved.** Important (watched senders) used to be first. It is now
  the first part of the history, behind Starred and labels. That follows the
  new spec; it is one constant to put back.
- **Pre-emption between batches.** Lanes are picked again for every batch of
  40, which takes about a second. A request already in flight is never
  cancelled.
- **No demotion.** A message that *loses* a star while still queued keeps its
  lane. History gives only the change, not the message's full label set.
- **One account at a time** (KNOWN_BUGS: folders do not filter by account).
- **Not in CI.** `npm test` is not added to CI, which runs Node 20.

## Files

- **New:**
  - `src/lib/mailSyncStatus.ts`
  - `src/lib/mailRows.ts`
  - their two `.test.ts` files
- **Changed:**
  - `src-tauri/src/commands/mail_sync.rs`
  - `src-tauri/src/mail_store.rs` (SCHEMA_V2)
  - `src-tauri/src/models.rs`
  - `src-tauri/src/commands/mail.rs`
  - `src/pages/MailIntelligence.tsx`
  - `src/components/MailSettingsPane.tsx`
  - `src/lib/types.ts`
  - `package.json` (`test` script)
  - `GMAIL-SETUP.md`
  - `CHANGELOG.md`
  - `PROJECT_STATE/*`

**Migrations.** No numbered migration; the next is still **046**. The mail
file goes from `user_version` 1 to 2. Queue priorities and walk tiers from
2.110.0 are renumbered in place, and nothing is lost.

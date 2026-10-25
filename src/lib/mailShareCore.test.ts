// Run with `npm test` (Node 22.18+ runs TypeScript directly).
import { test } from "node:test";
import assert from "node:assert/strict";
import {
  cursorAfter,
  cursorId,
  dropId,
  dropPrefix,
  leaseId,
  parseCursorDoc,
  parseDrop,
  parseLease,
  platformName,
  shareLine,
  takesLease,
  seeLease,
  presentUntil,
  EMPTY_SHARE_VIEW,
} from "./mailShareCore.ts";

const A = "0123456789abcdef0123456789abcdef";
const B = "fedcba9876543210fedcba9876543210";
const at = (ms: number) => ({ toMillis: () => ms });

test("one lease per mailbox, whatever the case of the address, with no address in it", () => {
  assert.equal(leaseId("BiznisMarko9@gmail.com"), leaseId(" biznismarko9@gmail.com "));
  assert.notEqual(leaseId("a@gmail.com"), leaseId("b@gmail.com"));
  assert.match(leaseId("a@gmail.com"), /^lease~[0-9a-f]{16}$/);
  assert.equal(leaseId("a"), "lease~af63dc4c8601ec8c", "FNV-1a 64 of \"a\"");
});

test("a lease is taken when it is free, already this computer's, or not renewed - never from a live reader", () => {
  const now = 10_000_000;
  const lease = { dev: B, atMs: now - 30_000, account: "a@gmail.com", platform: "Win32" };
  const seen = seeLease(lease, null, now);
  assert.equal(takesLease(null, A, null, now), true);
  assert.equal(takesLease(lease, B, null, now), true, "its own");
  assert.equal(takesLease(lease, A, seen, now), false, "the other computer read half a minute ago");
  // The same lease three minutes on: nobody renewed it.
  const later = now + 181_000;
  assert.equal(takesLease(lease, A, seeLease(lease, seen, later), later), true, "not renewed for over 3 minutes");
  // Renewed in between: seen afresh, so still the other computer's.
  const renewed = { ...lease, atMs: now + 150_000 };
  assert.equal(takesLease(renewed, A, seeLease(renewed, seen, later), later), false);
});

test("a clock that is off never makes a live reader look gone", () => {
  const now = 10_000_000;
  // This computer's clock runs ten minutes fast: the lease looks old by it.
  const lease = { dev: B, atMs: now - 10 * 60_000, account: "a@gmail.com", platform: "MacIntel" };
  assert.equal(takesLease(lease, A, seeLease(lease, null, now), now), false, "renewed lately as far as anyone can tell");
  // Left from long ago: taken at first sight.
  const old = { ...lease, atMs: now - 16 * 60_000 };
  assert.equal(takesLease(old, A, seeLease(old, null, now), now), true);
});

test("a follower counts as there for three minutes after its cursor changes", () => {
  const now = 10_000_000;
  const cur = { follower: A, reader: B, upTo: { at: "", id: 0 }, received: 0, platform: "Win32", atMs: now - 60_000, nudge: 0 };
  assert.equal(presentUntil(cur, now, false), now + 180_000, "seen to change just now");
  assert.equal(presentUntil(cur, now, true), now + 120_000, "first look: a minute of it gone already");
  assert.ok(presentUntil({ ...cur, atMs: now - 86_400_000 }, now, true) < now, "left from yesterday");
  assert.equal(presentUntil({ ...cur, atMs: now + 60_000 }, now, true), now + 180_000, "a clock behind never adds time");
});

test("ids sort the way they were written, and cursors compare by time then row", () => {
  const a = dropId(A, 1_700_000_000_000, 1);
  const b = dropId(A, 1_700_000_000_000, 2);
  const c = dropId(A, 1_700_000_000_500, 0);
  assert.ok(a < b && b < c, `${a} ${b} ${c}`);
  assert.ok(a.startsWith(dropPrefix(A)));
  assert.match(a, /^drop~[0-9a-f]{32}~[0-9a-z]{12}$/);
  assert.equal(cursorId(A, B), `cursor~${A}~${B}`);
  assert.ok(cursorAfter({ at: "2026-10-02", id: 1 }, { at: "2026-10-01", id: 9 }));
  assert.ok(cursorAfter({ at: "2026-10-01", id: 10 }, { at: "2026-10-01", id: 9 }));
  assert.ok(!cursorAfter({ at: "2026-10-01", id: 9 }, { at: "2026-10-01", id: 9 }));
});

test("what Firestore holds is read only when it has the right shape", () => {
  const lease = parseLease({ dev: A, at: at(5), p: JSON.stringify({ account: "a@gmail.com", platform: "MacIntel" }) });
  assert.deepEqual(lease, { dev: A, atMs: 5, account: "a@gmail.com", platform: "MacIntel" });
  assert.equal(parseLease({ dev: "nope", p: "{}" }), null);
  assert.equal(parseLease({ dev: A, p: "not json" }), null);

  const c = parseCursorDoc(cursorId(B, A), { dev: B, at: at(7), p: JSON.stringify({ upTo: { at: "x", id: 3 }, received: 41, platform: "Win32" }) });
  assert.deepEqual(c, { follower: B, reader: A, upTo: { at: "x", id: 3 }, received: 41, platform: "Win32", atMs: 7, nudge: 0 });
  const nudged = parseCursorDoc(cursorId(B, A), { dev: B, at: at(7), p: JSON.stringify({ upTo: { at: "x", id: 3 }, nudge: 1234 }) });
  assert.equal(nudged?.nudge, 1234, "Sync pressed on the follower");
  assert.equal(parseCursorDoc(cursorId(B, A), { dev: B, p: JSON.stringify({ upTo: { at: "x", id: 3 }, nudge: "soon" }) })?.nudge, 0);
  assert.equal(parseCursorDoc(cursorId(B, A), { dev: A, p: "{}" }), null, "written by the follower only");
  assert.equal(parseCursorDoc("cursor~x~y", { dev: B, p: "{}" }), null);

  const d = parseDrop({ dev: A, p: JSON.stringify({ account: "a@gmail.com", upTo: { at: "x", id: 2 }, rows: [{ id: "m1" }, 7, null] }) });
  assert.equal(d?.sender, A);
  assert.equal(d?.rows.length, 1);
  assert.equal(parseDrop({ dev: A, p: JSON.stringify({ account: "a@gmail.com", rows: [] }) }), null, "no cursor");
});

test("the sentence says who reads Gmail", () => {
  const fmt = (n: number) => n.toLocaleString("en-US");
  assert.equal(platformName("MacIntel"), "Mac");
  assert.equal(platformName("Win32"), "Windows computer");
  assert.equal(
    shareLine({ ...EMPTY_SHARE_VIEW, role: "follower", peer: "Mac", received: 84_000 }, fmt),
    "Your Mac reads Gmail and sends the mail here - 84,000 received.",
  );
  assert.equal(
    shareLine({ ...EMPTY_SHARE_VIEW, role: "reader", peer: "Windows computer", sent: 12 }, fmt),
    "This computer reads Gmail and sends the mail to your Windows computer - 12 sent.",
  );
  assert.equal(
    shareLine({ ...EMPTY_SHARE_VIEW, role: "reader" }, fmt),
    "This computer reads Gmail. When your other computer is on too, it gets the mail from here.",
  );
  assert.equal(shareLine({ ...EMPTY_SHARE_VIEW, role: "solo", detail: "Live sync is off here." }, fmt), "Live sync is off here.");
});

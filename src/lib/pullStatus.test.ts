// Run with `npm test` (Node 22.18+ runs TypeScript directly).
import { test } from "node:test";
import assert from "node:assert/strict";
import { isPullDone, isReceivedPullDone } from "./pullStatus.ts";

test("a pull is completed only when it is both paid and transferred", () => {
  assert.equal(isPullDone({ paid: true, transferDone: true }), true);
  assert.equal(isPullDone({ paid: true, transferDone: false }), false);
  assert.equal(isPullDone({ paid: false, transferDone: true }), false);
  assert.equal(isPullDone({ paid: false, transferDone: false }), false);
});

test("a received pull is completed once it became an order or its event is over", () => {
  const today = new Date(2026, 9, 20); // 20 Oct 2026, local time
  assert.equal(isReceivedPullDone({ orderId: 7, eventDate: "2027-01-01" }, today), true);
  assert.equal(isReceivedPullDone({ orderId: null, eventDate: "2026-10-19" }, today), true);
  // The day of the event itself is still active.
  assert.equal(isReceivedPullDone({ orderId: null, eventDate: "2026-10-20" }, today), false);
  assert.equal(isReceivedPullDone({ orderId: null, eventDate: "2026-12-31" }, today), false);
  assert.equal(isReceivedPullDone({ orderId: null, eventDate: null }, today), false);
});

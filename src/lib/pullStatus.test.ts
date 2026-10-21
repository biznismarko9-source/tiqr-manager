// Run with `npm test` (Node 22.18+ runs TypeScript directly).
import { test } from "node:test";
import assert from "node:assert/strict";
import { formatMoney } from "./format.ts";
import { feeTotals, isPullDone, isReceivedPullDone, soldCell } from "./pullStatus.ts";

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

test("a received pull shows what its tickets sold for, and how many of them", () => {
  const eur = (cents: number) => [{ currency: "EUR", cents }];
  assert.deepEqual(soldCell({ orderId: 7, quantity: 2, sold: eur(45_000), soldQuantity: 2 }), {
    amounts: [formatMoney(45_000, "EUR")],
    part: null,
    title: "2 tickets sold",
  });
  assert.deepEqual(soldCell({ orderId: 7, quantity: 4, sold: eur(20_000), soldQuantity: 1 }), {
    amounts: [formatMoney(20_000, "EUR")],
    part: "1/4",
    title: "1 of 4 tickets sold",
  });
  // Two currencies stay side by side, never added together.
  const mixed = soldCell({
    orderId: 7,
    quantity: 3,
    sold: [{ currency: "GBP", cents: 20_000 }, { currency: "EUR", cents: 10_000 }],
    soldQuantity: 3,
  });
  assert.deepEqual(mixed.amounts, [formatMoney(20_000, "GBP"), formatMoney(10_000, "EUR")]);
  assert.equal(soldCell({ orderId: 7, quantity: 1, sold: eur(9_900), soldQuantity: 1 }).title, "1 ticket sold");
  assert.deepEqual(soldCell({ orderId: 7, quantity: 2, sold: [], soldQuantity: 0 }), {
    amounts: [],
    part: null,
    title: "Nothing sold yet",
  });
  assert.deepEqual(soldCell({ orderId: null, quantity: 2, sold: [], soldQuantity: 0 }), {
    amounts: [],
    part: null,
    title: "Not linked to an order",
  });
});

test("pulls done for others add up to what they earned, paid and still owed", () => {
  const money = formatMoney;
  const t = feeTotals([
    { priceCents: 4_000, currency: "EUR", paid: true },
    { priceCents: 2_500, currency: "EUR", paid: false },
    { priceCents: 5_000, currency: "GBP", paid: false },
    { priceCents: 0, currency: "EUR", paid: false },
  ]);
  assert.deepEqual(t, {
    count: 4,
    // Each currency on its own, the biggest first, and in that same order below.
    earned: `${money(6_500, "EUR")} + ${money(5_000, "GBP")}`,
    paid: money(4_000, "EUR"),
    outstanding: `${money(2_500, "EUR")} + ${money(5_000, "GBP")}`,
    anyEarned: true,
  });
  const nothingPaid = feeTotals([{ priceCents: 3_000, currency: "EUR", paid: false }]);
  assert.equal(nothingPaid?.paid, money(0, "EUR"));
  assert.equal(nothingPaid?.outstanding, money(3_000, "EUR"));
  const allPaid = feeTotals([{ priceCents: 3_000, currency: "EUR", paid: true }]);
  assert.equal(allPaid?.outstanding, money(0, "EUR"));
  assert.equal(feeTotals([{ priceCents: 0, currency: "EUR", paid: true }])?.anyEarned, false);
  assert.equal(feeTotals([]), null);
});

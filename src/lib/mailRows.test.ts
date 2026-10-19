// Run with `npm test` (Node 22.18+ runs TypeScript directly).
import { test } from "node:test";
import assert from "node:assert/strict";
import { mergeTopPage, sameRow } from "./mailRows.ts";

interface Row {
  id: number;
  receivedAt: string;
  subject: string;
  isUnread: boolean;
}

function row(id: number, day: number, subject = `m${id}`, isUnread = true): Row {
  return { id, receivedAt: `2026-10-${String(day).padStart(2, "0")}T10:00:00Z`, subject, isUnread };
}

/** Newest first, the way a folder is listed. */
function folder(...rows: Row[]): Row[] {
  return [...rows].sort((a, b) => (a.receivedAt === b.receivedAt ? b.id - a.id : a.receivedAt < b.receivedAt ? 1 : -1));
}

test("an unchanged message keeps the very same object, so its row is not drawn again", () => {
  const prev = folder(row(1, 1), row(2, 2), row(3, 3));
  const fresh = folder(row(1, 1), row(2, 2), row(3, 3), row(4, 4));
  const { rows, hasMore } = mergeTopPage(prev, fresh, 10);
  assert.equal(rows.length, 4);
  assert.equal(rows[0].id, 4, "new mail on top");
  for (const old of prev) {
    assert.equal(rows.find((r) => r.id === old.id), old, `row ${old.id} kept as it was`);
  }
  assert.equal(hasMore, false);
});

test("a message that changed is replaced", () => {
  const prev = folder(row(1, 1), row(2, 2));
  const fresh = folder(row(1, 1), row(2, 2, "m2", false));
  const { rows } = mergeTopPage(prev, fresh, 10);
  const two = rows.find((r) => r.id === 2)!;
  assert.notEqual(two, prev.find((r) => r.id === 2));
  assert.equal(two.isUnread, false);
});

test("messages already on screen stay visible while a long read pours mail in", () => {
  // He scrolled down to 30 messages; the page size is 10.
  const prev = folder(...Array.from({ length: 30 }, (_, i) => row(i + 1, i + 1)));
  // Two new messages arrive: the fresh first page is the newest 10.
  const all = folder(row(31, 31), row(32, 31), ...prev);
  const fresh = all.slice(0, 10);
  const { rows, hasMore } = mergeTopPage(prev, fresh, 10);
  assert.equal(rows.length, 32, "nothing he had is gone");
  assert.deepEqual(rows.slice(0, 2).map((r) => r.id), [32, 31]);
  assert.equal(new Set(rows.map((r) => r.id)).size, rows.length, "no row twice");
  assert.equal(hasMore, null, "what is below is left as it was");
  const last = prev[prev.length - 1];
  assert.equal(rows[rows.length - 1], last, "the oldest row he scrolled to is the same object");
});

test("a small folder is simply the fresh page - a message that left it goes", () => {
  const prev = folder(row(1, 1), row(2, 2), row(3, 3));
  const fresh = folder(row(1, 1), row(3, 3));
  const { rows } = mergeTopPage(prev, fresh, 10);
  assert.deepEqual(rows.map((r) => r.id), [3, 1]);
});

test("rows compare field by field", () => {
  assert.ok(sameRow(row(1, 1), row(1, 1)));
  assert.ok(!sameRow(row(1, 1), row(1, 1, "other")));
});

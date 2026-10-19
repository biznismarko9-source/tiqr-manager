// Run with `npm test` (Node 22.18+ runs TypeScript directly).
import { test } from "node:test";
import assert from "node:assert/strict";
import {
  batches,
  classify,
  fromStored,
  INLINE_BYTES,
  laterCursor,
  partId,
  phaseLabel,
  replaces,
  retryDelay,
  splitBody,
  toStored,
  utf8Length,
  type SyncDoc,
} from "./liveSyncCore.ts";

const v = (h: number, dev: string, del = false) => ({ h, dev, del });

test("a newer version replaces an older one, and a tie goes to the higher device", () => {
  assert.equal(replaces(v(2, "a"), v(1, "b")), true);
  assert.equal(replaces(v(1, "b"), v(2, "a")), false);
  assert.equal(replaces(v(5, "b"), v(5, "a")), true);
  assert.equal(replaces(v(5, "a"), v(5, "b")), false);
  assert.equal(replaces(v(5, "a"), v(5, "a")), false, "the same version again is not a change");
  assert.equal(replaces(v(1, "a"), null), true);
});

test("a deletion beats any edit, and no edit brings a deleted record back", () => {
  assert.equal(replaces(v(1, "a", true), v(9, "b")), true, "an older deletion still wins");
  assert.equal(replaces(v(9, "b"), v(1, "a", true)), false, "a newer edit does not undo it");
  assert.equal(replaces(v(2, "a", true), v(1, "b", true)), true);
  assert.equal(replaces(v(1, "a", true), v(2, "b", true)), false);
});

test("sizes are counted the way Firestore counts them", () => {
  assert.equal(utf8Length("abc"), 3);
  assert.equal(utf8Length("čšž"), 6);
  assert.equal(utf8Length("€"), 3);
  assert.equal(utf8Length("😀"), 4);
});

test("a big body is split into parts and put back exactly, emoji included", () => {
  const body = "a😀".repeat(10) + "b";
  for (let units = 1; units < 8; units++) {
    const parts = splitBody(body, units);
    assert.equal(parts.join(""), body);
    for (const p of parts) {
      const last = p.charCodeAt(p.length - 1);
      assert.ok(!(last >= 0xd800 && last <= 0xdbff), `a part ends inside an emoji at ${units}`);
    }
  }
  assert.deepEqual(splitBody(""), [""]);
});

test("a record goes inline when it fits and in parts when it does not", () => {
  const small: SyncDoc = { id: "orders~u1", t: "orders", u: "u1", h: 7, del: false, d: { code: "ORD-1", notes: "ok" }, dev: "aa" };
  const inline = toStored(small);
  assert.equal(inline.parts.length, 0);
  assert.deepEqual(fromStored(small.id, inline.row), small);

  const picture = "x".repeat(INLINE_BYTES + 10);
  const big: SyncDoc = { ...small, id: "note_images~u2", t: "note_images", u: "u2", d: { data_uri: picture } };
  const stored = toStored(big);
  assert.ok(stored.parts.length >= 2);
  assert.equal(stored.row.body, undefined);
  assert.equal(fromStored(big.id, stored.row, stored.parts.slice(0, -1).concat([undefined as unknown as string])), null, "a missing part means wait, not a broken record");
  assert.deepEqual(fromStored(big.id, stored.row, stored.parts), big);
  assert.equal(partId(big.id, 7, 0), "note_images~u2~7~0");
});

test("a deletion travels without a body", () => {
  const gone: SyncDoc = { id: "orders~u1", t: "orders", u: "u1", h: 9, del: true, d: {}, dev: "bb" };
  const s = toStored(gone);
  assert.equal(s.row.del, true);
  assert.deepEqual(fromStored(gone.id, s.row), gone);
});

test("writes are grouped under Firestore's limits", () => {
  const items = Array.from({ length: 1000 }, (_, i) => i);
  const groups = batches(items, () => 10, 400, 8_000_000);
  assert.deepEqual(groups.map((g) => g.length), [400, 400, 200]);
  const heavy = batches([1, 2, 3, 4], () => 3_000_000, 400, 8_000_000);
  assert.deepEqual(heavy.map((g) => g.length), [2, 2]);
  const huge = batches([1], () => 9_000_000);
  assert.deepEqual(huge, [[1]], "one item over the byte budget still goes, alone");
});

test("failures say what is wrong and wait sensibly", () => {
  assert.equal(classify("permission-denied"), "rules");
  assert.equal(classify("resource-exhausted"), "quota");
  assert.equal(classify("unavailable"), "offline");
  assert.equal(classify("unauthenticated"), "signed-out");
  assert.equal(classify("internal"), "error");
  assert.equal(classify(undefined), "error");
  assert.equal(retryDelay("offline", 1, () => 0.5), 2_000);
  assert.equal(retryDelay("offline", 3, () => 0.5), 8_000);
  assert.equal(retryDelay("offline", 30, () => 0.5), 300_000);
  assert.equal(retryDelay("quota", 1), 900_000);
  const low = retryDelay("error", 2, () => 0);
  const high = retryDelay("error", 2, () => 0.999);
  assert.ok(low >= 3_200 && high <= 4_800 && low < high);
});

test("the cursor is the later of two commit times", () => {
  const a = { seconds: 10, nanoseconds: 5 };
  const b = { seconds: 10, nanoseconds: 9 };
  const c = { seconds: 11, nanoseconds: 0 };
  assert.equal(laterCursor(a, b), b);
  assert.equal(laterCursor(c, b), c);
  assert.equal(laterCursor(null, a), a);
  assert.equal(laterCursor(a, null), a);
});

test("the sidebar line says what is going on in a few words", () => {
  assert.equal(phaseLabel("live", 0), "Live");
  assert.equal(phaseLabel("live", 3), "Live · 3 to send");
  assert.equal(phaseLabel("offline", 2), "Offline · 2 queued");
  assert.equal(phaseLabel("rules", 0), "Sync needs setup");
  for (const p of ["off", "starting", "catching-up", "live", "sending", "rules", "quota", "offline", "signed-out", "error"] as const) {
    assert.ok(phaseLabel(p, 12).length <= 19, `${p} fits the sidebar: ${phaseLabel(p, 12)}`);
  }
  assert.equal(phaseLabel("off", 0), "Live sync off");
});

import { test } from "node:test";
import assert from "node:assert/strict";
import { createArtworkStore } from "./artworkStore.ts";
import type { ArtworkImage, ArtworkRef } from "./types.ts";

/** A promise resolved by hand, to put replies in any order. */
function later<T>() {
  let resolve!: (v: T) => void;
  let reject!: (e: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

const flush = () => new Promise((r) => setTimeout(r, 0));

function ref(eventId: number, key: string, rev = "r1"): ArtworkRef {
  return { eventId, key, rev, chosen: "auto" };
}

function img(key: string, rev = "r1"): ArtworkImage {
  return { key, rev, src: `data:image/png;base64,${key}`, fit: "cover", title: key, source: "deezer", page: "" };
}

test("the map is read once however many screens ask, and a picture once for every event that shows it", async () => {
  let mapCalls = 0;
  const imageCalls: string[][] = [];
  const store = createArtworkStore({
    map: async () => {
      mapCalls += 1;
      return [ref(1, "a:artist:coldplay"), ref(2, "a:artist:coldplay"), ref(3, "d:name:muse")];
    },
    images: async (keys) => {
      imageCalls.push(keys);
      return keys.map((k) => img(k));
    },
  });
  // Three screens render at once.
  for (let i = 0; i < 3; i++) assert.equal(store.refFor(1), undefined);
  await flush();
  assert.equal(mapCalls, 1);
  // Every row of every screen asks in one pass: one request, each key once.
  for (const id of [1, 2, 3, 1, 2]) {
    const r = store.refFor(id);
    assert.ok(r);
    assert.equal(store.imageFor(r), undefined);
  }
  await flush();
  assert.deepEqual(imageCalls, [["a:artist:coldplay", "d:name:muse"]]);
  // The same artwork on all screens - the very same object.
  const one = store.imageFor(store.refFor(1)!);
  const two = store.imageFor(store.refFor(2)!);
  assert.ok(one);
  assert.equal(one, two);
  assert.equal(store.refFor(99), null, "an event without a picture shows the placeholder");
});

test("a reply that describes the database before a change is never shown after it", async () => {
  const replies = [later<ArtworkRef[]>(), later<ArtworkRef[]>()];
  let call = 0;
  const store = createArtworkStore({ map: () => replies[call++].promise, images: async () => [] });
  store.refFor(1);
  // A choice is made while the first read is still out.
  store.invalidate();
  replies[0].resolve([ref(1, "a:artist:old")]);
  await flush();
  await flush();
  assert.equal(call, 2, "read again, because it changed meanwhile");
  assert.equal(store.refFor(1), undefined, "the old answer was dropped");
  replies[1].resolve([ref(1, "d:name:new", "r2")]);
  await flush();
  assert.equal(store.refFor(1)?.key, "d:name:new");
});

test("a picture the backend does not have stays a placeholder and is not asked for again", async () => {
  let calls = 0;
  const store = createArtworkStore({
    map: async () => [ref(1, "d:name:gone")],
    images: async () => {
      calls += 1;
      return [];
    },
  });
  store.refFor(1);
  await flush();
  const r = store.refFor(1)!;
  store.imageFor(r);
  await flush();
  assert.equal(store.imageFor(r), null);
  store.imageFor(r);
  await flush();
  assert.equal(calls, 1);
  // A new revision is a new picture: asked for.
  store.imageFor({ ...r, rev: "r2" });
  await flush();
  assert.equal(calls, 2);
});

test("a failed request is reported and asked again only after a pause", async () => {
  let clock = 1_000;
  let fail = true;
  let calls = 0;
  const reports: string[] = [];
  const store = createArtworkStore(
    {
      map: async () => [ref(1, "a:artist:coldplay")],
      images: async (keys) => {
        calls += 1;
        if (fail) throw new Error("backend busy");
        return keys.map((k) => img(k));
      },
      report: (where) => reports.push(where),
    },
    { now: () => clock, retryMs: 30_000 },
  );
  store.refFor(1);
  await flush();
  const r = store.refFor(1)!;
  store.imageFor(r);
  await flush();
  assert.deepEqual(reports, ["artwork pictures"], "never swallowed");
  // Every render in the next half minute: nothing asked.
  for (let i = 0; i < 5; i++) assert.equal(store.imageFor(r), undefined);
  await flush();
  assert.equal(calls, 1);
  clock += 31_000;
  fail = false;
  store.imageFor(r);
  await flush();
  assert.equal(calls, 2);
  assert.ok(store.imageFor(r));
});

test("a picture that changed after the map was read is shown, and the map is read again", async () => {
  let mapCalls = 0;
  const store = createArtworkStore({
    map: async () => {
      mapCalls += 1;
      return [ref(1, "d:name:coldplay", mapCalls === 1 ? "r1" : "r2")];
    },
    images: async () => [img("d:name:coldplay", "r2")],
  });
  store.refFor(1);
  await flush();
  const old = store.refFor(1)!;
  store.imageFor(old);
  await flush();
  assert.equal(store.imageFor(old)?.rev, "r2", "the newest picture, never the older one");
  await flush();
  assert.equal(mapCalls, 2);
  assert.equal(store.refFor(1)?.rev, "r2");
});

test("the pictures kept stay bounded, the least recently shown going first", async () => {
  const store = createArtworkStore(
    {
      map: async () => [1, 2, 3, 4].map((n) => ref(n, `k${n}`)),
      images: async (keys) => keys.map((k) => img(k)),
    },
    { maxImages: 2 },
  );
  store.refFor(1);
  await flush();
  for (const n of [1, 2]) store.imageFor(store.refFor(n)!);
  await flush();
  store.imageFor(store.refFor(1)!); // shown again: now the newest
  store.imageFor(store.refFor(3)!);
  await flush();
  assert.equal(store.stats().images, 2);
  assert.ok(store.imageFor(store.refFor(1)!), "kept - it was shown last");
  assert.equal(store.imageFor(store.refFor(2)!), undefined, "dropped, and asked for again");
});

test("a map that cannot be read is reported, and the screens keep their placeholders", async () => {
  const reports: string[] = [];
  const store = createArtworkStore({
    map: async () => {
      throw new Error("database is locked");
    },
    images: async () => [],
    report: (where) => reports.push(where),
  });
  assert.equal(store.refFor(1), undefined);
  await flush();
  assert.deepEqual(reports, ["artwork map"]);
  assert.equal(store.refFor(1), undefined);
});

// 2.116.0 - event pictures, shared by every screen.
//
// One copy of "which picture does each event show" (the map, from
// event_artwork_map) and of the pictures themselves (by key and revision,
// from event_artwork_images), for the Events list, an event's page,
// Inventory, an order, Sales and a sale alike. A screen never looks for a
// picture of its own: it asks this store for its event's picture, which is
// the same answer every other screen gets.
//
// * The map is loaded once and again only when something changed
//   (`invalidate`): a choice made here, an event saved, or live sync bringing
//   events or choices from the other computer. A reply that arrives after a
//   newer `invalidate` is dropped - it describes the database as it was.
// * Pictures are asked for in batches (everything a screen renders in one
//   pass is one request), each key and revision at most once at a time, and
//   kept by revision: a changed picture has a new revision, so a cached copy
//   is never shown after it changed. At most `maxImages` are kept; the least
//   recently shown go first.
// * A picture the backend does not have is remembered as missing for that
//   revision (the placeholder stays, nothing is asked again); a request that
//   failed is asked again only after a pause, never in a loop.
//
// Free of React and of Tauri, so it is tested on its own
// (artworkStore.test.ts); lib/eventArtwork.ts connects it to both.
import type { ArtworkImage, ArtworkRef } from "./types.ts";

export interface ArtworkFetchers {
  map: () => Promise<ArtworkRef[]>;
  images: (keys: string[]) => Promise<ArtworkImage[]>;
  /** Where a failure goes - it is never dropped silently. */
  report?: (where: string, e: unknown) => void;
}

export interface ArtworkStoreOptions {
  maxImages?: number;
  /** How a batch is put off until the current render pass is done. */
  defer?: (fn: () => void) => void;
  now?: () => number;
  /** How long a failed request waits before it may be asked again. */
  retryMs?: number;
}

export interface ArtworkStore {
  /** `undefined` while the map is loading; `null` when the event shows no
   *  picture. */
  refFor(eventId: number): ArtworkRef | null | undefined;
  /** `undefined` while the picture is loading; `null` when there is none. */
  imageFor(ref: ArtworkRef): ArtworkImage | null | undefined;
  /** Loads the map again - after a change made here or arriving from the
   *  other computer. */
  invalidate(): void;
  subscribe(listener: () => void): () => void;
  getVersion(): number;
  /** For tests and the diagnostics report. */
  stats(): { events: number; images: number; missing: number; inFlight: number };
}

export function createArtworkStore(f: ArtworkFetchers, opts: ArtworkStoreOptions = {}): ArtworkStore {
  const maxImages = opts.maxImages ?? 400;
  const defer = opts.defer ?? ((fn: () => void) => queueMicrotask(fn));
  const now = opts.now ?? (() => Date.now());
  const retryMs = opts.retryMs ?? 30_000;

  let refs: Map<number, ArtworkRef> | null = null;
  let generation = 0;
  let loading: Promise<void> | null = null;

  // key@rev -> picture, in the order last shown (a Map keeps insertion order).
  const images = new Map<string, ArtworkImage>();
  const missing = new Set<string>();
  const inFlight = new Set<string>();
  const failedUntil = new Map<string, number>();
  let queued = new Map<string, string>();
  let scheduled = false;

  let version = 0;
  const listeners = new Set<() => void>();
  const changed = () => {
    version += 1;
    for (const l of [...listeners]) l();
  };
  const id = (key: string, rev: string) => `${key}@${rev}`;

  function loadMap(): Promise<void> {
    if (loading) return loading;
    const gen = generation;
    loading = f
      .map()
      .then((list) => {
        if (gen !== generation) return;
        refs = new Map(list.map((r) => [r.eventId, r]));
        changed();
      })
      .catch((e) => f.report?.("artwork map", e))
      .finally(() => {
        loading = null;
        // Invalidated while this one was out: load what is true now.
        if (gen !== generation) void loadMap();
      });
    return loading;
  }

  function trim() {
    while (images.size > maxImages) {
      const oldest = images.keys().next().value;
      if (oldest === undefined) break;
      images.delete(oldest);
    }
    if (missing.size > 4000) missing.clear();
  }

  function flush() {
    scheduled = false;
    const batch = queued;
    queued = new Map();
    if (batch.size === 0) return;
    for (const [key, rev] of batch) inFlight.add(id(key, rev));
    f.images([...batch.keys()])
      .then((list) => {
        const got = new Set<string>();
        let moved = false;
        for (const img of list) {
          const asked = batch.get(img.key);
          if (asked === undefined) continue;
          got.add(img.key);
          images.set(id(img.key, img.rev), img);
          if (asked !== img.rev) {
            // The picture changed after the map was read: the newer one is
            // shown, and the map is read again so it agrees.
            images.set(id(img.key, asked), img);
            moved = true;
          }
        }
        for (const [key, rev] of batch) {
          if (!got.has(key)) missing.add(id(key, rev));
        }
        trim();
        changed();
        if (moved) invalidate();
      })
      .catch((e) => {
        const until = now() + retryMs;
        for (const [key, rev] of batch) failedUntil.set(id(key, rev), until);
        f.report?.("artwork pictures", e);
      })
      .finally(() => {
        for (const [key, rev] of batch) inFlight.delete(id(key, rev));
      });
  }

  function request(key: string, rev: string) {
    const k = id(key, rev);
    if (inFlight.has(k) || queued.get(key) === rev) return;
    const until = failedUntil.get(k);
    if (until !== undefined) {
      if (now() < until) return;
      failedUntil.delete(k);
    }
    queued.set(key, rev);
    if (!scheduled) {
      scheduled = true;
      defer(flush);
    }
  }

  function invalidate() {
    generation += 1;
    void loadMap();
  }

  return {
    refFor(eventId) {
      if (!refs) {
        void loadMap();
        return undefined;
      }
      return refs.get(eventId) ?? null;
    },
    imageFor(ref) {
      const k = id(ref.key, ref.rev);
      const hit = images.get(k);
      if (hit) {
        // Shown again: the last to be dropped.
        images.delete(k);
        images.set(k, hit);
        return hit;
      }
      if (missing.has(k)) return null;
      request(ref.key, ref.rev);
      return undefined;
    },
    invalidate,
    subscribe(listener) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
    getVersion: () => version,
    stats: () => ({ events: refs?.size ?? 0, images: images.size, missing: missing.size, inFlight: inFlight.size }),
  };
}

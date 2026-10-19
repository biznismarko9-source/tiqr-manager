// 2.112.0 - live sync between computers: one session against Firestore.
//
// The records, the clock, newer-edit-wins and the queue of what is unsent
// all live in Rust (commands/live_sync.rs). A session only carries documents
// between that queue and Firestore, under the signed-in account's own space:
//
//   users/{uid}/rows/{table~uid}      one document per record
//   users/{uid}/parts/{id~h~n}        the body of a record too big for one
//   users/{uid}/settings/{group}      mail rules + senders, webhooks
//   users/{uid}/devices/{device}      which computers sync live
//
// On start it reads everything newer than where it got to last time, in
// pages, then keeps a listener open for whatever arrives next. Only once it
// has caught up does it send: a change waiting here since yesterday must be
// compared with what the other computer did meanwhile, not pushed over it.
// Firestore's rules (firestore.rules) refuse to overwrite a newer version,
// so even two computers sending at the same moment cannot undo each other.
//
// Everything it talks to comes in as `LiveSyncDeps` - the app's own in
// lib/liveSync.ts, and two computers against the Firestore emulator in
// scripts/live-sync-emulator-check.ts.
import {
  collection,
  doc,
  getDoc,
  getDocs,
  limit,
  onSnapshot,
  orderBy,
  query,
  runTransaction,
  serverTimestamp,
  setDoc,
  startAfter,
  Timestamp,
  where,
  writeBatch,
  type DocumentData,
  type Firestore,
  type QueryConstraint,
  type QueryDocumentSnapshot,
  type QuerySnapshot,
  type Unsubscribe,
} from "firebase/firestore";
import {
  batches,
  classify,
  fromStored,
  laterCursor,
  partId,
  replaces,
  retryDelay,
  toStored,
  type Cursor,
  type LivePhase,
  type StoredRow,
  type SyncDoc,
} from "./liveSyncCore.ts";
import type {
  LiveSyncAck,
  LiveSyncApplyOutcome,
  LiveSyncDoc,
  LiveSyncOutItem,
  LiveSyncSettingsDoc,
  LiveSyncStatus,
  LiveSyncTake,
} from "./types";

export type LiveSyncView = {
  phase: LivePhase;
  /** Changes made here, not sent yet. */
  outbox: number;
  /** Changes from the other computer waiting for a record they point at. */
  waiting: number;
  /** What went wrong, as Firebase said it. */
  detail: string | null;
  lastPullAt: string | null;
  lastPushAt: string | null;
  /** Records that could not be applied, last few. */
  reasons: string[];
};

export const EMPTY_VIEW: LiveSyncView = {
  phase: "off",
  outbox: 0,
  waiting: 0,
  detail: null,
  lastPullAt: null,
  lastPushAt: null,
  reasons: [],
};

/** The Rust side - `api.liveSync*` in the app. */
export interface LiveSyncBackend {
  liveSyncStatus(): Promise<LiveSyncStatus>;
  liveSyncTake(limit: number): Promise<LiveSyncTake>;
  liveSyncAck(acks: LiveSyncAck[], settings: { group: string; stamp: string }[]): Promise<void>;
  liveSyncApply(docs: LiveSyncDoc[]): Promise<LiveSyncApplyOutcome>;
  liveSyncApplySettings(docs: LiveSyncSettingsDoc[]): Promise<string[]>;
  liveSyncSetCursor(seconds: number, nanos: number): Promise<void>;
  liveSyncReport(error: string | null): Promise<void>;
}

export interface LiveSyncDeps {
  db: Firestore;
  backend: LiveSyncBackend;
  view: { get(): LiveSyncView; set(patch: Partial<LiveSyncView>): void };
  /** Tables whose records changed here, for the screens. */
  publish(tables: string[]): void;
  online(): boolean;
  onConnectivity(handler: (online: boolean) => void): () => void;
  platform: string;
  /** Shorter waits and pages for the emulator check; the app uses the defaults. */
  timing?: Partial<Timing>;
}

type Timing = {
  /** How often the queue is looked at when nothing asked for it sooner. */
  idleMs: number;
  /** Records read per page while catching up. */
  page: number;
  /** Records taken from the queue per round. */
  take: number;
  /** Records a rows listener may deliver before it starts afresh. */
  listenRestart: number;
  /** How long a record whose parts are not readable yet waits for them. */
  heldMs: number;
};

const DEFAULT_TIMING: Timing = { idleMs: 2_000, page: 400, take: 200, listenRestart: 3_000, heldMs: 3_000 };

export function errorCode(e: unknown): string | undefined {
  return typeof e === "object" && e !== null && "code" in e ? String((e as { code: unknown }).code) : undefined;
}

export function errorText(e: unknown): string {
  if (e instanceof Error) return e.message;
  return typeof e === "string" ? e : "Something went wrong.";
}

type StoredWithAt = StoredRow & { at?: Timestamp };

/** This computer, on or off, for the others to see (`devicesAllLive`). */
export async function announceDevice(db: Firestore, uid: string, deviceId: string, live: boolean, platform: string) {
  await setDoc(doc(db, "users", uid, "devices", deviceId), { live, lastSeen: serverTimestamp(), platform }, { merge: true });
}

/** Every other computer seen in the last 30 days syncs live, and there is at
 *  least one. */
export async function devicesAllLive(db: Firestore, uid: string, me: string, now = Date.now()): Promise<boolean> {
  const snap = await getDocs(collection(db, "users", uid, "devices"));
  const recent = now - 30 * 24 * 3600_000;
  const others = snap.docs.filter((d) => {
    if (d.id === me) return false;
    const seen = (d.data() as { lastSeen?: Timestamp }).lastSeen;
    return !seen || seen.toMillis() >= recent;
  });
  return others.length > 0 && others.every((d) => (d.data() as { live?: unknown }).live === true);
}

export class LiveSyncSession {
  readonly uid: string;
  /** Set once the session has looked at whether live sync is on at all. */
  started = false;
  private readonly deps: LiveSyncDeps;
  private readonly timing: Timing;
  private stopped = false;
  private caughtUp = false;
  private unsubs: Unsubscribe[] = [];
  private unsubRows: Unsubscribe | null = null;
  private offConnectivity: (() => void) | null = null;
  /** Records whose parts were not readable yet, with how often they were tried. */
  private held = new Map<string, { snap: QueryDocumentSnapshot<DocumentData>; tries: number }>();
  private heldTimer: ReturnType<typeof setTimeout> | null = null;
  private pushTimer: ReturnType<typeof setTimeout> | null = null;
  private retryTimer: ReturnType<typeof setTimeout> | null = null;
  private pushing = false;
  private attempt = 0;
  private cursor: Cursor | null = null;
  /** Applies one after another, never two at once. */
  private queue: Promise<void> = Promise.resolve();

  constructor(uid: string, deps: LiveSyncDeps) {
    this.uid = uid;
    this.deps = deps;
    this.timing = { ...DEFAULT_TIMING, ...deps.timing };
  }

  private setView(patch: Partial<LiveSyncView>) {
    this.deps.view.set(patch);
  }

  private rows() {
    return collection(this.deps.db, "users", this.uid, "rows");
  }

  private parts() {
    return collection(this.deps.db, "users", this.uid, "parts");
  }

  private settings() {
    return collection(this.deps.db, "users", this.uid, "settings");
  }

  /** Waits for whatever is being applied right now - for the emulator check. */
  idle(): Promise<void> {
    return this.queue;
  }

  stop() {
    this.stopped = true;
    this.unsubRows?.();
    this.unsubRows = null;
    for (const u of this.unsubs) u();
    this.unsubs = [];
    this.offConnectivity?.();
    this.offConnectivity = null;
    if (this.heldTimer) clearTimeout(this.heldTimer);
    if (this.pushTimer) clearTimeout(this.pushTimer);
    if (this.retryTimer) clearTimeout(this.retryTimer);
  }

  nudge() {
    if (this.caughtUp) this.schedulePush(300);
  }

  async start() {
    this.offConnectivity = this.deps.onConnectivity((online) => {
      if (!online) {
        this.setView({ phase: "offline" });
        return;
      }
      if (this.deps.view.get().phase === "offline") this.setView({ phase: "live", detail: null });
      this.nudge();
    });
    await this.begin();
  }

  private async begin() {
    if (this.stopped) return;
    try {
      const s = await this.deps.backend.liveSyncStatus();
      if (this.stopped) return;
      this.started = true;
      this.setView({ outbox: s.outbox, waiting: s.waiting, lastPullAt: s.lastPullAt, lastPushAt: s.lastPushAt });
      if (!s.enabled) {
        this.setView({ phase: "off", detail: null });
        // Was on once: the other computers must stop treating Drive as only
        // a backup while this one is not sending.
        if (s.seededAt) void announceDevice(this.deps.db, this.uid, s.deviceId, false, this.deps.platform).catch(() => {});
        return;
      }
      this.cursor = s.cursorS !== null ? { seconds: s.cursorS, nanoseconds: s.cursorNs ?? 0 } : null;
      this.setView({ phase: "catching-up", detail: null });
      await this.catchUp();
      if (this.stopped) return;
      this.listen();
      this.caughtUp = true;
      this.attempt = 0;
      this.setView({ phase: this.deps.online() ? "live" : "offline", detail: null });
      this.schedulePush(0);
      // Here, and live - see `devicesAllLive`.
      void announceDevice(this.deps.db, this.uid, s.deviceId, true, this.deps.platform).catch(() => {});
    } catch (e) {
      this.started = true;
      this.fail(e, () => this.restart());
    }
  }

  /** Listeners down, everything from the stored cursor again. */
  private restart() {
    this.unsubRows?.();
    this.unsubRows = null;
    for (const u of this.unsubs) u();
    this.unsubs = [];
    this.caughtUp = false;
    void this.begin();
  }

  /** Everything newer than the cursor, page by page. `>=` and not `>`: the
   *  records one batch wrote share one commit time, and a page can end in
   *  the middle of them. The ones read twice are applied as unchanged. */
  private async catchUp() {
    let last: QueryDocumentSnapshot<DocumentData> | null = null;
    for (;;) {
      const constraints: QueryConstraint[] = [
        ...(this.cursor ? [where("at", ">=", new Timestamp(this.cursor.seconds, this.cursor.nanoseconds))] : []),
        orderBy("at"),
        ...(last ? [startAfter(last)] : []),
        limit(this.timing.page),
      ];
      const snap: QuerySnapshot<DocumentData> = await getDocs(query(this.rows(), ...constraints));
      if (this.stopped) return;
      // Offline, Firestore answers from what it has in memory - nothing, at
      // start. That is not "caught up", and sending before it would be
      // sending blind.
      if (snap.metadata.fromCache) throw Object.assign(new Error("No connection to Firebase."), { code: "unavailable" });
      await this.applyRows(snap.docs);
      if (snap.docs.length < this.timing.page) return;
      last = snap.docs[snap.docs.length - 1];
    }
  }

  private listen() {
    this.listenRows();
    this.listenSettings();
  }

  /** From the cursor - or, with nothing read yet, from a few minutes ago:
   *  everything before that was just read by the catch-up, and reading it
   *  again would cost a read per record for nothing. Every record changed
   *  since stays in a listener's results, so after a few thousand it starts
   *  again from the cursor rather than carrying them all. */
  private listenRows() {
    this.unsubRows?.();
    const from = this.cursor
      ? new Timestamp(this.cursor.seconds, this.cursor.nanoseconds)
      : Timestamp.fromMillis(Date.now() - 5 * 60_000);
    let seen = 0;
    this.unsubRows = onSnapshot(
      query(this.rows(), where("at", ">=", from), orderBy("at")),
      (snap) => {
        const changed = snap
          .docChanges()
          .filter((c) => c.type !== "removed" && !c.doc.metadata.hasPendingWrites)
          .map((c) => c.doc);
        if (!changed.length) return;
        seen += changed.length;
        this.enqueue(async () => {
          await this.applyRows(changed);
          if (seen > this.timing.listenRestart && !this.stopped) this.listenRows();
        });
      },
      (e) => this.fail(e, () => this.restart()),
    );
  }

  private listenSettings() {
    this.unsubs.push(
      onSnapshot(
        this.settings(),
        (snap) => {
          const docs: LiveSyncSettingsDoc[] = [];
          for (const d of snap.docs) {
            if (d.metadata.hasPendingWrites) continue;
            const parsed = parseSettings(d.data());
            if (parsed) docs.push(parsed);
          }
          if (!docs.length) return;
          this.enqueue(async () => {
            const taken = await this.deps.backend.liveSyncApplySettings(docs);
            if (taken.length) this.deps.publish(["app_settings", ...taken.map((g) => `settings:${g}`)]);
          });
        },
        (e) => this.fail(e, () => this.restart()),
      ),
    );
  }

  private enqueue(work: () => Promise<void>) {
    this.queue = this.queue.then(work).catch((e) => {
      this.fail(e, () => this.restart());
    });
  }

  /** Firestore documents -> the other computer's changes, applied here. */
  private async applyRows(snaps: QueryDocumentSnapshot<DocumentData>[]) {
    const sorted = [...snaps].sort((a, b) => {
      const x = (a.data() as StoredWithAt).at;
      const y = (b.data() as StoredWithAt).at;
      return (x?.toMillis() ?? 0) - (y?.toMillis() ?? 0);
    });
    const docs: SyncDoc[] = [];
    let newest: Cursor | null = null;
    const unreadable: string[] = [];
    for (const s of sorted) {
      const row = s.data() as StoredWithAt;
      if (!row.at) continue;
      newest = laterCursor(newest, { seconds: row.at.seconds, nanoseconds: row.at.nanoseconds });
      const parts = row.body === undefined && row.parts ? await this.readParts(s.id, row) : undefined;
      const d = fromStored(s.id, row, parts);
      if (d) {
        docs.push(d);
        this.held.delete(s.id);
      } else if (parts) {
        // Parts are written before their record, so this is a moment's
        // race at most: tried again shortly, three times.
        const tries = (this.held.get(s.id)?.tries ?? 0) + 1;
        if (tries <= 3) this.held.set(s.id, { snap: s, tries });
        else {
          this.held.delete(s.id);
          unreadable.push(s.id.split("~")[0]);
        }
      } else unreadable.push(s.id.split("~")[0]);
    }
    const view = this.deps.view.get();
    if (unreadable.length) {
      this.setView({
        reasons: [`${unreadable.length} record(s) from the other computer could not be read (${unreadable[0]})`, ...view.reasons].slice(0, 8),
      });
    }
    if (this.held.size && !this.heldTimer) {
      this.heldTimer = setTimeout(() => {
        this.heldTimer = null;
        const again = [...this.held.values()].map((h) => h.snap);
        if (again.length && !this.stopped) this.enqueue(() => this.applyRows(again));
      }, this.timing.heldMs);
    }
    if (this.stopped) return;
    const changed = new Set<string>();
    for (const group of batches(docs, (d) => JSON.stringify(d.d).length + 200, 300, 6_000_000)) {
      const out = await this.deps.backend.liveSyncApply(group);
      for (const t of out.changedTables) changed.add(t);
      const reasons = this.deps.view.get().reasons;
      this.setView({
        waiting: out.pending,
        lastPullAt: new Date().toISOString(),
        reasons: out.reasons.length ? [...out.reasons, ...reasons].slice(0, 8) : reasons,
      });
    }
    if (changed.size) this.deps.publish([...changed]);
    if (newest) {
      this.cursor = laterCursor(this.cursor, newest);
      if (this.cursor) await this.deps.backend.liveSyncSetCursor(this.cursor.seconds, this.cursor.nanoseconds);
    }
  }

  private async readParts(id: string, row: StoredRow): Promise<(string | undefined)[]> {
    const n = row.parts ?? 0;
    const reads = Array.from({ length: n }, (_, i) => getDoc(doc(this.parts(), partId(id, row.h, i))));
    const snaps = await Promise.all(reads);
    return snaps.map((s) => (s.exists() ? String((s.data() as { p?: unknown }).p ?? "") : undefined));
  }

  private schedulePush(ms: number) {
    if (this.stopped) return;
    if (this.pushTimer) clearTimeout(this.pushTimer);
    this.pushTimer = setTimeout(() => void this.push(), ms);
  }

  /** Sends what is waiting here. Resolves when this round is done - for the
   *  emulator check, which cannot wait for the timer. */
  async push(): Promise<void> {
    if (this.stopped || !this.caughtUp) return;
    if (this.pushing) {
      this.schedulePush(this.timing.idleMs);
      return;
    }
    this.pushing = true;
    if (this.pushTimer) clearTimeout(this.pushTimer);
    let next = this.timing.idleMs;
    try {
      let sentAny = false;
      for (let round = 0; round < 50 && !this.stopped; round++) {
        const taken = await this.deps.backend.liveSyncTake(this.timing.take);
        if (!taken.items.length && !taken.settings.length) break;
        sentAny = true;
        this.setView({ phase: "sending" });
        const acks = await this.writeRows(taken.items);
        const settingsAcks = await this.writeSettings(taken.settings);
        await this.deps.backend.liveSyncAck(acks, settingsAcks);
      }
      if (sentAny || this.deps.view.get().phase !== "live") {
        const s = await this.deps.backend.liveSyncStatus();
        this.setView({
          phase: this.deps.online() ? "live" : "offline",
          detail: null,
          outbox: s.outbox,
          waiting: s.waiting,
          lastPushAt: s.lastPushAt,
        });
      }
      this.attempt = 0;
    } catch (e) {
      next = this.fail(e);
    } finally {
      this.pushing = false;
      this.schedulePush(next);
    }
  }

  /** Parts first, so whoever reads a record can always read its parts; then
   *  the records, a batch at a time. Firestore refuses a whole batch for one
   *  record it already holds a newer version of - that batch then goes one
   *  record at a time, each read first. */
  private async writeRows(items: LiveSyncOutItem[]): Promise<LiveSyncAck[]> {
    const db = this.deps.db;
    const stored = items.map((it) => ({ it, ...toStored(it.doc) }));
    const partWrites = stored.flatMap((s) => s.parts.map((p, i) => ({ id: partId(s.it.doc.id, s.it.doc.h, i), p })));
    for (const group of batches(partWrites, (w) => w.p.length * 3 + 100, 400, 8_000_000)) {
      const b = writeBatch(db);
      for (const w of group) b.set(doc(this.parts(), w.id), { p: w.p });
      await b.commit();
    }
    const acks: LiveSyncAck[] = [];
    const ackOf = (it: LiveSyncOutItem, written: boolean): LiveSyncAck => ({
      table: it.table,
      rowId: it.rowId,
      hlc: it.hlc,
      uid: it.doc.u,
      written,
    });
    for (const group of batches(stored, (s) => (s.row.body?.length ?? 0) * 3 + 300, 400, 8_000_000)) {
      try {
        const b = writeBatch(db);
        for (const s of group) b.set(doc(this.rows(), s.it.doc.id), { ...s.row, at: serverTimestamp() });
        await b.commit();
        for (const s of group) acks.push(ackOf(s.it, true));
      } catch (e) {
        if (errorCode(e) !== "permission-denied") throw e;
        for (const s of group) acks.push(ackOf(s.it, await this.writeOne(s.it.doc.id, s.row)));
      }
    }
    return acks;
  }

  /** One record, read first: written only if it replaces what is there. */
  private async writeOne(id: string, row: StoredRow): Promise<boolean> {
    const ref = doc(this.rows(), id);
    let read = false;
    try {
      return await runTransaction(this.deps.db, async (tx) => {
        const cur = await tx.get(ref);
        read = true;
        const have = cur.exists() ? (cur.data() as StoredRow) : null;
        if (have && !replaces(row, have)) return false;
        tx.set(ref, { ...row, at: serverTimestamp() });
        return true;
      });
    } catch (e) {
      // Reading worked and writing did not: the rules refused this one
      // record. Said, and stepped over - one bad record must not hold back
      // every change made after it.
      if (read && errorCode(e) === "permission-denied") {
        const reasons = this.deps.view.get().reasons;
        this.setView({ reasons: [`Firebase refused ${id.split("~")[0]}: ${errorText(e)}`, ...reasons].slice(0, 8) });
        return false;
      }
      throw e;
    }
  }

  private async writeSettings(docs: LiveSyncSettingsDoc[]): Promise<{ group: string; stamp: string }[]> {
    const out: { group: string; stamp: string }[] = [];
    for (const s of docs) {
      const ref = doc(this.settings(), s.group);
      await runTransaction(this.deps.db, async (tx) => {
        const cur = await tx.get(ref);
        const have = cur.exists() ? (cur.data() as { stamp?: unknown }) : null;
        if (typeof have?.stamp === "string" && have.stamp >= s.stamp) return;
        tx.set(ref, {
          group: s.group,
          stamp: s.stamp,
          dev: s.dev,
          body: JSON.stringify({ keys: s.keys, tables: s.tables }),
          at: serverTimestamp(),
        });
      });
      out.push({ group: s.group, stamp: s.stamp });
    }
    return out;
  }

  /** Says what went wrong, and when to try again (returned, and scheduled
   *  when `retry` is given). */
  private fail(e: unknown, retry?: () => void): number {
    if (this.stopped) return this.timing.idleMs;
    this.attempt += 1;
    const trouble = classify(errorCode(e));
    const wait = retryDelay(trouble, this.attempt);
    this.setView({ phase: trouble, detail: errorText(e) });
    void this.deps.backend.liveSyncReport(errorText(e)).catch(() => {});
    if (retry) {
      if (this.retryTimer) clearTimeout(this.retryTimer);
      this.retryTimer = setTimeout(() => {
        if (!this.stopped) retry();
      }, wait);
    }
    return wait;
  }
}

export function parseSettings(data: DocumentData): LiveSyncSettingsDoc | null {
  if (typeof data.group !== "string" || typeof data.stamp !== "string" || typeof data.body !== "string") return null;
  try {
    const body = JSON.parse(data.body) as { keys?: LiveSyncSettingsDoc["keys"]; tables?: LiveSyncSettingsDoc["tables"] };
    return {
      group: data.group,
      stamp: data.stamp,
      dev: typeof data.dev === "string" ? data.dev : "",
      keys: body.keys ?? {},
      tables: body.tables ?? {},
    };
  } catch {
    return null;
  }
}

// 2.112.0 - live sync, two computers and the REAL rules, against the
// Firestore emulator. Not part of `npm test`: it needs the emulator running.
//
//   npx firebase-tools@15 emulators:exec --config firebase.emulator.json \
//     --only firestore,auth --project demo-tiqr "node scripts/live-sync-emulator-check.ts"
//
// from the repository root (Java 11+ for the emulator, Node 22.18+ for the
// TypeScript). firebase.emulator.json points the emulator at the repository's
// own firestore.rules, so what is checked is exactly what marko pastes into
// the console - and it is only ever an emulator config: nothing deploys it.
//
// Each computer is lib/liveSyncSession.ts - the code the app runs - with a
// small stand-in for the Rust side: records, an outbox, newer-edit-wins.
// The Rust side itself is tested on its own (commands/live_sync.rs, two
// computers and a fake Firestore with the same rule).
import assert from "node:assert/strict";
import { deleteApp, initializeApp, type FirebaseApp } from "firebase/app";
import { connectAuthEmulator, createUserWithEmailAndPassword, getAuth, signInWithEmailAndPassword } from "firebase/auth";
import {
  collection,
  connectFirestoreEmulator,
  deleteDoc,
  doc,
  getDoc,
  getDocs,
  getFirestore,
  setDoc,
  Timestamp,
  type Firestore,
} from "firebase/firestore";
import { replaces } from "../src/lib/liveSyncCore.ts";
import {
  devicesAllLive,
  EMPTY_VIEW,
  LiveSyncSession,
  announceDevice,
  type LiveSyncBackend,
  type LiveSyncView,
} from "../src/lib/liveSyncSession.ts";
import type {
  LiveSyncAck,
  LiveSyncApplyOutcome,
  LiveSyncDoc,
  LiveSyncSettingsDoc,
  LiveSyncStatus,
  LiveSyncTake,
} from "../src/lib/types.ts";

type Row = { t: string; u: string; d: Record<string, unknown> };

/** What commands/live_sync.rs keeps, in miniature. */
class Computer implements LiveSyncBackend {
  readonly dev: string;
  hlc = 0;
  rows = new Map<string, Row>();
  dead = new Set<string>();
  outbox = new Map<string, { hlc: number; del: boolean; rowId: number }>();
  meta = new Map<string, { h: number; dev: string }>();
  settings = new Map<string, LiveSyncSettingsDoc>();
  pushedSettings = new Map<string, string>();
  cursor: { s: number; ns: number } | null = null;
  applied = 0;
  enabled = true;
  private ids = new Map<string, number>();

  constructor(dev: string) {
    this.dev = dev;
  }

  private rowId(id: string) {
    if (!this.ids.has(id)) this.ids.set(id, this.ids.size + 1);
    return this.ids.get(id)!;
  }

  private tick() {
    this.hlc = Math.max(Date.now(), this.hlc + 1);
    return this.hlc;
  }

  edit(t: string, u: string, d: Record<string, unknown>, later = 0) {
    if (later) this.hlc = Math.max(Date.now(), this.hlc) + later;
    const id = `${t}~${u}`;
    this.rows.set(id, { t, u, d });
    this.outbox.set(id, { hlc: this.tick(), del: false, rowId: this.rowId(id) });
  }

  remove(t: string, u: string) {
    const id = `${t}~${u}`;
    this.rows.delete(id);
    this.dead.add(id);
    this.outbox.set(id, { hlc: this.tick(), del: true, rowId: this.rowId(id) });
  }

  setGroup(group: string, keys: Record<string, string | null>) {
    this.settings.set(group, { group, stamp: new Date().toISOString(), keys, tables: {}, dev: this.dev });
  }

  async liveSyncStatus(): Promise<LiveSyncStatus> {
    return {
      enabled: this.enabled,
      decided: true,
      deviceId: this.dev,
      cursorS: this.cursor?.s ?? null,
      cursorNs: this.cursor?.ns ?? null,
      outbox: this.outbox.size,
      waiting: 0,
      seededAt: "x",
      lastPushAt: null,
      lastPullAt: null,
      lastError: null,
      lastErrorAt: null,
      pausedAfterRestore: false,
    };
  }

  async liveSyncTake(limit: number): Promise<LiveSyncTake> {
    const items = [...this.outbox.entries()].slice(0, limit).map(([id, e]) => {
      const [t, u] = id.split("~");
      const doc: LiveSyncDoc = { id, t, u, h: e.hlc, del: e.del, d: e.del ? {} : this.rows.get(id)!.d, dev: this.dev };
      return { table: t, rowId: e.rowId, hlc: e.hlc, doc };
    });
    const settings = [...this.settings.values()].filter((s) => (this.pushedSettings.get(s.group) ?? "") < s.stamp);
    return { items, settings };
  }

  async liveSyncAck(acks: LiveSyncAck[], settings: { group: string; stamp: string }[]) {
    for (const a of acks) {
      const id = `${a.table}~${a.uid}`;
      if (this.outbox.get(id)?.hlc === a.hlc) this.outbox.delete(id);
      if (a.written) this.meta.set(id, { h: a.hlc, dev: this.dev });
    }
    for (const s of settings) this.pushedSettings.set(s.group, s.stamp);
  }

  async liveSyncApply(docs: LiveSyncDoc[]): Promise<LiveSyncApplyOutcome> {
    const out: LiveSyncApplyOutcome = { applied: 0, unchanged: 0, older: 0, deleted: 0, pending: 0, conflicts: 0, changedTables: [], reasons: [] };
    for (const d of docs) {
      this.hlc = Math.max(this.hlc, d.h);
      if (d.del) {
        if (this.rows.delete(d.id)) out.deleted++;
        this.dead.add(d.id);
        this.outbox.delete(d.id);
        this.meta.set(d.id, { h: d.h, dev: d.dev });
        continue;
      }
      if (this.dead.has(d.id)) {
        out.older++;
        continue;
      }
      const meta = this.meta.get(d.id);
      const queued = this.outbox.get(d.id);
      const mine = Math.max(meta?.h ?? -1, queued?.hlc ?? -1);
      const myDev = queued && queued.hlc >= (meta?.h ?? -1) ? this.dev : meta?.dev ?? "";
      if (d.h < mine || (d.h === mine && d.dev <= myDev)) {
        out.older++;
        continue;
      }
      this.rows.set(d.id, { t: d.t, u: d.u, d: d.d });
      this.meta.set(d.id, { h: d.h, dev: d.dev });
      this.outbox.delete(d.id);
      this.applied++;
      out.applied++;
      if (!out.changedTables.includes(d.t)) out.changedTables.push(d.t);
    }
    return out;
  }

  async liveSyncApplySettings(docs: LiveSyncSettingsDoc[]): Promise<string[]> {
    const taken: string[] = [];
    for (const s of docs) {
      const ours = this.settings.get(s.group);
      if (ours && ours.stamp >= s.stamp) continue;
      this.settings.set(s.group, s);
      this.pushedSettings.set(s.group, s.stamp);
      taken.push(s.group);
    }
    return taken;
  }

  async liveSyncSetCursor(seconds: number, nanos: number) {
    this.cursor = { s: seconds, ns: nanos };
  }

  async liveSyncReport(_error: string | null) {}
}

const config = { apiKey: "demo-key", authDomain: "demo-tiqr.firebaseapp.com", projectId: "demo-tiqr" };
const apps: FirebaseApp[] = [];

async function firebaseFor(name: string, email: string, create: boolean): Promise<{ db: Firestore; uid: string }> {
  const app = initializeApp(config, name);
  apps.push(app);
  const auth = getAuth(app);
  connectAuthEmulator(auth, "http://127.0.0.1:9099", { disableWarnings: true });
  const db = getFirestore(app);
  connectFirestoreEmulator(db, "127.0.0.1", 8085);
  const cred = create
    ? await createUserWithEmailAndPassword(auth, email, "secret-123")
    : await signInWithEmailAndPassword(auth, email, "secret-123");
  return { db, uid: cred.user.uid };
}

function start(name: string, db: Firestore, uid: string, c: Computer) {
  let view: LiveSyncView = EMPTY_VIEW;
  const session = new LiveSyncSession(uid, {
    db,
    backend: c,
    view: { get: () => view, set: (p) => (view = { ...view, ...p }) },
    publish: () => {},
    online: () => true,
    onConnectivity: () => () => {},
    platform: name,
    timing: { idleMs: 200, page: 3, heldMs: 200 },
  });
  void session.start();
  return { session, view: () => view };
}

async function until(what: string, ok: () => boolean, ms = 15_000) {
  const end = Date.now() + ms;
  while (!ok()) {
    if (Date.now() > end) throw new Error(`timed out waiting for: ${what}`);
    await new Promise((r) => setTimeout(r, 50));
  }
}

async function denied(what: string, p: Promise<unknown>) {
  try {
    await p;
  } catch (e) {
    assert.equal((e as { code?: string }).code, "permission-denied", what);
    return;
  }
  throw new Error(`${what}: allowed, should have been refused`);
}

const results: string[] = [];
async function step(name: string, body: () => Promise<void>) {
  await body();
  results.push(`ok - ${name}`);
  console.log(`ok - ${name}`);
}

async function main() {
  const marko = await firebaseFor("a", "marko@example.com", true);
  const second = await firebaseFor("b", "marko@example.com", false);
  const stranger = await firebaseFor("c", "someone@example.com", true);
  assert.equal(marko.uid, second.uid);

  const a = new Computer("aaaa");
  const b = new Computer("bbbb");
  const A = start("A", marko.db, marko.uid, a);
  let B = start("B", second.db, second.uid, b);
  await until("both live", () => A.view().phase === "live" && B.view().phase === "live");

  await step("records made on one computer arrive on the other, a picture in parts included", async () => {
    a.edit("orders", "o1", { code: "ORD-1", notes: "first" });
    for (let i = 2; i <= 8; i++) a.edit("tickets", `t${i}`, { code: `TKT-${i}`, order_id: { $ref: "o1" } });
    const picture = "data:image/png;base64," + "Q".repeat(1_500_000);
    a.edit("note_images", "img1", { data_uri: picture });
    await A.session.push();
    await until("B has all nine", () => b.rows.size === 9);
    assert.equal(b.rows.get("note_images~img1")!.d.data_uri, picture);
    const parts = await getDocs(collection(marko.db, "users", marko.uid, "parts"));
    assert.ok(parts.size >= 2, "the picture went in parts");
    assert.ok(b.cursor, "B remembers where it got to");
  });

  await step("a later edit on the other computer comes back", async () => {
    b.edit("orders", "o1", { code: "ORD-1", notes: "changed on B" }, 1_000);
    await B.session.push();
    await until("A has B's edit", () => a.rows.get("orders~o1")!.d.notes === "changed on B");
  });

  await step("Firestore refuses an older version over a newer one, and the batch still goes", async () => {
    const before = (await getDoc(doc(marko.db, "users", marko.uid, "rows", "orders~o1"))).data()!;
    // A change of A's stamped before B's edit, as if A had been offline.
    a.rows.set("orders~o1", { t: "orders", u: "o1", d: { code: "ORD-1", notes: "stale from A" } });
    a.outbox.set("orders~o1", { hlc: before.h - 5_000, del: false, rowId: 1 });
    a.edit("orders", "o2", { code: "ORD-2" });
    await A.session.push();
    const after = (await getDoc(doc(marko.db, "users", marko.uid, "rows", "orders~o1"))).data()!;
    assert.equal(after.h, before.h, "the newer version stayed");
    assert.equal(JSON.parse(after.body).notes, "changed on B");
    assert.equal(a.outbox.size, 0, "both acknowledged - the stale one as not written");
    await until("B has o2", () => b.rows.has("orders~o2"));
  });

  await step("a deletion wins over a later edit that reached Firestore first", async () => {
    b.edit("tickets", "t2", { code: "TKT-2", notes: "edited on B" }, 60_000);
    await B.session.push();
    a.remove("tickets", "t2");
    await A.session.push();
    const stored = (await getDoc(doc(marko.db, "users", marko.uid, "rows", "tickets~t2"))).data()!;
    assert.equal(stored.del, true);
    await until("B deleted it", () => !b.rows.has("tickets~t2"));
    // And no edit brings it back.
    b.dead.delete("tickets~t2");
    b.edit("tickets", "t2", { code: "TKT-2", notes: "resurrected?" }, 120_000);
    await B.session.push();
    const still = (await getDoc(doc(marko.db, "users", marko.uid, "rows", "tickets~t2"))).data()!;
    assert.equal(still.del, true, "a deleted record stays deleted in Firestore");
  });

  await step("shared settings travel, newest stamp wins", async () => {
    a.setGroup("mail", { mail_scope: "important" });
    await A.session.push();
    await until("B has the mail group", () => b.settings.get("mail")?.keys.mail_scope === "important");
  });

  await step("a restarted computer reads only what is new since it stopped", async () => {
    B.session.stop();
    const before = b.applied;
    a.edit("orders", "o3", { code: "ORD-3" });
    await A.session.push();
    B = start("B", second.db, second.uid, b);
    await until("B caught up", () => b.rows.has("orders~o3"));
    await B.session.idle();
    assert.ok(b.applied - before <= 2, `re-applied ${b.applied - before}, not everything again`);
  });

  await step("both computers live: Drive is only the backup", async () => {
    await until("A announced", async () => true);
    await new Promise((r) => setTimeout(r, 300));
    assert.equal(await devicesAllLive(marko.db, marko.uid, "aaaa"), true);
    await announceDevice(marko.db, marko.uid, "aaaa", false, "A");
    assert.equal(await devicesAllLive(second.db, second.uid, "bbbb"), false, "A turned it off");
    await announceDevice(marko.db, marko.uid, "aaaa", true, "A");
  });

  await step("another account can neither read nor write marko's records", async () => {
    await denied("read rows", getDocs(collection(stranger.db, "users", marko.uid, "rows")));
    await denied("read settings", getDocs(collection(stranger.db, "users", marko.uid, "settings")));
    await denied("read devices", getDocs(collection(stranger.db, "users", marko.uid, "devices")));
    await denied(
      "write a row",
      setDoc(doc(stranger.db, "users", marko.uid, "rows", "orders~x"), { t: "orders", u: "x", h: 1, del: false, dev: "z", body: "{}", at: Timestamp.now() }),
    );
  });

  await step("the rules refuse malformed records and deletes", async () => {
    const rows = (id: string) => doc(marko.db, "users", marko.uid, "rows", id);
    await denied("client clock instead of the server's", setDoc(rows("orders~bad1"), { t: "orders", u: "bad1", h: 1, del: false, dev: "a", body: "{}", at: Timestamp.now() }));
    const { serverTimestamp } = await import("firebase/firestore");
    await denied("id not matching table~uid", setDoc(rows("orders~bad2"), { t: "tickets", u: "bad2", h: 1, del: false, dev: "a", body: "{}", at: serverTimestamp() }));
    await denied("a field that is not ours", setDoc(rows("orders~bad3"), { t: "orders", u: "bad3", h: 1, del: false, dev: "a", body: "{}", extra: 1, at: serverTimestamp() }));
    await denied("deleting a record document", deleteDoc(rows("orders~o1")));
    assert.equal(replaces({ h: 1, del: true, dev: "a" }, { h: 9, del: false, dev: "b" }), true);
  });

  A.session.stop();
  B.session.stop();
  console.log(`\n${results.length} checks passed against the emulator with firestore.rules`);
}

main()
  .then(async () => {
    await Promise.all(apps.map((a) => deleteApp(a)));
    process.exit(0);
  })
  .catch((e) => {
    console.error("FAILED:", e);
    process.exit(1);
  });

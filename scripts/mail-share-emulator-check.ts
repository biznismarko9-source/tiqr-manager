// 2.115.0 - two computers, one Gmail reader, and the REAL rules, against the
// Firestore emulator. Not part of `npm test`: it needs the emulator running.
//
//   npx firebase-tools@15 emulators:exec --config firebase.emulator.json \
//     --only firestore,auth --project demo-tiqr "node scripts/mail-share-emulator-check.ts"
//
// from the repository root (Java 11+ for the emulator, Node 22.18+ for the
// TypeScript), like scripts/live-sync-emulator-check.ts.
//
// Each computer is lib/mailShareSession.ts - the code the app runs - with a
// small stand-in for the Rust side: a mailbox, the export in (labels_at, id)
// order, an import that keeps the newer labels, a cursor per sender, and the
// switch that stops its own Gmail reading. The Rust side itself is tested on
// its own (commands/mail_share.rs).
import assert from "node:assert/strict";
import { deleteApp, initializeApp, type FirebaseApp } from "firebase/app";
import { connectAuthEmulator, createUserWithEmailAndPassword, getAuth, signInWithEmailAndPassword } from "firebase/auth";
import {
  collection,
  connectFirestoreEmulator,
  doc,
  documentId,
  getDoc,
  getDocs,
  getFirestore,
  query,
  serverTimestamp,
  setDoc,
  Timestamp,
  where,
  type Firestore,
} from "firebase/firestore";
import { cursorId, dropId, EMPTY_SHARE_VIEW, leaseId, type MailShareView } from "../src/lib/mailShareCore.ts";
import { MailShareSession, type MailShareBackend } from "../src/lib/mailShareSession.ts";
import type { ShareBatch, ShareCursor, ShareImported, ShareInfo, ShareProgress, SharedMail } from "../src/lib/types.ts";

const after = (a: ShareCursor, b: ShareCursor) => a.at > b.at || (a.at === b.at && a.id > b.id);

/** What commands/mail_share.rs and the reader do, in miniature. */
class Computer implements MailShareBackend {
  readonly dev: string;
  readonly account: string;
  enabled = true;
  paused = false;
  /** Its own Gmail reading stands by - `mail_share_follow`. */
  following = false;
  rows = new Map<string, { row: number; mail: SharedMail }>();
  cursors = new Map<string, ShareProgress>();
  imports = 0;
  wakes = 0;
  readHandler: (() => void) | null = null;
  private nextRow = 1;

  constructor(dev: string, account: string) {
    this.dev = dev;
    this.account = account;
  }

  /** A message read from Gmail here - or a new view of its labels. */
  read(id: string, labels = "inbox") {
    const mail: SharedMail = {
      id,
      thread: `t${id}`,
      fromRaw: `Shop <news@shop.com>`,
      fromAddress: "news@shop.com",
      fromName: "Shop",
      to: this.account,
      replyTo: null,
      subject: `Message ${id} ${"é".repeat(200)}`,
      snippet: `about ${id}`,
      receivedAt: "2026-10-01T10:00:00+00:00",
      labels,
      unread: labels.includes("unread"),
      starred: labels.includes("starred"),
      labelsAt: new Date().toISOString(),
      text: null,
    };
    const had = this.rows.get(id);
    this.rows.set(id, { row: had?.row ?? this.nextRow++, mail });
  }

  /** The reader stored something: the window hears of it. */
  mailRead() {
    this.readHandler?.();
  }

  labels(id: string) {
    return this.rows.get(id)?.mail.labels;
  }

  async mailShareInfo(): Promise<ShareInfo> {
    return { enabled: this.enabled, connected: true, account: this.account, device: this.dev, paused: this.paused };
  }

  async mailShareExport(from: ShareCursor, maxBytes = 600_000): Promise<ShareBatch> {
    const ordered = [...this.rows.values()]
      .map((r) => ({ c: { at: r.mail.labelsAt, id: r.row }, mail: r.mail }))
      .filter((r) => after(r.c, from))
      .sort((a, b) => (after(a.c, b.c) ? 1 : -1));
    const rows: SharedMail[] = [];
    let upTo = from;
    let bytes = 0;
    for (const r of ordered) {
      const size = JSON.stringify(r.mail).length;
      if (rows.length > 0 && bytes + size > maxBytes) return { account: this.account, rows, upTo, more: true };
      bytes += size;
      rows.push(r.mail);
      upTo = r.c;
    }
    return { account: this.account, rows, upTo, more: false };
  }

  async mailShareImport(sender: string, account: string, rows: SharedMail[], upTo: ShareCursor): Promise<ShareImported> {
    if (account !== this.account) throw new Error("That mail is from another mailbox than the one connected here.");
    this.imports++;
    let storedNew = 0;
    let updated = 0;
    for (const m of rows) {
      const had = this.rows.get(m.id);
      if (had) {
        if (m.labelsAt <= had.mail.labelsAt) continue;
        had.mail = { ...had.mail, labels: m.labels, unread: m.unread, starred: m.starred, labelsAt: m.labelsAt };
        updated++;
        continue;
      }
      this.rows.set(m.id, { row: this.nextRow++, mail: m });
      storedNew++;
    }
    const received = (this.cursors.get(sender)?.received ?? 0) + rows.length;
    this.cursors.set(sender, { upTo, received });
    return { storedNew, updated, received };
  }

  async mailShareCursor(sender: string): Promise<ShareProgress> {
    return this.cursors.get(sender) ?? { upTo: { at: "", id: 0 }, received: 0 };
  }

  async mailShareFollow(on: boolean) {
    this.following = on;
  }
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

const TIMING = { tickMs: 300, staleMs: 1_500, abandonedMs: 15 * 60_000, maxBytes: 4_000 };

function start(db: Firestore, uid: string, c: Computer, platform: string, skewMs = 0) {
  let view: MailShareView = EMPTY_SHARE_VIEW;
  const session = new MailShareSession(uid, {
    db,
    backend: c,
    liveOn: async () => true,
    platform,
    view: { get: () => view, set: (p) => (view = { ...view, ...p }) },
    onMailRead: (handler) => {
      c.readHandler = handler;
      return () => {
        c.readHandler = null;
      };
    },
    wakeReader: () => {
      c.wakes++;
    },
    timing: TIMING,
    now: () => Date.now() + skewMs,
  });
  session.start();
  return { session, view: () => view };
}

async function until(what: string, ok: () => boolean, ms = 20_000) {
  const end = Date.now() + ms;
  while (!ok()) {
    if (Date.now() > end) throw new Error(`timed out waiting for: ${what}`);
    await new Promise((r) => setTimeout(r, 50));
  }
}

const pause = (ms: number) => new Promise((r) => setTimeout(r, ms));

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

const dev = (c: string) => c.repeat(32);

async function main() {
  const marko = await firebaseFor("a", "marko@example.com", true);
  const second = await firebaseFor("b", "marko@example.com", false);
  const stranger = await firebaseFor("c", "someone@example.com", true);
  assert.equal(marko.uid, second.uid);
  const uid = marko.uid;
  const mailDoc = (db: Firestore, id: string) => doc(db, "users", uid, "mail", id);
  const dropsLeft = async () =>
    (await getDocs(query(collection(marko.db, "users", uid, "mail"), where(documentId(), ">=", "drop~"), where(documentId(), "<", "drop~"))))
      .size;

  const MAILBOX = "marko@gmail.com";
  const mac = new Computer(dev("a"), MAILBOX);
  const win = new Computer(dev("b"), MAILBOX);
  for (let i = 0; i < 60; i++) mac.read(`m${String(i).padStart(3, "0")}`, i % 3 ? "inbox" : "inbox,unread");
  for (let i = 50; i < 55; i++) win.read(`m${String(i).padStart(3, "0")}`);

  let M = start(marko.db, uid, mac, "MacIntel");
  await until("the Mac reads", () => M.session.current() === "reader");
  let W = start(second.db, uid, win, "Win32");

  await step("while both run, one reads Gmail and the other stands by", async () => {
    await until("Windows follows", () => W.session.current() === "follower" && win.following);
    assert.equal(mac.following, false, "the reader reads");
    assert.equal(W.view().peer, "Mac");
    await until("the Mac sees Windows", () => M.view().peer === "Windows computer");
  });

  await step("everything the reader has arrives, in batches, and no batch is left in Firestore", async () => {
    await until("Windows has all 60", () => win.rows.size === 60);
    await W.session.idle();
    assert.ok(win.imports > 3, `${win.imports} batches for a small budget`);
    assert.equal(win.labels("m000"), "inbox,unread");
    await pause(400);
    assert.equal(await dropsLeft(), 0);
    assert.equal(W.view().received, 60);
  });

  await step("new mail and a label change on the reader arrive at once", async () => {
    mac.read("m900", "inbox,unread");
    mac.read("m001", "inbox,starred");
    mac.mailRead();
    await until("Windows has the new one and the star", () => win.rows.has("m900") && win.labels("m001") === "inbox,starred");
  });

  await step("Sync pressed on the follower asks the reader to look at Gmail now", async () => {
    const before = mac.wakes;
    assert.equal(W.session.requestSync(), true);
    await until("the Mac was asked", () => mac.wakes > before);
    assert.equal(M.session.requestSync(), false, "the reader reads itself");
  });

  await step("the reader closes: the lease goes back at once and the other computer reads Gmail itself", async () => {
    await M.session.stop();
    const lease = await getDoc(mailDoc(marko.db, leaseId(MAILBOX)));
    assert.equal(lease.exists(), false, "handed back");
    await until("Windows reads", () => W.session.current() === "reader" && !win.following, 5_000);
  });

  await step("the Mac comes back: it follows Windows and gets only what it lacks", async () => {
    win.read("w100");
    M = start(marko.db, uid, mac, "MacIntel");
    await until("the Mac follows", () => M.session.current() === "follower" && mac.following);
    win.mailRead();
    await until("the Mac has Windows' new message", () => mac.rows.has("w100"));
    assert.equal(mac.rows.size, 62);
  });

  await step("a batch from another mailbox is refused, said, and still deleted", async () => {
    await setDoc(mailDoc(second.db, dropId(mac.dev, Date.now(), 7)), {
      dev: win.dev,
      at: serverTimestamp(),
      p: JSON.stringify({ account: "someone.else@gmail.com", rows: [], upTo: { at: "z", id: 1 } }),
    });
    await until("refused", () => (M.view().detail ?? "").includes("another mailbox"));
    await pause(400);
    assert.equal(await dropsLeft(), 0);
    assert.equal(mac.rows.size, 62);
  });

  await step("a paused computer steps out: it neither reads for the other one nor follows", async () => {
    mac.paused = true;
    void M.session.tick();
    await until("the Mac steps out", () => M.session.current() === "solo" && !mac.following);
    mac.paused = false;
    void M.session.tick();
    await until("back to following", () => M.session.current() === "follower" && mac.following);
  });

  M.session.stop();
  W.session.stop();
  await pause(300);

  // A reader that disappears without handing the lease back - a crash, a
  // laptop lid closed - and a computer whose clock is ten minutes fast.
  const LOST = "lost@gmail.com";
  const ghost = dev("c");
  await setDoc(mailDoc(marko.db, leaseId(LOST)), { dev: ghost, at: serverTimestamp(), p: JSON.stringify({ account: LOST, platform: "Win32" }) });
  const fast = new Computer(dev("d"), LOST);
  const F = start(second.db, uid, fast, "MacIntel", 10 * 60_000);

  await step("a clock ten minutes fast does not take a lease at first sight", async () => {
    await until("it follows", () => F.session.current() === "follower" && fast.following);
    await pause(TIMING.tickMs * 2);
    assert.equal(F.session.current(), "follower");
  });

  await step("a reader that just disappears loses the lease once it stayed the same for the stale time", async () => {
    await until("it reads Gmail itself", () => F.session.current() === "reader" && !fast.following, 10_000);
    const lease = (await getDoc(mailDoc(marko.db, leaseId(LOST)))).data()!;
    assert.equal(lease.dev, fast.dev);
  });

  await step("a reader whose clock is fast still sees its follower and sends to it", async () => {
    const slow = new Computer(dev("e"), LOST);
    for (let i = 0; i < 5; i++) fast.read(`f${i}`);
    const S = start(marko.db, uid, slow, "Win32");
    await until("it follows the fast one", () => S.session.current() === "follower");
    fast.mailRead();
    await until("the mail arrives", () => slow.rows.size === 5);
    S.session.stop();
  });
  F.session.stop();

  await step("the rules refuse what is not a lease, a cursor or a batch", async () => {
    const at = serverTimestamp();
    const lease = mailDoc(marko.db, leaseId("rules@gmail.com"));
    const p = JSON.stringify({ account: "rules@gmail.com", platform: "Win32" });
    await setDoc(lease, { dev: dev("a"), at, p });
    await denied("client clock instead of the server's", setDoc(lease, { dev: dev("a"), at: Timestamp.now(), p }));
    await denied("a field that is not ours", setDoc(lease, { dev: dev("a"), at, p, extra: 1 }));
    await denied("not a computer's id", setDoc(lease, { dev: "Mac", at, p }));
    await denied("an id of no known shape", setDoc(mailDoc(marko.db, "inbox"), { dev: dev("a"), at, p }));
    await denied(
      "a cursor written for another computer",
      setDoc(mailDoc(marko.db, cursorId(dev("b"), dev("a"))), { dev: dev("a"), at, p: "{}" }),
    );
    await denied(
      "a batch over the size Firestore takes",
      setDoc(mailDoc(marko.db, dropId(dev("b"), Date.now(), 1)), { dev: dev("a"), at, p: "x".repeat(900_001) }),
    );
    await denied("a big lease", setDoc(lease, { dev: dev("a"), at, p: "x".repeat(2_001) }));
  });

  await step("another account can neither read nor write marko's mail", async () => {
    await denied("read", getDocs(collection(stranger.db, "users", uid, "mail")));
    await denied(
      "write",
      setDoc(doc(stranger.db, "users", uid, "mail", leaseId(MAILBOX)), { dev: dev("f"), at: serverTimestamp(), p: "{}" }),
    );
  });

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

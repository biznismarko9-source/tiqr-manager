// 2.115.0 - two computers, one Gmail reader: the session against Firestore.
//
// Every TICK_MS each computer looks at the mailbox's lease
// (users/{uid}/mail/lease~{mailbox}, in a transaction):
//
//   * free, its own, or not renewed lately - it takes or renews it and READS: its reader
//     reads Gmail as before, and every computer that follows it gets what it
//     stored, one batch at a time (drop~{follower}~{seq}). The next batch
//     goes when the follower's cursor says the last one is stored - so a
//     slow or sleeping computer is never buried, and nothing piles up in
//     Firestore.
//   * held by a computer that renewed it lately - it FOLLOWS: its reader
//     stands by (commands/mail_share.rs, `mail_share_follow`), it stores each
//     batch as it arrives, moves its cursor and deletes the batch.
//
// A reader that closes hands the lease back at once; one that just
// disappears loses it once the follower has seen it unchanged for
// LEASE_STALE_MS, and the follower reads Gmail itself from then on. Every
// "lately" is measured by this computer's own clock against what it saw
// change, never against the other computer's clock. Whatever goes wrong
// with Firebase, this computer reads Gmail itself - it never stands by on a
// lease nobody renews.
//
// Batches addressed to this computer are stored whatever it is doing, so
// none is left in Firestore after the roles change.
//
// Everything it talks to comes in as `MailShareDeps` - the app's own in
// lib/mailShare.ts, and two computers against the Firestore emulator in
// scripts/mail-share-emulator-check.ts.
import {
  collection,
  deleteDoc,
  doc,
  documentId,
  onSnapshot,
  query,
  runTransaction,
  serverTimestamp,
  setDoc,
  where,
  type DocumentData,
  type Firestore,
  type QueryDocumentSnapshot,
  type Unsubscribe,
} from "firebase/firestore";
import {
  cursorId,
  dropId,
  dropPrefix,
  leaseId,
  parseCursorDoc,
  parseDrop,
  parseLease,
  platformName,
  presentUntil,
  seeLease,
  takesLease,
  LEASE_ABANDONED_MS,
  LEASE_STALE_MS,
  TICK_MS,
  type CursorDoc,
  type LeaseSeen,
  type MailShareView,
  type ShareRole,
} from "./mailShareCore.ts";
import type { ShareBatch, ShareCursor, ShareImported, ShareInfo, ShareProgress, SharedMail } from "./types";

/** The Rust side - `api.mailShare*` in the app. */
export interface MailShareBackend {
  mailShareInfo(): Promise<ShareInfo>;
  mailShareExport(after: ShareCursor, maxBytes?: number): Promise<ShareBatch>;
  mailShareImport(sender: string, account: string, rows: SharedMail[], upTo: ShareCursor): Promise<ShareImported>;
  mailShareCursor(sender: string): Promise<ShareProgress>;
  mailShareFollow(on: boolean): Promise<void>;
}

export interface MailShareDeps {
  db: Firestore;
  backend: MailShareBackend;
  /** Live sync runs on this computer - the sharing goes through its Firestore space. */
  liveOn(): Promise<boolean>;
  platform: string;
  view: { get(): MailShareView; set(patch: Partial<MailShareView>): void };
  /** Mail this computer just read from Gmail: sent on at once, not at the next tick. */
  onMailRead?(handler: () => void): () => void;
  /** Sync was pressed on the computer this one reads for: look at Gmail now. */
  wakeReader?(): void;
  timing?: Partial<Timing>;
  now?: () => number;
}

type Timing = { tickMs: number; staleMs: number; abandonedMs: number; maxBytes: number };
const DEFAULT_TIMING: Timing = { tickMs: TICK_MS, staleMs: LEASE_STALE_MS, abandonedMs: LEASE_ABANDONED_MS, maxBytes: 600_000 };

const sameCursor = (a: ShareCursor, b: ShareCursor) => a.at === b.at && a.id === b.id;

const PROBLEM_SHOWN_MS = 10 * 60_000;

function code(e: unknown): string {
  return typeof e === "object" && e !== null && "code" in e ? String((e as { code: unknown }).code) : "";
}

/** Why Firebase would not do it, for Mail -> Settings. */
function trouble(e: unknown): string {
  switch (code(e)) {
    case "permission-denied":
      return "Firebase refused - publish the new firestore.rules (AKO-ZACAT.txt). Until then this computer reads Gmail itself.";
    case "resource-exhausted":
      return "Firebase's daily free limit is used up, so this computer reads Gmail itself until it resets.";
    default:
      return "Firebase could not be reached, so this computer reads Gmail itself for now.";
  }
}

export class MailShareSession {
  readonly uid: string;
  private readonly deps: MailShareDeps;
  private readonly timing: Timing;
  private stopped = false;
  private timer: ReturnType<typeof setTimeout> | null = null;
  private role: ShareRole = "off";
  private me: string | null = null;
  private account = "";
  /** The lease as last seen - whether its holder still renews it. */
  private leaseSeen: LeaseSeen | null = null;
  /** While reading: the followers' cursors as last seen, and until when
   *  (this computer's clock) each counts as there. */
  private followers = new Map<string, { cur: CursorDoc; until: number }>();
  /** While reading: the batch sent to each follower - from where - and not
   *  stored there yet. */
  private inFlight = new Map<string, { from: ShareCursor; at: number }>();
  /** While following: the computer that reads, and when Sync was last
   *  pressed here. */
  private reader: string | null = null;
  private nudgeAt = 0;
  /** A batch that could not be stored, said until one is - or for ten minutes. */
  private problem: { text: string; at: number } | null = null;
  private unsubCursors: Unsubscribe | null = null;
  private unsubDrops: Unsubscribe | null = null;
  private offMailRead: (() => void) | null = null;
  private pumping = false;
  private pumpAgain = false;
  private ticking = false;
  private tickAgain = false;
  /** Settles when the tick under way is over - what `stop` waits for. */
  private tickDone: Promise<void> = Promise.resolve();
  private seq = 0;
  /** Batches are stored one after another, never two at once. */
  private storing: Promise<void> = Promise.resolve();

  constructor(uid: string, deps: MailShareDeps) {
    this.uid = uid;
    this.deps = deps;
    this.timing = { ...DEFAULT_TIMING, ...deps.timing };
  }

  private col() {
    return collection(this.deps.db, "users", this.uid, "mail");
  }

  private now() {
    return this.deps.now ? this.deps.now() : Date.now();
  }

  private setView(patch: Partial<MailShareView>) {
    this.deps.view.set(patch);
  }

  /** What it is doing - for the emulator check. */
  current(): ShareRole {
    return this.role;
  }

  /** Waits for the batch being stored right now - for the emulator check. */
  idle(): Promise<void> {
    return this.storing;
  }

  start() {
    this.offMailRead = this.deps.onMailRead?.(() => void this.pump()) ?? null;
    void this.tick();
  }

  /** Closing: the lease goes back at once, so the other computer reads
   *  Gmail from now on rather than in three minutes. */
  async stop() {
    this.stopped = true;
    if (this.timer) clearTimeout(this.timer);
    // A tick under way may be taking the lease right now: after it, not before.
    await this.tickDone;
    this.offMailRead?.();
    this.unsubCursors?.();
    this.unsubDrops?.();
    this.unsubCursors = this.unsubDrops = null;
    if (this.role === "reader") await this.handBack();
    if (this.role === "follower") await this.deps.backend.mailShareFollow(false).catch(() => {});
    this.role = "off";
    this.reader = null;
  }

  private async handBack() {
    const me = this.me;
    if (!me || !this.account) return;
    const ref = doc(this.col(), leaseId(this.account));
    await runTransaction(this.deps.db, async (tx) => {
      if (parseLease((await tx.get(ref)).data())?.dev === me) tx.delete(ref);
    }).catch(() => {});
  }

  private schedule() {
    if (this.stopped) return;
    if (this.timer) clearTimeout(this.timer);
    this.timer = setTimeout(() => void this.tick(), this.timing.tickMs);
  }

  /** One look at the lease - and what follows from it. */
  async tick() {
    if (this.stopped) return;
    if (this.ticking) {
      this.tickAgain = true;
      return;
    }
    this.ticking = true;
    let done = () => {};
    this.tickDone = new Promise<void>((r) => (done = r));
    try {
      const info = await this.deps.backend.mailShareInfo();
      if (!info.connected) return await this.alone("off", null);
      if (!info.enabled) return await this.alone("solo", "Sharing the reading is off, so each computer reads Gmail itself.");
      if (info.paused) {
        return await this.alone("solo", "Mail is paused on this computer - it neither reads Gmail nor gets mail from your other one.");
      }
      if (!info.device || !(await this.deps.liveOn())) {
        return await this.alone(
          "solo",
          "Live sync is not running on this computer, so it reads Gmail itself. The mail goes between your computers through live sync (Settings → Data).",
        );
      }
      const me = info.device;
      this.me = me;
      this.account = info.account;
      const ref = doc(this.col(), leaseId(info.account));
      const holder = await runTransaction(this.deps.db, async (tx) => {
        const lease = parseLease((await tx.get(ref)).data());
        const now = this.now();
        this.leaseSeen = seeLease(lease, this.leaseSeen, now);
        if (takesLease(lease, me, this.leaseSeen, now, this.timing.staleMs, this.timing.abandonedMs)) {
          tx.set(ref, { dev: me, at: serverTimestamp(), p: JSON.stringify({ account: info.account, platform: this.deps.platform }) });
          return null;
        }
        return lease;
      });
      if (this.stopped) {
        // Closed while it was taking the lease: back at once, as on any close.
        if (!holder) await this.handBack();
        return;
      }
      if (holder) await this.follow(holder.dev, holder.platform);
      else await this.lead();
      if (this.problem && this.now() - this.problem.at > PROBLEM_SHOWN_MS) this.problem = null;
      this.setView({ detail: this.problem?.text ?? null, warn: !!this.problem });
    } catch (e) {
      await this.alone("solo", trouble(e), true);
    } finally {
      this.ticking = false;
      done();
      if (this.tickAgain && !this.stopped) {
        this.tickAgain = false;
        void this.tick();
      } else {
        this.schedule();
      }
    }
  }

  /** Not sharing: the reader reads Gmail here. */
  private async alone(role: "off" | "solo", detail: string | null, warn = false) {
    if (this.role === "follower") await this.deps.backend.mailShareFollow(false).catch(() => {});
    if (this.role === "reader") await this.handBack();
    this.unsubCursors?.();
    this.unsubDrops?.();
    this.unsubCursors = this.unsubDrops = null;
    this.followers.clear();
    this.inFlight.clear();
    this.role = role;
    this.reader = null;
    this.setView({ role, peer: null, detail, warn });
  }

  // ---- reading ------------------------------------------------------------

  private async lead() {
    if (this.role === "follower") await this.deps.backend.mailShareFollow(false);
    this.role = "reader";
    this.reader = null;
    this.listenDrops();
    if (!this.unsubCursors) {
      const from = "cursor~";
      // Until Firestore itself has answered, a cursor counts only by the time
      // it carries; after that, by when it was seen to change.
      let firstLook = true;
      this.unsubCursors = onSnapshot(
        query(this.col(), where(documentId(), ">=", from), where(documentId(), "<", `${from}`)),
        (snap) => {
          const now = this.now();
          let wake = false;
          for (const c of snap.docChanges()) {
            const cur = parseCursorDoc(c.doc.id, c.doc.data());
            if (!cur || cur.reader !== this.me) continue;
            if (c.type === "removed") {
              this.followers.delete(cur.follower);
              continue;
            }
            const before = this.followers.get(cur.follower)?.cur.nudge ?? 0;
            if (!firstLook && cur.nudge > before) wake = true;
            this.followers.set(cur.follower, { cur, until: presentUntil(cur, now, firstLook, this.timing.staleMs) });
          }
          if (wake) this.deps.wakeReader?.();
          if (!snap.metadata.fromCache) firstLook = false;
          void this.pump();
        },
        () => {
          // Looked at again at the next tick.
          this.unsubCursors = null;
        },
      );
    }
    this.showPeer();
    void this.pump();
  }

  /** Followers that said they are there lately. */
  private present(): CursorDoc[] {
    const now = this.now();
    return [...this.followers.values()].filter((f) => f.cur.follower !== this.me && now <= f.until).map((f) => f.cur);
  }

  private showPeer() {
    if (this.role !== "reader") return;
    const f = this.present()[0];
    this.setView({ role: "reader", peer: f ? platformName(f.platform) : null });
  }

  /** The next batch for every follower whose last one is stored. */
  private async pump() {
    if (this.role !== "reader" || this.stopped || !this.me) return;
    if (this.pumping) {
      this.pumpAgain = true;
      return;
    }
    this.pumping = true;
    try {
      do {
        this.pumpAgain = false;
        for (const f of this.present()) {
          const sent = this.inFlight.get(f.follower);
          // Its cursor moved since: stored - or it started again from
          // somewhere else, and goes on from there.
          const stored = !sent || !sameCursor(f.upTo, sent.from);
          // Still being stored there - unless it went missing; then once more.
          if (!stored && this.now() - sent.at < 2 * this.timing.staleMs) continue;
          const batch = await this.deps.backend.mailShareExport(f.upTo, this.timing.maxBytes);
          if (this.stopped || this.role !== "reader") return;
          if (batch.rows.length === 0) {
            this.inFlight.delete(f.follower);
            continue;
          }
          await setDoc(doc(this.col(), dropId(f.follower, this.now(), this.seq++)), {
            dev: this.me,
            at: serverTimestamp(),
            p: JSON.stringify({ account: batch.account, rows: batch.rows, upTo: batch.upTo }),
          });
          this.inFlight.set(f.follower, { from: f.upTo, at: this.now() });
          this.setView({ sent: this.deps.view.get().sent + batch.rows.length });
        }
      } while (this.pumpAgain && !this.stopped);
    } catch {
      // The next cursor change or tick sends it again.
    } finally {
      this.pumping = false;
    }
    this.showPeer();
  }

  // ---- following ----------------------------------------------------------

  private async follow(reader: string, platform: string) {
    const me = this.me;
    if (!me) return;
    if (this.role === "reader") {
      this.unsubCursors?.();
      this.unsubCursors = null;
      this.followers.clear();
      this.inFlight.clear();
    }
    await this.deps.backend.mailShareFollow(true);
    if (this.stopped) {
      // Closed meanwhile: this computer's reader must not stand by for nobody.
      await this.deps.backend.mailShareFollow(false).catch(() => {});
      return;
    }
    this.role = "follower";
    this.reader = reader;
    this.setView({ role: "follower", peer: platformName(platform) });
    // Where it got to - which is also how the reader knows it is here. In
    // line behind any batch being stored, so it never says less than that -
    // and not waited for, so a long line never holds up the next tick.
    void this.enqueue(async () => {
      const progress = await this.deps.backend.mailShareCursor(reader);
      await this.writeCursor(reader, progress.upTo, progress.received);
      this.setView({ received: progress.received });
    });
    this.listenDrops();
  }

  /** The batches addressed to this computer, stored one after another. */
  private listenDrops() {
    const me = this.me;
    if (!me) return;
    if (!this.unsubDrops) {
      const from = dropPrefix(me);
      this.unsubDrops = onSnapshot(
        query(this.col(), where(documentId(), ">=", from), where(documentId(), "<", `${from}`)),
        (snap) => {
          const added = snap
            .docChanges()
            .filter((c) => c.type === "added")
            .map((c) => c.doc)
            .sort((a, b) => (a.id < b.id ? -1 : 1));
          for (const d of added) void this.enqueue(() => this.store(d));
        },
        () => {
          this.unsubDrops = null;
        },
      );
    }
  }

  /** Sync pressed here while the other computer reads: it is asked to look
   *  at Gmail now. Whether it was asked - false when this computer reads
   *  Gmail itself. */
  requestSync(): boolean {
    const reader = this.reader;
    if (this.role !== "follower" || !reader) return false;
    this.nudgeAt = this.now();
    void this.enqueue(async () => {
      const progress = await this.deps.backend.mailShareCursor(reader);
      await this.writeCursor(reader, progress.upTo, progress.received);
    });
    return true;
  }

  /** One after another; one that fails never holds up the next. */
  private enqueue(task: () => Promise<void>): Promise<void> {
    this.storing = this.storing.then(task).catch(() => {});
    return this.storing;
  }

  private async writeCursor(reader: string, upTo: ShareCursor, received: number) {
    if (!this.me) return;
    await setDoc(doc(this.col(), cursorId(this.me, reader)), {
      dev: this.me,
      at: serverTimestamp(),
      p: JSON.stringify({ upTo, received, platform: this.deps.platform, nudge: this.nudgeAt }),
    });
  }

  /** One batch: stored, the cursor moved, the batch deleted - in that order,
   *  so a crash in between only means the same batch is stored once more. */
  private async store(d: QueryDocumentSnapshot<DocumentData>) {
    if (this.stopped) return;
    const drop = parseDrop(d.data());
    try {
      if (drop) {
        const r = await this.deps.backend.mailShareImport(drop.sender, drop.account, drop.rows, drop.upTo);
        await this.writeCursor(drop.sender, drop.upTo, r.received);
        if (this.problem) {
          this.problem = null;
          this.setView({ detail: null, warn: false });
        }
        this.setView({ received: r.received });
      }
    } catch (e) {
      const text = `A batch from the other computer could not be stored: ${e instanceof Error ? e.message : String(e)}`;
      this.problem = { text, at: this.now() };
      this.setView({ detail: text, warn: true });
    }
    // Stored, or one that can never be: either way it does not stay in Firestore.
    await deleteDoc(d.ref).catch(() => {});
  }
}

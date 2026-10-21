// 2.115.0 - two computers, one Gmail reader, for the app.
//
// The one session this app runs (lib/mailShareSession.ts), wired to
// Firestore, the Rust side (commands/mail_share.rs) and the screens. Layout
// starts it beside live sync, whose Firestore space it uses; Mail -> Settings
// and the Mail screen read what it is doing from here.
import { useEffect, useState } from "react";
import { api } from "./api";
import { db } from "./firebase";
import { liveSyncView, onLiveSync } from "./liveSync";
import { onMailSynced, requestMailSync } from "./mailSync";
import { EMPTY_SHARE_VIEW, type MailShareView } from "./mailShareCore";
import { MailShareSession, type MailShareDeps } from "./mailShareSession";

export type { MailShareView } from "./mailShareCore";

const VIEW_EVENT = "tiqr:mail-share";

let view: MailShareView = EMPTY_SHARE_VIEW;

function setView(patch: Partial<MailShareView>) {
  view = { ...view, ...patch };
  window.dispatchEvent(new CustomEvent<MailShareView>(VIEW_EVENT, { detail: view }));
}

export function mailShareView(): MailShareView {
  return view;
}

export function onMailShare(handler: (v: MailShareView) => void): () => void {
  const listener = (e: Event) => handler((e as CustomEvent<MailShareView>).detail);
  window.addEventListener(VIEW_EVENT, listener);
  return () => window.removeEventListener(VIEW_EVENT, listener);
}

/** What the sharing is doing, kept current - for a screen. */
export function useMailShareView(): MailShareView {
  const [v, setV] = useState(view);
  useEffect(() => {
    setV(view);
    return onMailShare(setV);
  }, []);
  return v;
}

/** Live sync is up: Firestore answers, and this account's space is in use. */
const live = () => {
  const p = liveSyncView().phase;
  return p === "live" || p === "sending" || p === "catching-up";
};

const deps: MailShareDeps = {
  db,
  backend: api,
  liveOn: async () => live(),
  platform: navigator.platform || "",
  view: { get: () => view, set: setView },
  onMailRead: (handler) =>
    onMailSynced((r) => {
      if (r.storedNew > 0 || r.changed) handler();
    }),
  wakeReader: () => requestMailSync(),
};

let session: MailShareSession | null = null;

/** Starts sharing for this Firebase account, or restarts it. */
export function startMailShare(uid: string): () => void {
  void session?.stop();
  const mine = new MailShareSession(uid, deps);
  session = mine;
  mine.start();
  // The moment live sync is up, not at the next tick: until then this
  // computer reads Gmail itself, and so may the other one.
  let wasLive = live();
  const off = onLiveSync(() => {
    const now = live();
    if (now && !wasLive) void mine.tick();
    wasLive = now;
  });
  return () => {
    off();
    void mine.stop();
    if (session === mine) session = null;
  };
}

/** Sync pressed while the other computer reads Gmail for this one: that
 *  computer is asked to look now. False when this one reads Gmail itself. */
export function askReaderToSync(): boolean {
  return session?.requestSync() ?? false;
}

/** Sharing turned on or off: look again now. */
export function nudgeMailShare() {
  void session?.tick();
}

/** The window is closing: the lease goes back now, so the other computer
 *  reads Gmail from the next minute rather than in three. */
export async function closeMailShare(): Promise<void> {
  const s = session;
  session = null;
  await s?.stop();
}

// 2.112.0 - live sync between computers, for the app.
//
// marko: "ako by sme vedeli urobit nejaky sync ktory v realnom case po
// zatvoreni apky a otvoreni v inom zariadeni by tam boli insta informacie" -
// "ze vsetko by ako keby fungovalo spolu".
//
// The one session this app runs (lib/liveSyncSession.ts), wired to the real
// Firestore, the Rust side (commands/live_sync.rs) and the screens. Layout
// starts it once Firebase says who is signed in; the sidebar line and
// Settings -> Data read what it is doing from here.
import { api } from "./api";
import { publishDataChanged } from "./dataChanged";
import { db } from "./firebase";
import type { LivePhase } from "./liveSyncCore";
import {
  announceDevice,
  devicesAllLive,
  EMPTY_VIEW,
  LiveSyncSession,
  type LiveSyncDeps,
  type LiveSyncView,
} from "./liveSyncSession";
import { quietly } from "./diagnostics";

export type { LiveSyncView } from "./liveSyncSession";

const VIEW_EVENT = "tiqr:live-sync";

let view: LiveSyncView = EMPTY_VIEW;

function setView(patch: Partial<LiveSyncView>) {
  view = { ...view, ...patch };
  window.dispatchEvent(new CustomEvent<LiveSyncView>(VIEW_EVENT, { detail: view }));
}

export function liveSyncView(): LiveSyncView {
  return view;
}

export function onLiveSync(handler: (v: LiveSyncView) => void): () => void {
  const listener = (e: Event) => handler((e as CustomEvent<LiveSyncView>).detail);
  window.addEventListener(VIEW_EVENT, listener);
  return () => window.removeEventListener(VIEW_EVENT, listener);
}

const deps: LiveSyncDeps = {
  db,
  backend: api,
  view: { get: () => view, set: setView },
  publish: publishDataChanged,
  online: () => navigator.onLine !== false,
  onConnectivity: (handler) => {
    const on = () => handler(true);
    const off = () => handler(false);
    window.addEventListener("online", on);
    window.addEventListener("offline", off);
    return () => {
      window.removeEventListener("online", on);
      window.removeEventListener("offline", off);
    };
  },
  platform: navigator.platform || "",
};

let session: LiveSyncSession | null = null;

/** Starts live sync for this Firebase account, or restarts it. */
export function startLiveSync(uid: string): () => void {
  session?.stop();
  const mine = new LiveSyncSession(uid, deps);
  session = mine;
  void mine.start();
  return () => {
    mine.stop();
    if (session === mine) session = null;
  };
}

/** Something was just changed here: send it now rather than in a moment. */
export function nudgeLiveSync() {
  session?.nudge();
}

/** On or off for this computer, from Settings or the first-run offer. The
 *  other computers are told (`users/{uid}/devices`), because whether Google
 *  Drive is still a way changes travel depends on all of them. */
export async function setLiveSyncEnabled(uid: string, enabled: boolean) {
  const s = await api.liveSyncSetEnabled(enabled);
  await announceDevice(db, uid, s.deviceId, enabled, deps.platform).catch(quietly("liveSync: liveSyncSetEnabled"));
  startLiveSync(uid);
  return s;
}

/** Resolves once live sync has caught up (true), or is off or failing
 *  (false), or after `ms` (false) - for the launch-time Drive check. */
export function waitForLiveSync(ms: number): Promise<boolean> {
  const settled = (p: LivePhase) =>
    p === "live" || p === "sending" ? true : p === "catching-up" || p === "starting" ? null : false;
  const now = settled(view.phase);
  const notLookedYet = view.phase === "off" && !!session && !session.started;
  if (now !== null && !notLookedYet) return Promise.resolve(now);
  return new Promise((resolve) => {
    const timer = setTimeout(() => {
      off();
      resolve(false);
    }, ms);
    const off = onLiveSync((v) => {
      const r = settled(v.phase);
      if (r === null) return;
      clearTimeout(timer);
      off();
      resolve(r);
    });
  });
}

/**
 * Whether Google Drive is only a backup now: this computer is live, and so
 * is every other computer seen in the last 30 days. Then every change
 * already reached this computer live, and combining the Drive copy at launch
 * would only ask a question with nothing behind it. A computer still without
 * live sync keeps Drive as its way across, so then nothing changes.
 */
export async function driveIsBackupOnly(uid: string): Promise<boolean> {
  if (view.phase !== "live" && view.phase !== "sending") return false;
  try {
    const me = (await api.liveSyncStatus()).deviceId;
    return await devicesAllLive(db, uid, me);
  } catch {
    return false;
  }
}

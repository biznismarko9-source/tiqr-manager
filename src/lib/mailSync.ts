// 2.110.0 - the line between the mail reader and the Mail screen.
//
// marko: "chcem aby to citalo spravy ... samo, nie ze ja vzdy sync musim
// davat". So the reading runs for the whole app, whichever screen is open -
// since 2.114.3 in the backend, on a thread of its own (commands/mail_sync.rs,
// started by Layout.tsx). The Mail screen only listens: every run is
// announced here, and the screen refreshes what it shows from it.
//
// The Sync button does not run a sync of its own - two loops racing each
// other is how the same message used to be fetched twice. It asks the one
// reader to go now instead of at its next turn.
import type { MailSyncResult } from "./types";

const SYNCED = "tiqr:mail-synced";
const SYNC_NOW = "tiqr:mail-sync-now";

let last: MailSyncResult | null = null;

/** Called by the reader after every run. */
export function publishMailSync(result: MailSyncResult) {
  last = result;
  window.dispatchEvent(new CustomEvent<MailSyncResult>(SYNCED, { detail: result }));
}

/** The most recent run, for a screen that opens between two of them. */
export function lastMailSync(): MailSyncResult | null {
  return last;
}

export function onMailSynced(handler: (result: MailSyncResult) => void): () => void {
  const listener = (e: Event) => handler((e as CustomEvent<MailSyncResult>).detail);
  window.addEventListener(SYNCED, listener);
  return () => window.removeEventListener(SYNCED, listener);
}

/** "Now, not in a minute." */
export function requestMailSync() {
  window.dispatchEvent(new Event(SYNC_NOW));
}

export function onMailSyncRequested(handler: () => void): () => void {
  window.addEventListener(SYNC_NOW, handler);
  return () => window.removeEventListener(SYNC_NOW, handler);
}

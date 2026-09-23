/**
 * What automatic sync did, last few times, ON THIS MACHINE.
 *
 * 2.48.0. marko: "AUTOSYNC NEFUNGUJE, OKAMZITE HO OPRAVIT NA OBOCH STRANACH".
 * The hard part of that report is that automatic sync had no way to be wrong
 * out loud: `Layout.tsx`'s tick ended in a bare `catch {}`, so a machine whose
 * upload was being refused looked precisely like a machine with nothing to
 * send. Nothing was written down and nothing was shown.
 *
 * This is the record. It is deliberately per-machine (localStorage, not the
 * database): "did MY computer sync" is a question about this computer, and
 * putting it in the database would make it one more thing the sync itself has
 * to carry between the two.
 *
 * It holds the last few attempts, not one: "it worked at 14:05 and has failed
 * every five minutes since" is the shape of the answer marko needs, and a
 * single slot cannot show it.
 */

const KEY = "tiqr.autosync.log";
const KEEP = 8;

export interface AutoSyncRecord {
  /** Epoch ms. */
  at: number;
  /** The backend's own plan - push | pull | merge | idle | off | offline -
   *  or "error" when the attempt threw before any plan could be carried out. */
  action: string;
  /** The backend's own sentence, or the error text. */
  reason: string;
  /** Non-null only for a real failure, so a reader can count failures without
   *  having to parse `action`. */
  error: string | null;
}

/** Never throws: a private window, cleared site data or a full quota must not
 *  take the sync tick down with it - this is bookkeeping, not the work. */
export function recordAutoSync(rec: AutoSyncRecord): void {
  try {
    const list = readAutoSyncLog();
    list.unshift(rec);
    window.localStorage.setItem(KEY, JSON.stringify(list.slice(0, KEEP)));
  } catch {
    /* storage unavailable - the banner still tells him, which is the part
       that matters. */
  }
}

export function readAutoSyncLog(): AutoSyncRecord[] {
  try {
    const raw = window.localStorage.getItem(KEY);
    if (!raw) return [];
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    // Hand-written into localStorage, so nothing here is trusted: anything
    // that is not the right shape is dropped rather than rendered.
    return parsed.filter(
      (r): r is AutoSyncRecord =>
        !!r &&
        typeof r === "object" &&
        typeof (r as AutoSyncRecord).at === "number" &&
        typeof (r as AutoSyncRecord).action === "string" &&
        typeof (r as AutoSyncRecord).reason === "string",
    );
  } catch {
    return [];
  }
}

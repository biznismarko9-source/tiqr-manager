import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export type { Update };

export interface UpdateProgress {
  downloaded: number;
  total: number | null;
}

/** 2.11.0: how often the app re-checks on its own while it stays open.
 * Deliberately long - marko asked explicitly for no aggressive polling. One
 * check shortly after launch (Layout.tsx) and then this, which for a desktop
 * app that is usually opened and closed the same day means most sessions
 * check exactly once. "Check for updates" in Settings is always available and
 * ignores this entirely. */
export const UPDATE_CHECK_INTERVAL_MS = 6 * 60 * 60 * 1000;

/** When the last check FINISHED, successful or not, as epoch ms - or null if
 * none has completed this session. In-memory only and deliberately not
 * persisted: it exists so Settings and the Dashboard can say "last checked
 * ..." without either of them running a check of its own, and a value from a
 * previous run of the app would say nothing useful about this one. */
let lastCheckedAt: number | null = null;

/** Result of the most recent completed check, so a second screen can render
 * the current state without triggering another network call. */
let lastResult: Update | null = null;

export function getLastUpdateCheck(): { at: number | null; update: Update | null } {
  return { at: lastCheckedAt, update: lastResult };
}

/** Asks GitHub Releases (see tauri.conf.json -> plugins.updater.endpoints)
 * whether a newer signed version exists. Returns null both when already
 * up to date and when the check itself fails (e.g. offline) - callers that
 * want to distinguish the two should catch separately.
 *
 * 2.11.0: records when it finished and what it found. A failed check still
 * updates the timestamp (we did look) but leaves `lastResult` alone, so a
 * transient offline moment never makes an already-found update disappear
 * from the UI. */
export async function checkForUpdate(): Promise<Update | null> {
  try {
    const update = await check();
    lastCheckedAt = Date.now();
    lastResult = update;
    return update;
  } catch (e) {
    lastCheckedAt = Date.now();
    throw e;
  }
}

/** Downloads the new installer, verifies its signature, and runs it, then
 * relaunches the app. Rejects if any step fails - the caller decides how to
 * surface that. */
export async function installUpdate(update: Update, onProgress?: (p: UpdateProgress) => void): Promise<void> {
  let downloaded = 0;
  let total: number | null = null;
  await update.downloadAndInstall((event) => {
    if (event.event === "Started") {
      total = event.data.contentLength ?? null;
      onProgress?.({ downloaded: 0, total });
    } else if (event.event === "Progress") {
      downloaded += event.data.chunkLength;
      onProgress?.({ downloaded, total });
    } else if (event.event === "Finished") {
      onProgress?.({ downloaded: total ?? downloaded, total });
    }
  });
  await relaunch();
}

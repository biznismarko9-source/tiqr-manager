import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export type { Update };

export interface UpdateProgress {
  downloaded: number;
  total: number | null;
}

/** Asks GitHub Releases (see tauri.conf.json -> plugins.updater.endpoints)
 * whether a newer signed version exists. Returns null both when already
 * up to date and when the check itself fails (e.g. offline) - callers that
 * want to distinguish the two should catch separately. */
export async function checkForUpdate(): Promise<Update | null> {
  return check();
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

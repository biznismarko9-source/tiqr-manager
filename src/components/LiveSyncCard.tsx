// 2.112.0 - Settings -> Data: live sync between computers.
//
// The switch, what it is doing right now in a few words, and - for the one
// thing nothing in the app can do by itself - how to publish the Firestore
// rules it needs. See lib/liveSync.ts and commands/live_sync.rs.
import { useEffect, useState } from "react";
import { api, errMsg } from "../lib/api";
import { auth } from "../lib/firebase";
import { formatDateTime } from "../lib/format";
import { liveSyncView, onLiveSync, setLiveSyncEnabled, type LiveSyncView } from "../lib/liveSync";
import { phaseLabel, phaseTone } from "../lib/liveSyncCore";
import { useToast } from "../lib/toast";
import type { LiveSyncStatus } from "../lib/types";
import { Badge, Card, CHECKBOX_CLASS } from "./ui";

const TONE_BADGE: Record<"ok" | "busy" | "warn" | "off", string> = {
  ok: "paid",
  busy: "partial",
  warn: "unpaid",
  off: "available",
};

export function LiveSyncCard() {
  const toast = useToast();
  const [status, setStatus] = useState<LiveSyncStatus | null>(null);
  const [live, setLive] = useState<LiveSyncView>(liveSyncView());
  const [busy, setBusy] = useState(false);
  const uid = auth.currentUser?.uid ?? null;

  useEffect(() => onLiveSync(setLive), []);
  useEffect(() => {
    api.liveSyncStatus().then(setStatus).catch(() => {});
  }, [live.phase, live.lastPullAt, live.lastPushAt]);

  const toggle = async (on: boolean) => {
    if (!uid) return;
    setBusy(true);
    try {
      setStatus(await setLiveSyncEnabled(uid, on));
      toast.success(on ? "Live sync is on. Turn it on on your other computer too." : "Live sync is off on this computer.");
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setBusy(false);
    }
  };

  const enabled = status?.enabled ?? false;
  return (
    <Card className="p-5">
      <h3 className="mb-1 text-sm font-semibold text-slate-800 dark:text-slate-200">Live sync</h3>
      <p className="mb-3 text-xs text-slate-500 dark:text-slate-400">
        Every change - an order, a sale, a note, a mail rule - shows up on your other computer within seconds. If both
        computers changed the same record, the newer change wins. Google Drive stays as the backup copy.
      </p>
      {!uid ? (
        <p className="text-xs text-amber-700 dark:text-amber-400">Sign in to your TIQR account first.</p>
      ) : (
        <>
          <label className="flex cursor-pointer items-center gap-2.5 text-sm text-slate-700 dark:text-slate-300">
            <input
              type="checkbox"
              className={CHECKBOX_CLASS}
              checked={enabled}
              disabled={busy || !status}
              onChange={(e) => void toggle(e.target.checked)}
            />
            Live sync on this computer
          </label>

          {!enabled && status?.pausedAfterRestore && (
            <p className="mt-2 text-xs text-amber-700 dark:text-amber-400">{status.lastError}</p>
          )}

          {enabled && (
            <div className="mt-3 space-y-2 text-xs">
              <div className="flex flex-wrap items-center gap-2">
                <Badge tone={TONE_BADGE[phaseTone(live.phase)]}>{phaseLabel(live.phase, live.outbox)}</Badge>
                <span className="text-slate-500 dark:text-slate-400">
                  {status?.lastPullAt ? `Received ${formatDateTime(status.lastPullAt)}` : "Nothing received yet"}
                  {" · "}
                  {status?.lastPushAt ? `sent ${formatDateTime(status.lastPushAt)}` : "nothing sent yet"}
                </span>
              </div>

              {live.waiting > 0 && (
                <p className="text-slate-600 dark:text-slate-300">
                  {live.waiting} change{live.waiting === 1 ? "" : "s"} from the other computer{" "}
                  {live.waiting === 1 ? "is" : "are"} waiting for a record {live.waiting === 1 ? "it points" : "they point"} at -
                  they land by themselves when it arrives.
                </p>
              )}

              {live.phase === "rules" && (
                <div className="rounded-lg bg-amber-50 px-3 py-2.5 text-amber-900 ring-1 ring-inset ring-amber-200 dark:bg-amber-500/10 dark:text-amber-200 dark:ring-amber-500/25">
                  <p className="font-semibold">Firebase does not allow it yet - one step, once.</p>
                  <ol className="mt-1 list-decimal space-y-0.5 pl-4">
                    <li>Open console.firebase.google.com → the TIQR Manager project → Firestore Database → Rules.</li>
                    <li>
                      Replace everything there with the contents of <span className="font-mono">firestore.rules</span>{" "}
                      from this release, and press Publish.
                    </li>
                  </ol>
                  <p className="mt-1">It starts by itself within a minute.</p>
                </div>
              )}

              {live.phase === "quota" && (
                <p className="text-amber-800 dark:text-amber-300">
                  Firebase&apos;s free daily limit is used up - usually only on the very first day, while every record
                  goes up once. It carries on by itself; nothing is lost meanwhile.
                </p>
              )}

              {live.phase === "offline" && (
                <p className="text-slate-600 dark:text-slate-300">
                  No connection. Changes made here wait and go as soon as it is back.
                </p>
              )}

              {live.detail && live.phase !== "live" && live.phase !== "sending" && live.phase !== "rules" && (
                <p className="text-red-600 dark:text-red-400">{live.detail}</p>
              )}

              {live.reasons.length > 0 && (
                <details className="text-slate-600 dark:text-slate-300">
                  <summary className="cursor-pointer">
                    {live.reasons.length} thing{live.reasons.length === 1 ? "" : "s"} could not be applied
                  </summary>
                  <ul className="mt-1 list-disc space-y-0.5 pl-4 font-mono text-[11px]">
                    {live.reasons.map((r) => (
                      <li key={r}>{r}</li>
                    ))}
                  </ul>
                </details>
              )}

              <p className="text-slate-400 dark:text-slate-500">
                Gmail is connected on each computer by itself - its sign-in never leaves this computer.
              </p>
            </div>
          )}
        </>
      )}
    </Card>
  );
}

import logo from "../assets/logo.png";
import { Spinner } from "./ui";
import type { UpdateProgress } from "../lib/updater";

/** 2.0.39: full-screen branded takeover shown while an update downloads and
 * installs (Settings.tsx, "Software updates" section, `installing === true`).
 * Before this, that moment was just a small progress bar inside a settings
 * card - marko asked for a real "screen" here instead, matching the app's
 * own theme (logo, name, brand colors) rather than whatever plain/default
 * look was showing through around it. Purely presentational: reuses
 * installUpdate()'s existing progress callback (installProgress state, still
 * owned by Settings.tsx) completely untouched, just renders it full-screen
 * instead of inline. `position: fixed; inset-0` covers the whole app window
 * regardless of where this is mounted in the tree (verified no ancestor sets
 * transform/filter/perspective, which would otherwise break that). z-[70] -
 * one above ConfirmDialog's z-[60], the highest layer that existed before
 * this - so this always wins if it's ever showing. */
export function UpdateOverlay({ version, progress }: { version?: string; progress: UpdateProgress | null }) {
  const pct = progress?.total ? Math.min(100, Math.round((progress.downloaded / progress.total) * 100)) : null;
  return (
    <div className="fixed inset-0 z-[70] flex flex-col items-center justify-center bg-gradient-to-br from-brand-600 to-brand-900 dark:from-brand-800 dark:to-brand-950">
      <img src={logo} alt="TIQR Manager" className="h-20 w-20 rounded-2xl shadow-lg" />
      <h1 className="mt-5 text-2xl font-semibold text-white">TIQR Manager</h1>
      <p className="mt-1 text-sm text-brand-100">
        Installing {version}
        {pct !== null ? ` - ${pct}%` : "..."}
      </p>
      <div className="mt-6 h-1.5 w-64 max-w-[80vw] overflow-hidden rounded-full bg-white/20">
        <div
          className="h-full rounded-full bg-white transition-all"
          style={{ width: pct !== null ? `${pct}%` : "30%" }}
        />
      </div>
      <p className="mt-6 flex items-center gap-2 text-xs text-brand-200">
        <Spinner className="h-4 w-4" />
        The app will restart automatically when this finishes.
      </p>
    </div>
  );
}

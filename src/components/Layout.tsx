import { useEffect, useRef, useState } from "react";
import { Link, NavLink, Outlet, useLocation } from "react-router-dom";
import {
  IconAlertTriangle,
  IconCalendarDays,
  IconChevronUp,
  IconGauge,
  IconLogOut,
  IconMoon,
  IconPackage,
  IconReceipt,
  IconSettings,
  IconSun,
  IconUsers,
  IconWallet,
} from "./icons";
import { relaunch } from "@tauri-apps/plugin-process";
import { Spinner } from "./ui";
import { checkForUpdate, UPDATE_CHECK_INTERVAL_MS } from "../lib/updater";
import { api, errMsg } from "../lib/api";
import { recordAutoSync } from "../lib/autoSyncLog";
import { useToast } from "../lib/toast";
import { useAuth } from "../lib/auth";
import { useTheme } from "../lib/theme";
import { listen } from "@tauri-apps/api/event";
import type { ScanRunFinishedPayload } from "../lib/types";
import { Tour } from "./Tour";
import logo from "../assets/logo.png";

// 2.14.0: how often automatic sync looks at the other machine while the app
// stays open. Five minutes is picked for what it costs, not for how fast it
// feels: a check is one small Drive metadata request, but a check that finds
// unsent work uploads the whole database, so a tighter interval would mean
// re-uploading the same file every couple of minutes through a long working
// session. Five minutes is far below the time it takes to walk from one
// machine to the other, which is the only deadline that matters here.
const AUTO_SYNC_INTERVAL_MS = 5 * 60 * 1000;

// 2.0.44: initials shown in the profile widget's avatar circle - up to 2,
// from up to 2 words of the name, uppercased. "T" for an empty/whitespace
// name rather than crashing on `[0]` of an empty array.
function initialsFor(name: string): string {
  const parts = name.trim().split(/\s+/).filter(Boolean);
  if (parts.length === 0) return "?";
  return parts
    .slice(0, 2)
    .map((p) => p[0]!.toUpperCase())
    .join("");
}

// 2.4.4: marko's own request to group Events/Orders/Tickets/Sales/Inventory
// under one collapsible "Tickets" sidebar entry instead of 5 flat top-level
// rows - none of these 5 routes themselves changed (still /events, /orders,
// etc., unchanged everywhere else that links to them), this only changes
// how the sidebar GROUPS the links to them.
//
// 2.5.1: marko's own follow-up - Ticket Center (briefly a Finance subtab in
// 2.4.4, see Finance.tsx's own 2.5.1 comment) is back out as its own
// top-level entry, and the whole top-level order changed to match his exact
// list: Dashboard, Tickets, Price Checker, Pulls, Finance, Ticket Center,
// Calendar. See TicketCenter.tsx's own module doc comment for why it's an
// ORDERS list now (not the old per-ticket Control Center/Fulfillment Center
// pages, both removed this version).
// Explicit discriminated union (rather than letting TS infer one from the
// NAV array literal below) so the "children" in item check in the render
// below narrows cleanly - a plain inferred type here widens to one merged
// shape with every field optional instead of a real A | B union.
// 2.43.0: ONE shape. The collapsible "Tickets" group and the unused
// `heading` row are both gone - marko: "odstran tuto zalozku ze bude tam
// vzdy ukazovat vsetko events inventory sales pulls". Every entry is a plain
// link now, always visible, nothing to open or close before you can click.
// (The `heading` variant had had no entries since 2.36.0 removed "Market &
// money"; it went with the group rather than sitting here unused.)
type NavItem = { to: string; label: string; icon: typeof IconGauge; end?: boolean };

const NAV: NavItem[] = [
  { to: "/", label: "Dashboard", icon: IconGauge, end: true },
  // 2.35.1 folded /inventory into /tickets; 2.39.0 folded Orders and
  // Inventory into one list at /orders; 2.43.0 takes the wrapper off
  // entirely. The five working screens sit flat, in the order the job runs:
  // the event exists, you buy stock for it, you sell it, and a pull is the
  // variant where you bought it for someone else.
  { to: "/events", label: "Events", icon: IconCalendarDays },
  { to: "/orders", label: "Inventory", icon: IconPackage },
  { to: "/sales", label: "Sales", icon: IconReceipt },
  { to: "/pulls", label: "Pulls", icon: IconUsers },
  // Finance is deliberately last and deliberately NOT grouped with the four
  // above: it covers money that has nothing to do with tickets (rent, fees,
  // personal spend) and owns its own four tabs, categories and accounts.
  { to: "/finance", label: "Finance", icon: IconWallet },
];

// Shared by every NavLink below, so the active/hover look is defined in
// exactly one place. 2.43.0: there is no longer a group header computing an
// equivalent class of its own - every row in the sidebar is a real link.
// 2.6.0 (visual redesign): the active state is now a tinted surface plus a
// short accent bar pinned to the item's left edge (see `NAV_ACTIVE_BAR`
// below), instead of a flat brand-tinted pill. The bar is what makes the
// current page findable at a glance in a 192px-wide rail; the tint alone
// was easy to miss next to the hover state, which used a similar weight.
// Nothing about which items exist, their order, or where they link changed.
const NAV_BASE =
  "group relative flex items-center gap-2.5 rounded-lg px-3 py-[5px] text-[12.5px] transition focus:outline-none focus-visible:ring-2 focus-visible:ring-brand-500";
// 2.29.0 (Onyx): the current item is PRESSED INTO the rail rather than
// tinted on top of it - the one gesture this material has that a flat one
// does not, and it is unmistakable at a glance in a narrow rail. The accent
// bar stays: the dent alone is quiet, and the bar is what 2.6.0 added
// precisely because the tint alone was easy to miss.
const NAV_ACTIVE =
  "bg-brand-500/[0.16] font-semibold text-slate-900 dark:text-slate-50";
const NAV_IDLE =
  "font-medium text-slate-500 hover:text-slate-900 dark:text-slate-400 dark:hover:text-slate-100";
const NAV_ACTIVE_BAR =
  "absolute left-0 top-1/2 h-4 w-[3px] -translate-y-1/2 rounded-r-full bg-brand-500";

const navLinkClass = ({ isActive }: { isActive: boolean }) =>
  `${NAV_BASE} ${isActive ? NAV_ACTIVE : NAV_IDLE}`;

export default function Layout() {
  const toast = useToast();

  // 2.27.0 - marko asked to be told when an automatic run finishes, so he can
  // start one and go and do something else. (2.28.0: the wording no longer
  // mentions a map - that feature was removed.) The run exists so he can go
  // and do
  // something else, so the thing that tells him it is done has to reach him
  // wherever he is - not on the Price Checker page he has already left. This
  // listener lives in the Layout, which is mounted for every page.
  //
  // The backend ALSO raises an OS notification for the same event, which is
  // what reaches him when TIQR is behind another window. This one is for when
  // he is looking at the app.
  useEffect(() => {
    let dispose: (() => void) | undefined;
    let disposed = false;
    listen<ScanRunFinishedPayload>("price-scanner-run-finished", (event) => {
      const p = event.payload;
      // A run he stopped himself needs no announcement.
      if (p.stopped) return;
      toast.success(
        `Price scan finished - ${p.listingCount} listing${p.listingCount === 1 ? "" : "s"} read (${p.reason}).`,
      );
    }).then((fn) => {
      if (disposed) fn();
      else dispose = fn;
    });
    return () => {
      disposed = true;
      dispose?.();
    };
  }, [toast]);
  const { user, logout } = useAuth();
  const location = useLocation();
  const [profileOpen, setProfileOpen] = useState(false);
  const profileRef = useRef<HTMLDivElement>(null);
  // 2.13.1 checked and told; 2.14.0 acts - marko asked for the hand-off
  // between his Mac and his Windows PC to stop needing a click. This holds
  // the one sentence the backend could NOT decide by itself (see
  // cloud_sync.rs's `decide_auto`); everything it could decide happens
  // without ever reaching this banner.
  const [syncNotice, setSyncNotice] = useState<string | null>(null);
  // 2.48.0: what automatic sync did last, and whether it FAILED. Until now
  // the tick swallowed every error (`catch {}`), so a machine whose upload was
  // being refused looked exactly like a machine with nothing to send - marko:
  // "AUTOSYNC NEFUNGUJE". It could not have told him, on either machine.
  const [syncFailure, setSyncFailure] = useState<string | null>(null);
  // 2.49.2: true when Drive refused for the ONE reason no retry can ever fix -
  // the signed-in Google account never granted Drive. marko: "doteraz to
  // fungovalo, urob to tak aby to fungovalo aj teraz bez zmien". A permission
  // that was never granted cannot be conjured from this side; what CAN be
  // removed is the hunt for it, so the banner carries the sign-in itself.
  const [needsGoogleConsent, setNeedsGoogleConsent] = useState(false);
  const [reconsenting, setReconsenting] = useState(false);
  // The banner re-runs the SAME tick the timer does, rather than a second
  // copy of the logic that could drift from it.
  const tickRef = useRef<null | (() => Promise<void>)>(null);
  // Guards against a tick starting while the previous one is still
  // uploading - a slow upload on a slow connection must not stack.
  const autoSyncBusy = useRef(false);
  // 2.48.1: the `atStartup` flag is GONE, and that is the fix.
  //
  // It used to be true for exactly one tick, at mount, and only that tick was
  // allowed to pull or merge. Two things went wrong with that. At mount the
  // Google token is usually not ready, so the one privileged tick returned
  // `off` and the chance was spent having never had one. And every later tick
  // was unprivileged, so two machines holding different data raised a banner
  // every five minutes and never combined - marko: "stale sa nespajaju tie
  // info ... a stale to nieje automaticke".
  //
  // Now every tick may pull or merge. The only thing that ever defers it is
  // `busyEditing()` below, which is the real question anyway: not "how long
  // has the app been open" but "would a reload throw away something he is
  // typing right now".
  // 2.17.0: what the app is doing with marko's data right now, in words. It
  // exists because a whole database crossing the internet takes real seconds
  // and, until this release, they were seconds of nothing - see SyncActivity
  // at the bottom of this file. `blocking` separates "carry on working, this
  // is happening quietly" from "this ends in a restart, there is nothing
  // useful to click".
  const [syncActivity, setSyncActivity] = useState<{ label: string; blocking: boolean } | null>(null);
  // 2.4.4: one-click light/dark toggle above the profile widget - replaces
  // Settings -> Appearance's old 3-way Light/System/Dark picker (marko's own
  // request). Reuses the exact same lib/theme.ts useTheme() hook that picker
  // used to call - only the UI moved, not the underlying preference
  // mechanism. `isDark` resolves "system" mode to whatever it's currently
  // actually rendering (same OS media query theme.ts's own applyMode already
  // checks) so the toggle's icon/label always matches reality, and clicking
  // it always sets an explicit light/dark mode - "system" isn't a state this
  // control ever sets, only one it can start from.
  const [themeMode, setThemeMode] = useTheme();
  const isDark = themeMode === "dark" || (themeMode === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);

  // 2.0.44: click-outside-to-close for the profile dropdown - it's a small
  // anchored menu, not a full-screen Modal (which already has its own
  // backdrop for this), so it needs its own listener.
  useEffect(() => {
    if (!profileOpen) return;
    const onClick = (e: MouseEvent) => {
      if (profileRef.current && !profileRef.current.contains(e.target as Node)) setProfileOpen(false);
    };
    document.addEventListener("mousedown", onClick);
    return () => document.removeEventListener("mousedown", onClick);
  }, [profileOpen]);

  useEffect(() => {
    // Quiet, one-time check on launch. Never blocks the UI and never
    // surfaces an error - if it's offline or GitHub is unreachable, the
    // app just carries on as a fully offline tool. Nothing downloads until
    // the user explicitly approves it from Settings.
    // 2.11.0: the same one-shot check as before, plus a slow repeat for the
    // rare session that stays open for days. UPDATE_CHECK_INTERVAL_MS is 6
    // hours - marko asked explicitly for no aggressive polling, and a desktop
    // app that gets opened and closed the same day will still only ever check
    // once. Failures stay silent here (offline is normal for this app); the
    // only place a check failure is ever surfaced is Settings, where the user
    // asked for it.
    const runCheck = () => {
      checkForUpdate()
        .then((update) => {
          if (update) toast.info(`TIQR Manager ${update.version} is available - open Settings to install it.`);
        })
        .catch(() => {});
    };
    runCheck();
    const interval = setInterval(runCheck, UPDATE_CHECK_INTERVAL_MS);
    return () => clearInterval(interval);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // 2.14.0: automatic two-machine sync. `cloud_sync_auto` DECIDES and does
  // nothing - the acting is done here by calling the very same push/pull the
  // Settings buttons call, so nothing destructive gained a second code path.
  //
  // Two deliberate asymmetries:
  //
  // * Uploading is invisible and can happen any time; it only ever adds a
  //   version to Drive and is refused outright by the backend's lost-update
  //   guard if the other machine got there first. Never `force`.
  // * Downloading REPLACES this database and therefore restarts the app, so
  //   it is only ever done automatically at launch. Mid-session it becomes
  //   the banner instead - restarting the app under marko while he is typing
  //   into a form would be a bug, not a feature. 2.16.0's merge follows the
  //   same rule for the same reason: it only adds rows, but they still land
  //   underneath whatever is already on screen, so it reloads the page.
  useEffect(() => {
    let cancelled = false;
    // Is marko in the middle of something a reload would destroy? A merge only
    // ADDS rows, so the database is never the risk - the page reload that
    // follows it is. An open modal or a field he has typed into is the whole
    // list of things worth waiting for.
    const busyEditing = () => {
      if (document.querySelector(".fixed.inset-0.z-50")) return true;
      const el = document.activeElement as HTMLElement | null;
      if (!el) return false;
      const tag = el.tagName;
      if (tag === "TEXTAREA" || tag === "SELECT") return true;
      if (tag === "INPUT") return (el as HTMLInputElement).value.trim().length > 0;
      return el.isContentEditable;
    };
    const tick = async () => {
      if (autoSyncBusy.current) return;
      autoSyncBusy.current = true;
      try {
        const plan = await api.cloudSyncAuto();
        if (cancelled) return;
        if (plan.action === "push") {
          setSyncActivity({ label: "Saving your changes to Google Drive", blocking: false });
          await api.cloudSyncPush();
        } else if (plan.action === "off" || plan.action === "offline") {
          // Both are normal states for a local-first app: recorded so the
          // history in Settings shows the timer IS alive, but never shown.
          recordAutoSync({ at: Date.now(), action: plan.action, reason: plan.reason, error: null });
          setSyncFailure(null);
          return;
        } else if (plan.action === "pull" && !busyEditing()) {
          setSyncActivity({ label: "Getting newer data from your other computer", blocking: true });
          const safetyPath = await api.cloudSyncPull();
          toast.success(`Synced down from your other computer. Your previous data was saved to ${safetyPath}. Restarting...`);
          setTimeout(() => relaunch(), 900);
        } else if (plan.action === "merge" && !busyEditing()) {
          // 2.16.0: the case that used to stop and ask which machine wins.
          // Nothing is replaced and nothing is deleted, so there is no
          // question left to put to marko - but the page still has to reload,
          // because rows arrived underneath everything already on screen.
          // A plain reload, not `relaunch()`: the database file was added to,
          // not swapped, so the running process is fine.
          setSyncActivity({ label: "Combining what's on both computers", blocking: true });
          const merged = await api.cloudMergePull();
          const parts = [`Added ${merged.totalInserted} record${merged.totalInserted === 1 ? "" : "s"} from your other computer.`];
          if (merged.totalDeleted > 0) {
            parts.push(`${merged.totalDeleted} removed here because you deleted them there.`);
          }
          if (merged.totalRenumbered > 0) {
            parts.push(`${merged.totalRenumbered} got a new code (both computers had used the same one).`);
          }
          if (merged.totalSkipped > 0) {
            parts.push(`${merged.totalSkipped} couldn't be added - see Settings → Data.`);
          }
          if (merged.totalIdentityClashes > 0) {
            parts.push(`${merged.totalIdentityClashes} couldn't be told apart from yours - see Settings → Data.`);
          }
          toast.success(parts.join(" "));
          setTimeout(() => window.location.reload(), 1200);
        } else if (plan.action === "pull" || plan.action === "merge") {
          // 2.48.1: only reached when marko IS mid-edit. The work is real and
          // waiting; it happens on the next tick once he is done, and the
          // banner is there so a long form is not a silent stall.
          setSyncNotice(`${plan.reason} It will finish once you're done editing.`);
        }
        // Whatever happened, it happened - including "nothing to do", which is
        // the answer marko most needs to be able to see when he believes sync
        // is dead.
        recordAutoSync({ at: Date.now(), action: plan.action, reason: plan.reason, error: null });
        setSyncFailure(null);
      } catch (e) {
        // 2.48.0: no longer silent. `Off` and `Offline` are decided by the
        // backend and arrive as a plan, not as a throw - so anything landing
        // HERE is a real failure: a refused upload, a poisoned lock, a broken
        // token refresh. Those used to vanish, which is the whole reason
        // automatic sync could be dead for days without saying so.
        const message = errMsg(e);
        recordAutoSync({ at: Date.now(), action: "error", reason: message, error: message });
        setSyncFailure(message);
        // The backend already classifies this (cloud_sync::forbidden_hint);
        // matching its own words keeps the two from drifting apart.
        setNeedsGoogleConsent(/does not include permission for Drive/i.test(message));
      } finally {
        autoSyncBusy.current = false;
        // Left standing on purpose when a restart or reload is already
        // scheduled above: clearing it would flash the app back to normal for
        // a second and make the restart look like a crash.
        setSyncActivity((current) => (current?.blocking ? current : null));
      }
    };
    // Once at launch, then quietly every five minutes.
    //
    // 2.50.1 REMOVES 2.48.1's focus/visibilitychange triggers. They were added
    // to make the hand-off between his two machines feel immediate, and they
    // did - but marko: "vzdy ked kliknem na tiqr tak sa spusti sync, ked
    // vyjdem na par sekund a vratim sa tak tiez". Alt-tabbing is not an event
    // worth moving a database for, and every one of those syncs announced
    // itself in the header. The five-minute timer already covers the same
    // hand-off within a few minutes, without turning every click on the
    // window into activity.
    tickRef.current = tick;
    const run = () => void tick();
    run();
    const interval = setInterval(run, AUTO_SYNC_INTERVAL_MS);
    return () => {
      cancelled = true;
      tickRef.current = null;
      clearInterval(interval);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // 2.0.76: periodic check for the outbound-notification feature (desktop/
  // ntfy - Settings -> Notifications) - see commands/notifications.rs's
  // module doc comment. Fires once shortly after mount,
  // not just after the first full interval, so a category that's already
  // due isn't left waiting up to 30 minutes to be noticed - then every 30
  // minutes after that for as long as the app stays open. Silent on both
  // success and failure, the same "never blocks, never surfaces an error"
  // shape as checkForUpdate right above - every channel is independently
  // optional, and any of them being off, misconfigured, or unreachable must
  // never interrupt the app with an error toast (the "Send test" buttons in
  // Settings are where a real failure IS shown, on purpose).
  useEffect(() => {
    const check = () => {
      api.checkAndSendNotifications().catch(() => {});
    };
    check();
    const interval = setInterval(check, 30 * 60 * 1000);
    return () => clearInterval(interval);
  }, []);

  return (
    <div className="flex h-full w-full overflow-hidden bg-slate-50 dark:bg-slate-950">
      {/* 2.0.30: w-56 (224px) -> w-48 (192px) - marko asked to make the
          sidebar narrower to give wide tables (Pulls) more room; nav labels
          and the logo lockup still have comfortable margin at this width.
          2.6.0: width deliberately UNCHANGED - the redesign buys its extra
          breathing room from tighter internal padding and a smaller nav type
          size, not by taking width back off the tables. */}
      <aside className="flex w-48 shrink-0 flex-col border-r border-line bg-surface-sunken">
        {/* Brand lockup. The hairline under it is what separates the app's
            identity from its navigation - the same "one quiet rule per
            section boundary" the sidebar uses throughout, rather than
            boxes or background changes. */}
        <div className="flex items-center gap-2.5 border-b border-slate-100 px-3.5 py-3.5 dark:border-slate-800/80">
          <img src={logo} alt="TIQR Manager" className="h-8 w-8 rounded-lg shadow-card ring-1 ring-slate-900/5 dark:ring-white/10" />
          <div className="min-w-0">
            <p className="truncate text-[13px] font-semibold leading-tight tracking-tight text-slate-900 dark:text-slate-50">
              TIQR Manager
            </p>
            <p className="truncate text-[11px] leading-tight text-slate-500 dark:text-slate-400">Reseller toolkit</p>
          </div>
        </div>

        {/* 2.25.0: `data-tour` anchors only - see components/Tour.tsx. */}
        <nav data-tour="nav" className="flex-1 space-y-0.5 overflow-y-auto px-2 py-3">
          {NAV.map((item) => (
            <NavLink key={item.to} data-tour={`nav:${item.to}`} to={item.to} end={item.end} className={navLinkClass}>
              {({ isActive }) => (
                <>
                  {isActive && <span className={NAV_ACTIVE_BAR} aria-hidden="true" />}
                  <item.icon className="h-[17px] w-[17px] shrink-0" />
                  <span className="truncate">{item.label}</span>
                </>
              )}
            </NavLink>
          ))}
        </nav>

        {/* 2.4.4: one-click light/dark toggle - see the isDark/setThemeMode
            comment above. Sits in its own bordered row directly above the
            profile widget, exactly where marko asked for it. 2.6.0: same
            single-click behaviour and the same useTheme() call, restyled -
            the icon now sits in its own small chip so the row reads as a
            control rather than as one more nav item. */}
        <div className="border-t border-slate-100 px-2 py-2 dark:border-slate-800/80">
          <button
            type="button"
            onClick={() => setThemeMode(isDark ? "light" : "dark")}
            title={isDark ? "Switch to light mode" : "Switch to dark mode"}
            className={`${NAV_BASE} w-full ${NAV_IDLE}`}
          >
            <span className="flex h-[17px] w-[17px] shrink-0 items-center justify-center">
              {isDark ? <IconMoon className="h-4 w-4" /> : <IconSun className="h-4 w-4" />}
            </span>
            <span className="flex-1 truncate text-left">{isDark ? "Dark mode" : "Light mode"}</span>
          </button>
        </div>

        {/* 2.0.44: profile widget - marko's own screenshot pointed at this
            exact spot (previously just the tagline below on its own). The
            dropdown opens UPWARD (bottom-full) since this sits at the very
            bottom of the sidebar - opening down would run off the window. */}
        <div ref={profileRef} className="relative border-t border-slate-100 p-2 dark:border-slate-800/80">
          {profileOpen && (
            // 2.0.74: same "pop-in" entrance as Modal/ConfirmDialog
            // (index.css) - `origin-bottom` so it visibly grows up out of
            // the button it's anchored to (this menu opens upward) instead
            // of scaling from its own center, which would look like it's
            // growing out of thin air above the button.
            <div className="absolute inset-x-2 bottom-full mb-1.5 origin-bottom animate-[pop-in_.16s_ease-out] overflow-hidden rounded-xl border border-slate-200 bg-surface p-1 shadow-overlay dark:border-slate-700 dark:bg-slate-800">
              <Link
                to="/settings"
                onClick={() => setProfileOpen(false)}
                className="flex items-center gap-2 rounded-lg px-2.5 py-2 text-xs font-medium text-slate-600 transition hover:bg-slate-100 hover:text-slate-900 dark:text-slate-300 dark:hover:bg-slate-700/70 dark:hover:text-slate-50"
              >
                <IconSettings className="h-3.5 w-3.5" /> Settings
              </Link>
              <button
                type="button"
                onClick={async () => {
                  setProfileOpen(false);
                  try {
                    await logout();
                  } catch {
                    toast.error("Couldn't log out - try again.");
                  }
                }}
                className="flex w-full items-center gap-2 rounded-lg px-2.5 py-2 text-left text-xs font-medium text-red-600 transition hover:bg-red-50 dark:text-red-400 dark:hover:bg-red-500/10"
              >
                <IconLogOut className="h-3.5 w-3.5" /> Log out
              </button>
            </div>
          )}
          <button
            type="button"
            onClick={() => setProfileOpen((o) => !o)}
            aria-expanded={profileOpen}
            className={`flex w-full items-center gap-2.5 rounded-lg px-2 py-2 text-left transition focus:outline-none focus-visible:ring-2 focus-visible:ring-brand-500 ${
              profileOpen
                ? "bg-slate-100 dark:bg-slate-800/70"
                : "hover:bg-slate-100 dark:hover:bg-slate-800/70"
            }`}
          >
            <span className="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-brand-600 text-[11px] font-semibold text-white shadow-card">
              {initialsFor(user?.name ?? "?")}
            </span>
            <span className="min-w-0 flex-1">
              <span className="block truncate text-xs font-semibold text-slate-800 dark:text-slate-200">
                {user?.name ?? "Account"}
              </span>
              <span className="block truncate text-[11px] text-slate-500 dark:text-slate-400">{user?.email ?? ""}</span>
            </span>
            <IconChevronUp
              className={`h-3.5 w-3.5 shrink-0 text-slate-400 transition-transform ${profileOpen ? "" : "rotate-180"}`}
            />
          </button>
        </div>
      </aside>
      <main className="min-w-0 flex-1 overflow-y-auto">
        {/* 2.0.31: this div used to cap out at max-w-[1400px] and center
            itself (mx-auto) - invisible on a normal-size window, but on a
            maximized/wide window it left real, visible empty space on both
            sides of the page content instead of filling it (marko's report,
            comparing a maximized vs. a smaller window side by side). Content
            now always fills the actual available width - no more max-w cap.
            Pages with their own grids (Dashboard's stat cards, Pulls'
            table) just get proportionally more breathing room on a wide
            monitor, nothing breaks by growing.
            2.6.0: still no max-width cap (that decision stands) - only the
            gutter changed, 24px -> 28px horizontal / 20px vertical, which is
            the app's page inset every screen now shares. */}
        {syncFailure && (
          <div className="flex flex-wrap items-center gap-2 border-b border-red-200 bg-red-50 px-7 py-2 text-xs text-red-800 dark:border-red-500/25 dark:bg-red-500/10 dark:text-red-300">
            <IconAlertTriangle className="h-3.5 w-3.5 shrink-0" />
            <span>
              <strong className="font-semibold">Automatic sync failed.</strong> {syncFailure}
            </span>
            {needsGoogleConsent ? (
              // 2.49.2: the fix for THIS failure is one Google consent, and
              // nothing else will do - a permission that was never granted
              // cannot be produced from this side. So the button is here
              // rather than three screens away, and the sync it was blocking
              // runs the moment it succeeds.
              <button
                type="button"
                disabled={reconsenting}
                onClick={async () => {
                  setReconsenting(true);
                  try {
                    await api.startGoogleSignIn();
                    setSyncFailure(null);
                    setNeedsGoogleConsent(false);
                    toast.success("Google access renewed - syncing now.");
                    void tickRef.current?.();
                  } catch (e) {
                    toast.error(errMsg(e));
                  } finally {
                    setReconsenting(false);
                  }
                }}
                className="rounded-md bg-red-600 px-2 py-1 font-semibold text-white transition hover:bg-red-500 disabled:opacity-60"
              >
                {reconsenting ? "Waiting for Google…" : "Allow Google Drive access"}
              </button>
            ) : (
              <Link to="/settings/data" className="font-semibold underline underline-offset-2">
                Open sync
              </Link>
            )}
            <button
              type="button"
              onClick={() => setSyncFailure(null)}
              className="ml-auto text-red-700/70 hover:text-red-900 dark:text-red-400/70 dark:hover:text-red-200"
            >
              Dismiss
            </button>
          </div>
        )}
        {syncNotice && (
          <div className="flex flex-wrap items-center gap-2 border-b border-amber-200 bg-amber-50 px-7 py-2 text-xs text-amber-800 dark:border-amber-500/25 dark:bg-amber-500/10 dark:text-amber-300">
            <IconAlertTriangle className="h-3.5 w-3.5 shrink-0" />
            {syncNotice}
            <Link to="/settings/data" className="font-semibold underline underline-offset-2">
              Open sync
            </Link>
            <button
              type="button"
              onClick={() => setSyncNotice(null)}
              className="ml-auto text-amber-700/70 hover:text-amber-900 dark:text-amber-400/70 dark:hover:text-amber-200"
            >
              Dismiss
            </button>
          </div>
        )}
        <div className="px-7 py-5">
          <Outlet />
        </div>
      </main>
      <SyncActivity activity={syncActivity} />
      {/* 2.25.0: the guided tour. Mounted HERE because it navigates between
          pages as it runs - anything rendered inside a route would unmount
          itself on its own first step. Renders nothing until started. */}
      <Tour />
    </div>
  );
}

/** 2.17.0: what the app is doing with marko's data, while it does it.
 *
 * The freeze this replaces was not a slow query - every Cloud Sync command was
 * a SYNCHRONOUS Tauri command, and Tauri runs those on the main thread, so
 * pushing a multi-megabyte database over the internet blocked the event loop
 * and the OS drew "Not responding" over the window. The commands are
 * `#[tauri::command(async)]` now, which is the actual fix; this is the other
 * half of it, because an app that is silently busy for eight seconds still
 * looks broken even when it is perfectly responsive.
 *
 * Two shapes on purpose. A background upload gets a corner pill - it must not
 * interrupt anything, marko did not ask for it and can keep working straight
 * through. A download or a merge gets the whole screen, because both end in a
 * restart or a reload: there is nothing useful to click, and a restart that
 * arrives with no warning reads as a crash.
 */
function SyncActivity({ activity }: { activity: { label: string; blocking: boolean } | null }) {
  if (!activity) return null;
  if (!activity.blocking) {
    return (
      <div
        role="status"
        // 2.19.0: bottom CENTRE, not bottom-right. The toast stack is
        // `fixed bottom-4 right-4 z-[100]` (lib/toast.tsx), so this pill was
        // sitting in the same corner underneath it - every toast hid the one
        // thing that was supposed to say the app is busy, which is the exact
        // opposite of what it is for.
        className="pointer-events-none fixed bottom-4 left-1/2 z-40 flex -translate-x-1/2 items-center gap-2 rounded-full border border-slate-200 bg-surface/95 px-3.5 py-2 text-xs text-slate-600 shadow-card backdrop-blur dark:border-slate-700 dark:bg-slate-900/95 dark:text-slate-300"
      >
        <Spinner className="h-3.5 w-3.5 text-brand-500" />
        {activity.label}...
      </div>
    );
  }
  return (
    <div
      role="status"
      // 2.19.0: z-[65], not z-50. Modal is z-50 and ConfirmDialog is z-[60]
      // (ui.tsx), so at z-50 a confirm dialog could sit on top of "the app is
      // about to reload" and be clicked into a database that is being merged
      // underneath it. Still below the updater's own overlay (z-[70]), which
      // outranks everything because it ends the process.
      className="fixed inset-0 z-[65] flex items-center justify-center bg-slate-900/45 backdrop-blur-sm"
    >
      <div className="mx-6 flex max-w-sm flex-col items-center gap-3 rounded-2xl border border-slate-200 bg-surface px-7 py-6 text-center shadow-raised dark:border-slate-700 dark:bg-slate-900">
        <Spinner className="h-7 w-7 text-brand-500" />
        <p className="text-sm font-semibold text-slate-800 dark:text-slate-100">{activity.label}...</p>
        <p className="text-xs leading-relaxed text-slate-500 dark:text-slate-400">
          This can take a moment over a slow connection. A backup of your current data is saved first - you can find
          it in Settings &rarr; Data. The app reloads by itself when it&apos;s done.
        </p>
      </div>
    </div>
  );
}

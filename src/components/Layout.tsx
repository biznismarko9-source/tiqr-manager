import { useEffect, useRef, useState } from "react";
import { Link, NavLink, Outlet, useLocation } from "react-router-dom";
import {
  IconAlertTriangle,
  IconBoxes,
  IconCalendarDays,
  IconChevronDown,
  IconChevronUp,
  IconGauge,
  IconLayoutGrid,
  IconLogOut,
  IconMoon,
  IconPackage,
  IconReceipt,
  IconSettings,
  IconSun,
  IconTag,
  IconTicket,
  IconUsers,
  IconWallet,
} from "./icons";
import { relaunch } from "@tauri-apps/plugin-process";
import { Spinner } from "./ui";
import { checkForUpdate, UPDATE_CHECK_INTERVAL_MS } from "../lib/updater";
import { api } from "../lib/api";
import { useToast } from "../lib/toast";
import { useAuth } from "../lib/auth";
import { useTheme } from "../lib/theme";
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
type NavChild = { to: string; label: string; icon: typeof IconGauge };
type NavItem =
  | { to: string; label: string; icon: typeof IconGauge; end?: boolean }
  | { group: string; icon: typeof IconGauge; children: NavChild[] }
  // 2.13.0: a quiet section label between groups of items. Not a link and
  // not clickable - it exists so the eleven entries read as three short
  // lists instead of one long one. Nothing about which items exist, their
  // order, or where they link changed.
  | { heading: string };

const TICKETS_GROUP_CHILDREN: NavChild[] = [
  { to: "/events", label: "Events", icon: IconCalendarDays },
  { to: "/orders", label: "Orders", icon: IconPackage },
  { to: "/tickets", label: "Tickets", icon: IconTicket },
  { to: "/sales", label: "Sales", icon: IconReceipt },
  { to: "/inventory", label: "Inventory", icon: IconBoxes },
];

const NAV: NavItem[] = [
  { to: "/", label: "Dashboard", icon: IconGauge, end: true },
  { group: "Tickets", icon: IconTicket, children: TICKETS_GROUP_CHILDREN },
  // 2.0.81: marko's own request - "Price Checker musí byť samostatná sekcia
  // v sidebar" (must be its own standalone sidebar section), not folded
  // into Events/Settings.
  { heading: "Market & money" },
  { to: "/price-checker", label: "Price Checker", icon: IconTag },
  { to: "/pulls", label: "Pulls", icon: IconUsers },
  // 2.0.83: same standalone-top-level-section treatment as Price Checker
  // above (not folded into Settings/Dashboard) - Finance is a big enough
  // feature of its own (personal + business money, separate from the
  // Orders/Sales side of the business) to earn its own sidebar entry.
  { to: "/finance", label: "Finance", icon: IconWallet },
  // 2.5.1: marko's own explicit order - Ticket Center sits right after
  // Finance, back out as its own top-level page (see TicketCenter.tsx).
  { heading: "Work" },
  { to: "/ticket-center", label: "Ticket Center", icon: IconLayoutGrid },
  // 2.5.0: "TIQR Operations Calendar" - a cross-domain overview page (every
  // event/order/sale/pull/attention item with a real date). 2.5.1: moved
  // from right after Dashboard to last, per marko's own exact ordering.
  { to: "/calendar", label: "Calendar", icon: IconCalendarDays },
];

// Shared by every actual NavLink below (both the flat top-level items and
// the Tickets group's own children) so the active/hover look can never
// drift between the two - only the group HEADER button (not a real
// NavLink, since "Tickets" has no single route of its own) computes its own
// equivalent class inline, from `ticketsGroupActive` below.
// 2.6.0 (visual redesign): the active state is now a tinted surface plus a
// short accent bar pinned to the item's left edge (see `NAV_ACTIVE_BAR`
// below), instead of a flat brand-tinted pill. The bar is what makes the
// current page findable at a glance in a 192px-wide rail; the tint alone
// was easy to miss next to the hover state, which used a similar weight.
// Nothing about which items exist, their order, or where they link changed.
const NAV_BASE =
  "group relative flex items-center gap-2.5 rounded-lg px-3 py-[5px] text-[12.5px] transition focus:outline-none focus-visible:ring-2 focus-visible:ring-brand-500";
const NAV_ACTIVE =
  "bg-brand-50 font-semibold text-brand-700 dark:bg-brand-500/[0.14] dark:text-brand-300";
const NAV_IDLE =
  "font-medium text-slate-600 hover:bg-slate-100 hover:text-slate-900 dark:text-slate-400 dark:hover:bg-slate-800/70 dark:hover:text-slate-100";
const NAV_ACTIVE_BAR =
  "absolute left-0 top-1/2 h-4 w-[3px] -translate-y-1/2 rounded-r-full bg-brand-600 dark:bg-brand-400";

const navLinkClass = ({ isActive }: { isActive: boolean }) =>
  `${NAV_BASE} ${isActive ? NAV_ACTIVE : NAV_IDLE}`;

export default function Layout() {
  const toast = useToast();
  const { user, logout } = useAuth();
  const location = useLocation();
  const [profileOpen, setProfileOpen] = useState(false);
  const profileRef = useRef<HTMLDivElement>(null);
  // 2.4.4: Tickets group starts expanded (matches every other nav item
  // already always being visible) - purely local, session-only UI state,
  // same convention as e.g. Dashboard's own eventsExpanded/ordersExpanded
  // (not persisted to disk either).
  const [ticketsOpen, setTicketsOpen] = useState(true);
  // 2.13.1 checked and told; 2.14.0 acts - marko asked for the hand-off
  // between his Mac and his Windows PC to stop needing a click. This holds
  // the one sentence the backend could NOT decide by itself (see
  // cloud_sync.rs's `decide_auto`); everything it could decide happens
  // without ever reaching this banner.
  const [syncNotice, setSyncNotice] = useState<string | null>(null);
  // Guards against a tick starting while the previous one is still
  // uploading - a slow upload on a slow connection must not stack.
  const autoSyncBusy = useRef(false);
  // 2.17.0: what the app is doing with marko's data right now, in words. It
  // exists because a whole database crossing the internet takes real seconds
  // and, until this release, they were seconds of nothing - see SyncActivity
  // at the bottom of this file. `blocking` separates "carry on working, this
  // is happening quietly" from "this ends in a restart, there is nothing
  // useful to click".
  const [syncActivity, setSyncActivity] = useState<{ label: string; blocking: boolean } | null>(null);
  const ticketsGroupActive = TICKETS_GROUP_CHILDREN.some(
    (c) => location.pathname === c.to || location.pathname.startsWith(`${c.to}/`),
  );
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
    const tick = async (atStartup: boolean) => {
      if (autoSyncBusy.current) return;
      autoSyncBusy.current = true;
      try {
        const plan = await api.cloudSyncAuto();
        if (cancelled) return;
        if (plan.action === "push") {
          setSyncActivity({ label: "Saving your changes to Google Drive", blocking: false });
          await api.cloudSyncPush();
        } else if (plan.action === "pull" && atStartup) {
          setSyncActivity({ label: "Getting newer data from your other computer", blocking: true });
          const safetyPath = await api.cloudSyncPull();
          toast.success(`Synced down from your other computer. Your previous data was saved to ${safetyPath}. Restarting...`);
          setTimeout(() => relaunch(), 900);
        } else if (plan.action === "merge" && atStartup) {
          // 2.16.0: the case that used to stop and ask which machine wins.
          // Nothing is replaced and nothing is deleted, so there is no
          // question left to put to marko - but the page still has to reload,
          // because rows arrived underneath everything already on screen.
          // A plain reload, not `relaunch()`: the database file was added to,
          // not swapped, so the running process is fine.
          setSyncActivity({ label: "Combining what's on both computers", blocking: true });
          const merged = await api.cloudMergePull();
          const parts = [`Added ${merged.totalInserted} record${merged.totalInserted === 1 ? "" : "s"} from your other computer.`];
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
          setSyncNotice(plan.reason);
        }
      } catch {
        // Offline, signed out, or the remote moved in the moment between
        // deciding and acting: all normal for a local-first app, none of them
        // worth interrupting marko over, and the next tick simply tries
        // again. Real failures are still shown where he asked for them - the
        // Sync buttons in Settings.
      } finally {
        autoSyncBusy.current = false;
        // Left standing on purpose when a restart or reload is already
        // scheduled above: clearing it would flash the app back to normal for
        // a second and make the restart look like a crash.
        setSyncActivity((current) => (current?.blocking ? current : null));
      }
    };
    tick(true);
    const interval = setInterval(() => tick(false), AUTO_SYNC_INTERVAL_MS);
    return () => {
      cancelled = true;
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
      <aside className="flex w-48 shrink-0 flex-col border-r border-slate-200 bg-white dark:border-slate-800 dark:bg-slate-900">
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
            <p className="truncate text-[11px] leading-tight text-slate-400 dark:text-slate-500">Reseller toolkit</p>
          </div>
        </div>

        <nav className="flex-1 space-y-0.5 overflow-y-auto px-2 py-3">
          {NAV.map((item) =>
            "heading" in item ? (
              <p
                key={item.heading}
                className="px-3 pb-1 pt-3.5 text-[10px] font-bold uppercase tracking-[0.12em] text-slate-400 dark:text-slate-500"
              >
                {item.heading}
              </p>
            ) : "children" in item ? (
              /* 2.13.0 (GRP-06): the group gets its own surface, so its five
                 children read as one object rather than five loose rows that
                 happen to be indented. Replaces the 2.6.0 hairline guide rail
                 - the card IS the grouping cue now, so the rail and the deep
                 indent both go. */
              <div
                key="tickets-group"
                className="my-1.5 rounded-xl border border-slate-200 bg-slate-50/70 p-1 dark:border-slate-800 dark:bg-slate-800/30"
              >
                <button
                  type="button"
                  onClick={() => setTicketsOpen((o) => !o)}
                  aria-expanded={ticketsOpen}
                  className={`${NAV_BASE} w-full ${ticketsGroupActive ? NAV_ACTIVE : NAV_IDLE}`}
                >
                  {ticketsGroupActive && <span className={NAV_ACTIVE_BAR} aria-hidden="true" />}
                  <item.icon className="h-[17px] w-[17px] shrink-0" />
                  <span className="flex-1 truncate text-left">{item.group}</span>
                  <IconChevronDown
                    className={`h-3.5 w-3.5 shrink-0 opacity-60 transition-transform ${ticketsOpen ? "" : "-rotate-90"}`}
                  />
                </button>
                {ticketsOpen && (
                  <div className="mt-0.5 space-y-0.5">
                    {item.children.map((child) => (
                      <NavLink key={child.to} to={child.to} className={navLinkClass}>
                        {({ isActive }) => (
                          <>
                            {isActive && <span className={NAV_ACTIVE_BAR} aria-hidden="true" />}
                            <child.icon className="h-[17px] w-[17px] shrink-0" />
                            <span className="truncate">{child.label}</span>
                          </>
                        )}
                      </NavLink>
                    ))}
                  </div>
                )}
              </div>
            ) : (
              <NavLink key={item.to} to={item.to} end={item.end} className={navLinkClass}>
                {({ isActive }) => (
                  <>
                    {isActive && <span className={NAV_ACTIVE_BAR} aria-hidden="true" />}
                    <item.icon className="h-[17px] w-[17px] shrink-0" />
                    <span className="truncate">{item.label}</span>
                  </>
                )}
              </NavLink>
            ),
          )}
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
            <div className="absolute inset-x-2 bottom-full mb-1.5 origin-bottom animate-[pop-in_.16s_ease-out] overflow-hidden rounded-xl border border-slate-200 bg-white p-1 shadow-overlay dark:border-slate-700 dark:bg-slate-800">
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
              <span className="block truncate text-[11px] text-slate-400 dark:text-slate-500">{user?.email ?? ""}</span>
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
        className="pointer-events-none fixed bottom-4 right-4 z-40 flex items-center gap-2 rounded-full border border-slate-200 bg-white/95 px-3.5 py-2 text-xs text-slate-600 shadow-card backdrop-blur dark:border-slate-700 dark:bg-slate-900/95 dark:text-slate-300"
      >
        <Spinner className="h-3.5 w-3.5 text-brand-500" />
        {activity.label}...
      </div>
    );
  }
  return (
    <div
      role="status"
      className="fixed inset-0 z-50 flex items-center justify-center bg-slate-900/45 backdrop-blur-sm"
    >
      <div className="mx-6 flex max-w-sm flex-col items-center gap-3 rounded-2xl border border-slate-200 bg-white px-7 py-6 text-center shadow-raised dark:border-slate-700 dark:bg-slate-900">
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

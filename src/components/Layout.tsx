import { useEffect, useRef, useState } from "react";
import { Link, NavLink, Outlet } from "react-router-dom";
import {
  IconBoxes,
  IconCalendarDays,
  IconCheck,
  IconChevronUp,
  IconGauge,
  IconLogOut,
  IconPackage,
  IconReceipt,
  IconSettings,
  IconTag,
  IconTicket,
  IconUsers,
  IconWallet,
} from "./icons";
import { checkForUpdate } from "../lib/updater";
import { api } from "../lib/api";
import { useToast } from "../lib/toast";
import { useAuth } from "../lib/auth";
import logo from "../assets/logo.png";

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

const NAV = [
  { to: "/", label: "Dashboard", icon: IconGauge, end: true },
  { to: "/events", label: "Events", icon: IconCalendarDays },
  { to: "/orders", label: "Orders", icon: IconPackage },
  { to: "/tickets", label: "Tickets", icon: IconTicket },
  { to: "/sales", label: "Sales", icon: IconReceipt },
  // 2.2.12: marko's own request for one dedicated place to see everything
  // sold that still needs finishing (payment/delivery/both) - placed right
  // after Sales (not standalone-top-level like Price Checker/Finance below)
  // since it's a specialized, narrower view OVER Sales' own data, not an
  // independent feature of its own. Reuses IconCheck (already defined, used
  // elsewhere for a plain checkmark) rather than adding a new icon.
  { to: "/fulfillment", label: "Fulfillment Center", icon: IconCheck },
  { to: "/inventory", label: "Inventory", icon: IconBoxes },
  { to: "/pulls", label: "Pulls", icon: IconUsers },
  // 2.0.81: marko's own request - "Price Checker musí byť samostatná sekcia
  // v sidebar" (must be its own standalone sidebar section), not folded
  // into Events/Settings.
  { to: "/price-checker", label: "Price Checker", icon: IconTag },
  // 2.0.83: same standalone-top-level-section treatment as Price Checker
  // above (not folded into Settings/Dashboard) - Finance is a big enough
  // feature of its own (personal + business money, separate from the
  // Orders/Sales side of the business) to earn its own sidebar entry.
  { to: "/finance", label: "Finance", icon: IconWallet },
];

export default function Layout() {
  const toast = useToast();
  const { user, logout } = useAuth();
  const [profileOpen, setProfileOpen] = useState(false);
  const profileRef = useRef<HTMLDivElement>(null);

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
    checkForUpdate()
      .then((update) => {
        if (update) toast.info(`TIQR Manager ${update.version} is available - open Settings to install it.`);
      })
      .catch(() => {});
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
          and the logo lockup still have comfortable margin at this width. */}
      <aside className="flex w-48 shrink-0 flex-col border-r border-slate-200 bg-white dark:border-slate-800 dark:bg-slate-900">
        <div className="flex items-center gap-2 px-4 py-4">
          <img src={logo} alt="TIQR Manager" className="h-8 w-8 rounded-lg shadow-sm" />
          <div>
            <p className="text-sm font-semibold leading-tight text-slate-900 dark:text-slate-100">TIQR Manager</p>
            <p className="text-[11px] leading-tight text-slate-400 dark:text-slate-500">Reseller toolkit</p>
          </div>
        </div>
        <nav className="flex-1 space-y-0.5 px-2 py-2">
          {NAV.map((item) => (
            <NavLink
              key={item.to}
              to={item.to}
              end={item.end}
              className={({ isActive }) =>
                `flex items-center gap-2.5 rounded-lg px-3 py-2 text-sm font-medium transition-colors ${
                  isActive
                    ? "bg-brand-50 text-brand-700 dark:bg-brand-500/10 dark:text-brand-400"
                    : "text-slate-600 hover:bg-slate-100 hover:text-slate-900 dark:text-slate-400 dark:hover:bg-slate-800 dark:hover:text-slate-100"
                }`
              }
            >
              <item.icon className="h-4 w-4" />
              {item.label}
            </NavLink>
          ))}
        </nav>
        {/* 2.0.44: profile widget - marko's own screenshot pointed at this
            exact spot (previously just the tagline below on its own). The
            dropdown opens UPWARD (bottom-full) since this sits at the very
            bottom of the sidebar - opening down would run off the window. */}
        <div ref={profileRef} className="relative border-t border-slate-100 dark:border-slate-800">
          {profileOpen && (
            // 2.0.74: same "pop-in" entrance as Modal/ConfirmDialog
            // (index.css) - `origin-bottom` so it visibly grows up out of
            // the button it's anchored to (this menu opens upward) instead
            // of scaling from its own center, which would look like it's
            // growing out of thin air above the button.
            <div className="absolute inset-x-2 bottom-full mb-1 origin-bottom animate-[pop-in_.16s_ease-out] overflow-hidden rounded-lg border border-slate-200 bg-white py-1 shadow-lg dark:border-slate-800 dark:bg-slate-900">
              <Link
                to="/settings"
                onClick={() => setProfileOpen(false)}
                className="flex items-center gap-2 px-3 py-2 text-xs font-medium text-slate-600 hover:bg-slate-50 dark:text-slate-300 dark:hover:bg-slate-800"
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
                className="flex w-full items-center gap-2 px-3 py-2 text-left text-xs font-medium text-red-600 hover:bg-red-50 dark:text-red-400 dark:hover:bg-red-500/10"
              >
                <IconLogOut className="h-3.5 w-3.5" /> Log out
              </button>
            </div>
          )}
          <button
            type="button"
            onClick={() => setProfileOpen((o) => !o)}
            className="flex w-full items-center gap-2.5 px-4 py-2.5 text-left hover:bg-slate-50 dark:hover:bg-slate-800/60"
          >
            <span className="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-brand-100 text-[11px] font-semibold text-brand-700 dark:bg-brand-500/20 dark:text-brand-400">
              {initialsFor(user?.name ?? "?")}
            </span>
            <span className="min-w-0 flex-1">
              <span className="block truncate text-xs font-medium text-slate-700 dark:text-slate-300">{user?.name ?? "Account"}</span>
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
            monitor, nothing breaks by growing. */}
        <div className="px-6 py-6">
          <Outlet />
        </div>
      </main>
    </div>
  );
}

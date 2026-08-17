import { useEffect } from "react";
import { NavLink, Outlet } from "react-router-dom";
import {
  IconBoxes,
  IconCalendarDays,
  IconGauge,
  IconPackage,
  IconReceipt,
  IconSettings,
  IconTicket,
} from "./icons";
import { checkForUpdate } from "../lib/updater";
import { useToast } from "../lib/toast";
import logo from "../assets/logo.png";

const NAV = [
  { to: "/", label: "Dashboard", icon: IconGauge, end: true },
  { to: "/events", label: "Events", icon: IconCalendarDays },
  { to: "/orders", label: "Orders", icon: IconPackage },
  { to: "/tickets", label: "Tickets", icon: IconTicket },
  { to: "/sales", label: "Sales", icon: IconReceipt },
  { to: "/inventory", label: "Inventory", icon: IconBoxes },
  { to: "/settings", label: "Settings", icon: IconSettings },
];

export default function Layout() {
  const toast = useToast();

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

  return (
    <div className="flex h-full w-full overflow-hidden bg-slate-50 dark:bg-slate-950">
      <aside className="flex w-56 shrink-0 flex-col border-r border-slate-200 bg-white dark:border-slate-800 dark:bg-slate-900">
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
        <div className="border-t border-slate-100 px-4 py-3 text-[11px] text-slate-400 dark:border-slate-800 dark:text-slate-500">
          Local-first &middot; your data stays on this device
        </div>
      </aside>
      <main className="min-w-0 flex-1 overflow-y-auto">
        <div className="mx-auto max-w-[1400px] px-6 py-6">
          <Outlet />
        </div>
      </main>
    </div>
  );
}

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
  return (
    <div className="flex h-full w-full overflow-hidden bg-slate-50">
      <aside className="flex w-56 shrink-0 flex-col border-r border-slate-200 bg-white">
        <div className="flex items-center gap-2 px-4 py-4">
          <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-brand-600 text-sm font-bold text-white">
            T
          </div>
          <div>
            <p className="text-sm font-semibold leading-tight text-slate-900">TIQR Manager</p>
            <p className="text-[11px] leading-tight text-slate-400">Reseller toolkit</p>
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
                    ? "bg-brand-50 text-brand-700"
                    : "text-slate-600 hover:bg-slate-100 hover:text-slate-900"
                }`
              }
            >
              <item.icon className="h-4 w-4" />
              {item.label}
            </NavLink>
          ))}
        </nav>
        <div className="border-t border-slate-100 px-4 py-3 text-[11px] text-slate-400">
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

import { useState } from "react";
import TicketControlCenter from "../TicketControlCenter";
import FulfillmentCenter from "../FulfillmentCenter";

type TicketCenterSubTab = "control" | "fulfillment";

const SUBTABS: { key: TicketCenterSubTab; label: string }[] = [
  { key: "control", label: "Control Center" },
  { key: "fulfillment", label: "Fulfillment" },
];

// 2.4.4: marko's own request - Ticket Control Center (2.4.3) and
// Fulfillment Center (2.2.12), previously two standalone top-level sidebar
// pages, now live together under Finance as one "Ticket Center" tab with
// two subtabs. Both are reused completely as-is (Control Center's own
// 2.4.4 fixes - sticky header background, Seats column, clickable Order
// cell - are unrelated to this move and documented in its own file); this
// file only supplies the subtab switcher, mirroring the exact same
// tab-shell pattern Finance.tsx itself already uses one level up (see that
// file's own 2.1.0 comment). Neither subtab shares Finance's own
// `FinanceData` load (entries/categories/accounts/...) - each already does
// its own independent data fetching, untouched here.
export default function TicketCenter() {
  const [tab, setTab] = useState<TicketCenterSubTab>("control");

  return (
    <div>
      <div className="mb-4 flex w-fit flex-wrap items-center gap-1 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-1">
        {SUBTABS.map((t) => (
          <button
            key={t.key}
            type="button"
            onClick={() => setTab(t.key)}
            className={`rounded-md px-3.5 py-1.5 text-sm font-medium transition-colors ${
              tab === t.key ? "bg-brand-600 text-white" : "text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800"
            }`}
          >
            {t.label}
          </button>
        ))}
      </div>

      {tab === "control" && <TicketControlCenter />}
      {tab === "fulfillment" && <FulfillmentCenter />}
    </div>
  );
}

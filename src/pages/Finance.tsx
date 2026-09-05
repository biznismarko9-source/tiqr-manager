import { useEffect, useState } from "react";
import { api, errMsg } from "../lib/api";
import { PageHeader } from "../components/ui";
import { useToast } from "../lib/toast";
import type { FinanceData } from "./finance/shared";
import Overview from "./finance/Overview";
import Transactions from "./finance/Transactions";
import Accounts from "./finance/Accounts";
import Reports from "./finance/Reports";
import TicketCenter from "./finance/TicketCenter";

// 2.1.0: marko's own "FINANCE 2.1" request - Finance is now 4 simple tabs
// (Overview/Transactions/Accounts/Reports, Overview shown by default) rather
// than one long page. This file is now just a thin shell: it owns the ONE
// shared data load (entries/categories/accounts/transfers/recurring - see
// `FinanceData` in finance/shared.ts) and the tab strip; every tab's own
// UI/behaviour lives in its own file under finance/. This mirrors the
// existing "impl function + thin command wrapper" split already used
// throughout the Rust backend, just applied to the frontend: one shared
// load, several focused views over the same data.
//
// Nothing about the ORIGINAL Finance feature's own logic changed here -
// Overview.tsx's stat cards/charts, Transactions.tsx's entry list/filters,
// and the EntryFormModal are the exact same code that used to live directly
// in this file, just moved. See PROTECTED-AREAS-NOTES.md's 2.1.0 section
// for the full file-by-file breakdown.

type FinanceTab = "overview" | "transactions" | "accounts" | "reports" | "ticketCenter";

const TABS: { key: FinanceTab; label: string }[] = [
  { key: "overview", label: "Overview" },
  { key: "transactions", label: "Transactions" },
  { key: "accounts", label: "Accounts" },
  { key: "reports", label: "Reports" },
  // 2.4.4: marko's own request - Ticket Control Center + Fulfillment Center
  // merged here as one tab with 2 subtabs (see finance/TicketCenter.tsx).
  // Unlike the 4 tabs above, this one doesn't read `tabProps`/`FinanceData`
  // at all - both subtabs already do their own independent data fetching.
  { key: "ticketCenter", label: "Ticket Center" },
];

export default function Finance() {
  const toast = useToast();
  const [tab, setTab] = useState<FinanceTab>("overview");
  const [data, setData] = useState<Omit<FinanceData, "loading" | "reload">>({
    entries: [],
    categories: [],
    accounts: [],
    transfers: [],
    recurringExpenses: [],
  });
  const [loading, setLoading] = useState(true);

  const load = () => {
    setLoading(true);
    Promise.all([
      api.listFinanceEntries(),
      api.listFinanceCategories(),
      api.listAccounts(),
      api.listTransfers(),
      api.listRecurringExpenses(),
    ])
      .then(([entries, categories, accounts, transfers, recurringExpenses]) => {
        setData({ entries, categories, accounts, transfers, recurringExpenses });
      })
      .catch((e) => toast.error(errMsg(e)))
      .finally(() => setLoading(false));
  };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  useEffect(load, []);

  const tabProps: FinanceData = { ...data, loading, reload: load };

  return (
    <div>
      <PageHeader title="Finance" subtitle="Personal and business money, tracked in one place." />

      <div className="mb-6 flex w-fit flex-wrap items-center gap-1 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-1">
        {TABS.map((t) => (
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

      {tab === "overview" && <Overview {...tabProps} />}
      {tab === "transactions" && <Transactions {...tabProps} />}
      {tab === "accounts" && <Accounts {...tabProps} />}
      {tab === "reports" && <Reports {...tabProps} />}
      {tab === "ticketCenter" && <TicketCenter />}
    </div>
  );
}

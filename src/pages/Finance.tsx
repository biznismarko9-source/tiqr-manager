import { useEffect, useState } from "react";
import { api, errMsg } from "../lib/api";
import { useDataChanged } from "../lib/dataChanged";
import { PageHeader, SEGMENTED_TRACK, segmentedItemClass } from "../components/ui";
import { useToast } from "../lib/toast";
import type { FinanceData } from "./finance/shared";
import Overview from "./finance/Overview";
import Transactions from "./finance/Transactions";
import Accounts from "./finance/Accounts";
import Reports from "./finance/Reports";

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
//
// 2.4.4 briefly added a 5th "Ticket Center" tab here (Control Center +
// Fulfillment Center merged, see that version's report). 2.5.1 moved it back
// out to its own top-level page/route - marko's own explicit request, and it
// no longer shares any code with the old per-ticket pages it replaced (see
// TicketCenter.tsx's own module doc comment). Back to the original 4 tabs.

type FinanceTab = "overview" | "transactions" | "accounts" | "reports";

const TABS: { key: FinanceTab; label: string }[] = [
  { key: "overview", label: "Overview" },
  { key: "transactions", label: "Transactions" },
  { key: "accounts", label: "Accounts" },
  { key: "reports", label: "Reports" },
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

  // 2.112.0: `quiet` refreshes in place, for changes from the other computer.
  const load = (quiet = false) => {
    if (!quiet) setLoading(true);
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
  useEffect(() => load(), []);
  // 2.112.0: changes from the other computer, shown without a click.
  useDataChanged(() => load(true));

  const tabProps: FinanceData = { ...data, loading, reload: () => load() };

  return (
    <div>
      {/* 2.34.2: the four tabs moved into PageHeader's own `actions` slot, so
          they sit to the right of "Finance" instead of on a row of their own
          underneath it - marko asked for the header band to take less height.
          Same buttons, same handler; only where they render changed. */}
      <PageHeader
        title="Finance"
        subtitle="Personal and business money, tracked in one place."
        actions={
          <>
            {/* 2.85.0 (global consistency pass): was a hand-rolled pill row with
              its own border, sizes and hover colours, while the rest of the
              app - including this section's own sub-pages - segments with
              SEGMENTED_TRACK + segmentedItemClass. The active state was
              already `bg-brand-600 text-white`, byte-identical; the track,
              inactive colours and focus ring move onto the shared tokens.
              Same buttons, same handler, same state. */}
            <div className={SEGMENTED_TRACK}>
              {TABS.map((t) => (
                <button
                  key={t.key}
                  type="button"
                  onClick={() => setTab(t.key)}
                  aria-pressed={tab === t.key}
                  className={segmentedItemClass(tab === t.key)}
                >
                  {t.label}
                </button>
              ))}
            </div>
          </>
        }
      />

      {tab === "overview" && <Overview {...tabProps} />}
      {tab === "transactions" && <Transactions {...tabProps} />}
      {tab === "accounts" && <Accounts {...tabProps} />}
      {tab === "reports" && <Reports {...tabProps} />}
    </div>
  );
}

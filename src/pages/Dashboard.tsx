import { useCallback, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { api, errMsg } from "../lib/api";
import type { DashboardData } from "../lib/types";
import { formatDate, formatMoney, formatPercent } from "../lib/format";
import { Badge, Card, EmptyState, LoadingBlock, PageHeader, StatCard } from "../components/ui";
import { IconCalendarDays, IconPackage, IconReceipt } from "../components/icons";
import { useToast } from "../lib/toast";

const PERIODS: { key: string; label: string }[] = [
  { key: "today", label: "Today" },
  { key: "7d", label: "Last 7 days" },
  { key: "30d", label: "Last 30 days" },
  { key: "month", label: "This month" },
  { key: "all", label: "All time" },
  { key: "custom", label: "Custom" },
];

export default function Dashboard() {
  const toast = useToast();
  const [period, setPeriod] = useState("30d");
  const [from, setFrom] = useState("");
  const [to, setTo] = useState("");
  const [data, setData] = useState<DashboardData | null>(null);
  const [loading, setLoading] = useState(true);

  const load = useCallback(() => {
    setLoading(true);
    api
      .getDashboard({ period, from: from || undefined, to: to || undefined })
      .then(setData)
      .catch((e) => toast.error(errMsg(e)))
      .finally(() => setLoading(false));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [period, from, to]);

  useEffect(() => {
    load();
  }, [load]);

  return (
    <div>
      <PageHeader
        title="Dashboard"
        subtitle="A real-time snapshot of your ticket reselling business."
        actions={
          <div className="flex flex-wrap items-center gap-1 rounded-lg border border-slate-200 bg-white p-1">
            {PERIODS.map((p) => (
              <button
                key={p.key}
                onClick={() => setPeriod(p.key)}
                className={`rounded-md px-2.5 py-1.5 text-xs font-medium transition-colors ${
                  period === p.key ? "bg-brand-600 text-white" : "text-slate-600 hover:bg-slate-100"
                }`}
              >
                {p.label}
              </button>
            ))}
          </div>
        }
      />

      {period === "custom" && (
        <Card className="mb-4 flex flex-wrap items-end gap-3 p-3">
          <label className="text-xs font-medium text-slate-600">
            From
            <input
              type="date"
              value={from}
              onChange={(e) => setFrom(e.target.value)}
              className="input mt-1"
            />
          </label>
          <label className="text-xs font-medium text-slate-600">
            To
            <input type="date" value={to} onChange={(e) => setTo(e.target.value)} className="input mt-1" />
          </label>
        </Card>
      )}

      {loading || !data ? (
        <LoadingBlock label="Loading dashboard..." />
      ) : (
        <>
          <p className="mb-2 text-xs font-semibold uppercase tracking-wide text-slate-400">
            Activity {data.periodFrom} &rarr; {data.periodTo}
          </p>
          <div className="mb-6 grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-6">
            <StatCard label="Revenue" value={formatMoney(data.period.revenueCents, "EUR")} />
            <StatCard label="Purchase cost" value={formatMoney(data.period.totalCostCents, "EUR")} />
            <StatCard
              label="Profit"
              value={formatMoney(data.period.profitCents, "EUR")}
              tone={data.period.profitCents > 0 ? "positive" : data.period.profitCents < 0 ? "negative" : "default"}
            />
            <StatCard label="Margin" value={formatPercent(data.period.margin)} />
            <StatCard label="ROI" value={formatPercent(data.period.roi)} />
            <StatCard
              label="Tickets sold"
              value={String(data.period.soldTickets)}
              sub={`${data.period.purchasedTickets} purchased in period`}
            />
          </div>

          <p className="mb-2 text-xs font-semibold uppercase tracking-wide text-slate-400">
            Current inventory (all time)
          </p>
          <div className="mb-8 grid grid-cols-2 gap-3 sm:grid-cols-4">
            <StatCard label="Available" value={String(data.inventory.availableTickets)} />
            <StatCard label="Listed" value={String(data.inventory.listedTickets)} />
            <StatCard label="Sold (total)" value={String(data.inventory.soldTickets)} />
            <StatCard label="Purchased (total)" value={String(data.inventory.purchasedTickets)} />
          </div>

          <div className="grid grid-cols-1 gap-5 lg:grid-cols-3">
            <RecentCard title="Recent events" icon={<IconCalendarDays className="h-4 w-4" />}>
              {data.recentEvents.length === 0 ? (
                <EmptyRow text="No events yet" />
              ) : (
                <ul className="divide-y divide-slate-100">
                  {data.recentEvents.map((ev) => (
                    <li key={ev.id}>
                      <Link
                        to={`/events/${ev.id}`}
                        className="flex items-center justify-between gap-2 px-4 py-2.5 hover:bg-slate-50"
                      >
                        <div className="min-w-0">
                          <p className="truncate text-sm font-medium text-slate-800">{ev.name}</p>
                          <p className="text-xs text-slate-400">{formatDate(ev.eventDate)}</p>
                        </div>
                        <Badge tone={ev.status}>{ev.status}</Badge>
                      </Link>
                    </li>
                  ))}
                </ul>
              )}
            </RecentCard>

            <RecentCard title="Recent orders" icon={<IconPackage className="h-4 w-4" />}>
              {data.recentOrders.length === 0 ? (
                <EmptyRow text="No orders yet" />
              ) : (
                <ul className="divide-y divide-slate-100">
                  {data.recentOrders.map((o) => (
                    <li key={o.id}>
                      <Link
                        to={`/orders/${o.id}`}
                        className="flex items-center justify-between gap-2 px-4 py-2.5 hover:bg-slate-50"
                      >
                        <div className="min-w-0">
                          <p className="truncate text-sm font-medium text-slate-800">{o.code}</p>
                          <p className="truncate text-xs text-slate-400">{o.eventName}</p>
                        </div>
                        <p className="shrink-0 text-sm tabular-nums text-slate-600">
                          {formatMoney(o.totalCostCents, o.currency)}
                        </p>
                      </Link>
                    </li>
                  ))}
                </ul>
              )}
            </RecentCard>

            <RecentCard title="Recent sales" icon={<IconReceipt className="h-4 w-4" />}>
              {data.recentSales.length === 0 ? (
                <EmptyRow text="No sales yet" />
              ) : (
                <ul className="divide-y divide-slate-100">
                  {data.recentSales.map((s) => (
                    <li key={s.id} className="flex items-center justify-between gap-2 px-4 py-2.5">
                      <div className="min-w-0">
                        <p className="truncate text-sm font-medium text-slate-800">{s.eventName}</p>
                        <p className="text-xs text-slate-400">{formatDate(s.saleDate)}</p>
                      </div>
                      <p className="shrink-0 text-sm tabular-nums text-emerald-600">
                        {formatMoney(s.salePriceCents, s.currency)}
                      </p>
                    </li>
                  ))}
                </ul>
              )}
            </RecentCard>
          </div>
        </>
      )}
    </div>
  );
}

function RecentCard({
  title,
  icon,
  children,
}: {
  title: string;
  icon: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <Card>
      <div className="flex items-center gap-2 border-b border-slate-100 px-4 py-3">
        <span className="text-slate-400">{icon}</span>
        <h3 className="text-sm font-semibold text-slate-800">{title}</h3>
      </div>
      {children}
    </Card>
  );
}

function EmptyRow({ text }: { text: string }) {
  return (
    <div className="p-4">
      <EmptyState title={text} />
    </div>
  );
}

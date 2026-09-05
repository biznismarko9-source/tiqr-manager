import { useEffect, useMemo, useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import { api, errMsg } from "../lib/api";
import type { SaleGroup } from "../lib/types";
import { formatMoneyOrMixed, formatSeatsSummary } from "../lib/format";
import { Badge, Card, EmptyState, LoadingBlock } from "../components/ui";
import { EventCategoryBadge } from "../components/EventCategoryBadge";
import { isSaleGroupDone } from "./Sales";
import { useToast } from "../lib/toast";

// 2.2.12: "Fulfillment Center" - marko's own request for one dedicated place
// to see everything sold that still needs finishing (payment/delivery/both)
// after the sale itself. Deliberately reuses EVERYTHING that already exists:
// the same `SaleGroup[]` Sales.tsx already fetches (`api.listSaleGroups`),
// the same `isSaleGroupDone` Sales.tsx's own Pending/Completed tabs already
// use (imported from there, now exported - see Sales.tsx's own 2.2.12
// comment on that function), and the same
// Badge/STATUS_TONES color language every other page already uses. No new
// backend command, no new migration, no new/parallel status system - every
// number on this page is derived, at render time, from fields that already
// exist on `SaleGroup`.
//
// "Ready to complete" is this page's one genuinely NEW concept, and it is a
// pure display derivation, not a stored status: within the Pending set (see
// isSaleGroupDone), a group is "ready" once it's fully paid AND fully
// delivered. The only way such a group can still be Pending at all is
// SaleGroup.soldCount falling short of ticketCount - which only happens
// after a PARTIAL refund (see that field's own doc comment in lib/types.ts)
// - so "ready to complete" in practice means "this batch just needs its
// remaining refund/resell bookkeeping looked at," never a group that's
// actually still missing payment or delivery. This page never touches
// refund/resell logic itself - it only surfaces the existing paid/delivered
// counts that already say a group has reached this point.
export function isReadyToComplete(g: SaleGroup): boolean {
  return g.paidCount === g.ticketCount && g.deliveredCount === g.ticketCount;
}

export type FulfillmentCategoryKey = "all" | "payment" | "delivery" | "ready";

// 2.2.12: doubles as BOTH the 4 KPI numbers marko asked for (Pending Sales/
// Awaiting Payment/Awaiting Delivery/Ready to Complete) AND the 4 clickable
// filter categories he separately listed (ALL PENDING/PAYMENT/DELIVERY/READY
// TO COMPLETE) - they are the exact same 4 counts under two names, so this
// page shows each number exactly once, as a clickable tile, rather than
// duplicating it as a plain KPI card AND a separate filter pill. Reuses the
// same clickable-tile pattern 2.2.11 just established for the Dashboard's
// Attention Center boxes, for a consistent, simple look across both new
// screens - not a new UI pattern of its own.
const FULFILLMENT_CATEGORIES: { key: FulfillmentCategoryKey; title: string; subtext: string }[] = [
  { key: "all", title: "Pending Sales", subtext: "Sold tickets not yet fully paid, delivered, and completed" },
  { key: "payment", title: "Awaiting Payment", subtext: "Pending sales still missing full payment" },
  { key: "delivery", title: "Awaiting Delivery", subtext: "Pending sales still missing delivery" },
  { key: "ready", title: "Ready to Complete", subtext: "Fully paid and delivered - just needs finishing" },
];

// 2.2.12: Awaiting Payment and Awaiting Delivery are deliberately NOT
// mutually exclusive - a group missing both counts under both categories at
// once (marko's own explicit test case, "oboje pending"). This mirrors how
// 2.2.11's Attention Center categories already work as independent lenses
// over the same data rather than a strict partition - not a new convention
// invented for this page.
export function matchesFulfillmentCategory(g: SaleGroup, key: FulfillmentCategoryKey): boolean {
  if (key === "all") return true;
  if (key === "payment") return g.paidCount !== g.ticketCount;
  if (key === "delivery") return g.deliveredCount !== g.ticketCount;
  return isReadyToComplete(g);
}

/** Same label/value/sub visual language as ui.tsx's StatCard, wrapped in a
 * real `<button>` - one of these 4 is always the active filter (unlike
 * Attention Center's boxes, there's no "deselect" state here, since this
 * page always needs to show SOME table). */
function FulfillmentCategoryCard({
  title,
  subtext,
  count,
  selected,
  onSelect,
}: {
  title: string;
  subtext: string;
  count: number;
  selected: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onSelect}
      className={`rounded-xl border p-4 text-left transition-colors ${
        selected
          ? "border-brand-500 bg-brand-50/60 dark:border-brand-500 dark:bg-brand-500/10"
          : "border-slate-200 bg-white hover:bg-slate-50 dark:border-slate-800 dark:bg-slate-900 dark:hover:bg-slate-800/60"
      }`}
    >
      <p className="text-xs font-medium uppercase tracking-wide text-slate-400 dark:text-slate-500">{title}</p>
      <p className="mt-1.5 text-2xl font-semibold tabular-nums text-slate-900 dark:text-slate-100">{count}</p>
      <p className="mt-1 text-xs text-slate-400 dark:text-slate-500">{subtext}</p>
    </button>
  );
}

/** Delivery-status summary badge for a whole SaleGroup - the one column
 * Sales.tsx's own table has no direct equivalent for (it only shows
 * Payment status + the combined Sold/Delivered/Paid "Completed" badge, never
 * delivery on its own). Reuses the exact same tone keys `InlineStatusSelect`
 * already uses per-ticket on Sale/Order Detail (`delivered`/"not delivered"
 * in ui.tsx's STATUS_TONES) - no new color, and "mixed" (already used
 * elsewhere for a group whose lines disagree) for a partially-delivered
 * batch. */
function deliveryStatusBadge(g: SaleGroup) {
  if (g.deliveredCount === g.ticketCount) return <Badge tone="delivered">Delivered</Badge>;
  if (g.deliveredCount === 0) return <Badge tone="not delivered">Not delivered</Badge>;
  return (
    <Badge tone="mixed" title={`${g.deliveredCount} of ${g.ticketCount} delivered`}>
      {g.deliveredCount}/{g.ticketCount} delivered
    </Badge>
  );
}

export default function FulfillmentCenter() {
  const toast = useToast();
  const navigate = useNavigate();
  const [groups, setGroups] = useState<SaleGroup[] | null>(null);
  const [category, setCategory] = useState<FulfillmentCategoryKey>("all");

  useEffect(() => {
    let cancelled = false;
    api
      .listSaleGroups({})
      .then((g) => {
        if (!cancelled) setGroups(g);
      })
      .catch((e) => toast.error(errMsg(e)));
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Every sale group not yet done by the app's own existing rule - the exact
  // same `isSaleGroupDone` Sales.tsx's Pending tab already uses, imported
  // from there rather than re-implemented, so this page can never disagree
  // with Sales.tsx about what counts as done (marko's own explicit
  // "existujúce Sales Completed/Pending pravidlo musí zostať konzistentné").
  // A fully refunded group is "done" under that same rule and so never
  // appears here, same as it never appears in Sales' own Pending tab.
  const pending = useMemo(() => (groups ? groups.filter((g) => !isSaleGroupDone(g)) : null), [groups]);

  const counts = useMemo(() => {
    if (!pending) return null;
    return {
      all: pending.length,
      payment: pending.filter((g) => matchesFulfillmentCategory(g, "payment")).length,
      delivery: pending.filter((g) => matchesFulfillmentCategory(g, "delivery")).length,
      ready: pending.filter((g) => matchesFulfillmentCategory(g, "ready")).length,
    };
  }, [pending]);

  const visible = useMemo(
    () => (pending ? pending.filter((g) => matchesFulfillmentCategory(g, category)) : []),
    [pending, category],
  );

  const activeCategory = FULFILLMENT_CATEGORIES.find((c) => c.key === category)!;

  return (
    <div>
      {/* 2.4.4: own PageHeader removed - no longer a standalone top-level
          route, now mounted inside Finance's "Ticket Center" tab as the
          "Fulfillment" subtab (see finance/TicketCenter.tsx), which already
          labels it - same reasoning as TicketControlCenter's own 2.4.4
          comment. */}
      {!pending || !counts ? (
        <LoadingBlock label="Loading fulfillment center..." />
      ) : (
        <>
          <div className="mb-6 grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-4">
            {FULFILLMENT_CATEGORIES.map((c) => (
              <FulfillmentCategoryCard
                key={c.key}
                title={c.title}
                subtext={c.subtext}
                count={counts[c.key]}
                selected={category === c.key}
                onSelect={() => setCategory(c.key)}
              />
            ))}
          </div>

          {visible.length === 0 ? (
            <Card className="p-4">
              <EmptyState
                title={counts.all === 0 ? "Nothing pending right now" : `Nothing in "${activeCategory.title}" right now`}
                description={
                  counts.all === 0
                    ? "Every sale is fully sold, paid, and delivered (or fully refunded)."
                    : "Try one of the other categories above."
                }
              />
            </Card>
          ) : (
            <div className="overflow-x-auto rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-sm">
              <table className="w-full table-fixed border-collapse">
                <colgroup>
                  <col className="w-[22%]" />
                  <col className="w-[20%]" />
                  <col className="w-[12%]" />
                  <col className="w-[13%]" />
                  <col className="w-[13%]" />
                  <col className="w-[12%]" />
                  <col className="w-[8%]" />
                </colgroup>
                <thead className="border-b border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-800/60">
                  <tr>
                    <th className="th-c">Event</th>
                    <th className="th-c">Ticket / Seats</th>
                    <th className="th-c text-right">Sale price</th>
                    <th className="th-c">Payment status</th>
                    <th className="th-c">Delivery status</th>
                    <th className="th-c">Overall status</th>
                    <th className="th-c" />
                  </tr>
                </thead>
                <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
                  {visible.map((g) => {
                    const seatsSummary = formatSeatsSummary(g.seats);
                    // Every row here already failed isSaleGroupDone (see
                    // `pending` above), so completionStatus(...) can never
                    // itself read "completed" for a row reaching this table -
                    // the only two real states left are "ready" (paid +
                    // delivered, just the partial-refund edge case keeping
                    // `pending` true - see isReadyToComplete's own doc
                    // comment) or genuinely still "pending".
                    const overall = isReadyToComplete(g)
                      ? { tone: "completed", label: "Ready to complete" }
                      : { tone: "pending", label: "Pending" };
                    return (
                      <tr
                        key={g.id}
                        onClick={() => navigate(`/sales/${g.id}`)}
                        className="cursor-pointer hover:bg-slate-50 dark:hover:bg-slate-800/60"
                      >
                        <td className="td-c" title={g.eventId && g.eventName ? g.eventName : undefined}>
                          {g.eventId && g.eventName ? (
                            <div className="flex items-center gap-1.5">
                              <span className="truncate">{g.eventName}</span>
                              {g.categoryName && g.categoryColorSlot !== null && (
                                <span className="shrink-0">
                                  <EventCategoryBadge name={g.categoryName} colorSlot={g.categoryColorSlot} />
                                </span>
                              )}
                            </div>
                          ) : (
                            <span className="italic text-slate-400 dark:text-slate-500">Mixed events</span>
                          )}
                        </td>
                        <td className="td-c">
                          <div className="truncate">
                            {g.ticketCount} ticket{g.ticketCount === 1 ? "" : "s"}
                          </div>
                          {seatsSummary && (
                            <div className="truncate text-xs text-slate-400 dark:text-slate-500" title={seatsSummary}>
                              {seatsSummary}
                            </div>
                          )}
                        </td>
                        <td className="td-c text-right tabular-nums whitespace-nowrap">
                          {formatMoneyOrMixed(g.revenueCents, g.currency)}
                          {g.refundedCount > 0 && (
                            <p
                              className="mt-0.5 truncate text-[11px] font-medium text-amber-700 dark:text-amber-400"
                              title={`${g.refundedCount} of ${g.ticketCount} refunded`}
                            >
                              {g.refundedCount}/{g.ticketCount} refunded
                            </p>
                          )}
                        </td>
                        <td className="td-c">
                          {g.paymentStatus ? <Badge tone={g.paymentStatus}>{g.paymentStatus}</Badge> : <Badge tone="mixed">Mixed</Badge>}
                        </td>
                        <td className="td-c">{deliveryStatusBadge(g)}</td>
                        <td className="td-c">
                          <Badge tone={overall.tone}>{overall.label}</Badge>
                        </td>
                        <td className="td-c text-right">
                          <Link
                            to={`/sales/${g.id}`}
                            onClick={(e) => e.stopPropagation()}
                            className="inline-flex items-center justify-center rounded-lg border border-slate-300 bg-white px-2.5 py-1 text-xs font-medium text-slate-700 hover:bg-slate-50 dark:border-slate-700 dark:bg-slate-900 dark:text-slate-300 dark:hover:bg-slate-800"
                          >
                            Open
                          </Link>
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          )}
        </>
      )}
    </div>
  );
}

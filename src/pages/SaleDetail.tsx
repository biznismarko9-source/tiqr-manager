import { useCallback, useEffect, useMemo, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { api, errMsg } from "../lib/api";
import type { Platform, Sale, SaleEditInput, SalePaymentStatus } from "../lib/types";
import { formatDate, formatMoney, formatMoneyOrMixed, formatPercentOrMixed, formatSeatLocation } from "../lib/format";
import {
  Badge,
  Button,
  CHECKBOX_CLASS,
  Card,
  ConfirmDialog,
  EmptyState,
  Field,
  Input,
  LoadingBlock,
  Modal,
  ModalFooter,
  Select,
  Textarea,
} from "../components/ui";
import { LookupSelect } from "../components/LookupSelect";
import { IconArrowLeft, IconTrash } from "../components/icons";
import { useToast } from "../lib/toast";
import { useNarrowTables } from "../lib/useNarrowTables";

/** Returns a value only when every line shares it, else null ("Mixed" in the UI). */
function uniform<T>(lines: Sale[], pick: (s: Sale) => T): T | null {
  if (lines.length === 0) return null;
  const first = pick(lines[0]);
  return lines.every((l) => pick(l) === first) ? first : null;
}

export default function SaleDetail() {
  const { id } = useParams();
  const saleId = Number(id);
  const navigate = useNavigate();
  const toast = useToast();
  const isNarrow = useNarrowTables();

  const [lines, setLines] = useState<Sale[] | null>(null);
  const [editTarget, setEditTarget] = useState<Sale | null>(null);
  const [refundTarget, setRefundTarget] = useState<Sale | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<Sale | null>(null);
  const [deleting, setDeleting] = useState(false);
  // 1.7.3: delete the whole sale (every line) at once, instead of one at a
  // time - separate from the per-line deleteTarget/deleting above.
  const [groupDeleteOpen, setGroupDeleteOpen] = useState(false);
  const [groupDeleting, setGroupDeleting] = useState(false);
  // 1.8.3: bulk selection - holds TICKET ids (s.ticketId), same as the
  // per-row checkboxes below key off. 1.9.2: the bulk action that consumes
  // this changed (payment-status only now, see SalePaymentStatusBar below)
  // but the selection itself is untouched - a Sale line and its ticket are
  // 1:1 here, so SalePaymentStatusBar just maps ticketId -> sale id itself
  // rather than this state needing to change shape.
  const [selected, setSelected] = useState<Set<number>>(new Set());

  const load = useCallback(() => {
    // Every reload (mount, delete, refund, edit, or a bulk edit just
    // applied) starts from a clean selection - a stale id could otherwise
    // point at a line that no longer exists here after a delete/refund.
    setSelected(new Set());
    api
      .listSalesByGroup(saleId)
      .then(setLines)
      .catch((e) => toast.error(errMsg(e)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [saleId]);

  useEffect(() => {
    load();
  }, [load]);

  const header = useMemo(() => {
    if (!lines || lines.length === 0) return null;
    const counted = lines.filter((s) => s.paymentStatus !== "refunded");
    // 1.6.0 audit H5: mirror the backend's GROUP_BASE_SELECT fix (sales.rs) -
    // currency must be derived from non-refunded lines only, the same scope
    // as the money fields below. Deriving it from ALL lines (including
    // refunded ones) meant a batch whose ONLY differently-currencied line
    // had been refunded still showed Mixed for money/margin/ROI, even though
    // what's left is a clean, fully-computable single-currency total. Falls
    // back to ALL lines only when the whole group is refunded (counted is
    // empty), so a fully-refunded single-currency group still reports its
    // currency instead of going blank.
    const currency = counted.length > 0 ? uniform(counted, (s) => s.currency) : uniform(lines, (s) => s.currency);
    const eventId = uniform(lines, (s) => s.eventId);
    const eventName = uniform(lines, (s) => s.eventName);
    const saleDate = uniform(lines, (s) => s.saleDate);
    const platformName = uniform(lines, (s) => s.platformName);
    const paymentStatus = uniform(lines, (s) => s.paymentStatus);
    const refundedCount = lines.filter((s) => s.paymentStatus === "refunded").length;
    // 1.9.0 (section 6, "Sale Detail"): Paid/Outstanding amounts - same
    // "only sum within one shared currency" rule as revenue/cost/profit
    // below, just scoped to the paid-only / pending-only subset of lines
    // instead of "every non-refunded line". Refunded lines fall into
    // neither bucket, so they can never end up counted as outstanding (or
    // paid). Falls back to the group's own `currency` (computed above via
    // `counted`, itself possibly null/Mixed) when a subset is empty, so an
    // honestly-zero Paid or Outstanding is never mislabeled "Mixed" just
    // because that subset has no lines - it can still legitimately show
    // "Mixed" the same way the rest of this card does, when the group as a
    // whole spans more than one currency.
    const paidLines = lines.filter((s) => s.paymentStatus === "paid");
    const pendingLines = lines.filter((s) => s.paymentStatus === "pending");
    const paidCents = paidLines.reduce((sum, s) => sum + s.salePriceCents, 0);
    const outstandingCents = pendingLines.reduce((sum, s) => sum + s.salePriceCents, 0);
    const paidCurrency = paidLines.length > 0 ? uniform(paidLines, (s) => s.currency) : currency;
    const outstandingCurrency = pendingLines.length > 0 ? uniform(pendingLines, (s) => s.currency) : currency;
    const revenueCents = counted.reduce((sum, s) => sum + s.salePriceCents, 0);
    const feesCents = counted.reduce((sum, s) => sum + s.sellingFeesCents, 0);
    const costCents = counted.reduce((sum, s) => sum + s.costCents, 0);
    const profitCents = counted.reduce((sum, s) => sum + s.profitCents, 0);
    // BUG #6: mirror the backend's SaleGroup rule (see map_sale_group in
    // sales.rs) - margin/ROI are only meaningful when every line here shares
    // one currency. `currency` above is already null ("Mixed") whenever the
    // lines don't, via the same `uniform()` helper used for every other
    // group-level field on this page, so reuse that instead of computing a
    // currency-blind ratio across e.g. EUR + USD.
    const margin = currency !== null && revenueCents !== 0 ? profitCents / revenueCents : null;
    const roi = currency !== null && costCents !== 0 ? profitCents / costCents : null;
    // The representative code is always the group's own lowest-id surviving
    // line's code (see backend GROUP_BASE_SELECT's MIN(s.code) - lines here
    // are already ordered by id ASC, so lines[0] is that same row). 1.6.0
    // audit finding: this used to prefer lines[0].batchId, a static value
    // copied once at creation time - correct for an untouched batch (where
    // it equals the original lowest code anyway), but stale after deleting
    // exactly that lowest-id row, since batchId doesn't shift to the next
    // surviving line the way lines[0].code (freshly fetched every load)
    // does. Always using lines[0].code matches the backend in every case.
    const code = lines[0].code;
    return {
      code,
      currency,
      eventId,
      eventName,
      saleDate,
      platformName,
      paymentStatus,
      refundedCount,
      paidCents,
      outstandingCents,
      paidCurrency,
      outstandingCurrency,
      revenueCents,
      feesCents,
      costCents,
      profitCents,
      margin,
      roi,
    };
  }, [lines]);

  if (lines === null || header === null) return <LoadingBlock />;

  // 1.8.3: bulk ticket actions - refunded lines are excluded from selection,
  // mirroring the existing rule below that already hides their per-line
  // Edit/Refund buttons (a refunded line is history, not something to
  // bulk-edit from this page).
  const selectableLines = lines.filter((s) => s.paymentStatus !== "refunded");
  const allSelectableSelected = selectableLines.length > 0 && selectableLines.every((s) => selected.has(s.ticketId));
  const toggleSelectAll = () => {
    setSelected(allSelectableSelected ? new Set() : new Set(selectableLines.map((s) => s.ticketId)));
  };
  const toggleOne = (ticketId: number) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(ticketId)) next.delete(ticketId);
      else next.add(ticketId);
      return next;
    });
  };

  return (
    <div>
      <Link to="/sales" className="mb-3 inline-flex items-center gap-1 text-sm text-slate-500 dark:text-slate-400 hover:text-slate-800 dark:hover:text-slate-200">
        <IconArrowLeft className="h-4 w-4" /> Back to sales
      </Link>

      <div className="mb-5 flex flex-wrap items-start justify-between gap-3">
        <div>
          <div className="flex items-center gap-2">
            <h1 className="text-xl font-semibold text-slate-900 dark:text-slate-100">{header.code}</h1>
            {header.paymentStatus ? (
              <Badge tone={header.paymentStatus}>{header.paymentStatus}</Badge>
            ) : (
              <Badge tone="mixed">Mixed</Badge>
            )}
          </div>
          {/* 1.9.1: the event name used to be a <Link> to Event Detail -
              removed per marko's request to stop every "this reference jumps
              me to a different section" link in Orders/Tickets/Sales. */}
          <p className="mt-1 text-sm text-slate-500 dark:text-slate-400">
            {header.eventId && header.eventName ? header.eventName : <span className="italic">Mixed events</span>}
            {" "}&middot; {lines.length} ticket{lines.length === 1 ? "" : "s"}
            {header.saleDate && ` · sold ${formatDate(header.saleDate)}`}
          </p>
        </div>
        <Button variant="danger" onClick={() => setGroupDeleteOpen(true)}>
          <IconTrash className="h-4 w-4" /> Delete entire sale
        </Button>
      </div>

      <Card className="mb-8 grid grid-cols-2 gap-4 p-4 sm:grid-cols-3 lg:grid-cols-6">
        <div>
          <p className="text-xs font-medium uppercase text-slate-400 dark:text-slate-500">Platform</p>
          <p className="mt-1 text-sm text-slate-700 dark:text-slate-300">{header.platformName ?? "-"}</p>
        </div>
        <div>
          <p className="text-xs font-medium uppercase text-slate-400 dark:text-slate-500">Sale date</p>
          <p className="mt-1 text-sm text-slate-700 dark:text-slate-300">
            {header.saleDate ? formatDate(header.saleDate) : "Mixed"}
          </p>
        </div>
        <div>
          <p className="text-xs font-medium uppercase text-slate-400 dark:text-slate-500">Currency</p>
          <p className="mt-1 text-sm text-slate-700 dark:text-slate-300">{header.currency ?? "Mixed"}</p>
        </div>
        <div>
          <p className="text-xs font-medium uppercase text-slate-400 dark:text-slate-500">Refunded</p>
          <p className="mt-1 text-sm text-slate-700 dark:text-slate-300">
            {header.refundedCount} of {lines.length}
          </p>
        </div>
        {/* 1.9.0 (section 6): Paid/Outstanding - see the `header` useMemo
            above for how these are derived (paid-only / pending-only line
            subsets, refunded lines counted in neither). */}
        <div>
          <p className="text-xs font-medium uppercase text-slate-400 dark:text-slate-500">Paid</p>
          <p className="mt-1 text-sm text-slate-700 dark:text-slate-300">
            {formatMoneyOrMixed(header.paidCents, header.paidCurrency)}
          </p>
        </div>
        <div>
          <p className="text-xs font-medium uppercase text-slate-400 dark:text-slate-500">Outstanding</p>
          <p className="mt-1 text-sm text-slate-700 dark:text-slate-300">
            {formatMoneyOrMixed(header.outstandingCents, header.outstandingCurrency)}
          </p>
        </div>
      </Card>

      {/* 1.8.2: SUMMARY - the 6 numbers the brief calls out by name (Revenue,
          Fees, Cost, Profit, Margin, ROI), each its own card so the page
          answers "how much / what did it cost / what did I make" at a
          glance without reading a table. All 6 values were already computed
          in the `header` useMemo above (including costCents, which existed
          but had no card before this) - this is a display-only change, nothing
          about how these numbers are calculated has moved. `lg:` in this
          app's fixed 1080px-minimum window is effectively always active (see
          REDESIGN-1.8.2-REPORT.md), so this reads as one row on every real
          window size; the plain/sm classes are a defensive fallback only. */}
      <div className="mb-8 grid grid-cols-3 gap-3 lg:grid-cols-6">
        <Card className="p-4">
          <p className="text-xs font-medium uppercase text-slate-400 dark:text-slate-500">Revenue</p>
          <p className="mt-1 text-lg font-semibold">{formatMoneyOrMixed(header.revenueCents, header.currency)}</p>
        </Card>
        <Card className="p-4">
          <p className="text-xs font-medium uppercase text-slate-400 dark:text-slate-500">Fees</p>
          <p className="mt-1 text-lg font-semibold">{formatMoneyOrMixed(header.feesCents, header.currency)}</p>
        </Card>
        <Card className="p-4">
          <p className="text-xs font-medium uppercase text-slate-400 dark:text-slate-500">Cost</p>
          <p className="mt-1 text-lg font-semibold">{formatMoneyOrMixed(header.costCents, header.currency)}</p>
        </Card>
        <Card className="p-4">
          <p className="text-xs font-medium uppercase text-slate-400 dark:text-slate-500">Profit</p>
          <p
            className={`mt-1 text-lg font-semibold ${header.profitCents > 0 ? "text-emerald-600 dark:text-emerald-400" : header.profitCents < 0 ? "text-red-600 dark:text-red-400" : ""}`}
          >
            {formatMoneyOrMixed(header.profitCents, header.currency)}
          </p>
        </Card>
        <Card className="p-4">
          <p className="text-xs font-medium uppercase text-slate-400 dark:text-slate-500">Margin</p>
          <p className="mt-1 text-lg font-semibold">{formatPercentOrMixed(header.margin, header.currency)}</p>
        </Card>
        <Card className="p-4">
          <p className="text-xs font-medium uppercase text-slate-400 dark:text-slate-500">ROI</p>
          <p className="mt-1 text-lg font-semibold">{formatPercentOrMixed(header.roi, header.currency)}</p>
        </Card>
      </div>
      <p className="-mt-5 mb-8 text-xs text-slate-400 dark:text-slate-500">
        Revenue, fees, cost, profit, margin and ROI above exclude any refunded ticket in this sale - they are never
        counted as realized.
      </p>

      <SalePaymentStatusBar
        lines={lines}
        selected={selected}
        onClear={() => setSelected(new Set())}
        onApplied={() => load()}
      />

      <h2 className="mb-3 text-sm font-semibold text-slate-800 dark:text-slate-200">Tickets in this sale ({lines.length})</h2>
      {lines.length === 0 ? (
        <EmptyState title="No tickets found for this sale" />
      ) : (
        // 2.0.35: same proportional-percentage model as Sales.tsx now uses
        // (see that file's colgroup comment for the full history and the
        // honest narrow-window tradeoff it explains - applies here
        // identically). Two changes bundled in together since this file's
        // exact columns were already being touched for the % conversion:
        // Ticket and Order both went 84px -> 120px (basis for their new
        // percentages) - same truncating-10-char-code bug as Sale's own
        // 2.0.33 fix ("TIX-000001"/"ORD-000001" didn't fit in 84px either),
        // flagged since 2.0.33's report and now folded in rather than left
        // for a separate round. Seat's share grew from "whatever's left"
        // to an explicit 47.143% (660px at the 1400px reference) - marko's
        // own answer (chat) to widening it: it was already this table's
        // flexible column so it usually had real room already, but never a
        // guaranteed one, and "Sec 104 · Row A · Seat 12"-length strings
        // deserve one now, same reasoning as Event getting the largest
        // single share in Sales.tsx. Section/Row/Seat are merged into one
        // Seat column via formatSeatLocation (lib/format.ts) - the 3
        // underlying fields (s.section/s.rowLabel/s.seat) are untouched,
        // only how they display here did. `.th-c`/`.td-c` are the same
        // compact classes Sales.tsx uses - `.th`/`.td` elsewhere are
        // untouched. 1.8.3 added the leading checkbox column (bulk
        // actions) - always shown here, not selectionMode-gated like
        // Sales.tsx's own.
        // 2.0.37: same shift as Sales.tsx made - min-w-[1400px] plus a
        // single percentage set couldn't stop a horizontal scrollbar below
        // 1400px wide, only stop columns shrinking below their floor. Now
        // two full percentage sets switched by the same shared
        // useNarrowTables() breakpoint as every other table in the app:
        // Fees hides below 1690px (still on the Sale price/Cost/Profit
        // trio, and on Sales.tsx's own row for this sale - never the
        // Ticket/Seat/prices that matter most), everything else grows a
        // little and switches to the smaller .th-c-narrow/.td-c-narrow.
        // See Sales.tsx's own colgroup comment and PROTECTED-AREAS-NOTES.md
        // (2.0.37 section) for the full reasoning and verification.
        // 2.0.38: Ticket and Order's own code columns were STILL
        // under-measured (same root cause as every other table this
        // version - see PROTECTED-AREAS-NOTES.md's 2.0.38 section).
        // Recomputed every column against real rendered content this time,
        // which also caught a genuine gap in the 2.0.37 pass: the trailing
        // actions column (Edit/Refund/delete-icon buttons) never had a real
        // measured width at all - its old 7.827%/13.84% were unmeasured
        // guesses, not derived from anything. Now measured the same way as
        // everything else (worst case: an unrefunded line's Edit + Refund
        // text buttons + trash icon, all inline). Shared breakpoint moved to
        // 1649px (was 1690px) - see useNarrowTables.ts.
        <div className="overflow-x-auto rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-sm">
          <table className="w-full table-fixed border-collapse">
            {isNarrow ? (
              <colgroup>
                <col className="w-8" />
                <col className="w-[9.756%]" />
                <col className="w-[10.488%]" />
                <col className="w-[27.317%]" />
                <col className="w-[10%]" />
                <col className="w-[10%]" />
                <col className="w-[10.488%]" />
                <col className="w-[10%]" />
                <col className="w-[11.951%]" />
              </colgroup>
            ) : (
              <colgroup>
                <col className="w-8" />
                <col className="w-[7.638%]" />
                <col className="w-[8.133%]" />
                <col className="w-[39.463%]" />
                <col className="w-[7.779%]" />
                <col className="w-[7.779%]" />
                <col className="w-[7.779%]" />
                <col className="w-[8.133%]" />
                <col className="w-[6.365%]" />
                <col className="w-[6.931%]" />
              </colgroup>
            )}
            <thead className="border-b border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-800/60">
              <tr>
                <th className={isNarrow ? "th-c-narrow" : "th-c"}>
                  <input
                    type="checkbox"
                    className={CHECKBOX_CLASS}
                    checked={allSelectableSelected}
                    onChange={toggleSelectAll}
                    aria-label="Select all tickets in this sale"
                  />
                </th>
                <th className={isNarrow ? "th-c-narrow" : "th-c"}>Ticket</th>
                <th className={isNarrow ? "th-c-narrow" : "th-c"}>Order</th>
                <th className={isNarrow ? "th-c-narrow" : "th-c"}>Seat</th>
                <th className={`${isNarrow ? "th-c-narrow" : "th-c"} text-right`}>Sale price</th>
                {!isNarrow && <th className="th-c text-right">Fees</th>}
                <th className={`${isNarrow ? "th-c-narrow" : "th-c"} text-right`}>Cost</th>
                <th className={`${isNarrow ? "th-c-narrow" : "th-c"} text-right`}>Profit</th>
                <th className={isNarrow ? "th-c-narrow" : "th-c"}>Status</th>
                <th className={isNarrow ? "th-c-narrow" : "th-c"} />
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
              {lines.map((s) => {
                const seatLabel = formatSeatLocation(s.section, s.rowLabel, s.seat);
                const selectable = s.paymentStatus !== "refunded";
                return (
                  <tr
                    key={s.id}
                    className={`hover:bg-slate-50 dark:hover:bg-slate-800/60 ${selected.has(s.ticketId) ? "bg-brand-50/60 dark:bg-brand-500/5" : ""}`}
                  >
                    <td className={isNarrow ? "td-c-narrow" : "td-c"}>
                      {selectable && (
                        <input
                          type="checkbox"
                          className={CHECKBOX_CLASS}
                          checked={selected.has(s.ticketId)}
                          onChange={() => toggleOne(s.ticketId)}
                          aria-label={`Select ticket ${s.ticketCode}`}
                        />
                      )}
                    </td>
                    {/* 1.9.1: the ticket code and order code used to be
                        <Link>s (into /tickets and /orders/:id respectively)
                        - removed per marko's request to stop every "this
                        reference jumps me to a different section" link in
                        Orders/Tickets/Sales. Both are now plain text. */}
                    <td className={`${isNarrow ? "td-c-narrow" : "td-c"} truncate font-medium text-slate-900 dark:text-slate-100`} title={s.ticketCode}>
                      {s.ticketCode}
                    </td>
                    <td className={`${isNarrow ? "td-c-narrow" : "td-c"} truncate text-slate-500 dark:text-slate-400`} title={s.orderCode}>
                      {s.orderCode}
                    </td>
                    <td className={`${isNarrow ? "td-c-narrow" : "td-c"} truncate text-slate-500 dark:text-slate-400`} title={seatLabel}>
                      {seatLabel}
                    </td>
                    <td className={`${isNarrow ? "td-c-narrow" : "td-c"} text-right tabular-nums whitespace-nowrap`}>{formatMoney(s.salePriceCents, s.currency)}</td>
                    {!isNarrow && (
                      <td className="td-c text-right tabular-nums whitespace-nowrap">{formatMoney(s.sellingFeesCents, s.currency)}</td>
                    )}
                    <td className={`${isNarrow ? "td-c-narrow" : "td-c"} text-right tabular-nums whitespace-nowrap`}>{formatMoney(s.costCents, s.currency)}</td>
                    <td
                      className={`${isNarrow ? "td-c-narrow" : "td-c"} text-right tabular-nums whitespace-nowrap font-medium ${s.profitCents > 0 ? "text-emerald-600 dark:text-emerald-400" : s.profitCents < 0 ? "text-red-600 dark:text-red-400" : ""}`}
                    >
                      {formatMoney(s.profitCents, s.currency)}
                    </td>
                    <td className={isNarrow ? "td-c-narrow" : "td-c"}>
                      <Badge tone={s.paymentStatus}>{s.paymentStatus}</Badge>
                      {s.paymentStatus === "refunded" && s.refundedAt && (
                        <p
                          className="mt-0.5 truncate text-[11px] text-slate-400 dark:text-slate-500"
                          title={`${formatDate(s.refundedAt)}${s.refundReason ? ` · ${s.refundReason}` : ""}`}
                        >
                          {formatDate(s.refundedAt)}
                          {s.refundReason ? ` · ${s.refundReason}` : ""}
                        </p>
                      )}
                    </td>
                    <td className={isNarrow ? "td-c-narrow" : "td-c"}>
                      <div className="flex flex-wrap items-center justify-end gap-x-2 gap-y-0.5">
                        {s.paymentStatus !== "refunded" && (
                          <>
                            <button
                              className="text-xs font-medium text-brand-600 dark:text-brand-400 hover:underline"
                              onClick={() => setEditTarget(s)}
                            >
                              Edit
                            </button>
                            <button
                              className="text-xs font-medium text-amber-600 dark:text-amber-400 hover:underline"
                              onClick={() => setRefundTarget(s)}
                            >
                              Refund
                            </button>
                          </>
                        )}
                        <button
                          className="text-slate-400 dark:text-slate-500 hover:text-red-600 dark:hover:text-red-400"
                          title={
                            s.paymentStatus === "refunded"
                              ? "Delete refund record (ticket status is not affected)"
                              : "Delete sale (returns ticket to available)"
                          }
                          onClick={() => setDeleteTarget(s)}
                        >
                          <IconTrash className="h-4 w-4" />
                        </button>
                      </div>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}

      <SaleEditModal
        open={!!editTarget}
        sale={editTarget}
        onClose={() => setEditTarget(null)}
        onSaved={() => {
          setEditTarget(null);
          load();
        }}
      />

      <RefundDialog
        sale={refundTarget}
        onClose={() => setRefundTarget(null)}
        onRefunded={() => {
          setRefundTarget(null);
          load();
        }}
      />

      <ConfirmDialog
        open={!!deleteTarget}
        title={deleteTarget?.paymentStatus === "refunded" ? "Delete this refund record?" : "Delete this sale?"}
        message={
          deleteTarget?.paymentStatus === "refunded" ? (
            <>
              This permanently deletes the refund record for sale <b>{deleteTarget?.code}</b> (ticket{" "}
              <b>{deleteTarget?.ticketCode}</b>). The ticket itself is not affected - it already returned to
              Available when it was refunded. Once this record is gone, there will be no trace this ticket was ever
              sold and refunded. This cannot be undone.
            </>
          ) : (
            <>
              Use this only to undo a mistake (e.g. the wrong ticket was picked) - it permanently removes sale{" "}
              <b>{deleteTarget?.code}</b> with no record left behind, and sets ticket{" "}
              <b>{deleteTarget?.ticketCode}</b> back to Available. This cannot be undone.
              <br />
              If a real buyer is returning a ticket, cancel this and use <b>Refund</b> instead - it keeps a record.
            </>
          )
        }
        confirmLabel={deleteTarget?.paymentStatus === "refunded" ? "Delete refund record" : "Delete sale"}
        danger
        busy={deleting}
        onCancel={() => setDeleteTarget(null)}
        onConfirm={async () => {
          if (!deleteTarget) return;
          const wasRefunded = deleteTarget.paymentStatus === "refunded";
          setDeleting(true);
          try {
            await api.deleteSale(deleteTarget.id);
            toast.success(wasRefunded ? "Refund record deleted" : "Sale deleted, ticket is available again");
            setDeleteTarget(null);
            const remaining = (lines ?? []).filter((l) => l.id !== deleteTarget.id);
            if (remaining.length === 0) {
              // That was the last (or only) line in this sale - nothing left
              // to show here, so go back to the list rather than an empty page.
              navigate("/sales");
            } else if (deleteTarget.id === saleId) {
              // This page's URL is anchored to the batch's lowest sale id
              // (see backend GROUP_BASE_SELECT's MIN(s.id)), and we just
              // deleted exactly that row. Reloading with the same saleId
              // would now 404 even though the rest of the batch is fine, so
              // re-point the URL at the new lowest surviving id instead -
              // the useParams()-driven effect below then reloads correctly.
              // `replace` so the now-dead URL doesn't linger in history
              // (e.g. the Back button landing back on a 404).
              const newAnchorId = Math.min(...remaining.map((l) => l.id));
              navigate(`/sales/${newAnchorId}`, { replace: true });
            } else {
              load();
            }
          } catch (e) {
            toast.error(errMsg(e));
          } finally {
            setDeleting(false);
          }
        }}
      />

      <ConfirmDialog
        open={groupDeleteOpen}
        title="Delete entire sale?"
        message={
          <>
            This permanently deletes all {lines.length} ticket{lines.length === 1 ? "" : "s"} in sale{" "}
            <b>{header.code}</b> at once. Tickets that are actively sold return to Available; any refunded lines are
            removed as history with no trace left. This cannot be undone.
          </>
        }
        confirmLabel="Delete entire sale"
        danger
        busy={groupDeleting}
        onCancel={() => setGroupDeleteOpen(false)}
        onConfirm={async () => {
          setGroupDeleting(true);
          try {
            const count = await api.deleteSaleGroup(saleId);
            toast.success(`Sale ${header.code} deleted - ${count} ticket${count === 1 ? "" : "s"} affected`);
            setGroupDeleteOpen(false);
            // The whole group is gone - nothing left to anchor this page to,
            // so always go back to the list (unlike per-line delete above,
            // there's no "reload the remaining lines" case here).
            navigate("/sales");
          } catch (e) {
            toast.error(errMsg(e));
          } finally {
            setGroupDeleting(false);
          }
        }}
      />
    </div>
  );
}

/** Sale Detail's bulk action (1.9.2, sections 3/4): mark many selected,
 * non-refunded sale lines as Paid or Pending in one click. Replaces the old
 * shared `BulkTicketEditBar` that used to sit here (it edited Section/Row/
 * Seat/Listing price - those stay editable, just one ticket at a time, via
 * the per-line "Edit" button/`SaleEditModal` above and Order Detail's own
 * Ticket Edit). 1.9.3: Order Detail also moved off `BulkTicketEditBar`, onto
 * its own narrow `TicketStatusBar` - so nothing in the UI renders
 * `BulkTicketEditBar` any more. It's left in place regardless (same
 * "small, self-contained, already-tested code - deleting it would only add
 * risk, not value" reasoning as the unused CSV "export everything" backend
 * commands from 1.9.1's report), in case a general bulk field editor is
 * ever wanted again on some page. Deliberately just two buttons and no modal - the whole
 * action is "set payment_status on the selection", nothing to configure.
 *
 * Refunded lines can never reach here: `selected` is only ever populated
 * from `selectableLines` (already excludes them - see the page component
 * above), and `bulk_update_sale_payment_status_impl` rejects a refunded id
 * in the batch regardless, as defense in depth.
 *
 * Takes the page's existing `lines`/`selected` (ticket ids) rather than sale
 * ids directly, and maps ticketId -> sale id internally (a Sale line and its
 * ticket are 1:1), so none of the page's selection plumbing (toggleOne/
 * toggleSelectAll/the table's checkboxes) needed to change shape to support
 * this. */
function SalePaymentStatusBar({
  lines,
  selected,
  onClear,
  onApplied,
}: {
  lines: Sale[];
  selected: Set<number>;
  onClear: () => void;
  onApplied: () => void;
}) {
  const toast = useToast();
  const [saving, setSaving] = useState<"pending" | "paid" | null>(null);

  const selectedSaleIds = lines.filter((l) => selected.has(l.ticketId)).map((l) => l.id);
  if (selectedSaleIds.length === 0) return null;

  const apply = async (paymentStatus: "pending" | "paid") => {
    setSaving(paymentStatus);
    try {
      const updated = await api.bulkUpdateSalePaymentStatus({ saleIds: selectedSaleIds, paymentStatus });
      toast.success(`${updated.length} sale${updated.length === 1 ? "" : "s"} marked as ${paymentStatus}`);
      onApplied();
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setSaving(null);
    }
  };

  return (
    <div className="mb-4 flex items-center gap-3 rounded-lg bg-brand-50 dark:bg-brand-500/10 px-4 py-2.5 text-sm ring-1 ring-inset ring-brand-200 dark:ring-brand-500/30">
      <span className="font-medium text-brand-800 dark:text-brand-300">Selected: {selectedSaleIds.length}</span>
      <Button variant="secondary" onClick={() => apply("paid")} disabled={saving !== null}>
        {saving === "paid" ? "Marking as Paid..." : "Mark as Paid"}
      </Button>
      <Button variant="secondary" onClick={() => apply("pending")} disabled={saving !== null}>
        {saving === "pending" ? "Marking as Pending..." : "Mark as Pending"}
      </Button>
      <button
        type="button"
        className="ml-auto text-xs font-medium text-brand-700 dark:text-brand-400 hover:underline disabled:opacity-50"
        onClick={onClear}
        disabled={saving !== null}
      >
        Clear selection
      </button>
    </div>
  );
}

function RefundDialog({
  sale,
  onClose,
  onRefunded,
}: {
  sale: Sale | null;
  onClose: () => void;
  onRefunded: () => void;
}) {
  const toast = useToast();
  const [reason, setReason] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    setReason("");
  }, [sale]);

  if (!sale) return null;

  const confirm = async () => {
    setBusy(true);
    try {
      await api.refundSale(sale.id, reason.trim() || undefined);
      toast.success(`${sale.code} refunded - ${sale.ticketCode} is available again`);
      onRefunded();
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Modal open={!!sale} onClose={onClose} title={`Refund ${sale.code}`} width="max-w-sm">
      <p className="text-sm text-slate-500 dark:text-slate-400">
        Ticket <b>{sale.ticketCode}</b> will return to Available so it can be sold again. The sale itself stays on
        record marked as refunded and is excluded from revenue/profit - it can no longer be edited or deleted
        afterwards. This cannot be undone.
      </p>
      <div className="mt-3">
        <Field label="Reason (optional)">
          <Textarea rows={2} value={reason} onChange={(e) => setReason(e.target.value)} placeholder="e.g. buyer couldn't attend" />
        </Field>
      </div>
      <ModalFooter>
        <Button variant="secondary" onClick={onClose} disabled={busy}>
          Cancel
        </Button>
        <Button variant="danger" onClick={confirm} disabled={busy}>
          {busy ? "Refunding..." : "Refund sale"}
        </Button>
      </ModalFooter>
    </Modal>
  );
}

function SaleEditModal({
  open,
  sale,
  onClose,
  onSaved,
}: {
  open: boolean;
  sale: Sale | null;
  onClose: () => void;
  onSaved: () => void;
}) {
  const toast = useToast();
  const [platforms, setPlatforms] = useState<Platform[]>([]);
  const [platformId, setPlatformId] = useState<number | null>(null);
  const [saleDate, setSaleDate] = useState("");
  const [salePrice, setSalePrice] = useState("");
  const [sellingFees, setSellingFees] = useState("");
  const [paymentStatus, setPaymentStatus] = useState<SalePaymentStatus>("pending");
  const [buyerReference, setBuyerReference] = useState("");
  const [notes, setNotes] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!sale) return;
    api.listPlatforms().then(setPlatforms).catch(() => {});
    setPlatformId(sale.platformId);
    setSaleDate(sale.saleDate);
    setSalePrice((sale.salePriceCents / 100).toFixed(2));
    setSellingFees((sale.sellingFeesCents / 100).toFixed(2));
    setPaymentStatus(sale.paymentStatus);
    setBuyerReference(sale.buyerReference ?? "");
    setNotes(sale.notes ?? "");
    setError(null);
  }, [sale]);

  if (!sale) return null;

  const submit = async () => {
    const s = salePrice.trim().replace(",", ".");
    if (!/^\d+(\.\d{1,2})?$/.test(s)) return setError("Sale price is not a valid amount");
    const feesStr = sellingFees.trim().replace(",", ".") || "0";
    if (!/^\d+(\.\d{1,2})?$/.test(feesStr)) return setError("Selling fees is not a valid amount");

    const input: SaleEditInput = {
      platformId,
      saleDate,
      salePriceCents: Math.round(parseFloat(s) * 100),
      sellingFeesCents: Math.round(parseFloat(feesStr) * 100),
      paymentStatus,
      buyerReference: buyerReference || null,
      notes: notes || null,
    };
    setSaving(true);
    setError(null);
    try {
      await api.updateSale(sale.id, input);
      toast.success("Sale updated");
      onSaved();
    } catch (e) {
      setError(errMsg(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal open={open} onClose={onClose} title={`Edit ${sale.code}`}>
      <div className="grid grid-cols-2 gap-4">
        <LookupSelect
          label="Platform"
          // 1.9.3: sale/both only - marko split the shared platform pool
          // into where-you-bought vs where-you-sold lists (Settings ->
          // Lookups). onCreate right below was already tagging new
          // platforms "sale" from here even before 1.9.3; only this display
          // filter is new.
          options={platforms.filter((p) => p.kind === "sale" || p.kind === "both")}
          value={platformId}
          onChange={setPlatformId}
          onCreate={async (name) => {
            const p = await api.createPlatform(name, "sale");
            setPlatforms((prev) => [...prev, p]);
            return p;
          }}
        />
        <Field label="Sale date" required>
          <Input type="date" value={saleDate} onChange={(e) => setSaleDate(e.target.value)} />
        </Field>
        <Field label={`Sale price (${sale.currency})`} required>
          <Input inputMode="decimal" value={salePrice} onChange={(e) => setSalePrice(e.target.value)} />
        </Field>
        <Field label="Selling fees">
          <Input inputMode="decimal" value={sellingFees} onChange={(e) => setSellingFees(e.target.value)} />
        </Field>
        <Field label="Payment status" hint="Use the Refund action to refund this ticket's sale">
          <Select value={paymentStatus} onChange={(e) => setPaymentStatus(e.target.value as SalePaymentStatus)}>
            <option value="pending">Pending</option>
            <option value="paid">Paid</option>
          </Select>
        </Field>
        <Field label="Buyer / reference">
          <Input value={buyerReference} onChange={(e) => setBuyerReference(e.target.value)} />
        </Field>
        <div className="col-span-2">
          <Field label="Notes">
            <Textarea rows={2} value={notes} onChange={(e) => setNotes(e.target.value)} />
          </Field>
        </div>
      </div>
      {error && <p className="mt-3 text-sm text-red-600 dark:text-red-400">{error}</p>}
      <ModalFooter>
        <Button variant="secondary" onClick={onClose} disabled={saving}>
          Cancel
        </Button>
        <Button variant="primary" onClick={submit} disabled={saving}>
          {saving ? "Saving..." : "Save changes"}
        </Button>
      </ModalFooter>
    </Modal>
  );
}

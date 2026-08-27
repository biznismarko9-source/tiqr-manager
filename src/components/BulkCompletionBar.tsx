import { useState } from "react";
import { errMsg } from "../lib/api";
import { Button } from "./ui";
import { useToast } from "../lib/toast";

/** 2.0.67: marko's own request - "paid/not paid, delivered/not delivered
 * must be doable all at once, not one at a time". Shared bulk action bar for
 * the Orders and Sales lists' existing selection mode (the same checkboxes
 * bulk-delete already uses there), next to `BulkDeleteBar` (ui.tsx) in the
 * same `{selectionMode && (...)}` block.
 *
 * Deliberately just 4 buttons and no modal - same "just buttons" philosophy
 * as Sale Detail's own `SalePaymentStatusBar` - but unlike that bar, this one
 * covers TWO independent axes (delivery and payment) rather than one, so
 * applying one never clears the selection: marko can mark a selection
 * Delivered and then, without re-selecting anything, also mark the same
 * selection Paid. Only the explicit "Clear selection" link clears it,
 * exactly like `SalePaymentStatusBar`/`BulkTicketEditBar` already behave.
 *
 * Deliberately page-agnostic: Orders.tsx selects whole ORDER ids and calls
 * `bulkSetOrdersDeliveryStatus`/`bulkSetOrdersPaymentStatus`, while Sales.tsx
 * selects SaleGroup ids and calls `bulkSetSaleGroupsDeliveryStatus`/
 * `bulkSetSaleGroupsPaymentStatus` - two different id shapes and backend
 * commands (see those commands' own doc comments in orders.rs/sales.rs for
 * how each resolves its selection down to actual tickets/sales). Rather than
 * this component knowing about either, the page injects its own API call as
 * a callback that resolves to how many rows were ACTUALLY changed (tickets
 * for delivery, sales for payment - which can be fewer than `count` selected
 * rows, e.g. an order with nothing sold yet) - this bar only owns the
 * shared UI: the busy/disabled state while a request is in flight, and the
 * resulting toast. */
export function BulkCompletionBar({
  count,
  itemLabel,
  onSetDelivered,
  onSetPaid,
  onClear,
}: {
  /** How many rows (orders, or sale groups) are currently selected - shown
   * in the "Selected: N" label. The number of tickets/sales actually
   * touched by a given action can differ (see this component's own doc
   * comment) and is reported in that action's own success toast instead. */
  count: number;
  /** Singular noun for one selected row, e.g. "order" or "sale" - pluralized here as needed. */
  itemLabel: string;
  /** Sets delivery status for every eligible ticket in the current
   * selection; resolves to how many tickets were actually changed. */
  onSetDelivered: (delivered: boolean) => Promise<number>;
  /** Sets payment status for every eligible sale in the current selection;
   * resolves to how many sales were actually changed. */
  onSetPaid: (paid: boolean) => Promise<number>;
  onClear: () => void;
}) {
  const toast = useToast();
  const [saving, setSaving] = useState<"delivered" | "notDelivered" | "paid" | "pending" | null>(null);

  if (count === 0) return null;

  const runDelivery = async (delivered: boolean) => {
    setSaving(delivered ? "delivered" : "notDelivered");
    try {
      const updated = await onSetDelivered(delivered);
      toast.success(
        updated === 0
          ? "No sold tickets in this selection to update"
          : `${updated} ticket${updated === 1 ? "" : "s"} marked ${delivered ? "Delivered" : "Not delivered"}`,
      );
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setSaving(null);
    }
  };

  const runPayment = async (paid: boolean) => {
    setSaving(paid ? "paid" : "pending");
    try {
      const updated = await onSetPaid(paid);
      toast.success(
        updated === 0
          ? "No active sales in this selection to update"
          : `${updated} sale${updated === 1 ? "" : "s"} marked ${paid ? "Paid" : "Pending"}`,
      );
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setSaving(null);
    }
  };

  const busy = saving !== null;
  const divider = <span className="hidden sm:block h-4 w-px bg-brand-200 dark:bg-brand-500/30" aria-hidden="true" />;
  const axisLabel = (text: string) => (
    <span className="text-xs font-semibold uppercase tracking-wide text-brand-600 dark:text-brand-400">{text}</span>
  );

  return (
    <div className="mb-4 flex flex-wrap items-center gap-3 rounded-lg bg-brand-50 dark:bg-brand-500/10 px-4 py-2.5 text-sm ring-1 ring-inset ring-brand-200 dark:ring-brand-500/30">
      <span className="font-medium text-brand-800 dark:text-brand-300">
        Selected: {count} {itemLabel}
        {count === 1 ? "" : "s"}
      </span>

      {divider}
      {axisLabel("Delivery")}
      <Button variant="secondary" onClick={() => runDelivery(true)} disabled={busy}>
        {saving === "delivered" ? "Marking..." : "Mark Delivered"}
      </Button>
      <Button variant="secondary" onClick={() => runDelivery(false)} disabled={busy}>
        {saving === "notDelivered" ? "Marking..." : "Mark Not delivered"}
      </Button>

      {divider}
      {axisLabel("Payment")}
      <Button variant="secondary" onClick={() => runPayment(true)} disabled={busy}>
        {saving === "paid" ? "Marking..." : "Mark Paid"}
      </Button>
      <Button variant="secondary" onClick={() => runPayment(false)} disabled={busy}>
        {saving === "pending" ? "Marking..." : "Mark Pending"}
      </Button>

      <button
        type="button"
        className="ml-auto text-xs font-medium text-brand-700 dark:text-brand-400 hover:underline disabled:opacity-50"
        onClick={onClear}
        disabled={busy}
      >
        Clear selection
      </button>
    </div>
  );
}

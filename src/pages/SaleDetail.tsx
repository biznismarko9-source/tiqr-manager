import { useCallback, useEffect, useMemo, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { api, errMsg } from "../lib/api";
import type { Platform, Sale, SaleEditInput, SalePaymentStatus } from "../lib/types";
import { formatDate, formatMoney, formatMoneyOrMixed, formatPercentOrMixed } from "../lib/format";
import {
  Badge,
  Button,
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

  const [lines, setLines] = useState<Sale[] | null>(null);
  const [editTarget, setEditTarget] = useState<Sale | null>(null);
  const [refundTarget, setRefundTarget] = useState<Sale | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<Sale | null>(null);
  const [deleting, setDeleting] = useState(false);

  const load = useCallback(() => {
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
      revenueCents,
      feesCents,
      costCents,
      profitCents,
      margin,
      roi,
    };
  }, [lines]);

  if (lines === null || header === null) return <LoadingBlock />;

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
          <p className="mt-1 text-sm text-slate-500 dark:text-slate-400">
            {header.eventId && header.eventName ? (
              <Link to={`/events/${header.eventId}`} className="hover:text-brand-700 dark:hover:text-brand-400">
                {header.eventName}
              </Link>
            ) : (
              <span className="italic">Mixed events</span>
            )}
            {" "}&middot; {lines.length} ticket{lines.length === 1 ? "" : "s"}
            {header.saleDate && ` · sold ${formatDate(header.saleDate)}`}
          </p>
        </div>
      </div>

      <Card className="mb-8 grid grid-cols-2 gap-4 p-4 sm:grid-cols-4">
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
      </Card>

      <div className="mb-8 grid grid-cols-2 gap-3 sm:grid-cols-4">
        <Card className="p-4">
          <p className="text-xs font-medium uppercase text-slate-400 dark:text-slate-500">Revenue</p>
          <p className="mt-1 text-lg font-semibold">{formatMoneyOrMixed(header.revenueCents, header.currency)}</p>
        </Card>
        <Card className="p-4">
          <p className="text-xs font-medium uppercase text-slate-400 dark:text-slate-500">Selling fees</p>
          <p className="mt-1 text-lg font-semibold">{formatMoneyOrMixed(header.feesCents, header.currency)}</p>
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
          <p className="text-xs font-medium uppercase text-slate-400 dark:text-slate-500">Margin / ROI</p>
          <p className="mt-1 text-lg font-semibold">
            {formatPercentOrMixed(header.margin, header.currency)} / {formatPercentOrMixed(header.roi, header.currency)}
          </p>
        </Card>
      </div>
      <p className="-mt-5 mb-8 text-xs text-slate-400 dark:text-slate-500">
        Revenue, fees, profit, margin and ROI above exclude any refunded ticket in this sale - they are never
        counted as realized.
      </p>

      <h2 className="mb-3 text-sm font-semibold text-slate-800 dark:text-slate-200">Tickets in this sale ({lines.length})</h2>
      {lines.length === 0 ? (
        <EmptyState title="No tickets found for this sale" />
      ) : (
        <div className="overflow-x-auto rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-sm">
          <table className="w-full min-w-[1050px] border-collapse">
            <thead className="border-b border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-800/60">
              <tr>
                <th className="th">Ticket</th>
                <th className="th">Section</th>
                <th className="th">Row</th>
                <th className="th">Seat</th>
                <th className="th text-right">Purchase cost</th>
                <th className="th text-right">Sale price</th>
                <th className="th text-right">Fees</th>
                <th className="th text-right">Profit</th>
                <th className="th">Payment</th>
                <th className="th" />
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
              {lines.map((s) => (
                <tr key={s.id} className="hover:bg-slate-50 dark:hover:bg-slate-800/60">
                  <td className="td font-medium text-slate-900 dark:text-slate-100">{s.ticketCode}</td>
                  <td className="td text-slate-500 dark:text-slate-400">{s.section ?? "-"}</td>
                  <td className="td text-slate-500 dark:text-slate-400">{s.rowLabel ?? "-"}</td>
                  <td className="td text-slate-500 dark:text-slate-400">{s.seat ?? "-"}</td>
                  <td className="td text-right tabular-nums">{formatMoney(s.costCents, s.currency)}</td>
                  <td className="td text-right tabular-nums">{formatMoney(s.salePriceCents, s.currency)}</td>
                  <td className="td text-right tabular-nums">{formatMoney(s.sellingFeesCents, s.currency)}</td>
                  <td
                    className={`td text-right tabular-nums font-medium ${s.profitCents > 0 ? "text-emerald-600 dark:text-emerald-400" : s.profitCents < 0 ? "text-red-600 dark:text-red-400" : ""}`}
                  >
                    {formatMoney(s.profitCents, s.currency)}
                  </td>
                  <td className="td">
                    <Badge tone={s.paymentStatus}>{s.paymentStatus}</Badge>
                    {s.paymentStatus === "refunded" && s.refundedAt && (
                      <p className="mt-0.5 text-xs text-slate-400 dark:text-slate-500">
                        {formatDate(s.refundedAt)}
                        {s.refundReason ? ` · ${s.refundReason}` : ""}
                      </p>
                    )}
                  </td>
                  <td className="td">
                    <div className="flex items-center justify-end gap-3">
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
              ))}
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
          options={platforms}
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

import { useEffect, useState } from "react";
import { api, errMsg } from "../lib/api";
import { formatDate, formatMoney } from "../lib/format";
import { PAYMENT_METHODS } from "../lib/types";
import type { Payment, PaymentInput, PaymentSummary } from "../lib/types";
import { Badge, Button, Card, ConfirmDialog, Field, Input, Modal, ModalFooter, Select } from "./ui";
import { IconPencil, IconPlus, IconTrash } from "./icons";
import { useToast } from "../lib/toast";

type Target = { type: "sale"; key: string } | { type: "order"; orderId: number };

const STATUS_TONE: Record<string, "paid" | "pending" | "refunded" | "partial"> = {
  paid: "paid",
  pending: "pending",
  refunded: "refunded",
  partial: "partial",
  mixed: "pending",
};

/** Payments 2.0: Paid/Outstanding/Status + full history + Add/Edit/Delete,
 * shared between Sale Detail and Order Detail so there's one payment system
 * in the app, not two - see payments.rs's own module doc comment. */
export function PaymentsSection({ target, refreshKey }: { target: Target; refreshKey?: unknown }) {
  const toast = useToast();
  const [summary, setSummary] = useState<PaymentSummary | null>(null);
  const [formOpen, setFormOpen] = useState<Payment | null | "new">(null);
  const [confirmDelete, setConfirmDelete] = useState<Payment | null>(null);
  const [deleting, setDeleting] = useState(false);

  const load = () => {
    const req = target.type === "sale" ? api.getPaymentSummaryForSale(target.key) : api.getPaymentSummaryForOrder(target.orderId);
    req.then(setSummary).catch((e) => toast.error(errMsg(e)));
  };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  useEffect(load, [target.type === "sale" ? target.key : target.orderId, refreshKey]);

  if (!summary) return null;

  const mixed = summary.currency == null;
  const defaultCurrency = summary.totalCurrency ?? "EUR";

  return (
    <Card className="mb-8 p-4">
      <p className="mb-3 text-xs font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">Payments</p>
      <div className="mb-4 grid grid-cols-3 gap-3">
        <div>
          <p className="text-xs font-medium uppercase text-slate-400 dark:text-slate-500">Paid</p>
          <p className="mt-1 text-lg font-semibold text-emerald-600 dark:text-emerald-400">
            {mixed ? "Mixed" : formatMoney(summary.receivedCents ?? 0, summary.currency!)}
          </p>
        </div>
        <div>
          <p className="text-xs font-medium uppercase text-slate-400 dark:text-slate-500">Outstanding</p>
          <p className="mt-1 text-lg font-semibold text-slate-800 dark:text-slate-200">
            {mixed ? "Mixed" : formatMoney(summary.outstandingCents ?? 0, summary.currency!)}
          </p>
        </div>
        <div>
          <p className="text-xs font-medium uppercase text-slate-400 dark:text-slate-500">Status</p>
          <p className="mt-1">
            <Badge tone={STATUS_TONE[summary.status] ?? "pending"}>{summary.status}</Badge>
          </p>
        </div>
      </div>

      {summary.payments.length > 0 && (
        <div className="mb-3 divide-y divide-slate-100 dark:divide-slate-800 rounded-lg border border-slate-200 dark:border-slate-800">
          {summary.payments.map((p) => (
            <div key={p.id} className="flex items-center justify-between gap-3 px-3 py-2 text-sm">
              <div className="min-w-0">
                <span className="font-medium text-slate-800 dark:text-slate-200">{formatDate(p.paymentDate)}</span>{" "}
                <span className="text-slate-500 dark:text-slate-400">
                  {formatMoney(p.amountCents, p.currency)} ·{" "}
                  {PAYMENT_METHODS.find((m) => m.value === p.method)?.label ?? p.method}
                  {p.method === "other" && p.methodOtherNote ? ` (${p.methodOtherNote})` : ""}
                  {p.reference ? ` · ${p.reference}` : ""}
                  {p.isShortcut ? " · shortcut" : ""}
                </span>
              </div>
              <div className="flex shrink-0 gap-1">
                <button
                  className="rounded p-1 text-slate-400 hover:bg-slate-100 hover:text-slate-700 dark:hover:bg-slate-800 dark:hover:text-slate-200"
                  onClick={() => setFormOpen(p)}
                  aria-label="Edit payment"
                >
                  <IconPencil className="h-4 w-4" />
                </button>
                <button
                  className="rounded p-1 text-slate-400 hover:bg-red-50 hover:text-red-600 dark:hover:bg-red-500/10 dark:hover:text-red-400"
                  onClick={() => setConfirmDelete(p)}
                  aria-label="Delete payment"
                >
                  <IconTrash className="h-4 w-4" />
                </button>
              </div>
            </div>
          ))}
        </div>
      )}

      {summary.status !== "refunded" && (
        <Button variant="secondary" onClick={() => setFormOpen("new")}>
          <IconPlus className="h-4 w-4" /> Add payment
        </Button>
      )}

      {formOpen && (
        <PaymentFormModal
          target={target}
          initial={formOpen === "new" ? null : formOpen}
          defaultCurrency={defaultCurrency}
          onClose={() => setFormOpen(null)}
          onSaved={() => {
            setFormOpen(null);
            load();
          }}
        />
      )}

      <ConfirmDialog
        open={!!confirmDelete}
        title="Delete this payment?"
        message="This removes it from the payment history. This cannot be undone."
        confirmLabel="Delete payment"
        danger
        busy={deleting}
        onCancel={() => setConfirmDelete(null)}
        onConfirm={async () => {
          if (!confirmDelete) return;
          setDeleting(true);
          try {
            await api.deletePayment(confirmDelete.id);
            toast.success("Payment deleted");
            setConfirmDelete(null);
            load();
          } catch (e) {
            toast.error(errMsg(e));
          } finally {
            setDeleting(false);
          }
        }}
      />
    </Card>
  );
}

function PaymentFormModal({
  target,
  initial,
  defaultCurrency,
  onClose,
  onSaved,
}: {
  target: Target;
  initial: Payment | null;
  defaultCurrency: string;
  onClose: () => void;
  onSaved: () => void;
}) {
  const toast = useToast();
  const [amount, setAmount] = useState(initial ? (initial.amountCents / 100).toFixed(2) : "");
  const [currency, setCurrency] = useState(initial?.currency ?? defaultCurrency);
  const [date, setDate] = useState(initial?.paymentDate ?? new Date().toISOString().slice(0, 10));
  const [method, setMethod] = useState(initial?.method ?? "bank_transfer");
  const [methodOther, setMethodOther] = useState(initial?.methodOtherNote ?? "");
  const [reference, setReference] = useState(initial?.reference ?? "");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = async () => {
    const cents = Math.round(parseFloat(amount.replace(",", ".")) * 100);
    if (!amount || Number.isNaN(cents) || cents <= 0) return setError("Enter a valid amount greater than 0");
    if (method === "other" && !methodOther.trim()) return setError('Describe the method when using "Other"');
    const input: PaymentInput = {
      saleGroupKey: target.type === "sale" ? target.key : null,
      orderId: target.type === "order" ? target.orderId : null,
      amountCents: cents,
      currency: currency.trim().toUpperCase(),
      paymentDate: date,
      method,
      methodOtherNote: method === "other" ? methodOther.trim() : null,
      reference: reference.trim() || null,
    };
    setSaving(true);
    setError(null);
    try {
      if (initial) await api.updatePayment(initial.id, input);
      else await api.createPayment(input);
      toast.success(initial ? "Payment updated" : "Payment recorded");
      onSaved();
    } catch (e) {
      setError(errMsg(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal open onClose={onClose} title={initial ? "Edit payment" : "Add payment"}>
      <div className="grid grid-cols-2 gap-4">
        <Field label="Amount" required>
          <Input inputMode="decimal" value={amount} onChange={(e) => setAmount(e.target.value)} placeholder="0.00" />
        </Field>
        <Field label="Currency" required>
          <Input value={currency} onChange={(e) => setCurrency(e.target.value.toUpperCase())} />
        </Field>
        <Field label="Date" required>
          <Input type="date" value={date} onChange={(e) => setDate(e.target.value)} />
        </Field>
        <Field label="Method" required>
          <Select value={method} onChange={(e) => setMethod(e.target.value)}>
            {PAYMENT_METHODS.map((m) => (
              <option key={m.value} value={m.value}>
                {m.label}
              </option>
            ))}
          </Select>
        </Field>
        {method === "other" && (
          <div className="col-span-2">
            <Field label="Describe method" required>
              <Input value={methodOther} onChange={(e) => setMethodOther(e.target.value)} />
            </Field>
          </div>
        )}
        <div className="col-span-2">
          <Field label="Reference / note">
            <Input value={reference} onChange={(e) => setReference(e.target.value)} />
          </Field>
        </div>
      </div>
      {error && <p className="mt-3 text-sm text-red-600 dark:text-red-400">{error}</p>}
      <ModalFooter>
        <Button variant="secondary" onClick={onClose} disabled={saving}>
          Cancel
        </Button>
        <Button onClick={submit} disabled={saving}>
          {saving ? "Saving..." : initial ? "Save changes" : "Add payment"}
        </Button>
      </ModalFooter>
    </Modal>
  );
}

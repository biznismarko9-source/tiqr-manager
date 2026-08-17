import { useEffect, useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { api, errMsg } from "../lib/api";
import type { EventWithStats, Platform, Sale, SaleEditInput, SaleInput, SalePaymentStatus, Ticket } from "../lib/types";
import { formatDate, formatMoney, formatPercent, todayIso } from "../lib/format";
import {
  Badge,
  Button,
  ConfirmDialog,
  EmptyState,
  Field,
  Input,
  LoadingBlock,
  Modal,
  ModalFooter,
  PageHeader,
  Select,
  Textarea,
} from "../components/ui";
import { LookupSelect } from "../components/LookupSelect";
import { IconPlus, IconReceipt, IconSearch, IconTrash } from "../components/icons";
import { useToast } from "../lib/toast";

export default function Sales() {
  const toast = useToast();
  const [sales, setSales] = useState<Sale[] | null>(null);
  const [events, setEvents] = useState<EventWithStats[]>([]);
  const [search, setSearch] = useState("");
  const [eventId, setEventId] = useState<number | "">("");
  const [paymentStatus, setPaymentStatus] = useState("");
  const [dateFrom, setDateFrom] = useState("");
  const [dateTo, setDateTo] = useState("");
  const [modalOpen, setModalOpen] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<Sale | null>(null);
  const [deleting, setDeleting] = useState(false);
  const [editTarget, setEditTarget] = useState<Sale | null>(null);

  useEffect(() => {
    api.listEvents().then(setEvents).catch(() => {});
  }, []);

  const load = () => {
    api
      .listSales({
        search: search || undefined,
        eventId: eventId || undefined,
        paymentStatus: paymentStatus || undefined,
        dateFrom: dateFrom || undefined,
        dateTo: dateTo || undefined,
      })
      .then(setSales)
      .catch((e) => toast.error(errMsg(e)));
  };

  useEffect(() => {
    const t = setTimeout(load, 200);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [search, eventId, paymentStatus, dateFrom, dateTo]);

  const totals = useMemo(() => {
    if (!sales) return null;
    return sales.reduce(
      (acc, s) => ({
        revenue: acc.revenue + s.salePriceCents,
        profit: acc.profit + s.profitCents,
      }),
      { revenue: 0, profit: 0 },
    );
  }, [sales]);

  return (
    <div>
      <PageHeader
        title="Sales"
        subtitle="Every ticket you've sold, with profit calculated automatically."
        actions={
          <Button variant="primary" onClick={() => setModalOpen(true)}>
            <IconPlus className="h-4 w-4" /> New Sale
          </Button>
        }
      />

      <div className="mb-4 flex flex-wrap items-end gap-3">
        <div className="w-56">
          <span className="label">Search</span>
          <div className="relative">
            <IconSearch className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-400" />
            <Input
              placeholder="Sale code, ticket, buyer..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="pl-9"
            />
          </div>
        </div>
        <div className="w-52">
          <span className="label">Event</span>
          <Select value={eventId} onChange={(e) => setEventId(e.target.value ? Number(e.target.value) : "")}>
            <option value="">All events</option>
            {events.map((ev) => (
              <option key={ev.id} value={ev.id}>
                {ev.name}
              </option>
            ))}
          </Select>
        </div>
        <div className="w-40">
          <span className="label">Payment</span>
          <Select value={paymentStatus} onChange={(e) => setPaymentStatus(e.target.value)}>
            <option value="">All</option>
            <option value="pending">Pending</option>
            <option value="paid">Paid</option>
            <option value="refunded">Refunded</option>
          </Select>
        </div>
        <div className="w-36">
          <span className="label">From</span>
          <Input type="date" value={dateFrom} onChange={(e) => setDateFrom(e.target.value)} />
        </div>
        <div className="w-36">
          <span className="label">To</span>
          <Input type="date" value={dateTo} onChange={(e) => setDateTo(e.target.value)} />
        </div>
        {totals && sales && (
          <p className="ml-auto text-xs text-slate-400">
            {sales.length} sales &middot; revenue {formatMoney(totals.revenue, "EUR")} &middot; profit{" "}
            {formatMoney(totals.profit, "EUR")}
          </p>
        )}
      </div>

      {sales === null ? (
        <LoadingBlock />
      ) : sales.length === 0 ? (
        <EmptyState
          icon={<IconReceipt className="h-8 w-8" />}
          title="No sales match these filters"
          action={
            <Button variant="primary" onClick={() => setModalOpen(true)}>
              <IconPlus className="h-4 w-4" /> New Sale
            </Button>
          }
        />
      ) : (
        <div className="overflow-x-auto rounded-xl border border-slate-200 bg-white shadow-sm">
          <table className="w-full min-w-[1100px] border-collapse">
            <thead className="border-b border-slate-200 bg-slate-50">
              <tr>
                <th className="th">Sale</th>
                <th className="th">Event / Ticket</th>
                <th className="th">Date</th>
                <th className="th">Platform</th>
                <th className="th text-right">Price</th>
                <th className="th text-right">Fees</th>
                <th className="th text-right">Profit</th>
                <th className="th text-right">Margin</th>
                <th className="th text-right">ROI</th>
                <th className="th">Payment</th>
                <th className="th" />
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100">
              {sales.map((s) => (
                <tr key={s.id} className="hover:bg-slate-50">
                  <td className="td font-medium text-slate-900">
                    {s.code}
                    {s.isDemo && (
                      <span className="ml-1.5">
                        <Badge tone="demo">demo</Badge>
                      </span>
                    )}
                  </td>
                  <td className="td">
                    <Link to={`/events/${s.eventId}`} className="hover:text-brand-700">
                      {s.eventName}
                    </Link>
                    <p className="text-xs text-slate-400">{s.ticketCode}</p>
                  </td>
                  <td className="td whitespace-nowrap">{formatDate(s.saleDate)}</td>
                  <td className="td">{s.platformName ?? "-"}</td>
                  <td className="td text-right tabular-nums">{formatMoney(s.salePriceCents, s.currency)}</td>
                  <td className="td text-right tabular-nums">{formatMoney(s.sellingFeesCents, s.currency)}</td>
                  <td
                    className={`td text-right tabular-nums font-medium ${s.profitCents > 0 ? "text-emerald-600" : s.profitCents < 0 ? "text-red-600" : ""}`}
                  >
                    {formatMoney(s.profitCents, s.currency)}
                  </td>
                  <td className="td text-right tabular-nums">{formatPercent(s.margin)}</td>
                  <td className="td text-right tabular-nums">{formatPercent(s.roi)}</td>
                  <td className="td">
                    <Badge tone={s.paymentStatus}>{s.paymentStatus}</Badge>
                  </td>
                  <td className="td">
                    <div className="flex items-center justify-end gap-3">
                      <button
                        className="text-xs font-medium text-brand-600 hover:underline"
                        onClick={() => setEditTarget(s)}
                      >
                        Edit
                      </button>
                      <button
                        className="text-slate-400 hover:text-red-600"
                        title="Delete sale (returns ticket to available)"
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

      <SaleFormModal
        open={modalOpen}
        onClose={() => setModalOpen(false)}
        onCreated={() => {
          setModalOpen(false);
          load();
        }}
      />

      <SaleEditModal
        open={!!editTarget}
        sale={editTarget}
        onClose={() => setEditTarget(null)}
        onSaved={() => {
          setEditTarget(null);
          load();
        }}
      />

      <ConfirmDialog
        open={!!deleteTarget}
        title="Delete this sale?"
        message={
          <>
            This removes sale <b>{deleteTarget?.code}</b> and sets ticket{" "}
            <b>{deleteTarget?.ticketCode}</b> back to Available. This cannot be undone.
          </>
        }
        confirmLabel="Delete sale"
        danger
        busy={deleting}
        onCancel={() => setDeleteTarget(null)}
        onConfirm={async () => {
          if (!deleteTarget) return;
          setDeleting(true);
          try {
            await api.deleteSale(deleteTarget.id);
            toast.success("Sale deleted, ticket is available again");
            setDeleteTarget(null);
            load();
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

function SaleFormModal({
  open,
  onClose,
  onCreated,
}: {
  open: boolean;
  onClose: () => void;
  onCreated: () => void;
}) {
  const toast = useToast();
  const [ticket, setTicket] = useState<Ticket | null>(null);
  const [ticketQuery, setTicketQuery] = useState("");
  const [ticketOptions, setTicketOptions] = useState<Ticket[]>([]);
  const [platforms, setPlatforms] = useState<Platform[]>([]);
  const [platformId, setPlatformId] = useState<number | null>(null);
  const [saleDate, setSaleDate] = useState(todayIso());
  const [salePrice, setSalePrice] = useState("");
  const [sellingFees, setSellingFees] = useState("0");
  const [paymentStatus, setPaymentStatus] = useState<SalePaymentStatus>("pending");
  const [buyerReference, setBuyerReference] = useState("");
  const [notes, setNotes] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!open) return;
    setTicket(null);
    setTicketQuery("");
    setTicketOptions([]);
    setPlatformId(null);
    setSaleDate(todayIso());
    setSalePrice("");
    setSellingFees("0");
    setPaymentStatus("pending");
    setBuyerReference("");
    setNotes("");
    setError(null);
    api.listPlatforms().then(setPlatforms).catch(() => {});
  }, [open]);

  useEffect(() => {
    if (!open || ticket) return;
    const t = setTimeout(() => {
      api
        .listTickets({ search: ticketQuery || undefined, status: "available,listed", sortBy: "created", sortDir: "desc" })
        .then((res) => setTicketOptions(res.slice(0, 25)))
        .catch(() => {});
    }, 200);
    return () => clearTimeout(t);
  }, [open, ticket, ticketQuery]);

  const submit = async () => {
    setError(null);
    if (!ticket) return setError("Select a ticket to sell first");
    const s = salePrice.trim().replace(",", ".");
    if (!/^\d+(\.\d{1,2})?$/.test(s)) return setError("Sale price is not a valid amount");
    const feesStr = sellingFees.trim().replace(",", ".") || "0";
    if (!/^\d+(\.\d{1,2})?$/.test(feesStr)) return setError("Selling fees is not a valid amount");
    if (!saleDate) return setError("Sale date is required");

    const input: SaleInput = {
      ticketId: ticket.id,
      platformId,
      saleDate,
      salePriceCents: Math.round(parseFloat(s) * 100),
      sellingFeesCents: Math.round(parseFloat(feesStr) * 100),
      paymentStatus,
      buyerReference: buyerReference || null,
      notes: notes || null,
    };
    setSaving(true);
    try {
      const sale = await api.createSale(input);
      toast.success(`${sale.code} recorded - ${ticket.code} marked as sold`);
      onCreated();
    } catch (e) {
      setError(errMsg(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal open={open} onClose={onClose} title="New sale" width="max-w-xl">
      {!ticket ? (
        <div>
          <Field label="Find a ticket to sell" required hint="Only Available and Listed tickets can be sold">
            <Input
              autoFocus
              placeholder="Search by code, event, seat..."
              value={ticketQuery}
              onChange={(e) => setTicketQuery(e.target.value)}
            />
          </Field>
          <div className="mt-3 max-h-72 divide-y divide-slate-100 overflow-y-auto rounded-lg border border-slate-200">
            {ticketOptions.length === 0 ? (
              <p className="p-4 text-center text-sm text-slate-400">
                {ticketQuery ? "No matching available/listed tickets" : "Start typing to search your inventory"}
              </p>
            ) : (
              ticketOptions.map((t) => (
                <button
                  key={t.id}
                  className="flex w-full items-center justify-between gap-3 px-3 py-2 text-left hover:bg-slate-50"
                  onClick={() => setTicket(t)}
                >
                  <span className="min-w-0">
                    <span className="block truncate text-sm font-medium text-slate-800">{t.code} &middot; {t.eventName}</span>
                    <span className="block truncate text-xs text-slate-400">
                      {[t.section, t.rowLabel, t.seat].filter(Boolean).join(" / ") || "No seat info"}
                    </span>
                  </span>
                  <span className="shrink-0">
                    <Badge tone={t.status}>{t.status}</Badge>
                  </span>
                </button>
              ))
            )}
          </div>
        </div>
      ) : (
        <>
          <div className="mb-4 flex items-center justify-between rounded-lg bg-slate-50 px-4 py-3">
            <div>
              <p className="text-sm font-semibold text-slate-800">
                {ticket.code} &middot; {ticket.eventName}
              </p>
              <p className="text-xs text-slate-400">
                Cost {formatMoney(ticket.totalCostCents, ticket.currency)}
                {[ticket.section, ticket.rowLabel, ticket.seat].some(Boolean)
                  ? ` · ${[ticket.section, ticket.rowLabel, ticket.seat].filter(Boolean).join(" / ")}`
                  : ""}
              </p>
            </div>
            <button className="text-xs font-medium text-brand-600 hover:underline" onClick={() => setTicket(null)}>
              Change
            </button>
          </div>

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
            <Field label={`Sale price (${ticket.currency})`} required>
              <Input inputMode="decimal" placeholder="0.00" value={salePrice} onChange={(e) => setSalePrice(e.target.value)} />
            </Field>
            <Field label="Selling fees">
              <Input inputMode="decimal" value={sellingFees} onChange={(e) => setSellingFees(e.target.value)} />
            </Field>
            <Field label="Payment status">
              <Select value={paymentStatus} onChange={(e) => setPaymentStatus(e.target.value as SalePaymentStatus)}>
                <option value="pending">Pending</option>
                <option value="paid">Paid</option>
                <option value="refunded">Refunded</option>
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
        </>
      )}

      {error && <p className="mt-3 text-sm text-red-600">{error}</p>}
      <ModalFooter>
        <Button variant="secondary" onClick={onClose} disabled={saving}>
          Cancel
        </Button>
        <Button variant="primary" onClick={submit} disabled={saving || !ticket}>
          {saving ? "Recording..." : "Record sale"}
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
        <Field label="Payment status">
          <Select value={paymentStatus} onChange={(e) => setPaymentStatus(e.target.value as SalePaymentStatus)}>
            <option value="pending">Pending</option>
            <option value="paid">Paid</option>
            <option value="refunded">Refunded</option>
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
      {error && <p className="mt-3 text-sm text-red-600">{error}</p>}
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

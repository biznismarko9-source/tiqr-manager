import { useEffect, useState, type ReactNode } from "react";
import { api, errMsg } from "../lib/api";
import type { Platform, Pull, PullEditInput, PullInput } from "../lib/types";
import {
  centsToDecimalString,
  decimalStringToCents,
  formatDate,
  formatMoney,
  formatSeatLocation,
  todayIso,
} from "../lib/format";
import {
  Button,
  CHECKBOX_CLASS,
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
import { IconAlertTriangle, IconPlus, IconSearch, IconUsers } from "../components/icons";
import { useToast } from "../lib/toast";

const CURRENCIES = ["EUR", "USD", "GBP", "CHF", "CZK", "PLN", "HUF", "SEK", "NOK", "DKK", "RON", "TRY", "BGN"];

// Same safety-cap convention as Orders.tsx/Tickets.tsx/Sales.tsx - mirrors
// the backend's own LIST_CAP (commands/pulls.rs) so the banner below only
// ever shows once the backend has actually truncated the results.
const LIST_CAP = 5000;

// Session-only "remember the last search" convention, same as Orders.tsx's
// lastOrdersSearch / Sales.tsx's lastFilters - resets on app restart.
let lastPullsSearch: string | null = null;

// 1.9.8: how many days before the event the "transfer this!" warning starts
// showing (and keeps showing every day, escalating once the event date
// itself has passed) - replaces the old manual "Transfer deadline" field
// marko had to fill in by hand. Local to this file only.
const WARNING_WINDOW_DAYS = 3;

type TransferFilter = "all" | "pending" | "done";

/** Whole days between today and `dateIso` (positive = in the future,
 * negative = already passed). Plain calendar-day difference, not
 * time-of-day-sensitive - matches how `eventDate`/`todayIso()` are always
 * plain "YYYY-MM-DD" strings in this app. */
function daysUntil(dateIso: string): number {
  const start = new Date(`${todayIso()}T00:00:00`);
  const end = new Date(dateIso.length <= 10 ? `${dateIso}T00:00:00` : dateIso);
  return Math.round((end.getTime() - start.getTime()) / 86_400_000);
}

function warningLabel(daysLeft: number): string {
  if (daysLeft > 0) return `${daysLeft}d left`;
  if (daysLeft === 0) return "Today!";
  return `${Math.abs(daysLeft)}d overdue`;
}

/** Pull (1.9.7): tickets bought on someone else's behalf for a fee. Unlike
 * Orders/Sales, a pull has no child entities (no tickets are generated) so
 * there's no separate Detail page - this single list page owns both create
 * and edit via one shared PullFormModal below. See
 * src-tauri/migrations/005_pulls.sql for the full feature rationale. */
export default function Pulls() {
  const toast = useToast();
  const [pulls, setPulls] = useState<Pull[] | null>(null);
  const [search, setSearch] = useState(lastPullsSearch ?? "");
  const [statusFilter, setStatusFilter] = useState<TransferFilter>("all");
  // undefined = modal closed, null = create mode, a Pull = edit mode.
  const [modalPull, setModalPull] = useState<Pull | null | undefined>(undefined);
  const [deleteTarget, setDeleteTarget] = useState<Pull | null>(null);
  const [deleting, setDeleting] = useState(false);

  useEffect(() => {
    lastPullsSearch = search;
  }, [search]);

  const load = (q?: string, filter?: TransferFilter) => {
    const f = filter ?? statusFilter;
    api
      .listPulls({ search: q || undefined, transferDone: f === "all" ? undefined : f === "done" })
      .then(setPulls)
      .catch((e) => toast.error(errMsg(e)));
  };

  useEffect(() => {
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const t = setTimeout(() => load(search), 250);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [search, statusFilter]);

  const confirmDelete = async () => {
    if (!deleteTarget) return;
    setDeleting(true);
    try {
      await api.deletePull(deleteTarget.id);
      toast.success(`Pull ${deleteTarget.code} deleted`);
      setDeleteTarget(null);
      setModalPull(undefined);
      load(search);
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setDeleting(false);
    }
  };

  const toggleTransferDone = async (p: Pull) => {
    try {
      const updated = await api.setPullTransferDone(p.id, !p.transferDone);
      setPulls((prev) => (prev ? prev.map((x) => (x.id === updated.id ? updated : x)) : prev));
    } catch (e) {
      toast.error(errMsg(e));
    }
  };

  return (
    <div>
      <PageHeader
        title="Pulls"
        subtitle="Tickets bought on someone else's behalf for a fee - queue, pay, transfer, get paid."
        actions={
          <Button variant="primary" onClick={() => setModalPull(null)}>
            <IconPlus className="h-4 w-4" /> New Pull
          </Button>
        }
      />

      <div className="mb-4 flex flex-wrap items-center gap-3">
        <div className="relative max-w-xs flex-1">
          <IconSearch className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-400 dark:text-slate-500" />
          <Input
            placeholder="Search pulls..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="pl-9"
          />
        </div>
        <div className="w-48">
          <Select value={statusFilter} onChange={(e) => setStatusFilter(e.target.value as TransferFilter)}>
            <option value="all">All pulls</option>
            <option value="pending">Not transferred yet</option>
            <option value="done">Transferred</option>
          </Select>
        </div>
      </div>

      {pulls && pulls.length >= LIST_CAP && (
        <div className="mb-4 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-800 dark:border-amber-500/30 dark:bg-amber-500/10 dark:text-amber-400">
          Showing the most recent {LIST_CAP.toLocaleString()} pulls that match your filters. Narrow the search to see
          the rest.
        </div>
      )}

      {pulls === null ? (
        <LoadingBlock />
      ) : pulls.length === 0 ? (
        <EmptyState
          icon={<IconUsers className="h-8 w-8" />}
          title="No pulls yet"
          description="Record a pull when you queue up to buy tickets on someone else's behalf."
          action={
            <Button variant="primary" onClick={() => setModalPull(null)}>
              <IconPlus className="h-4 w-4" /> New Pull
            </Button>
          }
        />
      ) : (
        // Same table-fixed + colgroup convention as Orders.tsx/Sales.tsx.
        // 1.9.8 first put seat location and More info inside the Event cell
        // - marko said no, he wants them as their own columns instead (they
        // were getting lost stacked under the event name), so Event went
        // back to just name + date, and Seats/More info are now two
        // dedicated columns of their own. Seats shows via the same
        // formatSeatLocation helper Sale Detail uses (falls back to
        // "General admission" when Section/Row/Seat are all blank). The old
        // manual "Deadline" column is still gone - replaced by a warning
        // that appears automatically starting WARNING_WINDOW_DAYS before the
        // event date and disappears the moment transfer is marked done.
        // Row click opens the edit modal (no separate Detail page exists for
        // Pull, unlike Order/Sale) - guarded so a click on the checkbox
        // doesn't also open it.
        <div className="overflow-x-auto rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-sm">
          <table className="w-full table-fixed border-collapse">
            <colgroup>
              <col className="w-[80px]" />
              <col className="w-[92px]" />
              <col />
              <col className="w-[92px]" />
              <col className="w-[136px]" />
              <col className="w-8" />
              <col className="w-[84px]" />
              <col className="w-[72px]" />
              <col className="w-[76px]" />
              <col className="w-11" />
            </colgroup>
            <thead className="border-b border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-800/60">
              <tr>
                <th className="th-c">Pull</th>
                <th className="th-c">For</th>
                <th className="th-c">Event</th>
                <th className="th-c">Seats</th>
                <th className="th-c">More info</th>
                <th className="th-c text-right">Ks</th>
                <th className="th-c">Platform</th>
                <th className="th-c text-right">Fee</th>
                <th className="th-c">Warning</th>
                <th className="th-c text-center">Done</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
              {pulls.map((p) => {
                const daysLeft = p.eventDate ? daysUntil(p.eventDate) : null;
                const seatLocation = formatSeatLocation(p.section, p.rowLabel, p.seat);
                const showWarning = !p.transferDone && daysLeft !== null && daysLeft <= WARNING_WINDOW_DAYS;
                const warningText = daysLeft !== null ? warningLabel(daysLeft) : "";
                const warningTone = daysLeft !== null && daysLeft <= 0 ? "red" : "amber";
                return (
                  <tr
                    key={p.id}
                    className="cursor-pointer hover:bg-slate-50 dark:hover:bg-slate-800/60"
                    onClick={(e) => {
                      if ((e.target as HTMLElement).closest("input, button")) return;
                      setModalPull(p);
                    }}
                  >
                    <td
                      className="td-c truncate align-top font-medium text-slate-900 dark:text-slate-100"
                      title={`Added ${formatDate(p.createdAt)}`}
                    >
                      {p.code}
                    </td>
                    <td className="td-c truncate align-top" title={p.buyerName}>
                      {p.buyerName}
                    </td>
                    <td className="td-c align-top py-2">
                      <div className="truncate font-medium text-slate-800 dark:text-slate-200" title={p.eventName}>
                        {p.eventName}
                      </div>
                      {p.eventDate && (
                        <div className="truncate text-xs text-slate-400 dark:text-slate-500">
                          {formatDate(p.eventDate)}
                        </div>
                      )}
                    </td>
                    <td
                      className="td-c truncate align-top text-xs text-slate-500 dark:text-slate-400"
                      title={seatLocation}
                    >
                      {seatLocation}
                    </td>
                    <td
                      className="td-c truncate align-top text-xs text-slate-500 dark:text-slate-400"
                      title={p.moreInfo ?? undefined}
                    >
                      {p.moreInfo || "-"}
                    </td>
                    <td className="td-c text-right align-top tabular-nums">{p.quantity}</td>
                    <td className="td-c truncate align-top" title={p.platformName ?? undefined}>
                      {p.platformName ?? "-"}
                    </td>
                    <td className="td-c text-right align-top tabular-nums">{formatMoney(p.priceCents, p.currency)}</td>
                    <td className="td-c align-top">
                      {showWarning && (
                        <span
                          className={`inline-flex items-center gap-1 whitespace-nowrap text-xs font-medium ${
                            warningTone === "red" ? "text-red-600 dark:text-red-400" : "text-amber-600 dark:text-amber-400"
                          }`}
                        >
                          <IconAlertTriangle className="h-3.5 w-3.5 shrink-0" />
                          {warningText}
                        </span>
                      )}
                    </td>
                    <td className="td-c text-center align-top">
                      <input
                        type="checkbox"
                        className={CHECKBOX_CLASS}
                        checked={p.transferDone}
                        onChange={() => toggleTransferDone(p)}
                        aria-label={`Mark pull ${p.code} as ${p.transferDone ? "not transferred" : "transferred"}`}
                      />
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}

      <PullFormModal
        open={modalPull !== undefined}
        pull={modalPull ?? null}
        onClose={() => setModalPull(undefined)}
        onSaved={() => {
          setModalPull(undefined);
          load(search);
        }}
        onRequestDelete={(p) => setDeleteTarget(p)}
      />

      <ConfirmDialog
        open={!!deleteTarget}
        title="Delete this pull?"
        message={
          <>
            This removes <strong>{deleteTarget?.code}</strong> ({deleteTarget?.buyerName} - {deleteTarget?.eventName})
            permanently. This can't be undone.
          </>
        }
        confirmLabel="Delete"
        danger
        busy={deleting}
        onConfirm={confirmDelete}
        onCancel={() => setDeleteTarget(null)}
      />
    </div>
  );
}

/** Same lightweight section-grouping helper as Orders.tsx's own local
 * FormGroup - kept local here too rather than promoted to ui.tsx, same
 * reasoning as that file's comment (still only two forms in the app want
 * this kind of sectioning). */
function FormGroup({ title, children }: { title?: string; children: ReactNode }) {
  return (
    <div className="border-t border-slate-200 pt-4 first:border-t-0 first:pt-0 dark:border-slate-800">
      {title && (
        <p className="mb-3 text-xs font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">{title}</p>
      )}
      <div className="grid grid-cols-2 gap-4">{children}</div>
    </div>
  );
}

function PullFormModal({
  open,
  pull,
  onClose,
  onSaved,
  onRequestDelete,
}: {
  open: boolean;
  pull: Pull | null;
  onClose: () => void;
  onSaved: (pull: Pull) => void;
  onRequestDelete: (pull: Pull) => void;
}) {
  const toast = useToast();
  const editing = pull !== null;
  const [platforms, setPlatforms] = useState<Platform[]>([]);

  const [buyerName, setBuyerName] = useState("");
  const [eventName, setEventName] = useState("");
  const [eventDate, setEventDate] = useState("");
  const [quantity, setQuantity] = useState("1");
  const [platformId, setPlatformId] = useState<number | null>(null);
  const [section, setSection] = useState("");
  const [rowLabel, setRowLabel] = useState("");
  const [seat, setSeat] = useState("");
  const [moreInfo, setMoreInfo] = useState("");
  const [price, setPrice] = useState("");
  const [currency, setCurrency] = useState("EUR");
  const [customCurrency, setCustomCurrency] = useState(false);
  const [transferDone, setTransferDone] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!open) return;
    api.listPlatforms().then(setPlatforms).catch(() => {});
    if (pull) {
      setBuyerName(pull.buyerName);
      setEventName(pull.eventName);
      setEventDate(pull.eventDate ?? "");
      setQuantity(String(pull.quantity));
      setPlatformId(pull.platformId);
      setSection(pull.section ?? "");
      setRowLabel(pull.rowLabel ?? "");
      setSeat(pull.seat ?? "");
      setMoreInfo(pull.moreInfo ?? "");
      setPrice(centsToDecimalString(pull.priceCents));
      setCurrency(pull.currency);
      setCustomCurrency(!CURRENCIES.includes(pull.currency));
      setTransferDone(pull.transferDone);
    } else {
      setBuyerName("");
      setEventName("");
      setEventDate("");
      setQuantity("1");
      setPlatformId(null);
      setSection("");
      setRowLabel("");
      setSeat("");
      setMoreInfo("");
      setPrice("");
      setCurrency("EUR");
      setCustomCurrency(false);
      setTransferDone(false);
    }
    setError(null);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, pull]);

  const submit = async () => {
    setError(null);
    const q = parseInt(quantity, 10);
    const priceCents = decimalStringToCents(price);

    if (!buyerName.trim()) return setError("Buyer name is required");
    if (!eventName.trim()) return setError("Event name is required");
    if (!Number.isFinite(q) || q < 1) return setError("Quantity must be at least 1");
    if (priceCents === null) return setError("Fee is not a valid amount");
    if (!currency.trim()) return setError("Currency is required");

    setSaving(true);
    try {
      if (editing && pull) {
        const input: PullEditInput = {
          buyerName: buyerName.trim(),
          eventName: eventName.trim(),
          eventDate: eventDate || null,
          quantity: q,
          platformId,
          section: section.trim() || null,
          rowLabel: rowLabel.trim() || null,
          seat: seat.trim() || null,
          moreInfo: moreInfo.trim() || null,
          priceCents,
          currency,
          transferDone,
        };
        const updated = await api.updatePull(pull.id, input);
        toast.success(`Pull ${updated.code} updated`);
        onSaved(updated);
      } else {
        const input: PullInput = {
          buyerName: buyerName.trim(),
          eventName: eventName.trim(),
          eventDate: eventDate || null,
          quantity: q,
          platformId,
          section: section.trim() || null,
          rowLabel: rowLabel.trim() || null,
          seat: seat.trim() || null,
          moreInfo: moreInfo.trim() || null,
          priceCents,
          currency,
        };
        const created = await api.createPull(input);
        toast.success(`Pull ${created.code} created`);
        onSaved(created);
      }
    } catch (e) {
      setError(errMsg(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal open={open} onClose={onClose} title={editing ? `Edit ${pull?.code}` : "New pull"} width="max-w-2xl">
      <div className="flex flex-col gap-4">
        <FormGroup title="Pull">
          <Field label="For (buyer)" required hint="Who you're pulling these tickets for">
            <Input value={buyerName} onChange={(e) => setBuyerName(e.target.value)} />
          </Field>
          <Field label="Quantity" required>
            <Input type="number" min={1} step={1} value={quantity} onChange={(e) => setQuantity(e.target.value)} />
          </Field>
          <Field label="Event name" required hint="Free text - not linked to your Events list">
            <Input value={eventName} onChange={(e) => setEventName(e.target.value)} />
          </Field>
          <Field label="Event date">
            <Input type="date" value={eventDate} onChange={(e) => setEventDate(e.target.value)} />
          </Field>
          <div className="col-span-2">
            <LookupSelect
              label="Platform"
              options={platforms.filter((p) => p.kind === "purchase" || p.kind === "both")}
              value={platformId}
              onChange={setPlatformId}
              onCreate={async (name) => {
                const p = await api.createPlatform(name, "purchase");
                setPlatforms((prev) => [...prev, p]);
                return p;
              }}
            />
          </div>
        </FormGroup>

        <FormGroup title="Seats & details">
          <Field label="Section" hint="Optional - leave blank for general admission">
            <Input value={section} onChange={(e) => setSection(e.target.value)} />
          </Field>
          <Field label="Row" hint="Optional">
            <Input value={rowLabel} onChange={(e) => setRowLabel(e.target.value)} />
          </Field>
          <Field label="Seat" hint="Optional">
            <Input value={seat} onChange={(e) => setSeat(e.target.value)} />
          </Field>
          <Field label="More info" hint="Optional - e.g. the email the tickets will arrive on">
            <Textarea rows={2} value={moreInfo} onChange={(e) => setMoreInfo(e.target.value)} />
          </Field>
        </FormGroup>

        <FormGroup title="Fee & tracking">
          <Field label={`Your fee (${currency})`} required hint="What you earn for the pull - not the ticket price">
            <Input inputMode="decimal" placeholder="0.00" value={price} onChange={(e) => setPrice(e.target.value)} />
          </Field>
          <div>
            <div className="flex items-center justify-between">
              <span className="label mb-1">Currency</span>
              <button
                type="button"
                className="mb-1 text-xs font-medium text-brand-600 dark:text-brand-400 hover:underline"
                onClick={() => setCustomCurrency((c) => !c)}
              >
                {customCurrency ? "Choose from list" : "Other..."}
              </button>
            </div>
            {customCurrency ? (
              <Input
                autoFocus
                placeholder="e.g. AED"
                value={currency}
                onChange={(e) => setCurrency(e.target.value.toUpperCase())}
              />
            ) : (
              <Select value={currency} onChange={(e) => setCurrency(e.target.value)}>
                {(CURRENCIES.includes(currency) ? CURRENCIES : [currency, ...CURRENCIES]).map((c) => (
                  <option key={c} value={c}>
                    {c}
                  </option>
                ))}
              </Select>
            )}
          </div>
          <div className="col-span-2">
            {editing ? (
              <label className="flex items-center gap-2">
                <input
                  type="checkbox"
                  className={CHECKBOX_CLASS}
                  checked={transferDone}
                  onChange={(e) => setTransferDone(e.target.checked)}
                />
                <span className="text-sm text-slate-700 dark:text-slate-300">Transfer done</span>
              </label>
            ) : (
              <p className="text-xs text-slate-400 dark:text-slate-500">
                A warning appears automatically starting {WARNING_WINDOW_DAYS} days before the event date, and every
                day after that, until this pull is marked as transferred.
              </p>
            )}
          </div>
        </FormGroup>
      </div>

      {error && <p className="mt-3 text-sm text-red-600 dark:text-red-400">{error}</p>}
      <ModalFooter>
        {editing && pull && (
          <Button variant="danger" className="mr-auto" onClick={() => onRequestDelete(pull)} disabled={saving}>
            Delete
          </Button>
        )}
        <Button variant="secondary" onClick={onClose} disabled={saving}>
          Cancel
        </Button>
        <Button variant="primary" onClick={submit} disabled={saving}>
          {saving ? "Saving..." : editing ? "Save changes" : "Create pull"}
        </Button>
      </ModalFooter>
    </Modal>
  );
}

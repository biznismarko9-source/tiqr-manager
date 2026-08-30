import { useEffect, useMemo, useState } from "react";
import { api, errMsg } from "../../lib/api";
import type { Account, FinanceCategory, FinanceEntry, FinanceEntryInput, Transfer } from "../../lib/types";
import { centsToDecimalString, decimalStringToCents, formatDate, formatMoney, todayIso } from "../../lib/format";
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
  Spinner,
  Textarea,
} from "../../components/ui";
import { FinanceCategoryBadge } from "../../components/FinanceCategoryBadge";
import { IconPencil, IconPlus, IconSearch, IconTrash } from "../../components/icons";
import { useToast } from "../../lib/toast";
import { CURRENCIES } from "../Orders";
import { PERIODS, SCOPES, periodBounds, type FinanceData, type PeriodKey, type ScopeFilter } from "./shared";

// 2.1.0: marko's own point 11 - "Transactions sekcia má byť hlavný zoznam:
// Income, Expense, Transfer." This is the ORIGINAL Finance.tsx's own entry
// list/EntryFormModal (unchanged - just moved here), now merged with
// transfers into one unified, sortable, filterable list. A transfer is
// still never stored as (or treated as) a FinanceEntry anywhere - the merge
// happens ONLY here, at display time, exactly the same "flat list + 100%
// client-side filtering" philosophy the entries list already used.

type TypeFilter = "all" | "income" | "expense" | "transfer";

/** One unified row - either a real `FinanceEntry` or a `Transfer`, tagged so
 * the table/filters can treat both uniformly without ever pretending a
 * transfer IS an entry. */
type Row = { kind: "entry"; date: string; entry: FinanceEntry } | { kind: "transfer"; date: string; transfer: Transfer };

export default function Transactions({ entries, categories, accounts, transfers, loading, reload }: FinanceData) {
  const toast = useToast();

  const [period, setPeriod] = useState<PeriodKey>("all");
  const [customFrom, setCustomFrom] = useState("");
  const [customTo, setCustomTo] = useState("");
  const [scopeFilter, setScopeFilter] = useState<ScopeFilter>("all");
  const [typeFilter, setTypeFilter] = useState<TypeFilter>("all");
  const [accountFilter, setAccountFilter] = useState<string>("all");
  const [categoryFilter, setCategoryFilter] = useState<string>("all");
  const [search, setSearch] = useState("");

  const [formOpen, setFormOpen] = useState(false);
  const [editing, setEditing] = useState<FinanceEntry | null>(null);
  const [deleteEntryTarget, setDeleteEntryTarget] = useState<FinanceEntry | null>(null);
  const [deleteTransferTarget, setDeleteTransferTarget] = useState<Transfer | null>(null);
  const [deleting, setDeleting] = useState(false);

  const { from, to } = periodBounds(period, customFrom, customTo);
  const customDatesMissing = period === "custom" && !customFrom && !customTo;

  const rows = useMemo<Row[]>(() => {
    if (customDatesMissing) return [];
    const entryRows: Row[] = entries.map((entry) => ({ kind: "entry", date: entry.entryDate, entry }));
    const transferRows: Row[] = transfers.map((transfer) => ({ kind: "transfer", date: transfer.transferDate, transfer }));
    return [...entryRows, ...transferRows]
      .filter((r) => {
        if (from && r.date < from) return false;
        if (to && r.date > to) return false;
        return true;
      })
      .filter((r) => {
        // Scope only applies to entries - a transfer has no scope concept
        // (marko's own point 5: it's neither income nor expense, personal
        // or business, just a movement of money already his own).
        if (scopeFilter === "all") return true;
        return r.kind === "entry" && r.entry.scope === scopeFilter;
      })
      .filter((r) => {
        if (typeFilter === "all") return true;
        if (typeFilter === "transfer") return r.kind === "transfer";
        return r.kind === "entry" && r.entry.entryType === typeFilter;
      })
      .filter((r) => {
        if (accountFilter === "all") return true;
        const id = Number(accountFilter);
        if (r.kind === "entry") return r.entry.accountId === id;
        return r.transfer.fromAccountId === id || r.transfer.toAccountId === id;
      })
      .filter((r) => {
        // A transfer never has a category - excluded whenever a specific
        // category filter (including "No category") is active, shown only
        // when browsing every category at once.
        if (categoryFilter === "all") return true;
        if (r.kind === "transfer") return false;
        if (categoryFilter === "none") return r.entry.categoryId === null;
        return String(r.entry.categoryId) === categoryFilter;
      })
      .filter((r) => {
        if (!search.trim()) return true;
        const q = search.trim().toLowerCase();
        const hay =
          r.kind === "entry"
            ? `${r.entry.place ?? ""} ${r.entry.note ?? ""} ${r.entry.categoryName ?? ""}`
            : `${r.transfer.note ?? ""} ${r.transfer.fromAccountName ?? ""} ${r.transfer.toAccountName ?? ""}`;
        return hay.toLowerCase().includes(q);
      })
      .sort((a, b) => (a.date < b.date ? 1 : a.date > b.date ? -1 : 0));
  }, [entries, transfers, from, to, scopeFilter, typeFilter, accountFilter, categoryFilter, search, customDatesMissing]);

  const openAdd = () => {
    setEditing(null);
    setFormOpen(true);
  };
  const openEdit = (entry: FinanceEntry) => {
    setEditing(entry);
    setFormOpen(true);
  };

  const doDeleteEntry = async () => {
    if (!deleteEntryTarget) return;
    setDeleting(true);
    try {
      await api.deleteFinanceEntry(deleteEntryTarget.id);
      setDeleteEntryTarget(null);
      toast.success("Entry deleted.");
      reload();
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setDeleting(false);
    }
  };

  const doDeleteTransfer = async () => {
    if (!deleteTransferTarget) return;
    setDeleting(true);
    try {
      await api.deleteTransfer(deleteTransferTarget.id);
      setDeleteTransferTarget(null);
      toast.success("Transfer deleted.");
      reload();
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setDeleting(false);
    }
  };

  return (
    <div>
      <Card className="mb-4 flex flex-wrap items-center gap-3 p-3">
        <div className="flex flex-wrap items-center gap-1 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-1">
          {PERIODS.map((p) => (
            <button
              key={p.key}
              onClick={() => setPeriod(p.key)}
              className={`rounded-md px-2.5 py-1.5 text-xs font-medium transition-colors ${
                period === p.key ? "bg-brand-600 text-white" : "text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800"
              }`}
            >
              {p.label}
            </button>
          ))}
        </div>
        <div className="flex flex-wrap items-center gap-1 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-1">
          {SCOPES.map((s) => (
            <button
              key={s.key}
              onClick={() => setScopeFilter(s.key)}
              className={`rounded-md px-2.5 py-1.5 text-xs font-medium transition-colors ${
                scopeFilter === s.key ? "bg-brand-600 text-white" : "text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800"
              }`}
            >
              {s.label}
            </button>
          ))}
        </div>
        {period === "custom" && (
          <div className="flex flex-wrap items-end gap-3">
            <label className="text-xs font-medium text-slate-600 dark:text-slate-400">
              From
              <Input type="date" value={customFrom} onChange={(e) => setCustomFrom(e.target.value)} className="mt-1" />
            </label>
            <label className="text-xs font-medium text-slate-600 dark:text-slate-400">
              To
              <Input type="date" value={customTo} onChange={(e) => setCustomTo(e.target.value)} className="mt-1" />
            </label>
          </div>
        )}
        <Button variant="primary" className="ml-auto" onClick={openAdd}>
          <IconPlus className="h-4 w-4" /> New entry
        </Button>
      </Card>

      {loading ? (
        <LoadingBlock label="Loading transactions..." />
      ) : customDatesMissing ? (
        <div className="rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-800 dark:border-amber-500/30 dark:bg-amber-500/10 dark:text-amber-400">
          Please select at least one date.
        </div>
      ) : (
        <Card>
          <div className="flex flex-wrap items-center gap-2 border-b border-slate-100 dark:border-slate-800 px-4 py-3">
            <h3 className="mr-auto text-sm font-semibold text-slate-800 dark:text-slate-200">Transactions</h3>
            <div className="relative">
              <IconSearch className="pointer-events-none absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-slate-400 dark:text-slate-500" />
              <Input placeholder="Search place / note..." value={search} onChange={(e) => setSearch(e.target.value)} className="w-48 pl-8" />
            </div>
            <div className="w-32">
              <Select value={typeFilter} onChange={(e) => setTypeFilter(e.target.value as TypeFilter)}>
                <option value="all">All types</option>
                <option value="income">Income</option>
                <option value="expense">Expense</option>
                <option value="transfer">Transfer</option>
              </Select>
            </div>
            <div className="w-40">
              <Select value={accountFilter} onChange={(e) => setAccountFilter(e.target.value)}>
                <option value="all">All accounts</option>
                {accounts.map((a) => (
                  <option key={a.id} value={a.id}>
                    {a.name}
                  </option>
                ))}
              </Select>
            </div>
            <div className="w-44">
              <Select value={categoryFilter} onChange={(e) => setCategoryFilter(e.target.value)}>
                <option value="all">All categories</option>
                <option value="none">No category</option>
                {categories.map((c) => (
                  <option key={c.id} value={c.id}>
                    {c.name}
                  </option>
                ))}
              </Select>
            </div>
          </div>
          {rows.length === 0 ? (
            <div className="p-6">
              <EmptyState title="No transactions match these filters" />
            </div>
          ) : (
            <div className="overflow-x-auto">
              <table className="w-full border-collapse">
                <thead className="border-b border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-800/60">
                  <tr>
                    <th className="th">Date</th>
                    <th className="th">Type</th>
                    <th className="th">Category</th>
                    <th className="th">Account</th>
                    <th className="th">Place / who</th>
                    <th className="th">Note</th>
                    <th className="th text-right">Amount</th>
                    <th className="th" />
                  </tr>
                </thead>
                <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
                  {rows.map((r) =>
                    r.kind === "entry" ? (
                      <EntryRow key={`e-${r.entry.id}`} entry={r.entry} onEdit={openEdit} onDelete={setDeleteEntryTarget} />
                    ) : (
                      <TransferRow key={`t-${r.transfer.id}`} transfer={r.transfer} onDelete={setDeleteTransferTarget} />
                    ),
                  )}
                </tbody>
              </table>
            </div>
          )}
        </Card>
      )}

      <EntryFormModal open={formOpen} onClose={() => setFormOpen(false)} onSaved={reload} categories={categories} accounts={accounts} initial={editing} />

      <ConfirmDialog
        open={!!deleteEntryTarget}
        title="Delete this entry?"
        message={
          deleteEntryTarget ? (
            <>
              Removes the {deleteEntryTarget.entryType === "income" ? "income" : "expense"} of{" "}
              <b>{formatMoney(deleteEntryTarget.amountCents, deleteEntryTarget.currency)}</b> from {formatDate(deleteEntryTarget.entryDate)}. This
              cannot be undone.
            </>
          ) : (
            ""
          )
        }
        confirmLabel="Delete entry"
        danger
        busy={deleting}
        onCancel={() => setDeleteEntryTarget(null)}
        onConfirm={doDeleteEntry}
      />

      <ConfirmDialog
        open={!!deleteTransferTarget}
        title="Delete this transfer?"
        message={
          deleteTransferTarget ? (
            <>
              Removes the transfer of <b>{formatMoney(deleteTransferTarget.amountCents, deleteTransferTarget.currency)}</b> from{" "}
              {deleteTransferTarget.fromAccountName ?? "?"} to {deleteTransferTarget.toAccountName ?? "?"} on{" "}
              {formatDate(deleteTransferTarget.transferDate)}. Both accounts' balances will update. This cannot be undone.
            </>
          ) : (
            ""
          )
        }
        confirmLabel="Delete transfer"
        danger
        busy={deleting}
        onCancel={() => setDeleteTransferTarget(null)}
        onConfirm={doDeleteTransfer}
      />
    </div>
  );
}

function EntryRow({ entry, onEdit, onDelete }: { entry: FinanceEntry; onEdit: (e: FinanceEntry) => void; onDelete: (e: FinanceEntry) => void }) {
  return (
    <tr className="cursor-pointer hover:bg-slate-50 dark:hover:bg-slate-800/60" onClick={(ev) => {
      if ((ev.target as HTMLElement).closest("button")) return;
      onEdit(entry);
    }}>
      <td className="td whitespace-nowrap">{formatDate(entry.entryDate)}</td>
      <td className="td">
        <Badge tone={entry.entryType === "income" ? "sold" : "cancelled"}>{entry.entryType === "income" ? "Income" : "Expense"}</Badge>
        <span className="ml-1.5 text-xs text-slate-400 dark:text-slate-500">{entry.scope === "business" ? "Business" : "Personal"}</span>
      </td>
      <td className="td">
        {entry.categoryName && entry.categoryColorSlot !== null ? (
          <FinanceCategoryBadge name={entry.categoryName} colorSlot={entry.categoryColorSlot} />
        ) : (
          <span className="text-slate-300 dark:text-slate-600">-</span>
        )}
      </td>
      <td className="td">{entry.accountName ?? <span className="text-slate-300 dark:text-slate-600">-</span>}</td>
      <td className="td truncate" title={entry.place ?? undefined}>
        {entry.place ?? <span className="text-slate-300 dark:text-slate-600">-</span>}
      </td>
      <td className="td max-w-[220px] truncate" title={entry.note ?? undefined}>
        {entry.note ?? <span className="text-slate-300 dark:text-slate-600">-</span>}
      </td>
      <td className={`td text-right tabular-nums ${entry.entryType === "income" ? "text-emerald-600 dark:text-emerald-400" : "text-slate-700 dark:text-slate-300"}`}>
        {entry.entryType === "income" ? "+" : "-"}
        {formatMoney(entry.amountCents, entry.currency)}
      </td>
      <td className="td">
        <div className="flex items-center justify-end gap-1">
          <button type="button" className="rounded p-1 text-slate-300 hover:text-brand-600 dark:text-slate-600 dark:hover:text-brand-400" title="Edit" onClick={() => onEdit(entry)}>
            <IconPencil className="h-4 w-4" />
          </button>
          <button type="button" className="rounded p-1 text-slate-300 hover:text-red-600 dark:text-slate-600 dark:hover:text-red-400" title="Delete" onClick={() => onDelete(entry)}>
            <IconTrash className="h-4 w-4" />
          </button>
        </div>
      </td>
    </tr>
  );
}

function TransferRow({ transfer, onDelete }: { transfer: Transfer; onDelete: (t: Transfer) => void }) {
  return (
    <tr className="hover:bg-slate-50 dark:hover:bg-slate-800/60">
      <td className="td whitespace-nowrap">{formatDate(transfer.transferDate)}</td>
      <td className="td">
        <Badge tone="listed">Transfer</Badge>
      </td>
      <td className="td">
        <span className="text-slate-300 dark:text-slate-600">-</span>
      </td>
      <td className="td whitespace-nowrap">
        {transfer.fromAccountName ?? "?"} &rarr; {transfer.toAccountName ?? "?"}
      </td>
      <td className="td">
        <span className="text-slate-300 dark:text-slate-600">-</span>
      </td>
      <td className="td max-w-[220px] truncate" title={transfer.note ?? undefined}>
        {transfer.note ?? <span className="text-slate-300 dark:text-slate-600">-</span>}
      </td>
      <td className="td text-right tabular-nums text-slate-600 dark:text-slate-400">{formatMoney(transfer.amountCents, transfer.currency)}</td>
      <td className="td">
        <div className="flex items-center justify-end gap-1">
          <button type="button" className="rounded p-1 text-slate-300 hover:text-red-600 dark:text-slate-600 dark:hover:text-red-400" title="Delete" onClick={() => onDelete(transfer)}>
            <IconTrash className="h-4 w-4" />
          </button>
        </div>
      </td>
    </tr>
  );
}

// ---------------------------------------------------------------------------
// Add/edit entry modal - the ORIGINAL Finance.tsx's own EntryFormModal,
// unchanged except for the new optional Account field.
// ---------------------------------------------------------------------------

function EntryFormModal({
  open,
  onClose,
  onSaved,
  categories,
  accounts,
  initial,
}: {
  open: boolean;
  onClose: () => void;
  onSaved: () => void;
  categories: FinanceCategory[];
  accounts: Account[];
  initial: FinanceEntry | null;
}) {
  const toast = useToast();
  const [entryType, setEntryType] = useState<"income" | "expense">("expense");
  const [scope, setScope] = useState<"personal" | "business">("personal");
  const [entryDate, setEntryDate] = useState(todayIso());
  const [amount, setAmount] = useState("");
  const [currency, setCurrency] = useState("EUR");
  const [categoryId, setCategoryId] = useState("");
  const [accountId, setAccountId] = useState("");
  const [place, setPlace] = useState("");
  const [note, setNote] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    setEntryType(initial?.entryType ?? "expense");
    setScope(initial?.scope ?? "personal");
    setEntryDate(initial?.entryDate ?? todayIso());
    setAmount(initial ? centsToDecimalString(initial.amountCents) : "");
    setCurrency(initial?.currency ?? "EUR");
    setCategoryId(initial?.categoryId ? String(initial.categoryId) : "");
    setAccountId(initial?.accountId ? String(initial.accountId) : "");
    setNote(initial?.note ?? "");
    setPlace(initial?.place ?? "");
    setError(null);
  }, [open, initial]);

  // Only categories tagged for this entry type (or "both") are offered - a
  // category already picked before switching Income/Expense that no longer
  // fits just falls back to "No category" rather than silently staying
  // selected but hidden from the dropdown.
  const relevantCategories = categories.filter((c) => c.kind === entryType || c.kind === "both");
  useEffect(() => {
    if (categoryId && !relevantCategories.some((c) => String(c.id) === categoryId)) {
      setCategoryId("");
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [entryType]);

  // An account picked before switching currency that no longer matches
  // (accounts.rs requires an entry's currency to match its linked account's
  // own currency - see finance_entries::validate_account) falls back to "No
  // account" rather than silently staying selected but guaranteed to fail
  // on submit.
  const relevantAccounts = accounts.filter((a) => a.currency === currency.trim().toUpperCase());
  useEffect(() => {
    if (accountId && !relevantAccounts.some((a) => String(a.id) === accountId)) {
      setAccountId("");
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currency]);

  const submit = async () => {
    const cents = decimalStringToCents(amount);
    if (cents === null || cents <= 0) {
      setError("Enter a valid amount greater than 0.");
      return;
    }
    if (!entryDate) {
      setError("Pick a date.");
      return;
    }
    setSaving(true);
    setError(null);
    const input: FinanceEntryInput = {
      entryType,
      entryDate,
      amountCents: cents,
      currency,
      scope,
      categoryId: categoryId ? Number(categoryId) : null,
      accountId: accountId ? Number(accountId) : null,
      place: place.trim() || null,
      note: note.trim() || null,
    };
    try {
      if (initial) {
        await api.updateFinanceEntry(initial.id, input);
        toast.success("Entry updated.");
      } else {
        await api.createFinanceEntry(input);
        toast.success("Entry added.");
      }
      onSaved();
      onClose();
    } catch (e) {
      setError(errMsg(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal open={open} onClose={onClose} title={initial ? "Edit entry" : "New entry"}>
      <div className="space-y-3">
        <div className="grid grid-cols-2 gap-2">
          <div>
            <span className="label">Type</span>
            <div className="flex rounded-lg border border-slate-200 dark:border-slate-800 p-1">
              {(["expense", "income"] as const).map((t) => (
                <button
                  key={t}
                  type="button"
                  onClick={() => setEntryType(t)}
                  className={`flex-1 rounded-md px-2.5 py-1.5 text-xs font-medium transition-colors ${
                    entryType === t ? "bg-brand-600 text-white" : "text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800"
                  }`}
                >
                  {t === "income" ? "Income" : "Expense"}
                </button>
              ))}
            </div>
          </div>
          <div>
            <span className="label">Scope</span>
            <div className="flex rounded-lg border border-slate-200 dark:border-slate-800 p-1">
              {(["personal", "business"] as const).map((s) => (
                <button
                  key={s}
                  type="button"
                  onClick={() => setScope(s)}
                  className={`flex-1 rounded-md px-2.5 py-1.5 text-xs font-medium transition-colors ${
                    scope === s ? "bg-brand-600 text-white" : "text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800"
                  }`}
                >
                  {s === "personal" ? "Personal" : "Business"}
                </button>
              ))}
            </div>
          </div>
        </div>

        <Field label="Date" required>
          <Input type="date" value={entryDate} onChange={(e) => setEntryDate(e.target.value)} />
        </Field>

        <div className="grid grid-cols-[1fr_110px] gap-2">
          <Field label="Amount" required>
            <Input inputMode="decimal" placeholder="0.00" value={amount} onChange={(e) => setAmount(e.target.value)} />
          </Field>
          <Field label="Currency">
            <Select value={currency} onChange={(e) => setCurrency(e.target.value)}>
              {(CURRENCIES.includes(currency) ? CURRENCIES : [currency, ...CURRENCIES]).map((c) => (
                <option key={c} value={c}>
                  {c}
                </option>
              ))}
            </Select>
          </Field>
        </div>

        <Field label="Category" hint="Manage the list in Settings -> Lookups.">
          <Select value={categoryId} onChange={(e) => setCategoryId(e.target.value)}>
            <option value="">No category</option>
            {relevantCategories.map((c) => (
              <option key={c.id} value={c.id}>
                {c.name}
              </option>
            ))}
          </Select>
        </Field>

        <Field label="Account" hint={relevantAccounts.length === 0 && accounts.length > 0 ? `No ${currency} account yet - manage accounts on the Accounts tab.` : undefined}>
          <Select value={accountId} onChange={(e) => setAccountId(e.target.value)}>
            <option value="">No account</option>
            {relevantAccounts.map((a) => (
              <option key={a.id} value={a.id}>
                {a.name}
              </option>
            ))}
          </Select>
        </Field>

        <Field label="Place / who">
          <Input placeholder="e.g. Tesco, Ján..." value={place} onChange={(e) => setPlace(e.target.value)} />
        </Field>

        <Field label="Note">
          <Textarea rows={2} value={note} onChange={(e) => setNote(e.target.value)} />
        </Field>

        {error && <p className="text-xs text-red-600 dark:text-red-400">{error}</p>}
      </div>
      <ModalFooter>
        <Button variant="secondary" onClick={onClose} disabled={saving}>
          Cancel
        </Button>
        <Button variant="primary" onClick={submit} disabled={saving}>
          {saving ? <Spinner className="h-4 w-4" /> : null}
          {initial ? "Save changes" : "Add entry"}
        </Button>
      </ModalFooter>
    </Modal>
  );
}

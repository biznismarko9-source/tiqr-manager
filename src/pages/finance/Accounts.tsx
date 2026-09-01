import { useEffect, useState, type ReactNode } from "react";
import { api, errMsg } from "../../lib/api";
import type { Account, AccountInput, FinanceCategory, RecurringExpense, RecurringExpenseInput, TransferInput } from "../../lib/types";
import { centsToDecimalString, decimalStringToCents, formatDate, formatMoney, todayIso } from "../../lib/format";
import {
  Badge,
  Button,
  Card,
  CHECKBOX_CLASS,
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
import { IconLink, IconPencil, IconPlus, IconTrash, IconWallet } from "../../components/icons";
import { useToast } from "../../lib/toast";
import { CURRENCIES } from "../Orders";
import type { FinanceData } from "./shared";

// 2.1.0: marko's own "FINANCE 2.1" request - this tab is where the money
// INFRASTRUCTURE lives (as opposed to Transactions, which is the flat
// ledger of what happened): Accounts/Wallets and their balances (point 18:
// every balance below comes from list_accounts' own single aggregate query
// - nothing here re-sums anything client-side), the New Transfer action
// (point 6: transfer safety - same-account and cross-currency are both
// impossible to submit here, not just rejected server-side), and Recurring
// Expenses (point 8/9: Create/Skip/Pause/Resume, next_date never advances
// on its own). A transfer's own history is deliberately NOT re-listed here
// - it already lives in Transactions' unified ledger (marko's point 11) and
// showing it twice would just be two places that could drift apart.

const ACCOUNT_TYPE_OPTIONS: Account["accountType"][] = ["bank", "revolut", "paypal", "cash", "credit_card", "other"];
const ACCOUNT_TYPE_LABELS: Record<Account["accountType"], string> = {
  bank: "Bank",
  revolut: "Revolut",
  paypal: "PayPal",
  cash: "Cash",
  credit_card: "Credit card",
  other: "Other",
};

const FREQUENCY_OPTIONS: RecurringExpense["frequency"][] = ["weekly", "monthly", "quarterly", "yearly"];
const FREQUENCY_LABELS: Record<RecurringExpense["frequency"], string> = {
  weekly: "Weekly",
  monthly: "Monthly",
  quarterly: "Quarterly",
  yearly: "Yearly",
};

export default function Accounts({ accounts, categories, recurringExpenses, loading, reload }: FinanceData) {
  const toast = useToast();

  const [accountFormOpen, setAccountFormOpen] = useState(false);
  const [editingAccount, setEditingAccount] = useState<Account | null>(null);
  const [deleteAccountTarget, setDeleteAccountTarget] = useState<Account | null>(null);
  const [deletingAccount, setDeletingAccount] = useState(false);

  const [transferFormOpen, setTransferFormOpen] = useState(false);

  const [recurringFormOpen, setRecurringFormOpen] = useState(false);
  const [editingRecurring, setEditingRecurring] = useState<RecurringExpense | null>(null);
  const [deleteRecurringTarget, setDeleteRecurringTarget] = useState<RecurringExpense | null>(null);
  const [deletingRecurring, setDeletingRecurring] = useState(false);
  const [busyRecurringId, setBusyRecurringId] = useState<number | null>(null);

  const openNewAccount = () => {
    setEditingAccount(null);
    setAccountFormOpen(true);
  };
  const openEditAccount = (a: Account) => {
    setEditingAccount(a);
    setAccountFormOpen(true);
  };
  const openNewRecurring = () => {
    setEditingRecurring(null);
    setRecurringFormOpen(true);
  };
  const openEditRecurring = (r: RecurringExpense) => {
    setEditingRecurring(r);
    setRecurringFormOpen(true);
  };

  const doDeleteAccount = async () => {
    if (!deleteAccountTarget) return;
    setDeletingAccount(true);
    try {
      await api.deleteAccount(deleteAccountTarget.id);
      setDeleteAccountTarget(null);
      toast.success("Account deleted.");
      reload();
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setDeletingAccount(false);
    }
  };

  const doDeleteRecurring = async () => {
    if (!deleteRecurringTarget) return;
    setDeletingRecurring(true);
    try {
      await api.deleteRecurringExpense(deleteRecurringTarget.id);
      setDeleteRecurringTarget(null);
      toast.success("Recurring expense deleted.");
      reload();
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setDeletingRecurring(false);
    }
  };

  const doCreateFromRecurring = async (r: RecurringExpense) => {
    setBusyRecurringId(r.id);
    try {
      const result = await api.createFromRecurring(r.id);
      toast.success(`Logged ${formatMoney(result.entry.amountCents, result.entry.currency)} - next due ${formatDate(result.recurring.nextDate)}.`);
      reload();
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setBusyRecurringId(null);
    }
  };

  const doSkip = async (r: RecurringExpense) => {
    setBusyRecurringId(r.id);
    try {
      const updated = await api.skipRecurringExpense(r.id);
      toast.success(`Skipped - next due ${formatDate(updated.nextDate)}.`);
      reload();
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setBusyRecurringId(null);
    }
  };

  const doTogglePause = async (r: RecurringExpense) => {
    setBusyRecurringId(r.id);
    try {
      if (r.isActive) {
        await api.pauseRecurringExpense(r.id);
        toast.success("Paused.");
      } else {
        await api.resumeRecurringExpense(r.id);
        toast.success("Resumed.");
      }
      reload();
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setBusyRecurringId(null);
    }
  };

  return (
    <div>
      <div className="mb-4 flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="text-sm font-semibold text-slate-800 dark:text-slate-200">Accounts</h2>
          <p className="mt-0.5 text-xs text-slate-400 dark:text-slate-500">
            Your wallets - balances update automatically from income, expenses and transfers.
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          <Button variant="secondary" onClick={() => setTransferFormOpen(true)} disabled={accounts.length < 2} title={accounts.length < 2 ? "Add at least 2 accounts first" : undefined}>
            <IconLink className="h-4 w-4" /> New transfer
          </Button>
          <Button variant="primary" onClick={openNewAccount}>
            <IconPlus className="h-4 w-4" /> New account
          </Button>
        </div>
      </div>

      {loading ? (
        <LoadingBlock label="Loading accounts..." />
      ) : (
        <>
          {accounts.length === 0 ? (
            <EmptyState
              icon={<IconWallet className="h-8 w-8" />}
              title="No accounts yet"
              description="Add a bank, Revolut, PayPal, cash or other wallet to start tracking balances and transfers."
              action={
                <Button variant="primary" onClick={openNewAccount}>
                  <IconPlus className="h-4 w-4" /> Add your first account
                </Button>
              }
            />
          ) : (
            // 2.2.1: marko's own request - the old sm:grid-cols-2 lg:grid-cols-3
            // grid of full AccountCards ("zaberaju zbytocne vela miesta" - takes
            // up unnecessary space) is replaced with one compact divide-y list,
            // same dense-row language this codebase already uses for lookup
            // lists (PlatformList/EventCategoryList, Settings.tsx) and the
            // Recurring expenses table just below - every account's balance is
            // still the most prominent number on its row, just without a whole
            // card's padding/borders per account. A colored icon per account
            // TYPE (not per account) gives quick visual grouping at a glance
            // without adding a picker - see ACCOUNT_TYPE_COLORS below.
            <Card className="mb-8 overflow-hidden p-0">
              <ul className="divide-y divide-slate-100 dark:divide-slate-800">
                {accounts.map((a) => (
                  <AccountRow key={a.id} account={a} onEdit={openEditAccount} onDelete={setDeleteAccountTarget} />
                ))}
              </ul>
            </Card>
          )}

          <div className="mb-4 mt-8 flex flex-wrap items-start justify-between gap-3">
            <div>
              <h2 className="text-sm font-semibold text-slate-800 dark:text-slate-200">Upcoming recurring expenses</h2>
              <p className="mt-0.5 text-xs text-slate-400 dark:text-slate-500">
                Nothing posts automatically - Create logs today's occurrence as a real expense, Skip just moves on.
              </p>
            </div>
            <Button variant="primary" onClick={openNewRecurring}>
              <IconPlus className="h-4 w-4" /> New recurring expense
            </Button>
          </div>

          {recurringExpenses.length === 0 ? (
            <EmptyState title="No recurring expenses yet" description="e.g. rent, subscriptions, insurance - add one and it'll show up here when it's due." />
          ) : (
            <Card>
              <div className="overflow-x-auto">
                <table className="w-full border-collapse">
                  <thead className="border-b border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-800/60">
                    <tr>
                      <th className="th">Name</th>
                      <th className="th">Amount</th>
                      <th className="th">Frequency</th>
                      <th className="th">Category</th>
                      <th className="th">Account</th>
                      <th className="th">Next</th>
                      <th className="th" />
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
                    {recurringExpenses.map((r) => (
                      <RecurringRow
                        key={r.id}
                        item={r}
                        busy={busyRecurringId === r.id}
                        onCreate={doCreateFromRecurring}
                        onSkip={doSkip}
                        onTogglePause={doTogglePause}
                        onEdit={openEditRecurring}
                        onDelete={setDeleteRecurringTarget}
                      />
                    ))}
                  </tbody>
                </table>
              </div>
            </Card>
          )}
        </>
      )}

      <AccountFormModal open={accountFormOpen} onClose={() => setAccountFormOpen(false)} onSaved={reload} initial={editingAccount} />
      <TransferFormModal open={transferFormOpen} onClose={() => setTransferFormOpen(false)} onSaved={reload} accounts={accounts} />
      <RecurringFormModal
        open={recurringFormOpen}
        onClose={() => setRecurringFormOpen(false)}
        onSaved={reload}
        categories={categories}
        accounts={accounts}
        initial={editingRecurring}
      />

      <ConfirmDialog
        open={!!deleteAccountTarget}
        title="Delete this account?"
        message={
          deleteAccountTarget ? (
            <>
              Deletes <b>{deleteAccountTarget.name}</b>. Entries and recurring expenses that used it will show "No account" instead of being
              deleted. If a transfer still uses it, deletion will be blocked - delete that transfer first.
            </>
          ) : (
            ""
          )
        }
        confirmLabel="Delete account"
        danger
        busy={deletingAccount}
        onCancel={() => setDeleteAccountTarget(null)}
        onConfirm={doDeleteAccount}
      />

      <ConfirmDialog
        open={!!deleteRecurringTarget}
        title="Delete this recurring expense?"
        message={
          deleteRecurringTarget ? (
            <>
              Deletes the <b>{deleteRecurringTarget.name}</b> template. Entries it already created stay in Transactions - this only removes the
              schedule going forward.
            </>
          ) : (
            ""
          )
        }
        confirmLabel="Delete"
        danger
        busy={deletingRecurring}
        onCancel={() => setDeleteRecurringTarget(null)}
        onConfirm={doDeleteRecurring}
      />
    </div>
  );
}

// ---------------------------------------------------------------------------
// Account row
// ---------------------------------------------------------------------------

// 2.2.1: a per-TYPE color (not per-account - nothing to pick, same "assigned
// automatically" spirit as EventCategoryBadge/FinanceCategoryBadge) so a
// dense list of many accounts is still quick to scan by kind, the way the
// old card grid's icon color already was (always brand before this - one
// flat color regardless of type). Falls back to the "other" slate tone for
// any future account type this map hasn't been updated for.
const ACCOUNT_TYPE_COLORS: Record<Account["accountType"], string> = {
  bank: "bg-blue-50 text-blue-600 dark:bg-blue-500/10 dark:text-blue-400",
  revolut: "bg-violet-50 text-violet-600 dark:bg-violet-500/10 dark:text-violet-400",
  paypal: "bg-indigo-50 text-indigo-600 dark:bg-indigo-500/10 dark:text-indigo-400",
  cash: "bg-emerald-50 text-emerald-600 dark:bg-emerald-500/10 dark:text-emerald-400",
  credit_card: "bg-amber-50 text-amber-600 dark:bg-amber-500/10 dark:text-amber-400",
  other: "bg-slate-100 text-slate-500 dark:bg-slate-800 dark:text-slate-400",
};

function AccountRow({ account, onEdit, onDelete }: { account: Account; onEdit: (a: Account) => void; onDelete: (a: Account) => void }) {
  const negative = account.currentBalanceCents < 0;
  return (
    <li className={`flex items-center gap-3 px-4 py-2.5 hover:bg-slate-50 dark:hover:bg-slate-800/60 ${account.isActive ? "" : "opacity-60"}`}>
      <span className={`flex h-7 w-7 shrink-0 items-center justify-center rounded-full ${ACCOUNT_TYPE_COLORS[account.accountType]}`}>
        <IconWallet className="h-3.5 w-3.5" />
      </span>
      <div className="min-w-0 flex-1">
        <p className="truncate text-sm font-medium text-slate-800 dark:text-slate-200">
          {account.name}
          {!account.isActive && <span className="ml-1.5 text-xs font-normal text-slate-400 dark:text-slate-500">(inactive)</span>}
        </p>
        <p className="text-xs text-slate-400 dark:text-slate-500">
          {ACCOUNT_TYPE_LABELS[account.accountType]} &middot; {account.currency}
        </p>
      </div>
      <p
        className={`shrink-0 text-right text-sm font-semibold tabular-nums ${negative ? "text-red-600 dark:text-red-400" : "text-slate-900 dark:text-slate-100"}`}
        title={`Opening: ${formatMoney(account.openingBalanceCents, account.currency)}`}
      >
        {formatMoney(account.currentBalanceCents, account.currency)}
      </p>
      <div className="flex shrink-0 items-center gap-1">
        <button type="button" className="rounded p-1 text-slate-300 hover:text-brand-600 dark:text-slate-600 dark:hover:text-brand-400" title="Edit" onClick={() => onEdit(account)}>
          <IconPencil className="h-4 w-4" />
        </button>
        <button type="button" className="rounded p-1 text-slate-300 hover:text-red-600 dark:text-slate-600 dark:hover:text-red-400" title="Delete" onClick={() => onDelete(account)}>
          <IconTrash className="h-4 w-4" />
        </button>
      </div>
    </li>
  );
}

// ---------------------------------------------------------------------------
// Recurring expense row
// ---------------------------------------------------------------------------

function MiniButton({
  children,
  onClick,
  disabled,
  tone = "default",
}: {
  children: ReactNode;
  onClick: () => void;
  disabled?: boolean;
  tone?: "default" | "primary" | "warn" | "positive";
}) {
  const tones: Record<string, string> = {
    default: "border-slate-200 text-slate-600 hover:bg-slate-50 dark:border-slate-700 dark:text-slate-400 dark:hover:bg-slate-800",
    primary: "border-brand-200 text-brand-700 hover:bg-brand-50 dark:border-brand-500/30 dark:text-brand-400 dark:hover:bg-brand-500/10",
    warn: "border-amber-200 text-amber-700 hover:bg-amber-50 dark:border-amber-500/30 dark:text-amber-400 dark:hover:bg-amber-500/10",
    positive: "border-emerald-200 text-emerald-700 hover:bg-emerald-50 dark:border-emerald-500/30 dark:text-emerald-400 dark:hover:bg-emerald-500/10",
  };
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={onClick}
      className={`shrink-0 rounded-md border px-2 py-1 text-xs font-medium transition-colors disabled:opacity-40 disabled:pointer-events-none ${tones[tone]}`}
    >
      {children}
    </button>
  );
}

function RecurringRow({
  item,
  busy,
  onCreate,
  onSkip,
  onTogglePause,
  onEdit,
  onDelete,
}: {
  item: RecurringExpense;
  busy: boolean;
  onCreate: (r: RecurringExpense) => void;
  onSkip: (r: RecurringExpense) => void;
  onTogglePause: (r: RecurringExpense) => void;
  onEdit: (r: RecurringExpense) => void;
  onDelete: (r: RecurringExpense) => void;
}) {
  // "Overdue" is a purely client-side comparison (next_date < today) shown
  // only while active - matches finance_recurring.rs's own doc comment: a
  // paused template's next_date is frozen and only becomes actionable (and
  // worth flagging) again once resumed.
  const overdue = item.isActive && item.nextDate < todayIso();
  return (
    <tr className="hover:bg-slate-50 dark:hover:bg-slate-800/60">
      <td className="td">
        <span className="font-medium text-slate-800 dark:text-slate-200">{item.name}</span>
        {item.note && (
          <span className="block max-w-[200px] truncate text-xs text-slate-400 dark:text-slate-500" title={item.note}>
            {item.note}
          </span>
        )}
      </td>
      <td className="td tabular-nums text-slate-700 dark:text-slate-300">{formatMoney(item.amountCents, item.currency)}</td>
      <td className="td">
        {FREQUENCY_LABELS[item.frequency]}
        <span className="ml-1.5 text-xs text-slate-400 dark:text-slate-500">{item.scope === "business" ? "Business" : "Personal"}</span>
      </td>
      <td className="td">
        {item.categoryName && item.categoryColorSlot !== null ? (
          <FinanceCategoryBadge name={item.categoryName} colorSlot={item.categoryColorSlot} />
        ) : (
          <span className="text-slate-300 dark:text-slate-600">-</span>
        )}
      </td>
      <td className="td">{item.accountName ?? <span className="text-slate-300 dark:text-slate-600">-</span>}</td>
      <td className="td whitespace-nowrap">
        {!item.isActive ? (
          <Badge tone="soldout">Paused</Badge>
        ) : (
          <>
            <span className={overdue ? "font-medium text-red-600 dark:text-red-400" : "text-slate-700 dark:text-slate-300"}>{formatDate(item.nextDate)}</span>
            {overdue && (
              <span className="ml-1.5">
                <Badge tone="cancelled">Overdue</Badge>
              </span>
            )}
          </>
        )}
      </td>
      <td className="td">
        <div className="flex flex-wrap items-center justify-end gap-1.5">
          {item.isActive ? (
            <>
              <MiniButton tone="primary" disabled={busy} onClick={() => onCreate(item)}>
                Create
              </MiniButton>
              <MiniButton tone="default" disabled={busy} onClick={() => onSkip(item)}>
                Skip
              </MiniButton>
              <MiniButton tone="warn" disabled={busy} onClick={() => onTogglePause(item)}>
                Pause
              </MiniButton>
            </>
          ) : (
            <MiniButton tone="positive" disabled={busy} onClick={() => onTogglePause(item)}>
              Resume
            </MiniButton>
          )}
          <button type="button" className="rounded p-1 text-slate-300 hover:text-brand-600 dark:text-slate-600 dark:hover:text-brand-400" title="Edit" onClick={() => onEdit(item)}>
            <IconPencil className="h-4 w-4" />
          </button>
          <button type="button" className="rounded p-1 text-slate-300 hover:text-red-600 dark:text-slate-600 dark:hover:text-red-400" title="Delete" onClick={() => onDelete(item)}>
            <IconTrash className="h-4 w-4" />
          </button>
        </div>
      </td>
    </tr>
  );
}

// ---------------------------------------------------------------------------
// New/edit account modal
// ---------------------------------------------------------------------------

function AccountFormModal({ open, onClose, onSaved, initial }: { open: boolean; onClose: () => void; onSaved: () => void; initial: Account | null }) {
  const toast = useToast();
  const [name, setName] = useState("");
  const [accountType, setAccountType] = useState<Account["accountType"]>("bank");
  const [currency, setCurrency] = useState("EUR");
  const [openingBalance, setOpeningBalance] = useState("0");
  const [isActive, setIsActive] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    setName(initial?.name ?? "");
    setAccountType(initial?.accountType ?? "bank");
    setCurrency(initial?.currency ?? "EUR");
    setOpeningBalance(initial ? centsToDecimalString(initial.openingBalanceCents) : "0");
    setIsActive(initial?.isActive ?? true);
    setError(null);
  }, [open, initial]);

  const submit = async () => {
    if (!name.trim()) {
      setError("Enter an account name.");
      return;
    }
    const cents = decimalStringToCents(openingBalance);
    if (cents === null) {
      setError("Enter a valid opening balance (0 is fine).");
      return;
    }
    setSaving(true);
    setError(null);
    const input: AccountInput = { name: name.trim(), accountType, currency, openingBalanceCents: cents, isActive };
    try {
      if (initial) {
        await api.updateAccount(initial.id, input);
        toast.success("Account updated.");
      } else {
        await api.createAccount(input);
        toast.success("Account added.");
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
    <Modal open={open} onClose={onClose} title={initial ? "Edit account" : "New account"}>
      <div className="space-y-3">
        <Field label="Name" required>
          <Input placeholder="e.g. Revolut, Tatra banka, Cash..." value={name} onChange={(e) => setName(e.target.value)} />
        </Field>

        <div className="grid grid-cols-2 gap-2">
          <Field label="Type">
            <Select value={accountType} onChange={(e) => setAccountType(e.target.value as Account["accountType"])}>
              {ACCOUNT_TYPE_OPTIONS.map((t) => (
                <option key={t} value={t}>
                  {ACCOUNT_TYPE_LABELS[t]}
                </option>
              ))}
            </Select>
          </Field>
          {/* Locked once created: an account's balance is a running total in
              ONE currency (opening balance + its own entries/transfers) -
              changing currency after the fact wouldn't convert anything,
              just relabel a total that no longer means what it says. Same
              currency-integrity spirit as finance_entries::validate_account
              (an entry's currency must match its account), just enforced
              here instead since there's no server-side check against
              editing an account out from under its own history. */}
          <Field label="Currency" hint={initial ? "Can't change after creation - delete and recreate instead." : undefined}>
            <Select value={currency} onChange={(e) => setCurrency(e.target.value)} disabled={!!initial}>
              {(CURRENCIES.includes(currency) ? CURRENCIES : [currency, ...CURRENCIES]).map((c) => (
                <option key={c} value={c}>
                  {c}
                </option>
              ))}
            </Select>
          </Field>
        </div>

        <Field label="Opening balance" hint="How much is in this account right now, before anything you log here.">
          <Input inputMode="decimal" placeholder="0.00" value={openingBalance} onChange={(e) => setOpeningBalance(e.target.value)} />
        </Field>

        <label className="flex items-center gap-2 text-sm text-slate-700 dark:text-slate-300">
          <input type="checkbox" className={CHECKBOX_CLASS} checked={isActive} onChange={(e) => setIsActive(e.target.checked)} />
          Active
        </label>

        {error && <p className="text-xs text-red-600 dark:text-red-400">{error}</p>}
      </div>
      <ModalFooter>
        <Button variant="secondary" onClick={onClose} disabled={saving}>
          Cancel
        </Button>
        <Button variant="primary" onClick={submit} disabled={saving}>
          {saving ? <Spinner className="h-4 w-4" /> : null}
          {initial ? "Save changes" : "Add account"}
        </Button>
      </ModalFooter>
    </Modal>
  );
}

// ---------------------------------------------------------------------------
// New transfer modal
// ---------------------------------------------------------------------------

function TransferFormModal({ open, onClose, onSaved, accounts }: { open: boolean; onClose: () => void; onSaved: () => void; accounts: Account[] }) {
  const toast = useToast();
  const [transferDate, setTransferDate] = useState(todayIso());
  const [fromAccountId, setFromAccountId] = useState("");
  const [toAccountId, setToAccountId] = useState("");
  const [amount, setAmount] = useState("");
  const [note, setNote] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    setTransferDate(todayIso());
    setFromAccountId(accounts[0] ? String(accounts[0].id) : "");
    setToAccountId("");
    setAmount("");
    setNote("");
    setError(null);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  const fromAccount = accounts.find((a) => String(a.id) === fromAccountId) ?? null;
  // Only accounts sharing the From account's own currency are ever offered
  // as a To account - create_transfer_impl (finance_accounts.rs) rejects
  // cross-currency transfers outright (no invented exchange rate), so
  // filtering here means this form can never be submitted in a way the
  // backend would reject for that reason.
  const toOptions = accounts.filter((a) => a.id !== fromAccount?.id && a.currency === fromAccount?.currency);

  useEffect(() => {
    if (toAccountId && !toOptions.some((a) => String(a.id) === toAccountId)) {
      setToAccountId("");
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [fromAccountId]);

  const submit = async () => {
    if (!fromAccountId || !toAccountId) {
      setError("Pick both a From and a To account.");
      return;
    }
    const cents = decimalStringToCents(amount);
    if (cents === null || cents <= 0) {
      setError("Enter a valid amount greater than 0.");
      return;
    }
    if (!transferDate) {
      setError("Pick a date.");
      return;
    }
    setSaving(true);
    setError(null);
    const input: TransferInput = {
      transferDate,
      fromAccountId: Number(fromAccountId),
      toAccountId: Number(toAccountId),
      amountCents: cents,
      note: note.trim() || null,
    };
    try {
      await api.createTransfer(input);
      toast.success("Transfer recorded.");
      onSaved();
      onClose();
    } catch (e) {
      setError(errMsg(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal open={open} onClose={onClose} title="New transfer">
      <div className="space-y-3">
        <Field label="Date" required>
          <Input type="date" value={transferDate} onChange={(e) => setTransferDate(e.target.value)} />
        </Field>

        <Field label="From account" required>
          <Select value={fromAccountId} onChange={(e) => setFromAccountId(e.target.value)}>
            {accounts.map((a) => (
              <option key={a.id} value={a.id}>
                {a.name} ({a.currency})
              </option>
            ))}
          </Select>
        </Field>

        <Field label="To account" required hint={fromAccount && toOptions.length === 0 ? `No other ${fromAccount.currency} account yet - add one first.` : undefined}>
          <Select value={toAccountId} onChange={(e) => setToAccountId(e.target.value)} disabled={toOptions.length === 0}>
            <option value="">Select an account...</option>
            {toOptions.map((a) => (
              <option key={a.id} value={a.id}>
                {a.name}
              </option>
            ))}
          </Select>
        </Field>

        <Field label="Amount" required hint={fromAccount ? `In ${fromAccount.currency}, same as the From account.` : undefined}>
          <Input inputMode="decimal" placeholder="0.00" value={amount} onChange={(e) => setAmount(e.target.value)} />
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
        <Button variant="primary" onClick={submit} disabled={saving || toOptions.length === 0}>
          {saving ? <Spinner className="h-4 w-4" /> : null}
          Record transfer
        </Button>
      </ModalFooter>
    </Modal>
  );
}

// ---------------------------------------------------------------------------
// New/edit recurring expense modal
// ---------------------------------------------------------------------------

function RecurringFormModal({
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
  initial: RecurringExpense | null;
}) {
  const toast = useToast();
  const [name, setName] = useState("");
  const [amount, setAmount] = useState("");
  const [currency, setCurrency] = useState("EUR");
  const [scope, setScope] = useState<"personal" | "business">("personal");
  const [categoryId, setCategoryId] = useState("");
  const [accountId, setAccountId] = useState("");
  const [frequency, setFrequency] = useState<RecurringExpense["frequency"]>("monthly");
  const [startDate, setStartDate] = useState(todayIso());
  const [note, setNote] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    setName(initial?.name ?? "");
    setAmount(initial ? centsToDecimalString(initial.amountCents) : "");
    setCurrency(initial?.currency ?? "EUR");
    setScope(initial?.scope ?? "personal");
    setCategoryId(initial?.categoryId ? String(initial.categoryId) : "");
    setAccountId(initial?.accountId ? String(initial.accountId) : "");
    setFrequency(initial?.frequency ?? "monthly");
    setStartDate(initial?.startDate ?? todayIso());
    setNote(initial?.note ?? "");
    setError(null);
  }, [open, initial]);

  // Recurring expenses only ever create an "expense" FinanceEntry
  // (create_from_recurring_impl hardcodes entry_type: "expense") - so, same
  // filtering idea as EntryFormModal's own relevantCategories, only
  // expense/both categories are offered.
  const relevantCategories = categories.filter((c) => c.kind === "expense" || c.kind === "both");

  // An account picked before switching currency that no longer matches
  // (validate_account requires an entry's currency to match its linked
  // account's own currency) falls back to "No account" rather than silently
  // staying selected but guaranteed to fail on submit - same pattern as
  // Transactions.tsx's own EntryFormModal.
  const relevantAccounts = accounts.filter((a) => a.currency === currency.trim().toUpperCase());
  useEffect(() => {
    if (accountId && !relevantAccounts.some((a) => String(a.id) === accountId)) {
      setAccountId("");
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currency]);

  const submit = async () => {
    if (!name.trim()) {
      setError("Enter a name.");
      return;
    }
    const cents = decimalStringToCents(amount);
    if (cents === null || cents <= 0) {
      setError("Enter a valid amount greater than 0.");
      return;
    }
    if (!startDate) {
      setError("Pick a start date.");
      return;
    }
    setSaving(true);
    setError(null);
    const input: RecurringExpenseInput = {
      name: name.trim(),
      amountCents: cents,
      currency,
      scope,
      categoryId: categoryId ? Number(categoryId) : null,
      accountId: accountId ? Number(accountId) : null,
      frequency,
      startDate,
      note: note.trim() || null,
    };
    try {
      if (initial) {
        await api.updateRecurringExpense(initial.id, input);
        toast.success("Recurring expense updated.");
      } else {
        await api.createRecurringExpense(input);
        toast.success("Recurring expense added.");
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
    <Modal open={open} onClose={onClose} title={initial ? "Edit recurring expense" : "New recurring expense"}>
      <div className="space-y-3">
        <Field label="Name" required>
          <Input placeholder="e.g. Rent, Adobe, Insurance..." value={name} onChange={(e) => setName(e.target.value)} />
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

        <Field label="Frequency">
          <Select value={frequency} onChange={(e) => setFrequency(e.target.value as RecurringExpense["frequency"])}>
            {FREQUENCY_OPTIONS.map((f) => (
              <option key={f} value={f}>
                {FREQUENCY_LABELS[f]}
              </option>
            ))}
          </Select>
        </Field>

        <Field
          label="Start date"
          required
          hint={
            initial
              ? `Already scheduled up to ${formatDate(initial.nextDate)} - changing this won't move that.`
              : "The first occurrence - later ones follow the frequency above."
          }
        >
          <Input type="date" value={startDate} onChange={(e) => setStartDate(e.target.value)} />
        </Field>

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

        <Field label="Account" hint={relevantAccounts.length === 0 && accounts.length > 0 ? `No ${currency} account yet - manage accounts above.` : undefined}>
          <Select value={accountId} onChange={(e) => setAccountId(e.target.value)}>
            <option value="">No account</option>
            {relevantAccounts.map((a) => (
              <option key={a.id} value={a.id}>
                {a.name}
              </option>
            ))}
          </Select>
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
          {initial ? "Save changes" : "Add recurring expense"}
        </Button>
      </ModalFooter>
    </Modal>
  );
}

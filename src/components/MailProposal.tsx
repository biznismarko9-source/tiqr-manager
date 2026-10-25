// 2.113.0: what an email could become, shown so it can be argued with.
//
// marko: "AI nesmie rozhodovat naslepo - kazdy navrh musi mat zdrojove udaje,
// mieru istoty a kontrolu duplicit. Nejednoznacne pripady patria na manualne
// potvrdenie." So the card shows every figure WITH the words it was read from
// and how sure TIQR is of it, says plainly when something like it already
// exists, says what Automatic did or would do and why, and - for anything
// unclear - opens a short form to correct it before anything is created.
//
// The backend decides all of it (commands/mail.rs, `build_proposal`); this
// file only shows it and sends marko's corrections back.
import { useEffect, useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { api, errMsg } from "../lib/api";
import { centsToDecimalString, decimalStringToCents } from "../lib/format";
import { useToast } from "../lib/toast";
import type {
  EventWithStats,
  MailCreateOverrides,
  MailDuplicate,
  MailProposal,
  MailSaleCandidate,
  Platform,
} from "../lib/types";
import { IconAlertTriangle, IconCheck } from "./icons";
import { Button, CHECKBOX_CLASS, ConfirmDialog, Field, Input, Modal, ModalFooter, Select, Spinner } from "./ui";
import { quietly } from "../lib/diagnostics";

const CURRENCIES = ["EUR", "USD", "GBP", "CHF", "CZK", "PLN", "HUF", "SEK", "NOK", "DKK", "RON", "TRY", "BGN", "CAD", "AUD"];

const KIND_WORD: Record<MailProposal["kind"], string> = { order: "order", sale: "sale", finance: "finance entry" };

const CERTAINTY: Record<"high" | "medium" | "low", { label: string; dot: string; text: string }> = {
  high: { label: "Certain", dot: "bg-emerald-500", text: "text-emerald-700 dark:text-emerald-400" },
  medium: { label: "Check it", dot: "bg-amber-500", text: "text-amber-700 dark:text-amber-400" },
  low: { label: "Unsure", dot: "bg-red-500", text: "text-red-700 dark:text-red-400" },
};

function createLabel(p: MailProposal): string {
  return p.kind === "finance" ? "Record in Finance" : p.kind === "order" ? "Create order" : "Record sale";
}

function recordLink(kind: string, id: number): string {
  return kind === "order" ? `/orders/${id}` : kind === "sale" ? `/sales/${id}` : "/finance";
}

function Certainty({ level }: { level: "high" | "medium" | "low" }) {
  const c = CERTAINTY[level];
  return (
    <span className={`inline-flex items-center gap-1 text-[11px] font-medium ${c.text}`}>
      <span className={`h-1.5 w-1.5 rounded-full ${c.dot}`} aria-hidden="true" />
      {c.label}
    </span>
  );
}

function DuplicateList({ duplicates }: { duplicates: MailDuplicate[] }) {
  return (
    <div className="mt-3 rounded-lg border border-amber-300/70 bg-amber-50 px-3 py-2 dark:border-amber-500/30 dark:bg-amber-500/10">
      <p className="flex items-center gap-1.5 text-xs font-semibold text-amber-800 dark:text-amber-300">
        <IconAlertTriangle className="h-3.5 w-3.5" /> This may already be in TIQR
      </p>
      <ul className="mt-1 space-y-0.5">
        {duplicates.map((d) => (
          <li key={`${d.kind}-${d.id}`} className="text-xs text-amber-800 dark:text-amber-300">
            <Link to={recordLink(d.kind, d.id)} className="font-medium underline-offset-2 hover:underline">
              {d.label}
            </Link>{" "}
            - {d.reason}
          </li>
        ))}
      </ul>
    </div>
  );
}

/** The proposal card in the mail reader. `onChanged` gets the new proposal
 *  after a create, or null when the message was marked not relevant. */
export function MailProposalCard({
  messageId,
  proposal,
  onChanged,
}: {
  messageId: number;
  proposal: MailProposal;
  onChanged: (p: MailProposal | null) => void;
}) {
  const toast = useToast();
  const [busy, setBusy] = useState(false);
  const [reviewing, setReviewing] = useState(false);
  const [confirmAnyway, setConfirmAnyway] = useState(false);

  if (proposal.createdId) {
    return (
      <div className="mt-3 rounded-xl bg-surface-muted px-4 py-3">
        <p className="section-title">Already done</p>
        <p className="mt-1 flex items-center gap-1.5 text-sm text-slate-700 dark:text-slate-200">
          <IconCheck className="h-4 w-4 text-emerald-500" />
          Made{" "}
          <Link
            to={recordLink(proposal.createdKind ?? proposal.kind, proposal.createdId)}
            className="font-medium text-brand-600 hover:underline dark:text-brand-400"
          >
            {proposal.createdLabel ?? `${proposal.createdKind} #${proposal.createdId}`}
          </Link>
          {proposal.decision === "created" ? " automatically" : ""}.
        </p>
      </div>
    );
  }

  const create = async (overrides?: MailCreateOverrides) => {
    setBusy(true);
    try {
      const p = await api.createFromMail(messageId, overrides);
      toast.success(`${p.createdLabel ?? KIND_WORD[p.kind]} created`);
      onChanged(p);
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setBusy(false);
      setConfirmAnyway(false);
    }
  };

  const notRelevant = async () => {
    setBusy(true);
    try {
      await api.setMailReviewOutcome(messageId, "irrelevant");
      toast.info("Marked not relevant - TIQR will not propose it again.");
      onChanged(null);
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setBusy(false);
    }
  };

  const hasDuplicates = proposal.duplicates.length > 0;

  return (
    <div className="mt-3 rounded-xl bg-surface-muted px-4 py-3">
      <div className="flex flex-wrap items-center gap-2">
        <p className="section-title">TIQR can make this</p>
        <Certainty level={proposal.confidence} />
      </div>
      <p className="mt-1 text-sm font-medium text-slate-800 dark:text-slate-100">{proposal.summary}</p>

      {/* Every figure, the words it came from, and how sure. */}
      <dl className="mt-2 divide-y divide-slate-200/70 rounded-lg bg-surface px-3 dark:divide-slate-800">
        {proposal.evidence.map((e) => (
          <div key={e.field} className="grid grid-cols-[6.5rem_1fr_auto] items-baseline gap-2 py-1.5 text-xs">
            <dt className="text-slate-500 dark:text-slate-400">{e.field}</dt>
            <dd className="min-w-0">
              <span className="font-medium text-slate-800 dark:text-slate-100">{e.value}</span>
              <span className="mt-0.5 block truncate text-[11px] text-slate-500 dark:text-slate-400" title={e.source}>
                {e.source}
              </span>
            </dd>
            <Certainty level={e.confidence} />
          </div>
        ))}
      </dl>

      {hasDuplicates && <DuplicateList duplicates={proposal.duplicates} />}

      {proposal.blockers.length > 0 && (
        <ul className="mt-2 space-y-0.5">
          {proposal.blockers.map((b) => (
            <li key={b} className="text-xs text-amber-700 dark:text-amber-400">
              {b}
            </li>
          ))}
        </ul>
      )}

      {/* What Automatic did, or would do, and why. */}
      <p className="mt-2 text-[11px] text-slate-500 dark:text-slate-400">
        <span className="font-medium text-slate-600 dark:text-slate-300">Automatic: </span>
        {proposal.decision === "review" && proposal.decisionReason ? `left it for you - ${proposal.decisionReason}` : proposal.autoReason}
      </p>

      <div className="mt-3 flex flex-wrap items-center gap-2">
        {proposal.ready && !hasDuplicates && (
          <Button variant="primary" disabled={busy} onClick={() => create()}>
            {busy ? <Spinner className="h-4 w-4" /> : null}
            {createLabel(proposal)}
          </Button>
        )}
        {proposal.ready && hasDuplicates && (
          <Button variant="secondary" disabled={busy} onClick={() => setConfirmAnyway(true)}>
            Create anyway
          </Button>
        )}
        <Button variant={proposal.ready ? "secondary" : "primary"} disabled={busy} onClick={() => setReviewing(true)}>
          {proposal.ready ? "Review and edit" : "Complete and create"}
        </Button>
        <Button variant="ghost" disabled={busy} onClick={notRelevant}>
          Not relevant
        </Button>
      </div>

      <ConfirmDialog
        open={confirmAnyway}
        title={`Create this ${KIND_WORD[proposal.kind]} anyway?`}
        message="TIQR found records that look the same. Create it only if this really is a different one."
        confirmLabel="Create anyway"
        busy={busy}
        onConfirm={() => create({ allowDuplicate: true })}
        onCancel={() => setConfirmAnyway(false)}
      />
      {reviewing && (
        <MailCreateModal
          messageId={messageId}
          proposal={proposal}
          onClose={() => setReviewing(false)}
          onCreated={(p) => {
            setReviewing(false);
            onChanged(p);
          }}
        />
      )}
    </div>
  );
}

/** The short form for an unclear case: what was read, ready to correct. */
function MailCreateModal({
  messageId,
  proposal,
  onClose,
  onCreated,
}: {
  messageId: number;
  proposal: MailProposal;
  onClose: () => void;
  onCreated: (p: MailProposal) => void;
}) {
  const toast = useToast();
  const [events, setEvents] = useState<EventWithStats[]>([]);
  const [platforms, setPlatforms] = useState<Platform[]>([]);
  const [eventId, setEventId] = useState<number | "">(proposal.matchedEventId ?? "");
  const [quantity, setQuantity] = useState(proposal.quantity ? String(proposal.quantity) : "");
  const [amount, setAmount] = useState(proposal.amountCents != null ? centsToDecimalString(proposal.amountCents) : "");
  const [currency, setCurrency] = useState(proposal.currency ?? "EUR");
  const [date, setDate] = useState(proposal.date);
  const [platformId, setPlatformId] = useState<number | "">(proposal.platformId ?? "");
  const [reference, setReference] = useState(proposal.reference ?? "");
  const [entryType, setEntryType] = useState<"income" | "expense" | "">(proposal.entryType ?? "");
  const [candidates, setCandidates] = useState<MailSaleCandidate[] | null>(null);
  const [picked, setPicked] = useState<Set<number>>(new Set(proposal.ticketIds));
  const [anyway, setAnyway] = useState(false);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (proposal.kind !== "finance") api.listEvents().then(setEvents).catch(quietly("MailProposal: listEvents"));
    api.listPlatforms().then(setPlatforms).catch(quietly("MailProposal: listPlatforms"));
  }, [proposal.kind]);

  useEffect(() => {
    if (proposal.kind !== "sale" || eventId === "") {
      setCandidates(null);
      return;
    }
    setCandidates(null);
    api
      .listMailSaleCandidates(eventId)
      .then((c) => {
        setCandidates(c);
        // A new event: nothing of the old pick still applies.
        setPicked((prev) => new Set([...prev].filter((id) => c.some((t) => t.ticketId === id))));
      })
      .catch(() => setCandidates([]));
  }, [proposal.kind, eventId]);

  const cents = decimalStringToCents(amount);
  const qty = Number.parseInt(quantity, 10);
  const sortedEvents = useMemo(() => [...events].sort((a, b) => a.name.localeCompare(b.name)), [events]);
  const problems: string[] = [];
  if (cents == null || cents <= 0) problems.push("an amount");
  if (proposal.kind === "order" && eventId === "") problems.push("the event");
  if (proposal.kind === "order" && !(qty > 0)) problems.push("how many tickets");
  if (proposal.kind === "sale" && picked.size === 0) problems.push("the tickets that were sold");
  if (proposal.kind === "finance" && entryType === "") problems.push("money in or out");
  const needsConfirm = proposal.duplicates.length > 0;

  const submit = async () => {
    setSaving(true);
    try {
      const overrides: MailCreateOverrides = {
        amountCents: cents,
        currency,
        date,
        reference: reference.trim() || null,
        platformId: platformId === "" ? null : platformId,
        allowDuplicate: anyway,
      };
      if (proposal.kind !== "finance") overrides.eventId = eventId === "" ? null : eventId;
      if (proposal.kind === "order") overrides.quantity = qty;
      if (proposal.kind === "sale") overrides.ticketIds = [...picked];
      if (proposal.kind === "finance") overrides.entryType = entryType === "" ? null : entryType;
      const p = await api.createFromMail(messageId, overrides);
      toast.success(`${p.createdLabel ?? KIND_WORD[p.kind]} created`);
      onCreated(p);
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setSaving(false);
    }
  };

  const title = proposal.kind === "order" ? "Create an order" : proposal.kind === "sale" ? "Record a sale" : "Record in Finance";

  return (
    <Modal open onClose={onClose} title={title} width="max-w-xl">
      <p className="mb-4 text-xs text-slate-500 dark:text-slate-400">
        Filled in from the email. Correct anything that is wrong - nothing is created until you press the button.
      </p>
      <div className="grid gap-4 sm:grid-cols-2">
        {proposal.kind !== "finance" && (
          <div className="sm:col-span-2">
            <Field label="Event">
              <Select value={eventId} onChange={(e) => setEventId(e.target.value ? Number(e.target.value) : "")}>
                <option value="">Pick the event...</option>
                {sortedEvents.map((ev) => (
                  <option key={ev.id} value={ev.id}>
                    {ev.name}
                    {ev.eventDate ? ` · ${ev.eventDate.slice(0, 10)}` : ""}
                  </option>
                ))}
              </Select>
            </Field>
          </div>
        )}
        {proposal.kind === "finance" && (
          <div className="sm:col-span-2">
            <Field label="Direction" hint="A payout is money in. A refund is money in from where you buy, and out from where you sell.">
              <Select value={entryType} onChange={(e) => setEntryType(e.target.value as "income" | "expense" | "")}>
                <option value="">Choose...</option>
                <option value="income">Money in (income)</option>
                <option value="expense">Money out (expense)</option>
              </Select>
            </Field>
          </div>
        )}
        {proposal.kind === "order" && (
          <Field label="Tickets">
            <Input inputMode="numeric" value={quantity} onChange={(e) => setQuantity(e.target.value)} />
          </Field>
        )}
        <Field label={proposal.kind === "order" ? "Total paid (fees included)" : proposal.kind === "sale" ? "Sold for (total)" : "Amount"}>
          <Input inputMode="decimal" value={amount} onChange={(e) => setAmount(e.target.value)} />
        </Field>
        <Field label="Currency">
          <Select value={currency} onChange={(e) => setCurrency(e.target.value)}>
            {[...new Set([currency, ...CURRENCIES])].map((c) => (
              <option key={c} value={c}>
                {c}
              </option>
            ))}
          </Select>
        </Field>
        <Field label="Date">
          <Input type="date" value={date} onChange={(e) => setDate(e.target.value)} />
        </Field>
        {proposal.kind !== "finance" && (
          <Field label="Platform">
            <Select value={platformId} onChange={(e) => setPlatformId(e.target.value ? Number(e.target.value) : "")}>
              <option value="">None</option>
              {platforms
                .filter((p) => (proposal.kind === "order" ? p.kind !== "sale" : p.kind !== "purchase"))
                .map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.name}
                  </option>
                ))}
            </Select>
          </Field>
        )}
        <Field label="Reference" hint="Kept on the record, so a later email about it finds it.">
          <Input value={reference} onChange={(e) => setReference(e.target.value)} />
        </Field>
      </div>

      {proposal.kind === "sale" && (
        <div className="mt-4">
          <p className="label">Sold tickets</p>
          {eventId === "" ? (
            <p className="text-xs text-slate-500 dark:text-slate-400">Pick the event first.</p>
          ) : candidates === null ? (
            <div className="flex items-center gap-2 text-xs text-slate-500 dark:text-slate-400">
              <Spinner className="h-3.5 w-3.5" /> Loading unsold tickets...
            </div>
          ) : candidates.length === 0 ? (
            <p className="text-xs text-amber-700 dark:text-amber-400">This event has no unsold tickets in TIQR.</p>
          ) : (
            <ul className="max-h-48 space-y-1 overflow-y-auto rounded-lg bg-surface-muted p-2">
              {candidates.map((t) => (
                <li key={t.ticketId}>
                  <label className="flex items-center gap-2 text-xs text-slate-700 dark:text-slate-200">
                    <input
                      type="checkbox"
                      className={CHECKBOX_CLASS}
                      checked={picked.has(t.ticketId)}
                      onChange={(e) =>
                        setPicked((prev) => {
                          const next = new Set(prev);
                          if (e.target.checked) next.add(t.ticketId);
                          else next.delete(t.ticketId);
                          return next;
                        })
                      }
                    />
                    <span className="font-medium">{t.orderCode}</span>
                    <span className="text-slate-500 dark:text-slate-400">
                      {[t.section && `Sec ${t.section}`, t.rowLabel && `Row ${t.rowLabel}`, t.seat && `Seat ${t.seat}`]
                        .filter(Boolean)
                        .join(" · ") || "General admission"}
                    </span>
                  </label>
                </li>
              ))}
            </ul>
          )}
          {picked.size > 0 && cents != null && cents > 0 && (
            <p className="mt-1 text-[11px] text-slate-500 dark:text-slate-400">
              {picked.size} ticket{picked.size === 1 ? "" : "s"} · {centsToDecimalString(Math.floor(cents / picked.size))} {currency} each
            </p>
          )}
        </div>
      )}

      {needsConfirm && (
        <>
          <DuplicateList duplicates={proposal.duplicates} />
          <label className="mt-2 flex items-center gap-2 text-xs font-medium text-slate-700 dark:text-slate-200">
            <input type="checkbox" className={CHECKBOX_CLASS} checked={anyway} onChange={(e) => setAnyway(e.target.checked)} />
            I checked - this is a different one, create it anyway
          </label>
        </>
      )}

      {problems.length > 0 && <p className="mt-3 text-xs text-amber-700 dark:text-amber-400">Still needed: {problems.join(", ")}.</p>}
      <ModalFooter>
        <Button variant="ghost" onClick={onClose}>
          Cancel
        </Button>
        <Button variant="primary" disabled={saving || problems.length > 0 || (needsConfirm && !anyway)} onClick={submit}>
          {saving ? <Spinner className="h-4 w-4" /> : null}
          {createLabel(proposal)}
        </Button>
      </ModalFooter>
    </Modal>
  );
}

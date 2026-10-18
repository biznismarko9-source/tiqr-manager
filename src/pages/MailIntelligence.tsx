// Mail (2.105.0) - closer to Gmail, on purpose.
//
// marko asked for the look and the behaviour he already knows:
//   "1:1 prekopirovali ako vyzera gmail aj funkcnosti"
//   "vsetky starred maily ... je jedno kolko ich je a ake su stare"
//   "aby tam tiez boli zalozky" / "nechat zalozky ktore mam v gmaily"
//   "vediet ho rozkliknut a kuknut odkial ide z akeho mailu"
//   "policko kde budu chodit vyhradne len OTP kody"
//   "vediet rozlisit tiez promotions, spam kos atd"
//
// So: three panes - folders, list, message - and the folder rail is built
// from GMAIL'S OWN labels rather than categories invented here. Spam and
// Trash are browsable, and deliberately never trusted (see
// commands/mail.rs::UNTRUSTED_LABELS): a spam message claiming a sale can
// never reach Important.
//
// Two things this still is not, said in the UI rather than left to look
// like a bug: read state is TIQR's own, because the scope is read-only and
// cannot write a Gmail label; and nothing here creates a sale, an order or
// a finance entry.
import { useCallback, useEffect, useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { api, errMsg } from "../lib/api";
import {
  type MailConnectionStatus,
  type MailLabel,
  type MailMessageFilter,
  type MailMessageSummary,
  type MailProposal,
  type MailReviewOutcome,
} from "../lib/types";
import { Badge, Button, Card, EmptyState, Input, PageHeader, Spinner } from "../components/ui";
import { IconMail } from "../components/icons";
import { MailSettingsPane } from "../components/MailSettingsPane";
import { useToast } from "../lib/toast";

/** Folders this app defines itself, above Gmail's own. */
type Builtin =
  | "important"
  | "otp"
  | "starred"
  | "inbox"
  | "review"
  | "promotions"
  | "social"
  | "updates"
  | "forums"
  | "spam"
  | "trash";

const BUILTIN_FOLDERS: { key: Builtin; label: string }[] = [
  { key: "important", label: "Important" },
  { key: "otp", label: "OTP codes" },
  { key: "starred", label: "Starred" },
  { key: "inbox", label: "Inbox" },
  { key: "review", label: "Needs review" },
  { key: "promotions", label: "Promotions" },
  { key: "social", label: "Social" },
  { key: "updates", label: "Updates" },
  { key: "forums", label: "Forums" },
  { key: "spam", label: "Spam" },
  { key: "trash", label: "Trash" },
];

/* Not a folder: the rail's Settings row reuses `folder` as the selected
   pane, and this key can never collide with a Gmail label id - those are
   `INBOX`, `STARRED` or `Label_8`, never a double underscore. */
const SETTINGS_FOLDER = "__settings";

const CATEGORY_LABELS: Record<string, string> = {
  sale_confirmation: "Sale",
  purchase_confirmation: "Purchase",
  payout: "Payout",
  refund: "Refund",
  cancellation: "Cancellation",
  event_update: "Event update",
  order_status: "Order status",
  account_notice: "Account notice",
  other: "Other",
};

const categoryLabel = (c: string) => CATEGORY_LABELS[c] ?? c;
const labelsOf = (m: MailMessageSummary) => m.labels.split(",");
const has = (m: MailMessageSummary, l: string) => labelsOf(m).includes(l);
const isJunk = (m: MailMessageSummary) => has(m, "spam") || has(m, "trash");

function formatWhen(iso: string) {
  try {
    const d = new Date(iso);
    return d.toDateString() === new Date().toDateString()
      ? d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })
      : d.toLocaleDateString();
  } catch {
    return iso;
  }
}

/** Which messages a folder holds. One place, so the rail's counts and the
 *  list can never disagree. */
function inFolder(m: MailMessageSummary, folder: string): boolean {
  // TIQR's own trash takes a message out of every other folder - the whole
  // point of a trash is that things leave where they were. Gmail's own
  // `trash` label lands in the same folder, so there is one Trash here and
  // not two.
  if (m.trashedAt) return folder === "trash";
  switch (folder) {
    case "important":
      return m.category !== "other" && !isJunk(m);
    case "otp":
      return !!m.otpCode;
    case "starred":
      return m.isStarred;
    case "review":
      return m.status === "review_required";
    case "inbox":
      return (
        has(m, "inbox") &&
        !isJunk(m) &&
        !has(m, "category_promotions") &&
        !has(m, "category_social") &&
        !has(m, "category_updates") &&
        !has(m, "category_forums")
      );
    case "promotions":
      return has(m, "category_promotions");
    case "social":
      return has(m, "category_social");
    case "updates":
      return has(m, "category_updates");
    case "forums":
      return has(m, "category_forums");
    case "spam":
      return has(m, "spam");
    case "trash":
      return has(m, "trash");  // the locally-trashed are caught above
    default:
      // One of marko's own Gmail labels, matched by its id.
      return has(m, folder);
  }
}

function FolderRow({
  label,
  count,
  unread,
  active,
  onClick,
}: {
  label: string;
  count: number;
  unread: number;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`flex w-full items-center gap-2 rounded-lg px-2.5 py-1.5 text-left text-[12.5px] transition ${
        active
          ? "bg-brand-600/15 font-semibold text-brand-700 dark:text-brand-300"
          : "text-slate-600 hover:bg-surface-muted dark:text-slate-300"
      }`}
    >
      <span className="min-w-0 flex-1 truncate">{label}</span>
      {unread > 0 ? (
        <span className="num shrink-0 text-[11px] font-semibold text-brand-600 dark:text-brand-400">{unread}</span>
      ) : count > 0 ? (
        <span className="num shrink-0 text-[11px] text-slate-400 dark:text-slate-500">{count}</span>
      ) : null}
    </button>
  );
}

function MailRow({
  m,
  active,
  checked,
  onClick,
  onToggle,
}: {
  m: MailMessageSummary;
  active: boolean;
  checked: boolean;
  onClick: () => void;
  onToggle: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`w-full border-b border-slate-200 px-3 py-2.5 text-left transition dark:border-slate-800 ${
        active ? "bg-brand-600/10" : "hover:bg-surface-muted"
      }`}
    >
      <div className="flex items-baseline gap-2">
        {/* The checkbox sits inside the row button, so a stray click
            selects rather than doing nothing - but it stops the event, so
            picking a message and selecting it stay two separate acts. */}
        <input
          type="checkbox"
          className="mt-0.5 h-3.5 w-3.5 shrink-0 cursor-pointer"
          checked={checked}
          onClick={(e) => e.stopPropagation()}
          onChange={(e) => {
            e.stopPropagation();
            onToggle();
          }}
          aria-label="Select"
        />
        {m.isUnread && <span className="h-1.5 w-1.5 shrink-0 rounded-full bg-brand-500" aria-label="Unread" />}
        {m.isStarred && <span className="shrink-0 text-[11px] text-amber-500">&#9733;</span>}
        <span
          className={`min-w-0 flex-1 truncate text-[13px] ${
            m.isUnread ? "font-semibold text-slate-900 dark:text-slate-100" : "text-slate-600 dark:text-slate-300"
          }`}
        >
          {m.fromName ?? m.fromAddress}
        </span>
        <span className="shrink-0 whitespace-nowrap text-[11px] text-slate-400 dark:text-slate-500">
          {formatWhen(m.receivedAt)}
        </span>
      </div>
      <p
        className={`mt-0.5 truncate text-[13px] ${
          m.isUnread ? "text-slate-800 dark:text-slate-200" : "text-slate-500 dark:text-slate-400"
        }`}
      >
        {m.subject || "(no subject)"}
      </p>
      {/* The code, right in the list. A one-time code is useless slowly -
          by the time you have opened the mail and gone back, half the
          window is gone. */}
      {m.otpCode ? (
        <p className="num mt-1 text-base font-bold tracking-[0.18em] text-brand-600 dark:text-brand-400">
          {m.otpCode}
        </p>
      ) : (
        m.snippet && <p className="mt-0.5 truncate text-[11px] text-slate-400 dark:text-slate-500">{m.snippet}</p>
      )}
      {m.category !== "other" && !isJunk(m) && (
        <div className="mt-1">
          <Badge tone="sold">{categoryLabel(m.category)}</Badge>
        </div>
      )}
    </button>
  );
}

function Reader({ message, onChanged }: { message: MailMessageSummary; onChanged: () => void }) {
  const toast = useToast();
  const [body, setBody] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [showHeaders, setShowHeaders] = useState(false);
  const [proposal, setProposal] = useState<MailProposal | null>(null);
  const [creating, setCreating] = useState(false);

  useEffect(() => {
    setBody(null);
    setLoading(true);
    setShowHeaders(false);
    setProposal(null);
    api
      .getMailProposal(message.id)
      .then(setProposal)
      .catch(() => setProposal(null));
    api
      .getMailBody(message.id)
      .then((b) => {
        setBody(b);
        onChanged();
      })
      .catch((e) => {
        toast.error(errMsg(e));
        setBody("");
      })
      .finally(() => setLoading(false));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [message.id]);

  const junk = isJunk(message);

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="border-b border-slate-200 px-5 py-4 dark:border-slate-800">
        <h2 className="text-base font-semibold text-slate-900 dark:text-slate-100">
          {message.subject || "(no subject)"}
        </h2>

        {/* "odkial ide, z akeho mailu" - the real address first, the
            display name second, because a display name is whatever the
            sender typed and the address is not. */}
        <div className="mt-1.5 flex flex-wrap items-baseline gap-x-2 gap-y-1">
          {message.fromName && (
            <span className="text-sm text-slate-700 dark:text-slate-200">{message.fromName}</span>
          )}
          <span className="break-all font-mono text-xs text-slate-500 dark:text-slate-400">
            &lt;{message.fromAddress}&gt;
          </span>
          <button
            type="button"
            onClick={() => setShowHeaders((v) => !v)}
            className="text-[11px] text-brand-600 hover:underline dark:text-brand-400"
          >
            {showHeaders ? "Hide details" : "Details"}
          </button>
        </div>

        {showHeaders && (
          <dl className="mt-2 space-y-1 rounded-lg bg-surface-muted px-3 py-2 text-[11px]">
            {[
              ["From", message.fromRaw ?? message.fromAddress],
              ["To", message.toAddress],
              ["Reply-To", message.replyTo],
              ["Gmail labels", message.labels || null],
            ].map(([k, v]) =>
              v ? (
                <div key={k} className="flex gap-2">
                  <dt className="w-24 shrink-0 text-slate-500 dark:text-slate-400">{k}</dt>
                  <dd className="min-w-0 break-all font-mono text-slate-600 dark:text-slate-300">{v}</dd>
                </div>
              ) : null,
            )}
            {/* A reply address that disagrees with the sender is the single
                most useful thing on this panel, so it is pointed at. */}
            {message.replyTo && !message.replyTo.includes(message.fromAddress) && (
              <p className="pt-1 text-amber-600 dark:text-amber-400">
                A reply would go somewhere other than the sender.
              </p>
            )}
          </dl>
        )}

        <div className="mt-2 flex flex-wrap items-center gap-2">
          <span className="text-[11px] text-slate-400 dark:text-slate-500">
            {new Date(message.receivedAt).toLocaleString()}
          </span>
          {message.isStarred && <span className="text-[11px] text-amber-500">&#9733; Starred</span>}
          {message.category !== "other" && !junk && <Badge tone="sold">{categoryLabel(message.category)}</Badge>}
          {junk && <Badge tone="cancelled">{has(message, "spam") ? "spam" : "trash"}</Badge>}
        </div>

        {message.otpCode && (
          <div className="mt-3 rounded-xl bg-surface-muted px-4 py-3">
            <p className="section-title">One-time code</p>
            <p className="num mt-1 text-2xl font-bold tracking-[0.3em] text-brand-600 dark:text-brand-400">
              {message.otpCode}
            </p>
          </div>
        )}

        {/* What the message SAYS, read out of it. Shown and not acted on:
            creating an order or a sale from these is the next release, and
            marko gets to see whether the numbers are right first. */}
        {(message.extractedAmountCents != null ||
          message.extractedQuantity != null ||
          message.extractedReference) && !junk && (
          <div className="mt-3 rounded-xl bg-surface-muted px-4 py-3">
            <p className="section-title">What TIQR read</p>
            <dl className="mt-1.5 flex flex-wrap gap-x-6 gap-y-1 text-xs">
              {message.extractedAmountCents != null && (
                <div>
                  <dt className="text-slate-500 dark:text-slate-400">
                    Total{message.extractedBasis ? ` (${message.extractedBasis})` : ""}
                  </dt>
                  <dd className="num text-sm text-slate-800 dark:text-slate-200">
                    {(message.extractedAmountCents / 100).toFixed(2)} {message.extractedCurrency}
                  </dd>
                </div>
              )}
              {/* Per ticket is the TOTAL divided - fees already in it -
                  never a per-ticket price quoted in the mail, because that
                  one leaves the fees out and is not what a ticket cost. */}
              {message.extractedUnitCents != null && (
                <div>
                  <dt className="text-slate-500 dark:text-slate-400">Per ticket (incl. fees)</dt>
                  <dd className="num text-sm text-slate-800 dark:text-slate-200">
                    {(message.extractedUnitCents / 100).toFixed(2)} {message.extractedCurrency}
                  </dd>
                </div>
              )}
              {message.extractedQuantity != null && (
                <div>
                  <dt className="text-slate-500 dark:text-slate-400">Tickets</dt>
                  <dd className="num text-sm text-slate-800 dark:text-slate-200">
                    {message.extractedQuantity}
                  </dd>
                </div>
              )}
              {message.extractedReference && (
                <div>
                  <dt className="text-slate-500 dark:text-slate-400">Reference</dt>
                  <dd className="font-mono text-sm text-slate-800 dark:text-slate-200">
                    {message.extractedReference}
                  </dd>
                </div>
              )}
            </dl>
            <p className="mt-2 text-[11px] text-slate-400 dark:text-slate-500">
              Read only - TIQR does not create records from email yet.
            </p>
          </div>
        )}

        {/* WHAT THIS COULD BECOME.
            Nothing is guessed into existence: an order needs an event and
            a sale needs an order, and those are MATCHED against what is
            already in TIQR. When nothing matches, the card says so in
            words rather than offering a button that would invent one. */}
        {proposal && (
          <div className="mt-3 rounded-xl bg-surface-muted px-4 py-3">
            {proposal.createdId ? (
              <>
                <p className="section-title">Already done</p>
                <p className="mt-1 text-sm text-slate-700 dark:text-slate-200">
                  This created {proposal.createdKind} #{proposal.createdId}.
                </p>
              </>
            ) : (
              <>
                <p className="section-title">TIQR can make this</p>
                <p className="mt-1 text-sm text-slate-800 dark:text-slate-200">{proposal.summary}</p>
                {proposal.matchedEventName && (
                  <p className="mt-0.5 text-xs text-slate-500 dark:text-slate-400">
                    Event: {proposal.matchedEventName}
                  </p>
                )}
                {proposal.matchedOrderCode && (
                  <p className="mt-0.5 text-xs text-slate-500 dark:text-slate-400">
                    Order: {proposal.matchedOrderCode}
                  </p>
                )}
                {proposal.ready ? (
                  <div className="mt-2">
                    <Button
                      variant="primary"
                      disabled={creating}
                      onClick={async () => {
                        setCreating(true);
                        try {
                          const p = await api.createFromMail(message.id);
                          setProposal(p);
                          toast.success(`${p.createdKind} #${p.createdId} created`);
                          onChanged();
                        } catch (e) {
                          toast.error(errMsg(e));
                        } finally {
                          setCreating(false);
                        }
                      }}
                    >
                      {creating ? <Spinner className="h-4 w-4" /> : null}
                      {proposal.kind === "finance"
                        ? "Record in Finance"
                        : proposal.kind === "order"
                          ? "Create order"
                          : "Record sale"}
                    </Button>
                  </div>
                ) : (
                  <ul className="mt-2 space-y-0.5">
                    {proposal.blockers.map((b) => (
                      <li key={b} className="text-xs text-amber-600 dark:text-amber-400">
                        {b}
                      </li>
                    ))}
                  </ul>
                )}
              </>
            )}
          </div>
        )}

        {junk && (
          <p className="mt-3 rounded-lg bg-surface-muted px-3 py-2 text-[11px] text-amber-600 dark:text-amber-400">
            This is in {has(message, "spam") ? "Spam" : "Trash"}. TIQR never classifies these and will never act
            on one, however convincing it looks.
          </p>
        )}
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto px-5 py-4">
        {loading ? (
          <div className="flex items-center gap-2 text-xs text-slate-500 dark:text-slate-400">
            <Spinner className="h-4 w-4" /> Loading from Gmail...
          </div>
        ) : body ? (
          /* PLAIN TEXT, always. The HTML was stripped in Rust before this
             reached the frontend, and there is deliberately no path here
             that renders a message body as markup. */
          <p className="whitespace-pre-wrap break-words text-sm leading-relaxed text-slate-700 dark:text-slate-300">
            {body}
          </p>
        ) : (
          <p className="text-xs text-slate-500 dark:text-slate-400">
            Nothing to show - this message has no readable text.
          </p>
        )}
      </div>

      <div className="flex flex-wrap items-center gap-2 border-t border-slate-200 px-5 py-3 dark:border-slate-800">
        <Button
          variant="secondary"
          disabled={busy}
          onClick={async () => {
            setBusy(true);
            try {
              await api.setMailReviewOutcome(message.id, "reviewed" as MailReviewOutcome);
              onChanged();
            } catch (e) {
              toast.error(errMsg(e));
            } finally {
              setBusy(false);
            }
          }}
        >
          Mark reviewed
        </Button>
        <Button
          variant="ghost"
          disabled={busy}
          onClick={async () => {
            try {
              await api.setMailRead(message.id, message.isUnread);
              onChanged();
            } catch (e) {
              toast.error(errMsg(e));
            }
          }}
        >
          {message.isUnread ? "Mark read" : "Mark unread"}
        </Button>
        <span className="text-[11px] text-slate-400 dark:text-slate-500">
          Read state is TIQR&apos;s own - Gmail is never changed.
        </span>
      </div>
    </div>
  );
}

export default function MailIntelligence() {
  const toast = useToast();
  const [folder, setFolder] = useState<string>("important");
  const [status, setStatus] = useState<MailConnectionStatus | null>(null);
  const [labels, setLabels] = useState<MailLabel[]>([]);
  const [rows, setRows] = useState<MailMessageSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [listLoading, setListLoading] = useState(false);
  const [syncing, setSyncing] = useState(false);
  const [openId, setOpenId] = useState<number | null>(null);
  const [search, setSearch] = useState("");
  const [autoBusy, setAutoBusy] = useState(false);
  const [syncProgress, setSyncProgress] = useState(0);
  const [tick, setTick] = useState(0);
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [confirmDelete, setConfirmDelete] = useState(false);

  const filter = useMemo<MailMessageFilter>(
    () => ({ search: search.trim() || null, limit: 2000 }),
    [search],
  );

  const visible = useMemo(() => rows.filter((m) => inFolder(m, folder)), [rows, folder]);
  const open = useMemo(() => visible.find((m) => m.id === openId) ?? null, [visible, openId]);

  /* Counts come from the same `inFolder` the list uses, so the rail can
     never show a number the folder does not contain. */
  const countOf = useCallback(
    (key: string) => {
      const inIt = rows.filter((m) => inFolder(m, key));
      return { total: inIt.length, unread: inIt.filter((m) => m.isUnread).length };
    },
    [rows],
  );

  const loadHeader = useCallback(async () => {
    const [s, l] = await Promise.all([api.getMailStatus(), api.listMailLabels()]);
    setStatus(s);
    setLabels(l);
  }, []);

  const loadList = useCallback(async () => {
    setListLoading(true);
    try {
      setRows(await api.listMailMessages(filter));
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setListLoading(false);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [filter]);

  useEffect(() => {
    api
      .clearStaleMailSync()
      .then(() => loadHeader())
      .catch((e) => toast.error(errMsg(e)))
      .finally(() => setLoading(false));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    void loadList();
  }, [loadList]);

  const refresh = useCallback(() => {
    void loadHeader();
    void loadList();
  }, [loadHeader, loadList]);

  const clearSelection = useCallback(() => setSelected(new Set()), []);

  const bulk = async (what: "trash" | "restore" | "delete" | "read" | "unread") => {
    const ids = [...selected];
    if (ids.length === 0) return;
    try {
      const n =
        what === "trash"
          ? await api.trashMail(ids)
          : what === "restore"
            ? await api.restoreMail(ids)
            : what === "delete"
              ? await api.deleteMailForever(ids)
              : await api.setMailReadBulk(ids, what === "read");
      toast.success(`${n} message${n === 1 ? "" : "s"} updated`);
      clearSelection();
      setConfirmDelete(false);
      setOpenId(null);
      refresh();
    } catch (e) {
      toast.error(errMsg(e));
    }
  };

  /* 2.107.0: the screen keeps itself current.
     marko: "nemusim vzdy manualne davat sync ale ze v realnom case by sa
     tam zobrazovali maily".

     This is POLLING and the honest limit is the interval - Gmail's real
     push needs a public endpoint and TIQR has no server. While this screen
     is open it runs on marko's configured interval and refreshes the list
     after each run, so new mail arrives on its own.

     setTimeout and not setInterval: a sync that takes longer than the
     interval must not stack another one behind it. The backend refuses an
     overlapping run anyway, but a queue of refused calls is still noise. */
  useEffect(() => {
    if (!status?.connected || status.paused) return;
    const seconds = status.syncIntervalSeconds;
    if (seconds <= 0) return;

    let stopped = false;
    const timer = setTimeout(async () => {
      if (stopped) return;
      setAutoBusy(true);
      try {
        // The automatic poll chains too, but only a few batches at a time
        // - it is meant to keep up, not to run a backfill in the
        // background while marko is reading.
        for (let batch = 0; batch < 3; batch++) {
          const r = await api.syncMailNow();
          if (r.status !== "synced") break;
          // Only reload when something actually arrived - a refresh that
          // changes nothing still re-renders the list under his cursor.
          if (r.storedNew > 0) await loadList();
          if (!r.moreWaiting) break;
        }
        await loadHeader();
      } catch {
        // Silent on purpose: an automatic run must never throw a toast at
        // someone who did not ask for it. A real failure is already on the
        // status strip.
      } finally {
        if (!stopped) {
          setAutoBusy(false);
          setTick((t) => t + 1);
        }
      }
    }, seconds * 1000);

    return () => {
      stopped = true;
      clearTimeout(timer);
    };
    // `tick` is what re-arms the timer after each run.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tick, status?.connected, status?.paused, status?.syncIntervalSeconds]);

  /* One batch is about six seconds of work, because Google's quota caps
     this at roughly 50 messages a second however it is written. So a long
     first sync is CHAINED: each batch stores what it got, the list
     refreshes, and the next batch starts straight away. Same total time,
     but mail arrives in visible waves instead of after one long silence.

     Bounded, so a mailbox that keeps reporting more can never spin here
     forever - it stops and the ordinary poll picks the rest up. */
  const doSync = async () => {
    setSyncing(true);
    let fetched = 0;
    try {
      for (let batch = 0; batch < 40; batch++) {
        const result = await api.syncMailNow();
        if (result.status === "skipped") {
          if (batch === 0) toast.info(result.message);
          break;
        }
        if (result.status === "failed") {
          toast.error(result.message);
          break;
        }
        fetched += result.storedNew;
        setSyncProgress(fetched);
        refresh();
        if (!result.moreWaiting) break;
        // A breath between batches: it keeps the UI responsive and leaves
        // Google a little headroom under the per-second quota.
        await new Promise((r) => setTimeout(r, 250));
      }
      if (fetched > 0) toast.success(`${fetched} new message${fetched === 1 ? "" : "s"}`);
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setSyncing(false);
      setSyncProgress(0);
    }
  };

  if (loading) {
    return (
      <div>
        <PageHeader title="Mail" subtitle="Your inbox, with what matters pulled to the front." />
        <Card className="p-5">
          <div className="flex items-center gap-2 text-sm text-slate-500 dark:text-slate-400">
            <Spinner className="h-4 w-4" /> Loading...
          </div>
        </Card>
      </div>
    );
  }

  if (!status?.connected) {
    return (
      <div>
        <PageHeader title="Mail" subtitle="Your inbox, with what matters pulled to the front." />
        <EmptyState
          icon={<IconMail className="h-5 w-5" />}
          title="No mailbox connected"
          description="Connect a Gmail account to read your inbox here and let TIQR recognise sale, payout and refund mail."
          action={
            <Link to="/settings/integrations">
              <Button variant="primary">Connect in Settings</Button>
            </Link>
          }
        />
      </div>
    );
  }

  /* EVERY one of his own labels, including the empty ones.
     marko: "nech to cita vsetky zalozky". The old `l.total > 0` hid any
     label whose mail had not been fetched yet - which during a first sync
     is most of them, so the rail looked like Gmail with folders missing
     and there was no way to tell that from a label that is genuinely
     empty. An empty folder is information; a folder that is not there is
     a bug report waiting to happen. */
  const userLabels = labels.filter((l) => l.kind !== "system");

  return (
    /* NOT `h-full`: Layout gives pages no height (PROTECTED_AREAS 2.55.0),
       so the three-pane block below pins its own, the way Notes does. */
    <div>
      <PageHeader
        title="Mail"
        subtitle={
          status.accountEmail
            ? `${status.accountEmail}${status.unreadCount > 0 ? ` · ${status.unreadCount} unread` : ""}`
            : "Your inbox."
        }

        actions={
          <>
            {/* Says out loud that it is keeping itself current, so marko
                does not sit there wondering whether to press Sync.
                `PageHeader` has title / subtitle / actions and nothing
                else - checked, not assumed - so this lives here. */}
            {status.syncIntervalSeconds > 0 && (
              <span className="text-[11px] text-slate-400 dark:text-slate-500">
                {syncing && syncProgress > 0
                  ? `Fetching... ${syncProgress} so far`
                  : autoBusy || syncing
                  ? "Checking for new mail..."
                  : `Checks every ${
                      status.syncIntervalSeconds < 60
                        ? `${status.syncIntervalSeconds}s`
                        : `${Math.round(status.syncIntervalSeconds / 60)} min`
                    }`}
              </span>
            )}
            <div className="w-56">
              <Input placeholder="Search mail" value={search} onChange={(e) => setSearch(e.target.value)} />
            </div>
            <Button variant="secondary" disabled={syncing || status.paused} onClick={() => void doSync()}>
              {syncing ? <Spinner className="h-4 w-4" /> : null}
              Sync
            </Button>
          </>
        }
      />

      {status.lastError && (
        <Card className="mb-4 p-3">
          <p className="text-xs text-red-600 dark:text-red-400">{status.lastError}</p>
        </Card>
      )}

      <div className="flex gap-4" style={{ height: "calc(100vh - 220px)", minHeight: "440px" }}>
        {/* Folders - Gmail's own, plus the few this app adds. */}
        <Card className="flex w-[190px] shrink-0 flex-col overflow-y-auto p-2">
          {BUILTIN_FOLDERS.map((f) => {
            const c = countOf(f.key);
            return (
              <FolderRow
                key={f.key}
                label={f.label}
                count={c.total}
                unread={c.unread}
                active={folder === f.key}
                onClick={() => {
                  setFolder(f.key);
                  setOpenId(null);
                  clearSelection();
                  setConfirmDelete(false);
                }}
              />
            );
          })}
          {userLabels.length > 0 && (
            <>
              <p className="section-title mt-3 px-2.5 pb-1">Your labels</p>
              {userLabels.map((l) => (
                <FolderRow
                  key={l.id}
                  label={l.name}
                  count={l.total}
                  unread={l.unread}
                  active={folder === l.id}
                  onClick={() => {
                    setFolder(l.id);
                    setOpenId(null);
                  }}
                />
              ))}
            </>
          )}

          {/* Pinned to the bottom of the rail, where marko asked for it:
              "tu dole dat nastavenia". `mt-auto` is what pins it - the
              rail is a flex column, so the margin eats whatever space is
              left and the row sits on the floor however many labels are
              above it. */}
          <div className="sticky bottom-0 mt-auto border-t border-slate-200 bg-surface pt-1 dark:border-slate-800">
            <FolderRow
              label="Settings"
              count={0}
              unread={0}
              active={folder === SETTINGS_FOLDER}
              onClick={() => {
                setFolder(SETTINGS_FOLDER);
                setOpenId(null);
                clearSelection();
                setConfirmDelete(false);
              }}
            />
          </div>
        </Card>

        {folder === SETTINGS_FOLDER ? (
          <MailSettingsPane status={status} onStatus={setStatus} />
        ) : (
        <>
        <Card className="flex w-[340px] shrink-0 flex-col overflow-hidden p-0">
          {selected.size > 0 && (
            <div className="flex flex-wrap items-center gap-2 border-b border-slate-200 px-3 py-2 dark:border-slate-800">
              <span className="text-xs font-medium text-slate-700 dark:text-slate-200">
                {selected.size} selected
              </span>
              <button
                type="button"
                className="text-[11px] text-brand-600 hover:underline dark:text-brand-400"
                onClick={clearSelection}
              >
                Clear
              </button>
              <div className="ml-auto flex flex-wrap items-center gap-1.5">
                <Button variant="ghost" onClick={() => void bulk("read")}>
                  Read
                </Button>
                <Button variant="ghost" onClick={() => void bulk("unread")}>
                  Unread
                </Button>
                {folder === "trash" ? (
                  <>
                    <Button variant="secondary" onClick={() => void bulk("restore")}>
                      Restore
                    </Button>
                    {/* Delete is the one irreversible thing here, so it
                        asks - and the BACKEND refuses anything not already
                        in the trash regardless of what this button does. */}
                    {confirmDelete ? (
                      <Button variant="danger" onClick={() => void bulk("delete")}>
                        Delete forever?
                      </Button>
                    ) : (
                      <Button variant="ghost" onClick={() => setConfirmDelete(true)}>
                        Delete
                      </Button>
                    )}
                  </>
                ) : (
                  <Button variant="secondary" onClick={() => void bulk("trash")}>
                    Move to trash
                  </Button>
                )}
              </div>
            </div>
          )}
          {listLoading ? (
            <div className="flex items-center gap-2 p-4 text-xs text-slate-500 dark:text-slate-400">
              <Spinner className="h-4 w-4" /> Loading...
            </div>
          ) : visible.length === 0 ? (
            <div className="p-4">
              <p className="text-sm font-medium text-slate-700 dark:text-slate-200">Nothing here</p>
              <p className="mt-1 text-xs text-slate-500 dark:text-slate-400">Press Sync to fetch new mail.</p>
            </div>
          ) : (
            <div className="min-h-0 flex-1 overflow-y-auto">
              {visible.map((m) => (
                <MailRow
                  key={m.id}
                  m={m}
                  active={openId === m.id}
                  checked={selected.has(m.id)}
                  onClick={() => setOpenId(m.id)}
                  onToggle={() =>
                    setSelected((prev) => {
                      const next = new Set(prev);
                      if (!next.delete(m.id)) next.add(m.id);
                      return next;
                    })
                  }
                />
              ))}
            </div>
          )}
        </Card>

        <Card className="flex min-w-0 flex-1 flex-col overflow-hidden p-0">
          {open ? (
            <Reader message={open} onChanged={refresh} />
          ) : (
            <div className="flex flex-1 items-center justify-center p-8">
              <p className="text-sm text-slate-500 dark:text-slate-400">Pick a message to read it.</p>
            </div>
          )}
        </Card>
        </>
        )}
      </div>
    </div>
  );
}

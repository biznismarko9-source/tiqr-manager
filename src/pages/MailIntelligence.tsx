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
import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Link } from "react-router-dom";
import { api, errMsg } from "../lib/api";
import { lastMailSync, onMailSynced, requestMailSync } from "../lib/mailSync";
import {
  type MailConnectionStatus,
  type MailFolderCount,
  type MailLabel,
  type MailMessageSummary,
  type MailProposal,
  type MailReviewOutcome,
  type MailSyncResult,
} from "../lib/types";
import { Badge, Button, Card, EmptyState, Input, PageHeader, Spinner } from "../components/ui";
import { IconMail } from "../components/icons";
import { MailSettingsPane } from "../components/MailSettingsPane";
import { useToast } from "../lib/toast";

/** Folders this app defines itself, above Gmail's own. */
type Builtin =
  | "inbox"
  | "important"
  | "starred"
  | "finances"
  | "otp"
  | "review"
  | "promotions"
  | "spam"
  | "trash";

/* 2.110.0, in the order marko gave: "hlavny inbox: tam budu vsetky spravy
   okrem napriklad spamu a promotions, potom bude starred, potom budu
   finances, potom spam, trash a labels" - and "najskor treba important".
   Social / Updates / Forums are not folders any more: they are inbox mail,
   and the Inbox holds them. What each folder contains is decided in ONE
   place, the backend (commands/mail.rs::folder_expr). `gap` starts a new
   group in the rail. */
const BUILTIN_FOLDERS: { key: Builtin; label: string; gap?: boolean }[] = [
  { key: "inbox", label: "Inbox" },
  { key: "important", label: "Important" },
  { key: "starred", label: "Starred" },
  { key: "finances", label: "Finances" },
  { key: "otp", label: "OTP codes", gap: true },
  { key: "review", label: "Needs review" },
  { key: "promotions", label: "Promotions", gap: true },
  { key: "spam", label: "Spam" },
  { key: "trash", label: "Trash" },
];

/** How many messages one page of a folder loads. Scrolling loads the next
 *  page - there is no end to how far back a folder goes. */
const PAGE = 200;

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

const fmt = (n: number) => n.toLocaleString();

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
      title={label}
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

/* Memoised: a folder can hold thousands of rows once he has scrolled, and
   a new page arriving must not re-render every one already on screen. */
const MailRow = memo(function MailRow({
  m,
  active,
  checked,
  onOpen,
  onToggle,
}: {
  m: MailMessageSummary;
  active: boolean;
  checked: boolean;
  onOpen: (id: number) => void;
  onToggle: (id: number) => void;
}) {
  const onClick = () => onOpen(m.id);
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
            onToggle(m.id);
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
});

/** What changed in the reader: a field of this message (patched in place,
 *  so the list does not jump), or `null` for "something bigger - reload". */
type ReaderChange = Partial<MailMessageSummary> | null;

function Reader({ message, onChanged }: { message: MailMessageSummary; onChanged: (change: ReaderChange) => void }) {
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
        // Opening marks it read, in TIQR only.
        onChanged({ isUnread: false });
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
              Read from the message - check it before anything is created from it.
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
                          onChanged(null);
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
              onChanged(null);
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
              onChanged({ isUnread: !message.isUnread });
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
  const [folder, setFolder] = useState<string>("inbox");
  const [status, setStatus] = useState<MailConnectionStatus | null>(null);
  const [labels, setLabels] = useState<MailLabel[]>([]);
  const [counts, setCounts] = useState<Record<string, MailFolderCount>>({});
  const [rows, setRows] = useState<MailMessageSummary[]>([]);
  const [hasMore, setHasMore] = useState(false);
  const [loading, setLoading] = useState(true);
  const [listLoading, setListLoading] = useState(false);
  const [loadingMore, setLoadingMore] = useState(false);
  const [openId, setOpenId] = useState<number | null>(null);
  const [search, setSearch] = useState("");
  const [query, setQuery] = useState<string | null>(null);
  const [lastRun, setLastRun] = useState<MailSyncResult | null>(lastMailSync());
  const [nudged, setNudged] = useState(false);
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [confirmDelete, setConfirmDelete] = useState(false);

  /* Which folder and search the screen is showing. A page that comes back
     after he has already moved on is dropped rather than shown under the
     wrong folder. */
  const view = `${folder}|${query ?? ""}`;
  const viewRef = useRef(view);
  viewRef.current = view;
  const rowsRef = useRef(rows);
  rowsRef.current = rows;

  // Typing searches as he types, without a query per keystroke.
  useEffect(() => {
    const t = setTimeout(() => setQuery(search.trim() || null), 250);
    return () => clearTimeout(t);
  }, [search]);

  const open = useMemo(() => rows.find((m) => m.id === openId) ?? null, [rows, openId]);

  const loadHeader = useCallback(async () => {
    const [s, l, c] = await Promise.all([api.getMailStatus(), api.listMailLabels(), api.getMailFolderCounts()]);
    setStatus(s);
    setLabels(l);
    setCounts(Object.fromEntries(c.map((x) => [x.key, x])));
  }, []);

  /** The newest page of the folder on screen. */
  const loadFolder = useCallback(async () => {
    if (folder === SETTINGS_FOLDER) return;
    const key = viewRef.current;
    setListLoading(true);
    try {
      const page = await api.listMailFolder(folder, query, null, PAGE);
      if (viewRef.current !== key) return;
      setRows(page);
      setHasMore(page.length === PAGE);
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      if (viewRef.current === key) setListLoading(false);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [folder, query]);

  /** The next, older page - what scrolling to the bottom asks for. */
  const loadMore = useCallback(async () => {
    const current = rowsRef.current;
    if (!hasMore || loadingMore || current.length === 0 || folder === SETTINGS_FOLDER) return;
    const key = viewRef.current;
    const last = current[current.length - 1];
    setLoadingMore(true);
    try {
      const page = await api.listMailFolder(folder, query, { receivedAt: last.receivedAt, id: last.id }, PAGE);
      if (viewRef.current !== key) return;
      setRows((prev) => {
        const seen = new Set(prev.map((m) => m.id));
        return [...prev, ...page.filter((m) => !seen.has(m.id))];
      });
      setHasMore(page.length === PAGE);
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setLoadingMore(false);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [folder, query, hasMore, loadingMore]);

  /** New mail arrived: refresh the top of the folder without losing his
   *  place further down. */
  const refreshTop = useCallback(async () => {
    if (folder === SETTINGS_FOLDER) return;
    const key = viewRef.current;
    try {
      const page = await api.listMailFolder(folder, query, null, PAGE);
      if (viewRef.current !== key) return;
      const prev = rowsRef.current;
      if (prev.length <= PAGE || page.length < PAGE) {
        setRows(page);
        setHasMore(page.length === PAGE);
        return;
      }
      // He has scrolled past the first page: the fresh first page in
      // front, and everything older than it that he already had, kept.
      const edge = page[page.length - 1];
      const older = prev.filter(
        (m) => m.receivedAt < edge.receivedAt || (m.receivedAt === edge.receivedAt && m.id < edge.id),
      );
      setRows([...page, ...older]);
    } catch {
      // Silent: the next run refreshes again.
    }
  }, [folder, query]);

  useEffect(() => {
    api
      .clearStaleMailSync()
      .then(() => loadHeader())
      .catch((e) => toast.error(errMsg(e)))
      .finally(() => setLoading(false));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // A different folder or search: start from its newest message.
  useEffect(() => {
    setRows([]);
    setHasMore(false);
    void loadFolder();
  }, [loadFolder]);

  /* Every run of the reader (Layout.tsx) is announced; this screen
     refreshes from it. At most every three seconds while a long first read
     is pouring mail in - counting 100 000 messages is not free, and nobody
     reads a list that changes ten times a second. */
  const lastRefresh = useRef(0);
  const pendingRefresh = useRef<ReturnType<typeof setTimeout> | null>(null);
  useEffect(() => {
    const off = onMailSynced((r) => {
      setLastRun(r);
      setNudged(false);
      if (!(r.storedNew > 0 || r.changed || r.status === "failed")) return;
      const go = () => {
        lastRefresh.current = Date.now();
        pendingRefresh.current = null;
        void refreshTop();
        void loadHeader();
      };
      const since = Date.now() - lastRefresh.current;
      if (since >= 3_000) go();
      else if (!pendingRefresh.current) pendingRefresh.current = setTimeout(go, 3_000 - since);
    });
    return () => {
      off();
      if (pendingRefresh.current) clearTimeout(pendingRefresh.current);
      pendingRefresh.current = null;
    };
  }, [refreshTop, loadHeader]);

  const clearSelection = useCallback(() => setSelected(new Set()), []);

  const pick = (key: string) => {
    setFolder(key);
    setOpenId(null);
    clearSelection();
    setConfirmDelete(false);
  };

  const onOpen = useCallback((id: number) => setOpenId(id), []);
  const onToggle = useCallback(
    (id: number) =>
      setSelected((prev) => {
        const next = new Set(prev);
        if (!next.delete(id)) next.add(id);
        return next;
      }),
    [],
  );

  const onReaderChanged = useCallback(
    (change: ReaderChange) => {
      if (change && openId !== null) {
        setRows((prev) => prev.map((m) => (m.id === openId ? { ...m, ...change } : m)));
      } else {
        void refreshTop();
      }
      void loadHeader();
    },
    [openId, refreshTop, loadHeader],
  );

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
      void loadFolder();
      void loadHeader();
    } catch (e) {
      toast.error(errMsg(e));
    }
  };

  const onListScroll = (e: React.UIEvent<HTMLDivElement>) => {
    const el = e.currentTarget;
    if (el.scrollTop + el.clientHeight >= el.scrollHeight - 600) void loadMore();
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
            <Button variant="primary" onClick={() => pick(SETTINGS_FOLDER)}>
              Connect in Mail settings
            </Button>
          }
        />
        {folder === SETTINGS_FOLDER && (
          <div className="mt-4">
            <MailSettingsPane status={status!} onStatus={setStatus} />
          </div>
        )}
      </div>
    );
  }

  /* EVERY one of his own labels, including the empty ones.
     marko: "nech to cita vsetky zalozky". An empty folder is information;
     a folder that is not there is a bug report waiting to happen. */
  const userLabels = labels.filter((l) => l.kind !== "system");

  /* What the reader is doing, in words. A long first read looks exactly
     like a stuck one unless the screen says which folder it is on and how
     much is left. */
  const progress = lastRun?.progress ?? status.progress;
  const failed = lastRun?.status === "failed";
  const activity = status.paused
    ? "Paused"
    : failed
      ? "Not reading right now - see below"
      : lastRun?.rateLimited
        ? "Google asked TIQR to slow down - carrying on in a moment"
        : progress.reading
          ? `Reading ${progress.reading}${progress.waiting > 0 ? ` · ${fmt(progress.waiting)} waiting` : ""}`
          : status.syncIntervalSeconds > 0
            ? `Up to date · checks every ${
                status.syncIntervalSeconds < 60
                  ? `${status.syncIntervalSeconds}s`
                  : `${Math.round(status.syncIntervalSeconds / 60)} min`
              }`
            : "Up to date";
  const busy = nudged || (!!progress.reading && !status.paused && !failed);

  return (
    /* NOT `h-full`: Layout gives pages no height (PROTECTED_AREAS 2.55.0),
       so the three-pane block below pins its own, the way Notes does. */
    <div>
      <PageHeader
        title="Mail"
        subtitle={`${status.accountEmail ?? "Your inbox"} · ${fmt(progress.storedTotal)} messages${
          status.unreadCount > 0 ? ` · ${fmt(status.unreadCount)} unread` : ""
        }`}
        actions={
          <>
            {/* Says out loud that it reads by itself, so marko does not sit
                there wondering whether to press Sync. */}
            <span className="flex max-w-[340px] items-center gap-1.5 truncate text-[11px] text-slate-400 dark:text-slate-500">
              {busy && <Spinner className="h-3 w-3 shrink-0" />}
              <span className="truncate">{activity}</span>
            </span>
            <div className="w-56">
              <Input placeholder="Search this folder" value={search} onChange={(e) => setSearch(e.target.value)} />
            </div>
            <Button
              variant="secondary"
              disabled={status.paused || nudged}
              onClick={() => {
                setNudged(true);
                requestMailSync();
              }}
            >
              Sync now
            </Button>
          </>
        }
      />

      {status.lastError && lastRun?.status !== "synced" && (
        <Card className="mb-4 p-3">
          <p className="text-xs text-red-600 dark:text-red-400">{status.lastError}</p>
        </Card>
      )}

      <div className="flex gap-4" style={{ height: "calc(100vh - 220px)", minHeight: "440px" }}>
        {/* Folders - TIQR's own in marko's order, then his Gmail labels.
            Only the list scrolls; Settings sits BELOW it. It used to be a
            sticky row inside the scroll, and with enough labels they slid
            under it and showed through its see-through background. */}
        <Card className="flex w-[200px] shrink-0 flex-col overflow-hidden p-2">
          <div className="min-h-0 flex-1 overflow-y-auto">
          {BUILTIN_FOLDERS.map((f) => {
            const c = counts[f.key];
            return (
              <div key={f.key} className={f.gap ? "mt-2 border-t border-slate-200 pt-2 dark:border-slate-800" : ""}>
                <FolderRow
                  label={f.label}
                  count={c?.total ?? 0}
                  unread={c?.unread ?? 0}
                  active={folder === f.key}
                  onClick={() => pick(f.key)}
                />
              </div>
            );
          })}
          {userLabels.length > 0 && (
            <>
              <p className="section-title mt-3 border-t border-slate-200 px-2.5 pb-1 pt-3 dark:border-slate-800">
                Your labels
              </p>
              {userLabels.map((l) => (
                <FolderRow
                  key={l.id}
                  label={l.name}
                  count={l.total}
                  unread={l.unread}
                  active={folder === l.id}
                  onClick={() => pick(l.id)}
                />
              ))}
            </>
          )}

          </div>

          {/* At the bottom of the rail, where marko asked for it: "tu dole
              dat nastavenia". */}
          <div className="mt-1 shrink-0 border-t border-slate-200 pt-1 dark:border-slate-800">
            <FolderRow
              label="Settings"
              count={0}
              unread={0}
              active={folder === SETTINGS_FOLDER}
              onClick={() => pick(SETTINGS_FOLDER)}
            />
          </div>
        </Card>

        {folder === SETTINGS_FOLDER ? (
          <MailSettingsPane status={status} onStatus={setStatus} />
        ) : (
          <>
            <Card className="flex w-[360px] shrink-0 flex-col overflow-hidden p-0">
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
              {listLoading && rows.length === 0 ? (
                <div className="flex items-center gap-2 p-4 text-xs text-slate-500 dark:text-slate-400">
                  <Spinner className="h-4 w-4" /> Loading...
                </div>
              ) : rows.length === 0 ? (
                <div className="p-4">
                  <p className="text-sm font-medium text-slate-700 dark:text-slate-200">
                    {query ? "Nothing matches" : progress.reading ? "Nothing here yet" : "Nothing here"}
                  </p>
                  <p className="mt-1 text-xs leading-relaxed text-slate-500 dark:text-slate-400">
                    {query
                      ? "No message in this folder matches the search."
                      : progress.reading
                        ? `TIQR is reading your mail by itself, newest first - ${progress.reading} now. This folder fills in as it goes.`
                        : "There is no mail in this folder."}
                  </p>
                </div>
              ) : (
                <div className="min-h-0 flex-1 overflow-y-auto" onScroll={onListScroll}>
                  {rows.map((m) => (
                    <MailRow
                      key={m.id}
                      m={m}
                      active={openId === m.id}
                      checked={selected.has(m.id)}
                      onOpen={onOpen}
                      onToggle={onToggle}
                    />
                  ))}
                  {hasMore ? (
                    <div className="flex justify-center p-3">
                      <Button variant="ghost" disabled={loadingMore} onClick={() => void loadMore()}>
                        {loadingMore ? <Spinner className="h-4 w-4" /> : null}
                        Older messages
                      </Button>
                    </div>
                  ) : (
                    <p className="p-3 text-center text-[11px] text-slate-400 dark:text-slate-500">
                      {progress.reading ? "More arrives as TIQR reads further back." : "That is everything in this folder."}
                    </p>
                  )}
                </div>
              )}
            </Card>

            <Card className="flex min-w-0 flex-1 flex-col overflow-hidden p-0">
              {open ? (
                <Reader message={open} onChanged={onReaderChanged} />
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

// Mail (2.104.0) - a real inbox.
//
// marko: "urob normalne z toho mailu normalne cely inbox".
//
// 2.100.0 shipped this as a classification queue: a table of what TIQR had
// recognised, with no way to read anything. 2.101.0 then narrowed the sync to
// marketplace senders after his complaint that there was too much spam. Both
// were answers to the wrong question. What he wants is his mailbox, in TIQR,
// with the things that matter pulled to the front - so:
//
//   * The sync fetches the WHOLE mailbox again (headers only - see
//     commands/mail.rs). Junk is excluded by Gmail's own spam folder, not by
//     a filter this app invents.
//   * The spam problem is answered by TABS, not by refusing to fetch:
//     Important first, then all mail, then Gmail's own Promotions and
//     Social categories, then the review queue.
//   * It is a two-pane reader - list left, message right - the same shape
//     Notes already uses for list + content.
//
// WHAT THIS IS STILL NOT, said in the UI rather than left to look like a
// bug: marking a message read here does NOT mark it read in Gmail. The scope
// this app holds is `gmail.readonly` on purpose and cannot write a label.
// And nothing here creates a sale, an order or a finance entry - that is
// phase 3.
import { useCallback, useEffect, useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { api, errMsg } from "../lib/api";
import {
  type MailConnectionStatus,
  type MailMessageFilter,
  type MailMessageSummary,
  type MailOverview,
  type MailReviewOutcome,
} from "../lib/types";
import {
  Badge,
  Button,
  Card,
  EmptyState,
  Field,
  Figure,
  Figures,
  Input,
  PageHeader,
  SEGMENTED_TRACK,
  Select,
  Spinner,
  segmentedItemClass,
} from "../components/ui";
import { IconMail } from "../components/icons";
import { useToast } from "../lib/toast";

/** The tabs, and what each asks for.
 *
 *  `promotions` and `social` match Gmail's OWN category labels rather than
 *  anything this app guesses - a better answer to "too much spam" than a
 *  rule invented here, because it is the same split marko already sees in
 *  Gmail. */
type Tab = "important" | "inbox" | "promotions" | "social" | "review" | "overview";

const TABS: { key: Tab; label: string }[] = [
  { key: "important", label: "Important" },
  { key: "inbox", label: "All mail" },
  { key: "promotions", label: "Promotions" },
  { key: "social", label: "Social" },
  { key: "review", label: "Review" },
  { key: "overview", label: "Overview" },
];

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

const STATUS_LABELS: Record<string, string> = {
  detected: "Detected",
  classified: "Classified",
  review_required: "Needs review",
  reviewed: "Reviewed",
  irrelevant: "Not relevant",
  processed: "Processed",
  failed: "Failed",
};

const categoryLabel = (c: string) => CATEGORY_LABELS[c] ?? c;

function hasLabel(m: MailMessageSummary, label: string) {
  return m.labels.split(",").includes(label);
}

function formatWhen(iso: string) {
  try {
    const d = new Date(iso);
    // A mail list shows a time for today and a date for everything else; a
    // full timestamp on every row is noise at this density.
    return d.toDateString() === new Date().toDateString()
      ? d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })
      : d.toLocaleDateString();
  } catch {
    return iso;
  }
}

function CategoryBadge({ category }: { category: string }) {
  if (category === "other") return null;
  return <Badge tone="sold">{categoryLabel(category)}</Badge>;
}

/** One row: sender, subject, preview, time - what a mail list is for. */
function MailRow({ m, active, onClick }: { m: MailMessageSummary; active: boolean; onClick: () => void }) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`w-full border-b border-slate-200 px-3 py-2.5 text-left transition dark:border-slate-800 ${
        active ? "bg-brand-600/10" : "hover:bg-surface-muted"
      }`}
    >
      <div className="flex items-baseline gap-2">
        {m.isUnread && <span className="h-1.5 w-1.5 shrink-0 rounded-full bg-brand-500" aria-label="Unread" />}
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
      {m.snippet && <p className="mt-0.5 truncate text-[11px] text-slate-400 dark:text-slate-500">{m.snippet}</p>}
      {m.category !== "other" && (
        <div className="mt-1">
          <CategoryBadge category={m.category} />
        </div>
      )}
    </button>
  );
}

/** The reading pane. Fetches the body on open; never stores it. */
function Reader({ message, onChanged }: { message: MailMessageSummary; onChanged: () => void }) {
  const toast = useToast();
  const [body, setBody] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    setBody(null);
    setLoading(true);
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

  const verdict = async (outcome: MailReviewOutcome) => {
    setBusy(true);
    try {
      await api.setMailReviewOutcome(message.id, outcome);
      onChanged();
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="border-b border-slate-200 px-5 py-4 dark:border-slate-800">
        <h2 className="text-base font-semibold text-slate-900 dark:text-slate-100">
          {message.subject || "(no subject)"}
        </h2>
        <p className="mt-1 break-all text-xs text-slate-500 dark:text-slate-400">
          {message.fromName ? `${message.fromName} · ` : ""}
          {message.fromAddress}
        </p>
        <div className="mt-2 flex flex-wrap items-center gap-2">
          <span className="text-[11px] text-slate-400 dark:text-slate-500">
            {new Date(message.receivedAt).toLocaleString()}
          </span>
          <CategoryBadge category={message.category} />
          {message.detectedPlatform && (
            <span className="text-[11px] text-slate-500 dark:text-slate-400">{message.detectedPlatform}</span>
          )}
          <span className="text-[11px] text-slate-400 dark:text-slate-500">
            {STATUS_LABELS[message.status] ?? message.status}
          </span>
        </div>
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

      {message.reviewReason && (
        <div className="border-t border-slate-200 px-5 py-3 dark:border-slate-800">
          <p className="text-xs text-slate-500 dark:text-slate-400">{message.reviewReason}</p>
        </div>
      )}

      <div className="flex flex-wrap items-center gap-2 border-t border-slate-200 px-5 py-3 dark:border-slate-800">
        <Button variant="secondary" disabled={busy} onClick={() => void verdict("reviewed")}>
          Mark reviewed
        </Button>
        <Button variant="ghost" disabled={busy} onClick={() => void verdict("irrelevant")}>
          Not relevant
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
  const [tab, setTab] = useState<Tab>("important");
  const [status, setStatus] = useState<MailConnectionStatus | null>(null);
  const [overview, setOverview] = useState<MailOverview | null>(null);
  const [rows, setRows] = useState<MailMessageSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [listLoading, setListLoading] = useState(false);
  const [syncing, setSyncing] = useState(false);
  const [openId, setOpenId] = useState<number | null>(null);
  const [search, setSearch] = useState("");
  const [category, setCategory] = useState("");

  const filter = useMemo<MailMessageFilter>(
    () => ({
      search: search.trim() || null,
      category: category || null,
      status: tab === "review" ? "review_required" : null,
      limit: 500,
    }),
    [search, category, tab],
  );

  /* The list is fetched broadly and the TAB narrows it here rather than in
     SQL: Gmail's categories live in a comma-separated `labels` column, and
     a LIKE would also match a label that merely CONTAINS another. */
  const visible = useMemo(() => {
    switch (tab) {
      case "important":
        return rows.filter((m) => m.category !== "other");
      case "promotions":
        return rows.filter((m) => hasLabel(m, "category_promotions"));
      case "social":
        return rows.filter((m) => hasLabel(m, "category_social"));
      case "inbox":
        return rows.filter((m) => !hasLabel(m, "category_promotions") && !hasLabel(m, "category_social"));
      default:
        return rows;
    }
  }, [rows, tab]);

  const loadHeader = useCallback(async () => {
    const [s, o] = await Promise.all([api.getMailStatus(), api.getMailOverview()]);
    setStatus(s);
    setOverview(o);
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
    if (tab === "overview") return;
    void loadList();
  }, [tab, loadList]);

  const refresh = useCallback(() => {
    void loadHeader();
    void loadList();
  }, [loadHeader, loadList]);

  const doSync = async () => {
    setSyncing(true);
    try {
      const result = await api.syncMailNow();
      if (result.status === "synced") {
        toast.success(`${result.storedNew} new, ${result.duplicatesIgnored} already seen.`);
      } else if (result.status === "skipped") {
        toast.info(result.message);
      } else {
        toast.error(result.message);
      }
      await loadHeader();
      if (tab !== "overview") await loadList();
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setSyncing(false);
    }
  };

  const open = useMemo(() => visible.find((m) => m.id === openId) ?? null, [visible, openId]);

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

  return (
    /* NOT `h-full`. `Layout` renders pages in a plain `px-7 py-5` div with
       no height, so `h-full` collapses to the content - see PROTECTED_AREAS,
       2.55.0. The two-pane block below pins its own height the same way the
       Notes page does, and the root stays an ordinary block. */
    <div>
      <PageHeader
        title="Mail"
        subtitle={
          status.accountEmail
            ? `${status.accountEmail}${status.unreadCount > 0 ? ` · ${status.unreadCount} unread` : ""}`
            : "Your inbox, with what matters pulled to the front."
        }
        actions={
          <>
            <div className={SEGMENTED_TRACK}>
              {TABS.map((t) => (
                <button
                  key={t.key}
                  type="button"
                  onClick={() => {
                    setTab(t.key);
                    setOpenId(null);
                  }}
                  aria-pressed={tab === t.key}
                  className={segmentedItemClass(tab === t.key)}
                >
                  {t.label}
                </button>
              ))}
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

      {tab === "overview" ? (
        overview && (
          <div className="space-y-5">
            <Figures>
              <Figure label="Messages" value={String(overview.totalMessages)} emphasis />
              <Figure label="Unread" value={String(status.unreadCount)} />
              <Figure label="New since last sync" value={String(overview.newSinceLastSync)} />
              <Figure label="Recognised" value={String(overview.recognisedCount)} />
              <Figure
                label="Awaiting review"
                value={String(overview.reviewRequiredCount)}
                tone={overview.reviewRequiredCount > 0 ? "warning" : "default"}
              />
            </Figures>
            <Card className="p-4">
              <h3 className="section-title mb-3">By category</h3>
              {overview.byCategory.length === 0 ? (
                <p className="text-xs text-slate-500 dark:text-slate-400">Nothing classified yet.</p>
              ) : (
                <ul className="space-y-1.5">
                  {overview.byCategory.map((c) => (
                    <li key={c.category} className="flex items-center justify-between gap-2">
                      <span className="truncate text-sm text-slate-600 dark:text-slate-300">
                        {categoryLabel(c.category)}
                      </span>
                      <span className="num text-sm text-slate-700 dark:text-slate-200">{c.count}</span>
                    </li>
                  ))}
                </ul>
              )}
            </Card>
          </div>
        )
      ) : (
        <>
          <div className="mb-3 flex flex-wrap items-end gap-2">
            <div className="min-w-[220px] flex-1">
              <Field label="Search">
                <Input placeholder="Sender or subject" value={search} onChange={(e) => setSearch(e.target.value)} />
              </Field>
            </div>
            <div className="w-44">
              <Field label="Category">
                <Select value={category} onChange={(e) => setCategory(e.target.value)}>
                  <option value="">All</option>
                  {Object.keys(CATEGORY_LABELS).map((c) => (
                    <option key={c} value={c}>
                      {CATEGORY_LABELS[c]}
                    </option>
                  ))}
                </Select>
              </Field>
            </div>
          </div>

          {/* Two panes: list and reader - the same shape Notes already uses
              for list + content, so the app has one answer to that layout. */}
          <div
            className="flex min-h-[420px] gap-4"
            /* The one place this page sets a real height, same mechanism as
               Notes (2.55.0): the offset covers the sidebar's padding, the
               page header and the filter row above. Without it the reading
               pane has no height to scroll inside and the whole list just
               pushes the page down. */
            style={{ height: "calc(100vh - 290px)" }}
          >
            <Card className="flex w-[360px] shrink-0 flex-col overflow-hidden p-0">
              {listLoading ? (
                <div className="flex items-center gap-2 p-4 text-xs text-slate-500 dark:text-slate-400">
                  <Spinner className="h-4 w-4" /> Loading...
                </div>
              ) : visible.length === 0 ? (
                <div className="p-4">
                  <p className="text-sm font-medium text-slate-700 dark:text-slate-200">Nothing here</p>
                  <p className="mt-1 text-xs text-slate-500 dark:text-slate-400">
                    {tab === "review"
                      ? "Recognised mail that needs a look will appear here."
                      : "Press Sync to fetch new mail."}
                  </p>
                </div>
              ) : (
                <div className="min-h-0 flex-1 overflow-y-auto">
                  {visible.map((m) => (
                    <MailRow key={m.id} m={m} active={openId === m.id} onClick={() => setOpenId(m.id)} />
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
          </div>
        </>
      )}
    </div>
  );
}

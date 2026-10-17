// Mail Intelligence (2.100.0) - PHASE 1.
//
// Three tabs, matching what marko asked for: Overview, Inbox Activity and
// the Review Queue. What this screen can honestly show today is bounded by
// what the backend actually does, and the screen says so out loud rather
// than implying more:
//
//   * It shows what arrived and what TIQR made of it.
//   * It does NOT create sales, orders, tickets or finance entries. Nothing
//     in commands/mail.rs writes one (see that module's doc comment), so
//     there is no button here that pretends otherwise. The review queue's
//     verdicts are "reviewed" and "not relevant" - deliberately not
//     "approve and create", because there is nothing behind it yet.
//
// The connection itself is configured in Settings -> Integrations -> Email;
// this screen carries only a compact status strip and "Sync now", so the
// connection form lives in exactly one place.
import { Fragment, useCallback, useEffect, useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { api, errMsg } from "../lib/api";
import {
  type MailConnectionStatus,
  type MailMessageDetail,
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
  TableSkeleton,
  segmentedItemClass,
} from "../components/ui";
import { IconMail } from "../components/icons";
import { useToast } from "../lib/toast";

type Tab = "overview" | "activity" | "review";

const TABS: { key: Tab; label: string }[] = [
  { key: "overview", label: "Overview" },
  { key: "activity", label: "Inbox Activity" },
  { key: "review", label: "Review Queue" },
];

// The nine categories commands/mail.rs can produce, with the labels marko
// reads. Kept as one list so the filter dropdown and every badge agree -
// a second copy is how a renamed category silently stops matching.
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

function categoryLabel(c: string) {
  return CATEGORY_LABELS[c] ?? c;
}

/** A category badge. `other` is deliberately quiet - most mail is `other`
 *  and colouring it would make the list look alarming for no reason. */
function CategoryBadge({ category }: { category: string }) {
  const tone = category === "other" ? "available" : "sold";
  return <Badge tone={tone}>{categoryLabel(category)}</Badge>;
}

/** Confidence, as words rather than a number. A percentage would imply a
 *  precision the rules do not have - they are keyword matches, not a model. */
function ConfidenceMark({ confidence }: { confidence: string }) {
  const tone =
    confidence === "high"
      ? "text-emerald-600 dark:text-emerald-400"
      : confidence === "medium"
        ? "text-amber-600 dark:text-amber-400"
        : "text-slate-400 dark:text-slate-500";
  return <span className={`text-[11px] font-medium ${tone}`}>{confidence}</span>;
}

function formatWhen(iso: string) {
  try {
    return new Date(iso).toLocaleString();
  } catch {
    return iso;
  }
}

/** The one place the connection state is summarised on this screen. The full
 *  form is in Settings; this says enough to know whether the numbers below
 *  are current, and offers the one action worth having here. */
function ConnectionStrip({
  status,
  syncing,
  onSync,
}: {
  status: MailConnectionStatus | null;
  syncing: boolean;
  onSync: () => void;
}) {
  if (!status) return null;

  if (!status.connected) {
    return (
      <Card className="mb-5 flex flex-wrap items-center justify-between gap-3 p-4">
        <div className="min-w-0">
          <p className="text-sm font-medium text-slate-800 dark:text-slate-200">No mailbox connected</p>
          <p className="mt-0.5 text-xs text-slate-500 dark:text-slate-400">
            Connect a Gmail account to let TIQR recognise sale, payout and refund mail.
          </p>
        </div>
        <Link to="/settings/integrations">
          <Button variant="primary">Connect in Settings</Button>
        </Link>
      </Card>
    );
  }

  // Connected but Google did not grant the mail scope is a real state and a
  // confusing one - say exactly what is wrong rather than failing silently
  // on every sync.
  const scopeProblem = !status.gmailScopeGranted;

  return (
    <Card className="mb-5 flex flex-wrap items-center justify-between gap-3 p-4">
      <div className="min-w-0">
        <div className="flex flex-wrap items-center gap-2">
          <p className="truncate text-sm font-medium text-slate-800 dark:text-slate-200">
            {status.accountEmail ?? "Mailbox connected"}
          </p>
          {status.paused && <Badge tone="available">Paused</Badge>}
          {/* `cancelled` is the red tone in STATUS_TONES (ui.tsx). There is
              no "negative" key there - using one would silently fall back to
              grey, which is the wrong colour for a broken permission. */}
          {scopeProblem && <Badge tone="cancelled">no mail permission</Badge>}
        </div>
        <p className="mt-0.5 text-xs text-slate-500 dark:text-slate-400">
          {status.lastSyncAt
            ? `Last sync ${formatWhen(status.lastSyncAt)} - ${status.lastSyncCount} new`
            : "Not synced yet."}
        </p>
        {status.lastError && (
          <p className="mt-1 text-xs text-red-600 dark:text-red-400">{status.lastError}</p>
        )}
      </div>
      <Button variant="secondary" disabled={syncing || status.paused} onClick={onSync}>
        {syncing ? <Spinner className="h-4 w-4" /> : null}
        Sync now
      </Button>
    </Card>
  );
}

/** One message, expanded: what was detected, and the whole audit trail. */
function MessageDetail({
  detail,
  onVerdict,
  busy,
}: {
  detail: MailMessageDetail;
  onVerdict: (outcome: MailReviewOutcome) => void;
  busy: boolean;
}) {
  const m = detail.message;
  return (
    <div className="border-t border-slate-200 px-4 py-4 dark:border-slate-800">
      <div className="grid gap-4 lg:grid-cols-2">
        <div>
          <p className="section-title">What TIQR detected</p>
          <dl className="mt-2 space-y-1.5 text-xs">
            <div className="flex gap-2">
              <dt className="w-28 shrink-0 text-slate-500 dark:text-slate-400">Category</dt>
              <dd className="text-slate-700 dark:text-slate-200">{categoryLabel(m.category)}</dd>
            </div>
            <div className="flex gap-2">
              <dt className="w-28 shrink-0 text-slate-500 dark:text-slate-400">Confidence</dt>
              <dd>
                <ConfidenceMark confidence={m.confidence} />
              </dd>
            </div>
            <div className="flex gap-2">
              <dt className="w-28 shrink-0 text-slate-500 dark:text-slate-400">Platform</dt>
              <dd className="text-slate-700 dark:text-slate-200">
                {m.detectedPlatform ?? <span className="text-slate-400 dark:text-slate-500">not recognised</span>}
              </dd>
            </div>
            <div className="flex gap-2">
              <dt className="w-28 shrink-0 text-slate-500 dark:text-slate-400">Sender</dt>
              <dd className="break-all text-slate-700 dark:text-slate-200">{m.fromAddress}</dd>
            </div>
          </dl>
          {m.reviewReason && (
            <p className="mt-3 rounded-lg bg-surface-muted px-3 py-2 text-xs text-slate-600 dark:text-slate-300">
              {m.reviewReason}
            </p>
          )}
          {m.bodyExcerpt && (
            <>
              <p className="section-title mt-4">Excerpt</p>
              {/* Plain text, always. The backend stripped the HTML before it
                  was ever stored, and this renders it as text - there is no
                  path in this app that hands a message body to the DOM as
                  markup, which is the whole point of stripping it. */}
              <p className="mt-1 whitespace-pre-wrap break-words text-xs text-slate-600 dark:text-slate-300">
                {m.bodyExcerpt}
              </p>
            </>
          )}
        </div>

        <div>
          <p className="section-title">Processing history</p>
          <ol className="mt-2 space-y-2">
            {detail.history.length === 0 && (
              <li className="text-xs text-slate-500 dark:text-slate-400">Nothing recorded yet.</li>
            )}
            {detail.history.map((h) => (
              <li key={h.id} className="rounded-lg bg-surface-muted px-3 py-2">
                <div className="flex flex-wrap items-center gap-2">
                  <span className="text-xs font-medium text-slate-700 dark:text-slate-200">{h.step}</span>
                  <span className="text-[11px] text-slate-500 dark:text-slate-400">{h.outcome}</span>
                  <span className="ml-auto font-mono text-[11px] text-slate-400 dark:text-slate-500">
                    {formatWhen(h.at)}
                  </span>
                </div>
                {h.detail && (
                  <p className="mt-1 text-[11px] text-slate-500 dark:text-slate-400">{h.detail}</p>
                )}
              </li>
            ))}
          </ol>

          {/* The verdicts phase 1 can honestly offer. There is no "approve
              and create" here because there is nothing behind it yet - see
              this file's header. */}
          <div className="mt-4 flex flex-wrap items-center gap-2">
            <Button variant="secondary" disabled={busy} onClick={() => onVerdict("reviewed")}>
              Mark reviewed
            </Button>
            <Button variant="ghost" disabled={busy} onClick={() => onVerdict("irrelevant")}>
              Not relevant
            </Button>
            {m.status !== "review_required" && (
              <Button variant="ghost" disabled={busy} onClick={() => onVerdict("needs_review")}>
                Back to review
              </Button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

/** The shared list. Inbox Activity and the Review Queue are the same table
 *  with a different filter, so they are the same component - two copies
 *  would drift the moment a column changed. */
function MessageList({
  rows,
  loading,
  emptyTitle,
  emptyDescription,
  onChanged,
}: {
  rows: MailMessageSummary[];
  loading: boolean;
  emptyTitle: string;
  emptyDescription: string;
  onChanged: () => void;
}) {
  const toast = useToast();
  const [openId, setOpenId] = useState<number | null>(null);
  const [detail, setDetail] = useState<MailMessageDetail | null>(null);
  const [detailBusy, setDetailBusy] = useState(false);

  const open = async (id: number) => {
    if (openId === id) {
      setOpenId(null);
      setDetail(null);
      return;
    }
    setOpenId(id);
    setDetail(null);
    try {
      setDetail(await api.getMailMessage(id));
    } catch (e) {
      toast.error(errMsg(e));
      setOpenId(null);
    }
  };

  const verdict = async (id: number, outcome: MailReviewOutcome) => {
    setDetailBusy(true);
    try {
      setDetail(await api.setMailReviewOutcome(id, outcome));
      onChanged();
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setDetailBusy(false);
    }
  };

  if (loading) return <TableSkeleton />;
  if (rows.length === 0) {
    return <EmptyState icon={<IconMail className="h-5 w-5" />} title={emptyTitle} description={emptyDescription} />;
  }

  return (
    <div className="table-shell">
      <table className="w-full text-sm">
        <colgroup>
          <col style={{ width: "26%" }} />
          <col style={{ width: "34%" }} />
          <col style={{ width: "14%" }} />
          <col style={{ width: "14%" }} />
          <col style={{ width: "12%" }} />
        </colgroup>
        <thead>
          <tr>
            <th className="th">Sender</th>
            <th className="th">Subject</th>
            <th className="th">Category</th>
            <th className="th">Received</th>
            <th className="th">Status</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((m) => (
            <Fragment key={m.id}>
              <tr
                className="cursor-pointer hover:bg-surface-muted"
                onClick={() => void open(m.id)}
              >
                <td className="td">
                  <p className="truncate font-medium text-slate-800 dark:text-slate-200">
                    {m.fromName ?? m.fromAddress}
                  </p>
                  {m.fromName && (
                    <p className="truncate text-[11px] text-slate-500 dark:text-slate-400">{m.fromAddress}</p>
                  )}
                </td>
                <td className="td">
                  <p className="truncate text-slate-700 dark:text-slate-200">{m.subject || "(no subject)"}</p>
                  {m.detectedPlatform && (
                    <p className="truncate text-[11px] text-slate-500 dark:text-slate-400">{m.detectedPlatform}</p>
                  )}
                </td>
                <td className="td">
                  <div className="flex flex-wrap items-center gap-1.5">
                    <CategoryBadge category={m.category} />
                    <ConfidenceMark confidence={m.confidence} />
                  </div>
                </td>
                <td className="td num whitespace-nowrap text-slate-600 dark:text-slate-300">
                  {formatWhen(m.receivedAt)}
                </td>
                <td className="td text-slate-600 dark:text-slate-300">{STATUS_LABELS[m.status] ?? m.status}</td>
              </tr>
              {openId === m.id && (
                <tr>
                  <td className="p-0" colSpan={5}>
                    {detail ? (
                      <MessageDetail
                        detail={detail}
                        busy={detailBusy}
                        onVerdict={(o) => void verdict(m.id, o)}
                      />
                    ) : (
                      <div className="flex items-center gap-2 px-4 py-4 text-xs text-slate-500 dark:text-slate-400">
                        <Spinner className="h-4 w-4" /> Loading...
                      </div>
                    )}
                  </td>
                </tr>
              )}
            </Fragment>
          ))}
        </tbody>
      </table>
    </div>
  );
}

export default function MailIntelligence() {
  const toast = useToast();
  const [tab, setTab] = useState<Tab>("overview");
  const [status, setStatus] = useState<MailConnectionStatus | null>(null);
  const [overview, setOverview] = useState<MailOverview | null>(null);
  const [rows, setRows] = useState<MailMessageSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [listLoading, setListLoading] = useState(false);
  const [syncing, setSyncing] = useState(false);

  const [search, setSearch] = useState("");
  const [category, setCategory] = useState("");
  const [statusFilter, setStatusFilter] = useState("");
  const [fromDate, setFromDate] = useState("");
  const [toDate, setToDate] = useState("");

  const filter = useMemo<MailMessageFilter>(
    () => ({
      search: search.trim() || null,
      category: category || null,
      // The Review Queue IS the status filter - its own tab pins it, so the
      // dropdown below is only offered on Inbox Activity.
      status: tab === "review" ? "review_required" : statusFilter || null,
      fromDate: fromDate || null,
      toDate: toDate || null,
    }),
    [search, category, statusFilter, fromDate, toDate, tab],
  );

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
    // A hard kill can leave a sync claim behind. Cleared once, here, on the
    // screen that would be blocked by it - never on a timer.
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

  const doSync = async () => {
    setSyncing(true);
    try {
      const result = await api.syncMailNow();
      // Three outcomes, three messages. "skipped" is neither success nor
      // failure - paused, not connected, or a run already going - and
      // showing it as an error would be wrong.
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

  if (loading) {
    return (
      <div>
        <PageHeader title="Mail Intelligence" subtitle="What arrived, and what TIQR made of it." />
        <TableSkeleton />
      </div>
    );
  }

  return (
    <div>
      <PageHeader
        title="Mail Intelligence"
        subtitle="What arrived, and what TIQR made of it. Nothing here changes your sales, inventory or finances."
        actions={
          <div className={SEGMENTED_TRACK}>
            {TABS.map((t) => (
              <button
                key={t.key}
                type="button"
                onClick={() => setTab(t.key)}
                aria-pressed={tab === t.key}
                className={segmentedItemClass(tab === t.key)}
              >
                {t.label}
              </button>
            ))}
          </div>
        }
      />

      <ConnectionStrip status={status} syncing={syncing} onSync={() => void doSync()} />

      {tab === "overview" && overview && (
        <div className="space-y-5">
          {/* One panel divided by hairlines, not a row of cards - the house
              pattern for a row of figures since 2.90.0. */}
          <Figures>
            <Figure label="Messages seen" value={String(overview.totalMessages)} emphasis />
            <Figure label="New since last sync" value={String(overview.newSinceLastSync)} />
            <Figure label="Recognised" value={String(overview.recognisedCount)} />
            <Figure
              label="Awaiting review"
              value={String(overview.reviewRequiredCount)}
              tone={overview.reviewRequiredCount > 0 ? "warning" : "default"}
            />
            <Figure label="Reviewed" value={String(overview.processedCount)} />
            {/* No "Failed" figure, deliberately. Nothing in phase 1 sets a
                message's status to `failed` - there is no processing step
                that can fail yet - so the counter could only ever read 0.
                A figure that is structurally always zero reads as a broken
                number rather than a true one. `failedCount` stays on the
                model and in the backend count for the phase that can
                actually set it; a sync failure itself is shown where it
                belongs, on the connection strip above. */}
          </Figures>

          <div className="grid gap-5 lg:grid-cols-[minmax(0,1fr)_320px]">
            <Card className="p-4">
              <h3 className="section-title mb-3">Recent activity</h3>
              {overview.recent.length === 0 ? (
                <EmptyState
                  icon={<IconMail className="h-5 w-5" />}
                  title="Nothing yet"
                  description="Connect a mailbox and sync to see what arrives here."
                />
              ) : (
                <ul className="divide-y divide-slate-100 dark:divide-slate-800">
                  {overview.recent.map((m) => (
                    <li key={m.id} className="flex flex-wrap items-center gap-2 py-2">
                      <span className="min-w-0 flex-1 truncate text-sm text-slate-700 dark:text-slate-200">
                        {m.subject || "(no subject)"}
                      </span>
                      <CategoryBadge category={m.category} />
                      <span className="whitespace-nowrap font-mono text-[11px] text-slate-400 dark:text-slate-500">
                        {formatWhen(m.receivedAt)}
                      </span>
                    </li>
                  ))}
                </ul>
              )}
            </Card>

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
        </div>
      )}

      {tab !== "overview" && (
        <div className="space-y-4">
          <Card className="flex flex-wrap items-end gap-3 p-4">
            <div className="min-w-[200px] flex-1">
              <Field label="Search">
                <Input
                  placeholder="Sender or subject"
                  value={search}
                  onChange={(e) => setSearch(e.target.value)}
                />
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
            {tab === "activity" && (
              <div className="w-44">
                <Field label="Status">
                  <Select value={statusFilter} onChange={(e) => setStatusFilter(e.target.value)}>
                    <option value="">All</option>
                    {Object.keys(STATUS_LABELS).map((s) => (
                      <option key={s} value={s}>
                        {STATUS_LABELS[s]}
                      </option>
                    ))}
                  </Select>
                </Field>
              </div>
            )}
            <div className="w-40">
              <Field label="From">
                <Input type="date" value={fromDate} onChange={(e) => setFromDate(e.target.value)} />
              </Field>
            </div>
            <div className="w-40">
              <Field label="To">
                <Input type="date" value={toDate} onChange={(e) => setToDate(e.target.value)} />
              </Field>
            </div>
          </Card>

          <MessageList
            rows={rows}
            loading={listLoading}
            emptyTitle={tab === "review" ? "Nothing waiting" : "No messages match"}
            emptyDescription={
              tab === "review"
                ? "Recognised mail that needs a human look will appear here."
                : "Change the filters, or sync to fetch new mail."
            }
            onChanged={() => {
              void loadHeader();
              void loadList();
            }}
          />
        </div>
      )}
    </div>
  );
}

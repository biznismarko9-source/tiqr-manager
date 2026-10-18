// Mail's own settings (2.109.0).
//
// marko: "tie nastavenia ktore si dal do integrations ku mailu tak odstran
// a presun ich do tych settings v maily samotnom v zlozke".
//
// So everything about the mailbox now lives behind the Settings row at the
// bottom of Mail's folder rail, next to the mail it governs - not three
// screens away under Integrations. Settings -> Integrations keeps a single
// line saying where it went, because a link is a better answer than a
// section that silently disappeared.
//
// Two of these sections are new, and both come from the same message:
//
//   "ze napriklad ktoru hlavicku rovno ma riesit alebo ktoru rovno dat do
//    presunutych alebo co a kedy do spamu atd, pohraj sa s tym"
//   "toto s moznostou upravit, odstranit pridat"
//
// -> RULES, and a sender list he can actually edit.
//
// THE LIMIT, stated here and on screen rather than left to look like a bug:
// the Gmail scope this app holds is read-only. No rule touches Gmail. Every
// action below happens inside TIQR, and the worst a wrong rule can do is
// move mail into TIQR's own trash, where it can be moved straight back.
import { useCallback, useEffect, useState } from "react";
import { api, errMsg } from "../lib/api";
import {
  type MailConnectionStatus,
  type MailRule,
  type MailSender,
  type MailTestResult,
} from "../lib/types";
import { Badge, Button, Card, Input, Select, Spinner } from "./ui";
import { useToast } from "../lib/toast";

/* What each rule action does, in marko's terms rather than the column it
   writes. The screen reads these out next to the rule, so a rule list is
   never a list of verbs you have to remember the meaning of. */
const ACTION_LABEL: Record<string, string> = {
  trash: "Move to trash",
  read: "Mark as read",
  review: "Send to Needs review",
  keep: "Keep - stop here",
};

const ACTION_HINT: Record<string, string> = {
  trash: "Goes to TIQR's trash. Still restorable; Gmail is untouched.",
  read: "Stays where it is, just stops counting as unread.",
  review: "Shows up in Needs review so you see it first.",
  keep: "Protects it: no rule below this one can act on it.",
};

const FIELD_LABEL: Record<string, string> = {
  from: "Sender",
  subject: "Subject",
  preview: "Preview text",
};

const MATCH_LABEL: Record<string, string> = {
  contains: "contains",
  equals: "is exactly",
  starts: "starts with",
  ends: "ends with",
};

/* Starting points, not magic. Each one fills the form and is then edited
   like anything else - the point is that the first rule is the hardest to
   write, not that these are the right rules for his mailbox. */
const RULE_PRESETS: { label: string; hint: string; rule: Omit<MailRule, "id" | "createdAt" | "hits" | "lastHitAt"> }[] = [
  {
    label: "Protect order confirmations",
    hint: "Put this first. Nothing below it can trash an order mail.",
    rule: { position: 0, enabled: true, field: "subject", matchKind: "contains", needle: "order confirmation", action: "keep" },
  },
  {
    label: "Payouts straight to Needs review",
    hint: "Money mail surfaces instead of sitting in the pile.",
    rule: { position: 0, enabled: true, field: "subject", matchKind: "contains", needle: "payout", action: "review" },
  },
  {
    label: "Silence newsletters",
    hint: "Keeps them, stops them counting as unread.",
    rule: { position: 0, enabled: true, field: "from", matchKind: "contains", needle: "newsletter", action: "read" },
  },
  {
    label: "Trash 'no-reply' noise",
    hint: "Broad on purpose - put a Keep rule above it first.",
    rule: { position: 0, enabled: true, field: "from", matchKind: "contains", needle: "no-reply", action: "trash" },
  },
];

const BLANK: MailRule = {
  id: "",
  position: 0,
  enabled: true,
  field: "subject",
  matchKind: "contains",
  needle: "",
  action: "review",
  createdAt: "",
  hits: 0,
  lastHitAt: null,
};

/* Three states, not two. Moved here with the rest of the mailbox settings
   in 2.109.0; the convention is the Discord test line's (2.99.0) and the
   reason is the same: a request that never came back is NOT a refusal, and
   calling it one sends marko off reconfiguring something that was never
   broken. "Not confirmed" is the honest third answer. */
function MailTestLine({ result }: { result: MailTestResult }) {
  const tone =
    result.status === "ok"
      ? "text-emerald-600 dark:text-emerald-400"
      : result.status === "unknown"
        ? "text-amber-600 dark:text-amber-400"
        : "text-red-600 dark:text-red-400";
  const heading = result.status === "ok" ? "Reachable" : result.status === "unknown" ? "Not confirmed" : "Failed";
  return (
    <div className="rounded-lg border border-slate-200 bg-slate-50 px-3 py-2 dark:border-slate-800 dark:bg-slate-800/40">
      <p className={`text-xs font-semibold ${tone}`}>
        {heading}
        {result.errorCode ? ` (${result.errorCode})` : ""}
      </p>
      <p className="mt-0.5 text-xs text-slate-600 dark:text-slate-300">{result.message}</p>
      {result.accountEmail && (
        <p className="mt-0.5 text-[11px] text-slate-500 dark:text-slate-400">
          {result.accountEmail}
          {typeof result.messagesTotal === "number" ? ` - ${result.messagesTotal} messages` : ""}
        </p>
      )}
    </div>
  );
}

function Section({ title, hint, children }: { title: string; hint?: string; children: React.ReactNode }) {
  return (
    <section className="border-b border-slate-200 px-5 py-5 last:border-b-0 dark:border-slate-800">
      <h3 className="text-sm font-semibold text-slate-800 dark:text-slate-200">{title}</h3>
      {hint && <p className="mb-3 mt-0.5 text-xs text-slate-500 dark:text-slate-400">{hint}</p>}
      <div className={hint ? "" : "mt-3"}>{children}</div>
    </section>
  );
}

export function MailSettingsPane({
  status,
  onStatus,
}: {
  status: MailConnectionStatus;
  onStatus: (s: MailConnectionStatus) => void;
}) {
  const toast = useToast();

  const [rules, setRules] = useState<MailRule[]>([]);
  const [senders, setSenders] = useState<MailSender[]>([]);
  const [loading, setLoading] = useState(true);
  const [draft, setDraft] = useState<MailRule>(BLANK);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [newSender, setNewSender] = useState("");
  const [editSender, setEditSender] = useState<{ was: string; now: string } | null>(null);
  const [customQuery, setCustomQuery] = useState(status.customQuery ?? "");
  const [retention, setRetention] = useState(String(status.retentionDays));
  const [busy, setBusy] = useState(false);
  const [applying, setApplying] = useState(false);
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<MailTestResult | null>(null);

  const load = useCallback(async () => {
    try {
      const [r, s] = await Promise.all([api.listMailRules(), api.listMailSenders()]);
      setRules(r);
      setSenders(s);
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setLoading(false);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  /* ---- rules --------------------------------------------------------- */

  const saveRule = async () => {
    if (!draft.needle.trim()) {
      toast.error("Type what the rule should look for.");
      return;
    }
    setBusy(true);
    try {
      setRules(await api.saveMailRule(draft));
      setDraft(BLANK);
      setEditingId(null);
      toast.success(editingId ? "Rule updated" : "Rule added");
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setBusy(false);
    }
  };

  const removeRule = async (id: string) => {
    try {
      setRules(await api.deleteMailRule(id));
      if (editingId === id) {
        setDraft(BLANK);
        setEditingId(null);
      }
    } catch (e) {
      toast.error(errMsg(e));
    }
  };

  const toggleRule = async (r: MailRule) => {
    try {
      setRules(await api.saveMailRule({ ...r, enabled: !r.enabled }));
    } catch (e) {
      toast.error(errMsg(e));
    }
  };

  /* Order is the rule's meaning - a Keep only protects what sits below it -
     so moving one is an edit, not a view preference. Swapping positions
     with the neighbour keeps every other rule where it was. */
  const move = async (index: number, dir: -1 | 1) => {
    const other = rules[index + dir];
    const self = rules[index];
    if (!other || !self) return;
    try {
      await api.saveMailRule({ ...self, position: other.position });
      setRules(await api.saveMailRule({ ...other, position: self.position }));
    } catch (e) {
      toast.error(errMsg(e));
    }
  };

  const runNow = async () => {
    setApplying(true);
    try {
      const n = await api.applyMailRulesNow();
      toast.success(
        n === 0 ? "No stored mail matched your rules." : `${n} message${n === 1 ? "" : "s"} changed.`,
      );
      setRules(await api.listMailRules());
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setApplying(false);
    }
  };

  /* ---- senders -------------------------------------------------------- */

  const addSender = async () => {
    if (!newSender.trim()) return;
    try {
      setSenders(await api.saveMailSender(newSender));
      setNewSender("");
      onStatus(await api.getMailStatus());
    } catch (e) {
      toast.error(errMsg(e));
    }
  };

  const commitSenderEdit = async () => {
    if (!editSender) return;
    const { was, now } = editSender;
    if (now.trim() === was) {
      setEditSender(null);
      return;
    }
    try {
      setSenders(await api.saveMailSender(now, was));
      setEditSender(null);
      onStatus(await api.getMailStatus());
    } catch (e) {
      toast.error(errMsg(e));
    }
  };

  const removeSender = async (domain: string) => {
    try {
      setSenders(await api.deleteMailSender(domain));
      onStatus(await api.getMailStatus());
    } catch (e) {
      toast.error(errMsg(e));
    }
  };

  /* ---- mailbox -------------------------------------------------------- */

  const wrap = async (fn: () => Promise<MailConnectionStatus>, ok: string) => {
    try {
      onStatus(await fn());
      if (ok) toast.success(ok);
    } catch (e) {
      toast.error(errMsg(e));
    }
  };

  const saveCustomQuery = async () => {
    try {
      await api.setMailCustomQuery(customQuery);
      onStatus(await api.getMailStatus());
      toast.success(customQuery.trim() ? "Filter saved" : "Filter cleared");
    } catch (e) {
      toast.error(errMsg(e));
    }
  };

  if (loading) {
    return (
      <Card className="flex flex-1 items-center justify-center p-6">
        <span className="flex items-center gap-2 text-sm text-slate-500 dark:text-slate-400">
          <Spinner className="h-4 w-4" /> Loading settings...
        </span>
      </Card>
    );
  }

  const connected = status.connected;

  return (
    <Card className="flex-1 overflow-y-auto p-0">
      {/* ---- 1. the mailbox --------------------------------------------- */}
      <Section
        title="Mailbox"
        hint="Read-only access. TIQR cannot send, delete or change anything in Gmail - everything below happens inside TIQR."
      >
        <div className="mb-3 flex flex-wrap items-center gap-2">
          <Badge tone={connected ? "sold" : "available"}>{connected ? "Connected" : "Off"}</Badge>
          {connected && status.paused && <Badge tone="available">Paused</Badge>}
          {status.accountEmail && (
            <span className="text-xs text-slate-600 dark:text-slate-300">{status.accountEmail}</span>
          )}
          {status.queueDepth > 0 && (
            <Badge tone="available">{status.queueDepth.toLocaleString()} still to fetch</Badge>
          )}
        </div>

        {!status.oauthAvailable && (
          <p className="mb-3 rounded-lg bg-surface-muted px-3 py-2 text-xs text-amber-600 dark:text-amber-400">
            This build has no Google client embedded, so a mailbox cannot be connected from it. A release build can.
          </p>
        )}
        {connected && !status.gmailScopeGranted && (
          <p className="mb-3 rounded-lg bg-surface-muted px-3 py-2 text-xs text-red-600 dark:text-red-400">
            Google did not grant permission to read mail. Disconnect, connect again, and leave the mail checkbox
            ticked on Google's screen.
          </p>
        )}
        {testResult && (
          <div className="mb-3">
            <MailTestLine result={testResult} />
          </div>
        )}

        <div className="flex flex-wrap gap-2">
          {!connected ? (
            <Button
              variant="primary"
              disabled={!status.oauthAvailable}
              onClick={() => void wrap(() => api.connectMailGmail(), "Mailbox connected")}
            >
              Connect Gmail
            </Button>
          ) : (
            <>
              <Button
                variant="secondary"
                disabled={testing}
                onClick={async () => {
                  setTesting(true);
                  try {
                    const r = await api.testMailConnection();
                    setTestResult(r);
                    if (r.status === "failed") toast.error(r.message);
                  } catch (e) {
                    toast.error(errMsg(e));
                  } finally {
                    setTesting(false);
                  }
                }}
              >
                {testing ? <Spinner className="h-4 w-4" /> : null}
                Test connection
              </Button>
              <Button variant="secondary" onClick={() => void wrap(() => api.setMailPaused(!status.paused), "")}>
                {status.paused ? "Resume sync" : "Pause sync"}
              </Button>
              <Button variant="ghost" onClick={() => void wrap(() => api.disconnectMail(), "Mailbox disconnected")}>
                Disconnect
              </Button>
            </>
          )}
        </div>
      </Section>

      {/* ---- 2. rules ---------------------------------------------------- */}
      <Section
        title="Rules"
        hint="Run top to bottom on new mail. The first rule that matches wins and the rest are skipped - so a Keep above a Move to trash is what makes a broad rule safe to write."
      >
        {rules.length === 0 ? (
          <p className="mb-3 text-xs text-slate-500 dark:text-slate-400">
            No rules yet. Mail arrives exactly as Gmail sends it.
          </p>
        ) : (
          <div className="mb-4 overflow-x-auto">
            <table className="w-full table-fixed text-xs">
              <colgroup>
                <col style={{ width: "44px" }} />
                <col style={{ width: "40%" }} />
                <col style={{ width: "27%" }} />
                <col style={{ width: "72px" }} />
                <col style={{ width: "116px" }} />
              </colgroup>
              <tbody>
                {rules.map((r, i) => (
                  <tr key={r.id} className="border-b border-slate-100 last:border-b-0 dark:border-slate-800/70">
                    <td className="py-2 align-top">
                      <div className="flex flex-col gap-0.5">
                        <button
                          type="button"
                          aria-label="Move up"
                          disabled={i === 0}
                          className="h-4 w-6 rounded text-[10px] leading-none text-slate-400 enabled:hover:bg-surface-muted enabled:hover:text-slate-700 disabled:opacity-25 dark:enabled:hover:text-slate-200"
                          onClick={() => void move(i, -1)}
                        >
                          ▲
                        </button>
                        <button
                          type="button"
                          aria-label="Move down"
                          disabled={i === rules.length - 1}
                          className="h-4 w-6 rounded text-[10px] leading-none text-slate-400 enabled:hover:bg-surface-muted enabled:hover:text-slate-700 disabled:opacity-25 dark:enabled:hover:text-slate-200"
                          onClick={() => void move(i, 1)}
                        >
                          ▼
                        </button>
                      </div>
                    </td>
                    <td className={`py-2 pr-2 align-top ${r.enabled ? "" : "opacity-45"}`}>
                      <span className="text-slate-700 dark:text-slate-200">
                        {FIELD_LABEL[r.field] ?? r.field} {MATCH_LABEL[r.matchKind] ?? r.matchKind}{" "}
                      </span>
                      <span className="break-words font-medium text-slate-900 dark:text-slate-100">
                        &ldquo;{r.needle}&rdquo;
                      </span>
                    </td>
                    <td className={`py-2 pr-2 align-top ${r.enabled ? "" : "opacity-45"}`}>
                      <span
                        className="text-slate-700 dark:text-slate-200"
                        title={ACTION_HINT[r.action] ?? ""}
                      >
                        {ACTION_LABEL[r.action] ?? r.action}
                      </span>
                    </td>
                    <td className="py-2 pr-2 align-top text-right tabular-nums text-slate-400 dark:text-slate-500">
                      {r.hits > 0 ? `${r.hits} hit${r.hits === 1 ? "" : "s"}` : "-"}
                    </td>
                    <td className="py-2 align-top">
                      <div className="flex justify-end gap-1">
                        <button
                          type="button"
                          className="rounded px-1.5 py-0.5 text-[11px] text-slate-500 hover:bg-surface-muted hover:text-slate-800 dark:text-slate-400 dark:hover:text-slate-100"
                          onClick={() => void toggleRule(r)}
                        >
                          {r.enabled ? "Off" : "On"}
                        </button>
                        <button
                          type="button"
                          className="rounded px-1.5 py-0.5 text-[11px] text-brand-600 hover:bg-surface-muted dark:text-brand-400"
                          onClick={() => {
                            setDraft(r);
                            setEditingId(r.id);
                          }}
                        >
                          Edit
                        </button>
                        <button
                          type="button"
                          className="rounded px-1.5 py-0.5 text-[11px] text-red-600 hover:bg-surface-muted dark:text-red-400"
                          onClick={() => void removeRule(r.id)}
                        >
                          Remove
                        </button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}

        {/* the editor */}
        <div className="rounded-lg bg-surface-muted p-3">
          <div className="flex flex-wrap items-center gap-2">
            {/* The width goes on a WRAPPER, not on Select: Select draws
                its own `relative w-full` box around the element, so a width
                class passed in lands on the inner <select> and the box
                still takes the whole flex line. */}
            <div className="w-[132px]">
            <Select
              value={draft.field}
              onChange={(e) => setDraft({ ...draft, field: e.target.value as MailRule["field"] })}
            >
              {Object.entries(FIELD_LABEL).map(([k, v]) => (
                <option key={k} value={k}>
                  {v}
                </option>
              ))}
            </Select>
            </div>
            <div className="w-[124px]">
            <Select
              value={draft.matchKind}
              onChange={(e) => setDraft({ ...draft, matchKind: e.target.value as MailRule["matchKind"] })}
            >
              {Object.entries(MATCH_LABEL).map(([k, v]) => (
                <option key={k} value={k}>
                  {v}
                </option>
              ))}
            </Select>
            <div className="min-w-[180px] flex-1">
              <Input
                placeholder="viagogo.com, order confirmation, payout..."
                value={draft.needle}
                onChange={(e) => setDraft({ ...draft, needle: e.target.value })}
                onKeyDown={(e) => {
                  if (e.key === "Enter") void saveRule();
                }}
              />
            </div>
            </div>
            <span className="text-xs text-slate-500 dark:text-slate-400">then</span>
            <div className="w-[184px]">
            <Select
              value={draft.action}
              onChange={(e) => setDraft({ ...draft, action: e.target.value as MailRule["action"] })}
            >
              {Object.entries(ACTION_LABEL).map(([k, v]) => (
                <option key={k} value={k}>
                  {v}
                </option>
              ))}
            </Select>
            </div>
            <Button variant="primary" disabled={busy} onClick={() => void saveRule()}>
              {editingId ? "Save rule" : "Add rule"}
            </Button>
            {editingId && (
              <Button
                variant="ghost"
                onClick={() => {
                  setDraft(BLANK);
                  setEditingId(null);
                }}
              >
                Cancel
              </Button>
            )}
          </div>
          <p className="mt-2 text-[11px] text-slate-500 dark:text-slate-400">
            {ACTION_HINT[draft.action]} Matching ignores capital letters.
          </p>
        </div>

        <div className="mt-3 flex flex-wrap items-center gap-2">
          <span className="text-[11px] text-slate-500 dark:text-slate-400">Start from:</span>
          {RULE_PRESETS.map((p) => (
            <button
              key={p.label}
              type="button"
              title={p.hint}
              className="rounded-full border border-slate-300 px-2.5 py-1 text-[11px] text-slate-600 hover:bg-surface-muted dark:border-slate-700 dark:text-slate-300"
              onClick={() => {
                setDraft({ ...BLANK, ...p.rule });
                setEditingId(null);
              }}
            >
              {p.label}
            </button>
          ))}
        </div>

        {rules.length > 0 && (
          <div className="mt-4 flex flex-wrap items-center gap-2">
            <Button variant="secondary" disabled={applying} onClick={() => void runNow()}>
              {applying ? <Spinner className="h-4 w-4" /> : null}
              Run rules on mail I already have
            </Button>
            <span className="text-[11px] text-slate-500 dark:text-slate-400">
              Otherwise rules only apply to mail that has not arrived yet.
            </span>
          </div>
        )}
      </Section>

      {/* ---- 3. senders --------------------------------------------------- */}
      <Section
        title="Senders TIQR watches"
        hint={
          status.scope === "known"
            ? "With 'marketplace senders only' selected below, these are the only senders TIQR asks Gmail for."
            : "TIQR is fetching the whole mailbox, so this list is not filtering anything right now. It still matters if you switch to marketplace senders only."
        }
      >
        <div className="mb-3 flex flex-wrap gap-1.5">
          {senders.map((s) =>
            editSender?.was === s.domain ? (
              <span key={s.domain} className="inline-flex items-center gap-1">
                <input
                  autoFocus
                  className="w-44 rounded-md border border-brand-400 bg-surface px-2 py-1 font-mono text-[11px] text-slate-800 outline-none dark:text-slate-100"
                  value={editSender.now}
                  onChange={(e) => setEditSender({ was: s.domain, now: e.target.value })}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") void commitSenderEdit();
                    if (e.key === "Escape") setEditSender(null);
                  }}
                  onBlur={() => void commitSenderEdit()}
                />
              </span>
            ) : (
              <span
                key={s.domain}
                className="group inline-flex items-center gap-1 rounded-md bg-surface-muted py-1 pl-2 pr-1 font-mono text-[11px] text-slate-600 dark:text-slate-300"
              >
                <button
                  type="button"
                  title="Click to edit"
                  className="hover:text-slate-900 dark:hover:text-slate-50"
                  onClick={() => setEditSender({ was: s.domain, now: s.domain })}
                >
                  {s.domain}
                </button>
                <button
                  type="button"
                  aria-label={`Remove ${s.domain}`}
                  className="rounded px-1 text-slate-400 hover:bg-red-500/10 hover:text-red-500"
                  onClick={() => void removeSender(s.domain)}
                >
                  ×
                </button>
              </span>
            ),
          )}
          {senders.length === 0 && (
            <p className="text-xs text-slate-500 dark:text-slate-400">
              Empty. With no senders listed, &ldquo;marketplace senders only&rdquo; fetches the whole mailbox
              instead - it will not quietly fall back to the built-in list.
            </p>
          )}
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <div className="w-60">
            <Input
              placeholder="oeticket.com"
              value={newSender}
              onChange={(e) => setNewSender(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") void addSender();
              }}
            />
          </div>
          <Button variant="secondary" onClick={() => void addSender()}>
            Add sender
          </Button>
        </div>

        <div className="mt-4 w-72">
          <label className="mb-1 block text-xs font-medium text-slate-600 dark:text-slate-300">
            What to fetch
          </label>
          <Select
            value={status.scope}
            onChange={(e) =>
              void wrap(() => api.setMailScope(e.target.value as "all" | "known"), "Saved")
            }
          >
            <option value="all">The whole mailbox</option>
            <option value="known">Marketplace senders only</option>
          </Select>
        </div>
      </Section>

      {/* ---- 4. the Gmail filter ------------------------------------------ */}
      <Section
        title="Your own Gmail filter"
        hint="Gmail's own search syntax, added to every sync. Narrows what arrives - it can never widen it past the date window."
      >
        <div className="flex flex-wrap items-center gap-2">
          <div className="min-w-[260px] flex-1">
            <Input
              placeholder="-category:promotions"
              value={customQuery}
              onChange={(e) => setCustomQuery(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") void saveCustomQuery();
              }}
            />
          </div>
          <Button variant="secondary" onClick={() => void saveCustomQuery()}>
            Save filter
          </Button>
        </div>
        <p className="mt-2 text-[11px] text-slate-500 dark:text-slate-400">
          For example <code>-category:promotions</code>, <code>has:attachment</code>,{" "}
          <code>label:PULLS</code>, <code>-from:noreply@x.com</code>. Unbalanced brackets or quotes are
          refused rather than half-applied.
        </p>
        <p className="mt-1 text-[11px] text-slate-500 dark:text-slate-400">
          Starred mail ignores this filter completely, and has no date limit either - starred always arrives,
          however old it is.
        </p>
      </Section>

      {/* ---- 5. automation ------------------------------------------------ */}
      <Section
        title="What TIQR may create by itself"
        hint="Manual is the safe one: TIQR prepares the record and waits for your button. Auto only ever fires when every detail matched exactly - never on a guess."
      >
        <div className="flex flex-wrap gap-3">
          {(
            [
              ["order", "Orders", status.autoOrder],
              ["sale", "Sales", status.autoSale],
              ["finance", "Finance", status.autoFinance],
            ] as const
          ).map(([kind, label, value]) => (
            <div key={kind} className="w-[180px]">
              <label className="mb-1 block text-xs font-medium text-slate-600 dark:text-slate-300">
                {label}
              </label>
              <Select
                value={value}
                onChange={(e) =>
                  void wrap(
                    () =>
                      api.setMailAutomation(
                        kind,
                        e.target.value as "off" | "manual" | "auto",
                      ),
                    "Saved",
                  )
                }
              >
                <option value="off">Off</option>
                <option value="manual">Manual - ask me</option>
                <option value="auto">Automatic</option>
              </Select>
            </div>
          ))}
        </div>
      </Section>

      {/* ---- 6. fetching -------------------------------------------------- */}
      <Section title="Fetching and keeping">
        <div className="flex flex-wrap gap-3">
          <div className="w-[200px]">
            <label className="mb-1 block text-xs font-medium text-slate-600 dark:text-slate-300">
              Check for new mail
            </label>
            <Select
              value={String(status.syncIntervalSeconds)}
              onChange={(e) =>
                void wrap(() => api.setMailSyncInterval(Number(e.target.value)), "Saved")
              }
            >
              <option value="0">Only when I press Sync</option>
              <option value="30">Every 30 seconds</option>
              <option value="60">Every minute</option>
              <option value="300">Every 5 minutes</option>
              <option value="900">Every 15 minutes</option>
            </Select>
          </div>
          <div className="w-[200px]">
            <label className="mb-1 block text-xs font-medium text-slate-600 dark:text-slate-300">
              Keep mail for (days)
            </label>
            <div className="flex gap-2">
              <Input
                value={retention}
                onChange={(e) => setRetention(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key !== "Enter") return;
                  const days = Number(retention);
                  if (!Number.isFinite(days) || days < 1) {
                    toast.error("Keep for at least 1 day.");
                    return;
                  }
                  void wrap(() => api.setMailRetentionDays(Math.round(days)), "Saved");
                }}
              />
            </div>
          </div>
        </div>
        <p className="mt-2 text-[11px] text-slate-500 dark:text-slate-400">
          TIQR has no background service: it only syncs while the app is open. Nothing is ever deleted from
          Gmail - this only clears TIQR's own copy.
        </p>
      </Section>
    </Card>
  );
}

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
//
// 2.113.0: "settings len v maily a to tiez rozdelil ako je to v primarnych
// settings" - so the one long page became sections down the left, the way
// Settings itself is laid out: Mailbox, Reading, What is read, Rules and
// Automatic. A rule can now also file mail in a folder.
//
// 2.114.0: "viac veci pridat do settings aby si clovek presne vedel nastavit
// ako ten mail ma fungovat, co a ako prijat, co posielat dalej, ako posielat,
// rules lepsie, mute". Which folders are read at all; what mail goes on to
// Discord and what its message carries; rules that look anywhere, star, and
// say how many messages they would catch before they are saved; and the
// muted senders, added and unmuted here.
import { useCallback, useEffect, useState } from "react";
import { api, errMsg } from "../lib/api";
import { useDataChanged } from "../lib/dataChanged";
import { requestMailSync } from "../lib/mailSync";
import { nudgeMailShare, useMailShareView } from "../lib/mailShare";
import { shareLine } from "../lib/mailShareCore";
import { lastSyncText, laneViews, syncHeadline, storedVsGmail } from "../lib/mailSyncStatus";
import { MAIL_LANGUAGES } from "../lib/mailText";
import { MUTE_SPANS, muteKind, muteState, type MuteKind } from "../lib/mailMute";
import {
  type DiscordMailConfig,
  type MailConnectionStatus,
  type MailLabel,
  type MailLocalFolder,
  type MailMute,
  type MailRule,
  type MailSender,
  type MailTestResult,
  type NotificationStatus,
} from "../lib/types";
import { Badge, Button, Card, CHECKBOX_CLASS, Input, Select, Spinner } from "./ui";
import { IconBellOff, IconGauge, IconMail, IconRefresh, IconSend, IconTag, IconUsers } from "./icons";
import { useToast } from "../lib/toast";

/* What each rule action does, in marko's terms rather than the column it
   writes. The screen reads these out next to the rule, so a rule list is
   never a list of verbs you have to remember the meaning of. */
const ACTION_LABEL: Record<string, string> = {
  trash: "Move to trash",
  read: "Mark as read",
  review: "Send to Needs review",
  move: "Move to a folder",
  star: "Star it",
  keep: "Keep - stop here",
};

const ACTION_HINT: Record<string, string> = {
  trash: "Goes to TIQR's trash. Still restorable; Gmail is untouched.",
  read: "Stays where it is, just stops counting as unread.",
  review: "Shows up in Needs review so you see it first.",
  move: "Files it in TIQR - it shows in that folder instead of the Inbox. Gmail is untouched.",
  star: "Puts a star on it in TIQR, so it is in Starred.",
  keep: "Protects it: no rule below this one can act on it.",
};

/** A rule's action is its kind - `move:<place>` is the kind "move". */
const actionKind = (a: string) => (a.startsWith("move:") ? "move" : a);

/* 2.113.0: the sections, down the left like Settings itself. */
type SectionKey = "mailbox" | "reading" | "senders" | "rules" | "muted" | "discord" | "automatic";
const SECTIONS: { key: SectionKey; title: string; description: string; icon: typeof IconMail }[] = [
  { key: "mailbox", title: "Mailbox", description: "Connection and progress", icon: IconMail },
  { key: "reading", title: "Reading", description: "How often, pictures, translate", icon: IconRefresh },
  { key: "senders", title: "What is read", description: "Folders, senders, your filter", icon: IconUsers },
  { key: "rules", title: "Rules", description: "Sort new mail by itself", icon: IconTag },
  { key: "muted", title: "Muted", description: "Senders and subjects, hidden", icon: IconBellOff },
  { key: "discord", title: "Discord", description: "What goes on, and how", icon: IconSend },
  { key: "automatic", title: "Automatic", description: "Records made from mail", icon: IconGauge },
];

/* 2.114.0: the folders that can be left out of reading altogether. */
const SKIPPABLE: { key: "promotions" | "spam" | "trash"; label: string; hint: string }[] = [
  { key: "promotions", label: "Promotions", hint: "Gmail's Promotions tab" },
  { key: "spam", label: "Spam", hint: "Never trusted anyway" },
  { key: "trash", label: "Trash", hint: "Gmail's own trash" },
];

/* 2.114.0: which mail goes on to Discord by itself, and what it carries. */
const MAIL_KINDS: { key: keyof DiscordMailConfig; label: string }[] = [
  { key: "sales", label: "Sales" },
  { key: "purchases", label: "Purchases" },
  { key: "money", label: "Payouts and refunds" },
  { key: "changes", label: "Cancellations and event changes" },
  { key: "orderUpdates", label: "Order updates - tickets ready, transfers" },
];
const MAIL_PARTS: { key: keyof DiscordMailConfig; label: string; hint: string }[] = [
  { key: "keyDetails", label: "Key details", hint: "Where the tickets go, by when, the event, the other prices." },
  {
    key: "personal",
    label: "Personal details",
    hint: "The buyer's name and e-mail - always as a spoiler. Off: never sent to Discord at all.",
  },
  { key: "preview", label: "The opening words", hint: "Only when nothing better is known about the mail." },
];

/* Where the pane opens: the section open last, for this session. */
let lastSection: SectionKey = "mailbox";

const FIELD_LABEL: Record<string, string> = {
  from: "Sender",
  subject: "Subject",
  preview: "Preview text",
  any: "Sender, subject or preview",
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

/** Gmail syntax in running text - set apart, or it reads as prose. */
function Code({ children }: { children: React.ReactNode }) {
  return (
    <code className="rounded bg-surface-muted px-1 py-px font-mono text-[10.5px] text-slate-700 dark:text-slate-200">
      {children}
    </code>
  );
}

function bytes(n: number) {
  if (n < 1024 * 1024) return `${Math.max(1, Math.round(n / 1024))} KB`;
  if (n < 1024 * 1024 * 1024) return `${(n / (1024 * 1024)).toFixed(1)} MB`;
  return `${(n / (1024 * 1024 * 1024)).toFixed(2)} GB`;
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

const MAIL_GROUP = ["settings:mail"] as const;

export function MailSettingsPane({
  status,
  onStatus,
  localFolders = [],
  labels = [],
}: {
  status: MailConnectionStatus;
  onStatus: (s: MailConnectionStatus) => void;
  /** 2.113.0: where a rule can move mail - marko's own folders and labels. */
  localFolders?: MailLocalFolder[];
  labels?: MailLabel[];
}) {
  const toast = useToast();
  const share = useMailShareView();
  const [section, setSection] = useState<SectionKey>(lastSection);
  const choose = (k: SectionKey) => {
    lastSection = k;
    setSection(k);
  };
  const places = [
    { key: "inbox", label: "Inbox" },
    { key: "finances", label: "Finances" },
    { key: "promotions", label: "Promotions" },
    ...labels.map((l) => ({ key: l.id, label: l.name })),
    ...localFolders.map((f) => ({ key: f.id, label: f.name })),
  ];
  const actionLabel = (a: string) =>
    a.startsWith("move:")
      ? `Move to ${places.find((p) => p.key === a.slice(5))?.label ?? "a folder that is gone"}`
      : (ACTION_LABEL[a] ?? a);

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
  /* 2.114.0 */
  const [notif, setNotif] = useState<NotificationStatus | null>(null);
  const [mutes, setMutes] = useState<MailMute[]>([]);
  const [muteWho, setMuteWho] = useState("");
  /** 2.115.0: an address or domain, or words in the subject. */
  const [muteBy, setMuteBy] = useState<MuteKind>("sender");
  const [muteDays, setMuteDays] = useState<string>("7");
  const [ruleMatches, setRuleMatches] = useState<number | null>(null);

  const load = useCallback(async () => {
    try {
      const [r, s, n, m] = await Promise.all([
        api.listMailRules(),
        api.listMailSenders(),
        api.getNotificationStatus(),
        api.listMailMutes(),
      ]);
      setRules(r);
      setSenders(s);
      setNotif(n);
      setMutes(m);
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

  // 2.112.0: rules and senders changed on the other computer arrive live.
  useDataChanged(() => void load(), MAIL_GROUP);

  /* 2.114.0: how many messages already here the rule in the editor would
     act on - while it is written, so a rule is never a guess. */
  useEffect(() => {
    if (!draft.needle.trim()) {
      setRuleMatches(null);
      return;
    }
    const t = window.setTimeout(() => {
      api
        .countMailRuleMatches(draft)
        .then(setRuleMatches)
        .catch(() => setRuleMatches(null));
    }, 300);
    return () => window.clearTimeout(t);
  }, [draft]);

  /* ---- what is read, Discord, muted (2.114.0) ------------------------- */

  const toggleSkip = (key: "promotions" | "spam" | "trash", read: boolean) => {
    const skip = new Set(status.skipFolders ?? []);
    if (read) skip.delete(key);
    else skip.add(key);
    void wrap(() => api.setMailSkipFolders([...skip]), read ? "TIQR reads it again" : "TIQR no longer reads it", true);
  };

  const saveDiscord = async (input: Parameters<typeof api.setNotificationConfig>[0]) => {
    try {
      setNotif(await api.setNotificationConfig(input));
      toast.success("Saved");
    } catch (e) {
      toast.error(errMsg(e));
    }
  };

  const addMute = async () => {
    const who = muteWho.trim();
    if (!who) return;
    const days = muteDays === "always" ? null : Number(muteDays);
    try {
      setMutes(await (muteBy === "subject" ? api.muteMailSubject(who, days) : api.muteMailSender(who, days)));
      setMuteWho("");
      toast.success(muteBy === "subject" ? `Mail with "${who}" in the subject muted - it is under Muted.` : `${who} muted - its mail is under Muted.`);
    } catch (e) {
      toast.error(errMsg(e));
    }
  };

  const unmute = async (kind: MuteKind, who: string) => {
    try {
      setMutes(await (kind === "subject" ? api.unmuteMailSubject(who) : api.unmuteMailSender(who)));
      toast.success(kind === "subject" ? `"${who}" unmuted` : `${who} unmuted`);
    } catch (e) {
      toast.error(errMsg(e));
    }
  };

  /* ---- rules --------------------------------------------------------- */

  const saveRule = async () => {
    if (!draft.needle.trim()) {
      toast.error("Type what the rule should look for.");
      return;
    }
    if (draft.action === "move:") {
      toast.error("Pick the folder the rule moves mail to.");
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
      requestMailSync();
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

  /* `read` = this changes what is read or whether it runs, so the reader
     (Layout.tsx) is asked to go now rather than at its next turn. */
  const wrap = async (fn: () => Promise<MailConnectionStatus>, ok: string, read = false) => {
    try {
      const next = await fn();
      onStatus(next);
      if (ok) toast.success(ok);
      if (read && next.connected && !next.paused) requestMailSync();
    } catch (e) {
      toast.error(errMsg(e));
    }
  };

  const saveRetention = () => {
    const days = Number(retention);
    if (!Number.isFinite(days) || days < 1 || days > 3650) {
      toast.error("Between 1 and 3650 days.");
      return;
    }
    void wrap(() => api.setMailRetentionDays(Math.round(days)), "Saved");
  };

  const saveCustomQuery = async () => {
    try {
      await api.setMailCustomQuery(customQuery);
      onStatus(await api.getMailStatus());
      toast.success(customQuery.trim() ? "Filter saved" : "Filter cleared");
      requestMailSync();
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
    <Card className="flex min-w-0 flex-1 overflow-hidden p-0">
      {/* The sections, down the left - the way Settings itself is laid out. */}
      <nav className="flex w-48 shrink-0 flex-col gap-0.5 overflow-y-auto border-r border-slate-200 p-2 dark:border-slate-800">
        <p className="section-title px-2.5 pb-1.5 pt-1">Mail settings</p>
        {SECTIONS.map((sec) => {
          const on = sec.key === section;
          return (
            <button
              key={sec.key}
              type="button"
              aria-current={on ? "page" : undefined}
              onClick={() => choose(sec.key)}
              className={`flex items-start gap-2 rounded-lg px-2.5 py-2 text-left transition ${
                on
                  ? "bg-brand-500/[0.12] text-brand-700 dark:text-brand-300"
                  : "text-slate-600 hover:bg-surface-muted dark:text-slate-300"
              }`}
            >
              <sec.icon className="mt-0.5 h-4 w-4 shrink-0" />
              <span className="min-w-0">
                <span className={`block text-[12.5px] ${on ? "font-semibold" : "font-medium"}`}>{sec.title}</span>
                <span className="block text-[10.5px] leading-snug text-slate-400 dark:text-slate-500">{sec.description}</span>
              </span>
            </button>
          );
        })}
      </nav>

      <div className="min-w-0 flex-1 overflow-y-auto">
        {section === "mailbox" && (
          <>
      {/* ---- 1. the mailbox --------------------------------------------- */}
      <Section
        title="Mailbox"
        hint="Read-only access. TIQR cannot send, delete or change anything in Gmail - everything below happens inside TIQR. The connection, rules, senders and filter are the same on your other computer through cloud sync."
      >
        <div className="mb-3 flex flex-wrap items-center gap-2">
          <Badge tone={connected ? "sold" : "available"}>{connected ? "Connected" : "Off"}</Badge>
          {connected && status.paused && <Badge tone="available">Paused</Badge>}
          {status.accountEmail && (
            <span className="text-xs text-slate-600 dark:text-slate-300">{status.accountEmail}</span>
          )}
        </div>

        {/* Where the reader is, in the same words as the Mail screen
            (lib/mailSyncStatus.ts). A long first read looks exactly like a
            stuck one unless this says which lane and how much is left. */}
        {connected && (
          <div className="mb-3 rounded-lg bg-surface-muted px-3 py-2 text-xs text-slate-600 dark:text-slate-300">
            <p className="font-medium text-slate-800 dark:text-slate-100">
              {/* While the other computer reads for this one, its own lanes
                  stand still - what they would say is not what happens. */}
              {share.role === "follower"
                ? shareLine(share, (n) => n.toLocaleString())
                : syncHeadline({
                    progress: status.progress,
                    paused: status.paused,
                    failed: false,
                    rateLimited: false,
                    intervalSeconds: status.syncIntervalSeconds,
                  })}
            </p>
            {share.role !== "follower" && (
              <ul className="mt-1 grid grid-cols-[auto_1fr] gap-x-3 gap-y-0.5 text-[11px]">
                {laneViews(status.progress, (n) => n.toLocaleString()).map((l) => (
                  <li key={l.key} className="contents" title={l.hint}>
                    <span className="text-slate-500 dark:text-slate-400">{l.label}</span>
                    <span className="tabular-nums">{l.text}</span>
                  </li>
                ))}
              </ul>
            )}
            <p className="mt-1 text-[11px] text-slate-500 dark:text-slate-400">
              {storedVsGmail(status.progress, (n) => n.toLocaleString())} &middot; {bytes(status.storeBytes)}
              {/* This computer's own last read of Gmail - not news while the other one reads. */}
              {share.role !== "follower" && (
                <> &middot; {lastSyncText(status.progress.lastSuccessAt ?? status.lastSyncAt, new Date())}</>
              )}
            </p>
          </div>
        )}

        {/* 2.115.0, marko: "ak naraz tak len jeden a ten to bude posielat
            tomu druhemu aby to bolo obmedzene" - two computers reading one
            mailbox at once is what made Google slow TIQR down. */}
        {connected && (
          <div className="mb-3 rounded-lg bg-surface-muted px-3 py-2">
            <label className="flex items-start gap-2 text-xs text-slate-700 dark:text-slate-200">
              <input
                type="checkbox"
                className={`${CHECKBOX_CLASS} mt-0.5`}
                checked={status.shareReading}
                onChange={(e) => {
                  const on = e.target.checked;
                  void wrap(async () => {
                    const next = await api.setMailShare(on);
                    nudgeMailShare();
                    return next;
                  }, on ? "Your computers share the reading" : "Each computer reads Gmail itself");
                }}
              />
              <span>
                <span className="font-medium">Two computers, one reader</span>
                <span className="mt-0.5 block text-[11px] text-slate-500 dark:text-slate-400">
                  While TIQR runs on both computers, only one reads Gmail and sends the other what it stored - so
                  Google has no reason to slow TIQR down. The mail goes through your TIQR account&apos;s Firebase and
                  is deleted there as soon as the other computer has stored it. A computer running alone reads Gmail
                  itself. Uses live sync (Settings &rarr; Data); this switch is the same on both computers.
                </span>
              </span>
            </label>
            {/* While following, the line above already says it. */}
            {share.role !== "follower" && (
              <p
                className={`mt-1.5 text-[11px] font-medium ${
                  share.warn ? "text-amber-600 dark:text-amber-400" : "text-slate-700 dark:text-slate-200"
                }`}
              >
                {shareLine(share, (n) => n.toLocaleString())}
              </p>
            )}
          </div>
        )}

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
              onClick={() => void wrap(() => api.connectMailGmail(), "Mailbox connected", true)}
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
              <Button
                variant="secondary"
                onClick={() =>
                  void wrap(async () => {
                    const next = await api.setMailPaused(!status.paused);
                    nudgeMailShare();
                    return next;
                  }, "", true)
                }
              >
                {status.paused ? "Resume sync" : "Pause sync"}
              </Button>
              <Button variant="ghost" onClick={() => void wrap(() => api.disconnectMail(), "Mailbox disconnected")}>
                Disconnect
              </Button>
            </>
          )}
        </div>
      </Section>

          </>
        )}

        {section === "reading" && (
          <>
      {/* ---- 6. fetching -------------------------------------------------- */}
      <Section
        title="Reading and keeping"
        hint="TIQR reads every folder by itself, newest to oldest - Important, Starred, your labels and Finances first, then everything else. Once everything is read it checks for new mail on the interval below."
      >
        <div className="flex flex-wrap items-end gap-3">
          <div className="w-[220px]">
            <label className="mb-1 block text-xs font-medium text-slate-600 dark:text-slate-300">
              Check for new mail
            </label>
            <Select
              value={String(status.syncIntervalSeconds)}
              onChange={(e) =>
                void wrap(() => api.setMailSyncInterval(Number(e.target.value)), "Saved", true)
              }
            >
              <option value="0">Only when I press Sync</option>
              <option value="30">Every 30 seconds</option>
              <option value="60">Every minute</option>
              <option value="300">Every 5 minutes</option>
              <option value="900">Every 15 minutes</option>
            </Select>
          </div>
          <div className="w-[260px]">
            <label className="mb-1 block text-xs font-medium text-slate-600 dark:text-slate-300">
              Keep preview text for (days)
            </label>
            <div className="flex gap-2">
              <div className="w-24">
                <Input
                  inputMode="numeric"
                  value={retention}
                  onChange={(e) => setRetention(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") saveRetention();
                  }}
                />
              </div>
              <Button variant="secondary" onClick={saveRetention}>
                Save
              </Button>
            </div>
          </div>
        </div>
        <p className="mt-3 text-[11px] leading-relaxed text-slate-500 dark:text-slate-400">
          Messages are never removed: older ones keep their sender, subject, date and labels, and only the
          short preview is cleared, to save space. A message can only leave TIQR through Delete forever in
          Trash - and nothing here ever touches Gmail. TIQR reads only while the app is open.
        </p>
      </Section>

      <Section
        title="Showing a message"
        hint="An opened message is cleaned before it is shown - nothing in it can run. Its pictures are copied into TIQR when it is first opened, by TIQR itself; after that they come from TIQR's copy, so a sender can see that first download at most, never each time you read it."
      >
        <label className="flex items-start gap-2 text-xs text-slate-700 dark:text-slate-200">
          <input
            type="checkbox"
            className={`${CHECKBOX_CLASS} mt-0.5`}
            checked={status.remoteImages}
            onChange={(e) => void wrap(() => api.setMailReaderOptions({ remoteImages: e.target.checked }), "Saved")}
          />
          <span>
            <span className="font-medium">Copy pictures from the web into TIQR</span>
            <span className="mt-0.5 block text-[11px] text-slate-500 dark:text-slate-400">
              Pictures inside the message itself are always shown. Off: nothing is fetched from the web, and a logo or
              banner kept there stays out. Tracking pixels are always left out.
            </span>
          </span>
        </label>
        <div className="mt-4 w-[220px]">
          <label className="mb-1 block text-xs font-medium text-slate-600 dark:text-slate-300">Translate into</label>
          <Select
            value={status.translateTo}
            onChange={(e) => void wrap(() => api.setMailReaderOptions({ translateTo: e.target.value }), "Saved")}
          >
            {MAIL_LANGUAGES.map((l) => (
              <option key={l.code} value={l.code}>
                {l.name}
              </option>
            ))}
          </Select>
        </div>
      </Section>

      <Section
        title="AI helper"
        hint="Translate and Summary in an opened message. Only on your click, never by itself - on the cheapest model, with long emails cut short, and every answer kept with the message so the same click never costs twice."
      >
        <p className="text-xs text-slate-700 dark:text-slate-200">
          This month on this computer:{" "}
          <span className="font-semibold tabular-nums">{status.aiCallsThisMonth.toLocaleString()}</span>{" "}
          answer{status.aiCallsThisMonth === 1 ? "" : "s"},{" "}
          <span className="tabular-nums">{status.aiTokensThisMonth.toLocaleString()}</span> tokens.
        </p>
        <p className="mt-1 text-[11px] text-slate-500 dark:text-slate-400">
          It uses your Anthropic key from Settings &rarr; Integrations &rarr; AI. Nothing in Mail uses AI unless you press
          Translate or Summary.
        </p>
      </Section>
          </>
        )}

        {section === "senders" && (
          <>
      {/* ---- 2.114.0: folders read at all ------------------------------- */}
      <Section
        title="Folders TIQR reads"
        hint="Untick a folder and TIQR stops reading it: nothing new from it is downloaded. What is already in TIQR stays, and a message you starred or put in one of your labels is still read."
      >
        <div className="flex flex-wrap gap-x-6 gap-y-2">
          {SKIPPABLE.map((f) => {
            const read = !(status.skipFolders ?? []).includes(f.key);
            return (
              <label key={f.key} className="flex items-start gap-2 text-xs text-slate-700 dark:text-slate-200">
                <input
                  type="checkbox"
                  className={`${CHECKBOX_CLASS} mt-0.5`}
                  checked={read}
                  onChange={(e) => toggleSkip(f.key, e.target.checked)}
                />
                <span>
                  <span className="font-medium">{f.label}</span>
                  <span className="block text-[11px] text-slate-500 dark:text-slate-400">{f.hint}</span>
                </span>
              </label>
            );
          })}
        </div>
      </Section>

      {/* ---- 3. senders --------------------------------------------------- */}
      <Section
        title="Senders TIQR watches"
        hint={
          status.scope === "known"
            ? "TIQR reads these senders first (the Important folder) - and with 'The mail that matters' selected below, nothing else besides Starred, your labels and Finances."
            : "TIQR reads these senders first, as the Important folder, and only then the rest of the mailbox."
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
              Empty. With no senders listed there is no Important walk, and &ldquo;The mail that matters&rdquo;
              reads the whole mailbox instead - it will not quietly fall back to the built-in list.
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
              void wrap(() => api.setMailScope(e.target.value as "all" | "known"), "Saved", true)
            }
          >
            <option value="all">The whole mailbox</option>
            <option value="known">The mail that matters</option>
          </Select>
        </div>
      </Section>

      {/* ---- 4. the Gmail filter ------------------------------------------ */}
      <Section
        title="Your own Gmail filter"
        hint="Gmail's own search syntax. It narrows the Inbox reading only - Important, Starred, your labels and Finances are never filtered by it."
      >
        <div className="flex flex-wrap items-center gap-2">
          <div className="min-w-[260px] flex-1">
            <Input
              placeholder="-from:noreply@x.com"
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
          For example <Code>-from:noreply@x.com</Code>, <Code>has:attachment</Code>,{" "}
          <Code>-category:social</Code>. Unbalanced brackets or quotes are refused rather than half-applied.
          Changing it reads the Inbox again from the newest message.
        </p>
      </Section>

          </>
        )}

        {section === "rules" && (
          <>
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
          /* One line per rule, read left to right as a sentence -
             "Subject contains 'payout' -> Send to Needs review". The fixed
             table columns this replaced left a gap the width of the pane
             between a condition and what it does. */
          <ul className="mb-4 divide-y divide-slate-100 rounded-lg border border-slate-200 dark:divide-slate-800/70 dark:border-slate-800">
            {rules.map((r, i) => (
              <li key={r.id} className="flex items-center gap-3 px-2 py-2 text-xs">
                <div className="flex shrink-0 flex-col">
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
                <p className={`min-w-0 flex-1 leading-relaxed ${r.enabled ? "" : "opacity-45"}`}>
                  <span className="text-slate-500 dark:text-slate-400">
                    {FIELD_LABEL[r.field] ?? r.field} {MATCH_LABEL[r.matchKind] ?? r.matchKind}{" "}
                  </span>
                  <span className="break-all font-medium text-slate-900 dark:text-slate-100">&ldquo;{r.needle}&rdquo;</span>
                  <span className="mx-1.5 text-slate-400">&rarr;</span>
                  <span className="whitespace-nowrap text-slate-700 dark:text-slate-200" title={ACTION_HINT[actionKind(r.action)] ?? ""}>
                    {actionLabel(r.action)}
                  </span>
                  {!r.enabled && <span className="ml-1.5 text-[11px] text-slate-400">(off)</span>}
                </p>
                <span className="w-14 shrink-0 text-right tabular-nums text-[11px] text-slate-400 dark:text-slate-500">
                  {r.hits > 0 ? `${r.hits} hit${r.hits === 1 ? "" : "s"}` : "no hits"}
                </span>
                <div className="flex shrink-0 gap-1">
                  <button
                    type="button"
                    className="rounded px-1.5 py-0.5 text-[11px] text-slate-500 hover:bg-surface-muted hover:text-slate-800 dark:text-slate-400 dark:hover:text-slate-100"
                    onClick={() => void toggleRule(r)}
                  >
                    {r.enabled ? "Turn off" : "Turn on"}
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
              </li>
            ))}
          </ul>
        )}

        {/* the editor */}
        <div className="rounded-lg bg-surface-muted p-3">
          <div className="flex flex-wrap items-center gap-2">
            {/* The width goes on a WRAPPER, not on Select: Select draws
                its own `relative w-full` box around the element, so a width
                class passed in lands on the inner <select> and the box
                still takes the whole flex line. */}
            <div className="w-[132px] shrink-0">
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
            <div className="w-[124px] shrink-0">
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
            </div>
            {/* 2.110.0: this field sat INSIDE the 124px wrapper above, so
                the text a rule looks for got squeezed under the select
                instead of taking the rest of the line. */}
            <div className="min-w-[200px] flex-1">
              <Input
                placeholder="viagogo.com, order confirmation, payout..."
                value={draft.needle}
                onChange={(e) => setDraft({ ...draft, needle: e.target.value })}
                onKeyDown={(e) => {
                  if (e.key === "Enter") void saveRule();
                }}
              />
            </div>
            <span className="text-xs text-slate-500 dark:text-slate-400">then</span>
            <div className="w-[208px] shrink-0">
              <Select
                value={actionKind(draft.action)}
                onChange={(e) =>
                  setDraft({ ...draft, action: (e.target.value === "move" ? "move:" : e.target.value) as MailRule["action"] })
                }
              >
                {Object.entries(ACTION_LABEL).map(([k, v]) => (
                  <option key={k} value={k}>
                    {v}
                  </option>
                ))}
              </Select>
            </div>
            {/* 2.113.0: a rule can file mail - in TIQR's own places, one of
                the Gmail labels or one of marko's own folders. */}
            {actionKind(draft.action) === "move" && (
              <div className="w-[180px] shrink-0">
                <Select
                  value={draft.action.slice(5)}
                  onChange={(e) => setDraft({ ...draft, action: `move:${e.target.value}` })}
                >
                  <option value="">Pick a folder...</option>
                  {places.map((p) => (
                    <option key={p.key} value={p.key}>
                      {p.label}
                    </option>
                  ))}
                </Select>
              </div>
            )}
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
            {ACTION_HINT[actionKind(draft.action)]} Matching ignores capital letters.
            {ruleMatches !== null && (
              <span className="ml-1 font-medium text-slate-700 dark:text-slate-200">
                {ruleMatches === 0
                  ? "It matches none of the mail you already have."
                  : `It matches ${ruleMatches.toLocaleString()} message${ruleMatches === 1 ? "" : "s"} you already have.`}
              </span>
            )}
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

          </>
        )}

        {section === "muted" && (
          <Section
            title="Muted"
            hint="A muted sender's mail, or mail with muted words in its subject, still arrives and is kept - hidden under Muted and not counted as unread. Orders, sales and payouts always show. A whole shared mailbox (gmail.com, azet.sk...) cannot be muted, only one address at it."
          >
            {mutes.length === 0 ? (
              <p className="mb-3 text-xs text-slate-500 dark:text-slate-400">Nothing is muted.</p>
            ) : (
              <ul className="mb-4 divide-y divide-slate-100 rounded-lg border border-slate-200 dark:divide-slate-800/70 dark:border-slate-800">
                {mutes.map((m) => (
                  <li key={`${muteKind(m)}:${m.who}`} className="flex items-center gap-3 px-3 py-2 text-xs">
                    <span className="w-14 shrink-0 text-[10.5px] uppercase tracking-wide text-slate-400 dark:text-slate-500">
                      {muteKind(m) === "subject" ? "Subject" : "Sender"}
                    </span>
                    <span className="min-w-0 flex-1 truncate" title={m.who}>
                      {muteKind(m) === "subject" ? (
                        <span className="text-slate-800 dark:text-slate-100">&ldquo;{m.who}&rdquo;</span>
                      ) : (
                        <span className="font-mono text-slate-800 dark:text-slate-100">{m.who}</span>
                      )}{" "}
                      <span className="text-slate-500 dark:text-slate-400">{muteState(m)}</span>
                    </span>
                    <button
                      type="button"
                      className="shrink-0 rounded px-1.5 py-0.5 text-[11px] text-brand-600 hover:bg-surface-muted dark:text-brand-400"
                      onClick={() => void unmute(muteKind(m), m.who)}
                    >
                      Unmute
                    </button>
                  </li>
                ))}
              </ul>
            )}
            <div className="flex flex-wrap items-center gap-2">
              <div className="w-44">
                <Select value={muteBy} onChange={(e) => setMuteBy(e.target.value as MuteKind)} aria-label="Mute by">
                  <option value="sender">Address or domain</option>
                  <option value="subject">Words in the subject</option>
                </Select>
              </div>
              <div className="w-64">
                <Input
                  placeholder={muteBy === "subject" ? "weekly digest" : "news@shop.com or shop.com"}
                  value={muteWho}
                  onChange={(e) => setMuteWho(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") void addMute();
                  }}
                />
              </div>
              <div className="w-36">
                <Select value={muteDays} onChange={(e) => setMuteDays(e.target.value)}>
                  {MUTE_SPANS.map((sp) => (
                    <option key={sp.label} value={sp.days === null ? "always" : String(sp.days)}>
                      {sp.label}
                    </option>
                  ))}
                </Select>
              </div>
              <Button variant="secondary" onClick={() => void addMute()}>
                Mute
              </Button>
            </div>
            <p className="mt-2 text-[11px] text-slate-500 dark:text-slate-400">
              Words in the subject: whole words, in that order, whatever the case and the accents - "posledna sanca"
              also mutes "Posledná šanca". Or open a message and use Mute there. The list is the same on your other
              computer.
            </p>
          </Section>
        )}

        {section === "discord" && (
          <>
            <Section
              title="Mail that goes on to Discord"
              hint="Important mail posts a short message by itself when it arrives - never old mail, never Spam or Trash, never mail a record was made from (the record's own message says it). The webhooks are set in Settings → Integrations → Discord."
            >
              {!notif ? (
                <Spinner className="h-4 w-4" />
              ) : (
                <>
                  {!notif.discordEnabled && (
                    <p className="mb-3 rounded-lg bg-surface-muted px-3 py-2 text-xs text-amber-600 dark:text-amber-400">
                      Discord is off. Switch it on in Settings → Integrations → Discord - these choices are kept.
                    </p>
                  )}
                  <label className="flex items-start gap-2 text-xs text-slate-700 dark:text-slate-200">
                    <input
                      type="checkbox"
                      className={`${CHECKBOX_CLASS} mt-0.5`}
                      checked={notif.discordAuto.mail}
                      onChange={(e) => void saveDiscord({ discordAuto: { ...notif.discordAuto, mail: e.target.checked } })}
                    />
                    <span className="font-medium">Post important mail by itself</span>
                  </label>
                  <div className={`mt-3 grid gap-2 sm:grid-cols-2 ${notif.discordAuto.mail ? "" : "pointer-events-none opacity-45"}`}>
                    {MAIL_KINDS.map((k) => (
                      <label key={k.key} className="flex items-center gap-2 text-xs text-slate-700 dark:text-slate-200">
                        <input
                          type="checkbox"
                          className={CHECKBOX_CLASS}
                          checked={notif.discordMailOptions[k.key]}
                          onChange={(e) =>
                            void saveDiscord({ discordMailOptions: { ...notif.discordMailOptions, [k.key]: e.target.checked } })
                          }
                        />
                        {k.label}
                      </label>
                    ))}
                  </div>
                </>
              )}
            </Section>
            <Section
              title="What a mail message carries"
              hint="Every message is a title and a few short lines - who and how much, then what you choose here. Send to Discord on a message uses the same choices."
            >
              {notif && (
                <div className="space-y-2.5">
                  {MAIL_PARTS.map((p) => (
                    <label key={p.key} className="flex items-start gap-2 text-xs text-slate-700 dark:text-slate-200">
                      <input
                        type="checkbox"
                        className={`${CHECKBOX_CLASS} mt-0.5`}
                        checked={notif.discordMailOptions[p.key]}
                        onChange={(e) =>
                          void saveDiscord({ discordMailOptions: { ...notif.discordMailOptions, [p.key]: e.target.checked } })
                        }
                      />
                      <span>
                        <span className="font-medium">{p.label}</span>
                        <span className="block text-[11px] text-slate-500 dark:text-slate-400">{p.hint}</span>
                      </span>
                    </label>
                  ))}
                </div>
              )}
            </Section>
          </>
        )}

        {section === "automatic" && (
          <>
      {/* ---- 5. automation ------------------------------------------------ */}
      <Section
        title="What TIQR may create by itself"
        hint="Manual is the safe one: TIQR prepares the record and waits for your button. Automatic makes it only when it is certain - the checks are below - and anything unclear waits for you in the message, with the reason."
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

        {/* 2.113.0: ONE computer makes the records, whichever has it - two
            computers reading the same mailbox would otherwise both make
            every order. The other one keeps proposing. */}
        <div className="mt-4 flex flex-wrap items-center gap-3 rounded-lg bg-surface-muted px-3 py-2.5 text-xs text-slate-600 dark:text-slate-300">
          <p className="min-w-0 flex-1">
            {status.automationHere ? (
              <>
                <span className="font-semibold text-slate-800 dark:text-slate-100">This computer makes the records.</span> Your
                other computer only proposes them, so nothing is made twice.
              </>
            ) : (
              <>
                <span className="font-semibold text-slate-800 dark:text-slate-100">Your other computer makes the records.</span>{" "}
                This one only proposes them, so nothing is made twice.
              </>
            )}
          </p>
          {!status.automationHere && (
            <Button
              variant="secondary"
              size="sm"
              onClick={() => void wrap(() => api.claimMailAutomation(), "This computer makes the records now")}
            >
              Make them here
            </Button>
          )}
        </div>

        <p className="mt-4 text-xs font-medium text-slate-700 dark:text-slate-200">Automatic makes a record only when</p>
        <ul className="mt-1 list-disc space-y-0.5 pl-4 text-[11px] leading-relaxed text-slate-500 dark:text-slate-400">
          <li>the mail arrived after you switched Automatic on - old mail is never acted on;</li>
          <li>every figure was read with certainty, from words you can see next to it in the message;</li>
          <li>an order&apos;s event, or a sale&apos;s tickets, matched exactly - nothing is guessed into existence;</li>
          <li>nothing like it is in TIQR yet - the same order number, the same email thread, or the same event, total and date;</li>
          <li>no rule sent the mail to Needs review or to the trash.</li>
        </ul>
      </Section>

          </>
        )}
      </div>
    </Card>
  );
}

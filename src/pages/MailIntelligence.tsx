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
// One thing this still is not, said in the UI rather than left to look like
// a bug: read state and where a message is filed are TIQR's own, because the
// scope is read-only and cannot write a Gmail label.
//
// 2.113.0: the message is shown the way it was designed, safely
// (components/MailReader.tsx), and mail can be moved anywhere - with Move
// to, or by dragging it onto a folder in the rail, including marko's own folders
// ("aby som vedel dane maily presunut tak kde chcem do ineho miesta").
import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { api, errMsg } from "../lib/api";
import { copyText } from "../lib/clipboard";
import { lastMailSync, onMailSynced, requestMailSync } from "../lib/mailSync";
import { mergeTopPage } from "../lib/mailRows";
import { isReading, lastSyncText, laneViews, syncHeadline, type LaneTone, storedVsGmail } from "../lib/mailSyncStatus";
import { askReaderToSync, nudgeMailShare, useMailShareView } from "../lib/mailShare";
import { shareLine } from "../lib/mailShareCore";
import {
  type MailConnectionStatus,
  type MailFolderCount,
  type MailLabel,
  type MailLocalFolder,
  type MailMessageSummary,
  type MailMute,
  type MailSyncResult,
} from "../lib/types";
import { Badge, Button, Card, ConfirmDialog, EmptyState, Input, PageHeader, Spinner } from "../components/ui";
import { IconBellOff, IconFolder, IconMail, IconPaperclip, IconPencil, IconPlus, IconStar, IconTrash } from "../components/icons";
import { muteKind, muteState, type MuteKind } from "../lib/mailMute";
import { MailSettingsPane } from "../components/MailSettingsPane";
import { PromoCleanup } from "../components/PromoCleanup";
import {
  categoryLabel,
  isJunk,
  MailMoveMenu,
  MailReader,
  type MailPlace,
  type ReaderChange,
} from "../components/MailReader";
import { useToast } from "../lib/toast";
import { useFillHeight, useRemeasure } from "../lib/useFillHeight";

/** Folders this app defines itself, above Gmail's own. */
type Builtin =
  | "inbox"
  | "important"
  | "starred"
  | "finances"
  | "otp"
  | "review"
  | "promotions"
  | "muted"
  | "spam"
  | "trash";

/* 2.110.0, in the order marko gave: "hlavny inbox: tam budu vsetky spravy
   okrem napriklad spamu a promotions, potom bude starred, potom budu
   finances, potom spam, trash a labels" - and "najskor treba important".
   Social / Updates / Forums are not folders any more: they are inbox mail,
   and the Inbox holds them. What each folder contains is decided in ONE
   place, the backend (commands/mail.rs::folder_expr). `gap` starts a new
   group in the rail. */
const BUILTIN_FOLDERS: { key: Builtin; label: string; gap?: boolean; onlyWhenUsed?: boolean }[] = [
  { key: "inbox", label: "Inbox" },
  { key: "important", label: "Important" },
  { key: "starred", label: "Starred" },
  { key: "finances", label: "Finances" },
  { key: "otp", label: "OTP codes", gap: true },
  { key: "review", label: "Needs review" },
  { key: "promotions", label: "Promotions", gap: true },
  // 2.113.1: shown once something is muted (commands/mail.rs, "MUTED SENDERS").
  { key: "muted", label: "Muted", onlyWhenUsed: true },
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

/** How a lane's state reads in the sync status line. */
const LANE_TONE: Record<LaneTone, string> = {
  active: "text-slate-700 dark:text-slate-200",
  done: "text-emerald-600 dark:text-emerald-400",
  waiting: "text-slate-400 dark:text-slate-500",
  warn: "text-amber-600 dark:text-amber-400",
};

/** What a dragged message carries: the ids, as JSON. Only TIQR's own rows
 *  set it, so a file dragged in from the desktop is never a "move". */
const DRAG_MIME = "application/x-tiqr-mail";

/** Built-in folders a message can be dropped on. The others are views
 *  across places (Important, Starred, Codes, Needs review) and Spam, which
 *  only Gmail decides. */
const DROPPABLE = new Set<string>(["inbox", "finances", "promotions", "trash"]);

/** Views show a message wherever it is filed, so a move leaves it there. */
const VIEWS = new Set<string>(["important", "starred", "otp", "review"]);

function FolderRow({
  label,
  count,
  unread,
  active,
  onClick,
  onDropIds,
  actions,
}: {
  label: string;
  count: number;
  unread: number;
  active: boolean;
  onClick: () => void;
  /** Set when messages can be dropped here. */
  onDropIds?: (ids: number[]) => void;
  /** Small buttons shown on hover (rename, delete). */
  actions?: React.ReactNode;
}) {
  const [over, setOver] = useState(false);
  const drop = onDropIds
    ? {
        onDragOver: (e: React.DragEvent) => {
          if (!e.dataTransfer.types.includes(DRAG_MIME)) return;
          e.preventDefault();
          e.dataTransfer.dropEffect = "move";
          setOver(true);
        },
        onDragLeave: () => setOver(false),
        onDrop: (e: React.DragEvent) => {
          setOver(false);
          const raw = e.dataTransfer.getData(DRAG_MIME);
          if (!raw) return;
          e.preventDefault();
          try {
            const ids = JSON.parse(raw);
            if (Array.isArray(ids) && ids.every((n) => typeof n === "number")) onDropIds(ids);
          } catch {
            // Not ours.
          }
        },
      }
    : {};
  return (
    <div className="group relative" {...drop}>
      <button
        type="button"
        onClick={onClick}
        title={label}
        className={`flex w-full items-center gap-2 rounded-lg px-2.5 py-1.5 text-left text-[12.5px] transition ${
          over
            ? "bg-brand-500/20 ring-2 ring-brand-500/60"
            : active
              ? "bg-brand-600/15 font-semibold text-brand-700 dark:text-brand-300"
              : "text-slate-600 hover:bg-surface-muted dark:text-slate-300"
        }`}
      >
        <span className="min-w-0 flex-1 truncate">{label}</span>
        {unread > 0 ? (
          <span className={`num shrink-0 text-[11px] font-semibold text-brand-600 dark:text-brand-400 ${actions ? "group-hover:invisible" : ""}`}>
            {unread}
          </span>
        ) : count > 0 ? (
          <span className={`num shrink-0 text-[11px] text-slate-400 dark:text-slate-500 ${actions ? "group-hover:invisible" : ""}`}>
            {count}
          </span>
        ) : null}
      </button>
      {actions && (
        <div className="absolute inset-y-0 right-1 hidden items-center gap-0.5 group-hover:flex">{actions}</div>
      )}
    </div>
  );
}

/** One name typed into the rail - a new folder, or a new name for one. */
function FolderNameInput({
  initial,
  onDone,
}: {
  initial: string;
  onDone: (name: string | null) => void;
}) {
  const [name, setName] = useState(initial);
  const done = useRef(false);
  const finish = (v: string | null) => {
    if (done.current) return;
    done.current = true;
    onDone(v);
  };
  return (
    <input
      autoFocus
      maxLength={40}
      placeholder="Folder name"
      value={name}
      onChange={(e) => setName(e.target.value)}
      onKeyDown={(e) => {
        if (e.key === "Enter") finish(name.trim() || null);
        if (e.key === "Escape") finish(null);
      }}
      onBlur={() => finish(name.trim() || null)}
      className="my-0.5 w-full rounded-lg border border-brand-400 bg-surface px-2.5 py-1 text-[12.5px] text-slate-800 outline-none dark:text-slate-100"
    />
  );
}

const railIcon =
  "rounded p-1 text-slate-400 hover:bg-slate-200/70 hover:text-slate-700 dark:hover:bg-slate-700/60 dark:hover:text-slate-200";

/* Memoised: a folder can hold thousands of rows once he has scrolled, and
   a new page arriving must not re-render every one already on screen. */
const MailRow = memo(function MailRow({
  m,
  active,
  checked,
  movedName,
  preview,
  tags,
  compact,
  onOpen,
  onToggle,
  onDragStart,
  onCopyCode,
  onStar,
}: {
  m: MailMessageSummary;
  active: boolean;
  checked: boolean;
  /** 2.115.0: the list's look (Mail -> Settings -> Mail screen). */
  preview: boolean;
  tags: boolean;
  compact: boolean;
  /** Where it was moved to, when that is not the folder on screen. */
  movedName: string | null;
  onOpen: (id: number) => void;
  onToggle: (id: number) => void;
  onDragStart: (id: number, e: React.DragEvent) => void;
  onCopyCode: (code: string) => void;
  onStar: (id: number, starred: boolean) => void;
}) {
  const onClick = () => onOpen(m.id);
  return (
    <button
      type="button"
      onClick={onClick}
      draggable
      data-mail-row={m.id}
      onDragStart={(e) => onDragStart(m.id, e)}
      className={`group w-full border-b border-slate-200 px-3 ${compact ? "py-1.5" : "py-2.5"} text-left transition dark:border-slate-800 ${
        active ? "bg-brand-600/10" : checked ? "bg-brand-500/[0.05]" : "hover:bg-surface-muted"
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
        {/* 2.113.1: one click stars it (TIQR's own star). Inside the row
            button, so it stops the click like the code does. */}
        <span
          role="button"
          tabIndex={-1}
          title={m.isStarred ? "Remove the star" : "Star it"}
          aria-label={m.isStarred ? "Remove the star" : "Star it"}
          onClick={(e) => {
            e.stopPropagation();
            onStar(m.id, !m.isStarred);
          }}
          className={`shrink-0 self-center rounded p-px transition hover:bg-amber-500/15 ${
            m.isStarred ? "text-amber-500" : "text-slate-300 opacity-0 group-hover:opacity-100 dark:text-slate-600"
          }`}
        >
          <IconStar className={`h-3 w-3 ${m.isStarred ? "fill-amber-400" : ""}`} />
        </span>
        <span
          className={`min-w-0 flex-1 truncate text-[13px] ${
            m.isUnread ? "font-semibold text-slate-900 dark:text-slate-100" : "text-slate-600 dark:text-slate-300"
          }`}
        >
          {m.fromName ?? m.fromAddress}
        </span>
        {m.hasAttachments && (
          <IconPaperclip className="h-3 w-3 shrink-0 self-center text-slate-400 dark:text-slate-500" aria-label="Has attachments" />
        )}
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
      {/* The code, right in the list - and one click copies it. A one-time
          code is useless slowly: by the time you have opened the mail and
          gone back, half the window is gone. */}
      {m.otpCode ? (
        <span
          role="button"
          tabIndex={-1}
          title="Click to copy"
          onClick={(e) => {
            e.stopPropagation();
            onCopyCode(m.otpCode!);
          }}
          className="num -mx-1 mt-1 inline-block rounded px-1 text-base font-bold tracking-[0.18em] text-brand-600 hover:bg-brand-500/10 dark:text-brand-400"
        >
          {m.otpCode}
        </span>
      ) : (
        preview &&
        m.snippet && <p className="mt-0.5 truncate text-[11px] text-slate-400 dark:text-slate-500">{m.snippet}</p>
      )}
      {((tags && m.category !== "other" && !isJunk(m)) || movedName) && (
        <div className={`${compact ? "mt-0.5" : "mt-1"} flex flex-wrap items-center gap-1.5`}>
          {tags && m.category !== "other" && !isJunk(m) && <Badge tone="sold">{categoryLabel(m.category)}</Badge>}
          {movedName && (
            <span className="inline-flex items-center gap-1 rounded bg-surface-muted px-1.5 py-px text-[10.5px] text-slate-500 dark:text-slate-400">
              <IconFolder className="h-2.5 w-2.5" />
              {movedName}
            </span>
          )}
        </div>
      )}
    </button>
  );
});

/* 2.115.0: where Mail opens (Mail -> Settings -> Mail screen). The choice is
   remembered here as well, so the very first look is already the right
   folder; "Where I left off" is this computer's own. */
const START_KEY = "tiqr:mail-start";
const LAST_KEY = "tiqr:mail-folder";
/* 2.115.0: the list's width, dragged by its edge. */
const LIST_WIDTH_KEY = "tiqr:mail-list-width";
const LIST_DEFAULT = 360;
const LIST_MIN = 260;
const LIST_MAX = 720;
const READER_MIN = 340;

function readStorage(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function writeStorage(key: string, value: string) {
  try {
    localStorage.setItem(key, value);
  } catch {
    // A window that keeps nothing: Mail opens in the Inbox next time.
  }
}

function firstFolder(): string {
  const choice = readStorage(START_KEY);
  if (choice === "important" || choice === "starred") return choice;
  if (choice === "last") return readStorage(LAST_KEY) || "inbox";
  return "inbox";
}

/** A key pressed while typing is the typing's, not a shortcut. */
function typing(target: EventTarget | null): boolean {
  const el = target as HTMLElement | null;
  return !!el && (el.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(el.tagName));
}

export default function MailIntelligence() {
  const toast = useToast();
  const [folder, setFolder] = useState<string>(firstFolder);
  const [status, setStatus] = useState<MailConnectionStatus | null>(null);
  const [labels, setLabels] = useState<MailLabel[]>([]);
  const [localFolders, setLocalFolders] = useState<MailLocalFolder[]>([]);
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
  /** 2.115.0: whether the other computer reads Gmail for this one. */
  const share = useMailShareView();
  const [nudged, setNudged] = useState(false);
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [confirmDelete, setConfirmDelete] = useState(false);
  /** A folder being named: `null` id = a new one. */
  const [naming, setNaming] = useState<{ id: string | null; name: string } | null>(null);
  const [deletingFolder, setDeletingFolder] = useState<MailLocalFolder | null>(null);
  /** 2.113.1: the muted senders. */
  const [mutes, setMutes] = useState<MailMute[]>([]);
  /** 2.114.0: a big batch waiting for "Move N to Trash?". */
  const [confirmTrash, setConfirmTrash] = useState<number[] | null>(null);
  const [selectingAll, setSelectingAll] = useState(false);

  /* Which folder and search the screen is showing. A page that comes back
     after he has already moved on is dropped rather than shown under the
     wrong folder. */
  const view = `${folder}|${query ?? ""}`;
  const viewRef = useRef(view);
  viewRef.current = view;
  const rowsRef = useRef(rows);
  rowsRef.current = rows;
  const selectedRef = useRef(selected);
  selectedRef.current = selected;

  // Typing searches as he types, without a query per keystroke.
  useEffect(() => {
    const t = setTimeout(() => setQuery(search.trim() || null), 250);
    return () => clearTimeout(t);
  }, [search]);

  const open = useMemo(() => rows.find((m) => m.id === openId) ?? null, [rows, openId]);

  const loadHeader = useCallback(async () => {
    const [s, l, f, c, mu] = await Promise.all([
      api.getMailStatus(),
      api.listMailLabels(),
      api.listMailLocalFolders(),
      api.getMailFolderCounts(),
      api.listMailMutes(),
    ]);
    setStatus(s);
    setLabels(l);
    setLocalFolders(f);
    setCounts(Object.fromEntries(c.map((x) => [x.key, x])));
    setMutes(mu);
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
   *  place further down - and without drawing again the rows that did not
   *  change (lib/mailRows.ts). */
  const refreshTop = useCallback(async () => {
    if (folder === SETTINGS_FOLDER) return;
    const key = viewRef.current;
    try {
      const page = await api.listMailFolder(folder, query, null, PAGE);
      if (viewRef.current !== key) return;
      const merged = mergeTopPage(rowsRef.current, page, PAGE);
      setRows(merged.rows);
      if (merged.hasMore !== null) setHasMore(merged.hasMore);
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
  /** 2.114.0: "v bulku vediet oznacit maily a vymazat ich". The rows on
   *  screen first; then, one click more, the whole folder. */
  const selectLoaded = () => setSelected(new Set(rowsRef.current.map((m) => m.id)));
  const selectWholeFolder = async () => {
    setSelectingAll(true);
    try {
      setSelected(new Set(await api.listMailFolderIds(folder, query)));
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setSelectingAll(false);
    }
  };

  const pick = (key: string) => {
    setFolder(key);
    setOpenId(null);
    clearSelection();
    setConfirmDelete(false);
  };

  /* 2.115.0: where Mail opens. The choice is remembered for next time; the
     first time it is known, Mail goes there - unless a folder was picked
     already. */
  const startedIn = useRef<string | null>(folder);
  useEffect(() => {
    if (!status) return;
    writeStorage(START_KEY, status.startFolder ?? "inbox");
    const from = startedIn.current;
    startedIn.current = null;
    if (from === null || folder !== from) return;
    const want = status.startFolder === "last" ? readStorage(LAST_KEY) || "inbox" : status.startFolder ?? "inbox";
    if (want !== folder) pick(want);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [status?.startFolder, status !== null]);
  useEffect(() => {
    if (folder !== SETTINGS_FOLDER) writeStorage(LAST_KEY, folder);
  }, [folder]);

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
  const onCopyCode = useCallback(
    async (code: string) => {
      if (await copyText(code)) toast.success(`Code ${code} copied`);
      else toast.error("Couldn't copy - open the message and select it.");
    },
    [toast],
  );

  /* Dragging a row carries it - or every selected row, when it is one of
     them - onto a folder in the rail. */
  const onDragStart = useCallback((id: number, e: React.DragEvent) => {
    const sel = selectedRef.current;
    const ids = sel.has(id) ? [...sel] : [id];
    e.dataTransfer.setData(DRAG_MIME, JSON.stringify(ids));
    e.dataTransfer.setData("text/plain", `${ids.length} message${ids.length === 1 ? "" : "s"}`);
    e.dataTransfer.effectAllowed = "move";
    const ghost = document.createElement("div");
    ghost.textContent = ids.length === 1 ? "Move 1 message" : `Move ${ids.length} messages`;
    ghost.className = "fixed -left-[999px] top-0 rounded-lg bg-brand-600 px-3 py-1.5 text-xs font-semibold text-white";
    document.body.appendChild(ghost);
    e.dataTransfer.setDragImage(ghost, 14, 14);
    setTimeout(() => ghost.remove(), 0);
  }, []);

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

  /* EVERY one of marko's own labels, including the empty ones.
     marko: "nech to cita vsetky zalozky". An empty folder is information;
     a folder that is not there is a bug report waiting to happen. */
  const userLabels = useMemo(() => labels.filter((l) => l.kind !== "system"), [labels]);

  /* 2.113.0: everywhere a message can be moved to - "aby som vedel dane
     maily presunut tak kde chcem do ineho miesta". */
  const places = useMemo<MailPlace[]>(
    () => [
      { key: "inbox", label: "Inbox", group: "TIQR" },
      { key: "finances", label: "Finances", group: "TIQR" },
      { key: "promotions", label: "Promotions", group: "TIQR" },
      ...userLabels.map((l) => ({ key: l.id, label: l.name, group: "Your labels" as const })),
      ...localFolders.map((f) => ({ key: f.id, label: f.name, group: "Your folders" as const })),
    ],
    [userLabels, localFolders],
  );
  const placeName = useCallback((key: string) => places.find((p) => p.key === key)?.label ?? null, [places]);

  /** After a change to some messages: the ones that left this folder go
   *  from the list at once, without losing the place in it, and the top of the
   *  folder and the counts are read again. */
  const settle = (ids: number[], gone: boolean, patch?: Partial<MailMessageSummary>) => {
    const hit = new Set(ids);
    // 2.115.0: the open one went - the next older, the next newer, or back
    // to the list, as marko chose. The list runs newest first.
    if (gone && openId !== null && hit.has(openId)) {
      const list = rowsRef.current;
      const at = list.findIndex((m) => m.id === openId);
      const step = status?.afterMove === "older" ? 1 : status?.afterMove === "newer" ? -1 : 0;
      let next: number | null = null;
      for (let i = at + step; step !== 0 && at >= 0 && i >= 0 && i < list.length; i += step) {
        if (!hit.has(list[i].id)) {
          next = list[i].id;
          break;
        }
      }
      setOpenId(next);
    }
    setRows((prev) =>
      gone ? prev.filter((m) => !hit.has(m.id)) : patch ? prev.map((m) => (hit.has(m.id) ? { ...m, ...patch } : m)) : prev,
    );
    clearSelection();
    setConfirmDelete(false);
    void refreshTop();
    void loadHeader().catch(() => undefined);
  };

  const moveIds = async (ids: number[], target: string) => {
    if (ids.length === 0 || target === folder) return;
    try {
      const n = await api.moveMail(ids, target);
      const where = target ? `to ${placeName(target) ?? target}` : "back to where Gmail has it";
      toast.success(`${n} message${n === 1 ? "" : "s"} moved ${where}`);
      settle(ids, !VIEWS.has(folder), { movedTo: target || null, trashedAt: null });
    } catch (e) {
      toast.error(errMsg(e));
    }
  };

  const bulk = async (what: "trash" | "restore" | "delete" | "read" | "unread", ids = [...selected]) => {
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
      if (what === "read" || what === "unread") settle(ids, false, { isUnread: what === "unread" });
      else settle(ids, what === "delete" || (what === "trash") !== (folder === "trash"));
    } catch (e) {
      toast.error(errMsg(e));
    }
  };

  /** To the Trash - after asking, for a big batch. Delete forever is only
   *  ever from the Trash (marko: "vzdy najskor presunutie do kosa"). */
  const deleteIds = (ids: number[]) => {
    if (ids.length > 50) setConfirmTrash(ids);
    else void bulk("trash", ids);
  };

  /* ---- stars and muted senders (2.113.1) -------------------------------- */

  const starIds = async (ids: number[], starred: boolean) => {
    if (ids.length === 0) return;
    try {
      await api.starMail(ids, starred);
      // Unstarred in Starred: it leaves the folder.
      settle(ids, folder === "starred" && !starred, { isStarred: starred });
    } catch (e) {
      toast.error(errMsg(e));
    }
  };
  const onStar = useCallback(
    (id: number, starred: boolean) => {
      void api
        .starMail([id], starred)
        .then(() => {
          setRows((prev) => prev.map((m) => (m.id === id ? { ...m, isStarred: starred } : m)));
          void loadHeader().catch(() => undefined);
        })
        .catch((e) => toast.error(errMsg(e)));
    },
    [loadHeader, toast],
  );

  /* 2.115.0: keys (Mail -> Settings -> Mail screen): up and down (or k and j)
     go through the list, s stars, u marks read or unread, Delete moves the
     open message to the Trash - never further - and / goes to the search.
     Never while typing, never under a dialog, and never inside the message
     itself, which keeps its own keys. */
  const keysRef = useRef<(e: KeyboardEvent) => void>(() => {});
  keysRef.current = (e: KeyboardEvent) => {
    if (!status?.shortcuts || folder === SETTINGS_FOLDER) return;
    if (e.defaultPrevented || e.metaKey || e.ctrlKey || e.altKey || typing(e.target)) return;
    if (document.querySelector('[role="dialog"]')) return;
    const list = rowsRef.current;
    const at = list.findIndex((m) => m.id === openId);
    const current = at >= 0 ? list[at] : null;
    const go = (i: number) => {
      const m = list[Math.max(0, Math.min(list.length - 1, i))];
      if (!m) return;
      setOpenId(m.id);
      const row = document.querySelector<HTMLElement>(`[data-mail-row="${m.id}"]`);
      row?.focus({ preventScroll: true });
      row?.scrollIntoView({ block: "nearest" });
    };
    switch (e.key) {
      case "ArrowDown":
      case "j":
        e.preventDefault();
        go(at + 1);
        break;
      case "ArrowUp":
      case "k":
        e.preventDefault();
        go(at < 0 ? 0 : at - 1);
        break;
      case "s":
        if (current) onStar(current.id, !current.isStarred);
        break;
      case "u":
        if (current) {
          const unread = !current.isUnread;
          void api
            .setMailRead(current.id, !unread)
            .then(() => {
              setRows((prev) => prev.map((m) => (m.id === current.id ? { ...m, isUnread: unread } : m)));
              void loadHeader().catch(() => undefined);
            })
            .catch((err) => toast.error(errMsg(err)));
        }
        break;
      case "Delete":
      case "#":
        // In the Trash, Delete forever stays a button of its own.
        if (current && folder !== "trash") {
          e.preventDefault();
          deleteIds([current.id]);
        }
        break;
      case "/":
        e.preventDefault();
        document.getElementById("mail-search")?.focus();
        break;
    }
  };
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => keysRef.current(e);
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  /** Its mail leaves every folder but Muted - the list is read again. */
  const mute = async (kind: MuteKind, who: string, days: number | null) => {
    try {
      setMutes(await (kind === "subject" ? api.muteMailSubject(who, days) : api.muteMailSender(who, days)));
      const span = days === null ? "until you unmute" : `for ${days} day${days === 1 ? "" : "s"}`;
      toast.success(
        kind === "subject"
          ? `Mail with "${who}" in the subject muted ${span} - it is under Muted. Orders, sales and payouts still show.`
          : `${who} muted ${span} - its mail is under Muted. Orders, sales and payouts still show.`,
      );
      void loadFolder();
      void loadHeader().catch(() => undefined);
    } catch (e) {
      toast.error(errMsg(e));
    }
  };
  const unmute = async (kind: MuteKind, who: string) => {
    try {
      setMutes(await (kind === "subject" ? api.unmuteMailSubject(who) : api.unmuteMailSender(who)));
      toast.success(
        kind === "subject"
          ? `"${who}" unmuted - that mail is back where it was.`
          : `${who} unmuted - all of its mail is back where it was.`,
      );
      void loadFolder();
      void loadHeader().catch(() => undefined);
    } catch (e) {
      toast.error(errMsg(e));
    }
  };

  /* ---- marko's own folders ---------------------------------------------- */

  const saveFolderName = async (id: string | null, name: string | null) => {
    setNaming(null);
    if (!name) return;
    try {
      const next = id ? await api.renameMailLocalFolder(id, name) : await api.createMailLocalFolder(name);
      setLocalFolders(next);
      if (!id) toast.success(`Folder "${name}" made - drag mail onto it, or use Move to.`);
    } catch (e) {
      toast.error(errMsg(e));
    }
  };

  const removeFolder = async (f: MailLocalFolder) => {
    setDeletingFolder(null);
    try {
      setLocalFolders(await api.deleteMailLocalFolder(f.id));
      if (folder === f.id) pick("inbox");
      void loadHeader();
      toast.success(`Folder "${f.name}" deleted - its mail is back where Gmail has it.`);
    } catch (e) {
      toast.error(errMsg(e));
    }
  };

  const onListScroll = (e: React.UIEvent<HTMLDivElement>) => {
    const el = e.currentTarget;
    if (el.scrollTop + el.clientHeight >= el.scrollHeight - 600) void loadMore();
  };

  // 2.113.0: the panes reach the bottom of the window (lib/useFillHeight.ts).
  const panesRef = useRef<HTMLDivElement>(null);
  const panesHeight = useFillHeight(panesRef);
  /* 2.115.0, marko: "ked je mail vacsi tak si vies potiahnut posuvnikom aby
     sa zvacsila ta cast kde je mail". The edge between the list and the
     message is a handle: dragged left, the message gets the room. Kept per
     computer - their screens differ. */
  const [listWidth, setListWidth] = useState(() => {
    const stored = Number(readStorage(LIST_WIDTH_KEY));
    return Number.isFinite(stored) && stored >= LIST_MIN ? Math.min(stored, LIST_MAX) : LIST_DEFAULT;
  });
  const [resizing, setResizing] = useState(false);
  const saveListWidth = (w: number) => {
    const width = Math.round(Math.max(LIST_MIN, Math.min(LIST_MAX, w)));
    setListWidth(width);
    writeStorage(LIST_WIDTH_KEY, String(width));
  };
  const startResize = (e: React.PointerEvent) => {
    e.preventDefault();
    const startX = e.clientX;
    const from = listWidth;
    // The message keeps room enough to read.
    const room = (panesRef.current?.clientWidth ?? 1200) - 200 - 32 - READER_MIN;
    const max = Math.max(LIST_MIN, Math.min(LIST_MAX, room));
    let last = from;
    setResizing(true);
    const move = (ev: PointerEvent) => {
      last = Math.max(LIST_MIN, Math.min(max, from + ev.clientX - startX));
      setListWidth(last);
    };
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      setResizing(false);
      saveListWidth(last);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  };
  useRemeasure([loading, status?.connected]);

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

  /* What the reader is doing, in words (lib/mailSyncStatus.ts). A long
     first read looks exactly like a stuck one unless the screen says which
     lane it is on and how much each still has to download. */
  const progress = lastRun?.progress ?? status.progress;
  const failed = lastRun?.status === "failed";
  // 2.115.0: while the other computer reads Gmail for this one, this one's
  // lanes stand still - the line says where the mail comes from instead.
  const following = share.role === "follower";
  const headline = following
    ? shareLine(share, fmt)
    : syncHeadline({
        progress,
        paused: status.paused,
        failed,
        rateLimited: !!lastRun?.rateLimited,
        intervalSeconds: status.syncIntervalSeconds,
      });
  const lanes = following ? [] : laneViews(progress, fmt);
  const busy = nudged || (!following && isReading(progress, status.paused, failed));
  const dropOn = (key: string) => (ids: number[]) => void (key === "trash" ? bulk("trash", ids) : moveIds(ids, key));

  return (
    /* NOT `h-full`: Layout gives pages no height (PROTECTED_AREAS 2.55.0),
       so the three-pane block below pins its own, the way Notes does. */
    <div>
      <PageHeader
        title="Mail"
        subtitle={`${status.accountEmail ?? "Your inbox"} · ${storedVsGmail(progress, fmt)}${
          status.unreadCount > 0 ? ` · ${fmt(status.unreadCount)} unread` : ""
        }`}
        actions={
          <>
            <div className="w-56">
              <Input
                id="mail-search"
                placeholder="Search this folder"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
              />
            </div>
            {/* Sync asks the one reader (commands/mail_sync.rs, started by
                Layout.tsx) to go now; when reading is paused, the same
                button resumes it. */}
            <Button
              variant="secondary"
              disabled={nudged}
              onClick={async () => {
                // 2.115.0: the other computer reads Gmail for this one - it
                // is the one asked to look now.
                if (following && !status.paused && askReaderToSync()) {
                  toast.success(`Your ${share.peer ?? "other computer"} checks Gmail now`);
                  return;
                }
                setNudged(true);
                try {
                  if (status.paused) {
                    setStatus(await api.setMailPaused(false));
                    nudgeMailShare();
                  }
                  requestMailSync();
                } catch (e) {
                  setNudged(false);
                  toast.error(errMsg(e));
                }
              }}
            >
              {status.paused ? "Resume" : "Sync now"}
            </Button>
          </>
        }
      />

      {/* The reader at a glance: which lane it is on, what each lane still
          has to download, and when it last finished. Says out loud that it
          reads by itself, so marko does not sit there wondering whether to
          press Sync. */}
      <Card className="mb-3 flex flex-wrap items-center gap-x-4 gap-y-1 px-3 py-2 text-[11px]">
        <span className="flex min-w-0 items-center gap-1.5 font-medium text-slate-700 dark:text-slate-200">
          {busy && <Spinner className="h-3 w-3 shrink-0" />}
          <span className="truncate">{headline}</span>
        </span>
        {lanes.map((l) => (
          <span key={l.key} title={l.hint} className="whitespace-nowrap">
            <span className="text-slate-400 dark:text-slate-500">{l.label}</span>{" "}
            <span className={`tabular-nums ${LANE_TONE[l.tone]}`}>{l.text}</span>
          </span>
        ))}
        {share.role === "reader" && share.peer && (
          <span className="whitespace-nowrap text-slate-500 dark:text-slate-400" title={shareLine(share, fmt)}>
            Sends to your {share.peer}
            {share.sent > 0 ? ` · ${fmt(share.sent)}` : ""}
          </span>
        )}
        <span className="ml-auto whitespace-nowrap text-slate-400 dark:text-slate-500">
          {/* While following: when mail last came from the other computer. */}
          {lastSyncText(
            following ? lastRun?.syncedAt ?? progress.lastSuccessAt : progress.lastSuccessAt ?? status.lastSyncAt,
            new Date(),
          )}
        </span>
      </Card>

      {status.lastError && lastRun?.status !== "synced" && (
        <Card className="mb-4 p-3">
          <p className="text-xs text-red-600 dark:text-red-400">{status.lastError}</p>
        </Card>
      )}

      <div
        ref={panesRef}
        className="flex gap-4"
        style={{ height: panesHeight ? `${panesHeight}px` : "calc(100vh - 168px)", minHeight: "440px" }}
      >
        {/* Folders - TIQR's own in marko's order, then the Gmail labels, then
            marko's own folders. Only the list scrolls; Settings sits BELOW it.
            Mail can be dropped on any place it can be moved to. */}
        <Card className="flex w-[200px] shrink-0 flex-col overflow-hidden p-2">
          <div className="min-h-0 flex-1 overflow-y-auto">
            {BUILTIN_FOLDERS.map((f) => {
              const c = counts[f.key];
              if (f.onlyWhenUsed && !(c?.total ?? 0) && mutes.length === 0 && folder !== f.key) return null;
              return (
                <div key={f.key} className={f.gap ? "mt-2 border-t border-slate-200 pt-2 dark:border-slate-800" : ""}>
                  <FolderRow
                    label={f.label}
                    count={c?.total ?? 0}
                    unread={c?.unread ?? 0}
                    active={folder === f.key}
                    onClick={() => pick(f.key)}
                    onDropIds={DROPPABLE.has(f.key) ? dropOn(f.key) : undefined}
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
                    count={counts[l.id]?.total ?? l.total}
                    unread={counts[l.id]?.unread ?? l.unread}
                    active={folder === l.id}
                    onClick={() => pick(l.id)}
                    onDropIds={dropOn(l.id)}
                  />
                ))}
              </>
            )}

            {/* 2.113.0: marko's own folders, in TIQR only. They travel to the
                other computer with the mail settings. */}
            <div className="mt-3 flex items-center border-t border-slate-200 pt-3 dark:border-slate-800">
              <p className="section-title min-w-0 flex-1 truncate px-2.5 pb-1">Your folders</p>
              <button
                type="button"
                title="New folder"
                aria-label="New folder"
                className={railIcon}
                onClick={() => setNaming({ id: null, name: "" })}
              >
                <IconPlus className="h-3.5 w-3.5" />
              </button>
            </div>
            {localFolders.map((f) =>
              naming?.id === f.id ? (
                <FolderNameInput key={f.id} initial={f.name} onDone={(name) => void saveFolderName(f.id, name)} />
              ) : (
                <FolderRow
                  key={f.id}
                  label={f.name}
                  count={counts[f.id]?.total ?? 0}
                  unread={counts[f.id]?.unread ?? 0}
                  active={folder === f.id}
                  onClick={() => pick(f.id)}
                  onDropIds={dropOn(f.id)}
                  actions={
                    <>
                      <button
                        type="button"
                        title="Rename"
                        aria-label={`Rename ${f.name}`}
                        className={railIcon}
                        onClick={() => setNaming({ id: f.id, name: f.name })}
                      >
                        <IconPencil className="h-3 w-3" />
                      </button>
                      <button
                        type="button"
                        title="Delete folder"
                        aria-label={`Delete ${f.name}`}
                        className={railIcon}
                        onClick={() => setDeletingFolder(f)}
                      >
                        <IconTrash className="h-3 w-3" />
                      </button>
                    </>
                  }
                />
              ),
            )}
            {naming?.id === null && <FolderNameInput initial="" onDone={(name) => void saveFolderName(null, name)} />}
            {localFolders.length === 0 && naming?.id !== null && (
              <button
                type="button"
                className="flex w-full items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-left text-[11.5px] text-slate-400 hover:bg-surface-muted hover:text-slate-600 dark:text-slate-500 dark:hover:text-slate-300"
                onClick={() => setNaming({ id: null, name: "" })}
              >
                <IconFolder className="h-3.5 w-3.5" /> New folder
              </button>
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
          <MailSettingsPane status={status} onStatus={setStatus} localFolders={localFolders} labels={userLabels} />
        ) : (
          <>
            <Card className="flex shrink flex-col overflow-hidden p-0" style={{ width: listWidth, minWidth: LIST_MIN }}>
              {/* 2.114.0: "80% z toho su shits" - sorted on a click, moved only
                  when marko says so (components/PromoCleanup.tsx). */}
              {folder === "promotions" && !query && (
                <PromoCleanup
                  onMoved={() => {
                    void loadFolder();
                    void loadHeader().catch(() => undefined);
                  }}
                />
              )}
              {rows.length > 0 && selected.size === 0 && (
                <label className="flex cursor-pointer items-center gap-2 border-b border-slate-200 px-3 py-1.5 text-[11px] text-slate-400 hover:text-slate-600 dark:border-slate-800 dark:text-slate-500 dark:hover:text-slate-300">
                  <input type="checkbox" className="h-3.5 w-3.5 cursor-pointer" checked={false} onChange={selectLoaded} />
                  Select all
                </label>
              )}
              {selected.size > 0 && (
                <div className="flex flex-wrap items-center gap-1.5 border-b border-slate-200 px-3 py-2 dark:border-slate-800">
                  {(() => {
                    const allShown = rows.length > 0 && rows.every((m) => selected.has(m.id));
                    const total = query ? null : (counts[folder]?.total ?? null);
                    const more = allShown && (hasMore || (total !== null && total > selected.size));
                    return (
                      <>
                        <input
                          type="checkbox"
                          className="h-3.5 w-3.5 cursor-pointer"
                          checked={allShown}
                          aria-label={allShown ? "Clear the selection" : "Select all"}
                          onChange={() => (allShown ? clearSelection() : selectLoaded())}
                        />
                        <span className="text-xs font-medium text-slate-700 dark:text-slate-200">{fmt(selected.size)} selected</span>
                        {more && (
                          <button
                            type="button"
                            disabled={selectingAll}
                            className="text-[11px] font-medium text-brand-600 hover:underline disabled:opacity-50 dark:text-brand-400"
                            onClick={() => void selectWholeFolder()}
                          >
                            {selectingAll ? "Selecting..." : total !== null ? `Select all ${fmt(total)}` : "Select every match"}
                          </button>
                        )}
                      </>
                    );
                  })()}
                  <button
                    type="button"
                    className="text-[11px] text-brand-600 hover:underline dark:text-brand-400"
                    onClick={clearSelection}
                  >
                    Clear
                  </button>
                  <div className="ml-auto flex flex-wrap items-center gap-1">
                    <Button variant="ghost" size="sm" onClick={() => void bulk("read")}>
                      Read
                    </Button>
                    <Button variant="ghost" size="sm" onClick={() => void bulk("unread")}>
                      Unread
                    </Button>
                    {(() => {
                      const all = rows.filter((m) => selected.has(m.id));
                      const starred = all.length > 0 && all.every((m) => m.isStarred);
                      return (
                        <Button variant="ghost" size="sm" onClick={() => void starIds([...selected], !starred)}>
                          <IconStar className={`h-3.5 w-3.5 ${starred ? "fill-amber-400 text-amber-500" : ""}`} />
                          {starred ? "Unstar" : "Star"}
                        </Button>
                      );
                    })()}
                    <MailMoveMenu
                      places={places}
                      current={folder}
                      align="right"
                      onPick={(t) => void moveIds([...selected], t)}
                    />
                    {folder === "trash" ? (
                      <>
                        <Button variant="secondary" size="sm" onClick={() => void bulk("restore")}>
                          Restore
                        </Button>
                        {/* Delete is the one irreversible thing here, so it
                            asks - and the BACKEND refuses anything not already
                            in the trash regardless of what this button does. */}
                        {confirmDelete ? (
                          <Button variant="danger" size="sm" onClick={() => void bulk("delete")}>
                            Delete {fmt(selected.size)} forever?
                          </Button>
                        ) : (
                          <Button variant="ghost" size="sm" onClick={() => setConfirmDelete(true)}>
                            Delete
                          </Button>
                        )}
                      </>
                    ) : (
                      <Button
                        variant="secondary"
                        size="sm"
                        title="Moves them to Trash - restore them or delete them forever from there"
                        onClick={() => deleteIds([...selected])}
                      >
                        <IconTrash className="h-3.5 w-3.5" /> Delete
                      </Button>
                    )}
                  </div>
                </div>
              )}
              {/* 2.113.1: who is muted, for how long, and Unmute. */}
              {folder === "muted" && mutes.length > 0 && (
                <div className="border-b border-slate-200 px-3 py-2 dark:border-slate-800">
                  <p className="section-title mb-1 flex items-center gap-1.5">
                    <IconBellOff className="h-3.5 w-3.5" /> Muted
                  </p>
                  <ul className="space-y-0.5">
                    {mutes.map((m) => (
                      <li key={`${muteKind(m)}:${m.who}`} className="flex items-center gap-2 text-[11.5px]">
                        <span className="min-w-0 flex-1 truncate" title={m.who}>
                          {muteKind(m) === "subject" ? (
                            <span className="text-slate-700 dark:text-slate-200">
                              Subject &ldquo;{m.who}&rdquo;
                            </span>
                          ) : (
                            <span className="font-mono text-slate-700 dark:text-slate-200">{m.who}</span>
                          )}{" "}
                          <span className="text-slate-400 dark:text-slate-500">{muteState(m)}</span>
                        </span>
                        <button
                          type="button"
                          className="shrink-0 text-[11px] font-medium text-brand-600 hover:underline dark:text-brand-400"
                          onClick={() => void unmute(muteKind(m), m.who)}
                        >
                          Unmute
                        </button>
                      </li>
                    ))}
                  </ul>
                  <p className="mt-1 text-[10.5px] leading-snug text-slate-400 dark:text-slate-500">
                    Their mail still arrives and is kept here. Orders, sales and payouts always show where they belong.
                  </p>
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
                      : localFolders.some((f) => f.id === folder)
                        ? "Drag mail onto this folder in the list on the left, or open a message and use Move to."
                        : folder === "muted"
                          ? "Nothing is muted. Open a message from a sender that spams and use Mute to hide its mail for a while."
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
                      movedName={m.movedTo && m.movedTo !== folder ? placeName(m.movedTo) : null}
                      preview={status.listPreview ?? true}
                      tags={status.listTags ?? true}
                      compact={status.listCompact ?? false}
                      onOpen={onOpen}
                      onToggle={onToggle}
                      onDragStart={onDragStart}
                      onCopyCode={onCopyCode}
                      onStar={onStar}
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

            {/* The handle: dragged, or the arrow keys once it has focus;
                a double click puts it back. It sits in the gap, so the gap
                looks as before. */}
            <div
              role="separator"
              aria-orientation="vertical"
              aria-label="Width of the message list"
              aria-valuenow={listWidth}
              aria-valuemin={LIST_MIN}
              aria-valuemax={LIST_MAX}
              tabIndex={0}
              title="Drag to make the message wider or narrower - double-click to put it back"
              onPointerDown={startResize}
              onDoubleClick={() => saveListWidth(LIST_DEFAULT)}
              onKeyDown={(e) => {
                if (e.key === "ArrowLeft" || e.key === "ArrowRight") {
                  e.preventDefault();
                  e.stopPropagation();
                  saveListWidth(listWidth + (e.key === "ArrowLeft" ? -40 : 40));
                }
              }}
              className="group -mx-3 flex w-2 shrink-0 cursor-col-resize items-center justify-center rounded focus:outline-none"
            >
              <span
                className={`h-12 w-1 rounded-full transition ${
                  resizing ? "bg-brand-500" : "bg-slate-300 opacity-0 group-hover:opacity-100 group-focus-visible:opacity-100 dark:bg-slate-600"
                }`}
              />
            </div>
            {/* While dragging, the message's own frame must not catch the mouse. */}
            {resizing && <div className="fixed inset-0 z-50 cursor-col-resize" />}
            <Card className="flex min-w-0 flex-1 flex-col overflow-hidden p-0" style={{ minWidth: READER_MIN }}>
              {open ? (
                <MailReader
                  message={open}
                  status={status}
                  places={places}
                  placeName={placeName}
                  mutes={mutes}
                  onMove={(ids, t) => void moveIds(ids, t)}
                  onTrash={(ids, restore) => void bulk(restore ? "restore" : "trash", ids)}
                  onMute={(kind, who, days) => void mute(kind, who, days)}
                  onUnmute={(kind, who) => void unmute(kind, who)}
                  onChanged={onReaderChanged}
                />
              ) : (
                <div className="flex flex-1 flex-col items-center justify-center gap-1 p-8 text-center">
                  <p className="text-sm text-slate-500 dark:text-slate-400">Pick a message to read it.</p>
                  <p className="text-[11px] text-slate-400 dark:text-slate-500">
                    Drag messages onto a folder on the left to move them.
                  </p>
                  {status.shortcuts && (
                    <p className="text-[11px] text-slate-400 dark:text-slate-500">
                      Or press <kbd className="font-sans">↓</kbd> - the arrow keys go through the list.
                    </p>
                  )}
                </div>
              )}
            </Card>
          </>
        )}
      </div>

      <ConfirmDialog
        open={!!deletingFolder}
        title={`Delete the folder "${deletingFolder?.name ?? ""}"?`}
        message="No mail is deleted: what is in it goes back to where Gmail has it. A rule that moves mail here stops moving it."
        confirmLabel="Delete folder"
        danger
        onCancel={() => setDeletingFolder(null)}
        onConfirm={() => deletingFolder && void removeFolder(deletingFolder)}
      />

      <ConfirmDialog
        open={!!confirmTrash}
        title={`Move ${fmt(confirmTrash?.length ?? 0)} messages to Trash?`}
        message="They stay in Trash - restore them from there any time, or delete them forever there. Gmail is not changed."
        confirmLabel="Move to Trash"
        onCancel={() => setConfirmTrash(null)}
        onConfirm={() => {
          const ids = confirmTrash ?? [];
          setConfirmTrash(null);
          void bulk("trash", ids);
        }}
      />
    </div>
  );
}

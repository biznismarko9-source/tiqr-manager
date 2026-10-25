// 2.116.0 - choosing an event's picture.
//
// What TIQR found is offered, never forced: a certain answer is shown on its
// own (and can be replaced), a likely one waits for "Yes, use it", anything
// less is only in the list to pick from by hand. Every picture carries its
// source and how sure TIQR is; a picture can be uploaded, removed ("No
// picture") or handed back to the automatic search, and that choice is kept
// for every event of the same name - or for one event only.
//
// Two ways to use it:
// * `immediate` (an event's page): a choice is stored the moment it is made.
// * `deferred` (the New event and Edit forms): the choice is handed back to
//   the form and stored when the form saves - cancelling the form changes
//   nothing. The search never holds up the save either way.
import { useEffect, useRef, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api, errMsg } from "../lib/api";
import { reportError } from "../lib/diagnostics";
import { refreshArtwork, shrinkArtwork, sourceName, useArtworkSuggestion } from "../lib/eventArtwork";
import type { ArtworkCandidate, ArtworkDecideInput, ArtworkLevel, ArtworkSuggestion } from "../lib/types";
import { useToast } from "../lib/toast";
import { ArtFrame } from "./EventArt";
import { IconCheck, IconExternalLink, IconRefresh, IconUpload, IconX } from "./icons";
import { Button, Spinner } from "./ui";

export interface ArtworkChoice {
  decide: ArtworkDecideInput;
  preview: { src: string | null; fit: "cover" | "contain"; title: string; source: string };
}

const LEVEL_TEXT: Record<ArtworkLevel, string> = {
  high: "Sure match",
  medium: "Likely - needs a yes",
  low: "Unsure",
};

const LEVEL_TONE: Record<ArtworkLevel, string> = {
  high: "text-emerald-700 dark:text-emerald-400",
  medium: "text-amber-700 dark:text-amber-400",
  low: "text-slate-500 dark:text-slate-400",
};

function openSource(url: string) {
  if (!url.startsWith("https://")) return;
  openUrl(url).catch((e) => reportError("event pictures: open source", e, { severity: "warning" }));
}

/** What the picker says about the picture, in one line. */
function statusLine(s: ArtworkSuggestion | null, choice: ArtworkChoice | null | undefined, loading: boolean): string {
  if (choice) {
    if (choice.decide.state === "none") return "No picture - saved with the event.";
    if (choice.decide.state === "auto") return "Back to automatic - saved with the event.";
    return "Your choice - saved with the event.";
  }
  if (!s) return loading ? "Looking for a picture..." : "Type the event's name to find its picture.";
  if (s.decision?.state === "set") {
    if (s.decision.key.startsWith("event:")) return "Chosen by hand for this event only.";
    return s.shared > 1 ? `Chosen by hand - used by all ${s.shared} events named "${s.label}".` : "Chosen by hand.";
  }
  if (s.decision?.state === "none") return "No picture, by choice.";
  if (s.shown) {
    if (s.shown.source === "upload" || s.shown.key.startsWith("d:")) return `Chosen by hand for "${s.shown.title || s.label}" - shown here too.`;
    return `Found on ${sourceName(s.shown.source)} - a sure match.`;
  }
  switch (s.outcome) {
    case "ask":
      return "Is this the right one? It is shown only once confirmed.";
    case "none":
      return `No picture that is certainly "${s.label}" was found.`;
    case "failed":
      return s.message ?? "The picture services could not be reached - tried again later.";
    case "off":
      return "Automatic search is off - search by hand.";
    default:
      return loading ? "Looking for a picture..." : "Not looked for yet.";
  }
}

export function ArtworkPicker({
  eventId,
  name,
  categoryId,
  mode,
  choices,
  onChoice,
  onSaved,
  browse = false,
  onSubject,
}: {
  eventId?: number | null;
  name: string;
  categoryId: number | null;
  mode: "immediate" | "deferred";
  /** Deferred: the choices waiting for the form to save, by subject - two
   *  rows with the same name share one. */
  choices?: Record<string, ArtworkChoice>;
  onChoice?: (identityKey: string, c: ArtworkChoice | null) => void;
  onSaved?: () => void;
  browse?: boolean;
  /** Which subject the typed name is now - so a form stores only the choice
   *  for the name it saves, never one made for a name typed before. */
  onSubject?: (identityKey: string | null) => void;
}) {
  const toast = useToast();
  const input = name.trim() ? { eventId: eventId ?? null, name, categoryId } : null;
  const { suggestion: s, loading, error, load, fresh } = useArtworkSuggestion(input, mode === "immediate" ? "cached" : "search");
  const choice = s ? choices?.[s.identityKey] ?? null : null;
  const subject = name.trim() && fresh ? s?.identityKey ?? null : null;
  const reportSubject = useRef(onSubject);
  reportSubject.current = onSubject;
  useEffect(() => {
    reportSubject.current?.(subject);
  }, [subject]);
  const [browsing, setBrowsing] = useState(browse);
  const [scope, setScope] = useState<"subject" | "event">("subject");
  const [saving, setSaving] = useState(false);
  const fileRef = useRef<HTMLInputElement>(null);

  // A choice for one event only is shown as such the next time.
  useEffect(() => {
    setScope(s?.decision?.key.startsWith("event:") ? "event" : "subject");
  }, [s?.decision?.key]);

  const targetKey = scope === "event" && s?.eventKey ? s.eventKey : s?.identityKey;

  async function commit(c: ArtworkChoice | null, done: string) {
    if (mode === "deferred") {
      if (s) onChoice?.(s.identityKey, c);
      setBrowsing(false);
      return;
    }
    if (!c) return;
    setSaving(true);
    try {
      await api.eventArtworkDecide(c.decide);
      refreshArtwork();
      await load("cached");
      setBrowsing(false);
      toast.success(done);
      onSaved?.();
    } catch (e) {
      toast.error(errMsg(e));
      reportError("event pictures: save a choice", e);
    } finally {
      setSaving(false);
    }
  }

  function base(): Pick<ArtworkDecideInput, "key" | "label" | "queryKey"> | null {
    if (!s || !targetKey) return null;
    return { key: targetKey, label: s.label, queryKey: s.queryKey };
  }

  function pick(c: ArtworkCandidate) {
    const b = base();
    if (!b || !c.thumb) return;
    void commit(
      {
        decide: {
          ...b,
          state: "set",
          image: c.thumb,
          fit: c.fit,
          source: c.source,
          sourceId: c.sourceId,
          sourceTitle: c.title,
          sourcePage: c.page,
          imageUrl: c.imageUrl,
          confidence: c.confidence,
        },
        preview: { src: c.thumb, fit: c.fit, title: c.title, source: c.source },
      },
      "Picture saved",
    );
  }

  function choose(state: "none" | "auto") {
    const b = base();
    if (!b) return;
    if (mode === "deferred" && state === "auto" && !s?.decision) {
      // Nothing stored to hand back: just forget the pending choice.
      if (s) onChoice?.(s.identityKey, null);
      return;
    }
    void commit(
      { decide: { ...b, state }, preview: { src: null, fit: "cover", title: s?.label ?? name, source: "" } },
      state === "none" ? "No picture for this event" : "Back to the automatic picture",
    );
  }

  async function upload(file: File) {
    const b = base();
    if (!b) return;
    try {
      const { src, fit } = await shrinkArtwork(file);
      // The file's own name is not kept: it is the computer's, not the picture's.
      void commit(
        {
          decide: { ...b, state: "set", image: src, fit, source: "upload" },
          preview: { src, fit, title: s?.label ?? name, source: "upload" },
        },
        "Picture saved",
      );
    } catch (e) {
      toast.error(errMsg(e));
    }
  }

  function openList() {
    setBrowsing(true);
    // A choice already made means the alternatives were not fetched with it.
    const pictured = s?.candidates.filter((c) => c.thumb && c.source !== "earlier") ?? [];
    if (!s || pictured.length === 0 || s.decision) void load("refresh");
  }

  const shown = choice ? choice.preview : s?.shown ?? null;
  const top = s?.candidates.find((c) => c.thumb && c.source !== "earlier");
  const asking = !choice && !s?.decision && !s?.shown && s?.outcome === "ask" && top?.level === "medium";
  const preview = shown ?? (asking ? { src: top!.thumb!, fit: top!.fit, title: top!.title, source: top!.source } : null);
  const busy = loading || saving;
  const decided = !!choice || !!s?.decision;
  const listed = (s?.candidates ?? []).filter((c) => c.thumb);
  const sourcePage = choice ? choice.decide.sourcePage ?? "" : s?.shown?.page ?? (asking ? top?.page ?? "" : "");

  return (
    <div className="rounded-xl border border-slate-200 bg-slate-50/60 p-3 dark:border-white/10 dark:bg-white/[0.03]">
      <div className="flex items-start gap-3">
        <div className="relative">
          <ArtFrame image={preview} name={s?.label || name || "this event"} size="lg" />
          {asking && (
            <span className="absolute -right-1 -top-1 grid h-5 w-5 place-items-center rounded-full bg-amber-500 text-[11px] font-bold text-white shadow">
              ?
            </span>
          )}
        </div>
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <p className="truncate text-sm font-medium text-slate-800 dark:text-slate-100">{s?.label || name || "Picture"}</p>
            {busy && <Spinner className="h-3.5 w-3.5" />}
          </div>
          <p className="mt-0.5 text-xs text-slate-500 dark:text-slate-400" aria-live="polite">
            {error ?? statusLine(s, choice, loading)}
          </p>
          {asking && top && (
            <p className={`mt-0.5 text-xs ${LEVEL_TONE[top.level]}`}>
              {top.title} · {sourceName(top.source)} · {LEVEL_TEXT[top.level]}
            </p>
          )}
          {sourcePage && (
            <button
              type="button"
              className="mt-0.5 inline-flex items-center gap-1 text-[11px] text-slate-500 underline-offset-2 hover:text-slate-800 hover:underline dark:text-slate-400 dark:hover:text-slate-200"
              onClick={() => openSource(sourcePage)}
            >
              Source <IconExternalLink className="h-3 w-3" />
            </button>
          )}
          <div className="mt-2 flex flex-wrap items-center gap-1.5">
            {asking && top && (
              <Button variant="primary" size="sm" disabled={busy} onClick={() => pick(top)}>
                <IconCheck className="h-3.5 w-3.5" /> Yes, use it
              </Button>
            )}
            <Button variant="secondary" size="sm" disabled={busy || !s} onClick={() => (browsing ? setBrowsing(false) : openList())}>
              {browsing ? "Hide results" : shown || asking ? "Choose another" : "Show results"}
            </Button>
            <Button variant="secondary" size="sm" disabled={busy || !s} onClick={() => fileRef.current?.click()}>
              <IconUpload className="h-3.5 w-3.5" /> Upload
            </Button>
            {!(choice?.decide.state === "none" || (!choice && s?.decision?.state === "none")) && (
              <Button variant="secondary" size="sm" disabled={busy || !s} onClick={() => choose("none")}>
                <IconX className="h-3.5 w-3.5" /> No picture
              </Button>
            )}
            {decided && (
              <Button variant="ghost" size="sm" disabled={busy || !s} onClick={() => choose("auto")}>
                Back to automatic
              </Button>
            )}
            <Button
              variant="ghost"
              size="sm"
              disabled={busy || !s}
              onClick={() => void load("refresh")}
              title="Ask the picture services again"
            >
              <IconRefresh className="h-3.5 w-3.5" /> Search again
            </Button>
          </div>
          {mode === "immediate" && s?.eventKey && s.shared > 1 && (
            <div className="mt-2 flex flex-wrap gap-3 text-xs text-slate-600 dark:text-slate-300" role="radiogroup" aria-label="Which events the picture is for">
              <label className="inline-flex items-center gap-1.5">
                <input type="radio" checked={scope === "subject"} onChange={() => setScope("subject")} />
                All {s.shared} events named "{s.label}"
              </label>
              <label className="inline-flex items-center gap-1.5">
                <input type="radio" checked={scope === "event"} onChange={() => setScope("event")} />
                Only this event
              </label>
            </div>
          )}
        </div>
      </div>

      {browsing && (
        <div className="mt-3 border-t border-slate-200 pt-3 dark:border-white/10">
          {loading && listed.length === 0 ? (
            <p className="flex items-center gap-2 text-xs text-slate-500 dark:text-slate-400">
              <Spinner className="h-3.5 w-3.5" /> Asking the picture services...
            </p>
          ) : listed.length === 0 ? (
            <p className="text-xs text-slate-500 dark:text-slate-400">
              Nothing to choose from - upload a picture of your own instead.
            </p>
          ) : (
            <ul className="grid grid-cols-2 gap-2 sm:grid-cols-3">
              {listed.map((c) => (
                <li key={c.id}>
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() => pick(c)}
                    title={c.reasons.join(" · ")}
                    className="flex w-full items-center gap-2 rounded-lg border border-slate-200 bg-white p-1.5 text-left hover:border-brand-500 disabled:opacity-60 dark:border-white/10 dark:bg-white/[0.04] dark:hover:border-brand-400"
                  >
                    <ArtFrame image={{ src: c.thumb!, fit: c.fit, title: c.title, source: c.source }} name={c.title} size="md" />
                    <span className="min-w-0">
                      <span className="block truncate text-xs font-medium text-slate-800 dark:text-slate-100">{c.title}</span>
                      <span className="block truncate text-[11px] text-slate-500 dark:text-slate-400">
                        {sourceName(c.source)}
                        {c.description ? ` · ${c.description}` : ""}
                      </span>
                      <span className={`block text-[11px] ${LEVEL_TONE[c.level]}`}>{LEVEL_TEXT[c.level]}</span>
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      )}

      <input
        ref={fileRef}
        type="file"
        accept="image/png,image/jpeg,image/webp,image/gif"
        className="hidden"
        onChange={(e) => {
          const f = e.target.files?.[0];
          e.target.value = "";
          if (f) void upload(f);
        }}
      />
    </div>
  );
}

/** A row's picture in the New event form: what will be used, at a glance,
 *  and a click to choose. */
export function RowArtwork({
  name,
  categoryId,
  choices,
  active,
  onToggle,
  onSubject,
}: {
  name: string;
  categoryId: number | null;
  choices: Record<string, ArtworkChoice>;
  active: boolean;
  onToggle: () => void;
  /** Which subject this row is now - so the form stores only the choices of
   *  rows it still has. */
  onSubject: (identityKey: string | null) => void;
}) {
  const input = name.trim() ? { eventId: null, name, categoryId } : null;
  const { suggestion: s, loading, fresh } = useArtworkSuggestion(input, "search", 900);
  const subject = name.trim() && fresh ? s?.identityKey ?? null : null;
  const report = useRef(onSubject);
  report.current = onSubject;
  useEffect(() => {
    report.current(subject);
  }, [subject]);
  const choice = s ? choices[s.identityKey] ?? null : null;
  const top = s?.candidates.find((c) => c.thumb && c.source !== "earlier");
  const asking = !choice && !s?.decision && !s?.shown && s?.outcome === "ask";
  const preview = choice ? choice.preview : s?.shown ?? (asking && top ? { src: top.thumb!, fit: top.fit, title: top.title, source: top.source } : null);
  const label = !name.trim()
    ? "Picture - type a name first"
    : choice
      ? "Picture: your choice"
      : s?.shown
        ? "Picture found - click to change"
        : asking
          ? "A picture needs a yes - click"
          : loading
            ? "Looking for a picture"
            : "No picture - click to choose";
  return (
    <button
      type="button"
      onClick={onToggle}
      disabled={!name.trim()}
      aria-pressed={active}
      aria-label={label}
      title={label}
      className={`relative rounded-lg ${active ? "ring-2 ring-brand-500" : ""} disabled:opacity-50`}
    >
      <ArtFrame image={preview} name={name || "this event"} size="xs" />
      {asking && (
        <span className="absolute -right-1 -top-1 grid h-3.5 w-3.5 place-items-center rounded-full bg-amber-500 text-[9px] font-bold text-white">
          ?
        </span>
      )}
      {loading && !preview && (
        <span className="absolute inset-0 grid place-items-center">
          <Spinner className="h-3 w-3" />
        </span>
      )}
    </button>
  );
}

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { IconAlertTriangle, IconCheck, IconRefresh, IconUpload, IconX } from "./icons";
import { Button, Spinner } from "./ui";
import { errMsg } from "../lib/api";
import {
  AiImportSession,
  TICKET_GROUP_LABELS,
  fieldLabel,
  imageFromClipboard,
  imageFromDrop,
  prepareImage,
  type AiImportApplied,
  type AiImportImage,
} from "../lib/aiImport";
import type { AiImportConfidence, AiImportKind, AiImportResult, AiImportTicketGroup } from "../lib/types";

/**
 * TIQR Manager - AI Import Assistant panel (2.7.0)
 *
 * marko's own request: one compact panel that sits INSIDE the existing New
 * Event / New Order / New Sale forms - "Nechcem veľký AI dashboard. Nechcem
 * chat. Nechcem AI sidebar cez celú obrazovku." So this is a small block at
 * the top of a form, not a page, not a route, not a tab.
 *
 * What it is allowed to do is exactly one thing: hand `onApply` a bag of
 * strings. It has no access to `api.createEvent`/`createOrder`/`createSale`,
 * it never submits anything, and the form it sits in is unchanged apart from
 * a `fill` function that types those strings into its own existing state.
 * The Save/Create button, the validation, and every backend command are the
 * same ones that were there before this feature existed.
 *
 * The three input routes all end in the same place (`handleFile`):
 *   - **Ctrl+V** - a document-level paste listener, active only while this
 *     panel is mounted and idle. It calls `preventDefault()` ONLY when the
 *     clipboard actually held an image; a normal text paste into a form
 *     field behaves exactly as it always did (marko's section 2: "Ak
 *     clipboard neobsahuje image: nič nerozbíjaj").
 *   - **Drag & drop** - plain HTML drop events, which only reach the webview
 *     because `dragDropEnabled` is false in tauri.conf.json. See
 *     `imageFromDrop`'s comment.
 *   - **Upload** - a hidden `<input type="file">`, no dialog plugin needed.
 *
 * See `lib/aiImport.ts` for the image preparation and the per-session
 * duplicate guard, and `src-tauri/src/commands/ai_import.rs` for the API
 * call and the strict schema.
 */

type Phase = "idle" | "preparing" | "analyzing" | "review" | "error";

const CONFIDENCE_TONE: Record<AiImportConfidence, string> = {
  high: "text-emerald-600 dark:text-emerald-400",
  medium: "text-amber-600 dark:text-amber-400",
  low: "text-red-600 dark:text-red-400",
};

/** Low confidence has to be visible at a glance (marko's section 7: "Fields
 * s nízkou confidence musia byť vizuálne označené") without turning the list
 * into a wall of color - so the dot and the word carry it, and only `low`
 * additionally tints its input. */
function ConfidenceChip({ confidence }: { confidence: AiImportConfidence }) {
  return (
    <span
      className={`inline-flex shrink-0 items-center gap-1 text-[11px] font-medium capitalize ${CONFIDENCE_TONE[confidence]}`}
    >
      <span className="h-1.5 w-1.5 rounded-full bg-current" aria-hidden="true" />
      {confidence}
    </span>
  );
}

export default function AiImportPanel({
  kind,
  onApply,
  className = "",
}: {
  kind: AiImportKind;
  /** Called only when marko presses "Fill form". Never called automatically. */
  onApply: (applied: AiImportApplied) => void;
  className?: string;
}) {
  // One session per mounted panel - this is what makes "the same image is
  // never analyzed twice" true across a close/reopen of the review step.
  const session = useMemo(() => new AiImportSession(kind), [kind]);
  const fileInput = useRef<HTMLInputElement>(null);

  const [phase, setPhase] = useState<Phase>("idle");
  const [image, setImage] = useState<AiImportImage | null>(null);
  const [result, setResult] = useState<AiImportResult | null>(null);
  const [edits, setEdits] = useState<Record<string, string>>({});
  const [groupIndex, setGroupIndex] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [dragging, setDragging] = useState(false);

  const busy = phase === "preparing" || phase === "analyzing";

  const runAnalysis = useCallback(
    async (img: AiImportImage, forceRetry: boolean) => {
      setPhase("analyzing");
      setError(null);
      try {
        const res = forceRetry ? await session.retry(img) : await session.analyze(img);
        setResult(res);
        // Seed the editable copy from what came back. Only non-null values
        // become entries, so a field the image didn't show stays absent all
        // the way through to `onApply` and can never blank a form field.
        const seeded: Record<string, string> = {};
        for (const f of res.fields) {
          if (f.value != null) seeded[f.field] = f.value;
        }
        setEdits(seeded);
        setGroupIndex(0);
        setPhase("review");
      } catch (e) {
        setError(errMsg(e));
        setPhase("error");
      }
    },
    [session],
  );

  const handleFile = useCallback(
    async (file: File) => {
      setPhase("preparing");
      setError(null);
      setResult(null);
      try {
        const img = await prepareImage(file);
        setImage(img);
        await runAnalysis(img, false);
      } catch (e) {
        setImage(null);
        setError(errMsg(e));
        setPhase("error");
      }
    },
    [runAnalysis],
  );

  // Ctrl+V. Deliberately on `document` rather than a focused element: marko
  // pastes without clicking the panel first. It is removed the moment the
  // panel unmounts (the modal closes) and while a request is in flight, so
  // it can never queue a second analysis.
  useEffect(() => {
    if (busy) return;
    const onPaste = (e: ClipboardEvent) => {
      const file = imageFromClipboard(e);
      // No image on the clipboard -> do nothing at all, and in particular do
      // NOT preventDefault: the app's normal paste behaviour has to survive
      // this panel existing.
      if (!file) return;
      e.preventDefault();
      void handleFile(file);
    };
    document.addEventListener("paste", onPaste);
    return () => document.removeEventListener("paste", onPaste);
  }, [busy, handleFile]);

  const reset = () => {
    setPhase("idle");
    setImage(null);
    setResult(null);
    setEdits({});
    setGroupIndex(0);
    setError(null);
  };

  const groups: AiImportTicketGroup[] = result?.ticketGroups ?? [];
  const activeGroup = groups[groupIndex] ?? null;

  const apply = () => {
    const fields: Record<string, string> = {};
    for (const [key, value] of Object.entries(edits)) {
      const trimmed = value.trim();
      if (trimmed) fields[key] = trimmed;
    }
    onApply({ fields, group: activeGroup });
  };

  return (
    <div
      className={`rounded-xl border border-slate-200 bg-slate-50/70 p-3 dark:border-slate-800 dark:bg-slate-800/25 ${className}`}
      onDragOver={(e) => {
        e.preventDefault();
        if (!busy) setDragging(true);
      }}
      onDragLeave={() => setDragging(false)}
      onDrop={(e) => {
        e.preventDefault();
        setDragging(false);
        if (busy) return;
        const file = imageFromDrop(e.dataTransfer?.files ?? null);
        if (file) void handleFile(file);
      }}
    >
      <div className="mb-2 flex items-center justify-between gap-2">
        <p className="section-title">AI import</p>
        {phase !== "idle" && (
          <button
            type="button"
            onClick={reset}
            disabled={busy}
            className="rounded text-[11px] font-medium text-slate-400 transition hover:text-slate-600 disabled:opacity-50 dark:text-slate-500 dark:hover:text-slate-300"
          >
            Clear
          </button>
        )}
      </div>

      <input
        ref={fileInput}
        type="file"
        accept="image/png,image/jpeg,image/webp"
        className="hidden"
        onChange={(e) => {
          const file = e.target.files?.[0];
          // Reset the input's value so picking the SAME file again still
          // fires a change event (the session's own fingerprint cache is
          // what stops that from costing anything).
          e.target.value = "";
          if (file) void handleFile(file);
        }}
      />

      {phase === "idle" && (
        <button
          type="button"
          onClick={() => fileInput.current?.click()}
          className={`flex w-full flex-col items-center gap-1.5 rounded-lg border border-dashed px-3 py-4 text-center transition ${
            dragging
              ? "border-brand-500 bg-brand-50/70 dark:bg-brand-500/10"
              : "border-slate-300 hover:border-slate-400 hover:bg-white dark:border-slate-700 dark:hover:border-slate-600 dark:hover:bg-slate-900/50"
          }`}
        >
          <IconUpload className="h-4 w-4 text-slate-400 dark:text-slate-500" />
          <span className="text-xs font-medium text-slate-700 dark:text-slate-300">
            Drop a screenshot, or click to upload
          </span>
          <span className="text-[11px] text-slate-400 dark:text-slate-500">
            or press Ctrl + V to paste one &middot; PNG, JPG, WebP
          </span>
        </button>
      )}

      {phase !== "idle" && (
        <div className="flex items-start gap-3">
          {image && (
            <img
              src={image.previewUrl}
              alt=""
              className="h-16 w-16 shrink-0 rounded-lg border border-slate-200 object-cover dark:border-slate-700"
            />
          )}
          <div className="min-w-0 flex-1">
            {busy && (
              <p className="flex items-center gap-2 py-3 text-xs font-medium text-slate-500 dark:text-slate-400">
                <Spinner className="h-3.5 w-3.5" />
                {phase === "preparing" ? "Preparing image..." : "Analyzing image..."}
              </p>
            )}

            {phase === "error" && (
              <div className="py-1">
                <p className="flex items-start gap-1.5 text-xs font-medium text-red-600 dark:text-red-400">
                  <IconAlertTriangle className="mt-px h-3.5 w-3.5 shrink-0" />
                  {error ?? "AI analysis failed. Try again."}
                </p>
                <div className="mt-2.5 flex gap-2">
                  {/* The image is deliberately still here, so Retry costs
                      marko nothing but a click - section 13. */}
                  {image && (
                    <Button size="sm" onClick={() => void runAnalysis(image, true)}>
                      <IconRefresh className="h-3.5 w-3.5" /> Retry
                    </Button>
                  )}
                  <Button size="sm" variant="ghost" onClick={reset}>
                    Remove
                  </Button>
                </div>
              </div>
            )}

            {phase === "review" && result && !result.readable && (
              <div className="py-1">
                <p className="flex items-start gap-1.5 text-xs font-medium text-amber-600 dark:text-amber-400">
                  <IconAlertTriangle className="mt-px h-3.5 w-3.5 shrink-0" />
                  Image quality too low to reliably extract data.
                </p>
                <div className="mt-2.5 flex gap-2">
                  {image && (
                    <Button size="sm" onClick={() => void runAnalysis(image, true)}>
                      <IconRefresh className="h-3.5 w-3.5" /> Retry
                    </Button>
                  )}
                  <Button size="sm" variant="ghost" onClick={reset}>
                    Remove
                  </Button>
                </div>
              </div>
            )}

            {phase === "review" && result?.readable && (
              <p className="text-xs font-medium text-slate-700 dark:text-slate-300">
                Review extracted data
              </p>
            )}
          </div>
        </div>
      )}

      {phase === "review" && result?.readable && (
        <div className="mt-3">
          {result.fields.length === 0 && groups.length === 0 ? (
            <p className="text-xs text-slate-500 dark:text-slate-400">
              Could not reliably identify required fields.
            </p>
          ) : (
            <>
              <div className="space-y-1.5">
                {result.fields.map((f) => {
                  const value = edits[f.field] ?? "";
                  const missing = f.value == null;
                  return (
                    <div key={f.field} className="flex items-center gap-2">
                      <span className="w-28 shrink-0 truncate text-[11px] text-slate-500 dark:text-slate-400">
                        {fieldLabel(f.field)}
                      </span>
                      <input
                        // `input-error`, not `field-invalid`: the latter is
                        // the WRAPPER class <Field> puts around a control
                        // (index.css defines it as `.field-invalid .input`),
                        // and this input has no such wrapper.
                        className={`input h-7 flex-1 py-0 text-xs ${
                          !missing && f.confidence === "low" ? "input-error" : ""
                        }`}
                        value={value}
                        placeholder={missing ? "not on the image" : ""}
                        onChange={(e) =>
                          // Purely local - editing a value never re-runs the
                          // analysis (marko's section 14).
                          setEdits((prev) => ({ ...prev, [f.field]: e.target.value }))
                        }
                      />
                      {!missing && <ConfidenceChip confidence={f.confidence} />}
                    </div>
                  );
                })}
              </div>

              {groups.length > 0 && activeGroup && (
                <div className="mt-3 rounded-lg border border-slate-200 bg-white p-2.5 dark:border-slate-700 dark:bg-slate-900/60">
                  <div className="mb-2 flex flex-wrap items-center justify-between gap-2">
                    <p className="section-title">
                      {groups.length === 1 ? "Tickets" : `Ticket group ${groupIndex + 1} of ${groups.length}`}
                    </p>
                    {groups.length > 1 && (
                      <div className="flex flex-wrap gap-1">
                        {groups.map((_, i) => (
                          <button
                            key={i}
                            type="button"
                            onClick={() => setGroupIndex(i)}
                            className={`rounded px-1.5 py-0.5 text-[11px] font-medium transition ${
                              i === groupIndex
                                ? "bg-brand-600 text-white"
                                : "text-slate-500 hover:bg-slate-100 dark:text-slate-400 dark:hover:bg-slate-800"
                            }`}
                          >
                            {i + 1}
                          </button>
                        ))}
                      </div>
                    )}
                  </div>
                  {groups.length > 1 && (
                    // Two groups can't be one order: OrderInput carries a
                    // single section/row/tier/price for the whole order (see
                    // ai_import.rs's AiImportTicketGroup comment). Saying so
                    // out loud beats silently filling one and losing the
                    // other.
                    <p className="mb-2 text-[11px] leading-relaxed text-slate-500 dark:text-slate-400">
                      This screenshot has {groups.length} groups with different seating or prices.
                      An order holds one, so fill this group first, then create a second order for
                      the next.
                    </p>
                  )}
                  <dl className="grid grid-cols-2 gap-x-3 gap-y-1">
                    {(Object.keys(TICKET_GROUP_LABELS) as (keyof typeof TICKET_GROUP_LABELS)[]).map(
                      (key) =>
                        activeGroup[key] ? (
                          <div key={key} className="flex items-baseline gap-1.5 overflow-hidden">
                            <dt className="shrink-0 text-[11px] text-slate-400 dark:text-slate-500">
                              {TICKET_GROUP_LABELS[key]}
                            </dt>
                            <dd className="truncate text-xs font-medium text-slate-800 tabular-nums dark:text-slate-200">
                              {activeGroup[key]}
                            </dd>
                          </div>
                        ) : null,
                    )}
                    {activeGroup.seats.length > 0 && (
                      <div className="col-span-2 flex items-baseline gap-1.5 overflow-hidden">
                        <dt className="shrink-0 text-[11px] text-slate-400 dark:text-slate-500">Seats</dt>
                        <dd className="truncate text-xs font-medium text-slate-800 tabular-nums dark:text-slate-200">
                          {activeGroup.seats.join(", ")}
                        </dd>
                      </div>
                    )}
                  </dl>
                </div>
              )}

              <div className="mt-3 flex flex-wrap items-center gap-2">
                <Button size="sm" variant="primary" onClick={apply}>
                  <IconCheck className="h-3.5 w-3.5" /> Fill form
                </Button>
                {image && (
                  <Button size="sm" variant="ghost" onClick={() => void runAnalysis(image, true)}>
                    <IconRefresh className="h-3.5 w-3.5" /> Re-analyze
                  </Button>
                )}
                <Button size="sm" variant="ghost" onClick={reset}>
                  <IconX className="h-3.5 w-3.5" /> Discard
                </Button>
                <span className="ml-auto text-[11px] text-slate-400 dark:text-slate-500">
                  Nothing is saved until you submit the form
                </span>
              </div>
            </>
          )}
        </div>
      )}
    </div>
  );
}

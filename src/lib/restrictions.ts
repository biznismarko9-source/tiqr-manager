/**
 * Ticket restrictions — marko: *"chcem aby sme vedeli pridat restriction pri
 * listkov napr ako restricted view, 16+ atd, take najhlavnejsie"*.
 *
 * These are the things a buyer has to be told BEFORE they pay. A restricted
 * view seat sold as a normal one is a refund; an 18+ ticket sold to someone
 * who cannot use it is a refund and an argument.
 *
 * ## The codes are the data, the labels are not
 *
 * Migration 037 stores short codes in `tickets.restrictions_json`. The labels
 * live here, in one place, so renaming one is a text edit and not a data
 * migration — and so the same words appear in Inventory, in Sales and in the
 * editor without three copies drifting apart.
 *
 * An unknown code (from a newer version, after a sync) is shown as-is rather
 * than dropped, both here and in Rust.
 */

export type RestrictionTone = "warn" | "info" | "neutral";

export const TICKET_RESTRICTIONS: {
  code: string;
  label: string;
  short: string;
  tone: RestrictionTone;
}[] = [
  // 2.72.0: English, and only the ones that actually come up - marko:
  // "restrictions by mali byt napisane v anglictine, a mali by obsahovat len
  // zakladne restrictions ake mozu byt".
  //
  // Safe to rewrite wholesale: no ticket in the database had a restriction
  // set when this changed (checked, not assumed - zero rows with a non-empty
  // `restrictions_json`), so nothing was orphaned by dropping a code. If that
  // is ever false again, a dropped code still RENDERS - `restrictionShort`
  // falls back to the code itself - but it stops being selectable.
  //
  // `ao` (minor with an adult) went: 16+/18+ already carry the age question,
  // and marko asked for a basic list, not a complete one.
  //
  // `aw` is new and is the football one that actually bites on resale - an
  // away-end seat sold to a home fan is a refund and a turned-away buyer.

  // The ones that cost money when they are missed come first.
  { code: "rv", label: "Restricted view", short: "Restricted", tone: "warn" },
  { code: "18", label: "18+", short: "18+", tone: "warn" },
  { code: "16", label: "16+", short: "16+", tone: "warn" },
  { code: "id", label: "ID required", short: "ID", tone: "warn" },
  { code: "aw", label: "Away fans only", short: "Away end", tone: "warn" },
  // Things the buyer must know, but that do not void the ticket.
  { code: "nr", label: "No re-entry", short: "No re-entry", tone: "info" },
  { code: "st", label: "Standing", short: "Standing", tone: "neutral" },
  { code: "wc", label: "Wheelchair space", short: "Wheelchair", tone: "neutral" },
];

const BY_CODE = new Map(TICKET_RESTRICTIONS.map((r) => [r.code, r]));

/** The short label for a badge. An unknown code shows itself. */
export function restrictionShort(code: string): string {
  return BY_CODE.get(code)?.short ?? code;
}

/** The full label, for tooltips and the editor. */
export function restrictionLabel(code: string): string {
  return BY_CODE.get(code)?.label ?? code;
}

export function restrictionTone(code: string): RestrictionTone {
  return BY_CODE.get(code)?.tone ?? "neutral";
}

export const RESTRICTION_TONE_CLASS: Record<RestrictionTone, string> = {
  warn: "bg-amber-100 text-amber-800 dark:bg-amber-950/60 dark:text-amber-300",
  info: "bg-blue-100 text-blue-800 dark:bg-blue-950/60 dark:text-blue-300",
  neutral: "bg-slate-100 text-slate-600 dark:bg-slate-800 dark:text-slate-300",
};

/** `["rv","18"]` → `"Restricted view · 18+"`, for a tooltip. */
export function restrictionsTitle(codes: string[]): string {
  return codes.map(restrictionLabel).join(" · ");
}

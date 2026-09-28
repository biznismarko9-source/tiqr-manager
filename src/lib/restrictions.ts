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
  // The ones that cost money when they are missed come first.
  { code: "rv", label: "Obmedzený výhľad", short: "Výhľad", tone: "warn" },
  { code: "18", label: "18+", short: "18+", tone: "warn" },
  { code: "16", label: "16+", short: "16+", tone: "warn" },
  { code: "id", label: "Na meno / treba doklad", short: "Doklad", tone: "warn" },
  { code: "nr", label: "Bez opätovného vstupu", short: "Bez návratu", tone: "info" },
  { code: "ao", label: "Maloletý len s dospelým", short: "S dospelým", tone: "info" },
  { code: "st", label: "Na státie", short: "Státie", tone: "neutral" },
  { code: "wc", label: "Bezbariérové miesto", short: "Bezbariér.", tone: "neutral" },
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

/** `["rv","18"]` → `"Obmedzený výhľad · 18+"`, for a tooltip. */
export function restrictionsTitle(codes: string[]): string {
  return codes.map(restrictionLabel).join(" · ");
}

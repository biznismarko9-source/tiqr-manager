/**
 * 2.0.27: fixed-order categorical palette for event categories (Events/
 * Orders/Sales filtering + color-coding, marko's own request - "vyfiltrovat
 * eventy podla toho ci to je futbal koncert atd a aj nafarbit ich nejak, tak
 * aby to ostalo v teme"). Chosen with the dataviz skill: colorblind-safe in
 * both light and dark mode (validated with scripts/validate_palette.js), and
 * deliberately drawn from Tailwind families STATUS_TONES (ui.tsx) never
 * uses - green/sky/rose/teal/indigo/cyan/purple/orange, vs. STATUS_TONES'
 * slate/blue/emerald/red/amber/violet - so a category badge is never
 * confusable with a status badge at a glance.
 *
 * Every class name below is a full literal string (never built with a
 * template literal like `bg-${family}-50`) because Tailwind's JIT scans
 * source text for complete class names - a dynamically-assembled name would
 * silently be missing from the compiled CSS. Same convention STATUS_TONES
 * itself already follows.
 *
 * Indexed by `colorSlot % length` (see event_categories.rs's `color_slot`
 * doc comment: a plain integer assigned once at creation via
 * `MAX(color_slot)+1`, never recomputed) so this degrades gracefully past 8
 * categories - colors repeat rather than erroring. NEVER reorder this array
 * or change what index an entry lives at - assign categorical hues in fixed
 * order, never cycled (the dataviz skill's central rule) - a category
 * that's always rendered "rose" must keep rendering rose even after new
 * categories are added later, since `colorSlot` values already stored in
 * the database point at fixed positions here.
 */
const CATEGORY_TONES = [
  {
    badge: "bg-green-50 text-green-600 ring-green-200 dark:bg-green-500/10 dark:text-green-600 dark:ring-green-500/30",
    dot: "bg-green-500",
  },
  {
    badge: "bg-sky-50 text-sky-600 ring-sky-200 dark:bg-sky-500/10 dark:text-sky-600 dark:ring-sky-500/30",
    dot: "bg-sky-500",
  },
  {
    badge: "bg-rose-50 text-rose-600 ring-rose-200 dark:bg-rose-500/10 dark:text-rose-500 dark:ring-rose-500/30",
    dot: "bg-rose-500",
  },
  {
    badge: "bg-teal-50 text-teal-600 ring-teal-200 dark:bg-teal-500/10 dark:text-teal-600 dark:ring-teal-500/30",
    dot: "bg-teal-500",
  },
  {
    badge: "bg-indigo-50 text-indigo-600 ring-indigo-200 dark:bg-indigo-500/10 dark:text-indigo-500 dark:ring-indigo-500/30",
    dot: "bg-indigo-500",
  },
  {
    badge: "bg-cyan-50 text-cyan-600 ring-cyan-200 dark:bg-cyan-500/10 dark:text-cyan-600 dark:ring-cyan-500/30",
    dot: "bg-cyan-500",
  },
  {
    badge: "bg-purple-50 text-purple-600 ring-purple-200 dark:bg-purple-500/10 dark:text-purple-500 dark:ring-purple-500/30",
    dot: "bg-purple-500",
  },
  {
    badge: "bg-orange-50 text-orange-600 ring-orange-200 dark:bg-orange-500/10 dark:text-orange-600 dark:ring-orange-500/30",
    dot: "bg-orange-500",
  },
];

/** Safe even for a stray negative slot (shouldn't happen - color_slot is
 * always assigned via MAX(color_slot)+1, starting at 0 - but this is cheap
 * insurance against ever indexing out of bounds). */
function toneFor(colorSlot: number) {
  const len = CATEGORY_TONES.length;
  const i = ((colorSlot % len) + len) % len;
  return CATEGORY_TONES[i];
}

/** The pill shown in Events/Orders/Sales tables - name + fixed color, same
 * visual idiom as `Badge` (ui.tsx) but keyed by `colorSlot` instead of a
 * hardcoded status string. */
export function EventCategoryBadge({ name, colorSlot }: { name: string; colorSlot: number }) {
  return (
    <span
      className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium ring-1 ring-inset ${toneFor(colorSlot).badge}`}
    >
      {name}
    </span>
  );
}

/** A small solid-color dot, no text - for Settings' Event Categories list,
 * where the name is already shown as its own text right next to it (a full
 * badge there would just repeat it, e.g. "Concert [Concert]"). */
export function EventCategorySwatch({ colorSlot, className = "" }: { colorSlot: number; className?: string }) {
  return <span className={`inline-block h-2.5 w-2.5 shrink-0 rounded-full ${toneFor(colorSlot).dot} ${className}`} />;
}

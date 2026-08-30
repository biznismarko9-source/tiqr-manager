/**
 * 2.0.83: fixed-order categorical palette for Finance categories (Settings ->
 * Lookups, Finance.tsx's category filter/breakdown chart) - byte-for-byte the
 * same 8-tone palette as EventCategoryBadge.tsx's own CATEGORY_TONES (already
 * validated colorblind-safe with the dataviz skill), copied into its OWN
 * array here rather than imported from that file. Deliberately a separate
 * copy, not a shared one: `colorSlot` values for Finance categories and Event
 * categories are two completely independent sequences (each assigned via its
 * own table's own `MAX(color_slot)+1`, see finance_entries.rs/
 * event_categories.rs), so sharing one array would only invite a future edit
 * to one feature's palette to silently affect the other's already-stored
 * slots. The two features' badges are never shown side by side, so there is
 * no visual-collision downside to the colors happening to match.
 *
 * Every rule from EventCategoryBadge.tsx's own doc comment applies
 * identically here - see that file for the full reasoning. In short: full
 * literal class name strings only (Tailwind JIT), indexed by
 * `colorSlot % length`, and this array must NEVER be reordered.
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

/** Safe even for a stray negative slot - cheap insurance against ever
 * indexing out of bounds, same as EventCategoryBadge.tsx's own `toneFor`. */
function toneFor(colorSlot: number) {
  const len = CATEGORY_TONES.length;
  const i = ((colorSlot % len) + len) % len;
  return CATEGORY_TONES[i];
}

/** The pill shown on Finance's entries list/filters - name + fixed color,
 * same visual idiom as `EventCategoryBadge`. */
export function FinanceCategoryBadge({ name, colorSlot }: { name: string; colorSlot: number }) {
  return (
    <span
      className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium ring-1 ring-inset ${toneFor(colorSlot).badge}`}
    >
      {name}
    </span>
  );
}

/** A small solid-color dot, no text - for Settings' Finance Categories lists,
 * where the name is already shown as its own text right next to it. */
export function FinanceCategorySwatch({ colorSlot, className = "" }: { colorSlot: number; className?: string }) {
  return <span className={`inline-block h-2.5 w-2.5 shrink-0 rounded-full ${toneFor(colorSlot).dot} ${className}`} />;
}

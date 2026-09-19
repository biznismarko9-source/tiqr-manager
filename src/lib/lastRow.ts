/** 2.40.0: which row you left a list from, so returning to it can say so.
 *
 * marko picked "hlavička, ktorá zostane" out of thirty. Half of that card -
 * the sticky table header - has shipped since 2.6.0 (`.table-shell thead th`
 * in index.css); this file is the other half: after opening a record and
 * coming back, the row you opened lights up for a moment, so you pick up
 * where you were instead of re-finding your place in eighty rows.
 *
 * Module-level and session-only, the exact same convention Sales.tsx's own
 * `lastFilters` and Orders.tsx's `lastOrdersSearch` already use: it survives
 * a route change, it does not survive a restart, and it never touches the
 * database. Keyed by list so two lists can never flash each other's row.
 *
 * The mark is consumed on read (`takeRow` deletes it), so a row flashes once
 * per departure - not every time you visit the list afterwards.
 */
const marks = new Map<string, number>();

/** Called as a list navigates into a record's own page. */
export function markRow(list: string, id: number): void {
  marks.set(list, id);
}

/** Read once, at list mount. Returns null when you did not arrive by Back. */
export function takeRow(list: string): number | null {
  const id = marks.get(list);
  if (id === undefined) return null;
  marks.delete(list);
  return id;
}

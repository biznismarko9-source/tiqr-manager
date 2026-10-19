// 2.112.0 - "something changed underneath you", for the screens.
//
// Live sync (lib/liveSync.ts) writes the other computer's changes straight
// into this database while a screen is open on it. The screens load what they
// show once and then hold it, so without this they would keep showing the old
// order until marko clicked away and back - the opposite of "everything
// works as one".
//
// Changes arrive in bursts (a page of records, then the ones that were
// waiting for them), so they are gathered for a moment and announced once.
import { useEffect, useRef } from "react";

const CHANGED = "tiqr:data-changed";
const GATHER_MS = 250;

let gathered = new Set<string>();
let timer: ReturnType<typeof setTimeout> | null = null;

/** Tables whose records changed here, by name (`orders`, `tickets`, ...). */
export function publishDataChanged(tables: string[]) {
  if (!tables.length) return;
  for (const t of tables) gathered.add(t);
  if (timer) return;
  timer = setTimeout(() => {
    const detail = [...gathered];
    gathered = new Set();
    timer = null;
    window.dispatchEvent(new CustomEvent<string[]>(CHANGED, { detail }));
  }, GATHER_MS);
}

export function onDataChanged(handler: (tables: string[]) => void): () => void {
  const listener = (e: Event) => handler((e as CustomEvent<string[]>).detail);
  window.addEventListener(CHANGED, listener);
  return () => window.removeEventListener(CHANGED, listener);
}

/**
 * Runs `reload` - the screen's own, latest one, with whatever filters it has
 * now - whenever one of `tables` changed. No list means any change.
 */
export function useDataChanged(reload: () => void, tables?: readonly string[]) {
  const latest = useRef(reload);
  latest.current = reload;
  const key = tables ? tables.join(",") : "";
  useEffect(() => {
    const wanted = key ? key.split(",") : null;
    return onDataChanged((changed) => {
      if (!wanted || changed.some((t) => wanted.includes(t))) latest.current();
    });
  }, [key]);
}

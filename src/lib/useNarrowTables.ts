import { useEffect, useState } from "react";

/** Below this window width, list tables switch from their normal (wide)
 * column set to a narrower one: a handful of secondary columns hide (their
 * data is still one click away, and NEVER the row's main identifier/amount
 * columns - see PROTECTED-AREAS-NOTES.md for the exact per-table drop
 * list), remaining columns get a smaller font and tighter padding, and every
 * column's percentage share is resized so the table can never need
 * horizontal scrolling, all the way down to the app's enforced 1080px
 * minimum window width. Verified (Playwright, real locale-formatted data
 * across en-US/sk-SK/de-DE) against the app's true content-width floor -
 * see the 2.0.37 section of PROTECTED-AREAS-NOTES.md before changing this
 * number, both modes were sized against it specifically. */
const NARROW_BREAKPOINT_PX = 1690;

/** Reactive "is the window narrow enough that tables must switch to compact
 * mode" flag, shared by every list/detail table in the app so they all
 * switch at exactly the same window width (this is what makes every table
 * look the same, per marko's request - one shared threshold, not a
 * per-table guess). Uses a matchMedia listener (not a raw resize listener)
 * so it only re-renders when the window actually crosses the threshold,
 * not on every pixel of a drag-resize. */
export function useNarrowTables(): boolean {
  const [isNarrow, setIsNarrow] = useState(
    () => typeof window !== "undefined" && window.innerWidth < NARROW_BREAKPOINT_PX,
  );

  useEffect(() => {
    const mq = window.matchMedia(`(max-width: ${NARROW_BREAKPOINT_PX - 1}px)`);
    const onChange = () => setIsNarrow(mq.matches);
    onChange();
    mq.addEventListener("change", onChange);
    return () => mq.removeEventListener("change", onChange);
  }, []);

  return isNarrow;
}

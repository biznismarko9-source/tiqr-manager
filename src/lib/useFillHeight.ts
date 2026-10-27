// 2.113.0 - a block that reaches the bottom of the window, whatever sits
// above it.
//
// Layout gives pages no height (PROTECTED_AREAS 2.55.0), so a screen that
// wants panes down to the bottom has to say how tall they are. A fixed
// `calc(100vh - 266px)` guessed what was above the Mail panes and guessed
// ~98px too much - marko: "tie ramceky posunut nizsie lebo tam je priestor".
// This measures instead: the window's height minus where the block starts,
// re-measured on resize and once a second (a banner appearing above it moves
// it without resizing anything this could observe).
import { useEffect, useLayoutEffect, useState, type RefObject } from "react";

export function useFillHeight(ref: RefObject<HTMLElement | null>, bottomGap = 20, min = 440): number | null {
  const [height, setHeight] = useState<number | null>(null);
  useLayoutEffect(() => {
    const measure = () => {
      const el = ref.current;
      if (!el) return;
      // 2.117.0: on a phone the tab bar covers the bottom of the window
      // (Layout's PhoneNav); elsewhere it is not shown and has no size.
      const nav = document.querySelector("[data-phone-nav]")?.getBoundingClientRect();
      const bottom = nav && nav.height > 0 ? nav.top : window.innerHeight;
      const next = Math.max(min, Math.floor(bottom - el.getBoundingClientRect().top - bottomGap));
      setHeight((cur) => (cur === next ? cur : next));
    };
    measure();
    window.addEventListener("resize", measure);
    const timer = window.setInterval(measure, 1000);
    return () => {
      window.removeEventListener("resize", measure);
      window.clearInterval(timer);
    };
  }, [ref, bottomGap, min]);
  return height;
}

/** For screens that render the block only after loading: measure again once it exists. */
export function useRemeasure(deps: unknown[]) {
  useEffect(() => {
    window.dispatchEvent(new Event("resize"));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);
}

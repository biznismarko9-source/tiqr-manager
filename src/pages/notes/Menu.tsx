import { useEffect, useRef } from "react";
import type { MouseEvent as ReactMouseEvent } from "react";

/**
 * The "⋯" menu, for sub-tabs and for notes (2.62.0).
 *
 * ## Why it is positioned in viewport coordinates
 *
 * The obvious build - an absolutely positioned panel inside the button's own
 * parent - cannot work here. The sub-tab strip is a horizontal scroller
 * (`overflow-x-auto`), and a scroller clips anything inside it in BOTH
 * directions, so the panel would be cut off at the strip's edge. Taking the
 * button's own rect and drawing at those fixed coordinates is the only
 * placement that survives it, and it costs nothing in the note list either.
 *
 * `up` is for the strip along the bottom of the editor: there is no room
 * below it, so the menu grows upwards from the button instead.
 */

export type MenuItem = { label: string; onClick: () => void; danger?: boolean; disabled?: boolean } | "-";

export type MenuAt = { x: number; y: number; up: boolean };

/** Where to draw, taken from the button that was clicked. */
export function menuAt(e: ReactMouseEvent<HTMLElement>, up = false): MenuAt {
  const r = e.currentTarget.getBoundingClientRect();
  return { x: r.right, y: up ? r.top - 4 : r.bottom + 4, up };
}

export default function Menu({
  at,
  items,
  onClose,
}: {
  at: MenuAt | null;
  items: MenuItem[];
  onClose: () => void;
}) {
  const box = useRef<HTMLDivElement>(null);
  const close = useRef(onClose);
  close.current = onClose;

  useEffect(() => {
    if (!at) return;
    const away = (e: MouseEvent) => {
      if (!box.current?.contains(e.target as Node)) close.current();
    };
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape") close.current();
    };
    // The click that opened the menu is still travelling; listening now would
    // close it again in the same gesture.
    const t = setTimeout(() => document.addEventListener("mousedown", away), 0);
    document.addEventListener("keydown", key);
    return () => {
      clearTimeout(t);
      document.removeEventListener("mousedown", away);
      document.removeEventListener("keydown", key);
    };
  }, [at]);

  if (!at) return null;
  return (
    <div
      ref={box}
      role="menu"
      className="fixed z-[70] min-w-[196px] overflow-hidden rounded-lg bg-surface py-1 shadow-overlay ring-1 ring-slate-200 dark:ring-slate-700"
      style={{ left: at.x, top: at.y, transform: `translate(-100%, ${at.up ? "-100%" : "0"})` }}
    >
      {items.map((it, i) =>
        it === "-" ? (
          <div key={i} className="my-1 h-px bg-slate-200 dark:bg-slate-700" />
        ) : (
          <button
            key={i}
            type="button"
            role="menuitem"
            disabled={it.disabled}
            onClick={() => {
              onClose();
              it.onClick();
            }}
            className={`flex w-full items-center px-3 py-1.5 text-left text-[12.5px] transition hover:bg-surface-sunken disabled:opacity-40 disabled:hover:bg-transparent ${
              it.danger ? "text-red-600 dark:text-red-400" : "text-slate-700 dark:text-slate-200"
            }`}
          >
            {it.label}
          </button>
        ),
      )}
    </div>
  );
}

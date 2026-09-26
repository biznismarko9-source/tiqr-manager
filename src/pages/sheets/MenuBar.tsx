import { useEffect, useRef, useState } from "react";

/**
 * The File / Edit / View / Insert / Format / Data bar.
 *
 * marko: "toto co je hore ze file edit data to je good aj tie zakladne
 * funkcie". It is the part of a spreadsheet everybody already knows how to
 * use, so it carries the actions instead of a wall of toolbar icons.
 *
 * Deliberately dumb: it renders what it is handed and calls back. Every action
 * lives with the thing it acts on (the grid, the sheet list), which is what
 * keeps this file from turning into a second copy of the app's logic.
 */

export type MenuItem =
  | { kind: "item"; label: string; hint?: string; danger?: boolean; onClick: () => void }
  | { kind: "sep" }
  | { kind: "check"; label: string; on: boolean; onClick: () => void };

export type Menu = { label: string; items: MenuItem[] };

export default function MenuBar({ menus }: { menus: Menu[] }) {
  const [open, setOpen] = useState<number | null>(null);
  const barRef = useRef<HTMLDivElement>(null);

  // Click anywhere else, or press Escape, and the menu closes.
  useEffect(() => {
    if (open === null) return;
    const away = (e: MouseEvent) => {
      if (!barRef.current?.contains(e.target as Node)) setOpen(null);
    };
    const esc = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(null);
    };
    document.addEventListener("mousedown", away);
    document.addEventListener("keydown", esc);
    return () => {
      document.removeEventListener("mousedown", away);
      document.removeEventListener("keydown", esc);
    };
  }, [open]);

  return (
    <div ref={barRef} className="relative flex items-center gap-0.5 text-[12.5px]">
      {menus.map((m, i) => (
        <div key={m.label} className="relative">
          <button
            type="button"
            onClick={() => setOpen((o) => (o === i ? null : i))}
            // Once one menu is open, sliding across the bar opens the next —
            // the behaviour every desktop menu bar has.
            onMouseEnter={() => setOpen((o) => (o === null ? o : i))}
            className={`rounded px-2.5 py-1 transition ${
              open === i
                ? "bg-surface-sunken text-slate-900 dark:text-slate-50"
                : "text-slate-600 hover:bg-surface-sunken dark:text-slate-300"
            }`}
          >
            {m.label}
          </button>
          {open === i && (
            <div className="absolute left-0 top-full z-40 mt-0.5 min-w-[216px] overflow-hidden rounded-md border border-slate-200 bg-surface py-1 shadow-xl dark:border-slate-700">
              {m.items.map((it, k) =>
                it.kind === "sep" ? (
                  <div key={k} className="my-1 border-t border-slate-200 dark:border-slate-700" />
                ) : (
                  <button
                    key={k}
                    type="button"
                    onClick={() => {
                      setOpen(null);
                      it.onClick();
                    }}
                    className={`flex w-full items-center gap-3 px-3 py-1.5 text-left transition hover:bg-surface-sunken ${
                      it.kind === "item" && it.danger
                        ? "text-red-600 dark:text-red-400"
                        : "text-slate-700 dark:text-slate-200"
                    }`}
                  >
                    {it.kind === "check" && (
                      <span className="w-3 text-brand-600 dark:text-brand-400">{it.on ? "✓" : ""}</span>
                    )}
                    <span className="flex-1">{it.label}</span>
                    {it.kind === "item" && it.hint && (
                      <span className="text-[11px] text-slate-400 dark:text-slate-500">{it.hint}</span>
                    )}
                  </button>
                ),
              )}
            </div>
          )}
        </div>
      ))}
    </div>
  );
}

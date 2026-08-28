import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from "react";

type ToastKind = "success" | "error" | "info";
interface ToastItem {
  id: number;
  kind: ToastKind;
  message: string;
  /** 2.0.74: true for the last ~180ms of a toast's life, while it's
   * playing its exit animation (`toast-out` in index.css) - see push()
   * below for why removal is two-staged instead of instant. */
  leaving: boolean;
}
interface ToastContextValue {
  push: (kind: ToastKind, message: string) => void;
  success: (message: string) => void;
  error: (message: string) => void;
  info: (message: string) => void;
}

const ToastContext = createContext<ToastContextValue | null>(null);

// 2.0.74: how long the `toast-out` exit animation (index.css) takes - kept
// as one named constant since it has to agree with the animation's own
// duration below (the class strings need the same number spelled out again,
// Tailwind's arbitrary-value syntax can't read a JS variable) and with the
// second setTimeout that actually removes the item once the animation has
// had time to finish.
const TOAST_EXIT_MS = 180;

export function ToastProvider({ children }: { children: ReactNode }) {
  const [items, setItems] = useState<ToastItem[]>([]);

  const push = useCallback((kind: ToastKind, message: string) => {
    const id = Date.now() + Math.random();
    setItems((prev) => [...prev, { id, kind, message, leaving: false }]);
    window.setTimeout(() => {
      // Marks it "leaving" first (plays the exit animation) rather than
      // removing it outright - a toast auto-dismissing with nobody having
      // clicked anything reads as a glitch if it just vanishes instantly.
      setItems((prev) => prev.map((i) => (i.id === id ? { ...i, leaving: true } : i)));
      window.setTimeout(() => {
        setItems((prev) => prev.filter((i) => i.id !== id));
      }, TOAST_EXIT_MS);
    }, 5000);
  }, []);

  const value = useMemo<ToastContextValue>(
    () => ({
      push,
      success: (m: string) => push("success", m),
      error: (m: string) => push("error", m),
      info: (m: string) => push("info", m),
    }),
    [push],
  );

  return (
    <ToastContext.Provider value={value}>
      {children}
      <div className="pointer-events-none fixed bottom-4 right-4 z-[100] flex w-96 max-w-[90vw] flex-col gap-2">
        {items.map((i) => (
          <div
            key={i.id}
            role="status"
            className={
              "pointer-events-auto rounded-lg border px-4 py-3 text-sm font-medium shadow-lg " +
              (i.leaving ? "animate-[toast-out_.18s_ease-in_forwards]" : "animate-[pop-in_.18s_ease-out]") +
              " " +
              (i.kind === "error"
                ? "border-red-200 bg-red-50 text-red-800 dark:border-red-500/30 dark:bg-red-500/10 dark:text-red-400"
                : i.kind === "success"
                  ? "border-emerald-200 bg-emerald-50 text-emerald-800 dark:border-emerald-500/30 dark:bg-emerald-500/10 dark:text-emerald-400"
                  : "border-slate-700 bg-slate-800 text-white dark:border-slate-600 dark:bg-slate-700")
            }
          >
            {i.message}
          </div>
        ))}
      </div>
    </ToastContext.Provider>
  );
}

export function useToast(): ToastContextValue {
  const ctx = useContext(ToastContext);
  if (!ctx) throw new Error("useToast must be used inside <ToastProvider>");
  return ctx;
}

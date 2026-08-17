import { useCallback, useEffect, useState } from "react";
import { api } from "./api";

export type ThemeMode = "light" | "dark" | "system";

const SETTING_KEY = "theme";

function systemPrefersDark(): boolean {
  return typeof window !== "undefined" && window.matchMedia("(prefers-color-scheme: dark)").matches;
}

function applyMode(mode: ThemeMode) {
  const dark = mode === "dark" || (mode === "system" && systemPrefersDark());
  document.documentElement.classList.toggle("dark", dark);
}

function isThemeMode(value: string | null): value is ThemeMode {
  return value === "light" || value === "dark" || value === "system";
}

/** Loads the persisted theme preference (stored locally via app_settings),
 * applies it to <html> as a "dark" class for Tailwind's dark: variants,
 * and keeps it synced with the OS setting while in "system" mode. Every
 * caller gets its own reactive `mode`, but `setMode` always updates the
 * real <html> class and persists immediately, so multiple components
 * (e.g. a launch-time applier and the Settings toggle) stay consistent. */
export function useTheme(): [ThemeMode, (mode: ThemeMode) => void] {
  const [mode, setModeState] = useState<ThemeMode>("system");

  useEffect(() => {
    let cancelled = false;
    api
      .getAppSetting(SETTING_KEY)
      .then((value) => {
        if (cancelled) return;
        const loaded = isThemeMode(value) ? value : "system";
        setModeState(loaded);
        applyMode(loaded);
      })
      .catch(() => applyMode("system"));
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (mode !== "system") return;
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = () => applyMode("system");
    mq.addEventListener("change", onChange);
    return () => mq.removeEventListener("change", onChange);
  }, [mode]);

  const setMode = useCallback((next: ThemeMode) => {
    setModeState(next);
    applyMode(next);
    api.setAppSetting(SETTING_KEY, next).catch(() => {});
  }, []);

  return [mode, setMode];
}

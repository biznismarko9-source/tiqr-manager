import { useCallback, useEffect, useState } from "react";
import { api } from "./api";

/** 2.0.59: shared "Active vs Completed" list-tab pattern - marko wanted
 * Events/Orders/Tickets/Sales to each get something like the Dashboard's own
 * Overview/Financials/Activity tabs, so finished items (a completed event, a
 * paid order, a sold ticket, a settled sale) move out of the way into a
 * second tab instead of sitting mixed in with everything still active.
 *
 * This generalizes Dashboard.tsx's own useDashboardTab (same load-on-mount +
 * persist-immediately-on-change shape, same "unrecognized saved value falls
 * back instead of crashing" safety) to any 2-key tab set, so each page gets
 * its own independent, remembered choice under its own settings key rather
 * than four copies of the same hook. Dashboard's own hook is untouched - it
 * already shipped and works, and it manages 3 tabs (not 2), so there was
 * nothing to gain by migrating it onto this shared version.
 */
export function useListTab<T extends string>(settingKey: string, keys: readonly T[]): [T, (tab: T) => void] {
  const [tab, setTabState] = useState<T>(keys[0]);

  useEffect(() => {
    let cancelled = false;
    api
      .getAppSetting(settingKey)
      .then((raw) => {
        if (cancelled || !raw || !(keys as readonly string[]).includes(raw)) return;
        setTabState(raw as T);
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [settingKey]);

  const setTab = useCallback(
    (next: T) => {
      setTabState(next);
      api.setAppSetting(settingKey, next).catch(() => {});
    },
    [settingKey],
  );

  return [tab, setTab];
}

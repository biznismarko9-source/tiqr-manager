import { useEffect, useState } from "react";
import { api } from "./api";

/**
 * The currency this app converts INTO, and the one it treats as "already
 * fine" - Settings -> Preferred currency.
 *
 * 2.50.0. marko: "do settings daj moznost preffered currency podla toho co
 * clovek chce, na vyber gbp, eur, usd, tiez ten convert bude podla toho co
 * mas zapnute".
 *
 * Cached at module level because it is read by half a dozen screens to label
 * a button and never changes except from one place. The cache is what keeps
 * every "Convert to X" label in the app saying the same X without each screen
 * making its own round trip.
 */

export const PREFERRED_CURRENCIES = ["EUR", "USD", "GBP"] as const;
export type PreferredCurrency = (typeof PREFERRED_CURRENCIES)[number];

let cached: string | null = null;
let inFlight: Promise<string> | null = null;
const listeners = new Set<(c: string) => void>();

async function load(): Promise<string> {
  if (cached) return cached;
  if (!inFlight) {
    inFlight = api
      .getPreferredCurrency()
      .then((c) => {
        cached = c;
        return c;
      })
      // EUR is the backend's own default, so falling back to it here can
      // never disagree with what a conversion would actually do.
      .catch(() => "EUR")
      .finally(() => {
        inFlight = null;
      });
  }
  return inFlight;
}

/** Writes it, then tells every mounted screen, so labels change immediately
 *  instead of after a reload. */
export async function savePreferredCurrency(currency: string): Promise<string> {
  const saved = await api.setPreferredCurrency(currency);
  cached = saved;
  listeners.forEach((fn) => fn(saved));
  return saved;
}

/** Starts at EUR - the backend default - so a label is never blank while the
 *  real value is on its way. */
export function usePreferredCurrency(): string {
  const [currency, setCurrency] = useState<string>(cached ?? "EUR");
  useEffect(() => {
    let alive = true;
    void load().then((c) => {
      if (alive) setCurrency(c);
    });
    listeners.add(setCurrency);
    return () => {
      alive = false;
      listeners.delete(setCurrency);
    };
  }, []);
  return currency;
}

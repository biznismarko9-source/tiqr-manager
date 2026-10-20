// 2.114.0 - the Promotions cleanup's words (components/PromoCleanup.tsx).
import type { PromoScan } from "./types";

/** What a scan cost, in words - at the cheapest model's price ($0.10 per
 *  million tokens in, $0.50 out). */
export function costText(s: PromoScan): string {
  if (s.calls === 0) return "no AI needed - all kept by TIQR's own rules";
  const dollars = (s.inputTokens * 0.1 + s.outputTokens * 0.5) / 1_000_000;
  const price = dollars < 0.01 ? "less than a cent" : `about $${dollars.toFixed(2)}`;
  return `${s.calls} request${s.calls === 1 ? "" : "s"}, ${(s.inputTokens + s.outputTokens).toLocaleString("en")} tokens - ${price}`;
}

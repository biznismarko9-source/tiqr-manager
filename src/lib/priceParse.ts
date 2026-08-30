// 2.0.82: "faster price entry" for Price Checker (see PriceChecker.tsx's
// SavePriceCheckModal) - marko copies a chunk of text straight off a
// marketplace's own listings page (StubHub/Vivid Seats/Ticombo) and pastes
// it into a textarea; this module pulls the individual ticket prices out of
// that raw text so the lowest/average/highest/count fields can be filled in
// automatically instead of retyped one at a time.
//
// Still 100% manual in the sense that matters: nothing in here ever visits
// a marketplace's website itself - marko opens the page and copies the text
// himself, exactly like before (see this project's own researched decision
// against any live API/scraping, documented in
// src-tauri/src/commands/price_checker.rs's module doc comment). This only
// saves him retyping numbers he's already looking at. Every field it fills
// stays fully editable in the modal, and if it can't find anything usable
// it says so instead of guessing.

/** Currency symbols/codes recognized in pasted text, most specific first.
 * Deliberately does NOT include a bare "kr" - that symbol is shared by
 * SEK/NOK/DKK with no reliable way to tell them apart from text alone, so
 * it's left for marko to pick from the existing currency dropdown rather
 * than risk a confidently-wrong guess. */
const CURRENCY_MARKERS: { pattern: RegExp; code: string }[] = [
  { pattern: /€|EUR\b/i, code: "EUR" },
  { pattern: /\$|USD\b/i, code: "USD" },
  { pattern: /£|GBP\b/i, code: "GBP" },
  { pattern: /CHF\b/i, code: "CHF" },
  { pattern: /Kč|CZK\b/i, code: "CZK" },
  { pattern: /zł|PLN\b/i, code: "PLN" },
  { pattern: /\bFt\b|HUF\b/i, code: "HUF" },
  { pattern: /SEK\b/i, code: "SEK" },
  { pattern: /NOK\b/i, code: "NOK" },
  { pattern: /DKK\b/i, code: "DKK" },
  { pattern: /lei\b|RON\b/i, code: "RON" },
  { pattern: /₺|TRY\b/i, code: "TRY" },
  { pattern: /лв|BGN\b/i, code: "BGN" },
];

/** Best-effort currency guess from pasted text. Returns null (leave
 * whatever the currency dropdown is already set to alone) unless exactly
 * ONE currency's marker shows up anywhere in the text - if it mentions more
 * than one (or none), guessing wrong is worse than not guessing at all. */
export function detectCurrencyFromText(text: string): string | null {
  const found = new Set<string>();
  for (const { pattern, code } of CURRENCY_MARKERS) {
    if (pattern.test(text)) found.add(code);
  }
  return found.size === 1 ? [...found][0] : null;
}

/** A plain 4-digit whole number that looks exactly like a calendar year -
 * event dates show up constantly in copied listing-page text ("Sat, Aug 30,
 * 2026"). A price with no decimals/grouping and no currency marker next to
 * it that also happens to equal a plausible year is far more likely to BE
 * that year than a real ticket price, so the no-currency-marker fallback
 * pass (see extractPricesFromText) excludes these. The currency-adjacent
 * pass never applies this filter - a year is never glued to a $ sign. */
function looksLikeYear(whole: number, hadDecimalOrGrouping: boolean): boolean {
  return !hadDecimalOrGrouping && whole >= 1900 && whole <= 2099;
}

/** Turns one matched numeric token ("1,234.56", "1.234,56", "85,50", "120",
 * ...) into a plain number, correctly regardless of whether the source uses
 * comma or period for the decimal point. Rule: look at the LAST separator
 * in the token. If exactly 1-2 digits follow it, that's the decimal point.
 * Otherwise (0, or 3+ digits - money never has 3 decimal places) every
 * separator in the token was thousands-grouping, so they're all stripped
 * and folded back into one whole number. This reads correctly without
 * needing to know the source locale up front. */
function normalizeAmountToken(raw: string): { value: number; hadDecimalOrGrouping: boolean } | null {
  const s = raw.replace(/\s/g, "");
  const lastSepIdx = Math.max(s.lastIndexOf("."), s.lastIndexOf(","));
  if (lastSepIdx === -1) {
    const whole = parseInt(s, 10);
    return Number.isFinite(whole) ? { value: whole, hadDecimalOrGrouping: false } : null;
  }
  const tail = s.slice(lastSepIdx + 1);
  const head = s.slice(0, lastSepIdx).replace(/[.,]/g, "");
  const isDecimal = tail.length >= 1 && tail.length <= 2 && /^\d+$/.test(tail);
  if (isDecimal) {
    const whole = parseInt(head || "0", 10);
    if (!Number.isFinite(whole)) return null;
    const frac = parseInt(tail.padEnd(2, "0").slice(0, 2), 10);
    return { value: whole + frac / 100, hadDecimalOrGrouping: true };
  }
  const wholeStr = (head + tail).replace(/\D/g, "");
  const whole = parseInt(wholeStr || "0", 10);
  return Number.isFinite(whole) ? { value: whole, hadDecimalOrGrouping: true } : null;
}

// Wide on purpose - just enough to exclude clearly-not-a-price numbers
// (phone numbers, years already handled separately, stray large IDs...).
// Not meant to validate real prices; marko does that himself by looking at
// the filled-in fields before saving.
const MIN_PLAUSIBLE_PRICE = 1;
const MAX_PLAUSIBLE_PRICE = 100000;

const AMOUNT_TOKEN = /\d[\d.,]*\d|\d/g;
const CURRENCY_SYMBOL_OR_CODE =
  "(?:[€$£₺]|\\b(?:USD|EUR|GBP|CHF|CZK|PLN|HUF|SEK|NOK|DKK|RON|TRY|BGN|Ft|lei)\\b|Kč|zł|лв)";
// The "number, then currency" half needs `(?!\d)` right after the currency
// marker - without it, "Section 200 $99" would let the "$" that actually
// belongs to "99" get pulled onto "200" instead (matching "200 $" as if
// "$" were a trailing symbol for 200, then losing "99" entirely). The
// lookahead means a currency marker only counts as a SUFFIX for the number
// before it when it isn't itself immediately followed by another digit -
// i.e. when it isn't actually a PREFIX for the next number.
const CONFIDENT_TOKEN = new RegExp(
  `${CURRENCY_SYMBOL_OR_CODE}\\s?(\\d[\\d.,]*\\d|\\d)|(\\d[\\d.,]*\\d|\\d)\\s?${CURRENCY_SYMBOL_OR_CODE}(?!\\d)`,
  "gi",
);

/** Pulls every plausible ticket price out of raw pasted text, plus a
 * best-effort currency guess (see detectCurrencyFromText).
 *
 * Two passes:
 * 1. "Confident" - numbers sitting directly next to a currency symbol/code
 *    (either side, e.g. "$120" or "120 Kč"). Used whenever the text has
 *    ANY of these, since that's the normal shape of copied price listings
 *    and correctly ignores unrelated numbers nearby (row/section numbers,
 *    ticket counts, dates) that never touch a currency marker.
 * 2. Fallback - every number-looking token in the text, only used when pass
 *    1 found nothing at all (e.g. a plain price column with no symbols).
 *    Excludes obvious calendar years here, since without a currency marker
 *    to anchor on that's the single most common false positive. */
export function extractPricesFromText(text: string): { prices: number[]; currency: string | null } {
  const currency = detectCurrencyFromText(text);

  const confident: number[] = [];
  for (const m of text.matchAll(CONFIDENT_TOKEN)) {
    const token = m[1] ?? m[2];
    const parsed = token ? normalizeAmountToken(token) : null;
    if (parsed && parsed.value >= MIN_PLAUSIBLE_PRICE && parsed.value <= MAX_PLAUSIBLE_PRICE) {
      confident.push(Math.round(parsed.value * 100) / 100);
    }
  }
  if (confident.length > 0) return { prices: confident, currency };

  const fallback: number[] = [];
  for (const m of text.matchAll(AMOUNT_TOKEN)) {
    const parsed = normalizeAmountToken(m[0]);
    if (!parsed) continue;
    if (looksLikeYear(parsed.value, parsed.hadDecimalOrGrouping)) continue;
    if (parsed.value >= MIN_PLAUSIBLE_PRICE && parsed.value <= MAX_PLAUSIBLE_PRICE) {
      fallback.push(Math.round(parsed.value * 100) / 100);
    }
  }
  return { prices: fallback, currency };
}

// 2.113.0: when a pull counts as finished - marko: "rozdelit dalej pulls na
// dokoncene a nedokoncene". Pulls.tsx splits both of its lists by these.

import { formatMoney } from "./format.ts";
import type { SoldTotal } from "./types";

/** A pull marko did for someone is finished once both of its checks are -
 *  the same two the Completed badge reads. */
export function isPullDone(p: { paid: boolean; transferDone: boolean }): boolean {
  return p.paid && p.transferDone;
}

/** A pull someone did for marko has no checks of its own; it is finished
 *  once its tickets became one of marko's orders, or once the event is over. */
export function isReceivedPullDone(p: { orderId: number | null; eventDate: string | null }, today = new Date()): boolean {
  if (p.orderId !== null) return true;
  if (!p.eventDate) return false;
  const iso = `${today.getFullYear()}-${String(today.getMonth() + 1).padStart(2, "0")}-${String(today.getDate()).padStart(2, "0")}`;
  return p.eventDate.slice(0, 10) < iso;
}

/** What Pulls (Received) shows in its "Sold for" column. */
export interface SoldCell {
  /** One amount per currency, biggest first - e.g. ["€450.00"], or
   *  ["£360.00", "€120.00"] for sales in two currencies (never added
   *  together); empty while nothing is sold. */
  amounts: string[];
  /** "2/4" while only part of the pulled tickets is sold, else null. */
  part: string | null;
  /** How many tickets the amounts are for, or why there are none. */
  title: string;
}

/** 2.114.1, marko: "pri tych co som predal inym tak aka ciastka" - what a
 *  received pull's tickets were sold for, from its linked order's sales. */
export function soldCell(p: { orderId: number | null; quantity: number; sold: SoldTotal[]; soldQuantity: number }): SoldCell {
  if (p.orderId === null) return { amounts: [], part: null, title: "Not linked to an order" };
  if (p.sold.length === 0) return { amounts: [], part: null, title: "Nothing sold yet" };
  const amounts = p.sold.map((s) => formatMoney(s.cents, s.currency));
  const tickets = (n: number) => `${n} ticket${n === 1 ? "" : "s"}`;
  if (p.soldQuantity < p.quantity) {
    return { amounts, part: `${p.soldQuantity}/${p.quantity}`, title: `${p.soldQuantity} of ${tickets(p.quantity)} sold` };
  }
  return { amounts, part: null, title: `${tickets(p.soldQuantity)} sold` };
}

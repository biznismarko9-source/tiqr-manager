// 2.113.0: when a pull counts as finished - marko: "rozdelit dalej pulls na
// dokoncene a nedokoncene". Pulls.tsx splits both of its lists by these.

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

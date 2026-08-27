/** 2.0.66: shared "is this fully wrapped up" indicator - Orders, Sales, and
 * Pulls (Given) each derive their own set of checks (see each page's own
 * `completionChecks`-style helper) and pass them here to get a consistent
 * Badge tone/label, plus a hover title spelling out every check's own state.
 * Mirrors the existing inventoryStatus() convention (Tickets.tsx) - "derive
 * one small status value from several fields, render with <Badge>" - but
 * shared across pages instead of living on one page, since this same
 * "Completed" concept now applies everywhere (see
 * REDESIGN-2.0.66-REPORT.md).
 *
 * Deliberately independent from any page's own pre-existing Active/Paid
 * (Orders) or Pending/Completed (Sales) tabs - those keep their existing,
 * ticket-count/payment-only meaning unchanged (2.0.60/2.0.59 respectively);
 * this is a new, separate, stricter concept that layers on top rather than
 * replacing them. */
export interface CompletionCheck {
  /** Short label naming the thing that's done/pending, phrased so
   * "Not <label, lowercased>" reads naturally - e.g. "Sold" -> "Not sold",
   * "Delivered" -> "Not delivered", "Transferred" -> "Not transferred". */
  label: string;
  done: boolean;
}

export interface CompletionResult {
  /** Feed straight into <Badge tone={...}>. */
  tone: "completed" | "pending";
  /** Short, always-visible label. Names the one missing check directly when
   * exactly one is missing (so "which one is wrong" is answered without
   * needing the hover), otherwise a count. */
  label: string;
  /** Full breakdown of every check's own state (e.g. "Sold: done ·
   * Delivered: pending · Paid: done"), meant for the Badge's `title` prop -
   * a native hover tooltip to "check exactly" what's outstanding. */
  title: string;
}

export function completionStatus(checks: CompletionCheck[]): CompletionResult {
  const title = checks.map((c) => `${c.label}: ${c.done ? "done" : "pending"}`).join(" · ");
  const missing = checks.filter((c) => !c.done);
  if (missing.length === 0) {
    return { tone: "completed", label: "Completed", title };
  }
  const label = missing.length === 1 ? `Not ${missing[0].label.toLowerCase()}` : `${missing.length} pending`;
  return { tone: "pending", label, title };
}

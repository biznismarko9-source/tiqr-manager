// 2.117.0: Web access's answers, as the window's backend would give them
// (web/core.ts).
export const NOT_SIGNED_IN = "Sign in to use TIQR.";
export const UNREACHABLE = "TIQR on the server cannot be reached. Check the connection, and that TIQR is running there.";

/** What went wrong, as the window's backend would say it. */
export async function failure(res: Response): Promise<unknown> {
  try {
    const body = (await res.json()) as { error?: unknown };
    if (body && "error" in body) return body.error;
  } catch {
    // Not TIQR's answer - the tunnel's own page when TIQR is not running.
  }
  if ([502, 503, 504, 521, 522, 523, 530].includes(res.status)) return UNREACHABLE;
  return `TIQR on the server answered ${res.status}.`;
}

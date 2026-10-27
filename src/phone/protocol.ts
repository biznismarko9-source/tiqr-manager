// 2.118.0: what the page and TIQR's worker say to each other (worker.ts,
// backend.ts). The worker holds TIQR itself - its Rust, built for the browser,
// and the database in this site's storage on the phone.

/** Page to worker: run a command. */
export type Call = { id: number; cmd: string; args: Record<string, unknown> };

/** Worker to page. */
export type Message =
  /** TIQR is open and answers commands. */
  | { kind: "ready" }
  /** TIQR could not open (the browser keeps no data for this site, ...). */
  | { kind: "failed"; message: string }
  /** A command's answer - a value, or the bytes of a file. */
  | { kind: "answer"; id: number; value: unknown }
  /** A command's error, as the computer's TIQR words it. */
  | { kind: "error"; id: number; error: unknown }
  /** An event a command sent (a mail run finished, a change arrived, ...). */
  | { kind: "event"; event: string; payload: unknown }
  /** TIQR stopped where it did not plan to; only a reload brings it back. */
  | { kind: "stopped"; detail: string };

/** What the page shows when TIQR on the phone stopped. */
export const STOPPED = "TIQR stopped on this phone. Reload the page to go on - what was saved is kept.";

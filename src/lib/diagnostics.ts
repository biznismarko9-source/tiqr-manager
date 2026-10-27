// 2.116.0 - one place where failures go.
//
// Before this, a failure that a screen chose not to show (a background
// refresh, a picture that did not load) went to `.catch(() => {})` and was
// gone: nothing on screen, nothing in a log, nothing to look at afterwards.
// Now it is reported here - with when, where, how bad, and a short id the
// screen can show - and kept in a small ring that Settings -> Support can
// copy as a diagnostic report. Nothing is ever kept in a form that could
// leak: every message goes through `redact` first (e-mail addresses,
// tokens, keys, passwords, codes, long secrets and pictures are replaced),
// so a copied report is safe to send.
//
// Kept free of React and of the Tauri API, so it can be tested on its own
// (diagnostics.test.ts) and used from anywhere, including before the app
// has rendered.

export type Severity = "info" | "warning" | "error" | "fatal";

export interface DiagnosticEntry {
  /** Short, shown to a person: "E-7K2Q". The same id is in the report. */
  id: string;
  at: string;
  severity: Severity;
  /** What was being done: "sales list", "page Events", "unhandled". */
  where: string;
  message: string;
  /** The same failure again, folded into one entry instead of a flood. */
  count: number;
}

const MAX_ENTRIES = 200;
const MAX_MESSAGE = 500;
const entries: DiagnosticEntry[] = [];
const listeners = new Set<(e: DiagnosticEntry) => void>();

/** Replaces whatever could identify a person or open an account. Applied to
 *  every message before it is kept, shown or sent anywhere. */
export function redact(text: string): string {
  return (
    text
      // A picture carried inline is never worth keeping, and can be huge.
      .replace(/data:[a-z]+\/[a-z0-9.+-]+;base64,[A-Za-z0-9+/=]+/gi, "data:[picture]")
      // JSON Web Tokens (Firebase, Google).
      .replace(/\beyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\b/g, "[token]")
      .replace(/\b(bearer|basic)\s+[A-Za-z0-9._~+/=-]{8,}/gi, "$1 [token]")
      // name=value pairs that are secrets, in addresses, JSON and prose.
      .replace(
        /\b(access_token|refresh_token|id_token|client_secret|api[_-]?key|apikey|key|token|password|passwd|pwd|secret|code|sig|signature|auth|authorization|cookie|session)(["']?\s*[:=]\s*["']?)([^&\s"',;}]+)/gi,
        "$1$2[redacted]",
      )
      .replace(/[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}/g, "[email]")
      // Anthropic and other prefixed keys, and any long unbroken secret.
      .replace(/\bsk-[A-Za-z0-9_-]{10,}/g, "[key]")
      .replace(/\b[A-Za-z0-9_-]{32,}\b/g, "[long-id]")
      .slice(0, MAX_MESSAGE)
  );
}

function messageOf(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message || e.name;
  if (e && typeof e === "object" && "message" in e && typeof (e as { message: unknown }).message === "string") {
    return (e as { message: string }).message;
  }
  try {
    return JSON.stringify(e) ?? String(e);
  } catch {
    return String(e);
  }
}

const ID_CHARS = "23456789ABCDEFGHJKLMNPQRSTUVWXYZ";

function newId(): string {
  let out = "E-";
  for (let i = 0; i < 4; i++) out += ID_CHARS[Math.floor(Math.random() * ID_CHARS.length)];
  return out;
}

/**
 * Records a failure and returns its entry (whose `id` a screen may show).
 * The same message from the same place within a minute is counted on the
 * entry already there, so a loop that fails every second is one line.
 */
export function reportError(
  where: string,
  e: unknown,
  opts: { severity?: Severity; now?: Date } = {},
): DiagnosticEntry {
  const severity = opts.severity ?? "error";
  const now = opts.now ?? new Date();
  const message = redact(messageOf(e));
  const last = entries[entries.length - 1];
  if (
    last &&
    last.where === where &&
    last.message === message &&
    last.severity === severity &&
    now.getTime() - new Date(last.at).getTime() < 60_000
  ) {
    last.count += 1;
    return last;
  }
  const entry: DiagnosticEntry = { id: newId(), at: now.toISOString(), severity, where: redact(where), message, count: 1 };
  entries.push(entry);
  if (entries.length > MAX_ENTRIES) entries.splice(0, entries.length - MAX_ENTRIES);
  for (const l of [...listeners]) {
    try {
      l(entry);
    } catch {
      // A listener that throws must not stop the report itself.
    }
  }
  return entry;
}

/** For a failure no screen needs to show: kept in the log, never shown,
 *  never lost - what `.catch(() => {})` used to throw away. */
export function quietly(where: string): (e: unknown) => void {
  return (e) => {
    reportError(where, e, { severity: "warning" });
  };
}

/** Everything kept, oldest first. */
export function diagnostics(): DiagnosticEntry[] {
  return entries.map((e) => ({ ...e }));
}

export function onDiagnostic(listener: (e: DiagnosticEntry) => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** The kept entries as plain text, one per line - already redacted. */
export function diagnosticsText(list: DiagnosticEntry[] = entries): string {
  return list
    .map((e) => `${e.at} ${e.severity.toUpperCase()} [${e.id}] ${e.where}: ${e.message}${e.count > 1 ? ` (x${e.count})` : ""}`)
    .join("\n");
}

/** Test seam: forget everything. */
export function clearDiagnostics() {
  entries.length = 0;
}

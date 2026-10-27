// 2.117.0: `listen` for the web build. The events TIQR's window gets on the
// server (a mail run finished, ...) come down one stream per browser tab
// (src-tauri/src/web_access.rs, /api/events), opened while anything listens
// and opened again when it drops - the server ends it when the token runs
// out, and the next one carries a fresh token.
import { parseFrame } from "./frames";
import { idToken } from "./session";

export type UnlistenFn = () => void;
type Handler = (event: { event: string; id: number; payload: unknown }) => void;

const handlers = new Map<string, Set<Handler>>();
let running = false;
let abort: AbortController | null = null;
let nextId = 1;

export async function listen<T>(
  event: string,
  handler: (event: { event: string; id: number; payload: T }) => void,
): Promise<UnlistenFn> {
  const set = handlers.get(event) ?? new Set<Handler>();
  set.add(handler as Handler);
  handlers.set(event, set);
  if (!running) {
    running = true;
    void stream();
  }
  return () => {
    set.delete(handler as Handler);
    if (![...handlers.values()].some((s) => s.size > 0)) {
      running = false;
      abort?.abort();
    }
  };
}

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

async function stream() {
  let delay = 1_000;
  while (running) {
    try {
      const token = await idToken();
      if (token) {
        abort = new AbortController();
        const res = await fetch("/api/events", { headers: { authorization: `Bearer ${token}` }, signal: abort.signal });
        if (res.ok && res.body) {
          delay = 1_000;
          await read(res.body);
        }
      }
    } catch {
      // Dropped or refused: tried again below.
    }
    if (running) {
      await sleep(delay);
      delay = Math.min(delay * 2, 30_000);
    }
  }
}

async function read(body: ReadableStream<Uint8Array>) {
  const reader = body.getReader();
  const decoder = new TextDecoder();
  let buffer = "";
  for (;;) {
    const { value, done } = await reader.read();
    if (done) return;
    buffer += decoder.decode(value, { stream: true }).replace(/\r/g, "");
    let end = buffer.indexOf("\n\n");
    while (end >= 0) {
      deliver(buffer.slice(0, end));
      buffer = buffer.slice(end + 2);
      end = buffer.indexOf("\n\n");
    }
  }
}

function deliver(frame: string) {
  const parsed = parseFrame(frame);
  if (!parsed) return;
  const set = handlers.get(parsed.event);
  if (!set?.size) return;
  let payload: unknown;
  try {
    payload = JSON.parse(parsed.data);
  } catch {
    payload = parsed.data;
  }
  for (const handler of [...set]) handler({ event: parsed.event, id: nextId++, payload });
}

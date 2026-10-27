// 2.118.0: the page's side of TIQR on a phone - one worker for the whole page
// (worker.ts), the commands sent to it and the events it sends back.
import type { Call, Message } from "./protocol";
import { STOPPED } from "./protocol";

type Waiting = { resolve: (value: unknown) => void; reject: (error: unknown) => void };
type Listener = (event: string, payload: unknown) => void;

let worker: Worker | null = null;
let nextId = 1;
const waiting = new Map<number, Waiting>();
const listeners = new Set<Listener>();
/** Why TIQR is not answering, once it cannot: it never opened, or it stopped. */
let broken: string | null = null;

function failAll(reason: string) {
  broken = reason;
  for (const w of waiting.values()) w.reject(reason);
  waiting.clear();
  for (const l of listeners) l("tiqr-phone-broken", reason);
}

function open(): Worker {
  if (worker) return worker;
  worker = new Worker(new URL("./worker.ts", import.meta.url), { type: "module" });
  worker.onmessage = (e: MessageEvent<Message>) => {
    const m = e.data;
    switch (m.kind) {
      case "answer": {
        const w = waiting.get(m.id);
        waiting.delete(m.id);
        w?.resolve(m.value);
        break;
      }
      case "error": {
        const w = waiting.get(m.id);
        waiting.delete(m.id);
        w?.reject(m.error);
        break;
      }
      case "event":
        for (const l of listeners) l(m.event, m.payload);
        break;
      case "failed":
        failAll(`TIQR could not open on this phone: ${m.message}`);
        break;
      case "stopped":
        console.error("TIQR stopped on this phone", m.detail);
        failAll(STOPPED);
        break;
      case "ready":
        break;
    }
  };
  worker.onerror = (e) => {
    console.error("TIQR's worker failed", e.message);
    failAll(STOPPED);
  };
  return worker;
}

/** Runs a command in TIQR on this phone, as `invoke` does on a computer. */
export function call<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  if (broken) return Promise.reject(broken);
  const w = open();
  const id = nextId++;
  return new Promise<T>((resolve, reject) => {
    waiting.set(id, { resolve: resolve as (v: unknown) => void, reject });
    const message: Call = { id, cmd, args };
    w.postMessage(message);
  });
}

/** Hears every event TIQR on this phone sends. */
export function onEvent(listener: Listener): () => void {
  open();
  listeners.add(listener);
  return () => listeners.delete(listener);
}

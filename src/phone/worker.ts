// 2.118.0: TIQR on a phone, in a worker of the page. The Rust the computer's
// app runs is built for the browser (src-tauri/src/phone.rs, `npm run
// build:phone`); this loads it, opens the database in this site's storage on
// the phone, and runs each command the page sends - one after another, as the
// screens wait on them. A worker and not the page itself: the browser's
// storage for a database works only here, and a long command never freezes
// the screen.
/// <reference lib="webworker" />
import init, { invoke, start } from "./pkg/tiqr_manager_lib.js";
import wasmUrl from "./pkg/tiqr_manager_lib_bg.wasm?url";
import type { Call, Message } from "./protocol";

declare const self: DedicatedWorkerGlobalScope;

const send = (message: Message, transfer: Transferable[] = []) => self.postMessage(message, transfer);

const parse = (text: unknown): unknown => {
  if (typeof text !== "string") return text;
  try {
    return JSON.parse(text);
  } catch {
    return text;
  }
};

let stopped = false;

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

/** What the page says when another tab has TIQR open. */
const OPEN_ELSEWHERE = "TIQR is open in another tab or window on this phone. Close it there, then reload this page.";

/** The database's storage opens in one place at a time, so TIQR holds this
 *  for as long as this worker lives. After a reload the page that went away
 *  may still hold it for a moment - it is waited for; another open tab
 *  holds it for good, and is named. */
function holdTheDatabase(): Promise<boolean> {
  return new Promise((resolve) => {
    navigator.locks
      .request("tiqr-database", { signal: AbortSignal.timeout(8_000) }, () => {
        resolve(true);
        return new Promise<void>(() => undefined);
      })
      .catch(() => resolve(false));
  });
}

/** Whether `start` failed only because the storage was still held - by the
 *  page a reload just closed, which lets go a moment later. */
const stillHeld = (e: unknown) => /NoModificationAllowedError|Access Handles cannot be created/i.test(String(e));

const emit = (event: string, payload: string) => {
  if (event === "tiqr-phone-stopped") {
    stopped = true;
    send({ kind: "stopped", detail: payload });
    return;
  }
  send({ kind: "event", event, payload: parse(payload) });
};

/** Headers a browser sets by itself, and refuses (noisily) to be given. */
const BROWSERS_OWN = new Set(["host", "user-agent", "content-length", "connection", "accept-encoding", "cookie", "origin", "referer"]);

/** A request to the internet from TIQR's Rust (exchange rates, ntfy), the
 *  way the computer's app makes it: the answer comes before this returns. A
 *  worker may wait like that; the screen does not wait with it. A service
 *  that does not let a web page call it (CORS) fails here as "could not
 *  connect". Redirects are always followed - TIQR refuses, on the phone,
 *  the one kind of request that must not follow them (phone.rs). */
function http(method: string, url: string, headers: string, body: Uint8Array | undefined, timeoutMs: number) {
  const xhr = new XMLHttpRequest();
  xhr.open(method, url, false);
  xhr.responseType = "arraybuffer";
  if (timeoutMs > 0) xhr.timeout = timeoutMs;
  for (const [name, value] of JSON.parse(headers) as [string, string][]) {
    if (!BROWSERS_OWN.has(name.toLowerCase())) xhr.setRequestHeader(name, value);
  }
  // Anthropic answers a web page only when it says it knows it is one: the
  // key is this person's own, typed into this phone.
  if (new URL(url).host === "api.anthropic.com") xhr.setRequestHeader("anthropic-dangerous-direct-browser-access", "true");
  try {
    xhr.send(body ? new Uint8Array(body) : null);
  } catch (e) {
    throw e instanceof DOMException && e.name === "TimeoutError" ? "operation timed out" : "could not connect";
  }
  if (xhr.status === 0) throw "could not connect";
  return {
    status: xhr.status,
    headers: xhr.getAllResponseHeaders(),
    body: new Uint8Array((xhr.response as ArrayBuffer | null) ?? new ArrayBuffer(0)),
    url: xhr.responseURL || url,
  };
}

/** TIQR opens once; commands that come before wait for it. */
const opened: Promise<boolean> = (async () => {
  try {
    const t0 = performance.now();
    if (!(await holdTheDatabase())) {
      send({ kind: "failed", message: OPEN_ELSEWHERE });
      return false;
    }
    const t1 = performance.now();
    await init({ module_or_path: wasmUrl });
    const t2 = performance.now();
    for (let attempt = 1; ; attempt++) {
      try {
        await start(emit, http);
        break;
      } catch (e) {
        if (attempt >= 20 || !stillHeld(e)) throw e;
        await sleep(250);
      }
    }
    const t3 = performance.now();
    send({ kind: "ready", took: { wait: Math.round(t1 - t0), load: Math.round(t2 - t1), open: Math.round(t3 - t2) } });
    return true;
  } catch (e) {
    send({ kind: "failed", message: stillHeld(e) ? OPEN_ELSEWHERE : e instanceof Error ? e.message : String(e) });
    return false;
  }
})();

/** One at a time, in the order they came - the database has one connection. */
let queue: Promise<void> = Promise.resolve();

self.onmessage = (e: MessageEvent<Call>) => {
  const { id, cmd, args } = e.data;
  queue = queue.then(async () => {
    if (!(await opened) || stopped) {
      send({ kind: "error", id, error: "TIQR is not open on this phone. Reload the page." });
      return;
    }
    try {
      const value: unknown = await invoke(cmd, JSON.stringify(args ?? {}));
      if (value instanceof Uint8Array) {
        const bytes = value.slice().buffer;
        send({ kind: "answer", id, value: bytes }, [bytes]);
      } else {
        send({ kind: "answer", id, value: parse(value) });
      }
    } catch (error) {
      // A wasm trap (a stop the code did not plan for) arrives here too,
      // after the panic hook has told the page.
      send({ kind: "error", id, error: stopped ? "TIQR stopped on this phone. Reload the page." : parse(error) });
    }
  });
};

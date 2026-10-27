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

/** TIQR opens once; commands that come before wait for it. */
const opened: Promise<boolean> = (async () => {
  try {
    await init({ module_or_path: wasmUrl });
    await start((event: string, payload: string) => {
      if (event === "tiqr-phone-stopped") {
        stopped = true;
        send({ kind: "stopped", detail: payload });
        return;
      }
      send({ kind: "event", event, payload: parse(payload) });
    });
    send({ kind: "ready" });
    return true;
  } catch (e) {
    send({ kind: "failed", message: e instanceof Error ? e.message : String(e) });
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

// 2.116.0 - the screen's failures, into the app's log.
//
// lib/diagnostics.ts keeps what went wrong on the screen; this sends each new
// entry to the backend's log (src-tauri/src/diag.rs) so it is still there
// after the window closes, and catches what nothing else catches: an error
// thrown outside React, and a promise that failed with nobody listening.
import { invoke } from "@tauri-apps/api/core";
import { onDiagnostic, reportError, type DiagnosticEntry } from "./diagnostics";

let installed = false;

function forward(entry: DiagnosticEntry) {
  if (entry.severity === "info") return;
  invoke("report_client_error", { report: entry }).catch((e) => {
    // The log itself cannot be reached: the console is all that is left.
    console.warn("TIQR: a problem could not be written to the log", e);
  });
}

export function installDiagnostics() {
  if (installed) return;
  installed = true;
  onDiagnostic(forward);
  window.addEventListener("error", (e) => {
    reportError("unhandled error", e.error ?? e.message, { severity: "error" });
  });
  window.addEventListener("unhandledrejection", (e) => {
    reportError("unhandled promise", e.reason, { severity: "error" });
  });
}

// 2.116.0 - a screen that fails, fails alone.
//
// Before this, an error while drawing any screen took the whole window with
// it: React unmounts everything on an uncaught render error, so the app went
// blank - sidebar, unsaved form and all - and nothing said why. Now each
// screen is drawn inside one of these (Layout.tsx), and the app as a whole
// inside another (App.tsx): the failure is reported (lib/diagnostics.ts,
// with an id that is also in the log), the rest of the app stays, and the
// screen offers to try again. Nothing is hidden - the message says what
// happened and where to find the details.
import { Component, type ErrorInfo, type ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { reportError, type DiagnosticEntry } from "../lib/diagnostics";
import { copyText } from "../lib/clipboard";

type Props = {
  /** What is drawn inside - "page /events", "app" - for the report. */
  area: string;
  /** Changing it (another page) starts the boundary over. */
  resetKey?: string;
  /** The whole app: no router or providers to lean on any more. */
  root?: boolean;
  children: ReactNode;
};

type State = { failed: boolean; entry: DiagnosticEntry | null; copied: "" | "yes" | "no" };

/** The first few component names of the stack - where, not the whole tree. */
function where(stack: string | null | undefined): string {
  if (!stack) return "";
  const names = stack
    .split("\n")
    .map((l) => l.trim().replace(/^at\s+/, "").split(" ")[0])
    .filter(Boolean)
    .slice(0, 4);
  return names.length ? ` (in ${names.join(" < ")})` : "";
}

export class ErrorBoundary extends Component<Props, State> {
  state: State = { failed: false, entry: null, copied: "" };

  static getDerivedStateFromError(): Partial<State> {
    return { failed: true };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    const entry = reportError(`${this.props.area}`, `${error?.message || String(error)}${where(info.componentStack)}`, {
      severity: "fatal",
    });
    this.setState({ entry });
  }

  componentDidUpdate(prev: Props) {
    if (prev.resetKey !== this.props.resetKey && this.state.failed) {
      this.setState({ failed: false, entry: null, copied: "" });
    }
  }

  private retry = () => this.setState({ failed: false, entry: null, copied: "" });

  private copy = async () => {
    let text: string;
    try {
      text = await invoke<string>("export_diagnostics");
    } catch (e) {
      reportError("diagnostic report", e);
      text = `TIQR problem ${this.state.entry?.id ?? ""}: ${this.state.entry?.message ?? ""}`;
    }
    const ok = await copyText(text);
    this.setState({ copied: ok ? "yes" : "no" });
  };

  render() {
    if (!this.state.failed) return this.props.children;
    const id = this.state.entry?.id;
    const root = this.props.root;
    return (
      <div className={root ? "flex min-h-screen items-center justify-center bg-[var(--app-bg)] p-6" : "py-10"}>
        <div role="alert" className="mx-auto max-w-lg rounded-xl border border-slate-200 bg-surface p-6 text-center shadow-card dark:border-white/10">
          <h2 className="text-base font-semibold text-slate-900 dark:text-slate-100">
            {root ? "TIQR ran into a problem" : "This screen ran into a problem"}
          </h2>
          <p className="mt-2 text-sm text-slate-600 dark:text-slate-300">
            What was already saved is safe - only this view stopped. The problem was written down
            {id ? (
              <>
                {" "}as <span className="font-mono font-semibold">{id}</span>
              </>
            ) : null}
            , and the diagnostic report has the details.
          </p>
          {this.state.entry?.message && (
            <p className="mt-2 break-words text-xs text-slate-500 dark:text-slate-400">{this.state.entry.message}</p>
          )}
          <div className="mt-5 flex flex-wrap justify-center gap-2">
            {root ? (
              <button
                type="button"
                className="rounded-lg bg-brand-600 px-3.5 py-2 text-sm font-medium text-white hover:bg-brand-700"
                onClick={() => window.location.reload()}
              >
                Reload TIQR
              </button>
            ) : (
              <>
                <button
                  type="button"
                  className="rounded-lg bg-brand-600 px-3.5 py-2 text-sm font-medium text-white hover:bg-brand-700"
                  onClick={this.retry}
                >
                  Try again
                </button>
                <button
                  type="button"
                  className="rounded-lg border border-slate-200 px-3.5 py-2 text-sm font-medium text-slate-700 hover:bg-slate-50 dark:border-white/10 dark:text-slate-200 dark:hover:bg-white/5"
                  onClick={() => {
                    window.location.hash = "#/";
                  }}
                >
                  Go to the Dashboard
                </button>
              </>
            )}
            <button
              type="button"
              className="rounded-lg border border-slate-200 px-3.5 py-2 text-sm font-medium text-slate-700 hover:bg-slate-50 dark:border-white/10 dark:text-slate-200 dark:hover:bg-white/5"
              onClick={() => void this.copy()}
            >
              {this.state.copied === "yes" ? "Copied" : this.state.copied === "no" ? "Could not copy" : "Copy diagnostic report"}
            </button>
          </div>
        </div>
      </div>
    );
  }
}

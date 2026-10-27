// 2.118.0: `invoke` for the phone build. The command and its arguments go to
// TIQR in this page's worker (backend.ts) - the same Rust the computer's app
// runs - and come back the way the computer's app answers: the result, a
// file's bytes, or the command's own error.
import { call } from "./backend";

export function invoke<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  return call<T>(cmd, args);
}

// 2.117.0: `invoke` for the web build. The command and arguments the window
// hands its own backend go to TIQR on the server instead, with the signed-in
// account's token (src-tauri/src/web_access.rs). It answers the way the
// window's backend does: the result, a file's bytes, or the command's own
// error - so every screen handles both builds the same way.
import { failure, NOT_SIGNED_IN, UNREACHABLE } from "./answers";
import { idToken } from "./session";

export async function invoke<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  let token = await idToken();
  if (!token) throw NOT_SIGNED_IN;
  let res = await send(cmd, args, token);
  if (res.status === 401) {
    // A token that ran out a moment ago: once more with a fresh one.
    token = await idToken(true);
    if (!token) throw NOT_SIGNED_IN;
    res = await send(cmd, args, token);
  }
  if (res.ok) {
    if ((res.headers.get("content-type") ?? "").startsWith("application/octet-stream")) {
      return (await res.arrayBuffer()) as T;
    }
    return (await res.json()) as T;
  }
  throw await failure(res);
}

async function send(cmd: string, args: Record<string, unknown>, token: string): Promise<Response> {
  try {
    return await fetch("/api/invoke", {
      method: "POST",
      headers: { "content-type": "application/json", authorization: `Bearer ${token}` },
      body: JSON.stringify({ cmd, args }),
    });
  } catch {
    throw UNREACHABLE;
  }
}

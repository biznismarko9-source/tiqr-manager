// 2.117.0: the signed-in account's Firebase ID token - what Web access checks
// on every request (src-tauri/src/web_access.rs). Firebase keeps it fresh;
// `force` asks for a new one when the server says the old one ran out.
import { auth } from "../lib/firebase";

export async function idToken(force = false): Promise<string | null> {
  const user = auth.currentUser;
  if (!user) return null;
  return user.getIdToken(force);
}

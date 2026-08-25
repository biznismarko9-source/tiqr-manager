// 2.0.45: turns a raw Firebase Auth error into a plain-language message -
// shared by Welcome.tsx (login/register) and Settings.tsx's Account section
// (name save), the only two places that call into lib/auth.tsx's Firebase-
// backed functions. Firebase's own error objects carry a stable `.code`
// string (e.g. "auth/wrong-password") - matched on that, never on `.message`
// (which is meant for developers, not end users, and isn't guaranteed
// wording-stable across SDK versions).
const MESSAGES: Record<string, string> = {
  "auth/invalid-email": "That doesn't look like a valid email address.",
  "auth/user-disabled": "This account has been disabled.",
  "auth/user-not-found": "No account found with that email.",
  "auth/wrong-password": "Wrong password.",
  // Newer Firebase SDK versions return this single generic code instead of
  // user-not-found/wrong-password specifically, on purpose - Google's own
  // guidance is this is a deliberate security change (doesn't reveal
  // whether the email exists), not a bug to work around.
  "auth/invalid-credential": "Wrong email or password.",
  "auth/email-already-in-use": "An account with that email already exists - try logging in instead.",
  "auth/weak-password": "Password must be at least 6 characters.",
  "auth/too-many-requests": "Too many attempts - wait a bit and try again.",
  "auth/network-request-failed": "Couldn't reach the server - check your internet connection.",
  // Means the Email/Password (or Google) provider hasn't been turned on yet
  // in the Firebase console's Authentication -> Sign-in method tab.
  "auth/operation-not-allowed": "This sign-in method isn't turned on yet in Firebase - check Authentication -> Sign-in method in the Firebase console.",
};

export function firebaseAuthErrorMessage(err: unknown): string {
  const code = (err as { code?: string } | null)?.code;
  if (code && MESSAGES[code]) return MESSAGES[code];
  return "Something went wrong. Try again.";
}

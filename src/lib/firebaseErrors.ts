// 2.0.45: turns a raw Firebase Auth error into a plain-language message -
// shared by Welcome.tsx (login/register, and since 2.0.46 the
// signInWithCredential step of "Continue with Google") and Settings.tsx's
// Account section (name save). Firebase's own error objects carry a stable
// `.code` string (e.g. "auth/wrong-password") - matched on that, never on
// `.message` (which is meant for developers, not end users, and isn't
// guaranteed wording-stable across SDK versions). Deliberately does NOT
// handle errors from this app's own Rust/Tauri commands (e.g.
// api.startFirebaseGoogleSignIn's "Google sign-in was cancelled.") - those
// already carry a human-readable message of their own (see `errMsg` in
// lib/api.ts) and have no `.code` to match here; Welcome.tsx checks for a
// `.code` first and only calls this function when one is present.
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
  // 2.0.46: the "Continue with Google" analogue of email-already-in-use -
  // hit whenever the Google account's email already has a password-based
  // account (a likely real case: someone registers with email+password
  // first, then later tries Google sign-in using an account with that same
  // email address).
  "auth/account-exists-with-different-credential":
    "An account with that email already exists using a different sign-in method - try logging in with your password instead.",
  "auth/weak-password": "Password must be at least 6 characters.",
  "auth/too-many-requests": "Too many attempts - wait a bit and try again.",
  "auth/network-request-failed": "Couldn't reach the server - check your internet connection.",
  // Means the Email/Password (or Google) provider hasn't been turned on yet
  // in the Firebase console's Authentication -> Sign-in method tab.
  "auth/operation-not-allowed": "This sign-in method isn't turned on yet in Firebase - check Authentication -> Sign-in method in the Firebase console.",
  // 2.5.2: ResetPassword.tsx's verifyPasswordResetCode/confirmPasswordReset -
  // a reset link Firebase already used once, or that was ever a stale/wrong
  // oobCode (never valid to begin with, e.g. a manually mangled URL).
  "auth/invalid-action-code": "This reset link has already been used or isn't valid. Request a new one.",
  // 2.5.2: same two call sites - Firebase reset links expire after 1 hour.
  "auth/expired-action-code": "This reset link has expired. Request a new one.",
};

export function firebaseAuthErrorMessage(err: unknown): string {
  const code = (err as { code?: string } | null)?.code;
  if (code && MESSAGES[code]) return MESSAGES[code];
  return "Something went wrong. Try again.";
}

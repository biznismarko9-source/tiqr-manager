import { initializeApp } from "firebase/app";
import { getAuth } from "firebase/auth";
import { getFirestore } from "firebase/firestore";

// 2.0.45: marko's own real Firebase project ("tiqr-manager-b890a"), pasted
// from Firebase Console -> Project settings -> Your apps -> Web app.
//
// This config (including apiKey) is NOT a secret and is meant to ship
// inside client code - confirmed against Firebase's own docs
// (firebase.google.com/docs/projects/api-keys): "API keys for Firebase
// services are not used to control access to backend resources - that can
// only be done with Firebase Security Rules and Firebase App Check. API
// keys only identify your Firebase project/app to Firebase's services."
// This is a DIFFERENT situation from the Google Sheets service-account key
// elsewhere in this app (google_sheets.rs / build.rs), which genuinely IS
// secret and stays out of source, injected only at build time - do not
// apply that same secrecy habit here, it doesn't apply and was verified,
// not assumed (see PROTECTED-AREAS-NOTES.md's 2.0.45 section for the
// research trail, same discipline as 2.0.42's Sheets-locale research).
const firebaseConfig = {
  apiKey: "AIzaSyBZZKkmIW4EOVYMp6pSpmVFbEY7a98d5OA",
  authDomain: "tiqr-manager-b890a.firebaseapp.com",
  projectId: "tiqr-manager-b890a",
  storageBucket: "tiqr-manager-b890a.firebasestorage.app",
  messagingSenderId: "251875496438",
  appId: "1:251875496438:web:d1a2ad7112b4f76fe5b7d7",
  measurementId: "G-KFWK18TQ6R",
};

const app = initializeApp(firebaseConfig);

// Only the Auth SDK was used anywhere in this app until 2.0.71 - no Realtime
// Database, no Storage, no Analytics. Marko's own console snippet included
// `firebase/analytics`, deliberately not pulled in here: it would add a
// dependency and network calls for a feature nobody asked for (see the
// "Local-first" philosophy notes elsewhere in this codebase).
export const auth = getAuth(app);

// 2.0.71: added for exactly one purpose - the account-approval gate (marko's
// own request: stop one person from registering multiple accounts). Holds a
// single `users/{uid}` doc per Firebase account with one meaningful field
// (`approved`), written once at registration and never again from this app -
// see lib/auth.tsx's register()/loginWithGoogle()/fetchApproved, and
// firestore.rules (repo root) for the security rules that make the "never
// again from this app" part actually enforced, not just a convention. Do not
// reach for this for anything else without updating that rules file too.
export const db = getFirestore(app);

// 2.5.2: "Forgot password?" (see lib/auth.tsx's requestPasswordReset) - the
// short version of a much longer story, told in full here because every
// piece of it lives in a different file and someone reading just one of them
// needs to find the rest.
//
// Firebase's password-reset email always contains a LINK, never a short
// typeable code (confirmed against Firebase's own docs, not assumed) - the
// `oobCode` in that link is a long opaque token, not a 6-digit PIN. Building
// a real short-code flow would mean generating/verifying codes ourselves,
// which Firebase's client SDK cannot do alone: sending a custom email and
// changing a password for someone who isn't currently signed in both need a
// trusted backend (Firebase Admin SDK), and this app has never had one - no
// Cloud Functions, no `firebase.json`/`.firebaserc` anywhere in this repo
// (verified before writing this). Standing one up would also force this
// project onto Firebase's paid "Blaze" plan (a billing card on the project,
// even though actual usage would stay inside the free quota) - marko chose
// against paying that cost for this feature (same reasoning he gave for
// leaving Discord sign-in for later - see PROTECTED_AREAS.md's 2.5.2 entry).
//
// So this uses Firebase's own link-based reset, but points the link at
// TIQR itself instead of a browser tab:
//
// 1. `handleCodeInApp: true` below is what makes Firebase build the emailed
//    link as `<url>?mode=resetPassword&oobCode=...&apiKey=...` (this exact
//    `url`, with the action params appended) instead of Firebase's own
//    generic hosted handler page.
// 2. `url` has to be a real http(s) page - Firebase rejects (and requires
//    listing in Console -> Authentication -> Settings -> Authorized domains)
//    anything else, a custom `tiqrmanager://` scheme included. So `url`
//    points at `docs/reset-redirect.html`, published for free via the same
//    GitHub Pages setup this repo already uses for the privacy policy page
//    (see REDESIGN-2.0.5-REPORT.md section 3 - `docs/privacy.html` is live
//    at `https://biznismarko9-source.github.io/tiqr-manager/privacy.html`;
//    this reuses that exact same Pages site, just one more file in it).
// 3. That static page immediately forwards its own query string to
//    `tiqrmanager://reset-password?...` - seat this app's custom URL scheme,
//    registered via tauri-plugin-deep-link (see src-tauri/tauri.conf.json's
//    `plugins.deep-link` and src-tauri/Cargo.toml's own comment on that
//    dependency). Windows hands that straight to TIQR if it's installed.
// 4. App.tsx's deep-link bridge catches it, pulls `oobCode` back out, and
//    sends the person to `/reset-password` (src/pages/ResetPassword.tsx) -
//    which finishes the reset with `verifyPasswordResetCode`/
//    `confirmPasswordReset` (see lib/auth.tsx), both plain client-side
//    Firebase Auth calls, no backend involved anywhere in this chain.
//
// ONE-TIME MANUAL STEP THIS NEEDS (same "can't be deployed by code, has to
// be pasted/clicked by hand once" situation as firestore.rules - see that
// file's own comment): add `biznismarko9-source.github.io` to Firebase
// Console -> Authentication -> Settings -> Authorized domains. GitHub Pages
// itself is very likely already on (it was required for the Google Sheets
// OAuth consent screen back in 2.0.5) - worth a quick check that
// `docs/reset-redirect.html` actually loads before relying on this.
export const PASSWORD_RESET_ACTION_CODE_SETTINGS = {
  url: "https://biznismarko9-source.github.io/tiqr-manager/reset-redirect.html",
  handleCodeInApp: true,
};

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

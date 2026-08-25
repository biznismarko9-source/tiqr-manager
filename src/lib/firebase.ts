import { initializeApp } from "firebase/app";
import { getAuth } from "firebase/auth";

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

// Only the Auth SDK is used anywhere in this app right now - no Firestore,
// no Realtime Database, no Storage, no Analytics. Marko's own console
// snippet included `firebase/analytics`, deliberately not pulled in here:
// it would add a dependency and network calls for a feature nobody asked
// for (see the "Local-first" philosophy notes elsewhere in this codebase).
export const auth = getAuth(app);

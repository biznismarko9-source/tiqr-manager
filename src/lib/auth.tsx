import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import {
  createUserWithEmailAndPassword,
  GoogleAuthProvider,
  onAuthStateChanged,
  signInWithCredential,
  signInWithEmailAndPassword,
  signOut,
  updateProfile,
  type User,
} from "firebase/auth";
import { auth } from "./firebase";
import { api } from "./api";

// 2.0.45: real Firebase email/password auth - replaces 2.0.44's localStorage
// placeholder (that version's own doc comment explained why a placeholder
// shipped first). See firebase.ts for the project config + why it's safe to
// ship in source.
//
// 2.0.46: Google sign-in is real too now. Firebase's usual browser-popup
// flow (`signInWithPopup`) doesn't fit a Tauri webview - this app's own
// history already found that out building the Sheets "Sign in with Google"
// feature (see google_oauth.rs's module doc comment) - so `loginWithGoogle`
// below reuses that exact proven shape instead: a Rust command
// (`api.startFirebaseGoogleSignIn`) opens the person's own system browser,
// runs a loopback-listener/PKCE OAuth dance against a SEPARATE OAuth client
// (google_oauth::embedded_firebase_oauth_client - a different Google Cloud
// project and a narrower identity-only scope than the Sheets one), and
// hands back a Google ID token. That ID token is what actually completes
// the Firebase side of the sign-in here (`GoogleAuthProvider.credential` +
// `signInWithCredential`) - Firebase never sees a password, a popup, or
// anything running inside this app's own window.

export interface AuthUser {
  name: string;
  email: string;
  provider: "password" | "google";
}

interface AuthContextValue {
  user: AuthUser | null;
  /** True only until Firebase's own session-restore has resolved once on
   * launch - see the effect below for why App.tsx's RequireAuth needs this. */
  loading: boolean;
  login: (email: string, password: string) => Promise<void>;
  register: (name: string, email: string, password: string) => Promise<void>;
  loginWithGoogle: () => Promise<void>;
  logout: () => Promise<void>;
  updateName: (name: string) => Promise<void>;
}

const AuthContext = createContext<AuthContextValue | null>(null);

function toAuthUser(u: User): AuthUser {
  return {
    name: u.displayName || u.email?.split("@")[0] || "You",
    email: u.email ?? "",
    provider: u.providerData[0]?.providerId === "google.com" ? "google" : "password",
  };
}

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<AuthUser | null>(null);
  // Firebase restores a persisted session ASYNCHRONOUSLY on launch -
  // `auth.currentUser` reads null for a brief moment even when someone
  // really is signed in, until this first fires. Without tracking that
  // separately from `user`, every single app launch would flash the
  // Welcome/login screen for an instant before snapping back to whatever
  // page was open - see App.tsx's RequireAuth, which waits on this instead
  // of treating `user === null` as "definitely signed out" during that
  // window.
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const unsubscribe = onAuthStateChanged(auth, (firebaseUser) => {
      setUser(firebaseUser ? toAuthUser(firebaseUser) : null);
      setLoading(false);
    });
    return unsubscribe;
  }, []);

  const login = useCallback(async (email: string, password: string) => {
    await signInWithEmailAndPassword(auth, email, password);
    // No manual setUser needed - onAuthStateChanged above fires on its own
    // the moment Firebase accepts the sign-in.
  }, []);

  const register = useCallback(async (name: string, email: string, password: string) => {
    const cred = await createUserWithEmailAndPassword(auth, email, password);
    await updateProfile(cred.user, { displayName: name });
    // updateProfile alone does NOT re-fire onAuthStateChanged (a profile
    // field change isn't an auth-state change) - update locally too so the
    // name appears immediately rather than only after a relaunch.
    setUser(toAuthUser(cred.user));
  }, []);

  const loginWithGoogle = useCallback(async (): Promise<void> => {
    // Errors from this first step are Rust/Tauri errors (e.g. "Google
    // sign-in was cancelled.") - plain, already human-readable strings with
    // no Firebase `.code`, deliberately left to propagate as-is rather than
    // wrapped. Welcome.tsx's catch block checks for `.code` to decide which
    // formatter (firebaseAuthErrorMessage vs errMsg) a given error needs.
    const { idToken } = await api.startFirebaseGoogleSignIn();
    const credential = GoogleAuthProvider.credential(idToken);
    const cred = await signInWithCredential(auth, credential);
    // Belt-and-suspenders, same reasoning as register() above: Firebase
    // normally fires onAuthStateChanged on its own right after this
    // resolves, but setting eagerly here means the UI never has to wait on
    // that round trip to show the right name.
    setUser(toAuthUser(cred.user));
  }, []);

  const logout = useCallback(async () => {
    await signOut(auth);
  }, []);

  const updateName = useCallback(async (name: string) => {
    if (!auth.currentUser) return;
    await updateProfile(auth.currentUser, { displayName: name });
    setUser(toAuthUser(auth.currentUser));
  }, []);

  const value = useMemo<AuthContextValue>(
    () => ({ user, loading, login, register, loginWithGoogle, logout, updateName }),
    [user, loading, login, register, loginWithGoogle, logout, updateName],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth(): AuthContextValue {
  const ctx = useContext(AuthContext);
  if (!ctx) throw new Error("useAuth must be used inside <AuthProvider>");
  return ctx;
}

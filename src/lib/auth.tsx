import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import {
  createUserWithEmailAndPassword,
  getAdditionalUserInfo,
  GoogleAuthProvider,
  onAuthStateChanged,
  signInWithCredential,
  signInWithEmailAndPassword,
  signOut,
  updateProfile,
  type User,
} from "firebase/auth";
import { doc, getDoc, serverTimestamp, setDoc } from "firebase/firestore";
import { auth, db } from "./firebase";
import { api, errMsg } from "./api";

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
  /** 2.0.71: null while the Firestore approval check for the current `user`
   * hasn't resolved yet (or there is no user), true if they can use the app,
   * false if App.tsx's RequireAuth should show PendingApproval instead. See
   * fetchApproved below for exactly what "approved" means. */
  approved: boolean | null;
  /** 2.0.72: true once the per-account database file for `user` is open and
   * migrated - see `switchDatabaseFor` below. Only meaningful once `approved`
   * is true; stays false the whole time an account is still pending, since
   * there is nothing to switch to yet. */
  dbReady: boolean;
  /** 2.0.72: set when switching to this account's database file failed (e.g.
   * disk full or a permissions problem). App.tsx's RequireAuth shows
   * DatabaseError instead of the app itself while this is non-null. */
  dbError: string | null;
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

// 2.0.71: any account created before this feature shipped has no
// `users/{uid}` doc at all and must never be blocked by a gate that didn't
// exist when they signed up (marko's own existing account included) - a
// missing doc for one of those is treated as approved. An account created
// ON OR AFTER this cutoff is a different story: for THOSE, a missing doc
// means the write in register()/loginWithGoogle() below failed (Firestore
// not enabled yet, rules not pasted in, offline at exactly the wrong
// moment, ...) - and the correct failure direction there is "still
// pending", never "let them in". Deliberately a fixed instant, not
// "today" recomputed live - it must keep meaning the same real moment
// every time this code runs, on every machine, in every timezone.
const APPROVAL_GATE_CUTOFF = new Date("2026-08-28T00:00:00Z");

// 2.0.72: pulled out of fetchApproved (used to be an inline closure there
// named `createdBeforeGate`) so the per-account database switch below can
// call the exact same check - an account is "legacy" (keeps using the one
// original shared database file) if and only if it already existed before
// the approval gate's own cutoff. Keeping this as one single function is
// what guarantees the approval check and the database-file choice can never
// drift apart from each other.
function isGrandfatheredAccount(firebaseUser: User): boolean {
  const createdAt = firebaseUser.metadata.creationTime ? new Date(firebaseUser.metadata.creationTime) : null;
  return createdAt !== null && createdAt < APPROVAL_GATE_CUTOFF;
}

/** 2.0.71: the one Firestore read this whole app makes. See
 * APPROVAL_GATE_CUTOFF's own comment for the missing-doc reasoning, and
 * firestore.rules (repo root) for why the client can create this doc with
 * approved:false but can never itself flip it to true - only marko can, by
 * hand, in the Firebase Console.
 *
 * Deliberately never rejects - both "no doc" AND "the read itself failed"
 * (offline, Firestore not enabled yet, rules not pasted in yet, ...) fall
 * back to the exact same cutoff rule below, on purpose: a failed read must
 * NOT be treated as automatic approval, or shipping this before Firestore
 * is actually set up in the Console would silently wave every brand-new
 * registration straight through with no gate at all. An old-enough account
 * gets the same benefit of the doubt either way (missing doc or failed
 * read); a new-enough one gets approved only by an actual `approved: true`
 * successfully read back. */
async function fetchApproved(firebaseUser: User): Promise<boolean> {
  try {
    const snap = await getDoc(doc(db, "users", firebaseUser.uid));
    return snap.exists() ? snap.data().approved === true : isGrandfatheredAccount(firebaseUser);
  } catch {
    return isGrandfatheredAccount(firebaseUser);
  }
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
  // 2.0.71: see AuthContextValue's own doc comment for what each value means.
  const [approved, setApproved] = useState<boolean | null>(null);
  // 2.0.72: see AuthContextValue's own doc comments for what these mean.
  const [dbReady, setDbReady] = useState(false);
  const [dbError, setDbError] = useState<string | null>(null);

  // 2.0.72: swaps in this account's own SQLite file - called once, right
  // after `approved` resolves to true. There is nothing meaningful to switch
  // to before that: a still-pending account never reaches a data-consuming
  // page (App.tsx's RequireAuth checks `approved` first, before ever looking
  // at `dbReady`). Never throws - a failure is reported through `dbError`
  // instead, the same pattern `fetchApproved` already uses above for
  // Firestore read failures.
  const switchDatabaseFor = useCallback(async (firebaseUser: User) => {
    setDbError(null);
    try {
      await api.switchActiveDatabase(firebaseUser.uid, isGrandfatheredAccount(firebaseUser));
      setDbReady(true);
    } catch (err) {
      setDbReady(false);
      setDbError(errMsg(err));
    }
  }, []);

  useEffect(() => {
    const unsubscribe = onAuthStateChanged(auth, (firebaseUser) => {
      setUser(firebaseUser ? toAuthUser(firebaseUser) : null);
      if (!firebaseUser) {
        setApproved(null);
        setDbReady(false);
        setDbError(null);
        setLoading(false);
        return;
      }
      // fetchApproved never rejects - see its own doc comment for how it
      // handles a Firestore read failure (not the same as "approved").
      fetchApproved(firebaseUser)
        .then((isApproved) => {
          setApproved(isApproved);
          // Fire-and-forget: RequireAuth independently gates on `dbReady`,
          // so there's nothing more to sequence here.
          if (isApproved) switchDatabaseFor(firebaseUser);
        })
        .finally(() => setLoading(false));
    });
    return unsubscribe;
  }, [switchDatabaseFor]);

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
    // 2.0.71: marks this brand-new account pending - see firestore.rules for
    // why the app can write approved:false here but can never write true.
    // Failure is swallowed on purpose, not surfaced as a registration error:
    // the account itself was created successfully either way, and
    // fetchApproved's own cutoff logic already treats a missing doc on a
    // new-enough account as pending, not approved - so there's no silent
    // bypass here, just a worse Console experience for marko on that one
    // account (no name/email to see there, only pending status once he
    // notices it in Authentication instead of Firestore).
    try {
      await setDoc(doc(db, "users", cred.user.uid), { name, email, approved: false, createdAt: serverTimestamp() });
    } catch {
      // see comment above
    }
    setApproved(false);
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
    // 2.0.71: a first-ever Google sign-in is also a brand-new account - same
    // gate as register() above. getAdditionalUserInfo is the modular SDK's
    // own way to tell "just created" apart from "signed in before", no
    // extra network round trip needed to know which one this is.
    if (getAdditionalUserInfo(cred)?.isNewUser) {
      try {
        await setDoc(doc(db, "users", cred.user.uid), {
          name: toAuthUser(cred.user).name,
          email: cred.user.email ?? "",
          approved: false,
          createdAt: serverTimestamp(),
        });
      } catch {
        // see register()'s own comment on why this is swallowed on purpose
      }
      setApproved(false);
    } else {
      const isApproved = await fetchApproved(cred.user);
      setApproved(isApproved);
      if (isApproved) await switchDatabaseFor(cred.user);
    }
  }, [switchDatabaseFor]);

  const logout = useCallback(async () => {
    await signOut(auth);
  }, []);

  const updateName = useCallback(async (name: string) => {
    if (!auth.currentUser) return;
    await updateProfile(auth.currentUser, { displayName: name });
    setUser(toAuthUser(auth.currentUser));
  }, []);

  const value = useMemo<AuthContextValue>(
    () => ({ user, loading, approved, dbReady, dbError, login, register, loginWithGoogle, logout, updateName }),
    [user, loading, approved, dbReady, dbError, login, register, loginWithGoogle, logout, updateName],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth(): AuthContextValue {
  const ctx = useContext(AuthContext);
  if (!ctx) throw new Error("useAuth must be used inside <AuthProvider>");
  return ctx;
}

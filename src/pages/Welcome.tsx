import { useEffect, useState, type FormEvent } from "react";
import { Navigate, useNavigate } from "react-router-dom";
import { Button, Card, Field, Input, Spinner } from "../components/ui";
import { IconGoogle } from "../components/icons";
import { api, errMsg } from "../lib/api";
import { useAuth } from "../lib/auth";
import { firebaseAuthErrorMessage } from "../lib/firebaseErrors";
import { useToast } from "../lib/toast";
import logo from "../assets/logo.png";

type Mode = "login" | "register" | "forgot";

// 2.0.44: the app's very first screen when nobody is signed in yet - see
// App.tsx's RequireAuth wrapper, which sends any not-signed-in visit here
// regardless of which page it was actually headed to. 2.0.44 shipped this
// screen backed by a disposable placeholder auth so marko could review the
// UX before any real backend work began; 2.0.45 wired up real Firebase
// email/password, and 2.0.46 wired up real "Continue with Google" too (see
// lib/auth.tsx for both). None of the app's own data (orders/tickets/
// sales/...) is touched by any of this - only what's shown before you
// reach it.
//
// 2.5.2: "forgot" is a third mode, but not a peer of login/register the way
// they are of each other - it's only ever reached FROM login (the tab
// switcher below hides itself in this mode; "Back to log in" is how you
// leave it). See lib/auth.tsx's requestPasswordReset and
// pages/ResetPassword.tsx for the rest of the flow this kicks off.
export default function Welcome() {
  const { user, loading, login, register, loginWithGoogle, requestPasswordReset } = useAuth();
  const navigate = useNavigate();
  const toast = useToast();
  const [mode, setMode] = useState<Mode>("login");
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [forgotEmail, setForgotEmail] = useState("");
  const [forgotSent, setForgotSent] = useState(false);
  const [forgotBusy, setForgotBusy] = useState(false);
  // 2.0.46: defaults to false (not null) on purpose - starting "enabled"
  // and flipping to disabled a moment later would be a worse flash than
  // starting disabled and flipping to enabled once this quick, local-only
  // check actually resolves (see commands/firebase_google_auth.rs's own
  // doc comment - no network call, just reading whether this build has an
  // OAuth client embedded).
  const [googleAvailable, setGoogleAvailable] = useState(false);
  const [googleBusy, setGoogleBusy] = useState(false);

  useEffect(() => {
    api
      .firebaseGoogleSignInAvailable()
      .then(setGoogleAvailable)
      .catch(() => setGoogleAvailable(false));
  }, []);

  // Still restoring a persisted Firebase session - see lib/auth.tsx's own
  // doc comment. Render nothing rather than flashing this form first.
  if (loading) return null;
  // Already signed in (e.g. typed /welcome by hand, or a stale tab) - no
  // reason to show this screen again.
  if (user) return <Navigate to="/" replace />;

  async function handleSubmit(e: FormEvent) {
    e.preventDefault();
    if (!email.trim() || !password) {
      toast.error("Fill in your email and password.");
      return;
    }
    if (mode === "register") {
      if (!name.trim()) {
        toast.error("Enter your name.");
        return;
      }
      if (password !== confirmPassword) {
        toast.error("Passwords don't match.");
        return;
      }
    }
    setBusy(true);
    try {
      if (mode === "login") await login(email.trim(), password);
      else await register(name.trim(), email.trim(), password);
      navigate("/");
    } catch (err) {
      toast.error(firebaseAuthErrorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  async function handleGoogle() {
    setGoogleBusy(true);
    try {
      await loginWithGoogle();
      navigate("/");
    } catch (err) {
      // Two genuinely different error shapes can reach here (see
      // loginWithGoogle's own doc comment): a Rust/Tauri error from opening
      // the browser and waiting on it (no `.code`, already a plain
      // human-readable string - errMsg handles that), or a real Firebase
      // auth error once a credential was actually attempted (has a
      // `.code`, mapped by firebaseAuthErrorMessage same as login/register
      // above). Checking for `.code` first routes each to the formatter
      // that actually understands it, instead of losing the specific
      // Rust-side message behind a generic fallback.
      const hasFirebaseCode = typeof err === "object" && err !== null && "code" in err;
      toast.error(hasFirebaseCode ? firebaseAuthErrorMessage(err) : errMsg(err));
    } finally {
      setGoogleBusy(false);
    }
  }

  // 2.0.12's fix for the Sheets "Sign in with Google" card applies here too
  // (see Settings.tsx's GoogleSignInCard) - without a way to interrupt it,
  // closing the browser tab mid-flow would leave this button stuck reading
  // "Waiting for you to finish in your browser..." for up to 5 minutes.
  async function handleCancelGoogle() {
    try {
      await api.cancelFirebaseGoogleSignIn();
    } catch (err) {
      toast.error(errMsg(err));
    }
  }

  // 2.5.2: carries over whatever was already typed in the login form's email
  // field, if anything - a small courtesy so switching to "forgot" doesn't
  // throw away a correctly-typed email.
  function openForgotPassword() {
    setForgotEmail(email);
    setForgotSent(false);
    setMode("forgot");
  }

  async function handleForgotSubmit(e: FormEvent) {
    e.preventDefault();
    if (!forgotEmail.trim()) {
      toast.error("Enter your email.");
      return;
    }
    setForgotBusy(true);
    try {
      await requestPasswordReset(forgotEmail.trim());
      setForgotSent(true);
    } catch (err) {
      toast.error(firebaseAuthErrorMessage(err));
    } finally {
      setForgotBusy(false);
    }
  }

  return (
    <div className="flex min-h-full w-full items-center justify-center bg-slate-50 px-4 py-10 dark:bg-slate-950">
      <div className="w-full max-w-sm">
        <div className="mb-6 flex flex-col items-center text-center">
          <img src={logo} alt="TIQR Manager" className="h-12 w-12 rounded-xl shadow-sm" />
          <h1 className="mt-3 text-lg font-semibold text-slate-900 dark:text-slate-100">TIQR Manager</h1>
          <p className="mt-0.5 text-xs text-slate-400 dark:text-slate-500">Reseller toolkit</p>
        </div>

        <Card className="p-5">
          {/* 2.5.2: "forgot" replaces the whole card body rather than sitting
              alongside login/register in the tab switcher above - it isn't a
              peer of those two, it's a detour off of "Log in" (see
              openForgotPassword). "Back to log in" is the only way out. */}
          {mode === "forgot" ? (
            <>
              {!forgotSent ? (
                <form onSubmit={handleForgotSubmit} className="space-y-3">
                  <p className="text-sm text-slate-600 dark:text-slate-300">
                    Enter your email and we'll send you a link to reset your password.
                  </p>
                  <Field label="Email">
                    <Input
                      type="email"
                      value={forgotEmail}
                      onChange={(e) => setForgotEmail(e.target.value)}
                      placeholder="you@example.com"
                      autoComplete="email"
                      autoFocus
                    />
                  </Field>
                  <Button type="submit" variant="primary" className="w-full justify-center" disabled={forgotBusy}>
                    {forgotBusy ? <Spinner className="h-4 w-4" /> : "Send reset link"}
                  </Button>
                </form>
              ) : (
                <p className="text-sm text-slate-600 dark:text-slate-300">
                  If an account exists for{" "}
                  <span className="font-medium text-slate-900 dark:text-slate-100">{forgotEmail.trim()}</span>, a
                  reset link is on its way - opening it brings you straight back to TIQR Manager.
                </p>
              )}
              <button
                type="button"
                onClick={() => setMode("login")}
                className="mt-4 w-full text-center text-xs text-slate-400 hover:text-slate-600 dark:text-slate-500 dark:hover:text-slate-300"
              >
                Back to log in
              </button>
            </>
          ) : (
            <>
              <div className="mb-4 inline-flex w-full rounded-lg border border-slate-200 p-0.5 dark:border-slate-800">
                <button
                  type="button"
                  onClick={() => setMode("login")}
                  className={`flex-1 rounded-md py-1.5 text-sm font-medium transition-colors ${
                    mode === "login"
                      ? "bg-brand-600 text-white"
                      : "text-slate-500 hover:text-slate-900 dark:text-slate-400 dark:hover:text-slate-100"
                  }`}
                >
                  Log in
                </button>
                <button
                  type="button"
                  onClick={() => setMode("register")}
                  className={`flex-1 rounded-md py-1.5 text-sm font-medium transition-colors ${
                    mode === "register"
                      ? "bg-brand-600 text-white"
                      : "text-slate-500 hover:text-slate-900 dark:text-slate-400 dark:hover:text-slate-100"
                  }`}
                >
                  Sign up
                </button>
              </div>

              <form onSubmit={handleSubmit} className="space-y-3">
                {mode === "register" && (
                  <Field label="Name">
                    <Input value={name} onChange={(e) => setName(e.target.value)} placeholder="Your name" autoComplete="name" />
                  </Field>
                )}
                <Field label="Email">
                  <Input
                    type="email"
                    value={email}
                    onChange={(e) => setEmail(e.target.value)}
                    placeholder="you@example.com"
                    autoComplete="email"
                  />
                </Field>
                <Field label="Password">
                  <Input
                    type="password"
                    value={password}
                    onChange={(e) => setPassword(e.target.value)}
                    placeholder="********"
                    autoComplete={mode === "login" ? "current-password" : "new-password"}
                  />
                </Field>
                {mode === "login" && (
                  <button
                    type="button"
                    onClick={openForgotPassword}
                    className="-mt-1 text-xs text-slate-400 hover:text-brand-600 dark:text-slate-500 dark:hover:text-brand-400"
                  >
                    Forgot password?
                  </button>
                )}
                {mode === "register" && (
                  <Field label="Confirm password">
                    <Input
                      type="password"
                      value={confirmPassword}
                      onChange={(e) => setConfirmPassword(e.target.value)}
                      placeholder="********"
                      autoComplete="new-password"
                    />
                  </Field>
                )}
                <Button type="submit" variant="primary" className="w-full justify-center" disabled={busy}>
                  {mode === "login" ? "Log in" : "Create account"}
                </Button>
              </form>

              <div className="my-4 flex items-center gap-2 text-[11px] uppercase tracking-wide text-slate-400 dark:text-slate-500">
                <span className="h-px flex-1 bg-slate-200 dark:bg-slate-800" />
                or
                <span className="h-px flex-1 bg-slate-200 dark:bg-slate-800" />
              </div>

              {/* 2.0.46: real now - see lib/auth.tsx's loginWithGoogle. Disabled
                  only when this particular build has no Firebase OAuth client
                  embedded (googleAvailable, checked on mount above) - same
                  "never silently fake it" honesty as the Sheets sign-in card in
                  Settings, just without a persistent signed-in state to show
                  alongside it here. */}
              <Button
                variant="secondary"
                className={`w-full justify-center ${googleAvailable ? "" : "cursor-not-allowed opacity-60"}`}
                disabled={!googleAvailable || googleBusy}
                onClick={handleGoogle}
                title={googleAvailable ? undefined : "Google sign-in isn't available in this build."}
              >
                {googleBusy ? <Spinner className="h-4 w-4" /> : <IconGoogle className="h-4 w-4" />}
                {googleBusy ? "Waiting for you to finish in your browser..." : "Continue with Google"}
              </Button>
              {googleBusy && (
                <button
                  type="button"
                  onClick={handleCancelGoogle}
                  className="mt-2 w-full text-center text-xs text-slate-400 hover:text-slate-600 dark:text-slate-500 dark:hover:text-slate-300"
                >
                  Cancel
                </button>
              )}
            </>
          )}
        </Card>

        <p className="mt-4 text-center text-[11px] text-slate-400 dark:text-slate-500">
          Local-first &middot; your data stays on this device
        </p>
      </div>
    </div>
  );
}

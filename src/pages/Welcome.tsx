import { useState, type FormEvent } from "react";
import { Navigate, useNavigate } from "react-router-dom";
import { Button, Card, Field, Input } from "../components/ui";
import { IconGoogle } from "../components/icons";
import { useAuth } from "../lib/auth";
import { firebaseAuthErrorMessage } from "../lib/firebaseErrors";
import { useToast } from "../lib/toast";
import logo from "../assets/logo.png";

type Mode = "login" | "register";

// 2.0.44 (Phase 1): the app's very first screen when nobody is signed in
// yet - see App.tsx's RequireAuth wrapper, which sends any not-signed-in
// visit here regardless of which page it was actually headed to. Backed by
// the placeholder AuthProvider (lib/auth.tsx) for now, not real Firebase -
// marko asked to see and click through the actual screens before that gets
// wired in (see REDESIGN-2.0.44-REPORT.md). None of the app's own data
// (orders/tickets/sales/...) is touched by any of this - only what's shown
// before you reach it.
export default function Welcome() {
  const { user, loading, login, register } = useAuth();
  const navigate = useNavigate();
  const toast = useToast();
  const [mode, setMode] = useState<Mode>("login");
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [busy, setBusy] = useState(false);

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

  return (
    <div className="flex min-h-full w-full items-center justify-center bg-slate-50 px-4 py-10 dark:bg-slate-950">
      <div className="w-full max-w-sm">
        <div className="mb-6 flex flex-col items-center text-center">
          <img src={logo} alt="TIQR Manager" className="h-12 w-12 rounded-xl shadow-sm" />
          <h1 className="mt-3 text-lg font-semibold text-slate-900 dark:text-slate-100">TIQR Manager</h1>
          <p className="mt-0.5 text-xs text-slate-400 dark:text-slate-500">Reseller toolkit</p>
        </div>

        <Card className="p-5">
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

          {/* 2.0.45: Google sign-in isn't wired up yet (see lib/auth.tsx's
              doc comment) - shown but disabled, rather than silently faking
              a login the way 2.0.44's placeholder did, so it's honest about
              what actually works right now. */}
          <Button variant="secondary" className="w-full cursor-not-allowed justify-center opacity-60" disabled title="Coming soon">
            <IconGoogle className="h-4 w-4" /> Continue with Google
            <span className="ml-1 text-[10px] font-normal text-slate-400 dark:text-slate-500">(coming soon)</span>
          </Button>
        </Card>

        <p className="mt-4 text-center text-[11px] text-slate-400 dark:text-slate-500">
          Local-first &middot; your data stays on this device
        </p>
      </div>
    </div>
  );
}

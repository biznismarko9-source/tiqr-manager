import { useEffect, useState, type FormEvent } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { Button, Card, Field, Input, Spinner } from "../components/ui";
import { useAuth } from "../lib/auth";
import { firebaseAuthErrorMessage } from "../lib/firebaseErrors";
import { useToast } from "../lib/toast";
import logo from "../assets/logo.png";

// 2.5.2: the landing page for TIQR's own end of the "Forgot password?" flow
// - see lib/firebase.ts's PASSWORD_RESET_ACTION_CODE_SETTINGS for the full
// mechanism that gets a person here (short version: they click a link in a
// Firebase email, which opens THIS app via the `tiqrmanager://` scheme -
// App.tsx's deep-link bridge is what actually turns that OS-level URL into
// a navigation to this route, handing the oobCode it found along as router
// state - see this component's own `location.state` read below).
//
// Only ever reached two ways: via that deep link (the real path, always has
// an oobCode), or by someone typing/pasting the URL by hand with nothing
// after it (no oobCode - handled below as "missing" rather than crashing on
// a null). Deliberately outside RequireAuth in App.tsx, same as Welcome -
// resetting a password is, by definition, something you do while unable to
// sign in, so this can never depend on an existing session.
type Status = "checking" | "ready" | "invalid" | "missing" | "done";

export default function ResetPassword() {
  const { verifyPasswordResetCode, confirmPasswordReset } = useAuth();
  const location = useLocation();
  const navigate = useNavigate();
  const toast = useToast();

  const oobCode = (location.state as { oobCode?: string } | null)?.oobCode ?? null;

  const [status, setStatus] = useState<Status>(oobCode ? "checking" : "missing");
  const [email, setEmail] = useState("");
  const [invalidMessage, setInvalidMessage] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!oobCode) return;
    let cancelled = false;
    verifyPasswordResetCode(oobCode)
      .then((resolvedEmail) => {
        if (cancelled) return;
        setEmail(resolvedEmail);
        setStatus("ready");
      })
      .catch((err) => {
        if (cancelled) return;
        setInvalidMessage(firebaseAuthErrorMessage(err));
        setStatus("invalid");
      });
    // Deliberately no dependency on verifyPasswordResetCode itself beyond
    // mount - re-running this on every render would re-spend the oobCode's
    // one-time validity check for no reason. oobCode never changes for the
    // lifetime of this page (it comes from the route it was navigated to
    // with, not from anything that can be edited in place).
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [oobCode]);

  async function handleSubmit(e: FormEvent) {
    e.preventDefault();
    if (!oobCode) return;
    if (newPassword.length < 6) {
      toast.error("Password must be at least 6 characters.");
      return;
    }
    if (newPassword !== confirmPassword) {
      toast.error("Passwords don't match.");
      return;
    }
    setBusy(true);
    try {
      await confirmPasswordReset(oobCode, newPassword);
      setStatus("done");
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
          <p className="mt-0.5 text-xs text-slate-400 dark:text-slate-500">Reset your password</p>
        </div>

        <Card className="p-5">
          {status === "checking" && (
            <div className="flex flex-col items-center gap-3 py-6 text-sm text-slate-500 dark:text-slate-400">
              <Spinner className="h-5 w-5" />
              Checking your reset link...
            </div>
          )}

          {status === "missing" && (
            <div className="space-y-3 text-center">
              <p className="text-sm text-slate-600 dark:text-slate-300">
                This page opens from the link in a TIQR Manager password-reset email - it doesn't do anything on its
                own.
              </p>
              <Button variant="secondary" className="w-full justify-center" onClick={() => navigate("/welcome")}>
                Back to log in
              </Button>
            </div>
          )}

          {status === "invalid" && (
            <div className="space-y-3 text-center">
              <p className="text-sm text-slate-600 dark:text-slate-300">{invalidMessage}</p>
              <Button variant="secondary" className="w-full justify-center" onClick={() => navigate("/welcome")}>
                Back to log in
              </Button>
            </div>
          )}

          {status === "ready" && (
            <form onSubmit={handleSubmit} className="space-y-3">
              <p className="text-sm text-slate-600 dark:text-slate-300">
                Set a new password for <span className="font-medium text-slate-900 dark:text-slate-100">{email}</span>.
              </p>
              <Field label="New password">
                <Input
                  type="password"
                  value={newPassword}
                  onChange={(e) => setNewPassword(e.target.value)}
                  placeholder="********"
                  autoComplete="new-password"
                  autoFocus
                />
              </Field>
              <Field label="Confirm new password">
                <Input
                  type="password"
                  value={confirmPassword}
                  onChange={(e) => setConfirmPassword(e.target.value)}
                  placeholder="********"
                  autoComplete="new-password"
                />
              </Field>
              <Button type="submit" variant="primary" className="w-full justify-center" disabled={busy}>
                {busy ? <Spinner className="h-4 w-4" /> : "Set new password"}
              </Button>
            </form>
          )}

          {status === "done" && (
            <div className="space-y-3 text-center">
              <p className="text-sm text-slate-600 dark:text-slate-300">
                Your password has been changed. Log in with your new password.
              </p>
              <Button variant="primary" className="w-full justify-center" onClick={() => navigate("/welcome")}>
                Continue to log in
              </Button>
            </div>
          )}
        </Card>
      </div>
    </div>
  );
}

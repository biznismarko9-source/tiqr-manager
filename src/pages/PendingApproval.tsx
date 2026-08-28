import { Button, Card } from "../components/ui";
import { useAuth } from "../lib/auth";
import logo from "../assets/logo.png";

// 2.0.71: shown by App.tsx's RequireAuth instead of the app itself, for a
// signed-in account marko hasn't approved yet (see lib/auth.tsx's `approved`
// + firestore.rules). Deliberately has nothing to do but wait and log out -
// no "check again" button, because the exact same check this screen failed
// already re-runs automatically on every fresh sign-in and every app
// launch, not on a click here.
export default function PendingApproval() {
  const { user, logout } = useAuth();

  return (
    <div className="flex min-h-full w-full items-center justify-center bg-slate-50 px-4 py-10 dark:bg-slate-950">
      <div className="w-full max-w-sm">
        <div className="mb-6 flex flex-col items-center text-center">
          <img src={logo} alt="TIQR Manager" className="h-12 w-12 rounded-xl shadow-sm" />
          <h1 className="mt-3 text-lg font-semibold text-slate-900 dark:text-slate-100">TIQR Manager</h1>
        </div>

        <Card className="p-5 text-center">
          <h2 className="text-base font-semibold text-slate-900 dark:text-slate-100">Account pending approval</h2>
          <p className="mt-2 text-sm text-slate-500 dark:text-slate-400">
            {user?.email ? `${user.email} is` : "Your account is"} waiting to be approved before you can use TIQR
            Manager. This is usually quick - check back shortly.
          </p>
          <Button variant="secondary" className="mt-5 w-full justify-center" onClick={logout}>
            Log out
          </Button>
        </Card>
      </div>
    </div>
  );
}

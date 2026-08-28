import { Button, Card } from "../components/ui";
import { useAuth } from "../lib/auth";
import logo from "../assets/logo.png";

// 2.0.72: shown by App.tsx's RequireAuth instead of the app itself, for a
// signed-in AND approved account whose own database file failed to open
// (disk full, a permissions problem, ...) - see lib/auth.tsx's
// `switchDatabaseFor`. Kept as its own separate screen rather than folded
// into PendingApproval: this is a different, much rarer problem, and "your
// account isn't approved yet" would misrepresent what actually went wrong.
// Like PendingApproval, there's deliberately no "try again" button - the
// same switch this screen reports failing already re-runs automatically on
// every fresh sign-in and every app launch, not on a click here.
export default function DatabaseError() {
  const { dbError, logout } = useAuth();

  return (
    <div className="flex min-h-full w-full items-center justify-center bg-slate-50 px-4 py-10 dark:bg-slate-950">
      <div className="w-full max-w-sm">
        <div className="mb-6 flex flex-col items-center text-center">
          <img src={logo} alt="TIQR Manager" className="h-12 w-12 rounded-xl shadow-sm" />
          <h1 className="mt-3 text-lg font-semibold text-slate-900 dark:text-slate-100">TIQR Manager</h1>
        </div>

        <Card className="p-5 text-center">
          <h2 className="text-base font-semibold text-slate-900 dark:text-slate-100">Couldn't open your data</h2>
          <p className="mt-2 text-sm text-slate-500 dark:text-slate-400">
            Something went wrong opening your account's database{dbError ? `: ${dbError}` : "."} Try logging out and
            back in - if it keeps happening, get in touch for help.
          </p>
          <Button variant="secondary" className="mt-5 w-full justify-center" onClick={logout}>
            Log out
          </Button>
        </Card>
      </div>
    </div>
  );
}

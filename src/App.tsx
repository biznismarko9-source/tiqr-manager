import { useEffect, type ReactNode } from "react";
import { HashRouter, Navigate, Route, Routes, useNavigate } from "react-router-dom";
import { getCurrent, onOpenUrl } from "@tauri-apps/plugin-deep-link";
import Layout from "./components/Layout";
import { ToastProvider } from "./lib/toast";
import { AuthProvider, useAuth } from "./lib/auth";
import { useTheme } from "./lib/theme";
import Welcome from "./pages/Welcome";
import ResetPassword from "./pages/ResetPassword";
import PendingApproval from "./pages/PendingApproval";
import DatabaseError from "./pages/DatabaseError";
import Dashboard from "./pages/Dashboard";
import Calendar from "./pages/Calendar";
import Events from "./pages/Events";
import EventDetail from "./pages/EventDetail";
import Orders from "./pages/Orders";
import OrderDetail from "./pages/OrderDetail";
import Tickets from "./pages/Tickets";
import Inventory from "./pages/Inventory";
import Sales from "./pages/Sales";
import SaleDetail from "./pages/SaleDetail";
import Pulls from "./pages/Pulls";
import PriceChecker from "./pages/PriceChecker";
import Finance from "./pages/Finance";
import TicketCenter from "./pages/TicketCenter";
import Settings from "./pages/Settings";

// 2.0.44: gates the whole app behind sign-in (see Welcome.tsx + lib/auth.tsx)
// - anything not logged in gets sent to /welcome instead, no matter what
// path it was actually headed to. Every existing route below is completely
// unchanged - this only wraps them.
//
// 2.0.45: while `loading` is true, renders nothing rather than redirecting -
// real Firebase restores a persisted session ASYNCHRONOUSLY on launch, so
// `user` reads null for a brief instant even when someone really is signed
// in. Redirecting during that window would flash the Welcome screen on
// every single app launch before snapping back. A blank instant is better
// than a wrong-then-corrected screen.
//
// 2.0.71: `approved === null` gets the exact same "render nothing, don't
// flash the wrong screen" treatment as `loading` - it means the Firestore
// approval check for this `user` hasn't resolved yet (see lib/auth.tsx).
// Checked after the `!user` redirect on purpose: there is nothing to check
// approval FOR until someone is actually signed in.
//
// 2.0.72: `dbError`/`dbReady` are checked last, after `approved` is
// confirmed true - there is nothing to switch a database for until then (see
// lib/auth.tsx's `switchDatabaseFor`), so a still-pending account can never
// reach either of these checks.
function RequireAuth({ children }: { children: ReactNode }) {
  const { user, loading, approved, dbReady, dbError } = useAuth();
  if (loading) return null;
  if (!user) return <Navigate to="/welcome" replace />;
  if (approved === null) return null;
  if (!approved) return <PendingApproval />;
  if (dbError) return <DatabaseError />;
  if (!dbReady) return null;
  return <>{children}</>;
}

// 2.5.2: the receiving end of "Forgot password?" - see lib/firebase.ts's
// PASSWORD_RESET_ACTION_CODE_SETTINGS for how a link in an email turns into
// an OS-level open of this app via the `tiqrmanager://` scheme. Mounted
// once, unconditionally, alongside <Routes> rather than inside any one
// route or behind RequireAuth - the whole point of a password reset is that
// it has to work while signed out, and the link can arrive at any time
// (app already open on some other page, or a cold start).
//
// Two separate cases from @tauri-apps/plugin-deep-link, both needed:
// `getCurrent()` covers a cold start (app wasn't running - the OS launched
// it because of this link), `onOpenUrl` covers the app already running
// (single-instance + the `deep-link` Cargo feature on it - see Cargo.toml's
// own comment - route it here as an event instead of a second launch).
// Only ever acts on a URL that actually carries an `oobCode` - anything
// else (there is no other kind of tiqrmanager:// link yet, but this stays
// defensive rather than assuming) is silently ignored.
function PasswordResetDeepLinkBridge() {
  const navigate = useNavigate();

  useEffect(() => {
    function handleUrls(urls: string[] | null) {
      const oobCode = extractOobCode(urls);
      if (oobCode) navigate("/reset-password", { state: { oobCode } });
    }

    let cancelled = false;
    getCurrent()
      .then((urls) => {
        if (!cancelled) handleUrls(urls);
      })
      .catch(() => {
        // No deep-link support in this build/platform, or nothing to read -
        // not an error worth surfacing, the app works fine either way.
      });

    const unlistenPromise = onOpenUrl((urls) => handleUrls(urls));
    return () => {
      cancelled = true;
      unlistenPromise.then((unlisten) => unlisten()).catch(() => {});
    };
  }, [navigate]);

  return null;
}

function extractOobCode(urls: string[] | null): string | null {
  if (!urls) return null;
  for (const raw of urls) {
    try {
      const oobCode = new URL(raw).searchParams.get("oobCode");
      if (oobCode) return oobCode;
    } catch {
      // Not a parseable URL - ignore rather than throw, this is best-effort.
    }
  }
  return null;
}

export default function App() {
  // Applies the saved theme (light/dark/system) to <html> as early as
  // possible on launch. The Settings page owns the interactive toggle.
  useTheme();

  return (
    <AuthProvider>
      <ToastProvider>
        <HashRouter>
          <PasswordResetDeepLinkBridge />
          <Routes>
            <Route path="/welcome" element={<Welcome />} />
            {/* 2.5.2: outside RequireAuth, same reasoning as /welcome - see
                PasswordResetDeepLinkBridge above for how you actually get
                here (never a link you'd type by hand). */}
            <Route path="/reset-password" element={<ResetPassword />} />
            <Route
              element={
                <RequireAuth>
                  <Layout />
                </RequireAuth>
              }
            >
              <Route index element={<Dashboard />} />
              {/* 2.5.0: "TIQR Operations Calendar" - a new cross-domain
                  overview page, same level as Dashboard (not nested under
                  Tickets/Finance) - see Layout.tsx's own 2.5.0 comment and
                  commands/calendar.rs's module doc comment. */}
              <Route path="calendar" element={<Calendar />} />
              <Route path="events" element={<Events />} />
              <Route path="events/:id" element={<EventDetail />} />
              <Route path="orders" element={<Orders />} />
              <Route path="orders/:id" element={<OrderDetail />} />
              <Route path="tickets" element={<Tickets />} />
              <Route path="inventory" element={<Inventory />} />
              <Route path="sales" element={<Sales />} />
              <Route path="sales/:id" element={<SaleDetail />} />
              <Route path="pulls" element={<Pulls />} />
              <Route path="price-checker" element={<PriceChecker />} />
              <Route path="finance" element={<Finance />} />
              {/* 2.5.1: Ticket Center is a standalone top-level route again -
                  briefly a Finance subtab in 2.4.4 (see Finance.tsx and
                  Layout.tsx's own 2.5.1 comments), marko asked for it back
                  out on its own, and rebuilt around orders rather than the
                  old per-ticket Control Center/Fulfillment Center pages
                  (both removed this version - see TicketCenter.tsx). */}
              <Route path="ticket-center" element={<TicketCenter />} />
              <Route path="settings" element={<Settings />} />
              {/* 1.8.2: Settings Home (above) plus one real route per section -
                  HashRouter makes this refresh-stable with zero extra config,
                  see REDESIGN-1.8.2-REPORT.md section 5. Settings.tsx branches
                  on useParams().section to render either view - no new page
                  component, no new router. */}
              <Route path="settings/:section" element={<Settings />} />
            </Route>
          </Routes>
        </HashRouter>
      </ToastProvider>
    </AuthProvider>
  );
}

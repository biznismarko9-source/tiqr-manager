import { type ReactNode } from "react";
import { HashRouter, Navigate, Route, Routes } from "react-router-dom";
import Layout from "./components/Layout";
import { ToastProvider } from "./lib/toast";
import { AuthProvider, useAuth } from "./lib/auth";
import { useTheme } from "./lib/theme";
import Welcome from "./pages/Welcome";
import Dashboard from "./pages/Dashboard";
import Events from "./pages/Events";
import EventDetail from "./pages/EventDetail";
import Orders from "./pages/Orders";
import OrderDetail from "./pages/OrderDetail";
import Tickets from "./pages/Tickets";
import Inventory from "./pages/Inventory";
import Sales from "./pages/Sales";
import SaleDetail from "./pages/SaleDetail";
import Pulls from "./pages/Pulls";
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
function RequireAuth({ children }: { children: ReactNode }) {
  const { user, loading } = useAuth();
  if (loading) return null;
  if (!user) return <Navigate to="/welcome" replace />;
  return <>{children}</>;
}

export default function App() {
  // Applies the saved theme (light/dark/system) to <html> as early as
  // possible on launch. The Settings page owns the interactive toggle.
  useTheme();

  return (
    <AuthProvider>
      <ToastProvider>
        <HashRouter>
          <Routes>
            <Route path="/welcome" element={<Welcome />} />
            <Route
              element={
                <RequireAuth>
                  <Layout />
                </RequireAuth>
              }
            >
              <Route index element={<Dashboard />} />
              <Route path="events" element={<Events />} />
              <Route path="events/:id" element={<EventDetail />} />
              <Route path="orders" element={<Orders />} />
              <Route path="orders/:id" element={<OrderDetail />} />
              <Route path="tickets" element={<Tickets />} />
              <Route path="inventory" element={<Inventory />} />
              <Route path="sales" element={<Sales />} />
              <Route path="sales/:id" element={<SaleDetail />} />
              <Route path="pulls" element={<Pulls />} />
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

import { HashRouter, Route, Routes } from "react-router-dom";
import Layout from "./components/Layout";
import { ToastProvider } from "./lib/toast";
import { useTheme } from "./lib/theme";
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

export default function App() {
  // Applies the saved theme (light/dark/system) to <html> as early as
  // possible on launch. The Settings page owns the interactive toggle.
  useTheme();

  return (
    <ToastProvider>
      <HashRouter>
        <Routes>
          <Route element={<Layout />}>
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
  );
}

import { HashRouter, Route, Routes } from "react-router-dom";
import Layout from "./components/Layout";
import { ToastProvider } from "./lib/toast";
import Dashboard from "./pages/Dashboard";
import Events from "./pages/Events";
import EventDetail from "./pages/EventDetail";
import Orders from "./pages/Orders";
import OrderDetail from "./pages/OrderDetail";
import Tickets from "./pages/Tickets";
import Inventory from "./pages/Inventory";
import Sales from "./pages/Sales";
import Settings from "./pages/Settings";

export default function App() {
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
            <Route path="settings" element={<Settings />} />
          </Route>
        </Routes>
      </HashRouter>
    </ToastProvider>
  );
}

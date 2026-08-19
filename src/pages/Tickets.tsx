import { useEffect, useMemo, useState } from "react";
import { Link, useLocation, useNavigate, useSearchParams } from "react-router-dom";
import { api, errMsg } from "../lib/api";
import type { EventWithStats, OrderRecord, Platform, Supplier, Ticket, TicketStatus, TicketUpdateInput } from "../lib/types";
import { formatDate, formatMoney } from "../lib/format";
import {
  Badge,
  Button,
  EmptyState,
  Field,
  Input,
  LoadingBlock,
  Modal,
  ModalFooter,
  PageHeader,
  Select,
  Textarea,
} from "../components/ui";
import { IconBoxes, IconSearch } from "../components/icons";
import { useToast } from "../lib/toast";

export default function Tickets() {
  return <TicketsView title="Tickets" subtitle="Every order you've purchased, grouped with its tickets." />;
}

/** An order's inventory status, derived purely from its ticket counts (there
 * is no separate DB column for this - see the report). "Active" means it
 * still has stock that could be listed/sold; "Sold out" means every ticket
 * has been sold or cancelled; "Cancelled" means the whole order was voided. */
function inventoryStatus(o: OrderRecord): { key: string; label: string } {
  if (o.cancelledCount === o.quantity && o.quantity > 0) return { key: "cancelled", label: "Cancelled" };
  if (o.availableCount + o.listedCount > 0) return { key: "active", label: "Active" };
  return { key: "soldout", label: "Sold out" };
}

// 1.8.3 (section 8 of the brief): remembers each page's last-used filters
// for this app session only, same module-level/session-only convention
// Sales.tsx already established in 1.8.0 (see its own `lastFilters`). Keyed
// by pathname (not a single shared value) because this one component backs
// TWO different pages - Tickets ("/tickets") and Inventory ("/inventory") -
// which must never leak each other's search/filters into one another.
interface TicketsFilterState {
  search: string;
  status: string;
  eventId: number | "";
  supplierId: number | "";
  platformId: number | "";
  section: string;
  dateFrom: string;
  dateTo: string;
}
const lastTicketsFilters = new Map<string, TicketsFilterState>();

/** Shared list view, reused (pre-filtered) by the Inventory page. Groups
 * tickets by their order - one row per order, not per ticket - so the list
 * stays fast and readable no matter how many individual tickets an order
 * generated. Clicking a row opens the existing Order Detail page, which
 * loads that order's individual tickets (and a sales summary) on demand. */
export function TicketsView({
  title,
  subtitle,
  lockedStatus,
}: {
  title: string;
  subtitle: string;
  lockedStatus?: string;
}) {
  const toast = useToast();
  const location = useLocation();
  const navigate = useNavigate();
  const [params] = useSearchParams();
  const cached = lastTicketsFilters.get(location.pathname);
  const [orders, setOrders] = useState<OrderRecord[] | null>(null);
  const [events, setEvents] = useState<EventWithStats[]>([]);
  const [suppliers, setSuppliers] = useState<Supplier[]>([]);
  const [platforms, setPlatforms] = useState<Platform[]>([]);
  const [search, setSearch] = useState(params.get("code") ?? cached?.search ?? "");
  const [status, setStatus] = useState(lockedStatus ?? cached?.status ?? "");
  const [eventId, setEventId] = useState<number | "">(cached?.eventId ?? "");
  const [supplierId, setSupplierId] = useState<number | "">(cached?.supplierId ?? "");
  const [platformId, setPlatformId] = useState<number | "">(cached?.platformId ?? "");
  const [section, setSection] = useState(cached?.section ?? "");
  const [dateFrom, setDateFrom] = useState(cached?.dateFrom ?? "");
  const [dateTo, setDateTo] = useState(cached?.dateTo ?? "");

  useEffect(() => {
    api.listEvents().then(setEvents).catch(() => {});
    api.listSuppliers().then(setSuppliers).catch(() => {});
    api.listPlatforms().then(setPlatforms).catch(() => {});
  }, []);

  // 1.8.3 (section 8): persist this page's own filters (see
  // lastTicketsFilters above) so returning here - in particular via Order
  // Detail's now context-aware Back link - finds the same search/filters
  // instead of a blank slate.
  useEffect(() => {
    lastTicketsFilters.set(location.pathname, { search, status, eventId, supplierId, platformId, section, dateFrom, dateTo });
  }, [location.pathname, search, status, eventId, supplierId, platformId, section, dateFrom, dateTo]);

  const load = () => {
    api
      .listOrders({
        search: search || undefined,
        eventId: eventId || undefined,
        supplierId: supplierId || undefined,
        platformId: platformId || undefined,
        status: status || undefined,
        section: section || undefined,
        dateFrom: dateFrom || undefined,
        dateTo: dateTo || undefined,
      })
      .then(setOrders)
      .catch((e) => toast.error(errMsg(e)));
  };

  useEffect(() => {
    const t = setTimeout(load, 200);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [search, status, eventId, supplierId, platformId, section, dateFrom, dateTo]);

  const summary = useMemo(() => {
    if (!orders) return null;
    const totalTickets = orders.reduce((sum, o) => sum + o.quantity, 0);
    const availableTickets = orders.reduce((sum, o) => sum + o.availableCount + o.listedCount, 0);
    return { orderCount: orders.length, totalTickets, availableTickets };
  }, [orders]);

  return (
    <div>
      <PageHeader title={title} subtitle={subtitle} />

      <div className="mb-4 flex flex-wrap items-end gap-3">
        <div className="w-52">
          <span className="label">Search</span>
          <div className="relative">
            <IconSearch className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-400 dark:text-slate-500" />
            <Input
              placeholder="Order code, event..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="pl-9"
            />
          </div>
        </div>
        {!lockedStatus && (
          <div className="w-40">
            <span className="label">Status</span>
            <Select value={status} onChange={(e) => setStatus(e.target.value)}>
              <option value="">All statuses</option>
              <option value="available">Available</option>
              <option value="listed">Listed</option>
              <option value="sold">Sold</option>
              <option value="cancelled">Cancelled</option>
            </Select>
          </div>
        )}
        <div className="w-48">
          <span className="label">Event</span>
          <Select value={eventId} onChange={(e) => setEventId(e.target.value ? Number(e.target.value) : "")}>
            <option value="">All events</option>
            {events.map((ev) => (
              <option key={ev.id} value={ev.id}>
                {ev.name}
              </option>
            ))}
          </Select>
        </div>
        <div className="w-40">
          <span className="label">Supplier</span>
          <Select value={supplierId} onChange={(e) => setSupplierId(e.target.value ? Number(e.target.value) : "")}>
            <option value="">All suppliers</option>
            {suppliers.map((s) => (
              <option key={s.id} value={s.id}>
                {s.name}
              </option>
            ))}
          </Select>
        </div>
        <div className="w-40">
          <span className="label">Platform</span>
          <Select value={platformId} onChange={(e) => setPlatformId(e.target.value ? Number(e.target.value) : "")}>
            <option value="">All platforms</option>
            {platforms.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name}
              </option>
            ))}
          </Select>
        </div>
        <div className="w-32">
          <span className="label">Section</span>
          <Input placeholder="e.g. 101" value={section} onChange={(e) => setSection(e.target.value)} />
        </div>
        <div className="w-36">
          <span className="label">From</span>
          <Input type="date" value={dateFrom} onChange={(e) => setDateFrom(e.target.value)} />
        </div>
        <div className="w-36">
          <span className="label">To</span>
          <Input type="date" value={dateTo} onChange={(e) => setDateTo(e.target.value)} />
        </div>
        {summary && (
          <p className="ml-auto text-xs text-slate-400 dark:text-slate-500">
            {summary.orderCount} orders &middot; {summary.totalTickets} tickets &middot; {summary.availableTickets} still
            sellable
          </p>
        )}
      </div>

      {orders && orders.length >= 5000 && (
        <div className="mb-4 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-800 dark:border-amber-500/30 dark:bg-amber-500/10 dark:text-amber-400">
          Showing the most recent 5,000 orders that match your filters. Narrow the search, status, or event filter to
          see the rest.
        </div>
      )}

      {orders === null ? (
        <LoadingBlock />
      ) : orders.length === 0 ? (
        <EmptyState icon={<IconBoxes className="h-8 w-8" />} title="No orders match these filters" />
      ) : (
        // 1.8.3 table-UX audit: table-layout:fixed + <colgroup> (see
        // Sales.tsx for the full rationale) instead of the old
        // min-w-[1000px]+overflow-x-auto pattern, which could scroll
        // horizontally on this app's smallest supported window. Also added
        // whole-row click-to-navigate, mirroring Events.tsx's own BUG #7 fix
        // (a click anywhere in the row that doesn't land on a link still
        // navigates to Order Detail; a click that does land on a link defers
        // to it, so nothing double-navigates) - this table's hover highlight
        // already visually implied the whole row was clickable, but only the
        // two link cells actually were. `state={{ from: location.pathname }}`
        // lets Order Detail's Back link return to this exact page (section 8).
        <div className="overflow-x-auto rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-sm">
          <table className="w-full table-fixed border-collapse">
            <colgroup>
              <col className="w-[92px]" />
              <col />
              <col className="w-[100px]" />
              <col className="w-[88px]" />
              <col className="w-12" />
              <col className="w-[70px]" />
              <col className="w-12" />
              <col className="w-[88px]" />
              <col className="w-[88px]" />
            </colgroup>
            <thead className="border-b border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-800/60">
              <tr>
                <th className="th-c">Order</th>
                <th className="th-c">Event</th>
                <th className="th-c">Supplier</th>
                <th className="th-c">Purchase date</th>
                <th className="th-c text-right">Total</th>
                <th className="th-c text-right">Available</th>
                <th className="th-c text-right">Sold</th>
                <th className="th-c text-right">Total cost</th>
                <th className="th-c">Status</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
              {orders.map((o) => {
                const inv = inventoryStatus(o);
                return (
                  <tr
                    key={o.id}
                    className="cursor-pointer hover:bg-slate-50 dark:hover:bg-slate-800/60"
                    onClick={(e) => {
                      if ((e.target as HTMLElement).closest("a")) return;
                      navigate(`/orders/${o.id}`, { state: { from: location.pathname } });
                    }}
                  >
                    <td className="td-c truncate" title={o.code}>
                      <Link
                        to={`/orders/${o.id}`}
                        state={{ from: location.pathname }}
                        className="font-medium text-slate-900 dark:text-slate-100 hover:text-brand-700 dark:hover:text-brand-400"
                      >
                        {o.code}
                      </Link>
                    </td>
                    <td className="td-c truncate" title={o.eventName}>
                      <Link
                        to={`/orders/${o.id}`}
                        state={{ from: location.pathname }}
                        className="hover:text-brand-700 dark:hover:text-brand-400"
                      >
                        {o.eventName}
                      </Link>
                    </td>
                    <td className="td-c truncate text-slate-500 dark:text-slate-400" title={o.supplierName ?? undefined}>
                      {o.supplierName ?? "-"}
                    </td>
                    <td className="td-c whitespace-nowrap">{formatDate(o.purchaseDate)}</td>
                    <td className="td-c text-right tabular-nums">{o.quantity}</td>
                    <td className="td-c text-right tabular-nums">{o.availableCount + o.listedCount}</td>
                    <td className="td-c text-right tabular-nums">{o.soldCount}</td>
                    <td className="td-c text-right tabular-nums">{formatMoney(o.totalCostCents, o.currency)}</td>
                    <td className="td-c">
                      <Badge tone={inv.key}>{inv.label}</Badge>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

export function TicketEditModal({
  open,
  ticket,
  onClose,
  onSaved,
}: {
  open: boolean;
  ticket: Ticket | null;
  onClose: () => void;
  onSaved: () => void;
}) {
  const toast = useToast();
  const [section, setSection] = useState("");
  const [rowLabel, setRowLabel] = useState("");
  const [seat, setSeat] = useState("");
  const [ticketType, setTicketType] = useState("");
  const [listingPrice, setListingPrice] = useState("");
  const [status, setStatus] = useState<TicketStatus>("available");
  const [notes, setNotes] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!ticket) return;
    setSection(ticket.section ?? "");
    setRowLabel(ticket.rowLabel ?? "");
    setSeat(ticket.seat ?? "");
    setTicketType(ticket.ticketType ?? "");
    setListingPrice(ticket.listingPriceCents != null ? (ticket.listingPriceCents / 100).toFixed(2) : "");
    setStatus(ticket.status);
    setNotes(ticket.notes ?? "");
    setError(null);
  }, [ticket]);

  if (!ticket) return null;
  const locked = ticket.status === "sold";

  const submit = async () => {
    setError(null);
    let listingCents: number | null = null;
    if (listingPrice.trim() !== "") {
      const s = listingPrice.trim().replace(",", ".");
      if (!/^\d+(\.\d{1,2})?$/.test(s)) {
        setError("Listing price is not a valid amount");
        return;
      }
      listingCents = Math.round(parseFloat(s) * 100);
    }
    const input: TicketUpdateInput = {
      section: section || null,
      rowLabel: rowLabel || null,
      seat: seat || null,
      ticketType: ticketType || null,
      listingPriceCents: listingCents,
      status: locked ? undefined : status,
      notes: notes || null,
    };
    setSaving(true);
    try {
      await api.updateTicket(ticket.id, input);
      toast.success("Ticket updated");
      onSaved();
    } catch (e) {
      setError(errMsg(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal open={open} onClose={onClose} title={`Edit ${ticket.code}`}>
      <div className="grid grid-cols-2 gap-4">
        <Field label="Section">
          <Input value={section} onChange={(e) => setSection(e.target.value)} />
        </Field>
        <Field label="Ticket type">
          <Input value={ticketType} onChange={(e) => setTicketType(e.target.value)} />
        </Field>
        <Field label="Row">
          <Input value={rowLabel} onChange={(e) => setRowLabel(e.target.value)} />
        </Field>
        <Field label="Seat">
          <Input value={seat} onChange={(e) => setSeat(e.target.value)} />
        </Field>
        <Field label={`Listing price (${ticket.currency})`}>
          <Input inputMode="decimal" placeholder="0.00" value={listingPrice} onChange={(e) => setListingPrice(e.target.value)} />
        </Field>
        <Field label="Status">
          {locked ? (
            <div>
              <div className="input flex items-center bg-slate-50 dark:bg-slate-800/60 text-slate-500 dark:text-slate-400">Sold</div>
              <p className="mt-1 text-xs text-slate-400 dark:text-slate-500">Refund or delete the sale on the Sales screen to make this available again.</p>
            </div>
          ) : (
            <Select value={status} onChange={(e) => setStatus(e.target.value as TicketStatus)}>
              <option value="available">Available</option>
              <option value="listed">Listed</option>
              <option value="cancelled">Cancelled</option>
            </Select>
          )}
        </Field>
        <div className="col-span-2">
          <Field label="Notes">
            <Textarea rows={2} value={notes} onChange={(e) => setNotes(e.target.value)} />
          </Field>
        </div>
      </div>
      <div className="mt-3 grid grid-cols-3 gap-3 rounded-lg bg-slate-50 dark:bg-slate-800/60 px-4 py-3 text-xs text-slate-500 dark:text-slate-400">
        <div>Purchase cost: {formatMoney(ticket.purchaseCostCents, ticket.currency)}</div>
        <div>Fees: {formatMoney(ticket.purchaseFeesCents, ticket.currency)}</div>
        <div>Other: {formatMoney(ticket.otherCostsCents, ticket.currency)}</div>
      </div>
      {error && <p className="mt-3 text-sm text-red-600 dark:text-red-400">{error}</p>}
      <ModalFooter>
        <Button variant="secondary" onClick={onClose} disabled={saving}>
          Cancel
        </Button>
        <Button variant="primary" onClick={submit} disabled={saving}>
          {saving ? "Saving..." : "Save changes"}
        </Button>
      </ModalFooter>
    </Modal>
  );
}

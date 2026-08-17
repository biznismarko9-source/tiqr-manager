import { useEffect, useMemo, useState } from "react";
import { Link, useSearchParams } from "react-router-dom";
import { api, errMsg } from "../lib/api";
import type { EventWithStats, Ticket, TicketStatus, TicketUpdateInput } from "../lib/types";
import { formatMoney } from "../lib/format";
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
import { IconChevronDown, IconChevronUp, IconSearch, IconTicket } from "../components/icons";
import { useToast } from "../lib/toast";

const SORT_OPTIONS = [
  { key: "id", label: "Newest" },
  { key: "code", label: "Code" },
  { key: "event", label: "Event" },
  { key: "status", label: "Status" },
  { key: "price", label: "Listing price" },
  { key: "cost", label: "Cost" },
];

export default function Tickets() {
  return <TicketsView title="Tickets" subtitle="Every ticket you have ever purchased, across all events." />;
}

/** Shared list view, reused (pre-filtered) by the Inventory page. */
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
  const [params] = useSearchParams();
  const [tickets, setTickets] = useState<Ticket[] | null>(null);
  const [events, setEvents] = useState<EventWithStats[]>([]);
  const [search, setSearch] = useState(params.get("code") ?? "");
  const [status, setStatus] = useState(lockedStatus ?? "");
  const [eventId, setEventId] = useState<number | "">("");
  const [sortBy, setSortBy] = useState("id");
  const [sortDir, setSortDir] = useState<"asc" | "desc">("desc");
  const [editTicket, setEditTicket] = useState<Ticket | null>(null);

  useEffect(() => {
    api.listEvents().then(setEvents).catch(() => {});
  }, []);

  const load = () => {
    api
      .listTickets({
        search: search || undefined,
        status: status || undefined,
        eventId: eventId || undefined,
        sortBy,
        sortDir,
      })
      .then(setTickets)
      .catch((e) => toast.error(errMsg(e)));
  };

  useEffect(() => {
    const t = setTimeout(load, 200);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [search, status, eventId, sortBy, sortDir]);

  const toggleSort = (key: string) => {
    if (sortBy === key) {
      setSortDir((d) => (d === "asc" ? "desc" : "asc"));
    } else {
      setSortBy(key);
      setSortDir("asc");
    }
  };

  const summary = useMemo(() => {
    if (!tickets) return null;
    const listingValue = tickets.reduce((sum, t) => sum + (t.listingPriceCents ?? 0), 0);
    return { count: tickets.length, listingValue };
  }, [tickets]);

  return (
    <div>
      <PageHeader title={title} subtitle={subtitle} />

      <div className="mb-4 flex flex-wrap items-end gap-3">
        <div className="w-56">
          <span className="label">Search</span>
          <div className="relative">
            <IconSearch className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-400" />
            <Input
              placeholder="Code, seat, event..."
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
        <div className="w-56">
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
        <div className="w-44">
          <span className="label">Sort by</span>
          <Select value={sortBy} onChange={(e) => setSortBy(e.target.value)}>
            {SORT_OPTIONS.map((o) => (
              <option key={o.key} value={o.key}>
                {o.label}
              </option>
            ))}
          </Select>
        </div>
        <Button variant="secondary" onClick={() => setSortDir((d) => (d === "asc" ? "desc" : "asc"))}>
          {sortDir === "asc" ? <IconChevronUp className="h-4 w-4" /> : <IconChevronDown className="h-4 w-4" />}
          {sortDir === "asc" ? "Asc" : "Desc"}
        </Button>
        {summary && (
          <p className="ml-auto text-xs text-slate-400">
            {summary.count} tickets &middot; listing value {formatMoney(summary.listingValue, "EUR")}
          </p>
        )}
      </div>

      {tickets === null ? (
        <LoadingBlock />
      ) : tickets.length === 0 ? (
        <EmptyState icon={<IconTicket className="h-8 w-8" />} title="No tickets match these filters" />
      ) : (
        <div className="overflow-x-auto rounded-xl border border-slate-200 bg-white shadow-sm">
          <table className="w-full min-w-[1000px] border-collapse">
            <thead className="border-b border-slate-200 bg-slate-50">
              <tr>
                <SortTh label="Code" k="code" sortBy={sortBy} sortDir={sortDir} onClick={toggleSort} />
                <SortTh label="Event" k="event" sortBy={sortBy} sortDir={sortDir} onClick={toggleSort} />
                <th className="th">Order</th>
                <th className="th">Seat</th>
                <SortTh label="Cost" k="cost" sortBy={sortBy} sortDir={sortDir} onClick={toggleSort} right />
                <SortTh label="Listing price" k="price" sortBy={sortBy} sortDir={sortDir} onClick={toggleSort} right />
                <SortTh label="Status" k="status" sortBy={sortBy} sortDir={sortDir} onClick={toggleSort} />
                <th className="th" />
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100">
              {tickets.map((t) => (
                <tr key={t.id} className="hover:bg-slate-50">
                  <td className="td font-medium text-slate-900">
                    {t.code}
                    {t.isDemo && (
                      <span className="ml-1.5">
                        <Badge tone="demo">demo</Badge>
                      </span>
                    )}
                  </td>
                  <td className="td">
                    <Link to={`/events/${t.eventId}`} className="hover:text-brand-700">
                      {t.eventName}
                    </Link>
                  </td>
                  <td className="td">
                    <Link to={`/orders/${t.orderId}`} className="text-slate-500 hover:text-brand-700">
                      {t.orderCode}
                    </Link>
                  </td>
                  <td className="td text-slate-500">
                    {[t.section, t.rowLabel, t.seat].filter(Boolean).join(" / ") || "-"}
                  </td>
                  <td className="td text-right tabular-nums">{formatMoney(t.totalCostCents, t.currency)}</td>
                  <td className="td text-right tabular-nums">
                    {t.listingPriceCents != null ? formatMoney(t.listingPriceCents, t.currency) : "-"}
                  </td>
                  <td className="td">
                    <Badge tone={t.status}>{t.status}</Badge>
                  </td>
                  <td className="td text-right">
                    <button
                      className="text-xs font-medium text-brand-600 hover:underline"
                      onClick={() => setEditTicket(t)}
                    >
                      Edit
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      <TicketEditModal
        open={!!editTicket}
        ticket={editTicket}
        onClose={() => setEditTicket(null)}
        onSaved={() => {
          setEditTicket(null);
          load();
        }}
      />
    </div>
  );
}

function SortTh({
  label,
  k,
  sortBy,
  sortDir,
  onClick,
  right,
}: {
  label: string;
  k: string;
  sortBy: string;
  sortDir: "asc" | "desc";
  onClick: (k: string) => void;
  right?: boolean;
}) {
  const active = sortBy === k;
  return (
    <th className={`th cursor-pointer select-none ${right ? "text-right" : ""}`} onClick={() => onClick(k)}>
      <span className={`inline-flex items-center gap-1 ${right ? "flex-row-reverse" : ""}`}>
        {label}
        {active && (sortDir === "asc" ? <IconChevronUp className="h-3 w-3" /> : <IconChevronDown className="h-3 w-3" />)}
      </span>
    </th>
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
              <div className="input flex items-center bg-slate-50 text-slate-500">Sold</div>
              <p className="mt-1 text-xs text-slate-400">Delete the sale on the Sales screen to make this available again.</p>
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
      <div className="mt-3 grid grid-cols-3 gap-3 rounded-lg bg-slate-50 px-4 py-3 text-xs text-slate-500">
        <div>Purchase cost: {formatMoney(ticket.purchaseCostCents, ticket.currency)}</div>
        <div>Fees: {formatMoney(ticket.purchaseFeesCents, ticket.currency)}</div>
        <div>Other: {formatMoney(ticket.otherCostsCents, ticket.currency)}</div>
      </div>
      {error && <p className="mt-3 text-sm text-red-600">{error}</p>}
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

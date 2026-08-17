import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { api, errMsg } from "../lib/api";
import type { EventInput, EventStatus, EventWithStats } from "../lib/types";
import { formatDate, formatMoney, formatPercent } from "../lib/format";
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
import { IconCalendarDays, IconPlus, IconSearch } from "../components/icons";
import { useToast } from "../lib/toast";

const EMPTY_INPUT: EventInput = {
  name: "",
  artistTeam: "",
  venue: "",
  city: "",
  country: "",
  eventDate: "",
  category: "",
  status: "upcoming",
  notes: "",
};

export default function Events() {
  const toast = useToast();
  const [events, setEvents] = useState<EventWithStats[] | null>(null);
  const [search, setSearch] = useState("");
  const [modalOpen, setModalOpen] = useState(false);
  const [editing, setEditing] = useState<EventWithStats | null>(null);

  const load = (q?: string) => {
    api
      .listEvents(q || undefined)
      .then(setEvents)
      .catch((e) => toast.error(errMsg(e)));
  };

  useEffect(() => {
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const t = setTimeout(() => load(search), 250);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [search]);

  return (
    <div>
      <PageHeader
        title="Events"
        subtitle="Every event you buy or sell tickets for."
        actions={
          <Button
            variant="primary"
            onClick={() => {
              setEditing(null);
              setModalOpen(true);
            }}
          >
            <IconPlus className="h-4 w-4" /> New Event
          </Button>
        }
      />

      <div className="mb-4 max-w-xs">
        <div className="relative">
          <IconSearch className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-400 dark:text-slate-500" />
          <Input
            placeholder="Search events..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="pl-9"
          />
        </div>
      </div>

      {events === null ? (
        <LoadingBlock />
      ) : events.length === 0 ? (
        <EmptyState
          icon={<IconCalendarDays className="h-8 w-8" />}
          title="No events found"
          description="Create your first event to start tracking orders, tickets and sales for it."
          action={
            <Button variant="primary" onClick={() => setModalOpen(true)}>
              <IconPlus className="h-4 w-4" /> New Event
            </Button>
          }
        />
      ) : (
        <div className="overflow-x-auto rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-sm">
          <table className="w-full min-w-[900px] border-collapse">
            <thead className="border-b border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-800/60">
              <tr>
                <th className="th">Event</th>
                <th className="th">Date</th>
                <th className="th">Status</th>
                <th className="th text-right">Tickets</th>
                <th className="th text-right">Available</th>
                <th className="th text-right">Cost</th>
                <th className="th text-right">Revenue</th>
                <th className="th text-right">Profit</th>
                <th className="th text-right">Margin</th>
                <th className="th text-right">ROI</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
              {events.map((ev) => (
                <tr
                  key={ev.id}
                  className="cursor-pointer hover:bg-slate-50 dark:hover:bg-slate-800/60"
                  onClick={() => (window.location.hash = `#/events/${ev.id}`)}
                >
                  <td className="td">
                    <Link to={`/events/${ev.id}`} className="font-medium text-slate-900 dark:text-slate-100 hover:text-brand-700 dark:hover:text-brand-400">
                      {ev.name}
                    </Link>
                    {ev.isDemo && (
                      <span className="ml-1.5">
                        <Badge tone="demo">demo</Badge>
                      </span>
                    )}
                    <p className="text-xs text-slate-400 dark:text-slate-500">
                      {[ev.venue, ev.city].filter(Boolean).join(", ")}
                    </p>
                  </td>
                  <td className="td whitespace-nowrap">{formatDate(ev.eventDate)}</td>
                  <td className="td">
                    <Badge tone={ev.status}>{ev.status}</Badge>
                  </td>
                  <td className="td text-right tabular-nums">{ev.stats.purchasedTickets}</td>
                  <td className="td text-right tabular-nums">{ev.stats.availableTickets}</td>
                  <td className="td text-right tabular-nums">{formatMoney(ev.stats.totalCostCents, "EUR")}</td>
                  <td className="td text-right tabular-nums">{formatMoney(ev.stats.revenueCents, "EUR")}</td>
                  <td
                    className={`td text-right tabular-nums font-medium ${ev.stats.profitCents > 0 ? "text-emerald-600 dark:text-emerald-400" : ev.stats.profitCents < 0 ? "text-red-600 dark:text-red-400" : ""}`}
                  >
                    {formatMoney(ev.stats.profitCents, "EUR")}
                  </td>
                  <td className="td text-right tabular-nums">{formatPercent(ev.stats.margin)}</td>
                  <td className="td text-right tabular-nums">{formatPercent(ev.stats.roi)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      <EventFormModal
        open={modalOpen}
        initial={editing}
        onClose={() => setModalOpen(false)}
        onSaved={() => {
          setModalOpen(false);
          load(search);
        }}
      />
    </div>
  );
}

export function EventFormModal({
  open,
  initial,
  onClose,
  onSaved,
}: {
  open: boolean;
  initial: EventWithStats | null;
  onClose: () => void;
  onSaved: (id: number) => void;
}) {
  const toast = useToast();
  const [form, setForm] = useState<EventInput>(EMPTY_INPUT);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (open) {
      setForm(
        initial
          ? {
              name: initial.name,
              artistTeam: initial.artistTeam ?? "",
              venue: initial.venue ?? "",
              city: initial.city ?? "",
              country: initial.country ?? "",
              eventDate: initial.eventDate ?? "",
              category: initial.category ?? "",
              status: initial.status,
              notes: initial.notes ?? "",
            }
          : EMPTY_INPUT,
      );
      setError(null);
    }
  }, [open, initial]);

  const submit = async () => {
    if (!form.name.trim()) {
      setError("Event name is required");
      return;
    }
    setSaving(true);
    setError(null);
    try {
      if (initial) {
        await api.updateEvent(initial.id, form);
        toast.success("Event updated");
        onSaved(initial.id);
      } else {
        const created = await api.createEvent(form);
        toast.success("Event created");
        onSaved(created.id);
      }
    } catch (e) {
      setError(errMsg(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal open={open} onClose={onClose} title={initial ? "Edit event" : "New event"}>
      <div className="grid grid-cols-2 gap-4">
        <div className="col-span-2">
          <Field label="Event name" required>
            <Input value={form.name} onChange={(e) => setForm({ ...form, name: e.target.value })} autoFocus />
          </Field>
        </div>
        <Field label="Artist / Team">
          <Input
            value={form.artistTeam ?? ""}
            onChange={(e) => setForm({ ...form, artistTeam: e.target.value })}
          />
        </Field>
        <Field label="Category">
          <Input
            placeholder="Concert, Sports, ..."
            value={form.category ?? ""}
            onChange={(e) => setForm({ ...form, category: e.target.value })}
          />
        </Field>
        <Field label="Venue">
          <Input value={form.venue ?? ""} onChange={(e) => setForm({ ...form, venue: e.target.value })} />
        </Field>
        <Field label="Event date">
          <Input
            type="date"
            value={form.eventDate ?? ""}
            onChange={(e) => setForm({ ...form, eventDate: e.target.value })}
          />
        </Field>
        <Field label="City">
          <Input value={form.city ?? ""} onChange={(e) => setForm({ ...form, city: e.target.value })} />
        </Field>
        <Field label="Country">
          <Input value={form.country ?? ""} onChange={(e) => setForm({ ...form, country: e.target.value })} />
        </Field>
        <div className="col-span-2">
          <Field label="Status">
            <Select
              value={form.status ?? "upcoming"}
              onChange={(e) => setForm({ ...form, status: e.target.value as EventStatus })}
            >
              <option value="upcoming">Upcoming</option>
              <option value="completed">Completed</option>
              <option value="cancelled">Cancelled</option>
            </Select>
          </Field>
        </div>
        <div className="col-span-2">
          <Field label="Notes">
            <Textarea
              rows={3}
              value={form.notes ?? ""}
              onChange={(e) => setForm({ ...form, notes: e.target.value })}
            />
          </Field>
        </div>
      </div>
      {error && <p className="mt-3 text-sm text-red-600 dark:text-red-400">{error}</p>}
      <ModalFooter>
        <Button variant="secondary" onClick={onClose} disabled={saving}>
          Cancel
        </Button>
        <Button variant="primary" onClick={submit} disabled={saving}>
          {saving ? "Saving..." : initial ? "Save changes" : "Create event"}
        </Button>
      </ModalFooter>
    </Modal>
  );
}

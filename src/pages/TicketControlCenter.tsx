import { useEffect, useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";
import { save } from "@tauri-apps/plugin-dialog";
import { api, errMsg } from "../lib/api";
import type { ControlCenterFilters, ControlCenterTicket, EventWithStats, Marketplace } from "../lib/types";
import { formatDateNumeric, formatMoney, formatSeatLocation, todayIso } from "../lib/format";
import { Badge, Button, CHECKBOX_CLASS, EmptyState, Field, Input, LoadingBlock, Modal, ModalFooter, Select } from "../components/ui";
import { IconSearch } from "../components/icons";
import { BulkTicketEditBar } from "../components/BulkTicketEditBar";
import { DELIVERY_STATUS_OPTIONS, RESALE_STATUS_OPTIONS } from "./Tickets";
import { useToast } from "../lib/toast";
import { useNarrowTables } from "../lib/useNarrowTables";

// 2.4.3: "Ticket Control Center" - marko's own request for one dense work
// screen to manage/inspect tickets across EVERY event at once, built
// entirely on top of the EXISTING tickets/listings/sales data - not a new
// parallel ticket system. See commands::ticket_control_center's own module
// doc comment (Rust) for the query this page reads from.
//
// Every write this page can trigger reuses a command that already existed
// before this feature:
// - Section/Row/Tier/Seat/Listing price -> `BulkTicketEditBar` -> the
//   existing `bulkUpdateTickets` (now with a "Tier / Level" option, added
//   this same release - see that component's own doc comment).
// - "Change listing status" -> the existing `bulkUpdateTicketListingsStatus`
//   (2.2.5), the exact same command EventDetail.tsx's own Listings tab bulk
//   bar already calls - this page just offers it from a second place.
// - "Export selected" -> the existing `exportTicketsCsvSelected` (1.9.1),
//   the same command Settings -> Data's ticket export already calls -
//   selection already happened in this page's own table, so there's no need
//   for `ExportPickerModal`'s own search-driven picker on top of it, just
//   its same save-dialog-then-export shape.
// Ticket status itself is never editable here (see BulkTicketEditBar's own
// doc comment for why bulk status changes are structurally unsafe) - only
// the Sales screen can move a ticket into or out of "sold". Refund/resell,
// batch_id, and every money/cents column are untouched by anything on this
// page - it only ever reads them.
//
// No Sort control - marko's own spec lists ZOBRAZENIE/FILTERS/QUICK
// FILTERS/SEARCH/BULK, not a sort, so this page shows the backend's own
// fixed order (soonest event first) rather than adding a control nobody
// asked for.
//
// A ticket currently listed on more than one marketplace at once appears as
// more than one row - one per (ticket, listing) pair, exactly like
// EventDetail.tsx's own Listings tab already shows for a single event. Each
// row still needs a stable, unique selection key even though `t.id` repeats
// across such rows - `rowKey` below uses the listing's own id when a row has
// one (globally unique across `ticket_listings`) and a negated ticket id
// otherwise (never collides with a real, positive listing id).
function rowKey(t: ControlCenterTicket): number {
  return t.listingRowId ?? -t.id;
}

type QuickFilterKey = "all" | "unsold" | "unlisted" | "listed" | "sold" | "pendingPayment" | "pendingDelivery" | "refunded";

const QUICK_FILTERS: { key: QuickFilterKey; label: string }[] = [
  { key: "all", label: "All" },
  { key: "unsold", label: "Unsold" },
  { key: "unlisted", label: "Unlisted" },
  { key: "listed", label: "Listed" },
  { key: "sold", label: "Sold" },
  { key: "pendingPayment", label: "Pending payment" },
  { key: "pendingDelivery", label: "Pending delivery" },
  { key: "refunded", label: "Refunded" },
];

// Session-only, remembered for this app session only (not persisted to
// disk) - same convention Tickets.tsx's own `lastTicketsFilters` already
// established, simplified to a bare module variable (no per-pathname Map
// needed - this page has exactly one route, unlike TicketsView which backs
// both /tickets and /inventory).
interface ControlCenterFilterState {
  search: string;
  eventId: number | "";
  dateFrom: string;
  dateTo: string;
  tier: string;
  section: string;
  rowLabel: string;
  ticketStatus: string;
  listingStatus: string;
  saleStatus: string;
  paymentStatus: string;
  deliveryStatus: string;
  marketplaceId: number | "";
  refundedOnly: boolean;
}
let lastControlCenterFilters: ControlCenterFilterState | null = null;

export default function TicketControlCenter() {
  const toast = useToast();
  const navigate = useNavigate();
  const isNarrow = useNarrowTables();
  const cached = lastControlCenterFilters;

  const [tickets, setTickets] = useState<ControlCenterTicket[] | null>(null);
  const [events, setEvents] = useState<EventWithStats[]>([]);
  const [marketplaces, setMarketplaces] = useState<Marketplace[]>([]);
  const [selected, setSelected] = useState<Set<number>>(new Set());

  const [search, setSearch] = useState(cached?.search ?? "");
  const [eventId, setEventId] = useState<number | "">(cached?.eventId ?? "");
  const [dateFrom, setDateFrom] = useState(cached?.dateFrom ?? "");
  const [dateTo, setDateTo] = useState(cached?.dateTo ?? "");
  const [tier, setTier] = useState(cached?.tier ?? "");
  const [section, setSection] = useState(cached?.section ?? "");
  const [rowLabel, setRowLabel] = useState(cached?.rowLabel ?? "");
  const [ticketStatus, setTicketStatus] = useState(cached?.ticketStatus ?? "");
  const [listingStatus, setListingStatus] = useState(cached?.listingStatus ?? "");
  const [saleStatus, setSaleStatus] = useState(cached?.saleStatus ?? "");
  const [paymentStatus, setPaymentStatus] = useState(cached?.paymentStatus ?? "");
  const [deliveryStatus, setDeliveryStatus] = useState(cached?.deliveryStatus ?? "");
  const [marketplaceId, setMarketplaceId] = useState<number | "">(cached?.marketplaceId ?? "");
  const [refundedOnly, setRefundedOnly] = useState(cached?.refundedOnly ?? false);

  const [listingStatusModalOpen, setListingStatusModalOpen] = useState(false);
  const [bulkListingStatus, setBulkListingStatus] = useState<"active" | "sold" | "removed">("active");
  const [listingStatusBusy, setListingStatusBusy] = useState(false);
  const [listingStatusError, setListingStatusError] = useState<string | null>(null);
  const [exporting, setExporting] = useState(false);

  useEffect(() => {
    api.listEvents().then(setEvents).catch(() => {});
    api.listMarketplaces().then(setMarketplaces).catch(() => {});
  }, []);

  // 8: remembers this page's own filters for the rest of this app session -
  // see `lastControlCenterFilters` above.
  useEffect(() => {
    lastControlCenterFilters = {
      search, eventId, dateFrom, dateTo, tier, section, rowLabel,
      ticketStatus, listingStatus, saleStatus, paymentStatus, deliveryStatus,
      marketplaceId, refundedOnly,
    };
  }, [search, eventId, dateFrom, dateTo, tier, section, rowLabel, ticketStatus, listingStatus, saleStatus, paymentStatus, deliveryStatus, marketplaceId, refundedOnly]);

  const load = () => {
    const filters: ControlCenterFilters = {
      search: search || undefined,
      eventId: eventId || undefined,
      dateFrom: dateFrom || undefined,
      dateTo: dateTo || undefined,
      tier: tier || undefined,
      section: section || undefined,
      rowLabel: rowLabel || undefined,
      ticketStatus: ticketStatus || undefined,
      listingStatus: listingStatus || undefined,
      saleStatus: saleStatus || undefined,
      paymentStatus: paymentStatus || undefined,
      deliveryStatus: deliveryStatus || undefined,
      marketplaceId: marketplaceId || undefined,
      refundedOnly: refundedOnly || undefined,
    };
    api.listControlCenterTickets(filters).then(setTickets).catch((e) => toast.error(errMsg(e)));
  };

  useEffect(() => {
    const t = setTimeout(load, 200);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [search, eventId, dateFrom, dateTo, tier, section, rowLabel, ticketStatus, listingStatus, saleStatus, paymentStatus, deliveryStatus, marketplaceId, refundedOnly]);

  // Keeps the selection consistent with whatever `tickets` actually holds
  // after every load - same "a row that's gone can never linger in the
  // selection" convention as EventDetail.tsx's own ListingsTab.
  useEffect(() => {
    if (tickets === null) return;
    setSelected((prev) => {
      const validKeys = new Set(tickets.map(rowKey));
      const next = new Set(Array.from(prev).filter((k) => validKeys.has(k)));
      return next.size === prev.size ? prev : next;
    });
  }, [tickets]);

  // Quick Filters are presets over the SAME filter fields the manual
  // dropdowns below use - not a second, parallel filter system. Clicking one
  // resets every status-ish dimension to a clean slate first, then sets only
  // the one(s) that preset implies, so the result is always predictable
  // regardless of whatever was set before (same spirit as FulfillmentCenter's
  // own always-exactly-one-active category tiles).
  const applyQuickFilter = (key: QuickFilterKey) => {
    setTicketStatus("");
    setListingStatus("");
    setSaleStatus("");
    setPaymentStatus("");
    setDeliveryStatus("");
    setRefundedOnly(false);
    if (key === "unsold") setTicketStatus("available,listed"); // same "real sellable stock" definition Inventory.tsx's own lockedStatus already uses
    else if (key === "unlisted") setSaleStatus("Unlisted");
    else if (key === "listed") setSaleStatus("Listed");
    else if (key === "sold") setTicketStatus("sold");
    else if (key === "pendingPayment") setPaymentStatus("pending");
    else if (key === "pendingDelivery") setDeliveryStatus("Not delivered");
    else if (key === "refunded") setRefundedOnly(true);
  };

  // Derived, never stored separately - so the highlighted pill can never go
  // stale relative to the actual filter values (e.g. after someone changes
  // the Ticket status dropdown by hand instead of clicking a pill). `null`
  // when the current combination doesn't match any single preset exactly,
  // which is simply honest rather than highlighting a wrong guess.
  const activeQuickFilter = useMemo<QuickFilterKey | null>(() => {
    const blank = !ticketStatus && !listingStatus && !saleStatus && !paymentStatus && !deliveryStatus && !refundedOnly;
    if (blank) return "all";
    const only = (field: string, value: string) =>
      field === value &&
      [ticketStatus, listingStatus, saleStatus, paymentStatus, deliveryStatus].filter((f) => f !== field).every((f) => !f) &&
      !refundedOnly;
    if (only(ticketStatus, "available,listed") && !listingStatus) return "unsold";
    if (only(saleStatus, "Unlisted")) return "unlisted";
    if (only(saleStatus, "Listed")) return "listed";
    if (only(ticketStatus, "sold")) return "sold";
    if (only(paymentStatus, "pending")) return "pendingPayment";
    if (only(deliveryStatus, "Not delivered")) return "pendingDelivery";
    if (refundedOnly && !ticketStatus && !listingStatus && !saleStatus && !paymentStatus && !deliveryStatus) return "refunded";
    return null;
  }, [ticketStatus, listingStatus, saleStatus, paymentStatus, deliveryStatus, refundedOnly]);

  const hasActiveFilters = Boolean(
    search || eventId || dateFrom || dateTo || tier || section || rowLabel ||
    ticketStatus || listingStatus || saleStatus || paymentStatus || deliveryStatus || marketplaceId || refundedOnly,
  );
  const clearFilters = () => {
    setSearch("");
    setEventId("");
    setDateFrom("");
    setDateTo("");
    setTier("");
    setSection("");
    setRowLabel("");
    setMarketplaceId("");
    applyQuickFilter("all");
  };

  const allSelected = (tickets ?? []).length > 0 && (tickets ?? []).every((t) => selected.has(rowKey(t)));
  const toggleSelectAll = () => setSelected(allSelected ? new Set() : new Set((tickets ?? []).map(rowKey)));
  const toggleOne = (key: number) =>
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });

  // One entry per distinct TICKET among selected rows (a ticket selected via
  // 2 of its own listing rows must still only count/act once) - fed to
  // BulkTicketEditBar and to "Export selected". `ticketById` also backs the
  // shared-currency check below (any one of a ticket's own rows carries the
  // same ticket-level fields, so the first one seen is enough).
  const ticketById = useMemo(() => {
    const m = new Map<number, ControlCenterTicket>();
    (tickets ?? []).forEach((t) => {
      if (!m.has(t.id)) m.set(t.id, t);
    });
    return m;
  }, [tickets]);
  const selectedTicketIds = useMemo(() => {
    const ids = new Set<number>();
    (tickets ?? []).forEach((t) => {
      if (selected.has(rowKey(t))) ids.add(t.id);
    });
    return Array.from(ids);
  }, [tickets, selected]);
  // Only the selected rows that actually represent a real marketplace
  // listing - fed to "Change listing status" (`bulkUpdateTicketListingsStatus`
  // operates on LISTING ids, never ticket ids).
  const selectedListingRowIds = useMemo(() => {
    const ids: number[] = [];
    (tickets ?? []).forEach((t) => {
      if (selected.has(rowKey(t)) && t.listingRowId != null) ids.push(t.listingRowId);
    });
    return ids;
  }, [tickets, selected]);
  const selectedCurrency = useMemo(() => {
    const currencies = new Set(selectedTicketIds.map((id) => ticketById.get(id)?.currency).filter((c): c is string => Boolean(c)));
    return currencies.size === 1 ? Array.from(currencies)[0] : null;
  }, [selectedTicketIds, ticketById]);

  const submitListingStatus = async () => {
    setListingStatusBusy(true);
    setListingStatusError(null);
    try {
      const updated = await api.bulkUpdateTicketListingsStatus({ ids: selectedListingRowIds, status: bulkListingStatus });
      toast.success(`${updated.length} listing${updated.length === 1 ? "" : "s"} marked ${bulkListingStatus}`);
      setListingStatusModalOpen(false);
      load();
    } catch (e) {
      setListingStatusError(errMsg(e));
    } finally {
      setListingStatusBusy(false);
    }
  };

  const exportSelected = async () => {
    const path = await save({
      defaultPath: `tiqr-control-center-selected-${todayIso()}.csv`,
      filters: [{ name: "CSV", extensions: ["csv"] }],
    });
    if (!path || Array.isArray(path)) return;
    setExporting(true);
    try {
      const count = await api.exportTicketsCsvSelected(path, selectedTicketIds);
      toast.success(`Exported ${count} row${count === 1 ? "" : "s"} to ${path}`);
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setExporting(false);
    }
  };

  const openDetail = (t: ControlCenterTicket) => {
    // "podľa dostupnosti" - Sale Detail when this ticket has an active sale,
    // Order Detail otherwise (there is no standalone Ticket Detail page in
    // this app - Order Detail is where an individual ticket's own edit
    // control already lives).
    if (t.saleId != null) navigate(`/sales/${t.saleId}`);
    else navigate(`/orders/${t.orderId}`);
  };

  const thCls = isNarrow ? "th-c-narrow" : "th-c";
  const tdCls = isNarrow ? "td-c-narrow" : "td-c";
  // 2.4.4 fix: dark mode was `dark:bg-slate-800/60` - a translucent 60%
  // opacity copied from this app's normal (non-sticky) table header
  // convention (e.g. FulfillmentCenter's <thead>), where it's harmless
  // because nothing ever scrolls underneath a non-sticky header. Here the
  // header IS sticky above actively scrolling rows, so that translucency let
  // scrolled-past row text show straight through it - marko's screenshot
  // ("zavadza" - misleading). A fully opaque dark background is required for
  // a sticky header to actually mask what's scrolled beneath it.
  const stickyTh = "sticky top-0 z-10 bg-slate-50 dark:bg-slate-800";

  return (
    <div>
      {/* 2.4.4: own PageHeader removed - this page is no longer a standalone
          top-level route, it's mounted inside Finance's new "Ticket Center"
          tab (see finance/TicketCenter.tsx) right below that tab's own
          "Control Center" subtab pill, which already labels it. Matches
          Finance's own existing 4 tabs (Overview/Transactions/Accounts/
          Reports), none of which repeat their own title as a second header
          either - one less redundant header, one less row eating into
          marko's "no unnecessary page scroll" table space below. */}

      {/* Sticky filters/quick filters/search - stays visible while the table
          below scrolls (see the table wrapper's own comment for the other
          half of "no unnecessary page scroll"). */}
      <div className="sticky top-0 z-20 -mt-1 bg-slate-50 pb-3 pt-1 dark:bg-slate-950">
        <div className="mb-3 flex flex-wrap items-end gap-3">
          <div className="w-56">
            <span className="label">Search</span>
            <div className="relative">
              <IconSearch className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-400 dark:text-slate-500" />
              <Input
                placeholder="Ticket, order, event, section, row, marketplace, listing..."
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                className="pl-9"
              />
            </div>
          </div>
          <div className="w-44">
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
          <div className="w-32">
            <span className="label">From</span>
            <Input type="date" value={dateFrom} onChange={(e) => setDateFrom(e.target.value)} />
          </div>
          <div className="w-32">
            <span className="label">To</span>
            <Input type="date" value={dateTo} onChange={(e) => setDateTo(e.target.value)} />
          </div>
          <div className="w-28">
            <span className="label">Tier / Level</span>
            <Input value={tier} onChange={(e) => setTier(e.target.value)} placeholder="VIP..." />
          </div>
          <div className="w-24">
            <span className="label">Section</span>
            <Input value={section} onChange={(e) => setSection(e.target.value)} />
          </div>
          <div className="w-20">
            <span className="label">Row</span>
            <Input value={rowLabel} onChange={(e) => setRowLabel(e.target.value)} />
          </div>
          <div className="w-36">
            <span className="label">Ticket status</span>
            <Select value={ticketStatus} onChange={(e) => setTicketStatus(e.target.value)}>
              <option value="">All statuses</option>
              <option value="available">Available</option>
              <option value="listed">Listed</option>
              <option value="sold">Sold</option>
              <option value="cancelled">Cancelled</option>
            </Select>
          </div>
          <div className="w-32">
            <span className="label">Listing status</span>
            <Select value={listingStatus} onChange={(e) => setListingStatus(e.target.value)}>
              <option value="">All</option>
              <option value="active">Active</option>
              <option value="sold">Sold</option>
              <option value="removed">Removed</option>
            </Select>
          </div>
          <div className="w-32">
            <span className="label">Sale status</span>
            <Select value={saleStatus} onChange={(e) => setSaleStatus(e.target.value)}>
              <option value="">All</option>
              {RESALE_STATUS_OPTIONS.map((s) => (
                <option key={s} value={s}>
                  {s}
                </option>
              ))}
            </Select>
          </div>
          <div className="w-32">
            <span className="label">Payment status</span>
            {/* "Refunded" is deliberately not an option here - the active-
                sale join this filter runs against can never carry a refunded
                sale (same guard `tickets::BASE_SQL` uses), so it would
                always return zero rows. Use the "Refunded" Quick Filter
                below instead - it reads the ticket's full sale history, not
                just its current active sale. */}
            <Select value={paymentStatus} onChange={(e) => setPaymentStatus(e.target.value)}>
              <option value="">All</option>
              <option value="pending">Pending</option>
              <option value="paid">Paid</option>
            </Select>
          </div>
          <div className="w-36">
            <span className="label">Delivery status</span>
            <Select value={deliveryStatus} onChange={(e) => setDeliveryStatus(e.target.value)}>
              <option value="">All</option>
              {DELIVERY_STATUS_OPTIONS.map((s) => (
                <option key={s} value={s}>
                  {s}
                </option>
              ))}
            </Select>
          </div>
          <div className="w-40">
            <span className="label">Marketplace</span>
            <Select value={marketplaceId} onChange={(e) => setMarketplaceId(e.target.value ? Number(e.target.value) : "")}>
              <option value="">All marketplaces</option>
              {marketplaces.map((m) => (
                <option key={m.id} value={m.id}>
                  {m.name}
                </option>
              ))}
            </Select>
          </div>
          {hasActiveFilters && (
            <button
              type="button"
              className="mb-2 text-xs font-medium text-brand-700 hover:underline dark:text-brand-400"
              onClick={clearFilters}
            >
              Clear filters
            </button>
          )}
        </div>

        <div className="flex flex-wrap items-center gap-1">
          {QUICK_FILTERS.map((qf) => (
            <button
              key={qf.key}
              type="button"
              onClick={() => applyQuickFilter(qf.key)}
              className={`rounded-full px-3 py-1 text-xs font-medium transition-colors ${
                activeQuickFilter === qf.key
                  ? "bg-brand-600 text-white"
                  : "bg-white text-slate-600 ring-1 ring-inset ring-slate-200 hover:bg-slate-50 dark:bg-slate-900 dark:text-slate-400 dark:ring-slate-700 dark:hover:bg-slate-800"
              }`}
            >
              {qf.label}
            </button>
          ))}
        </div>
      </div>

      <BulkTicketEditBar selectedIds={selectedTicketIds} currency={selectedCurrency} onClear={() => setSelected(new Set())} onApplied={load} />
      {selectedTicketIds.length > 0 && (
        <div className="-mt-2 mb-4 flex flex-wrap items-center gap-2">
          <Button
            variant="secondary"
            disabled={selectedListingRowIds.length === 0}
            onClick={() => {
              setBulkListingStatus("active");
              setListingStatusError(null);
              setListingStatusModalOpen(true);
            }}
          >
            Change listing status{selectedListingRowIds.length > 0 ? ` (${selectedListingRowIds.length})` : ""}
          </Button>
          <Button variant="secondary" disabled={exporting} onClick={exportSelected}>
            {exporting ? "Exporting..." : `Export selected (${selectedTicketIds.length})`}
          </Button>
          {selectedListingRowIds.length === 0 && (
            <span className="text-xs text-slate-400 dark:text-slate-500">
              None of the selected rows have a marketplace listing to change status on.
            </span>
          )}
        </div>
      )}

      {tickets && tickets.length >= 5000 && (
        <div className="mb-4 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-800 dark:border-amber-500/30 dark:bg-amber-500/10 dark:text-amber-400">
          Showing the most relevant 5,000 rows that match your filters. Narrow the search, event, or status filters to
          see the rest.
        </div>
      )}

      {tickets === null ? (
        <LoadingBlock />
      ) : tickets.length === 0 ? (
        <EmptyState
          title={hasActiveFilters ? "No tickets match these filters" : "No tickets yet"}
          description={hasActiveFilters ? "Try a different search, quick filter, or filter combination." : undefined}
          action={
            hasActiveFilters ? (
              <Button variant="secondary" onClick={clearFilters}>
                Clear filters
              </Button>
            ) : undefined
          }
        />
      ) : (
        // The table owns its own scroll (max-h + overflow-y-auto) with a
        // sticky <thead> inside it, rather than letting the whole page grow
        // past the window - marko's own "žiadny zbytočný page scroll
        // (tabuľka môže mať vlastný scroll)". `sticky` is applied per-<th>
        // rather than on <thead> itself - the more broadly-reliable pattern
        // across engines when a table also uses border-collapse. 68vh is a
        // plain, robust constant (not measured against exact chrome pixel
        // heights like useNarrowTables' own breakpoint was) - easy to tune
        // in one place if marko wants the table taller/shorter.
        <div className="overflow-x-auto overflow-y-auto rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-sm max-h-[68vh]">
          <table className="w-full table-fixed border-collapse">
            {isNarrow ? (
              <colgroup>
                <col className="w-[4%]" />
                <col className="w-[20%]" />
                <col className="w-[18%]" />
                <col className="w-[11%]" />
                <col className="w-[13%]" />
                <col className="w-[9%]" />
                <col className="w-[9%]" />
                <col className="w-[9%]" />
                <col className="w-[7%]" />
              </colgroup>
            ) : (
              <colgroup>
                <col className="w-[3%]" />
                <col className="w-[16%]" />
                <col className="w-[8%]" />
                <col className="w-[13%]" />
                <col className="w-[7%]" />
                <col className="w-[8%]" />
                <col className="w-[8%]" />
                <col className="w-[10%]" />
                <col className="w-[7%]" />
                <col className="w-[7%]" />
                <col className="w-[7%]" />
                <col className="w-[6%]" />
              </colgroup>
            )}
            <thead>
              <tr>
                <th className={`${thCls} ${stickyTh}`}>
                  <input type="checkbox" className={CHECKBOX_CLASS} checked={allSelected} onChange={toggleSelectAll} aria-label="Select all" />
                </th>
                <th className={`${thCls} ${stickyTh}`}>Event</th>
                {!isNarrow && <th className={`${thCls} ${stickyTh}`}>Order</th>}
                {/* 2.4.4: renamed from "Ticket / Seats" - marko's own request
                    to show only seats here (the cell below no longer shows
                    the ticket code at all, just its seat location - see that
                    cell's own comment). */}
                <th className={`${thCls} ${stickyTh}`}>Seats</th>
                {!isNarrow && <th className={`${thCls} ${stickyTh}`}>Tier</th>}
                {!isNarrow && <th className={`${thCls} ${stickyTh} text-right`}>Purchase price</th>}
                <th className={`${thCls} ${stickyTh} text-right`}>Listing price</th>
                <th className={`${thCls} ${stickyTh}`}>Listing status</th>
                <th className={`${thCls} ${stickyTh}`}>Sale status</th>
                <th className={`${thCls} ${stickyTh}`}>Payment</th>
                <th className={`${thCls} ${stickyTh}`}>Delivery</th>
                <th className={`${thCls} ${stickyTh}`}>Overall</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
              {tickets.map((t) => {
                const key = rowKey(t);
                const seats = formatSeatLocation(t.section, t.rowLabel, t.seat);
                return (
                  <tr key={key} onClick={() => openDetail(t)} className="cursor-pointer hover:bg-slate-50 dark:hover:bg-slate-800/60">
                    <td className={tdCls} onClick={(e) => e.stopPropagation()}>
                      <input
                        type="checkbox"
                        className={CHECKBOX_CLASS}
                        checked={selected.has(key)}
                        onChange={() => toggleOne(key)}
                        aria-label={`Select ${t.code}`}
                      />
                    </td>
                    <td className={`${tdCls} truncate`} title={t.eventName}>
                      <div className="truncate font-medium text-slate-900 dark:text-slate-100">{t.eventName}</div>
                      {t.eventDate && <div className="text-xs text-slate-400 dark:text-slate-500">{formatDateNumeric(t.eventDate)}</div>}
                    </td>
                    {!isNarrow && (
                      // 2.4.4: marko's own request - the Order cell now opens
                      // Order Detail directly and independently of the row's
                      // own click (which goes to Sale Detail instead, for a
                      // sold ticket - see openDetail above), the same
                      // stopPropagation pattern FulfillmentCenter's own "Open"
                      // link already uses on its row.
                      <td className={`${tdCls} truncate`} title={t.orderCode} onClick={(e) => e.stopPropagation()}>
                        <button
                          type="button"
                          onClick={() => navigate(`/orders/${t.orderId}`)}
                          className="truncate text-brand-700 hover:underline dark:text-brand-400"
                        >
                          {t.orderCode}
                        </button>
                      </td>
                    )}
                    {/* 2.4.4: renamed to Seats-only (was "Ticket / Seats",
                        showing the ticket code as the primary line) - marko's
                        own request. The ticket code is still available on
                        hover (title) and wasn't dropped from the app anywhere
                        else - Order Detail/Sale Detail (this row's own click
                        target) both still show it. */}
                    <td className={`${tdCls} truncate`} title={t.code}>
                      {seats ? (
                        <div className="truncate font-medium text-slate-900 dark:text-slate-100">{seats}</div>
                      ) : (
                        <span className="text-slate-400 dark:text-slate-500">-</span>
                      )}
                    </td>
                    {!isNarrow && <td className={tdCls}>{t.tier ?? "-"}</td>}
                    {!isNarrow && <td className={`${tdCls} text-right tabular-nums whitespace-nowrap`}>{formatMoney(t.totalCostCents, t.currency)}</td>}
                    <td className={`${tdCls} text-right tabular-nums whitespace-nowrap`}>
                      {t.listingPriceCents != null ? formatMoney(t.listingPriceCents, t.currency) : <span className="text-slate-400 dark:text-slate-500">-</span>}
                    </td>
                    <td className={tdCls}>
                      {t.listingStatus ? (
                        <div>
                          {t.marketplaceName && <div className="truncate text-[11px] text-slate-400 dark:text-slate-500">{t.marketplaceName}</div>}
                          <Badge tone={t.listingStatus}>{t.listingStatus}</Badge>
                        </div>
                      ) : (
                        <span className="text-slate-400 dark:text-slate-500">-</span>
                      )}
                    </td>
                    <td className={tdCls}>
                      {t.resaleStatus ? <Badge tone={t.resaleStatus.toLowerCase()}>{t.resaleStatus}</Badge> : <span className="text-slate-400 dark:text-slate-500">-</span>}
                    </td>
                    <td className={tdCls}>
                      {t.salePaymentStatus ? (
                        <Badge tone={t.salePaymentStatus}>{t.salePaymentStatus}</Badge>
                      ) : t.isRefunded ? (
                        <Badge tone="refunded">Refunded</Badge>
                      ) : (
                        <span className="text-slate-400 dark:text-slate-500">-</span>
                      )}
                    </td>
                    <td className={tdCls}>
                      {t.deliveryStatus ? <Badge tone={t.deliveryStatus.toLowerCase()}>{t.deliveryStatus}</Badge> : <span className="text-slate-400 dark:text-slate-500">-</span>}
                    </td>
                    <td className={tdCls}>
                      <Badge tone={t.status}>{t.status}</Badge>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}

      <Modal
        open={listingStatusModalOpen}
        onClose={() => setListingStatusModalOpen(false)}
        title={`Change listing status for ${selectedListingRowIds.length} listing${selectedListingRowIds.length === 1 ? "" : "s"}`}
        width="max-w-sm"
      >
        <Field label="New status">
          <Select value={bulkListingStatus} onChange={(e) => setBulkListingStatus(e.target.value as "active" | "sold" | "removed")}>
            <option value="active">Active</option>
            <option value="sold">Sold</option>
            <option value="removed">Removed</option>
          </Select>
        </Field>
        <p className="mt-3 text-xs text-slate-400 dark:text-slate-500">
          Only the marketplace listing's own status changes - the ticket's own status, section, seat, and every
          money/cents field are untouched.
        </p>
        {listingStatusError && <p className="mt-3 text-sm text-red-600 dark:text-red-400">{listingStatusError}</p>}
        <ModalFooter>
          <Button variant="secondary" onClick={() => setListingStatusModalOpen(false)} disabled={listingStatusBusy}>
            Cancel
          </Button>
          <Button variant="primary" onClick={submitListingStatus} disabled={listingStatusBusy}>
            {listingStatusBusy ? "Applying..." : `Apply to ${selectedListingRowIds.length}`}
          </Button>
        </ModalFooter>
      </Modal>
    </div>
  );
}

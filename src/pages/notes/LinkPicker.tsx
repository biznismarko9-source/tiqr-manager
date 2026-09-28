import { useCallback, useEffect, useMemo, useState } from "react";
import { api, errMsg } from "../../lib/api";
import type { NoteLinkKind, OrderRecord, Ticket } from "../../lib/types";
import { Button, Input, Modal, ModalFooter, Select } from "../../components/ui";
import { useToast } from "../../lib/toast";
import { formatMoney, formatSeatLocation } from "../../lib/format";

/**
 * "Attach this tab to…" — six kinds, one picker.
 *
 * 2.61.0 reworks two of them on marko's own report:
 *
 * · **Inventory is browsed BY ORDER.** *"tam by to malo fungovat ako objednavka
 *   ze sa ukaze cela order a ked ju rozkliknes vies si vybrat ci vsetky listky
 *   alebo len niektore"* — a flat list of two hundred ticket codes is not
 *   something anyone picks from. Orders first; expand one; take all of it or
 *   tick the seats you mean.
 *
 * · **Finance entries read as something.** *"ked k tomu nemas popis tak vidis
 *   bez popisu"* — an entry with no note used to be a row saying nothing. It is
 *   now built from whatever the entry DOES have: category, place, order,
 *   account, and always the amount and the date.
 *
 * A link belongs to a SUB-TAB, not to the note, so this takes `pageId`.
 */

export const LINK_KINDS: { kind: NoteLinkKind; label: string }[] = [
  { kind: "order", label: "Objednávka" },
  { kind: "event", label: "Event" },
  { kind: "ticket", label: "Inventár" },
  { kind: "sale", label: "Predaj" },
  { kind: "pull", label: "Pull" },
  { kind: "finance", label: "Financie" },
];

type Row = { id: number; label: string; sub: string };

/** Never "(bez popisu)". Built from whatever the entry actually has, most
 *  identifying first, and it always ends with the amount and the date. */
function financeLabel(f: {
  note: string | null;
  categoryName: string | null;
  place: string | null;
  orderCode: string | null;
  accountName: string | null;
  entryType: "income" | "expense";
}): string {
  const parts = [f.note?.trim(), f.categoryName, f.place, f.orderCode, f.accountName].filter(
    (x): x is string => !!x && x.trim() !== "",
  );
  if (parts.length === 0) return f.entryType === "income" ? "Príjem" : "Výdavok";
  return parts.slice(0, 3).join(" · ");
}

export default function LinkPicker({
  open,
  pageId,
  pageName,
  onClose,
  onLinked,
}: {
  open: boolean;
  pageId: number | null;
  pageName: string;
  onClose: () => void;
  onLinked: () => void;
}) {
  const toast = useToast();
  const [kind, setKind] = useState<NoteLinkKind>("order");
  const [query, setQuery] = useState("");
  const [rows, setRows] = useState<Row[] | null>(null);
  /** Inventory only: the order being browsed, and which of its seats are ticked. */
  const [orders, setOrders] = useState<OrderRecord[] | null>(null);
  const [openOrder, setOpenOrder] = useState<OrderRecord | null>(null);
  const [orderTickets, setOrderTickets] = useState<Ticket[] | null>(null);
  const [picked, setPicked] = useState<Set<number>>(new Set());
  const [saving, setSaving] = useState(false);

  const load = useCallback(
    async (k: NoteLinkKind) => {
      setRows(null);
      setOrders(null);
      setOpenOrder(null);
      setOrderTickets(null);
      setPicked(new Set());
      try {
        if (k === "ticket") {
          setOrders(await api.listOrders({}));
          return;
        }
        if (k === "order") {
          const list = await api.listOrders({});
          setRows(
            list.map((o) => ({
              id: o.id,
              label: `${o.code} · ${o.eventName}`,
              sub: `${o.quantity} ks · ${formatMoney(o.totalCostCents, o.currency)} · ${o.purchaseDate}`,
            })),
          );
          return;
        }
        if (k === "event") {
          const list = await api.listEvents();
          setRows(
            list.map((e) => ({
              id: e.id,
              label: e.name,
              sub: [e.city, e.eventDate].filter(Boolean).join(" · "),
            })),
          );
          return;
        }
        if (k === "sale") {
          const list = await api.listSales({});
          setRows(
            list.map((s) => ({
              id: s.id,
              label: `${s.code} · ${s.eventName}`,
              sub: `${s.saleDate} · ${formatMoney(s.salePriceCents, s.currency)}`,
            })),
          );
          return;
        }
        if (k === "pull") {
          const list = await api.listPulls({});
          setRows(
            list.map((p) => ({
              id: p.id,
              label: `${p.code} · ${p.eventName}`,
              sub: `${p.quantity} ks · ${p.paid ? "zaplatené" : "nezaplatené"}${p.transferDone ? " · prevedené" : ""}`,
            })),
          );
          return;
        }
        const list = await api.listFinanceEntries();
        setRows(
          list.map((f) => ({
            id: f.id,
            label: financeLabel(f),
            sub: `${f.entryDate} · ${f.entryType === "income" ? "+" : "−"}${formatMoney(f.amountCents, f.currency)}`,
          })),
        );
      } catch (e) {
        toast.error(errMsg(e));
        setRows([]);
      }
    },
    [toast],
  );

  useEffect(() => {
    if (!open) return;
    setQuery("");
    void load(kind);
  }, [open, kind, load]);

  async function expandOrder(o: OrderRecord) {
    setOpenOrder(o);
    setOrderTickets(null);
    setPicked(new Set());
    try {
      setOrderTickets(await api.listTickets({ orderId: o.id }));
    } catch (e) {
      toast.error(errMsg(e));
      setOrderTickets([]);
    }
  }

  async function attach(ids: number[], k: NoteLinkKind) {
    if (pageId === null || ids.length === 0) return;
    setSaving(true);
    try {
      // One call per id: the backend treats a repeat on the same tab as a
      // no-op, so a double click costs nothing and nothing half-applies.
      for (const id of ids) await api.addNoteLink(pageId, k, id);
      toast.success(ids.length === 1 ? "Priradené" : `Priradené: ${ids.length}`);
      onLinked();
      onClose();
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setSaving(false);
    }
  }

  const shownRows = useMemo(() => {
    const q = query.trim().toLowerCase();
    const list = rows ?? [];
    return (q ? list.filter((r) => `${r.label} ${r.sub}`.toLowerCase().includes(q)) : list).slice(0, 60);
  }, [rows, query]);

  const shownOrders = useMemo(() => {
    const q = query.trim().toLowerCase();
    const list = orders ?? [];
    return (
      q ? list.filter((o) => `${o.code} ${o.eventName} ${o.eventCity ?? ""}`.toLowerCase().includes(q)) : list
    ).slice(0, 60);
  }, [orders, query]);

  return (
    <Modal
      open={open}
      onClose={onClose}
      title={pageName ? `Priradiť k podkarte „${pageName}"` : "Priradiť k podkarte"}
      width="max-w-2xl"
    >
      <div className="flex flex-col gap-3">
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-[170px_1fr]">
          <label>
            <span className="label">Typ</span>
            <Select value={kind} onChange={(e) => setKind(e.target.value as NoteLinkKind)}>
              {LINK_KINDS.map((k) => (
                <option key={k.kind} value={k.kind}>
                  {k.label}
                </option>
              ))}
            </Select>
          </label>
          <label>
            <span className="label">Hľadať</span>
            <Input value={query} onChange={(e) => setQuery(e.target.value)} placeholder="kód, event, dátum…" />
          </label>
        </div>

        <div className="max-h-[360px] overflow-y-auto rounded-lg border border-slate-200 dark:border-slate-700">
          {/* ── inventory: orders, then the seats inside one ───────────── */}
          {kind === "ticket" ? (
            openOrder ? (
              <div>
                <div className="sticky top-0 flex items-center gap-2 border-b border-slate-200 bg-surface px-3 py-2 dark:border-slate-700">
                  <button
                    type="button"
                    onClick={() => setOpenOrder(null)}
                    className="text-[12px] text-slate-500 underline underline-offset-2 hover:text-slate-800 dark:hover:text-slate-100"
                  >
                    ← Objednávky
                  </button>
                  <span className="truncate text-sm font-semibold text-slate-900 dark:text-slate-50">
                    {openOrder.code} · {openOrder.eventName}
                  </span>
                  <Button
                    variant="secondary"
                    className="ml-auto !py-1 !text-[12px]"
                    disabled={saving || !orderTickets?.length}
                    onClick={() => void attach((orderTickets ?? []).map((t) => t.id), "ticket")}
                  >
                    Všetky ({orderTickets?.length ?? 0})
                  </Button>
                </div>
                {orderTickets === null ? (
                  <p className="p-4 text-sm text-slate-500 dark:text-slate-400">Načítavam…</p>
                ) : orderTickets.length === 0 ? (
                  <p className="p-4 text-sm text-slate-500 dark:text-slate-400">Táto objednávka nemá lístky.</p>
                ) : (
                  <>
                    {orderTickets.map((t) => (
                      <label
                        key={t.id}
                        className="flex cursor-pointer items-center gap-3 border-b border-slate-100 px-3 py-2 last:border-0 hover:bg-surface-sunken dark:border-slate-800"
                      >
                        <input
                          type="checkbox"
                          checked={picked.has(t.id)}
                          onChange={() =>
                            setPicked((cur) => {
                              const next = new Set(cur);
                              if (next.has(t.id)) next.delete(t.id);
                              else next.add(t.id);
                              return next;
                            })
                          }
                        />
                        <span className="min-w-0 flex-1 truncate text-sm text-slate-900 dark:text-slate-50">
                          {t.code}
                        </span>
                        <span className="shrink-0 text-[11px] text-slate-500 dark:text-slate-400">
                          {formatSeatLocation(t.section, t.rowLabel, t.seat) || "—"}
                        </span>
                      </label>
                    ))}
                    <div className="sticky bottom-0 flex items-center gap-2 border-t border-slate-200 bg-surface px-3 py-2 dark:border-slate-700">
                      <span className="text-[12px] text-slate-500 dark:text-slate-400">
                        Označené: {picked.size}
                      </span>
                      <Button
                        variant="primary"
                        className="ml-auto !py-1 !text-[12px]"
                        disabled={saving || picked.size === 0}
                        onClick={() => void attach([...picked], "ticket")}
                      >
                        Priradiť označené
                      </Button>
                    </div>
                  </>
                )}
              </div>
            ) : orders === null ? (
              <p className="p-4 text-sm text-slate-500 dark:text-slate-400">Načítavam…</p>
            ) : shownOrders.length === 0 ? (
              <p className="p-4 text-sm text-slate-500 dark:text-slate-400">Nič sa nenašlo.</p>
            ) : (
              shownOrders.map((o) => (
                <button
                  key={o.id}
                  type="button"
                  onClick={() => void expandOrder(o)}
                  className="flex w-full items-baseline gap-3 border-b border-slate-100 px-3 py-2 text-left transition last:border-0 hover:bg-surface-sunken dark:border-slate-800"
                >
                  <span className="min-w-0 flex-1 truncate text-sm text-slate-900 dark:text-slate-50">
                    {o.code} · {o.eventName}
                  </span>
                  <span className="shrink-0 text-[11px] text-slate-500 dark:text-slate-400">
                    {o.quantity} ks →
                  </span>
                </button>
              ))
            )
          ) : /* ── everything else: one flat list ───────────────────────── */
          rows === null ? (
            <p className="p-4 text-sm text-slate-500 dark:text-slate-400">Načítavam…</p>
          ) : shownRows.length === 0 ? (
            <p className="p-4 text-sm text-slate-500 dark:text-slate-400">Nič sa nenašlo.</p>
          ) : (
            shownRows.map((r) => (
              <button
                key={r.id}
                type="button"
                disabled={saving}
                onClick={() => void attach([r.id], kind)}
                className="flex w-full items-baseline gap-3 border-b border-slate-100 px-3 py-2 text-left transition last:border-0 hover:bg-surface-sunken dark:border-slate-800"
              >
                <span className="min-w-0 flex-1 truncate text-sm text-slate-900 dark:text-slate-50">{r.label}</span>
                <span className="shrink-0 text-[11px] text-slate-500 dark:text-slate-400">{r.sub}</span>
              </button>
            ))
          )}
        </div>
      </div>
      <ModalFooter>
        <Button variant="secondary" onClick={onClose}>
          Zavrieť
        </Button>
      </ModalFooter>
    </Modal>
  );
}

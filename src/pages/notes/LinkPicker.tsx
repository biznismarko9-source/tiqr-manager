import { useCallback, useEffect, useMemo, useState } from "react";
import { api, errMsg } from "../../lib/api";
import type { NoteLinkKind } from "../../lib/types";
import { Button, Input, Modal, ModalFooter, Select } from "../../components/ui";
import { useToast } from "../../lib/toast";
import { formatMoney } from "../../lib/format";

/**
 * "Attach this note to…" — marko: *"neni to len pre kody ale aj do buducna na
 * zapisovacky, priradovanie eventu, pullu, inventaru, sellu atd"* + *"aj
 * finance"*.
 *
 * Six kinds, one picker. Each kind reads its own existing list command rather
 * than a new backend search, because those lists already know how to describe
 * their own records and a seventh way of finding an order is one too many.
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

export default function LinkPicker({
  open,
  noteId,
  onClose,
  onLinked,
}: {
  open: boolean;
  noteId: number;
  onClose: () => void;
  onLinked: () => void;
}) {
  const toast = useToast();
  const [kind, setKind] = useState<NoteLinkKind>("order");
  const [query, setQuery] = useState("");
  const [rows, setRows] = useState<Row[] | null>(null);

  const load = useCallback(
    async (k: NoteLinkKind) => {
      setRows(null);
      try {
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
          setRows(list.map((e) => ({ id: e.id, label: e.name, sub: e.eventDate ?? "" })));
          return;
        }
        if (k === "ticket") {
          const list = await api.listTickets({});
          setRows(
            list.map((t) => ({
              id: t.id,
              label: `${t.code} · ${t.eventName}`,
              sub: [t.section, t.rowLabel, t.seat].filter(Boolean).join(" · "),
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
            label: f.note?.trim() || "(bez popisu)",
            sub: `${f.entryDate} · ${formatMoney(f.amountCents, "EUR")}`,
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

  const shown = useMemo(() => {
    const q = query.trim().toLowerCase();
    const list = rows ?? [];
    const filtered = q ? list.filter((r) => `${r.label} ${r.sub}`.toLowerCase().includes(q)) : list;
    // The list is only for picking one thing; a hundred rows would be scrolling
    // where a search box is faster.
    return filtered.slice(0, 60);
  }, [rows, query]);

  return (
    <Modal open={open} onClose={onClose} title="Priradiť k záznamu" width="max-w-xl">
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

        <div className="max-h-[320px] overflow-y-auto rounded-lg border border-slate-200 dark:border-slate-700">
          {rows === null ? (
            <p className="p-4 text-sm text-slate-500 dark:text-slate-400">Načítavam…</p>
          ) : shown.length === 0 ? (
            <p className="p-4 text-sm text-slate-500 dark:text-slate-400">Nič sa nenašlo.</p>
          ) : (
            shown.map((r) => (
              <button
                key={r.id}
                type="button"
                onClick={async () => {
                  try {
                    await api.addNoteLink(noteId, kind, r.id);
                    onLinked();
                    onClose();
                  } catch (e) {
                    toast.error(errMsg(e));
                  }
                }}
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

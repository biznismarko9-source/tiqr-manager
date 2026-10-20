// 2.114.0 - cleaning up Promotions with the AI helper.
//
// marko: "ked velmi vela spamuju promotions tak 80% z toho su shits ktore
// vobec netreba, AI scan dat uplne co najlacnejsi ako moze byt s tym ze bude
// fungovat rovnako dobre". The backend sorts (commands/mail_cleanup.rs): free
// rules first, the cheapest model for the rest, every answer kept. This
// screen only SHOWS what it suggests - the sure junk ticked, the unsure not -
// and moves nothing until marko says so. Moving means TIQR's Trash, where
// every message can be restored.
import { useEffect, useMemo, useState } from "react";
import { api, errMsg } from "../lib/api";
import { costText } from "../lib/promoCleanup";
import { useToast } from "../lib/toast";
import type { PromoScan, PromoSuggestion } from "../lib/types";
import { IconSparkles, IconTrash } from "./icons";
import { Button, Modal, Spinner } from "./ui";

export function PromoCleanup({ onMoved }: { onMoved: () => void }) {
  const toast = useToast();
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [scan, setScan] = useState<PromoScan | null>(null);
  const [items, setItems] = useState<PromoSuggestion[] | null>(null);
  const [ticked, setTicked] = useState<Set<number>>(new Set());
  const [moving, setMoving] = useState(false);

  const load = async () => {
    const list = await api.listPromoSuggestions();
    setItems(list);
    // The sure junk is ticked; the unsure is for marko to decide.
    setTicked(new Set(list.filter((s) => s.verdict === "junk").map((s) => s.id)));
  };

  useEffect(() => {
    void load().catch(() => setItems([]));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const run = async () => {
    setBusy(true);
    setOpen(true);
    try {
      setScan(await api.scanPromotions());
      await load();
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setBusy(false);
    }
  };

  const junk = useMemo(() => (items ?? []).filter((s) => s.verdict === "junk"), [items]);
  const unsure = useMemo(() => (items ?? []).filter((s) => s.verdict === "unsure"), [items]);
  const toggle = (id: number) =>
    setTicked((prev) => {
      const next = new Set(prev);
      if (!next.delete(id)) next.add(id);
      return next;
    });

  const move = async () => {
    const ids = [...ticked];
    if (ids.length === 0) return;
    setMoving(true);
    try {
      const n = await api.trashMail(ids);
      toast.success(`${n.toLocaleString()} promotion${n === 1 ? "" : "s"} moved to Trash - restore them from there any time.`);
      await load();
      onMoved();
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setMoving(false);
    }
  };

  const row = (s: PromoSuggestion) => (
    <li key={s.id}>
      <label className="flex cursor-pointer items-start gap-2 rounded-md px-2 py-1.5 hover:bg-surface-muted">
        <input type="checkbox" className="mt-0.5 h-3.5 w-3.5 shrink-0" checked={ticked.has(s.id)} onChange={() => toggle(s.id)} />
        <span className="min-w-0 flex-1">
          <span className="flex items-baseline gap-2">
            <span className="truncate text-xs font-medium text-slate-800 dark:text-slate-100">{s.fromName || s.fromAddress}</span>
            <span className="shrink-0 text-[10.5px] text-slate-400 dark:text-slate-500">
              {new Date(s.receivedAt).toLocaleDateString()}
            </span>
          </span>
          <span className="block truncate text-[11.5px] text-slate-600 dark:text-slate-300" title={s.snippet ?? undefined}>
            {s.subject || "(no subject)"}
          </span>
        </span>
      </label>
    </li>
  );

  const waiting = junk.length + unsure.length;
  return (
    <>
      <div className="flex items-center gap-2 border-b border-slate-200 px-3 py-2 dark:border-slate-800">
        <IconSparkles className="h-3.5 w-3.5 shrink-0 text-brand-600 dark:text-brand-400" />
        <p className="min-w-0 flex-1 text-[11px] leading-snug text-slate-500 dark:text-slate-400">
          {waiting > 0 ? (
            <>
              <b className="font-semibold text-slate-700 dark:text-slate-200">{junk.length.toLocaleString()}</b> look like junk
              {unsure.length > 0 ? `, ${unsure.length.toLocaleString()} not sure` : ""}.
            </>
          ) : (
            "Sort junk from what matters - TIQR's own rules first, the cheapest AI for the rest."
          )}
        </p>
        {waiting > 0 && (
          <Button variant="ghost" size="sm" onClick={() => setOpen(true)}>
            Review
          </Button>
        )}
        <Button variant="secondary" size="sm" disabled={busy} onClick={() => void run()}>
          {busy ? <Spinner className="h-3.5 w-3.5" /> : <IconSparkles className="h-3.5 w-3.5" />}
          Clean up with AI
        </Button>
      </div>

      {open && (
        <Modal open onClose={() => setOpen(false)} title="Clean up Promotions" width="max-w-2xl">
          {busy && (
            <p className="mb-3 flex items-center gap-2 text-xs text-slate-500 dark:text-slate-400">
              <Spinner className="h-4 w-4" /> Sorting promotions - sixty to a request, each one only once...
            </p>
          )}
          {scan && !busy && (
            <p className="mb-3 rounded-lg bg-surface-muted px-3 py-2 text-[11.5px] leading-snug text-slate-600 dark:text-slate-300">
              Looked at {scan.scanned.toLocaleString()}: {scan.junk.toLocaleString()} junk, {scan.unsure.toLocaleString()} not
              sure, {scan.keep.toLocaleString()} kept. {costText(scan)}.
              {scan.remaining > 0 && ` ${scan.remaining.toLocaleString()} more were not looked at yet.`}
            </p>
          )}
          {items && items.length === 0 && !busy ? (
            <p className="text-sm text-slate-500 dark:text-slate-400">Nothing to clean up right now.</p>
          ) : (
            <div className="max-h-[52vh] space-y-3 overflow-y-auto">
              {junk.length > 0 && (
                <section>
                  <p className="section-title mb-1">Junk - ticked ({junk.length.toLocaleString()})</p>
                  <ul>{junk.map(row)}</ul>
                </section>
              )}
              {unsure.length > 0 && (
                <section>
                  <p className="section-title mb-1">Not sure - yours to decide ({unsure.length.toLocaleString()})</p>
                  <ul>{unsure.map(row)}</ul>
                </section>
              )}
            </div>
          )}
          <div className="-mx-5 -mb-4 mt-4 flex flex-wrap items-center justify-end gap-2 bg-surface-muted px-5 py-3">
            <p className="mr-auto text-[10.5px] leading-snug text-slate-400 dark:text-slate-500">
              Only the sender&apos;s domain, the subject and the first words are sent. Starred mail, mail from ticket
              marketplaces, codes and orders are always kept.
            </p>
            {scan && scan.remaining > 0 && (
              <Button variant="ghost" size="sm" disabled={busy} onClick={() => void run()}>
                Look at {Math.min(scan.remaining, 600).toLocaleString()} more
              </Button>
            )}
            <Button variant="danger" size="sm" disabled={moving || ticked.size === 0} onClick={() => void move()}>
              {moving ? <Spinner className="h-3.5 w-3.5" /> : <IconTrash className="h-3.5 w-3.5" />}
              Move {ticked.size.toLocaleString()} to Trash
            </Button>
          </div>
        </Modal>
      )}
    </>
  );
}

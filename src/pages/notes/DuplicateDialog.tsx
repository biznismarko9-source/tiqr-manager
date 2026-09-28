import { useEffect, useState } from "react";
import { Button, Modal, ModalFooter } from "../../components/ui";

/**
 * "Duplicate this — with or without what it is attached to?" (2.62.0)
 *
 * The question is asked rather than assumed on purpose. A copy that silently
 * drags the assignments along is the kind of thing you only notice after it
 * has already been synced to the other machine; a copy that silently drops
 * them is the kind of thing you only notice when you go looking for them.
 * Either default is wrong for somebody, so neither is the default.
 */

export default function DuplicateDialog({
  open,
  title,
  intro,
  contentLabel,
  linksLabel,
  onCancel,
  onConfirm,
}: {
  open: boolean;
  title: string;
  intro: string;
  contentLabel: string;
  linksLabel: string;
  onCancel: () => void;
  onConfirm: (withLinks: boolean) => void;
}) {
  const [withLinks, setWithLinks] = useState(false);
  useEffect(() => {
    if (open) setWithLinks(false);
  }, [open]);

  const opt = (on: boolean) =>
    `flex w-full items-start gap-2.5 rounded-lg px-3 py-2.5 text-left transition ring-1 ${
      on
        ? "bg-brand-50 ring-brand-400 dark:bg-brand-500/10 dark:ring-brand-500"
        : "ring-slate-200 hover:bg-surface-sunken dark:ring-slate-700"
    }`;

  return (
    <Modal open={open} onClose={onCancel} title={title} width="max-w-md">
      <p className="mb-3 text-[12.5px] leading-relaxed text-slate-500 dark:text-slate-400">{intro}</p>
      <div className="flex flex-col gap-2">
        {[false, true].map((v) => (
          <button key={String(v)} type="button" onClick={() => setWithLinks(v)} className={opt(withLinks === v)}>
            <span
              className={`mt-1 h-3.5 w-3.5 shrink-0 rounded-full ${
                withLinks === v ? "border-[4px] border-brand-600" : "border-[1.5px] border-slate-400 dark:border-slate-500"
              }`}
            />
            <span className="min-w-0">
              <b className="block text-[13px] font-semibold text-slate-900 dark:text-slate-50">
                {v ? "Obsah + priradenia" : "Len obsah"}
              </b>
              <span className="block text-[11.5px] text-slate-500 dark:text-slate-400">
                {v ? linksLabel : contentLabel}
              </span>
            </span>
          </button>
        ))}
      </div>
      <ModalFooter>
        <Button variant="secondary" onClick={onCancel}>
          Zrušiť
        </Button>
        <Button variant="primary" onClick={() => onConfirm(withLinks)}>
          Duplikovať
        </Button>
      </ModalFooter>
    </Modal>
  );
}

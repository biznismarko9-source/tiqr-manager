import { useEffect, useState } from "react";
import { api, errMsg } from "../../lib/api";
import type { ChecklistItem, WorkspaceField, WorkspaceItem, WorkspaceKind } from "../../lib/types";
import { Button, Input, Modal, ModalFooter, Select, Textarea } from "../../components/ui";
import { IconPlus, IconX } from "../../components/icons";
import { useToast } from "../../lib/toast";

/**
 * One editor for all three kinds, because they ARE one row (migration 032).
 *
 * The brief: "Do not force the user to know the final structure when creating
 * information" - so `kind` is a control inside the editor, not a decision made
 * before it opens. Changing it here converts the item in place; nothing typed
 * is lost, because fields, checklist and due date all survive the change and
 * simply stop being shown by the kind that has no use for them.
 */

export const CATEGORIES = ["General", "Buyers", "Codes", "Accounts", "Tasks", "Plans", "Important", "Other"];

/** A password-ish field is masked in the editor too, not only in search. This
 *  is not security - the database is a plain file - it is only about not
 *  putting a secret on screen when nobody asked to see it. */
function looksSecret(name: string): boolean {
    const n = name.toLowerCase();
    return ["password", "heslo", "pass", "pin", "secret", "token", "api key", "apikey", "2fa", "seed"].some((k) =>
      n.includes(k),
    );
}

export default function ItemEditor({
  open,
  initial,
  defaultKind,
  onClose,
  onSaved,
}: {
  open: boolean;
  initial: WorkspaceItem | null;
  defaultKind: WorkspaceKind;
  onClose: () => void;
  onSaved: (item: WorkspaceItem) => void;
}) {
  const toast = useToast();
  const [kind, setKind] = useState<WorkspaceKind>(defaultKind);
  const [title, setTitle] = useState("");
  const [content, setContent] = useState("");
  const [category, setCategory] = useState("");
  const [tagText, setTagText] = useState("");
  const [fields, setFields] = useState<WorkspaceField[]>([]);
  const [checklist, setChecklist] = useState<ChecklistItem[]>([]);
  const [dueDate, setDueDate] = useState("");
  const [status, setStatus] = useState("open");
  const [reveal, setReveal] = useState<number | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!open) return;
    setKind(initial?.kind ?? defaultKind);
    setTitle(initial?.title ?? "");
    setContent(initial?.content ?? "");
    setCategory(initial?.category ?? "");
    setTagText((initial?.tags ?? []).join(", "));
    setFields(initial?.fields ?? []);
    setChecklist(initial?.checklist ?? []);
    setDueDate(initial?.dueDate ?? "");
    setStatus(initial?.status ?? "open");
    setReveal(null);
  }, [open, initial, defaultKind]);

  async function save() {
    setSaving(true);
    try {
      const saved = await api.saveWorkspaceItem({
        id: initial?.id,
        kind,
        title,
        content,
        category: category.trim() || null,
        tags: tagText
          .split(/[,\s]+/)
          .map((t) => t.trim())
          .filter(Boolean),
        fields: fields.filter((f) => f.name.trim() || f.value.trim()),
        checklist: checklist.filter((c) => c.text.trim()),
        status: kind === "task" ? status : null,
        dueDate: dueDate.trim() || null,
        pinned: initial?.pinned ?? false,
        archived: initial?.archived ?? false,
      });
      toast.success(initial ? "Saved" : "Created");
      onSaved(saved);
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      setSaving(false);
    }
  }

  return (
    <Modal
      open={open}
      onClose={onClose}
      title={initial ? "Edit" : kind === "record" ? "New record" : kind === "task" ? "New task" : "New note"}
      width="max-w-3xl"
    >
      <div className="flex flex-col gap-3">
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-[1fr_150px_170px]">
          <label>
            <span className="label">Title</span>
            <Input value={title} onChange={(e) => setTitle(e.target.value)} placeholder="Optional" />
          </label>
          <label>
            <span className="label">Kind</span>
            <Select value={kind} onChange={(e) => setKind(e.target.value as WorkspaceKind)}>
              <option value="note">Note</option>
              <option value="record">Record</option>
              <option value="task">Task</option>
            </Select>
          </label>
          <label>
            <span className="label">Category</span>
            <Select value={category} onChange={(e) => setCategory(e.target.value)}>
              <option value="">—</option>
              {CATEGORIES.map((c) => (
                <option key={c} value={c}>
                  {c}
                </option>
              ))}
            </Select>
          </label>
        </div>

        <label>
          <span className="label">{kind === "record" ? "Notes" : "Content"}</span>
          <Textarea
            rows={kind === "record" ? 3 : 7}
            value={content}
            onChange={(e) => setContent(e.target.value)}
            placeholder="John123 bought 4 Oasis codes, send the remaining 2 tomorrow…"
          />
        </label>

        {kind === "record" && (
          <div>
            <span className="label">Fields</span>
            <div className="flex flex-col gap-2">
              {fields.map((f, i) => (
                <div key={i} className="flex flex-wrap items-center gap-2">
                  <Input
                    value={f.name}
                    onChange={(e) =>
                      setFields((fs) => fs.map((x, k) => (k === i ? { ...x, name: e.target.value } : x)))
                    }
                    placeholder="Nick"
                    className="w-[190px]"
                    aria-label={`Field name ${i + 1}`}
                  />
                  <Input
                    value={f.value}
                    type={looksSecret(f.name) && reveal !== i ? "password" : "text"}
                    onChange={(e) =>
                      setFields((fs) => fs.map((x, k) => (k === i ? { ...x, value: e.target.value } : x)))
                    }
                    placeholder="John123"
                    className="min-w-[180px] flex-1"
                    aria-label={`Field value ${i + 1}`}
                  />
                  {looksSecret(f.name) && (
                    <Button variant="secondary" onClick={() => setReveal((r) => (r === i ? null : i))}>
                      {reveal === i ? "Hide" : "Show"}
                    </Button>
                  )}
                  <button
                    type="button"
                    onClick={() => setFields((fs) => fs.filter((_, k) => k !== i))}
                    aria-label={`Remove field ${i + 1}`}
                    className="rounded-md p-1 text-slate-400 transition hover:text-red-600 dark:hover:text-red-400"
                  >
                    <IconX className="h-4 w-4" />
                  </button>
                </div>
              ))}
              <Button variant="secondary" onClick={() => setFields((fs) => [...fs, { name: "", value: "" }])}>
                <IconPlus className="h-4 w-4" /> Add field
              </Button>
            </div>
          </div>
        )}

        {kind === "task" && (
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
            <label>
              <span className="label">Due</span>
              <Input type="date" value={dueDate} onChange={(e) => setDueDate(e.target.value)} />
            </label>
            <label>
              <span className="label">Status</span>
              <Select value={status} onChange={(e) => setStatus(e.target.value)}>
                <option value="open">Open</option>
                <option value="done">Done</option>
              </Select>
            </label>
          </div>
        )}

        {kind !== "task" && (
          <label>
            <span className="label">Date (optional)</span>
            <Input type="date" value={dueDate} onChange={(e) => setDueDate(e.target.value)} />
          </label>
        )}

        <div>
          <span className="label">Checklist</span>
          <div className="flex flex-col gap-1.5">
            {checklist.map((c, i) => (
              <div key={i} className="flex items-center gap-2">
                <input
                  type="checkbox"
                  checked={c.done}
                  onChange={(e) =>
                    setChecklist((cs) => cs.map((x, k) => (k === i ? { ...x, done: e.target.checked } : x)))
                  }
                  aria-label={`Done, step ${i + 1}`}
                />
                <Input
                  value={c.text}
                  onChange={(e) => setChecklist((cs) => cs.map((x, k) => (k === i ? { ...x, text: e.target.value } : x)))}
                  placeholder="Send the remaining codes"
                  aria-label={`Step ${i + 1}`}
                />
                <button
                  type="button"
                  onClick={() => setChecklist((cs) => cs.filter((_, k) => k !== i))}
                  aria-label={`Remove step ${i + 1}`}
                  className="rounded-md p-1 text-slate-400 transition hover:text-red-600 dark:hover:text-red-400"
                >
                  <IconX className="h-4 w-4" />
                </button>
              </div>
            ))}
            <Button variant="secondary" onClick={() => setChecklist((cs) => [...cs, { text: "", done: false }])}>
              <IconPlus className="h-4 w-4" /> Add step
            </Button>
          </div>
        </div>

        <label>
          <span className="label">Tags</span>
          <Input
            value={tagText}
            onChange={(e) => setTagText(e.target.value)}
            placeholder="oasis, followup, important"
          />
        </label>
      </div>

      <ModalFooter>
        <Button variant="secondary" onClick={onClose} disabled={saving}>
          Cancel
        </Button>
        <Button variant="primary" onClick={save} disabled={saving}>
          {initial ? "Save" : "Create"}
        </Button>
      </ModalFooter>
    </Modal>
  );
}

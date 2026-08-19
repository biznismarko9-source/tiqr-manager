import { useState } from "react";
import { api, errMsg } from "../lib/api";
import type { BulkTicketField, Ticket } from "../lib/types";
import { Button, CHECKBOX_CLASS, Field, Input, Modal, ModalFooter, Select } from "./ui";
import { useToast } from "../lib/toast";

// Re-exported so pages that add a bulk-selection column next to this bar
// don't need a second import for the same constant.
export { CHECKBOX_CLASS };

const FIELD_OPTIONS: { value: BulkTicketField; label: string; kind: "text" | "money" }[] = [
  { value: "section", label: "Section", kind: "text" },
  { value: "rowLabel", label: "Row", kind: "text" },
  { value: "seat", label: "Seat", kind: "text" },
  { value: "listingPriceCents", label: "Listing price", kind: "money" },
];

/** Sale Detail and Order Detail both need "select some tickets, change one
 * field on all of them at once" - this is that one shared engine (1.8.3
 * brief section 7: "ak je rovnaká bulk edit engine využiteľná ... použi
 * spoločnú komponentu"), not two parallel implementations.
 *
 * Deliberately offers only the 4 fields already safe to edit on a ticket
 * regardless of its status in the existing single-ticket `TicketEditModal`
 * (section, row, seat, listing price) - never status itself. See
 * `bulk_update_tickets_impl` (tickets.rs) for the full safety reasoning: a
 * naive bulk status change could silently create a sold ticket with no
 * active sale (or the reverse), which nothing else in the app could detect
 * or repair. This component can only ever call a backend command that has
 * no code path into the `status` column at all - the guard is structural,
 * not just a UI omission.
 *
 * 1.9.1: "Ticket type" used to be a 5th option here - removed at marko's
 * request; it's now set once when the order is created (see Orders.tsx's
 * OrderFormModal) instead of being changeable in bulk afterwards. */
export function BulkTicketEditBar({
  selectedIds,
  currency,
  onClear,
  onApplied,
}: {
  selectedIds: number[];
  /** Shown next to the listing price field. Pass the ticket/order's single
   * currency, or null when the selection could span more than one - the
   * field still just sets a raw amount either way, same as single-ticket
   * edit already allows regardless of currency. */
  currency: string | null;
  onClear: () => void;
  onApplied: (updated: Ticket[]) => void;
}) {
  const toast = useToast();
  const [modalOpen, setModalOpen] = useState(false);
  const [field, setField] = useState<BulkTicketField>("section");
  const [value, setValue] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  if (selectedIds.length === 0) return null;

  const openModal = () => {
    setField("section");
    setValue("");
    setError(null);
    setModalOpen(true);
  };

  const activeOption = FIELD_OPTIONS.find((f) => f.value === field) ?? FIELD_OPTIONS[0];
  const trimmedValue = value.trim();
  // Require an actual value - an empty bulk edit would silently NULL this
  // field on every selected ticket, which is never what "Apply" should do
  // by accident. (Single-ticket edit allows blank-to-clear deliberately;
  // here the blast radius is N tickets, so this stays a click away instead
  // of the default state.)
  const canSubmit = trimmedValue !== "";

  const submit = async () => {
    setError(null);
    if (!canSubmit) return;
    let textValue: string | null = null;
    let centsValue: number | null = null;
    if (activeOption.kind === "money") {
      const s = trimmedValue.replace(",", ".");
      if (!/^\d+(\.\d{1,2})?$/.test(s)) {
        setError("Listing price is not a valid amount");
        return;
      }
      centsValue = Math.round(parseFloat(s) * 100);
    } else {
      textValue = trimmedValue;
    }
    setSaving(true);
    try {
      const updated = await api.bulkUpdateTickets({
        ticketIds: selectedIds,
        field,
        textValue,
        centsValue,
      });
      toast.success(`${updated.length} ticket${updated.length === 1 ? "" : "s"} updated`);
      setModalOpen(false);
      onApplied(updated);
    } catch (e) {
      setError(errMsg(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <>
      <div className="mb-4 flex items-center gap-3 rounded-lg bg-brand-50 dark:bg-brand-500/10 px-4 py-2.5 text-sm ring-1 ring-inset ring-brand-200 dark:ring-brand-500/30">
        <span className="font-medium text-brand-800 dark:text-brand-300">Selected: {selectedIds.length}</span>
        <Button variant="secondary" onClick={openModal}>
          Bulk edit...
        </Button>
        <button
          type="button"
          className="ml-auto text-xs font-medium text-brand-700 dark:text-brand-400 hover:underline"
          onClick={onClear}
        >
          Clear selection
        </button>
      </div>

      <Modal
        open={modalOpen}
        onClose={() => setModalOpen(false)}
        title={`Bulk edit ${selectedIds.length} ticket${selectedIds.length === 1 ? "" : "s"}`}
        width="max-w-sm"
      >
        <Field label="Field to change">
          <Select value={field} onChange={(e) => setField(e.target.value as BulkTicketField)}>
            {FIELD_OPTIONS.map((f) => (
              <option key={f.value} value={f.value}>
                {f.label}
              </option>
            ))}
          </Select>
        </Field>
        <div className="mt-4">
          <Field
            label={
              activeOption.kind === "money"
                ? `New listing price${currency ? ` (${currency})` : ""}`
                : `New ${activeOption.label.toLowerCase()}`
            }
          >
            <Input
              autoFocus
              inputMode={activeOption.kind === "money" ? "decimal" : undefined}
              placeholder={activeOption.kind === "money" ? "0.00" : ""}
              value={value}
              onChange={(e) => setValue(e.target.value)}
            />
          </Field>
        </div>
        <p className="mt-3 text-xs text-slate-400 dark:text-slate-500">
          Only this ticket detail is changed on every selected ticket - sale price, fees and payment status are not
          affected. Ticket status can&apos;t be changed here; use the Sales screen (create, refund or delete a sale)
          instead.
        </p>
        {error && <p className="mt-3 text-sm text-red-600 dark:text-red-400">{error}</p>}
        <ModalFooter>
          <Button variant="secondary" onClick={() => setModalOpen(false)} disabled={saving}>
            Cancel
          </Button>
          <Button variant="primary" onClick={submit} disabled={saving || !canSubmit}>
            {saving ? "Applying..." : `Apply to ${selectedIds.length}`}
          </Button>
        </ModalFooter>
      </Modal>
    </>
  );
}

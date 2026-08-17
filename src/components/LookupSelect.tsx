import { useState } from "react";
import { Button, Input, Select } from "./ui";

export interface LookupOption {
  id: number;
  name: string;
}

/** A <select> of existing lookup rows (platforms/suppliers) with an inline
 * "+ New" affordance that creates a new row on the fly, so the user is never
 * blocked waiting to go manage lookups elsewhere first. */
export function LookupSelect({
  label,
  options,
  value,
  onChange,
  onCreate,
  placeholder = "None",
}: {
  label: string;
  options: LookupOption[];
  value: number | null;
  onChange: (id: number | null) => void;
  onCreate: (name: string) => Promise<LookupOption>;
  placeholder?: string;
}) {
  const [adding, setAdding] = useState(false);
  const [newName, setNewName] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const confirmAdd = async () => {
    if (!newName.trim()) return;
    setBusy(true);
    setError(null);
    try {
      const created = await onCreate(newName.trim());
      onChange(created.id);
      setNewName("");
      setAdding(false);
    } catch (e) {
      setError(typeof e === "string" ? e : "Could not create");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div>
      <div className="flex items-center justify-between">
        <span className="label mb-1">{label}</span>
        <button
          type="button"
          className="mb-1 text-xs font-medium text-brand-600 hover:underline"
          onClick={() => {
            setAdding((a) => !a);
            setError(null);
          }}
        >
          {adding ? "Cancel" : "+ New"}
        </button>
      </div>
      {adding ? (
        <div className="flex gap-2">
          <Input
            autoFocus
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
            placeholder={`New ${label.toLowerCase()} name`}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                confirmAdd();
              }
            }}
          />
          <Button type="button" variant="secondary" disabled={busy || !newName.trim()} onClick={confirmAdd}>
            Add
          </Button>
        </div>
      ) : (
        <Select
          value={value ?? ""}
          onChange={(e) => onChange(e.target.value ? Number(e.target.value) : null)}
        >
          <option value="">{placeholder}</option>
          {options.map((o) => (
            <option key={o.id} value={o.id}>
              {o.name}
            </option>
          ))}
        </Select>
      )}
      {error && <p className="mt-1 text-xs text-red-600">{error}</p>}
    </div>
  );
}

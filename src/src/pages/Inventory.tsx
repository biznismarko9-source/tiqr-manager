import { TicketsView } from "./Tickets";

export default function Inventory() {
  return (
    <TicketsView
      title="Inventory"
      subtitle="Your current sellable stock — available and listed tickets only."
      lockedStatus="available,listed"
    />
  );
}

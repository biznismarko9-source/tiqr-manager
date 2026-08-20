import { TicketsView } from "./Tickets";

export default function Inventory() {
  return (
    <TicketsView
      title="Inventory"
      subtitle="Your current sellable stock — available and listed tickets only."
      lockedStatus="available,listed"
      // 1.9.2 (section 1): Inventory is the one page marko explicitly kept
      // Order/Event cross-links on - see TicketsView's allowCrossLinks doc
      // comment in Tickets.tsx for the full reasoning.
      allowCrossLinks
    />
  );
}

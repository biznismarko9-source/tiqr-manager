// The Seats field of an order, read and written. Pure, so `npm test` runs it
// in Node (seats.test.ts).

/** Turns the free-form "Seats" input into one label per ticket.
 * Accepts a numeric range ("12-15" -> 12,13,14,15, either direction) or a
 * comma-separated list ("12, 14, 16A"). Blank input -> no seats assigned. */
export function parseSeats(raw: string): string[] {
  const trimmed = raw.trim();
  if (!trimmed) return [];
  const rangeMatch = trimmed.match(/^(\d+)\s*-\s*(\d+)$/);
  if (rangeMatch) {
    const start = parseInt(rangeMatch[1], 10);
    const end = parseInt(rangeMatch[2], 10);
    const step = start <= end ? 1 : -1;
    const out: string[] = [];
    for (let n = start; step > 0 ? n <= end : n >= end; n += step) out.push(String(n));
    return out;
  }
  return trimmed
    .split(",")
    .map((s) => s.trim())
    .filter((s) => s.length > 0);
}

/** 2.115.0: the first seat of a run of numbered seats ("23", "23-26",
 *  "23, 24, 25") - `null` for anything else. */
export function seatRun(seats: string[]): number | null {
  if (seats.length === 0 || !seats.every((s) => /^\d+$/.test(s))) return null;
  const n = seats.map(Number).sort((a, b) => a - b);
  return n.every((v, i) => i === 0 || v === n[i - 1] + 1) ? n[0] : null;
}

/** The longest run a typed count grows the seats to by itself. */
export const MAX_SEAT_RUN = 500;

/** 2.115.0, marko: "pri new order nejde menit kvantita manualne". The seats
 *  once the count is typed: numbered seats in a run follow it from their
 *  first seat - "23" with a count of 3 is "23-25". `null` leaves them as
 *  they are: none, not a run, or not a count. */
export function seatsForCount(seatsRaw: string, count: number): string | null {
  const first = seatRun(parseSeats(seatsRaw));
  if (first === null || !Number.isInteger(count) || count < 1 || count > MAX_SEAT_RUN) return null;
  return count === 1 ? String(first) : `${first}-${first + count - 1}`;
}

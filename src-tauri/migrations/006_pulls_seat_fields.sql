-- TIQR Manager - 006_pulls_seat_fields
-- 1.9.8: marko asked for Pull's single free-text "seats" field to be split
-- into the same section/row/seat shape `tickets` already uses (see
-- tickets.section/row_label/seat, format.ts's formatSeatLocation), instead
-- of one generic text box - a general-admission pull just leaves all three
-- blank, same as a general-admission ticket does today.
--
-- Existing data: whatever was already typed into the old `seats` column is
-- best-effort carried over into the new `seat` column (the closest of the
-- three in meaning) rather than silently discarded - see the UPDATE below.
-- The old `seats` column itself is deliberately left in place afterwards,
-- just unused from now on: SQLite's `ALTER TABLE ... DROP COLUMN` exists
-- (3.35+) but isn't exercised anywhere else in this codebase yet, so adding
-- 3 columns + copying data is the more conservative choice here, consistent
-- with this feature's other columns that were left in place rather than
-- dropped (see 1.9.8's report for `transfer_deadline`).

ALTER TABLE pulls ADD COLUMN section TEXT;
ALTER TABLE pulls ADD COLUMN row_label TEXT;
ALTER TABLE pulls ADD COLUMN seat TEXT;

UPDATE pulls SET seat = seats WHERE seats IS NOT NULL AND trim(seats) <> '';

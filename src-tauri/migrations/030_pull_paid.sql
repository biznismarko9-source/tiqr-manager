-- 030: a pull's SECOND checkbox - has the person paid? (2.31.0)
--
-- Marko: "do pulls taktiez mali by byt 2 veci dane co sa daju checknut a to je
-- payment ci uz zaplatili a + transfer ale ten tam uz je."
--
-- A pull is two obligations running in parallel and they finish in either
-- order: marko transfers the tickets, and the buyer pays marko's fee
-- (`price_cents` - never the ticket price, which is on the buyer's own card).
-- `transfer_done` has tracked the first since 005. This adds the second,
-- deliberately as its own flag rather than as a second meaning for
-- `transfer_done`, because "transferred but not paid" is the state marko
-- actually needs to see.
--
-- Shaped exactly like `transfer_done`/`transfer_done_at` right down to the
-- CHECK and the index, so the two columns behave identically everywhere and
-- `commands/pulls.rs` can apply one timestamp rule to both.
BEGIN TRANSACTION;

ALTER TABLE pulls ADD COLUMN paid INTEGER NOT NULL DEFAULT 0 CHECK (paid IN (0,1));

-- Auto-stamped the moment `paid` flips 0->1, cleared back to NULL if it is
-- ever flipped back. Not user-editable, same as `transfer_done_at`.
ALTER TABLE pulls ADD COLUMN paid_at TEXT;

CREATE INDEX IF NOT EXISTS idx_pulls_paid ON pulls(paid);

COMMIT;

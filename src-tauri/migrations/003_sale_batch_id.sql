-- 003_sale_batch_id
-- Sales stay one-row-per-ticket forever (sales.ticket_id UNIQUE from 001 is
-- the database-level guarantee a ticket can never be sold twice) - this
-- migration does NOT change that. It only adds a way to recognise which
-- rows were submitted together as one "New sale" action covering several
-- tickets, so the UI can group them into a single sale transaction without
-- inventing a second, redundant "sale" record.
--
-- NULL for an ordinary single-ticket sale. For a multi-ticket batch, every
-- row in that batch shares the same batch_id, set to the first ticket's own
-- sale code (e.g. 'SAL-000045') - no new counter/identifier scheme needed,
-- since codes are already unique and sequential within a batch.
ALTER TABLE sales ADD COLUMN batch_id TEXT;
CREATE INDEX IF NOT EXISTS idx_sales_batch ON sales(batch_id);

-- 002_refunds
-- A refund is never a delete: the sale row stays forever (history), only its
-- payment_status flips to 'refunded' (already a valid value since 001). These
-- two columns record when and why. Both nullable - only set by the dedicated
-- refund_sale command, never by a plain sale edit.

ALTER TABLE sales ADD COLUMN refunded_at TEXT;
ALTER TABLE sales ADD COLUMN refund_reason TEXT;

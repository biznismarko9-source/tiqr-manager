-- TIQR Manager - 013_notifications
-- 2.0.76: backs the new outbound notification feature (desktop/email/
-- Pushover - see commands/notifications.rs). Same shape/spirit as
-- schema_migrations itself: a tiny log table whose entire job is enforcing
-- one business rule at the database layer, the same technique
-- 004_sales_active_unique.sql already used for "a ticket can never have two
-- simultaneously active sales".
--
-- The rule here: at most ONE notification per category per calendar day,
-- regardless of how many times the periodic check runs that day. `sent_on`
-- is a plain 'YYYY-MM-DD' string derived from chrono::Local (never UTC) -
-- the same local-date convention todayIso() (frontend) and dashboard.rs
-- already use everywhere else, so this table's idea of "today" never
-- disagrees with the Dashboard's own badges near a timezone boundary.
--
-- `category` is a small fixed set of string keys owned by
-- commands::notifications::NotificationCategory (e.g. "unpaid_orders") -
-- deliberately TEXT, not a foreign key to anything, since these categories
-- are Dashboard-alert concepts, not rows in another table.

CREATE TABLE IF NOT EXISTS notification_log (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  category   TEXT NOT NULL,
  sent_on    TEXT NOT NULL,
  sent_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  UNIQUE(category, sent_on)
);

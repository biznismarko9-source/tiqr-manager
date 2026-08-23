-- TIQR Manager - 012_event_categories
-- 2.0.27: marko wants to filter Events/Orders/Sales by event category
-- (football, concert, etc.) and see it color-coded. `events.category` has
-- existed as a plain free-text column since 001_initial_schema, and the
-- frontend already offered a fixed list of 6 options (Concert, Sports,
-- Theatre / Musical, Festival, Comedy, Motorsport) plus a free-text
-- "Other..." escape hatch (Events.tsx's CATEGORY_OPTIONS) - but it was never
-- a real lookup table, so it couldn't be filtered on or reliably color-coded
-- (two events both typed as "Concert" are the same string today, but nothing
-- stopped "concert"/"Concert "/etc from silently drifting apart either).
--
-- This adds a real lookup table - same shape/spirit as `platforms` and
-- `suppliers` (see 001_initial_schema.sql) - plus a `color_slot` column: an
-- integer index into a fixed, ordered categorical color palette the frontend
-- owns (see EventCategoryBadge.tsx), assigned once at creation time and never
-- recomputed from sort order, so a category's color never shifts just
-- because another category was added/removed/renamed. Not a hex string on
-- purpose: keeping the actual color values in the frontend (and only an
-- index here) means the palette itself can be refined later without a
-- migration.
--
-- `events.category` (the old free-text column) is deliberately left in
-- place, exactly the precedent 006_pulls_seat_fields.sql set for `pulls.
-- seats` - the app's own convention is additive-only, never DROP a shipped
-- column. It keeps being written going forward too (kept in sync with
-- category_id's name by commands::events), so anything that still reads it
-- directly (csv_export.rs's Events export) keeps working unmodified and
-- never goes stale.
--
-- events.category_id is nullable (an event can have no category, same as
-- today) and ON DELETE SET NULL (same convention as orders.platform_id/
-- orders.supplier_id in 001_initial_schema.sql) - deleting a category must
-- never block or cascade-delete the events that used it, it should just
-- leave them uncategorized. commands::event_categories::delete_event_category
-- additionally clears the matching events.category text in the same
-- transaction, so the old free-text mirror never goes stale either (the DB's
-- own ON DELETE SET NULL only ever touches category_id, not category).

CREATE TABLE IF NOT EXISTS event_categories (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  name        TEXT NOT NULL UNIQUE,
  color_slot  INTEGER NOT NULL DEFAULT 0,
  is_demo     INTEGER NOT NULL DEFAULT 0,
  created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

-- Seed the 6 options the frontend already offered, in their existing order,
-- so every event already using one of these exact names backfills cleanly
-- below instead of accidentally minting a duplicate "custom" category.
INSERT INTO event_categories (name, color_slot) VALUES
  ('Concert', 0),
  ('Sports', 1),
  ('Theatre / Musical', 2),
  ('Festival', 3),
  ('Comedy', 4),
  ('Motorsport', 5);

ALTER TABLE events ADD COLUMN category_id INTEGER REFERENCES event_categories(id) ON DELETE SET NULL;

-- Carry over any custom "Other..." value marko already typed (free text that
-- doesn't match one of the 6 seeded names above) as its own new category, so
-- nothing already saved is silently lost or reset to "no category". Slots
-- continue right after the 6 seeded ones (6, 7, 8, ...) in alphabetical
-- order of the value, so the very next brand-new category created from the
-- app afterwards continues the sequence from wherever this leaves off.
INSERT INTO event_categories (name, color_slot)
SELECT name, 5 + ROW_NUMBER() OVER (ORDER BY name)
FROM (
  SELECT DISTINCT trim(category) AS name
  FROM events
  WHERE category IS NOT NULL AND trim(category) <> ''
    AND trim(category) NOT IN (SELECT name FROM event_categories)
);

-- Point every event with a non-blank category at the matching lookup row
-- (guaranteed to exist now, either seeded above or just backfilled).
UPDATE events
SET category_id = (SELECT id FROM event_categories WHERE event_categories.name = trim(events.category))
WHERE category IS NOT NULL AND trim(category) <> '';

CREATE INDEX IF NOT EXISTS idx_events_category_id ON events(category_id);

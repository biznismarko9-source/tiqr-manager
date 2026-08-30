-- TIQR Manager - 015_finance
-- 2.0.83: marko's own request - "chcel by som mat vacsi prehlad o mojich
-- financiach, chcel by som mat moznost si vediet zapisat vydavky, kde boli
-- peniaze minute, kedy, kolko, kolko som prijal atd" (a daily/monthly/yearly
-- overview of his own money - record expenses/income: where, when, how much).
-- Confirmed as a new section INSIDE TIQR Manager (not a separate app), and
-- deliberately covering BOTH his personal money AND his ticket business's
-- money ("aj aj") in one place, even though Orders/Sales already track the
-- business side on their own - see finance_entries.scope's comment below for
-- how these two stay side by side without double-counting anything.
--
-- Two tables, same shape/spirit as everything else in this app:
--
-- `finance_categories` is a lookup list combining two patterns already used
-- elsewhere - `kind` (this category applies to expense/income/both) is
-- exactly `platforms.kind`'s own convention (001_initial_schema.sql), and
-- `color_slot` is exactly `event_categories.color_slot`'s own convention
-- (012_event_categories.sql: a fixed palette index assigned once at creation
-- via MAX(color_slot)+1, never recomputed - see FinanceCategoryBadge.tsx).
-- Seeded below with a starter set marko asked for ("navrhni ty" - you
-- propose) - same as every other lookup list in this app, fully
-- editable/deletable afterward from Settings -> Lookups, nothing hardcoded.
--
-- `finance_entries` is the actual ledger - one row per thing marko typed in
-- (manual entry only, no bank connection - his own answer #4). Money is
-- INTEGER cents, same rule as every other table (see 001_initial_schema.sql's
-- header comment). `currency` defaults EUR (his own answer #3 - "hlavne
-- euro"); a non-EUR entry can be converted to EUR afterward the exact same
-- way a non-EUR Order already can (commands::currency::convert_currency,
-- unchanged, reused as-is - see Finance.tsx's mixed-currency banner) rather
-- than any new conversion logic here.

CREATE TABLE IF NOT EXISTS finance_categories (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  name        TEXT NOT NULL UNIQUE,
  kind        TEXT NOT NULL DEFAULT 'both' CHECK (kind IN ('expense','income','both')),
  color_slot  INTEGER NOT NULL DEFAULT 0,
  is_demo     INTEGER NOT NULL DEFAULT 0,
  created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

-- Starter categories - expenses first (slots 0-7), then income (slots 8-10),
-- continuing the same single incrementing color_slot sequence
-- create_finance_category_impl uses for anything added later (so a brand new
-- category never repeats a seeded one's color while any seeded slot is still
-- unused). Names in Slovak, same as marko's own examples in the original
-- proposal (navrh-osobne-financie.md's "Jedlo, Bývanie, Doprava..." list) -
-- unlike Platforms/Event categories (English brand names / ticketing terms),
-- these are categories marko reads and picks from constantly for his own
-- day-to-day money, in his own language. Every one of these is a plain
-- editable row like any other lookup list entry - rename, delete or add to
-- freely from Settings -> Lookups, nothing here is fixed.
INSERT INTO finance_categories (name, kind, color_slot) VALUES
  ('Jedlo a nákupy', 'expense', 0),
  ('Bývanie',        'expense', 1),
  ('Doprava',        'expense', 2),
  ('Zábava',         'expense', 3),
  ('Zdravie',        'expense', 4),
  ('Predplatné',     'expense', 5),
  ('Biznis náklady', 'expense', 6),
  ('Iné výdavky',    'expense', 7),
  ('Výplata',        'income',  8),
  ('Biznis príjem',  'income',  9),
  ('Iné príjmy',     'income', 10);

CREATE TABLE IF NOT EXISTS finance_entries (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  entry_type   TEXT NOT NULL CHECK (entry_type IN ('income','expense')),
  entry_date   TEXT NOT NULL, -- ISO 8601 date, same convention as orders.purchase_date/sales.sale_date
  amount_cents INTEGER NOT NULL CHECK (amount_cents >= 0), -- sign comes from entry_type, same rule as sales.sale_price_cents etc.
  currency     TEXT NOT NULL DEFAULT 'EUR',
  -- 'personal' vs 'business' (marko's own answer #2 - "aj aj", both belong in
  -- this one ledger). Deliberately just a label on an otherwise-identical
  -- entry, NOT a link into orders/tickets/sales - this ledger is fully
  -- independent of that data, by design: it's manual entry only (answer #4),
  -- so a business entry here is something marko typed in himself, exactly
  -- like everything else in this table, never an automatic mirror of an
  -- Order/Sale. That keeps this migration simple and completely safe against
  -- ever double-counting or drifting out of sync with the existing
  -- Orders/Sales figures the Dashboard already shows - the two are
  -- deliberately independent views, same as a person's own bank ledger is
  -- independent of a separate business accounting book even when one person
  -- keeps both.
  scope        TEXT NOT NULL DEFAULT 'personal' CHECK (scope IN ('personal','business')),
  category_id  INTEGER REFERENCES finance_categories(id) ON DELETE SET NULL,
  place        TEXT, -- "kde" (where/who - e.g. a shop, a client, "Tesco", "Ján")
  note         TEXT,
  is_demo      INTEGER NOT NULL DEFAULT 0,
  created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE INDEX IF NOT EXISTS idx_finance_entries_date ON finance_entries(entry_date);
CREATE INDEX IF NOT EXISTS idx_finance_entries_category ON finance_entries(category_id);
CREATE INDEX IF NOT EXISTS idx_finance_entries_type ON finance_entries(entry_type);
CREATE INDEX IF NOT EXISTS idx_finance_entries_scope ON finance_entries(scope);
CREATE INDEX IF NOT EXISTS idx_finance_entries_is_demo ON finance_entries(is_demo);

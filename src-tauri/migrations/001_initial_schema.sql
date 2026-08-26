-- TIQR Manager - 001_initial_schema
-- Money is ALWAYS stored as INTEGER cents (never REAL/float). Example: EUR 12.34 -> 1234.

CREATE TABLE IF NOT EXISTS app_settings (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS counters (
  name  TEXT PRIMARY KEY,
  value INTEGER NOT NULL DEFAULT 0
);
INSERT OR IGNORE INTO counters(name, value) VALUES ('order', 0);
INSERT OR IGNORE INTO counters(name, value) VALUES ('ticket', 0);
INSERT OR IGNORE INTO counters(name, value) VALUES ('sale', 0);

CREATE TABLE IF NOT EXISTS platforms (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  name       TEXT NOT NULL UNIQUE,
  kind       TEXT NOT NULL DEFAULT 'both' CHECK (kind IN ('purchase','sale','both')),
  is_demo    INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TABLE IF NOT EXISTS suppliers (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  name       TEXT NOT NULL UNIQUE,
  contact    TEXT,
  is_demo    INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TABLE IF NOT EXISTS events (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  name        TEXT NOT NULL,
  artist_team TEXT,
  venue       TEXT,
  city        TEXT,
  country     TEXT,
  event_date  TEXT, -- ISO 8601 date, nullable (TBD events)
  category    TEXT,
  status      TEXT NOT NULL DEFAULT 'upcoming' CHECK (status IN ('upcoming','completed','cancelled')),
  notes       TEXT,
  is_demo     INTEGER NOT NULL DEFAULT 0,
  created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE INDEX IF NOT EXISTS idx_events_date ON events(event_date);
CREATE INDEX IF NOT EXISTS idx_events_status ON events(status);
CREATE INDEX IF NOT EXISTS idx_events_is_demo ON events(is_demo);

CREATE TABLE IF NOT EXISTS orders (
  id                INTEGER PRIMARY KEY AUTOINCREMENT,
  code              TEXT NOT NULL UNIQUE,
  event_id          INTEGER NOT NULL REFERENCES events(id) ON DELETE RESTRICT,
  supplier_id       INTEGER REFERENCES suppliers(id) ON DELETE SET NULL,
  platform_id       INTEGER REFERENCES platforms(id) ON DELETE SET NULL,
  purchase_date     TEXT NOT NULL,
  quantity          INTEGER NOT NULL CHECK (quantity > 0),
  unit_price_cents  INTEGER NOT NULL DEFAULT 0 CHECK (unit_price_cents >= 0),
  fees_cents        INTEGER NOT NULL DEFAULT 0 CHECK (fees_cents >= 0),
  other_costs_cents INTEGER NOT NULL DEFAULT 0 CHECK (other_costs_cents >= 0),
  total_cost_cents  INTEGER NOT NULL DEFAULT 0 CHECK (total_cost_cents >= 0),
  currency          TEXT NOT NULL DEFAULT 'EUR',
  payment_status    TEXT NOT NULL DEFAULT 'unpaid' CHECK (payment_status IN ('unpaid','partial','paid')),
  notes             TEXT,
  is_demo           INTEGER NOT NULL DEFAULT 0,
  created_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE INDEX IF NOT EXISTS idx_orders_event ON orders(event_id);
CREATE INDEX IF NOT EXISTS idx_orders_supplier ON orders(supplier_id);
CREATE INDEX IF NOT EXISTS idx_orders_platform ON orders(platform_id);
CREATE INDEX IF NOT EXISTS idx_orders_date ON orders(purchase_date);
CREATE INDEX IF NOT EXISTS idx_orders_is_demo ON orders(is_demo);

CREATE TABLE IF NOT EXISTS tickets (
  id                   INTEGER PRIMARY KEY AUTOINCREMENT,
  code                 TEXT NOT NULL UNIQUE,
  event_id             INTEGER NOT NULL REFERENCES events(id) ON DELETE RESTRICT,
  order_id             INTEGER NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
  section              TEXT,
  row_label            TEXT,
  seat                 TEXT,
  ticket_type          TEXT,
  purchase_cost_cents  INTEGER NOT NULL DEFAULT 0 CHECK (purchase_cost_cents >= 0),
  purchase_fees_cents  INTEGER NOT NULL DEFAULT 0 CHECK (purchase_fees_cents >= 0),
  other_costs_cents    INTEGER NOT NULL DEFAULT 0 CHECK (other_costs_cents >= 0),
  listing_price_cents  INTEGER CHECK (listing_price_cents IS NULL OR listing_price_cents >= 0),
  currency             TEXT NOT NULL DEFAULT 'EUR',
  status               TEXT NOT NULL DEFAULT 'available' CHECK (status IN ('available','listed','sold','cancelled')),
  notes                TEXT,
  is_demo              INTEGER NOT NULL DEFAULT 0,
  created_at           TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at           TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE INDEX IF NOT EXISTS idx_tickets_event ON tickets(event_id);
CREATE INDEX IF NOT EXISTS idx_tickets_order ON tickets(order_id);
CREATE INDEX IF NOT EXISTS idx_tickets_status ON tickets(status);
CREATE INDEX IF NOT EXISTS idx_tickets_is_demo ON tickets(is_demo);

CREATE TABLE IF NOT EXISTS sales (
  id                  INTEGER PRIMARY KEY AUTOINCREMENT,
  code                TEXT NOT NULL UNIQUE,
  ticket_id           INTEGER NOT NULL UNIQUE REFERENCES tickets(id) ON DELETE RESTRICT,
  platform_id         INTEGER REFERENCES platforms(id) ON DELETE SET NULL,
  sale_date           TEXT NOT NULL,
  sale_price_cents    INTEGER NOT NULL DEFAULT 0 CHECK (sale_price_cents >= 0),
  selling_fees_cents  INTEGER NOT NULL DEFAULT 0 CHECK (selling_fees_cents >= 0),
  currency            TEXT NOT NULL DEFAULT 'EUR',
  payment_status      TEXT NOT NULL DEFAULT 'pending' CHECK (payment_status IN ('pending','paid','refunded')),
  buyer_reference     TEXT,
  notes               TEXT,
  is_demo             INTEGER NOT NULL DEFAULT 0,
  created_at          TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at          TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
-- ticket_id UNIQUE is the database-level guarantee that a ticket can never be sold twice.
CREATE INDEX IF NOT EXISTS idx_sales_platform ON sales(platform_id);
CREATE INDEX IF NOT EXISTS idx_sales_date ON sales(sale_date);
CREATE INDEX IF NOT EXISTS idx_sales_is_demo ON sales(is_demo);

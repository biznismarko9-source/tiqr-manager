-- TIQR Manager - 016_finance_v2
-- 2.1.0: marko's own request - expand Finance with Accounts/Wallets,
-- Transfers between accounts, Recurring Expenses, and a Cashflow Forecast
-- (see FINANCE-2.1.0-REPORT.md for the full spec/rationale). Purely
-- additive on top of 015_finance.sql - no existing table is changed in a
-- breaking way, and every existing finance_categories/finance_entries row
-- and calculation keeps working exactly as before.
--
-- Three new tables + one additive column, same "INTEGER cents, is_demo,
-- created_at" conventions as every other table in this app:
--
-- `accounts` - marko's own wallets (Bank/Revolut/PayPal/Cash/Credit
-- card/Other, or any custom name he types - `account_type` only picks the
-- icon/preset, `name` is always free text). `opening_balance_cents` is the
-- only balance value ever STORED - the CURRENT balance is always computed
-- fresh from this account's own finance_entries + transfers in one single
-- aggregate query (see commands::finance_accounts::list_accounts), same
-- "never cache a derived number" spirit as finance.rs's own P&L and
-- dashboard.rs's own aggregates.
--
-- `transfers` - a transfer is fundamentally different from an income/expense
-- entry (it moves marko's own money between two of his own accounts - never
-- a P&L event), so it gets its own dedicated table rather than a third
-- finance_entries.entry_type. This also makes "a transfer is never counted
-- as income/expense" true by construction - finance_entries' own
-- income/expense sums (and every existing Finance calculation) simply never
-- scan this table, rather than something every future query has to
-- remember to filter out. `currency` is always derived server-side from
-- from_account/to_account (which must already share one currency - v1 does
-- not support cross-currency transfers, marko's own preferred "simpler,
-- safer" option), never trusted from client input - see
-- commands::finance_accounts::create_transfer_impl.
--
-- `recurring_expenses` - a scheduled TEMPLATE, not a transaction. Creating
-- the actual finance_entries row only ever happens through an explicit user
-- action (commands::finance_recurring::create_from_recurring_impl) - see
-- that module's doc comment for why this is what makes "never create a
-- duplicate transaction on repeated app opens" true by construction
-- (nothing runs automatically on app open at all).
--
-- `finance_entries.account_id` - additive, nullable FK. Every entry that
-- existed before this migration gets NULL here automatically (plain SQLite
-- ADD COLUMN with no default), and every existing Income/Expense flow keeps
-- working exactly as before with no account picked - "Account" is an
-- OPTIONAL field on an entry, same as `category_id` already is.
--
-- FK delete behaviour (all enforced - this app runs with
-- `PRAGMA foreign_keys = ON` on every connection, see db.rs::open_connection
-- and db.rs::test_conn):
--   - finance_entries.account_id    -> ON DELETE SET NULL (same convention
--     as category_id - deleting an account only detaches old entries, never
--     deletes marko's own manually-typed transaction history)
--   - recurring_expenses.account_id -> ON DELETE SET NULL (same reasoning -
--     the template survives, just loses its suggested account)
--   - transfers.from_account_id / to_account_id -> ON DELETE RESTRICT (a
--     transfer with a missing side would be a broken, meaningless record
--     that would silently corrupt the balance aggregate - so an account
--     still referenced by any transfer simply cannot be deleted; the
--     app-level `delete_account` command checks this first and returns a
--     clear validation error before ever reaching the database - see
--     finance_accounts.rs)

CREATE TABLE IF NOT EXISTS accounts (
  id                    INTEGER PRIMARY KEY AUTOINCREMENT,
  name                  TEXT NOT NULL,
  -- Preset used for icon/label only, never restricts what `name` can say -
  -- 'bank' | 'revolut' | 'paypal' | 'cash' | 'credit_card' | 'other'.
  account_type          TEXT NOT NULL DEFAULT 'other'
                           CHECK (account_type IN ('bank','revolut','paypal','cash','credit_card','other')),
  currency              TEXT NOT NULL DEFAULT 'EUR',
  opening_balance_cents INTEGER NOT NULL DEFAULT 0,
  is_active             INTEGER NOT NULL DEFAULT 1,
  is_demo               INTEGER NOT NULL DEFAULT 0,
  created_at            TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at            TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE INDEX IF NOT EXISTS idx_accounts_is_active ON accounts(is_active);

CREATE TABLE IF NOT EXISTS transfers (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  transfer_date   TEXT NOT NULL,
  from_account_id INTEGER NOT NULL REFERENCES accounts(id) ON DELETE RESTRICT,
  to_account_id   INTEGER NOT NULL REFERENCES accounts(id) ON DELETE RESTRICT,
  amount_cents    INTEGER NOT NULL CHECK (amount_cents > 0),
  -- Always equal to from_account.currency/to_account.currency - see this
  -- file's own header comment. Stored (not just derived via JOIN at read
  -- time) purely for cheap display, same denormalization spirit as
  -- finance_entries.category_name.
  currency        TEXT NOT NULL,
  note            TEXT,
  is_demo         INTEGER NOT NULL DEFAULT 0,
  created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  CHECK (from_account_id <> to_account_id)
);
CREATE INDEX IF NOT EXISTS idx_transfers_date ON transfers(transfer_date);
CREATE INDEX IF NOT EXISTS idx_transfers_from_account ON transfers(from_account_id);
CREATE INDEX IF NOT EXISTS idx_transfers_to_account ON transfers(to_account_id);

CREATE TABLE IF NOT EXISTS recurring_expenses (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  name         TEXT NOT NULL,
  amount_cents INTEGER NOT NULL CHECK (amount_cents > 0),
  currency     TEXT NOT NULL DEFAULT 'EUR',
  scope        TEXT NOT NULL DEFAULT 'business' CHECK (scope IN ('personal','business')),
  category_id  INTEGER REFERENCES finance_categories(id) ON DELETE SET NULL,
  account_id   INTEGER REFERENCES accounts(id) ON DELETE SET NULL,
  frequency    TEXT NOT NULL CHECK (frequency IN ('weekly','monthly','quarterly','yearly')),
  start_date   TEXT NOT NULL,
  -- The next occurrence this template will produce. Only ever advanced by
  -- an explicit Create/Skip action (commands::finance_recurring) - never by
  -- a background job or on app startup, so a paused/forgotten template just
  -- shows as "overdue" (next_date in the past) rather than silently
  -- generating anything. See finance_recurring.rs's module doc comment.
  next_date    TEXT NOT NULL,
  is_active    INTEGER NOT NULL DEFAULT 1,
  note         TEXT,
  is_demo      INTEGER NOT NULL DEFAULT 0,
  created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE INDEX IF NOT EXISTS idx_recurring_expenses_next_date ON recurring_expenses(next_date);
CREATE INDEX IF NOT EXISTS idx_recurring_expenses_is_active ON recurring_expenses(is_active);
CREATE INDEX IF NOT EXISTS idx_recurring_expenses_account ON recurring_expenses(account_id);

-- Additive, nullable - every row that already exists gets NULL here, and
-- every existing Income/Expense flow keeps working with no account picked
-- (see finance_entries.rs's own doc comment for the full "Account" flow).
ALTER TABLE finance_entries ADD COLUMN account_id INTEGER REFERENCES accounts(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS idx_finance_entries_account ON finance_entries(account_id);

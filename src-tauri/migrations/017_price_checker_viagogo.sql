-- TIQR Manager - 017_price_checker_viagogo
-- 2.1.6: marko's request - "je sposob, akym to vieme opravit na vsetkych 3
-- platformach [...] a stubhub by som chcel zmenit za viagogo.com" (replace
-- StubHub with Viagogo). Confirmed via AskUserQuestion before building:
-- keep StubHub's existing history fully readable, but stop offering it for
-- brand-new links/checks; Viagogo becomes a real, active marketplace from
-- here on, same footing as Vivid Seats/Ticombo.
--
-- `active` distinguishes "marko can start something NEW against this
-- marketplace" from "historical only, still fully readable" - see
-- commands::price_checker::get_price_checker_summary_impl's own doc comment
-- for exactly how this flag changes which marketplaces a given event's page
-- shows. Nothing about event_marketplace_links/price_checks changes at all -
-- StubHub's row, its id, and every row that references it are untouched, so
-- its full history survives exactly as marko asked; only NEW events without
-- any existing StubHub link/check stop being offered it.
--
-- Wrapped in an explicit transaction (post-review hardening, added before
-- first release): db::run_migrations applies each migration file via a
-- plain `execute_batch` with no transaction of its own (see that function's
-- own doc comment), so without BEGIN/COMMIT below, the 4 statements here
-- would each autocommit individually. That mattered concretely: the INSERT
-- further down used to be a plain INSERT, and if it had ever hit a
-- pre-existing 'Viagogo' row (nothing in the shipped UI can create one
-- today, but create_marketplace is a real, reachable command - see
-- price_checker.rs) it would have failed with a UNIQUE violation AFTER the
-- ALTER TABLE/UPDATE below had already committed - and because this
-- migration would then never reach run_migrations' own "mark as applied"
-- step, EVERY later launch would retry this exact file from the top, where
-- the ALTER TABLE would now ALSO fail ("duplicate column name: active")
-- against the already-half-applied schema. That's an unrecoverable
-- app-never-starts-again state with no in-app fix. OR IGNORE on that INSERT
-- (below) closes the specific known trigger; BEGIN/COMMIT here means ANY
-- other failure in this file rolls everything back together instead,
-- leaving a clean, retryable state either way (SQLite fully supports DDL
-- inside a transaction, so this is safe). Local to this one file - not a
-- change to how the other 16 migrations run.
BEGIN TRANSACTION;

ALTER TABLE marketplaces ADD COLUMN active INTEGER NOT NULL DEFAULT 1;

-- StubHub retired from new checks - see this file's own doc comment above.
UPDATE marketplaces SET active = 0 WHERE name = 'StubHub';

-- Seeded the same way 014_price_checker.sql seeded the original 3 - marko
-- manages this list himself from here on (list/create/delete already exist,
-- unchanged), same as every other marketplace. Defaults to active = 1 (the
-- column's own DEFAULT above), exactly what a brand-new marketplace needs.
-- OR IGNORE (not a plain INSERT) - see this file's own opening note on why.
INSERT OR IGNORE INTO marketplaces (name) VALUES ('Viagogo');

-- Holds the one secret this app has ever needed to store (marko's own
-- Anthropic API key, for the new AI-assisted extraction fallback - see
-- commands::price_checker_auto's module doc comment, "AI-assisted
-- extraction fallback (2.1.6)"). Deliberately its OWN table, never the
-- existing generic `app_settings` KV store (commands::settings) even though
-- the shape looks identical: `app_settings` is read back to the frontend
-- VERBATIM by its own generic get_app_setting command (already used for
-- things like the remembered dashboard tab, useListTab.ts), and a real
-- secret must never be reachable that way. The only two commands that ever
-- touch this table are get_anthropic_api_key_configured/set_anthropic_api_key
-- (commands::settings) - neither one ever returns the actual stored value to
-- the frontend, only whether a key is currently set, same "presence flag,
-- never the value" convention Settings.tsx's ntfy topic field already uses.
CREATE TABLE IF NOT EXISTS app_secrets (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

COMMIT;

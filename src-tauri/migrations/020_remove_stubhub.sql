-- TIQR Manager - 020_remove_stubhub
-- 2.2.0: marko's explicit follow-up on top of the Market Analysis spec -
-- "taktiez z price checkera kompletne vymazat stubhub" (also completely
-- remove StubHub from Price Checker). Confirmed via AskUserQuestion before
-- writing this: he wants the OLD history gone too, not just hidden - a
-- deliberate step further than 2.1.6 (migrations/017_price_checker_
-- viagogo.sql), which retired StubHub from NEW checks but explicitly kept
-- its existing history fully readable, per his OWN request at the time.
-- That earlier choice is not being second-guessed here - marko was asked
-- again, directly, and chose full removal this time.
--
-- Deletes child rows before the parent `marketplaces` row, explicitly and
-- in the correct order, rather than relying on `ON DELETE CASCADE` alone -
-- `db::open_connection` does enable `PRAGMA foreign_keys = ON` before
-- `run_migrations` ever runs, so cascade WOULD fire correctly on its own,
-- but being explicit here costs nothing and doesn't depend on that ordering
-- staying true. Every statement is a plain DELETE keyed off a subquery, so
-- this is safe to run whether or not a StubHub row (or any of its history)
-- actually exists - matches this migration runner's own "each file runs at
-- most once, but must not assume anything about what it finds" spirit.
--
-- Wrapped in BEGIN/COMMIT for the exact reason documented in
-- 017_price_checker_viagogo.sql's own opening comment: `run_migrations`
-- applies each file via a plain `execute_batch` with no transaction of its
-- own, so without this wrapper the 4 statements below would each autocommit
-- individually - an interruption partway through would leave, say,
-- price_checks deleted but the marketplaces row itself still present. Every
-- statement here is independently idempotent regardless, but atomicity is
-- still the right default for an irreversible multi-statement delete.
BEGIN TRANSACTION;

DELETE FROM price_check_tiers
 WHERE price_check_id IN (
   SELECT id FROM price_checks
    WHERE marketplace_id = (SELECT id FROM marketplaces WHERE name = 'StubHub')
 );

DELETE FROM price_checks
 WHERE marketplace_id = (SELECT id FROM marketplaces WHERE name = 'StubHub');

DELETE FROM event_marketplace_links
 WHERE marketplace_id = (SELECT id FROM marketplaces WHERE name = 'StubHub');

DELETE FROM marketplaces WHERE name = 'StubHub';

COMMIT;

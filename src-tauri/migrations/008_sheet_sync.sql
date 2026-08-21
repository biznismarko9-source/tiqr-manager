-- Google Sheets sync (Settings -> Integrations). Purely additive - touches
-- no existing table, and every column here is only ever written/read by the
-- new sync commands (see commands/sheets_sync.rs), never by any of the
-- app's existing financial/inventory logic.
--
-- Deliberately data-source-agnostic (`data_source` is a plain string, e.g.
-- 'pulls' today) rather than a `pull_id` foreign key, so a second connected
-- data source (Tickets/Orders, planned next) never needs its own migration
-- or its own copy of this table.
--
-- `sheet_marker` is the value the app writes into a dedicated column it
-- appends to the user's sheet (never reusing an existing column - some
-- existing sheets, e.g. marko's real Pulls tracker, already use their first
-- column for something else entirely) so a repeat sync can match a sheet
-- row back to the exact local record it already knows about, without ever
-- creating duplicates.
--
-- `last_synced_snapshot` is a JSON object of the field values as they stood
-- immediately after the last successful sync, for exactly one purpose:
-- telling "only the app changed this field since last time" apart from
-- "only the sheet changed it" apart from "both changed it" (a genuine
-- conflict, which sync must surface for a person to resolve - see
-- commands/sheets_sync.rs - never silently guess a winner).
CREATE TABLE IF NOT EXISTS sheet_sync_links (
    data_source TEXT NOT NULL,
    local_id INTEGER NOT NULL,
    sheet_marker TEXT NOT NULL,
    last_synced_snapshot TEXT NOT NULL,
    last_synced_at TEXT NOT NULL,
    PRIMARY KEY (data_source, local_id)
);

CREATE INDEX IF NOT EXISTS idx_sheet_sync_links_marker
    ON sheet_sync_links (data_source, sheet_marker);

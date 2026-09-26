-- 034: Sheet formatting - cell colour, row height, column width.
--
-- WHY THIS EXISTS
--
-- marko: "vies si zvacsit zmensit riadok, jeho velkost, farba textu, proste
-- nech je to viac komplxnejsie". The grid was a place to put text; this makes
-- it a place to see the text, which for a sheet full of codes and statuses is
-- most of the value.
--
-- WHY FORMATS LIVE ON THE ROW, POSITIONALLY, LIKE CELLS
--
-- `note_rows.formats_json` is a JSON array of strings aligned by INDEX with
-- that row's `cells_json`, exactly like the cells are aligned with the
-- sheet's `columns_json`. See PROTECTED_AREAS, "a note row's cells are
-- POSITIONAL".
--
-- The alternative - a `cell_formats` table keyed by (row_id, col_index), or a
-- map on the sheet - would drift the moment a column is added, deleted or
-- moved, because nothing would carry the format along with the cell. Storing
-- it in the same shape means `reshape_rows` moves BOTH arrays through the same
-- transformation in the same transaction, so a format cannot end up on a
-- different cell than the text it was applied to. That is the whole reason for
-- the shape, and it is why `reshape_rows` now takes and returns a pair.
--
-- WHAT A FORMAT STRING IS
--
-- A short flag string, not an object: one optional colour letter plus optional
-- style letters. "" is plain.
--
--     r  red      o  amber     g  green
--     b  blue     p  purple    m  muted grey
--     B  bold     I  italic
--
-- So "rB" is bold red. Compact enough that a full sheet of them is still a
-- small blob, and `fit()`-able with no extra code because it is just strings.
-- An unknown letter is ignored by the UI rather than rejected here - a newer
-- version writing a flag this one does not know must not make the cell
-- unreadable on the other machine.
--
-- SIZES
--
-- `note_rows.height` is a per-row override in px; 0 means "use the sheet's".
-- `note_sheets.row_height` is that sheet default. `note_sheets.widths_json` is
-- a JSON array of per-column widths in px, aligned with `columns_json` and
-- reshaped with it; a missing or 0 entry means the default width.
--
-- SYNC
--
-- No new tables, so nothing new to wire: these are columns on two tables that
-- already carry `uid`, tombstones and MERGE_TABLES entries. Existing rows get
-- the defaults and read back exactly as before.

ALTER TABLE note_rows ADD COLUMN formats_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE note_rows ADD COLUMN height INTEGER NOT NULL DEFAULT 0;

ALTER TABLE note_sheets ADD COLUMN widths_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE note_sheets ADD COLUMN row_height INTEGER NOT NULL DEFAULT 24;

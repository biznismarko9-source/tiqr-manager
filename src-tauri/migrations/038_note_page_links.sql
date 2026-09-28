-- 038: Links belong to a SUB-TAB, and notes can be re-ordered by hand.
--
-- WHY THIS EXISTS
--
-- marko, using 2.58.0: "to priradenie musi fungovat osobitne v kazdej podkarte
-- zvlast nie spolu" and "mala by byt moznost hybat s poznamkamy".
--
-- WHY page_id AND note_id BOTH STAY
--
-- `note_id` is not redundant. It is what makes "every link on this note" one
-- index lookup instead of a join through `note_pages`, and it is the column
-- 036's MERGE_TABLES entry already declares. `page_id` is the new truth about
-- WHERE a link is shown; `note_id` stays the truth about WHICH NOTE owns it,
-- and the two can never disagree because only the backend writes them, always
-- together, from the page's own `note_id`.
--
-- Both cascade, so a deleted tab takes its links and a deleted note takes
-- everything. The tab's cascade is the one that matters day to day.
--
-- BACKFILL
--
-- Links made in 2.58.0 have no page. They are given the note's FIRST tab -
-- which is the only tab most of them will have, and the one whose editor they
-- were created from. Leaving them NULL would mean a link that belongs to no
-- tab and is therefore shown on none of them: silently invisible, which is
-- worse than being on the wrong tab where it can be moved.
--
-- WHY notes.position
--
-- Until now the list was pinned-first then most-recently-changed. That is a
-- good default and stays the tiebreaker, but it means a note marko wants at
-- the top sinks the moment he edits another one. `position` lets him put a
-- note where he wants it and have it stay there.

ALTER TABLE note_links ADD COLUMN page_id INTEGER REFERENCES note_pages(id) ON DELETE CASCADE;

UPDATE note_links
   SET page_id = (
       SELECT p.id FROM note_pages p
        WHERE p.note_id = note_links.note_id
        ORDER BY p.position, p.id
        LIMIT 1
   )
 WHERE page_id IS NULL;

ALTER TABLE notes ADD COLUMN position INTEGER NOT NULL DEFAULT 0;

-- Existing notes keep the order they are already shown in, so nothing jumps
-- the first time this runs: newest-changed first, pinned ones above.
UPDATE notes
   SET position = (
       SELECT COUNT(*) FROM notes AS earlier
        WHERE earlier.pinned > notes.pinned
           OR (earlier.pinned = notes.pinned AND earlier.updated_at > notes.updated_at)
           OR (earlier.pinned = notes.pinned AND earlier.updated_at = notes.updated_at AND earlier.id < notes.id)
   );

CREATE INDEX IF NOT EXISTS idx_note_links_page ON note_links(page_id);
CREATE INDEX IF NOT EXISTS idx_notes_order ON notes(archived, pinned, position);

-- 045: marko's own rules, and a sender list he can actually edit.
--
-- Two requests, both from the same afternoon:
--
--   "ze napriklad ktoru hlavicku rovno ma riesit alebo ktoru rovno dat do
--    presunutych alebo co a kedy do spamu atd, pohraj sa s tym"
--   "toto s moznostou upravit, odstranit pridat"  (of the sender chips)
--
-- ---------------------------------------------------------------------------
-- 1. RULES - "co a kedy"
-- ---------------------------------------------------------------------------
--
-- A rule is: look at ONE field, for ONE piece of text, and do ONE thing.
-- Deliberately that small. The moment a rule grows AND/OR, nobody can say
-- what it will do to a mailbox of ten thousand messages, and a rule nobody
-- can predict is worse than no rule - it quietly eats an order confirmation
-- and marko finds out a week later when the order is missing.
--
-- WHAT A RULE CANNOT DO, and why that is not a shortcut:
--
-- The scope this app holds is `gmail.readonly`. A rule cannot delete a
-- message in Gmail, cannot mark one read there, cannot move one to Gmail's
-- spam. EVERY action here is local to TIQR. The settings screen says so in
-- those words rather than letting a rule look like a Gmail filter that
-- silently failed.
--
-- The four actions, and the column each one actually writes:
--
--   trash   -> mail_messages.trashed_at   TIQR's trash. Still on disk, still
--                                         restorable. marko's standing rule
--                                         "vzdy najskor presunutie do kosa,
--                                         az potom zmazanie" holds: no rule
--                                         deletes anything, ever.
--   read    -> mail_messages.is_unread=0  The "stop shouting at me" action
--                                         for newsletters worth keeping.
--   review  -> status = review_required   Pushes it INTO Needs review, for
--                                         the senders he wants to see first.
--   keep    -> writes nothing at all      A guard. It stops the rule list
--                                         dead, so no later rule can trash
--                                         or mute that message.
--
-- `keep` is the one that makes the rest safe to use. Rules run top to bottom
-- and the first match wins, so a `keep` on "order confirmation" sitting above
-- a `trash` on "noreply@" means the broad rule can never eat the narrow case.
-- Without it, writing a useful trash rule means betting the mailbox on having
-- thought of every exception in advance.
CREATE TABLE IF NOT EXISTS mail_rules (
    id          TEXT PRIMARY KEY,
    -- Lower runs first. First match wins, so this IS the rule's meaning,
    -- not a display preference.
    position    INTEGER NOT NULL,
    enabled     INTEGER NOT NULL DEFAULT 1,
    -- 'from' | 'subject' | 'preview'
    field       TEXT NOT NULL,
    -- 'contains' | 'equals' | 'starts' | 'ends'
    match_kind  TEXT NOT NULL,
    -- Stored lower-cased; matching is lower-cased on both sides, because
    -- nobody types a rule expecting "NOREPLY@" to be a different sender
    -- from "noreply@".
    needle      TEXT NOT NULL,
    -- 'trash' | 'read' | 'review' | 'keep'
    action      TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    -- How many messages this rule has actually acted on, and when it last
    -- did. Shown next to the rule. A rule with 0 hits after a month is
    -- either wrong or pointless, and both are worth seeing - a rules screen
    -- that cannot tell you which rules do anything is a screen full of
    -- guesses.
    hits        INTEGER NOT NULL DEFAULT 0,
    last_hit_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_mail_rules_order ON mail_rules (enabled, position);

-- ---------------------------------------------------------------------------
-- 2. SENDERS - the chips, now editable
-- ---------------------------------------------------------------------------
--
-- Until now this list lived in two places that could not be reconciled: a
-- hard-coded array in commands/mail.rs that marko could not touch, and a
-- free-text "also watch these" box underneath it. So the seventeen he could
-- see were the seventeen he was stuck with, and the only way to disagree
-- with one was to add more.
--
-- One table, one list. `builtin` records where a row came from so the screen
-- can say "this one shipped with TIQR" - it grants no privilege: a built-in
-- row deletes exactly like any other, which is the point of the change.
--
-- The seed is in Rust (commands/mail.rs::seed_senders) rather than in INSERT
-- statements here, so the shipped list and the list used by classification
-- cannot drift apart into two different truths. It also runs ONCE: a domain
-- marko deleted must not reappear at the next launch, which is exactly what
-- `INSERT OR IGNORE` on every start would do.
CREATE TABLE IF NOT EXISTS mail_senders (
    domain   TEXT PRIMARY KEY,
    label    TEXT NOT NULL,
    builtin  INTEGER NOT NULL DEFAULT 0,
    added_at TEXT NOT NULL
);

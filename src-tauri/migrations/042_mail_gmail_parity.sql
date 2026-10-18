-- 042: closer to Gmail - starred mail, real folders, OTP codes, full sender.
--
-- marko, in his own words:
--   "chcem aby sa mi vsetky starred maily ukazovali a je jedno kolko ich je
--    a ake su stare"
--   "chcem aby tam tiez boli zalozky"
--   "chcem ked mi ukaze mail tak vediet ho rozkliknut a kuknut odkial ide
--    z akeho mailu"
--   "chcem mat policko kde budu chodit vyhradne len OTP kody"
--
-- Four requests, four pieces of storage.

-- ---------------------------------------------------------------------------
-- 1. STARRED, and why it needs its own column rather than reading `labels`
-- ---------------------------------------------------------------------------
--
-- `labels` already carries `starred`, so this column is redundant for
-- READING. It exists because of how the SYNC has to work.
--
-- Every sync so far has been a date window - "mail since the last run,
-- overlapping by a day". marko's requirement is the exact opposite: ALL
-- starred mail, "je jedno ... ake su stare". So the sync now runs a SECOND,
-- unbounded Gmail query (`is:starred`) with no `after:` at all, and this
-- column is what lets a row say "I am here because I am starred", which is
-- the difference between a message that fell outside the window and one that
-- was never starred.
--
-- Without it, a starred mail from 2023 arriving through the unbounded query
-- would be indistinguishable from a gap in the windowed one.
ALTER TABLE mail_messages ADD COLUMN is_starred INTEGER NOT NULL DEFAULT 0;

-- ---------------------------------------------------------------------------
-- 2. THE FULL SENDER - "odkial ide, z akeho mailu"
-- ---------------------------------------------------------------------------
--
-- `from_address` is the parsed address and was all the list needed. Opening
-- a message asks a different question: who really sent this, and where would
-- a reply go. Those are three different headers and they routinely disagree
-- - which is exactly what makes a phishing mail recognisable.
--
-- Stored verbatim, as received. Never trusted, only shown.
ALTER TABLE mail_messages ADD COLUMN to_address   TEXT;
ALTER TABLE mail_messages ADD COLUMN reply_to     TEXT;
-- The `From` header exactly as it arrived, display name and all, so the
-- detail view can show the raw thing rather than this app's reading of it.
ALTER TABLE mail_messages ADD COLUMN from_raw     TEXT;

-- ---------------------------------------------------------------------------
-- 3. OTP CODES - "policko kde budu chodit vyhradne len OTP kody"
-- ---------------------------------------------------------------------------
--
-- The code itself, pulled out of the subject or the preview so marko can
-- read it from the list without opening anything - which is the whole point
-- of a one-time code, since by the time you have opened the mail, found the
-- digits and gone back, half the window is gone.
--
-- NULL means "no code found", never a guess. See commands/mail.rs::extract_otp
-- for the rules: a code keyword must be present AND a standalone 4-8 digit
-- run must sit near it. A bare number anywhere in a message is an order
-- number far more often than it is a one-time code.
ALTER TABLE mail_messages ADD COLUMN otp_code TEXT;

CREATE INDEX IF NOT EXISTS idx_mail_messages_starred ON mail_messages (is_starred, received_at DESC);
CREATE INDEX IF NOT EXISTS idx_mail_messages_otp     ON mail_messages (otp_code) WHERE otp_code IS NOT NULL;

-- ---------------------------------------------------------------------------
-- 4. FOLDERS - "chcem aby tam tiez boli zalozky"
-- ---------------------------------------------------------------------------
--
-- Gmail returns LABEL IDS on a message (`INBOX`, `STARRED`, `Label_8`), not
-- names. A user-made label is only ever an opaque id there, so without this
-- table marko's own folders would show up as "Label_8" - which is worse than
-- not showing them.
--
-- Fetched from `users.labels.list` on every sync. It is one cheap call and
-- labels get renamed, so re-reading them is simpler and more correct than
-- trying to keep a copy in step.
--
-- `kind` is Gmail's own `type`: `system` for INBOX/STARRED/SENT and the
-- CATEGORY_* tabs, `user` for the folders marko made himself. The UI lists
-- his own folders separately, because those are the ones he will look for.
CREATE TABLE IF NOT EXISTS mail_labels (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    kind       TEXT NOT NULL DEFAULT 'user',
    updated_at TEXT NOT NULL
);

-- ---------------------------------------------------------------------------
-- 5. WHAT THE MESSAGE SAYS - the numbers, pulled out
-- ---------------------------------------------------------------------------
--
-- marko: "ked uvidi ze order confirmation tak mi to da oznamenie na ktore
-- ked kliknem tak podla informacii mi napriklad vytvori order".
--
-- That is two jobs and this migration only stores the first: READING the
-- numbers. Creating a record from them is the next release, deliberately -
-- orders, sales and finance entries are protected areas, and a parser that
-- writes money records before its reading has ever been checked against
-- real mail is how a mailbox turns into a corrupted ledger. The same rule
-- phase 1 followed: show marko what it would have written, first.
--
-- Money is integer cents and the currency is a separate CODE column, which
-- is this app's rule everywhere and not negotiable here either.
--
-- NULL always means "not found", never zero and never a guess. A sale
-- confirmation that did not state a quantity leaves `extracted_quantity`
-- NULL rather than assuming 1.
ALTER TABLE mail_messages ADD COLUMN extracted_amount_cents INTEGER;
ALTER TABLE mail_messages ADD COLUMN extracted_currency     TEXT;
ALTER TABLE mail_messages ADD COLUMN extracted_quantity     INTEGER;
-- The order / booking / transaction reference the message quotes. This is
-- the field that will eventually MATCH a mail to an existing TIQR record,
-- which is why it is worth storing even before anything matches on it.
ALTER TABLE mail_messages ADD COLUMN extracted_reference    TEXT;

-- ---------------------------------------------------------------------------
-- 6. TIQR'S OWN TRASH - "vzdy najskor presunutie do kosa, az potom zmazanie"
-- ---------------------------------------------------------------------------
--
-- First, the limit, because it shapes everything here: the scope this app
-- holds is `gmail.readonly`. It CANNOT delete a message, trash one, or move
-- one in Gmail. Anything this column does is local to TIQR, and the UI says
-- so rather than letting it look like a Gmail action that silently failed.
--
-- `trashed_at` NULL means the row is live. Set, it is in TIQR's trash: out
-- of every folder except Trash, still on disk, still restorable.
--
-- The two-step rule marko asked for is enforced in the BACKEND, not in the
-- UI: `delete_mail_forever` refuses any id whose `trashed_at` is NULL. A
-- confirm dialog is a habit; a refusal in the command is a guarantee, and
-- the only one worth having for something irreversible.
ALTER TABLE mail_messages ADD COLUMN trashed_at TEXT;

CREATE INDEX IF NOT EXISTS idx_mail_messages_trashed ON mail_messages (trashed_at);

-- Per ticket, derived as total / quantity - never read as its own number.
-- marko's rule: "nerozdelovat fees a cenu listka ale spojit". The total
-- already has the fees in it, so dividing is the only way to get a
-- per-ticket figure that is actually what a ticket cost. NULL when the
-- message did not say how many tickets, because dividing by an assumed 1
-- turns a four-ticket order into a per-ticket price four times too high.
ALTER TABLE mail_messages ADD COLUMN extracted_unit_cents INTEGER;
-- WHICH figure was chosen and why - "order total", "largest amount",
-- "parts added together". Shown in the UI so the reading can be argued
-- with instead of taken on trust.
ALTER TABLE mail_messages ADD COLUMN extracted_basis TEXT;

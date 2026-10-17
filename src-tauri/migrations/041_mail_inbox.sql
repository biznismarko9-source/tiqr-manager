-- 041: Mail Intelligence becomes a real inbox.
--
-- marko: "urob normalne z toho mailu normalne cely inbox".
--
-- 2.101.0 had narrowed the sync to known marketplace senders because of his
-- earlier complaint ("je tam velmi vela spamu co zbytocne pozera"). Those two
-- requests only look contradictory. What he objected to was spam getting in
-- the WAY, not spam existing - so the answer is to fetch the whole mailbox
-- and let the UI separate "everything" from "what matters", rather than to
-- refuse to fetch it.
--
-- Two columns is all that needs.
--
-- WHY `is_unread` IS LOCAL AND NOT WRITTEN BACK TO GMAIL
--
-- The scope this app asks Google for is `gmail.readonly`. It CANNOT mark a
-- message read in Gmail, and that is deliberate (see commands/mail.rs). So
-- this column is TIQR's own read state: opening a mail here marks it read
-- here, and Gmail is untouched. The UI says so rather than letting it look
-- like a sync that is broken.
--
-- `is_unread` starts at 1 for anything already stored, because every message
-- synced before this migration arrived without the flag and "unread" is the
-- honest default for "nobody has opened this in TIQR".
ALTER TABLE mail_messages ADD COLUMN is_unread INTEGER NOT NULL DEFAULT 1;

-- Gmail's own labels for the message, comma separated and lower-cased
-- (`inbox`, `unread`, `category_promotions`, ...). Stored as text rather
-- than a join table: they are only ever read back as a whole, never queried
-- across messages, and a text column is one migration instead of three.
--
-- `category_promotions` and `category_social` are what Gmail itself calls
-- the Promotions and Social tabs. Keeping them means the inbox here can
-- offer the same split marko already knows from Gmail, which is a better
-- answer to "too much spam" than a filter this app invents.
ALTER TABLE mail_messages ADD COLUMN labels TEXT;

-- The inbox list is "newest first, unread first" and filters on labels.
CREATE INDEX IF NOT EXISTS idx_mail_messages_unread ON mail_messages (is_unread, received_at DESC);

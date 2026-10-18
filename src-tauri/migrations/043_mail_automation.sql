-- 043: what a message turned into, and whether it was done for marko.
--
-- marko: "nech sa nepletie a radsej robi pomaly order po ordri ze ja si
-- vlastne viem vybrat v maily ci to urobit automaticky alebo manualne,
-- to iste sales, finances, taktiez nejake webhooks".
--
-- THE LINK IS WHAT MAKES THIS SAFE.
--
-- One column answers the only question that really matters here: has this
-- message already become a record? Without it, a second sync, a second
-- click or a restart mid-run creates the order twice - and a duplicated
-- order is exactly the kind of damage this app must never do on its own.
--
-- `created_kind` is 'order' | 'sale' | 'finance'; `created_id` is that
-- record's own id. Both NULL means nothing has been created from this
-- message, which is the state every message starts in and most stay in.
--
-- Deliberately NOT a foreign key. The record is marko's and he may delete
-- it; that must not cascade back and delete his mail, and it must not stop
-- the delete either. A dangling id here simply means "this was created once
-- and the record is gone now", which is true and worth keeping.
ALTER TABLE mail_messages ADD COLUMN created_kind TEXT;
ALTER TABLE mail_messages ADD COLUMN created_id   INTEGER;
ALTER TABLE mail_messages ADD COLUMN created_at   TEXT;
-- 'manual' when marko pressed the button, 'auto' when the sync did it for
-- him. Kept because the two deserve different trust when something later
-- looks wrong, and because the audit trail should say who decided.
ALTER TABLE mail_messages ADD COLUMN created_by   TEXT;

CREATE INDEX IF NOT EXISTS idx_mail_messages_created ON mail_messages (created_kind, created_id);

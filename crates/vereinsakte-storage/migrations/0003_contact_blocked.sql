ALTER TABLE members ADD COLUMN contact_blocked INTEGER NOT NULL DEFAULT 0 CHECK (contact_blocked IN (0, 1));
ALTER TABLE members ADD COLUMN contact_notice_seen INTEGER NOT NULL DEFAULT 0 CHECK (contact_notice_seen IN (0, 1));

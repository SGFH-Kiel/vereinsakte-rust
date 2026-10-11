ALTER TABLE members ADD COLUMN contact_blocked INTEGER NOT NULL DEFAULT 0 CHECK (contact_blocked IN (0, 1));

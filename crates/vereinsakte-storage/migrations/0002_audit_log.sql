-- Änderungsprotokoll: unveränderliche Einträge über jede schreibende Aktion.
--
-- Bewusst ohne Fremdschlüssel auf `members`: Das Protokoll muss auch dann
-- erhalten bleiben, wenn die Akte wiederhergestellt oder ein Mitglied ersetzt
-- wird. Es enthält nur Kennungen und Feldnamen, keine Werte personenbezogener
-- Felder.

CREATE TABLE audit_log (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    at                TEXT    NOT NULL,
    actor_id          INTEGER NOT NULL CHECK (actor_id > 0),
    action            TEXT    NOT NULL CHECK (action <> ''),
    object_kind       TEXT    NOT NULL,
    object_id         TEXT    NOT NULL,
    subject_member_id INTEGER CHECK (subject_member_id IS NULL OR subject_member_id > 0),
    summary           TEXT    NOT NULL
) STRICT;

CREATE INDEX audit_log_at ON audit_log (at);
CREATE INDEX audit_log_actor ON audit_log (actor_id);
CREATE INDEX audit_log_subject ON audit_log (subject_member_id);

CREATE TRIGGER audit_log_no_update BEFORE UPDATE ON audit_log
BEGIN
    SELECT RAISE(ABORT, 'Protokolleinträge sind unveränderlich');
END;

CREATE TRIGGER audit_log_no_delete BEFORE DELETE ON audit_log
BEGIN
    SELECT RAISE(ABORT, 'Protokolleinträge sind unveränderlich');
END;

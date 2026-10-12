-- Neuer Aufgabenstatus „waiting“ (die CHECK-Bedingung lässt sich nicht ändern,
-- daher wird die Tabelle neu angelegt; Verweise darauf werden vorher gesichert
-- und danach wiederhergestellt), dazu Kommentare und Anhänge (Dateien als BLOB).
CREATE TEMP TABLE saved_assignees AS SELECT task_id, position, member_id FROM task_assignees;
CREATE TEMP TABLE saved_entry_tasks AS SELECT id, task_id FROM work_entries WHERE task_id IS NOT NULL;
UPDATE work_entries SET task_id = NULL WHERE task_id IS NOT NULL;

CREATE TABLE tasks_new (
    id              TEXT    PRIMARY KEY,
    position        INTEGER NOT NULL,
    version  INTEGER NOT NULL DEFAULT 1,
    title           TEXT    NOT NULL,
    description     TEXT,
    boat_id         TEXT    REFERENCES boats (id),
    estimated_hours REAL,
    due_at          TEXT,
    status          TEXT    NOT NULL CHECK (status IN ('open', 'in_progress', 'waiting', 'done')),
    done_at         TEXT
) STRICT;
INSERT INTO tasks_new (id, position, version, title, description, boat_id, estimated_hours, due_at, status, done_at)
    SELECT id, position, version, title, description, boat_id, estimated_hours, due_at, status, done_at FROM tasks;
DROP TABLE tasks;
ALTER TABLE tasks_new RENAME TO tasks;
INSERT INTO task_assignees (task_id, position, member_id)
    SELECT task_id, position, member_id FROM saved_assignees;
UPDATE work_entries SET task_id = (SELECT task_id FROM saved_entry_tasks WHERE saved_entry_tasks.id = work_entries.id)
    WHERE id IN (SELECT id FROM saved_entry_tasks);
DROP TABLE saved_assignees;
DROP TABLE saved_entry_tasks;

CREATE TABLE task_comments (
    id        TEXT    PRIMARY KEY,
    task_id   TEXT    NOT NULL REFERENCES tasks (id) ON DELETE CASCADE,
    author_id INTEGER NOT NULL REFERENCES members (id),
    at        TEXT    NOT NULL,
    text      TEXT    NOT NULL CHECK (length(text) BETWEEN 1 AND 5000)
) STRICT;
CREATE INDEX task_comments_task ON task_comments (task_id, at);

CREATE TABLE task_attachments (
    id           TEXT    PRIMARY KEY,
    task_id      TEXT    NOT NULL REFERENCES tasks (id) ON DELETE CASCADE,
    uploader_id  INTEGER NOT NULL REFERENCES members (id),
    at           TEXT    NOT NULL,
    kind         TEXT    NOT NULL CHECK (kind IN ('image', 'pdf')),
    filename     TEXT    NOT NULL,
    content_type TEXT    NOT NULL,
    alt          TEXT,
    size         INTEGER NOT NULL,
    data         BLOB    NOT NULL
) STRICT;
CREATE INDEX task_attachments_task ON task_attachments (task_id, at);

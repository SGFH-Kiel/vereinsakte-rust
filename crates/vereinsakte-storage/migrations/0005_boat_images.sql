-- Ein Bild je Boot (Dateidaten als BLOB); Ersetzen ist ein Upsert.
CREATE TABLE boat_images (
    boat_id      TEXT    PRIMARY KEY REFERENCES boats (id) ON DELETE CASCADE,
    content_type TEXT    NOT NULL,
    alt          TEXT,
    size         INTEGER NOT NULL,
    updated_at   TEXT    NOT NULL,
    data         BLOB    NOT NULL
) STRICT;

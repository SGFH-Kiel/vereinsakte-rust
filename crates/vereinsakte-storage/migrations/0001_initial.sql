-- Relationales Schema der Vereinsakte.
--
-- Konventionen:
--   * jede Eigenschaft eine typisierte Spalte (STRICT-Tabellen), keine JSON-Spalten
--   * Zeitpunkte als RFC3339-Text mit Offset (`*_at`, `start_at`, `end_at`)
--   * Geldbeträge als ganze Cent (`*_cents`)
--   * Auswahlwerte als Text in snake_case mit CHECK auf die Werte des Fachmodells
--   * `position` erhält die Reihenfolge der Listen der Vereinsakte
--   * Verweise sind Fremdschlüssel (PRAGMA foreign_keys = ON beim Verbinden)
--   * jede eigenständig änderbare Wurzeltabelle hat eine Spalte `version`
--     (optimistische Sperre je Datensatz)

-- Mitglieder --------------------------------------------------------------

CREATE TABLE members (
    id                INTEGER PRIMARY KEY CHECK (id > 0),
    position          INTEGER NOT NULL,
    version  INTEGER NOT NULL DEFAULT 1,
    last_name         TEXT    NOT NULL,
    first_name        TEXT    NOT NULL,
    street            TEXT    NOT NULL,
    postal_code       TEXT    NOT NULL,
    city              TEXT    NOT NULL,
    phone_landline    TEXT,
    phone_business    TEXT,
    phone_mobile      TEXT,
    email             TEXT,
    login_email       TEXT COLLATE NOCASE UNIQUE,
    address_valid     INTEGER NOT NULL CHECK (address_valid IN (0, 1)),
    membership_type   TEXT    NOT NULL CHECK (membership_type IN
                          ('regular', 'student', 'extraordinary', 'honorary', 'child_of_member', 'erasmus')),
    child_of          INTEGER REFERENCES members (id),
    board_consent     INTEGER NOT NULL CHECK (board_consent IN (0, 1)),
    joined_at         TEXT    NOT NULL,
    provisional_until TEXT,
    limited_until     TEXT,
    left_at           TEXT,
    expelled_at       TEXT
) STRICT;

CREATE TABLE member_sailing_licenses (
    member_id   INTEGER NOT NULL REFERENCES members (id) ON DELETE CASCADE,
    position    INTEGER NOT NULL,
    kind        TEXT    NOT NULL,
    acquired_at TEXT,
    PRIMARY KEY (member_id, position)
) STRICT;

CREATE TABLE member_radio_licenses (
    member_id   INTEGER NOT NULL REFERENCES members (id) ON DELETE CASCADE,
    position    INTEGER NOT NULL,
    kind        TEXT    NOT NULL,
    acquired_at TEXT,
    PRIMARY KEY (member_id, position)
) STRICT;

-- Boote -------------------------------------------------------------------

CREATE TABLE boats (
    id           TEXT    PRIMARY KEY,
    position     INTEGER NOT NULL,
    version  INTEGER NOT NULL DEFAULT 1,
    name         TEXT    NOT NULL,
    boat_type    TEXT    NOT NULL CHECK (boat_type IN ('dinghy', 'keelboat')),
    class        TEXT,
    year_built   INTEGER,
    length_m     REAL,
    beam_m       REAL,
    draft_m      REAL,
    sail_area_m2 REAL,
    berths       INTEGER,
    engine       TEXT,
    mooring      TEXT
) STRICT;

CREATE TABLE crew_rules (
    boat_id             TEXT    NOT NULL REFERENCES boats (id) ON DELETE CASCADE,
    position            INTEGER NOT NULL,
    area                TEXT    NOT NULL CHECK (area IN ('home_waters', 'outside')),
    min_members         INTEGER NOT NULL CHECK (min_members >= 0),
    min_persons         INTEGER NOT NULL CHECK (min_persons >= 0),
    co_skipper_required INTEGER NOT NULL CHECK (co_skipper_required IN (0, 1)),
    PRIMARY KEY (boat_id, area)
) STRICT;

-- Gruppen einer Anforderung: area NULL = Skipper-Anforderung des Bootes,
-- sonst Co-Skipper-Anforderung der Besatzungsregel dieses Fahrtgebiets.
CREATE TABLE requirement_groups (
    id       INTEGER PRIMARY KEY,
    boat_id  TEXT    NOT NULL REFERENCES boats (id) ON DELETE CASCADE,
    area     TEXT    CHECK (area IS NULL OR area IN ('home_waters', 'outside')),
    position INTEGER NOT NULL,
    FOREIGN KEY (boat_id, area) REFERENCES crew_rules (boat_id, area) ON DELETE CASCADE
) STRICT;

CREATE TABLE requirement_qualifications (
    group_id  INTEGER NOT NULL REFERENCES requirement_groups (id) ON DELETE CASCADE,
    position  INTEGER NOT NULL,
    kind      TEXT    NOT NULL CHECK (kind IN
                  ('sailing_license', 'radio_license', 'boat_permit', 'boat_permit_for_type')),
    license   TEXT,
    boat_ref  TEXT    REFERENCES boats (id),
    boat_type TEXT    CHECK (boat_type IS NULL OR boat_type IN ('dinghy', 'keelboat')),
    PRIMARY KEY (group_id, position),
    CHECK (
        (kind IN ('sailing_license', 'radio_license')
            AND license IS NOT NULL AND boat_ref IS NULL AND boat_type IS NULL)
        OR (kind = 'boat_permit' AND boat_ref IS NOT NULL AND license IS NULL AND boat_type IS NULL)
        OR (kind = 'boat_permit_for_type' AND boat_type IS NOT NULL AND license IS NULL AND boat_ref IS NULL)
    )
) STRICT;

CREATE TABLE boat_releases (
    boat_id   TEXT    NOT NULL REFERENCES boats (id) ON DELETE CASCADE,
    position  INTEGER NOT NULL,
    season    INTEGER NOT NULL,
    by_member INTEGER NOT NULL REFERENCES members (id),
    at        TEXT    NOT NULL,
    PRIMARY KEY (boat_id, position)
) STRICT;

CREATE TABLE boat_blocks (
    boat_id   TEXT    NOT NULL REFERENCES boats (id) ON DELETE CASCADE,
    position  INTEGER NOT NULL,
    start_at  TEXT    NOT NULL,
    end_at    TEXT,
    reason    TEXT    NOT NULL,
    by_member INTEGER NOT NULL REFERENCES members (id),
    lifted_by INTEGER REFERENCES members (id),
    PRIMARY KEY (boat_id, position)
) STRICT;

-- Ämter, Abnahmen, Widerrufe ----------------------------------------------

CREATE TABLE offices (
    id           TEXT    PRIMARY KEY,
    position     INTEGER NOT NULL,
    version  INTEGER NOT NULL DEFAULT 1,
    role         TEXT    NOT NULL CHECK (role IN ('chair', 'vice_chair', 'treasurer', 'secretary',
                     'head_boatwarden', 'auditor', 'boatwarden', 'examiner')),
    role_boat_id TEXT    REFERENCES boats (id),
    member_id    INTEGER NOT NULL REFERENCES members (id),
    start_at     TEXT    NOT NULL,
    end_at       TEXT,
    CHECK ((role IN ('boatwarden', 'examiner')) = (role_boat_id IS NOT NULL))
) STRICT;

CREATE TABLE boat_checkouts (
    id          TEXT    PRIMARY KEY,
    position    INTEGER NOT NULL,
    version  INTEGER NOT NULL DEFAULT 1,
    member_id   INTEGER NOT NULL REFERENCES members (id),
    boat_id     TEXT    NOT NULL REFERENCES boats (id),
    at          TEXT    NOT NULL,
    result      TEXT    NOT NULL CHECK (result IN ('passed', 'failed')),
    examiner_id INTEGER NOT NULL REFERENCES members (id),
    remark      TEXT
) STRICT;

CREATE TABLE revocations (
    id        TEXT    PRIMARY KEY,
    position  INTEGER NOT NULL,
    version  INTEGER NOT NULL DEFAULT 1,
    member_id INTEGER NOT NULL REFERENCES members (id),
    boat_id   TEXT    NOT NULL REFERENCES boats (id),
    at        TEXT    NOT NULL,
    reason    TEXT    NOT NULL,
    by_member INTEGER NOT NULL REFERENCES members (id)
) STRICT;

-- Arbeitstermine, Aufgaben, Arbeitsstunden ----------------------------------

CREATE TABLE work_events (
    id               TEXT    PRIMARY KEY,
    position         INTEGER NOT NULL,
    version  INTEGER NOT NULL DEFAULT 1,
    title            TEXT    NOT NULL,
    start_at         TEXT    NOT NULL,
    end_at           TEXT    NOT NULL,
    location         TEXT,
    description      TEXT,
    boat_id          TEXT    REFERENCES boats (id),
    max_participants INTEGER CHECK (max_participants IS NULL OR max_participants >= 0)
) STRICT;

CREATE TABLE work_event_registrations (
    event_id  TEXT    NOT NULL REFERENCES work_events (id) ON DELETE CASCADE,
    position  INTEGER NOT NULL,
    member_id INTEGER NOT NULL REFERENCES members (id),
    PRIMARY KEY (event_id, position),
    UNIQUE (event_id, member_id)
) STRICT;

CREATE TABLE tasks (
    id              TEXT    PRIMARY KEY,
    position        INTEGER NOT NULL,
    version  INTEGER NOT NULL DEFAULT 1,
    title           TEXT    NOT NULL,
    description     TEXT,
    boat_id         TEXT    REFERENCES boats (id),
    estimated_hours REAL,
    due_at          TEXT,
    status          TEXT    NOT NULL CHECK (status IN ('open', 'in_progress', 'done')),
    done_at         TEXT
) STRICT;

CREATE TABLE task_assignees (
    task_id   TEXT    NOT NULL REFERENCES tasks (id) ON DELETE CASCADE,
    position  INTEGER NOT NULL,
    member_id INTEGER NOT NULL REFERENCES members (id),
    PRIMARY KEY (task_id, position),
    UNIQUE (task_id, member_id)
) STRICT;

-- Umfang entweder (date_at, hours) oder (start_at, end_at);
-- subject_boat_id NULL = Arbeit für den Verein ohne Boot.
CREATE TABLE work_entries (
    id              TEXT    PRIMARY KEY,
    position        INTEGER NOT NULL,
    version  INTEGER NOT NULL DEFAULT 1,
    member_id       INTEGER NOT NULL REFERENCES members (id),
    date_at         TEXT,
    hours           REAL,
    start_at        TEXT,
    end_at          TEXT,
    subject_boat_id TEXT    REFERENCES boats (id),
    work_event_id   TEXT    REFERENCES work_events (id),
    task_id         TEXT    REFERENCES tasks (id),
    description     TEXT,
    confirmed_by    INTEGER REFERENCES members (id),
    confirmed_at    TEXT,
    CHECK ((date_at IS NOT NULL AND hours IS NOT NULL AND start_at IS NULL AND end_at IS NULL)
        OR (date_at IS NULL AND hours IS NULL AND start_at IS NOT NULL AND end_at IS NOT NULL)),
    CHECK (work_event_id IS NULL OR task_id IS NULL),
    CHECK ((confirmed_by IS NULL) = (confirmed_at IS NULL))
) STRICT;

CREATE TABLE hour_transfers (
    id          TEXT    PRIMARY KEY,
    position    INTEGER NOT NULL,
    version  INTEGER NOT NULL DEFAULT 1,
    year        INTEGER NOT NULL,
    from_member INTEGER NOT NULL REFERENCES members (id),
    to_member   INTEGER NOT NULL REFERENCES members (id),
    hours       REAL    NOT NULL,
    description TEXT
) STRICT;

-- Saison und Gebühren -------------------------------------------------------

CREATE TABLE seasons (
    year                    INTEGER PRIMARY KEY,
    position                INTEGER NOT NULL,
    version  INTEGER NOT NULL DEFAULT 1,
    hours_quota             REAL,
    compensation_rate_cents INTEGER,
    has_fee_schedule        INTEGER NOT NULL CHECK (has_fee_schedule IN (0, 1))
) STRICT;

CREATE TABLE season_type_rates (
    year            INTEGER NOT NULL REFERENCES seasons (year) ON DELETE CASCADE,
    schedule        TEXT    NOT NULL CHECK (schedule IN ('annual', 'admission')),
    position        INTEGER NOT NULL,
    membership_type TEXT    NOT NULL CHECK (membership_type IN
                        ('regular', 'student', 'extraordinary', 'honorary', 'child_of_member', 'erasmus')),
    amount_cents    INTEGER NOT NULL,
    PRIMARY KEY (year, schedule, position)
) STRICT;

CREATE TABLE season_tour_rates (
    year             INTEGER NOT NULL REFERENCES seasons (year) ON DELETE CASCADE,
    position         INTEGER NOT NULL,
    tariff_group     TEXT    NOT NULL CHECK (tariff_group IN ('regular', 'student', 'extraordinary', 'guest')),
    day_cents        INTEGER NOT NULL,
    week_cents       INTEGER NOT NULL,
    first_week_cents INTEGER,
    PRIMARY KEY (year, position)
) STRICT;

CREATE TABLE annual_records (
    member_id                INTEGER NOT NULL REFERENCES members (id),
    year                     INTEGER NOT NULL,
    position                 INTEGER NOT NULL,
    version  INTEGER NOT NULL DEFAULT 1,
    reduced_hours            REAL,
    reduction_reason         TEXT,
    compensation_hours       REAL,
    compensation_cents       INTEGER,
    compensation_approved_at TEXT,
    compensation_reason      TEXT,
    PRIMARY KEY (member_id, year),
    CHECK ((reduced_hours IS NULL) = (reduction_reason IS NULL)),
    CHECK ((compensation_hours IS NULL) = (compensation_approved_at IS NULL))
) STRICT;

-- Reservierungen ------------------------------------------------------------

CREATE TABLE reservations (
    id            TEXT    PRIMARY KEY,
    position      INTEGER NOT NULL,
    version  INTEGER NOT NULL DEFAULT 1,
    boat_id       TEXT    NOT NULL REFERENCES boats (id),
    start_at      TEXT    NOT NULL,
    end_at        TEXT    NOT NULL,
    area          TEXT    NOT NULL CHECK (area IN ('home_waters', 'outside')),
    tour          INTEGER NOT NULL CHECK (tour IN (0, 1)),
    skipper_id    INTEGER NOT NULL REFERENCES members (id),
    co_skipper_id INTEGER REFERENCES members (id),
    remark        TEXT
) STRICT;

CREATE TABLE reservation_crew (
    reservation_id TEXT    NOT NULL REFERENCES reservations (id) ON DELETE CASCADE,
    position       INTEGER NOT NULL,
    member_id      INTEGER NOT NULL REFERENCES members (id),
    from_at        TEXT,
    until_at       TEXT,
    PRIMARY KEY (reservation_id, position)
) STRICT;

CREATE TABLE reservation_guests (
    reservation_id TEXT    NOT NULL REFERENCES reservations (id) ON DELETE CASCADE,
    position       INTEGER NOT NULL,
    name           TEXT    NOT NULL,
    phone          TEXT,
    invited_by     INTEGER REFERENCES members (id),
    from_at        TEXT,
    until_at       TEXT,
    PRIMARY KEY (reservation_id, position)
) STRICT;

-- Mitgliedskonto -------------------------------------------------------------

CREATE TABLE claims (
    id             TEXT    PRIMARY KEY,
    member_id      INTEGER NOT NULL REFERENCES members (id) ON DELETE CASCADE,
    position       INTEGER NOT NULL,
    kind           TEXT    NOT NULL CHECK (kind IN
                       ('annual_fee', 'admission_fee', 'tour_fee', 'work_compensation', 'other')),
    year           INTEGER,
    reservation_id TEXT    REFERENCES reservations (id),
    guest          TEXT,
    amount_cents   INTEGER NOT NULL,
    text           TEXT    NOT NULL,
    created_at     TEXT    NOT NULL,
    due_at         TEXT,
    cancelled_at   TEXT,
    cancel_reason  TEXT,
    CHECK ((kind IN ('annual_fee', 'work_compensation')) = (year IS NOT NULL)),
    CHECK ((kind = 'tour_fee') = (reservation_id IS NOT NULL)),
    CHECK (guest IS NULL OR kind = 'tour_fee'),
    CHECK ((cancelled_at IS NULL) = (cancel_reason IS NULL))
) STRICT;

CREATE TABLE payments (
    id           TEXT    PRIMARY KEY,
    member_id    INTEGER NOT NULL REFERENCES members (id) ON DELETE CASCADE,
    position     INTEGER NOT NULL,
    amount_cents INTEGER NOT NULL,
    at           TEXT    NOT NULL,
    text         TEXT
) STRICT;

-- Anmeldung -----------------------------------------------------------------

CREATE TABLE identities (
    provider  TEXT    NOT NULL,
    subject   TEXT    NOT NULL,
    member_id INTEGER NOT NULL UNIQUE REFERENCES members (id) ON DELETE CASCADE,
    PRIMARY KEY (provider, subject)
) STRICT;

-- Indizes für die gezielten Abfragen ------------------------------------------

CREATE INDEX members_position            ON members (position);
CREATE INDEX offices_member              ON offices (member_id);
CREATE INDEX offices_role_boat           ON offices (role_boat_id);
CREATE INDEX boat_checkouts_member       ON boat_checkouts (member_id);
CREATE INDEX boat_checkouts_boat         ON boat_checkouts (boat_id);
CREATE INDEX revocations_member          ON revocations (member_id);
CREATE INDEX work_entries_member         ON work_entries (member_id);
CREATE INDEX work_entries_task           ON work_entries (task_id);
CREATE INDEX work_entries_event          ON work_entries (work_event_id);
CREATE INDEX hour_transfers_year         ON hour_transfers (year);
CREATE INDEX hour_transfers_from         ON hour_transfers (from_member);
CREATE INDEX hour_transfers_to           ON hour_transfers (to_member);
CREATE INDEX claims_member               ON claims (member_id);
CREATE INDEX claims_reservation          ON claims (reservation_id);
CREATE INDEX payments_member             ON payments (member_id);
CREATE INDEX requirement_groups_boat     ON requirement_groups (boat_id, area);
CREATE INDEX reservations_boat_start     ON reservations (boat_id, start_at);
CREATE INDEX reservation_crew_member     ON reservation_crew (member_id);
CREATE INDEX work_event_registrations_member ON work_event_registrations (member_id);
CREATE INDEX task_assignees_member       ON task_assignees (member_id);

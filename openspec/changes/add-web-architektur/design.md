# Design

## Context

Stand nach `add-datenmodell`: ein Binary-Crate mit `src/model/` (Fachmodell, ~7.700 Zeilen inkl. Tests), `main.rs` nutzt das Modell nicht. Das Modell ist rein (keine I/O), arbeitet auf dem Wurzelobjekt `ClubRecord` und enthält alle Fachregeln (Segelerlaubnis, Rechte, Konto …). Testdaten stehen im Modul `example` (nur `cfg(test)`) und als `src/model/testdata/vereinsakte_beispiel.json`. Konvention: Code englisch, Doku/Texte deutsch. Motivation und Umfang: siehe proposal.md; Verhalten: specs.

Getroffene Nutzerentscheidungen: Web-App mit serverseitigem HTML + htmx, SQLite mit vollständig relationalem Schema (keine JSON-Spalten), Anmeldung aller Mitglieder über Microsoft Entra ID, Cargo-Workspace, Gerüst plus Durchstich „Meine Daten“/Mitgliederverzeichnis.

## Goals / Non-Goals

**Goals:**
- Feste Schichten mit einseitigen Abhängigkeiten, sodass das Fachmodell frei von I/O, HTTP und SQL bleibt.
- Fachregeln werden nur im Core implementiert; App- und Web-Schicht rufen sie auf, statt sie nachzubauen.
- Jede Schicht ist ohne die darüberliegenden testbar (Core rein, App gegen In-Memory-SQLite, Web per `tower::ServiceExt::oneshot`).
- Ein vollständiger Durchstich als Vorlage für alle weiteren Fachseiten.

**Non-Goals:**
- Kein Schreiben über die Oberfläche (außer Anmeldung/Abmeldung), keine weiteren Fachseiten.
- Keine Optimierung für große Datenmengen (Verein: wenige hundert Mitglieder).
- Kein Deployment, kein Mailversand, kein JavaScript-Build.

## Decisions

### 1. Workspace und Abhängigkeitsrichtung
```
crates/
  vereinsakte-core/      Fachmodell (bisher src/model), keine I/O
  vereinsakte-app/       Anwendungsfälle, Zugriffsprüfung, Ports (Traits)   → core
  vereinsakte-storage/   SQLite-Adapter der Ports, Migrationen               → app, core
  vereinsakte-web/       axum-Router, Templates, OIDC, Sitzungen             → app, core
  vereinsakte/           Binary: CLI, Konfiguration, Zusammenbau             → alle
```
Die App-Schicht definiert anwendungsfallbezogene Ports (siehe 5); Storage implementiert sie mit SQL. Web kennt Storage nicht – nur das Binary verdrahtet beides (Composition Root). Gemeinsame Versionen über `[workspace.dependencies]`, gemeinsame Lints über `[workspace.lints]`.
*Alternativen:* ein Crate mit Modulen (vom Nutzer verworfen); Storage direkt unter Web (würde SQL bis in Handler ziehen und Web-Tests an SQLite binden).

### 2. Core übernimmt das Modell, plus Anmeldekonto
`src/model/*` wird zu `crates/vereinsakte-core/src/*` (`mod.rs` → `lib.rs`), Pfade `super::`/`crate::model::` werden angepasst. Einzige fachliche Erweiterung: `Member::login_email: Option<String>` (Adresse des Microsoft-Kontos, getrennt von der Kontakt-E-Mail `email`, `#[serde(default, skip_serializing_if)]`) und eine Prüfung in `ClubRecord::validate()` auf doppelte Anmeldekonten ohne Beachtung der Groß-/Kleinschreibung (`RecordIssue::DuplicateLoginEmail`). Die Beispielakte erhält Anmeldekonten der Form `vorname.nachname@sgfh.example` (bewusst nicht die echte Vereinsdomäne); die JSON-Beispieldatei wird neu erzeugt.
*Alternative:* Anmeldekonto nur in der Tabelle `identities` des Storage (vom Vorstand per Befehl vorbelegt) – verworfen, weil es Stammdaten sind, die mit der Akte exportiert und später in der Oberfläche gepflegt werden sollen. Das Testdaten-Modul wird als `#[cfg(any(test, feature = "example"))] pub mod example;` veröffentlicht; andere Crates aktivieren das Feature **nur** in `[dev-dependencies]`, damit es im Produktiv-Build nicht auftaucht. Die JSON-Beispieldatei zieht nach `crates/vereinsakte-core/testdata/` und dient zusätzlich als Startdaten für die Entwicklung (`vereinsakte import`).

### 3. Speicherung: vollständig relationales Schema
Jede Eigenschaft hat eine eigene Spalte, Listen stehen in Untertabellen, Verweise sind Fremdschlüssel (`PRAGMA foreign_keys = ON`); keine JSON-Spalten (Nutzerentscheidung). Typabbildung:

| Fachtyp | SQLite |
|---|---|
| `MemberId` | `INTEGER` (Primärschlüssel `members.id`) |
| UUID-Kennungen | `TEXT` (kanonische UUID-Schreibweise) |
| `Timestamp` | `TEXT` im RFC3339-Format mit Offset (Offset bleibt erhalten, Sortierung über `julianday()` bzw. im Code) |
| `Money` | `INTEGER` in Cent (verlustfrei, da `Money` auf Cent normiert ist) |
| Stunden, Maße | `REAL` |
| Auswahl-Enums | `TEXT` mit `CHECK (… IN (…))` in snake_case wie im JSON |
| Schein-/Lizenzarten | `TEXT` (Anzeigeform, z. B. `SKS`, auch freie Bezeichnungen) |
| `Option<T>` | Spalte `NULL`-bar |

Tabellen (Auszug, vollständig in `0001_initial.sql`):
```sql
members(id, version, last_name, first_name, street, postal_code, city, phone_landline, phone_business, phone_mobile,
        email, login_email UNIQUE COLLATE NOCASE, address_valid, membership_type CHECK(...), child_of REFERENCES members, board_consent,
        joined_at, provisional_until, limited_until, left_at, expelled_at)
member_sailing_licenses(member_id REFERENCES members ON DELETE CASCADE, position, kind, acquired_at, PK(member_id, position))
member_radio_licenses(...)                                   -- analog
boats(id, name, boat_type CHECK(...), class, year_built, length_m, beam_m, draft_m, sail_area_m2, berths, engine, mooring)
crew_rules(boat_id, area CHECK(...), min_members, min_persons, co_skipper_required, PK(boat_id, area))
requirement_groups(id INTEGER PRIMARY KEY, boat_id, area NULL, position)
        -- area NULL = Skipper-Anforderung des Bootes, sonst Co-Skipper-Anforderung der Besatzungsregel
        -- FOREIGN KEY (boat_id, area) REFERENCES crew_rules wenn area NOT NULL
requirement_qualifications(group_id, position, kind CHECK(sailing_license|radio_license|boat_permit|boat_permit_for_type),
        license TEXT, boat_ref REFERENCES boats, boat_type, CHECK genau ein Wert passend zu kind)
boat_releases(boat_id, season, by_member REFERENCES members, at, PK(boat_id, season, at))
boat_blocks(boat_id, position, start, end, reason, by_member, lifted_by)
offices(id, role CHECK(...), role_boat_id REFERENCES boats NULL, member_id, start, end,
        CHECK ((role IN ('boatwarden','examiner')) = (role_boat_id IS NOT NULL)))
boat_checkouts(id, member_id, boat_id, at, result CHECK(...), examiner_id, remark)
revocations(id, member_id, boat_id, at, reason, by_member)
work_events(id, title, start, end, location, description, boat_id NULL, max_participants)
work_event_registrations(event_id, position, member_id, PK(event_id, member_id))
tasks(id, title, description, boat_id NULL, estimated_hours, due_at, status CHECK(...), done_at)
task_assignees(task_id, position, member_id, PK(task_id, member_id))
work_entries(id, member_id, date, hours, start, end, subject_boat_id NULL /* NULL = Verein */,
        work_event_id NULL, task_id NULL, description, confirmed_by NULL, confirmed_at NULL,
        CHECK ((date IS NOT NULL AND hours IS NOT NULL AND start IS NULL AND end IS NULL)
            OR (date IS NULL AND hours IS NULL AND start IS NOT NULL AND end IS NOT NULL)),
        CHECK (work_event_id IS NULL OR task_id IS NULL),
        CHECK ((confirmed_by IS NULL) = (confirmed_at IS NULL)))
hour_transfers(id, year, from_member, to_member, hours, description)
seasons(year PRIMARY KEY, hours_quota, compensation_rate_cents, has_fee_schedule)
season_type_rates(year, schedule CHECK(annual|admission), membership_type, amount_cents, position)
season_tour_rates(year, tariff_group, day_cents, week_cents, first_week_cents, position)
annual_records(member_id, year, reduced_hours, reduction_reason, compensation_hours, compensation_cents,
        compensation_approved_at, compensation_reason, PK(member_id, year))
reservations(id, boat_id, start, end, area, tour, skipper_id, co_skipper_id, remark)
reservation_crew(reservation_id, position, member_id, from_at, until_at)
reservation_guests(reservation_id, position, name, phone, invited_by, from_at, until_at)
claims(id, member_id, position, kind CHECK(...), year NULL, reservation_id REFERENCES reservations NULL,
       guest NULL, amount_cents, text, created_at, due_at, cancelled_at NULL, cancel_reason NULL)
payments(id, member_id, position, amount_cents, at, text)
identities(provider, subject, member_id NOT NULL UNIQUE REFERENCES members, PRIMARY KEY (provider, subject))
```
- Die Reihenfolge von Listen bleibt über `position` erhalten, damit Laden → Speichern verlustfrei ist (Spec `datenhaltung`).
- **Zuordnung**: Das Storage-Crate enthält je Tabelle eine Zeilen-Struct (`sqlx::FromRow`) und Umwandlungen `Row ↔ Fachtyp`; Abfragen mit `sqlx::query_as` zur Laufzeit. Vollständigkeit sichern Round-Trip-Tests mit der Beispielakte, die jedes Feld belegt.
- **Gezielte Abfragen** je Port-Methode (siehe 5); Zeitvergleiche über `julianday(…)`, weil die Texte unterschiedliche Offsets tragen; Jahr eines Einsatzes über das Datum im gespeicherten Offset.
- **Schreiben je Aggregat** (Mitglied inkl. Scheine und Konto, Boot inkl. Regeln/Freigaben/Sperren, Reservierung inkl. Crew/Gästen, …): Elternzeile per Upsert bzw. `UPDATE … WHERE version = ?`, Kindzeilen löschen und neu einfügen.
- **Sicherung/Wiederherstellung**: Export lädt alle Tabellen zur ganzen Akte; Import gleicht den Bestand in einer Transaktion an die Akte an (nur geänderte Aggregate schreiben, entfernte löschen, `PRAGMA defer_foreign_keys = ON`) und schreibt alle Tabellen; Fremdschlüssel-, CHECK- und Fachprüfungen (`ClubRecord::validate`) lehnen ungültige Akten ab. Fremdschlüssel-Verletzungen werden in eine verständliche Meldung („unbekannter Verweis …“) übersetzt.
- Migrationen über `sqlx::migrate!` (eingebettet), ausgeführt bei `serve` und `migrate`.
*Alternativen:* JSON-Spalten je Sammlung (erster Entwurf, vom Nutzer verworfen); compile-time geprüfte `sqlx::query!`-Makros (bräuchten `DATABASE_URL` bzw. `cargo sqlx prepare` im Build – zurückgestellt, kann später ergänzt werden).

**Folge für das Fachmodell:** Das Modell erlaubt Verweise auf unbekannte Objekte (Prüfung meldet sie), die Datenbank nicht. Akten mit solchen Verweisen lassen sich nicht speichern; das ist gewollt (Spec `datenhaltung`, referentielle Integrität).

### 4. Fachregeln auf Teildaten (Core)
Die Datenbank ist führend; die ganze Akte wird nur für Sicherung/Import geladen. Fachregeln, die der Durchstich braucht, werden deshalb aus `ClubRecord` herausgelöst und arbeiten auf kleinen Eingabe-Strukturen, die genau ihre Daten enthalten:

```rust
/// Alles, was für Stunden, Beitrag und Segelerlaubnis eines Mitglieds in einem Jahr nötig ist.
pub struct MemberYear<'a> {
    pub member: &'a Member,                 // inkl. Konto (Forderungen, Einzahlungen)
    pub year: i32,
    pub season: Option<&'a Season>,
    pub annual_record: Option<&'a AnnualRecord>,
    pub work_entries: &'a [WorkEntry],      // nur dieses Mitglied, dieses Jahr
    pub hour_transfers: &'a [HourTransfer], // nur Überträge dieses Jahres mit diesem Mitglied
}
impl MemberYear<'_> {
    fn credited_hours(&self) -> f64; fn required_hours(&self) -> f64;
    fn fee_arrears(&self) -> Option<Money>; fn sailing_permit(&self) -> Result<(), Vec<SailingPermitMissing>>;
    fn sailing_permit_at(&self, t) -> …;
}
/// Abnahmen und Widerrufe eines Mitglieds → Bootserlaubnis je Boot.
pub struct PermitHistory<'a> { member: &'a Member, checkouts: &'a [BoatCheckout], revocations: &'a [Revocation] }
impl PermitHistory<'_> { fn boat_permit(&self, boat, t) -> Result<(), PermitMissing>; }
/// Ämter-Abfragen auf einer Liste von Ämtern (z. B. den zu `t` aktiven).
pub struct Offices<'a>(&'a [Office]);  // holds, holders, boatwardens, board, is_board_member
```
`ClubRecord::credited_hours`, `sailing_permit`, `boat_permit`, `holds` usw. bauen diese Strukturen aus ihren Listen und delegieren – damit bleiben alle bestehenden Core-Tests gültig und belegen die Gleichwertigkeit. Weitere Regeln (Reservierungsprüfung, Eignung, Rechte, Tourgebühren) werden in den Changes herausgelöst, die ihre Anwendungsfälle bauen.
*Alternative:* Teil-`ClubRecord` je Anwendungsfall laden (vom Nutzer verworfen: unscharf, welche Daten eine Regel braucht).

### 5. App-Schicht: gezielte Abfragen, Befehle, Sichtmodelle
- Ports sind **anwendungsfallbezogen** statt „lade alles“:
  - `MemberRepository`: `member(id) -> Option<Versioned<Member>>` (inkl. Scheine und Konto), `member_by_login_email(email)`, `active_members(t) -> Vec<Member>` (ohne Konto), `save_member(&Member, expected: Version) -> Version`
  - `OfficeRepository`: `active_offices(t) -> Vec<Office>`
  - `BoatRepository`: `boats() -> Vec<Boat>` (wenige Boote)
  - `YearRepository`: `member_year(member, year) -> MemberYearData` (Saison, Jahresnachweis, Einsätze und Überträge des Mitglieds im Jahr)
  - `PermitRepository`: `permit_history(member) -> (Vec<BoatCheckout>, Vec<Revocation>)`
  - `IdentityRepository` wie bisher, `Health::ping()`
  Storage implementiert sie mit SQL (Filter auf Mitglied, Jahr und Zeitraum in der Datenbank; Indizes auf den Fremdschlüssel- und Zeitspalten).
- **Versionierung je Aggregat:** Wurzeltabellen (z. B. `members`, `boats`, `reservations`, `work_entries`) erhalten eine Spalte `version`. Schreibbefehle aktualisieren mit `… WHERE id = ? AND version = ?` und erhöhen die Version; trifft keine Zeile, folgt `AppError::Conflict`. Die globale `record_meta`-Version entfällt. `Versioned<T> { value, version }` transportiert den Stand zur Oberfläche.
- **Invarianten** beim Schreiben: Datenbank (UNIQUE, Fremdschlüssel, CHECK) plus gezielte Prüfungen im jeweiligen Befehl; `ClubRecord::validate()` läuft nur noch beim Import über die ganze Akte.
- `CurrentUser { member }` ist Pflichtparameter aller Anwendungsfälle; Zugriffsentscheidungen (z. B. Kontaktdaten nur für Ämter) fallen hier über `Offices` – nicht im Template. Rückgaben sind **Sichtmodelle** (`MyDataView`, `DirectoryView`) mit nur den sichtbaren Daten.
- `Clock` liefert „jetzt“ in Vereinszeit; Tests nutzen eine feste Uhr (2026-10-09).
- `AuthService::resolve_login(LoginClaims { provider, subject, email })`: Verknüpfung über `subject`, sonst `member_by_login_email` (NOCASE) und aktive Mitgliedschaft → Verknüpfung speichern; nie Kontakt-E-Mail. Mandantenprüfung in der Web-Schicht.
- In-Memory-Implementierungen der Ports (für App- und Web-Tests) arbeiten auf einem `ClubRecord` aus der Beispielakte; das ist reine Testinfrastruktur.
- Fehler mit `thiserror`: `AppError::{NotFound, Forbidden, Conflict, Invalid(Vec<RecordIssue>), Storage(..)}`.

### 6. Web: axum + askama + htmx
- axum 0.8 Router; Handler sind dünn: Sitzung → `CurrentUser` (Extractor `RequireUser`, leitet sonst zu `/login?next=…` um) → App-Aufruf → askama-Template.
- askama-Templates unter `crates/vereinsakte-web/templates/`, Basislayout mit Navigation (Einträge abhängig von Ämtern), deutsche Texte, Formatierung von Geld/Datum über kleine Template-Filter (`Money` hat bereits deutsches `Display`).
- htmx wird als Datei eingebettet (`include_bytes!`) und unter `/static/htmx.min.js` mit Cache-Header ausgeliefert – kein CDN, keine Abhängigkeit zur Laufzeit. Die Suche im Verzeichnis nutzt `hx-get` auf `/mitglieder?q=…` mit Teil-Template (`HX-Request`-Header → nur Tabelle).
- CSRF: Token pro Sitzung, in Formularen als verstecktes Feld, für htmx global per `hx-headers` im Layout; Middleware prüft alle Nicht-GET/HEAD-Anfragen.
- Sicherheits-Header (CSP ohne Inline-Skripte, `X-Frame-Options: DENY`, `Referrer-Policy`) über `tower-http`.
- Routen: `/` (Weiterleitung), `/login`, `/auth/callback`, `/logout` (POST), `/meine-daten`, `/mitglieder`, `/health`, `/static/*`, im Entwicklungsmodus `/dev/login`.

### 7. Anmeldung: Microsoft Entra ID über `openidconnect`
- Aussteller ist `https://login.microsoftonline.com/{tenant_id}/v2.0` (Single-Tenant-App-Registrierung des Vereins); beim Start Discovery, Fehler → Start bricht ab (Produktion).
- `/login`: Authorization-Code-Flow mit PKCE, `state` und `nonce` in der Sitzung; Scopes `openid email profile`. Redirect-URI `{base_url}/auth/callback` muss in der App-Registrierung eingetragen sein.
- `/auth/callback`: `state` prüfen, Code tauschen, ID-Token verifizieren (Signatur, `nonce`, Aussteller, Audience) und zusätzlich `tid == tenant_id` prüfen. An `AuthService` gehen: `provider = Aussteller`, `subject = oid` (unveränderliche Objekt-ID, über Apps hinweg stabil; Entra-`sub` ist pro App paarweise), `email = email` oder ersatzweise `preferred_username`. Bei Erfolg Sitzungs-ID rotieren (Schutz vor Session-Fixation) und `member` in der Sitzung speichern.
- Entra ID liefert kein `email_verified`; die Adresse gilt als vertrauenswürdig, weil nur Konten des eigenen Mandanten angenommen werden und dessen Adressen (einheitlich aus der Microsoft-Domäne des Vereins) der Verein verwaltet. Verglichen wird mit dem Anmeldekonto des Mitglieds, nicht mit der Kontakt-E-Mail. In der App-Registrierung ist der optionale Claim `email` zu aktivieren (README).
- Tokens des Anbieters werden nicht gespeichert (keine API-Zugriffe nötig).
- **Entwicklungsmodus** (`mode = "development"`): Es wird kein Entra-Client gebaut und keine Discovery ausgeführt; `EntraConfig` ist optional. `/login` leitet auf `/dev/login` um (Auswahlliste aller aktiven Mitglieder mit ihren aktuellen Ämtern, gruppiert z. B. „Vorstand“, „Bootswarte“, „ohne Amt“). Zusätzlich gibt es `POST /dev/switch` (Formularfelder `member`, `next`, CSRF-Token): Sitzung rotieren, neues Mitglied setzen, zurück zu `next`. Das Basislayout bindet im Entwicklungsmodus eine farbig markierte Leiste „Entwicklungsmodus – angemeldet als …“ mit einer Auswahlliste ein, die beim Ändern (`hx-post` bzw. `onchange`-freies Formular mit Absende-Knopf, damit die CSP ohne Inline-Skripte auskommt) auf `/dev/switch` wechselt. Im Produktionsmodus werden weder die Routen registriert noch die Leiste gerendert (Flag im Template-Kontext aus der Konfiguration, nicht aus der Anfrage).
*Alternative:* eigene Passwörter (vom Nutzer verworfen zugunsten OIDC).

### 8. Sitzungen
`tower-sessions` 0.14 mit `tower-sessions-sqlx-store` (Tabelle `tower_sessions` in derselben SQLite-Datenbank, Sitzungsdaten als BLOB – keine Fachdaten); Ablauf bei Inaktivität (Konfiguration, Standard 8 h).

### 9. Binary, CLI und Konfiguration
- `clap` (derive) mit `serve | migrate | import <datei> [--replace] | export <datei>`.
- Konfiguration mit `figment`: `vereinsakte.toml` (Pfad per `--config`) + Umgebung `VEREINSAKTE_*` (verschachtelt mit `__`, z. B. `VEREINSAKTE_OIDC__ISSUER`). Struktur `Config { listen, port, database, base_url, mode, session_idle_minutes, entra: Option<EntraConfig> }` mit `EntraConfig { tenant_id, client_id, client_secret }`; `client_secret` als `SecretString`-Newtype mit maskiertem `Debug`. Umgebung z. B. `VEREINSAKTE_ENTRA__TENANT_ID`.
- Beispielkonfiguration `vereinsakte.example.toml` im Repo (Entwicklungsmodus, DB `vereinsakte.db`).
- Async-Laufzeit `tokio`; Logging `tracing` + `tracing-subscriber` (`EnvFilter` aus `RUST_LOG`) + `tower-http::TraceLayer`.

### 10. Tests und Testdaten
- Core: bestehende Tests wandern mit.
- Storage/App: SQLite `:memory:` + Beispielakte (Feature `example`); Spec-Szenarien als Tests (Version-Konflikt, ungültige Änderung, Import/Export, Fremdschlüssel- und CHECK-Verletzungen, Identitätsverknüpfung, Sichtbarkeit).
- Web: Router mit Entwicklungs-Login und fester Uhr, Anfragen per `oneshot`; Prüfen von Status, Weiterleitungen, Cookies und Inhalten (z. B. „20,5“ bei Clara, keine E-Mail-Adressen bei Greta).
- OIDC-Callback wird über eine austauschbare `IdTokenVerifier`-Abstraktion getestet (Fake liefert Claims); die Discovery gegen einen echten Anbieter bleibt manueller Test.

## Risks / Trade-offs

- [Zwei Wege zu den Regeln] → Solange `ClubRecord` die Regeln über die Teildaten-Strukturen delegiert, gibt es nur eine Implementierung; Tests auf beiden Wegen sichern die Gleichwertigkeit.
- [Abfragen müssen genau die Daten liefern, die eine Regel braucht] → die Eingabe-Strukturen machen das explizit; Storage-Tests vergleichen Ergebnisse der gezielten Abfragen mit der Berechnung auf der ganzen Beispielakte.
- [Invarianten über mehrere Datensätze] (z. B. Ämterhäufung im Vorstand) → nicht mehr durch Prüfen der ganzen Akte bei jedem Speichern, sondern im jeweiligen Befehl gezielt bzw. beim Import; die Befehle dafür kommen mit der Ämterverwaltung.
- [Zuordnung über Anmeldekonto] → Die erste Anmeldung ordnet über `login_email` zu; das ist nur sicher, weil ausschließlich Konten des Vereins-Mandanten akzeptiert werden. Danach gilt die Objekt-ID `oid`; spätere Änderungen der Adresse wirken nicht auf bestehende Verknüpfungen.
- [Anmeldekonten müssen gepflegt sein] → Bis eine Pflegeoberfläche existiert, trägt der Vorstand die Anmeldekonten per JSON-Import ein; fehlt eines, scheitert die Anmeldung mit Hinweis an den Vorstand.
- [Optionaler Claim `email` nicht aktiviert] → Ersatzweise `preferred_username` (UPN); README beschreibt die Einstellung in der App-Registrierung.
- [Ohne Mandant keine Entwicklung?] → Entwicklung und Tests laufen über den Entwicklungs-Login mit Benutzerwechsel bzw. einen Fake-`IdTokenVerifier`; Rollen werden über die Wahl eines Mitglieds mit dem passenden Amt getestet (Ämter kommen aus den Daten).
- [Entwicklungs-Login versehentlich produktiv] → Routen und Leiste hängen ausschließlich am Konfigurationswert `mode`; der Produktionsmodus verlangt eine vollständige Entra-Konfiguration, und ein Test stellt sicher, dass `/dev/login` und `/dev/switch` dort 404 liefern.
- [Relationales Schema bindet Modelländerungen an Migrationen] → jede neue Eigenschaft braucht eine Migration und Zuordnungscode; der Round-Trip-Test mit der Beispielakte schlägt fehl, wenn ein Feld vergessen wird.
- [Fremdschlüssel strenger als das Modell] → Akten mit Verweisen auf unbekannte Objekte sind nicht speicherbar; Import meldet den Verweis verständlich.
- [Crate-Versionen] → sqlx 0.9 verlangt Rust 1.94 (installiert: 1.91), daher **sqlx 0.8**; `tower-sessions-sqlx-store` 0.15 baut auf `tower-sessions-core` 0.14, daher **tower-sessions 0.14** (kompatibel mit axum 0.8). Ein Wechsel auf sqlx 0.9 ist nach einem Toolchain-Update eine reine Versionsanhebung.
- [Verschieben des Modells] → große, rein mechanische Änderung; durch die unveränderten Core-Tests abgesichert (gleiche Anzahl, alle grün).

## Migration Plan

1. Workspace anlegen, Modell verschieben, Core-Tests grün (keine Verhaltensänderung).
2. Neue Crates schrittweise hinzufügen; das alte `src/` entfällt mit dem neuen Binary.
3. Lokale Inbetriebnahme: `vereinsakte migrate`, `vereinsakte import crates/vereinsakte-core/testdata/vereinsakte_beispiel.json`, `vereinsakte serve` im Entwicklungsmodus.
Rückweg: Git-Revert; es gibt noch keine Produktivdaten.


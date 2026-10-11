# Vereinsakte

Verwaltung der Vereinsakte der Segelgruppe: Mitglieder, Boote, Ämter, Segel- und
Bootserlaubnisse, Arbeitsstunden, Beiträge und Konten. Die Fachregeln sind in
`openspec/specs` beschrieben.

## Architektur

Cargo-Workspace mit Abhängigkeiten nur in einer Richtung:

```
vereinsakte (Binary: Konfiguration, CLI, Zusammenbau)
 ├─ vereinsakte-web      Router, Templates (askama), htmx, Anmeldung, Sitzungen
 │    └─ vereinsakte-app      Anwendungsfälle und Ports (Repositories, Uhr, Health)
 │         └─ vereinsakte-core     Fachmodell und Fachregeln, ohne I/O
 └─ vereinsakte-storage  SQLite-Implementierung der Ports (sqlx)
      └─ vereinsakte-app
```

Die Web-Schicht kennt die Speicherschicht nicht; sie bekommt Ports und
Sitzungsspeicher vom Binary übergeben.

### Datenbankschema (SQLite, relational, `crates/vereinsakte-storage/migrations`)

| Bereich | Tabellen |
| --- | --- |
| Mitglieder | `members`, `member_sailing_licenses`, `member_radio_licenses` |
| Boote | `boats`, `crew_rules`, `requirement_groups`, `requirement_qualifications`, `boat_releases`, `boat_blocks` |
| Ämter | `offices` |
| Bootserlaubnis | `boat_checkouts`, `revocations` |
| Arbeitsstunden | `work_events`, `work_event_registrations`, `tasks`, `task_assignees`, `work_entries`, `hour_transfers` |
| Saisons und Gebühren | `seasons`, `season_type_rates`, `season_tour_rates`, `annual_records` |
| Reservierungen | `reservations`, `reservation_crew`, `reservation_guests` |
| Konto | `claims`, `payments` |
| Anmeldung | `identities` (Verknüpfung Entra-Konto ↔ Mitglied), Sitzungstabelle |
| Protokoll | `audit_log` (unveränderlich, nur Kennungen und Feldnamen) |

Die Wurzeltabellen tragen eine `version`-Spalte für optimistische Sperren
(gleichzeitige Änderungen führen zu einem Konflikt statt zu Datenverlust).

## Lokaler Start

```sh
cp vereinsakte.example.toml vereinsakte.toml     # mode = "development"
cargo run -p vereinsakte -- -c vereinsakte.toml migrate
cargo run -p vereinsakte -- -c vereinsakte.toml import akte.json   # JSON-Sicherung der Vereinsakte
cargo run -p vereinsakte -- -c vereinsakte.toml serve
```

Eine Beispielakte liegt in `crates/vereinsakte-core/testdata/vereinsakte_beispiel.json`.
Dann ist `http://localhost:8080/health` erreichbar, und `/login` führt im
Entwicklungsmodus zu einer Auswahlliste der Mitglieder (`/dev/login`).

Weitere Befehle: `import <datei> --replace` ersetzt den Bestand,
`export <datei>` schreibt eine Sicherung. Die Protokollstufe steuert `RUST_LOG`.
Jeder Konfigurationswert lässt sich per `VEREINSAKTE_*` überschreiben, verschachtelte
Werte mit `__` (z. B. `VEREINSAKTE_ENTRA__CLIENT_SECRET`).

## Anmeldung mit Microsoft Entra ID (Produktion)

1. In Entra ID unter *App-Registrierungen* eine neue Registrierung anlegen,
   Kontotyp **nur dieses Verzeichnis (Single-Tenant)**.
2. Als Redirect-URI (Plattform *Web*) `<base_url>/auth/callback` eintragen.
3. Unter *Zertifikate & Geheimnisse* ein Client-Geheimnis erzeugen.
4. Unter *Token-Konfiguration* den optionalen Claim `email` für das ID-Token
   hinzufügen (ohne ihn wird `preferred_username` verwendet).
5. Konfiguration (`mode = "production"`):

   ```toml
   base_url = "https://akte.example.org"
   [entra]
   tenant_id = "<Verzeichnis-ID>"
   client_id = "<Anwendungs-ID>"
   # client_secret per VEREINSAKTE_ENTRA__CLIENT_SECRET setzen
   ```

Ein Mitglied meldet sich mit dem im Mitgliedsdatensatz hinterlegten Anmeldekonto
(`login_email`) an; die Kontakt-E-Mail wird dafür nie verwendet. Beim ersten
Login wird die Objekt-ID (`oid`) des Kontos mit dem Mitglied verknüpft.

## Rechte und Bedienung

Die Rechte folgen den Ämtern zum aktuellen Zeitpunkt; ein Amtswechsel gilt ab der
nächsten Anfrage. Nicht erlaubte Aktionen werden weder angezeigt noch ausgeführt
(403). Kassenwart und Oberbootswart gehören nach der Satzung zum Vorstand.

| Bereich | Mitglied | Bootswart / Abnehmer (Boot) | Vorstand | Revisor |
| --- | --- | --- | --- | --- |
| Eigene Kontaktdaten, Scheine, Lizenzen | ändern | ändern | ändern | ändern |
| Mitglieder aufnehmen, Stammdaten, Austritt | – | – | ja | lesen |
| Ämter | ansehen, eigenes abgeben | wie Mitglied | ernennen, abgeben, Historie | ansehen |
| Boote | ansehen | eigenes Boot ändern, sperren, freigeben, abnehmen | alle; Widerruf; Boot anlegen | ansehen |
| Arbeitseinsätze | eigene erfassen, solange unbestätigt | Einsätze des Bootes bestätigen | alle ändern, Übertrag, Abgeltung | lesen |
| Arbeitstermine, Aufgaben | anmelden; eigene Aufgaben | Aufgaben anlegen | anlegen, ändern | ansehen |
| Reservierungen | eigene anlegen, ändern, stornieren | alle Reservierungen des Bootes | alle | ansehen |
| Konto, Gebühren, Außenstände | eigenes Konto | – | alle buchen, stornieren, Saisons | alle lesen |
| Protokoll | eigene Einträge | eigene Einträge | alle Einträge | alle Einträge |

Kurz je Rolle:

- **Mitglied:** „Meine Daten“ (Kontaktdaten, Scheine und Lizenzen ohne Nachweis), Einsätze erfassen, zu Terminen anmelden, Boote reservieren, eigenes Konto.
- **Bootswart/Abnehmer:** zusätzlich Boot sperren und freigeben, Abnahmen erfassen, offene Arbeitseinsätze bestätigen (nie die eigenen).
- **Vorstand:** Mitglieder und Ämter verwalten, Widerrufe, Arbeitstermine, Stundenübertrag und Abgeltung, Konten, Saisons und Gebührensätze, Jahresbeiträge und Tourgebühren anlegen, Protokoll einsehen.
- **Revisor:** liest alle Konten, Stunden und das Protokoll, ändert nichts.

**Sichtbarkeit von Kontaktdaten:** E-Mail-Adresse und Telefonnummern sind unter allen
aktiven Mitgliedern sichtbar (Mitgliederliste, Mitgliedsseite, Ämterübersicht). Jedes
Mitglied kann unter „Meine Daten“ einen **Sperrvermerk** setzen; dann sehen nur das
Mitglied selbst und der Vorstand die Angaben, alle anderen „Kontaktdaten gesperrt“.
Anschrift, Segelerlaubnis, Stunden und Konto bleiben auf das Mitglied, den Vorstand und
die Revisoren beschränkt. Nach dem Update zeigt die Startseite jedem Mitglied einmalig
einen Hinweis auf die geänderte Sichtbarkeit; der Verein sollte seine Mitglieder
zusätzlich vorab darüber informieren.

Jede schreibende Aktion steht mit Urheber und Zeitpunkt im Protokoll (`/protokoll`);
dort stehen nur Kennungen und Feldnamen, keine Beträge oder Kontaktdaten. Gleichzeitige
Änderungen am selben Datensatz werden erkannt (HTTP 409) statt überschrieben.

### Rollen im Entwicklungsmodus testen

Im Entwicklungsmodus erscheint oben eine gelbe Leiste: Dort lässt sich jederzeit zu einem
anderen Mitglied wechseln. Mit der Beispielakte: Eva (1. Vorsitzende), Anna (2. Vorsitzende),
Jonas (Kassenwart), Karin (Schriftwart), Frank (Oberbootswart), Ben (Bootswart Harald Blauzahn),
David (Bootswart und Abnehmer Filou), Clara und Lukas (Revisoren), Greta (ohne Amt).

### Sicherung

`vereinsakte export <datei>` sichert die Akte als JSON; das Änderungsprotokoll und die
Anmeldeverknüpfungen gehören bewusst nicht dazu. Für eine vollständige Sicherung also die
Datenbankdatei (`vereinsakte.db`, im Betrieb samt `-wal`/`-shm`) kopieren.

### Bildschirmfotos zur Prüfung des Designs

```sh
cd tools && npm install && cd ..
node tools/screenshots.mjs /tmp/screens   # 375 px und 1280 px, hell und dunkel
```

## Entwicklung

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
```

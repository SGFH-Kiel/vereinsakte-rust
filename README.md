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

## Entwicklung

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
```

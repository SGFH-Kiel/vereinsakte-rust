# Proposal

## Why

Das Datenmodell der Vereinsakte steht (Change `add-datenmodell`), ist aber nur eine Sammlung von Typen ohne Anwendung: keine Speicherung, keine Oberfläche, kein Zugang für Mitglieder. Bevor einzelne Fachfunktionen (Stunden, Reservierungen, Konto) gebaut werden, braucht die Anwendung ein tragfähiges Gerüst: klare Schichten, dauerhafte Speicherung, Anmeldung und eine erste durchgehende Funktion, an der sich alle weiteren orientieren.

## What Changes

- **BREAKING (intern)**: Das Crate wird zu einem Cargo-Workspace; das bestehende Modul `model` wird zur Library `vereinsakte-core` und bleibt fachlich unverändert bis auf das neue Anmeldekonto.
- Neue Crates mit festen Abhängigkeitsrichtungen: `vereinsakte-core` (Fachmodell), `vereinsakte-storage` (SQLite), `vereinsakte-app` (Anwendungsfälle, Zugriffsprüfung), `vereinsakte-web` (HTTP-Server, Oberfläche, Anmeldung) und das Programm `vereinsakte` (Kommandozeile: Server starten, Datenbank migrieren, JSON importieren/exportieren).
- **Datenhaltung** in SQLite als führender Datenquelle: gezielte Abfragen und Änderungen je Anwendungsfall, Versionsstand je Datensatz statt für die ganze Akte, vollständig relationales Schema (eine Spalte je Eigenschaft, Untertabellen für Listen, Fremdschlüssel für Verweise, keine JSON-Spalten) und versionierten Migrationen; Import und Export der Vereinsakte als JSON im bestehenden Format nur noch für Sicherung und Wiederherstellung; Erkennung gleichzeitiger Änderungen.
- **Fachmodell**: Die vom Durchstich genutzten Fachregeln (Segelerlaubnis mit Stunden und Beitragsrückstand, Bootserlaubnis, Ämter) arbeiten auf kleinen Eingabe-Strukturen mit genau den nötigen Daten statt auf der ganzen Akte; `ClubRecord` delegiert an sie und bleibt Format für Sicherung/Import.
- **Fachmodell**: Mitglieder erhalten ein optionales, eindeutiges Anmeldekonto (`login_email`, Adresse des Microsoft-Kontos), getrennt von der Kontakt-E-Mail.
- **Anmeldung** über Microsoft Entra ID (OpenID Connect, nur Konten des Vereins-Mandanten); Zuordnung der Identität zu einem Mitglied über dessen Anmeldekonto; Sitzungen per Cookie; ein Entwicklungs-Login nur im Entwicklungsmodus.
- **Web-Oberfläche** mit serverseitig erzeugtem HTML und htmx; Grundlayout mit Navigation abhängig von Ämtern.
- **Durchstich**: Seite „Meine Daten“ (Stammdaten, Scheine, Segelerlaubnis, Arbeitsstunden, Kontostand) und ein Mitgliederverzeichnis mit Sichtbarkeit nach Amt.
- **Betrieb**: Konfiguration über Datei und Umgebungsvariablen, Health-Check, strukturiertes Logging.

Nicht im Umfang: Bearbeiten von Daten über die Oberfläche (außer der Anmeldung), weitere Fachseiten (Stunden erfassen, Reservieren, Konto, Ämterverwaltung), E-Mail-Versand, Deployment/Hosting, Mandantenfähigkeit, Barrierefreiheits-Audit.

## Capabilities

### New Capabilities
- `datenhaltung`: Dauerhafte Speicherung der Vereinsakte in einem relationalen SQLite-Schema, referentielle Integrität, Migrationen, JSON-Import/-Export, Schutz vor gleichzeitigen Änderungen.
- `anmeldung`: Anmeldung über Microsoft Entra ID (OIDC), Zuordnung zu Mitgliedern, Sitzungen, Abmeldung, Entwicklungs-Login.
- `mitgliederbereich`: Seite „Meine Daten“ eines angemeldeten Mitglieds.
- `mitgliederverzeichnis`: Liste der Mitglieder mit amtsabhängiger Sichtbarkeit von Kontaktdaten.
- `betrieb`: Konfiguration, Kommandozeile, Health-Check und Logging der Anwendung.

### Modified Capabilities
- `mitglieder`: neues Anmeldekonto (Adresse des Microsoft-Kontos) je Mitglied, getrennt von der Kontakt-E-Mail und eindeutig.

## Impact

- Code: `src/model/*` wandert nach `crates/vereinsakte-core/src/`; `src/main.rs` entfällt zugunsten von `crates/vereinsakte/`. Neue Crates `vereinsakte-storage`, `vereinsakte-app`, `vereinsakte-web`.
- Neue Abhängigkeiten u. a.: `tokio`, `axum`, `askama`, `sqlx` (SQLite), `openidconnect`, `tower-sessions`, `clap`, `tracing`, `serde_json`/`toml` für Konfiguration.
- Neue Dateien: SQL-Migrationen (relationales Schema), Zuordnungscode zwischen Tabellenzeilen und Fachobjekten, HTML-Templates, gebündeltes `htmx.min.js`, Beispielkonfiguration.
- Externe Abhängigkeit zur Laufzeit: der Microsoft-Entra-ID-Mandant des Vereins mit einer App-Registrierung (Produktivbetrieb); für die Entwicklung nicht nötig.

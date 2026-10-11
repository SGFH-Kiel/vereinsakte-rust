# Spec Delta

## Purpose

Legt fest, wie die Anwendung gestartet, konfiguriert und überwacht wird, damit sie lokal entwickelt und später auf einem Server betrieben werden kann.

## ADDED Requirements

### Requirement: Kommandozeile
Das Programm `vereinsakte` SHALL die Befehle `serve` (Webserver starten), `migrate` (Datenbank auf aktuellen Stand bringen), `import <datei>` (mit Option zum Ersetzen) und `export <datei>` anbieten und bei falscher Bedienung eine Hilfe ausgeben.

#### Scenario: Unbekannter Befehl
- **WHEN** `vereinsakte frobnicate` aufgerufen wird
- **THEN** endet das Programm mit Fehlercode und nennt die verfügbaren Befehle

### Requirement: Konfiguration
Die Anwendung SHALL aus einer TOML-Datei konfiguriert werden; jeder Wert SHALL sich über Umgebungsvariablen mit Präfix `VEREINSAKTE_` überschreiben lassen. Konfigurierbar SHALL sein: Adresse und Port, Datenbankpfad, öffentliche Basis-URL, Modus (Entwicklung oder Produktion), Tenant-ID, Client-ID und Client-Geheimnis der Entra-App-Registrierung sowie die Sitzungsdauer.

#### Scenario: Überschreiben per Umgebung
- **WHEN** die Datei Port 8080 festlegt und `VEREINSAKTE_PORT=9000` gesetzt ist
- **THEN** lauscht der Server auf Port 9000

### Requirement: Anmelde-Konfiguration je Modus
Im Produktionsmodus SHALL der Start mit einer verständlichen Meldung abbrechen, wenn eine Angabe zur Entra-Anmeldung fehlt. Im Entwicklungsmodus SHALL die Anmelde-Konfiguration optional sein.

#### Scenario: Entwicklungsmodus ohne Entra
- **WHEN** im Entwicklungsmodus keine Entra-Angaben konfiguriert sind
- **THEN** startet der Server

#### Scenario: Fehlende Anmelde-Konfiguration
- **WHEN** im Produktionsmodus keine Tenant-ID konfiguriert ist
- **THEN** startet der Server nicht und meldet den fehlenden Wert

### Requirement: Geheimnisse nicht protokollieren
Client-Geheimnis, Sitzungs-IDs und Tokens SHALL in keiner Log-Ausgabe und keiner Fehlermeldung an Nutzer erscheinen.

#### Scenario: Konfiguration im Log
- **WHEN** die geladene Konfiguration beim Start protokolliert wird
- **THEN** erscheint das Client-Geheimnis maskiert

### Requirement: Health-Check
Der Server SHALL unter `/health` ohne Anmeldung den Status liefern: erfolgreich, wenn die Datenbank erreichbar ist, sonst mit Fehlerstatus.

#### Scenario: Datenbank erreichbar
- **WHEN** `/health` bei laufender Anwendung aufgerufen wird
- **THEN** antwortet der Server mit Status 200

### Requirement: Strukturiertes Logging
Jede HTTP-Anfrage SHALL mit Methode, Pfad, Status und Dauer protokolliert werden; die Ausführlichkeit SHALL über die Umgebungsvariable `RUST_LOG` einstellbar sein.

#### Scenario: Anfrage protokolliert
- **WHEN** `/health` aufgerufen wird
- **THEN** enthält das Log einen Eintrag mit `GET`, `/health` und Status 200

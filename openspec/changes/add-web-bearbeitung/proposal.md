# Proposal

## Why

Die Weboberfläche zeigt bisher nur „Meine Daten“ und das Mitgliederverzeichnis. Alle anderen Daten der Vereinsakte (Boote, Ämter, Arbeitsstunden, Reservierungen, Konten, Gebühren) lassen sich nur über JSON-Import und -Export ändern. Damit kann der Verein die Akte im Alltag nicht führen: Vorstand, Bootswarte und Mitglieder brauchen Lese- und Schreibzugriff im Browser, jeweils begrenzt durch das Amt, das sie gerade innehaben.

## What Changes

- Alle Bereiche der Vereinsakte werden im Web les- und schreibbar: Mitglieder, Ämter, Boote (inkl. Sperre, Freigabe, Abnahmen, Widerrufe), Arbeitseinsätze, Arbeitstermine, Aufgaben, Reservierungen, Saisons und Gebühren, Forderungen, Einzahlungen und Tourgebühren.
- Ein einheitliches Rechtemodell legt je Bereich fest, wer lesen und wer schreiben darf. Es leitet sich aus den Ämtern zum aktuellen Zeitpunkt ab; die Regeln aus der Spec `aemter` (Sperren, Freigaben, Bestätigungen, Abnahmen, Widerrufe) bleiben unverändert maßgeblich. Nicht erlaubte Aktionen werden weder angeboten noch ausgeführt.
- Jede Änderung wird mit Urheber und Zeitpunkt in einem Änderungsprotokoll festgehalten, das Vorstand und Revisoren einsehen können.
- Ein einheitliches, funktionales Design: modern, ohne Frameworks und ohne Build-Schritt, auf Desktop und Mobilgeräten bedienbar, auch ohne JavaScript nutzbar (htmx nur als Verbesserung).
- Gleichzeitige Änderungen werden erkannt (Versionsprüfung der Formulare) statt überschrieben.
- **BREAKING**: keine. Bestehende Seiten, Routen und der JSON-Import/-Export bleiben erhalten; das Datenbankschema erhält nur eine zusätzliche Tabelle.

## Capabilities

### New Capabilities
- `rechte`: Rollen und Rechtematrix für Lesen und Schreiben je Bereich, abgeleitet aus den aktuellen Ämtern; Verhalten bei fehlender Berechtigung.
- `oberflaeche`: Design, Navigation, responsives Layout, Formular- und Tabellenverhalten, Rückmeldungen und Barrierefreiheit der Weboberfläche.
- `aenderungsprotokoll`: Protokoll aller schreibenden Aktionen mit Urheber, Zeitpunkt, Gegenstand und Einsicht.
- `mitgliederverwaltung`: Mitglieder ansehen, Kontaktdaten sowie Scheine und Lizenzen (ohne Nachweis, auf Vertrauensbasis) selbst pflegen, Mitglieder aufnehmen, ändern und austreten lassen.
- `amtsverwaltung`: Ämter einsehen, ernennen und abgeben.
- `bootsverwaltung`: Boote und ihre Anforderungen pflegen, sperren, freigeben sowie Abnahmen und Widerrufe erfassen.
- `arbeitsverwaltung`: Arbeitseinsätze, Bestätigungen, Arbeitstermine mit Anmeldungen, Aufgaben, Stundenüberträge und Abgeltungen.
- `reservierungsverwaltung`: Reservierungen einsehen, anlegen, ändern und stornieren mit allen fachlichen Prüfungen.
- `finanzverwaltung`: Saisons und Gebührensätze, Jahresnachweise, Forderungen, Einzahlungen, Storno, Außenstände und Tourgebühren.

### Modified Capabilities
<!-- keine: bestehende Anforderungen ändern sich nicht -->

## Impact

- `crates/vereinsakte-core`: Berechtigungsprüfung aus `ClubRecord::authorize` auf Teildaten herauslösen (`Authorizer`), neue Rechte-Aufzählung; Fachregeln bleiben unverändert.
- `crates/vereinsakte-app`: schreibende Anwendungsfälle je Bereich und neue Schreib-Ports mit Versionsprüfung; Änderungsprotokoll.
- `crates/vereinsakte-storage`: Migration `0002` (Tabelle `audit_log`), SQLite-Implementierungen der Schreib-Ports, jeweils in einer Transaktion samt Protokolleintrag.
- `crates/vereinsakte-web`: Formular-Handler, Templates, Stylesheet und (optional) ein kleines eigenes Skript für die mobile Navigation; Content-Security-Policy bleibt streng (`'self'`).
- Keine neuen Laufzeit-Abhängigkeiten außer ggf. einem Test-Hilfsmittel für Bildschirmfotos.

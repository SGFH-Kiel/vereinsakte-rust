# datenhaltung Specification

## Purpose

Speichert die Vereinsakte dauerhaft in einem relationalen SQLite-Schema, das die führende Datenquelle ist, hält das Schema über versionierte Migrationen aktuell und erlaubt Sicherung und Wiederherstellung der gesamten Akte im bestehenden JSON-Format.

## Requirements

### Requirement: Verlustfreie Speicherung
Eine Vereinsakte SHALL in der Datenbank gespeichert und wieder geladen werden können, ohne dass Informationen verloren gehen.

#### Scenario: Speichern und Laden
- **WHEN** die vollständige Beispielakte gespeichert und anschließend geladen wird
- **THEN** ist die geladene Akte gleich der gespeicherten

#### Scenario: Leere Datenbank
- **WHEN** aus einer frisch angelegten Datenbank geladen wird
- **THEN** ergibt sich eine leere Vereinsakte

### Requirement: Relationales Schema
Jede Eigenschaft eines Fachobjekts SHALL in einer eigenen, typisierten Spalte gespeichert werden; Listen (z. B. Segelscheine, Crew, Gäste, Anforderungsgruppen, Anmeldungen, Forderungen) SHALL in eigenen Tabellen mit Verweis auf ihr Elternobjekt stehen. Spalten mit JSON-Dokumenten SHALL nicht verwendet werden. Auswahlwerte SHALL nur die im Fachmodell definierten Werte annehmen können.

#### Scenario: Abfrage mit SQL
- **WHEN** die Datenbank mit einem SQL-Werkzeug nach allen Mitgliedern mit SKS gefragt wird
- **THEN** genügt eine Verknüpfung der Tabellen für Mitglieder und Segelscheine ohne JSON-Funktionen

#### Scenario: Doppeltes Anmeldekonto in der Datenbank
- **WHEN** direkt per SQL ein zweites Mitglied mit dem Anmeldekonto `ANNA.PETERSEN@sgfh.example` eingetragen werden soll
- **THEN** lehnt die Datenbank den Eintrag ab

#### Scenario: Unzulässiger Auswahlwert
- **WHEN** direkt per SQL eine Mitgliedsart „vip“ eingetragen werden soll
- **THEN** lehnt die Datenbank den Eintrag ab

### Requirement: Referentielle Integrität
Verweise zwischen Fachobjekten (z. B. Mitglied, Boot, Reservierung, Termin, Aufgabe) SHALL als Fremdschlüssel gespeichert werden; ein Verweis auf ein nicht vorhandenes Objekt SHALL nicht gespeichert werden können.

#### Scenario: Import mit unbekanntem Verweis
- **WHEN** eine Akte importiert wird, deren Arbeitseinsatz auf eine nicht vorhandene Aufgabe verweist
- **THEN** wird der Import mit einem Hinweis auf den unbekannten Verweis abgelehnt und die Datenbank bleibt unverändert

### Requirement: Migrationen
Das Datenbankschema SHALL über versionierte Migrationen angelegt und weiterentwickelt werden. Beim Start der Anwendung und mit einem eigenen Befehl SHALL eine Datenbank auf den aktuellen Stand gebracht werden; bereits angewandte Migrationen SHALL nicht erneut ausgeführt werden.

#### Scenario: Neue Datenbank
- **WHEN** die Migration auf eine nicht vorhandene Datenbankdatei angewandt wird
- **THEN** wird die Datei mit dem aktuellen Schema angelegt

#### Scenario: Wiederholte Migration
- **WHEN** die Migration auf eine bereits aktuelle Datenbank erneut angewandt wird
- **THEN** ändert sich nichts und es wird kein Fehler gemeldet

### Requirement: Gezielte Zugriffe
Lesende und schreibende Zugriffe SHALL nur die Datensätze betreffen, die der jeweilige Anwendungsfall braucht; das Laden der gesamten Vereinsakte SHALL auf Sicherung (Export) und Wiederherstellung (Import) beschränkt sein.

#### Scenario: Meine Daten
- **WHEN** „Meine Daten“ für Anna angezeigt wird
- **THEN** werden nur Annas Mitgliedsdaten, ihre Einsätze und Überträge des Jahres, ihr Jahresnachweis, die Saison, ihre Abnahmen und Widerrufe sowie die Boote gelesen – nicht die Daten anderer Mitglieder

### Requirement: Änderungen in Transaktionen
Jede Änderung SHALL vollständig oder gar nicht gespeichert werden. Verstößt sie gegen Regeln, die die Datenbank prüft (Eindeutigkeit, Fremdschlüssel, zulässige Werte), SHALL nichts gespeichert werden.

#### Scenario: Ungültige Änderung
- **WHEN** einem Mitglied ein Anmeldekonto gegeben werden soll, das bereits ein anderes Mitglied hat
- **THEN** wird die Änderung abgelehnt und die Datenbank bleibt unverändert

### Requirement: Schutz vor gleichzeitigen Änderungen
Jeder eigenständig änderbare Datensatz (z. B. ein Mitglied samt Scheinen und Konto) SHALL einen eigenen Versionsstand tragen. Eine Änderung, die auf einem veralteten Stand dieses Datensatzes beruht, SHALL abgelehnt werden; Änderungen an verschiedenen Datensätzen SHALL sich nicht gegenseitig behindern.

#### Scenario: Zwei Bearbeitungen desselben Mitglieds
- **WHEN** zwei Bearbeitungen denselben Stand von Mitglied 101 laden, die erste speichert und danach die zweite speichern will
- **THEN** wird die zweite mit dem Hinweis auf eine zwischenzeitliche Änderung abgelehnt

#### Scenario: Bearbeitungen verschiedener Mitglieder
- **WHEN** gleichzeitig Mitglied 101 und Mitglied 102 bearbeitet und gespeichert werden
- **THEN** werden beide Änderungen gespeichert

### Requirement: JSON-Import und -Export
Die gesamte Vereinsakte SHALL als Sicherung im bestehenden JSON-Format (siehe `datenformat`) exportiert und importiert werden können. Ein Import SHALL nur in eine leere Datenbank oder mit ausdrücklicher Bestätigung zum Ersetzen erfolgen und SHALL eine Akte ablehnen, die die Prüfung der Vereinsakte nicht besteht.

#### Scenario: Beispielakte importieren
- **WHEN** die JSON-Beispieldatei in eine leere Datenbank importiert und danach exportiert wird
- **THEN** stimmt der Export inhaltlich mit der Beispieldatei überein

#### Scenario: Import in gefüllte Datenbank
- **WHEN** ohne Bestätigung in eine Datenbank importiert wird, die bereits Mitglieder enthält
- **THEN** wird der Import abgelehnt und die Datenbank bleibt unverändert

#### Scenario: Ungültige Akte
- **WHEN** eine JSON-Datei mit doppelter Mitgliedsnummer importiert wird
- **THEN** wird der Import mit der Meldung der Prüfung abgelehnt

### Requirement: Verknüpfung von Anmeldeidentitäten
Die Datenhaltung SHALL zu jedem Mitglied höchstens eine externe Anmeldeidentität (Anbieter und Subjekt-Kennung) speichern; eine Identität SHALL höchstens einem Mitglied zugeordnet sein. Diese Verknüpfung SHALL nicht Teil des JSON-Exports der Vereinsakte sein.

#### Scenario: Identität doppelt verknüpfen
- **WHEN** eine bereits mit Mitglied 101 verknüpfte Identität mit Mitglied 102 verknüpft werden soll
- **THEN** wird die Verknüpfung abgelehnt

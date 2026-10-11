# mitgliederverwaltung Specification

## Purpose

Ermöglicht im Web, Mitglieder anzusehen und zu verwalten: Mitglieder pflegen Kontaktdaten, Scheine und Lizenzen selbst, der Vorstand nimmt Mitglieder auf und ändert Stammdaten, Mitgliedsart und Mitgliedschaftsverlauf.

## Requirements

### Requirement: Mitgliedsseite
Zu jedem aktiven Mitglied SHALL es eine Seite mit Name, Mitgliedsnummer, Mitgliedsart, Ämtern und Eintrittsdatum geben. Kontaktdaten, Anschrift, Segelerlaubnis, Konto und Arbeitsstunden SHALL nur auf der Seite des eigenen Mitglieds, für den Vorstand und im Rahmen der Spec `rechte` erscheinen.

#### Scenario: Greta sieht Clara
- **WHEN** Greta die Seite von Clara öffnet
- **THEN** sieht sie Name und Ämter, aber keine Kontaktdaten und kein Konto

#### Scenario: Vorstand sieht alles
- **WHEN** Eva die Seite von Clara öffnet
- **THEN** sieht sie Kontaktdaten, Segelerlaubnis, Arbeitsstunden und Konto

### Requirement: Kontaktdaten selbst pflegen
Ein Mitglied SHALL seine Anschrift, Telefonnummern und Kontakt-E-Mail ändern können. Mitgliedsnummer, Mitgliedsart, Mitgliedschaftsverlauf und Anmeldekonto SHALL es nicht ändern können. Die Meldeadresse SHALL geprüft werden.

#### Scenario: Eigene Änderung
- **WHEN** Anna ihre Straße ändert und speichert
- **THEN** zeigt „Meine Daten“ die neue Straße

#### Scenario: Unzulässiges Feld
- **WHEN** Greta per manuell gesendetem POST ihre Mitgliedsart ändert
- **THEN** bleibt die Mitgliedsart unverändert

### Requirement: Mitglied aufnehmen
Der Vorstand SHALL ein Mitglied mit Name, Mitgliedsart, Eintrittsdatum und Kontaktdaten aufnehmen können. Das System SHALL die nächste freie Mitgliedsnummer vorschlagen und eine doppelte Nummer ablehnen. Für ein außerordentliches Mitglied SHALL die Vorstandszustimmung nach der Spec `mitglieder` erfasst werden, ein Anmeldekonto SHALL optional und eindeutig sein.

#### Scenario: Neue Nummer
- **WHEN** der Vorstand ein Mitglied ohne Nummerneingabe aufnimmt
- **THEN** erhält es die nächste freie Mitgliedsnummer

#### Scenario: Doppelte Nummer
- **WHEN** der Vorstand eine bereits vergebene Mitgliedsnummer eingibt
- **THEN** zeigt das Formular die Fehlermeldung und speichert nichts

#### Scenario: Doppeltes Anmeldekonto
- **WHEN** das Anmeldekonto ohne Beachtung der Schreibweise bereits einem Mitglied gehört
- **THEN** wird dies am Feld gemeldet

### Requirement: Stammdaten und Mitgliedschaft ändern
Der Vorstand SHALL Stammdaten, Mitgliedsart, Anmeldekonto und den Verlauf der Mitgliedschaft (Austritt, Ausschluss, Wechsel der Art) ändern können. Beendete Mitgliedschaften SHALL als Verlauf sichtbar bleiben; ein ausgeschiedenes Mitglied SHALL aus dem Verzeichnis und der Anmeldung verschwinden, seine Daten aber erhalten bleiben.

#### Scenario: Austritt
- **WHEN** der Vorstand für Inès einen Austritt zum 30.11.2026 erfasst
- **THEN** steht sie im Dezember nicht mehr im Verzeichnis, ihre Daten und ihr Verlauf bleiben einsehbar

### Requirement: Scheine und Lizenzen selbst pflegen
Ein Mitglied SHALL seine eigenen Segelscheine mit Art, Datum und freier Bezeichnung sowie seine Funklizenzen erfassen, ändern und entfernen können. Das System SHALL dafür weder einen Nachweis verlangen noch speichern und keine Freigabe durch den Verein vorsehen. Der Vorstand SHALL Scheine und Lizenzen jedes Mitglieds ebenso pflegen können. Die Segelerlaubnis SHALL nach dem Speichern sofort neu berechnet erscheinen.

#### Scenario: Schein ergänzen
- **WHEN** Frank auf „Meine Daten“ einen SKS-Schein erfasst
- **THEN** zeigt seine Seite „SKS“, seine Segelerlaubnis ist neu berechnet und das Verzeichnis mit Filter „SKS“ enthält ihn

#### Scenario: Kein Nachweis
- **WHEN** ein Mitglied einen Schein erfasst
- **THEN** enthält das Formular kein Feld für einen Nachweis oder Dateiupload

#### Scenario: Fremder Schein
- **WHEN** Greta den Schein von Clara ändern will
- **THEN** wird dies mit 403 abgelehnt

#### Scenario: Vorstand pflegt mit
- **WHEN** Eva für Clara eine Funklizenz ergänzt
- **THEN** wird sie gespeichert

### Requirement: Änderungen an Scheinen sind nachvollziehbar
Jede Änderung an Segelscheinen und Funklizenzen SHALL im Änderungsprotokoll mit handelndem Mitglied und Art der Änderung (hinzugefügt, geändert, entfernt) erscheinen, damit die Vertrauensbasis nachvollziehbar bleibt.

#### Scenario: Eintrag bei Entfernen
- **WHEN** Frank seinen SKS-Schein entfernt
- **THEN** enthält das Protokoll „Segelschein entfernt“ mit Frank als Handelndem

### Requirement: Gleichzeitige Bearbeitung
Ein Formular SHALL die Version des Mitglieds enthalten, auf der es beruht. Wurde das Mitglied zwischenzeitlich geändert, SHALL das Speichern abgelehnt, nichts verändert und der aktuelle Stand mit den eingegebenen Werten zum Abgleich angezeigt werden.

#### Scenario: Konflikt
- **WHEN** Eva und Anna dasselbe Mitglied öffnen und Eva zuerst speichert
- **THEN** erhält Anna beim Speichern einen Hinweis auf die zwischenzeitliche Änderung und ihre Eingaben bleiben im Formular

# Spec Delta

## Purpose

Ermöglicht im Web, Mitglieder anzusehen und zu verwalten: Mitglieder pflegen ihre Kontaktdaten selbst, der Vorstand nimmt Mitglieder auf, ändert Stammdaten, Mitgliedsart und Mitgliedschaftsverlauf und pflegt Scheine und Lizenzen.

## ADDED Requirements

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
- **WHEN** Anna per manuell gesendetem POST ihre Mitgliedsart ändert
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

### Requirement: Scheine und Lizenzen pflegen
Der Vorstand SHALL Segelscheine mit Art, Datum und freier Bezeichnung sowie Funklizenzen erfassen, ändern und entfernen können. Die Segelerlaubnis des Mitglieds SHALL nach dem Speichern sofort neu berechnet erscheinen.

#### Scenario: Schein ergänzen
- **WHEN** der Vorstand für Frank einen SKS-Schein erfasst
- **THEN** zeigt Franks Seite „SKS“ und die Mitgliederliste mit Filter „SKS“ enthält ihn

### Requirement: Gleichzeitige Bearbeitung
Ein Formular SHALL die Version des Mitglieds enthalten, auf der es beruht. Wurde das Mitglied zwischenzeitlich geändert, SHALL das Speichern abgelehnt, nichts verändert und der aktuelle Stand mit den eingegebenen Werten zum Abgleich angezeigt werden.

#### Scenario: Konflikt
- **WHEN** Eva und Anna dasselbe Mitglied öffnen und Eva zuerst speichert
- **THEN** erhält Anna beim Speichern einen Hinweis auf die zwischenzeitliche Änderung und ihre Eingaben bleiben im Formular

# Spec Delta

## Purpose

Zeigt einem angemeldeten Mitglied auf einer Seite „Meine Daten“ alles, was der Verein über es führt und was es zum Segeln braucht – als erste durchgehende Funktion von der Datenbank bis zur Oberfläche.

## ADDED Requirements

### Requirement: Stammdaten anzeigen
Die Seite „Meine Daten“ SHALL dem angemeldeten Mitglied seine Stammdaten anzeigen: Name, Mitgliedsnummer, Mitgliedsart, Anschrift, Telefonnummern, E-Mail, Eintritt sowie Segelscheine und Funklizenzen. Ist die Meldeadresse als ungültig markiert, SHALL ein deutlicher Hinweis erscheinen.

#### Scenario: Anna sieht ihre Daten
- **WHEN** Anna Petersen „Meine Daten“ aufruft
- **THEN** sieht sie Mitgliedsnummer 101, „ordentliches Mitglied“, ihre Anschrift in Kiel sowie SBF See, SKS und SRC

### Requirement: Segelerlaubnis und Bootserlaubnisse
Die Seite SHALL anzeigen, ob das Mitglied im laufenden Jahr die Segelerlaubnis hat – andernfalls mit allen Gründen – und für welche Boote es eine Bootserlaubnis besitzt.

#### Scenario: Fehlende Stunden
- **WHEN** Clara Jensen 2026 „Meine Daten“ aufruft
- **THEN** sieht sie, dass ihr die Segelerlaubnis fehlt, weil 4,5 Arbeitsstunden fehlen, und ihre Bootserlaubnisse für Filou, Dipl.Ing.ling und Harald Blauzahn

### Requirement: Arbeitsstunden des Jahres
Die Seite SHALL die Arbeitseinsätze des laufenden Jahres mit Datum, Stunden, Gegenstand, Beschreibung und Bestätigungsstatus sowie Soll und anrechenbare Stunden anzeigen.

#### Scenario: Unbestätigter Einsatz
- **WHEN** Clara ihre Arbeitsstunden ansieht
- **THEN** ist „Segel der Filou geflickt“ als unbestätigt gekennzeichnet und die anrechenbaren Stunden betragen 20,5 von 25

### Requirement: Kontostand
Die Seite SHALL die Forderungen mit Zahlstatus (bezahlt, teilweise, offen, storniert), die Einzahlungen und den Kontostand anzeigen.

#### Scenario: Guthaben
- **WHEN** Anna ihr Konto ansieht
- **THEN** wird ein Guthaben von 10,00 € ausgewiesen

### Requirement: Nur eigene Daten
Ein Mitglied SHALL über „Meine Daten“ ausschließlich seine eigenen Daten sehen; eine Angabe einer anderen Mitgliedsnummer in der Anfrage SHALL keine fremden Daten liefern.

#### Scenario: Fremde Nummer in der URL
- **WHEN** Clara versucht, die Daten von Mitglied 101 über die Adresse der Seite abzurufen
- **THEN** werden ihr nur ihre eigenen Daten angezeigt

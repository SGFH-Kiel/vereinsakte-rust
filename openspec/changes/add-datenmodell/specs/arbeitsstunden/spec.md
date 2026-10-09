# Spec Delta

## Purpose

Erfasst die von Mitgliedern geleisteten Arbeitsstunden – bei gemeinsamen Arbeitsterminen oder bei allein erledigten Aufgaben – sowie Stundenübertrag zwischen Mitgliedern und finanzielle Abgeltung, und ermittelt daraus die anrechenbaren Stunden eines Jahres.

## ADDED Requirements

### Requirement: Arbeitseinsatz
Ein Arbeitseinsatz SHALL einem Mitglied zugeordnet sein und seinen Umfang entweder als (Datum, Stunden) oder als (Start, Ende) angeben. Eine Beschreibung SHALL optional möglich sein. Optional SHALL er auf einen Arbeitstermin oder eine Aufgabe verweisen, nicht auf beides; ein Einsatz SHALL auch ohne Termin und ohne Aufgabe erfasst werden können.

#### Scenario: Einsatz mit Datum und Menge
- **WHEN** ein Einsatz am 12. April 2026 mit 3,5 Stunden und Beschreibung „Unterwasserschiff geschliffen“ erfasst wird
- **THEN** zählt er mit 3,5 Stunden

#### Scenario: Einsatz mit Start und Ende
- **WHEN** ein Einsatz von 09:00 bis 13:30 Uhr am selben Tag erfasst wird
- **THEN** zählt er mit 4,5 Stunden

#### Scenario: Beide Formen zugleich
- **WHEN** JSON eines Einsatzes sowohl `hours` als auch `start`/`end` enthält
- **THEN** schlägt das Einlesen mit einem Fehler fehl

#### Scenario: Keine Umfangsangabe
- **WHEN** JSON eines Einsatzes weder `date`/`hours` noch `start`/`end` enthält
- **THEN** schlägt das Einlesen mit einem Fehler fehl

#### Scenario: Einsatz ohne Termin und Aufgabe
- **WHEN** Clara 2 Stunden „Segel der Filou geflickt“ für die „Filou“ erfasst, ohne Termin und ohne Aufgabe
- **THEN** ist der Einsatz gültig

### Requirement: Gegenstand eines Einsatzes
Jeder Arbeitseinsatz SHALL einen Gegenstand angeben: entweder ein Boot oder den Verein allgemein (Arbeiten ohne Boot wie Webseite, IT oder Vereinszeitung). Verweist der Einsatz auf einen Arbeitstermin oder eine Aufgabe mit Boot, SHALL der Gegenstand dieses Boot sein.

#### Scenario: Fehlender Gegenstand
- **WHEN** JSON eines Einsatzes ohne Gegenstand eingelesen wird
- **THEN** schlägt das Einlesen mit einem Fehler fehl

#### Scenario: Vereinsarbeit
- **WHEN** David 3 Stunden „Webseite aktualisiert“ mit Gegenstand „Verein“ erfasst
- **THEN** ist der Einsatz gültig und keinem Boot zugeordnet

#### Scenario: Widerspruch zur Aufgabe
- **WHEN** ein Einsatz auf die Aufgabe „Trigon Antifouling“ verweist, aber die „Filou“ als Gegenstand angibt
- **THEN** wird der Einsatz als ungültig gemeldet

### Requirement: Bestätigung
Ein Arbeitseinsatz SHALL bestätigt werden können; die Bestätigung SHALL das bestätigende Mitglied und den Zeitpunkt festhalten (wer bestätigen darf: siehe aemter). Ein neu erfasster Einsatz SHALL unbestätigt sein. Unbestätigte Einsätze SHALL sichtbar bleiben, aber nicht angerechnet werden.

#### Scenario: Neuer Einsatz
- **WHEN** ein Mitglied einen Einsatz erfasst
- **THEN** ist der Einsatz unbestätigt

#### Scenario: Unbestätigte Einsätze eines Mitglieds
- **WHEN** die Einsätze eines Mitglieds abgefragt werden
- **THEN** werden bestätigte und unbestätigte Einsätze geliefert und als solche erkennbar

### Requirement: Plausibilität eines Einsatzes
Ein Einsatz SHALL als ungültig gemeldet werden, wenn die Stunden negativ sind oder das Ende nicht nach dem Start liegt.

#### Scenario: Negative Stunden
- **WHEN** ein Einsatz mit −2 Stunden geprüft wird
- **THEN** wird er als ungültig gemeldet

#### Scenario: Ende vor Start
- **WHEN** ein Einsatz von 13:00 bis 09:00 Uhr geprüft wird
- **THEN** wird er als ungültig gemeldet

### Requirement: Zuordnung zum Jahr
Ein Einsatz SHALL dem Kalenderjahr seines Datums bzw. seines Starts zugerechnet werden, jeweils in dem angegebenen Zeitzonen-Offset.

#### Scenario: Jahreswechsel
- **WHEN** ein Einsatz am 31. Dezember 2026 um 22:00 Uhr (+01:00) startet
- **THEN** zählt er vollständig zu 2026

### Requirement: Stundenübertrag
Stunden SHALL von einem Mitglied an ein anderes für ein Jahr übertragen werden können (z. B. Paare), mit positiver Stundenzahl und optionaler Beschreibung. Ein Übertrag an sich selbst SHALL ungültig sein.

#### Scenario: Ausgleich unter Partnern
- **WHEN** Ben 2026 fünf Stunden an Clara überträgt
- **THEN** werden Clara 2026 fünf Stunden angerechnet und Ben fünf Stunden abgezogen

#### Scenario: Übertrag an sich selbst
- **WHEN** ein Übertrag mit gleichem Geber und Empfänger geprüft wird
- **THEN** wird er als ungültig gemeldet

### Requirement: Finanzielle Abgeltung
Für ein Mitglied und ein Jahr SHALL eine vom Vorstand genehmigte Abgeltung erfasst werden können: abgegoltene Stunden, Betrag, Genehmigungszeitpunkt und optionale Begründung. Ohne Betragsangabe SHALL der Betrag aus dem Stundensatz der Saison berechnet werden. Für jede Abgeltung SHALL eine Forderung über den Betrag in das Konto des Mitglieds eingetragen werden können, fällig mit der Genehmigung und ohne Doppelungen bei wiederholtem Anlegen.

#### Scenario: Betrag aus Stundensatz
- **WHEN** 5 Stunden abgegolten werden und der Stundensatz der Saison 15 € beträgt
- **THEN** beträgt der Abgeltungsbetrag 75 €

#### Scenario: Forderung zur Abgeltung
- **WHEN** die Forderungen zu Abgeltungen angelegt werden und für Anna 2026 eine Abgeltung von 75 € genehmigt ist
- **THEN** enthält Annas Konto eine Forderung „Arbeitsstunden-Abgeltung 2026“ über 75 €

### Requirement: Anrechenbare Stunden
Die anrechenbaren Stunden eines Mitglieds in einem Jahr SHALL sein: Summe seiner bestätigten Einsätze im Jahr + erhaltene Überträge − abgegebene Überträge + abgegoltene Stunden.

#### Scenario: Gesamtrechnung
- **WHEN** ein Mitglied 2026 bestätigte Einsätze von 4 h, 2,5 h und 8 h und einen unbestätigten von 6 h hat, 5 h erhält, 2 h abgibt und 3 h abgegolten sind
- **THEN** betragen seine anrechenbaren Stunden 2026 genau 20,5

#### Scenario: Anderes Jahr zählt nicht
- **WHEN** ein Mitglied nur Einsätze im Jahr 2025 hat
- **THEN** betragen seine anrechenbaren Stunden 2026 genau 0

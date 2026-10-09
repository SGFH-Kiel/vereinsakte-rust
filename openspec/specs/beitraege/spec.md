# beitraege Specification

## Purpose

Ermittelt nach der Beitragsordnung, welche Jahresbeiträge und Aufnahmegebühren die Mitglieder schulden, und trägt sie als Forderungen in die Mitgliedskonten ein.

## Requirements

### Requirement: Gebührensätze je Saison
Je Saison SHALL eine Gebührentabelle hinterlegt werden können. Fehlt sie, SHALL die Beitragsordnung gelten: Jahresbeitrag ordentlich 240 €, studentisch 170 €, außerordentlich 60 €, Erasmus 100 € pauschal, Ehrenmitglied und Kind eines Mitglieds 0 €; Aufnahmegebühr ordentlich 60 €, studentisch 30 €, alle anderen 0 €.

#### Scenario: Standardsätze
- **WHEN** für 2026 keine Gebührentabelle hinterlegt ist
- **THEN** beträgt der Jahresbeitrag eines studentischen Mitglieds 170 € und seine Aufnahmegebühr 30 €

#### Scenario: Geänderte Sätze
- **WHEN** die Saison 2027 einen ordentlichen Jahresbeitrag von 260 € festlegt
- **THEN** wird 2027 für ordentliche Mitglieder 260 € gefordert

### Requirement: Jahresbeitrag
Für jedes beitragspflichtige Mitglied, das in einem Jahr aktiv ist, SHALL eine Jahresbeitragsforderung für dieses Jahr entstehen. Im Eintrittsjahr SHALL der Beitrag monatsweise anteilig ab dem Eintrittsmonat (einschließlich) berechnet werden; die Erasmus-Pauschale SHALL nicht anteilig sein.

#### Scenario: Volles Jahr
- **WHEN** ein ordentliches Mitglied seit 2020 Mitglied ist
- **THEN** beträgt seine Beitragsforderung 2026 genau 240 €

#### Scenario: Eintritt im Juli
- **WHEN** ein ordentliches Mitglied im Juli 2026 eintritt
- **THEN** beträgt seine Beitragsforderung 2026 genau 6/12 von 240 € = 120 €

#### Scenario: Beitragsfrei
- **WHEN** ein Ehrenmitglied betrachtet wird
- **THEN** entsteht keine Beitragsforderung

### Requirement: Fälligkeit des Jahresbeitrags
Der Jahresbeitrag SHALL am 1. April des Jahres fällig sein; fällt dieser auf ein Wochenende, Karfreitag oder Ostermontag, am nächsten Geschäftstag. Bei Eintritt nach diesem Tag SHALL er mit dem Eintritt fällig sein.

#### Scenario: 1. April an einem Samstag
- **WHEN** die Fälligkeit für ein Jahr bestimmt wird, in dem der 1. April ein Samstag ist
- **THEN** ist der Beitrag am Montag, dem 3. April, fällig

#### Scenario: Ostermontag
- **WHEN** der 1. April auf den Ostermontag fällt (z. B. 2024)
- **THEN** ist der Beitrag am 2. April fällig

### Requirement: Aufnahmegebühr
Für jedes neue Mitglied SHALL einmalig eine Aufnahmegebühr nach seiner Mitgliedsart gefordert werden, fällig mit dem Eintritt. Ist der Satz 0 €, SHALL keine Forderung entstehen.

#### Scenario: Erasmus ohne Aufnahmegebühr
- **WHEN** ein Erasmus-Mitglied eintritt
- **THEN** entsteht keine Aufnahmegebühr

### Requirement: Forderungen ohne Doppelungen anlegen
Das Anlegen von Beitrags- und Aufnahmeforderungen SHALL für ein Jahr beliebig oft wiederholbar sein, ohne dass eine Forderung doppelt entsteht: Bereits vorhandene Forderungen gleicher Art und gleichen Jahres im Konto des Mitglieds (auch stornierte) SHALL übersprungen werden.

#### Scenario: Zweiter Lauf
- **WHEN** die Beitragsforderungen für 2026 zweimal angelegt werden
- **THEN** existiert für jedes Mitglied genau eine Beitragsforderung 2026

# Spec Delta

## Purpose

Berechnet nach der Beitragsordnung die Tourgebühren für Touren mit Vereinsbooten je Person an Bord und trägt sie als Forderungen in die Mitgliedskonten ein, damit sie eingesehen und als bezahlt nachvollzogen werden können.

## ADDED Requirements

### Requirement: Gebührenpflicht
Tourgebühren SHALL nur für Reservierungen entstehen, die als Tour gekennzeichnet sind. Gebührenpflichtig SHALL jede Person an Bord sein: Skipper, Co-Skipper, Crew und Gäste.

#### Scenario: Keine Tour
- **WHEN** ein Boot für einige Stunden ohne Tour-Kennzeichen reserviert ist
- **THEN** entstehen keine Tourgebühren

### Requirement: Tourtage
Die Tourtage einer Person SHALL die Zahl der Kalendertage sein, an denen sie an Bord ist (eigener Zeitraum oder Zeitraum der Reservierung); angefangene und endende Tage zählen voll.

#### Scenario: Wochenende
- **WHEN** eine Person von Freitag 16:00 bis Sonntag 18:00 an Bord ist
- **THEN** hat sie 3 Tourtage

#### Scenario: Crewwechsel
- **WHEN** Clara vom 1. bis 7. Juli und David vom 7. bis 14. Juli an Bord sind
- **THEN** hat Clara 7 und David 8 Tourtage, beide zahlen den 7. Juli voll

### Requirement: Tarifgruppen
Jede Person SHALL einer Tarifgruppe zugeordnet werden: ordentlich (ordentliche Mitglieder, Ehrenmitglieder, Erasmus), studentisch (studentische Mitglieder, Kinder von Mitgliedern), außerordentlich oder Gast.

#### Scenario: Kind eines Mitglieds
- **WHEN** ein Kind eines Mitglieds an einer Tour teilnimmt
- **THEN** zahlt es nach der studentischen Tarifgruppe

### Requirement: Berechnung
Die Gebühr SHALL je Person berechnet werden als: Anzahl voller Wochen (7 Tage) × Wochensatz + verbleibende Tage × Tagessatz. Hat die Tarifgruppe einen abweichenden Satz für die erste Woche, SHALL dieser für die erste volle Woche gelten. Die Sätze SHALL je Saison hinterlegt werden können; ohne Angabe gelten die Sätze der Beitragsordnung (Tag/Woche): ordentlich 12 €/70 €, studentisch 8 €/50 €, außerordentlich 30 €/erste Woche 180 €, jede weitere 70 €, Gast 30 €/240 €.

#### Scenario: Ordentlich 10 Tage
- **WHEN** ein ordentliches Mitglied 10 Tourtage hat
- **THEN** beträgt seine Gebühr 1 × 70 € + 3 × 12 € = 106 €

#### Scenario: Außerordentlich 15 Tage
- **WHEN** ein außerordentliches Mitglied 15 Tourtage hat
- **THEN** beträgt seine Gebühr 180 € + 70 € + 1 × 30 € = 280 €

#### Scenario: Studentisch 3 Tage
- **WHEN** ein studentisches Mitglied 3 Tourtage hat
- **THEN** beträgt seine Gebühr 3 × 8 € = 24 €

### Requirement: Forderungen aus einer Tour
Für jede gebührenpflichtige Person SHALL eine Forderung mit Bezug auf Reservierung und Person entstehen, fällig eine Woche nach ihrem letzten Tag an Bord. Die Forderung für ein Mitglied SHALL in dessen Konto eingetragen werden; die für einen Gast in das Konto des Mitglieds, das ihn eingeladen hat, ohne Angabe auf das des Skippers. Wiederholtes Anlegen SHALL keine Doppelungen erzeugen.

#### Scenario: Gast auf Skipper-Konto
- **WHEN** Gast „Max Gast“ ohne einladendes Mitglied 3 Tage an einer Tour mit Skipper Anna teilnimmt
- **THEN** enthält Annas Konto eine Tourgebühr über 90 €, deren Text den Gast nennt

#### Scenario: Fälligkeit
- **WHEN** eine Tour am 14. Juli endet
- **THEN** sind die Tourgebühren am 21. Juli fällig

#### Scenario: Erneutes Anlegen
- **WHEN** die Forderungen derselben Tour zweimal angelegt werden
- **THEN** existiert je Person genau eine Forderung

### Requirement: Tourgebühren einsehen
Für eine Reservierung SHALL abrufbar sein, welche Tourgebühren je Person gefordert sind und ob sie (nach der Berechnung der offenen Forderungen) bezahlt, teilweise bezahlt oder offen sind.

#### Scenario: Status einer Tour
- **WHEN** Anna ihre Tourgebühr bezahlt hat und Ben nicht
- **THEN** zeigt die Übersicht der Tour Annas Gebühr als bezahlt und Bens als offen

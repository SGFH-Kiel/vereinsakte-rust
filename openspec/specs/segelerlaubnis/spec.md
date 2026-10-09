# segelerlaubnis Specification

## Purpose

Leitet die jährliche Segelerlaubnis eines Mitglieds ab – Voraussetzung, um überhaupt auf Vereinsbooten mitzusegeln – aus Mitgliedschaft, Mitgliedsart, Beitragsrückständen laut Mitgliedskonto und den im Jahr anrechenbaren Arbeitsstunden.

## Requirements

### Requirement: Saison
Je Kalenderjahr SHALL eine Saison festgelegt werden können mit dem Richtsatz der Arbeitsstunden und dem Stundensatz für die finanzielle Abgeltung. Fehlt die Saison oder eine Angabe, SHALL der Richtsatz 25 Stunden und der Stundensatz 15 € betragen.

#### Scenario: Ohne Saisonangabe
- **WHEN** für 2027 keine Saison hinterlegt ist
- **THEN** gelten für 2027 ein Richtsatz von 25 Stunden und ein Stundensatz von 15 €

#### Scenario: Abweichender Richtsatz
- **WHEN** die Saison 2026 einen Richtsatz von 30 Stunden festlegt
- **THEN** gilt 2026 für stundenpflichtige Mitglieder ein Soll von 30 Stunden

### Requirement: Jahresnachweis
Je Mitglied und Jahr SHALL ein Jahresnachweis geführt werden können mit optional einem individuell reduzierten Soll mit Begründung und optional einer Abgeltung (siehe arbeitsstunden).

#### Scenario: Reduziertes Soll
- **WHEN** der Vorstand für Anna 2026 das Soll aus gesundheitlichen Gründen auf 10 Stunden reduziert
- **THEN** beträgt Annas Soll 2026 genau 10 Stunden und die Begründung ist erfasst

### Requirement: Soll-Stunden
Das Soll eines Mitglieds in einem Jahr SHALL 0 sein, wenn seine Mitgliedsart keine Stundenpflicht hat; sonst das individuell reduzierte Soll, falls vorhanden, andernfalls der Richtsatz der Saison.

#### Scenario: Kind eines Mitglieds
- **WHEN** das Soll eines Mitglieds der Art „Kind eines Mitglieds“ abgefragt wird
- **THEN** beträgt es 0 Stunden

### Requirement: Segelerlaubnis im Jahr
Ein Mitglied SHALL in einem Jahr die Segelerlaubnis haben, genau wenn: es in diesem Jahr aktiv ist, kein Beitragsrückstand besteht, die anrechenbaren Stunden das Soll erreichen und – bei außerordentlichen Mitgliedern – die Nutzungszustimmung des Vorstands vorliegt. Sonst SHALL das Ergebnis alle Gründe nennen.

#### Scenario: Alles erfüllt
- **WHEN** ein ordentliches Mitglied 2026 aktiv ist, keinen Beitragsrückstand hat und 25 von 25 Stunden angerechnet bekommt
- **THEN** hat es 2026 die Segelerlaubnis

#### Scenario: Stunden fehlen
- **WHEN** kein Beitragsrückstand besteht, aber 2026 nur 20 von 25 Stunden angerechnet werden
- **THEN** hat das Mitglied 2026 keine Segelerlaubnis, und als Grund werden 5 fehlende Stunden genannt

#### Scenario: Beitrag offen und Stunden fehlen
- **WHEN** ein Beitragsrückstand besteht und Stunden fehlen
- **THEN** nennt das Ergebnis beide Gründe

#### Scenario: Ehrenmitglied ohne Beitrag
- **WHEN** ein Ehrenmitglied keine Beitragsforderung hat, aber das Soll erreicht hat
- **THEN** hat es die Segelerlaubnis

#### Scenario: Außerordentlich ohne Zustimmung
- **WHEN** ein außerordentliches Mitglied keinen Beitragsrückstand hat, aber keine Nutzungszustimmung vorliegt
- **THEN** hat es keine Segelerlaubnis, und als Grund wird die fehlende Zustimmung genannt

#### Scenario: Nicht aktiv
- **WHEN** ein Mitglied 2026 keinen Tag aktiv ist
- **THEN** hat es 2026 keine Segelerlaubnis

### Requirement: Beitragsrückstand
Ein Beitragsrückstand SHALL bestehen, wenn eine Jahresbeitragsforderung bis einschließlich des betrachteten Jahres nach der Berechnung der offenen Forderungen nicht vollständig bezahlt ist, oder wenn ein beitragspflichtiges Mitglied für das Jahr noch keine Beitragsforderung hat.

#### Scenario: Beitrag gefordert und bezahlt
- **WHEN** für Anna 2026 eine Beitragsforderung von 240 € und eine Einzahlung von 240 € erfasst sind und sonst nichts offen ist
- **THEN** besteht kein Beitragsrückstand

#### Scenario: Beitrag nicht gefordert
- **WHEN** für ein ordentliches Mitglied 2026 noch keine Beitragsforderung eingetragen ist
- **THEN** besteht ein Beitragsrückstand

#### Scenario: Beitragsfreies Mitglied
- **WHEN** ein Ehrenmitglied keine Beitragsforderung hat
- **THEN** besteht kein Beitragsrückstand

#### Scenario: Zahlung tilgt erst ältere Tourgebühr
- **WHEN** Anna eine im März fällige Tourgebühr von 36 € und den im April fälligen Beitrag von 240 € schuldet und 240 € einzahlt
- **THEN** sind vom Beitrag noch 36 € offen und es besteht ein Beitragsrückstand

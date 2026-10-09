# mitgliedskonto Specification

## Purpose

Führt je Mitglied eine Liste der Forderungen des Vereins und eine Liste der Einzahlungen, damit berechnet werden kann, was bezahlt ist, was noch fehlt und was überfällig ist.

## Requirements

### Requirement: Mitgliedskonto
Jedes Mitglied SHALL ein Konto mit zwei Listen haben: Forderungen und Einzahlungen. Ein Mitglied ohne Angaben SHALL ein leeres Konto haben.

#### Scenario: Neues Mitglied
- **WHEN** ein Mitglied ohne Kontoangaben eingelesen wird
- **THEN** hat es keine Forderungen und keine Einzahlungen

### Requirement: Forderung
Eine Forderung SHALL eine Art (Jahresbeitrag mit Jahr, Aufnahmegebühr, Tourgebühr mit Reservierung und ggf. Gastname, Arbeitsstunden-Abgeltung mit Jahr, Sonstiges), einen positiven Betrag, einen Text, einen Erstellungszeitpunkt und optional eine Fälligkeit haben. Eine Forderung SHALL mit Zeitpunkt und Grund storniert werden können; stornierte Forderungen SHALL nicht mehr zählen, aber sichtbar bleiben.

#### Scenario: Tourgebühr fordern
- **WHEN** für Anna eine Tourgebühr von 36 € fällig am 21. Juli eingetragen wird
- **THEN** enthält Annas Konto diese Forderung mit Art, Betrag, Text und Fälligkeit

#### Scenario: Storno
- **WHEN** eine Forderung über 36 € storniert wird
- **THEN** bleibt sie mit Stornogrund im Konto, zählt aber nicht mehr zu den Forderungen

#### Scenario: Ungültiger Betrag
- **WHEN** eine Forderung oder Einzahlung mit Betrag 0 oder negativem Betrag geprüft wird
- **THEN** wird sie als ungültig gemeldet

### Requirement: Einzahlung
Eine Einzahlung SHALL einen positiven Betrag, einen Zeitpunkt und optional einen Text (z. B. Verwendungszweck) haben. Einzahlungen SHALL keiner bestimmten Forderung zugeordnet werden.

#### Scenario: Einzahlung erfassen
- **WHEN** Anna am 3. April 240 € überweist
- **THEN** enthält Annas Konto diese Einzahlung

### Requirement: Kontostand
Der Kontostand eines Mitglieds SHALL die Summe der nicht stornierten Forderungen minus die Summe der Einzahlungen sein. Ein positiver Kontostand ist ein offener Betrag, ein negativer ein Guthaben.

#### Scenario: Offener Betrag
- **WHEN** Anna Forderungen über 240 € und 36 € und eine Einzahlung über 240 € hat
- **THEN** beträgt ihr Kontostand 36 €

### Requirement: Offene Forderungen
Für jedes Mitglied SHALL berechnet werden können, welche Forderungen bezahlt, teilweise bezahlt oder offen sind. Die Summe der Einzahlungen SHALL dazu die nicht stornierten Forderungen in der Reihenfolge ihrer Fälligkeit (ohne Fälligkeit: Erstellungszeitpunkt) tilgen. Die Summe der offenen Beträge SHALL dem Kontostand entsprechen, sofern er positiv ist.

#### Scenario: Älteste zuerst
- **WHEN** Anna Forderungen über 240 € (fällig April) und 36 € (fällig Juli) hat und 250 € einzahlt
- **THEN** ist der Beitrag bezahlt und von der Tourgebühr sind noch 26 € offen

#### Scenario: Überzahlung
- **WHEN** Anna mehr einzahlt, als gefordert ist
- **THEN** sind alle Forderungen bezahlt, und der negative Kontostand weist das Guthaben aus

### Requirement: Übersicht der Außenstände
Es SHALL abrufbar sein, welche Mitglieder zu einem Zeitpunkt offene und welche überfällige Beträge haben (Fälligkeit vor dem Zeitpunkt), jeweils mit Betrag.

#### Scenario: Überfällig
- **WHEN** eine Tourgebühr am 8. Juli fällig war und am 20. Juli noch offen ist
- **THEN** erscheint das Mitglied am 20. Juli in der Übersicht mit diesem überfälligen Betrag

#### Scenario: Alles bezahlt
- **WHEN** kein Mitglied einen positiven Kontostand hat
- **THEN** ist die Übersicht leer

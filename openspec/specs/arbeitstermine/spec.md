# arbeitstermine Specification

## Purpose

Bildet die angekündigten gemeinsamen Arbeitstermine des Vereins (z. B. Kranen, Winterarbeit) in einem Terminkalender ab, zu denen sich Mitglieder anmelden und bei denen sie Arbeitsstunden leisten.

## Requirements

### Requirement: Arbeitstermin
Ein Arbeitstermin SHALL einen Titel, Beginn und Ende als Zeitpunkte haben. Ort, Beschreibung, betroffenes Boot und eine maximale Teilnehmerzahl SHALL optional sein.

#### Scenario: Termin anlegen
- **WHEN** der Termin „Boote kranen“ am 18. April 2026 von 9 bis 15 Uhr am Sporthafen Düsternbrook angelegt wird
- **THEN** enthält der Termin Titel, Zeitraum und Ort wie angegeben

#### Scenario: Ende vor Beginn
- **WHEN** ein Termin geprüft wird, dessen Ende nicht nach dem Beginn liegt
- **THEN** wird er als ungültig gemeldet

### Requirement: Anmeldungen
Mitglieder SHALL sich zu einem Arbeitstermin anmelden und wieder abmelden können; jedes Mitglied SHALL höchstens einmal angemeldet sein. Ist eine maximale Teilnehmerzahl gesetzt und erreicht, SHALL eine weitere Anmeldung abgelehnt werden.

#### Scenario: Doppelte Anmeldung
- **WHEN** sich ein bereits angemeldetes Mitglied erneut anmeldet
- **THEN** wird die Anmeldung abgelehnt und die Teilnehmerliste bleibt unverändert

#### Scenario: Termin voll
- **WHEN** ein Termin mit maximal 2 Teilnehmern bereits 2 Anmeldungen hat und sich ein drittes Mitglied anmeldet
- **THEN** wird die Anmeldung abgelehnt

#### Scenario: Abmeldung
- **WHEN** sich ein angemeldetes Mitglied abmeldet
- **THEN** ist es nicht mehr in der Teilnehmerliste

### Requirement: Anmeldung schreibt keine Stunden gut
Arbeitsstunden SHALL nur über Arbeitseinsätze angerechnet werden, die auf den Termin verweisen können; die Anmeldung allein SHALL keine Stunden gutschreiben.

#### Scenario: Anmeldung ohne Einsatz
- **WHEN** ein Mitglied zu einem Termin angemeldet ist, aber kein Arbeitseinsatz erfasst wurde
- **THEN** bleiben seine anrechenbaren Stunden unverändert

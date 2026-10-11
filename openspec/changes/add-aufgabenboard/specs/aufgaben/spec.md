## MODIFIED Requirements

### Requirement: Aufgabe
Eine Aufgabe SHALL einen Titel und einen Status (offen, in Arbeit, wartet, erledigt) haben. Beschreibung, betroffenes Boot, zuständige Mitglieder, geschätzte Stunden und Fälligkeit SHALL optional sein. Eine neue Aufgabe SHALL den Status „offen“ haben. Bestehende Aufgaben SHALL ihren Status behalten.

#### Scenario: Aufgabe anlegen
- **WHEN** die Aufgabe „Fock von Harald Blauzahn nähen“ mit geschätzten 4 Stunden für Ben angelegt wird
- **THEN** ist sie offen, Ben zugeordnet und auf „Harald Blauzahn“ bezogen

#### Scenario: Wartende Aufgabe
- **WHEN** eine Aufgabe auf „wartet“ gesetzt wird
- **THEN** hat sie den Status „wartet“ und keinen Erledigungszeitpunkt

### Requirement: Zuständigkeit und Status
Mitglieder SHALL einer Aufgabe zugewiesen und wieder entzogen werden können, jedes höchstens einmal. Der Status SHALL frei zwischen offen, in Arbeit, wartet und erledigt gesetzt werden können; beim Setzen auf „erledigt“ SHALL der Zeitpunkt der Erledigung festgehalten werden, jeder andere Status SHALL ihn löschen.

#### Scenario: Doppelte Zuweisung
- **WHEN** Ben einer Aufgabe zugewiesen wird, der er schon zugewiesen ist
- **THEN** bleibt die Liste der Zuständigen unverändert

#### Scenario: Erledigt
- **WHEN** eine Aufgabe am 20. Mai 2026 als erledigt markiert wird
- **THEN** hat sie den Status „erledigt“ mit Erledigungszeitpunkt 20. Mai 2026

#### Scenario: Wieder geöffnet
- **WHEN** eine erledigte Aufgabe auf „in Arbeit“ gesetzt wird
- **THEN** hat sie keinen Erledigungszeitpunkt mehr

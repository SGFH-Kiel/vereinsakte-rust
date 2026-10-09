# Spec Delta

## Purpose

Bildet Einzelaufgaben ab, die Mitglieder unabhängig von gemeinsamen Arbeitsterminen allein erledigen (z. B. Segel reparieren, Bootswartung), damit sie verteilt, verfolgt und mit Arbeitsstunden verknüpft werden können.

## ADDED Requirements

### Requirement: Aufgabe
Eine Aufgabe SHALL einen Titel und einen Status (offen, in Arbeit, erledigt) haben. Beschreibung, betroffenes Boot, zuständige Mitglieder, geschätzte Stunden und Fälligkeit SHALL optional sein. Eine neue Aufgabe SHALL den Status „offen“ haben.

#### Scenario: Aufgabe anlegen
- **WHEN** die Aufgabe „Fock von Harald Blauzahn nähen“ mit geschätzten 4 Stunden für Ben angelegt wird
- **THEN** ist sie offen, Ben zugeordnet und auf „Harald Blauzahn“ bezogen

### Requirement: Zuständigkeit und Status
Mitglieder SHALL einer Aufgabe zugewiesen und wieder entzogen werden können, jedes höchstens einmal. Der Status SHALL frei zwischen offen, in Arbeit und erledigt gesetzt werden können; beim Setzen auf „erledigt“ SHALL der Zeitpunkt der Erledigung festgehalten werden.

#### Scenario: Doppelte Zuweisung
- **WHEN** Ben einer Aufgabe zugewiesen wird, der er schon zugewiesen ist
- **THEN** bleibt die Liste der Zuständigen unverändert

#### Scenario: Erledigt
- **WHEN** eine Aufgabe am 20. Mai 2026 als erledigt markiert wird
- **THEN** hat sie den Status „erledigt“ mit Erledigungszeitpunkt 20. Mai 2026

### Requirement: Stunden über Arbeitseinsätze
Die für eine Aufgabe geleisteten Stunden SHALL über Arbeitseinsätze erfasst werden, die auf die Aufgabe verweisen; die Summe dieser Einsätze SHALL je Aufgabe abfragbar sein. Das Erledigen einer Aufgabe SHALL selbst keine Stunden gutschreiben.

#### Scenario: Stunden einer Aufgabe
- **WHEN** Ben 2,5 Stunden und Clara 1,5 Stunden mit Verweis auf dieselbe Aufgabe erfassen
- **THEN** betragen die geleisteten Stunden der Aufgabe 4

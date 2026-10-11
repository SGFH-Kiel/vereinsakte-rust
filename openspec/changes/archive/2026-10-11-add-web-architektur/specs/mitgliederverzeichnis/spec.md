# Spec Delta

## Purpose

Bietet angemeldeten Mitgliedern eine Liste der aktiven Vereinsmitglieder, wobei Kontaktdaten aus Datenschutzgründen nur für Mitglieder mit einem Amt sichtbar sind.

## ADDED Requirements

### Requirement: Liste aktiver Mitglieder
Jedes angemeldete Mitglied SHALL eine nach Nachnamen sortierte Liste der zum aktuellen Zeitpunkt aktiven Mitglieder mit Name und ihren aktuellen Ämtern sehen können.

#### Scenario: Ausgeschiedene erscheinen nicht
- **WHEN** die Liste im November 2026 aufgerufen wird
- **THEN** erscheint Inès Dupont (Erasmus bis Oktober) nicht, Eva Möller erscheint mit „1. Vorsitzende(r)“

### Requirement: Kontaktdaten nur für Ämter
Mitglieder, die zum aktuellen Zeitpunkt ein Amt innehaben, SHALL zusätzlich Mitgliedsnummer, Mitgliedsart, E-Mail und Telefonnummern sehen. Alle anderen SHALL diese Angaben nicht erhalten – weder in der Seite noch in nachgeladenen Teilen.

#### Scenario: Mitglied ohne Amt
- **WHEN** Greta Lorenzen (kein Amt) die Liste aufruft
- **THEN** sieht sie Namen und Ämter, aber keine E-Mail-Adressen oder Telefonnummern

#### Scenario: Bootswart
- **WHEN** Ben Hansen (Bootswart) die Liste aufruft
- **THEN** sieht er auch E-Mail-Adressen und Telefonnummern

### Requirement: Suche
Die Liste SHALL nach Teilen des Vor- oder Nachnamens gefiltert werden können, ohne die Seite neu zu laden.

#### Scenario: Suche nach Namensteil
- **WHEN** „peter“ eingegeben wird
- **THEN** werden nur Anna und Hannes Petersen angezeigt

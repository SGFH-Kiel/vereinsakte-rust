## MODIFIED Requirements

### Requirement: Liste aktiver Mitglieder
Jedes angemeldete Mitglied SHALL eine nach Nachnamen sortierte Liste der zum aktuellen Zeitpunkt aktiven Mitglieder mit Name und ihren aktuellen Ämtern sehen können. Segelscheine, Funklizenzen und Mitgliedsart SHALL als Symbole mit Tooltip (Name des Scheins bzw. der Art) erscheinen, jeweils mit einer für Hilfstechnik lesbaren Beschriftung; ein vorhandener Schein SHALL erkennbar von einem fehlenden unterschieden werden, ohne dass eine Spalte Platz für jeden denkbaren Schein belegt.

#### Scenario: Ausgeschiedene erscheinen nicht
- **WHEN** die Liste im November 2026 aufgerufen wird
- **THEN** erscheint Inès Dupont (Erasmus bis Oktober) nicht, Eva Möller erscheint mit „1. Vorsitzende(r)“

#### Scenario: Scheine als Symbole
- **WHEN** Clara mit Sportküstenschifferschein und UBI in der Liste steht
- **THEN** zeigt ihre Zeile ein Schein-Symbol je Schein, dessen Tooltip „SKS“ bzw. „UBI“ nennt, und ein Symbol für die Mitgliedsart mit Tooltip „Ordentliches Mitglied“

#### Scenario: Hilfstechnik
- **WHEN** ein Screenreader die Zeile vorliest
- **THEN** nennt er die Scheine und die Mitgliedsart als Text

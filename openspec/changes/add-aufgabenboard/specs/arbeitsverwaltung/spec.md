## MODIFIED Requirements

### Requirement: Aufgaben
Amtsträger SHALL Aufgaben anlegen und Zuständige zuweisen können. Ein Zuständiger oder Amtsträger SHALL den Status einer Aufgabe ändern können; jedes Mitglied SHALL die Aufgaben mit Zuständigen und Status lesen und nach der Spec `aufgabenboard` kommentieren. Zu einer Aufgabe erfasste Einsätze SHALL die Stunden der Aufgabe zeigen.

#### Scenario: Status setzen
- **WHEN** der Zuständige eine Aufgabe auf „erledigt“ setzt
- **THEN** zeigt das Board die Aufgabe in „DONE“ mit Zeitpunkt

#### Scenario: Fremde Aufgabe
- **WHEN** ein Mitglied ohne Zuständigkeit und ohne Amt den Status ändern will
- **THEN** wird dies mit 403 abgelehnt

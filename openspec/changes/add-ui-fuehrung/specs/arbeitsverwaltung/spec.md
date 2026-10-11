## MODIFIED Requirements

### Requirement: Arbeitstermine und Anmeldungen
Der Vorstand und der Oberbootswart SHALL Arbeitstermine mit Datum, Beschreibung und Platzzahl anlegen und ändern können. Jedes Mitglied SHALL die Termine im Bereich „Kalender“ nach der Spec `oberflaeche` gemeinsam mit den Reservierungen, aber erkennbar verschieden, als Liste oder Kalender sehen; Standard SHALL der Zeitraum ab heute sein, vergangene Termine stehen nur im Archiv. Jedes Mitglied SHALL sich zu einem Termin an- und abmelden können; die Anmeldung SHALL keine Stunden gutschreiben, solange kein Einsatz erfasst ist. Ein ausgebuchter Termin SHALL keine weitere Anmeldung annehmen.

#### Scenario: Anmelden
- **WHEN** Anna sich zum Arbeitstermin „Frühjahrsputz“ anmeldet
- **THEN** erscheint sie in der Teilnehmerliste und auf ihrer Startseite

#### Scenario: Ausgebucht
- **WHEN** alle Plätze vergeben sind und ein weiteres Mitglied sich anmelden will
- **THEN** zeigt die Seite „ausgebucht“ und es entsteht keine Anmeldung

#### Scenario: Terminkalender
- **WHEN** Anna den Kalender für den März öffnet
- **THEN** erscheint jeder Arbeitstermin an seinem Tag mit Titel, dem Wort „Arbeitstermin“ und freien Plätzen, und ein Klick führt zum Termin

#### Scenario: Vergangene Termine
- **WHEN** Anna den Kalender ohne weitere Angaben öffnet
- **THEN** erscheinen nur Termine ab heute

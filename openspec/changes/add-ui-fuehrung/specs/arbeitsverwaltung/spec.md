## MODIFIED Requirements

### Requirement: Arbeitstermine und Anmeldungen
Der Vorstand und der Oberbootswart SHALL Arbeitstermine mit Datum, Beschreibung und Platzzahl anlegen und ändern können. Jedes Mitglied SHALL die Termine wahlweise als Liste oder als Kalender sehen; Standard SHALL der Zeitraum ab heute sein, vergangene Termine stehen nur im Archiv. Jedes Mitglied SHALL sich zu einem Termin an- und abmelden können; die Anmeldung SHALL keine Stunden gutschreiben, solange kein Einsatz erfasst ist. Ein ausgebuchter Termin SHALL keine weitere Anmeldung annehmen.

#### Scenario: Anmelden
- **WHEN** Anna sich zum Arbeitstermin „Frühjahrsputz“ anmeldet
- **THEN** erscheint sie in der Teilnehmerliste und auf ihrer Startseite

#### Scenario: Ausgebucht
- **WHEN** alle Plätze vergeben sind und ein weiteres Mitglied sich anmelden will
- **THEN** zeigt die Seite „ausgebucht“ und es entsteht keine Anmeldung

#### Scenario: Terminkalender
- **WHEN** Anna die Arbeitstermine als Kalender für den März öffnet
- **THEN** erscheint jeder Termin an seinem Tag mit Titel und freien Plätzen, und ein Klick führt zum Termin

#### Scenario: Vergangene Termine
- **WHEN** Anna die Arbeitstermine ohne weitere Angaben öffnet
- **THEN** erscheinen nur Termine ab heute

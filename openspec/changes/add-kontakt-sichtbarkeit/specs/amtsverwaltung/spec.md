## MODIFIED Requirements

### Requirement: Ämterübersicht
Alle aktiven Mitglieder SHALL eine Übersicht der aktuell besetzten Ämter mit Inhaber und Beginn sehen, gegliedert in Vorstand, Revisoren und Bootswarte/Abnehmer je Boot; E-Mail und Telefonnummern der Inhaber SHALL nach der Spec `mitgliederverzeichnis` erscheinen, also für alle aktiven Mitglieder, außer bei Sperrvermerk.

#### Scenario: Besetzung
- **WHEN** Greta die Ämterübersicht öffnet
- **THEN** sieht sie Eva als 1. Vorsitzende und Ben als Bootswart der Harald Blauzahn, jeweils mit E-Mail und Telefonnummern

#### Scenario: Gesperrte Kontaktdaten eines Amtsträgers
- **WHEN** Ben den Sperrvermerk gesetzt hat und Greta die Übersicht öffnet
- **THEN** steht bei Ben „Kontaktdaten gesperrt“

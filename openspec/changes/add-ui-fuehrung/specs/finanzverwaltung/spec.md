## MODIFIED Requirements

### Requirement: Tourgebühren abrechnen
Der Kassenwart SHALL zu einer Tour-Reservierung die Gebühren nach der Spec `tourgebuehren` für Skipper, Co-Skipper und Crew berechnen lassen, prüfen und als Forderungen an die Mitglieder anlegen können, ohne Doppelung bei erneutem Anlegen. Gäste werden nicht abgerechnet.

#### Scenario: Tour abrechnen
- **WHEN** der Kassenwart eine siebentägige Tour mit zwei Mitgliedern abrechnet
- **THEN** entstehen zwei Forderungen auf den Mitgliedskonten mit dem berechneten Betrag

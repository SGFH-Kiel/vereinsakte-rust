## RENAMED Requirements

- FROM: `### Requirement: Kontaktdaten nur für Ämter`
- TO: `### Requirement: Kontaktdaten vereinsintern sichtbar`

## MODIFIED Requirements

### Requirement: Kontaktdaten vereinsintern sichtbar
Jedes aktive Mitglied SHALL im Verzeichnis zusätzlich Mitgliedsnummer, Mitgliedsart, Segelscheine, Funklizenzen, E-Mail und Telefonnummern der aktiven Mitglieder sehen. Hat ein Mitglied den Sperrvermerk gesetzt, SHALL anderen Mitgliedern statt E-Mail und Telefonnummern der Hinweis „Kontaktdaten gesperrt“ erscheinen – weder in der Seite noch in nachgeladenen Teilen dürfen die Angaben enthalten sein. Das Mitglied selbst und der Vorstand SHALL die Angaben weiterhin sehen. Personen außerhalb des Vereins SHALL sie nie erhalten.

#### Scenario: Mitglied ohne Amt
- **WHEN** Greta Lorenzen (kein Amt) die Liste aufruft
- **THEN** sieht sie Namen, Ämter, E-Mail-Adressen und Telefonnummern

#### Scenario: Bootswart
- **WHEN** Ben Hansen (Bootswart) die Liste aufruft
- **THEN** sieht er E-Mail-Adressen und Telefonnummern

#### Scenario: Sperrvermerk gesetzt
- **WHEN** Hannes den Sperrvermerk gesetzt hat und Greta die Liste aufruft
- **THEN** steht bei Hannes „Kontaktdaten gesperrt“, und seine Adresse und Nummern kommen weder im Text noch in nachgeladenen Teilen der Seite vor

#### Scenario: Vorstand und Betroffener
- **WHEN** Eva (Vorstand) oder Hannes selbst die Liste aufruft
- **THEN** sehen sie Hannes' E-Mail und Telefonnummern

#### Scenario: Bootswart ohne Sonderrecht
- **WHEN** Ben (Bootswart) die Liste aufruft und Hannes den Sperrvermerk gesetzt hat
- **THEN** sieht auch er nur „Kontaktdaten gesperrt“

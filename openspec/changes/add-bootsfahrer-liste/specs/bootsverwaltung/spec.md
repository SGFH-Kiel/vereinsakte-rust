## ADDED Requirements

### Requirement: Liste der Mitglieder mit Bootserlaubnis
Die Bootsseite SHALL für alle aktiven Mitglieder die Liste der aktiven Mitglieder zeigen, denen die Bootserlaubnis für dieses Boot erteilt und nicht widerrufen wurde. Weitere Voraussetzungen (Skipper-Anforderungen, Scheine, Segelerlaubnis, Meldeadresse) SHALL für die Liste keine Rolle spielen. Die Liste SHALL nach Name sortiert und durch Klick auf Spaltenköpfe sortierbar sein.

#### Scenario: Erteilte Erlaubnis
- **WHEN** Greta die Bootsseite der Filou öffnet und David die Abnahme der Filou bestanden hat
- **THEN** steht David in der Liste, mit Link zu seinem Profil

#### Scenario: Widerrufene Erlaubnis
- **WHEN** die Bootserlaubnis eines Mitglieds für die Filou widerrufen wurde
- **THEN** steht es nicht in der Liste

#### Scenario: Nicht bestandene Abnahme
- **WHEN** ein Mitglied die Abnahme der Filou nicht bestanden hat
- **THEN** steht es nicht in der Liste

#### Scenario: Weitere Voraussetzungen unerheblich
- **WHEN** ein Mitglied die Bootserlaubnis für die Dehler 31 hat, aber den geforderten Funkschein nicht besitzt
- **THEN** steht es trotzdem in der Liste der Dehler 31

#### Scenario: Ausgeschiedene
- **WHEN** ein Mitglied mit Bootserlaubnis aus dem Verein ausgeschieden ist
- **THEN** steht es nicht in der Liste

### Requirement: Kontaktaufnahme aus der Liste
Zu jedem Eintrag der Liste SHALL der Name zur Profilseite führen und E-Mail sowie Telefonnummer nach der Spec `mitgliederverzeichnis` erscheinen, bei gesetztem Sperrvermerk als „Kontaktdaten gesperrt“. Die Liste SHALL Segelscheine, Funklizenzen und Mitgliedsart als Symbole zeigen.

#### Scenario: Kontakt sichtbar
- **WHEN** Greta die Liste der Filou öffnet
- **THEN** stehen bei den Mitgliedern E-Mail und Telefonnummern, sofern kein Sperrvermerk gesetzt ist

#### Scenario: Gesperrt
- **WHEN** David den Sperrvermerk gesetzt hat
- **THEN** steht bei ihm „Kontaktdaten gesperrt“ und im Seitentext keine seiner Angaben

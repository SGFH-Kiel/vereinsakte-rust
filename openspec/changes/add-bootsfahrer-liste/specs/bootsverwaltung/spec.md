## ADDED Requirements

### Requirement: Liste der Skipper eines Bootes
Die Bootsseite SHALL für alle aktiven Mitglieder die Liste der aktiven Mitglieder zeigen, die das Boot nach Qualifikation als Skipper führen dürfen, das heißt eine nicht ruhende Bootserlaubnis für dieses Boot haben und die Skipper-Anforderungen des Bootes erfüllen. Die Liste SHALL die Segelerlaubnis der Saison weder auswerten noch anzeigen und stattdessen darauf hinweisen, dass sie zusätzlich nötig ist. Sie SHALL nach Name sortiert sein und durch Klick auf Spaltenköpfe sortierbar bleiben.

#### Scenario: Skipper der Filou
- **WHEN** Greta die Bootsseite der Filou öffnet
- **THEN** sieht sie David und die anderen Mitglieder mit bestandener Abnahme für die Filou, jeweils mit Namenslink

#### Scenario: Skipper-Anforderungen
- **WHEN** die Dehler 31 SKS, SBF See und SRC verlangt und Clara SKS und SBF See, aber kein SRC hat
- **THEN** erscheint Clara nicht in der Skipperliste der Dehler

#### Scenario: Segelerlaubnis nicht sichtbar
- **WHEN** ein Mitglied die Bootserlaubnis hat, aber 2026 keine Segelerlaubnis
- **THEN** erscheint es trotzdem in der Liste, und ein Hinweis nennt die Segelerlaubnis als zusätzliche Voraussetzung

#### Scenario: Ruhende Erlaubnis
- **WHEN** ein Mitglied eine ungültige Meldeadresse hat
- **THEN** erscheint es nicht in der Liste

### Requirement: Liste der Co-Skipper eines Bootes
Die Bootsseite SHALL zusätzlich je Fahrtgebiet mit Besatzungsregel die aktiven Mitglieder auflisten, die die Co-Skipper-Anforderungen dieser Regel erfüllen, ohne die Segelerlaubnis auszuwerten. Ein Fahrtgebiet ohne Co-Skipper-Anforderungen SHALL darauf hinweisen, dass kein zusätzlicher Nachweis verlangt wird.

#### Scenario: Co-Skipper außerhalb
- **WHEN** die Regel „außerhalb“ der Dehler 31 vom Co-Skipper [Bootserlaubnis „Harald Blauzahn“] und [SBF See] verlangt
- **THEN** listet die Bootsseite unter „außerhalb“ die Mitglieder, die beides haben

#### Scenario: Fahrtgebiet ohne Anforderung
- **WHEN** die Regel für das Heimrevier keine Co-Skipper-Anforderung enthält
- **THEN** steht dort „kein zusätzlicher Nachweis nötig“ statt einer Mitgliederliste

### Requirement: Kontaktaufnahme aus der Liste
Zu jedem Eintrag der Skipper- und Co-Skipper-Listen SHALL der Name zur Profilseite führen und E-Mail sowie Telefonnummer nach der Spec `mitgliederverzeichnis` erscheinen, bei gesetztem Sperrvermerk als „Kontaktdaten gesperrt“. Die Liste SHALL nach Segelschein, Funklizenz und Mitgliedsart als Symbole zeigen.

#### Scenario: Kontakt sichtbar
- **WHEN** Greta die Liste der Filou öffnet
- **THEN** stehen bei den Skippern ihre E-Mail und Telefonnummern, sofern kein Sperrvermerk gesetzt ist

#### Scenario: Gesperrt
- **WHEN** David den Sperrvermerk gesetzt hat
- **THEN** steht bei ihm „Kontaktdaten gesperrt“ und im Seitentext keine seiner Angaben

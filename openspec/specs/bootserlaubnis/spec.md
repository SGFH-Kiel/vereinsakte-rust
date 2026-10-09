# bootserlaubnis Specification

## Purpose

Bildet die Bootserlaubnis (Bootsführererlaubnis) der Mitglieder je Vereinsboot ab – erworben durch eine vereinsinterne Abnahme, gültig bis auf Widerruf – und prüft, ob ein Mitglied ein Boot als Skipper oder Co-Skipper führen darf.

## Requirements

### Requirement: Bootsabnahme
Eine Bootsabnahme SHALL einem Mitglied und einem Boot zugeordnet sein, einen Zeitpunkt, ein Ergebnis (bestanden oder nicht bestanden) und das abnehmende Mitglied haben sowie optional eine Bemerkung tragen. Ein Mitglied SHALL je Boot mehrere Abnahmen haben können.

#### Scenario: Nicht bestanden und wiederholt
- **WHEN** ein Mitglied die Abnahme für „Harald Blauzahn“ zuerst nicht besteht und später besteht
- **THEN** sind beide Abnahmen mit Ergebnis und Abnehmer erfasst

### Requirement: Widerruf
Eine Bootserlaubnis SHALL durch einen Widerruf mit Mitglied, Boot, Zeitpunkt, Grund und widerrufendem Vorstandsmitglied aufgehoben werden können.

#### Scenario: Widerruf erfassen
- **WHEN** der Vorstand die Erlaubnis von Anna für „Trigon“ am 1. Juli 2026 widerruft
- **THEN** ist der Widerruf mit Grund erfasst

### Requirement: Gültigkeit bis auf Widerruf
Ein Mitglied SHALL zu einem Zeitpunkt die Bootserlaubnis für ein Boot haben, wenn es vor diesem Zeitpunkt eine bestandene Abnahme für genau dieses Boot gibt und danach bis zum Zeitpunkt kein Widerruf für dieses Boot erfolgt ist. Die Erlaubnis SHALL nicht zeitlich ablaufen.

#### Scenario: Dauerhaft gültig
- **WHEN** ein Mitglied die Abnahme für „Filou“ 2019 bestanden hat und kein Widerruf vorliegt
- **THEN** hat es die Bootserlaubnis für „Filou“ auch 2026, aber nicht für „Trigon“

#### Scenario: Widerrufen
- **WHEN** nach der bestandenen Abnahme ein Widerruf erfolgt ist
- **THEN** hat das Mitglied ab dem Widerruf keine Bootserlaubnis für dieses Boot

#### Scenario: Erneut abgenommen
- **WHEN** nach einem Widerruf eine neue Abnahme bestanden wird
- **THEN** hat das Mitglied ab dieser Abnahme wieder die Bootserlaubnis

#### Scenario: Nur nicht bestanden
- **WHEN** ein Mitglied für ein Boot ausschließlich nicht bestandene Abnahmen hat
- **THEN** hat es keine Bootserlaubnis für dieses Boot

### Requirement: Ruhen ohne Meldeadresse
Ist die Meldeadresse eines Mitglieds ungültig, SHALL jede seiner Bootserlaubnisse ruhen, d. h. als nicht vorhanden gelten, ohne dass Abnahmen oder Widerrufe verändert werden.

#### Scenario: Erlaubnis ruht
- **WHEN** ein Mitglied mit bestandener Abnahme für „Filou“ eine ungültige Meldeadresse hat
- **THEN** hat es keine Bootserlaubnis für „Filou“, und als Grund wird „Meldeadresse ungültig“ genannt

#### Scenario: Erlaubnis lebt wieder auf
- **WHEN** die Meldeadresse wieder als gültig markiert wird
- **THEN** hat das Mitglied die Bootserlaubnis für „Filou“ wieder, ohne neue Abnahme

### Requirement: Eignung als Skipper
Ein Mitglied SHALL ein Boot zu einem Zeitpunkt als Skipper führen dürfen, genau wenn es im Jahr des Zeitpunkts die Segelerlaubnis hat, die Bootserlaubnis für dieses Boot hat und die Skipper-Anforderungen des Bootes erfüllt. Andernfalls SHALL das Ergebnis alle fehlenden Voraussetzungen nennen.

#### Scenario: Skipper geeignet
- **WHEN** ein Mitglied 2026 die Segelerlaubnis und die Bootserlaubnis für „Schruvtrekker“ hat und SKS, SBF See und SRC besitzt
- **THEN** darf es „Schruvtrekker“ im Juni 2026 als Skipper führen

#### Scenario: Mehrere Gründe
- **WHEN** einem Mitglied für „Schruvtrekker“ sowohl die Bootserlaubnis als auch die Funklizenz fehlt
- **THEN** darf es nicht führen, und das Ergebnis nennt beide fehlenden Voraussetzungen

### Requirement: Eignung als Co-Skipper
Ein Mitglied SHALL zu einem Zeitpunkt in einem Fahrtgebiet als Co-Skipper eines Bootes geeignet sein, genau wenn es im Jahr des Zeitpunkts die Segelerlaubnis hat und die Co-Skipper-Anforderungen der Besatzungsregel dieses Fahrtgebiets erfüllt. Die Bootserlaubnis für das Boot selbst SHALL nur nötig sein, wenn die Anforderungen sie verlangen.

#### Scenario: Co-Skipper auf der Dehler außerhalb
- **WHEN** die Regel „außerhalb“ der Dehler 31 vom Co-Skipper [Bootserlaubnis „Harald Blauzahn“] und [SBF See] verlangt und ein Mitglied beides hat, aber keine Bootserlaubnis für die Dehler
- **THEN** ist es als Co-Skipper geeignet

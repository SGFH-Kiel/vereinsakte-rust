## MODIFIED Requirements

### Requirement: Gebührenpflicht
Tourgebühren SHALL nur für Reservierungen entstehen, die als Tour gelten. Gebührenpflichtig SHALL jedes Mitglied an Bord sein: Skipper, Co-Skipper und Crew. Gäste sind nicht erfasst; ihre Gebühr rechnet der Skipper auf der Fahrt manuell ab.

#### Scenario: Keine Tour
- **WHEN** ein Boot für einige Stunden reserviert ist
- **THEN** entstehen keine Tourgebühren

### Requirement: Forderungen aus einer Tour
Für jedes gebührenpflichtige Mitglied SHALL eine Forderung mit Bezug auf Reservierung und Mitglied entstehen, fällig eine Woche nach seinem letzten Tag an Bord, und in dessen Konto eingetragen werden. Wiederholtes Anlegen SHALL keine Doppelungen erzeugen.

#### Scenario: Fälligkeit
- **WHEN** eine Tour am 14. Juli endet
- **THEN** sind die Tourgebühren am 21. Juli fällig

#### Scenario: Erneutes Anlegen
- **WHEN** die Forderungen derselben Tour zweimal angelegt werden
- **THEN** existiert je Mitglied genau eine Forderung

#### Scenario: Gast auf Skipper-Konto
- **WHEN** eine Tour mit Skipper Anna abgerechnet wird, an der ein Gast teilnahm
- **THEN** entsteht keine Forderung für den Gast, und Annas Konto enthält nur ihre eigene Gebühr

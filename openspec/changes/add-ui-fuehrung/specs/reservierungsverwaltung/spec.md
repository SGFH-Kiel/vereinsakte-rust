## MODIFIED Requirements

### Requirement: Reservierungskalender
Alle aktiven Mitglieder SHALL Reservierungen je Boot im Bereich „Kalender“ nach der Spec `oberflaeche` sehen, in der Listen- wie in der Kalenderdarstellung, mit Zeitraum, Boot, Skipper, Fahrtgebiet und Tour-Kennzeichen. Standard SHALL der Zeitraum ab heute sein; vergangene Reservierungen SHALL nur über das Archiv erreichbar sein. Die Ansicht SHALL nach Boot, Zeitraum und Beteiligung („nur meine“) filterbar sein.

#### Scenario: Nur meine
- **WHEN** Anna den Filter „nur meine“ wählt
- **THEN** erscheinen nur Reservierungen, in denen sie Skipper, Co-Skipper oder Crew ist

#### Scenario: Kalenderansicht
- **WHEN** Anna den Kalender für den Juli öffnet
- **THEN** erscheint jede Reservierung über ihre Tage, mit Boot und Skipper, und ein Klick führt zur Reservierung

#### Scenario: Normalfall ohne Vergangenes
- **WHEN** Anna den Kalender ohne weitere Angaben öffnet
- **THEN** sieht sie nur Einträge ab heute, und beendete Reservierungen stehen nur im Archiv

#### Scenario: Kalender am Handy
- **WHEN** der Kalender bei 375 px Breite geöffnet wird
- **THEN** erscheint er als Tagesliste der Woche ohne seitliches Scrollen

### Requirement: Reservierung anlegen
Ein Mitglied SHALL von der Bootsseite oder der Bootsliste aus für ein Boot, einen Zeitraum und ein Fahrtgebiet eine Reservierung mit sich selbst als Skipper anlegen können, optional mit Co-Skipper, Crew, Gästen, eigenem Zeitraum an Bord und Bemerkung. Das Fahrtgebiet SHALL mit dem Heimrevier vorbelegt sein. Ein Tour-Kennzeichen SHALL nicht abgefragt werden; es ergibt sich aus der Dauer. Berechtigte nach der Spec `rechte` SHALL für andere Skipper reservieren können.

#### Scenario: Reservierung mit Crew
- **WHEN** Anna die Schruvtrekker für Samstag 10–18 Uhr im Heimrevier mit Co-Skipper Ben, Crew Clara und Gast „Max Gast“ reserviert
- **THEN** erscheint sie im Kalender mit allen Beteiligten

#### Scenario: Tour ohne Haken
- **WHEN** Anna die Schruvtrekker vom 1. bis 7. Juli reserviert
- **THEN** zeigt die Reservierung „Tour“, und das Formular hatte kein Tour-Feld

### Requirement: Tour und Gebühr
Bei einer Reservierung, die als Tour gilt, SHALL die Seite die nach der Spec `tourgebuehren` zu erwartende Gebühr für Crew und Gäste anzeigen. Die Forderung SHALL erst der Kassenwart nach der Fahrt anlegen.

#### Scenario: Gebühr anzeigen
- **WHEN** Anna eine Tour für zwei Mitglieder und einen Gast über sieben Tage ansieht
- **THEN** zeigt die Seite die erwartete Gebühr je Person und gesamt

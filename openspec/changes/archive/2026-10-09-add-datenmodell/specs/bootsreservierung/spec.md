# Spec Delta

## Purpose

Bildet Reservierungen von Vereinsbooten im Vormerkkalender ab – mit Skipper, optionalem Co-Skipper, Crew aus Vereinsmitgliedern, Gästen, Fahrtgebiet und Tour-Kennzeichen – und prüft sie gegen die Regeln der Segelordnung.

## ADDED Requirements

### Requirement: Reservierungsdaten
Eine Reservierung SHALL auf ein Boot verweisen, Beginn und Ende als Zeitpunkte, ein Fahrtgebiet (Heimrevier oder außerhalb), genau einen Skipper, optional einen Co-Skipper, eine Liste der Crew (Mitglieder) und eine Liste von Gästen haben. Optional SHALL eine Bemerkung erfasst werden können.

#### Scenario: Reservierung mit Co-Skipper, Crew und Gast
- **WHEN** „Schruvtrekker“ für Samstag 10–18 Uhr im Heimrevier mit Skipper Anna, Co-Skipper Ben, Crew Clara und Gast „Max Gast“ reserviert wird
- **THEN** enthält die Reservierung genau diese Angaben

### Requirement: Tour-Kennzeichen
Eine Reservierung SHALL als Tour gekennzeichnet werden können; eine Tour ist eine Fahrt mit mindestens einer Übernachtung in einem fremden Hafen. Ohne Angabe SHALL eine Reservierung keine Tour sein.

#### Scenario: Kurze Ausfahrt
- **WHEN** ein Boot für einen Nachmittag ohne Tour-Kennzeichen reserviert wird
- **THEN** ist die Reservierung keine Tour

### Requirement: Eigener Zeitraum an Bord
Für jedes Crew-Mitglied und jeden Gast SHALL optional ein eigener Zeitraum an Bord (von, bis) angegeben werden können, z. B. bei Crewwechsel. Ohne Angabe SHALL der Zeitraum der Reservierung gelten. Ein eigener Zeitraum SHALL innerhalb der Reservierung liegen und sein Ende nach seinem Beginn.

#### Scenario: Crewwechsel
- **WHEN** Clara bei einer Tour vom 1. bis 14. Juli nur vom 1. bis 7. Juli und David vom 7. bis 14. Juli an Bord ist
- **THEN** hat jedes der beiden Crew-Mitglieder seinen eigenen Zeitraum

#### Scenario: Zeitraum außerhalb der Reservierung
- **WHEN** ein Crew-Zeitraum vor dem Beginn der Reservierung beginnt
- **THEN** wird die Reservierung als ungültig gemeldet

### Requirement: Gäste
Ein Gast SHALL mit Namen erfasst werden; Telefonnummer und das einladende Mitglied SHALL optional sein. Gäste sind keine Vereinsmitglieder und SHALL keine Segelerlaubnis benötigen.

#### Scenario: Gast ohne Telefonnummer
- **WHEN** ein Gast nur mit Namen eingetragen wird
- **THEN** ist die Reservierung insoweit gültig

### Requirement: Plausibilität einer Reservierung
Eine Reservierung SHALL als ungültig erkannt werden, wenn das Ende nicht nach dem Beginn liegt, Skipper und Co-Skipper dieselbe Person sind, Skipper oder Co-Skipper zugleich in der Crew stehen, ein Mitglied mehrfach in der Crew steht oder ein eigener Zeitraum an Bord unzulässig ist. Alle Verstöße SHALL gemeinsam gemeldet werden.

#### Scenario: Ende vor Beginn
- **WHEN** eine Reservierung um 18 Uhr beginnt und um 10 Uhr desselben Tages endet
- **THEN** wird sie mit dem Grund „Ende nicht nach Beginn“ gemeldet

#### Scenario: Skipper doppelt eingetragen
- **WHEN** Anna als Skipper und zugleich in der Crew eingetragen ist
- **THEN** wird die Reservierung als ungültig gemeldet

### Requirement: Gesamtprüfung
Für eine Reservierung SHALL geprüft werden können, ob sie nach der Segelordnung zulässig ist. Die Prüfung SHALL zum Beginn der Reservierung erfolgen, Plausibilität, Segelerlaubnis, Eignung, Bootsfreigabe und Mindestbesatzung umfassen und alle Beanstandungen gemeinsam melden. Verweise auf unbekannte Mitglieder oder Boote SHALL beanstandet werden.

#### Scenario: Zulässige Jollenreservierung
- **WHEN** ein Mitglied mit Segelerlaubnis und Bootserlaubnis für die freigegebene Jolle „Filou“ diese allein im Heimrevier reserviert
- **THEN** meldet die Prüfung keine Beanstandung

#### Scenario: Unbekanntes Mitglied
- **WHEN** die Crew eine Kennung enthält, zu der es kein Mitglied gibt
- **THEN** beanstandet die Prüfung das unbekannte Mitglied

### Requirement: Segelerlaubnis aller Mitglieder an Bord
Jedes Mitglied an Bord (Skipper, Co-Skipper, Crew) SHALL zum Beginn aktives Mitglied sein und die Segelerlaubnis des Jahres haben.

#### Scenario: Crew ohne Segelerlaubnis
- **WHEN** Crew-Mitglied Clara 2026 ihre Arbeitsstunden nicht erreicht hat
- **THEN** beanstandet die Prüfung die fehlende Segelerlaubnis von Clara

### Requirement: Eignung von Skipper und Co-Skipper
Der Skipper SHALL als Skipper des Bootes geeignet sein; ein eingetragener Co-Skipper SHALL für das Fahrtgebiet als Co-Skipper geeignet sein.

#### Scenario: Skipper ohne Bootserlaubnis
- **WHEN** der Skipper keine Bootserlaubnis für das reservierte Boot hat
- **THEN** beanstandet die Prüfung die fehlende Bootserlaubnis des Skippers

### Requirement: Bootsfreigabe
Das Boot SHALL zum Beginn für die Saison freigegeben und nicht gesperrt sein.

#### Scenario: Gesperrtes Boot
- **WHEN** das reservierte Boot zum Beginn gesperrt ist
- **THEN** beanstandet die Prüfung die Sperre mit ihrem Grund

### Requirement: Mindestbesatzung
Die Besatzungsregel des Fahrtgebiets SHALL erfüllt sein: Die Zahl der Mitglieder an Bord erreicht die Mindestzahl an Mitgliedern, die Zahl aller Personen (inkl. Gäste) die Mindestzahl an Personen, und ein verlangter Co-Skipper ist eingetragen. Fehlt die Regel, SHALL „Fahrtgebiet nicht freigegeben“ beanstandet werden. Gezählt SHALL jede eingetragene Person einmal werden, unabhängig von eigenen Zeiträumen an Bord; die zeitliche Prüfung bei Crewwechsel bleibt dem Vorstand vorbehalten.

#### Scenario: Folkeboot im Heimrevier zu zweit
- **WHEN** das Folkeboot im Heimrevier (mindestens 2 Mitglieder) mit Skipper und einem Crew-Mitglied reserviert wird
- **THEN** ist die Mindestbesatzung erfüllt

#### Scenario: Matcher außerhalb zu zweit
- **WHEN** „Trigon“ außerhalb (mindestens 3 Personen, Co-Skipper erforderlich) nur mit Skipper und Co-Skipper reserviert wird
- **THEN** beanstandet die Prüfung die Unterschreitung der Mindestbesatzung

#### Scenario: Co-Skipper fehlt
- **WHEN** die Regel einen Co-Skipper verlangt, aber keiner eingetragen ist
- **THEN** beanstandet die Prüfung den fehlenden Co-Skipper

#### Scenario: Crewwechsel wird nicht zeitgenau geprüft
- **WHEN** bei einer Tour mit Mindestbesatzung 3 Personen Clara nur in der ersten und David nur in der zweiten Woche an Bord ist und sonst nur Skipper und Co-Skipper
- **THEN** zählt die Prüfung 4 Personen und meldet keine Unterschreitung; ob zu jedem Zeitpunkt genug Personen an Bord sind, prüft der Vorstand anhand der erfassten Zeiträume

#### Scenario: Gäste zählen nur als Personen
- **WHEN** die Regel 2 Mitglieder verlangt und nur der Skipper und ein Gast eingetragen sind
- **THEN** beanstandet die Prüfung die Unterschreitung der Mitgliederzahl

### Requirement: Überschneidung erkennen
Für zwei Reservierungen SHALL feststellbar sein, ob sie dasselbe Boot in sich überschneidenden Zeiträumen betreffen. Reservierungen, bei denen eine genau endet, wenn die andere beginnt, SHALL sich nicht überschneiden.

#### Scenario: Aneinandergrenzende Reservierungen
- **WHEN** „Filou“ einmal von 10–14 Uhr und einmal von 14–18 Uhr reserviert ist
- **THEN** überschneiden sich die Reservierungen nicht

#### Scenario: Überlappung
- **WHEN** „Filou“ einmal von 10–14 Uhr und einmal von 13–18 Uhr reserviert ist
- **THEN** überschneiden sich die Reservierungen

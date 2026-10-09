# boote Specification

## Purpose

Bildet die Vereinsboote mit ihren Eigenschaften ab und hält die bootsbezogenen Regeln der Segelordnung – Anforderungen an Skipper und Co-Skipper, Mindestbesatzung je Fahrtgebiet, Saisonfreigabe und Sperre – als Daten fest.

## Requirements

### Requirement: Bootsstammdaten
Ein Boot SHALL einen Namen und einen Bootstyp (Jolle oder Kielboot) haben. Optional SHALL es Klasse (z. B. „Matcher 37“), Baujahr, Länge, Breite, Tiefgang, Segelfläche, Kojenzahl, Motor und Liegeplatz haben.

#### Scenario: Boot anlegen
- **WHEN** das Boot „Trigon“ als Kielboot der Klasse „Matcher 37“, Baujahr 1996, Länge 11,10 m angelegt wird
- **THEN** enthält das Boot genau diese Angaben und eine eigene Kennung

#### Scenario: Jolle ohne technische Daten
- **WHEN** die Jolle „Filou“ nur mit Name und Typ angelegt wird
- **THEN** ist das Boot gültig und alle technischen Daten sind nicht gesetzt

### Requirement: Anforderungen als Nachweisgruppen
Anforderungen SHALL als Liste von Gruppen beschrieben werden; jede Gruppe ist erfüllt, wenn mindestens einer ihrer Nachweise vorliegt, und die Anforderung ist erfüllt, wenn alle Gruppen erfüllt sind. Ein Nachweis SHALL sein: ein Segelschein, eine Funklizenz, die Bootserlaubnis für ein bestimmtes Boot oder die Bootserlaubnis für irgendein Boot eines Bootstyps.

#### Scenario: Leere Anforderung
- **WHEN** eine Anforderung keine Gruppen enthält
- **THEN** ist sie für jedes Mitglied erfüllt

#### Scenario: Mindestens SKS
- **WHEN** eine Anforderung die Gruppe [SKS | SSS | SHS | BR] enthält und ein Mitglied BR besitzt
- **THEN** ist die Anforderung erfüllt

#### Scenario: Segelkenntnisse
- **WHEN** eine Anforderung die Gruppe [Bootserlaubnis für eine Jolle | SBF See | SKS | BR] enthält und ein Mitglied nur die Bootserlaubnis für die Jolle „Filou“ hat
- **THEN** ist die Anforderung erfüllt

### Requirement: Skipper-Anforderungen
Ein Boot SHALL Anforderungen an den Skipper (Bootsführer) festlegen können, zusätzlich zur stets nötigen Bootserlaubnis für genau dieses Boot. Ohne Angabe SHALL keine weitere Anforderung bestehen.

#### Scenario: Jolle ohne Schein
- **WHEN** für eine Jolle keine Skipper-Anforderungen festgelegt sind
- **THEN** genügt zum Führen die Bootserlaubnis für diese Jolle

#### Scenario: Dehler 31
- **WHEN** ein Boot die Gruppen [SKS | SSS | SHS | BR], [SBF See] und [SRC | LRC] verlangt und ein Mitglied SKS und SBF See, aber keine Funklizenz besitzt
- **THEN** ist die Anforderung nicht erfüllt und die Gruppe [SRC | LRC] wird als fehlend genannt

### Requirement: Besatzungsregeln je Fahrtgebiet
Ein Boot SHALL je Fahrtgebiet (Heimrevier, außerhalb) eine Besatzungsregel haben können: Mindestzahl an Vereinsmitgliedern an Bord, Mindestzahl an Personen insgesamt, ob ein Co-Skipper erforderlich ist und welche Anforderungen der Co-Skipper erfüllen muss. Fehlt die Regel für ein Fahrtgebiet, SHALL das Boot in diesem Fahrtgebiet nicht ohne Vorstandsentscheidung gesegelt werden dürfen.

#### Scenario: Folkeboot außerhalb
- **WHEN** für ein Folkeboot außerhalb des Heimreviers 2 Mitglieder und ein Co-Skipper mit Segelkenntnissen verlangt sind
- **THEN** liefert das Boot für „außerhalb“ genau diese Regel

#### Scenario: Jolle außerhalb ohne Regel
- **WHEN** für eine Jolle keine Regel für „außerhalb“ hinterlegt ist
- **THEN** meldet das Boot für dieses Fahrtgebiet „Fahrtgebiet nicht freigegeben“

### Requirement: Saisonfreigabe
Ein Boot SHALL nur in Saisons (Kalenderjahren) gesegelt werden dürfen, für die es freigegeben wurde. Eine Freigabe SHALL Saison, freigebendes Mitglied und Zeitpunkt festhalten (wer freigeben darf: siehe aemter).

#### Scenario: Nicht freigegeben
- **WHEN** ein Boot nur für 2025 freigegeben ist und für Mai 2026 geprüft wird
- **THEN** gilt es als nicht freigegeben

### Requirement: Sperre
Ein Boot SHALL mit Beginn, optionalem Ende und Grund gesperrt werden können (z. B. Schaden); die Sperre SHALL das sperrende Mitglied festhalten. Eine Sperre ohne Ende SHALL durch Setzen des Endes aufgehoben werden können, unter Angabe des aufhebenden Mitglieds. Während einer Sperre SHALL das Boot nicht nutzbar sein, auch wenn es freigegeben ist.

#### Scenario: Gesperrt wegen Schaden
- **WHEN** ein freigegebenes Boot ab 3. Juni 2026 ohne Ende mit Grund „Ruderschaden“ gesperrt ist
- **THEN** ist es am 10. Juni 2026 nicht nutzbar, und der Grund „Ruderschaden“ wird genannt

#### Scenario: Sperre beendet
- **WHEN** eine Sperre am 15. Juni 2026 endet
- **THEN** ist das Boot am 16. Juni 2026 wieder nutzbar

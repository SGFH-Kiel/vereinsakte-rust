# Spec Delta

## Purpose

Macht den Vormerkkalender der Boote im Web nutzbar: Reservierungen ansehen, anlegen, ändern und stornieren, jeweils mit allen fachlichen Prüfungen der Segelordnung und verständlichen Fehlermeldungen.

## ADDED Requirements

### Requirement: Reservierungskalender
Alle aktiven Mitglieder SHALL Reservierungen je Boot in Kalender- und Listenansicht sehen, mit Zeitraum, Skipper, Fahrtgebiet und Tour-Kennzeichen. Die Ansicht SHALL nach Boot, Zeitraum und Beteiligung („nur meine“) filterbar sein.

#### Scenario: Nur meine
- **WHEN** Anna den Filter „nur meine“ wählt
- **THEN** erscheinen nur Reservierungen, in denen sie Skipper, Co-Skipper oder Crew ist

### Requirement: Reservierung anlegen
Ein Mitglied SHALL für ein Boot, einen Zeitraum und ein Fahrtgebiet eine Reservierung mit sich selbst als Skipper anlegen können, optional mit Co-Skipper, Crew, Gästen, eigenem Zeitraum an Bord, Tour-Kennzeichen und Bemerkung. Berechtigte nach der Spec `rechte` SHALL für andere Skipper reservieren können.

#### Scenario: Reservierung mit Crew
- **WHEN** Anna die Schruvtrekker für Samstag 10–18 Uhr im Heimrevier mit Co-Skipper Ben, Crew Clara und Gast „Max Gast“ reserviert
- **THEN** erscheint sie im Kalender mit allen Beteiligten

### Requirement: Fachliche Prüfung beim Speichern
Beim Speichern SHALL die Gesamtprüfung der Spec `bootsreservierung` laufen: Segelerlaubnis aller Mitglieder an Bord, Eignung von Skipper und Co-Skipper, Bootsfreigabe und Sperre, Mindestbesatzung und Überschneidung. Jeder Verstoß SHALL im Formular verständlich benannt werden und die Reservierung nicht gespeichert werden.

#### Scenario: Skipper ohne Eignung
- **WHEN** Greta ohne Abnahme als Skipper der Filou reservieren will
- **THEN** zeigt das Formular „keine Bootserlaubnis für die Filou“ und es wird nichts gespeichert

#### Scenario: Überschneidung
- **WHEN** das Boot im gewählten Zeitraum bereits reserviert ist
- **THEN** zeigt das Formular die kollidierende Reservierung mit Zeitraum

#### Scenario: Gesperrtes Boot
- **WHEN** das Boot im Zeitraum gesperrt ist
- **THEN** wird dies als Grund genannt

### Requirement: Ändern und Stornieren
Der Skipper SHALL eine eigene künftige Reservierung ändern und stornieren können, Berechtigte nach der Spec `rechte` jede. Eine Änderung SHALL erneut vollständig geprüft werden. Eine bereits begonnene oder beendete Reservierung SHALL nur der Vorstand ändern.

#### Scenario: Stornieren
- **WHEN** Anna ihre künftige Reservierung storniert
- **THEN** verschwindet sie aus dem Kalender und der Zeitraum ist wieder frei

#### Scenario: Fremde Reservierung
- **WHEN** Greta die Reservierung von Anna stornieren will
- **THEN** wird dies mit 403 abgelehnt

### Requirement: Tour und Gebühr
Bei einer als Tour gekennzeichneten Reservierung SHALL die Seite die nach der Spec `tourgebuehren` zu erwartende Gebühr für Crew und Gäste anzeigen. Die Forderung SHALL erst der Kassenwart nach der Fahrt anlegen.

#### Scenario: Gebühr anzeigen
- **WHEN** Anna eine Tour für zwei Mitglieder und einen Gast über sieben Tage ansieht
- **THEN** zeigt die Seite die erwartete Gebühr je Person und gesamt

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
Ein Mitglied SHALL von der Bootsseite oder der Bootsliste aus für ein Boot, einen Zeitraum und ein Fahrtgebiet eine Reservierung mit sich selbst als Skipper anlegen können, optional mit Co-Skipper, Crew, eigenem Zeitraum an Bord und Bemerkung. Das Fahrtgebiet SHALL mit dem Heimrevier vorbelegt sein. Die Crew SHALL im Formular als Liste der Mitfahrer erscheinen, denen man einzeln hinzufügen und die man einzeln entfernen kann, statt als Freitextfeld; die Mitglieder werden aus den aktiven Mitgliedern gewählt, optional mit eigenem Zeitraum an Bord. Gäste gibt es als eigene Angabe nicht; sie stehen in der Bemerkung, die als großes, mehrzeiliges Feld (mindestens acht Zeilen, bis 5.000 Zeichen) erscheint. Ein Tour-Kennzeichen SHALL nicht abgefragt werden; es ergibt sich aus der Dauer. Berechtigte nach der Spec `rechte` SHALL für andere Skipper reservieren können.

#### Scenario: Reservierung mit Crew
- **WHEN** Anna die Schruvtrekker für Samstag 10–18 Uhr im Heimrevier mit Co-Skipper Ben, Crew Clara und der Bemerkung „Gast: Max Gast“ reserviert
- **THEN** erscheint sie im Kalender mit allen Beteiligten

#### Scenario: Crew hinzufügen und entfernen
- **WHEN** Anna im Reservierungsformular Clara und David als Crew hinzufügt und David wieder entfernt
- **THEN** zeigt die Liste nur Clara als Link zu ihrer Profilseite, und beim Speichern ist nur Clara Crew

#### Scenario: Ohne JavaScript
- **WHEN** ohne JavaScript „Hinzufügen“ gewählt wird
- **THEN** erscheint dasselbe Formular mit dem neuen Mitfahrer in der Liste, ohne dass gespeichert wurde und mit erhaltenen übrigen Eingaben

#### Scenario: Gäste in der Bemerkung
- **WHEN** Anna in der Bemerkung drei Gäste mit Namen und Telefonnummern einträgt
- **THEN** speichert und zeigt die Reservierung den vollständigen Text, und es gibt kein eigenes Gästefeld

#### Scenario: Große Bemerkung
- **WHEN** das Reservierungsformular bei 375 px Breite geöffnet wird
- **THEN** zeigt das Bemerkungsfeld mindestens acht Zeilen und lässt sich bis 5.000 Zeichen füllen

#### Scenario: Doppelte Person
- **WHEN** ein bereits eingetragenes Crew-Mitglied oder der Skipper erneut hinzugefügt wird
- **THEN** zeigt das Formular „bereits an Bord“ und die Liste bleibt unverändert

#### Scenario: Tour ohne Haken
- **WHEN** Anna die Schruvtrekker vom 1. bis 7. Juli reserviert
- **THEN** zeigt die Reservierung „Tour“, und das Formular hatte kein Tour-Feld

### Requirement: Tour und Gebühr
Bei einer Reservierung, die als Tour gilt, SHALL die Seite die nach der Spec `tourgebuehren` zu erwartende Gebühr für Skipper, Co-Skipper und Crew anzeigen. Die Forderung SHALL erst der Kassenwart nach der Fahrt anlegen.

#### Scenario: Gebühr anzeigen
- **WHEN** Anna eine Tour für zwei Mitglieder über sieben Tage ansieht
- **THEN** zeigt die Seite die erwartete Gebühr je Person und gesamt

### Requirement: Fachliche Prüfung beim Speichern
Beim Speichern SHALL die Gesamtprüfung der Spec `bootsreservierung` laufen: Segelerlaubnis aller Mitglieder an Bord, Eignung von Skipper und Co-Skipper, Bootsfreigabe und Sperre sowie Überschneidung. Die Mindestbesatzung SHALL nicht geprüft werden. Jeder Verstoß SHALL im Formular verständlich benannt werden und die Reservierung nicht gespeichert werden.

#### Scenario: Skipper ohne Eignung
- **WHEN** Greta ohne Abnahme als Skipper der Filou reservieren will
- **THEN** zeigt das Formular „keine Bootserlaubnis für die Filou“ und es wird nichts gespeichert

#### Scenario: Überschneidung
- **WHEN** das Boot im gewählten Zeitraum bereits reserviert ist
- **THEN** zeigt das Formular die kollidierende Reservierung mit Zeitraum

#### Scenario: Gesperrtes Boot
- **WHEN** das Boot im Zeitraum gesperrt ist
- **THEN** wird dies als Grund genannt

#### Scenario: Wenig Besatzung
- **WHEN** „Trigon“ für eine Fahrt außerhalb nur mit dem Skipper reserviert wird
- **THEN** speichert das System die Reservierung ohne Hinweis zur Besatzungsstärke

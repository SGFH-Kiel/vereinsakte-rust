## MODIFIED Requirements

### Requirement: Reservierungsdaten
Eine Reservierung SHALL auf ein Boot verweisen, Beginn und Ende als Zeitpunkte, ein Fahrtgebiet (Heimrevier oder außerhalb), genau einen Skipper, optional einen Co-Skipper und eine Liste der Crew (Mitglieder) haben. Optional SHALL eine Bemerkung als längerer Freitext (bis 5.000 Zeichen) erfasst werden können; Gäste und andere Hinweise zur Fahrt stehen dort, nicht in eigenen Daten.

#### Scenario: Reservierung mit Co-Skipper, Crew und Gast
- **WHEN** „Schruvtrekker“ für Samstag 10–18 Uhr im Heimrevier mit Skipper Anna, Co-Skipper Ben, Crew Clara und der Bemerkung „Gast: Max Gast“ reserviert wird
- **THEN** enthält die Reservierung genau diese Angaben

### Requirement: Eigener Zeitraum an Bord
Für jedes Crew-Mitglied SHALL optional ein eigener Zeitraum an Bord (von, bis) angegeben werden können, z. B. bei Crewwechsel. Ohne Angabe SHALL der Zeitraum der Reservierung gelten. Ein eigener Zeitraum SHALL innerhalb der Reservierung liegen und sein Ende nach seinem Beginn.

#### Scenario: Crewwechsel
- **WHEN** Clara bei einer Tour vom 1. bis 14. Juli nur vom 1. bis 7. Juli und David vom 7. bis 14. Juli an Bord ist
- **THEN** hat jedes der beiden Crew-Mitglieder seinen eigenen Zeitraum

#### Scenario: Zeitraum außerhalb der Reservierung
- **WHEN** ein Crew-Zeitraum vor dem Beginn der Reservierung beginnt
- **THEN** wird die Reservierung als ungültig gemeldet

### Requirement: Gesamtprüfung
Für eine Reservierung SHALL geprüft werden können, ob sie nach der Segelordnung zulässig ist. Die Prüfung SHALL zum Beginn der Reservierung erfolgen, Plausibilität, Segelerlaubnis, Eignung und Bootsfreigabe umfassen und alle Beanstandungen gemeinsam melden. Die Mindestbesatzung SHALL nicht geprüft werden, weil sie sich bis zur Abfahrt ändern kann. Verweise auf unbekannte Mitglieder oder Boote SHALL beanstandet werden.

#### Scenario: Zulässige Jollenreservierung
- **WHEN** ein Mitglied mit Segelerlaubnis und Bootserlaubnis für die freigegebene Jolle „Filou“ diese allein im Heimrevier reserviert
- **THEN** meldet die Prüfung keine Beanstandung

#### Scenario: Unbekanntes Mitglied
- **WHEN** die Crew eine Kennung enthält, zu der es kein Mitglied gibt
- **THEN** beanstandet die Prüfung das unbekannte Mitglied

#### Scenario: Keine Besatzungsprüfung
- **WHEN** „Trigon“ außerhalb nur mit Skipper reserviert wird
- **THEN** meldet die Prüfung keine Beanstandung wegen der Besatzungsstärke

### Requirement: Tour-Kennzeichen
Eine Reservierung SHALL als Tour gelten, wenn ihr Zeitraum mindestens eine Übernachtung umfasst, das heißt ihr Ende auf einem späteren Kalendertag liegt als ihr Beginn. Die Eigenschaft SHALL sich allein aus dem Zeitraum ergeben und nicht angegeben werden können; ändert sich der Zeitraum, ändert sich die Eigenschaft mit.

#### Scenario: Kurze Ausfahrt
- **WHEN** ein Boot für einen Nachmittag reserviert wird
- **THEN** ist die Reservierung keine Tour

#### Scenario: Mehrtägige Fahrt
- **WHEN** ein Boot vom 1. bis 7. Juli reserviert wird
- **THEN** gilt die Reservierung als Tour

#### Scenario: Zeitraum verkürzt
- **WHEN** eine Tour vom 1. bis 7. Juli auf den 1. Juli nachmittags verkürzt wird
- **THEN** gilt sie nicht mehr als Tour

## REMOVED Requirements

### Requirement: Gäste
**Reason**: Gäste haben keinen Portalzugang und werden manuell abgerechnet; sie stehen künftig in der Bemerkung der Reservierung.
**Migration**: Bereits gespeicherte Gäste werden beim Anzeigen als Zeilen am Ende der Bemerkung ausgegeben, danach nicht mehr gepflegt; die Speicherspalten bleiben ungenutzt.

### Requirement: Mindestbesatzung
**Reason**: Die Besatzung ändert sich oft noch bis zur Abfahrt; die Prüfung bei der Reservierung ist daher nicht aussagekräftig.
**Migration**: Besatzungsregeln bleiben am Boot erhalten (sie bestimmen weiterhin die Anforderungen an einen Co-Skipper); nur die Prüfung der Mindestzahlen beim Reservieren entfällt.

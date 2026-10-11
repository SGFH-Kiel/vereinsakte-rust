# mitglieder Specification

## Purpose

Bildet die Mitglieder der Segelgruppe ab – Stammdaten, Kontakt- und Meldeadresse, Mitgliedsart nach Satzung und Beitragsordnung, den Verlauf der Mitgliedschaft sowie erworbene Segelscheine und Funklizenzen.

## Requirements

### Requirement: Mitgliederstammdaten
Ein Mitglied SHALL Nachname, Vorname und Mitgliedsnummer als Pflichtangaben haben. Die Mitgliedsnummer SHALL eine positive ganze Zahl sein und SHALL zugleich die Kennung des Mitglieds sein.

#### Scenario: Mitglied mit Pflichtangaben
- **WHEN** ein Mitglied mit Nachname „Müller“, Vorname „Anna“ und Mitgliedsnummer 42 angelegt wird
- **THEN** enthält das Mitglied genau diese Werte, und im JSON steht `"id": 42`

#### Scenario: Fehlende Pflichtangabe
- **WHEN** JSON eines Mitglieds ohne Feld `last_name` eingelesen wird
- **THEN** schlägt das Einlesen mit einem Fehler fehl

### Requirement: Eindeutige Mitgliedsnummer
Die Mitgliedsnummer SHALL innerhalb der Vereinsakte eindeutig sein. Eine Vereinsakte mit doppelten Mitgliedsnummern SHALL bei der Prüfung der Vereinsakte als ungültig gemeldet werden. Beim Einlesen SHALL eine Mitgliedsnummer abgelehnt werden, die keine positive ganze Zahl ist.

#### Scenario: Doppelte Nummer
- **WHEN** eine Vereinsakte zwei Mitglieder mit der Nummer 42 enthält
- **THEN** meldet die Prüfung die doppelte Mitgliedsnummer 42

#### Scenario: Ungültige Nummer
- **WHEN** ein Mitglied mit Nummer `0`, `-5` oder `"42"` (Zeichenkette) eingelesen wird
- **THEN** schlägt das Einlesen mit einem Fehler fehl

### Requirement: Anschrift und Telefonnummern
Ein Mitglied SHALL eine Anschrift aus Straße (inkl. Hausnummer), PLZ und Ort haben; die PLZ SHALL als Zeichenkette geführt werden. Festnetz-, geschäftliche und Mobilnummer SHALL jeweils optional sein.

#### Scenario: PLZ mit führender Null
- **WHEN** ein Mitglied mit PLZ „01067“ und Ort „Dresden“ gespeichert wird
- **THEN** bleibt die PLZ als „01067“ erhalten

#### Scenario: Nur Mobilnummer
- **WHEN** ein Mitglied nur mit Mobilnummer erfasst wird
- **THEN** ist die Mobilnummer gesetzt und Festnetz sowie geschäftliche Nummer sind nicht gesetzt

### Requirement: Meldeadresse
Ein Mitglied SHALL optional eine E-Mail-Adresse haben und SHALL die Angabe tragen, ob seine Meldeadresse gültig ist (z. B. nicht, wenn Post zurückkommt). Ohne Angabe SHALL die Meldeadresse als gültig gelten.

#### Scenario: Unzustellbare Adresse
- **WHEN** die Meldeadresse eines Mitglieds als ungültig markiert wird
- **THEN** meldet das Mitglied „Meldeadresse ungültig“

### Requirement: Mitgliedsart
Jedes Mitglied SHALL genau eine Mitgliedsart haben: ordentlich, studentisch, außerordentlich, Ehrenmitglied, Kind eines Mitglieds oder Erasmus. Aus der Art SHALL folgen, ob Beitrags- und Arbeitsstundenpflicht bestehen (siehe Szenarien).

#### Scenario: Pflichten je Mitgliedsart
- **WHEN** die Pflichten je Mitgliedsart abgefragt werden
- **THEN** gilt: ordentlich und studentisch – Beitrag ja, Stunden ja; außerordentlich – Beitrag ja, Stunden nein; Ehrenmitglied – Beitrag nein, Stunden ja; Kind eines Mitglieds – Beitrag nein, Stunden nein; Erasmus – Beitrag ja (Pauschale), Stunden nein

#### Scenario: Kind verweist auf Elternteil
- **WHEN** ein Mitglied der Art „Kind eines Mitglieds“ angelegt wird
- **THEN** kann es auf das Mitglied verweisen, dessen Kind es ist

### Requirement: Vorstandszustimmung für außerordentliche Mitglieder
Ein außerordentliches Mitglied SHALL die Angabe tragen, ob der Vorstand der Nutzung der Vereinseinrichtungen zugestimmt hat. Ohne Angabe SHALL keine Zustimmung vorliegen.

#### Scenario: Ohne Zustimmung
- **WHEN** ein außerordentliches Mitglied ohne Zustimmungsangabe eingelesen wird
- **THEN** liegt keine Nutzungszustimmung vor

### Requirement: Verlauf der Mitgliedschaft
Ein Mitglied SHALL ein Eintrittsdatum haben und optional: vorläufig aufgenommen bis, Befristung bis, Austrittsdatum, Ausschlussdatum. Ein Mitglied SHALL zu einem Zeitpunkt aktiv sein, wenn der Zeitpunkt nicht vor dem Eintritt und vor Austritt, Ausschluss und Befristungsende liegt.

#### Scenario: Austritt wirkt zum Jahresende
- **WHEN** ein Mitglied am 10. Mai 2026 seinen Austritt erklärt
- **THEN** wird als Austrittsdatum der 31. Dezember 2026, 24:00 Uhr (= 1. Januar 2027, 00:00 Uhr Ortszeit) eingetragen, und das Mitglied ist im Dezember 2026 noch aktiv

#### Scenario: Erasmus befristet
- **WHEN** ein Erasmus-Mitglied mit Eintritt 1. März 2026 und Befristung bis 31. Oktober 2026 im November 2026 geprüft wird
- **THEN** ist es nicht aktiv

#### Scenario: Ausschluss
- **WHEN** ein Mitglied zum 1. Juni 2026 ausgeschlossen wurde
- **THEN** ist es am 2. Juni 2026 nicht mehr aktiv

#### Scenario: Vorläufige Aufnahme
- **WHEN** ein Mitglied vorläufig bis zur nächsten Mitgliederversammlung aufgenommen ist
- **THEN** ist es aktiv und als „vorläufig“ erkennbar

### Requirement: Segelscheine
Ein Mitglied SHALL beliebig viele Segelscheine besitzen können, jeweils optional mit Erwerbsdatum. Bekannte Scheinarten SHALL SBF See, SBF Binnen, SKS, SSS, SHS und BR (älterer DSV-Schein, weiterhin anerkannt) sein; andere Arten SHALL als freie Bezeichnung erfassbar sein.

#### Scenario: Mehrere Scheine
- **WHEN** ein Mitglied SBF See und SKS besitzt
- **THEN** beantwortet das Modell „besitzt SKS?“ mit ja und „besitzt SSS?“ mit nein

#### Scenario: Unbekannte Scheinart
- **WHEN** ein Schein mit der freien Bezeichnung „Jüngstenschein“ erfasst wird
- **THEN** bleibt die Bezeichnung über JSON-Hin- und Rückweg unverändert erhalten

### Requirement: Scheinarten als einfache Zeichenkette
Segelscheinarten und Funklizenzarten SHALL im JSON als einfache Zeichenkette erscheinen, bekannte wie freie Arten. Bekannte Arten SHALL die Schreibweisen `"SBF See"`, `"SBF Binnen"`, `"SKS"`, `"SSS"`, `"SHS"`, `"BR"` bzw. `"SRC"`, `"LRC"`, `"UBI"`, `"ROC"` haben. Beim Einlesen SHALL eine Zeichenkette, die – ohne Beachtung von Groß-/Kleinschreibung und Leerzeichen am Rand – einer bekannten Art entspricht, als diese Art erkannt werden; jede andere SHALL als freie Bezeichnung übernommen werden.

#### Scenario: Bekannte Art
- **WHEN** ein Schein der Art SKS serialisiert wird
- **THEN** steht im JSON `"SKS"`

#### Scenario: Freie Art
- **WHEN** ein Schein mit freier Bezeichnung „Jüngstenschein“ serialisiert wird
- **THEN** steht im JSON `"Jüngstenschein"`

#### Scenario: Abweichende Schreibweise
- **WHEN** JSON mit der Scheinart `"sbf see"` eingelesen wird
- **THEN** wird sie als SBF See erkannt und beim Schreiben als `"SBF See"` ausgegeben

### Requirement: Funklizenzen
Ein Mitglied SHALL beliebig viele Funklizenzen besitzen können. Bekannte Arten SHALL SRC, LRC, UBI und ROC sein; andere Arten SHALL als freie Bezeichnung erfassbar sein.

#### Scenario: Funklizenz SRC
- **WHEN** ein Mitglied mit Funklizenz SRC erfasst wird
- **THEN** beantwortet das Modell „besitzt SRC?“ mit ja

### Requirement: Anmeldekonto
Ein Mitglied SHALL optional die Adresse seines Microsoft-Kontos (Anmeldekonto) tragen. Sie SHALL von der Kontakt-E-Mail getrennt sein, ohne Beachtung der Groß-/Kleinschreibung innerhalb der Vereinsakte eindeutig sein und im JSON als Feld `login_email` erscheinen. Ohne Angabe SHALL das Mitglied kein Anmeldekonto haben.

#### Scenario: Kontakt und Anmeldung getrennt
- **WHEN** Anna die Kontakt-E-Mail `anna.petersen@example.org` und das Anmeldekonto `anna.petersen@sgfh.example` hat
- **THEN** sind beide Adressen unabhängig voneinander gespeichert und im JSON als `email` bzw. `login_email` enthalten

#### Scenario: Doppeltes Anmeldekonto
- **WHEN** zwei Mitglieder das Anmeldekonto `Ben.Hansen@sgfh.example` bzw. `ben.hansen@sgfh.example` haben
- **THEN** meldet die Prüfung der Vereinsakte das doppelt vergebene Anmeldekonto

#### Scenario: Ohne Anmeldekonto
- **WHEN** ein Mitglied ohne Feld `login_email` eingelesen wird
- **THEN** gelingt das Einlesen und das Mitglied hat kein Anmeldekonto

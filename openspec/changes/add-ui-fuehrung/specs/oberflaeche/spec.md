## MODIFIED Requirements

### Requirement: Navigation nach Berechtigung
Die Hauptnavigation SHALL ihre Einträge in die Gruppen „Persönlich“ (Start, Meine Daten), „Verein“ (Mitglieder, Ämter, Boote, Reservierungen, Arbeit) und „Verwaltung“ (Finanzen nur für Vorstand und Revisoren, Protokoll für alle) gliedern, jeden Eintrag mit Symbol und Text zeigen und den aktuellen Bereich hervorheben. Konto und Abmelden SHALL in einem Kontomenü mit dem Namen und den Ämtern des Mitglieds liegen. Auf Bildschirmen unter 48 rem SHALL stattdessen eine untere Leiste Start, Boote, Reservierungen, Arbeit und „Mehr“ anbieten, wobei „Mehr“ alle übrigen Einträge und das Kontomenü öffnet; das SHALL ohne JavaScript funktionieren.

#### Scenario: Navigation für Greta
- **WHEN** Greta ohne Amt eine Seite öffnet
- **THEN** zeigt die Navigation nicht „Finanzen“, aber „Ämter“ und „Protokoll“

#### Scenario: Menü ohne JavaScript
- **WHEN** auf einem schmalen Bildschirm JavaScript deaktiviert ist
- **THEN** lässt sich das Menü öffnen und schließen

#### Scenario: Gruppen am Desktop
- **WHEN** eine Seite bei 1280 px Breite geöffnet wird
- **THEN** stehen die Einträge unter den Gruppenüberschriften „Persönlich“, „Verein“ und „Verwaltung“, jeweils mit Symbol, und der aktuelle Bereich ist hervorgehoben

#### Scenario: Untere Leiste am Handy ohne JavaScript
- **WHEN** auf einem Bildschirm unter 48 rem Breite JavaScript deaktiviert ist und „Mehr“ gewählt wird
- **THEN** erscheinen die übrigen Einträge samt Konto und Abmelden, und das Menü lässt sich wieder schließen

### Requirement: Startseite
Die Startseite SHALL dem angemeldeten Mitglied zuerst unter „Was ist zu tun?“ die dringenden Handlungshinweise mit je einem Link zeigen (fehlende Segelerlaubnis, fehlende Arbeitsstunden, offenes Konto, offene Bestätigungen, offene Aufgaben) und bei nichts Dringendem eine Bestätigung „Alles erledigt“. Darunter SHALL sie Kennzahlen mit Zustandssymbol und die anstehenden Arbeitstermine sowie eigenen Reservierungen zeigen, die Arbeitsstunden zusätzlich als Fortschrittsanzeige.

#### Scenario: Hinweis für Clara
- **WHEN** Clara die Startseite öffnet
- **THEN** steht unter „Was ist zu tun?“ „Es fehlen 4,5 Arbeitsstunden“ mit einem Link zum Erfassen eines Einsatzes

#### Scenario: Nichts zu tun
- **WHEN** ein Mitglied mit Segelerlaubnis, erfüllten Stunden, ausgeglichenem Konto und ohne offene Bestätigungen die Startseite öffnet
- **THEN** zeigt der Bereich „Was ist zu tun?“ die Bestätigung „Alles erledigt“

## ADDED Requirements

### Requirement: Symbolsatz
Die Oberfläche SHALL Symbole für Navigation, Zustände, Aktionen und leere Zustände aus einem eigenen, in die Seite eingebetteten Satz beziehen. Ein Symbol SHALL stets von Text begleitet sein oder, bei reinen Symbol-Schaltflächen, eine für Hilfstechnik lesbare Beschriftung tragen, und SHALL nie allein einen Zustand ausdrücken.

#### Scenario: Symbol-Schaltfläche beschriftet
- **WHEN** eine Zeilenaktion nur als Symbol dargestellt wird
- **THEN** hat die Schaltfläche eine zugängliche Beschriftung wie „Einsatz ändern“ und einen Hinweistext

#### Scenario: Zustand mit Symbol und Text
- **WHEN** ein Boot gesperrt ist
- **THEN** zeigt die Plakette ein Symbol und das Wort „gesperrt“

### Requirement: Typografische Hierarchie
Die Oberfläche SHALL feste Schriftstufen für Seitentitel, Abschnittstitel, Beschriftung, Fließtext und Kennzahl verwenden, Beschriftungen von Werten durch Gewicht oder Farbe unterscheiden, Zahlen mit gleich breiten Ziffern setzen und Fließtext auf höchstens 70 Zeichen Zeilenlänge begrenzen.

#### Scenario: Eine Hauptüberschrift
- **WHEN** eine beliebige Seite geöffnet wird
- **THEN** enthält sie genau eine Überschrift erster Ebene, und Abschnittstitel folgen als zweite Ebene ohne übersprungene Stufen

### Requirement: Seitenkopf mit einer Hauptaktion
Jede Seite SHALL mit Bereich, Titel, einer Zustands- oder Beschreibungszeile und höchstens einer hervorgehobenen Hauptaktion beginnen. Weitere Aktionen der Seite SHALL in einem Menü „Weitere Aktionen“ liegen, das ohne JavaScript bedienbar ist.

#### Scenario: Mitgliedsseite des Vorstands
- **WHEN** der Vorsitzende die Seite eines Mitglieds öffnet
- **THEN** ist „Bearbeiten“ die einzige hervorgehobene Aktion im Kopf, und Jahresnachweis und Konto liegen unter „Weitere Aktionen“

#### Scenario: Menü per Tastatur
- **WHEN** „Weitere Aktionen“ per Tastatur geöffnet wird
- **THEN** sind alle enthaltenen Aktionen per Tastatur erreichbar

### Requirement: Seltene Aktionen eingeklappt
Selten benötigte oder zerstörende Aktionen mit eigenem Formular (zum Beispiel Boot sperren, Bootserlaubnis widerrufen, Abnahme erfassen) SHALL in einem standardmäßig geschlossenen, beschrifteten Bereich liegen, der ohne JavaScript geöffnet werden kann. Bei einem Formularfehler SHALL der betroffene Bereich geöffnet erscheinen.

#### Scenario: Boot sperren eingeklappt
- **WHEN** der Bootswart die Seite seines Boots öffnet
- **THEN** ist der Bereich „Boot sperren“ geschlossen, und der Zustand des Boots ist ohne Scrollen sichtbar

#### Scenario: Fehler öffnet den Bereich
- **WHEN** „Boot sperren“ ohne Grund abgeschickt wird
- **THEN** erscheint der Bereich geöffnet mit der Fehlermeldung am Feld

### Requirement: Kompakte Listen
Listen SHALL pro Eintrag die Hauptangabe mit einer Nebenzeile zeigen, Zustände als Plaketten mit Symbol, bei Personen einen Initialen-Kreis und die Zeilenaktionen am Zeilenende zusammengefasst. Einträge ohne Treffer SHALL einen leeren Zustand mit Symbol, Erklärung und, falls das Mitglied sie ausführen darf, dem nächsten Schritt zeigen.

#### Scenario: Leere Reservierungsliste
- **WHEN** ein Mitglied ohne Reservierungen „Reservierungen“ öffnet
- **THEN** erscheint ein leerer Zustand mit Symbol, dem Satz „Keine anstehenden Reservierungen“ und der Aktion „Boot reservieren“

#### Scenario: Personenliste
- **WHEN** das Mitgliederverzeichnis geöffnet wird
- **THEN** zeigt jede Zeile Initialen, den Namen und in der Nebenzeile Mitgliedsart und Ämter

### Requirement: Gegliederte Formulare
Formulare SHALL ihre Felder in benannte Abschnitte mit kurzer Erklärung gliedern, Pflichtfelder kennzeichnen, Hinweise getrennt von Fehlermeldungen darstellen und die Aktionsleiste mit der Hauptaktion am Ende des Formulars zeigen, auf schmalen Bildschirmen am unteren Rand sichtbar haftend, ohne Eingabefelder zu verdecken.

#### Scenario: Mitglied aufnehmen
- **WHEN** das Formular „Mitglied aufnehmen“ geöffnet wird
- **THEN** stehen die Felder unter den Abschnitten „Person“, „Kontakt“ und „Mitgliedschaft“, Pflichtfelder sind gekennzeichnet, und die Schaltfläche „Aufnehmen“ steht am Ende

#### Scenario: Haftende Leiste am Handy
- **WHEN** das Formular bei 375 px Breite geöffnet und bis zum letzten Feld gescrollt wird
- **THEN** bleibt die Hauptaktion sichtbar und verdeckt kein Feld

### Requirement: Fortschrittsanzeige
Mengen mit Soll und Ist (Arbeitsstunden, Saisonfortschritt) SHALL zusätzlich zur Zahl als Fortschrittsanzeige mit zugänglichem Textwert dargestellt werden, ohne dass die Seite dafür Inline-Stile oder Skripte benötigt.

#### Scenario: Arbeitsstunden
- **WHEN** ein Mitglied mit 4,5 von 10 Stunden die Startseite öffnet
- **THEN** zeigt die Fortschrittsanzeige 45 % und der Text nennt „4,5 von 10 Stunden“

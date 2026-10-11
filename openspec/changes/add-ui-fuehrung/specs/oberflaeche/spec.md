## MODIFIED Requirements

### Requirement: Navigation nach Berechtigung
Die Hauptnavigation SHALL ihre Einträge in die Gruppen „Persönlich“ (Start, Meine Daten), „Verein“ (Mitglieder, Ämter, Boote, Kalender, Aufgaben, Arbeit) und „Verwaltung“ (Finanzen nur für Vorstand und Revisoren, Protokoll für alle) gliedern, jeden Eintrag mit Symbol und Text zeigen und den aktuellen Bereich hervorheben. Konto und Abmelden SHALL in einem Kontomenü mit dem Namen und den Ämtern des Mitglieds liegen. Auf Bildschirmen unter 48 rem SHALL stattdessen eine untere Leiste Start, Boote, Kalender, Arbeit und „Mehr“ anbieten, wobei „Mehr“ alle übrigen Einträge und das Kontomenü öffnet; das SHALL ohne JavaScript funktionieren.

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
- **WHEN** ein Mitglied ohne Reservierungen und Termine den Kalender öffnet
- **THEN** erscheint ein leerer Zustand mit Symbol, dem Satz „Keine anstehenden Einträge“ und dem Link „Zu den Booten“

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

### Requirement: Auf den Normalfall optimiert
Jede Ansicht, jedes Formular und jede Liste SHALL den häufigsten Fall mit den wenigsten Schritten bedienen: sinnvolle Vorbelegungen, Standardfilter auf das Aktuelle (kommende Reservierungen und Termine, laufende Saison, aktive Mitglieder und Ämter) und Ausnahmen oder Seltenes einen Schritt entfernt. Pflichtangaben SHALL nur verlangt werden, wo sie fachlich nötig sind.

#### Scenario: Reservierung im Heimrevier
- **WHEN** ein Mitglied das Reservierungsformular öffnet
- **THEN** sind das Heimrevier vorbelegt, das Mitglied selbst als Skipper gesetzt und nur Boot und Zeitraum auszufüllen

#### Scenario: Aktuelles zuerst
- **WHEN** eine Liste mit zeitlichem Bezug ohne Filter geöffnet wird
- **THEN** zeigt sie nur Aktuelles und Kommendes, und ein Hinweis verweist auf das Archiv

### Requirement: Kein eigener Menüpunkt Reservierungen
Die Navigation SHALL keinen Menüpunkt „Reservierungen“ enthalten. Ein Boot reservieren SHALL von der Bootsseite und der Bootsliste aus möglich sein, mit dem Boot vorausgewählt. Die Reservierungen selbst SHALL im Bereich „Kalender“ erscheinen.

#### Scenario: Reservieren über das Boot
- **WHEN** ein Mitglied auf der Bootsseite der Schruvtrekker „Reservieren“ wählt
- **THEN** öffnet das Reservierungsformular mit der Schruvtrekker vorausgewählt

#### Scenario: Kein Menüpunkt
- **WHEN** ein Mitglied die Navigation betrachtet
- **THEN** gibt es „Kalender“ und „Boote“, aber nicht „Reservierungen“

### Requirement: Kalenderbereich mit Liste und Kalender
Der Bereich „Kalender“ SHALL Reservierungen und Arbeitstermine gemeinsam zeigen und über eine Auswahl „Liste / Kalender“ in beiden Darstellungen anbieten; die Darstellung SHALL in der Adresse stehen, Filter nach Art (alle, Reservierungen, Arbeitstermine), Boot und „nur meine“ SHALL in beiden gelten. Der Kalender SHALL ohne JavaScript bedienbar sein, mit Blättern nach Monat und Woche und Sprung auf heute.

#### Scenario: Umschalten
- **WHEN** ein Mitglied in der Liste „Kalender“ wählt
- **THEN** erscheinen dieselben Einträge im Kalender mit denselben Filtern

#### Scenario: Nur Arbeitstermine
- **WHEN** ein Mitglied die Art „Arbeitstermine“ wählt
- **THEN** erscheinen keine Reservierungen

#### Scenario: Kalender per Tastatur
- **WHEN** der Kalender nur mit der Tastatur bedient wird
- **THEN** sind Blättern, Heute und jeder Eintrag erreichbar

### Requirement: Liste nach Tagen gruppiert
Die Listendarstellung SHALL die Einträge nach Woche und darin nach Tag gruppieren, mit Überschriften wie „Diese Woche“ und „Samstag, 12. Juli“, Standard ab heute; mehrtägige Einträge SHALL unter ihrem ersten Tag mit Zeitraum erscheinen und an den Folgetagen kurz als „weiter“ wiederholt werden. Tage ohne Einträge SHALL nicht erscheinen.

#### Scenario: Gruppierung
- **WHEN** an einem Samstag eine Reservierung und ein Arbeitstermin liegen
- **THEN** stehen beide unter der Überschrift dieses Samstags in der Wochengruppe

#### Scenario: Tour über mehrere Tage
- **WHEN** eine Tour vom 1. bis 7. Juli läuft und die Liste ab dem 3. Juli beginnt
- **THEN** erscheint sie unter dem 3. Juli als „weiter bis 7. Juli“

### Requirement: Reservierungen und Arbeitstermine unterscheidbar
Reservierungen und Arbeitstermine SHALL in Liste und Kalender erkennbar verschieden gestaltet sein: durch eigenes Symbol, eigene Beschriftung („Reservierung“ bzw. „Arbeitstermin“), eigene Randfarbe und Form, nicht allein durch Farbe. Eigene Beteiligung SHALL zusätzlich mit Symbol und Text („dabei“, „angemeldet“) erkennbar sein.

#### Scenario: Unterscheidung
- **WHEN** ein Kalendertag eine Reservierung und einen Arbeitstermin enthält
- **THEN** zeigt die Reservierung das Bootssymbol mit dem Bootsnamen und der Arbeitstermin das Werkzeugsymbol mit dem Wort „Arbeitstermin“

#### Scenario: Ohne Farbwahrnehmung
- **WHEN** die Seite in Graustufen betrachtet wird
- **THEN** bleiben beide Arten durch Symbol und Beschriftung unterscheidbar

### Requirement: Farbthema der Segelgruppe
Die Oberfläche SHALL ihr Farbthema an der Webseite der Segelgruppe ausrichten: tiefes Marineblau für Kopf und Überschriften, ein mittleres Blau als Hauptfarbe für Aktionen, helle Cyantöne für Flächen und Hervorhebungen und ein warmes Gelb als Akzent. Jede Textfarbe SHALL auch auf ihrem Hintergrund 4,5:1 erreichen; helle Cyantöne SHALL nur als Fläche oder Zierde dienen. Das Thema SHALL im dunklen Farbschema ein gleichwertiges Gegenstück haben.

#### Scenario: Hauptaktion
- **WHEN** eine Seite mit Hauptaktion geöffnet wird
- **THEN** erscheint die Schaltfläche in der Hauptfarbe mit Text, der 4,5:1 erreicht

#### Scenario: Dunkles Schema
- **WHEN** das System das dunkle Schema verlangt
- **THEN** erscheinen Marineblau und Blau als dunkle Flächen mit hellen Cyan-Akzenten, ohne Kontrastverlust

### Requirement: Archiv für Nicht-Aktuelles
Datensätze, die nicht mehr aktuell sind (beendete Ämter, ausgeschiedene Mitglieder, aufgehobene und abgelaufene Sperren, vergangene Reservierungen und Arbeitstermine), SHALL aus allen Tagesansichten verschwinden und ausschließlich im Archiv beim Protokoll erscheinen. Das Archiv SHALL nach Art und Zeitraum filterbar sein.

#### Scenario: Ehemaliges Amt
- **WHEN** ein Amt beendet wurde
- **THEN** erscheint es nicht mehr in Ämterübersicht, Mitgliederseite und Mitgliederliste, sondern im Archiv unter „Ämter“

#### Scenario: Archiv filtern
- **WHEN** der Vorstand das Archiv nach „Reservierungen“ und dem Jahr 2025 filtert
- **THEN** erscheinen die vergangenen Reservierungen dieses Jahres

#### Scenario: Nicht berechtigt
- **WHEN** Greta ohne Amt das Archiv öffnen will
- **THEN** wird dies mit 403 abgelehnt

### Requirement: Archiv nur für Berechtigte, Finanzen ausgenommen
Das Archiv SHALL nur einsehen dürfen, wer das Protokoll vollständig einsehen darf (Vorstand und Revisoren). Finanzielle Datensätze wie stornierte Forderungen sind keine Archivdaten und SHALL in der Kontoansicht sichtbar bleiben.

#### Scenario: Stornierte Forderung
- **WHEN** der Kassenwart das Konto eines Mitglieds mit einer stornierten Forderung öffnet
- **THEN** ist die Forderung dort weiterhin mit Stornogrund zu sehen

### Requirement: Filter in einer Filterleiste
Suche und Filter von Listen mit mehreren Filtern (zuerst das Mitgliederverzeichnis mit Name, Mitgliedsart und Segelschein) SHALL in einer Filterleiste neben der Liste stehen, nicht über ihr. Auf schmalen Bildschirmen SHALL die Filterleiste hinter einer Schaltfläche „Filter“ einklappbar über der Liste liegen und die Zahl aktiver Filter anzeigen.

#### Scenario: Desktop
- **WHEN** das Mitgliederverzeichnis bei 1280 px Breite geöffnet wird
- **THEN** stehen Namenssuche, Mitgliedsart und Segelschein in einer Leiste neben der Liste

#### Scenario: Handy
- **WHEN** das Verzeichnis bei 375 px Breite mit dem Filter „Mitgliedsart: studentisch“ geöffnet wird
- **THEN** ist die Filterleiste eingeklappt und die Schaltfläche zeigt „Filter (1)“

#### Scenario: Ohne JavaScript
- **WHEN** ohne JavaScript nach „peter“ gefiltert wird
- **THEN** erscheint die gefilterte Liste auf einer vollständigen Seite mit weiterhin gefüllter Filterleiste

### Requirement: Filter bleiben verlinkbar und ohne JavaScript nutzbar
Filter in der Filterleiste SHALL in der Adresse stehen und ohne JavaScript funktionieren; die Namenssuche SHALL mit htmx die Liste ohne Seitenwechsel aktualisieren.

#### Scenario: Filter in der Adresse
- **WHEN** die Adresse eines gefilterten Verzeichnisses kopiert und erneut geöffnet wird
- **THEN** sind dieselben Filter in der Leiste gesetzt und dieselben Mitglieder gelistet

### Requirement: Sortierbare Tabellen
Jede Tabelle mit mehreren Zeilen SHALL durch Klick auf einen Spaltenkopf nach dieser Spalte sortierbar sein; ein weiterer Klick SHALL die Richtung umkehren. Die aktive Spalte und Richtung SHALL durch Symbol und für Hilfstechnik lesbaren Text (`aria-sort`) erkennbar sein, die Sortierung SHALL in der Adresse stehen und ohne JavaScript funktionieren.

#### Scenario: Nach Spalte sortieren
- **WHEN** der Kassenwart in der Forderungsliste auf den Kopf „Betrag“ klickt
- **THEN** ist die Liste nach Betrag aufsteigend sortiert, der Kopf zeigt ein Pfeilsymbol, und ein weiterer Klick kehrt die Richtung um

#### Scenario: Verlinkbar
- **WHEN** die Adresse einer sortierten Tabelle kopiert und erneut geöffnet wird
- **THEN** ist dieselbe Sortierung gesetzt

#### Scenario: Am Handy
- **WHEN** eine Tabelle bei 375 px Breite als Karten erscheint
- **THEN** bietet ein Auswahlfeld „Sortieren nach“ mit Richtung dieselben Spalten an

### Requirement: Sortierreihenfolge nach Art der Spalte
Zahlen, Beträge und Daten SHALL nach ihrem Wert, Text nach deutscher Sortierreihenfolge (Umlaute eingeordnet, Groß-/Kleinschreibung ignoriert) sortiert werden. Spalten ohne sinnvolle Ordnung, etwa Aktionen, SHALL nicht sortierbar sein.

#### Scenario: Umlaute
- **WHEN** das Verzeichnis nach „Name“ sortiert wird
- **THEN** steht „Möller“ zwischen „Lorenzen“ und „Petersen“

#### Scenario: Aktionsspalte
- **WHEN** eine Tabelle eine Aktionsspalte hat
- **THEN** ist deren Kopf kein Sortierlink

### Requirement: Bootstyp als Symbol
Wo ein Boot in Listen, Karten und im Kalender erscheint, SHALL sein Bootstyp als Symbol dargestellt werden: ein Symbol für Jolle und eines für Dickschiff (Kielboot). Der genaue Typ mit Klasse, zum Beispiel „Kielboot · Matcher 37“, SHALL als Tooltip und als für Hilfstechnik lesbarer Text erscheinen; der Bootsname bleibt sichtbarer Text.

#### Scenario: Bootsliste
- **WHEN** die Bootsliste geöffnet wird
- **THEN** zeigt die Zeile der Filou das Jollen-Symbol und die der Trigon das Dickschiff-Symbol, deren Tooltip „Kielboot · Matcher 37“ nennt

#### Scenario: Kalendereintrag
- **WHEN** eine Reservierung der Trigon im Kalender erscheint
- **THEN** steht vor dem Bootsnamen das Dickschiff-Symbol mit demselben Tooltip

#### Scenario: Hilfstechnik
- **WHEN** ein Screenreader die Zeile vorliest
- **THEN** nennt er „Kielboot, Matcher 37“ zusätzlich zum Namen

### Requirement: Personennamen als Links
Überall, wo der Name eines Mitglieds erscheint (Listen, Karten, Kalender, Reservierungen, Ämter, Aufgaben, Bestätigungen, Konten, Protokoll, Kontomenü), SHALL der Name ein Link zur Profilseite dieses Mitglieds sein. Namen von Personen ohne Profilseite (Gäste, ausgeschiedene Mitglieder, Namen im Protokolltext von gelöschten Datensätzen) SHALL als einfacher Text erscheinen. Der Link SHALL einen unterscheidbaren Fokus haben und in Tabellen und Karten die Zeile nicht als Ganzes verlinken.

#### Scenario: Skipper in der Reservierung
- **WHEN** eine Reservierung mit Skipper Anna und Gast „Max Gast“ angezeigt wird
- **THEN** ist „Anna Petersen“ ein Link zu ihrer Profilseite und „Max Gast“ einfacher Text

#### Scenario: Eigener Name
- **WHEN** Anna im Kontomenü ihren Namen anklickt
- **THEN** öffnet ihre Profilseite

#### Scenario: Ausgeschiedenes Mitglied
- **WHEN** das Protokoll einen Eintrag eines ausgeschiedenen Mitglieds zeigt
- **THEN** erscheint dessen Name als Text ohne Link

### Requirement: Einbindung in den Auftritt der Segelgruppe
Kopf und Fuß der Oberfläche SHALL sich am Auftritt der Webseite der Segelgruppe orientieren: ein heller Kopf mit dem Vereinslogo, dem Titel „Segelgruppe“ und der Zeile „an der Fachhochschule Kiel e.V.“ sowie dem Namen des Bereichs „Vereinsakte“, Überschriften in der Schrift der Webseite (Amaranth), ein marineblauer Fuß mit Urheberzeile und Verweisen auf die Webseite, Impressum und Datenschutz.

#### Scenario: Kopf
- **WHEN** eine beliebige Seite geöffnet wird
- **THEN** zeigt der Kopf das Logo, „Segelgruppe“, „an der Fachhochschule Kiel e.V.“ und „Vereinsakte“, und das Logo führt zur Startseite

#### Scenario: Fuß
- **WHEN** eine beliebige angemeldete Seite bis zum Ende gescrollt wird
- **THEN** steht ein marineblauer Fuß mit der Urheberzeile und Links zur Webseite, zum Impressum und zum Datenschutz

### Requirement: Eigene Ressourcen für den Auftritt
Logo, Schrift und Symbole des Auftritts SHALL von der eigenen Adresse der Vereinsakte stammen. Das Logo SHALL zur Startseite der Vereinsakte führen, und ein Verweis „Zur Webseite“ SHALL zur Webseite der Segelgruppe führen.

#### Scenario: Eigene Ressourcen
- **WHEN** eine Seite mit Logo und Überschriften geladen wird
- **THEN** stammen Logo und Schrift von der Adresse der Vereinsakte und keine Anfrage geht an die Webseite der Segelgruppe

### Requirement: Durchgängig professioneller Eindruck
Alle Seiten SHALL einheitlich gestaltet sein, einschließlich Anmeldeseite, Fehlerseiten (403, 404, 409, 422, 500), leerer Zustände, Meldungen und der Druckansicht des Jahresnachweises mit Briefkopf der Segelgruppe. Es SHALL keine ungestalteten Standardelemente des Browsers (nackte Listen, Tabellen, Schaltflächen, Auswahlfelder) geben, Abstände SHALL einem festen Raster folgen und Texte SHALL ohne Rechtschreib- und Zeichensetzungsfehler sein.

#### Scenario: Anmeldeseite
- **WHEN** die Anmeldeseite ohne Sitzung geöffnet wird
- **THEN** erscheint sie im Auftritt der Segelgruppe mit Logo, Titel und einer klar hervorgehobenen Anmeldeaktion

#### Scenario: Fehlerseite
- **WHEN** eine nicht vorhandene Adresse aufgerufen wird
- **THEN** erscheint eine gestaltete Seite mit Symbol, verständlichem Text und Link zurück zur Startseite

#### Scenario: Druck
- **WHEN** der Jahresnachweis gedruckt wird
- **THEN** enthält der Ausdruck Briefkopf mit Logo und Vereinsname, aber keine Navigation, und passt auf eine A4-Seite

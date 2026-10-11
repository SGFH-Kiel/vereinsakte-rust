# Proposal

## Why

Die Weboberfläche zeigt auf fast jeder Seite alles gleichzeitig: neun gleichrangige Textlinks in der Navigation, Karten ohne Gewichtung, lange Formulare und selten genutzte Verwaltungsaktionen (Sperren, Widerrufen, Abnahme) stehen ungefiltert unter den eigentlichen Inhalten. Nutzer müssen lesen statt erkennen. Funktional ist alles da; es fehlt die Führung: Was ist wichtig, was ist der nächste Schritt, was ist nur für Ausnahmen? Mit Symbolen, einer klaren Typografie-Hierarchie und einem ruhigeren Seitenaufbau wird das ohne neue Fachlogik besser.

## What Changes

- **Navigation neu geordnet**: auf dem Desktop eine seitliche Navigation mit Symbolen und Gruppen (Persönlich, Verein, Verwaltung); auf dem Handy eine untere Tab-Leiste mit den vier häufigsten Bereichen und „Mehr“. Konto und Abmelden wandern in ein Kontomenü. Weiterhin ohne JavaScript bedienbar.
- **Symbolsatz**: ein eigener, eingebetteter SVG-Satz (Navigation, Zustände, Aktionen, leere Zustände), ohne externe Ressourcen; Symbole ergänzen immer Text und tragen nie allein Bedeutung.
- **Typografie und Raster**: feste Schriftstufen (Seitentitel, Abschnitt, Beschriftung, Kennzahl), Überzeile mit Bereich, ruhigere Abstände, tabellarische Ziffern; Beschriftungen und Werte klar unterscheidbar.
- **Seitenaufbau mit Führung**: jede Seite beginnt mit Titel, einer Zustandszeile und genau einer Hauptaktion; weitere Aktionen liegen in einem „Weitere Aktionen“-Menü; selten benötigte oder zerstörende Formulare sind eingeklappt.
- **Startseite als Aufgabenseite**: oben „Was ist zu tun?“ mit Handlungshinweisen, darunter Kennzahlen mit Fortschrittsbalken (Arbeitsstunden) und Zustandssymbolen; Reihenfolge nach Dringlichkeit.
- **Listen kompakter**: Name mit Nebenzeile statt vieler Spalten, Initialen-Kreis bei Personen, Zustandsplaketten mit Symbol, Zeilenaktionen als Symbol-Schaltflächen mit Beschriftung für Hilfstechnik; sinnvolle leere Zustände mit nächstem Schritt.
- **Formulare gegliedert**: Felder in benannten Abschnitten, Pflicht und Hinweis klar abgesetzt, Aktionsleiste am Ende (auf dem Handy am unteren Rand sichtbar).
- **Auf den Normalfall optimiert**: Voreinstellungen und Standardansichten bedienen den häufigsten Fall (kommende Reservierungen und Termine, aktuelle Besetzung, laufende Saison); Ausnahmen sind einen Schritt entfernt.
- **Filterleiste statt Filterzeile**: Suche und Filter des Mitgliederverzeichnisses (und weiterer Listen mit mehreren Filtern) stehen in einer Leiste neben der Liste, am Handy einklappbar.
- **Alle Tabellen sortierbar** per Klick auf den Spaltenkopf (aufsteigend/absteigend, in der Adresse, ohne JavaScript; am Handy per Auswahl).
- **Mitgliederliste mit Symbolen**: Segelscheine, Funkscheine und Mitgliedsart erscheinen als Symbole mit Tooltip statt als Text. Ebenso der Bootstyp (zwei Symbole: Jolle, Dickschiff; der genaue Typ mit Klasse im Tooltip).
- **Tour ergibt sich aus der Buchungsdauer**: das Tour-Kennzeichen im Reservierungsformular entfällt; eine Reservierung gilt als Tour, sobald sie mindestens eine Übernachtung umfasst. **BREAKING** für Formular und gespeichertes Kennzeichen (wird ignoriert und aus dem Zeitraum berechnet).
- **Neuer Bereich „Kalender“** statt „Reservierungen“: zeigt Reservierungen und Arbeitstermine gemeinsam, wahlweise als Liste (nach Woche und Tag gruppiert) oder als Kalender (Monat/Woche), mit Filtern und verlinkbarer Adresse. Arbeitstermine sind durch Symbol, Beschriftung und Form klar von Reservierungen unterscheidbar. Reservieren geschieht über das Boot (Bootsseite und -liste), es gibt keinen Menüpunkt „Reservierungen“ mehr.
- **Farbthema der Segelgruppe**: Marineblau, Blau, Cyan und Gelb nach der Vereinswebseite, mit Dunkelvariante und geprüftem Kontrast.
- **Archiv statt Altlasten**: nicht mehr aktuelle Datensätze (beendete Ämter, ausgeschiedene Mitglieder, aufgehobene Sperren, vergangene Reservierungen und Arbeitstermine) verschwinden aus den Tagesansichten und stehen nur noch im Archiv beim Protokoll.
- Aufgaben als Kanban-Board mit Kommentaren, Bildern und Markdown-Beschreibung sind **nicht** Teil dieser Änderung, sondern die eigene Änderung `add-aufgabenboard`; hier erscheinen „Aufgaben“ nur als Menüpunkt auf oberster Ebene.
- Keine Änderung an Rechten, Fachregeln, Adressen oder Datenhaltung. Alle bisherigen Funktionen bleiben erreichbar.

## Capabilities

### New Capabilities

(keine)

### Modified Capabilities

- `oberflaeche`: Navigation (Gruppen, Symbole, Tab-Leiste, Kontomenü, „Kalender“ statt „Reservierungen“, „Aufgaben“ oben), Startseite (Aufgabenhinweise, Fortschritt); neue Anforderungen für Symbolsatz, Typografie, Seitenaufbau mit einer Hauptaktion, eingeklappte Seltenaktionen, kompakte Listen, gegliederte Formulare, Normalfall-Optimierung, Kalenderbereich mit Liste/Kalender, Gruppierung nach Tagen, Unterscheidung der Eintragsarten, Farbthema und Archiv.
- `bootsreservierung`: Tour-Kennzeichen wird aus der Dauer abgeleitet statt angegeben.
- `reservierungsverwaltung`: Reservierungen erscheinen im Bereich Kalender (Kalender bisher nur in der Spec, nicht umgesetzt), Anlegen vom Boot aus ohne Tour-Angabe, Standardzeitraum „kommende“.
- `arbeitsverwaltung`: Arbeitstermine erscheinen im Bereich Kalender, Standardansicht „kommende“.
- `amtsverwaltung`: beendete Ämter nur noch im Archiv statt in einer eigenen Amtshistorie.
- `mitgliederverzeichnis`: Scheine, Funklizenzen und Mitgliedsart als Symbole mit Tooltip.
- `tourgebuehren`: „gekennzeichnet“ wird zu „gilt als Tour“.

## Impact

- `crates/vereinsakte-core` (Reservierung: Tour berechnet), `crates/vereinsakte-app` (Zeitraum-Standards, Archiv-Ansichten), `crates/vereinsakte-web` (Kalenderansichten, Archiv), Reservierungsformular ohne Tour-Feld; Speicherspalte `tour` bleibt bestehen und wird nicht mehr gelesen (keine Migration).
- `crates/vereinsakte-web`: `templates/*` (Basislayout, nahezu alle Seiten und Bausteine), `static/app.css`, neue Symbol-Vorlage `templates/_icons.html`, `src/pages.rs` (Navigationsgruppen, Initialen), kleine View-Erweiterungen für Hinweise auf der Startseite.
- `crates/vereinsakte-web/src/tests.rs`: Navigations- und Layout-Tests angepasst, neue Tests (Symbolsatz vollständig, keine Inline-Styles, Fortschrittselement, eingeklappte Aktionen).
- `tools/screenshots.mjs`, `README.md`: Screenshots und Beschreibung aktualisieren.
- Keine neuen Abhängigkeiten, keine Datenbankänderung, keine Schnittstellenänderung.
- Reihenfolge: **2. von 4**, nach `add-kontakt-sichtbarkeit` (Scheine, Mitgliedsart und Kontaktdaten sind dann bereits für alle sichtbar, die Mitgliederliste kann sie als Symbole zeigen); vor `add-aufgabenboard`, das Navigation, Seitenkopf, Archiv und Bausteine übernimmt.

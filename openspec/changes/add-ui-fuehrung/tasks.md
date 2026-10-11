# Tasks

## 1. Fundament: Symbole, Typografie, Raster

- [ ] 1.1 Symbolsatz `templates/_icons.html` mit allen benötigten Symbolen (Jolle, Dickschiff, Segelschein, Funkschein, Mitgliedsarten, Navigation, Zustände ok/warn/gesperrt/offen, Aktionen bearbeiten/löschen/hinzufügen/suchen/filtern/mehr, leere Zustände) und Makro `icon(name)`; Test: jedes in Vorlagen verwendete `#i-…` existiert im Sprite, und keine Seite enthält `style=`.
- [ ] 1.2 CSS-Tokens für Schriftstufen, Abstände, Flächen und Initialen-Farben `av-0…5` im Farbthema der Segelgruppe (Marine, Blau, Cyan, Gelb; hell und dunkel; Kontrast ≥ 4,5:1 für jede Text-Hintergrund-Paarung per `tools/contrast.mjs` geprüft) und Basisregeln für Überschriften, Zahlen und Zeilenlänge; Test: Hell- und Dunkel-Screenshot der Startseite zeigt die Stufen.
- [ ] 1.3 Makros `page_head` (Überzeile, Titel, Zeile, Hauptaktion, „Weitere Aktionen“-Menü), `panel` (eingeklappter Bereich), `empty_state`, `badge` mit Symbol; Test: Web-Test, dass jede Seite genau ein `<h1>` hat und Überschriftenstufen nicht springen.

- [ ] 1.4 Sortierbaustein: `sort`/`dir` in `ListQuery`, deutscher Textvergleich, Makro `th_sort` mit Symbol und `aria-sort`, Auswahl „Sortieren nach“ für die Kartenansicht; Unit-Tests für Umlaute (Möller zwischen Lorenzen und Petersen), Beträge, Daten und unbekannte Schlüssel.

- [ ] 1.5 `PersonRef` in den Views und Makro `person`, Stil für Personenlinks; Unit-Test: aktives Mitglied → Link, Gast und Ausgeschiedener → Text.

- [ ] 1.6 Assets einbinden: Vereinslogo als `static/img/sgfh-logo.svg`, Amaranth Regular/Bold als WOFF2 mit `@font-face`, OFL-Lizenztext im Ordner, Favicon und `theme-color`; Prüfung: Seitenaufruf lädt Logo und Schrift von der eigenen Adresse (Test auf Pfade, HTTP 200, korrekter Typ), Netzwerk-Mitschnitt im Playwright-Lauf zeigt keine Anfrage nach außen.

## 2. Navigation und Rahmen

- [ ] 2.1 `pages.rs::navigation` liefert Gruppen (Persönlich, Verein, Verwaltung) mit Symbol, Ziel und Beschriftung nach Rechten sowie Initialen des Mitglieds; Unit-Test: Greta ohne „Finanzen“, Eva mit „Finanzen“, Gruppenreihenfolge, „Kalender“ statt „Reservierungen“ und „Aufgaben“ in der Gruppe Verein.
- [ ] 2.2 `base.html`: Seitenleiste ab 64 rem, Symbolleiste 48–64 rem, untere Tab-Leiste (Start, Boote, Kalender, Arbeit, Mehr) unter 48 rem mit Checkbox-Blatt für „Mehr“, Kontomenü mit Name, Ämtern und Abmelden; Tests: aktueller Bereich `aria-current`, Menü ohne JavaScript (Checkbox und Label vorhanden), Entwicklungsleiste bleibt.
- [ ] 2.3 Meldungen (`flash`) und Fehlerseiten im neuen Stil mit Symbol; Test: bestehende Flash- und 403/404/409/422-Tests grün.
- [ ] 2.4 Kopf und Fuß im Stil der Webseite (heller Kopf mit Logo, Titel, Untertitel, Plakette „Vereinsakte“, Kontomenü; marineblauer Fuß mit Links zu Webseite, Impressum, Datenschutz und Urheberzeile); Tabellenstil wie auf der Webseite; Tests: Kopf enthält Logo und Titel, Fuß enthält die drei Links, Logo führt zu `/`.
- [ ] 2.5 Anmeldeseite (auch Entwicklungsanmeldung), Fehlerseiten 403/404/409/422/500, leere Zustände und Druckansicht des Jahresnachweises mit Briefkopf im neuen Stil; Tests: Fehlerseiten zeigen Symbol, Text und Link zur Startseite, Druck enthält Briefkopf ohne Navigation (Playwright-PDF, eine A4-Seite).

## 3. Startseite und „Meine Daten“

- [ ] 3.1 Startseite: „Was ist zu tun?“ aus vorhandenen Zahlen mit Dringlichkeitsreihenfolge und „Alles erledigt“, Kennzahlen mit Symbol, `<progress>` für Arbeitsstunden; Tests: Clara sieht „Es fehlen 4,5 Arbeitsstunden“ mit Link, ein erfülltes Mitglied sieht „Alles erledigt“, Fortschrittswert korrekt.
- [ ] 3.2 „Meine Daten“ und `_member_details.html`: Zustandsstreifen oben (Segelerlaubnis, Stunden, Konto), Stammdaten/Scheine/Konto als gegliederte Karten, Bootserlaubnisse als Plaketten; Test: vorhandene Seitentests bleiben grün, Zustandsstreifen mit Text sichtbar.

## 4. Listen und Detailseiten

- [ ] 4.1 Mitgliederverzeichnis: Initialen-Kreis, Name mit Nebenzeile (Ämter), Segelscheine, Funkscheine und Mitgliedsart als Symbole mit Tooltip und Text für Hilfstechnik (nur vorhandene Scheine), Filter in einer Filterleiste neben der Liste (am Handy einklappbar mit Zähler „Filter (n)“), leerer Zustand; Tests: Clara zeigt Symbole für SKS und UBI mit `title` und verstecktem Text, Suche und Filter mit und ohne htmx unverändert, Karten am Handy ohne seitliches Scrollen (Screenshot 375 px).
- [ ] 4.2 Mitgliedsseite: Kopf mit „Bearbeiten“ als Hauptaktion und „Weitere Aktionen“ (Jahresnachweis, Konto); Test: nur Berechtigte sehen die Aktionen, Menü ohne JavaScript erreichbar.
- [ ] 4.3 Boote und Bootsseite: Bootstyp als Symbol (Jolle/Dickschiff, Typ mit Klasse im Tooltip und als versteckter Text) über Makro `boat_type_icon`, auch im Kalender; Zustandskarte zuerst (Eignung, Freigabe, Sperre), Stammdaten kompakt, Sperren/Abnahme/Widerruf als eingeklappte Bereiche, bei Fehler geöffnet; Tests: Filou zeigt Jollen-, Trigon Dickschiff-Symbol mit Tooltip „Kielboot · Matcher 37“, Bereich standardmäßig geschlossen, nach Fehler offen, Rechte unverändert.
- [ ] 4.4 Ämter und Reservierungsdetail: Listen mit Nebenzeile und Zustandsplaketten, leere Zustände mit nächstem Schritt; Reservierungsformular gegliedert und vom Boot aus erreichbar (Hauptaktion „Reservieren“ auf Bootsseite, Zeilenaktion in der Bootsliste, Boot vorausgewählt); Tests: Link von der Bootsseite wählt das Boot vor, Konfliktfehler bleibt am Feld, kein Menüpunkt „Reservierungen“.
- [ ] 4.5 Arbeit (Stunden, Termine, Aufgaben, Bestätigungen, Übersicht): Fortschritt oben, Zeilenaktionen als beschriftete Symbol-Schaltflächen, Unternavigation mit Symbolen, leere Zustände; Tests: Löschen verlangt weiterhin Bestätigung, Schaltflächen haben `aria-label`.
- [ ] 4.6 Sortierung in allen Tabellen aktivieren (Verzeichnis, Boote, Reservierungsdetails, Arbeitsstunden, Bestätigungen, Termine, Aufgaben, Ämter, Finanzen, Protokoll, Archiv) mit sinnvoller Standardreihenfolge; Test: jede Tabelle der Seiten mit mehreren Zeilen hat mindestens einen sortierbaren Kopf, Klick ändert die Reihenfolge, Adresse enthält die Sortierung.
- [ ] 4.7 Finanzen, Konto, Jahresnachweis und Protokoll: Kennzahlenkacheln, Zustandsplaketten mit Symbol, Protokoll als kompakte Zeitleiste; Tests: Druckansicht des Nachweises ohne Navigation, vorhandene Finanztests grün.

## 5. Tour aus der Dauer

- [ ] 5.1 Kern: `Reservation::is_tour()` (Ende-Kalendertag nach Beginn-Kalendertag), Feld `tour` nicht mehr schreiben/lesen, Gebühren, Anzeige und Audit darauf umstellen; Unit-Tests für Nachmittag, 1.–7. Juli, Nachtfahrt über Mitternacht, verkürzte Tour; `cargo test -p vereinsakte-core -p vereinsakte-app -p vereinsakte-storage` grün.
- [ ] 5.2 Web: Tour-Feld aus dem Reservierungsformular entfernen, Heimrevier und Skipper vorbelegen, Tour-Plakette und Gebührenanzeige aus der Berechnung; Tests: Formular ohne Tour-Feld, siebentägige Reservierung zeigt „Tour“ und Gebühr, Nachmittag nicht.
- [ ] 5.3 Crewliste im Reservierungsformular mit Hinzufügen/Entfernen ohne JavaScript (Formular-Rerender ohne Speichern) und per htmx als Ausschnitt, Crew als Namenslinks, Doppelte und Skipper abgelehnt, unsichtbare Standard-Speichern-Schaltfläche; Textarea-Format entfällt auch im Parser; Tests: Clara und David hinzufügen, David entfernen, nur Clara gespeichert, Link auf `/mitglieder/103`, doppelter Eintrag → „bereits an Bord“, Eingaben bleiben beim Rerender, Eingabetaste speichert, Reservierungsdetail zeigt Crew als Links.
- [ ] 5.4 Gäste und Mindestbesatzungsprüfung entfernen: `guests` und Besatzungsprüfung aus Kern, Anwendung und Formular; Gebührenermittlung und Abrechnung nur für Skipper, Co-Skipper, Crew; vorhandene Gäste beim Einlesen an die Bemerkung anhängen; Tests: Trigon außerhalb nur mit Skipper wird gespeichert, Tour mit Gast erzeugt nur Mitgliederforderungen, alte Akte mit Gast zeigt ihn in der Bemerkung, `cargo test --workspace` grün.
- [ ] 5.5 Bemerkung als großes mehrzeiliges Feld (acht Zeilen, 5.000 Zeichen, Anzeige mit Zeilenumbrüchen); Tests: 5.001 Zeichen abgelehnt, Umbrüche im Detail erhalten, Screenshot bei 375 px.

## 6. Bereich Kalender

- [ ] 6.1 Kalender-Baustein: gemeinsame Sammlung `CalendarEntry` aus Reservierungen und Arbeitstermine, Gruppierung nach Woche und Tag, Monatsraster mit `<caption>`, Wochenansicht, Blättern, Heute; Unit-Tests für Mehrtägiges über Monatsgrenzen, „weiter bis“, Wochenbeginn Montag und leere Tage.
- [ ] 6.2 Seite `/kalender` mit Auswahl Liste/Kalender, Filtern Art/Boot/„nur meine“ und Standard ab heute; `/reservierungen` und `/arbeit/termine` leiten um; Tests: Samstag zeigt Reservierung und Arbeitstermin gruppiert, Art-Filter, Adresse enthält Ansicht und Filter, Umleitungen.
- [ ] 6.3 Gestaltung der Eintragsarten (Symbol, Beschriftung, Kante/Form, Tönung) in Liste und Raster, Beteiligung mit Symbol und Text; Prüfung: Graustufen-Screenshot zeigt beide Arten unterscheidbar, Handy-Screenshot ohne seitliches Scrollen, Tastaturbedienung per Playwright.

## 7. Archiv

- [ ] 7.1 Tagesansichten auf Aktuelles beschränken (Ämter, Mitgliedsseiten, Sperren, Reservierungen, Termine); Test: beendetes Amt fehlt in Ämterübersicht, Mitgliedsseite und Liste.
- [ ] 7.2 Seite `/protokoll/archiv` mit Reitern und Filtern für Art und Zeitraum, Zugriff Vorstand und Revisoren, `/aemter/historie` leitet um; Tests: Ben gibt Amt ab → erscheint im Archiv, Greta erhält 403, Filter nach Reservierungen 2025, stornierte Forderung bleibt im Konto sichtbar.

- [ ] 7.3 Alle Stellen mit Personennamen auf `PersonRef` umstellen (Verzeichnis, Ämter, Reservierungen, Kalender, Aufgaben, Bestätigungen, Arbeitsübersicht, Konten, Protokoll, Archiv, Kontomenü); Test: auf jeder Seite des End-to-End-Laufs, die den Namen eines aktiven Mitglieds zeigt, steht ein Link `/mitglieder/{nr}`; „Max Gast“ bleibt Text.

## 8. Formulare

- [ ] 8.1 `_forms.html` um Abschnitt-Makro, Pflichtkennzeichnung (`*`, `aria-required`) und Legende erweitern; haftende Aktionsleiste unter 48 rem ohne Feldüberdeckung, Tab-Leiste auf Formularseiten ausgeblendet; Test: Pflichtfelder gekennzeichnet, Fehlermeldungen unverändert am Feld.
- [ ] 8.2 Arbeitseinsatz-Formular (neu und ändern) mit Umschalter „Stunden / Zeitraum“ (Radiobuttons, CSS-Ein-/Ausblenden), Server wertet nur die gewählte Umfangsform; Tests: Standard zeigt nur Stunden, Zeitraum zeigt Beginn/Ende, Fehler (Ende vor Beginn) behält „Zeitraum“ und Eingaben, erzwungener Aufruf mit beiden Formen wird abgelehnt, Playwright-Screenshot bei 375 px für beide Zustände.
- [ ] 8.3 Alle Formulare (Mitglied, Amt, Boot, Reservierung, Arbeitseinsatz (Abschnitte), Termin, Aufgabe, Finanzen) auf Abschnitte umstellen, Texte beibehalten; Test: `cargo test --workspace` grün und Screenshot „Mitglied aufnehmen“ bei 375 px zeigt letzte Felder unverdeckt.

## 9. Abschluss

- [ ] 9.0 Gesamteindruck abnehmen: Screenshots von Webseite (Start, Termine) und Vereinsakte (Start, Mitglieder, Boot, Kalender, Formular, Anmeldung, Fehler, Druck) nebeneinander prüfen: Logo, Farben, Überschriftenschrift, Fuß passen zusammen; Checkliste (keine nackten Browser-Elemente, Raster, Fokus, Texte ohne Fehler) abgehakt und Abweichungen behoben.
- [ ] 9.1 `tools/screenshots.mjs` an neue Struktur anpassen, alle Seiten bei 375 und 1280 px, hell und dunkel; Prüfung: kein horizontales Scrollen, keine Inline-Style-Verletzung in der Konsole, Sichtprüfung der Hauptseiten.
- [ ] 9.2 End-to-End-Test (alle Seiten × alle Mitglieder) und `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` grün; README (Oberfläche, Tour-Regel, Archiv, Screenshots) aktualisieren und die Befehle laufen wie beschrieben.

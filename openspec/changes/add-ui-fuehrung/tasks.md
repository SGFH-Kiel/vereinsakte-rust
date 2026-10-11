# Tasks

## 1. Fundament: Symbole, Typografie, Raster

- [ ] 1.1 Symbolsatz `templates/_icons.html` mit allen benötigten Symbolen (Navigation, Zustände ok/warn/gesperrt/offen, Aktionen bearbeiten/löschen/hinzufügen/suchen/filtern/mehr, leere Zustände) und Makro `icon(name)`; Test: jedes in Vorlagen verwendete `#i-…` existiert im Sprite, und keine Seite enthält `style=`.
- [ ] 1.2 CSS-Tokens für Schriftstufen, Abstände, Flächen und Initialen-Farben `av-0…5` (hell und dunkel, Kontrast ≥ 4,5:1 per Skript in `tools/` geprüft) und Basisregeln für Überschriften, Zahlen und Zeilenlänge; Test: Hell- und Dunkel-Screenshot der Startseite zeigt die Stufen.
- [ ] 1.3 Makros `page_head` (Überzeile, Titel, Zeile, Hauptaktion, „Weitere Aktionen“-Menü), `panel` (eingeklappter Bereich), `empty_state`, `badge` mit Symbol; Test: Web-Test, dass jede Seite genau ein `<h1>` hat und Überschriftenstufen nicht springen.

## 2. Navigation und Rahmen

- [ ] 2.1 `pages.rs::navigation` liefert Gruppen (Persönlich, Verein, Verwaltung) mit Symbol, Ziel und Beschriftung nach Rechten sowie Initialen des Mitglieds; Unit-Test: Greta ohne „Finanzen“, Eva mit „Finanzen“, Gruppenreihenfolge, „Aufgaben“ in der Gruppe Verein.
- [ ] 2.2 `base.html`: Seitenleiste ab 64 rem, Symbolleiste 48–64 rem, untere Tab-Leiste (Start, Boote, Reservierungen, Arbeit, Mehr) unter 48 rem mit Checkbox-Blatt für „Mehr“, Kontomenü mit Name, Ämtern und Abmelden; Tests: aktueller Bereich `aria-current`, Menü ohne JavaScript (Checkbox und Label vorhanden), Entwicklungsleiste bleibt.
- [ ] 2.3 Meldungen (`flash`) und Fehlerseiten im neuen Stil mit Symbol; Test: bestehende Flash- und 403/404/409/422-Tests grün.

## 3. Startseite und „Meine Daten“

- [ ] 3.1 Startseite: „Was ist zu tun?“ aus vorhandenen Zahlen mit Dringlichkeitsreihenfolge und „Alles erledigt“, Kennzahlen mit Symbol, `<progress>` für Arbeitsstunden; Tests: Clara sieht „Es fehlen 4,5 Arbeitsstunden“ mit Link, ein erfülltes Mitglied sieht „Alles erledigt“, Fortschrittswert korrekt.
- [ ] 3.2 „Meine Daten“ und `_member_details.html`: Zustandsstreifen oben (Segelerlaubnis, Stunden, Konto), Stammdaten/Scheine/Konto als gegliederte Karten, Bootserlaubnisse als Plaketten; Test: vorhandene Seitentests bleiben grün, Zustandsstreifen mit Text sichtbar.

## 4. Listen und Detailseiten

- [ ] 4.1 Mitgliederverzeichnis: Initialen-Kreis, Name mit Nebenzeile (Ämter), Segelscheine, Funkscheine und Mitgliedsart als Symbole mit Tooltip und Text für Hilfstechnik (nur vorhandene Scheine), Filter am Handy in einem einklappbaren Bereich, leerer Zustand; Tests: Clara zeigt Symbole für SKS und UBI mit `title` und verstecktem Text, Suche und Filter mit und ohne htmx unverändert, Karten am Handy ohne seitliches Scrollen (Screenshot 375 px).
- [ ] 4.2 Mitgliedsseite: Kopf mit „Bearbeiten“ als Hauptaktion und „Weitere Aktionen“ (Jahresnachweis, Konto); Test: nur Berechtigte sehen die Aktionen, Menü ohne JavaScript erreichbar.
- [ ] 4.3 Boote und Bootsseite: Zustandskarte zuerst (Eignung, Freigabe, Sperre), Stammdaten kompakt, Sperren/Abnahme/Widerruf als eingeklappte Bereiche, bei Fehler geöffnet; Tests: Bereich standardmäßig geschlossen, nach Fehler offen, Rechte unverändert.
- [ ] 4.4 Reservierungen und Ämter: Liste mit Nebenzeile und Zustandsplaketten, leere Zustände mit nächstem Schritt, Formular gegliedert; Tests: leerer Zustand zeigt „Boot reservieren“, Konfliktfehler bleibt am Feld.
- [ ] 4.5 Arbeit (Stunden, Termine, Aufgaben, Bestätigungen, Übersicht): Fortschritt oben, Zeilenaktionen als beschriftete Symbol-Schaltflächen, Unternavigation mit Symbolen, leere Zustände; Tests: Löschen verlangt weiterhin Bestätigung, Schaltflächen haben `aria-label`.
- [ ] 4.6 Finanzen, Konto, Jahresnachweis und Protokoll: Kennzahlenkacheln, Zustandsplaketten mit Symbol, Protokoll als kompakte Zeitleiste; Tests: Druckansicht des Nachweises ohne Navigation, vorhandene Finanztests grün.

## 5. Tour aus der Dauer

- [ ] 5.1 Kern: `Reservation::is_tour()` (Ende-Kalendertag nach Beginn-Kalendertag), Feld `tour` nicht mehr schreiben/lesen, Gebühren, Anzeige und Audit darauf umstellen; Unit-Tests für Nachmittag, 1.–7. Juli, Nachtfahrt über Mitternacht, verkürzte Tour; `cargo test -p vereinsakte-core -p vereinsakte-app -p vereinsakte-storage` grün.
- [ ] 5.2 Web: Tour-Feld aus dem Reservierungsformular entfernen, Heimrevier und Skipper vorbelegen, Tour-Plakette und Gebührenanzeige aus der Berechnung; Tests: Formular ohne Tour-Feld, siebentägige Reservierung zeigt „Tour“ und Gebühr, Nachmittag nicht.

## 6. Kalender und Liste

- [ ] 6.1 Kalender-Baustein (Monatsraster mit `<caption>`, Wochenansicht als Tagesliste, Blättern, Heute) und Umschalter „Liste / Kalender“, Adresse `?ansicht=…&monat=…`; Unit-Tests für Mehrtägiges über Monatsgrenzen und Wochenbeginn Montag.
- [ ] 6.2 Reservierungen: Kalender neben der Liste, Standard ab heute, gleiche Filter; Tests: Kalender zeigt Reservierung an allen Tagen, Klick-Ziel stimmt, Handy-Screenshot ohne seitliches Scrollen.
- [ ] 6.3 Arbeitstermine: Kalender neben der Liste mit freien Plätzen, Standard ab heute; Tests wie 6.2 sowie An-/Abmelden aus der Terminseite unverändert.

## 7. Archiv

- [ ] 7.1 Tagesansichten auf Aktuelles beschränken (Ämter, Mitgliedsseiten, Sperren, Reservierungen, Termine); Test: beendetes Amt fehlt in Ämterübersicht, Mitgliedsseite und Liste.
- [ ] 7.2 Seite `/protokoll/archiv` mit Reitern und Filtern für Art und Zeitraum, Zugriff Vorstand und Revisoren, `/aemter/historie` leitet um; Tests: Ben gibt Amt ab → erscheint im Archiv, Greta erhält 403, Filter nach Reservierungen 2025, stornierte Forderung bleibt im Konto sichtbar.

## 8. Formulare

- [ ] 8.1 `_forms.html` um Abschnitt-Makro, Pflichtkennzeichnung (`*`, `aria-required`) und Legende erweitern; haftende Aktionsleiste unter 48 rem ohne Feldüberdeckung, Tab-Leiste auf Formularseiten ausgeblendet; Test: Pflichtfelder gekennzeichnet, Fehlermeldungen unverändert am Feld.
- [ ] 8.2 Alle Formulare (Mitglied, Amt, Boot, Reservierung, Arbeitseinsatz, Termin, Aufgabe, Finanzen) auf Abschnitte umstellen, Texte beibehalten; Test: `cargo test --workspace` grün und Screenshot „Mitglied aufnehmen“ bei 375 px zeigt letzte Felder unverdeckt.

## 9. Abschluss

- [ ] 9.1 `tools/screenshots.mjs` an neue Struktur anpassen, alle Seiten bei 375 und 1280 px, hell und dunkel; Prüfung: kein horizontales Scrollen, keine Inline-Style-Verletzung in der Konsole, Sichtprüfung der Hauptseiten.
- [ ] 9.2 End-to-End-Test (alle Seiten × alle Mitglieder) und `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` grün; README (Oberfläche, Tour-Regel, Archiv, Screenshots) aktualisieren und die Befehle laufen wie beschrieben.

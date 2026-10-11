# Tasks

## 1. Kern und Speicher

- [ ] 1.1 `TaskStatus::Waiting` (`waiting`), `TaskComment`, `TaskImage`-Metadaten im Kern; `set_status` löscht `done_at` auch für `Waiting`; Unit-Tests für alle Übergänge inkl. Wiederöffnen, `cargo test -p vereinsakte-core` grün.
- [ ] 1.2 Migration `0003_task_attachments.sql` (Kommentare, Bilder als BLOB, Fremdschlüssel mit Cascade) und Lesen/Schreiben im Speicher mit Versionen; Test: Kommentar und Bild round-trip, Kaskade beim Löschen der Aufgabe, Listenabfragen laden keine Bilddaten (Test prüft Abfrage-Spalten).
- [ ] 1.3 Saisonfilter: `board(season_start)` liefert Aufgaben pro Spalte ohne erledigte aus früheren Saisons und `archived_tasks()`; Unit-Tests für Mai 2026 erledigt im Juli 2026 (DONE) und im Januar 2027 (Archiv) sowie Wiederöffnen.

## 2. Anwendungsfälle und Rechte

- [ ] 2.1 Use Cases: Aufgabe anlegen/ändern, Status verschieben (mit Version), Beschreibung ändern, Kommentar anlegen/ändern/löschen (15-Minuten-Regel), Bild hochladen/löschen mit Prüfung von Typ (Magic Bytes), Größe, Anzahl; Tests je Regel inkl. 403 für Fremde und Konflikt bei veralteter Version.
- [ ] 2.2 Rechtematrix und Protokollaktionen erweitern (`task.description_changed`, `task.comment_*`, `task.image_*`), `ACTIONS`-Sweep-Test und Rechtetests grün; Protokolltext enthält keine Beschreibungs- oder Kommentartexte (Test).

## 3. Web: Board und Aufgabenseite

- [ ] 3.1 Markdown-Renderer (`pulldown-cmark`) ohne rohes HTML und mit Link-Erlaubnisliste; Tests mit `<script>`, `javascript:`-Link, `onerror`, Aufgabenlisten, Fett; keine Bilder aus Markdown.
- [ ] 3.2 Seite `/aufgaben` (Menüpunkt oben): vier Spalten mit Anzahl, Karten mit Boot, Zuständigen, Fälligkeit („überfällig“ mit Text), Zählern; Filter „meine“/Boot in der Adresse; Verschieben-Formular je Karte; Altadresse `/arbeit/aufgaben` leitet um; Tests: Spalteninhalt, Filter, Rechte der Schaltflächen, 403 bei erzwungenem Aufruf.
- [ ] 3.3 Aufgabenseite `/aufgaben/{id}`: Kopf mit Status, Zuständige, Boot, Fälligkeit, Stunden, Verlauf; Beschreibung mit „Bearbeiten“ und Vorschau (POST ohne Speichern); Tests: Vorschau speichert nicht, Beschreibung fett gerendert, Fremde erhalten 403.
- [ ] 3.4 Kommentare: Liste, Formular, Korrektur/Löschen nach 15-Minuten-Regel; Tests: leerer Kommentar → Feldfehler, später Eingriff abgelehnt, Zähler auf der Karte.
- [ ] 3.5 Bilder: Mehrteil-Upload (Bodygrenze nur an dieser Route), Auslieferung mit `nosniff`, Sandbox-CSP und festem Typ, Alternativtext, Löschen; Tests: JPEG ok, Textdatei als PNG abgelehnt, SVG abgelehnt, 9-MB-Datei abgelehnt, ohne Anmeldung Umleitung, Fremde löschen → 403.
- [ ] 3.6 Neue Aufgabe anlegen (Amtsträger) mit Titel, Zuständigen, Boot, Fälligkeit, Schätzung; Test: erscheint in TODO und bei „meine“.

## 4. Gestaltung, Bedienung, Archiv

- [ ] 4.1 Board-Gestaltung: Spalten am Desktop nebeneinander, am Handy einklappbar untereinander, Karten mit Symbolen, Aufgabenseite zweispaltig (Inhalt/Meta); Prüfung: Screenshots 375 und 1280 px hell/dunkel ohne seitliches Scrollen.
- [ ] 4.2 `static/board.js`: Ziehen zwischen Spalten und Tastaturbedienung (Pfeiltasten verschieben), Rückfall bei Fehlern; Playwright-Test: Karte nach WIP ziehen, Seite neu laden, Status bleibt; ohne JavaScript funktioniert das Verschieben-Formular.
- [ ] 4.3 Archiv-Reiter „Aufgaben“ (aus `add-ui-fuehrung`) mit erledigten Aufgaben früherer Saisons; Test: Aufgabe aus 2026 erscheint 2027 dort, nicht mehr im Board.

## 5. Abschluss

- [ ] 5.1 Beispieldaten um eine WAIT-Aufgabe, einen Kommentar und ein kleines Bild ergänzen; End-to-End-Test (alle Seiten × alle Mitglieder) und Screenshots erweitern; `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` grün.
- [ ] 5.2 README: Aufgabenboard, Markdown-Umfang, Upload-Grenzen, Datenbankgröße/Backup und EXIF-Hinweis; die beschriebenen Befehle laufen wie dokumentiert.

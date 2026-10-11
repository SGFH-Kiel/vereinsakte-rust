# Design

## Context

Aufgaben liegen als `Task` (Titel, Beschreibung, Boot, Zuständige, Stunden, Fälligkeit, Status, Erledigt-Zeitpunkt) im Kern und in der Tabelle `tasks`; die Web-Seite `/arbeit/aufgaben` ist eine Liste mit Statusschaltern. Schreibende Zugriffe laufen über `ChangeRepository::commit` mit Versionen (optimistisch) und Protokolleinträgen; Rechte über `Capability`/`Authorizer`. Die Seiten sind askama-Vorlagen unter strenger CSP (Skripte nur von der eigenen Adresse, kein Inline-Stil). Motivation siehe proposal.md. `add-ui-fuehrung` liefert Navigation, Seitenkopf, Panels, Archiv und Kalenderbausteine.

## Goals / Non-Goals

**Goals:**
- Board, das ohne JavaScript voll funktioniert und mit JavaScript durch Ziehen angenehmer wird.
- Sichere Behandlung von Markdown und Uploads (Eingabe ist nicht vertrauenswürdig).
- Kommentare, Bilder und Beschreibung als Teil der Aufgabe, nachvollziehbar protokolliert.

**Non-Goals:**
- Keine Benachrichtigungen (E-Mail/Push), keine @-Erwähnungen, keine Anhänge außer Bildern.
- Keine frei konfigurierbaren Spalten, keine WIP-Limits, keine Unteraufgaben.
- Kein Echtzeit-Aktualisieren des Boards.

## Decisions

**Status „wartet“ als vierter Zustand.** `TaskStatus::Waiting` (serialisiert `waiting`), Spaltennamen TODO, WIP, WAIT, DONE wie gewünscht, mit deutschem Tooltip/Untertitel. Bestehende Daten bleiben gültig (kein Umbau von `open`/`in_progress`/`done`). Alternative: Spaltenzuordnung über Etiketten — mehr Flexibilität, aber unnötig.

**Verschieben ohne und mit JavaScript.** Jede Karte hat ein kleines Formular mit „Verschieben nach …“ (Auswahl der anderen Spalten, ohne JavaScript), POST auf `/aufgaben/{id}/status` mit Version. Mit JavaScript macht `static/board.js` (klein, ohne Bibliothek, HTML5-Drag-and-Drop plus Tastaturbedienung) dasselbe per `fetch` mit CSRF-Header und ersetzt das Board per htmx-ähnlichem Austausch; bei Fehler/Konflikt Rückfall auf Neuladen mit Hinweis. Alternative: htmx `Sortable`-Erweiterung (Fremdcode) — verworfen.

**Mobil.** Auf schmalen Bildschirmen erscheinen die Spalten untereinander als einklappbare Abschnitte (`<details>`, TODO und WIP offen) mit Anzahl; Verschieben nur per Auswahl. Auf dem Desktop vier Spalten nebeneinander, bei weniger Platz mit eigenem horizontalem Scrollen innerhalb des Boards, nicht der Seite.

**Saisonarchiv rein berechnet.** „Archiviert“ ist kein gespeicherter Zustand: eine erledigte Aufgabe, deren `done_at` vor Beginn der laufenden Saison liegt, wird vom Board gefiltert und im Archiv (aus `add-ui-fuehrung`) gelistet. Kein Hintergrundjob, kein Datenmigrationsschritt, Wiederöffnen wirkt sofort. Saisonbeginn = 1. Januar des laufenden Jahres (Kalenderjahr, wie Segel- und Beitragsjahr im Verein); bei abweichenden `Season`-Daten gilt deren Beginn.

**Anhänge: neue Tabellen, Bilddaten im BLOB.** Migration `0004_task_attachments.sql`: `task_comments(id, task_id FK, author, at, text, version)` und `task_images(id, task_id FK, uploader, at, filename, content_type, alt, size, data BLOB)`. Bilddaten in SQLite statt im Dateisystem: ein Backup, transaktional mit der Aufgabe, keine Pfadfragen; bei 20 × 8 MB je Aufgabe überschaubar für Vereinsgröße. Kommentare/Bilder lesen die Board-Ansicht nur als Zähler (eigene Abfrage), Bilddaten nie in Listenabfragen. Alternative: Dateisystem/Objektspeicher — zweiter Speicherort, schwieriger zu sichern. `ON DELETE CASCADE` wie bei Arbeitseinsätzen.

**Upload sicher.** axum `Multipart` mit Bodygrenze 9 MB auf genau dieser Route; Inhalt wird an den Magic Bytes erkannt (JPEG/PNG/WebP/GIF), der Client-Typ ignoriert; keine Bildverarbeitung (keine Dekodierung → keine Dekoder-Angriffsfläche), keine Vorschaubilder auf dem Server, die Vorschau skaliert der Browser (CSS). Auslieferung mit festem `Content-Type` aus der Erkennung, `X-Content-Type-Options: nosniff`, `Content-Security-Policy: default-src 'none'; sandbox`, `Cache-Control: private`. Dateiname wird nie im Pfad benutzt. SVG ist ausgeschlossen (Skripte). Alternative: serverseitig neu kodieren (Metadaten/EXIF entfernen) — wünschenswert für Standortdaten, aber braucht eine Bildbibliothek; als offene Frage vermerkt.

**Markdown mit `pulldown-cmark`.** HTML-Ereignisse werden zu Text umgewandelt (kein rohes HTML), Links nur mit Schema http/https/mailto, `rel="noopener noreferrer nofollow"`, Bilder in Markdown nicht gerendert (Bilder kommen nur als Anhänge), Tabellen, Aufgabenlisten, Durchstreichen an. Rendern bei der Anzeige; gespeichert wird der Quelltext. Vorschau als eigener POST, der dieselbe Seite mit Ergebnis liefert (kein JavaScript nötig).

**Rechte.** Neue `Capability`-Einträge nur dort nötig, wo es bisher keine gibt: Kommentieren/Bild hochladen für jedes aktive Mitglied; Beschreibung/Status für Zuständige und Amtsträger (bestehende Regel); Löschen fremder Kommentare/Bilder für den Vorstand. Rechtetest-Matrix wird ergänzt.

**Protokoll.** Neue Aktionen `task.status_changed` (vorhanden), `task.description_changed`, `task.comment_added/edited/deleted`, `task.image_added/removed`; Einträge nur mit Kennungen und Feldnamen. Die Aktionsliste samt Prüf-Test (`ACTIONS`) wird erweitert.

**Gleichzeitigkeit.** Aufgaben tragen eine Version wie andere Datensätze; Verschieben und Beschreibung nutzen sie. Kommentare und Bilder sind anfügend und brauchen nur Eigenversion für Änderung/Löschen.

## Risks / Trade-offs

- [Markdown als XSS-Weg] → HTML wird nie ausgegeben, Link-Schemas erlaubt-liste, CSP verbietet Inline-Skripte; Tests mit typischen Angriffsstrings (`<script>`, `javascript:`, `onerror`).
- [Upload als Angriffsweg und Speicherfresser] → Typ- und Größenprüfung, Anzahlgrenze, Bodygrenze nur an der Route, nur angemeldete Mitglieder, Auslieferung unter Sandbox-CSP; Datenbankgröße im README erklärt.
- [EXIF-Standortdaten in Fotos bleiben erhalten] → im README und am Upload-Feld hingewiesen; später optional serverseitig entfernen.
- [Drag-and-Drop mit Sonderfällen (Touch, Tastatur)] → Auswahl-Formular bleibt die Basis; JavaScript ist reine Ergänzung und wird mit Playwright geprüft.
- [Board bei vielen Aufgaben unübersichtlich] → Saisonarchiv, Filter „meine“/Boot, DONE nur laufende Saison.
- [Migration] → rein additiv; Rückbau durch Zurücksetzen des Commits, Tabellen bleiben ungenutzt.

## Migration Plan

Neue Migration `0004` läuft beim Start; bestehende Aufgaben unverändert. Backup-Hinweis im README (SQLite-Datei enthält nun Bilder).

## Open Questions

- Soll beim Hochladen EXIF/Standort entfernt werden? (Änderung bräuchte eine Bildbibliothek; kann später ergänzt werden, ohne Specs zu ändern.)

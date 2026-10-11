# Proposal

## Why

Aufgaben sind das Werkzeug, mit dem der Verein Arbeit verteilt (Segel nähen, Antifouling, Webseite), werden aber heute nur als flache Liste mit drei Zuständen gezeigt. Man sieht nicht auf einen Blick, was liegt, was läuft und was auf etwas anderes wartet; Absprachen und Fotos („so sieht der Schaden aus“) haben keinen Ort und landen in Chats; die Beschreibung lässt sich nicht nachträglich verbessern. Mit einem Kanban-Board, Kommentaren, Bildern und einer bearbeitbaren Markdown-Beschreibung wird die Aufgabe zum gemeinsamen Arbeitsplatz.

## What Changes

- **Aufgaben im Menü auf oberster Ebene** (Menüpunkt kommt mit `add-ui-fuehrung`; hier das Ziel dahinter).
- **Kanban-Board** mit den Spalten TODO, WIP, WAIT und DONE. Neuer Status „wartet“ (WAIT) für Aufgaben, die auf etwas oder jemanden warten. Karten zeigen Titel, Boot, Zuständige, Fälligkeit und Zähler für Kommentare und Bilder. Verschieben per Schaltfläche ohne JavaScript, zusätzlich per Ziehen mit JavaScript.
- **Saisonarchiv**: Erledigte Aufgaben einer vergangenen Saison verschwinden vom Board und stehen nur noch im Archiv (siehe `add-ui-fuehrung`). Erledigte der laufenden Saison bleiben in DONE.
- **Aufgabenseite**: Beschreibung, Zuständige, Fälligkeit, Stunden, Kommentare, Bilder und Verlauf an einem Ort.
- **Markdown-Beschreibung** bearbeitbar, mit Vorschau; Darstellung ohne rohes HTML.
- **Kommentare** durch alle aktiven Mitglieder, mit Autor und Zeitpunkt, in der Reihenfolge des Eingangs; eigener Kommentar kurzzeitig korrigierbar.
- **Bilder** zu einer Aufgabe hochladen (JPEG, PNG, WebP, GIF), mit Größen- und Anzahlgrenze, nur für angemeldete aktive Mitglieder abrufbar.
- Neue Protokollaktionen für Kommentieren, Bild hinzufügen/entfernen und Beschreibung ändern.

## Capabilities

### New Capabilities

- `aufgabenboard`: Board mit vier Spalten, Verschieben, Saisonarchiv, Aufgabenseite, Markdown-Beschreibung, Kommentare und Bilder im Web.

### Modified Capabilities

- `aufgaben`: neuer Status „wartet“ neben offen, in Arbeit und erledigt; Reihenfolge der Zustände; Anhänge (Kommentare, Bilder) als Teil der Aufgabe.
- `arbeitsverwaltung`: Anforderung „Aufgaben“ verweist auf das Board; Mitglieder lesen Aufgaben, kommentieren und laden Bilder hoch; Statuswechsel nur durch Zuständige und Amtsträger bleibt.

## Impact

- `crates/vereinsakte-core`: `TaskStatus::Waiting`, `TaskComment`, `TaskImage` (Metadaten).
- `crates/vereinsakte-storage`: Migration `0004` (Tabellen für Kommentare und Bilder, Bilddaten als BLOB), Status-Wert „waiting“.
- `crates/vereinsakte-app`: Use Cases für Board, Kommentar, Bild, Beschreibung, Archiv; Audit-Aktionen; Rechteprüfung.
- `crates/vereinsakte-web`: Board- und Aufgabenseiten, Mehrteil-Upload (axum `multipart`), Bildauslieferung, Markdown-Rendern; eine kleine statische Datei `board.js` für Ziehen (optional).
- Neue Abhängigkeiten: Markdown-Renderer (`pulldown-cmark`), axum-Feature `multipart`.
- Backup/Export umfasst nun Bilddaten (Datenbankgröße wächst; README ergänzen).
- Reihenfolge: **3. von 3**. Voraussetzung: `add-ui-fuehrung` (Navigation, Seitenkopf, Archiv, Panels); Migration folgt auf `0003` aus `add-kontakt-sichtbarkeit`.

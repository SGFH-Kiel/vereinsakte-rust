# Design

## Context

`add-aufgabenboard` führt Bild-Upload ein: Mehrteil-Formular, Erkennung des Formats an den Magic Bytes, Größengrenzen, Auslieferung mit festem Typ, `nosniff` und Sandbox-CSP, Speicherung als BLOB. Die Bootsverwaltung kennt bereits eine Bearbeitungsberechtigung je Boot (`can_edit` auf der Bootsseite: Vorstand, Oberbootswart, Bootswart dieses Bootes). `add-ui-fuehrung` stellt die Bootsliste auf Karten um. Siehe proposal.md.

## Goals / Non-Goals

**Goals:** ein Bild je Boot, gleiche Sicherheit wie Aufgabenbilder, keine Doppelimplementierung.

**Non-Goals:** keine Galerie, keine Bildbearbeitung (Zuschneiden, Skalieren) auf dem Server, keine Bilder für Abnahmen oder Sperren.

## Decisions

**Gemeinsames Bildmodul.** Erkennung, Grenzen und Antwort-Header liegen als Funktionen im Anwendungs- bzw. Webcrate und werden von Aufgabenbildern und Bootsbild genutzt; `add-aufgabenboard` legt sie dort an (Aufgabe 3.5), diese Änderung ruft sie nur auf.

**Eigene Tabelle `boat_images(boat_id PK FK, content_type, alt, size, data BLOB, updated_at)`.** Ein Bild je Boot per Primärschlüssel, Ersetzen = Upsert. Die Bootsliste lädt nie die Bilddaten, nur ob eines existiert (Spalte/Join auf `size`); das Bild kommt über `/boote/{id}/bild?v={updated_at}` (Cache-Busting, `Cache-Control: private, max-age` mit Version). Alternative: Spalte am Boot — würde jede Bootsabfrage schwer machen.

**Rechte.** Wiederverwendung der Bearbeitungsfähigkeit der Bootsverwaltung; das Bildfeld erscheint im Bootsformular und als Schaltfläche „Bild ändern“ am Bild der Bootsseite (Menü „Weitere Aktionen“ aus `add-ui-fuehrung`).

**Darstellung.** Karten der Bootsliste zeigen das Bild im Seitenverhältnis 4:3 mit `object-fit: cover` und `loading="lazy"` samt Alternativtext (Standard: „Foto der <Boot>“); Platzhalter mit Bootstyp-Symbol aus `add-ui-fuehrung`. Bootsseite: größeres Bild im Kopf.

**Protokoll.** Aktionen `boat.image_set` und `boat.image_removed` mit Boot-Kennung.

## Risks / Trade-offs

- [Große Fotos verlangsamen die Liste] → bis 8 MB zulässig, aber lazy loading; Hinweis am Formular, kleine Bilder zu wählen. Serverseitiges Verkleinern bleibt ein späterer Schritt (bräuchte Bildbibliothek).
- [Datenbank wächst] → ein Bild je Boot, überschaubar; Backup-Hinweis steht schon im README (aufgabenboard).
- [EXIF-Standort in Fotos] → Hinweis am Upload-Feld; wie bei Aufgabenbildern eine offene Frage.

## Migration Plan

Additive Migration `0005`; Boote ohne Bild zeigen den Platzhalter. Rückbau durch Zurücksetzen.

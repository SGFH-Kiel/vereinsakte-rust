# Proposal

## Why

Die Bootsliste besteht aus Namen und Zuständen; wer ein Boot nicht kennt (neue Mitglieder, Gäste der Crew), erkennt es nicht wieder. Ein Foto je Boot macht die Übersicht auf einen Blick lesbar und passt zu den neuen Karten der Bootsliste aus `add-ui-fuehrung`.

## What Changes

- Jedes Boot kann **ein Bild** haben, das in der Bootsübersicht (Liste/Karten), auf der Bootsseite und beim Boot im Kalender- bzw. Reservierungsdetail als Vorschau erscheint. Ohne Bild erscheint ein Platzhalter mit dem Bootstyp-Symbol.
- Wer das Boot **bearbeiten darf** (nach den bestehenden Rechten der Bootsverwaltung), kann das Bild hochladen, ersetzen und entfernen, jeweils mit optionalem Alternativtext.
- Die Prüfung entspricht den Aufgabenbildern aus `add-aufgabenboard` (JPEG, PNG, WebP, GIF; Inhalt statt Dateiname; Größengrenze; gesicherte Auslieferung nur an angemeldete Mitglieder).
- Hoch-, Ersetzen und Entfernen steht im Änderungsprotokoll (ohne Bilddaten).

## Capabilities

### New Capabilities

(keine)

### Modified Capabilities

- `bootsverwaltung`: neue Anforderungen für Bootsbild (Anzeige mit Platzhalter, Pflege durch Berechtigte, Prüfung und Auslieferung); die Bootsübersicht zeigt das Bild.

## Impact

- `crates/vereinsakte-core`: Boot-Bildmetadaten (Typ, Alternativtext); Bild liegt wie bei Aufgaben als BLOB im Speicher.
- `crates/vereinsakte-storage`: Migration `0005` (Tabelle `boat_images`, ein Bild je Boot) – Tabellen/Prüfung des Bildformats gemeinsam mit dem Aufgabenbild.
- `crates/vereinsakte-app`: Use Cases Bild setzen/entfernen mit vorhandener Bootsberechtigung; gemeinsames Bildmodul (Erkennung, Grenzen) aus `add-aufgabenboard`.
- `crates/vereinsakte-web`: Bootsliste, Bootsseite, Bootsformular (Bildfeld), Auslieferungsroute `/boote/{id}/bild`.
- Keine neuen Abhängigkeiten (Upload kommt mit `add-aufgabenboard`).
- Reihenfolge: **4. von 4**, nach `add-aufgabenboard` (wiederverwendet dessen Upload-Prüfung und Auslieferung) und nach `add-ui-fuehrung` (Karten der Bootsliste).

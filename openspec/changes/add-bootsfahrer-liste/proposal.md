# Proposal

## Why

Wer eine Ausfahrt plant, sucht Crew und Skipper: „Wer darf die Dehler überhaupt fahren?“ Heute zeigt die Bootsseite nur die eigene Eignung. Wer wen fragen kann, erfährt man über Umwege oder gar nicht. Eine Liste der Mitglieder, die ein Boot führen dürfen, macht die Crewsuche direkt am Boot möglich.

## What Changes

- Jede Bootsseite erhält einen Bereich **„Wer darf dieses Boot fahren?“** mit der Liste der aktiven Mitglieder, die das Boot als **Skipper** führen dürfen, und einer zweiten Liste der als **Co-Skipper** geeigneten Mitglieder je Fahrtgebiet.
- Die Liste zeigt Name (Link zum Profil), Mitgliedsart- und Scheinsymbole sowie Kontaktdaten (oder „Kontaktdaten gesperrt“) und ist sortierbar.
- Maßstab ist die **Qualifikation**: Bootserlaubnis für dieses Boot (nicht ruhend) und erfüllte Skipper-Anforderungen. Die gültige Segelerlaubnis der Saison wird **nicht** ausgewertet oder angezeigt, damit die Liste nicht verrät, wer Beiträge oder Arbeitsstunden schuldet; ein Hinweis nennt, dass sie zusätzlich nötig ist.
- Lesbar für alle aktiven Mitglieder.

## Capabilities

### New Capabilities

(keine)

### Modified Capabilities

- `bootsverwaltung`: neue Anforderungen für die Liste der Skipper und Co-Skipper eines Bootes.

## Impact

- `crates/vereinsakte-app`: Abfrage „geeignete Mitglieder je Boot“ (Skipper / Co-Skipper je Fahrtgebiet) auf Basis der vorhandenen Eignungsregeln ohne Segelerlaubnis; Sicht-Typ mit Kontaktdaten nach `add-kontakt-sichtbarkeit`.
- `crates/vereinsakte-web`: Bereich auf der Bootsseite (oder eigene Unterseite `/boote/{id}/fahrer`), Sortierung, Personenlinks.
- Keine Datenbank-, Migrations- oder Abhängigkeitsänderung.
- Reihenfolge: **5. von 5**, nach `add-kontakt-sichtbarkeit` (Kontaktdaten sichtbar, Sperrvermerk) und `add-ui-fuehrung` (Personenlinks, Sortierung, Symbole); unabhängig von `add-aufgabenboard` und `add-bootsbild`.

# Proposal

## Why

Wer eine Ausfahrt plant, sucht Crew: „Wer darf die Dehler überhaupt fahren?“ Heute zeigt die Bootsseite nur die eigene Eignung. Eine Liste der Mitglieder mit Bootserlaubnis für dieses Boot macht die Crewsuche direkt am Boot möglich.

## What Changes

- Jede Bootsseite erhält einen Bereich **„Wer darf dieses Boot fahren?“** mit allen aktiven Mitgliedern, denen die **Bootserlaubnis für dieses Boot erteilt** (Abnahme bestanden) und **nicht widerrufen** wurde.
- Mehr wird bewusst nicht geprüft: keine Skipper- oder Co-Skipper-Anforderungen, keine Scheine, keine Segelerlaubnis, keine Meldeadresse. Die Liste beantwortet allein „wer hat die Bootserlaubnis“.
- Die Liste zeigt Name (Link zum Profil), Mitgliedsart- und Scheinsymbole sowie Kontaktdaten (oder „Kontaktdaten gesperrt“) und ist sortierbar.
- Lesbar für alle aktiven Mitglieder.

## Capabilities

### New Capabilities

(keine)

### Modified Capabilities

- `bootsverwaltung`: neue Anforderungen für die Liste der Mitglieder mit Bootserlaubnis.

## Impact

- `crates/vereinsakte-app`: Abfrage „aktive Mitglieder mit erteilter, nicht widerrufener Bootserlaubnis je Boot“ aus den vorhandenen Abnahmen und Widerrufen; Sicht-Typ mit Kontaktdaten nach `add-kontakt-sichtbarkeit`.
- `crates/vereinsakte-web`: Bereich auf der Bootsseite, Sortierung, Personenlinks.
- Keine Datenbank-, Migrations- oder Abhängigkeitsänderung.
- Reihenfolge: **5. von 5**, nach `add-kontakt-sichtbarkeit` (Kontaktdaten, Sperrvermerk) und `add-ui-fuehrung` (Personenlinks, Sortierung, Symbole); unabhängig von `add-aufgabenboard` und `add-bootsbild`.

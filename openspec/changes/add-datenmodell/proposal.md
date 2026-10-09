# Proposal

## Why

Die Vereinsakte soll die Verwaltung der Segelgruppe an der Fachhochschule Kiel e.V. abbilden: Mitglieder, ihre Scheine und Bootserlaubnisse, die jährliche Segelerlaubnis (Beitrag + Arbeitsstunden), Bootsreservierungen, gemeinsame Arbeitstermine und Einzelaufgaben sowie die Forderungen des Vereins (Beiträge, Aufnahme-, Tour- und Abgeltungsgebühren) und deren Bezahlung. Das Projekt ist derzeit leer (`src/main.rs` gibt nur „Hello, world!“ aus). Ein gemeinsames, serialisierbares Datenmodell, das die Regeln aus Satzung (04.04.2019), Beitragsordnung und Segelordnung (14.04.2026) prüfbar macht, ist die Grundlage für alle weiteren Funktionen.

## What Changes

- Neues Modul `model` mit Rust-Structs und -Enums sowie einem Wurzelobjekt „Vereinsakte“, das alle Daten des Vereins als ein JSON-Dokument hält.
- **Mitglieder** mit Name, Mitgliedsnummer (zugleich Kennung des Mitglieds), Anschrift, Telefonnummern, E-Mail/Meldeadresse, Mitgliedsart (ordentlich, studentisch, außerordentlich, Ehrenmitglied, Kind eines Mitglieds, Erasmus) und Mitgliedschaftsverlauf (Eintritt, vorläufige Aufnahme, Austritt, Ausschluss, Befristung).
- **Segelscheine** (SBF See, SBF Binnen, SKS, SSS, SHS, BR, Sonstige) und **Funklizenzen** (SRC, LRC, UBI, ROC, Sonstige).
- **Boote** mit Typ (Jolle/Kielboot), Klasse, technischen Daten, Skipper-Anforderungen, Besatzungsregeln je Fahrtgebiet (inkl. Co-Skipper-Anforderungen), Saisonfreigabe und Sperre.
- **Bootserlaubnis** je Mitglied und Boot über Abnahmen (bestanden/nicht bestanden); gilt bis auf Widerruf und ruht ohne gültige Meldeadresse.
- **Arbeitsstunden** als Liste von Einsätzen, wahlweise (Datum, Stunden) oder (Start, Ende), mit Beschreibung und Bestätigung; Stundenübertrag zwischen Mitgliedern und finanzielle Abgeltung.
- **Segelerlaubnis** je Jahr aus Saison-Richtsatz, individuell reduziertem Soll, Beitragsstatus und Mitgliedsart.
- **Reservierungen** mit Skipper, optionalem Co-Skipper, Crew und Gästen (jeweils optional mit eigenem Zeitraum an Bord), Fahrtgebiet und Tour-Kennzeichen; Prüfung von Segelerlaubnis, Bootserlaubnis, Scheinen, Mindestbesatzung und Bootsfreigabe.
- **Mitgliedskonto**: je Mitglied eine Liste der Forderungen und eine Liste der Einzahlungen; daraus berechnet: Kontostand, bezahlte/offene/überfällige Forderungen und eine Übersicht der Außenstände.
- **Beiträge**: Jahresbeitrag (anteilig im Eintrittsjahr, fällig 1. April) und Aufnahmegebühr nach Mitgliedsart als Forderungen.
- **Tourgebühren** je Person und Tour nach Tarifgruppe (Tages-/Wochensätze), als Forderungen; Übersicht, wer bezahlt hat.
- Die Segelerlaubnis prüft Beitragsrückstände anhand des Mitgliedskontos.
- **Ämter** nach Satzung (Vorstand mit 1./2. Vorsitzendem, Kassenwart, Schriftwart, Oberbootswart; Revisoren; Bootswarte je Boot) und Abnehmer je Boot, mit Amtszeiten, Satzungsprüfung und Rechten: Bootswarte, Oberbootswart und Vorstand sperren/geben Boote frei und bestätigen Arbeitsstunden; nur bestätigte Stunden zählen.
- **Arbeitstermine** (gemeinsame, angekündigte Termine) und **Aufgaben** (Einzelaufgaben, die Mitglieder allein erledigen).
- Alle Typen sind per `serde` als JSON serialisierbar; Zeitpunkte als RFC3339 mit Uhrzeit und Zeitzone.
- Geldbeträge exakt als Dezimalzahl (`rust_decimal`), im JSON als Zeichenkette wie `"240.00"`.
- Neue Abhängigkeiten: `serde`, `serde_json` (Tests), `chrono`, `uuid`, `rust_decimal`.

Nicht im Umfang: Persistenz (Datei/Datenbank), Benutzeroberfläche/CLI, Benutzerverwaltung, doppelte Buchführung, Vereinskonten und Ausgaben, Jahresabschluss, SEPA-Lastschriftdateien, Mahnwesen, Mitgliederversammlung und Wahlen, Rechte für Forderungen und Einzahlungen, Logbuch, Schadensmeldungen, Schwimmzeugnis und Einverständniserklärungen für Gäste/Minderjährige, Segelverbote als Sanktion, Vorrang von Mitgliedern vor Gästen sowie die automatische Konfliktauflösung im Reservierungskalender (das Modell erkennt nur Überschneidungen zweier Reservierungen).

## Capabilities

### New Capabilities
- `datenformat`: JSON-Konventionen (IDs, Zeitpunkte als RFC3339 mit Zeitzone, Feldbenennung) und das Wurzelobjekt Vereinsakte.
- `mitglieder`: Mitgliederstammdaten, Mitgliedsart, Mitgliedschaftsverlauf, Meldeadresse, Segelscheine und Funklizenzen.
- `boote`: Boote mit Typ, technischen Daten, Skipper-Anforderungen, Besatzungsregeln, Saisonfreigabe und Sperre.
- `bootserlaubnis`: Abnahme, Widerruf und Ruhen der Bootserlaubnis sowie die Eignung als Skipper oder Co-Skipper.
- `arbeitsstunden`: Arbeitseinsätze, Stundenübertrag, finanzielle Abgeltung und die Ist-Stunden eines Jahres.
- `segelerlaubnis`: Saison-Richtsatz, Jahresnachweis (reduziertes Soll, Abgeltung), Beitragsrückstand und die daraus abgeleitete jährliche Segelerlaubnis.
- `bootsreservierung`: Reservierungen mit Skipper, Co-Skipper, Crew, Gästen und Fahrtgebiet sowie deren Prüfung.
- `arbeitstermine`: Kalender gemeinsamer Arbeitstermine mit Anmeldung.
- `aufgaben`: Einzelaufgaben, die Mitglieder allein erledigen.
- `mitgliedskonto`: Forderungen und Einzahlungen je Mitglied, Kontostand, offene und überfällige Forderungen, Außenstände.
- `beitraege`: Gebührensätze je Saison, Jahresbeitrag und Aufnahmegebühr als Forderungen.
- `tourgebuehren`: Berechnung der Tourgebühren je Person und Anlage als Forderungen.
- `aemter`: Ämter mit Amtszeiten, Satzungsregeln für den Vorstand und die daraus folgenden Rechte (Sperren, Freigeben, Stunden bestätigen, Abnahme, Widerruf).

### Modified Capabilities
- keine

## Impact

- Code: neues Modul `src/model/` mit Untermodulen, `src/main.rs` bindet es ein.
- Abhängigkeiten in `Cargo.toml`: `serde` (derive), `chrono` (Feature `serde`), `uuid` (Features `v4`, `serde`), `rust_decimal`; `serde_json` und `rust_decimal_macros` als dev-dependencies.
- Keine bestehenden APIs oder Specs betroffen (Greenfield).

# Tasks

## 1. Kern und Speicher

- [x] 1.1 `Member.contact_blocked` (Standard `false`, `serde(default)`) und `contact_visible(viewer, subject)` im Kern bzw. in der Anwendungsschicht; Unit-Tests: selbst, Vorstand, Amtsträger ohne Vorstand, normales Mitglied, mit und ohne Sperrvermerk; `cargo test -p vereinsakte-core -p vereinsakte-app` grün.
- [x] 1.2 Migration `0003_contact_blocked.sql` mit `contact_blocked`, Lesen/Schreiben, alte Akten ohne Merkmal lesbar; Test: Round-trip, Import ohne Merkmal ergibt `false`.

## 2. Anwendungsfälle

- [x] 2.1 Verzeichnis, Mitgliedsseite und Ämterübersicht liefern `Contact::{Visible, Blocked}`; Mitgliedsnummer, Art, Scheine, Funklizenzen für alle sichtbar, Anschrift/Konto/Stunden wie bisher eingeschränkt; Tests: Greta sieht Kontaktdaten von Clara, bei Sperrvermerk nur „gesperrt“, Eva und der Betroffene sehen sie, Ben (Bootswart) nicht.
- [x] 2.2 Use Case „Sperrvermerk setzen/entfernen“ nur für das eigene Mitglied (sonst 403), Protokollaktion `member.contact_block_changed` ohne Werte, `ACTIONS`-Sweep-Test grün.

## 3. Web

- [x] 3.1 Vorlagen: Liste, Mitgliedsseite, Ämter zeigen E-Mail und Telefon als Links (`mailto:`, `tel:`) oder „Kontaktdaten gesperrt“ mit Symbol und Text; alte Hinweiszeile „Kontaktdaten sind nur für Mitglieder mit einem Amt sichtbar“ entfällt; Tests: gesperrte Werte kommen im Seitentext und in der htmx-Teilantwort nicht vor.
- [x] 3.2 „Meine Daten“: Schalter „Kontaktdaten nicht anzeigen“ mit Erklärung (wer sie sonst sieht, Vorstand weiterhin); Test: Anna setzt den Schalter, Greta sieht „gesperrt“, Protokoll nennt „Sperrvermerk“; erzwungener POST für fremdes Mitglied → 403.
- [x] 3.3 Einmaliger Hinweis auf der Startseite zur geänderten Sichtbarkeit mit Link zum Schalter, schließbar; Test: erscheint genau einmal je Mitglied bis zum Schließen.

## 4. Abschluss

- [x] 4.1 Bestehende Tests der alten Regel anpassen, End-to-End-Test (alle Seiten × alle Mitglieder) und Screenshots aktualisieren; `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` grün.
- [x] 4.2 README: Sichtbarkeit der Kontaktdaten, Sperrvermerk, Hinweis zur Information der Mitglieder; Befehle laufen wie beschrieben.

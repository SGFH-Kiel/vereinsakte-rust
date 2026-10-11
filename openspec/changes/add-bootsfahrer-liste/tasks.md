# Tasks

## 1. Kern und Anwendung

- [ ] 1.1 Kern: `qualified_as_skipper` und `qualified_as_co_skipper` (ohne Segelerlaubnis); Gesamteignung ruft sie auf und ergänzt nur die Segelerlaubnis; Unit-Tests: Bootserlaubnis fehlt, ruhende Erlaubnis (ungültige Meldeadresse), fehlende SRC bei der Dehler, Co-Skipper außerhalb, Mitglied ohne Segelerlaubnis ist qualifiziert, bestehende Eignungstests bleiben grün (`cargo test -p vereinsakte-core`).
- [ ] 1.2 Anwendung: Abfrage `boat_crew_candidates(boat, principal)` für Skipper und Co-Skipper je Fahrtgebiet mit Person, Kontakt (nach Sichtbarkeitsregel), Scheinen, Mitgliedsart; nur aktive Mitglieder; Tests: Filou-Skipper, Dehler ohne Clara, gesperrter Kontakt als `Blocked`, kein Segelerlaubnis-Feld im Ergebnistyp.

## 2. Web

- [ ] 2.1 Bereich „Wer darf dieses Boot fahren?“ auf der Bootsseite: Skipperliste offen, Co-Skipper je Fahrtgebiet eingeklappt, Hinweis zur Segelerlaubnis, Personenlinks, Symbole, sortierbare Köpfe, leere Zustände; Tests: Greta sieht die Skipper der Filou mit Kontaktdaten, bei Sperrvermerk nur „gesperrt“ ohne die Werte im Text, Hinweis vorhanden, Heimrevier ohne Anforderung zeigt „kein zusätzlicher Nachweis nötig“.
- [ ] 2.2 Handy- und Darstellungsprüfung: Karten statt Tabelle bei 375 px ohne seitliches Scrollen; Screenshot-Skript um die Bootsseite mit Liste erweitern.

## 3. Abschluss

- [ ] 3.1 End-to-End-Test (alle Seiten × alle Mitglieder) und `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` grün; README ergänzen (Liste, Hinweis zur Segelerlaubnis) und die beschriebenen Schritte laufen wie dokumentiert.

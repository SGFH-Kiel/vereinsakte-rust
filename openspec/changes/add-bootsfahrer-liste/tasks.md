# Tasks

## 1. Anwendung

- [x] 1.1 Abfrage `boat_permit_holders(boat, principal)`: aktive Mitglieder mit bestandener Abnahme und ohne späteren Widerruf für dieses Boot, mit Person, Kontakt (nach Sichtbarkeitsregel), Scheinen, Mitgliedsart; Tests: bestandene Abnahme → drin, nicht bestanden → nicht, Widerruf → nicht, Widerruf und spätere neue Abnahme → drin, fehlender Funkschein bei der Dehler → trotzdem drin, Ausgeschiedene → nicht, gesperrter Kontakt als `Blocked`; `cargo test -p vereinsakte-app` grün.

## 2. Web

- [x] 2.1 Abschnitt „Wer darf dieses Boot fahren?“ auf der Bootsseite: sortierbare Tabelle, Personenlinks, Symbole, Kontaktdarstellung, Anzahl in der Überschrift, leerer Zustand; Tests: Greta sieht David bei der Filou mit Kontaktdaten, bei Sperrvermerk nur „gesperrt“ ohne die Werte im Seitentext, widerrufene und nicht bestandene fehlen.
- [x] 2.2 Darstellung bei 375 px als Karten ohne seitliches Scrollen; Screenshot-Skript um die Bootsseite mit Liste erweitern.

## 3. Abschluss

- [x] 3.1 End-to-End-Test (alle Seiten × alle Mitglieder) und `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` grün; README ergänzen (Liste, Maßstab: erteilt und nicht widerrufen) und die beschriebenen Schritte laufen wie dokumentiert.

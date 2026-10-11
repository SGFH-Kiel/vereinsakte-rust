# Tasks

## 1. Speicher und Anwendung

- [ ] 1.1 Migration `0005_boat_images.sql` und Repository-Funktionen (Bild setzen/ersetzen/entfernen/lesen, „hat Bild“ für Listen ohne Bilddaten); Test: Upsert ersetzt, Listenabfrage lädt keine BLOBs, Kaskade beim Löschen des Boots.
- [ ] 1.2 Use Cases mit der Bearbeitungsberechtigung des Bootes und dem gemeinsamen Bildmodul (Format, 8 MB); Tests: Bootswart des eigenen Boots darf, Bootswart eines anderen Boots und Greta nicht (403), SVG und Textdatei als JPEG abgelehnt, altes Bild bleibt bei Ablehnung.
- [ ] 1.3 Protokollaktionen `boat.image_set` und `boat.image_removed` ohne Bilddaten, `ACTIONS`-Sweep-Test grün.

## 2. Web

- [ ] 2.1 Route `/boote/{id}/bild` mit gesicherter Auslieferung (nur angemeldet, fester Typ, `nosniff`, Sandbox-CSP, versionierter Cache); Test: ohne Anmeldung Umleitung, Header korrekt, unbekanntes Boot 404.
- [ ] 2.2 Bootsliste (Karten) und Bootsseite zeigen Bild oder Platzhalter mit Bootstyp-Symbol, `loading="lazy"`, Alternativtext; Tests: Filou mit Bild zeigt `<img>`, Trigon ohne Bild den Platzhalter; Screenshots 375/1280 px ohne seitliches Scrollen.
- [ ] 2.3 Bild-Upload im Bootsformular und „Bild ändern/entfernen“ auf der Bootsseite nur für Berechtigte, Fehlermeldungen am Feld; Tests: Ben lädt für die Harald Blauzahn hoch und sieht das Bild, Ersetzen lässt ein Bild, zu große und falsche Dateien werden mit Meldung abgelehnt.

## 3. Abschluss

- [ ] 3.1 Beispieldaten um ein kleines Bild für eine Jolle ergänzen, End-to-End-Test (alle Seiten × alle Mitglieder) und Screenshots aktualisieren; `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` grün.
- [ ] 3.2 README ergänzen (Bootsbild, Grenzen, Berechtigte); die beschriebenen Befehle laufen wie dokumentiert.

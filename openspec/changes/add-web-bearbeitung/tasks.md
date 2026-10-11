# Tasks

Konvention: Code englisch, Doku/Texte deutsch. Tests bauen auf der Beispielakte auf (Feature `example` des Core, nur in `[dev-dependencies]`) und nutzen eine feste Uhr (2026-10-09 in Vereinszeit). Jede Gruppe endet mit `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings` und `cargo fmt --check` ohne Befunde. Schreibende Routen prüfen immer: erlaubt → Änderung und Protokolleintrag; verboten → 403, keine Änderung, kein Eintrag; veralteter Stand → 409.

## 1. Rechte im Core

- [x] 1.1 `Capability` und `Authorizer` im Core einführen (Basis: `Offices` plus Aktor und Zusatzdaten), die Logik aus `ClubRecord::authorize` dorthin verschieben und `ClubRecord::authorize` delegieren lassen; verifizieren: alle bestehenden Core-Tests unverändert grün
- [x] 1.2 Rechtematrix aus design.md als `Capability`-Regeln umsetzen (eigene Kontaktdaten/Scheine/Lizenzen, Mitgliederverwaltung, Ämter, Boote, Arbeit, Reservierungen, Finanzen, Protokoll); Tests: Tabelle Mitglied der Beispielakte (Anna, Ben, Clara, Eva, Frank, Greta, Hannes) × Capability, darunter Greta darf eigene, aber nicht Claras Scheine ändern, Ben darf Filou nicht sperren, Greta nichts Schreibendes, Revisor nur lesend, Amtswechsel wirkt sofort

## 2. Schreib-Ports, Protokoll und Teildaten-Regeln

- [x] 2.1 Migration `0002` mit `audit_log` (Indizes auf `at`, `actor_id`, `subject_member_id`; Trigger gegen `UPDATE`/`DELETE`) und `AuditRepository` (anhängen in der Transaktion des Befehls, filtern, seitenweise lesen); Tests: Eintrag wird mit der Änderung gespeichert, bei Rollback fehlt er, Änderung/Löschen schlägt fehl, Filter nach Zeitraum, Akteur, Art
- [x] 2.2 Schreibenden Port `ChangeRepository::commit` (Änderungen `Save`/`Delete` über alle zwölf Fachobjekte mit Versionsprüfung, plus Protokolleinträge in derselben Transaktion) in `vereinsakte-app` definieren, In-Memory-Implementierungen und SQLite-Implementierungen in `vereinsakte-storage` bereitstellen; Tests je Port: speichern, veraltete Version → `Conflict` und unverändert, Zeilen anderer Objekte (`rowid`) unberührt
- [x] 2.3 Mehrobjekt-Regeln (Reservierungsprüfung, Beitrags- und Tourgebührenberechnung) auf Teildaten stützen: `ScopeRepository` liefert einen `ClubRecord` mit nur den für den Fall nötigen Objekten (Beteiligte samt Konto, Nachweisen, Stunden, Abnahmen; Boot; Saisons; Reservierungen im Zeitraum), die Core-Regeln laufen unverändert darauf; Tests: für alle Beispiel-Reservierungen liefert die Prüfung auf den Teildaten dasselbe wie auf der ganzen Akte, ebenso Jahresbeiträge, Aufnahmegebühren und Tourgebühren; SQLite- und In-Memory-Teildaten sind gleich und enthalten nur die Beteiligten

## 3. Designsystem und Grundgerüst

- [x] 3.1 `app.css` mit Tokens (hell/dunkel), Basis, Komponenten (Karte, Datenliste, Tabelle mit Kartenansicht, Formularraster, Schaltflächen, Plakette, Meldung, Filterleiste, Seitenwechsel, Unternavigation) und Druckregeln; verifizieren: Bildschirmfotos bei 375 px und 1280 px (hell und dunkel) mit `tools/screenshots.mjs` (Playwright, vorinstalliertes Chromium) zeigen kein horizontales Scrollen und lesbare Kontraste
- [x] 3.2 Basislayout überarbeiten: Navigation nach Berechtigung mit aktuellem Bereich, Menü hinter `<details>` auf schmalen Bildschirmen, Flash-Meldungen aus der Sitzung, Breadcrumbs, `lang`, Skip-Link; Tests: aktueller Bereich markiert, Menü-Element ohne JavaScript vorhanden, keine Inline-Skripte, -Stile oder externen Ressourcen im HTML (CSP), Sitzung eines ausgeschiedenen Mitglieds endet; die Sichtbarkeit von „Finanzen“ und „Protokoll“ nach Berechtigung wird in 9.1 und 10.1 geprüft, sobald diese Seiten bestehen
- [x] 3.3 Formular-Infrastruktur in `vereinsakte-web`: Eingabetypen mit deutschen Zahl-, Betrags- und Datumsformaten, `FieldErrors`, Formularfragmente für Feld/Fehler/Hinweis, 422-Wiedergabe mit erhaltenen Eingaben, 403- und 409-Seiten, Post/Redirect/Get mit Flash; Tests: „12,50“ und „12.50“ ergeben 12,50 €, ungültiges Datum erzeugt Feldfehler, 409 erhält Eingaben
- [x] 3.4 Listen-Infrastruktur: Abfrageparameter-Typ (`q`, `status`, `jahr`, `seite`, `sort`), Seitenwechsel-Baustein, Teil-Template bei `HX-Request`; Test: gleiche Anfrage mit und ohne htmx liefert dieselben Zeilen, Seite 2 ab 26 Einträgen
- [x] 3.5 Startseite „Meine nächsten Schritte“ mit Segelerlaubnis, Arbeitsstunden und Konto; Tests: Clara sieht „es fehlen 4,5 Arbeitsstunden“, Anna sieht Guthaben und erteilte Segelerlaubnis ohne Mangelhinweis. Die Karten für anstehende Arbeitstermine, eigene Reservierungen sowie offene Bestätigungen und Aufgaben je Amt entstehen mit 7.3, 7.4, 7.2 und 8.1, sobald deren Daten lesbar sind

## 4. Mitglieder

- [x] 4.1 Mitgliedsseite `/mitglieder/{nr}` mit Sichtbarkeit nach Rolle (Name/Ämter für alle; Kontakt, Segelerlaubnis, Stunden, Konto nur eigen oder berechtigt) und erweitertes Verzeichnis (Filter Mitgliedsart, Schein „SKS“); Tests: Greta sieht bei Clara keine Kontaktdaten, Eva alles, Filter „SKS“ liefert Anna, Ben, Clara, Eva, Frank
- [x] 4.2 Eigene Kontaktdaten bearbeiten (`/meine-daten/bearbeiten`) mit Meldeadress-Prüfung und Versionsfeld; Tests: Anna ändert Straße, Greta ändert Mitgliedsart per manuellem POST bleibt unverändert, Konflikt → 409 mit erhaltenen Eingaben
- [x] 4.3 Mitglied aufnehmen und Stammdaten/Mitgliedsart/Verlauf ändern (Vorstand): Nummernvorschlag, Doppelprüfung Nummer und Anmeldekonto, Vorstandszustimmung außerordentlicher Mitglieder, Austritt/Ausschluss mit Datum; Tests: neue Nummer, doppelte Nummer/Anmeldekonto (Schreibweise egal) am Feld, Inès im Dezember nicht mehr im Verzeichnis und ohne Anmeldung, Greta → 403
- [x] 4.4 Eigene Scheine und Funklizenzen auf „Meine Daten“ erfassen, ändern, entfernen (ohne Nachweis, ohne Freigabe; Vorstand darf für alle) mit sofort neu berechneter Segelerlaubnis und Protokolleintrag; Tests: Frank ergänzt SKS und Segelerlaubnis und Filter „SKS“ aktualisieren sich, Formular enthält kein Upload- oder Nachweisfeld, Greta ändert Claras Schein → 403, Eva ergänzt Claras Funklizenz, Entfernen erzeugt Protokolleintrag „Segelschein entfernt“

## 5. Ämter

- [x] 5.1 Ämterübersicht und Amtshistorie (`/aemter`); Tests: Greta sieht Eva als 1. Vorsitzende und Ben als Bootswart der Harald Blauzahn ohne Kontaktdaten, Historie nur für den Vorstand
- [x] 5.2 Ernennen und Abgeben inklusive Prüfhinweisen der Spec `aemter` und Berechtigungen (Vorstand alle, Oberbootswart Bootswarte/Abnehmer, Mitglied eigenes Amt); Tests: Bootswartin Greta darf anschließend die Filou sperren, Ben darf kein Vorstandsamt vergeben (403), Ende vor Beginn → Meldung, Doppelbesetzung → Hinweis, Bens Abgabe wirkt auf seiner nächsten Anfrage

## 6. Boote, Sperren, Abnahmen

- [x] 6.1 Bootsliste und Bootsseite mit Status, Ämtern, Anforderungen, Besatzungsregeln und eigener Eignung samt Begründung; Tests: gesperrte Filou zeigt Grund und Zeitraum, Anna erhält Eignung oder fehlende Anforderung
- [x] 6.2 Boot anlegen, Stammdaten, Anforderungsgruppen und Besatzungsregeln pflegen (Oberbootswart, Bootswart des Bootes, Vorstand; Anlegen nur Oberbootswart/Vorstand); Tests: Trigon anlegen, ungültige Besatzungsregel → Feldfehler, fremder Bootswart → 403
- [x] 6.3 Sperren, Aufheben und Saisonfreigabe über die `aemter`-Berechtigung mit Urheber und Zeitpunkt; Tests: Ben sperrt Harald Blauzahn, Ben sperrt Filou → 403, Aufheben ohne offene Sperre → Meldung, Greta sieht keine Schaltfläche
- [x] 6.4 Abnahmen und Widerrufe erfassen mit sofortiger Neuberechnung der Bootserlaubnis; Tests: Abnehmer erfasst Gretas Abnahme der Filou, Eva widerruft mit Begründung, Bootswart-Widerruf → 403

## 7. Arbeit

- [x] 7.1 Arbeitseinsätze erfassen, eigene unbestätigte ändern/löschen, Stundenübersicht je Jahr; Tests: Clara erfasst 3 Stunden an der Filou, beide Umfangsformen → Feldfehler, Änderung eines bestätigten Einsatzes → 403, Änderung hebt Bestätigung auf
- [x] 7.2 Offene Einsätze gesammelt bestätigen nach Zuständigkeit (inkl. Karte „offene Bestätigungen“ auf der Startseite); Tests: Ben bestätigt Claras Einsatz an der Harald Blauzahn, eigener Einsatz → abgelehnt, Greta sieht die Liste nicht
- [x] 7.3 Arbeitstermine mit Plätzen und Anmeldung/Abmeldung (inkl. Karte „anstehende Termine“ auf der Startseite); Tests: Anna meldet sich an und ist in der Teilnehmerliste, ausgebucht → keine Anmeldung, Anmeldung schreibt keine Stunden gut
- [x] 7.4 Aufgaben anlegen, Zuständige zuweisen, Status ändern (inkl. Karte „meine Aufgaben“ auf der Startseite); Tests: Zuständiger setzt „erledigt“ mit Zeitpunkt, fremdes Mitglied ohne Amt → 403, Stunden der Aufgabe über Einsätze sichtbar
- [x] 7.5 Stundenüberträge und finanzielle Abgeltung (Vorstand) mit Forderung im Konto; Tests: 5 Stunden bei 15 € → Forderung 75,00 €, Übertrag an sich selbst → Feldfehler, Rückstandsliste zeigt Clara

## 8. Reservierungen

- [x] 8.1 Reservierungskalender (Woche/Monat) und Liste mit Filtern Boot, Zeitraum, „nur meine“ (inkl. Karte „meine Reservierungen“ auf der Startseite); Tests: Filter „nur meine“ für Anna, Kalender ohne JavaScript als Liste nach Tagen
- [x] 8.2 Reservierung anlegen mit Co-Skipper, Crew, Gästen, eigenem Zeitraum, Tour-Kennzeichen, Bemerkung und der Gesamtprüfung aus 2.3 mit verständlichen Fehlern; Tests: gültige Reservierung mit Crew und Gast, Skipper ohne Abnahme, Überschneidung nennt die kollidierende Reservierung, gesperrtes Boot, fehlende Segelerlaubnis eines Crewmitglieds
- [x] 8.3 Reservierung ändern und stornieren (Skipper künftig, Berechtigte jederzeit, begonnene nur Vorstand) mit erneuter Prüfung und Anzeige der erwarteten Tourgebühr; Tests: Anna storniert, Greta fremde Reservierung → 403, Änderung wird erneut geprüft, Tourgebühr je Person angezeigt

## 9. Finanzen

- [x] 9.1 Eigenes Konto (Kontostand, Forderungen mit Zustand, Einzahlungen) und fremde Konten für Kassenwart, Vorstand, Revisor; Tests: Anna „Guthaben“ „10,00 €“, Lukas offen 276,00 €, Greta auf fremdem Konto → 403, Revisor ohne Schaltflächen und POST → 403
- [x] 9.2 Forderungen und Einzahlungen buchen, Storno mit Pflichtbegründung; Tests: 240,00 € Einzahlung wirkt im Kontostand, negativer Betrag → Feldfehler, Schlüsselpfand-Storno bleibt mit Grund sichtbar und zählt nicht mehr, Löschen gibt es nicht
- [x] 9.3 Außenstände zum Stichtag mit Filter „überfällig“ und Sortierung; Test: Stichtag 20. Juli zeigt nur zu diesem Zeitpunkt überfällige Beträge, Adresse mit Filter ist verlinkbar
- [x] 9.4 Saisons und Gebührensätze pflegen; Jahresbeiträge gesammelt anlegen mit Vorschau ohne Doppelungen; Tests: Satz ändern lässt bestehende Forderungen unverändert, zweites Anlegen zeigt „bereits vorhanden“ und erzeugt keine Forderung
- [x] 9.5 Tourgebühren aus einer Tour-Reservierung berechnen, prüfen und anlegen (Mitglieder und Gastname), idempotent; Test: siebentägige Tour mit zwei Mitgliedern und einem Gast ergibt zwei plus eine Forderung, erneutes Anlegen ergibt keine Doppelung
- [x] 9.6 Jahresnachweis je Mitglied und Saison mit Druckansicht ohne Navigation; Test: Nachweis enthält Soll-/anrechenbare Stunden, Beitragsstand, Segelerlaubnis, Druckregel blendet Navigation aus

## 10. Protokoll

- [ ] 10.1 Protokollseite `/protokoll` mit Filtern Zeitraum, Akteur, Art, neueste zuerst; Mitglieder sehen nur Einträge, in denen sie handelnd oder betroffen sind; Tests: Revisor filtert „Einzahlung“, Greta sieht nur eigene Einträge, kein Eintrag enthält Feldwerte oder Sitzungsdaten
- [ ] 10.2 Sicherstellen, dass jeder schreibende Anwendungsfall aus den Gruppen 4–9 einen Eintrag schreibt (Sammeltest über eine Liste aller Befehle) und bei Fehler keinen; Test: für jeden Befehl Erfolg → genau ein Eintrag, 403/409/422 → keiner

## 11. Integration und Dokumentation

- [ ] 11.1 Ende-zu-Ende-Test im Binary-Crate über SQLite und Entwicklungs-Login: Greta erfasst einen Einsatz, Ben bestätigt ihn, Eva nimmt ein Mitglied auf, der Kassenwart bucht eine Einzahlung, der Revisor liest alles und scheitert beim Schreiben, Protokoll enthält alle Aktionen; verifizieren mit `cargo test --workspace`
- [ ] 11.2 Erscheinungsbild prüfen: `tools/screenshots.mjs` erzeugt Bildschirmfotos aller Hauptseiten bei 375 px und 1280 px (hell/dunkel) aus der Beispielakte; verifizieren: keine Seite scrollt seitwärts, Menü und Karten-Tabellen sind bedienbar, Befunde sind behoben
- [ ] 11.3 `README.md` um Rechte-Übersicht (Matrix), Bedienung je Rolle, Entwicklungsmodus mit Rollenwechsel zum Testen und Sicherung inkl. Protokoll ergänzen; verifizieren: die beschriebenen Schritte laufen auf einer leeren Kopie durch
- [ ] 11.4 Abschlussprüfung: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check` ohne Befunde; `cargo build --release -p vereinsakte` enthält keine Testdaten (Suche nach „Petersen“ im Binary ergibt nichts)

## Workflow follow-up

- Verbleibende Annahmen der Rechtematrix (Open Questions in design.md) vom Vorstand bestätigen lassen, bevor Gruppe 1 beginnt.
- Nach Abschluss mit `/opsx:archive` archivieren.

# Design

## Context

Stand nach `add-web-architektur`: Die Web-Schicht (axum, askama, htmx, tower-sessions, CSRF-Schutz) kennt nur die lesenden Seiten `/meine-daten` und `/mitglieder`. Die App-Schicht arbeitet über Ports mit gezielten Abfragen (`MemberRepository`, `OfficeRepository`, `BoatRepository`, `YearRepository`, `PermitRepository`); geschrieben wird bisher nur `save_member` mit Versionsprüfung. Die Speicherschicht kennt ein relationales SQLite-Schema, dessen Wurzeltabellen eine `version`-Spalte tragen. Die Fachregeln und die Prüfung von Handlungsrechten (`ClubRecord::authorize`, `block_boat`, `confirm_work`, …) liegen im Core, arbeiten aber auf der ganzen Akte. Die Content-Security-Policy erlaubt nur eigene Skripte und Stile (`'self'`), kein Inline-JavaScript.

Motivation und Umfang: siehe proposal.md. Die Anforderungen stehen in den Specs `rechte`, `oberflaeche`, `aenderungsprotokoll` und den fünf Verwaltungs-Specs.

## Goals / Non-Goals

**Goals:**
- Jede Berechtigung wird an genau einer Stelle entschieden und von Anzeige und Server gleich verwendet.
- Schreibende Anwendungsfälle laden und speichern nur die betroffenen Teile der Akte, jeweils in einer Transaktion samt Protokolleintrag.
- Ein kleines Designsystem (Tokens, Komponenten) trägt alle Seiten; neue Seiten sind überwiegend Zusammenstellung dieser Bausteine.
- Jede Funktion arbeitet ohne JavaScript; htmx beschleunigt nur.

**Non-Goals:**
- Kein Rollen-Editor und keine frei konfigurierbaren Rechte: Rechte folgen fest den Ämtern der Satzung und der Segelordnung.
- Keine Benachrichtigungen (E-Mail, Push), kein Dateiupload (auch keine Nachweise zu Scheinen), keine Zahlungsanbindung.
- Kein Einzelseiten-Frontend (SPA), kein CSS-Framework, kein Build-Schritt für Frontend-Dateien.
- Keine Änderung der Fachregeln, der JSON-Struktur oder bestehender Routen.

## Decisions

### 1. Rechte: `Capability` plus `Authorizer` im Core, Anzeige leitet sich daraus ab

Eine Aufzählung `Capability` benennt jede Handlung (`EditOwnContact`, `EditMember`, `Appoint`, `EditBoat(BoatId)`, `BlockBoat(BoatId)`, `PostPayment`, `ReadAccount(MemberId)`, `ReadAudit` …). `Authorizer` entscheidet auf Basis der `Offices` (bereits als Teilansicht vorhanden), der Aktor-Mitgliedschaft und – wo nötig – kleiner Zusatzdaten (z. B. Eigentümer eines Arbeitseinsatzes). Die bestehende Logik aus `ClubRecord::authorize` wird dorthin verschoben; `ClubRecord::authorize` delegiert, die Tests bleiben unverändert grün.

Die App-Schicht bildet pro Anfrage einen `Principal` (Mitglied, aktive Ämter zum Zeitpunkt) und liefert den Views `can_*`-Flags; Templates entscheiden nie selbst. Jeder schreibende Anwendungsfall ruft zuerst `Authorizer` auf und schlägt mit `AppError::Forbidden` fehl.

*Rechtematrix (Annahmen, siehe Open Questions):*

| Bereich | Mitglied | Bootswart/Abnehmer (Boot) | Oberbootswart | Kassenwart | Vorstand | Revisor |
| --- | --- | --- | --- | --- | --- | --- |
| Eigene Kontaktdaten, Scheine, Lizenzen | schreiben | schreiben | schreiben | schreiben | schreiben | schreiben |
| Fremde Mitgliederdaten | Name, Ämter | wie Mitglied | wie Mitglied | lesen | lesen/schreiben (inkl. Scheine) | lesen |
| Ämter | lesen | lesen | Bootswart/Abnehmer ernennen | lesen | alle schreiben | lesen |
| Boote | lesen | eigene schreiben, sperren, abnehmen | schreiben | lesen | schreiben | lesen |
| Arbeitseinsätze | eigene schreiben | Einsätze des Bootes bestätigen | wie Bootswart, alle Boote | lesen | alle, Abgeltung | lesen |
| Arbeitstermine/Aufgaben | anmelden, eigene Aufgaben | anlegen | anlegen | anlegen | anlegen | lesen |
| Reservierungen | eigene schreiben | alle Reservierungen des Bootes | alle | lesen | alle | lesen |
| Konto/Forderungen | eigenes lesen | – | – | alle schreiben | alle schreiben | alle lesen |
| Protokoll | eigene Einträge | – | – | lesen | lesen | lesen |

Kassenwart und Oberbootswart gehören nach Spec `aemter` zum Vorstand und haben zusätzlich zu den hier genannten Schwerpunkten alle Vorstandsrechte; die Spalten nennen ihre Schwerpunkte.

*Alternative:* Rechte in einer Datenbanktabelle konfigurierbar halten – abgelehnt, weil die Satzung die Rechte festlegt und Konfigurierbarkeit nur Fehlerquellen schafft.

### 2. Schreibpfad: Befehle je Aggregat statt Laden der ganzen Akte

Technisch gibt es dafür einen einzigen schreibenden Port `ChangeRepository::commit(changes, audit)`: `Change` ist `Save{record, expected}` oder `Delete{kind, key, expected}` über die zwölf Fachobjekte (`Record`), alle Änderungen und Protokolleinträge laufen in einer Transaktion und scheitern gemeinsam mit `Conflict`. Lesen übernehmen die vorhandenen Ports, die je Bereich um gezielte Abfragen erweitert werden. Je Bereich gibt es einen Befehls-Dienst in der App-Schicht (`MemberCommands`, `BoatCommands`, `WorkCommands`, `ReservationCommands`, `FinanceCommands`, `OfficeCommands`). Jeder Befehl: Principal bestimmen → Berechtigung prüfen → betroffene Daten über den Port mit Version laden → Fachregel des Core anwenden → mit Versionsprüfung speichern (`UPDATE … WHERE version = ?`) und Protokolleintrag schreiben, alles in einer Transaktion. Reine Anhänge (Einzahlung, Abnahme, Widerruf, Protokoll) brauchen keine Version.

Die Core-Fachregeln, die heute `ClubRecord` brauchen (z. B. Gesamtprüfung einer Reservierung), werden wie schon `MemberYear`/`PermitHistory`/`Offices` auf Teildaten herausgelöst (`ReservationCheck` mit Boot, Besatzung, überschneidenden Reservierungen und Erlaubnissen der Beteiligten). Die Speicherschicht liefert genau diese Teildaten per gezielter Abfrage.

*Alternative:* bei jedem Schreibzugriff die ganze Akte laden und zurückschreiben – abgelehnt: widerspricht der gezielten Speicherung aus `add-web-architektur`, erzeugt Konflikte zwischen unabhängigen Änderungen und skaliert schlecht.

### 3. Gleichzeitige Änderungen: Version im Formular

Formulare zu Wurzelobjekten tragen `version` als verstecktes Feld. Bei Konflikt antwortet die Web-Schicht mit 409 und derselben Seite, die den aktuellen Stand zeigt und die Eingaben erhält; es gibt kein stilles Überschreiben. Anhänge ohne Konfliktpotenzial (Einzahlung) ignorieren die Version.

### 4. Formulare: Post/Redirect/Get, Fehler am Feld, Flash-Meldungen

Jede Aktion ist ein normales HTML-Formular. Ein Formular-Typ je Aktion parst die Eingabe (deutsche Zahl- und Datumsformate, Betrag mit Komma) in einen typisierten Befehl; Fachfehler werden in `FieldErrors` je Feld abgebildet und das Formular mit Status 422 erneut gezeigt. Erfolg führt per `303` zur Ansicht, eine einmalige Meldung („Flash“) liegt in der Sitzung. Die vorhandene CSRF-Prüfung gilt unverändert. htmx ersetzt nur Teilbereiche (`hx-target`), die Handler liefern bei `HX-Request` das Teil-Template, sonst die ganze Seite.

### 5. Änderungsprotokoll als unveränderliche Tabelle

Neue Migration `0002`: `audit_log(id, at, actor_id, action, object_kind, object_id, subject_member_id, summary)` mit Indizes auf `at`, `actor_id`, `subject_member_id` und Triggern, die `UPDATE`/`DELETE` abbrechen. Der Eintrag entsteht in derselben Transaktion wie die Änderung. Es werden Feldnamen, keine Werte gespeichert (Datenschutz). Der Export der Akte enthält das Protokoll nicht; es wird getrennt gesichert.

### 6. Designsystem ohne Framework

Ein Stylesheet `app.css` (ca. 500 Zeilen, in Schichten: Tokens → Basis → Komponenten → Seitenregeln → Druck) mit CSS-Variablen für Farben, Abstände, Radien, Schriftgrößen; hell und dunkel über `prefers-color-scheme`. Systemschriften, keine Webfonts, keine Icon-Fonts (wenige Inline-SVG-Symbole). Bausteine: Seitenkopf mit Navigation, Karte (`.card`), Datenliste (`dl`), Tabelle (`.table` mit `data-label` für die Kartenansicht unter 640 px), Formularraster, Schaltflächen (primär, sekundär, gefährlich), Status-Plakette (`.badge` mit Text), Meldungsleiste, Seitenwechsel, Filterleiste, Tab-artige Unternavigation.

Layout: ein Spaltenraster mit maximal 72 rem; unter 48 rem klappt die Navigation hinter eine `<details>`-Schaltfläche (funktioniert ohne JavaScript), die Seitenleiste entfällt. Interaktionen haben 44 px Mindesthöhe, `:focus-visible` ist deutlich, Farbpaare sind auf 4,5:1 geprüft. Eine Druckansicht blendet Navigation und Schaltflächen aus (Jahresnachweis).

*Alternativen:* ein CSS-Framework (Tailwind, Bootstrap) – abgelehnt wegen Build-Schritt, Größe und strenger CSP; eine Komponentenbibliothek im Browser – abgelehnt, weil Nutzbarkeit ohne JavaScript Anforderung ist.

### 7. Listen: Filter in der Adresse, Seitenwechsel serverseitig

Filter, Sortierung und Seite stehen als Abfrageparameter (`?q=&status=&jahr=&seite=`); jede Liste hat einen Typ für diese Parameter. Die Ports liefern gefilterte, begrenzte Mengen (`LIMIT/OFFSET`), nie die ganze Tabelle.

### 8. Test- und Prüfstrategie

Eine Matrix-Tabelle (Mitglied der Beispielakte × `Capability`) prüft Core und Web gleichzeitig: erlaubt ⇒ Aktion klappt, verboten ⇒ 403 und keine Änderung, Anzeige zeigt die Schaltfläche genau dann, wenn erlaubt. Seitentests laufen wie bisher per `oneshot` mit fester Uhr (2026-10-09). Das Erscheinungsbild wird mit Playwright (vorinstalliertes Chromium) bei 375 px und 1280 px als Bildschirmfoto geprüft; das Skript liegt unter `tools/` und läuft nicht im `cargo test`.

## Risks / Trade-offs

- [Selbst erfasste Scheine bestimmen die Segelerlaubnis ohne Prüfung] → bewusst vom Verein so entschieden (Vertrauensbasis); Absicherung über das Änderungsprotokoll, das alle Änderungen an Scheinen mit Urheber festhält, und über die Bootsabnahme durch Abnehmer/Bootswart.

- [Umfang: alle Bereiche in einer Änderung] → Aufgabengruppen sind nach Abhängigkeit und Nutzen geordnet; jede Gruppe hinterlässt ein laufendes System mit grünen Tests, sodass sich nach jeder Gruppe ausliefern oder anhalten lässt.
- [Anzeige und Server weichen bei Rechten ab] → `Authorizer` ist einzige Quelle, die Matrix-Tests prüfen beide Seiten.
- [Teildaten-Refactoring der Core-Regeln ändert Fachverhalten] → bestehende Core-Tests bleiben unverändert, neue Äquivalenztests vergleichen Teildaten-Prüfung mit der Ganz-Akte-Prüfung für alle Beispiel-Reservierungen.
- [Konflikte bei mehreren gleichzeitigen Nutzern] → Version je Wurzelobjekt, 409 mit Stand-Abgleich.
- [Datenschutz im Protokoll] → nur Feldnamen und Kennungen, keine Werte; Zugriff auf Vorstand, Revisor und Betroffene beschränkt.
- [Mobile Bedienung mit großen Tabellen] → Kartenansicht unter 640 px, wenige Spalten, wichtige Aktion zuerst.
- [Fehlbedienung bei Buchungen] → Storno statt Löschen, Bestätigungsschritt bei zerstörenden Aktionen, Protokoll.

## Migration Plan

1. Migration `0002` legt `audit_log` an (`vereinsakte migrate` bzw. automatisch bei `serve`); bestehende Daten bleiben unberührt.
2. Auslieferung gruppenweise; Rückfall ist das Zurücksetzen der Programmversion, die Tabelle `audit_log` stört alte Versionen nicht.
3. Vor der ersten Nutzung Sicherung mit `vereinsakte export`.

## Open Questions

- Annahmen der Rechtematrix, die der Verein bestätigen sollte (ändern Specs, daher vor der Umsetzung klären; entschieden: Mitglieder pflegen Scheine und Lizenzen selbst, ohne Nachweis): (a) Der Revisor liest alles und ändert nichts. (b) Der Schriftwart hat keine Sonderrechte über den Vorstand hinaus. (c) Alle aktiven Mitglieder sehen Boote, Reservierungskalender, Arbeitstermine und Aufgaben. (d) Jedes Mitglied sieht in „Protokoll“ nur die eigenen Einträge.

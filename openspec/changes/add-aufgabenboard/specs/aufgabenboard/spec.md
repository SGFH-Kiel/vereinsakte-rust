## Purpose

Macht Aufgaben im Web als Kanban-Board mit Kommentaren, Anhängen (Bilder, PDFs) und bearbeitbarer Markdown-Beschreibung nutzbar, damit Verteilung, Stand und Absprachen zu einer Aufgabe an einem Ort liegen.

## ADDED Requirements

### Requirement: Board mit vier Spalten
Das Board SHALL die Aufgaben in den Spalten TODO (offen), WIP (in Arbeit), WAIT (wartet) und DONE (erledigt) zeigen, jede Spalte mit Anzahl. Eine Karte SHALL Titel, Boot, Zuständige, Fälligkeit (überfällig erkennbar mit Text) sowie die Zahl der Kommentare und Bilder zeigen. Alle aktiven Mitglieder SHALL das Board lesen können; der Menüpunkt „Aufgaben“ liegt auf oberster Ebene.

#### Scenario: Board öffnen
- **WHEN** Greta „Aufgaben“ öffnet
- **THEN** sieht sie vier Spalten mit den Aufgaben der laufenden Saison und der Anzahl je Spalte

#### Scenario: Überfällig
- **WHEN** eine offene Aufgabe eine Fälligkeit in der Vergangenheit hat
- **THEN** steht auf der Karte neben der Farbe das Wort „überfällig“

#### Scenario: Eigene Aufgaben
- **WHEN** Ben den Filter „meine“ wählt
- **THEN** erscheinen nur Aufgaben, denen er zugewiesen ist

### Requirement: Aufgaben verschieben
Ein Zuständiger oder Amtsträger SHALL eine Karte in eine andere Spalte verschieben können, per Schaltfläche an der Karte ohne JavaScript und zusätzlich per Ziehen, wenn JavaScript verfügbar ist. Wer nicht zuständig ist und kein Amt hat, SHALL die Schaltflächen nicht sehen, und ein erzwungener Aufruf SHALL mit 403 abgelehnt werden. Gleichzeitige Änderungen SHALL mit einem Konflikthinweis abgelehnt werden, statt still zu überschreiben.

#### Scenario: Verschieben per Schaltfläche
- **WHEN** Ben seine Aufgabe von TODO nach WIP verschiebt
- **THEN** steht sie danach in WIP, und das Protokoll enthält den Statuswechsel

#### Scenario: Nicht berechtigt
- **WHEN** Greta ohne Zuständigkeit eine Aufgabe verschieben will
- **THEN** wird dies mit 403 abgelehnt und die Aufgabe bleibt unverändert

#### Scenario: Konflikt
- **WHEN** zwei Zuständige dieselbe Karte nacheinander auf Basis desselben Standes verschieben
- **THEN** erhält der zweite einen Konflikthinweis mit dem aktuellen Stand

### Requirement: Saisonarchiv
Erledigte Aufgaben, deren Erledigung in einer früheren Saison liegt, SHALL nicht mehr auf dem Board erscheinen und ausschließlich im Archiv stehen. Erledigte Aufgaben der laufenden Saison SHALL in DONE bleiben. Ein Wiederöffnen einer archivierten Aufgabe SHALL sie wieder auf das Board bringen.

#### Scenario: Saisonwechsel
- **WHEN** im Januar 2027 das Board geöffnet wird und eine Aufgabe im März 2026 erledigt wurde
- **THEN** fehlt sie in DONE und steht im Archiv unter „Aufgaben“

#### Scenario: Erledigt in der Saison
- **WHEN** eine Aufgabe im Mai 2026 erledigt wurde und das Board im Juli 2026 geöffnet wird
- **THEN** steht sie in DONE

### Requirement: Aufgabenseite
Jede Aufgabe SHALL eine Seite mit Titel, Status, Zuständigen, Boot, Fälligkeit, geschätzten und geleisteten Stunden, Beschreibung, Anhängen, Kommentaren und dem Verlauf ihrer Statuswechsel haben, mit der Möglichkeit, den Status direkt dort zu ändern. Amtsträger SHALL Aufgaben anlegen und Titel, Zuständige, Boot, Fälligkeit und geschätzte Stunden ändern können.

#### Scenario: Seite öffnen
- **WHEN** ein Mitglied auf eine Karte klickt
- **THEN** erscheint die Aufgabenseite mit allen genannten Angaben

#### Scenario: Neue Aufgabe
- **WHEN** der Oberbootswart eine Aufgabe mit Titel „Fock nähen“ für Ben anlegt
- **THEN** steht sie in TODO und Ben sieht sie unter „meine“

### Requirement: Markdown-Beschreibung
Zuständige und Amtsträger SHALL die Beschreibung einer Aufgabe als Markdown bearbeiten können, mit einer Vorschau vor dem Speichern, die ohne JavaScript funktioniert. Die Darstellung SHALL Überschriften, Hervorhebungen, Listen, Aufgabenlisten, Zitate, Code und Links mit http, https oder mailto unterstützen und eingebettetes HTML nur als Text zeigen. Die Beschreibung SHALL auf höchstens 20.000 Zeichen begrenzt sein.

#### Scenario: Formatierung
- **WHEN** die Beschreibung „**Wichtig:** Fock nähen“ gespeichert wird
- **THEN** erscheint „Wichtig:“ fett

#### Scenario: Kein HTML
- **WHEN** die Beschreibung einen Script-Tag enthält
- **THEN** wird er als Text angezeigt und nicht ausgeführt

#### Scenario: Vorschau
- **WHEN** ohne JavaScript „Vorschau“ gewählt wird
- **THEN** erscheint dieselbe Seite mit gerendertem Text und dem unveränderten Eingabefeld, ohne dass gespeichert wurde

#### Scenario: Fremde Beschreibung
- **WHEN** Greta ohne Zuständigkeit und ohne Amt die Beschreibung ändern will
- **THEN** wird dies mit 403 abgelehnt

### Requirement: Kommentare
Jedes aktive Mitglied SHALL eine Aufgabe kommentieren können. Ein Kommentar SHALL Autor, Zeitpunkt und Text (Markdown wie die Beschreibung, höchstens 5.000 Zeichen) haben und in der Reihenfolge des Eingangs erscheinen. Der Autor SHALL seinen Kommentar innerhalb von 15 Minuten korrigieren und löschen können, danach nur der Vorstand löschen. Ein leerer Kommentar SHALL abgelehnt werden.

#### Scenario: Kommentieren
- **WHEN** Greta zur Aufgabe „Fock nähen“ „Segeltuch ist bestellt“ kommentiert
- **THEN** erscheint der Kommentar mit ihrem Namen und Zeitpunkt unter der Aufgabe, und die Karte zählt einen Kommentar mehr

#### Scenario: Leerer Kommentar
- **WHEN** ein leerer Kommentar abgeschickt wird
- **THEN** zeigt das Formular einen Fehler am Feld und es entsteht kein Kommentar

#### Scenario: Spät korrigieren
- **WHEN** Greta ihren Kommentar nach 30 Minuten ändern will
- **THEN** wird dies abgelehnt, und der Vorstand kann ihn löschen

### Requirement: Anhänge hochladen
Jedes aktive Mitglied SHALL zu einer Aufgabe Bilder (JPEG, PNG, WebP oder GIF) und PDF-Dokumente hochladen können, bis 10 MB je Datei und 20 Anhänge je Aufgabe. Bilder SHALL optional eine Bildbeschreibung (Alternativtext), PDFs eine Bezeichnung tragen. Der Inhalt SHALL anhand der Dateikennung geprüft werden, nicht anhand des Namens oder des angegebenen Typs; andere Formate, auch SVG, SHALL abgelehnt werden.

#### Scenario: Bild hochladen
- **WHEN** Ben ein JPEG von 2 MB zur Aufgabe hochlädt
- **THEN** erscheint es als Vorschaubild, und die Karte zählt einen Anhang mehr

#### Scenario: PDF hochladen
- **WHEN** Ben eine PDF-Datei „Angebot Segeltuch.pdf“ von 1 MB hochlädt
- **THEN** erscheint sie als Dateizeile mit PDF-Symbol, Name und Größe, und die Karte zählt einen Anhang mehr

#### Scenario: Falsches Format
- **WHEN** eine als „bild.png“ oder „angebot.pdf“ benannte Textdatei, eine SVG-Datei oder ein Word-Dokument hochgeladen wird
- **THEN** wird sie abgelehnt mit „Erlaubt sind Bilder (JPEG, PNG, WebP, GIF) und PDF-Dokumente“

#### Scenario: Zu groß
- **WHEN** eine Datei über 10 MB hochgeladen wird
- **THEN** wird sie abgelehnt mit Nennung der Grenze

### Requirement: Anhänge ansehen und löschen
Anhänge SHALL nur angemeldeten aktiven Mitgliedern ausgeliefert werden. Bilder SHALL als Vorschau erscheinen und in voller Größe aufrufbar sein, PDFs SHALL als Datei zum Herunterladen mit ihrem Namen ausgeliefert werden und nicht in der Seite eingebettet erscheinen. Der Hochladende und der Vorstand SHALL einen Anhang löschen können.

#### Scenario: Nicht angemeldet
- **WHEN** die Adresse eines Anhangs ohne Anmeldung aufgerufen wird
- **THEN** wird auf die Anmeldung umgeleitet und nichts ausgeliefert

#### Scenario: PDF herunterladen
- **WHEN** ein Mitglied die PDF-Zeile wählt
- **THEN** lädt der Browser die Datei mit dem hochgeladenen Namen herunter

#### Scenario: Fremden Anhang löschen
- **WHEN** Greta den Anhang von Ben löschen will
- **THEN** wird dies mit 403 abgelehnt, und der Vorstand kann ihn löschen

### Requirement: Protokoll der Aufgabenänderungen
Statuswechsel, Zuweisung, Beschreibungsänderung, Kommentar und Anhangänderungen SHALL im Änderungsprotokoll mit Aktion, Aufgabe und Akteur stehen. Das Protokoll SHALL weder Beschreibungs- noch Kommentartexte noch Dateidaten enthalten, nur Feldnamen und Kennungen.

#### Scenario: Kommentar im Protokoll
- **WHEN** Greta kommentiert
- **THEN** enthält das Protokoll „Aufgabe kommentiert“ mit Greta und der Aufgabe, aber nicht den Kommentartext

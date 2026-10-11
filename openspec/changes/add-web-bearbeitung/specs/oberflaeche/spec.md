# Spec Delta

## Purpose

Beschreibt Aufbau, Darstellung und Bedienung der Weboberfläche als funktionales, modernes Verwaltungswerkzeug, das auf Desktop und Mobilgeräten gleichermaßen nutzbar und ohne JavaScript bedienbar ist.

## ADDED Requirements

### Requirement: Einheitliches Erscheinungsbild
Alle Seiten SHALL dasselbe Layout, dieselben Farben, Abstände und Bedienelemente aus einem gemeinsamen Stylesheet verwenden. Das Design SHALL funktional und zurückhaltend sein, Farbe nur zur Hervorhebung von Zustand und Aktion einsetzen und ohne Fremdbibliotheken und externe Ressourcen auskommen.

#### Scenario: Keine externen Ressourcen
- **WHEN** eine beliebige Seite geladen wird
- **THEN** stammen alle Stylesheets, Skripte, Schriften und Bilder von der eigenen Adresse

### Requirement: Responsive Darstellung
Die Oberfläche SHALL auf Bildschirmbreiten von 360 px bis über 1440 px ohne horizontales Scrollen der Seite benutzbar sein. Tabellen SHALL auf schmalen Bildschirmen als gestapelte Karten mit beschrifteten Feldern erscheinen. Bedienelemente SHALL auf Touch-Geräten mindestens 44 px hoch sein.

#### Scenario: Handy
- **WHEN** das Mitgliederverzeichnis bei 375 px Breite geöffnet wird
- **THEN** erscheint jedes Mitglied als Karte mit beschrifteten Feldern und die Seite scrollt nicht seitwärts

#### Scenario: Desktop
- **WHEN** dieselbe Seite bei 1280 px Breite geöffnet wird
- **THEN** erscheint eine Tabelle mit sortierbaren Spalten

### Requirement: Navigation nach Berechtigung
Die Hauptnavigation SHALL die Bereiche Start, Meine Daten, Mitglieder, Boote, Reservierungen, Arbeit, Konto bzw. Finanzen und, nur mit Berechtigung, Ämter und Protokoll anbieten und den aktuellen Bereich hervorheben. Auf schmalen Bildschirmen SHALL sie hinter einer Menüschaltfläche liegen, die ohne JavaScript funktioniert.

#### Scenario: Navigation für Greta
- **WHEN** Greta ohne Amt eine Seite öffnet
- **THEN** zeigt die Navigation weder „Finanzen“ noch „Protokoll“

#### Scenario: Menü ohne JavaScript
- **WHEN** auf einem schmalen Bildschirm JavaScript deaktiviert ist
- **THEN** lässt sich das Menü öffnen und schließen

### Requirement: Formulare und Rückmeldungen
Jede schreibende Aktion SHALL als Formular mit beschrifteten Feldern, deutschen Fehlermeldungen am betroffenen Feld und erhaltenen Eingaben funktionieren. Nach Erfolg SHALL zur Ansicht weitergeleitet werden (Post/Redirect/Get) und eine Bestätigung erscheinen. Zerstörende Aktionen SHALL eine Bestätigung verlangen.

#### Scenario: Ungültige Eingabe
- **WHEN** ein Arbeitseinsatz mit Stunden und Zeitraum zugleich abgeschickt wird
- **THEN** zeigt dasselbe Formular die Fehlermeldung am Feld und behält die übrigen Eingaben

#### Scenario: Erfolg
- **WHEN** eine Sperre erfolgreich angelegt wurde
- **THEN** wird zur Bootsseite weitergeleitet und eine Bestätigung angezeigt

### Requirement: Fortschrittliche Verbesserung mit htmx
Suchen, Filter und Zustandswechsel einzelner Zeilen SHALL mit htmx ohne Seitenwechsel aktualisiert werden, ohne dass die Funktion von JavaScript abhängt: derselbe Aufruf SHALL ohne htmx als vollständige Seite funktionieren.

#### Scenario: Suche ohne JavaScript
- **WHEN** ohne JavaScript im Verzeichnis nach „peter“ gesucht wird
- **THEN** erscheint eine vollständige Seite mit demselben Ergebnis

### Requirement: Lesbarkeit und Barrierefreiheit
Text SHALL ein Kontrastverhältnis von mindestens 4,5:1 haben, Zustand SHALL nie allein durch Farbe, sondern zusätzlich durch Text oder Symbol erkennbar sein, der Tastaturfokus SHALL sichtbar und jede Aktion per Tastatur erreichbar sein. Die Oberfläche SHALL die Einstellung „dunkles Farbschema“ des Systems beachten und auf Seitenzoom bis 200 % bedienbar bleiben.

#### Scenario: Zustand nicht nur Farbe
- **WHEN** eine Forderung offen ist
- **THEN** steht neben der Farbe das Wort „offen“

### Requirement: Listen mit Suche, Filter und Seiten
Listen mit mehr als 25 Einträgen SHALL seitenweise erscheinen und nach Name oder Bezeichnung durchsuchbar sein; Zeitraum- und Statusfilter SHALL, wo fachlich sinnvoll, in der Adresse stehen, damit sich Ansichten verlinken lassen.

#### Scenario: Verlinkbare Ansicht
- **WHEN** der Kassenwart die Außenstände für 2026 filtert und die Adresse kopiert
- **THEN** zeigt die Adresse bei erneutem Öffnen denselben Filter

### Requirement: Startseite
Die Startseite SHALL dem angemeldeten Mitglied seine nächsten Schritte zeigen: offene Arbeitsstunden, offenes Konto, anstehende Arbeitstermine und eigene Reservierungen sowie, je nach Amt, offene Bestätigungen und Aufgaben.

#### Scenario: Hinweis für Clara
- **WHEN** Clara die Startseite öffnet
- **THEN** sieht sie „es fehlen 4,5 Arbeitsstunden“ und einen Link zum Erfassen eines Einsatzes

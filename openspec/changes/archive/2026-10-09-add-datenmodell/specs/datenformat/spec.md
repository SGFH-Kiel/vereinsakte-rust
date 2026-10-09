# Spec Delta

## Purpose

Legt die gemeinsamen Regeln fest, nach denen alle Fachobjekte der Vereinsakte als JSON dargestellt und wieder eingelesen werden, damit gespeicherte und ausgetauschte Daten stabil und eindeutig lesbar sind.

## ADDED Requirements

### Requirement: Vereinsakte als Gesamtdokument
Alle Daten des Vereins SHALL in einem Wurzelobjekt „Vereinsakte“ zusammengefasst sein, das Listen von Mitgliedern, Booten, Saisons, Jahresnachweisen, Bootsabnahmen, Widerrufen, Arbeitseinsätzen, Stundenübertragen, Reservierungen, Arbeitsterminen, Aufgaben und Ämtern enthält; Forderungen und Einzahlungen stehen im Konto des jeweiligen Mitglieds. Fehlende Listen SHALL beim Einlesen als leer gelten.

#### Scenario: Leere Vereinsakte
- **WHEN** das JSON `{}` als Vereinsakte eingelesen wird
- **THEN** gelingt das Einlesen und alle Listen sind leer

### Requirement: Verlustfreie JSON-Darstellung
Jedes Fachobjekt und die Vereinsakte als Ganzes SHALL sich als JSON serialisieren und aus diesem JSON wieder einlesen lassen, ohne dass Informationen verloren gehen.

#### Scenario: Hin- und Rückweg
- **WHEN** eine vollständig befüllte Vereinsakte als JSON geschrieben und dieses JSON wieder eingelesen wird
- **THEN** ist die eingelesene Vereinsakte gleich der Ausgangsakte

### Requirement: Zeitpunkte als RFC3339 mit Zeitzone
Alle Datums- und Zeitangaben SHALL im JSON als RFC3339-Zeichenkette mit Datum, Uhrzeit und Zeitzonen-Offset erscheinen. Der beim Erfassen angegebene Offset SHALL erhalten bleiben.

#### Scenario: Ausgabe mit Offset
- **WHEN** ein Reservierungsbeginn am 1. Mai 2026 um 10:00 Uhr mit Offset +02:00 serialisiert wird
- **THEN** steht im JSON `"2026-05-01T10:00:00+02:00"`

#### Scenario: Angabe ohne Zeitzone wird abgelehnt
- **WHEN** JSON eingelesen wird, dessen Zeitpunkt `"2026-05-01T10:00:00"` keinen Offset enthält
- **THEN** schlägt das Einlesen mit einem Fehler fehl

#### Scenario: Reines Datum wird abgelehnt
- **WHEN** JSON eingelesen wird, dessen Zeitpunkt nur `"2026-05-01"` lautet
- **THEN** schlägt das Einlesen mit einem Fehler fehl

### Requirement: Eindeutige Kennungen
Jedes eigenständige Fachobjekt SHALL eine Kennung im Feld `id` tragen. Beim Mitglied SHALL die Kennung seine Mitgliedsnummer sein, bei allen anderen Fachobjekten eine UUID-Zeichenkette. Verweise auf andere Objekte SHALL über diese Kennungen erfolgen.

#### Scenario: Neue Objekte erhalten verschiedene Kennungen
- **WHEN** zwei Boote neu angelegt werden
- **THEN** haben beide unterschiedliche `id`-Werte im UUID-Format

#### Scenario: Kennung eines Mitglieds
- **WHEN** Mitglied 42 serialisiert wird
- **THEN** steht im JSON `"id": 42`

#### Scenario: Verweis auf ein Mitglied
- **WHEN** Mitglied 42 als Skipper einer Reservierung eingetragen wird
- **THEN** steht im JSON der Reservierung `"skipper": 42`

#### Scenario: Ungültige Kennung wird abgelehnt
- **WHEN** JSON eines Bootes eingelesen wird, dessen `id` keine gültige UUID ist
- **THEN** schlägt das Einlesen mit einem Fehler fehl

### Requirement: Feldnamen und optionale Felder
Feldnamen und Auswahlwerte im JSON SHALL englisch in snake_case geschrieben sein (z. B. `last_name`, `street`, `"honorary"`); fachliche Kürzel wie Scheinarten bleiben in ihrer deutschen Schreibweise. Optionale Felder ohne Wert und leere Listen SHALL beim Schreiben weggelassen und beim Einlesen als „nicht gesetzt“ bzw. leer behandelt werden.

#### Scenario: Fehlendes optionales Feld
- **WHEN** ein Mitglied eingelesen wird, dessen JSON kein Feld `landline` enthält
- **THEN** gelingt das Einlesen und die Festnetznummer ist nicht gesetzt

### Requirement: Geldbeträge als Dezimalzeichenkette
Geldbeträge SHALL in Euro exakt (ohne Gleitkomma-Rundungsfehler) geführt und im JSON als Zeichenkette mit Punkt und genau zwei Nachkommastellen dargestellt werden. Beim Einlesen SHALL eine Zeichenkette mit höchstens zwei Nachkommastellen akzeptiert werden; Zahlen statt Zeichenketten und mehr als zwei Nachkommastellen SHALL abgelehnt werden.

#### Scenario: Abgeltungsbetrag
- **WHEN** eine Abgeltung von 75,00 € serialisiert wird
- **THEN** steht im JSON `"75.00"`

#### Scenario: Einlesen ohne Nachkommastellen
- **WHEN** der Betrag `"240"` eingelesen wird
- **THEN** ergibt das 240,00 € und wird als `"240.00"` zurückgeschrieben

#### Scenario: Ungültige Beträge
- **WHEN** der Betrag `75.0` (Zahl) oder `"75.005"` eingelesen wird
- **THEN** schlägt das Einlesen mit einem Fehler fehl

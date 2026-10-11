# finanzverwaltung Specification

## Purpose

Macht Konten, Gebühren und Außenstände im Web nutzbar: Mitglieder sehen ihr Konto, der Kassenwart pflegt Saisons, Gebührensätze, Forderungen und Einzahlungen, der Revisor liest alles.

## Requirements

### Requirement: Eigenes Konto
Jedes Mitglied SHALL sein Konto mit Kontostand (Guthaben bzw. offener Betrag), allen Forderungen mit Art, Betrag, Fälligkeit und Zustand (offen, teilweise bezahlt, bezahlt, storniert) und allen Einzahlungen sehen, chronologisch und mit Summen.

#### Scenario: Guthaben
- **WHEN** Anna ihr Konto öffnet
- **THEN** zeigt es „Guthaben“ und „10,00 €“

#### Scenario: Offene Forderung
- **WHEN** Lukas sein Konto öffnet
- **THEN** zeigt es offene Forderungen über insgesamt 276,00 € mit Fälligkeit

### Requirement: Fremde Konten und Außenstände
Der Kassenwart, der Vorstand und der Revisor SHALL jedes Konto öffnen und eine Übersicht der Außenstände zu einem Stichtag mit offenen und überfälligen Beträgen je Mitglied sehen, sortierbar nach Betrag und filterbar nach „überfällig“.

#### Scenario: Überfällig
- **WHEN** der Kassenwart Außenstände zum 20. Juli mit Filter „überfällig“ öffnet
- **THEN** erscheinen nur Mitglieder mit Beträgen, die vor dem 20. Juli fällig waren

#### Scenario: Ohne Berechtigung
- **WHEN** Greta die Außenstände öffnen will
- **THEN** wird dies mit 403 abgelehnt

### Requirement: Forderungen und Einzahlungen erfassen
Der Kassenwart und der Vorstand SHALL Forderungen aller Arten mit Betrag, Text und optionaler Fälligkeit sowie Einzahlungen mit Betrag, Zeitpunkt und Text erfassen können. Beträge SHALL positiv und in Euro mit Cent sein; Eingaben mit Komma oder Punkt als Dezimaltrenner SHALL akzeptiert werden.

#### Scenario: Einzahlung buchen
- **WHEN** der Kassenwart für Anna am 3. April 240,00 € bucht
- **THEN** erscheint die Einzahlung in Annas Konto und der Kontostand verändert sich entsprechend

#### Scenario: Negativer Betrag
- **WHEN** ein Betrag unter null eingegeben wird
- **THEN** zeigt das Formular die Fehlermeldung und speichert nichts

### Requirement: Storno
Der Kassenwart und der Vorstand SHALL eine Forderung mit Begründung stornieren können. Eine stornierte Forderung SHALL mit Zeitpunkt und Grund sichtbar bleiben und nicht mehr in den Kontostand eingehen; Löschen SHALL es nicht geben.

#### Scenario: Storno mit Grund
- **WHEN** der Kassenwart ein Schlüsselpfand von Ben mit dem Grund „Schlüssel zurückgegeben“ storniert
- **THEN** zeigt Bens Konto die Forderung als storniert mit Grund und Zeitpunkt

#### Scenario: Storno ohne Begründung
- **WHEN** der Grund leer bleibt
- **THEN** zeigt das Formular die Fehlermeldung

### Requirement: Saisons und Gebührensätze
Der Kassenwart und der Vorstand SHALL Saisons mit Gebührensätzen (Jahresbeitrag je Mitgliedsart, Aufnahmegebühr, Stundensatz, Tourgebühren je Tarifgruppe) und die Soll-Stunden pflegen können. Bereits angelegte Forderungen SHALL sich dabei nicht ändern.

#### Scenario: Satz ändern
- **WHEN** der Kassenwart den Jahresbeitrag der Saison 2027 ändert
- **THEN** bleiben Forderungen für 2026 und bereits für 2027 angelegte Forderungen unverändert

### Requirement: Jahresbeiträge anlegen
Der Kassenwart SHALL für eine Saison die Jahresbeiträge aller beitragspflichtigen aktiven Mitglieder gesammelt anlegen können, mit Vorschau und ohne Doppelungen: Mitglieder mit bereits vorhandenem Beitrag derselben Saison SHALL übersprungen und als „bereits vorhanden“ ausgewiesen werden.

#### Scenario: Wiederholung
- **WHEN** der Kassenwart die Jahresbeiträge für 2027 zweimal anlegt
- **THEN** entsteht jeder Beitrag nur einmal und die Vorschau der zweiten Ausführung zeigt „bereits vorhanden“

### Requirement: Tourgebühren abrechnen
Der Kassenwart SHALL zu einer Tour-Reservierung die Gebühren nach der Spec `tourgebuehren` berechnen lassen, prüfen und als Forderungen an die Mitglieder bzw. mit Gastname anlegen können, ohne Doppelung bei erneutem Anlegen.

#### Scenario: Tour abrechnen
- **WHEN** der Kassenwart eine siebentägige Tour mit zwei Mitgliedern und einem Gast abrechnet
- **THEN** entstehen zwei Forderungen auf den Mitgliedskonten und eine mit dem Gastnamen beim Skipper, jeweils mit dem berechneten Betrag

### Requirement: Jahresnachweise
Der Vorstand und der Kassenwart SHALL den Jahresnachweis eines Mitglieds (Soll- und anrechenbare Stunden, Beitragsstand, Segelerlaubnis) je Saison sehen und die Segelerlaubnis des Jahres als Nachweis ausdrucken können.

#### Scenario: Ausdruck
- **WHEN** der Vorstand den Nachweis eines Mitglieds druckt
- **THEN** enthält der Druck nur den Nachweis ohne Navigation und Menü
